//! Cross-repo contract drift detection (Chunk 45.6).
//!
//! Provides two operations:
//! - `branch_diff(left_ref, right_ref)` — compares symbols between two refs,
//!   returning added/removed/renamed/signature-changed symbols.
//! - `group_drift(group_label)` — across all repos in a group, detects
//!   contracts whose `signature_hash` differs between repos (breaking changes).

use std::collections::HashMap;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::symbol_index::IndexError;

// ─── Types ──────────────────────────────────────────────────────────────────

/// A single diff entry between two refs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolDiff {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    pub change: DiffChange,
}

/// The kind of change for a symbol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiffChange {
    Added,
    Removed,
    /// Symbol exists in both refs but signature_hash differs.
    Modified {
        old_file: String,
        old_line: u32,
    },
}

/// Result of a branch diff operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchDiffResult {
    pub repo_id: i64,
    pub left_ref: String,
    pub right_ref: String,
    pub added: Vec<SymbolDiff>,
    pub removed: Vec<SymbolDiff>,
    pub modified: Vec<SymbolDiff>,
}

/// A contract that has drifted across repos in a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftEntry {
    pub name: String,
    pub kind: String,
    /// Map of repo_label → signature_hash for this contract.
    pub signatures: HashMap<String, String>,
    /// The repos that have a different signature from the majority.
    pub drifted_repos: Vec<String>,
    /// The majority signature (considered "correct").
    pub expected_hash: String,
}

/// Result of a group drift scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupDriftResult {
    pub group_label: String,
    pub total_contracts: u32,
    pub drifted_contracts: Vec<DriftEntry>,
}

// ─── Branch Diff ────────────────────────────────────────────────────────────

/// Compare symbols between `left_ref` (base) and `right_ref` (branch overlay).
///
/// Uses the overlay schema: base symbols are `overlay_id IS NULL`,
/// branch symbols come from `code_branch_overlays` matching the refs.
pub fn branch_diff(
    conn: &Connection,
    repo_id: i64,
    left_ref: &str,
    right_ref: &str,
) -> Result<BranchDiffResult, IndexError> {
    // Load base symbols (left_ref = base, overlay_id IS NULL).
    let base_symbols = load_base_symbols(conn, repo_id)?;

    // Load overlay symbols for the right_ref.
    let overlay_symbols = load_overlay_symbols(conn, repo_id, left_ref, right_ref)?;

    // If no overlay exists, compare against itself (no diff).
    if overlay_symbols.is_empty() {
        return Ok(BranchDiffResult {
            repo_id,
            left_ref: left_ref.to_string(),
            right_ref: right_ref.to_string(),
            added: Vec::new(),
            removed: Vec::new(),
            modified: Vec::new(),
        });
    }

    // Build lookup maps by (name, kind, file).
    let base_map: HashMap<(&str, &str), &SymRow> = base_symbols
        .iter()
        .map(|s| ((s.name.as_str(), s.kind.as_str()), s))
        .collect();

    let overlay_map: HashMap<(&str, &str), &SymRow> = overlay_symbols
        .iter()
        .map(|s| ((s.name.as_str(), s.kind.as_str()), s))
        .collect();

    // Find files that the overlay touches.
    let overlay_files: std::collections::HashSet<&str> =
        overlay_symbols.iter().map(|s| s.file.as_str()).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();

    // Symbols in overlay but not in base (for the same file) = added.
    for (key, sym) in &overlay_map {
        if !base_map.contains_key(key) {
            added.push(SymbolDiff {
                name: sym.name.clone(),
                kind: sym.kind.clone(),
                file: sym.file.clone(),
                line: sym.line,
                change: DiffChange::Added,
            });
        }
    }

    // Symbols in base (for overlay files) but not in overlay = removed.
    for (key, sym) in &base_map {
        if overlay_files.contains(sym.file.as_str()) && !overlay_map.contains_key(key) {
            removed.push(SymbolDiff {
                name: sym.name.clone(),
                kind: sym.kind.clone(),
                file: sym.file.clone(),
                line: sym.line,
                change: DiffChange::Removed,
            });
        }
    }

    // Symbols in both but with different line (proxy for signature change).
    for (key, overlay_sym) in &overlay_map {
        if let Some(base_sym) = base_map.get(key) {
            if base_sym.line != overlay_sym.line || base_sym.file != overlay_sym.file {
                modified.push(SymbolDiff {
                    name: overlay_sym.name.clone(),
                    kind: overlay_sym.kind.clone(),
                    file: overlay_sym.file.clone(),
                    line: overlay_sym.line,
                    change: DiffChange::Modified {
                        old_file: base_sym.file.clone(),
                        old_line: base_sym.line,
                    },
                });
            }
        }
    }

    // Sort for determinism.
    added.sort_by(|a, b| a.name.cmp(&b.name));
    removed.sort_by(|a, b| a.name.cmp(&b.name));
    modified.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(BranchDiffResult {
        repo_id,
        left_ref: left_ref.to_string(),
        right_ref: right_ref.to_string(),
        added,
        removed,
        modified,
    })
}

// ─── Group Drift ────────────────────────────────────────────────────────────

/// Detect contract drift across all repos in a group.
///
/// For each contract name that appears in multiple repos, checks if the
/// `signature_hash` is consistent. Contracts with differing hashes are
/// reported as drifted.
pub fn group_drift(
    conn: &Connection,
    data_dir: &std::path::Path,
    group_label: &str,
) -> Result<GroupDriftResult, IndexError> {
    // Find the group.
    let group_id: i64 = conn
        .query_row(
            "SELECT id FROM code_repo_groups WHERE label = ?1",
            params![group_label],
            |r| r.get(0),
        )
        .map_err(|_| IndexError::InvalidPath(format!("group not found: {group_label}")))?;

    // Load the code index DB.
    let code_conn = super::symbol_index::open_db(data_dir)?;

    // Get all repos in this group.
    let mut stmt = code_conn.prepare(
        "SELECT r.id, r.label FROM code_repos r
         JOIN code_repo_group_members m ON m.repo_id = r.id
         WHERE m.group_id = ?1",
    )?;
    let repos: Vec<(i64, String)> = stmt
        .query_map(params![group_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    if repos.is_empty() {
        return Ok(GroupDriftResult {
            group_label: group_label.to_string(),
            total_contracts: 0,
            drifted_contracts: Vec::new(),
        });
    }

    // Load all contracts across repos in the group.
    let repo_ids: Vec<i64> = repos.iter().map(|(id, _)| *id).collect();
    let repo_labels: HashMap<i64, String> = repos.into_iter().collect();

    let placeholders: String = repo_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT repo_id, name, kind, signature_hash FROM code_contracts WHERE repo_id IN ({placeholders})"
    );
    let mut stmt = code_conn.prepare(&query)?;

    // Bind parameters manually.
    let params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = repo_ids
        .iter()
        .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
        .collect();
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();

    let contracts: Vec<(i64, String, String, String)> = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Group by (name, kind) → repo_label → signature_hash.
    let mut contract_map: HashMap<(String, String), HashMap<String, String>> = HashMap::new();
    for (repo_id, name, kind, sig_hash) in &contracts {
        let label = repo_labels.get(repo_id).cloned().unwrap_or_default();
        contract_map
            .entry((name.clone(), kind.clone()))
            .or_default()
            .insert(label, sig_hash.clone());
    }

    let total_contracts = contract_map.len() as u32;
    let mut drifted_contracts = Vec::new();

    // For each contract, find drift (signature mismatch across repos).
    for ((name, kind), signatures) in &contract_map {
        if signatures.len() < 2 {
            continue; // Only in one repo — no drift possible.
        }

        // Find the majority signature.
        let mut hash_counts: HashMap<&str, u32> = HashMap::new();
        for hash in signatures.values() {
            *hash_counts.entry(hash.as_str()).or_insert(0) += 1;
        }

        let (&majority_hash, _) = hash_counts.iter().max_by_key(|(_, &c)| c).unwrap();

        // If all are the same, no drift.
        if hash_counts.len() == 1 {
            continue;
        }

        // Find repos that differ from majority.
        let drifted_repos: Vec<String> = signatures
            .iter()
            .filter(|(_, hash)| hash.as_str() != majority_hash)
            .map(|(label, _)| label.clone())
            .collect();

        drifted_contracts.push(DriftEntry {
            name: name.clone(),
            kind: kind.clone(),
            signatures: signatures.clone(),
            drifted_repos,
            expected_hash: majority_hash.to_string(),
        });
    }

    // Sort for determinism.
    drifted_contracts.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(GroupDriftResult {
        group_label: group_label.to_string(),
        total_contracts,
        drifted_contracts,
    })
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Internal symbol row for diff computation.
#[derive(Debug)]
struct SymRow {
    name: String,
    kind: String,
    file: String,
    line: u32,
}

/// Load base symbols (overlay_id IS NULL) for a repo.
fn load_base_symbols(conn: &Connection, repo_id: i64) -> Result<Vec<SymRow>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT name, kind, file, line FROM code_symbols
         WHERE repo_id = ?1 AND overlay_id IS NULL",
    )?;
    let rows = stmt
        .query_map(params![repo_id], |row| {
            Ok(SymRow {
                name: row.get(0)?,
                kind: row.get(1)?,
                file: row.get(2)?,
                line: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

/// Load overlay symbols for a specific base_ref + branch_ref combination.
fn load_overlay_symbols(
    conn: &Connection,
    repo_id: i64,
    base_ref: &str,
    branch_ref: &str,
) -> Result<Vec<SymRow>, IndexError> {
    let mut stmt = conn.prepare(
        "SELECT s.name, s.kind, s.file, s.line
         FROM code_symbols s
         JOIN code_branch_overlays o ON o.id = s.overlay_id
         WHERE s.repo_id = ?1 AND o.base_ref = ?2 AND o.branch_ref = ?3",
    )?;
    let rows = stmt
        .query_map(params![repo_id, base_ref, branch_ref], |row| {
            Ok(SymRow {
                name: row.get(0)?,
                kind: row.get(1)?,
                file: row.get(2)?,
                line: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::branch_overlay::ensure_overlay_schema;
    use crate::coding::symbol_index::open_db;
    use tempfile::TempDir;

    fn setup_drift_db() -> (TempDir, Connection) {
        let dir = TempDir::new().unwrap();
        let conn = open_db(dir.path()).unwrap();
        ensure_overlay_schema(&conn).unwrap();

        (dir, conn)
    }

    #[test]
    fn branch_diff_detects_added_symbols() {
        let (_dir, conn) = setup_drift_db();

        // Create repo.
        conn.execute(
            "INSERT INTO code_repos (path, label, indexed_at) VALUES ('/repo', 'repo', 1000)",
            [],
        )
        .unwrap();

        // Base symbols.
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (1, 'existing_fn', 'function', 'src/lib.rs', 10, 20, NULL)",
            [],
        )
        .unwrap();

        // Create overlay (branch).
        conn.execute(
            "INSERT INTO code_branch_overlays (repo_id, base_ref, branch_ref, file, hash, indexed_at)
             VALUES (1, 'main', 'feat/add', 'src/lib.rs', 'newhash', 2000)",
            [],
        )
        .unwrap();
        let overlay_id: i64 = conn
            .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
            .unwrap();

        // Overlay has the existing symbol (at line 11 to avoid UNIQUE) + a new one.
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (1, 'existing_fn', 'function', 'src/lib.rs', 11, 20, ?1)",
            params![overlay_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (1, 'new_fn', 'function', 'src/lib.rs', 25, 35, ?1)",
            params![overlay_id],
        )
        .unwrap();

        let result = branch_diff(&conn, 1, "main", "feat/add").unwrap();
        assert_eq!(result.added.len(), 1);
        assert_eq!(result.added[0].name, "new_fn");
        // existing_fn shows as modified since line changed (10 → 11).
        assert_eq!(result.modified.len(), 1);
        assert_eq!(result.modified[0].name, "existing_fn");
    }

    #[test]
    fn branch_diff_detects_removed_symbols() {
        let (_dir, conn) = setup_drift_db();

        conn.execute(
            "INSERT INTO code_repos (path, label, indexed_at) VALUES ('/repo', 'repo', 1000)",
            [],
        )
        .unwrap();

        // Base has two symbols.
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (1, 'fn_a', 'function', 'src/lib.rs', 1, 5, NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (1, 'fn_b', 'function', 'src/lib.rs', 10, 15, NULL)",
            [],
        )
        .unwrap();

        // Overlay only has fn_a (at line 2 to avoid UNIQUE), fn_b removed.
        conn.execute(
            "INSERT INTO code_branch_overlays (repo_id, base_ref, branch_ref, file, hash, indexed_at)
             VALUES (1, 'main', 'feat/rm', 'src/lib.rs', 'hash2', 2000)",
            [],
        )
        .unwrap();
        let overlay_id: i64 = conn
            .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
            .unwrap();
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (1, 'fn_a', 'function', 'src/lib.rs', 2, 5, ?1)",
            params![overlay_id],
        )
        .unwrap();

        let result = branch_diff(&conn, 1, "main", "feat/rm").unwrap();
        assert_eq!(result.removed.len(), 1);
        assert_eq!(result.removed[0].name, "fn_b");
    }

    #[test]
    fn branch_diff_detects_modified_symbols() {
        let (_dir, conn) = setup_drift_db();

        conn.execute(
            "INSERT INTO code_repos (path, label, indexed_at) VALUES ('/repo', 'repo', 1000)",
            [],
        )
        .unwrap();

        // Base symbol at line 10.
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (1, 'handler', 'function', 'src/api.rs', 10, 20, NULL)",
            [],
        )
        .unwrap();

        // Overlay has same symbol but at different line (signature changed).
        conn.execute(
            "INSERT INTO code_branch_overlays (repo_id, base_ref, branch_ref, file, hash, indexed_at)
             VALUES (1, 'main', 'feat/sig', 'src/api.rs', 'changed', 2000)",
            [],
        )
        .unwrap();
        let overlay_id: i64 = conn
            .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
            .unwrap();
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (1, 'handler', 'function', 'src/api.rs', 15, 30, ?1)",
            params![overlay_id],
        )
        .unwrap();

        let result = branch_diff(&conn, 1, "main", "feat/sig").unwrap();
        assert_eq!(result.modified.len(), 1);
        assert_eq!(result.modified[0].name, "handler");
        assert!(matches!(
            result.modified[0].change,
            DiffChange::Modified { old_line: 10, .. }
        ));
    }

    #[test]
    fn group_drift_detects_signature_mismatch() {
        let (dir, conn) = setup_drift_db();

        // Create two repos.
        conn.execute(
            "INSERT INTO code_repos (path, label, indexed_at) VALUES ('/repo_a', 'frontend', 1000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_repos (path, label, indexed_at) VALUES ('/repo_b', 'backend', 1000)",
            [],
        )
        .unwrap();

        // Create a group.
        conn.execute(
            "INSERT INTO code_repo_groups (label, description, created_at) VALUES ('my-group', 'test', 1000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_repo_group_members (group_id, repo_id) VALUES (1, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_repo_group_members (group_id, repo_id) VALUES (1, 2)",
            [],
        )
        .unwrap();

        // Create symbols for contracts.
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (1, 'ApiResponse', 'struct', 'src/types.ts', 5, 15, NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (2, 'ApiResponse', 'struct', 'src/response.rs', 10, 25, NULL)",
            [],
        )
        .unwrap();

        // Contracts with different signature hashes (drift!).
        conn.execute(
            "INSERT INTO code_contracts (repo_id, symbol_id, name, kind, file, line, signature_hash, extracted_at)
             VALUES (1, 1, 'ApiResponse', 'struct', 'src/types.ts', 5, 'hash_v1', 1000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_contracts (repo_id, symbol_id, name, kind, file, line, signature_hash, extracted_at)
             VALUES (2, 2, 'ApiResponse', 'struct', 'src/response.rs', 10, 'hash_v2', 1000)",
            [],
        )
        .unwrap();

        let result = group_drift(&conn, dir.path(), "my-group").unwrap();
        assert_eq!(result.total_contracts, 1);
        assert_eq!(result.drifted_contracts.len(), 1);
        assert_eq!(result.drifted_contracts[0].name, "ApiResponse");
        assert_eq!(result.drifted_contracts[0].drifted_repos.len(), 1);
    }

    #[test]
    fn group_drift_no_drift_when_hashes_match() {
        let (dir, conn) = setup_drift_db();

        conn.execute(
            "INSERT INTO code_repos (path, label, indexed_at) VALUES ('/a', 'svc-a', 1000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_repos (path, label, indexed_at) VALUES ('/b', 'svc-b', 1000)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO code_repo_groups (label, description, created_at) VALUES ('aligned', 'test', 1000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_repo_group_members (group_id, repo_id) VALUES (1, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_repo_group_members (group_id, repo_id) VALUES (1, 2)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (1, 'Config', 'struct', 'a.rs', 1, 5, NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, overlay_id)
             VALUES (2, 'Config', 'struct', 'b.rs', 1, 5, NULL)",
            [],
        )
        .unwrap();

        // Same signature hash — no drift.
        conn.execute(
            "INSERT INTO code_contracts (repo_id, symbol_id, name, kind, file, line, signature_hash, extracted_at)
             VALUES (1, 1, 'Config', 'struct', 'a.rs', 1, 'same_hash', 1000)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO code_contracts (repo_id, symbol_id, name, kind, file, line, signature_hash, extracted_at)
             VALUES (2, 2, 'Config', 'struct', 'b.rs', 1, 'same_hash', 1000)",
            [],
        )
        .unwrap();

        let result = group_drift(&conn, dir.path(), "aligned").unwrap();
        assert_eq!(result.drifted_contracts.len(), 0);
    }
}
