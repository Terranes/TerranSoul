//! Apply LLM-produced file-write blocks to the working tree
//! (Chunk 25.10 — `apply_file`).
//!
//! The coding workflow asks the LLM to produce zero or more
//! `<file path="relative/path.ext">…content…</file>` blocks. This
//! module:
//!
//! 1. Parses those blocks from the raw reply.
//! 2. Validates that every path stays within the repo root (rejects
//!    traversal, absolute paths, symlink tricks).
//! 3. Writes each file atomically (temp + rename) so a mid-write crash
//!    cannot leave a torn file on disk.
//! 4. Stages the written files via `git add` (optional).
//! 5. Returns a summary suitable for the progress event stream.
//!
//! ## Security contract
//!
//! - No path component may be `..`.
//! - Absolute paths are rejected.
//! - The resolved canonical path must reside under the repo root.
//! - Existing `.git/` contents are never writable.
//!
//! These checks make the module safe to invoke on untrusted LLM output.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

/// A single file block parsed from the model's reply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileBlock {
    /// Relative path inside the repository (e.g. `src-tauri/src/foo.rs`).
    pub path: String,
    /// Full file contents to write (UTF-8 only).
    pub content: String,
}

/// Outcome of applying one file block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    pub path: String,
    pub written_bytes: usize,
    pub created: bool,
    pub git_added: bool,
}

/// Aggregate outcome of an apply-all pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplySummary {
    pub applied: Vec<ApplyResult>,
    pub rejected: Vec<ApplyRejection>,
}

/// A file block that was rejected by the security validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyRejection {
    pub path: String,
    pub reason: String,
}

// ── Parsing ─────────────────────────────────────────────────────────

/// Parse `<file path="…">…</file>` blocks from raw LLM output.
///
/// Handles:
/// - `<file path="path">content</file>`
/// - `<file path='path'>content</file>`
/// - Any amount of whitespace around the path attribute.
///
/// Stops at the first `</file>` for each opening tag. Blocks with no
/// `path` attribute or empty content are silently skipped. This is
/// intentionally lenient — the model sometimes adds commentary
/// outside the tags that we discard.
pub fn parse_file_blocks(reply: &str) -> Vec<FileBlock> {
    let mut blocks = Vec::new();
    let mut cursor = 0;

    while cursor < reply.len() {
        // Find next <file opening.
        let open_start = match reply[cursor..].find("<file ") {
            Some(pos) => cursor + pos,
            None => break,
        };
        // Find the end of the opening tag (`>`).
        let tag_end = match reply[open_start..].find('>') {
            Some(pos) => open_start + pos,
            None => break,
        };
        let tag_text = &reply[open_start..tag_end];
        // Extract path attribute value.
        let path = match extract_path_attr(tag_text) {
            Some(p) => p,
            None => {
                cursor = tag_end + 1;
                continue;
            }
        };
        // Content starts right after `>`.
        let content_start = tag_end + 1;
        // Find closing tag.
        let close_tag = "</file>";
        let close_start = match reply[content_start..].find(close_tag) {
            Some(pos) => content_start + pos,
            None => break,
        };
        let content = &reply[content_start..close_start];
        if !path.is_empty() {
            blocks.push(FileBlock {
                path: path.to_string(),
                content: content.to_string(),
            });
        }
        cursor = close_start + close_tag.len();
    }

    blocks
}

/// Pull the `path="…"` or `path='…'` attribute from a tag opener.
fn extract_path_attr(tag: &str) -> Option<&str> {
    let prefix_dq = "path=\"";
    let prefix_sq = "path='";
    if let Some(idx) = tag.find(prefix_dq) {
        let start = idx + prefix_dq.len();
        let end = tag[start..].find('"')? + start;
        return Some(&tag[start..end]);
    }
    if let Some(idx) = tag.find(prefix_sq) {
        let start = idx + prefix_sq.len();
        let end = tag[start..].find('\'')? + start;
        return Some(&tag[start..end]);
    }
    None
}

// ── Validation ──────────────────────────────────────────────────────

/// Validate that `rel_path` is safe to write under `repo_root`.
/// Returns the resolved absolute path on success.
pub fn validate_path(repo_root: &Path, rel_path: &str) -> Result<PathBuf, String> {
    let rel_path = rel_path.trim();
    // Reject absolute paths.
    if Path::new(rel_path).is_absolute() || rel_path.starts_with('/') || rel_path.starts_with('\\')
    {
        return Err(format!("absolute path rejected: `{rel_path}`"));
    }
    // Reject path components that try to escape.
    for component in rel_path.split(['/', '\\']) {
        if component == ".." {
            return Err(format!("path traversal rejected: `{rel_path}`"));
        }
    }
    let target = repo_root.join(rel_path);
    // Normalise via canonicalizing the parent (the file itself may not
    // exist yet — we create it).
    let parent = target
        .parent()
        .ok_or_else(|| format!("no parent dir for `{rel_path}`"))?;
    // Create parent dirs (so we can canonicalize).
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("create parent `{}`: {e}", parent.display()))?;
    let canon_parent = parent
        .canonicalize()
        .map_err(|e| format!("canonicalize `{}`: {e}", parent.display()))?;
    let canon_root = repo_root
        .canonicalize()
        .map_err(|e| format!("canonicalize repo root: {e}"))?;
    if !canon_parent.starts_with(&canon_root) {
        return Err(format!(
            "path escapes repo root: `{rel_path}` → `{}`",
            canon_parent.display()
        ));
    }
    // Forbid writes inside .git/
    let file_name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let full = canon_parent.join(&file_name);
    let relative_to_root = full
        .strip_prefix(&canon_root)
        .map_err(|e| format!("strip prefix: {e}"))?;
    for comp in relative_to_root.components() {
        let s = comp.as_os_str().to_string_lossy();
        if s == ".git" {
            return Err(format!("writes to .git/ are forbidden: `{rel_path}`"));
        }
    }
    Ok(full)
}

// ── Atomic write ────────────────────────────────────────────────────

/// Write `content` to `path` atomically via a sibling `.tmp` file +
/// rename. Creates parent directories if needed.
fn atomic_write(path: &Path, content: &str) -> Result<usize, String> {
    let tmp_path = path.with_extension("apply_tmp");
    let mut f = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("create tmp `{}`: {e}", tmp_path.display()))?;
    f.write_all(content.as_bytes())
        .map_err(|e| format!("write tmp `{}`: {e}", tmp_path.display()))?;
    f.flush()
        .map_err(|e| format!("flush tmp `{}`: {e}", tmp_path.display()))?;
    drop(f);
    std::fs::rename(&tmp_path, path)
        .map_err(|e| format!("rename `{}` → `{}`: {e}", tmp_path.display(), path.display()))?;
    Ok(content.len())
}

// ── Git staging ─────────────────────────────────────────────────────

/// Best-effort `git add <path>` relative to `repo_root`.
fn git_add(repo_root: &Path, file: &Path) -> bool {
    // On Windows, canonicalize can return a \\?\ path that doesn't
    // match the tempdir root. Fall back to using the path relative to
    // `repo_root` if strip_prefix fails after canonicalizing both.
    let root_canon = repo_root.canonicalize().unwrap_or_else(|_| repo_root.to_path_buf());
    let file_canon = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    let rel = match file_canon.strip_prefix(&root_canon) {
        Ok(r) => r.to_string_lossy().to_string(),
        Err(_) => match file.strip_prefix(repo_root) {
            Ok(r) => r.to_string_lossy().to_string(),
            Err(_) => return false,
        },
    };
    Command::new("git")
        .args(["add", "--", &rel])
        .current_dir(repo_root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── Public entry point ──────────────────────────────────────────────

/// Apply a set of [`FileBlock`]s to `repo_root`. Writes valid blocks
/// atomically and stages them; collects rejections for invalid paths.
///
/// `git_stage`: when `true`, each successfully written file is staged
/// via `git add`. When `false`, files are written but not staged (e.g.
/// for dry-run / preview use cases).
pub fn apply_blocks(
    repo_root: &Path,
    blocks: &[FileBlock],
    git_stage: bool,
) -> ApplySummary {
    let mut applied = Vec::new();
    let mut rejected = Vec::new();

    for block in blocks {
        match validate_path(repo_root, &block.path) {
            Err(reason) => {
                rejected.push(ApplyRejection {
                    path: block.path.clone(),
                    reason,
                });
            }
            Ok(abs_path) => {
                let created = !abs_path.exists();
                match atomic_write(&abs_path, &block.content) {
                    Ok(n) => {
                        let staged = if git_stage {
                            git_add(repo_root, &abs_path)
                        } else {
                            false
                        };
                        applied.push(ApplyResult {
                            path: block.path.clone(),
                            written_bytes: n,
                            created,
                            git_added: staged,
                        });
                    }
                    Err(reason) => {
                        rejected.push(ApplyRejection {
                            path: block.path.clone(),
                            reason,
                        });
                    }
                }
            }
        }
    }

    ApplySummary { applied, rejected }
}

/// Convenience: parse + apply in one call. Typical path after
/// receiving a model reply with `BareFileContents` output shape or
/// a multi-file "implement this chunk" response.
pub fn apply_from_reply(
    repo_root: &Path,
    reply: &str,
    git_stage: bool,
) -> ApplySummary {
    let blocks = parse_file_blocks(reply);
    apply_blocks(repo_root, &blocks, git_stage)
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parse_single_file_block() {
        let reply = r#"Here's the code:
<file path="src/main.rs">fn main() {
    println!("hello");
}
</file>
Done."#;
        let blocks = parse_file_blocks(reply);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].path, "src/main.rs");
        assert!(blocks[0].content.contains("fn main()"));
    }

    #[test]
    fn parse_multiple_file_blocks() {
        let reply = r#"<file path="a.rs">AAA</file>
some commentary
<file path='b.rs'>BBB</file>"#;
        let blocks = parse_file_blocks(reply);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].path, "a.rs");
        assert_eq!(blocks[0].content, "AAA");
        assert_eq!(blocks[1].path, "b.rs");
        assert_eq!(blocks[1].content, "BBB");
    }

    #[test]
    fn parse_skips_malformed_blocks() {
        // No path attribute.
        let reply = "<file>content</file>";
        assert!(parse_file_blocks(reply).is_empty());
        // Unclosed tag.
        let reply2 = r#"<file path="a.rs">content"#;
        assert!(parse_file_blocks(reply2).is_empty());
    }

    #[test]
    fn validate_rejects_absolute_path() {
        let dir = tempdir().unwrap();
        let err = validate_path(dir.path(), "/etc/passwd").unwrap_err();
        assert!(err.contains("absolute"), "{err}");
    }

    #[test]
    fn validate_rejects_traversal() {
        let dir = tempdir().unwrap();
        let err = validate_path(dir.path(), "../../etc/passwd").unwrap_err();
        assert!(err.contains("traversal"), "{err}");
    }

    #[test]
    fn validate_rejects_dot_git() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        let err = validate_path(dir.path(), ".git/config").unwrap_err();
        assert!(err.contains(".git"), "{err}");
    }

    #[test]
    fn validate_allows_normal_path() {
        let dir = tempdir().unwrap();
        let result = validate_path(dir.path(), "src/lib.rs");
        assert!(result.is_ok(), "{:?}", result);
    }

    #[test]
    fn validate_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = validate_path(dir.path(), "deep/nested/file.rs").unwrap();
        assert!(path.parent().unwrap().exists());
    }

    #[test]
    fn atomic_write_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        let n = atomic_write(&path, "world").unwrap();
        assert_eq!(n, 5);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "world");
        // No leftover tmp file.
        assert!(!dir.path().join("hello.apply_tmp").exists());
    }

    #[test]
    fn apply_blocks_writes_and_stages() {
        let dir = tempdir().unwrap();
        // Init a minimal git repo so `git add` works.
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        // git add requires at least one commit on some git versions, so
        // create an empty initial commit.
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let blocks = vec![
            FileBlock {
                path: "src/a.rs".into(),
                content: "fn a() {}".into(),
            },
            FileBlock {
                path: "b.txt".into(),
                content: "hello".into(),
            },
        ];
        let summary = apply_blocks(dir.path(), &blocks, true);
        assert_eq!(summary.applied.len(), 2);
        assert!(summary.rejected.is_empty());
        assert!(summary.applied[0].created);
        assert!(summary.applied[0].git_added);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("src/a.rs")).unwrap(),
            "fn a() {}"
        );
    }

    #[test]
    fn apply_blocks_rejects_traversal_but_applies_others() {
        let dir = tempdir().unwrap();
        let blocks = vec![
            FileBlock {
                path: "good.rs".into(),
                content: "ok".into(),
            },
            FileBlock {
                path: "../../bad.rs".into(),
                content: "evil".into(),
            },
        ];
        let summary = apply_blocks(dir.path(), &blocks, false);
        assert_eq!(summary.applied.len(), 1);
        assert_eq!(summary.rejected.len(), 1);
        assert!(summary.rejected[0].reason.contains("traversal"));
    }

    #[test]
    fn apply_from_reply_end_to_end() {
        let dir = tempdir().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let reply = r#"I've implemented the changes.
<file path="src/lib.rs">pub fn add(a: i32, b: i32) -> i32 { a + b }
</file>
<file path="tests/test.rs">#[test]
fn it_works() { assert_eq!(2 + 2, 4); }
</file>"#;
        let summary = apply_from_reply(dir.path(), reply, true);
        assert_eq!(summary.applied.len(), 2);
        assert!(summary.rejected.is_empty());
        assert!(dir.path().join("src/lib.rs").exists());
        assert!(dir.path().join("tests/test.rs").exists());
    }

    #[test]
    fn apply_overwrites_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.rs");
        std::fs::write(&path, "old content").unwrap();
        let blocks = vec![FileBlock {
            path: "file.rs".into(),
            content: "new content".into(),
        }];
        let summary = apply_blocks(dir.path(), &blocks, false);
        assert_eq!(summary.applied.len(), 1);
        assert!(!summary.applied[0].created, "should report overwrite");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "new content"
        );
    }
}
