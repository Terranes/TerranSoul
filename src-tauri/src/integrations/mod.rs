//! Companion AI ecosystem integrations.
//!
//! TerranSoul deliberately does not bundle, install, or auto-detect
//! companion AI tools in the background. Detection runs only on an
//! explicit user click in the integrations panel / quest UI; installation
//! always goes through an OS-level elevation prompt (UAC on Windows,
//! `osascript ... with administrator privileges` on macOS, `pkexec` on
//! Linux) so the consent gate lives in the operating system, not in
//! TerranSoul.
//!
//! Chunk reference: **INTEGRATE-1** in `rules/milestones.md`.

pub mod companions;
