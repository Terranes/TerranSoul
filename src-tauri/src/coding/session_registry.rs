//! Persistent session registry backed by `mcp-data/sessions.json`
//! (Chunk 43.1).
//!
//! Stores a mapping of memorable session names to their underlying
//! session ids (the sanitised filenames used by `handoff_store` and
//! `session_chat`). The registry is the single source of truth for
//! name ↔ id resolution; the existing `coding_workflow/sessions/`
//! directory continues to hold the actual session data.
//!
//! ## File format
//!
//! ```json
//! {
//!   "sessions": {
//!     "blazing-fox": { "session_id": "blazing-fox", "created_at": 1715100000000 },
//!     "gentle-owl": { "session_id": "gentle-owl", "created_at": 1715200000000 }
//!   }
//! }
//! ```

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::session_names;

const SESSIONS_FILE: &str = "sessions.json";

/// One entry in the registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionEntry {
    /// The underlying session id used by `handoff_store` / `session_chat`.
    pub session_id: String,
    /// Unix-ms when the entry was created.
    pub created_at: i64,
}

/// Top-level structure persisted as `sessions.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionRegistry {
    /// Memorable name → entry. Keys are always normalised (lowercase).
    pub sessions: HashMap<String, SessionEntry>,
}

fn registry_path(data_dir: &Path) -> PathBuf {
    data_dir.join(SESSIONS_FILE)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Load the registry from disk. Returns an empty registry if the file
/// does not exist yet. Returns `Err` on I/O or parse failures.
pub fn load(data_dir: &Path) -> Result<SessionRegistry, String> {
    let path = registry_path(data_dir);
    if !path.exists() {
        return Ok(SessionRegistry::default());
    }
    let body = fs::read(&path).map_err(|e| format!("read sessions.json: {e}"))?;
    serde_json::from_slice(&body).map_err(|e| format!("parse sessions.json: {e}"))
}

/// Atomically write the registry to disk via temp-file + rename.
pub fn save(data_dir: &Path, reg: &SessionRegistry) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("create data dir: {e}"))?;
    let path = registry_path(data_dir);
    let tmp = path.with_extension("json.tmp");
    let body =
        serde_json::to_vec_pretty(reg).map_err(|e| format!("serialise sessions.json: {e}"))?;
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("create tmp: {e}"))?;
        f.write_all(&body).map_err(|e| format!("write tmp: {e}"))?;
        f.sync_all().map_err(|e| format!("sync tmp: {e}"))?;
    }
    fs::rename(&tmp, &path).map_err(|e| format!("rename tmp -> final: {e}"))?;
    Ok(())
}

/// Look up a session by its memorable name (case-insensitive).
/// Returns `None` if no such session is registered.
pub fn resolve(data_dir: &Path, name: &str) -> Result<Option<SessionEntry>, String> {
    let reg = load(data_dir)?;
    let key = session_names::normalize(name);
    Ok(reg.sessions.get(&key).cloned())
}

/// Create a new session with an auto-generated memorable name.
/// Returns the name and the session id (which is the same as the name
/// since memorable names are already filesystem-safe).
pub fn create_session(data_dir: &Path) -> Result<(String, SessionEntry), String> {
    let mut reg = load(data_dir)?;
    let existing: HashSet<String> = reg.sessions.keys().cloned().collect();
    let name = session_names::generate_unique(&existing);
    let entry = SessionEntry {
        session_id: name.clone(),
        created_at: now_ms(),
    };
    reg.sessions.insert(name.clone(), entry.clone());
    save(data_dir, &reg)?;
    Ok((name, entry))
}

/// Register a specific memorable name for a session.
/// Returns `Err` if the name is already taken.
pub fn register(
    data_dir: &Path,
    name: &str,
    session_id: &str,
) -> Result<SessionEntry, String> {
    let mut reg = load(data_dir)?;
    let key = session_names::normalize(name);
    if reg.sessions.contains_key(&key) {
        return Err(format!("session name '{key}' is already taken"));
    }
    let entry = SessionEntry {
        session_id: session_id.to_string(),
        created_at: now_ms(),
    };
    reg.sessions.insert(key, entry.clone());
    save(data_dir, &reg)?;
    Ok(entry)
}

/// Remove a session from the registry by name. Does NOT delete the
/// underlying session files. Returns `true` if the entry existed.
pub fn unregister(data_dir: &Path, name: &str) -> Result<bool, String> {
    let mut reg = load(data_dir)?;
    let key = session_names::normalize(name);
    let existed = reg.sessions.remove(&key).is_some();
    if existed {
        save(data_dir, &reg)?;
    }
    Ok(existed)
}

/// List all registered session names, sorted alphabetically.
pub fn list_names(data_dir: &Path) -> Result<Vec<String>, String> {
    let reg = load(data_dir)?;
    let mut names: Vec<String> = reg.sessions.keys().cloned().collect();
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "ts-session-reg-{}-{}-{}",
            tag,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn create_and_resolve_round_trip() {
        let dir = tmp_dir("create-resolve");
        let (name, entry) = create_session(&dir).unwrap();
        assert!(session_names::is_valid_memorable_name(&name));
        let resolved = resolve(&dir, &name).unwrap().unwrap();
        assert_eq!(resolved.session_id, entry.session_id);
    }

    #[test]
    fn case_insensitive_lookup() {
        let dir = tmp_dir("case-insensitive");
        register(&dir, "blazing-fox", "blazing-fox").unwrap();
        assert!(resolve(&dir, "Blazing-Fox").unwrap().is_some());
        assert!(resolve(&dir, "BLAZING-FOX").unwrap().is_some());
        assert!(resolve(&dir, "blazing-fox").unwrap().is_some());
    }

    #[test]
    fn collision_avoidance() {
        let dir = tmp_dir("collision");
        register(&dir, "blazing-fox", "blazing-fox").unwrap();
        let err = register(&dir, "blazing-fox", "other-id");
        assert!(err.is_err());
    }

    #[test]
    fn unregister_removes_entry() {
        let dir = tmp_dir("unregister");
        register(&dir, "calm-owl", "calm-owl").unwrap();
        assert!(unregister(&dir, "calm-owl").unwrap());
        assert!(resolve(&dir, "calm-owl").unwrap().is_none());
    }

    #[test]
    fn list_names_sorted() {
        let dir = tmp_dir("list");
        register(&dir, "calm-owl", "calm-owl").unwrap();
        register(&dir, "agile-fox", "agile-fox").unwrap();
        register(&dir, "bold-wolf", "bold-wolf").unwrap();
        let names = list_names(&dir).unwrap();
        assert_eq!(names, vec!["agile-fox", "bold-wolf", "calm-owl"]);
    }

    #[test]
    fn empty_registry_loads_ok() {
        let dir = tmp_dir("empty");
        let reg = load(&dir).unwrap();
        assert!(reg.sessions.is_empty());
    }
}
