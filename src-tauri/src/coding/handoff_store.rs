//! Disk-backed persistence for coding-workflow [`HandoffState`] records
//! (Chunk 28.9 — wiring half of the long-session context-handoff feature).
//!
//! ## Storage layout
//!
//! ```text
//! <data_dir>/
//!   coding_workflow/
//!     sessions/
//!       <session_id>.json   # one [`HandoffState`] per file
//! ```
//!
//! Each file is an atomic JSON dump of a single [`HandoffState`]. Writes
//! go through a `*.tmp` sibling + rename so a crash mid-write cannot
//! leave a torn record on disk. `session_id` values are sanitised so a
//! malicious or sloppy caller cannot escape the sessions directory via
//! `..` traversal or path separators.
//!
//! ## Why a separate module
//!
//! [`super::handoff`] is a pure codec with no I/O — that lets it be
//! tested cheaply and reused by future agents (e.g. a long-running
//! reviewer agent) without dragging in `tokio::fs` or the data-dir
//! plumbing. This module is the I/O half: it owns the directory layout,
//! atomic-write contract, and listing. The two halves were separated
//! intentionally per `rules/architecture-rules.md`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::handoff::HandoffState;

/// Directory name (relative to the app data dir) where handoff
/// snapshots live.
pub const SESSIONS_SUBDIR: &str = "coding_workflow/sessions";

/// Lightweight summary returned by [`list_handoffs`] for the UI / CLI.
///
/// Keeping this lean (no `pending_steps`, no `open_artefacts`) lets the
/// frontend render a sessions list without paying the cost of loading
/// every full snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandoffSummary {
    /// Same value as the underlying [`HandoffState::session_id`].
    pub session_id: String,
    /// Same value as the underlying [`HandoffState::chunk_id`].
    pub chunk_id: String,
    /// Same value as the underlying [`HandoffState::last_action`].
    pub last_action: String,
    /// Same value as the underlying [`HandoffState::created_at`].
    pub created_at: i64,
    /// File modification time on disk, unix-ms.
    pub modified_at: i64,
    /// Byte size of the snapshot file on disk.
    pub bytes: u64,
}

/// Compute the absolute path to the sessions directory.
pub fn sessions_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(SESSIONS_SUBDIR)
}

/// Sanitise a session id so it is safe to use as a filename component.
///
/// Rules:
/// * Allowed characters: ASCII alphanumerics, `_`, `-`, `.`.
/// * Any other byte is replaced with `_`.
/// * Empty input becomes `"default"`.
/// * Leading dots are stripped to prevent dotfile abuse on Unix.
/// * Result is truncated to 64 bytes.
///
/// This mirrors the safe-id discipline used throughout the rest of the
/// data layer (see `memory::store::sanitize_session_id`).
pub fn sanitize_session_id(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len().min(64));
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
        if out.len() >= 64 {
            break;
        }
    }
    while out.starts_with('.') {
        out.remove(0);
    }
    if out.is_empty() {
        return "default".to_string();
    }
    out
}

fn snapshot_path(data_dir: &Path, session_id: &str) -> PathBuf {
    sessions_dir(data_dir).join(format!("{}.json", sanitize_session_id(session_id)))
}

/// Atomically write `state` to its on-disk slot. Creates the parent
/// directory tree on demand. Existing snapshots for the same
/// `session_id` are overwritten.
pub fn save_handoff(data_dir: &Path, state: &HandoffState) -> Result<(), String> {
    let dir = sessions_dir(data_dir);
    fs::create_dir_all(&dir).map_err(|e| format!("create sessions dir: {e}"))?;

    let path = snapshot_path(data_dir, &state.session_id);
    let tmp = path.with_extension("json.tmp");

    let body = serde_json::to_vec_pretty(state).map_err(|e| format!("serialise handoff: {e}"))?;

    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("create tmp: {e}"))?;
        f.write_all(&body).map_err(|e| format!("write tmp: {e}"))?;
        f.sync_all().map_err(|e| format!("sync tmp: {e}"))?;
    }

    fs::rename(&tmp, &path).map_err(|e| format!("rename tmp -> final: {e}"))?;
    Ok(())
}

/// Load the snapshot for `session_id`, or `Ok(None)` if no snapshot
/// exists. Returns `Err` only on I/O / parse failures (so the UI can
/// distinguish "never saved" from "corrupted on disk").
pub fn load_handoff(data_dir: &Path, session_id: &str) -> Result<Option<HandoffState>, String> {
    let path = snapshot_path(data_dir, session_id);
    if !path.exists() {
        return Ok(None);
    }
    let body = fs::read(&path).map_err(|e| format!("read snapshot: {e}"))?;
    let state: HandoffState =
        serde_json::from_slice(&body).map_err(|e| format!("parse snapshot: {e}"))?;
    Ok(Some(state))
}

/// Delete the snapshot for `session_id`. Returns `Ok(false)` if no
/// snapshot existed (so the UI can be idempotent without checking
/// first).
pub fn clear_handoff(data_dir: &Path, session_id: &str) -> Result<bool, String> {
    let path = snapshot_path(data_dir, session_id);
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path).map_err(|e| format!("delete snapshot: {e}"))?;
    Ok(true)
}

/// List every saved snapshot, newest first by modification time.
///
/// Corrupt JSON files are silently skipped so a single bad record can't
/// brick the sessions panel — the broken file stays on disk for a human
/// to inspect.
pub fn list_handoffs(data_dir: &Path) -> Result<Vec<HandoffSummary>, String> {
    let dir = sessions_dir(data_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("read sessions dir: {e}"))? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let body = match fs::read(&path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let state: HandoffState = match serde_json::from_slice(&body) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        out.push(HandoffSummary {
            session_id: state.session_id,
            chunk_id: state.chunk_id,
            last_action: state.last_action,
            created_at: state.created_at,
            modified_at,
            bytes: metadata.len(),
        });
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.modified_at));
    Ok(out)
}

/// Convenience helper: produce the current wall-clock time as unix-ms
/// for stamping fresh [`HandoffState::created_at`] values when the LLM
/// emits a seed without one.
pub fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(session_id: &str) -> HandoffState {
        HandoffState {
            session_id: session_id.to_string(),
            chunk_id: "28.9".into(),
            last_action: "wired persistence".into(),
            pending_steps: vec!["add tauri commands".into()],
            open_artefacts: vec!["src-tauri/src/coding/handoff_store.rs".into()],
            summary: "Disk-backed handoff store.".into(),
            created_at: 1_730_500_000_000,
        }
    }

    fn tmp_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "ts-handoff-{}-{}-{}",
            tag,
            std::process::id(),
            now_unix_ms()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tmp_dir("roundtrip");
        let state = sample("alpha");
        save_handoff(&dir, &state).unwrap();
        let got = load_handoff(&dir, "alpha").unwrap().unwrap();
        assert_eq!(got, state);
    }

    #[test]
    fn load_missing_returns_none() {
        let dir = tmp_dir("missing");
        assert!(load_handoff(&dir, "ghost").unwrap().is_none());
    }

    #[test]
    fn save_overwrites_existing() {
        let dir = tmp_dir("overwrite");
        let mut state = sample("beta");
        save_handoff(&dir, &state).unwrap();
        state.last_action = "updated".into();
        save_handoff(&dir, &state).unwrap();
        let got = load_handoff(&dir, "beta").unwrap().unwrap();
        assert_eq!(got.last_action, "updated");
    }

    #[test]
    fn clear_removes_file_and_is_idempotent() {
        let dir = tmp_dir("clear");
        let state = sample("gamma");
        save_handoff(&dir, &state).unwrap();
        assert!(clear_handoff(&dir, "gamma").unwrap());
        assert!(!clear_handoff(&dir, "gamma").unwrap());
        assert!(load_handoff(&dir, "gamma").unwrap().is_none());
    }

    #[test]
    fn list_returns_summaries_newest_first() {
        let dir = tmp_dir("list");
        let mut a = sample("aaa");
        a.created_at = 1;
        save_handoff(&dir, &a).unwrap();
        // Force the second file to have a strictly later mtime.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut b = sample("bbb");
        b.created_at = 2;
        b.last_action = "newer".into();
        save_handoff(&dir, &b).unwrap();

        let summaries = list_handoffs(&dir).unwrap();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].session_id, "bbb");
        assert_eq!(summaries[0].last_action, "newer");
        assert_eq!(summaries[1].session_id, "aaa");
    }

    #[test]
    fn list_skips_corrupt_files() {
        let dir = tmp_dir("corrupt");
        save_handoff(&dir, &sample("good")).unwrap();
        // Plant a corrupt file alongside the good one.
        let bad = sessions_dir(&dir).join("bad.json");
        fs::write(&bad, b"{not valid json").unwrap();
        let summaries = list_handoffs(&dir).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].session_id, "good");
    }

    #[test]
    fn list_on_missing_dir_returns_empty() {
        let dir = tmp_dir("never");
        // Don't save anything — the sessions dir won't exist.
        let summaries = list_handoffs(&dir).unwrap();
        assert!(summaries.is_empty());
    }

    #[test]
    fn sanitise_strips_path_separators_and_traversal() {
        assert_eq!(sanitize_session_id("../etc/passwd"), "_etc_passwd");
        assert_eq!(sanitize_session_id("a/b\\c"), "a_b_c");
        assert_eq!(sanitize_session_id(""), "default");
        assert_eq!(sanitize_session_id("....hidden"), "hidden");
        assert_eq!(sanitize_session_id("ok-id_123.v2"), "ok-id_123.v2");
    }

    #[test]
    fn sanitise_caps_length_at_64() {
        let long = "x".repeat(200);
        let s = sanitize_session_id(&long);
        assert_eq!(s.len(), 64);
    }

    #[test]
    fn save_load_uses_sanitised_filename() {
        let dir = tmp_dir("sanitised");
        let state = HandoffState {
            session_id: "weird/id..with*chars".into(),
            ..sample("ignored")
        };
        save_handoff(&dir, &state).unwrap();
        // Reload via the same raw id — the sanitiser must be stable so
        // both write and read map to the same on-disk slot.
        let got = load_handoff(&dir, "weird/id..with*chars").unwrap().unwrap();
        assert_eq!(got.session_id, "weird/id..with*chars");
    }

    #[test]
    fn now_unix_ms_is_monotonic_within_test() {
        let a = now_unix_ms();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = now_unix_ms();
        assert!(b >= a);
    }
}
