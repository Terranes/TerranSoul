//! Git pull-and-merge with optional LLM-assisted conflict resolution.
//!
//! On self-improve startup the engine pulls the latest `main` (or other
//! configured base) and fast-forward / 3-way merges it into the current
//! branch. When merge conflicts appear, the conflicted file contents are
//! handed to the configured Coding LLM with a strict prompt asking for
//! the final resolved file contents. The LLM's responses are written
//! back, `git add`-ed, and finally committed with a transparent message.
//!
//! Resilience: every failure path runs `git merge --abort` so the working
//! tree is left in a clean state. The function NEVER force-pushes, NEVER
//! discards uncommitted user work (it bails early when the tree is dirty),
//! and the LLM resolution path only writes files inside the repository.

use std::path::Path;
use std::process::Command;

use crate::brain::openai_client::OpenAiMessage;

use super::client::client_from;
use super::CodingLlmConfig;

/// Outcome of a `pull_main` attempt. `merged = true` means the working
/// tree was successfully advanced (or already up-to-date); the caller
/// should treat this as a success irrespective of whether the LLM was
/// invoked.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PullResult {
    pub merged: bool,
    pub fast_forward: bool,
    pub already_up_to_date: bool,
    /// Conflicted file paths the LLM resolved (empty on clean merges).
    pub resolved_conflicts: Vec<String>,
    /// Conflicted files the LLM could NOT resolve. Non-empty means the
    /// merge was aborted and the working tree is back to its pre-merge state.
    pub unresolved_conflicts: Vec<String>,
    pub message: String,
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("git not available: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return Err(if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        });
    }
    Ok(stdout.trim().to_string())
}

/// True when the working tree has no uncommitted changes.
pub fn working_tree_clean(repo_root: &Path) -> bool {
    run_git(repo_root, &["status", "--porcelain"])
        .map(|s| s.is_empty())
        .unwrap_or(false)
}

/// Currently checked-out branch name (or `None` for detached HEAD).
pub fn current_branch(repo_root: &Path) -> Option<String> {
    run_git(repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()
        .filter(|b| b != "HEAD")
}

/// List of files in the unmerged (conflict) state.
pub fn conflicted_files(repo_root: &Path) -> Vec<String> {
    run_git(repo_root, &["diff", "--name-only", "--diff-filter=U"])
        .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default()
}

/// Pull and merge the latest `base` (e.g. `"main"`) from `origin` into
/// the current branch. When `coding_llm` is `Some`, conflicts are
/// resolved through the LLM; otherwise conflicts cause `merge --abort`
/// and the function returns with `unresolved_conflicts` populated.
///
/// Refuses to operate on a dirty working tree — returns a `PullResult`
/// with `merged = false` and an explanatory message instead of stashing
/// the user's work (we never silently move user changes around).
pub async fn pull_main(
    repo_root: &Path,
    base: &str,
    coding_llm: Option<&CodingLlmConfig>,
) -> PullResult {
    if !working_tree_clean(repo_root) {
        return PullResult {
            merged: false,
            fast_forward: false,
            already_up_to_date: false,
            resolved_conflicts: vec![],
            unresolved_conflicts: vec![],
            message: "Working tree not clean — refusing to pull. Commit or stash first."
                .to_string(),
        };
    }

    if let Err(e) = run_git(repo_root, &["fetch", "origin", base]) {
        return PullResult {
            merged: false,
            fast_forward: false,
            already_up_to_date: false,
            resolved_conflicts: vec![],
            unresolved_conflicts: vec![],
            message: format!("git fetch origin {base} failed: {e}"),
        };
    }

    // Try a non-interactive merge. `--no-edit` avoids invoking $EDITOR.
    let target = format!("origin/{base}");
    match run_git(repo_root, &["merge", "--no-edit", "--no-ff", &target]) {
        Ok(out) => {
            let already_up_to_date = out.contains("Already up to date");
            let fast_forward = out.contains("Fast-forward");
            PullResult {
                merged: true,
                fast_forward,
                already_up_to_date,
                resolved_conflicts: vec![],
                unresolved_conflicts: vec![],
                message: if already_up_to_date {
                    format!("Already up to date with origin/{base}.")
                } else {
                    format!("Merged origin/{base} cleanly.")
                },
            }
        }
        Err(merge_err) => {
            let conflicts = conflicted_files(repo_root);
            if conflicts.is_empty() {
                // Merge failed for a reason other than conflicts (e.g. no
                // such ref). Make sure the tree is clean.
                let _ = run_git(repo_root, &["merge", "--abort"]);
                return PullResult {
                    merged: false,
                    fast_forward: false,
                    already_up_to_date: false,
                    resolved_conflicts: vec![],
                    unresolved_conflicts: vec![],
                    message: format!("git merge failed: {merge_err}"),
                };
            }
            let Some(cfg) = coding_llm else {
                let _ = run_git(repo_root, &["merge", "--abort"]);
                return PullResult {
                    merged: false,
                    fast_forward: false,
                    already_up_to_date: false,
                    resolved_conflicts: vec![],
                    unresolved_conflicts: conflicts,
                    message: "Conflicts detected and no Coding LLM available — \
                              merge aborted to keep tree clean."
                        .to_string(),
                };
            };
            // LLM-assisted resolution.
            match resolve_conflicts_with_llm(repo_root, &conflicts, cfg).await {
                Ok(resolved) => {
                    // Commit only when at least one file was resolved.
                    let commit_msg = format!(
                        "self-improve: resolve merge conflicts via coding LLM ({} files)",
                        resolved.len()
                    );
                    if let Err(e) = run_git(repo_root, &["commit", "-m", &commit_msg]) {
                        let _ = run_git(repo_root, &["merge", "--abort"]);
                        return PullResult {
                            merged: false,
                            fast_forward: false,
                            already_up_to_date: false,
                            resolved_conflicts: vec![],
                            unresolved_conflicts: conflicts,
                            message: format!("LLM resolved files but git commit failed: {e}"),
                        };
                    }
                    PullResult {
                        merged: true,
                        fast_forward: false,
                        already_up_to_date: false,
                        resolved_conflicts: resolved,
                        unresolved_conflicts: vec![],
                        message: format!(
                            "Merged origin/{base} with LLM-assisted conflict resolution."
                        ),
                    }
                }
                Err(e) => {
                    let _ = run_git(repo_root, &["merge", "--abort"]);
                    PullResult {
                        merged: false,
                        fast_forward: false,
                        already_up_to_date: false,
                        resolved_conflicts: vec![],
                        unresolved_conflicts: conflicts,
                        message: format!("LLM conflict resolution failed: {e}"),
                    }
                }
            }
        }
    }
}

/// Send each conflicted file to the Coding LLM and write back the
/// resolved contents. Returns the list of files that were successfully
/// resolved. On the first LLM error this returns `Err`; the caller is
/// expected to `merge --abort`.
async fn resolve_conflicts_with_llm(
    repo_root: &Path,
    files: &[String],
    cfg: &CodingLlmConfig,
) -> Result<Vec<String>, String> {
    let client = client_from(cfg);
    let mut resolved = Vec::with_capacity(files.len());
    for relpath in files {
        let abs = repo_root.join(relpath);
        let raw = std::fs::read_to_string(&abs)
            .map_err(|e| format!("read conflicted file {relpath}: {e}"))?;
        let prompt = build_conflict_prompt(relpath, &raw);
        let reply = client
            .chat(prompt)
            .await
            .map_err(|e| format!("LLM chat for {relpath}: {e}"))?;
        let cleaned = strip_code_fence(&reply);
        if cleaned.contains("<<<<<<<") || cleaned.contains(">>>>>>>") {
            return Err(format!(
                "LLM reply for {relpath} still contains conflict markers"
            ));
        }
        std::fs::write(&abs, cleaned).map_err(|e| format!("write {relpath}: {e}"))?;
        run_git(repo_root, &["add", relpath]).map_err(|e| format!("git add {relpath}: {e}"))?;
        resolved.push(relpath.clone());
    }
    Ok(resolved)
}

fn build_conflict_prompt(path: &str, raw: &str) -> Vec<OpenAiMessage> {
    let system = "You are a careful merge-conflict resolver. The user will \
                  give you a single source file containing Git conflict \
                  markers (<<<<<<<, =======, >>>>>>>). Produce ONLY the \
                  fully resolved file contents — no explanation, no \
                  Markdown fences. Preserve both sides' intent when \
                  possible; when in doubt, prefer the upstream (incoming) \
                  side. Never leave conflict markers in your output.";
    let user = format!(
        "File: {path}\n\nConflicted contents follow between <<<FILE>>>:\n\n<<<FILE>>>\n{raw}\n<<<FILE>>>"
    );
    vec![
        OpenAiMessage { role: "system".to_string(), content: system.to_string() },
        OpenAiMessage { role: "user".to_string(), content: user },
    ]
}

/// Strip a single ```lang … ``` fence the LLM may have wrapped its
/// answer in. Leaves un-fenced text alone.
fn strip_code_fence(reply: &str) -> String {
    let trimmed = reply.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    // Drop the opening fence line.
    let after_open = trimmed.split_once('\n').map(|x| x.1).unwrap_or("");
    // Drop the trailing closing fence (last ``` line).
    if let Some(idx) = after_open.rfind("```") {
        let inner = &after_open[..idx];
        return inner.trim_end().to_string();
    }
    after_open.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_code_fence_unwraps_typical_replies() {
        let s = "```rust\nfn main() {}\n```";
        assert_eq!(strip_code_fence(s), "fn main() {}");

        let unfenced = "fn main() {}\n";
        assert_eq!(strip_code_fence(unfenced), "fn main() {}");

        let with_lang = "```\nplain\n```";
        assert_eq!(strip_code_fence(with_lang), "plain");
    }

    #[test]
    fn build_conflict_prompt_includes_path_and_markers() {
        let raw = "<<<<<<< HEAD\nA\n=======\nB\n>>>>>>> incoming\n";
        let msgs = build_conflict_prompt("src/foo.rs", raw);
        assert_eq!(msgs.len(), 2);
        assert!(msgs[1].content.contains("src/foo.rs"));
        assert!(msgs[1].content.contains("<<<<<<<"));
    }

    /// Smoke test: in a clean tempdir that is NOT a git repo, `pull_main`
    /// fails fast with a helpful message rather than crashing.
    #[tokio::test]
    async fn pull_main_fails_gracefully_outside_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        let result = pull_main(dir.path(), "main", None).await;
        assert!(!result.merged);
        assert!(
            result.message.contains("not clean")
                || result.message.contains("fetch")
                || result.message.contains("git"),
            "unexpected message: {}",
            result.message
        );
    }

    /// Build a real local git repo, simulate divergence + a conflict, and
    /// verify `pull_main` aborts cleanly when no Coding LLM is supplied.
    #[tokio::test]
    async fn pull_main_aborts_on_conflict_without_llm() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Set up a "remote" bare repo + a working clone.
        let remote = root.join("remote.git");
        std::fs::create_dir_all(&remote).unwrap();
        let _ = run_git(&remote, &["init", "--bare"]).unwrap();
        // Bare repos default HEAD to refs/heads/master regardless of the
        // host's `init.defaultBranch`; point it at main so subsequent
        // clones land on `main` automatically.
        let _ = run_git(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"]).unwrap();

        let work = root.join("work");
        std::fs::create_dir_all(&work).unwrap();
        let _ = run_git(&work, &["init"]).unwrap();
        let _ = run_git(&work, &["config", "user.email", "t@t.t"]).unwrap();
        let _ = run_git(&work, &["config", "user.name", "t"]).unwrap();
        let _ = run_git(&work, &["config", "commit.gpgsign", "false"]);
        std::fs::write(work.join("a.txt"), "first\n").unwrap();
        let _ = run_git(&work, &["add", "a.txt"]).unwrap();
        let _ = run_git(&work, &["commit", "-m", "init"]).unwrap();
        // Normalise the initial branch to `main` regardless of the host
        // git's `init.defaultBranch` setting.
        let _ = run_git(&work, &["branch", "-M", "main"]).unwrap();
        let remote_url = remote.to_string_lossy().to_string();
        let _ = run_git(&work, &["remote", "add", "origin", &remote_url]).unwrap();
        let _ = run_git(&work, &["push", "-u", "origin", "main"]).unwrap();

        // Diverge: create a feature branch, edit a.txt one way.
        let _ = run_git(&work, &["checkout", "-b", "feature"]).unwrap();
        std::fs::write(work.join("a.txt"), "feature side\n").unwrap();
        let _ = run_git(&work, &["commit", "-am", "feature edit"]).unwrap();

        // Meanwhile, on `main` (via a second clone), edit a.txt differently.
        let other = root.join("other");
        let _ = run_git(root, &["clone", &remote_url, "other"]).unwrap();
        let _ = run_git(&other, &["config", "user.email", "o@o.o"]).unwrap();
        let _ = run_git(&other, &["config", "user.name", "o"]).unwrap();
        std::fs::write(other.join("a.txt"), "main side\n").unwrap();
        let _ = run_git(&other, &["commit", "-am", "main edit"]).unwrap();
        let _ = run_git(&other, &["push", "origin", "main"]).unwrap();

        // Now pull main into feature — should conflict and abort.
        let result = pull_main(&work, "main", None).await;
        assert!(!result.merged, "expected merge to fail: {result:?}");
        assert!(
            !result.unresolved_conflicts.is_empty(),
            "expected conflicts: {result:?}"
        );
        // After abort the tree is clean again.
        assert!(working_tree_clean(&work));
    }
}
