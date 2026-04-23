//! RAM-aware concurrency cap for the multi-agent roster.
//!
//! Each active agent has a RAM footprint that depends on its backend:
//!
//! | Backend                 | Estimated footprint |
//! |-------------------------|---------------------|
//! | Native Free / Paid API  | 200 MB (chat + RAG) |
//! | Native Local Ollama     | 200 MB + model size |
//! | External CLI worker     | 600 MB              |
//!
//! Given `free_ram_mb` (OS-reported **available** RAM) and a list of
//! candidate agents, we return the maximum number that may run
//! simultaneously without pushing the system past a safety reserve.
//!
//! Formula:
//!
//! ```text
//! cap = clamp( floor((free_mb - reserve_mb) / mean_per_agent_mb), 1, 8 )
//! ```
//!
//! where `reserve_mb = 1500` (leave headroom for the host OS and the
//! TerranSoul main process itself). We cap at 8 to match the UX
//! expectation — 8 characters on stage is already more than any real
//! user will juggle at once, and beyond 8 the OS scheduler starts
//! thrashing.
//!
//! The function is **pure** so it can be unit-tested exhaustively; the
//! only impure helper is [`free_ram_mb`] which asks `sysinfo` for the
//! current value.

use serde::Serialize;
use sysinfo::System;

use crate::agents::AgentBackendKind;

/// Safety headroom reserved for the host OS + TerranSoul main process.
pub const RESERVE_MB: u64 = 1500;

/// Lower bound on the cap — even on a tiny laptop, the user can always
/// run exactly one agent (the default) or the UI would be useless.
pub const MIN_CAP: usize = 1;

/// Upper bound on the cap — beyond this we start thrashing.
pub const MAX_CAP: usize = 8;

/// Per-agent footprint estimate in MB.
pub fn estimate_agent_mb(kind: AgentBackendKind, ollama_model_mb: Option<u64>) -> u64 {
    match kind {
        AgentBackendKind::NativeApi => 200,
        AgentBackendKind::LocalOllama => 200 + ollama_model_mb.unwrap_or(2_000),
        AgentBackendKind::ExternalCli => 600,
    }
}

/// Return the OS-reported available RAM in megabytes. This is the
/// "available" figure (free + reclaimable cache), not the "free" figure
/// reported by `top`, because modern OSes aggressively use free RAM as
/// disk cache and the number the user cares about is "how much can I
/// allocate without swapping".
pub fn free_ram_mb() -> u64 {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.available_memory() / 1_048_576
}

/// Inputs for the cap calculator. Each entry is the projected
/// footprint (MB) of activating that agent.
#[derive(Debug, Clone, Serialize)]
pub struct AgentFootprint {
    pub agent_id: String,
    pub kind: AgentBackendKind,
    pub estimated_mb: u64,
}

/// Result of a cap computation.
#[derive(Debug, Clone, Serialize)]
pub struct RamCap {
    pub free_mb: u64,
    pub reserve_mb: u64,
    pub cap: usize,
    pub mean_per_agent_mb: u64,
    pub reasoning: String,
}

/// Pure computation — decide how many agents may run simultaneously
/// given current free RAM and the list of candidate footprints.
///
/// * When the candidate list is empty, uses a default footprint of
///   400 MB (the midpoint of Native API and CLI) as the divisor.
/// * Always returns a value in `[MIN_CAP, MAX_CAP]`.
pub fn compute_max_concurrent_agents(free_mb: u64, agents: &[AgentFootprint]) -> RamCap {
    let mean_per_agent_mb = if agents.is_empty() {
        400
    } else {
        let total: u64 = agents.iter().map(|a| a.estimated_mb).sum();
        // ceil to avoid dividing by a zero-ish number.
        (total / agents.len() as u64).max(100)
    };
    let usable = free_mb.saturating_sub(RESERVE_MB);
    let raw_cap = if mean_per_agent_mb == 0 {
        MAX_CAP
    } else {
        (usable / mean_per_agent_mb) as usize
    };
    let cap = raw_cap.clamp(MIN_CAP, MAX_CAP);

    let reasoning = if free_mb <= RESERVE_MB {
        format!(
            "Only {free_mb} MB free RAM (≤ {RESERVE_MB} MB reserve). Forcing cap to {MIN_CAP}."
        )
    } else if raw_cap > MAX_CAP {
        format!(
            "System has plenty of headroom ({usable} MB usable); capping at hard ceiling {MAX_CAP}."
        )
    } else {
        format!(
            "{usable} MB usable / {mean_per_agent_mb} MB per agent = {cap} concurrent."
        )
    };

    RamCap {
        free_mb,
        reserve_mb: RESERVE_MB,
        cap,
        mean_per_agent_mb,
        reasoning,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(id: &str, mb: u64, kind: AgentBackendKind) -> AgentFootprint {
        AgentFootprint {
            agent_id: id.into(),
            kind,
            estimated_mb: mb,
        }
    }

    #[test]
    fn low_ram_forces_minimum_cap() {
        let out = compute_max_concurrent_agents(1_000, &[]);
        assert_eq!(out.cap, MIN_CAP);
        assert!(out.reasoning.contains("reserve"));
    }

    #[test]
    fn plenty_of_ram_capped_at_max() {
        let out = compute_max_concurrent_agents(64_000, &[fp("a", 200, AgentBackendKind::NativeApi)]);
        assert_eq!(out.cap, MAX_CAP);
    }

    #[test]
    fn mid_ram_uses_floor_division() {
        // 10_000 free − 1_500 reserve = 8_500 usable ; mean = 600 MB ; floor(8500/600) = 14 -> clamped to 8.
        let out = compute_max_concurrent_agents(
            10_000,
            &[fp("a", 600, AgentBackendKind::ExternalCli)],
        );
        assert_eq!(out.cap, MAX_CAP);

        // 5_000 free − 1_500 = 3_500 usable ; mean = 2_200 (local ollama + 2 GB model) ; floor = 1 -> clamped to 1.
        let out = compute_max_concurrent_agents(
            5_000,
            &[fp("a", 2_200, AgentBackendKind::LocalOllama)],
        );
        assert_eq!(out.cap, 1);
    }

    #[test]
    fn cap_scales_with_mixed_footprints() {
        // Mean of 200 + 600 = 400 ; 16_000 − 1_500 = 14_500 ; floor(14500/400) = 36 → clamp 8.
        let out = compute_max_concurrent_agents(
            16_000,
            &[
                fp("a", 200, AgentBackendKind::NativeApi),
                fp("b", 600, AgentBackendKind::ExternalCli),
            ],
        );
        assert_eq!(out.cap, MAX_CAP);

        // Mean of 200 + 600 = 400 ; 3_000 − 1_500 = 1_500 ; floor(1500/400) = 3.
        let out = compute_max_concurrent_agents(
            3_000,
            &[
                fp("a", 200, AgentBackendKind::NativeApi),
                fp("b", 600, AgentBackendKind::ExternalCli),
            ],
        );
        assert_eq!(out.cap, 3);
    }

    #[test]
    fn estimate_agent_mb_uses_model_size_for_ollama() {
        assert_eq!(estimate_agent_mb(AgentBackendKind::NativeApi, None), 200);
        assert_eq!(
            estimate_agent_mb(AgentBackendKind::LocalOllama, Some(4_000)),
            4_200
        );
        assert_eq!(estimate_agent_mb(AgentBackendKind::ExternalCli, None), 600);
    }

    #[test]
    fn cap_never_below_min_never_above_max() {
        for free in [0_u64, 100, 500, 1_000, 1_500, 2_000, 4_000, 8_000, 32_000, 128_000] {
            for mb in [100_u64, 200, 600, 2_200, 8_000] {
                let out = compute_max_concurrent_agents(
                    free,
                    &[fp("x", mb, AgentBackendKind::NativeApi)],
                );
                assert!(out.cap >= MIN_CAP, "cap {} < MIN {}", out.cap, MIN_CAP);
                assert!(out.cap <= MAX_CAP, "cap {} > MAX {}", out.cap, MAX_CAP);
            }
        }
    }

    #[test]
    fn free_ram_mb_is_non_zero_on_real_system() {
        // This is an integration-ish smoke test: any machine the tests
        // run on must have > 0 MB of free RAM, else the harness itself
        // couldn't run.
        let mb = free_ram_mb();
        assert!(mb > 0, "free_ram_mb reported 0 — sysinfo broken?");
    }
}
