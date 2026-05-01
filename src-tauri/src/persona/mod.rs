//! Persona-side domain logic that is **not** I/O — pure prompt
//! construction, reply parsing, and JSON normalisation.
//!
//! Tauri commands for persona persistence live in
//! [`crate::commands::persona`]. The brain agent that ferries the prompt
//! to the active LLM lives in [`crate::brain::OllamaAgent`]. This module
//! is the testable seam between them, in the same shape as
//! [`crate::memory::hyde`] and [`crate::memory::reranker`].
//!
//! See `docs/persona-design.md` § 3 (Master-Mirror loop) and § 9.3
//! (LLM-assisted persona authoring).

pub mod drift;
pub mod extract;
pub mod motion_clip;
pub mod motion_feedback;
pub mod pack;
pub mod pose_frame;
pub mod prosody;

#[cfg(feature = "motion-research")]
pub mod motion_smooth;
#[cfg(feature = "motion-research")]
pub mod motion_tokens;
#[cfg(feature = "motion-research")]
pub mod retarget;
