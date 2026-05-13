//! Repository binding for the self-improve loop.
//!
//! Detects whether a directory is inside a git working tree, and exposes a
//! tiny helper for creating + switching to a feature branch. Runs through
//! the system `git` binary via [`std::process::Command`] — chosen over a
//! native crate (`git2`) to avoid pulling in libgit2/openssl as new build
//! dependencies. The autonomous loop only ever runs short, well-known
//! commands here, so the shell-out approach is acceptable.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Snapshot of the repo's current state. `None` for any field means the
/// information could not be determined (e.g. detached HEAD, no upstream).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RepoState {
    pub is_git_repo: bool,
    pub root: Option<String>,
    pub current_branch: Option<String>,
    pub remote_url: Option<String>,
    pub clean: bool,
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let cmdline = if args.is_empty() {
        "git".to_string()
    } else {
        format!("git {}", args.join(" "))
    };
    super::sandbox::shell_preflight(cwd, &cmdline)?;

    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("git not available: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Inspect the directory `cwd` and return what we can learn about its git state.
/// Always returns `Ok` with `is_git_repo = false` when the directory is not
/// inside a git working tree — this is informational, not an error.
pub fn detect_repo(cwd: &Path) -> RepoState {
    let inside = run_git(cwd, &["rev-parse", "--is-inside-work-tree"])
        .map(|s| s == "true")
        .unwrap_or(false);
    if !inside {
        return RepoState {
            is_git_repo: false,
            root: None,
            current_branch: None,
            remote_url: None,
            clean: false,
        };
    }
    let root = run_git(cwd, &["rev-parse", "--show-toplevel"]).ok();
    let current_branch = run_git(cwd, &["rev-parse", "--abbrev-ref", "HEAD"]).ok();
    let remote_url = run_git(cwd, &["remote", "get-url", "origin"]).ok();
    // `git status --porcelain` returns empty when the tree is clean.
    let clean = run_git(cwd, &["status", "--porcelain"])
        .map(|s| s.is_empty())
        .unwrap_or(false);
    RepoState {
        is_git_repo: true,
        root,
        current_branch,
        remote_url,
        clean,
    }
}

/// Sanitise a chunk identifier into a safe branch name segment.
///
/// Allows `[A-Za-z0-9._-]`; replaces every other byte with `-`, then
/// collapses runs of `-` and trims leading/trailing dashes.
pub fn sanitize_branch_segment(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut last_dash = false;
    for ch in raw.chars() {
        let allowed = ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-';
        if allowed {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "chunk".to_string()
    } else {
        trimmed
    }
}

/// Build the canonical feature-branch name for an autonomous-loop chunk.
pub fn feature_branch_name(chunk_id: &str) -> String {
    format!(
        "terransoul/self-improve/{}",
        sanitize_branch_segment(chunk_id)
    )
}

/// Best-effort: locate the workspace root for the autonomous loop.
///
/// In production the loop should run against the user's TerranSoul checkout.
/// We start from the data dir's parent and walk upward looking for
/// `Cargo.toml` adjacent to `src-tauri/` — characteristic of this repo. If
/// nothing is found, fall back to the data dir itself (callers will detect
/// `is_git_repo = false`).
pub fn guess_repo_root(start: &Path) -> PathBuf {
    let mut cur = start.to_path_buf();
    for _ in 0..6 {
        if cur.join("src-tauri").join("Cargo.toml").exists() {
            return cur;
        }
        if !cur.pop() {
            break;
        }
    }
    start.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_unsafe_chars_and_collapses_dashes() {
        assert_eq!(
            sanitize_branch_segment("Chunk 25.4 — MVP!"),
            "Chunk-25.4-MVP"
        );
        assert_eq!(sanitize_branch_segment("///"), "chunk");
        assert_eq!(sanitize_branch_segment(""), "chunk");
        assert_eq!(sanitize_branch_segment("a/b\\c d"), "a-b-c-d");
    }

    #[test]
    fn feature_branch_uses_canonical_prefix() {
        let b = feature_branch_name("25.4");
        assert!(b.starts_with("terransoul/self-improve/"));
        assert!(b.ends_with("25.4"));
    }

    #[test]
    fn detect_repo_returns_not_a_repo_outside_git() {
        let dir = tempfile::tempdir().unwrap();
        let state = detect_repo(dir.path());
        assert!(!state.is_git_repo);
        assert!(state.root.is_none());
    }

    /// Smoke test: when run from this crate's own source tree the engine
    /// must successfully detect the TerranSoul git repo. Skipped silently
    /// if the test runner is operating outside a git checkout (e.g. a
    /// vendored build) so this never causes spurious CI failures.
    #[test]
    fn detect_repo_finds_terransoul_checkout() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // CARGO_MANIFEST_DIR points at src-tauri/. The git repo lives one
        // level above that.
        let workspace_root = manifest_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(manifest_dir);
        let state = detect_repo(&workspace_root);
        if !state.is_git_repo {
            // Running outside a git checkout (e.g. crate published to a
            // vendor directory). Don't fail — just exit early.
            return;
        }
        assert!(state.root.is_some(), "repo root should be populated");
        assert!(
            state.current_branch.is_some(),
            "current branch should be populated"
        );
    }

    /// Verify guess_repo_root walks upward from a deep starting point.
    #[test]
    fn guess_repo_root_walks_upward_to_find_workspace() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // Start deep inside the crate; the helper should walk up to the
        // workspace root that contains src-tauri/Cargo.toml.
        let deep = manifest_dir.join("src").join("coding");
        let root = guess_repo_root(&deep);
        assert!(
            root.join("src-tauri").join("Cargo.toml").exists(),
            "guess_repo_root should locate the workspace; got {root:?}"
        );
    }
}
