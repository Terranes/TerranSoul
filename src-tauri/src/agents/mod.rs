//! Multi-agent roster — Chunk 1.5.
//!
//! An **agent** in TerranSoul is a user-named persona with:
//!
//! * a VRM character (two agents may share one),
//! * a [`BrainBackend`] — either TerranSoul's native brain (Free / Paid /
//!   Local Ollama, via [`crate::brain::BrainMode`]) or an **external CLI
//!   worker** (`codex`, `claude`, `gemini`, or a custom allow-listed binary)
//!   bound to a working folder,
//! * an optional working folder used only by `ExternalCli` backends,
//! * a last-active timestamp used by the RAM-cap calculator to decide
//!   which active agent to suspend first if the cap is exceeded.
//!
//! Profiles are persisted as JSON under `<app_data_dir>/agents/<id>.json` so
//! they survive reinstalls (same per-user pattern as Chunk 1.3 VRM storage).
//!
//! The actual child-process execution lives in [`cli_worker`] and is
//! driven by the durable workflow engine in [`crate::workflows`], which
//! records every event to an append-only SQLite log so a killed app can
//! resume pending work.

pub mod cli_worker;
pub mod roster;

pub use roster::{
    default_agent, fresh_id, AgentBackendKind, AgentProfile, AgentRoster, BrainBackend, CliKind,
    MAX_AGENTS,
};
