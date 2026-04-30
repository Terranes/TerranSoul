//! Coding-workflow session handoff codec — pure utilities for stamping
//! and parsing a compact `[RESUMING SESSION]` system-prompt block.
//!
//! ## Why this module exists
//!
//! Long autonomous coding sessions (a single `run_coding_task` chunk
//! repeatedly invoked over hours, or a chained series of planner → coder
//! → reviewer calls) routinely exhaust the LLM's context window. The
//! VS Code Copilot host auto-summarises history when its budget fills,
//! local Ollama models drop sliding-window context after ~32 K tokens,
//! and cloud providers truncate silently above their declared limits.
//! When that happens, the model loses the thread of what it was doing
//! and starts over.
//!
//! This module is the pure half of the fix (Chunk 28.8a): every
//! [`run_coding_task`](super::workflow::run_coding_task) invocation can
//! stamp a compact JSON "next-session seed" describing what it just
//! did, what is still pending, and which files are open; the next
//! invocation prepends a `[RESUMING SESSION]` block to its prompt that
//! re-grounds the model in O(few-hundred-tokens). The persistence +
//! Tauri-command wiring lands in Chunk 28.9 (28.8b).
//!
//! Both functions in this module are I/O-free and exhaustively unit
//! tested. They follow the same shape as
//! [`crate::orchestrator::handoff`](crate::orchestrator::handoff) — the
//! agent-swap handoff block builder shipped in Chunk 23.2a — so a
//! reader who has reviewed that module already understands this one.
//!
//! ## Hard cap
//!
//! [`build_handoff_block`] guarantees the returned string is ≤
//! [`MAX_BLOCK_BYTES`]. If the structured input would exceed the cap,
//! fields are truncated in priority order (least-essential first):
//! `open_artefacts` → `pending_steps` → `summary` → `last_action`.
//! Truncated lists/strings are suffixed with `…` so the model can tell
//! information was clipped.

use serde::{Deserialize, Serialize};

/// Hard cap on the rendered `[RESUMING SESSION]` block, in bytes.
///
/// 4 KiB is roughly 1 K tokens for English text — enough for a
/// rich seed without dominating the prompt budget.
pub const MAX_BLOCK_BYTES: usize = 4 * 1024;

/// XML-style sentinel the model is asked to wrap its seed payload in.
pub const SEED_OPEN_TAG: &str = "<next_session_seed>";
/// Closing sentinel matching [`SEED_OPEN_TAG`].
pub const SEED_CLOSE_TAG: &str = "</next_session_seed>";

/// Structured handoff state passed between coding-workflow invocations.
///
/// Every field is plain `String` / `Vec<String>` / `i64` so the type
/// round-trips cleanly through JSON and through `serde_json::Value`
/// without information loss. The schema is intentionally narrow: the
/// model is bad at filling out long forms reliably, so we keep the
/// surface minimal and let the `summary` field carry free-form prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffState {
    /// Stable identifier for the long-running session. Same value
    /// across all invocations belonging to the same logical task.
    pub session_id: String,
    /// Milestone chunk id this session is working on (e.g. `"28.8"`).
    pub chunk_id: String,
    /// One-line description of the most recently completed step.
    pub last_action: String,
    /// Ordered list of remaining steps, most-imminent first.
    #[serde(default)]
    pub pending_steps: Vec<String>,
    /// Files / artefacts the next invocation should inspect first
    /// (workspace-relative paths, no leading slash).
    #[serde(default)]
    pub open_artefacts: Vec<String>,
    /// Free-form prose summarising why the session exists and what
    /// invariants the next step must preserve.
    pub summary: String,
    /// Unix-ms timestamp when this seed was produced.
    pub created_at: i64,
}

impl HandoffState {
    /// Construct an empty handoff seed for `session_id` / `chunk_id`.
    pub fn new(session_id: impl Into<String>, chunk_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            chunk_id: chunk_id.into(),
            last_action: String::new(),
            pending_steps: Vec::new(),
            open_artefacts: Vec::new(),
            summary: String::new(),
            created_at: 0,
        }
    }
}

/// Render `state` as a `[RESUMING SESSION]` system-prompt block.
///
/// The output is hard-capped at [`MAX_BLOCK_BYTES`]. When the
/// pre-truncation render would exceed the cap, fields are shrunk in
/// the order documented at the module level. The function is
/// deterministic: identical input always produces identical output.
pub fn build_handoff_block(state: &HandoffState) -> String {
    let mut clipped = state.clone();

    let mut rendered = render(&clipped);
    if rendered.len() <= MAX_BLOCK_BYTES {
        return rendered;
    }

    // Pass 1: clip open_artefacts (least-essential).
    let mut artefacts_dropped = 0usize;
    while rendered.len() > MAX_BLOCK_BYTES && !clipped.open_artefacts.is_empty() {
        // Pop the real entry. If we currently have a footer in place,
        // strip it before counting / popping again.
        if artefacts_dropped > 0 {
            clipped.open_artefacts.pop(); // remove old footer
        }
        clipped.open_artefacts.pop();
        artefacts_dropped += 1;
        if !clipped.open_artefacts.is_empty() || artefacts_dropped < state.open_artefacts.len() {
            clipped
                .open_artefacts
                .push(format!("…(+{artefacts_dropped} more)"));
        }
        rendered = render(&clipped);
    }
    if rendered.len() <= MAX_BLOCK_BYTES {
        return rendered;
    }

    // Pass 2: clip pending_steps.
    let mut pending_dropped = 0usize;
    while rendered.len() > MAX_BLOCK_BYTES && !clipped.pending_steps.is_empty() {
        if pending_dropped > 0 {
            clipped.pending_steps.pop();
        }
        clipped.pending_steps.pop();
        pending_dropped += 1;
        if !clipped.pending_steps.is_empty() || pending_dropped < state.pending_steps.len() {
            clipped
                .pending_steps
                .push(format!("…(+{pending_dropped} more)"));
        }
        rendered = render(&clipped);
    }
    if rendered.len() <= MAX_BLOCK_BYTES {
        return rendered;
    }

    // Pass 3: clip summary character-by-character.
    if rendered.len() > MAX_BLOCK_BYTES {
        let overflow = rendered.len() - MAX_BLOCK_BYTES + 1; // +1 for the ellipsis
        let new_len = clipped.summary.len().saturating_sub(overflow);
        clipped.summary = truncate_with_ellipsis(&clipped.summary, new_len);
        rendered = render(&clipped);
    }
    if rendered.len() <= MAX_BLOCK_BYTES {
        return rendered;
    }

    // Pass 4: last resort — clip last_action.
    if rendered.len() > MAX_BLOCK_BYTES {
        let overflow = rendered.len() - MAX_BLOCK_BYTES + 1;
        let new_len = clipped.last_action.len().saturating_sub(overflow);
        clipped.last_action = truncate_with_ellipsis(&clipped.last_action, new_len);
        rendered = render(&clipped);
    }

    // If we still overflow (pathological session_id / chunk_id), hard
    // truncate the whole block. This is a contract violation by the
    // caller, but we never return more than MAX_BLOCK_BYTES.
    if rendered.len() > MAX_BLOCK_BYTES {
        rendered.truncate(MAX_BLOCK_BYTES);
    }
    rendered
}

/// Pure formatter — no truncation, no I/O.
fn render(state: &HandoffState) -> String {
    let mut s = String::with_capacity(512);
    s.push_str("[RESUMING SESSION]\n");
    s.push_str(&format!("session: {}\n", state.session_id));
    s.push_str(&format!("chunk:   {}\n", state.chunk_id));
    s.push_str(&format!("last:    {}\n", state.last_action));
    s.push_str("pending:\n");
    if state.pending_steps.is_empty() {
        s.push_str("  (none)\n");
    } else {
        for step in &state.pending_steps {
            s.push_str(&format!("  - {step}\n"));
        }
    }
    s.push_str("open:\n");
    if state.open_artefacts.is_empty() {
        s.push_str("  (none)\n");
    } else {
        for art in &state.open_artefacts {
            s.push_str(&format!("  - {art}\n"));
        }
    }
    s.push_str(&format!("summary: {}\n", state.summary));
    s.push_str("[/RESUMING SESSION]");
    s
}

/// Truncate `s` to fit within `max` bytes, appending `…` when clipped.
/// The result respects UTF-8 boundaries.
fn truncate_with_ellipsis(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    if max == 0 {
        return "…".to_string();
    }
    // Reserve 3 bytes for the ellipsis (U+2026 is 3 bytes in UTF-8).
    let budget = max.saturating_sub(3);
    let mut end = budget;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = String::with_capacity(end + 3);
    out.push_str(&s[..end]);
    out.push('…');
    out
}

/// Build the system-prompt fragment that instructs the model to emit a
/// seed payload for the next session. Append the result to the
/// existing system prompt before sending to the LLM.
pub fn emit_handoff_seed_instruction() -> String {
    let mut s = String::with_capacity(512);
    s.push_str("\n\n# Session handoff\n\n");
    s.push_str("Before ending your reply, emit a JSON payload describing the next session's seed inside ");
    s.push_str(SEED_OPEN_TAG);
    s.push_str(" / ");
    s.push_str(SEED_CLOSE_TAG);
    s.push_str(" tags. Schema:\n\n");
    s.push_str("```json\n");
    s.push_str("{\n");
    s.push_str("  \"session_id\": \"<unchanged from the [RESUMING SESSION] block, or a new id if none was supplied>\",\n");
    s.push_str("  \"chunk_id\": \"<milestone chunk id, e.g. 28.8>\",\n");
    s.push_str("  \"last_action\": \"<one-line description of what you just did>\",\n");
    s.push_str("  \"pending_steps\": [\"<step 1>\", \"<step 2>\"],\n");
    s.push_str("  \"open_artefacts\": [\"<workspace-relative path>\"],\n");
    s.push_str("  \"summary\": \"<2-3 sentences for the next call>\",\n");
    s.push_str("  \"created_at\": <unix-ms integer>\n");
    s.push_str("}\n");
    s.push_str("```\n");
    s.push_str("Keep the payload under 3 KB. Omit fields you cannot fill rather than inventing values.\n");
    s
}

/// Extract a [`HandoffState`] from the model's reply.
///
/// Returns `None` when:
/// * the seed tags are absent,
/// * the tags are present but contain malformed JSON,
/// * the JSON is well-formed but missing required fields
///   (`session_id`, `chunk_id`, `last_action`, `summary`, `created_at`).
///
/// The function tolerates leading/trailing whitespace, fenced code
/// blocks (`` ```json `` … `` ``` ``) inside the tag region, and a
/// trailing newline before the closing tag.
pub fn parse_handoff_reply(reply: &str) -> Option<HandoffState> {
    let open_idx = reply.find(SEED_OPEN_TAG)?;
    let after_open = open_idx + SEED_OPEN_TAG.len();
    let rel_close = reply[after_open..].find(SEED_CLOSE_TAG)?;
    let raw = &reply[after_open..after_open + rel_close];
    let cleaned = strip_code_fence(raw.trim());
    serde_json::from_str::<HandoffState>(cleaned).ok()
}

/// If `s` is wrapped in a Markdown fenced code block (```` ```json ``` ````
/// or plain ```` ``` ````), return the inner body; otherwise return `s`.
fn strip_code_fence(s: &str) -> &str {
    let s = s.trim();
    if !s.starts_with("```") {
        return s;
    }
    // Drop the opening fence line.
    let after_open = match s.find('\n') {
        Some(i) => &s[i + 1..],
        None => return s,
    };
    // Drop the closing fence.
    if let Some(end) = after_open.rfind("```") {
        after_open[..end].trim_end_matches('\n').trim_end()
    } else {
        after_open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> HandoffState {
        HandoffState {
            session_id: "sess-001".into(),
            chunk_id: "28.8".into(),
            last_action: "drafted handoff codec module".into(),
            pending_steps: vec![
                "wire into run_coding_task".into(),
                "add Tauri commands".into(),
            ],
            open_artefacts: vec![
                "src-tauri/src/coding/handoff.rs".into(),
                "src-tauri/src/coding/workflow.rs".into(),
            ],
            summary: "Pure builder + parser for [RESUMING SESSION] blocks.".into(),
            created_at: 1_730_000_000_000,
        }
    }

    #[test]
    fn build_block_renders_all_fields() {
        let block = build_handoff_block(&sample_state());
        assert!(block.starts_with("[RESUMING SESSION]"));
        assert!(block.ends_with("[/RESUMING SESSION]"));
        assert!(block.contains("session: sess-001"));
        assert!(block.contains("chunk:   28.8"));
        assert!(block.contains("last:    drafted handoff codec module"));
        assert!(block.contains("- wire into run_coding_task"));
        assert!(block.contains("- src-tauri/src/coding/handoff.rs"));
        assert!(block.contains("summary: Pure builder + parser"));
    }

    #[test]
    fn build_block_handles_empty_lists() {
        let mut s = sample_state();
        s.pending_steps.clear();
        s.open_artefacts.clear();
        let block = build_handoff_block(&s);
        assert!(block.contains("pending:\n  (none)"));
        assert!(block.contains("open:\n  (none)"));
    }

    #[test]
    fn build_block_is_deterministic() {
        let s = sample_state();
        assert_eq!(build_handoff_block(&s), build_handoff_block(&s));
    }

    #[test]
    fn build_block_respects_hard_cap_clipping_artefacts_first() {
        let mut s = sample_state();
        // Pile up 1000 long artefact paths.
        s.open_artefacts = (0..1000)
            .map(|i| format!("very/long/path/segment/file_{i:04}.rs"))
            .collect();
        let block = build_handoff_block(&s);
        assert!(
            block.len() <= MAX_BLOCK_BYTES,
            "block was {} bytes",
            block.len()
        );
        // Pending steps should still be intact (artefacts clipped first).
        assert!(block.contains("- wire into run_coding_task"));
        // The "+N more" footer should appear when artefacts were clipped.
        assert!(block.contains("more)"));
    }

    #[test]
    fn build_block_clips_pending_when_artefacts_alone_insufficient() {
        let mut s = sample_state();
        s.open_artefacts.clear();
        s.pending_steps = (0..1000)
            .map(|i| format!("step number {i:04} with some descriptive prose to bulk it up"))
            .collect();
        let block = build_handoff_block(&s);
        assert!(block.len() <= MAX_BLOCK_BYTES);
        assert!(block.contains("more)"));
    }

    #[test]
    fn build_block_clips_summary_as_last_resort() {
        let mut s = sample_state();
        s.open_artefacts.clear();
        s.pending_steps.clear();
        s.summary = "x".repeat(MAX_BLOCK_BYTES * 2);
        let block = build_handoff_block(&s);
        assert!(block.len() <= MAX_BLOCK_BYTES);
        assert!(block.contains('…'));
    }

    #[test]
    fn truncate_with_ellipsis_respects_utf8() {
        let s = "héllo wörld";
        let out = truncate_with_ellipsis(s, 6);
        assert!(out.ends_with('…'));
        assert!(out.is_char_boundary(out.len() - '…'.len_utf8()));
    }

    #[test]
    fn parse_reply_extracts_seed() {
        let body = serde_json::to_string(&sample_state()).unwrap();
        let reply = format!(
            "Done.\n\n{open}\n{body}\n{close}\n",
            open = SEED_OPEN_TAG,
            body = body,
            close = SEED_CLOSE_TAG
        );
        let got = parse_handoff_reply(&reply).expect("seed");
        assert_eq!(got, sample_state());
    }

    #[test]
    fn parse_reply_handles_fenced_json() {
        let body = serde_json::to_string_pretty(&sample_state()).unwrap();
        let reply = format!(
            "{open}\n```json\n{body}\n```\n{close}",
            open = SEED_OPEN_TAG,
            body = body,
            close = SEED_CLOSE_TAG
        );
        let got = parse_handoff_reply(&reply).expect("seed");
        assert_eq!(got, sample_state());
    }

    #[test]
    fn parse_reply_returns_none_when_tags_absent() {
        assert!(parse_handoff_reply("no tags here").is_none());
        assert!(parse_handoff_reply(SEED_OPEN_TAG).is_none()); // no close
    }

    #[test]
    fn parse_reply_returns_none_on_malformed_json() {
        let reply = format!("{SEED_OPEN_TAG}{{ not json }}{SEED_CLOSE_TAG}");
        assert!(parse_handoff_reply(&reply).is_none());
    }

    #[test]
    fn parse_reply_returns_none_on_missing_required_field() {
        let reply = format!(
            "{SEED_OPEN_TAG}{{\"session_id\":\"x\"}}{SEED_CLOSE_TAG}"
        );
        assert!(parse_handoff_reply(&reply).is_none());
    }

    #[test]
    fn round_trip_through_render_and_parse() {
        let s = sample_state();
        let body = serde_json::to_string(&s).unwrap();
        let reply = format!("{SEED_OPEN_TAG}{body}{SEED_CLOSE_TAG}");
        let got = parse_handoff_reply(&reply).expect("seed");
        assert_eq!(got, s);
    }

    #[test]
    fn emit_instruction_mentions_tags_and_schema() {
        let i = emit_handoff_seed_instruction();
        assert!(i.contains(SEED_OPEN_TAG));
        assert!(i.contains(SEED_CLOSE_TAG));
        assert!(i.contains("session_id"));
        assert!(i.contains("pending_steps"));
        assert!(i.contains("open_artefacts"));
    }
}
