//! Branch-sync orchestration: git diff → overlay re-index (Chunk 45.2).
//!
//! Bridges `git diff` output with [`branch_overlay::branch_sync`] by:
//! 1. Running `git diff --name-only prev..new` to get changed files.
//! 2. Reading file contents at `new` (or from working tree).
//! 3. Calling the overlay sync with the collected data.

use std::path::Path;
use std::process::Command;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::branch_overlay::{self, BranchSyncResult};
use super::symbol_index::IndexError;

// ─── Types ──────────────────────────────────────────────────────────────────

/// Result from `execute_index_commit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCommitResult {
    pub commit: String,
    pub sync_result: Option<BranchSyncResult>,
    pub promoted_to_base: bool,
    pub message: String,
}

// ─── Public API ─────────────────────────────────────────────────────────────

/// Execute a branch sync: detect changed files via git, read their contents,
/// and sync the overlay.
pub fn execute_branch_sync(
    conn: &Connection,
    repo_path: &Path,
    _data_dir: &Path,
    prev_ref: &str,
    new_ref: &str,
) -> Result<BranchSyncResult, IndexError> {
    let repo_id = resolve_repo_id(conn, repo_path)?;

    // Get the list of changed files.
    let changed_files = git_diff_name_only(repo_path, prev_ref, new_ref)?;

    if changed_files.is_empty() {
        return Ok(BranchSyncResult {
            base_ref: prev_ref.to_string(),
            branch_ref: new_ref.to_string(),
            files_reindexed: 0,
            files_removed: 0,
            files_unchanged: 0,
            symbols_added: 0,
            edges_added: 0,
        });
    }

    // Read file contents from the working tree (post-checkout, files are on disk).
    let file_contents = read_file_contents(repo_path, &changed_files);

    branch_overlay::branch_sync(
        conn,
        repo_id,
        prev_ref,
        new_ref,
        &changed_files,
        &file_contents,
    )
}

/// Execute an index-commit: re-index files changed in the latest commit.
///
/// If the commit's parent equals the overlay's base_ref, the overlay is
/// promoted to base (overlay rows are absorbed).
pub fn execute_index_commit(
    conn: &Connection,
    repo_path: &Path,
    _data_dir: &Path,
    commit_sha: &str,
) -> Result<IndexCommitResult, IndexError> {
    let repo_id = resolve_repo_id(conn, repo_path)?;

    // Get files changed in this commit vs its parent.
    let parent = git_rev_parse(repo_path, &format!("{commit_sha}^")).unwrap_or_default();

    if parent.is_empty() {
        return Ok(IndexCommitResult {
            commit: commit_sha.to_string(),
            sync_result: None,
            promoted_to_base: false,
            message: "Could not resolve parent commit; skipping.".to_string(),
        });
    }

    let changed_files = git_diff_name_only(repo_path, &parent, commit_sha)?;
    let file_contents = read_file_contents(repo_path, &changed_files);

    // Check if there's an active overlay where base_ref == parent.
    let has_overlay: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM code_branch_overlays WHERE repo_id = ?1 AND base_ref = ?2",
            params![repo_id, parent],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if has_overlay {
        // Update existing overlay.
        let sync_result = branch_overlay::branch_sync(
            conn,
            repo_id,
            &parent,
            commit_sha,
            &changed_files,
            &file_contents,
        )?;

        // Check if HEAD now equals the main branch (promote to base).
        let main_ref = git_symbolic_ref(repo_path).unwrap_or_default();
        let promoted = if !main_ref.is_empty() {
            let main_sha = git_rev_parse(repo_path, &main_ref).unwrap_or_default();
            if main_sha == commit_sha {
                // Promote: delete overlay (symbols already in base from full re-index).
                branch_overlay::delete_branch_overlay(conn, repo_id, &parent, commit_sha)?;
                true
            } else {
                false
            }
        } else {
            false
        };

        Ok(IndexCommitResult {
            commit: commit_sha.to_string(),
            sync_result: Some(sync_result),
            promoted_to_base: promoted,
            message: if promoted {
                "Overlay promoted to base (HEAD matches main).".to_string()
            } else {
                "Overlay updated with commit changes.".to_string()
            },
        })
    } else {
        // No active overlay — just sync as new overlay.
        let sync_result = branch_overlay::branch_sync(
            conn,
            repo_id,
            &parent,
            commit_sha,
            &changed_files,
            &file_contents,
        )?;

        Ok(IndexCommitResult {
            commit: commit_sha.to_string(),
            sync_result: Some(sync_result),
            promoted_to_base: false,
            message: "Created new overlay for commit.".to_string(),
        })
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Resolve repo_id from a path.
fn resolve_repo_id(conn: &Connection, repo_path: &Path) -> Result<i64, IndexError> {
    let path_str = repo_path.to_string_lossy().to_string();
    conn.query_row(
        "SELECT id FROM code_repos WHERE path = ?1",
        params![path_str],
        |r| r.get(0),
    )
    .map_err(|_| {
        IndexError::InvalidPath(format!(
            "repo not indexed: {}. Run code_query first.",
            path_str
        ))
    })
}

/// Run `git diff --name-only prev..new` in the repo.
fn git_diff_name_only(repo_path: &Path, prev: &str, new: &str) -> Result<Vec<String>, IndexError> {
    let output = Command::new("git")
        .args(["diff", "--name-only", &format!("{prev}..{new}")])
        .current_dir(repo_path)
        .output()
        .map_err(IndexError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(IndexError::InvalidPath(format!(
            "git diff failed: {stderr}"
        )));
    }

    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    Ok(files)
}

/// Run `git rev-parse <ref>` in the repo.
fn git_rev_parse(repo_path: &Path, reference: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", reference])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Get the default branch symbolic ref (e.g. "main" or "master").
fn git_symbolic_ref(repo_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        // Fallback: try main then master.
        for branch in &["main", "master"] {
            let check = Command::new("git")
                .args(["rev-parse", "--verify", branch])
                .current_dir(repo_path)
                .output()
                .ok()?;
            if check.status.success() {
                return Some(branch.to_string());
            }
        }
        None
    }
}

/// Read file contents from the working tree for files that exist.
fn read_file_contents(repo_path: &Path, files: &[String]) -> Vec<(String, Vec<u8>)> {
    files
        .iter()
        .filter_map(|rel_path| {
            let full_path = repo_path.join(rel_path);
            std::fs::read(&full_path)
                .ok()
                .map(|bytes| (rel_path.clone(), bytes))
        })
        .collect()
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_file_contents_skips_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("exists.rs"), b"fn hello() {}").unwrap();

        let files = vec!["exists.rs".to_string(), "missing.rs".to_string()];
        let result = read_file_contents(dir.path(), &files);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "exists.rs");
    }
}
