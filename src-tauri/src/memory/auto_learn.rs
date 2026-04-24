//! Auto-learning policy — decides *when* the brain should automatically
//! call `extract_memories_from_session` based on recent conversation
//! activity.
//!
//! This module is the formal "daily conversation updates the brain"
//! decision point. It is a **pure function over state** with no I/O,
//! locks, or LLM calls — so it can be exhaustively unit-tested and is
//! cheap to evaluate after every chat turn.
//!
//! See `docs/brain-advanced-design.md` § 21 ("How Daily Conversation
//! Updates the Brain") for the full write-back / learning loop and how
//! this policy fits into it.
//!
//! ## Default policy
//!
//! - Fire after every **10** turns (one "turn" = one user message +
//!   one assistant reply).
//! - Cooldown: at minimum **3** turns must elapse between consecutive
//!   auto-runs even if the threshold would otherwise be met (prevents
//!   thrash when the user toggles settings mid-session).
//! - Disabled → never fires.
//!
//! These defaults are conservative: a typical user holding a 30-turn
//! conversation with the default brain will trigger 3 auto-extractions,
//! each producing ≤5 facts via `brain_memory::extract_facts`, for a
//! steady-state rate of ~15 new memories per long session.

use serde::{Deserialize, Serialize};

/// Tunable policy controlling auto-learn cadence. Lives in `AppSettings`
/// (see `src-tauri/src/settings/mod.rs`) so the user can override from
/// the Brain hub UI ("Daily learning" card).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoLearnPolicy {
    /// Master toggle — when false, the auto-learner never fires
    /// (the user can still extract manually via the Memory tab button).
    pub enabled: bool,
    /// Fire after every N turns. Must be ≥1; values <1 are clamped.
    pub every_n_turns: u32,
    /// Minimum number of turns between two consecutive auto-runs.
    /// Must be ≥1; values <1 are clamped.
    pub min_cooldown_turns: u32,
}

impl Default for AutoLearnPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            every_n_turns: 10,
            min_cooldown_turns: 3,
        }
    }
}

/// Why the auto-learner did or didn't fire — surfaced to the UI so the
/// user can see exactly what the brain is doing in the background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoLearnDecision {
    /// Fire `extract_memories_from_session` now.
    Fire,
    /// Don't fire — auto-learn is disabled in settings.
    SkipDisabled,
    /// Don't fire — fewer than `every_n_turns` since the last run.
    SkipBelowThreshold {
        turns_until_next: u32,
    },
    /// Don't fire — the cooldown after the previous auto-run hasn't
    /// elapsed yet.
    SkipCooldown {
        turns_remaining: u32,
    },
}

/// Decide whether to fire the auto-learner.
///
/// * `policy` — the user-configured cadence (see [`AutoLearnPolicy`]).
/// * `total_turns` — total assistant replies seen this session.
/// * `last_autolearn_turn` — value of `total_turns` at the moment of
///   the last auto-run, or `None` if the auto-learner has never fired
///   in this session.
///
/// Returns an [`AutoLearnDecision`] describing what to do and (for the
/// "skip" variants) why.
pub fn evaluate(
    policy: AutoLearnPolicy,
    total_turns: u32,
    last_autolearn_turn: Option<u32>,
) -> AutoLearnDecision {
    if !policy.enabled {
        return AutoLearnDecision::SkipDisabled;
    }

    let every = policy.every_n_turns.max(1);
    let cooldown = policy.min_cooldown_turns.max(1);

    let turns_since_last = match last_autolearn_turn {
        None => total_turns,
        Some(prev) => total_turns.saturating_sub(prev),
    };

    if turns_since_last < cooldown {
        return AutoLearnDecision::SkipCooldown {
            turns_remaining: cooldown - turns_since_last,
        };
    }

    if turns_since_last < every {
        return AutoLearnDecision::SkipBelowThreshold {
            turns_until_next: every - turns_since_last,
        };
    }

    AutoLearnDecision::Fire
}

/// Serializable transport-layer view of [`AutoLearnDecision`] for Tauri
/// commands. The frontend only needs to know `should_fire` and the
/// human-readable reason; we flatten the enum here so the JSON payload
/// is stable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoLearnDecisionDto {
    /// True iff the auto-learner should fire now.
    pub should_fire: bool,
    /// One of: `fire`, `disabled`, `below_threshold`, `cooldown`.
    pub reason: String,
    /// For `below_threshold`: turns until the next eligible run.
    /// For `cooldown`: turns left in the cooldown window.
    /// `None` for `fire` / `disabled`.
    pub turns_remaining: Option<u32>,
}

impl From<AutoLearnDecision> for AutoLearnDecisionDto {
    fn from(d: AutoLearnDecision) -> Self {
        match d {
            AutoLearnDecision::Fire => Self {
                should_fire: true,
                reason: "fire".into(),
                turns_remaining: None,
            },
            AutoLearnDecision::SkipDisabled => Self {
                should_fire: false,
                reason: "disabled".into(),
                turns_remaining: None,
            },
            AutoLearnDecision::SkipBelowThreshold { turns_until_next } => Self {
                should_fire: false,
                reason: "below_threshold".into(),
                turns_remaining: Some(turns_until_next),
            },
            AutoLearnDecision::SkipCooldown { turns_remaining } => Self {
                should_fire: false,
                reason: "cooldown".into(),
                turns_remaining: Some(turns_remaining),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled() -> AutoLearnPolicy {
        AutoLearnPolicy {
            enabled: true,
            every_n_turns: 10,
            min_cooldown_turns: 3,
        }
    }

    #[test]
    fn disabled_policy_never_fires() {
        let mut p = enabled();
        p.enabled = false;
        assert_eq!(
            evaluate(p, 9999, Some(0)),
            AutoLearnDecision::SkipDisabled
        );
    }

    #[test]
    fn first_run_fires_at_threshold() {
        let p = enabled();
        // 10 turns elapsed and never fired before → fire.
        assert_eq!(evaluate(p, 10, None), AutoLearnDecision::Fire);
    }

    #[test]
    fn first_run_does_not_fire_below_threshold() {
        let p = enabled();
        match evaluate(p, 7, None) {
            AutoLearnDecision::SkipBelowThreshold { turns_until_next } => {
                assert_eq!(turns_until_next, 3);
            }
            other => panic!("expected SkipBelowThreshold, got {other:?}"),
        }
    }

    #[test]
    fn cooldown_blocks_immediate_refire() {
        let p = enabled();
        // Last fired at turn 10, currently at turn 12 → cooldown=3 not met.
        match evaluate(p, 12, Some(10)) {
            AutoLearnDecision::SkipCooldown { turns_remaining } => {
                assert_eq!(turns_remaining, 1);
            }
            other => panic!("expected SkipCooldown, got {other:?}"),
        }
    }

    #[test]
    fn fires_after_every_n_turns_post_cooldown() {
        let p = enabled();
        // Last fired at turn 10, currently at turn 20 → 10 turns elapsed → fire.
        assert_eq!(evaluate(p, 20, Some(10)), AutoLearnDecision::Fire);
    }

    #[test]
    fn zero_or_negative_threshold_is_clamped_to_one() {
        let p = AutoLearnPolicy {
            enabled: true,
            every_n_turns: 0,
            min_cooldown_turns: 0,
        };
        // With every=1 and cooldown=1, the first turn after the previous
        // run should fire.
        assert_eq!(evaluate(p, 1, Some(0)), AutoLearnDecision::Fire);
    }

    #[test]
    fn no_underflow_when_total_less_than_last() {
        // Defensive: if a caller bug ever passes total_turns < last,
        // saturate to zero rather than panic.
        let p = enabled();
        match evaluate(p, 5, Some(100)) {
            AutoLearnDecision::SkipCooldown { turns_remaining } => {
                assert_eq!(turns_remaining, 3);
            }
            other => panic!("expected SkipCooldown, got {other:?}"),
        }
    }

    #[test]
    fn never_fires_with_disabled_even_at_huge_thresholds() {
        let p = AutoLearnPolicy {
            enabled: false,
            every_n_turns: 1,
            min_cooldown_turns: 1,
        };
        assert_eq!(
            evaluate(p, u32::MAX, None),
            AutoLearnDecision::SkipDisabled
        );
    }

    #[test]
    fn policy_round_trips_through_serde() {
        let p = AutoLearnPolicy {
            enabled: true,
            every_n_turns: 25,
            min_cooldown_turns: 5,
        };
        let s = serde_json::to_string(&p).unwrap();
        let back: AutoLearnPolicy = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn dto_flattens_each_decision_variant() {
        let fire: AutoLearnDecisionDto = AutoLearnDecision::Fire.into();
        assert!(fire.should_fire);
        assert_eq!(fire.reason, "fire");
        assert_eq!(fire.turns_remaining, None);

        let disabled: AutoLearnDecisionDto = AutoLearnDecision::SkipDisabled.into();
        assert!(!disabled.should_fire);
        assert_eq!(disabled.reason, "disabled");

        let below: AutoLearnDecisionDto =
            AutoLearnDecision::SkipBelowThreshold { turns_until_next: 4 }.into();
        assert!(!below.should_fire);
        assert_eq!(below.reason, "below_threshold");
        assert_eq!(below.turns_remaining, Some(4));

        let cooldown: AutoLearnDecisionDto =
            AutoLearnDecision::SkipCooldown { turns_remaining: 2 }.into();
        assert!(!cooldown.should_fire);
        assert_eq!(cooldown.reason, "cooldown");
        assert_eq!(cooldown.turns_remaining, Some(2));
    }
}
