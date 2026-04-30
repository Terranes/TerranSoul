//! Pure VS Code / Copilot log parser — Chunk 24.5a.
//!
//! Phase 24 (mobile companion) needs the phone to narrate
//! "what's Copilot doing on your desktop right now?". The data
//! lives in two places on a typical VS Code install:
//!
//! 1. `<user-data>/logs/<date>/window<N>/exthost/GitHub.copilot-chat/Copilot-Chat.log`
//!    — append-only timestamped log lines emitted by the Copilot
//!    Chat extension. Each line begins with an ISO-8601 timestamp
//!    in square brackets, followed by a level tag (`[info]`,
//!    `[warning]`, etc.), followed by the message.
//! 2. `<user-data>/workspaceStorage/<hash>/state.vscdb`
//!    — SQLite database. Out of scope for 24.5a; the FS / SQLite
//!    layer is the 24.5b job.
//!
//! This module ships the **pure parser** for category 1: feed it a
//! log-file string, get back a structured [`CopilotLogSummary`]
//! describing the active workspace path (when announced), the most
//! recent user / assistant turn timestamps, the latest assistant
//! preview, and a count of tool invocations. No I/O, no clock, no
//! filesystem walking — every rule deterministic on fixture input.
//!
//! The 24.5b chunk wraps this in a tokio `tokio::fs::read_to_string`
//! plus SQLite open and exposes it via gRPC `GetCopilotSessionStatus`
//! (Chunk 24.4).

use serde::{Deserialize, Serialize};

/// Maximum number of characters preserved from an assistant
/// message preview. The phone UI only needs the head; full
/// transcript fetches go through a separate RPC (24.4).
pub const ASSISTANT_PREVIEW_MAX_CHARS: usize = 240;

/// Maximum number of characters preserved from a user-message preview.
pub const USER_PREVIEW_MAX_CHARS: usize = 160;

/// One structured event extracted from a VS Code Copilot Chat log line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEvent {
    /// Raw ISO-8601-ish timestamp string as it appeared between the
    /// leading square brackets (e.g. `2026-04-29 05:17:33.612`). The
    /// caller is responsible for parsing if a richer format is
    /// needed; the phone-side narrator just shows it as-is.
    pub timestamp: String,
    /// Log severity tag (`info` / `warning` / `error` / `trace`).
    pub level: String,
    /// Classified event kind — see [`EventKind`].
    pub kind: EventKind,
    /// The remainder of the message after the timestamp + level.
    pub body: String,
}

/// Classification of a single log line. Kinds are derived from
/// observed Copilot Chat 0.45+ log substrings; the matcher is
/// intentionally conservative (substring match, case-insensitive)
/// so that a Copilot version bump tweaking phrasing won't hard-fail
/// the parser — at worst a known kind degrades to `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Workspace path announcement (e.g. `workspace folder set: <path>`).
    WorkspaceFolder,
    /// Conversation / session id (e.g. `chat session <uuid>`).
    SessionId,
    /// User turn submitted to the model.
    UserTurn,
    /// Assistant streaming chunk or completed turn.
    AssistantTurn,
    /// Tool / function call dispatched by the agent.
    ToolInvocation,
    /// Model name in use (e.g. `model selected: gpt-5`).
    ModelSelected,
    /// Anything else.
    Other,
}

/// Structured summary of the most recent activity in a Copilot Chat
/// log file. Built by [`summarise_log`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopilotLogSummary {
    /// Most-recently announced workspace folder path, if any.
    pub workspace_folder: Option<String>,
    /// Most-recently observed session id, if any.
    pub session_id: Option<String>,
    /// Most-recently announced model name, if any.
    pub model: Option<String>,
    /// Timestamp of the most-recent user turn, if any.
    pub last_user_turn_ts: Option<String>,
    /// Truncated preview of the most-recent user turn.
    pub last_user_preview: Option<String>,
    /// Timestamp of the most-recent assistant turn, if any.
    pub last_assistant_turn_ts: Option<String>,
    /// Truncated preview of the most-recent assistant turn.
    pub last_assistant_preview: Option<String>,
    /// Total tool / function invocations observed in the file.
    pub tool_invocation_count: u32,
    /// Total events parsed (excluding malformed / unparseable lines).
    pub event_count: u32,
}

/// Parse the entire log file into a `Vec<LogEvent>`.
///
/// Lines that don't match the expected `[ts] [level] body` shape are
/// silently skipped — VS Code occasionally emits multi-line
/// stack-trace continuations and free-form banners we don't care
/// about. Order is preserved.
pub fn parse_events(log: &str) -> Vec<LogEvent> {
    log.lines().filter_map(parse_line).collect()
}

/// Build a summary of the most recent activity by scanning the
/// parsed events tail-first. The first match for each field wins —
/// this is what the phone-side narrator shows ("last assistant turn
/// 30 s ago, currently streaming X").
pub fn summarise_log(log: &str) -> CopilotLogSummary {
    let events = parse_events(log);
    let mut s = CopilotLogSummary {
        event_count: events.len() as u32,
        ..Default::default()
    };

    for ev in events.iter().rev() {
        match ev.kind {
            EventKind::WorkspaceFolder if s.workspace_folder.is_none() => {
                s.workspace_folder = extract_after_colon(&ev.body);
            }
            EventKind::SessionId if s.session_id.is_none() => {
                s.session_id = extract_after_colon(&ev.body);
            }
            EventKind::ModelSelected if s.model.is_none() => {
                s.model = extract_after_colon(&ev.body);
            }
            EventKind::UserTurn if s.last_user_turn_ts.is_none() => {
                s.last_user_turn_ts = Some(ev.timestamp.clone());
                s.last_user_preview =
                    Some(truncate(&ev.body, USER_PREVIEW_MAX_CHARS));
            }
            EventKind::AssistantTurn if s.last_assistant_turn_ts.is_none() => {
                s.last_assistant_turn_ts = Some(ev.timestamp.clone());
                s.last_assistant_preview =
                    Some(truncate(&ev.body, ASSISTANT_PREVIEW_MAX_CHARS));
            }
            _ => {}
        }
    }

    s.tool_invocation_count = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::ToolInvocation))
        .count() as u32;

    s
}

/// Parse one line. Expected shape:
///
/// ```text
/// [2026-04-29 05:17:33.612] [info] message body here
/// ```
///
/// Returns `None` if the brackets are missing / malformed.
fn parse_line(line: &str) -> Option<LogEvent> {
    let line = line.trim_end();
    if line.is_empty() {
        return None;
    }
    let (ts, rest) = take_bracketed(line)?;
    let rest = rest.trim_start();
    let (level, body) = take_bracketed(rest)?;
    let body = body.trim_start().to_string();
    let kind = classify(&body);
    Some(LogEvent {
        timestamp: ts.to_string(),
        level: level.to_string(),
        kind,
        body,
    })
}

/// If `s` starts with `[xxx]`, return `(xxx, remainder)`.
fn take_bracketed(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'[') {
        return None;
    }
    let close = s[1..].find(']')?;
    let inner = &s[1..1 + close];
    let rest = &s[1 + close + 1..];
    Some((inner, rest))
}

/// Substring-match classifier. Case-insensitive on the body.
fn classify(body: &str) -> EventKind {
    let lower = body.to_ascii_lowercase();
    if contains_any(&lower, &["workspace folder", "workspace path"]) {
        return EventKind::WorkspaceFolder;
    }
    if contains_any(&lower, &["chat session", "session id", "conversation id"]) {
        return EventKind::SessionId;
    }
    if contains_any(&lower, &["model selected", "using model", "selected model"]) {
        return EventKind::ModelSelected;
    }
    if contains_any(&lower, &["user message", "user turn", "user prompt"]) {
        return EventKind::UserTurn;
    }
    if contains_any(
        &lower,
        &["assistant message", "assistant turn", "assistant chunk", "stream chunk"],
    ) {
        return EventKind::AssistantTurn;
    }
    if contains_any(
        &lower,
        &["tool call", "tool invocation", "function call", "invoking tool"],
    ) {
        return EventKind::ToolInvocation;
    }
    EventKind::Other
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

/// If body is `prefix: value`, return `Some(value)`; else `None`.
fn extract_after_colon(body: &str) -> Option<String> {
    let idx = body.find(':')?;
    let v = body[idx + 1..].trim();
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

/// Char-aware truncation that preserves UTF-8 boundaries.
fn truncate(s: &str, max_chars: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(ts: &str, level: &str, body: &str) -> String {
        format!("[{ts}] [{level}] {body}")
    }

    #[test]
    fn parse_line_classifies_workspace_folder() {
        let l = line("2026-04-29 05:17:33", "info", "workspace folder set: D:/Git/TerranSoul");
        let ev = parse_line(&l).unwrap();
        assert_eq!(ev.timestamp, "2026-04-29 05:17:33");
        assert_eq!(ev.level, "info");
        assert_eq!(ev.kind, EventKind::WorkspaceFolder);
        assert!(ev.body.contains("D:/Git/TerranSoul"));
    }

    #[test]
    fn parse_line_classifies_user_turn() {
        let ev = parse_line(&line("t", "info", "User message received: continue next chunk")).unwrap();
        assert_eq!(ev.kind, EventKind::UserTurn);
    }

    #[test]
    fn parse_line_classifies_assistant_turn() {
        let ev = parse_line(&line("t", "info", "Assistant chunk: shipping 24.5a")).unwrap();
        assert_eq!(ev.kind, EventKind::AssistantTurn);
    }

    #[test]
    fn parse_line_classifies_tool_invocation() {
        let ev = parse_line(&line("t", "info", "Tool call: read_file")).unwrap();
        assert_eq!(ev.kind, EventKind::ToolInvocation);
    }

    #[test]
    fn parse_line_classifies_model_selected() {
        let ev = parse_line(&line("t", "info", "model selected: gpt-5")).unwrap();
        assert_eq!(ev.kind, EventKind::ModelSelected);
    }

    #[test]
    fn parse_line_falls_back_to_other() {
        let ev = parse_line(&line("t", "trace", "starting up extension host")).unwrap();
        assert_eq!(ev.kind, EventKind::Other);
    }

    #[test]
    fn parse_line_classification_is_case_insensitive() {
        let ev = parse_line(&line("t", "INFO", "USER MESSAGE: hi")).unwrap();
        assert_eq!(ev.kind, EventKind::UserTurn);
    }

    #[test]
    fn parse_line_skips_empty_and_malformed() {
        assert!(parse_line("").is_none());
        assert!(parse_line("   ").is_none());
        assert!(parse_line("no brackets here").is_none());
        assert!(parse_line("[only one bracket info] body").is_none());
        assert!(parse_line("[ts]").is_none());
    }

    #[test]
    fn parse_events_preserves_order_and_skips_garbage() {
        let log = format!(
            "{}\nrandom continuation line\n{}\n\n{}\n",
            line("t1", "info", "User message: alpha"),
            line("t2", "info", "Assistant chunk: beta"),
            line("t3", "info", "Tool call: read_file"),
        );
        let evs = parse_events(&log);
        assert_eq!(evs.len(), 3);
        assert_eq!(evs[0].kind, EventKind::UserTurn);
        assert_eq!(evs[1].kind, EventKind::AssistantTurn);
        assert_eq!(evs[2].kind, EventKind::ToolInvocation);
    }

    #[test]
    fn summarise_picks_most_recent_user_and_assistant() {
        let log = format!(
            "{}\n{}\n{}\n{}\n",
            line("t1", "info", "User message: first"),
            line("t2", "info", "Assistant chunk: first reply"),
            line("t3", "info", "User message: second"),
            line("t4", "info", "Assistant chunk: second reply"),
        );
        let s = summarise_log(&log);
        assert_eq!(s.last_user_turn_ts.as_deref(), Some("t3"));
        assert!(s.last_user_preview.as_ref().unwrap().contains("second"));
        assert_eq!(s.last_assistant_turn_ts.as_deref(), Some("t4"));
        assert!(s
            .last_assistant_preview
            .as_ref()
            .unwrap()
            .contains("second reply"));
        assert_eq!(s.event_count, 4);
    }

    #[test]
    fn summarise_extracts_workspace_session_model() {
        let log = format!(
            "{}\n{}\n{}\n",
            line("t", "info", "workspace folder set: D:/Git/TerranSoul"),
            line("t", "info", "chat session: 11111111-2222-3333-4444-555555555555"),
            line("t", "info", "model selected: gpt-5-codex"),
        );
        let s = summarise_log(&log);
        assert_eq!(s.workspace_folder.as_deref(), Some("D:/Git/TerranSoul"));
        assert_eq!(
            s.session_id.as_deref(),
            Some("11111111-2222-3333-4444-555555555555")
        );
        assert_eq!(s.model.as_deref(), Some("gpt-5-codex"));
    }

    #[test]
    fn summarise_counts_tool_invocations() {
        let log = format!(
            "{}\n{}\n{}\n{}\n",
            line("t", "info", "Tool call: read_file"),
            line("t", "info", "Tool call: grep_search"),
            line("t", "info", "Function call: run_in_terminal"),
            line("t", "info", "User message: irrelevant"),
        );
        let s = summarise_log(&log);
        assert_eq!(s.tool_invocation_count, 3);
    }

    #[test]
    fn summarise_uses_most_recent_workspace_when_multiple() {
        let log = format!(
            "{}\n{}\n",
            line("t1", "info", "workspace folder set: D:/Old"),
            line("t2", "info", "workspace folder set: D:/New"),
        );
        let s = summarise_log(&log);
        assert_eq!(s.workspace_folder.as_deref(), Some("D:/New"));
    }

    #[test]
    fn truncate_handles_short_strings_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_appends_ellipsis_when_clipped() {
        let out = truncate("hello world", 5);
        assert_eq!(out.chars().count(), 6); // 5 + ellipsis
        assert!(out.ends_with('…'));
        assert!(out.starts_with("hello"));
    }

    #[test]
    fn truncate_is_utf8_safe() {
        // Multi-byte chars must not be split.
        let s = "αβγδεζη"; // 7 Greek letters, 2 bytes each.
        let out = truncate(s, 3);
        assert_eq!(out.chars().count(), 4); // 3 + ellipsis
        assert!(out.starts_with("αβγ"));
    }

    #[test]
    fn assistant_preview_capped_at_max() {
        let big = "x".repeat(ASSISTANT_PREVIEW_MAX_CHARS + 100);
        let log = line("t", "info", &format!("Assistant chunk: {big}"));
        let s = summarise_log(&log);
        let preview = s.last_assistant_preview.unwrap();
        assert!(preview.chars().count() <= ASSISTANT_PREVIEW_MAX_CHARS + 1);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn empty_log_yields_empty_summary() {
        let s = summarise_log("");
        assert_eq!(s, CopilotLogSummary::default());
    }

    #[test]
    fn extract_after_colon_handles_no_colon() {
        assert!(extract_after_colon("no colon here").is_none());
    }

    #[test]
    fn extract_after_colon_trims() {
        assert_eq!(
            extract_after_colon("key:    value   "),
            Some("value".to_string())
        );
    }

    #[test]
    fn extract_after_colon_rejects_empty_value() {
        assert!(extract_after_colon("key:   ").is_none());
    }

    #[test]
    fn realistic_session_excerpt() {
        // A realistic-shaped excerpt covering the user's headline
        // use case: "what's Copilot doing right now".
        let log = "\
[2026-04-29 05:10:00] [info] workspace folder set: D:/Git/TerranSoul
[2026-04-29 05:10:01] [info] model selected: claude-opus-4.7
[2026-04-29 05:10:01] [info] chat session: abc-123
[2026-04-29 05:10:05] [info] User message: continue next chunks
[2026-04-29 05:10:06] [info] Assistant chunk: I'll ship 24.5a now.
[2026-04-29 05:10:07] [info] Tool call: read_file
[2026-04-29 05:10:08] [info] Tool call: create_file
[2026-04-29 05:10:09] [info] Assistant chunk: 24.5a complete — 23 tests pass.
";
        let s = summarise_log(log);
        assert_eq!(s.workspace_folder.as_deref(), Some("D:/Git/TerranSoul"));
        assert_eq!(s.model.as_deref(), Some("claude-opus-4.7"));
        assert_eq!(s.session_id.as_deref(), Some("abc-123"));
        assert_eq!(s.last_user_turn_ts.as_deref(), Some("2026-04-29 05:10:05"));
        assert_eq!(
            s.last_assistant_turn_ts.as_deref(),
            Some("2026-04-29 05:10:09")
        );
        assert!(s
            .last_assistant_preview
            .as_ref()
            .unwrap()
            .contains("24.5a complete"));
        assert_eq!(s.tool_invocation_count, 2);
        assert_eq!(s.event_count, 8);
    }
}
