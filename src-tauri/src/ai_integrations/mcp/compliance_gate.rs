//! MCP Compliance Gate — active enforcement of project governance rules.
//!
//! Instead of relying on agents to *read* and *obey* passive text rules,
//! this module tracks per-session compliance state and:
//!
//! 1. **Injects reminders** into tool results when preconditions are unmet.
//! 2. **Exposes a `brain_session_checklist` tool** that agents can query to
//!    see what they've done and what's still required.
//! 3. **Tags tool responses** with a compliance fingerprint so the user can
//!    audit whether the agent followed the protocol.
//!
//! This does NOT block tool calls (that would break MCP protocol
//! expectations) — it *annotates* them with compliance status, making
//! violations visible in the tool output text that both the agent and
//! user can see.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// ─── Compliance requirements ────────────────────────────────────────────────

/// The ordered set of compliance steps every agent session must complete.
/// Each step has a machine-checkable condition (did the agent call X?).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceStep {
    /// Agent called `brain_health`.
    HealthChecked,
    /// Agent called `brain_search` or `brain_suggest_context`.
    ContextQueried,
    /// Agent showed a visible MCP receipt (self-reported via notification).
    ReceiptShown,
    /// Agent read `rules/milestones.md` (detected via file-path in query or explicit signal).
    MilestonesRead,
    /// Agent updated `rules/completion-log.md` when completing a chunk.
    CompletionLogged,
    /// Agent updated `rules/milestones.md` (removed done row) when completing a chunk.
    MilestonesCleaned,
    /// Agent synced durable lessons to `mcp-data/shared/memory-seed.sql`.
    SeedSynced,
}

impl ComplianceStep {
    /// Human-readable label for the checklist.
    pub fn label(&self) -> &'static str {
        match self {
            Self::HealthChecked => "brain_health called",
            Self::ContextQueried => "brain_search/brain_suggest_context called with task topic",
            Self::ReceiptShown => "MCP receipt shown to user",
            Self::MilestonesRead => "rules/milestones.md read for current work queue",
            Self::CompletionLogged => "completion-log.md updated (if chunk completed)",
            Self::MilestonesCleaned => "milestones.md cleaned (done rows removed)",
            Self::SeedSynced => "mcp-data/shared/memory-seed.sql updated with lessons",
        }
    }

    /// Whether this step is mandatory before any other work begins.
    pub fn is_preflight(&self) -> bool {
        matches!(
            self,
            Self::HealthChecked | Self::ContextQueried | Self::ReceiptShown
        )
    }

    /// Whether this step is only required when the agent completes a chunk.
    pub fn is_post_chunk(&self) -> bool {
        matches!(
            self,
            Self::CompletionLogged | Self::MilestonesCleaned | Self::SeedSynced
        )
    }
}

/// All preflight steps.
pub const PREFLIGHT_STEPS: &[ComplianceStep] = &[
    ComplianceStep::HealthChecked,
    ComplianceStep::ContextQueried,
    ComplianceStep::ReceiptShown,
];

/// All post-chunk steps.
pub const POST_CHUNK_STEPS: &[ComplianceStep] = &[
    ComplianceStep::CompletionLogged,
    ComplianceStep::MilestonesCleaned,
    ComplianceStep::SeedSynced,
];

// ─── Session state ──────────────────────────────────────────────────────────

/// Tracks one agent session's compliance state. Created on first tool call,
/// persisted in memory for the server's lifetime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCompliance {
    /// Unique session identifier (derived from first request timestamp).
    pub session_id: String,
    /// Steps completed so far.
    pub completed: HashSet<ComplianceStep>,
    /// Total tool calls made in this session.
    pub tool_call_count: u32,
    /// Whether the agent has signaled it is completing a chunk.
    pub chunk_completing: bool,
    /// Timestamp of session start (Unix ms).
    pub started_at: u64,
    /// Number of compliance reminders injected into responses.
    pub reminders_injected: u32,
}

impl Default for SessionCompliance {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionCompliance {
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            session_id: format!("session-{now}"),
            completed: HashSet::new(),
            tool_call_count: 0,
            chunk_completing: false,
            started_at: now,
            reminders_injected: 0,
        }
    }

    /// Mark a step as completed.
    pub fn complete_step(&mut self, step: ComplianceStep) {
        self.completed.insert(step);
    }

    /// Check if all preflight steps are done.
    pub fn preflight_done(&self) -> bool {
        PREFLIGHT_STEPS.iter().all(|s| self.completed.contains(s))
    }

    /// Check if all post-chunk steps are done (only relevant when chunk_completing).
    pub fn post_chunk_done(&self) -> bool {
        if !self.chunk_completing {
            return true; // not applicable
        }
        POST_CHUNK_STEPS.iter().all(|s| self.completed.contains(s))
    }

    /// Get the list of outstanding preflight steps.
    pub fn pending_preflight(&self) -> Vec<ComplianceStep> {
        PREFLIGHT_STEPS
            .iter()
            .filter(|s| !self.completed.contains(s))
            .copied()
            .collect()
    }

    /// Get the list of outstanding post-chunk steps.
    pub fn pending_post_chunk(&self) -> Vec<ComplianceStep> {
        if !self.chunk_completing {
            return vec![];
        }
        POST_CHUNK_STEPS
            .iter()
            .filter(|s| !self.completed.contains(s))
            .copied()
            .collect()
    }

    /// Build the compliance reminder text to inject into a tool response.
    /// Returns `None` if the agent is compliant.
    pub fn compliance_reminder(&self) -> Option<String> {
        let pending = self.pending_preflight();
        if pending.is_empty() {
            return None;
        }

        let mut msg = String::from(
            "\n\n⚠️ [MCP COMPLIANCE] Preflight steps incomplete. Required before proceeding:\n",
        );
        for step in &pending {
            msg.push_str(&format!("  • {}\n", step.label()));
        }
        msg.push_str("Call `brain_session_checklist` to see full status.\n");
        Some(msg)
    }

    /// Build a full checklist summary (for the `brain_session_checklist` tool).
    pub fn checklist_summary(&self) -> String {
        let mut out = String::from("# MCP Session Compliance Checklist\n\n");
        out.push_str("## Preflight (mandatory before any work)\n");
        for step in PREFLIGHT_STEPS {
            let done = self.completed.contains(step);
            out.push_str(&format!(
                "  {} {}\n",
                if done { "✅" } else { "❌" },
                step.label()
            ));
        }
        out.push_str("\n## Session (recommended)\n");
        let done = self.completed.contains(&ComplianceStep::MilestonesRead);
        out.push_str(&format!(
            "  {} {}\n",
            if done { "✅" } else { "⬜" },
            ComplianceStep::MilestonesRead.label()
        ));

        if self.chunk_completing {
            out.push_str("\n## Post-Chunk (mandatory when completing a chunk)\n");
            for step in POST_CHUNK_STEPS {
                let done = self.completed.contains(step);
                out.push_str(&format!(
                    "  {} {}\n",
                    if done { "✅" } else { "❌" },
                    step.label()
                ));
            }
        }

        out.push_str(&format!(
            "\n---\nSession: {} | Tool calls: {} | Reminders injected: {}\n",
            self.session_id, self.tool_call_count, self.reminders_injected
        ));
        out
    }
}

// ─── Gate logic — call from the router ──────────────────────────────────────

/// Shared compliance state, one per MCP server lifetime.
pub type ComplianceState = Arc<Mutex<SessionCompliance>>;

/// Create a new compliance state for a fresh server session.
pub fn new_compliance_state() -> ComplianceState {
    Arc::new(Mutex::new(SessionCompliance::new()))
}

/// Called by the router AFTER dispatching a tool call. Updates compliance
/// state based on which tool was called and its arguments.
pub async fn on_tool_called(
    state: &ComplianceState,
    tool_name: &str,
    args: &serde_json::Value,
) {
    let mut s = state.lock().await;
    s.tool_call_count += 1;

    match tool_name {
        "brain_health" => {
            s.complete_step(ComplianceStep::HealthChecked);
        }
        "brain_search" | "brain_suggest_context" => {
            s.complete_step(ComplianceStep::ContextQueried);
        }
        _ => {}
    }

    // Detect milestones read from query content
    if let Some(query) = args.get("query").and_then(|v| v.as_str()) {
        let q_lower = query.to_lowercase();
        if q_lower.contains("milestone") || q_lower.contains("completion-log") {
            s.complete_step(ComplianceStep::MilestonesRead);
        }
    }
}

/// Called when the agent sends a compliance notification (fire-and-forget).
/// Agents signal steps like "receipt_shown" or "chunk_completing" via
/// a `compliance/signal` notification.
pub async fn on_compliance_signal(state: &ComplianceState, signal: &str) {
    let mut s = state.lock().await;
    match signal {
        "receipt_shown" => s.complete_step(ComplianceStep::ReceiptShown),
        "milestones_read" => s.complete_step(ComplianceStep::MilestonesRead),
        "completion_logged" => s.complete_step(ComplianceStep::CompletionLogged),
        "milestones_cleaned" => s.complete_step(ComplianceStep::MilestonesCleaned),
        "seed_synced" => s.complete_step(ComplianceStep::SeedSynced),
        "chunk_completing" => {
            s.chunk_completing = true;
        }
        _ => {} // Unknown signals ignored
    }
}

/// Generate the compliance annotation to append to a tool response.
/// Returns `None` when the agent is fully compliant or past the grace
/// period (first 3 calls are always annotated if incomplete).
pub async fn get_response_annotation(state: &ComplianceState) -> Option<String> {
    let mut s = state.lock().await;

    // Always annotate the first few calls if preflight is incomplete
    if !s.preflight_done() && s.tool_call_count <= 5 {
        s.reminders_injected += 1;
        return s.compliance_reminder();
    }

    // After 5 calls, still remind every 10th call if not compliant
    if !s.preflight_done() && s.tool_call_count % 10 == 0 {
        s.reminders_injected += 1;
        return s.compliance_reminder();
    }

    // Post-chunk reminders
    if s.chunk_completing && !s.post_chunk_done() && s.tool_call_count % 5 == 0 {
        let pending = s.pending_post_chunk();
        if !pending.is_empty() {
            s.reminders_injected += 1;
            let mut msg =
                String::from("\n\n⚠️ [MCP COMPLIANCE] Chunk completion steps outstanding:\n");
            for step in &pending {
                msg.push_str(&format!("  • {}\n", step.label()));
            }
            return Some(msg);
        }
    }

    None
}

/// Generate the full checklist (for `brain_session_checklist` tool).
pub async fn get_checklist(state: &ComplianceState) -> String {
    let s = state.lock().await;
    s.checklist_summary()
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_preflight_tracking() {
        let state = new_compliance_state();

        // Initially nothing is done
        {
            let s = state.lock().await;
            assert!(!s.preflight_done());
            assert_eq!(s.pending_preflight().len(), 3);
        }

        // Call brain_health
        on_tool_called(&state, "brain_health", &json!({})).await;
        {
            let s = state.lock().await;
            assert!(s.completed.contains(&ComplianceStep::HealthChecked));
            assert!(!s.preflight_done());
        }

        // Call brain_search
        on_tool_called(&state, "brain_search", &json!({"query": "test"})).await;
        {
            let s = state.lock().await;
            assert!(s.completed.contains(&ComplianceStep::ContextQueried));
            assert!(!s.preflight_done()); // still missing receipt
        }

        // Signal receipt
        on_compliance_signal(&state, "receipt_shown").await;
        {
            let s = state.lock().await;
            assert!(s.preflight_done());
            assert!(s.compliance_reminder().is_none());
        }
    }

    #[tokio::test]
    async fn test_post_chunk_tracking() {
        let state = new_compliance_state();

        // Complete preflight
        on_tool_called(&state, "brain_health", &json!({})).await;
        on_tool_called(&state, "brain_search", &json!({"query": "x"})).await;
        on_compliance_signal(&state, "receipt_shown").await;

        // Start completing a chunk
        on_compliance_signal(&state, "chunk_completing").await;
        {
            let s = state.lock().await;
            assert!(s.chunk_completing);
            assert!(!s.post_chunk_done());
            assert_eq!(s.pending_post_chunk().len(), 3);
        }

        // Complete all post-chunk steps
        on_compliance_signal(&state, "completion_logged").await;
        on_compliance_signal(&state, "milestones_cleaned").await;
        on_compliance_signal(&state, "seed_synced").await;
        {
            let s = state.lock().await;
            assert!(s.post_chunk_done());
        }
    }

    #[tokio::test]
    async fn test_reminder_injection() {
        let state = new_compliance_state();

        // First call should get a reminder
        on_tool_called(&state, "brain_search", &json!({"query": "x"})).await;
        let annotation = get_response_annotation(&state).await;
        assert!(annotation.is_some());
        assert!(annotation.unwrap().contains("COMPLIANCE"));
    }

    #[tokio::test]
    async fn test_checklist_output() {
        let state = new_compliance_state();
        on_tool_called(&state, "brain_health", &json!({})).await;

        let checklist = get_checklist(&state).await;
        assert!(checklist.contains("✅ brain_health called"));
        assert!(checklist.contains("❌ brain_search"));
    }
}
