//! Bidirectional Obsidian sync — Chunk 17.7.
//!
//! Extends the one-way export (`obsidian_export.rs`) with:
//! - A markdown→memory parser (reverse of `render_markdown`).
//! - A file-watcher (`notify` crate) that detects vault changes.
//! - LWW conflict resolution: `file_mtime > last_exported` → import from disk.
//!
//! The sync engine runs as a background task in the Tauri async runtime.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use super::obsidian_export::{filename_for, render_markdown};
use super::store::{MemoryEntry, MemoryStore, MemoryTier, MemoryType, MemoryUpdate, NewMemory};

// ── Markdown → Memory parsing ──────────────────────────────────────────────────

/// Parsed result from an Obsidian markdown file.
#[derive(Debug, Clone)]
pub struct ParsedMemory {
    /// Memory ID from frontmatter (if present).
    pub id: Option<i64>,
    pub content: String,
    pub importance: i64,
    pub memory_type: MemoryType,
    pub tags: String,
    pub source_url: Option<String>,
    pub source_hash: Option<String>,
}

/// Parse YAML frontmatter + body from an Obsidian markdown file.
///
/// Returns `None` if the file doesn't have valid `---` delimited frontmatter.
pub fn parse_obsidian_markdown(text: &str) -> Option<ParsedMemory> {
    let text = text.trim_start_matches('\u{feff}'); // strip BOM
    if !text.starts_with("---\n") && !text.starts_with("---\r\n") {
        return None;
    }

    // Find end of frontmatter.
    let after_first = &text[4..]; // skip "---\n"
    let end_idx = after_first
        .find("\n---\n")
        .or_else(|| after_first.find("\n---\r\n"))?;
    let frontmatter = &after_first[..end_idx];
    let body_start = 4 + end_idx + 5; // "---\n" + frontmatter + "\n---\n"
    let body = if body_start < text.len() {
        text[body_start..].trim()
    } else {
        ""
    };

    // Parse frontmatter fields (simple line-by-line YAML).
    let mut id: Option<i64> = None;
    let mut importance: i64 = 3;
    let mut memory_type = MemoryType::Fact;
    let mut tags_list: Vec<String> = Vec::new();
    let mut source_url: Option<String> = None;
    let mut source_hash: Option<String> = None;
    let mut in_tags = false;

    for line in frontmatter.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- ") && in_tags {
            let tag = trimmed
                .trim_start_matches("- ")
                .trim_matches('"')
                .trim()
                .to_string();
            if !tag.is_empty() {
                tags_list.push(tag);
            }
            continue;
        }
        in_tags = false;

        if let Some(val) = trimmed.strip_prefix("id:") {
            id = val.trim().parse().ok();
        } else if let Some(val) = trimmed.strip_prefix("importance:") {
            importance = val.trim().parse().unwrap_or(3);
        } else if let Some(val) = trimmed.strip_prefix("memory_type:") {
            memory_type = MemoryType::from_str(val.trim().trim_matches('"'));
        } else if let Some(val) = trimmed.strip_prefix("source_url:") {
            let v = val.trim().trim_matches('"').to_string();
            if !v.is_empty() {
                source_url = Some(v);
            }
        } else if let Some(val) = trimmed.strip_prefix("source_hash:") {
            let v = val.trim().trim_matches('"').to_string();
            if !v.is_empty() {
                source_hash = Some(v);
            }
        } else if trimmed.starts_with("tags:") {
            in_tags = true;
        }
    }

    Some(ParsedMemory {
        id,
        content: body.to_string(),
        importance,
        memory_type,
        tags: tags_list.join(", "),
        source_url,
        source_hash,
    })
}

// ── Sync engine ────────────────────────────────────────────────────────────────

/// Result of a single sync cycle.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SyncReport {
    /// Files imported (vault → DB) because file was newer.
    pub imported: usize,
    /// Files exported (DB → vault) because DB was newer.
    pub exported: usize,
    /// Files skipped (no change).
    pub skipped: usize,
    /// Errors encountered (non-fatal).
    pub errors: Vec<String>,
}

/// Perform a full bidirectional sync cycle.
///
/// For each long-tier memory:
/// - If no vault file exists → export.
/// - If vault file exists and `file_mtime > last_exported` → import (LWW: file wins).
/// - If vault file exists and memory was modified after `last_exported` → export (DB wins).
/// - Otherwise → skip.
///
/// Also scans the vault directory for files not in the DB (new files created
/// externally) and imports them.
pub fn sync_bidirectional(vault_dir: &Path, store: &MemoryStore) -> Result<SyncReport, String> {
    let output_dir = vault_dir.join("TerranSoul");
    fs::create_dir_all(&output_dir).map_err(|e| format!("mkdir: {e}"))?;

    let entries = store.get_all().map_err(|e| format!("get_all: {e}"))?;

    let long_entries: Vec<&MemoryEntry> = entries
        .iter()
        .filter(|e| e.tier == MemoryTier::Long)
        .collect();

    let mut report = SyncReport::default();
    let mut known_files: HashMap<PathBuf, bool> = HashMap::new();
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    for entry in &long_entries {
        let fname = filename_for(entry);
        let fpath = output_dir.join(&fname);
        known_files.insert(fpath.clone(), true);

        let current_file_mtime_ms = file_mtime_ms(&fpath);
        let last_exported = entry.last_exported.unwrap_or(0);
        let memory_updated_ms = entry.last_accessed.unwrap_or(entry.created_at);

        if !fpath.exists() {
            // File doesn't exist → export.
            let content = render_markdown(entry);
            if let Err(e) = fs::write(&fpath, &content) {
                report
                    .errors
                    .push(format!("write {}: {e}", fpath.display()));
                continue;
            }
            let exported_at = file_mtime_ms(&fpath).max(now_ms);
            let _ = store.set_obsidian_sync(entry.id, &fname, exported_at);
            report.exported += 1;
        } else if current_file_mtime_ms > last_exported && current_file_mtime_ms > memory_updated_ms
        {
            // File is newer than both last export AND last DB modification → import.
            match import_file_to_entry(&fpath, entry.id, store) {
                Ok(()) => {
                    let _ = store.set_obsidian_sync(entry.id, &fname, now_ms);
                    report.imported += 1;
                }
                Err(e) => report.errors.push(e),
            }
        } else if memory_updated_ms > last_exported {
            // DB was modified after last export → re-export.
            let content = render_markdown(entry);
            if let Err(e) = fs::write(&fpath, &content) {
                report
                    .errors
                    .push(format!("write {}: {e}", fpath.display()));
                continue;
            }
            let exported_at = file_mtime_ms(&fpath).max(now_ms);
            let _ = store.set_obsidian_sync(entry.id, &fname, exported_at);
            report.exported += 1;
        } else {
            report.skipped += 1;
        }
    }

    // Scan for new files in the vault that aren't tracked by any memory.
    if let Ok(dir_entries) = fs::read_dir(&output_dir) {
        for dir_entry in dir_entries.flatten() {
            let path = dir_entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if known_files.contains_key(&path) {
                continue;
            }
            // New file — import as a new memory.
            match import_new_file(&path, store) {
                Ok(id) => {
                    let fname = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let _ = store.set_obsidian_sync(id, &fname, now_ms);
                    report.imported += 1;
                }
                Err(e) => report.errors.push(e),
            }
        }
    }

    Ok(report)
}

/// Import a vault file's content into an existing memory (update).
fn import_file_to_entry(path: &Path, id: i64, store: &MemoryStore) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let parsed = parse_obsidian_markdown(&text)
        .ok_or_else(|| format!("invalid frontmatter in {}", path.display()))?;

    let update = MemoryUpdate {
        content: Some(parsed.content),
        tags: Some(parsed.tags),
        importance: Some(parsed.importance),
        memory_type: Some(parsed.memory_type),
    };
    store
        .update(id, update)
        .map_err(|e| format!("update id={id}: {e}"))?;
    Ok(())
}

/// Import a new vault file as a brand new memory.
fn import_new_file(path: &Path, store: &MemoryStore) -> Result<i64, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let parsed = parse_obsidian_markdown(&text)
        .ok_or_else(|| format!("invalid frontmatter in {}", path.display()))?;

    // If the parsed file has an ID, check if it already exists.
    if let Some(existing_id) = parsed.id {
        if store.get_by_id(existing_id).is_ok() {
            // Already exists — treat as update.
            import_file_to_entry(path, existing_id, store)?;
            return Ok(existing_id);
        }
    }

    let new_mem = NewMemory {
        content: parsed.content,
        tags: parsed.tags,
        importance: parsed.importance,
        memory_type: parsed.memory_type,
        source_url: parsed.source_url,
        source_hash: parsed.source_hash,
        expires_at: None,
    };
    let entry = store.add(new_mem).map_err(|e| format!("add: {e}"))?;
    Ok(entry.id)
}

/// Get a file's mtime as Unix milliseconds.
fn file_mtime_ms(path: &Path) -> i64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ── File Watcher ───────────────────────────────────────────────────────────────

/// Handle to a running Obsidian sync watcher. Dropping it stops the watcher.
pub struct ObsidianWatcher {
    _watcher: RecommendedWatcher,
    shutdown_tx: mpsc::Sender<()>,
}

impl ObsidianWatcher {
    /// Start watching the vault directory for changes and auto-syncing.
    ///
    /// `store_mutex` must live for `'static` (typically `Arc<..>`-owned).
    /// The sync loop debounces events (1s) and runs `sync_bidirectional`.
    pub fn start(vault_dir: PathBuf, store: Arc<Mutex<MemoryStore>>) -> Result<Self, String> {
        Self::start_inner(vault_dir, store)
    }

    /// Start watching using an [`AppState`](crate::AppState) reference.
    ///
    /// This is the preferred entry point from Tauri commands since AppState
    /// is `Arc<AppStateInner>` and the `memory_store` field is accessible
    /// via `Deref`.
    pub fn start_with_state(vault_dir: PathBuf, state: crate::AppState) -> Result<Self, String> {
        // We can't extract Arc<Mutex<MemoryStore>> from AppState's Mutex field
        // directly, so we wrap the AppState and access .memory_store in the loop.
        let (event_tx, mut event_rx) = mpsc::channel::<()>(16);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let event_tx_clone = event_tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        let _ = event_tx_clone.try_send(());
                    }
                    _ => {}
                }
            }
        })
        .map_err(|e| format!("watcher init: {e}"))?;

        let watch_dir = vault_dir.join("TerranSoul");
        fs::create_dir_all(&watch_dir).map_err(|e| format!("mkdir: {e}"))?;
        watcher
            .watch(&watch_dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("watch: {e}"))?;

        let vault_dir_clone = vault_dir.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    Some(()) = event_rx.recv() => {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        while event_rx.try_recv().is_ok() {}
                        if let Ok(s) = state.memory_store.lock() {
                            let _ = sync_bidirectional(&vault_dir_clone, &s);
                        }
                    }
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            shutdown_tx,
        })
    }

    fn start_inner(vault_dir: PathBuf, store: Arc<Mutex<MemoryStore>>) -> Result<Self, String> {
        let (event_tx, mut event_rx) = mpsc::channel::<()>(16);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let event_tx_clone = event_tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        let _ = event_tx_clone.try_send(());
                    }
                    _ => {}
                }
            }
        })
        .map_err(|e| format!("watcher init: {e}"))?;

        let watch_dir = vault_dir.join("TerranSoul");
        fs::create_dir_all(&watch_dir).map_err(|e| format!("mkdir: {e}"))?;
        watcher
            .watch(&watch_dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("watch: {e}"))?;

        let vault_dir_clone = vault_dir.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    Some(()) = event_rx.recv() => {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        while event_rx.try_recv().is_ok() {}
                        if let Ok(s) = store.lock() {
                            let _ = sync_bidirectional(&vault_dir_clone, &s);
                        }
                    }
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            shutdown_tx,
        })
    }

    /// Stop the watcher and sync loop.
    pub fn stop(&self) {
        let _ = self.shutdown_tx.try_send(());
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_frontmatter() {
        let md = r#"---
id: 42
importance: 5
memory_type: "fact"
tags:
  - "rust"
  - "programming"
source_url: "https://example.com"
---

This is the memory content.
Multiple lines here.
"#;
        let parsed = parse_obsidian_markdown(md).unwrap();
        assert_eq!(parsed.id, Some(42));
        assert_eq!(parsed.importance, 5);
        assert_eq!(parsed.memory_type, MemoryType::Fact);
        assert_eq!(parsed.tags, "rust, programming");
        assert_eq!(parsed.source_url, Some("https://example.com".to_string()));
        assert_eq!(
            parsed.content,
            "This is the memory content.\nMultiple lines here."
        );
    }

    #[test]
    fn parse_minimal_frontmatter() {
        let md = "---\nid: 1\n---\n\nHello world\n";
        let parsed = parse_obsidian_markdown(md).unwrap();
        assert_eq!(parsed.id, Some(1));
        assert_eq!(parsed.content, "Hello world");
        assert_eq!(parsed.importance, 3);
    }

    #[test]
    fn parse_no_frontmatter_returns_none() {
        assert!(parse_obsidian_markdown("No frontmatter here").is_none());
    }

    #[test]
    fn parse_empty_body() {
        let md = "---\nid: 7\n---\n";
        let parsed = parse_obsidian_markdown(md).unwrap();
        assert_eq!(parsed.id, Some(7));
        assert_eq!(parsed.content, "");
    }

    #[test]
    fn sync_creates_and_imports_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path());

        // Add a memory.
        let entry = store
            .add(NewMemory {
                content: "Test memory content".to_string(),
                tags: "test, sync".to_string(),
                importance: 4,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        // First sync: should export.
        let vault = dir.path().join("vault");
        let report = sync_bidirectional(&vault, &store).unwrap();
        assert_eq!(report.exported, 1);
        assert_eq!(report.imported, 0);

        // Verify file exists.
        let expected_file = vault.join("TerranSoul").join(filename_for(&entry));
        assert!(expected_file.exists());

        // Second sync: should skip (nothing changed).
        let report2 = sync_bidirectional(&vault, &store).unwrap();
        assert_eq!(report2.exported, 0);
        assert_eq!(report2.skipped, 1);
    }

    #[test]
    fn sync_imports_externally_modified_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path());

        let entry = store
            .add(NewMemory {
                content: "Original content".to_string(),
                tags: "test".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        // Export first.
        let vault = dir.path().join("vault");
        let _ = sync_bidirectional(&vault, &store).unwrap();

        // Simulate external edit: overwrite file content with newer mtime.
        let fpath = vault.join("TerranSoul").join(filename_for(&entry));
        std::thread::sleep(Duration::from_millis(50));
        let new_content = format!(
            "---\nid: {}\nimportance: 5\nmemory_type: \"fact\"\ntags:\n  - \"updated\"\n---\n\nEdited in Obsidian\n",
            entry.id
        );
        fs::write(&fpath, &new_content).unwrap();

        // Sync again: file is newer → import.
        let report = sync_bidirectional(&vault, &store).unwrap();
        assert_eq!(report.imported, 1);

        // Verify the DB was updated.
        let updated = store.get_by_id(entry.id).unwrap();
        assert_eq!(updated.content, "Edited in Obsidian");
        assert_eq!(updated.importance, 5);
        assert_eq!(updated.tags, "updated");
    }

    #[test]
    fn sync_imports_new_external_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemoryStore::new(dir.path());

        let vault = dir.path().join("vault");
        let ts_dir = vault.join("TerranSoul");
        fs::create_dir_all(&ts_dir).unwrap();

        // Create a new file in the vault (not from any memory).
        let new_file = ts_dir.join("external-note.md");
        let content = "---\nimportance: 4\nmemory_type: \"preference\"\ntags:\n  - \"new\"\n---\n\nI was created externally\n";
        fs::write(&new_file, content).unwrap();

        // Sync: should import the new file.
        let report = sync_bidirectional(&vault, &store).unwrap();
        assert_eq!(report.imported, 1);

        // Verify a memory was created.
        let all = store.get_all().unwrap();
        let found = all.iter().find(|e| e.content == "I was created externally");
        assert!(found.is_some());
        let mem = found.unwrap();
        assert_eq!(mem.importance, 4);
        assert_eq!(mem.tags, "new");
    }
}
