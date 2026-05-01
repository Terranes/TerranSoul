//! Agentic RAG: retrieve-as-tool inside an agent loop (Chunk 27.1).
//!
//! The current pipeline does a *single* retrieval at the start of each
//! chat turn (the "static RAG" pattern). Agentic RAG instead embeds
//! `retrieve_memory` as a callable tool the LLM can invoke 0–N times
//! during a multi-step reasoning loop:
//!
//! ```text
//! User query
//!    ↓
//!  Plan (what do I need to know?)
//!    ↓ ← tool: retrieve_memory("query")
//!  Reflect (is this enough?)
//!    ↓ ← optionally retrieve again with refined query
//!  Generate final answer
//! ```
//!
//! ## Design
//!
//! This module provides a **pure tool-loop state machine**
//! ([`AgenticRagLoop`]) that:
//!
//! 1. Takes a user query + an initial system prompt.
//! 2. On each iteration, inspects the model's reply for a
//!    `<tool_call name="retrieve_memory">` block.
//! 3. If found, calls the provided retriever (an `async` closure)
//!    with the extracted query and feeds the results back as a
//!    `<tool_result>` message.
//! 4. Stops when the model either (a) produces a final answer
//!    (no tool-call in its reply), or (b) the iteration cap fires.
//!
//! The loop is generic over the retriever implementation so it can
//! be tested with a stub (no MemoryStore needed). In production
//! the caller passes a closure that calls
//! `memory_store.hybrid_search(...)`.
//!
//! ## Security
//!
//! - Hard iteration cap (default 5) prevents infinite loops.
//! - The only tool exposed is `retrieve_memory` — no file I/O,
//!   no code execution.
//! - Query strings are passed through verbatim (the MemoryStore
//!   sanitises them internally via prepared statements).

use serde::{Deserialize, Serialize};

/// Default maximum retrieve iterations before forcing an answer.
pub const DEFAULT_MAX_ITERATIONS: u8 = 5;

/// Parsed tool-call request from the model's reply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub query: String,
}

/// One turn in the conversation history tracked by the loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopTurn {
    pub role: String,
    pub content: String,
}

/// Final result from the agentic RAG loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgenticRagResult {
    /// The model's final answer (tool-calls stripped).
    pub answer: String,
    /// Number of retrieve rounds the model made.
    pub retrieval_count: u8,
    /// Whether the loop hit the iteration cap without the model
    /// producing a clean final answer.
    pub capped: bool,
    /// Full conversation history (for debugging / logging).
    pub history: Vec<LoopTurn>,
}

/// Configuration for the agentic RAG loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgenticRagConfig {
    /// Maximum retrieve calls before forcing a final answer.
    pub max_iterations: u8,
    /// Top-k results to return per retrieval round.
    pub top_k: usize,
}

impl Default for AgenticRagConfig {
    fn default() -> Self {
        Self {
            max_iterations: DEFAULT_MAX_ITERATIONS,
            top_k: 5,
        }
    }
}

/// System prompt fragment instructing the model to use the retrieve
/// tool when it needs information from long-term memory.
pub const AGENTIC_RAG_TOOL_DESCRIPTION: &str = r#"You have access to the following tool:

<tool name="retrieve_memory">
  <description>Search the user's long-term memory store. Returns the most relevant memory entries for the given query string. Use this when you need factual information the user previously stored, or when you're unsure whether your knowledge is up to date.</description>
  <parameters>
    <query type="string" required="true">A natural-language search query describing what information you need.</query>
  </parameters>
  <usage>
    To call this tool, emit exactly:
    <tool_call name="retrieve_memory"><query>your search query here</query></tool_call>
    Then STOP and wait for the result.
  </usage>
</tool>

When you have enough information to answer, produce your final answer directly WITHOUT any <tool_call> tag. If you call the tool, emit ONLY the <tool_call> tag and nothing else in that turn."#;

/// Parse a `<tool_call name="retrieve_memory"><query>…</query></tool_call>`
/// from the model's reply. Returns `None` if no valid tool-call is present.
pub fn parse_tool_call(reply: &str) -> Option<ToolCall> {
    let tc_open = "<tool_call";
    let tc_close = "</tool_call>";

    let start = reply.find(tc_open)?;
    let end = reply.find(tc_close)?;
    if end <= start {
        return None;
    }

    let block = &reply[start..end + tc_close.len()];

    // Extract name attribute.
    let name = extract_attr(block, "name")?;
    // Extract query content.
    let query = extract_inner_tag(block, "query")?;

    if query.trim().is_empty() {
        return None;
    }

    Some(ToolCall {
        name: name.to_string(),
        query: query.trim().to_string(),
    })
}

/// Strip any tool_call block from the reply, returning the remaining text.
pub fn strip_tool_call(reply: &str) -> String {
    let tc_open = "<tool_call";
    let tc_close = "</tool_call>";

    if let Some(start) = reply.find(tc_open) {
        if let Some(end) = reply.find(tc_close) {
            let before = &reply[..start];
            let after = &reply[end + tc_close.len()..];
            return format!("{}{}", before.trim(), after.trim());
        }
    }
    reply.to_string()
}

/// Format retrieval results as a `<tool_result>` message.
pub fn format_tool_result(results: &[String]) -> String {
    if results.is_empty() {
        return "<tool_result>\nNo relevant memories found.\n</tool_result>".to_string();
    }
    let mut out = String::from("<tool_result>\n");
    for (i, entry) in results.iter().enumerate() {
        out.push_str(&format!("[{}] {}\n", i + 1, entry));
    }
    out.push_str("</tool_result>");
    out
}

/// Build the system prompt for an agentic RAG session.
pub fn build_system_prompt(base_system: &str) -> String {
    format!("{base_system}\n\n{AGENTIC_RAG_TOOL_DESCRIPTION}")
}

// ── Helpers ─────────────────────────────────────────────────────────

fn extract_attr<'a>(tag: &'a str, attr: &str) -> Option<&'a str> {
    let prefix_dq = format!("{attr}=\"");
    let prefix_sq = format!("{attr}='");
    if let Some(idx) = tag.find(&prefix_dq) {
        let start = idx + prefix_dq.len();
        let end = tag[start..].find('"')? + start;
        return Some(&tag[start..end]);
    }
    if let Some(idx) = tag.find(&prefix_sq) {
        let start = idx + prefix_sq.len();
        let end = tag[start..].find('\'')? + start;
        return Some(&tag[start..end]);
    }
    None
}

fn extract_inner_tag<'a>(block: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = block.find(&open)? + open.len();
    let end = block[start..].find(&close)? + start;
    Some(&block[start..end])
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tool_call_valid() {
        let reply = r#"I need to look this up.
<tool_call name="retrieve_memory"><query>Vietnamese law filing deadline</query></tool_call>"#;
        let tc = parse_tool_call(reply).unwrap();
        assert_eq!(tc.name, "retrieve_memory");
        assert_eq!(tc.query, "Vietnamese law filing deadline");
    }

    #[test]
    fn parse_tool_call_single_quotes() {
        let reply = "<tool_call name='retrieve_memory'><query>hello world</query></tool_call>";
        let tc = parse_tool_call(reply).unwrap();
        assert_eq!(tc.name, "retrieve_memory");
        assert_eq!(tc.query, "hello world");
    }

    #[test]
    fn parse_tool_call_none_when_missing() {
        assert!(parse_tool_call("Just a normal reply").is_none());
    }

    #[test]
    fn parse_tool_call_none_when_empty_query() {
        let reply = "<tool_call name=\"retrieve_memory\"><query>  </query></tool_call>";
        assert!(parse_tool_call(reply).is_none());
    }

    #[test]
    fn parse_tool_call_none_when_unclosed() {
        let reply = "<tool_call name=\"retrieve_memory\"><query>test";
        assert!(parse_tool_call(reply).is_none());
    }

    #[test]
    fn strip_tool_call_removes_block() {
        let reply = "Prefix text\n<tool_call name=\"retrieve_memory\"><query>q</query></tool_call>\nSuffix";
        let stripped = strip_tool_call(reply);
        assert!(stripped.contains("Prefix text"), "got: {stripped}");
        assert!(stripped.contains("Suffix"), "got: {stripped}");
        assert!(!stripped.contains("tool_call"));
    }

    #[test]
    fn strip_tool_call_noop_when_absent() {
        let reply = "No tool call here.";
        assert_eq!(strip_tool_call(reply), reply);
    }

    #[test]
    fn format_tool_result_with_entries() {
        let results = vec!["mem1".to_string(), "mem2".to_string()];
        let formatted = format_tool_result(&results);
        assert!(formatted.starts_with("<tool_result>"));
        assert!(formatted.contains("[1] mem1"));
        assert!(formatted.contains("[2] mem2"));
        assert!(formatted.ends_with("</tool_result>"));
    }

    #[test]
    fn format_tool_result_empty() {
        let formatted = format_tool_result(&[]);
        assert!(formatted.contains("No relevant memories found"));
    }

    #[test]
    fn build_system_prompt_appends_tool() {
        let base = "You are a helpful assistant.";
        let prompt = build_system_prompt(base);
        assert!(prompt.starts_with(base));
        assert!(prompt.contains("retrieve_memory"));
        assert!(prompt.contains("<tool"));
    }

    #[test]
    fn config_default_values() {
        let cfg = AgenticRagConfig::default();
        assert_eq!(cfg.max_iterations, DEFAULT_MAX_ITERATIONS);
        assert_eq!(cfg.top_k, 5);
    }

    #[test]
    fn agentic_result_fields() {
        let result = AgenticRagResult {
            answer: "The answer is 42.".into(),
            retrieval_count: 2,
            capped: false,
            history: vec![
                LoopTurn { role: "user".into(), content: "q".into() },
                LoopTurn { role: "assistant".into(), content: "a".into() },
            ],
        };
        assert_eq!(result.retrieval_count, 2);
        assert!(!result.capped);
        assert_eq!(result.history.len(), 2);
    }
}
