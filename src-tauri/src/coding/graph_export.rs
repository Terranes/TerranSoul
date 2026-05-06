//! Code-graph JSON snapshot export (Chunk 36B.1).
//!
//! Produces a reviewable `code-graph.json` from the existing
//! `code_index.sqlite` symbol/edge data. The snapshot is
//! self-contained: it includes all symbols, edges, and repo
//! metadata so it can be committed to the repository for
//! version-controlled review of architecture evolution.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::symbol_index::{CodeEdge, EdgeKind, IndexError, Symbol, SymbolKind};

/// Top-level code graph snapshot suitable for JSON serialisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGraphSnapshot {
    /// Schema version for forward-compatible parsing.
    pub version: u32,
    /// Repository label (user-defined name or path).
    pub repo_label: String,
    /// Absolute path to the indexed repository.
    pub repo_path: String,
    /// Unix-ms timestamp of when the snapshot was generated.
    pub generated_at: i64,
    /// All extracted symbols.
    pub symbols: Vec<Symbol>,
    /// All extracted edges (imports, calls, etc.).
    pub edges: Vec<CodeEdge>,
    /// Summary statistics.
    pub stats: SnapshotStats,
}

/// Summary statistics embedded in the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotStats {
    pub file_count: u32,
    pub symbol_count: u32,
    pub edge_count: u32,
}

/// Current schema version for the snapshot format.
const SNAPSHOT_VERSION: u32 = 1;

/// Export a code-graph snapshot for the given repo_id.
///
/// Reads all symbols and edges from the code_index SQLite database
/// and packages them into a [`CodeGraphSnapshot`].
pub fn export_snapshot(conn: &Connection, repo_id: i64) -> Result<CodeGraphSnapshot, IndexError> {
    // Fetch repo metadata
    let (repo_path, repo_label, _indexed_at): (String, String, i64) = conn.query_row(
        "SELECT path, label, indexed_at FROM code_repos WHERE id = ?1",
        params![repo_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    // Fetch all symbols
    let mut sym_stmt = conn.prepare(
        "SELECT name, kind, file, line, end_line, parent
         FROM code_symbols WHERE repo_id = ?1 ORDER BY file, line",
    )?;
    let symbols: Vec<Symbol> = sym_stmt
        .query_map(params![repo_id], |row| {
            Ok(Symbol {
                name: row.get(0)?,
                kind: SymbolKind::parse(&row.get::<_, String>(1)?).unwrap_or(SymbolKind::Function),
                file: row.get(2)?,
                line: row.get(3)?,
                end_line: row.get(4)?,
                parent: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Fetch all edges
    let mut edge_stmt = conn.prepare(
        "SELECT from_file, from_line, from_col, end_line, end_col, kind, target_name
         FROM code_edges WHERE repo_id = ?1 ORDER BY from_file, from_line",
    )?;
    let edges: Vec<CodeEdge> = edge_stmt
        .query_map(params![repo_id], |row| {
            Ok(CodeEdge {
                from_file: row.get(0)?,
                from_line: row.get(1)?,
                from_col: row.get(2)?,
                end_line: row.get(3)?,
                end_col: row.get(4)?,
                kind: EdgeKind::parse(&row.get::<_, String>(5)?).unwrap_or(EdgeKind::Calls),
                target_name: row.get(6)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Count distinct files
    let file_count: u32 = conn.query_row(
        "SELECT COUNT(DISTINCT file) FROM code_symbols WHERE repo_id = ?1",
        params![repo_id],
        |row| row.get(0),
    )?;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    Ok(CodeGraphSnapshot {
        version: SNAPSHOT_VERSION,
        repo_label,
        repo_path,
        generated_at: now_ms,
        stats: SnapshotStats {
            file_count,
            symbol_count: symbols.len() as u32,
            edge_count: edges.len() as u32,
        },
        symbols,
        edges,
    })
}

/// Export the snapshot to a JSON file at the given path.
pub fn export_to_file(
    conn: &Connection,
    repo_id: i64,
    output_path: &Path,
) -> Result<CodeGraphSnapshot, IndexError> {
    let snapshot = export_snapshot(conn, repo_id)?;
    let json = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| IndexError::Io(std::io::Error::other(e)))?;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, json)?;
    Ok(snapshot)
}

/// Export for the first registered repo (convenience for single-repo use).
pub fn export_first_repo(
    data_dir: &Path,
    output_path: &Path,
) -> Result<CodeGraphSnapshot, IndexError> {
    let conn = super::symbol_index::open_db(data_dir)?;
    let repo_id: i64 = conn
        .query_row("SELECT id FROM code_repos ORDER BY id LIMIT 1", [], |row| {
            row.get(0)
        })
        .map_err(|_| {
            IndexError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no indexed repositories found",
            ))
        })?;
    export_to_file(&conn, repo_id, output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_test_db() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let conn = super::super::symbol_index::open_db(dir.path()).unwrap();
        // Insert a test repo
        conn.execute(
            "INSERT INTO code_repos (path, label, indexed_at) VALUES (?1, ?2, ?3)",
            params!["/tmp/test-repo", "test-repo", 1714000000000i64],
        )
        .unwrap();
        let repo_id = conn.last_insert_rowid();
        // Insert symbols
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, parent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                repo_id,
                "main",
                "function",
                "src/main.rs",
                1,
                10,
                None::<String>
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, parent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                repo_id,
                "helper",
                "function",
                "src/utils.rs",
                5,
                20,
                None::<String>
            ],
        )
        .unwrap();
        // Insert edges
        conn.execute(
            "INSERT INTO code_edges (repo_id, from_file, from_line, from_col, end_line, end_col, kind, target_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![repo_id, "src/main.rs", 3, None::<u32>, None::<u32>, None::<u32>, "calls", "helper"],
        )
        .unwrap();
        (dir, conn)
    }

    #[test]
    fn export_snapshot_contains_symbols_and_edges() {
        let (_dir, conn) = setup_test_db();
        let snapshot = export_snapshot(&conn, 1).unwrap();
        assert_eq!(snapshot.version, 1);
        assert_eq!(snapshot.repo_label, "test-repo");
        assert_eq!(snapshot.symbols.len(), 2);
        assert_eq!(snapshot.edges.len(), 1);
        assert_eq!(snapshot.stats.file_count, 2);
        assert_eq!(snapshot.stats.symbol_count, 2);
        assert_eq!(snapshot.stats.edge_count, 1);
        assert!(snapshot.generated_at > 0);
    }

    #[test]
    fn export_to_file_writes_valid_json() {
        let (dir, conn) = setup_test_db();
        let output = dir.path().join("code-graph.json");
        let snapshot = export_to_file(&conn, 1, &output).unwrap();
        assert!(output.exists());

        let content = std::fs::read_to_string(&output).unwrap();
        let parsed: CodeGraphSnapshot = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.symbols.len(), snapshot.symbols.len());
        assert_eq!(parsed.edges.len(), snapshot.edges.len());
    }

    #[test]
    fn export_first_repo_works() {
        let (dir, _conn) = setup_test_db();
        let output = dir.path().join("out/code-graph.json");
        let snapshot = export_first_repo(dir.path(), &output).unwrap();
        assert_eq!(snapshot.repo_label, "test-repo");
        assert!(output.exists());
    }

    #[test]
    fn export_snapshot_empty_repo_returns_empty() {
        let dir = tempdir().unwrap();
        let conn = super::super::symbol_index::open_db(dir.path()).unwrap();
        conn.execute(
            "INSERT INTO code_repos (path, label, indexed_at) VALUES (?1, ?2, ?3)",
            params!["/tmp/empty", "empty", 0i64],
        )
        .unwrap();
        let snapshot = export_snapshot(&conn, 1).unwrap();
        assert_eq!(snapshot.symbols.len(), 0);
        assert_eq!(snapshot.edges.len(), 0);
        assert_eq!(snapshot.stats.file_count, 0);
    }
}
