//! DB-backed verbose tool-output offload (CTX-OFFLOAD-1b).
//!
//! Companion to the file-based [`crate::coding::offload`] spill. When a tool
//! call returns a result whose length exceeds `threshold_chars`, this hook:
//!
//! 1. Builds a compact textual summary (head + tail preview).
//! 2. Inserts a `MemoryType::Context` row carrying the summary as content
//!    (tagged `tool_output,offloaded,call:<id>`) — the row id becomes the
//!    re-inflation handle.
//! 3. Persists the full raw bytes via
//!    [`MemoryStore::add_offload_payload`](crate::memory::MemoryStore::add_offload_payload).
//! 4. Replaces the in-context tool result with a JSON envelope
//!    `{kind: "tool_output_ref", id, summary, byte_count}` so the agent can
//!    re-inflate on demand via the `brain_drilldown_payload` MCP tool or
//!    the `memory_drilldown_payload` Tauri command.
//!
//! Failure mode: if any DB operation fails (lock poisoned, schema older
//! than V23, disk full, …) the hook returns the original tool result
//! unchanged. A broken turn is worse than a big context.

use std::sync::{Arc, Mutex};

use crate::coding::offload::OFFLOAD_CHAR_THRESHOLD;
use crate::coding::runtime_hooks::{
    AgentHook, AgentState, RunContext, ToolCall, ToolCallNext, ToolCallResult,
};
use crate::memory::store::{MemoryStore, MemoryType, NewMemory};

/// DB-backed tool-output offload hook.
pub struct OffloadToolOutputHook {
    store: Arc<Mutex<MemoryStore>>,
    threshold_chars: usize,
}

impl OffloadToolOutputHook {
    /// Construct a hook using the default [`OFFLOAD_CHAR_THRESHOLD`].
    pub fn new(store: Arc<Mutex<MemoryStore>>) -> Self {
        Self {
            store,
            threshold_chars: OFFLOAD_CHAR_THRESHOLD,
        }
    }

    /// Override the size threshold (characters).
    pub fn with_threshold(mut self, threshold_chars: usize) -> Self {
        self.threshold_chars = threshold_chars;
        self
    }

    /// Build a head + tail preview for the offloaded content.
    fn build_summary(content: &str) -> String {
        const HEAD_CHARS: usize = 400;
        const TAIL_CHARS: usize = 200;
        if content.len() <= HEAD_CHARS + TAIL_CHARS {
            return content.to_string();
        }
        // Char-boundary safe slicing.
        let head_end = floor_char_boundary(content, HEAD_CHARS);
        let tail_start = ceil_char_boundary(content, content.len() - TAIL_CHARS);
        format!(
            "{}\n\n…[{} chars offloaded]…\n\n{}",
            &content[..head_end],
            content.len() - head_end - (content.len() - tail_start),
            &content[tail_start..],
        )
    }

    /// Attempt the full offload pipeline. On any failure, returns `None`
    /// so the caller can fall back to the original content.
    fn try_offload(&self, call: &ToolCall, content: &str) -> Option<String> {
        let store = self.store.lock().ok()?;
        let bytes = content.as_bytes();
        let byte_count = bytes.len() as i64;
        let summary = Self::build_summary(content);
        let tags = format!("tool_output,offloaded,call:{}", call.id);
        let entry = store
            .add(NewMemory {
                content: summary.clone(),
                tags,
                importance: 2,
                memory_type: MemoryType::Context,
                source_url: None,
                source_hash: None,
                expires_at: None,
                created_at: None,
            })
            .ok()?;
        store
            .add_offload_payload(entry.id, bytes, "text/plain")
            .ok()?;
        let placeholder = serde_json::json!({
            "kind": "tool_output_ref",
            "id": entry.id,
            "summary": summary,
            "byte_count": byte_count,
        });
        Some(placeholder.to_string())
    }
}

impl AgentHook for OffloadToolOutputHook {
    fn hook_name(&self) -> &'static str {
        "tool_result_offload_db"
    }

    fn wrap_tool_call(
        &self,
        ctx: &RunContext,
        state: &mut AgentState,
        call: &ToolCall,
        next: &ToolCallNext<'_>,
    ) -> Result<ToolCallResult, String> {
        let result = next(ctx, state, call)?;
        if result.content.len() <= self.threshold_chars {
            return Ok(result);
        }
        match self.try_offload(call, &result.content) {
            Some(placeholder) => Ok(ToolCallResult::new(placeholder)),
            // Fall back to the unmodified content on any failure.
            None => Ok(result),
        }
    }
}

// `str::floor_char_boundary` and `ceil_char_boundary` are unstable on the
// pinned toolchain. Provide local equivalents that walk back/forward to the
// nearest UTF-8 boundary.
fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn ceil_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::runtime_hooks::{run_tool_call_hooks, AgentState, RunContext};
    use std::path::PathBuf;

    fn make_store() -> Arc<Mutex<MemoryStore>> {
        Arc::new(Mutex::new(MemoryStore::in_memory()))
    }

    #[test]
    fn small_results_pass_through_unchanged() {
        let store = make_store();
        let hook = Arc::new(OffloadToolOutputHook::new(Arc::clone(&store)));
        let hooks: Vec<Arc<dyn AgentHook>> = vec![hook];
        let ctx = RunContext::new("s", "a", PathBuf::from("."));
        let mut state = AgentState::default();
        let call = ToolCall::new("call-1", "tool", "{}");

        let result =
            run_tool_call_hooks(&hooks, &ctx, &mut state, &call, &|_, _, _| {
                Ok(ToolCallResult::new("short output".to_string()))
            })
            .unwrap();

        assert_eq!(result.content, "short output");
        // No memory row should have been inserted.
        let total = store
            .lock()
            .unwrap()
            .offload_payload_total_bytes()
            .unwrap();
        assert_eq!(total, 0);
    }

    #[test]
    fn large_results_are_offloaded_with_tool_output_ref() {
        let store = make_store();
        let hook = Arc::new(
            OffloadToolOutputHook::new(Arc::clone(&store)).with_threshold(100),
        );
        let hooks: Vec<Arc<dyn AgentHook>> = vec![hook];
        let ctx = RunContext::new("s", "a", PathBuf::from("."));
        let mut state = AgentState::default();
        let call = ToolCall::new("call-42", "tool", "{}");
        let big = "y".repeat(2_048);

        let result =
            run_tool_call_hooks(&hooks, &ctx, &mut state, &call, &|_, _, _| {
                Ok(ToolCallResult::new(big.clone()))
            })
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result.content)
            .expect("placeholder must be valid JSON");
        assert_eq!(parsed["kind"], "tool_output_ref");
        assert_eq!(parsed["byte_count"], 2_048);
        let id = parsed["id"].as_i64().expect("id present");
        assert!(id > 0);

        // Round-trip: retrieving the payload returns the original bytes.
        let store_guard = store.lock().unwrap();
        let payload = store_guard
            .get_offload_payload(id)
            .unwrap()
            .expect("payload row present");
        assert_eq!(payload.byte_count, 2_048);
        assert_eq!(payload.payload, big.as_bytes());
        assert_eq!(payload.mime_type, "text/plain");
    }

    #[test]
    fn summary_keeps_head_and_tail_with_marker() {
        let mut content = String::new();
        content.push_str(&"H".repeat(500));
        content.push_str(&"M".repeat(2_000));
        content.push_str(&"T".repeat(500));
        let summary = OffloadToolOutputHook::build_summary(&content);
        assert!(summary.starts_with("HHHH"));
        assert!(summary.ends_with("TTTT"));
        assert!(summary.contains("chars offloaded"));
        // Summary is much shorter than the original.
        assert!(summary.len() < content.len() / 2);
    }
}
