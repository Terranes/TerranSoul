//! Durable workflow engine — Chunk 1.5.
//!
//! Inspired by Temporal.io's append-only history + replay pattern but
//! without the heavyweight server stack. Every workflow records its
//! lifecycle (`Started`, `ActivityScheduled`, `ActivityCompleted`,
//! `Heartbeat`, `Completed`, `Failed`, `Cancelled`) as an immutable row
//! in SQLite. On app restart the engine loads every non-terminal
//! workflow and exposes it as `Resuming` so the UI can re-attach.
//!
//! We intentionally do **not** ship a full replay-based determinism
//! engine — that would require trapping every side-effecting call. For
//! TerranSoul's use case (a single CLI subprocess per workflow), the
//! much simpler contract "the history lets us tell the user what
//! happened and lets us re-spawn if we want to retry" is sufficient.

pub mod engine;

pub use engine::{
    WorkflowEngine, WorkflowEvent, WorkflowEventKind, WorkflowId, WorkflowStatus,
    WorkflowSummary,
};
