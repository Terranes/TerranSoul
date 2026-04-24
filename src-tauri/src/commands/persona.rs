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
    let agent = crate::brain::OllamaAgent::new(&model);
    match agent.propose_persona(&snippets).await {
        Some(candidate) => serde_json::to_string(&candidate)
            .map_err(|e| format!("Failed to serialise persona candidate: {e}")),
        // Empty string = "brain replied but couldn't be parsed". UI
        // surfaces a soft "try again" message rather than a hard error.
        None => Ok(String::new()),
    }
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
