//! Deterministic code-graph snapshot export/import (Chunk 45.3).
//!
//! Produces a `.codegraph/snapshot.json` file that is:
//! - **Deterministic**: sorted lexicographically, no timestamps, so two devs
//!   at the same commit produce byte-identical output.
//! - **Content-addressed**: each symbol/edge row is keyed by file + content_hash + line.
//! - **Embeddingless**: no vectors stored (recomputed lazily from local brain).
//!
//! This allows a freshly cloned repo to import the graph in one call without
//! needing a full tree-sitter re-parse, and avoids needing a git merge driver
//! since re-running at any commit is idempotent.

use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::symbol_index::IndexError;

// ─── Schema version ─────────────────────────────────────────────────────────

/// Current snapshot schema version. Increment on breaking format changes.
pub const SCHEMA_VERSION: u32 = 1;

// ─── Types ──────────────────────────────────────────────────────────────────

/// The full snapshot structure written to `.codegraph/snapshot.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeGraphSnapshot {
    /// Schema version for forward-compat detection.
    pub schema_version: u32,
    /// The base ref (commit SHA) this snapshot represents.
    pub base_ref: String,
    /// Repo label (directory name).
    pub repo_label: String,
    /// Files in the snapshot, sorted by path.
    pub files: Vec<SnapshotFile>,
    /// Symbols sorted by (file, line, name).
    pub symbols: Vec<SnapshotSymbol>,
    /// Edges sorted by (from_file, from_line, kind, target_name).
    pub edges: Vec<SnapshotEdge>,
}

/// Metadata sidecar written to `.codegraph/snapshot.meta.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotMeta {
    pub schema_version: u32,
    pub base_ref: String,
    pub repo_label: String,
    pub file_count: u32,
    pub symbol_count: u32,
    pub edge_count: u32,
}

/// A file record in the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SnapshotFile {
    pub path: String,
    pub hash: String,
}

/// A symbol record in the snapshot (no id, no timestamps).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SnapshotSymbol {
    pub file: String,
    pub line: u32,
    pub end_line: u32,
    pub name: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// An edge record in the snapshot (no id, no timestamps).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SnapshotEdge {
    pub from_file: String,
    pub from_line: u32,
    pub kind: String,
    pub target_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_col: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_col: Option<u32>,
}

// ─── Export ─────────────────────────────────────────────────────────────────

/// Export the base snapshot for a repo (overlay_id IS NULL only).
///
/// Returns the snapshot struct. The caller writes it to disk.
pub fn export_snapshot(
    conn: &Connection,
    repo_id: i64,
    base_ref: &str,
) -> Result<CodeGraphSnapshot, IndexError> {
    // Get repo label.
    let repo_label: String = conn
        .query_row(
            "SELECT label FROM code_repos WHERE id = ?1",
            params![repo_id],
            |r| r.get(0),
        )
        .map_err(|_| IndexError::InvalidPath("repo not found".to_string()))?;

    // Collect files (base only).
    let mut files: Vec<SnapshotFile> = {
        let mut stmt = conn
            .prepare("SELECT file, hash FROM code_file_hashes WHERE repo_id = ?1 ORDER BY file")?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok(SnapshotFile {
                path: row.get(0)?,
                hash: row.get(1)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };
    files.sort();

    // Collect symbols (base only: overlay_id IS NULL).
    let mut symbols: Vec<SnapshotSymbol> = {
        let mut stmt = conn.prepare(
            "SELECT file, line, end_line, name, kind, parent
             FROM code_symbols
             WHERE repo_id = ?1 AND overlay_id IS NULL
             ORDER BY file, line, name",
        )?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok(SnapshotSymbol {
                file: row.get(0)?,
                line: row.get(1)?,
                end_line: row.get(2)?,
                name: row.get(3)?,
                kind: row.get(4)?,
                parent: row.get(5)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };
    symbols.sort();

    // Collect edges (base only: overlay_id IS NULL).
    let mut edges: Vec<SnapshotEdge> = {
        let mut stmt = conn.prepare(
            "SELECT from_file, from_line, kind, target_name, from_col, end_line, end_col
             FROM code_edges
             WHERE repo_id = ?1 AND overlay_id IS NULL
             ORDER BY from_file, from_line, kind, target_name",
        )?;
        let rows = stmt.query_map(params![repo_id], |row| {
            Ok(SnapshotEdge {
                from_file: row.get(0)?,
                from_line: row.get(1)?,
                kind: row.get(2)?,
                target_name: row.get(3)?,
                from_col: row.get(4)?,
                end_line: row.get(5)?,
                end_col: row.get(6)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };
    edges.sort();

    Ok(CodeGraphSnapshot {
        schema_version: SCHEMA_VERSION,
        base_ref: base_ref.to_string(),
        repo_label,
        files,
        symbols,
        edges,
    })
}

/// Write a snapshot to `.codegraph/` directory within the repo.
pub fn write_snapshot(repo_path: &Path, snapshot: &CodeGraphSnapshot) -> Result<(), IndexError> {
    let codegraph_dir = repo_path.join(".codegraph");
    std::fs::create_dir_all(&codegraph_dir)?;

    // Write snapshot.json (deterministic: sorted keys via serde).
    let json = serde_json::to_string_pretty(snapshot)
        .map_err(|e| IndexError::InvalidPath(format!("JSON serialize error: {e}")))?;
    std::fs::write(codegraph_dir.join("snapshot.json"), json)?;

    // Write snapshot.meta.json.
    let meta = SnapshotMeta {
        schema_version: snapshot.schema_version,
        base_ref: snapshot.base_ref.clone(),
        repo_label: snapshot.repo_label.clone(),
        file_count: snapshot.files.len() as u32,
        symbol_count: snapshot.symbols.len() as u32,
        edge_count: snapshot.edges.len() as u32,
    };
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| IndexError::InvalidPath(format!("JSON serialize error: {e}")))?;
    std::fs::write(codegraph_dir.join("snapshot.meta.json"), meta_json)?;

    Ok(())
}

// ─── Import ─────────────────────────────────────────────────────────────────

/// Import a snapshot from `.codegraph/snapshot.json` into the code index.
///
/// Creates or updates the repo entry and bulk-inserts all symbols/edges/hashes.
/// Existing data for this repo is replaced (clean import).
pub fn import_snapshot(
    conn: &Connection,
    snapshot: &CodeGraphSnapshot,
    repo_path: &str,
) -> Result<ImportResult, IndexError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let tx = conn.unchecked_transaction()?;

    // Upsert repo.
    tx.execute(
        "INSERT INTO code_repos (path, label, indexed_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(path) DO UPDATE SET indexed_at = excluded.indexed_at, label = excluded.label",
        params![repo_path, snapshot.repo_label, now],
    )?;

    let repo_id: i64 = tx.query_row(
        "SELECT id FROM code_repos WHERE path = ?1",
        params![repo_path],
        |r| r.get(0),
    )?;

    // Clear existing base data for this repo (keep overlays intact).
    tx.execute(
        "DELETE FROM code_symbols WHERE repo_id = ?1 AND overlay_id IS NULL",
        params![repo_id],
    )?;
    tx.execute(
        "DELETE FROM code_edges WHERE repo_id = ?1 AND overlay_id IS NULL",
        params![repo_id],
    )?;
    tx.execute(
        "DELETE FROM code_file_hashes WHERE repo_id = ?1",
        params![repo_id],
    )?;

    // Insert file hashes.
    for file in &snapshot.files {
        tx.execute(
            "INSERT INTO code_file_hashes (repo_id, file, hash, indexed_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![repo_id, file.path, file.hash, now],
        )?;
    }

    // Insert symbols.
    for sym in &snapshot.symbols {
        tx.execute(
            "INSERT OR IGNORE INTO code_symbols (repo_id, name, kind, file, line, end_line, parent, overlay_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
            params![repo_id, sym.name, sym.kind, sym.file, sym.line, sym.end_line, sym.parent],
        )?;
    }

    // Insert edges.
    for edge in &snapshot.edges {
        tx.execute(
            "INSERT INTO code_edges (repo_id, from_file, from_line, from_col, end_line, end_col, kind, target_name, overlay_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)",
            params![
                repo_id,
                edge.from_file,
                edge.from_line,
                edge.from_col,
                edge.end_line,
                edge.end_col,
                edge.kind,
                edge.target_name,
            ],
        )?;
    }

    tx.commit()?;

    Ok(ImportResult {
        repo_id,
        files_imported: snapshot.files.len() as u32,
        symbols_imported: snapshot.symbols.len() as u32,
        edges_imported: snapshot.edges.len() as u32,
    })
}

/// Read a snapshot from a `.codegraph/snapshot.json` file.
pub fn read_snapshot(path: &Path) -> Result<CodeGraphSnapshot, IndexError> {
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str(&content)
        .map_err(|e| IndexError::InvalidPath(format!("invalid snapshot JSON: {e}")))
}

/// Result of importing a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub repo_id: i64,
    pub files_imported: u32,
    pub symbols_imported: u32,
    pub edges_imported: u32,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::branch_overlay::ensure_overlay_schema;
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

    fn populate_base_data(conn: &Connection, repo_id: i64) {
        // Files
        conn.execute(
            "INSERT INTO code_file_hashes (repo_id, file, hash, indexed_at) VALUES (?1, 'src/main.rs', 'aaa111', 1000)",
            params![repo_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO code_file_hashes (repo_id, file, hash, indexed_at) VALUES (?1, 'src/lib.rs', 'bbb222', 1000)",
            params![repo_id],
        ).unwrap();

        // Symbols (overlay_id NULL = base)
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, parent, overlay_id)
             VALUES (?1, 'main', 'function', 'src/main.rs', 1, 10, NULL, NULL)",
            params![repo_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, parent, overlay_id)
             VALUES (?1, 'helper', 'function', 'src/lib.rs', 5, 15, NULL, NULL)",
            params![repo_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, parent, overlay_id)
             VALUES (?1, 'MyStruct', 'struct', 'src/lib.rs', 20, 30, NULL, NULL)",
            params![repo_id],
        ).unwrap();

        // Edges
        conn.execute(
            "INSERT INTO code_edges (repo_id, from_file, from_line, kind, target_name, overlay_id)
             VALUES (?1, 'src/main.rs', 5, 'calls', 'helper', NULL)",
            params![repo_id],
        )
        .unwrap();
    }

    #[test]
    fn export_produces_sorted_deterministic_output() {
        let (_dir, conn) = setup_test_db();
        populate_base_data(&conn, 1);

        let snapshot = export_snapshot(&conn, 1, "abc123").unwrap();

        assert_eq!(snapshot.schema_version, SCHEMA_VERSION);
        assert_eq!(snapshot.base_ref, "abc123");
        assert_eq!(snapshot.repo_label, "test-repo");
        assert_eq!(snapshot.files.len(), 2);
        assert_eq!(snapshot.symbols.len(), 3);
        assert_eq!(snapshot.edges.len(), 1);

        // Files are sorted by path.
        assert_eq!(snapshot.files[0].path, "src/lib.rs");
        assert_eq!(snapshot.files[1].path, "src/main.rs");

        // Symbols are sorted by (file, line, name).
        assert_eq!(snapshot.symbols[0].file, "src/lib.rs");
        assert_eq!(snapshot.symbols[0].name, "helper");
        assert_eq!(snapshot.symbols[0].line, 5);
        assert_eq!(snapshot.symbols[1].file, "src/lib.rs");
        assert_eq!(snapshot.symbols[1].name, "MyStruct");
        assert_eq!(snapshot.symbols[1].line, 20);
        assert_eq!(snapshot.symbols[2].file, "src/main.rs");
        assert_eq!(snapshot.symbols[2].name, "main");
    }

    #[test]
    fn determinism_same_data_produces_identical_json() {
        let (_dir, conn) = setup_test_db();
        populate_base_data(&conn, 1);

        let snap1 = export_snapshot(&conn, 1, "commit_a").unwrap();
        let json1 = serde_json::to_string_pretty(&snap1).unwrap();

        let snap2 = export_snapshot(&conn, 1, "commit_a").unwrap();
        let json2 = serde_json::to_string_pretty(&snap2).unwrap();

        assert_eq!(json1, json2, "Same data should produce byte-identical JSON");
    }

    #[test]
    fn round_trip_export_import() {
        let (dir, conn) = setup_test_db();
        populate_base_data(&conn, 1);

        // Export.
        let snapshot = export_snapshot(&conn, 1, "abc123").unwrap();

        // Write to disk.
        write_snapshot(dir.path(), &snapshot).unwrap();
        assert!(dir.path().join(".codegraph/snapshot.json").exists());
        assert!(dir.path().join(".codegraph/snapshot.meta.json").exists());

        // Read meta.
        let meta_str =
            std::fs::read_to_string(dir.path().join(".codegraph/snapshot.meta.json")).unwrap();
        let meta: SnapshotMeta = serde_json::from_str(&meta_str).unwrap();
        assert_eq!(meta.file_count, 2);
        assert_eq!(meta.symbol_count, 3);
        assert_eq!(meta.edge_count, 1);

        // Create a fresh DB and import.
        let dir2 = TempDir::new().unwrap();
        let conn2 = open_db(dir2.path()).unwrap();
        ensure_overlay_schema(&conn2).unwrap();

        let read_snap = read_snapshot(&dir.path().join(".codegraph/snapshot.json")).unwrap();
        let result = import_snapshot(&conn2, &read_snap, "/new/repo/path").unwrap();

        assert_eq!(result.files_imported, 2);
        assert_eq!(result.symbols_imported, 3);
        assert_eq!(result.edges_imported, 1);

        // Re-export from the imported DB should produce identical snapshot (minus base_ref from new export).
        let re_exported = export_snapshot(&conn2, result.repo_id, "abc123").unwrap();
        assert_eq!(re_exported.files, snapshot.files);
        assert_eq!(re_exported.symbols, snapshot.symbols);
        assert_eq!(re_exported.edges, snapshot.edges);
    }

    #[test]
    fn import_replaces_existing_base_data() {
        let (_dir, conn) = setup_test_db();
        populate_base_data(&conn, 1);

        // Count before.
        let sym_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM code_symbols WHERE repo_id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sym_count, 3);

        // Import a smaller snapshot.
        let small_snapshot = CodeGraphSnapshot {
            schema_version: SCHEMA_VERSION,
            base_ref: "new_ref".to_string(),
            repo_label: "test-repo".to_string(),
            files: vec![SnapshotFile {
                path: "src/only.rs".to_string(),
                hash: "xxx".to_string(),
            }],
            symbols: vec![SnapshotSymbol {
                file: "src/only.rs".to_string(),
                line: 1,
                end_line: 5,
                name: "only_fn".to_string(),
                kind: "function".to_string(),
                parent: None,
            }],
            edges: vec![],
        };

        import_snapshot(&conn, &small_snapshot, "/test/repo").unwrap();

        // After import, only the new data exists.
        let sym_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM code_symbols WHERE repo_id = 1 AND overlay_id IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sym_count, 1);
    }

    #[test]
    fn snapshot_excludes_overlay_data() {
        let (_dir, conn) = setup_test_db();
        populate_base_data(&conn, 1);

        // Add overlay data.
        conn.execute(
            "INSERT INTO code_branch_overlays (repo_id, base_ref, branch_ref, file, hash, indexed_at)
             VALUES (1, 'base', 'feat', 'src/main.rs', 'overlay_hash', 2000)",
            [],
        )
        .unwrap();
        let overlay_id: i64 = conn
            .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
            .unwrap();
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, parent, overlay_id)
             VALUES (1, 'overlay_fn', 'function', 'src/main.rs', 50, 60, NULL, ?1)",
            params![overlay_id],
        )
        .unwrap();

        // Export should NOT include overlay data.
        let snapshot = export_snapshot(&conn, 1, "commit1").unwrap();
        let names: Vec<&str> = snapshot.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(!names.contains(&"overlay_fn"));
        assert!(names.contains(&"main"));
    }

    #[test]
    fn snapshot_no_timestamps_in_output() {
        let (_dir, conn) = setup_test_db();
        populate_base_data(&conn, 1);

        let snapshot = export_snapshot(&conn, 1, "abc").unwrap();
        let json = serde_json::to_string(&snapshot).unwrap();

        // The JSON should not contain "indexed_at" or any timestamp field.
        assert!(!json.contains("indexed_at"));
        assert!(!json.contains("received_at"));
        assert!(!json.contains("created_at"));
    }
}
