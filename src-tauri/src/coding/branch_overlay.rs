//! Branch-overlay schema & diff-based incremental sync (Chunk 45.1).
//!
//! Implements per-branch overlays for the code-intelligence graph. When
//! `HEAD ≠ base_ref`, only files whose content differs from base get
//! re-indexed and stored in the overlay table. Queries union base rows
//! with overlay rows, giving branch-accurate results at O(changed-files)
//! cost rather than O(repo-size).
//!
//! The overlay model avoids Graphify Issue #52: large repos with many
//! branches no longer require a full re-index on checkout.

use std::collections::HashSet;
use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::symbol_index::{IndexError, Symbol, SymbolKind};

// ─── Schema ─────────────────────────────────────────────────────────────────

/// Ensure the branch overlay tables exist. Called from `open_db` migrations.
pub fn ensure_overlay_schema(conn: &Connection) -> Result<(), IndexError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS code_branch_overlays (
            id          INTEGER PRIMARY KEY,
            repo_id     INTEGER NOT NULL REFERENCES code_repos(id) ON DELETE CASCADE,
            base_ref    TEXT    NOT NULL,
            branch_ref  TEXT    NOT NULL,
            file        TEXT    NOT NULL,
            hash        TEXT    NOT NULL,
            indexed_at  INTEGER NOT NULL,
            UNIQUE(repo_id, base_ref, branch_ref, file)
        );

        CREATE INDEX IF NOT EXISTS idx_branch_overlays_repo_branch
            ON code_branch_overlays(repo_id, base_ref, branch_ref);

        -- overlay_id links symbols/edges to a specific overlay.
        -- NULL means the row belongs to the base snapshot.
        "#,
    )?;

    // Migrate: add overlay_id column to code_symbols if missing.
    let has_overlay_id: bool = conn
        .prepare("SELECT overlay_id FROM code_symbols LIMIT 0")
        .is_ok();
    if !has_overlay_id {
        conn.execute_batch(
            "ALTER TABLE code_symbols ADD COLUMN overlay_id INTEGER REFERENCES code_branch_overlays(id) ON DELETE CASCADE;
             CREATE INDEX IF NOT EXISTS idx_code_symbols_overlay ON code_symbols(overlay_id);",
        )?;
    }

    // Migrate: add overlay_id column to code_edges if missing.
    let has_edge_overlay: bool = conn
        .prepare("SELECT overlay_id FROM code_edges LIMIT 0")
        .is_ok();
    if !has_edge_overlay {
        conn.execute_batch(
            "ALTER TABLE code_edges ADD COLUMN overlay_id INTEGER REFERENCES code_branch_overlays(id) ON DELETE CASCADE;
             CREATE INDEX IF NOT EXISTS idx_code_edges_overlay ON code_edges(overlay_id);",
        )?;
    }

    Ok(())
}

// ─── Types ──────────────────────────────────────────────────────────────────

/// Result of a branch sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSyncResult {
    pub base_ref: String,
    pub branch_ref: String,
    pub files_reindexed: u32,
    pub files_removed: u32,
    pub files_unchanged: u32,
    pub symbols_added: u32,
    pub edges_added: u32,
}

/// A single overlay file record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayFile {
    pub file: String,
    pub hash: String,
    pub indexed_at: i64,
}

// ─── Core API ───────────────────────────────────────────────────────────────

/// Perform a branch sync: compute `git diff --name-only base_ref..branch_ref`,
/// re-index only those changed files into the overlay, and remove overlay rows
/// for files that no longer differ.
///
/// `changed_files` is the list of relative paths from `git diff --name-only`.
/// `file_contents` provides `(rel_path, content_bytes)` for files that exist on
/// the branch (deleted files are absent from this map).
///
/// Returns a summary of the sync operation.
pub fn branch_sync(
    conn: &Connection,
    repo_id: i64,
    base_ref: &str,
    branch_ref: &str,
    changed_files: &[String],
    file_contents: &[(String, Vec<u8>)],
) -> Result<BranchSyncResult, IndexError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    // Set of files that are changed (exist on branch).
    let content_map: std::collections::HashMap<&str, &[u8]> = file_contents
        .iter()
        .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
        .collect();

    let changed_set: HashSet<&str> = changed_files.iter().map(|s| s.as_str()).collect();

    // Get existing overlay file records for this (repo, base, branch).
    let existing_overlay_files = get_overlay_files(conn, repo_id, base_ref, branch_ref)?;

    let tx = conn.unchecked_transaction()?;

    let mut result = BranchSyncResult {
        base_ref: base_ref.to_string(),
        branch_ref: branch_ref.to_string(),
        files_reindexed: 0,
        files_removed: 0,
        files_unchanged: 0,
        symbols_added: 0,
        edges_added: 0,
    };

    // Remove overlay rows for files no longer in the diff.
    for existing_file in &existing_overlay_files {
        if !changed_set.contains(existing_file.file.as_str()) {
            remove_overlay_file(&tx, repo_id, base_ref, branch_ref, &existing_file.file)?;
            result.files_removed += 1;
        }
    }

    // Re-index changed files.
    for file_path in changed_files {
        if let Some(content) = content_map.get(file_path.as_str()) {
            let hash = content_hash(content);

            // Check if existing overlay already has same hash (no re-index needed).
            let existing = existing_overlay_files
                .iter()
                .find(|f| f.file == *file_path);
            if let Some(existing) = existing {
                if existing.hash == hash {
                    result.files_unchanged += 1;
                    continue;
                }
            }

            // Remove old overlay data for this file before re-inserting.
            remove_overlay_file(&tx, repo_id, base_ref, branch_ref, file_path)?;

            // Upsert the overlay file record.
            tx.execute(
                "INSERT INTO code_branch_overlays (repo_id, base_ref, branch_ref, file, hash, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(repo_id, base_ref, branch_ref, file) DO UPDATE SET hash = excluded.hash, indexed_at = excluded.indexed_at",
                params![repo_id, base_ref, branch_ref, file_path, hash, now],
            )?;

            // Get the overlay_id for this file record.
            let overlay_id: i64 = tx.query_row(
                "SELECT id FROM code_branch_overlays WHERE repo_id = ?1 AND base_ref = ?2 AND branch_ref = ?3 AND file = ?4",
                params![repo_id, base_ref, branch_ref, file_path],
                |r| r.get(0),
            )?;

            // Parse the file and extract symbols + edges.
            let source = match String::from_utf8(content.to_vec()) {
                Ok(s) => s,
                Err(_) => {
                    result.files_reindexed += 1;
                    continue;
                }
            };

            let ext = Path::new(file_path)
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            let lang = super::parser_registry::detect_language(&ext);

            let (symbols, edges) = match lang {
                Some(lang_val) => parse_file_for_overlay(lang_val, &source, file_path)?,
                None => (Vec::new(), Vec::new()),
            };

            // Insert symbols with overlay_id.
            for sym in &symbols {
                tx.execute(
                    "INSERT OR IGNORE INTO code_symbols (repo_id, name, kind, file, line, end_line, parent, overlay_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        repo_id,
                        sym.name,
                        sym.kind.as_str(),
                        sym.file,
                        sym.line,
                        sym.end_line,
                        sym.parent,
                        overlay_id,
                    ],
                )?;
            }

            // Insert edges with overlay_id.
            for edge in &edges {
                tx.execute(
                    "INSERT INTO code_edges (repo_id, from_file, from_line, from_col, end_line, end_col, kind, target_name, overlay_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        repo_id,
                        edge.from_file,
                        edge.from_line,
                        edge.from_col,
                        edge.end_line,
                        edge.end_col,
                        edge.kind.as_str(),
                        edge.target_name,
                        overlay_id,
                    ],
                )?;
            }

            result.files_reindexed += 1;
            result.symbols_added += symbols.len() as u32;
            result.edges_added += edges.len() as u32;
        } else {
            // File was deleted on this branch — remove its overlay record.
            remove_overlay_file(&tx, repo_id, base_ref, branch_ref, file_path)?;
            result.files_removed += 1;
        }
    }

    tx.commit()?;
    Ok(result)
}

/// Delete an overlay and all associated data for a specific branch.
pub fn delete_branch_overlay(
    conn: &Connection,
    repo_id: i64,
    base_ref: &str,
    branch_ref: &str,
) -> Result<u32, IndexError> {
    // CASCADE on overlay_id handles symbols and edges automatically.
    let deleted = conn.execute(
        "DELETE FROM code_branch_overlays WHERE repo_id = ?1 AND base_ref = ?2 AND branch_ref = ?3",
        params![repo_id, base_ref, branch_ref],
    )?;
    Ok(deleted as u32)
}

/// List all active overlays for a repo.
pub fn list_overlays(
    conn: &Connection,
    repo_id: i64,
) -> Result<Vec<OverlaySummary>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT base_ref, branch_ref, COUNT(*) as file_count, MAX(indexed_at) as last_indexed
         FROM code_branch_overlays
         WHERE repo_id = ?1
         GROUP BY base_ref, branch_ref
         ORDER BY last_indexed DESC",
    )?;
    let rows = stmt.query_map(params![repo_id], |row| {
        Ok(OverlaySummary {
            base_ref: row.get(0)?,
            branch_ref: row.get(1)?,
            file_count: row.get(2)?,
            last_indexed: row.get(3)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Summary of a branch overlay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlaySummary {
    pub base_ref: String,
    pub branch_ref: String,
    pub file_count: u32,
    pub last_indexed: i64,
}

// ─── Overlay-aware queries ──────────────────────────────────────────────────

/// Query symbols by name, unioning base + overlay (overlay wins for same file).
///
/// For a file that exists in the overlay, base symbols for that file are
/// excluded. For files not in the overlay, base symbols are returned.
pub fn query_symbols_with_overlay(
    conn: &Connection,
    repo_id: i64,
    name: &str,
    base_ref: Option<&str>,
    branch_ref: Option<&str>,
) -> Result<Vec<Symbol>, IndexError> {
    match (base_ref, branch_ref) {
        (Some(base), Some(branch)) => {
            // Get the set of files that have overlay data.
            let overlay_files = get_overlay_file_set(conn, repo_id, base, branch)?;

            let mut stmt = conn.prepare(
                "SELECT name, kind, file, line, end_line, parent, overlay_id
                 FROM code_symbols
                 WHERE repo_id = ?1 AND name = ?2",
            )?;
            let rows = stmt.query_map(params![repo_id, name], |row| {
                Ok((
                    Symbol {
                        name: row.get(0)?,
                        kind: SymbolKind::parse(&row.get::<_, String>(1)?)
                            .unwrap_or(SymbolKind::Function),
                        file: row.get(2)?,
                        line: row.get(3)?,
                        end_line: row.get(4)?,
                        parent: row.get(5)?,
                    },
                    row.get::<_, Option<i64>>(6)?,
                ))
            })?;

            let all: Vec<(Symbol, Option<i64>)> =
                rows.filter_map(|r| r.ok()).collect();

            // Filter: include overlay rows for the active branch, and base
            // rows only for files NOT in the overlay.
            let overlay_ids = get_overlay_ids(conn, repo_id, base, branch)?;
            let mut results = Vec::new();
            for (sym, oid) in all {
                match oid {
                    Some(id) if overlay_ids.contains(&id) => {
                        // Overlay row for the active branch — include.
                        results.push(sym);
                    }
                    Some(_) => {
                        // Overlay row for a different branch — skip.
                    }
                    None => {
                        // Base row — include only if file not overlaid.
                        if !overlay_files.contains(sym.file.as_str()) {
                            results.push(sym);
                        }
                    }
                }
            }
            Ok(results)
        }
        _ => {
            // No overlay context — just return base symbols (overlay_id IS NULL).
            let mut stmt = conn.prepare(
                "SELECT name, kind, file, line, end_line, parent
                 FROM code_symbols
                 WHERE repo_id = ?1 AND name = ?2 AND overlay_id IS NULL",
            )?;
            let rows = stmt.query_map(params![repo_id, name], |row| {
                Ok(Symbol {
                    name: row.get(0)?,
                    kind: SymbolKind::parse(&row.get::<_, String>(1)?)
                        .unwrap_or(SymbolKind::Function),
                    file: row.get(2)?,
                    line: row.get(3)?,
                    end_line: row.get(4)?,
                    parent: row.get(5)?,
                })
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        }
    }
}

/// Query symbols in a file, overlay-aware. If the file has overlay data,
/// returns overlay symbols; otherwise returns base symbols.
pub fn query_symbols_in_file_with_overlay(
    conn: &Connection,
    repo_id: i64,
    file: &str,
    base_ref: Option<&str>,
    branch_ref: Option<&str>,
) -> Result<Vec<Symbol>, IndexError> {
    match (base_ref, branch_ref) {
        (Some(base), Some(branch)) => {
            let overlay_files = get_overlay_file_set(conn, repo_id, base, branch)?;

            if overlay_files.contains(file) {
                // File is overlaid — return only overlay symbols.
                let overlay_ids = get_overlay_ids(conn, repo_id, base, branch)?;
                let placeholders: String = overlay_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(",");

                let sql = format!(
                    "SELECT name, kind, file, line, end_line, parent
                     FROM code_symbols
                     WHERE repo_id = ?1 AND file = ?2 AND overlay_id IN ({placeholders})
                     ORDER BY line"
                );
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map(params![repo_id, file], |row| {
                    Ok(Symbol {
                        name: row.get(0)?,
                        kind: SymbolKind::parse(&row.get::<_, String>(1)?)
                            .unwrap_or(SymbolKind::Function),
                        file: row.get(2)?,
                        line: row.get(3)?,
                        end_line: row.get(4)?,
                        parent: row.get(5)?,
                    })
                })?;
                Ok(rows.filter_map(|r| r.ok()).collect())
            } else {
                // File not overlaid — return base symbols.
                let mut stmt = conn.prepare(
                    "SELECT name, kind, file, line, end_line, parent
                     FROM code_symbols
                     WHERE repo_id = ?1 AND file = ?2 AND overlay_id IS NULL
                     ORDER BY line",
                )?;
                let rows = stmt.query_map(params![repo_id, file], |row| {
                    Ok(Symbol {
                        name: row.get(0)?,
                        kind: SymbolKind::parse(&row.get::<_, String>(1)?)
                            .unwrap_or(SymbolKind::Function),
                        file: row.get(2)?,
                        line: row.get(3)?,
                        end_line: row.get(4)?,
                        parent: row.get(5)?,
                    })
                })?;
                Ok(rows.filter_map(|r| r.ok()).collect())
            }
        }
        _ => {
            let mut stmt = conn.prepare(
                "SELECT name, kind, file, line, end_line, parent
                 FROM code_symbols
                 WHERE repo_id = ?1 AND file = ?2 AND overlay_id IS NULL
                 ORDER BY line",
            )?;
            let rows = stmt.query_map(params![repo_id, file], |row| {
                Ok(Symbol {
                    name: row.get(0)?,
                    kind: SymbolKind::parse(&row.get::<_, String>(1)?)
                        .unwrap_or(SymbolKind::Function),
                    file: row.get(2)?,
                    line: row.get(3)?,
                    end_line: row.get(4)?,
                    parent: row.get(5)?,
                })
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        }
    }
}

// ─── Internal helpers ───────────────────────────────────────────────────────

/// SHA-256 content hash.
fn content_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Get overlay file records for a specific (repo, base, branch).
fn get_overlay_files(
    conn: &Connection,
    repo_id: i64,
    base_ref: &str,
    branch_ref: &str,
) -> Result<Vec<OverlayFile>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT file, hash, indexed_at FROM code_branch_overlays
         WHERE repo_id = ?1 AND base_ref = ?2 AND branch_ref = ?3",
    )?;
    let rows = stmt.query_map(params![repo_id, base_ref, branch_ref], |row| {
        Ok(OverlayFile {
            file: row.get(0)?,
            hash: row.get(1)?,
            indexed_at: row.get(2)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Get the set of file paths that have overlay records.
fn get_overlay_file_set(
    conn: &Connection,
    repo_id: i64,
    base_ref: &str,
    branch_ref: &str,
) -> Result<HashSet<String>, IndexError> {
    let files = get_overlay_files(conn, repo_id, base_ref, branch_ref)?;
    Ok(files.into_iter().map(|f| f.file).collect())
}

/// Get the overlay IDs for a specific (repo, base, branch).
fn get_overlay_ids(
    conn: &Connection,
    repo_id: i64,
    base_ref: &str,
    branch_ref: &str,
) -> Result<HashSet<i64>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT id FROM code_branch_overlays
         WHERE repo_id = ?1 AND base_ref = ?2 AND branch_ref = ?3",
    )?;
    let rows = stmt.query_map(params![repo_id, base_ref, branch_ref], |row| {
        row.get::<_, i64>(0)
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Remove all overlay data for a single file in a branch.
fn remove_overlay_file(
    conn: &Connection,
    repo_id: i64,
    base_ref: &str,
    branch_ref: &str,
    file: &str,
) -> Result<(), IndexError> {
    // Symbols and edges with this overlay_id are deleted via CASCADE.
    conn.execute(
        "DELETE FROM code_branch_overlays WHERE repo_id = ?1 AND base_ref = ?2 AND branch_ref = ?3 AND file = ?4",
        params![repo_id, base_ref, branch_ref, file],
    )?;
    Ok(())
}

/// Parse a file using the appropriate tree-sitter parser and extract symbols/edges.
fn parse_file_for_overlay(
    lang: super::parser_registry::Language,
    source: &str,
    file_path: &str,
) -> Result<(Vec<Symbol>, Vec<super::symbol_index::CodeEdge>), IndexError> {
    use super::parser_registry::Language;

    let mut parser = super::parser_registry::create_parser(lang);
    let tree = parser.parse(source, None).ok_or_else(|| IndexError::Parse {
        path: file_path.to_string(),
        detail: "tree-sitter parse returned None".to_string(),
    })?;

    let (symbols, edges) = match lang {
        Language::Rust => {
            super::symbol_index::extract_rust_symbols(source, tree.root_node(), file_path)
        }
        Language::TypeScript => {
            super::symbol_index::extract_ts_symbols(source, tree.root_node(), file_path)
        }
        #[allow(unreachable_patterns)]
        other => super::parser_registry::extract_symbols(other, source, tree.root_node(), file_path),
    };
    Ok((symbols, edges))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::symbol_index::open_db;
    use tempfile::TempDir;

    fn setup_test_db() -> (TempDir, Connection) {
        let dir = TempDir::new().unwrap();
        let conn = open_db(dir.path()).unwrap();
        ensure_overlay_schema(&conn).unwrap();

        // Create a test repo.
        conn.execute(
            "INSERT INTO code_repos (path, label, indexed_at) VALUES ('/test/repo', 'test-repo', 1000)",
            [],
        )
        .unwrap();

        (dir, conn)
    }

    fn insert_base_symbol(conn: &Connection, repo_id: i64, name: &str, file: &str, line: u32) {
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, parent, overlay_id)
             VALUES (?1, ?2, 'function', ?3, ?4, ?5, NULL, NULL)",
            params![repo_id, name, file, line, line + 10],
        )
        .unwrap();
    }

    #[test]
    fn overlay_schema_creates_tables() {
        let (_dir, conn) = setup_test_db();
        // Verify the table exists.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM code_branch_overlays",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);

        // Verify overlay_id column exists on code_symbols.
        conn.prepare("SELECT overlay_id FROM code_symbols LIMIT 0")
            .unwrap();

        // Verify overlay_id column exists on code_edges.
        conn.prepare("SELECT overlay_id FROM code_edges LIMIT 0")
            .unwrap();
    }

    #[test]
    fn branch_sync_indexes_changed_files() {
        let (_dir, conn) = setup_test_db();
        let repo_id = 1;

        // Insert some base symbols.
        insert_base_symbol(&conn, repo_id, "base_fn", "src/main.rs", 1);
        insert_base_symbol(&conn, repo_id, "helper", "src/lib.rs", 10);

        // Simulate a branch where src/main.rs was modified.
        let changed_files = vec!["src/main.rs".to_string()];
        let file_contents = vec![(
            "src/main.rs".to_string(),
            b"fn branch_fn() {}\nfn another() {}\n".to_vec(),
        )];

        let result =
            branch_sync(&conn, repo_id, "abc123", "def456", &changed_files, &file_contents)
                .unwrap();

        assert_eq!(result.base_ref, "abc123");
        assert_eq!(result.branch_ref, "def456");
        assert_eq!(result.files_reindexed, 1);
        assert_eq!(result.files_removed, 0);

        // Overlay should have one file record.
        let overlays = get_overlay_files(&conn, repo_id, "abc123", "def456").unwrap();
        assert_eq!(overlays.len(), 1);
        assert_eq!(overlays[0].file, "src/main.rs");
    }

    #[test]
    fn query_with_overlay_prefers_overlay_symbols() {
        let (_dir, conn) = setup_test_db();
        let repo_id = 1;

        // Base has "old_fn" in src/main.rs.
        insert_base_symbol(&conn, repo_id, "old_fn", "src/main.rs", 1);
        insert_base_symbol(&conn, repo_id, "lib_fn", "src/lib.rs", 5);

        // Branch modifies src/main.rs with "new_fn".
        let changed_files = vec!["src/main.rs".to_string()];
        let file_contents = vec![(
            "src/main.rs".to_string(),
            b"fn new_fn() {}\n".to_vec(),
        )];
        branch_sync(&conn, repo_id, "base1", "branch1", &changed_files, &file_contents).unwrap();

        // Query "old_fn" with overlay context — should NOT find it (file is overlaid).
        let results =
            query_symbols_with_overlay(&conn, repo_id, "old_fn", Some("base1"), Some("branch1"))
                .unwrap();
        assert!(results.is_empty(), "old_fn should be hidden by overlay");

        // Query "new_fn" with overlay context — should find it.
        let results =
            query_symbols_with_overlay(&conn, repo_id, "new_fn", Some("base1"), Some("branch1"))
                .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "new_fn");

        // Query "lib_fn" — not overlaid, should still be visible.
        let results =
            query_symbols_with_overlay(&conn, repo_id, "lib_fn", Some("base1"), Some("branch1"))
                .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "lib_fn");
    }

    #[test]
    fn query_without_overlay_returns_base_only() {
        let (_dir, conn) = setup_test_db();
        let repo_id = 1;

        insert_base_symbol(&conn, repo_id, "base_fn", "src/main.rs", 1);

        // Create overlay data.
        let changed_files = vec!["src/main.rs".to_string()];
        let file_contents = vec![(
            "src/main.rs".to_string(),
            b"fn overlay_fn() {}\n".to_vec(),
        )];
        branch_sync(&conn, repo_id, "b1", "br1", &changed_files, &file_contents).unwrap();

        // Query without overlay context — only base symbols visible.
        let results =
            query_symbols_with_overlay(&conn, repo_id, "base_fn", None, None).unwrap();
        assert_eq!(results.len(), 1);

        let results =
            query_symbols_with_overlay(&conn, repo_id, "overlay_fn", None, None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn delete_branch_overlay_removes_all_data() {
        let (_dir, conn) = setup_test_db();
        let repo_id = 1;

        let changed_files = vec!["src/a.rs".to_string(), "src/b.rs".to_string()];
        let file_contents = vec![
            ("src/a.rs".to_string(), b"fn a() {}\n".to_vec()),
            ("src/b.rs".to_string(), b"fn b() {}\n".to_vec()),
        ];
        branch_sync(&conn, repo_id, "base", "feat", &changed_files, &file_contents).unwrap();

        // Verify data exists.
        let overlays = list_overlays(&conn, repo_id).unwrap();
        assert_eq!(overlays.len(), 1);
        assert_eq!(overlays[0].file_count, 2);

        // Delete.
        let deleted = delete_branch_overlay(&conn, repo_id, "base", "feat").unwrap();
        assert_eq!(deleted, 2);

        // Verify gone.
        let overlays = list_overlays(&conn, repo_id).unwrap();
        assert!(overlays.is_empty());
    }

    #[test]
    fn branch_sync_handles_re_switch() {
        let (_dir, conn) = setup_test_db();
        let repo_id = 1;

        // First sync: branch A.
        let changed_files = vec!["src/main.rs".to_string()];
        let file_contents = vec![(
            "src/main.rs".to_string(),
            b"fn branch_a() {}\n".to_vec(),
        )];
        branch_sync(&conn, repo_id, "base", "branch-a", &changed_files, &file_contents).unwrap();

        // Second sync: branch B (different branch, same base).
        let changed_files_b = vec!["src/main.rs".to_string()];
        let file_contents_b = vec![(
            "src/main.rs".to_string(),
            b"fn branch_b() {}\n".to_vec(),
        )];
        branch_sync(
            &conn,
            repo_id,
            "base",
            "branch-b",
            &changed_files_b,
            &file_contents_b,
        )
        .unwrap();

        // Both overlays should coexist.
        let overlays = list_overlays(&conn, repo_id).unwrap();
        assert_eq!(overlays.len(), 2);

        // Query with branch-a context finds branch_a symbol.
        let results = query_symbols_with_overlay(
            &conn,
            repo_id,
            "branch_a",
            Some("base"),
            Some("branch-a"),
        )
        .unwrap();
        assert_eq!(results.len(), 1);

        // Query with branch-b context finds branch_b symbol, not branch_a.
        let results = query_symbols_with_overlay(
            &conn,
            repo_id,
            "branch_a",
            Some("base"),
            Some("branch-b"),
        )
        .unwrap();
        assert!(results.is_empty());

        let results = query_symbols_with_overlay(
            &conn,
            repo_id,
            "branch_b",
            Some("base"),
            Some("branch-b"),
        )
        .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn branch_sync_unchanged_file_skips_reindex() {
        let (_dir, conn) = setup_test_db();
        let repo_id = 1;

        let changed_files = vec!["src/main.rs".to_string()];
        let content = b"fn stable() {}\n".to_vec();
        let file_contents = vec![("src/main.rs".to_string(), content.clone())];

        // First sync.
        let r1 = branch_sync(&conn, repo_id, "b", "br", &changed_files, &file_contents).unwrap();
        assert_eq!(r1.files_reindexed, 1);

        // Second sync with same content — should skip.
        let r2 = branch_sync(&conn, repo_id, "b", "br", &changed_files, &file_contents).unwrap();
        assert_eq!(r2.files_reindexed, 0);
        assert_eq!(r2.files_unchanged, 1);
    }

    #[test]
    fn file_in_file_query_with_overlay() {
        let (_dir, conn) = setup_test_db();
        let repo_id = 1;

        // Base has symbols in src/main.rs.
        insert_base_symbol(&conn, repo_id, "base_fn1", "src/main.rs", 1);
        insert_base_symbol(&conn, repo_id, "base_fn2", "src/main.rs", 20);
        insert_base_symbol(&conn, repo_id, "other_fn", "src/other.rs", 1);

        // Overlay replaces src/main.rs.
        let changed_files = vec!["src/main.rs".to_string()];
        let file_contents = vec![(
            "src/main.rs".to_string(),
            b"fn overlay_fn() {}\n".to_vec(),
        )];
        branch_sync(&conn, repo_id, "base", "feat", &changed_files, &file_contents).unwrap();

        // Query symbols in src/main.rs with overlay — only overlay symbols.
        let results = query_symbols_in_file_with_overlay(
            &conn,
            repo_id,
            "src/main.rs",
            Some("base"),
            Some("feat"),
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "overlay_fn");

        // Query symbols in src/other.rs — not overlaid, returns base.
        let results = query_symbols_in_file_with_overlay(
            &conn,
            repo_id,
            "src/other.rs",
            Some("base"),
            Some("feat"),
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "other_fn");
    }
}
