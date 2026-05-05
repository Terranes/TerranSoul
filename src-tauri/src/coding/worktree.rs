//! Temporary git worktrees for isolated autonomous coding execution.
//!
//! The self-improve engine can use this when the user's checkout is dirty:
//! generated edits and test runs happen in a detached worktree, then a patch is
//! captured for review while the user's working tree is left untouched.

use std::path::{Path, PathBuf};
use std::process::Command;

use uuid::Uuid;

use super::repo::sanitize_branch_segment;

#[derive(Debug)]
pub struct TemporaryWorktree {
    original_root: PathBuf,
    path: PathBuf,
    cleaned: bool,
}

impl TemporaryWorktree {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn cached_diff(&self) -> Result<String, String> {
        run_git_raw_stdout(
            &self.path,
            &["diff", "--cached", "--binary", "--no-ext-diff", "--"],
        )
    }

    pub fn cleanup(&mut self) -> Result<(), String> {
        if self.cleaned {
            return Ok(());
        }

        let path_arg = self.path.to_string_lossy().to_string();
        let remove_result = run_git(
            &self.original_root,
            &["worktree", "remove", "--force", &path_arg],
        );
        let prune_result = run_git(&self.original_root, &["worktree", "prune"]);

        self.cleaned = true;

        if remove_result.is_err() && self.path.exists() {
            std::fs::remove_dir_all(&self.path)
                .map_err(|error| format!("remove temporary worktree fallback: {error}"))?;
        }

        prune_result.map(|_| ())
    }
}

impl Drop for TemporaryWorktree {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

pub fn create_temporary_worktree(
    repo_root: &Path,
    chunk_id: &str,
) -> Result<TemporaryWorktree, String> {
    create_worktree_in(repo_root, chunk_id, None)
}

/// Create a temporary worktree, optionally in a custom base directory.
///
/// When `base_dir` is `Some(path)`, the worktree is created under that
/// directory (e.g. `D:\Git\TerranSoul-worktrees\`). When `None`, uses
/// the OS temp directory.
///
/// Users can inspect the worktree with:
/// ```sh
/// git worktree list              # from the main repo
/// cd <worktree-path>             # open in terminal
/// # Or open in GitHub Desktop: File → Add Local Repository → <worktree-path>
/// ```
pub fn create_worktree_in(
    repo_root: &Path,
    chunk_id: &str,
    base_dir: Option<&Path>,
) -> Result<TemporaryWorktree, String> {
    ensure_git_repo(repo_root)?;
    let sanitized_chunk = sanitize_branch_segment(chunk_id);
    let base = match base_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir).map_err(|e| format!("create worktree base dir: {e}"))?;
            dir.to_path_buf()
        }
        None => std::env::temp_dir(),
    };
    let path = base.join(format!(
        "terransoul-self-improve-{sanitized_chunk}-{}",
        Uuid::new_v4().simple()
    ));
    if path.exists() {
        return Err(format!(
            "temporary worktree path already exists: {}",
            path.display()
        ));
    }

    let path_arg = path.to_string_lossy().to_string();
    run_git(
        repo_root,
        &["worktree", "add", "--detach", &path_arg, "HEAD"],
    )?;

    Ok(TemporaryWorktree {
        original_root: repo_root.to_path_buf(),
        path,
        cleaned: false,
    })
}

fn ensure_git_repo(repo_root: &Path) -> Result<(), String> {
    run_git(repo_root, &["rev-parse", "--is-inside-work-tree"])
        .and_then(|inside| {
            if inside == "true" {
                Ok(())
            } else {
                Err("not inside a git working tree".to_string())
            }
        })
        .map_err(|error| format!("temporary worktree unavailable: {error}"))
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    run_git_output(cwd, args, true)
}

fn run_git_raw_stdout(cwd: &Path, args: &[&str]) -> Result<String, String> {
    run_git_output(cwd, args, false)
}

fn run_git_output(cwd: &Path, args: &[&str], trim_stdout: bool) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("git not available: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout = if trim_stdout {
        stdout.trim().to_string()
    } else {
        stdout.to_string()
    };
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        return Ok(stdout);
    }
    Err(if stderr.is_empty() { stdout } else { stderr })
}

/// Information about a git worktree, as returned by `git worktree list`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorktreeInfo {
    /// Absolute path to the worktree directory.
    pub path: String,
    /// The HEAD commit hash (short form).
    pub head: String,
    /// Branch name or "(detached)" / "(bare)".
    pub branch: String,
}

/// List all git worktrees for the repository at `repo_root`.
///
/// Returns a list of `WorktreeInfo` entries. The first entry is always
/// the main working tree. Self-improve worktrees appear as detached HEAD.
///
/// Users can use these paths to:
/// - Open in GitHub Desktop: File → Add Local Repository → paste the path
/// - Open in VS Code: `code <path>`
/// - Browse in terminal: `cd <path>`
pub fn list_worktrees(repo_root: &Path) -> Result<Vec<WorktreeInfo>, String> {
    let output = run_git(repo_root, &["worktree", "list", "--porcelain"])?;
    let mut worktrees = Vec::new();
    let mut current_path = String::new();
    let mut current_head = String::new();
    let mut current_branch = String::new();

    for line in output.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            if !current_path.is_empty() {
                worktrees.push(WorktreeInfo {
                    path: std::mem::take(&mut current_path),
                    head: std::mem::take(&mut current_head),
                    branch: std::mem::take(&mut current_branch),
                });
            }
            current_path = p.to_string();
        } else if let Some(h) = line.strip_prefix("HEAD ") {
            current_head = h[..h.len().min(8)].to_string();
        } else if let Some(b) = line.strip_prefix("branch ") {
            current_branch = b.strip_prefix("refs/heads/").unwrap_or(b).to_string();
        } else if line == "detached" {
            current_branch = "(detached)".to_string();
        } else if line == "bare" {
            current_branch = "(bare)".to_string();
        }
    }
    if !current_path.is_empty() {
        worktrees.push(WorktreeInfo {
            path: current_path,
            head: current_head,
            branch: current_branch,
        });
    }

    Ok(worktrees)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporary_worktree_isolates_dirty_checkout_and_captures_patch() {
        let repo_dir = tempfile::tempdir().unwrap();
        let repo_root = repo_dir.path();
        init_repo(repo_root);

        std::fs::write(repo_root.join("tracked.txt"), "user dirty change\n").unwrap();

        let mut worktree = create_temporary_worktree(repo_root, "28.13").unwrap();
        assert!(worktree.path().exists());
        assert_eq!(
            normalize_newlines(
                &std::fs::read_to_string(worktree.path().join("tracked.txt")).unwrap()
            ),
            "base\n"
        );

        std::fs::write(worktree.path().join("tracked.txt"), "isolated change\n").unwrap();
        run_git(worktree.path(), &["add", "tracked.txt"]).unwrap();
        let patch = worktree.cached_diff().unwrap();

        assert!(patch.contains("isolated change"));
        assert!(patch.ends_with('\n'));
        assert_eq!(
            std::fs::read_to_string(repo_root.join("tracked.txt")).unwrap(),
            "user dirty change\n"
        );

        let worktree_path = worktree.path().to_path_buf();
        worktree.cleanup().unwrap();
        assert!(!worktree_path.exists());
    }

    #[test]
    fn create_temporary_worktree_rejects_non_repo() {
        let repo_dir = tempfile::tempdir().unwrap();
        let error = create_temporary_worktree(repo_dir.path(), "28.13").unwrap_err();
        assert!(error.contains("temporary worktree unavailable"));
    }

    fn init_repo(repo_root: &Path) {
        run_git(repo_root, &["init"]).unwrap();
        run_git(repo_root, &["config", "user.email", "test@example.com"]).unwrap();
        run_git(repo_root, &["config", "user.name", "TerranSoul Test"]).unwrap();
        run_git(repo_root, &["config", "commit.gpgsign", "false"]).unwrap();
        std::fs::write(repo_root.join("tracked.txt"), "base\n").unwrap();
        run_git(repo_root, &["add", "tracked.txt"]).unwrap();
        run_git(repo_root, &["commit", "-m", "init"]).unwrap();
    }

    fn normalize_newlines(contents: &str) -> String {
        contents.replace("\r\n", "\n")
    }
}
