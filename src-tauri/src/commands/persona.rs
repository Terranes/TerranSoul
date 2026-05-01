//! Tauri commands for persona persistence (main chain — no camera).
//!
//! Stores the active [`PersonaTraits`] JSON, a library of learned-expression
//! presets, and a library of learned-motion clips on disk under
//! `<app_data_dir>/persona/`. See `docs/persona-design.md` § 11 for the
//! full storage layout and § 12 for the command surface.
//!
//! ## Privacy contract (mandatory; see persona-design.md § 5)
//!
//! - **No camera commands exist in this module.** Webcam frames are
//!   processed entirely in the WebView (browser-only) by MediaPipe Tasks
//!   Vision. Only post-processed, user-confirmed JSON landmark artifacts
//!   ever cross the Tauri IPC boundary, and only on an explicit "Save"
//!   click — never automatically.
//! - **No persistent "camera enabled" state.** Per-session consent lives
//!   only in the frontend Pinia store and is never written here.
//!
//! ## Persona block routing
//!
//! `set_persona_block` lets the frontend push the rendered `[PERSONA]`
//! string to the backend so server-driven streaming paths (the Rust
//! `streaming.rs` Ollama / OpenAI clients) can splice it into the system
//! prompt alongside the existing `[LONG-TERM MEMORY]` block. The browser
//! streaming path renders the same block locally from `persona-prompt.ts`
//! and bypasses this round-trip.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

/// Sub-folder under `data_dir` that holds every persona artifact.
const PERSONA_DIR: &str = "persona";
/// Filename of the single active persona traits document.
const TRAITS_FILE: &str = "persona.json";
/// Sub-folder for learned facial expression preset JSON files.
const EXPRESSIONS_DIR: &str = "expressions";
/// Sub-folder for learned motion clip JSON files.
const MOTIONS_DIR: &str = "motions";

/// Default persona JSON used when no `persona.json` exists yet on disk.
/// Mirrors `defaultPersona()` in `src/stores/persona-types.ts`.
fn default_persona_json() -> &'static str {
    r#"{
  "version": 1,
  "name": "Soul",
  "role": "TerranSoul companion",
  "bio": "A curious AI companion who learns who you are over time.",
  "tone": ["warm", "concise"],
  "quirks": [],
  "avoid": ["unsolicited medical, legal, or financial advice"],
  "active": true,
  "updatedAt": 0
}"#
}

/// Resolve the persona root and ensure it exists.
fn persona_root(data_dir: &Path) -> Result<PathBuf, String> {
    let root = data_dir.join(PERSONA_DIR);
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("Failed to create persona directory: {e}"))?;
    Ok(root)
}

/// Resolve a sub-folder under the persona root and ensure it exists.
fn persona_subdir(data_dir: &Path, name: &str) -> Result<PathBuf, String> {
    let dir = persona_root(data_dir)?.join(name);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create persona sub-directory: {e}"))?;
    Ok(dir)
}

/// Validate an artifact id (used as a filename component). Rejects anything
/// other than `[A-Za-z0-9_-]+` so path-traversal and exotic filename attacks
/// are impossible regardless of caller behaviour.
fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 128 {
        return Err("Invalid persona artifact id (length out of range)".to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Invalid persona artifact id (illegal characters)".to_string());
    }
    Ok(())
}

/// Atomic write: write to `<dest>.tmp` then rename. Same shape used elsewhere
/// in TerranSoul (settings store, brain config) so power-loss can't leave a
/// half-written persona file behind. Uses `with_file_name(...)` instead of
/// `with_extension(...)` so multi-dot filenames (e.g. `foo.bar.json`) get a
/// correct sibling temp file (`foo.bar.json.tmp`) rather than a clobbered
/// extension.
fn atomic_write(dest: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {e}"))?;
    }
    let file_name = dest
        .file_name()
        .ok_or_else(|| "Persona destination has no file name".to_string())?
        .to_string_lossy()
        .into_owned();
    let tmp = dest.with_file_name(format!("{file_name}.tmp"));
    std::fs::write(&tmp, contents).map_err(|e| format!("Failed to write temp file: {e}"))?;
    std::fs::rename(&tmp, dest).map_err(|e| format!("Failed to commit write: {e}"))?;
    Ok(())
}

// ── persona traits ──────────────────────────────────────────────────────────

/// Get the active persona traits JSON, materialising the default on first call.
#[tauri::command]
pub async fn get_persona(state: State<'_, AppState>) -> Result<String, String> {
    let path = persona_root(&state.data_dir)?.join(TRAITS_FILE);
    if !path.exists() {
        return Ok(default_persona_json().to_string());
    }
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read persona: {e}"))
}

/// Persist the active persona traits JSON.
#[tauri::command]
pub async fn save_persona(json: String, state: State<'_, AppState>) -> Result<(), String> {
    serde_json::from_str::<serde_json::Value>(&json)
        .map_err(|e| format!("Invalid persona JSON: {e}"))?;
    let path = persona_root(&state.data_dir)?.join(TRAITS_FILE);
    atomic_write(&path, &json)
}

// ── persona block routing ───────────────────────────────────────────────────

/// Push the rendered `[PERSONA]` block into the shared `AppState.persona_block`
/// slot so server-driven streaming paths splice it into the system prompt.
///
/// An empty string clears the slot — used when the persona is toggled
/// inactive or all fields are blank.
#[tauri::command]
pub async fn set_persona_block(block: String, state: State<'_, AppState>) -> Result<(), String> {
    if block.len() > 8192 {
        return Err("Persona block too large (>8 KiB)".to_string());
    }
    let mut slot = state
        .persona_block
        .lock()
        .map_err(|e| format!("Persona block lock poisoned: {e}"))?;
    *slot = block;
    Ok(())
}

/// Read the current persona block (mostly used by tests + by the streaming
/// pipelines themselves; exposed as a command to ease frontend debugging).
#[tauri::command]
pub async fn get_persona_block(state: State<'_, AppState>) -> Result<String, String> {
    let slot = state
        .persona_block
        .lock()
        .map_err(|e| format!("Persona block lock poisoned: {e}"))?;
    Ok(slot.clone())
}

// ── handoff block routing (Chunk 23.2b) ────────────────────────────────────
//
// The frontend agent-roster store records a per-agent conversation-window
// summary on switchAgent and runs it through `buildHandoffBlock` (pure
// utility shipped in Chunk 23.2a) to produce a `[HANDOFF FROM <prev>]`
// block. That block is pushed here and spliced into the next system
// prompt by streaming.rs. **One-shot**: streaming reads-and-clears so
// the new agent is briefed exactly once.

/// Push a rendered `[HANDOFF FROM <prev>]` block into `AppState.handoff_block`.
/// Empty string clears the slot (useful for explicit reset on agent revert).
#[tauri::command]
pub async fn set_handoff_block(block: String, state: State<'_, AppState>) -> Result<(), String> {
    if block.len() > 8192 {
        return Err("Handoff block too large (>8 KiB)".to_string());
    }
    let mut slot = state
        .handoff_block
        .lock()
        .map_err(|e| format!("Handoff block lock poisoned: {e}"))?;
    *slot = block;
    Ok(())
}

/// Read the current handoff block (mostly for tests / debugging — the
/// streaming pipeline reads-and-clears via `state.handoff_block` directly).
#[tauri::command]
pub async fn get_handoff_block(state: State<'_, AppState>) -> Result<String, String> {
    let slot = state
        .handoff_block
        .lock()
        .map_err(|e| format!("Handoff block lock poisoned: {e}"))?;
    Ok(slot.clone())
}

#[cfg(test)]
mod handoff_tests {
    use super::*;

    fn block_state() -> AppState {
        AppState::for_test()
    }

    #[tokio::test]
    async fn set_then_get_round_trips() {
        let state = block_state();
        // We can't easily go through `State<'_, AppState>` in a unit test
        // without a Tauri app handle, so exercise the underlying mutex
        // directly — same code path the command uses.
        {
            let mut slot = state.handoff_block.lock().unwrap();
            *slot = "\n\n[HANDOFF FROM A]\nfoo\n[/HANDOFF]".to_string();
        }
        let got = state.handoff_block.lock().unwrap().clone();
        assert!(got.contains("[HANDOFF FROM A]"));
    }

    #[tokio::test]
    async fn empty_string_clears_slot() {
        let state = block_state();
        {
            let mut slot = state.handoff_block.lock().unwrap();
            *slot = "previous".to_string();
        }
        {
            let mut slot = state.handoff_block.lock().unwrap();
            *slot = String::new();
        }
        assert_eq!(state.handoff_block.lock().unwrap().as_str(), "");
    }
}

// ── learned expressions (side-chain artifacts; storage shipped early) ──────

/// Generic "JSON document with an id" envelope for the listing commands.
/// We deliberately do NOT typecheck the inner shape here — the frontend
/// `LearnedExpression` / `LearnedMotion` schemas may evolve faster than
/// the backend, and the backend's job is only to be a faithful filesystem
/// archive. The frontend `migratePersonaTraits`-style layer handles
/// schema changes.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LearnedAsset {
    pub id: String,
    #[serde(flatten)]
    pub rest: serde_json::Map<String, serde_json::Value>,
}

fn list_assets(dir: &Path) -> Result<Vec<LearnedAsset>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out: Vec<LearnedAsset> = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("Failed to list directory: {e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue, // Skip unreadable; non-blocking per design § 13.
        };
        match serde_json::from_str::<LearnedAsset>(&raw) {
            Ok(asset) => out.push(asset),
            Err(_) => continue, // Skip corrupt; non-blocking per design § 13.
        }
    }
    // Newest first (by `learnedAt` if present, else by filename).
    out.sort_by(|a, b| {
        let ta = a
            .rest
            .get("learnedAt")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let tb = b
            .rest
            .get("learnedAt")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        tb.cmp(&ta)
    });
    Ok(out)
}

fn save_asset(dir: &Path, json: &str) -> Result<(), String> {
    let parsed: LearnedAsset = serde_json::from_str(json)
        .map_err(|e| format!("Invalid learned asset JSON: {e}"))?;
    validate_id(&parsed.id)?;
    let path = dir.join(format!("{}.json", parsed.id));
    atomic_write(&path, json)
}

fn delete_asset(dir: &Path, id: &str) -> Result<(), String> {
    validate_id(id)?;
    let path = dir.join(format!("{id}.json"));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to delete: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn list_learned_expressions(
    state: State<'_, AppState>,
) -> Result<Vec<LearnedAsset>, String> {
    let dir = persona_subdir(&state.data_dir, EXPRESSIONS_DIR)?;
    list_assets(&dir)
}

#[tauri::command]
pub async fn save_learned_expression(
    json: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let dir = persona_subdir(&state.data_dir, EXPRESSIONS_DIR)?;
    save_asset(&dir, &json)
}

#[tauri::command]
pub async fn delete_learned_expression(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let dir = persona_subdir(&state.data_dir, EXPRESSIONS_DIR)?;
    delete_asset(&dir, &id)
}

// ── learned motions ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_learned_motions(
    state: State<'_, AppState>,
) -> Result<Vec<LearnedAsset>, String> {
    let dir = persona_subdir(&state.data_dir, MOTIONS_DIR)?;
    list_assets(&dir)
}

#[tauri::command]
pub async fn save_learned_motion(
    json: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let dir = persona_subdir(&state.data_dir, MOTIONS_DIR)?;
    save_asset(&dir, &json)
}

#[tauri::command]
pub async fn delete_learned_motion(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let dir = persona_subdir(&state.data_dir, MOTIONS_DIR)?;
    delete_asset(&dir, &id)
}

// ── brain-extracted persona suggestion (Chunk 14.2 — Master-Echo loop) ──────

/// Ask the active brain to propose a [`PersonaCandidate`] from the
/// user's recent conversation history + their long-term `personal:*`
/// memories. Returns the candidate as a JSON string the frontend
/// presents in the review-before-apply card; **nothing is written to
/// disk** in this command — application happens via the existing
/// `save_persona` command after the user clicks Apply.
///
/// Returns an error string when no brain is configured (so the UI can
/// disable the button + show a tooltip per `docs/persona-design.md`
/// § 13). Returns `Ok("")` when a brain is configured but the reply
/// could not be parsed — caller treats empty as "couldn't suggest right
/// now, try again". Never auto-saves.
#[tauri::command]
pub async fn extract_persona_from_brain(state: State<'_, AppState>) -> Result<String, String> {
    let model = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;

    // Snapshot the conversation history without holding the lock across
    // the await point.
    let history: Vec<(String, String)> = {
        let conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    };

    // Snapshot long-tier memories (the canonical "personal-tier" — see
    // `docs/persona-design.md` § 9.3) likewise without holding the lock.
    let memories: Vec<(String, String)> = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store
            .get_by_tier(&crate::memory::MemoryTier::Long)
            .unwrap_or_default()
            .into_iter()
            .map(|m| (m.content, m.tags))
            .collect()
    };

    let snippets = crate::persona::extract::assemble_snippets(&history, &memories);

    // Audio-prosody hints (Chunk 14.6) — only computed when the user has
    // ASR configured, so their typed turns reflect spoken patterns. The
    // analyzer is pure and I/O-free; we never read raw audio (it's gone
    // by the time text reaches the message log) and we never persist
    // the hints — they live only for the duration of this prompt.
    let asr_configured = state
        .voice_config
        .lock()
        .map_err(|e| e.to_string())?
        .asr_provider
        .is_some();
    let prosody_block: Option<String> = if asr_configured {
        let user_utterances: Vec<&str> = history
            .iter()
            .filter(|(role, _)| role.eq_ignore_ascii_case("user"))
            .map(|(_, content)| content.as_str())
            .collect();
        let hints = crate::persona::prosody::analyze_user_utterances(&user_utterances);
        crate::persona::prosody::render_prosody_block(&hints)
    } else {
        None
    };

    let agent = crate::brain::OllamaAgent::new(&model);
    match agent
        .propose_persona_with_hints(&snippets, prosody_block.as_deref())
        .await
    {
        Some(candidate) => serde_json::to_string(&candidate)
            .map_err(|e| format!("Failed to serialise persona candidate: {e}")),
        // Empty string = "brain replied but couldn't be parsed". UI
        // surfaces a soft "try again" message rather than a hard error.
        None => Ok(String::new()),
    }
}

// ── LLM-as-Animator: motion-clip generation (Chunk 14.16c2) ────────────────

/// Result envelope for [`generate_motion_from_text`].
///
/// Carries the generated `LearnedMotion` JSON (frontend-ready, never
/// auto-saved) plus parser diagnostics so the UI can surface "we had
/// to clean N frames" hints in the preview panel. Mirrors the
/// "preview-before-save" contract that `extract_persona_from_brain`
/// already follows — the frontend calls `save_learned_motion` after
/// the user clicks Accept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedMotionEnvelope {
    /// Pretty-printed JSON of the generated motion clip, ready to drop
    /// straight into `save_learned_motion` if the user accepts.
    pub motion_json: String,
    /// Slugged trigger word the parser produced from the user's
    /// description (`"learned-<slug>"`).
    pub trigger: String,
    /// Frames the parser dropped (invalid / unknown bones / bad shape).
    pub dropped_frames: usize,
    /// Bone components clamped into the safe rotation range.
    pub clamped_components: usize,
    /// Whether the parser had to repair non-monotonic timestamps.
    pub repaired_timestamps: bool,
    /// Whether the parser had to renormalise an over/undershooting duration.
    pub repaired_duration: bool,
}

/// Ask the active brain to generate a multi-frame VRM motion clip from
/// a short text description. Returns the parsed clip as a JSON string
/// the frontend previews (and saves on user accept) via the existing
/// `save_learned_motion` command — **nothing is written to disk here.**
///
/// Routes through `memory::brain_memory::complete_via_mode` so it works
/// across all four brain modes (Free / Paid / Local Ollama / LM Studio)
/// without bespoke per-mode wiring. When no `brain_mode` is configured
/// it falls back to the legacy `active_brain` Ollama model so users on
/// the old single-mode setup still get the feature.
///
/// Returns `Err` only on hard configuration / network failures; LLM
/// reply parse errors are surfaced as a typed message the UI shows in
/// the "couldn't generate, try again" toast.
#[tauri::command]
pub async fn generate_motion_from_text(
    description: String,
    duration_s: Option<f32>,
    fps: Option<u32>,
    state: State<'_, AppState>,
) -> Result<GeneratedMotionEnvelope, String> {
    use crate::persona::motion_clip::{
        build_motion_prompt_with_hint, parse_motion_payload, slugify_trigger, MotionRequest,
        DEFAULT_DURATION_S, DEFAULT_FPS,
    };

    let trimmed = description.trim();
    if trimmed.is_empty() {
        return Err("Description must not be empty".to_string());
    }
    if trimmed.len() > 500 {
        return Err("Description too long (max 500 chars)".to_string());
    }

    let request = MotionRequest {
        description: trimmed.to_string(),
        duration_s: duration_s.unwrap_or(DEFAULT_DURATION_S),
        fps: fps.unwrap_or(DEFAULT_FPS),
    }
    .sanitised();

    // Self-improve hint (Chunk 14.16e): pull the trusted-trigger
    // leaderboard from the feedback log so the brain can match the
    // user's preferred movement vocabulary. Empty when the user has
    // never accepted a generated motion before — the hint is a no-op
    // in that case.
    let hint = {
        let log_path = motion_feedback_path(&state.data_dir)?;
        let entries = crate::persona::motion_feedback::load_entries(&log_path)
            .unwrap_or_default();
        let stats = crate::persona::motion_feedback::aggregate_stats(&entries);
        crate::persona::motion_feedback::render_prompt_hint(&stats)
    };

    let (system_prompt, user_prompt) = build_motion_prompt_with_hint(&request, &hint);

    // Resolve brain mode → fall back to legacy `active_brain` Ollama.
    let brain_mode = state
        .brain_mode
        .lock()
        .map_err(|e| format!("brain_mode lock: {e}"))?
        .clone();
    let legacy_model = state
        .active_brain
        .lock()
        .map_err(|e| format!("active_brain lock: {e}"))?
        .clone();

    let effective_mode = match brain_mode {
        Some(m) => m,
        None => {
            let model = legacy_model.ok_or_else(|| {
                "No brain configured. Set up a brain first.".to_string()
            })?;
            crate::brain::BrainMode::LocalOllama { model }
        }
    };

    let reply = crate::memory::brain_memory::complete_via_mode(
        &effective_mode,
        &system_prompt,
        &user_prompt,
        &state.provider_rotator,
    )
    .await
    .map_err(|e| format!("Brain request failed: {e}"))?;

    let trigger = slugify_trigger(trimmed);
    let id = format!("motion-{}", uuid::Uuid::new_v4());
    let name = trimmed.chars().take(60).collect::<String>();
    let learned_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let (motion, diag) = parse_motion_payload(
        &reply,
        id,
        name,
        trigger.clone(),
        request.fps,
        request.duration_s,
        learned_at,
    )
    .map_err(|e| format!("Failed to parse motion clip from brain reply: {e}"))?;

    let motion_json = serde_json::to_string_pretty(&motion)
        .map_err(|e| format!("Failed to serialise generated motion: {e}"))?;

    Ok(GeneratedMotionEnvelope {
        motion_json,
        trigger,
        dropped_frames: diag.dropped_frames,
        clamped_components: diag.clamped_components,
        repaired_timestamps: diag.repaired_timestamps,
        repaired_duration: diag.repaired_duration,
    })
}

// ── motion-feedback log (Chunk 14.16e — self-improve loop) ─────────────────

/// Filename of the motion-feedback log under the persona root.
const MOTION_FEEDBACK_FILE: &str = "motion_feedback.jsonl";

/// Resolve the on-disk path of the motion-feedback log, ensuring the
/// persona root exists. Pure helper so the generator command + the
/// feedback commands stay DRY.
fn motion_feedback_path(data_dir: &Path) -> Result<PathBuf, String> {
    Ok(persona_root(data_dir)?.join(MOTION_FEEDBACK_FILE))
}

/// Frontend payload mirror of
/// [`crate::persona::motion_feedback::MotionFeedbackEntry`]. Lives here
/// so we can derive `Deserialize` for the Tauri argument without
/// requiring the frontend to know about backend filenames or to set
/// the `at` timestamp itself (we stamp it server-side).
#[derive(Debug, Clone, Deserialize)]
pub struct MotionFeedbackPayload {
    pub description: String,
    pub trigger: String,
    pub verdict: crate::persona::motion_feedback::FeedbackVerdict,
    #[serde(default)]
    pub duration_s: f32,
    #[serde(default)]
    pub fps: u32,
    #[serde(default)]
    pub dropped_frames: usize,
    #[serde(default)]
    pub clamped_components: usize,
}

/// Append a single accept/reject event to the motion-feedback log.
///
/// Called by `PersonaMotionGenerator.vue` whenever the user clicks
/// Accept or Discard on a generated motion candidate. The log feeds
/// the "trusted triggers" hint that
/// [`generate_motion_from_text`] splices into the system prompt next
/// time, closing the self-improve loop.
#[tauri::command]
pub async fn record_motion_feedback(
    payload: MotionFeedbackPayload,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if payload.description.trim().is_empty() {
        return Err("description must not be empty".to_string());
    }
    if payload.trigger.trim().is_empty() {
        return Err("trigger must not be empty".to_string());
    }
    if payload.description.len() > 500 {
        return Err("description too long (>500 chars)".to_string());
    }
    let entry = crate::persona::motion_feedback::MotionFeedbackEntry {
        description: payload.description,
        trigger: payload.trigger,
        verdict: payload.verdict,
        duration_s: payload.duration_s,
        fps: payload.fps,
        dropped_frames: payload.dropped_frames,
        clamped_components: payload.clamped_components,
        at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0),
    };
    let path = motion_feedback_path(&state.data_dir)?;
    crate::persona::motion_feedback::append_entry(&path, &entry)
        .map_err(|e| format!("Failed to record motion feedback: {e}"))
}

/// Read aggregated stats over the motion-feedback log. Powers the
/// "you've taught me N motions" UI pill in the persona panel and the
/// trusted-trigger leaderboard.
#[tauri::command]
pub async fn get_motion_feedback_stats(
    state: State<'_, AppState>,
) -> Result<crate::persona::motion_feedback::MotionFeedbackStats, String> {
    let path = motion_feedback_path(&state.data_dir)?;
    let entries = crate::persona::motion_feedback::load_entries(&path)
        .map_err(|e| format!("Failed to load motion feedback: {e}"))?;
    Ok(crate::persona::motion_feedback::aggregate_stats(&entries))
}

// ── persona drift detection (Chunk 14.8) ────────────────────────────────────

/// Compare the active persona traits against the user's `personal:*`
/// memories and return a [`crate::persona::drift::DriftReport`] indicating
/// whether the persona has drifted from the user's evolving interests.
///
/// This command is called by the frontend's auto-learn loop after a
/// configurable number of facts have been accumulated (default 25).
/// It is deliberately **not** a background loop — it piggybacks on the
/// existing auto-learn cadence so the brain is only bothered when the
/// user is actively chatting.
///
/// Returns an error string when no brain is configured. Returns a
/// "no drift" report when the brain replies but can't be parsed.
#[tauri::command]
pub async fn check_persona_drift(
    state: State<'_, AppState>,
) -> Result<crate::persona::drift::DriftReport, String> {
    let model = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;

    // Read the active persona traits from disk.
    let traits_path = persona_root(&state.data_dir)?.join(TRAITS_FILE);
    let persona_json = if traits_path.exists() {
        std::fs::read_to_string(&traits_path)
            .map_err(|e| format!("Failed to read persona: {e}"))?
    } else {
        default_persona_json().to_string()
    };

    // Snapshot `personal:*` long-tier memories without holding the lock
    // across the await point.
    let personal_memories: Vec<(String, String)> = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store
            .get_by_tier(&crate::memory::MemoryTier::Long)
            .unwrap_or_default()
            .into_iter()
            .filter(|m| m.tags.to_lowercase().contains("personal:"))
            .map(|m| (m.content, m.tags))
            .collect()
    };

    // Short-circuit: no personal memories → nothing to drift against.
    if personal_memories.is_empty() {
        return Ok(crate::persona::drift::DriftReport {
            drift_detected: false,
            summary: String::new(),
            suggested_changes: Vec::new(),
        });
    }

    let agent = crate::brain::OllamaAgent::new(&model);
    match agent
        .check_persona_drift(&persona_json, &personal_memories)
        .await
    {
        Some(report) => Ok(report),
        // Brain replied but response couldn't be parsed → treat as no drift.
        None => Ok(crate::persona::drift::DriftReport {
            drift_detected: false,
            summary: String::new(),
            suggested_changes: Vec::new(),
        }),
    }
}

// ── persona pack export / import (Chunk 14.7) ───────────────────────────────

/// Build a [`crate::persona::pack::PersonaPack`] from the on-disk
/// persona artifacts and return it as a pretty-printed JSON string the
/// frontend can copy to clipboard / save as a file / drop into Soul
/// Link sync. `note` is an optional free-form one-liner shown in the
/// import preview on the receiving side.
///
/// Reads through the same paths as `get_persona` / `list_learned_*`
/// so the pack is always a faithful snapshot of what the avatar would
/// load on next start. Corrupt asset files are silently skipped (same
/// "non-blocking" contract documented in `docs/persona-design.md`
/// § 13).
#[tauri::command]
pub async fn export_persona_pack(
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use crate::persona::pack::{build_pack, pack_to_string};

    // Traits → JSON value (preserves unknown fields for forward-compat).
    let traits_path = persona_root(&state.data_dir)?.join(TRAITS_FILE);
    let traits_raw = if traits_path.exists() {
        std::fs::read_to_string(&traits_path)
            .map_err(|e| format!("Failed to read persona: {e}"))?
    } else {
        default_persona_json().to_string()
    };
    let traits: serde_json::Value = serde_json::from_str(&traits_raw)
        .map_err(|e| format!("Persona file is not valid JSON: {e}"))?;

    // Assets → opaque JSON values (per-entry shape may evolve).
    let exp_dir = persona_subdir(&state.data_dir, EXPRESSIONS_DIR)?;
    let mot_dir = persona_subdir(&state.data_dir, MOTIONS_DIR)?;
    let expressions = list_assets_as_values(&exp_dir);
    let motions = list_assets_as_values(&mot_dir);

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let pack = build_pack(traits, expressions, motions, note, now_ms);
    pack_to_string(&pack)
}

/// Like [`list_assets`] but returns each file as a raw JSON `Value` so
/// the export pack preserves any extra fields the per-entry struct
/// would otherwise drop. Corrupt files are skipped silently per the
/// design doc § 13 contract.
fn list_assets_as_values(dir: &Path) -> Vec<serde_json::Value> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut out: Vec<serde_json::Value> = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) if v.is_object() => out.push(v),
            _ => continue,
        }
    }
    // Stable ordering for deterministic round-trips: by `learnedAt`
    // ascending, which matches the on-disk creation order.
    out.sort_by_key(|v| {
        v.get("learnedAt")
            .and_then(|x| x.as_i64())
            .unwrap_or(0)
    });
    out
}

/// Apply a user-supplied [`crate::persona::pack::PersonaPack`] JSON
/// string. **Replaces** the persona traits, **merges** the learned
/// asset libraries (existing files with matching ids are overwritten;
/// missing ones are added; pre-existing artifacts not in the pack are
/// left untouched).
///
/// Returns an [`crate::persona::pack::ImportReport`] so the UI can
/// surface "imported 3 expressions, 2 motions, skipped 1 (wrong
/// kind)" in a single round-trip.
#[tauri::command]
pub async fn import_persona_pack(
    json: String,
    state: State<'_, AppState>,
) -> Result<crate::persona::pack::ImportReport, String> {
    use crate::persona::pack::{note_motion_provenance, note_skip, parse_pack, validate_asset, ImportReport};

    let pack = parse_pack(&json)?;
    let mut report = ImportReport::default();

    // ── Traits ──────────────────────────────────────────────────────────
    // Re-serialise the traits value so we write a normalised document
    // (object keys in deterministic-ish order, no leading whitespace
    // / BOM). atomic_write guarantees no half-written state if rename
    // fails — the previous persona.json stays intact.
    let traits_path = persona_root(&state.data_dir)?.join(TRAITS_FILE);
    let traits_str = serde_json::to_string_pretty(&pack.traits)
        .map_err(|e| format!("Failed to serialise traits: {e}"))?;
    atomic_write(&traits_path, &traits_str)?;
    report.traits_applied = true;

    // ── Expressions ─────────────────────────────────────────────────────
    let exp_dir = persona_subdir(&state.data_dir, EXPRESSIONS_DIR)?;
    for (idx, asset) in pack.expressions.iter().enumerate() {
        let id = match validate_asset(asset, "expression") {
            Ok(id) => id,
            Err(e) => {
                note_skip(&mut report, format!("expression #{idx}: {e}"));
                continue;
            }
        };
        let path = exp_dir.join(format!("{id}.json"));
        let body = match serde_json::to_string_pretty(asset) {
            Ok(s) => s,
            Err(e) => {
                note_skip(&mut report, format!("expression {id}: serialise failed: {e}"));
                continue;
            }
        };
        if let Err(e) = atomic_write(&path, &body) {
            note_skip(&mut report, format!("expression {id}: write failed: {e}"));
            continue;
        }
        report.expressions_accepted += 1;
    }

    // ── Motions ─────────────────────────────────────────────────────────
    let mot_dir = persona_subdir(&state.data_dir, MOTIONS_DIR)?;
    for (idx, asset) in pack.motions.iter().enumerate() {
        let id = match validate_asset(asset, "motion") {
            Ok(id) => id,
            Err(e) => {
                note_skip(&mut report, format!("motion #{idx}: {e}"));
                continue;
            }
        };
        let path = mot_dir.join(format!("{id}.json"));
        let body = match serde_json::to_string_pretty(asset) {
            Ok(s) => s,
            Err(e) => {
                note_skip(&mut report, format!("motion {id}: serialise failed: {e}"));
                continue;
            }
        };
        if let Err(e) = atomic_write(&path, &body) {
            note_skip(&mut report, format!("motion {id}: write failed: {e}"));
            continue;
        }
        report.motions_accepted += 1;
        note_motion_provenance(&mut report, asset);
    }

    Ok(report)
}

/// Dry-run: parse the pack and return the per-asset acceptance report
/// without writing anything. Used by the UI's "Preview" button so the
/// user can see "this pack would import 3 expressions and skip 1"
/// before committing.
#[tauri::command]
pub async fn preview_persona_pack(
    json: String,
) -> Result<crate::persona::pack::ImportReport, String> {
    use crate::persona::pack::{note_motion_provenance, note_skip, parse_pack, validate_asset, ImportReport};

    let pack = parse_pack(&json)?;
    let mut report = ImportReport {
        traits_applied: pack.traits.is_object(),
        ..Default::default()
    };
    for (idx, asset) in pack.expressions.iter().enumerate() {
        match validate_asset(asset, "expression") {
            Ok(_) => report.expressions_accepted += 1,
            Err(e) => note_skip(&mut report, format!("expression #{idx}: {e}")),
        }
    }
    for (idx, asset) in pack.motions.iter().enumerate() {
        match validate_asset(asset, "motion") {
            Ok(_) => {
                report.motions_accepted += 1;
                note_motion_provenance(&mut report, asset);
            }
            Err(e) => note_skip(&mut report, format!("motion #{idx}: {e}")),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_persona_json_is_valid_and_active() {
        let v: serde_json::Value = serde_json::from_str(default_persona_json()).unwrap();
        assert_eq!(v["name"], "Soul");
        assert_eq!(v["active"], true);
        assert_eq!(v["version"], 1);
    }

    #[test]
    fn validate_id_accepts_safe_ids() {
        assert!(validate_id("lex_01HX2A").is_ok());
        assert!(validate_id("motion-with-dash").is_ok());
        assert!(validate_id("a").is_ok());
    }

    #[test]
    fn validate_id_rejects_traversal_and_exotic_chars() {
        assert!(validate_id("..").is_err());
        assert!(validate_id("a/b").is_err());
        assert!(validate_id("a\\b").is_err());
        assert!(validate_id("a b").is_err());
        assert!(validate_id("a.json").is_err());
        assert!(validate_id("").is_err());
        // Way too long
        let long: String = "a".repeat(200);
        assert!(validate_id(&long).is_err());
    }

    #[test]
    fn atomic_write_creates_then_commits_file() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("nested").join("file.json");
        atomic_write(&dest, r#"{"hello": 1}"#).unwrap();
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), r#"{"hello": 1}"#);
    }

    #[test]
    fn list_assets_returns_empty_for_missing_dir() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let assets = list_assets(&missing).unwrap();
        assert!(assets.is_empty());
    }

    #[test]
    fn save_then_list_roundtrips() {
        let dir = tempdir().unwrap();
        let json = r#"{"id":"lex_AAA","kind":"expression","name":"Test","trigger":"smug","weights":{"happy":0.5},"learnedAt":1700000000000}"#;
        save_asset(dir.path(), json).unwrap();
        let assets = list_assets(dir.path()).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id, "lex_AAA");
        assert_eq!(assets[0].rest.get("trigger").and_then(|v| v.as_str()), Some("smug"));
    }

    #[test]
    fn save_then_delete_clears_artifact() {
        let dir = tempdir().unwrap();
        let json = r#"{"id":"lex_DEL","kind":"expression","name":"X","trigger":"x","weights":{},"learnedAt":1}"#;
        save_asset(dir.path(), json).unwrap();
        assert_eq!(list_assets(dir.path()).unwrap().len(), 1);
        delete_asset(dir.path(), "lex_DEL").unwrap();
        assert!(list_assets(dir.path()).unwrap().is_empty());
        // Idempotent — deleting again does not error.
        delete_asset(dir.path(), "lex_DEL").unwrap();
    }

    #[test]
    fn save_asset_rejects_invalid_json() {
        let dir = tempdir().unwrap();
        let err = save_asset(dir.path(), "not json").unwrap_err();
        assert!(err.contains("Invalid"));
    }

    #[test]
    fn save_asset_rejects_traversal_id() {
        let dir = tempdir().unwrap();
        let json = r#"{"id":"../escape","kind":"expression","name":"X","trigger":"x","weights":{},"learnedAt":1}"#;
        let err = save_asset(dir.path(), json).unwrap_err();
        assert!(err.contains("Invalid persona artifact id"));
    }

    #[test]
    fn list_assets_skips_corrupt_files_without_failing() {
        let dir = tempdir().unwrap();
        let good = r#"{"id":"lex_OK","kind":"expression","name":"X","trigger":"x","weights":{},"learnedAt":2}"#;
        save_asset(dir.path(), good).unwrap();
        std::fs::write(dir.path().join("broken.json"), "not json").unwrap();
        let assets = list_assets(dir.path()).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id, "lex_OK");
    }

    #[test]
    fn list_assets_orders_newest_first_by_learned_at() {
        let dir = tempdir().unwrap();
        save_asset(
            dir.path(),
            r#"{"id":"old","kind":"motion","name":"A","trigger":"a","fps":30,"duration_s":1,"frames":[],"learnedAt":1000}"#,
        )
        .unwrap();
        save_asset(
            dir.path(),
            r#"{"id":"new","kind":"motion","name":"B","trigger":"b","fps":30,"duration_s":1,"frames":[],"learnedAt":2000}"#,
        )
        .unwrap();
        let assets = list_assets(dir.path()).unwrap();
        assert_eq!(assets[0].id, "new");
        assert_eq!(assets[1].id, "old");
    }
}
