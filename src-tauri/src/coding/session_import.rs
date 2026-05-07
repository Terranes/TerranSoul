//! Cross-harness session import (Chunk 43.12).
//!
//! Detects AI-coding-assistant session transcripts from other tools
//! and parses them into a uniform [`ImportedTurn`] format that can be
//! fed through the brain-memory extraction pipeline.
//!
//! Supported harness directories (under `$HOME`):
//! - `.claude/sessions/`       — Claude Code (JSON/JSONL)
//! - `.codex/sessions/`        — OpenAI Codex CLI (JSON/JSONL)
//! - `.opencode/sessions/`     — OpenCode (JSON/JSONL)
//! - `.cursor/transcripts/`    — Cursor (JSON)
//! - `.config/github-copilot/cli/` — GitHub Copilot CLI (JSON)
//!
//! No replay mode yet — this chunk only handles detection + parsing.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Known AI coding harnesses whose transcripts we can import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Harness {
    Claude,
    Codex,
    OpenCode,
    Cursor,
    CopilotCli,
}

impl Harness {
    /// Relative directory under `$HOME` where this harness stores sessions.
    pub fn relative_dir(&self) -> &'static str {
        match self {
            Self::Claude => ".claude/sessions",
            Self::Codex => ".codex/sessions",
            Self::OpenCode => ".opencode/sessions",
            Self::Cursor => ".cursor/transcripts",
            Self::CopilotCli => ".config/github-copilot/cli",
        }
    }

    /// Tag value used when storing imported memories.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
            Self::Cursor => "cursor",
            Self::CopilotCli => "copilot_cli",
        }
    }

    /// All known harnesses.
    pub const ALL: [Harness; 5] = [
        Self::Claude,
        Self::Codex,
        Self::OpenCode,
        Self::Cursor,
        Self::CopilotCli,
    ];
}

/// A single turn extracted from a transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedTurn {
    /// Which harness this came from.
    pub harness: Harness,
    /// Session/file identifier.
    pub session_id: String,
    /// Role: "user", "assistant", "system", "tool".
    pub role: String,
    /// Content of the turn (redacted).
    pub content: String,
    /// Turn index within the session (0-based).
    pub turn_index: usize,
}

/// Result of scanning for available harness sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedHarness {
    pub harness: Harness,
    pub directory: PathBuf,
    pub file_count: usize,
}

/// Result of importing a single session file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub harness: Harness,
    pub session_id: String,
    pub turns_extracted: usize,
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Scan the user's home directory for known harness transcript directories.
pub fn detect_harnesses(home: &Path) -> Vec<DetectedHarness> {
    Harness::ALL
        .iter()
        .filter_map(|&h| {
            let dir = home.join(h.relative_dir());
            if dir.is_dir() {
                let count = count_transcript_files(&dir);
                if count > 0 {
                    Some(DetectedHarness {
                        harness: h,
                        directory: dir,
                        file_count: count,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Count JSON/JSONL files in a directory (non-recursive).
fn count_transcript_files(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let p = e.path();
                    matches!(
                        p.extension().and_then(|x| x.to_str()),
                        Some("json") | Some("jsonl")
                    )
                })
                .count()
        })
        .unwrap_or(0)
}

/// List transcript files in a harness directory.
pub fn list_session_files(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let p = e.path();
                    if p.is_file()
                        && matches!(
                            p.extension().and_then(|x| x.to_str()),
                            Some("json") | Some("jsonl")
                        )
                    {
                        Some(p)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a single transcript file into turns.
///
/// Supports two formats:
/// 1. **JSON array** — file is `[{role, content}, ...]`
/// 2. **JSONL** — each line is `{role, content, ...}`
///
/// Unknown fields are ignored. If `content` is missing, the turn is
/// skipped. The `role` field defaults to `"unknown"` if absent.
pub fn parse_transcript(
    harness: Harness,
    path: &Path,
) -> ImportResult {
    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let raw = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return ImportResult {
                harness,
                session_id,
                turns_extracted: 0,
                errors: vec![format!("read error: {e}")],
            };
        }
    };

    let mut turns = Vec::new();
    let mut errors = Vec::new();

    // Try JSON array first.
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) {
        for (i, obj) in arr.iter().enumerate() {
            if let Some(turn) = value_to_turn(harness, &session_id, obj, i) {
                turns.push(turn);
            }
        }
    } else {
        // Fall back to JSONL.
        for (i, line) in raw.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(obj) => {
                    if let Some(turn) = value_to_turn(harness, &session_id, &obj, i) {
                        turns.push(turn);
                    }
                }
                Err(e) => {
                    errors.push(format!("line {}: {e}", i + 1));
                }
            }
        }
    }

    let count = turns.len();
    ImportResult {
        harness,
        session_id,
        turns_extracted: count,
        errors,
    }
}

/// Extract a turn from a JSON value.
fn value_to_turn(
    harness: Harness,
    session_id: &str,
    val: &serde_json::Value,
    index: usize,
) -> Option<ImportedTurn> {
    let content = val
        .get("content")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())?;

    let role = val
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    Some(ImportedTurn {
        harness,
        session_id: session_id.to_string(),
        role,
        content: redact_secrets(&content),
        turn_index: index,
    })
}

/// Convert turns into `(role, content)` pairs suitable for
/// `brain_memory::extract_facts`.
pub fn turns_to_history(turns: &[ImportedTurn]) -> Vec<(String, String)> {
    turns
        .iter()
        .map(|t| (t.role.clone(), t.content.clone()))
        .collect()
}

/// Build the tag string for imported memories.
pub fn import_tag(harness: Harness) -> String {
    format!("imported_from={}", harness.tag())
}

// ---------------------------------------------------------------------------
// Redaction
// ---------------------------------------------------------------------------

/// Best-effort redaction of secrets from transcript content.
///
/// Replaces patterns that look like API keys, bearer tokens, or
/// passwords with `[REDACTED]`. Uses simple substring scanning to
/// avoid a `regex` dependency.
pub fn redact_secrets(input: &str) -> String {
    let mut result = input.to_string();

    // Redact bearer tokens: "Bearer <long-token>"
    if let Some(redacted) = redact_after_prefix_ci(&result, "bearer ") {
        result = redacted;
    }

    // Redact sk-* keys
    result = redact_sk_keys(&result);

    // Redact key=value patterns for common secret labels
    for label in &[
        "api_key=",
        "apikey=",
        "api-key=",
        "token=",
        "secret=",
        "password=",
        "api_key:",
        "apikey:",
        "token:",
        "secret:",
        "password:",
    ] {
        if let Some(redacted) = redact_after_prefix_ci(&result, label) {
            result = redacted;
        }
    }

    result
}

/// Find `prefix` (case-insensitive) and redact the token-like chars after it.
fn redact_after_prefix_ci(input: &str, prefix: &str) -> Option<String> {
    let lower = input.to_ascii_lowercase();
    let idx = lower.find(&prefix.to_ascii_lowercase())?;
    let start = idx + prefix.len();
    let token_end = input[start..]
        .find(|c: char| !c.is_ascii_alphanumeric() && !"-_.~+/".contains(c))
        .map(|i| start + i)
        .unwrap_or(input.len());

    let token_len = token_end - start;
    if token_len < 20 {
        return None; // Too short to be a secret.
    }

    let mut result = String::with_capacity(input.len());
    result.push_str(&input[..start]);
    result.push_str("[REDACTED]");
    result.push_str(&input[token_end..]);
    Some(result)
}

/// Redact `sk-*` style API keys (at least 20 chars long).
fn redact_sk_keys(input: &str) -> String {
    let mut result = String::new();
    let mut remaining = input;

    while let Some(idx) = remaining.find("sk-") {
        result.push_str(&remaining[..idx]);
        let after = &remaining[idx + 3..];
        let end = after
            .find(|c: char| !c.is_ascii_alphanumeric() && !"-_.~+/".contains(c))
            .unwrap_or(after.len());

        if end >= 17 {
            // sk- + 17+ chars = 20+ total
            result.push_str("sk-[REDACTED]");
            remaining = &after[end..];
        } else {
            result.push_str("sk-");
            remaining = after;
        }
    }
    result.push_str(remaining);
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir_with_files(
        harness: Harness,
        files: &[(&str, &str)],
    ) -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(harness.relative_dir());
        fs::create_dir_all(&dir).unwrap();
        for (name, content) in files {
            let mut f = fs::File::create(dir.join(name)).unwrap();
            f.write_all(content.as_bytes()).unwrap();
        }
        (tmp, dir)
    }

    #[test]
    fn harness_tags_unique() {
        let tags: Vec<&str> = Harness::ALL.iter().map(|h| h.tag()).collect();
        let mut deduped = tags.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(tags.len(), deduped.len());
    }

    #[test]
    fn detect_harnesses_finds_dirs_with_files() {
        let tmp = tempfile::tempdir().unwrap();
        let claude_dir = tmp.path().join(".claude/sessions");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(claude_dir.join("session1.json"), "[]").unwrap();

        let detected = detect_harnesses(tmp.path());
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].harness, Harness::Claude);
        assert_eq!(detected[0].file_count, 1);
    }

    #[test]
    fn detect_harnesses_ignores_empty_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".codex/sessions");
        fs::create_dir_all(&dir).unwrap();
        // No files.
        let detected = detect_harnesses(tmp.path());
        assert!(detected.is_empty());
    }

    #[test]
    fn parse_json_array_transcript() {
        let content = r#"[
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi there!"}
        ]"#;
        let (_tmp, dir) = temp_dir_with_files(Harness::Claude, &[("sess.json", content)]);
        let result = parse_transcript(Harness::Claude, &dir.join("sess.json"));
        assert_eq!(result.turns_extracted, 2);
        assert!(result.errors.is_empty());
        assert_eq!(result.session_id, "sess");
    }

    #[test]
    fn parse_jsonl_transcript() {
        let content = r#"{"role": "user", "content": "Line 1"}
{"role": "assistant", "content": "Line 2"}
"#;
        let (_tmp, dir) = temp_dir_with_files(Harness::Codex, &[("s2.jsonl", content)]);
        let result = parse_transcript(Harness::Codex, &dir.join("s2.jsonl"));
        assert_eq!(result.turns_extracted, 2);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn parse_skips_entries_without_content() {
        let content = r#"[
            {"role": "system"},
            {"role": "user", "content": "Has content"}
        ]"#;
        let (_tmp, dir) = temp_dir_with_files(Harness::OpenCode, &[("s.json", content)]);
        let result = parse_transcript(Harness::OpenCode, &dir.join("s.json"));
        assert_eq!(result.turns_extracted, 1);
    }

    #[test]
    fn turns_to_history_conversion() {
        let turns = vec![
            ImportedTurn {
                harness: Harness::Claude,
                session_id: "s1".into(),
                role: "user".into(),
                content: "hello".into(),
                turn_index: 0,
            },
            ImportedTurn {
                harness: Harness::Claude,
                session_id: "s1".into(),
                role: "assistant".into(),
                content: "hi".into(),
                turn_index: 1,
            },
        ];
        let hist = turns_to_history(&turns);
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0], ("user".to_string(), "hello".to_string()));
    }

    #[test]
    fn import_tag_format() {
        assert_eq!(import_tag(Harness::Claude), "imported_from=claude");
        assert_eq!(import_tag(Harness::CopilotCli), "imported_from=copilot_cli");
    }

    #[test]
    fn redact_bearer_token() {
        let input = "Authorization: Bearer sk-abc123def456ghi789jkl012mno";
        let out = redact_secrets(input);
        assert!(out.contains("[REDACTED]"));
        assert!(!out.contains("abc123"));
    }

    #[test]
    fn redact_api_key() {
        let input = "api_key=sk-proj-abcdefghijklmnopqrstuvwxyz1234567890";
        let out = redact_secrets(input);
        assert!(out.contains("[REDACTED]"));
        assert!(!out.contains("abcdefghijklmnop"));
    }

    #[test]
    fn redact_preserves_normal_text() {
        let input = "This is a normal message with no secrets.";
        let out = redact_secrets(input);
        assert_eq!(out, input);
    }

    #[test]
    fn list_session_files_filters_json_jsonl() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.json"), "").unwrap();
        fs::write(tmp.path().join("b.jsonl"), "").unwrap();
        fs::write(tmp.path().join("c.txt"), "").unwrap();
        let files = list_session_files(tmp.path());
        assert_eq!(files.len(), 2);
    }
}
