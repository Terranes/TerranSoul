//! Context budget manager for long coding sessions (Chunk 28.10).
//!
//! ## Problem
//!
//! Long autonomous coding workflows (10+ planner/coder/reviewer cycles)
//! accumulate context that eventually overflows the LLM's token window.
//! Models silently truncate, lose the thread of the task, and start
//! repeating or contradicting prior work.
//!
//! ## Solution
//!
//! This module provides a **priority-based context budget** that:
//!
//! 1. Estimates token count per context section (approximate: 1 token ≈ 4 chars).
//! 2. Assigns priority to each section (errors > active plan > recent results > old history).
//! 3. When total exceeds the budget, prunes lowest-priority sections first.
//! 4. Generates a rolling summary of pruned content so the model retains
//!    high-level awareness of what was discarded.
//! 5. Chains handoff states across sessions for multi-session continuity.
//!
//! ## Integration
//!
//! Called by `run_coding_task` before prompt assembly. The budget manager
//! sits between context loading and prompt building:
//!
//! ```text
//! load_workflow_context() → ContextBudget::fit() → CodingPrompt::build()
//! ```

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default token budget for a single coding-workflow invocation.
/// Conservative: leaves room for the model's response.
pub const DEFAULT_TOKEN_BUDGET: usize = 24_000;

/// Approximate characters per token (English text, conservative).
const CHARS_PER_TOKEN: usize = 4;

/// Priority levels for context sections — higher value = higher priority
/// (kept last when budget is exceeded).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SectionPriority {
    /// Old completed steps, background info — first to be pruned.
    Background = 0,
    /// Previously completed plan steps.
    CompletedSteps = 1,
    /// Reference documentation snippets.
    Documentation = 2,
    /// Results from prior cycles (test output, build logs).
    PriorResults = 3,
    /// The active plan / next steps.
    ActivePlan = 4,
    /// Recent errors and failures — critical for not repeating mistakes.
    RecentErrors = 5,
    /// The handoff/resumption block — must always survive.
    Handoff = 6,
    /// System instructions — never pruned.
    System = 7,
}

/// A single section of context with its priority and content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSection {
    /// Human-readable label for this section (e.g. "plan", "test-output").
    pub label: String,
    /// Priority determines prune order (lowest pruned first).
    pub priority: SectionPriority,
    /// The actual text content of this section.
    pub content: String,
    /// Whether this section can be summarized (vs hard-dropped).
    pub summarizable: bool,
}

impl ContextSection {
    /// Estimate token count for this section.
    pub fn estimated_tokens(&self) -> usize {
        estimate_tokens(&self.content)
    }
}

/// Configuration for the context budget manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Maximum tokens allowed in the assembled context.
    pub max_tokens: usize,
    /// Reserve tokens for the model's response (excluded from budget).
    pub response_reserve: usize,
    /// Maximum tokens for a single summary of pruned content.
    pub summary_max_tokens: usize,
    /// Whether to generate summaries of pruned sections.
    pub generate_summaries: bool,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_tokens: DEFAULT_TOKEN_BUDGET,
            response_reserve: 4_000,
            summary_max_tokens: 500,
            generate_summaries: true,
        }
    }
}

/// Result of fitting sections into the budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetResult {
    /// Sections that fit within the budget (in original order).
    pub kept: Vec<ContextSection>,
    /// Sections that were pruned (lowest priority first).
    pub pruned: Vec<PrunedSection>,
    /// Total estimated tokens in the kept sections.
    pub total_tokens: usize,
    /// Budget that was applied.
    pub budget_tokens: usize,
    /// Generated summary of pruned content (if enabled).
    pub pruned_summary: Option<String>,
}

/// Record of a pruned section for diagnostics/summaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrunedSection {
    pub label: String,
    pub priority: SectionPriority,
    pub original_tokens: usize,
}

/// Session chain state for multi-session workflows.
///
/// Tracks cumulative progress across multiple coding sessions so each
/// new session starts with awareness of the full history without needing
/// to carry all the raw context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionChain {
    /// Unique identifier for the overarching workflow.
    pub workflow_id: String,
    /// Current session number (1-indexed).
    pub session_number: u32,
    /// Rolling summaries from prior sessions (newest last).
    pub prior_summaries: Vec<SessionSummary>,
    /// Total tokens consumed across all prior sessions.
    pub cumulative_tokens: u64,
    /// Total steps completed across all prior sessions.
    pub cumulative_steps_completed: u32,
}

/// Summary of a completed session in the chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Session number this summary represents.
    pub session_number: u32,
    /// Compact prose summary of what was accomplished.
    pub summary: String,
    /// Steps completed in this session.
    pub steps_completed: u32,
    /// Unix-ms timestamp when this session ended.
    pub ended_at: i64,
}

impl SessionChain {
    /// Start a new session chain for a workflow.
    pub fn new(workflow_id: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            session_number: 1,
            prior_summaries: Vec::new(),
            cumulative_tokens: 0,
            cumulative_steps_completed: 0,
        }
    }

    /// Advance to the next session, recording the just-completed session's summary.
    pub fn advance(&mut self, summary: String, steps_completed: u32, tokens_used: u64) {
        self.prior_summaries.push(SessionSummary {
            session_number: self.session_number,
            summary,
            steps_completed,
            ended_at: now_ms(),
        });
        self.cumulative_tokens += tokens_used;
        self.cumulative_steps_completed += steps_completed;
        self.session_number += 1;

        // Keep only the last 10 session summaries to bound chain growth.
        if self.prior_summaries.len() > 10 {
            let excess = self.prior_summaries.len() - 10;
            self.prior_summaries.drain(..excess);
        }
    }

    /// Build a context section from the session chain history.
    pub fn as_context_section(&self) -> ContextSection {
        let mut content = format!(
            "[SESSION CHAIN — workflow: {}, session #{}, {} prior steps completed]\n",
            self.workflow_id, self.session_number, self.cumulative_steps_completed
        );
        for s in &self.prior_summaries {
            content.push_str(&format!(
                "- Session {}: {} ({} steps)\n",
                s.session_number, s.summary, s.steps_completed
            ));
        }
        ContextSection {
            label: "session-chain".to_string(),
            priority: SectionPriority::Handoff,
            content,
            summarizable: false,
        }
    }
}

/// Fit a list of context sections into the token budget.
///
/// Sections are sorted by priority; lowest-priority sections are pruned
/// first when the total exceeds the budget. Within the same priority
/// level, larger sections are pruned first.
///
/// Returns a [`BudgetResult`] with the kept/pruned partition and
/// optional summary of what was dropped.
pub fn fit_to_budget(sections: Vec<ContextSection>, config: &BudgetConfig) -> BudgetResult {
    let effective_budget = config.max_tokens.saturating_sub(config.response_reserve);

    // If everything fits, return immediately.
    let total: usize = sections.iter().map(|s| s.estimated_tokens()).sum();
    if total <= effective_budget {
        return BudgetResult {
            total_tokens: total,
            budget_tokens: effective_budget,
            kept: sections,
            pruned: Vec::new(),
            pruned_summary: None,
        };
    }

    // Build indexed list for sorting while preserving original order.
    let mut indexed: Vec<(usize, &ContextSection)> = sections.iter().enumerate().collect();

    // Sort by priority (ascending) then by size (descending) for prune order.
    indexed.sort_by(|a, b| {
        a.1.priority
            .cmp(&b.1.priority)
            .then_with(|| b.1.estimated_tokens().cmp(&a.1.estimated_tokens()))
    });

    let mut prune_set: Vec<usize> = Vec::new();
    let mut pruned_tokens = 0usize;

    // Prune from front (lowest priority, largest first) until we're under budget.
    for &(idx, section) in &indexed {
        if total - pruned_tokens <= effective_budget {
            break;
        }
        // Never prune System-priority sections.
        if section.priority == SectionPriority::System {
            continue;
        }
        prune_set.push(idx);
        pruned_tokens += section.estimated_tokens();
    }

    let mut kept = Vec::new();
    let mut pruned = Vec::new();

    for (idx, section) in sections.into_iter().enumerate() {
        if prune_set.contains(&idx) {
            pruned.push(PrunedSection {
                label: section.label.clone(),
                priority: section.priority,
                original_tokens: section.estimated_tokens(),
            });
        } else {
            kept.push(section);
        }
    }

    let kept_tokens: usize = kept.iter().map(|s| s.estimated_tokens()).sum();

    // Build a brief summary of pruned content.
    let pruned_summary = if config.generate_summaries && !pruned.is_empty() {
        Some(build_pruned_summary(&pruned))
    } else {
        None
    };

    // If we generated a summary, add it as a Background section.
    let total_tokens = if let Some(ref summary) = pruned_summary {
        kept_tokens + estimate_tokens(summary)
    } else {
        kept_tokens
    };

    BudgetResult {
        kept,
        pruned,
        total_tokens,
        budget_tokens: effective_budget,
        pruned_summary,
    }
}

/// Build a compact textual summary of what was pruned.
fn build_pruned_summary(pruned: &[PrunedSection]) -> String {
    let total_pruned_tokens: usize = pruned.iter().map(|p| p.original_tokens).sum();
    let labels: Vec<&str> = pruned.iter().map(|p| p.label.as_str()).collect();
    format!(
        "[CONTEXT PRUNED: {} sections ({} tokens) removed to fit budget: {}]",
        pruned.len(),
        total_pruned_tokens,
        labels.join(", ")
    )
}

/// Estimate token count from character length.
pub fn estimate_tokens(text: &str) -> usize {
    // Conservative: ~4 chars per token for English.
    // More accurate for code which tends to have shorter tokens.
    text.len().div_ceil(CHARS_PER_TOKEN)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_section(label: &str, priority: SectionPriority, chars: usize) -> ContextSection {
        ContextSection {
            label: label.to_string(),
            priority,
            content: "x".repeat(chars),
            summarizable: true,
        }
    }

    #[test]
    fn everything_fits_returns_all() {
        let sections = vec![
            make_section("a", SectionPriority::Background, 100),
            make_section("b", SectionPriority::ActivePlan, 200),
        ];
        let config = BudgetConfig {
            max_tokens: 1000,
            response_reserve: 0,
            ..Default::default()
        };
        let result = fit_to_budget(sections, &config);
        assert_eq!(result.kept.len(), 2);
        assert!(result.pruned.is_empty());
        assert!(result.pruned_summary.is_none());
    }

    #[test]
    fn prunes_lowest_priority_first() {
        let sections = vec![
            make_section("background", SectionPriority::Background, 400), // 100 tokens
            make_section("errors", SectionPriority::RecentErrors, 400),   // 100 tokens
            make_section("plan", SectionPriority::ActivePlan, 400),       // 100 tokens
        ];
        let config = BudgetConfig {
            max_tokens: 250, // Only room for ~250 tokens total
            response_reserve: 0,
            generate_summaries: false,
            ..Default::default()
        };
        let result = fit_to_budget(sections, &config);
        assert_eq!(result.kept.len(), 2);
        assert_eq!(result.pruned.len(), 1);
        assert_eq!(result.pruned[0].label, "background");
    }

    #[test]
    fn never_prunes_system_priority() {
        let sections = vec![
            make_section("system", SectionPriority::System, 800), // 200 tokens
            make_section("bg", SectionPriority::Background, 800), // 200 tokens
        ];
        let config = BudgetConfig {
            max_tokens: 250,
            response_reserve: 0,
            generate_summaries: false,
            ..Default::default()
        };
        let result = fit_to_budget(sections, &config);
        // System is never pruned even if it alone exceeds budget
        assert!(result.kept.iter().any(|s| s.label == "system"));
        assert!(result.pruned.iter().any(|p| p.label == "bg"));
    }

    #[test]
    fn prunes_larger_sections_first_within_same_priority() {
        let sections = vec![
            make_section("small-bg", SectionPriority::Background, 100), // 25 tokens
            make_section("large-bg", SectionPriority::Background, 800), // 200 tokens
            make_section("plan", SectionPriority::ActivePlan, 400),     // 100 tokens
        ];
        let config = BudgetConfig {
            max_tokens: 200,
            response_reserve: 0,
            generate_summaries: false,
            ..Default::default()
        };
        let result = fit_to_budget(sections, &config);
        // large-bg (200 tokens) should be pruned first
        assert!(result.pruned.iter().any(|p| p.label == "large-bg"));
    }

    #[test]
    fn generates_pruned_summary() {
        let sections = vec![
            make_section("docs", SectionPriority::Documentation, 2000),
            make_section("plan", SectionPriority::ActivePlan, 400),
        ];
        let config = BudgetConfig {
            max_tokens: 200,
            response_reserve: 0,
            generate_summaries: true,
            ..Default::default()
        };
        let result = fit_to_budget(sections, &config);
        assert!(result.pruned_summary.is_some());
        let summary = result.pruned_summary.unwrap();
        assert!(summary.contains("CONTEXT PRUNED"));
        assert!(summary.contains("docs"));
    }

    #[test]
    fn estimate_tokens_basic() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("a".repeat(100).as_str()), 25);
    }

    #[test]
    fn response_reserve_reduces_effective_budget() {
        let sections = vec![make_section("a", SectionPriority::ActivePlan, 400)]; // 100 tokens
        let config = BudgetConfig {
            max_tokens: 150,
            response_reserve: 100, // effective budget = 50
            generate_summaries: false,
            ..Default::default()
        };
        let result = fit_to_budget(sections, &config);
        assert_eq!(result.budget_tokens, 50);
        // 100 tokens > 50 budget, so it should be pruned
        assert_eq!(result.pruned.len(), 1);
    }

    #[test]
    fn session_chain_new() {
        let chain = SessionChain::new("wf-001");
        assert_eq!(chain.workflow_id, "wf-001");
        assert_eq!(chain.session_number, 1);
        assert!(chain.prior_summaries.is_empty());
    }

    #[test]
    fn session_chain_advance() {
        let mut chain = SessionChain::new("wf-001");
        chain.advance("Completed setup".to_string(), 3, 5000);
        assert_eq!(chain.session_number, 2);
        assert_eq!(chain.cumulative_steps_completed, 3);
        assert_eq!(chain.cumulative_tokens, 5000);
        assert_eq!(chain.prior_summaries.len(), 1);
        assert_eq!(chain.prior_summaries[0].summary, "Completed setup");
    }

    #[test]
    fn session_chain_caps_at_10_summaries() {
        let mut chain = SessionChain::new("wf-001");
        for i in 0..15 {
            chain.advance(format!("session {i}"), 1, 1000);
        }
        assert_eq!(chain.prior_summaries.len(), 10);
        assert_eq!(chain.session_number, 16);
        // Oldest summaries should be dropped
        assert_eq!(chain.prior_summaries[0].session_number, 6);
    }

    #[test]
    fn session_chain_as_context_section() {
        let mut chain = SessionChain::new("wf-001");
        chain.advance("Did step A".to_string(), 2, 3000);
        chain.advance("Did step B".to_string(), 1, 2000);
        let section = chain.as_context_section();
        assert_eq!(section.priority, SectionPriority::Handoff);
        assert!(section.content.contains("SESSION CHAIN"));
        assert!(section.content.contains("Did step A"));
        assert!(section.content.contains("Did step B"));
        assert!(section.content.contains("3 prior steps completed"));
    }

    #[test]
    fn budget_config_default() {
        let cfg = BudgetConfig::default();
        assert_eq!(cfg.max_tokens, DEFAULT_TOKEN_BUDGET);
        assert_eq!(cfg.response_reserve, 4_000);
        assert!(cfg.generate_summaries);
    }

    #[test]
    fn serde_roundtrip_budget_result() {
        let result = BudgetResult {
            kept: vec![make_section("plan", SectionPriority::ActivePlan, 100)],
            pruned: vec![PrunedSection {
                label: "bg".to_string(),
                priority: SectionPriority::Background,
                original_tokens: 50,
            }],
            total_tokens: 25,
            budget_tokens: 100,
            pruned_summary: Some("pruned bg".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: BudgetResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kept.len(), 1);
        assert_eq!(back.pruned.len(), 1);
        assert_eq!(back.total_tokens, 25);
    }

    #[test]
    fn serde_roundtrip_session_chain() {
        let mut chain = SessionChain::new("wf-test");
        chain.advance("done".to_string(), 5, 10000);
        let json = serde_json::to_string(&chain).unwrap();
        let back: SessionChain = serde_json::from_str(&json).unwrap();
        assert_eq!(back.workflow_id, "wf-test");
        assert_eq!(back.session_number, 2);
        assert_eq!(back.prior_summaries.len(), 1);
    }

    #[test]
    fn empty_sections_returns_empty_result() {
        let config = BudgetConfig::default();
        let result = fit_to_budget(Vec::new(), &config);
        assert!(result.kept.is_empty());
        assert!(result.pruned.is_empty());
        assert_eq!(result.total_tokens, 0);
    }

    #[test]
    fn section_priority_ordering() {
        assert!(SectionPriority::Background < SectionPriority::CompletedSteps);
        assert!(SectionPriority::CompletedSteps < SectionPriority::Documentation);
        assert!(SectionPriority::Documentation < SectionPriority::PriorResults);
        assert!(SectionPriority::PriorResults < SectionPriority::ActivePlan);
        assert!(SectionPriority::ActivePlan < SectionPriority::RecentErrors);
        assert!(SectionPriority::RecentErrors < SectionPriority::Handoff);
        assert!(SectionPriority::Handoff < SectionPriority::System);
    }
}
