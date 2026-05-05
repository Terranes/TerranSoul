//! Persistent chat-history storage for coding-workflow sessions
//! (Chunk 30.2 — self-improve UX absorption from claw-code / Claude Code /
//! OpenClaw).
//!
//! ## Why this module exists
//!
//! [`crate::coding::handoff_store`] persists a single compact
//! [`HandoffState`](super::handoff::HandoffState) snapshot per session id —
//! perfect for re-grounding the LLM on the *next* invocation, but not the
//! verbatim turn-by-turn record a user expects when they ask "what did we
//! say in this session yesterday?". Claude Code's `--resume` and claw-code's
//! sessions sidebar both rely on a full transcript; this module is the
//! TerranSoul equivalent.
//!
//! ## Storage layout
//!
//! ```text
//! <data_dir>/
//!   coding_workflow/
//!     sessions/
//!       <session_id>.json          # HandoffState snapshot (existing)
//!       <session_id>.chat.jsonl    # one ChatMessage per line (this module)
//! ```
//!
//! Append-only JSONL keeps writes O(1) and crash-safe — a torn final line
//! is the worst possible outcome and is silently dropped on read.
//! [`sanitize_session_id`](super::handoff_store::sanitize_session_id) is
//! reused so chat files share the same safe-id discipline as snapshots.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::handoff_store::{sanitize_session_id, sessions_dir};

/// One persisted message in a coding-workflow session.
///
/// `kind` is a free-form discriminator (`"chat"`, `"system"`, `"run"`,
/// `"error"`, …) so the UI can render different message types distinctly
/// without needing a tagged enum that is hard to evolve.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    /// `"user"`, `"assistant"`, or `"system"`.
    pub role: String,
    /// Raw textual content — Markdown allowed.
    pub content: String,
    /// Unix-ms timestamp when the message was appended.
    pub ts_ms: i64,
    /// Optional discriminator (e.g. `"slash"`, `"run"`, `"chat"`).
    #[serde(default)]
    pub kind: String,
}

impl ChatMessage {
    /// Construct a fresh message stamped with the current wall-clock time.
    pub fn now(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            ts_ms: now_unix_ms(),
            kind: String::new(),
        }
    }

    /// Same as [`Self::now`] but with a typed `kind` discriminator.
    pub fn now_with_kind(
        role: impl Into<String>,
        content: impl Into<String>,
        kind: impl Into<String>,
    ) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            ts_ms: now_unix_ms(),
            kind: kind.into(),
        }
    }
}

/// Lightweight summary returned alongside the existing `HandoffSummary`
/// so the UI can render a sessions sidebar with chat counts without
/// loading every transcript.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ChatSummary {
    /// Number of well-formed messages on disk.
    pub message_count: usize,
    /// Last user-authored message content, truncated to 160 chars.
    /// Empty string when no user messages exist.
    pub last_user_preview: String,
    /// File modification time on disk, unix-ms. `0` when no file exists.
    pub modified_at: i64,
}

/// Hard cap on a single message body so a runaway model cannot bloat
/// the transcript file beyond reasonable bounds. 32 KiB is well above
/// any realistic chat turn but prevents accidental megabyte writes.
pub const MAX_MESSAGE_BYTES: usize = 32 * 1024;

/// Hard cap on the number of messages [`load_chat`] will return when
/// the caller passes `None` for `limit`. The full file is still on
/// disk — this only controls what the UI receives in one call.
pub const DEFAULT_LOAD_LIMIT: usize = 500;

fn chat_path(data_dir: &Path, session_id: &str) -> PathBuf {
    sessions_dir(data_dir).join(format!("{}.chat.jsonl", sanitize_session_id(session_id)))
}

/// Append `msg` to the on-disk transcript for `session_id`.
///
/// Creates the parent directory tree on demand. The message is rejected
/// with an `Err` if its rendered length exceeds [`MAX_MESSAGE_BYTES`] so
/// callers cannot accidentally write multi-megabyte rows.
pub fn append_message(data_dir: &Path, session_id: &str, msg: &ChatMessage) -> Result<(), String> {
    let dir = sessions_dir(data_dir);
    fs::create_dir_all(&dir).map_err(|e| format!("create sessions dir: {e}"))?;

    let line = serde_json::to_string(msg).map_err(|e| format!("serialise message: {e}"))?;
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(format!(
            "message too large ({} > {} bytes)",
            line.len(),
            MAX_MESSAGE_BYTES
        ));
    }

    let path = chat_path(data_dir, session_id);
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open chat file: {e}"))?;
    f.write_all(line.as_bytes())
        .map_err(|e| format!("write line: {e}"))?;
    f.write_all(b"\n").map_err(|e| format!("write nl: {e}"))?;
    Ok(())
}

/// Load the transcript for `session_id`, newest-last.
///
/// `limit` caps the number of returned messages from the *tail* of the
/// file (so a huge transcript still loads quickly). Pass `None` for
/// the default of [`DEFAULT_LOAD_LIMIT`]. Corrupt JSON lines are
/// silently skipped — they survive on disk for human inspection.
pub fn load_chat(
    data_dir: &Path,
    session_id: &str,
    limit: Option<usize>,
) -> Result<Vec<ChatMessage>, String> {
    let path = chat_path(data_dir, session_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let f = fs::File::open(&path).map_err(|e| format!("open chat file: {e}"))?;
    let reader = BufReader::new(f);
    let mut all = Vec::new();
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(msg) = serde_json::from_str::<ChatMessage>(trimmed) {
            all.push(msg);
        }
    }
    let cap = limit.unwrap_or(DEFAULT_LOAD_LIMIT);
    if all.len() > cap {
        let start = all.len() - cap;
        Ok(all.split_off(start))
    } else {
        Ok(all)
    }
}

/// Delete the transcript for `session_id`. Returns `Ok(false)` when no
/// file existed (so the UI can be idempotent).
pub fn clear_chat(data_dir: &Path, session_id: &str) -> Result<bool, String> {
    let path = chat_path(data_dir, session_id);
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path).map_err(|e| format!("delete chat file: {e}"))?;
    Ok(true)
}

/// Compute a [`ChatSummary`] for `session_id` without loading every
/// message into memory. Returns the default value (zeroes) when no
/// transcript exists.
pub fn chat_summary(data_dir: &Path, session_id: &str) -> Result<ChatSummary, String> {
    let path = chat_path(data_dir, session_id);
    if !path.exists() {
        return Ok(ChatSummary::default());
    }
    let metadata = fs::metadata(&path).map_err(|e| format!("stat chat file: {e}"))?;
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let f = fs::File::open(&path).map_err(|e| format!("open chat file: {e}"))?;
    let reader = BufReader::new(f);
    let mut count = 0usize;
    let mut last_user_preview = String::new();
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(msg) = serde_json::from_str::<ChatMessage>(trimmed) {
            count += 1;
            if msg.role == "user" {
                last_user_preview = truncate_preview(&msg.content, 160);
            }
        }
    }
    Ok(ChatSummary {
        message_count: count,
        last_user_preview,
        modified_at,
    })
}

/// Copy `source_session_id`'s transcript into `target_session_id`'s
/// slot, returning the number of messages copied. Used by the "fork
/// session" UX (Claude Code `--fork-session`).
///
/// Returns `Ok(0)` when the source has no transcript yet. Refuses to
/// overwrite an existing target transcript with `Err`.
pub fn fork_chat(
    data_dir: &Path,
    source_session_id: &str,
    target_session_id: &str,
) -> Result<usize, String> {
    let src = chat_path(data_dir, source_session_id);
    let dst = chat_path(data_dir, target_session_id);
    if !src.exists() {
        return Ok(0);
    }
    if dst.exists() {
        return Err(format!(
            "fork target {} already has a transcript",
            sanitize_session_id(target_session_id)
        ));
    }
    fs::create_dir_all(sessions_dir(data_dir)).map_err(|e| format!("create sessions dir: {e}"))?;
    fs::copy(&src, &dst).map_err(|e| format!("copy chat file: {e}"))?;
    let messages = load_chat(data_dir, target_session_id, None)?;
    Ok(messages.len())
}

fn truncate_preview(s: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max_chars {
            out.push('…');
            break;
        }
        if ch == '\n' || ch == '\r' {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "ts-session-chat-{}-{}-{}",
            tag,
            std::process::id(),
            now_unix_ms()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn user(content: &str) -> ChatMessage {
        ChatMessage {
            role: "user".into(),
            content: content.into(),
            ts_ms: 1,
            kind: String::new(),
        }
    }

    fn assistant(content: &str) -> ChatMessage {
        ChatMessage {
            role: "assistant".into(),
            content: content.into(),
            ts_ms: 2,
            kind: String::new(),
        }
    }

    #[test]
    fn append_then_load_roundtrips() {
        let dir = tmp_dir("roundtrip");
        append_message(&dir, "alpha", &user("hi")).unwrap();
        append_message(&dir, "alpha", &assistant("hello")).unwrap();
        let got = load_chat(&dir, "alpha", None).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].content, "hi");
        assert_eq!(got[1].role, "assistant");
    }

    #[test]
    fn load_missing_returns_empty() {
        let dir = tmp_dir("missing");
        let got = load_chat(&dir, "ghost", None).unwrap();
        assert!(got.is_empty());
    }

    #[test]
    fn limit_returns_tail() {
        let dir = tmp_dir("limit");
        for i in 0..10 {
            append_message(&dir, "s", &user(&format!("msg {i}"))).unwrap();
        }
        let got = load_chat(&dir, "s", Some(3)).unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].content, "msg 7");
        assert_eq!(got[2].content, "msg 9");
    }

    #[test]
    fn corrupt_lines_are_skipped() {
        let dir = tmp_dir("corrupt");
        append_message(&dir, "s", &user("good")).unwrap();
        let path = chat_path(&dir, "s");
        let mut f = fs::OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(b"{not valid json\n").unwrap();
        append_message(&dir, "s", &assistant("also good")).unwrap();
        let got = load_chat(&dir, "s", None).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].content, "good");
        assert_eq!(got[1].content, "also good");
    }

    #[test]
    fn append_rejects_oversized_messages() {
        let dir = tmp_dir("oversized");
        let huge = "x".repeat(MAX_MESSAGE_BYTES + 100);
        let err = append_message(&dir, "s", &user(&huge)).unwrap_err();
        assert!(err.contains("too large"), "got: {err}");
    }

    #[test]
    fn clear_removes_file_and_is_idempotent() {
        let dir = tmp_dir("clear");
        append_message(&dir, "s", &user("hi")).unwrap();
        assert!(clear_chat(&dir, "s").unwrap());
        assert!(!clear_chat(&dir, "s").unwrap());
        assert!(load_chat(&dir, "s", None).unwrap().is_empty());
    }

    #[test]
    fn summary_counts_and_previews_last_user() {
        let dir = tmp_dir("summary");
        append_message(&dir, "s", &user("first user")).unwrap();
        append_message(&dir, "s", &assistant("ok")).unwrap();
        append_message(&dir, "s", &user("second user msg")).unwrap();
        let s = chat_summary(&dir, "s").unwrap();
        assert_eq!(s.message_count, 3);
        assert_eq!(s.last_user_preview, "second user msg");
        assert!(s.modified_at > 0);
    }

    #[test]
    fn summary_for_missing_session_is_zeroed() {
        let dir = tmp_dir("summary-empty");
        let s = chat_summary(&dir, "nope").unwrap();
        assert_eq!(s, ChatSummary::default());
    }

    #[test]
    fn fork_copies_transcript_and_refuses_overwrite() {
        let dir = tmp_dir("fork");
        append_message(&dir, "src", &user("a")).unwrap();
        append_message(&dir, "src", &assistant("b")).unwrap();

        let copied = fork_chat(&dir, "src", "dst").unwrap();
        assert_eq!(copied, 2);

        // Forking onto an existing target must fail.
        let err = fork_chat(&dir, "src", "dst").unwrap_err();
        assert!(err.contains("already has a transcript"), "got: {err}");

        // Forking a non-existent source is a clean no-op.
        let copied = fork_chat(&dir, "ghost", "anywhere").unwrap();
        assert_eq!(copied, 0);
    }

    #[test]
    fn truncate_preview_replaces_newlines_and_caps_length() {
        let s = truncate_preview("line one\nline two", 100);
        assert_eq!(s, "line one line two");
        let s = truncate_preview(&"x".repeat(50), 10);
        assert_eq!(s, "xxxxxxxxxx…");
    }

    #[test]
    fn sanitised_ids_share_storage_slot() {
        let dir = tmp_dir("sanitise");
        append_message(&dir, "weird/id", &user("hi")).unwrap();
        // Same after sanitisation -> same slot.
        let got = load_chat(&dir, "weird_id", None).unwrap();
        assert_eq!(got.len(), 1);
    }
}
