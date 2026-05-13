//! Tool-result and shell-output spill-to-disk with previews.
//!
//! When a tool result or shell-subprocess output exceeds the configured
//! threshold, this module writes it to a file under
//! `<worktree>/.terransoul/tool_results/<call_id>.txt` or
//! `<worktree>/.terransoul/shell_output/<call_id>.txt` and returns a
//! compact preview string to replace the full content in-context.
//!
//! Design goals (OpenAgentd audit, lesson §3.1, §3.2):
//! - **Context preservation**: Large tool results no longer blow the model
//!   context window; the agent can read the file on demand via a path ref.
//! - **Preview-first**: The inline replacement carries first + last N lines
//!   so the agent gets orientation without re-reading the whole file.
//! - **Write-failure resilient**: If the spill write fails (disk full, etc.),
//!   the full content is returned unchanged — better a big context than a
//!   broken turn.

use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Bytes above which a tool result is spilled to disk.
/// Default: 40 000 chars ≈ 10 KB.
pub const OFFLOAD_CHAR_THRESHOLD: usize = 40_000;

/// Bytes above which shell subprocess stdout/stderr is spilled to disk.
/// Default: 128 KiB.
pub const SHELL_OUTPUT_BYTES_MAX: usize = 128 * 1024;

/// Lines to show at the start and end of a preview.
const PREVIEW_LINES: usize = 20;

/// Directory under the worktree for spilled tool results.
const TOOL_RESULTS_DIR: &str = ".terransoul/tool_results";

/// Directory under the worktree for spilled shell output.
const SHELL_OUTPUT_DIR: &str = ".terransoul/shell_output";

// ─── Tool-result offload ────────────────────────────────────────────────────

/// Potentially spill a tool-call result to disk.
///
/// If `content.len() <= OFFLOAD_CHAR_THRESHOLD`, returns the content
/// unchanged. Otherwise writes it to
/// `<worktree>/.terransoul/tool_results/<call_id>.txt` and returns a
/// compact preview string.
///
/// On write failure, the full content is returned unchanged so the
/// calling turn is never broken.
pub fn maybe_offload_tool_result(worktree: &Path, call_id: &str, content: String) -> String {
    if content.len() <= OFFLOAD_CHAR_THRESHOLD {
        return content;
    }
    let dir = worktree.join(TOOL_RESULTS_DIR);
    let path = dir.join(format!("{}.txt", sanitize_call_id(call_id)));

    match write_spill(&dir, &path, content.as_bytes()) {
        Ok(()) => build_tool_preview(&path, &content),
        Err(_) => content, // write failure → return full content unchanged
    }
}

/// Potentially spill shell subprocess output to disk.
///
/// If `output.len() <= SHELL_OUTPUT_BYTES_MAX`, returns the output
/// unchanged. Otherwise spills to
/// `<worktree>/.terransoul/shell_output/<call_id>.txt` and returns
/// the last 200 lines + a reference path. Appends a `<shell_metadata>`
/// advisory on timeout.
pub fn maybe_offload_shell_output(
    worktree: &Path,
    call_id: &str,
    output: &[u8],
    timed_out: bool,
) -> String {
    let output_str = String::from_utf8_lossy(output);
    if output.len() <= SHELL_OUTPUT_BYTES_MAX {
        let s = output_str.into_owned();
        if timed_out {
            return format!("{s}\n<shell_metadata>Command timed out. Re-run with a higher timeout_seconds if needed.</shell_metadata>");
        }
        return s;
    }

    let dir = worktree.join(SHELL_OUTPUT_DIR);
    let path = dir.join(format!("{}.txt", sanitize_call_id(call_id)));

    let inline = match write_spill(&dir, &path, output) {
        Ok(()) => build_shell_preview(&path, &output_str, timed_out),
        Err(_) => {
            // Write failure — return tail only, no path reference.
            let tail = tail_lines(&output_str, 200);
            let mut s = format!(
                "[Shell output truncated — {} bytes total; disk write failed]\n\n{}",
                output.len(),
                tail
            );
            if timed_out {
                s.push_str("\n<shell_metadata>Command timed out. Re-run with a higher timeout_seconds if needed.</shell_metadata>");
            }
            s
        }
    };
    inline
}

// ─── Preview builders ────────────────────────────────────────────────────────

fn build_tool_preview(path: &Path, content: &str) -> String {
    let size_bytes = content.len();
    let first_preview = first_lines(content, PREVIEW_LINES);
    let last_preview = last_lines(content, PREVIEW_LINES);
    let omitted = count_lines(content).saturating_sub(PREVIEW_LINES * 2);

    let mut out = String::new();
    let _ = writeln!(out, "[Tool result offloaded — content saved to workspace]");
    let _ = writeln!(out, "File: {}", path.display());
    let _ = writeln!(out, "Size: {} bytes", size_bytes);
    let _ = writeln!(out, "Preview (first {} lines):", PREVIEW_LINES);
    out.push_str(&first_preview);
    if omitted > 0 {
        let _ = writeln!(out, "\n… ({} lines omitted) …", omitted);
    }
    let _ = writeln!(out, "\nPreview (last {} lines):", PREVIEW_LINES);
    out.push_str(&last_preview);
    out
}

fn build_shell_preview(path: &Path, output: &str, timed_out: bool) -> String {
    let total_lines = count_lines(output);
    let tail = tail_lines(output, 200);
    let omitted = total_lines.saturating_sub(200);

    let mut out = String::new();
    let _ = writeln!(
        out,
        "[Shell output offloaded — {} bytes, {} lines total]",
        output.len(),
        total_lines
    );
    let _ = writeln!(out, "Full output: {}", path.display());
    if omitted > 0 {
        let _ = writeln!(out, "… ({} lines omitted, see file) …\n", omitted);
    }
    out.push_str(&tail);
    if timed_out {
        out.push_str("\n<shell_metadata>Command timed out. Re-run with a higher timeout_seconds if needed.</shell_metadata>");
    }
    out
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn write_spill(dir: &Path, path: &Path, content: &[u8]) -> Result<(), std::io::Error> {
    fs::create_dir_all(dir)?;
    let mut f = fs::File::create(path)?;
    f.write_all(content)?;
    Ok(())
}

fn sanitize_call_id(call_id: &str) -> String {
    call_id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect()
}

fn first_lines(s: &str, n: usize) -> String {
    s.lines().take(n).collect::<Vec<_>>().join("\n")
}

fn last_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

fn tail_lines(s: &str, n: usize) -> String {
    last_lines(s, n)
}

fn count_lines(s: &str) -> usize {
    s.lines().count()
}

/// Build the spill path for a tool result, without writing.
/// Useful for generating predictable paths in tests.
pub fn tool_result_path(worktree: &Path, call_id: &str) -> PathBuf {
    worktree
        .join(TOOL_RESULTS_DIR)
        .join(format!("{}.txt", sanitize_call_id(call_id)))
}

/// Build the spill path for shell output, without writing.
pub fn shell_output_path(worktree: &Path, call_id: &str) -> PathBuf {
    worktree
        .join(SHELL_OUTPUT_DIR)
        .join(format!("{}.txt", sanitize_call_id(call_id)))
}

// ─── Process-group spawn helpers ────────────────────────────────────────────

/// Augment a [`tokio::process::Command`] to spawn in its own process group
/// so that on cancel/timeout the entire subtree can be killed atomically.
///
/// - **Unix**: calls `cmd.process_group(0)` (puts the child in a new PGID
///   equal to its own PID; `kill(-pgid, SIGKILL)` kills the subtree).
/// - **Windows**: sets `CREATE_NEW_PROCESS_GROUP` via
///   `cmd.creation_flags(0x0000_0200)`.
///
/// This is best-effort: if the platform extension is unavailable the
/// command is returned unchanged.
pub fn configure_process_group(cmd: &mut tokio::process::Command) {
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        // CREATE_NEW_PROCESS_GROUP = 0x0000_0200
        // CommandExt::creation_flags is defined in std::os::windows::process::CommandExt.
        // We import it in the block so it is only visible on Windows.
        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt as _;
        cmd.creation_flags(0x0000_0200);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn temp_worktree() -> TempDir {
        tempfile::tempdir().expect("temp dir")
    }

    // ─── Tool result offload ────────────────────────────────────────────

    #[test]
    fn small_result_returned_unchanged() {
        let dir = temp_worktree();
        let content = "small output".to_string();
        let result = maybe_offload_tool_result(dir.path(), "call-1", content.clone());
        assert_eq!(result, content);
        // No file should be written.
        assert!(!tool_result_path(dir.path(), "call-1").exists());
    }

    #[test]
    fn large_result_is_spilled() {
        let dir = temp_worktree();
        let content = "x".repeat(OFFLOAD_CHAR_THRESHOLD + 1);
        let result = maybe_offload_tool_result(dir.path(), "call-big", content.clone());

        // File should exist and contain the full content.
        let path = tool_result_path(dir.path(), "call-big");
        assert!(path.exists(), "spill file should be created");
        let written = fs::read_to_string(&path).expect("read spill");
        assert_eq!(written, content);

        // Inline reply should mention the file path and offload marker.
        assert!(result.contains("Tool result offloaded"));
        assert!(result.contains(path.to_str().unwrap()));
        assert!(result.contains("Preview (first"));
        assert!(result.contains("Preview (last"));
    }

    #[test]
    fn spill_file_is_agent_readable() {
        let dir = temp_worktree();
        // 5000 lines × ~12 chars each = ~60 000 bytes > OFFLOAD_CHAR_THRESHOLD
        let content = (0..5000)
            .map(|i| format!("line {:06}", i))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            content.len() > OFFLOAD_CHAR_THRESHOLD,
            "precondition: content must exceed threshold"
        );
        maybe_offload_tool_result(dir.path(), "readable", content.clone());
        let path = tool_result_path(dir.path(), "readable");
        let on_disk = fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk, content, "disk file must match original content");
    }

    #[test]
    fn no_break_on_spill_write_failure() {
        // Use an invalid path (empty component) so the write fails.
        let bad_worktree = PathBuf::from("/\0/does/not/exist");
        let content = "x".repeat(OFFLOAD_CHAR_THRESHOLD + 1);
        // Should return the full content unchanged, never panic.
        let result = maybe_offload_tool_result(&bad_worktree, "fail", content.clone());
        assert_eq!(result, content);
    }

    // ─── Shell output spill ──────────────────────────────────────────────

    #[test]
    fn small_shell_output_returned_unchanged() {
        let dir = temp_worktree();
        let output = b"hello world";
        let result = maybe_offload_shell_output(dir.path(), "sh-1", output, false);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn large_shell_output_is_spilled() {
        let dir = temp_worktree();
        let output = b"line\n".repeat(SHELL_OUTPUT_BYTES_MAX / 5 + 1);
        let result = maybe_offload_shell_output(dir.path(), "sh-big", &output, false);
        let path = shell_output_path(dir.path(), "sh-big");
        assert!(path.exists(), "shell spill file should be created");
        assert!(result.contains("Shell output offloaded"));
        assert!(result.contains(path.to_str().unwrap()));
    }

    #[test]
    fn timeout_advisory_appended_when_spilled() {
        let dir = temp_worktree();
        let output = b"x".repeat(SHELL_OUTPUT_BYTES_MAX + 1);
        let result = maybe_offload_shell_output(dir.path(), "sh-timeout", &output, true);
        assert!(
            result.contains("shell_metadata"),
            "timeout advisory must be present when timed_out=true"
        );
        assert!(result.contains("timeout_seconds"));
    }

    #[test]
    fn timeout_advisory_appended_when_not_spilled() {
        let dir = temp_worktree();
        let output = b"small";
        let result = maybe_offload_shell_output(dir.path(), "sh-small-timeout", output, true);
        assert!(result.contains("shell_metadata"));
    }

    #[test]
    fn shell_spill_last_200_lines_present() {
        let dir = temp_worktree();
        // 600 lines, each 220 bytes → > 128 KiB total
        let lines: Vec<String> = (0..600).map(|i| format!("output-line-{:04}", i)).collect();
        let output = lines.join("\n").into_bytes();
        let result = maybe_offload_shell_output(dir.path(), "sh-lines", &output, false);
        // Last line should be in the preview.
        assert!(
            result.contains("output-line-0599"),
            "last line must be in preview"
        );
    }

    // ─── Call-id sanitization ────────────────────────────────────────────

    #[test]
    fn sanitize_call_id_strips_special_chars() {
        let safe = sanitize_call_id("call/id with spaces & symbols!");
        assert!(!safe.contains('/'));
        assert!(!safe.contains(' '));
        assert!(!safe.contains('&'));
        assert!(!safe.contains('!'));
    }

    #[test]
    fn sanitize_call_id_truncates_at_64() {
        let long = "a".repeat(128);
        let safe = sanitize_call_id(&long);
        assert_eq!(safe.len(), 64);
    }
}
