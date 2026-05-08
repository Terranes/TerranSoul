//! Multi-repo groups and contracts (Chunk 37.13).
//!
//! Provides cross-repo grouping, contract extraction, group status, and
//! cross-service query surfaces over the native code intelligence database.
//!
//! ## Concepts
//!
//! - **Group** — a named collection of indexed repositories that share a
//!   logical boundary (e.g., a microservice constellation, a frontend +
//!   backend pair, a monorepo split into packages).
//! - **Contract** — a public/exported symbol surface from a repo that other
//!   repos in the same group may depend on. Contracts include a
//!   `signature_hash` that detects shape changes across re-indexing runs.
//! - **Group status** — aggregated indexing/contract counts across all
//!   member repos.
//! - **Cross-service query** — symbol search restricted to a single group
//!   so queries return matches across all member repos in one call.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::symbol_index::{open_db, IndexError};

// ─── Public types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoGroup {
    pub id: i64,
    pub label: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub member_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub repo_id: i64,
    pub repo_label: String,
    pub repo_path: String,
    pub role: Option<String>,
    pub indexed_at: i64,
    pub symbol_count: u32,
    pub contract_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupStatus {
    pub group: RepoGroup,
    pub members: Vec<GroupMember>,
    pub total_symbols: u32,
    pub total_contracts: u32,
    pub stalest_indexed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEntry {
    pub repo_id: i64,
    pub repo_label: String,
    pub symbol_id: i64,
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    pub signature_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractExtractResult {
    pub repo_id: i64,
    pub extracted: usize,
    pub removed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossRepoMatch {
    pub repo_id: i64,
    pub repo_label: String,
    pub symbol_id: i64,
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    pub is_contract: bool,
}

// ─── Group CRUD ─────────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Create a new repo group. Fails if a group with the same label already
/// exists (UNIQUE constraint on `label`).
pub fn create_group(
    data_dir: &Path,
    label: &str,
    description: Option<&str>,
) -> Result<RepoGroup, IndexError> {
    let label = label.trim();
    if label.is_empty() {
        return Err(IndexError::InvalidPath("group label is empty".into()));
    }
    let conn = open_db(data_dir)?;
    let created_at = now_ms();
    conn.execute(
        "INSERT INTO code_repo_groups (label, description, created_at) VALUES (?1, ?2, ?3)",
        params![label, description, created_at],
    )?;
    let id = conn.last_insert_rowid();
    Ok(RepoGroup {
        id,
        label: label.to_string(),
        description: description.map(|s| s.to_string()),
        created_at,
        member_count: 0,
    })
}

/// List all groups with member counts.
pub fn list_groups(data_dir: &Path) -> Result<Vec<RepoGroup>, IndexError> {
    let conn = open_db(data_dir)?;
    let mut stmt = conn.prepare(
        "SELECT g.id, g.label, g.description, g.created_at,
                COUNT(m.id) AS member_count
         FROM code_repo_groups g
         LEFT JOIN code_repo_group_members m ON m.group_id = g.id
         GROUP BY g.id
         ORDER BY g.label ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(RepoGroup {
            id: r.get(0)?,
            label: r.get(1)?,
            description: r.get(2)?,
            created_at: r.get(3)?,
            member_count: r.get::<_, i64>(4)? as u32,
        })
    })?;
    let mut groups = Vec::new();
    for row in rows {
        groups.push(row?);
    }
    Ok(groups)
}

/// Delete a group and its memberships (contracts are NOT removed; they are
/// keyed on repos, not groups).
pub fn delete_group(data_dir: &Path, group_id: i64) -> Result<(), IndexError> {
    let conn = open_db(data_dir)?;
    let affected = conn.execute(
        "DELETE FROM code_repo_groups WHERE id = ?1",
        params![group_id],
    )?;
    if affected == 0 {
        return Err(IndexError::InvalidPath(format!(
            "group not found: {group_id}"
        )));
    }
    Ok(())
}

/// Add a repo to a group with an optional role tag (e.g., "frontend",
/// "backend", "shared"). Idempotent: re-adding updates the role.
pub fn add_repo_to_group(
    data_dir: &Path,
    group_id: i64,
    repo_id: i64,
    role: Option<&str>,
) -> Result<(), IndexError> {
    let conn = open_db(data_dir)?;
    // Verify both exist before insert to give better error messages.
    let group_exists: bool = conn
        .query_row(
            "SELECT 1 FROM code_repo_groups WHERE id = ?1",
            params![group_id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !group_exists {
        return Err(IndexError::InvalidPath(format!(
            "group not found: {group_id}"
        )));
    }
    let repo_exists: bool = conn
        .query_row(
            "SELECT 1 FROM code_repos WHERE id = ?1",
            params![repo_id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !repo_exists {
        return Err(IndexError::InvalidPath(format!(
            "repo not found: {repo_id}"
        )));
    }
    conn.execute(
        "INSERT INTO code_repo_group_members (group_id, repo_id, role)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(group_id, repo_id) DO UPDATE SET role = excluded.role",
        params![group_id, repo_id, role],
    )?;
    Ok(())
}

/// Remove a repo from a group.
pub fn remove_repo_from_group(
    data_dir: &Path,
    group_id: i64,
    repo_id: i64,
) -> Result<(), IndexError> {
    let conn = open_db(data_dir)?;
    let affected = conn.execute(
        "DELETE FROM code_repo_group_members WHERE group_id = ?1 AND repo_id = ?2",
        params![group_id, repo_id],
    )?;
    if affected == 0 {
        return Err(IndexError::InvalidPath(format!(
            "membership not found: group {group_id}, repo {repo_id}"
        )));
    }
    Ok(())
}

// ─── Group status ───────────────────────────────────────────────────────────

/// Return aggregated status for a group: members, symbol/contract counts,
/// and the stalest member's last-indexed timestamp.
pub fn group_status(data_dir: &Path, group_id: i64) -> Result<GroupStatus, IndexError> {
    let conn = open_db(data_dir)?;

    let group: RepoGroup = conn
        .query_row(
            "SELECT g.id, g.label, g.description, g.created_at,
                    (SELECT COUNT(*) FROM code_repo_group_members WHERE group_id = g.id)
             FROM code_repo_groups g WHERE g.id = ?1",
            params![group_id],
            |r| {
                Ok(RepoGroup {
                    id: r.get(0)?,
                    label: r.get(1)?,
                    description: r.get(2)?,
                    created_at: r.get(3)?,
                    member_count: r.get::<_, i64>(4)? as u32,
                })
            },
        )
        .map_err(|_| IndexError::InvalidPath(format!("group not found: {group_id}")))?;

    let mut stmt = conn.prepare(
        "SELECT r.id, r.label, r.path, m.role, r.indexed_at,
                (SELECT COUNT(*) FROM code_symbols WHERE repo_id = r.id)   AS sc,
                (SELECT COUNT(*) FROM code_contracts WHERE repo_id = r.id) AS cc
         FROM code_repo_group_members m
         JOIN code_repos r ON r.id = m.repo_id
         WHERE m.group_id = ?1
         ORDER BY r.label ASC",
    )?;
    let rows = stmt.query_map(params![group_id], |r| {
        Ok(GroupMember {
            repo_id: r.get(0)?,
            repo_label: r.get(1)?,
            repo_path: r.get(2)?,
            role: r.get(3)?,
            indexed_at: r.get(4)?,
            symbol_count: r.get::<_, i64>(5)? as u32,
            contract_count: r.get::<_, i64>(6)? as u32,
        })
    })?;
    let mut members = Vec::new();
    for row in rows {
        members.push(row?);
    }

    let total_symbols: u32 = members.iter().map(|m| m.symbol_count).sum();
    let total_contracts: u32 = members.iter().map(|m| m.contract_count).sum();
    let stalest_indexed_at = members.iter().map(|m| m.indexed_at).min();

    Ok(GroupStatus {
        group,
        members,
        total_symbols,
        total_contracts,
        stalest_indexed_at,
    })
}

// ─── Contract extraction ────────────────────────────────────────────────────

/// Compute a stable signature hash for a symbol. The hash captures
/// `name|kind|parent` so a public API surface change (rename, kind change,
/// reparent) produces a different hash and can be detected as a breaking
/// change across re-indexing runs.
fn signature_hash(name: &str, kind: &str, parent: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(b"|");
    hasher.update(kind.as_bytes());
    hasher.update(b"|");
    hasher.update(parent.unwrap_or("").as_bytes());
    hex::encode(hasher.finalize())
}

/// Extract public-API contracts for a repo. A contract is a top-level
/// symbol (no parent) whose kind is one of the contract-eligible kinds
/// (function, struct, enum, trait, class, interface, type_alias, constant).
///
/// Existing contracts for the repo are replaced atomically. Returns counts
/// of newly extracted and removed contracts.
pub fn extract_contracts(
    data_dir: &Path,
    repo_id: i64,
) -> Result<ContractExtractResult, IndexError> {
    let mut conn = open_db(data_dir)?;
    let now = now_ms();

    let prior_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM code_contracts WHERE repo_id = ?1",
        params![repo_id],
        |r| r.get(0),
    )?;

    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM code_contracts WHERE repo_id = ?1",
        params![repo_id],
    )?;

    let mut inserted = 0usize;
    {
        let mut stmt = tx.prepare(
            "SELECT id, name, kind, file, line, parent
             FROM code_symbols
             WHERE repo_id = ?1
               AND parent IS NULL
               AND kind IN ('function','struct','enum','trait','class','interface','type_alias','constant')",
        )?;
        let rows = stmt.query_map(params![repo_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, Option<String>>(5)?,
            ))
        })?;
        let mut insert = tx.prepare(
            "INSERT INTO code_contracts
                 (repo_id, symbol_id, name, kind, file, line, signature_hash, extracted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        for row in rows {
            let (sid, name, kind, file, line, parent) = row?;
            let hash = signature_hash(&name, &kind, parent.as_deref());
            insert.execute(params![repo_id, sid, name, kind, file, line, hash, now])?;
            inserted += 1;
        }
    }
    tx.commit()?;

    Ok(ContractExtractResult {
        repo_id,
        extracted: inserted,
        removed: prior_count as usize,
    })
}

/// List all contracts for a group, ordered by repo label then symbol name.
pub fn list_group_contracts(
    data_dir: &Path,
    group_id: i64,
) -> Result<Vec<ContractEntry>, IndexError> {
    let conn = open_db(data_dir)?;
    let mut stmt = conn.prepare(
        "SELECT c.repo_id, r.label, c.symbol_id, c.name, c.kind, c.file, c.line, c.signature_hash
         FROM code_contracts c
         JOIN code_repos r ON r.id = c.repo_id
         JOIN code_repo_group_members m ON m.repo_id = c.repo_id
         WHERE m.group_id = ?1
         ORDER BY r.label ASC, c.name ASC",
    )?;
    let rows = stmt.query_map(params![group_id], |r| {
        Ok(ContractEntry {
            repo_id: r.get(0)?,
            repo_label: r.get(1)?,
            symbol_id: r.get(2)?,
            name: r.get(3)?,
            kind: r.get(4)?,
            file: r.get(5)?,
            line: r.get::<_, i64>(6)? as u32,
            signature_hash: r.get(7)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

// ─── Cross-repo query ───────────────────────────────────────────────────────

/// Search for a symbol name across all repos in a group. Returns matches
/// ordered by repo label then symbol name. The `is_contract` flag
/// indicates whether the matched symbol is part of the repo's extracted
/// public-API contract surface.
pub fn cross_repo_query(
    data_dir: &Path,
    group_id: i64,
    name_pattern: &str,
    limit: usize,
) -> Result<Vec<CrossRepoMatch>, IndexError> {
    let conn = open_db(data_dir)?;

    // Verify group exists for clearer error.
    let group_exists: bool = conn
        .query_row(
            "SELECT 1 FROM code_repo_groups WHERE id = ?1",
            params![group_id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !group_exists {
        return Err(IndexError::InvalidPath(format!(
            "group not found: {group_id}"
        )));
    }

    let limit = limit.clamp(1, 1000) as i64;
    let pattern = format!("%{}%", name_pattern.trim());

    let mut stmt = conn.prepare(
        "SELECT s.repo_id, r.label, s.id, s.name, s.kind, s.file, s.line,
                EXISTS(SELECT 1 FROM code_contracts c WHERE c.symbol_id = s.id) AS is_contract
         FROM code_symbols s
         JOIN code_repos r ON r.id = s.repo_id
         JOIN code_repo_group_members m ON m.repo_id = s.repo_id
         WHERE m.group_id = ?1 AND s.name LIKE ?2
         ORDER BY r.label ASC, s.name ASC
         LIMIT ?3",
    )?;
    let rows = stmt.query_map(params![group_id, pattern, limit], |r| {
        Ok(CrossRepoMatch {
            repo_id: r.get(0)?,
            repo_label: r.get(1)?,
            symbol_id: r.get(2)?,
            name: r.get(3)?,
            kind: r.get(4)?,
            file: r.get(5)?,
            line: r.get::<_, i64>(6)? as u32,
            is_contract: r.get::<_, i64>(7)? != 0,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use tempfile::TempDir;

    /// Insert a fake repo + a couple of symbols for testing without running
    /// the full tree-sitter indexing pipeline.
    fn seed_repo(data_dir: &Path, path: &str, label: &str) -> i64 {
        let conn = open_db(data_dir).unwrap();
        conn.execute(
            "INSERT INTO code_repos (path, label, indexed_at) VALUES (?1, ?2, ?3)",
            params![path, label, 1_000_000i64],
        )
        .unwrap();
        let repo_id = conn.last_insert_rowid();

        // Top-level public-ish symbols (eligible kinds + no parent).
        let symbols = [
            ("public_fn", "function", "src/lib.rs", 10i64, None::<&str>),
            ("PublicStruct", "struct", "src/lib.rs", 20i64, None),
            ("HelperTrait", "trait", "src/lib.rs", 30i64, None),
            // Method (has parent) — should NOT be a contract.
            (
                "internal_method",
                "method",
                "src/lib.rs",
                25i64,
                Some("PublicStruct"),
            ),
            // Top-level but disallowed kind (module) — should NOT be a contract.
            ("internal_mod", "module", "src/lib.rs", 1i64, None),
        ];
        for (name, kind, file, line, parent) in symbols {
            conn.execute(
                "INSERT INTO code_symbols (repo_id, name, kind, file, line, end_line, parent)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![repo_id, name, kind, file, line, line, parent],
            )
            .unwrap();
        }
        repo_id
    }

    #[test]
    fn create_and_list_groups_works() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path();

        let g1 = create_group(data_dir, "frontend-stack", Some("Vue + Pinia stack")).unwrap();
        let g2 = create_group(data_dir, "backend-stack", None).unwrap();

        assert!(g1.id > 0);
        assert_eq!(g1.label, "frontend-stack");
        assert_eq!(g1.description.as_deref(), Some("Vue + Pinia stack"));
        assert_eq!(g1.member_count, 0);

        let groups = list_groups(data_dir).unwrap();
        assert_eq!(groups.len(), 2);
        // Sorted alphabetically.
        assert_eq!(groups[0].label, "backend-stack");
        assert_eq!(groups[1].label, "frontend-stack");

        // Duplicate label rejected.
        assert!(create_group(data_dir, "frontend-stack", None).is_err());
        // Empty label rejected.
        assert!(create_group(data_dir, "  ", None).is_err());

        // Delete works.
        delete_group(data_dir, g2.id).unwrap();
        let groups = list_groups(data_dir).unwrap();
        assert_eq!(groups.len(), 1);
        assert!(delete_group(data_dir, 99999).is_err());
    }

    #[test]
    fn add_remove_member_idempotent() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path();
        let g = create_group(data_dir, "svc", None).unwrap();
        let r = seed_repo(data_dir, "/tmp/svc-a", "svc-a");

        add_repo_to_group(data_dir, g.id, r, Some("primary")).unwrap();
        // Re-adding updates role (no error).
        add_repo_to_group(data_dir, g.id, r, Some("backup")).unwrap();

        let status = group_status(data_dir, g.id).unwrap();
        assert_eq!(status.members.len(), 1);
        assert_eq!(status.members[0].role.as_deref(), Some("backup"));

        // Bad ids fail with clear error.
        assert!(add_repo_to_group(data_dir, 99999, r, None).is_err());
        assert!(add_repo_to_group(data_dir, g.id, 99999, None).is_err());

        remove_repo_from_group(data_dir, g.id, r).unwrap();
        let status = group_status(data_dir, g.id).unwrap();
        assert_eq!(status.members.len(), 0);
        assert!(remove_repo_from_group(data_dir, g.id, r).is_err());
    }

    #[test]
    fn extract_contracts_filters_by_kind_and_parent() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path();
        let r = seed_repo(data_dir, "/tmp/repo-x", "repo-x");

        let result = extract_contracts(data_dir, r).unwrap();
        // 3 eligible (public_fn, PublicStruct, HelperTrait).
        assert_eq!(result.extracted, 3);
        assert_eq!(result.removed, 0);

        // Re-extracting replaces atomically.
        let result2 = extract_contracts(data_dir, r).unwrap();
        assert_eq!(result2.extracted, 3);
        assert_eq!(result2.removed, 3);
    }

    #[test]
    fn signature_hash_detects_breaking_changes() {
        let h1 = signature_hash("foo", "function", None);
        let h2 = signature_hash("foo", "function", None);
        let h3 = signature_hash("foo", "method", Some("Bar"));
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn group_status_aggregates_counts() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path();
        let g = create_group(data_dir, "platform", None).unwrap();
        let ra = seed_repo(data_dir, "/tmp/a", "a");
        let rb = seed_repo(data_dir, "/tmp/b", "b");
        add_repo_to_group(data_dir, g.id, ra, Some("api")).unwrap();
        add_repo_to_group(data_dir, g.id, rb, Some("ui")).unwrap();
        extract_contracts(data_dir, ra).unwrap();
        extract_contracts(data_dir, rb).unwrap();

        let status = group_status(data_dir, g.id).unwrap();
        assert_eq!(status.group.member_count, 2);
        assert_eq!(status.members.len(), 2);
        // Each repo seeded with 5 symbols.
        assert_eq!(status.total_symbols, 10);
        // Each repo has 3 contracts (function/struct/trait).
        assert_eq!(status.total_contracts, 6);
        assert!(status.stalest_indexed_at.is_some());

        assert!(group_status(data_dir, 99999).is_err());
    }

    #[test]
    fn list_group_contracts_returns_member_repos_only() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path();
        let g = create_group(data_dir, "g", None).unwrap();
        let ra = seed_repo(data_dir, "/tmp/in", "in");
        let rb = seed_repo(data_dir, "/tmp/out", "out");
        add_repo_to_group(data_dir, g.id, ra, None).unwrap();
        extract_contracts(data_dir, ra).unwrap();
        extract_contracts(data_dir, rb).unwrap();

        let contracts = list_group_contracts(data_dir, g.id).unwrap();
        assert_eq!(contracts.len(), 3);
        assert!(contracts.iter().all(|c| c.repo_label == "in"));
    }

    #[test]
    fn cross_repo_query_searches_across_members() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path();
        let g = create_group(data_dir, "svc", None).unwrap();
        let ra = seed_repo(data_dir, "/tmp/svc-a", "svc-a");
        let rb = seed_repo(data_dir, "/tmp/svc-b", "svc-b");
        add_repo_to_group(data_dir, g.id, ra, None).unwrap();
        add_repo_to_group(data_dir, g.id, rb, None).unwrap();
        extract_contracts(data_dir, ra).unwrap();
        // rb has no contracts extracted → is_contract false for matches there.

        let matches = cross_repo_query(data_dir, g.id, "PublicStruct", 50).unwrap();
        // Two repos × one matching name "PublicStruct" each = 2.
        assert_eq!(matches.len(), 2);
        let svc_a = matches.iter().find(|m| m.repo_label == "svc-a").unwrap();
        let svc_b = matches.iter().find(|m| m.repo_label == "svc-b").unwrap();
        assert!(svc_a.is_contract);
        assert!(!svc_b.is_contract);

        // Limit clamped.
        let one = cross_repo_query(data_dir, g.id, "PublicStruct", 1).unwrap();
        assert_eq!(one.len(), 1);

        // Bad group fails.
        assert!(cross_repo_query(data_dir, 99999, "x", 10).is_err());
    }
}
