//! App-level resilience: heartbeat telemetry, crash classification,
//! crash-loop guard, and safe-mode flag.
//!
//! Design doc: `docs/availability-slo.md`
//! Milestone: RESILIENCE-1 (Phase INFRA)

pub mod heartbeat;

pub use heartbeat::{
    classify_previous_shutdown, write_clean_exit_marker, CrashLoopGuard, HeartbeatWriter,
    ShutdownClass,
};
