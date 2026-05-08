//! Background-maintenance scheduler decision logic (Chunk 26.1 —
//! Phase 26 Brain Background Maintenance & Auto-Learn Completion).
//!
//! Pure decision module — given the last-run timestamps for the four
//! brain-maintenance jobs and a `now`, returns the list of jobs whose
//! cool-down has elapsed and that should fire on the next tick.
//!
//! ## Why this module is pure
//!
//! Same rationale as `brain::context_budget` / `brain::segmenter`:
//!
//! - **No I/O** — the caller owns the persistent state and the actual
//!   job dispatch (Tauri command invocation). This module only decides
//!   *which* jobs are due.
//! - **No tokio** — the calling site owns the `tokio::time::interval`
//!   loop. Keeping the decision pure means snapshot tests can pin
//!   "skip if ran in last 23h" / "fire all four in order" without
//!   spinning the runtime.
//! - **Deterministic** — same `(state, now, config)` triple ⇒ same
//!   `Vec<MaintenanceJob>`.
//!
//! The `tokio::time::interval` loop wrapper, persistence into the app
//! data dir, and Tauri-command dispatch land in the follow-up Chunk
//! 26.1b — keeping this PR small and easy to review.

use serde::{Deserialize, Serialize};

/// One brain-maintenance job. Order matters when multiple jobs fire on
/// the same tick: decay first (cheapest, no LLM), GC next, then
/// promotion, then edge extraction last (most LLM-expensive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaintenanceJob {
    /// `apply_memory_decay` — pure SQL update, no LLM.
    Decay,
    /// `garbage_collect_decayed_memories` — pure SQL delete, no LLM.
    GarbageCollect,
    /// `auto_promote_memories` — pure access-pattern heuristic, no LLM.
    PromoteTier,
    /// `extract_edges_via_brain` — LLM call, the only expensive job.
    EdgeExtract,
    /// One-way export of all long-tier memories to an Obsidian vault.
    /// Pure I/O (no LLM). Vault path read from `AppSettings` or
    /// defaults to `<data_dir>/wiki/`.
    ObsidianExport,
    /// Compact the ANN index by rebuilding from live data (Chunk 41.11).
    /// Pure CPU + I/O, no LLM. Only acts when fragmentation > 20%.
    AnnCompact,
}

impl MaintenanceJob {
    /// Canonical execution order (cheap → expensive). The scheduler
    /// uses this to make sure pure SQL jobs run before LLM jobs.
    pub const ORDER: [MaintenanceJob; 6] = [
        MaintenanceJob::Decay,
        MaintenanceJob::GarbageCollect,
        MaintenanceJob::PromoteTier,
        MaintenanceJob::AnnCompact,
        MaintenanceJob::EdgeExtract,
        MaintenanceJob::ObsidianExport,
    ];

    /// Stable string identifier — useful for logging and persistence
    /// keys.
    pub fn as_str(&self) -> &'static str {
        match self {
            MaintenanceJob::Decay => "decay",
            MaintenanceJob::GarbageCollect => "garbage_collect",
            MaintenanceJob::PromoteTier => "promote_tier",
            MaintenanceJob::EdgeExtract => "edge_extract",
            MaintenanceJob::ObsidianExport => "obsidian_export",
            MaintenanceJob::AnnCompact => "ann_compact",
        }
    }
}

/// Per-job cool-down configuration (in milliseconds). All defaults are
/// 23h so a daily tick at the same wall-clock time consistently fires
/// (a 24h cool-down would alternately skip-then-fire when the tick
/// drifts by even a second). Jitter is applied per-call by the caller.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaintenanceConfig {
    pub decay_cooldown_ms: u64,
    pub garbage_collect_cooldown_ms: u64,
    pub promote_tier_cooldown_ms: u64,
    pub edge_extract_cooldown_ms: u64,
    pub obsidian_export_cooldown_ms: u64,
    #[serde(default = "MaintenanceConfig::default_ann_compact_cooldown")]
    pub ann_compact_cooldown_ms: u64,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        let day_minus_one_h: u64 = 23 * 60 * 60 * 1000;
        Self {
            decay_cooldown_ms: day_minus_one_h,
            garbage_collect_cooldown_ms: day_minus_one_h,
            promote_tier_cooldown_ms: day_minus_one_h,
            edge_extract_cooldown_ms: day_minus_one_h,
            obsidian_export_cooldown_ms: day_minus_one_h,
            ann_compact_cooldown_ms: day_minus_one_h,
        }
    }
}

impl MaintenanceConfig {
    fn default_ann_compact_cooldown() -> u64 {
        23 * 60 * 60 * 1000
    }

    pub fn cooldown_for(&self, job: MaintenanceJob) -> u64 {
        match job {
            MaintenanceJob::Decay => self.decay_cooldown_ms,
            MaintenanceJob::GarbageCollect => self.garbage_collect_cooldown_ms,
            MaintenanceJob::PromoteTier => self.promote_tier_cooldown_ms,
            MaintenanceJob::EdgeExtract => self.edge_extract_cooldown_ms,
            MaintenanceJob::ObsidianExport => self.obsidian_export_cooldown_ms,
            MaintenanceJob::AnnCompact => self.ann_compact_cooldown_ms,
        }
    }
}

/// Persisted last-run state. Each field is the Unix-epoch ms when the
/// job last finished successfully. `0` means "never run" — the
/// scheduler treats this as "fire on next tick".
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaintenanceState {
    pub last_decay_ms: u64,
    pub last_garbage_collect_ms: u64,
    pub last_promote_tier_ms: u64,
    pub last_edge_extract_ms: u64,
    #[serde(default)]
    pub last_obsidian_export_ms: u64,
    #[serde(default)]
    pub last_ann_compact_ms: u64,
}

impl MaintenanceState {
    pub fn last_run_for(&self, job: MaintenanceJob) -> u64 {
        match job {
            MaintenanceJob::Decay => self.last_decay_ms,
            MaintenanceJob::GarbageCollect => self.last_garbage_collect_ms,
            MaintenanceJob::PromoteTier => self.last_promote_tier_ms,
            MaintenanceJob::EdgeExtract => self.last_edge_extract_ms,
            MaintenanceJob::ObsidianExport => self.last_obsidian_export_ms,
            MaintenanceJob::AnnCompact => self.last_ann_compact_ms,
        }
    }

    /// Set the last-run timestamp for a job. Useful after the
    /// caller's dispatch returns successfully.
    pub fn record_finished(&mut self, job: MaintenanceJob, finished_at_ms: u64) {
        match job {
            MaintenanceJob::Decay => self.last_decay_ms = finished_at_ms,
            MaintenanceJob::GarbageCollect => self.last_garbage_collect_ms = finished_at_ms,
            MaintenanceJob::PromoteTier => self.last_promote_tier_ms = finished_at_ms,
            MaintenanceJob::EdgeExtract => self.last_edge_extract_ms = finished_at_ms,
            MaintenanceJob::ObsidianExport => self.last_obsidian_export_ms = finished_at_ms,
            MaintenanceJob::AnnCompact => self.last_ann_compact_ms = finished_at_ms,
        }
    }
}

/// Decide which maintenance jobs are due to fire on this tick. Returns
/// jobs in [`MaintenanceJob::ORDER`] (cheap → expensive) so the caller
/// can dispatch them serially without rethinking the order.
///
/// A job fires when `now_ms - state.last_run_for(job) >= cooldown`.
/// `state.last_run_for(job) == 0` (never run) always fires.
///
/// Pure function — `now_ms` is an explicit parameter so tests pin the
/// clock without monkey-patching.
pub fn jobs_due(
    state: &MaintenanceState,
    config: &MaintenanceConfig,
    now_ms: u64,
) -> Vec<MaintenanceJob> {
    let mut due = Vec::with_capacity(4);
    for &job in MaintenanceJob::ORDER.iter() {
        if is_due(state, config, now_ms, job) {
            due.push(job);
        }
    }
    due
}

/// Single-job version of [`jobs_due`]. Useful when the caller wants to
/// fire only one job per tick (e.g. to spread LLM cost over a day).
pub fn is_due(
    state: &MaintenanceState,
    config: &MaintenanceConfig,
    now_ms: u64,
    job: MaintenanceJob,
) -> bool {
    let last = state.last_run_for(job);
    let cooldown = config.cooldown_for(job);
    if last == 0 {
        return true;
    }
    now_ms.saturating_sub(last) >= cooldown
}

/// Compute a small jitter (in ms) to add to the next-tick delay so
/// many devices don't all hit the brain at the same wall-clock minute
/// (the "thundering herd" problem mentioned in the spec). Caller
/// passes a `seed` (e.g. the device-id hash) so the jitter is
/// deterministic per-device but spread across the population.
///
/// Returns a value in `[0, max_jitter_ms)`. `max_jitter_ms == 0`
/// returns `0`.
pub fn jitter_ms(seed: u64, max_jitter_ms: u64) -> u64 {
    if max_jitter_ms == 0 {
        return 0;
    }
    // SplitMix64 step — small, fast, no external dep, good enough for
    // jitter spreading. Source: Vigna 2014 SplitMix.
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    z % max_jitter_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_order_is_cheap_to_expensive() {
        // Edge extraction is the only LLM-expensive job; it must be
        // near the end.
        let order = MaintenanceJob::ORDER;
        assert_eq!(order[0], MaintenanceJob::Decay);
        assert_eq!(order[4], MaintenanceJob::EdgeExtract);
    }

    #[test]
    fn job_as_str_is_stable() {
        // These strings end up in log lines and (later) persistence
        // keys — pin them so a future refactor can't silently change
        // the wire format.
        assert_eq!(MaintenanceJob::Decay.as_str(), "decay");
        assert_eq!(MaintenanceJob::GarbageCollect.as_str(), "garbage_collect");
        assert_eq!(MaintenanceJob::PromoteTier.as_str(), "promote_tier");
        assert_eq!(MaintenanceJob::EdgeExtract.as_str(), "edge_extract");
        assert_eq!(MaintenanceJob::ObsidianExport.as_str(), "obsidian_export");
        assert_eq!(MaintenanceJob::AnnCompact.as_str(), "ann_compact");
    }

    #[test]
    fn never_run_state_fires_all_four_jobs() {
        let state = MaintenanceState::default(); // all zeros
        let cfg = MaintenanceConfig::default();
        let due = jobs_due(&state, &cfg, 1_000_000);
        assert_eq!(due.len(), 6);
        assert_eq!(due, MaintenanceJob::ORDER.to_vec());
    }

    #[test]
    fn job_recently_run_does_not_fire() {
        let cfg = MaintenanceConfig::default();
        let now = 100_000_000_000u64;
        // Decay ran 1 hour ago — should NOT fire (cooldown is 23h).
        let state = MaintenanceState {
            last_decay_ms: now - (60 * 60 * 1000),
            ..Default::default()
        };
        let due = jobs_due(&state, &cfg, now);
        assert!(!due.contains(&MaintenanceJob::Decay));
        // The other five (never run) should still fire.
        assert_eq!(due.len(), 5);
    }

    #[test]
    fn job_at_exact_cooldown_fires() {
        let cfg = MaintenanceConfig::default();
        let now = 100_000_000_000u64;
        let state = MaintenanceState {
            last_decay_ms: now - cfg.decay_cooldown_ms,
            ..Default::default()
        };
        // 23h+ → exactly at threshold → should fire.
        assert!(is_due(&state, &cfg, now, MaintenanceJob::Decay));
    }

    #[test]
    fn job_one_ms_short_of_cooldown_does_not_fire() {
        let cfg = MaintenanceConfig::default();
        let now = 100_000_000_000u64;
        let state = MaintenanceState {
            last_decay_ms: now - cfg.decay_cooldown_ms + 1,
            ..Default::default()
        };
        assert!(!is_due(&state, &cfg, now, MaintenanceJob::Decay));
    }

    #[test]
    fn record_finished_updates_only_target_field() {
        let mut state = MaintenanceState::default();
        state.record_finished(MaintenanceJob::Decay, 12345);
        assert_eq!(state.last_decay_ms, 12345);
        assert_eq!(state.last_garbage_collect_ms, 0);
        assert_eq!(state.last_promote_tier_ms, 0);
        assert_eq!(state.last_edge_extract_ms, 0);
        assert_eq!(state.last_obsidian_export_ms, 0);
    }

    #[test]
    fn jobs_due_returns_in_canonical_order() {
        // Even when only some jobs are due, they come back in
        // ORDER (Decay → GC → Promote → Edge).
        let cfg = MaintenanceConfig::default();
        let now = 100_000_000_000u64;
        // Decay is recent; the other three are never-run.
        let state = MaintenanceState {
            last_decay_ms: now - 1000,
            ..Default::default()
        };
        let due = jobs_due(&state, &cfg, now);
        assert_eq!(
            due,
            vec![
                MaintenanceJob::GarbageCollect,
                MaintenanceJob::PromoteTier,
                MaintenanceJob::AnnCompact,
                MaintenanceJob::EdgeExtract,
                MaintenanceJob::ObsidianExport,
            ]
        );
    }

    #[test]
    fn cooldown_for_each_job_is_independent() {
        // Override only one cooldown; verify the others are unaffected.
        let cfg = MaintenanceConfig {
            decay_cooldown_ms: 1000,
            ..MaintenanceConfig::default()
        };
        assert_eq!(cfg.cooldown_for(MaintenanceJob::Decay), 1000);
        assert_eq!(
            cfg.cooldown_for(MaintenanceJob::GarbageCollect),
            MaintenanceConfig::default().garbage_collect_cooldown_ms
        );
    }

    #[test]
    fn now_at_unix_epoch_zero_does_not_panic() {
        // Defensive: even a clock-broken `now == 0` shouldn't underflow.
        let cfg = MaintenanceConfig::default();
        let state = MaintenanceState::default();
        let due = jobs_due(&state, &cfg, 0);
        // All six are never-run, so all should fire.
        assert_eq!(due.len(), 6);
    }

    #[test]
    fn jitter_zero_max_returns_zero() {
        assert_eq!(jitter_ms(42, 0), 0);
    }

    #[test]
    fn jitter_is_deterministic_for_same_seed() {
        let a = jitter_ms(0xDEAD_BEEF, 60_000);
        let b = jitter_ms(0xDEAD_BEEF, 60_000);
        assert_eq!(a, b);
    }

    #[test]
    fn jitter_stays_within_bounds() {
        for seed in [0u64, 1, 42, 0xFFFF_FFFF, u64::MAX] {
            let j = jitter_ms(seed, 60_000);
            assert!(j < 60_000, "seed {seed} -> {j}");
        }
    }

    #[test]
    fn jitter_spreads_across_seeds() {
        // Different seeds should produce different jitter values most
        // of the time (probabilistically). We sample 10 distinct seeds
        // and require at least 5 distinct results.
        let values: std::collections::HashSet<u64> =
            (0..10).map(|s| jitter_ms(s, 60_000)).collect();
        assert!(values.len() >= 5, "jitter not spreading: {:?}", values);
    }

    #[test]
    fn state_round_trips_through_serde() {
        let s = MaintenanceState {
            last_decay_ms: 1,
            last_garbage_collect_ms: 2,
            last_promote_tier_ms: 3,
            last_edge_extract_ms: 4,
            last_obsidian_export_ms: 5,
            last_ann_compact_ms: 6,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: MaintenanceState = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn config_round_trips_through_serde() {
        let cfg = MaintenanceConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: MaintenanceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn job_round_trips_through_serde() {
        for job in MaintenanceJob::ORDER {
            let json = serde_json::to_string(&job).unwrap();
            let back: MaintenanceJob = serde_json::from_str(&json).unwrap();
            assert_eq!(job, back);
        }
    }
}
