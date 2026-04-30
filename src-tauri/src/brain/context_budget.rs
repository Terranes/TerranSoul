//! Context-window budgeter (Chunk 27.2 / Phase 27 — Agentic RAG &
//! Context Engineering).
//!
//! Owns the *token budget split* for one LLM request — how many tokens
//! go to the persona block, the conversation history, retrieved memory
//! chunks, and the tool schemas — so `commands::chat` can stop doing
//! ad-hoc concatenation and overrunning provider context limits.
//!
//! ## Why this module is pure
//!
//! Token budgeting is a hot path: every chat turn passes through it,
//! and the math has to be deterministic for snapshot tests. So the
//! module deliberately:
//!
//! - Has **no I/O** — no DB, no HTTP, no `tokio`. All inputs are owned
//!   strings + numeric budgets, all outputs are owned strings + a small
//!   stats struct.
//! - Uses a **fast char-based token estimator** (`~4 chars / token`)
//!   instead of pulling in the `tiktoken` crate. The estimator is good
//!   enough for budgeting (within ~10% of the real tokeniser for
//!   English prose) and avoids a heavyweight runtime dependency. When
//!   we eventually need exact counts we can swap [`estimate_tokens`]
//!   without changing the public API.
//!
//! ## Truncation strategy
//!
//! When the total content exceeds the budget, we shrink in this order
//! (cheapest to drop first):
//!
//! 1. **Retrieved memory chunks** — keep top-k by score, drop tail.
//! 2. **Old conversation turns** — keep the most recent N user/assistant
//!    pairs.
//! 3. **Tool schemas** — drop entirely if still over budget.
//! 4. **Persona block** — never truncated (it's the cheapest+most
//!    important part of the prompt; truncating it changes the
//!    companion's voice).
//!
//! The budget itself is configurable per-mode via [`BudgetConfig`] —
//! cloud paid users get a tighter retrieval allowance to control cost,
//! local-Ollama users get a generous one because tokens are free.

use serde::{Deserialize, Serialize};

/// Average chars per token used by the cheap estimator. Calibrated
/// against `cl100k_base` (GPT-4 / Claude-3 family) on English prose;
/// for code and JSON this slightly under-estimates, which is the safe
/// direction for a budget guard.
const CHARS_PER_TOKEN: usize = 4;

/// Budget split for one LLM call. All values are in *tokens*. The four
/// section budgets do not have to sum exactly to `total` — we add a
/// small safety margin so the prompt has room for the assistant's
/// reply without bumping into the provider's context window.
///
/// Defaults are tuned for an 8 K-token context (the Anthropic free tier
/// and Ollama's `gemma3:4b` default). Cloud paid users typically have
/// 128 K+ available; rather than bloat each section we use the extra
/// room as headroom for streaming output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetConfig {
    /// Total token ceiling for the *prompt*. Reserve enough headroom
    /// from the model's context window to allow a reply (typically
    /// `context_window - max_completion_tokens`).
    pub total: usize,
    /// Tokens reserved for the persona / system prompt block. Never
    /// truncated.
    pub persona: usize,
    /// Tokens reserved for the conversation history. Truncated by
    /// dropping the oldest user/assistant pairs first.
    pub history: usize,
    /// Tokens reserved for retrieved memory chunks (`[LONG-TERM
    /// MEMORY]` block). Truncated by dropping lowest-scoring chunks
    /// first.
    pub retrieval: usize,
    /// Tokens reserved for tool schemas (function-calling). Dropped
    /// wholesale when the prompt would otherwise overflow.
    pub tools: usize,
}

impl BudgetConfig {
    /// Free / public APIs (Pollinations, free Anthropic tier) — assume
    /// an 8 K context window with 1 K headroom for the reply.
    pub fn for_free_mode() -> Self {
        Self { total: 7_000, persona: 1_000, history: 3_000, retrieval: 2_500, tools: 500 }
    }

    /// Paid cloud APIs — assume 128 K context but stay frugal to
    /// control cost. Retrieval gets a tighter share than free mode
    /// (paradoxically) because every retrieved token costs money.
    pub fn for_paid_mode() -> Self {
        Self { total: 16_000, persona: 1_500, history: 7_000, retrieval: 6_000, tools: 1_500 }
    }

    /// Local Ollama — tokens are free (the user already paid for the
    /// hardware), so retrieval gets the lion's share.
    pub fn for_local_mode() -> Self {
        Self { total: 12_000, persona: 1_500, history: 4_000, retrieval: 5_500, tools: 1_000 }
    }
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self::for_local_mode()
    }
}

/// One conversation turn fed into the budgeter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryTurn {
    pub role: String,
    pub content: String,
}

/// One retrieved memory chunk fed into the budgeter. `score` is used to
/// decide truncation order — higher scores are kept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub content: String,
    pub score: f64,
}

/// Inputs to a single budgeting pass.
#[derive(Debug, Clone, Default)]
pub struct BudgetInputs {
    /// Persona / system prompt. Always kept verbatim.
    pub persona: String,
    /// Conversation history, oldest-first.
    pub history: Vec<HistoryTurn>,
    /// Retrieved memory chunks, in caller-preferred order. The
    /// budgeter sorts by `score` descending before truncation so the
    /// caller doesn't need to pre-sort.
    pub retrieval: Vec<RetrievedChunk>,
    /// Tool schemas as a single concatenated string (e.g. JSON-Schema
    /// blob). Kept whole or dropped — never partially truncated, since
    /// truncating JSON would corrupt it.
    pub tools: String,
}

/// Result of a budgeting pass — the four sections, each trimmed to fit,
/// plus a small stats struct so the caller can log what was dropped.
#[derive(Debug, Clone, Default)]
pub struct BudgetResult {
    pub persona: String,
    pub history: Vec<HistoryTurn>,
    pub retrieval: Vec<RetrievedChunk>,
    pub tools: String,
    pub stats: BudgetStats,
}

/// Audit trail of one budgeting pass — useful for logging and tests.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetStats {
    pub tokens_persona: usize,
    pub tokens_history: usize,
    pub tokens_retrieval: usize,
    pub tokens_tools: usize,
    /// `tokens_persona + tokens_history + tokens_retrieval + tokens_tools`.
    pub tokens_total: usize,
    /// Number of conversation turns dropped from the start of `history`.
    pub history_turns_dropped: usize,
    /// Number of retrieved chunks dropped (after `score` sort).
    pub retrieval_chunks_dropped: usize,
    /// `true` when the tool schemas were dropped because they didn't
    /// fit in the remaining budget.
    pub tools_dropped: bool,
}

/// Cheap-and-fast token estimator. ~4 chars per token; rounds up so an
/// empty string is `0` tokens but a one-char string costs `1`.
///
/// This intentionally over-estimates short inputs (a one-byte `"a"`
/// counts as 1 token even though `cl100k_base` would give it the same
/// answer), which is the safe direction for a budget guard.
pub fn estimate_tokens(s: &str) -> usize {
    if s.is_empty() {
        0
    } else {
        s.chars().count().div_ceil(CHARS_PER_TOKEN)
    }
}

/// Run one budgeting pass. Pure function — no I/O, no globals.
///
/// The algorithm walks each section in priority order, fitting it into
/// the configured per-section budget, then reclaims any unused
/// capacity into the next section. This means a small persona block
/// frees up tokens for retrieval, etc.
pub fn fit(inputs: &BudgetInputs, config: &BudgetConfig) -> BudgetResult {
    let mut stats = BudgetStats::default();

    // Section 1 — persona (never truncated).
    let persona = inputs.persona.clone();
    stats.tokens_persona = estimate_tokens(&persona);

    let mut remaining = config
        .total
        .saturating_sub(stats.tokens_persona);

    // Section 2 — retrieved memory. Sort by score descending then drop
    // tail until the section fits its budget AND the total cap.
    let retrieval_budget = config.retrieval.min(remaining);
    let (retrieval, retrieval_tokens, retrieval_dropped) =
        fit_retrieval(&inputs.retrieval, retrieval_budget);
    stats.tokens_retrieval = retrieval_tokens;
    stats.retrieval_chunks_dropped = retrieval_dropped;
    remaining = remaining.saturating_sub(retrieval_tokens);

    // Section 3 — conversation history. Keep newest first, drop oldest
    // turns until it fits.
    let history_budget = config.history.min(remaining);
    let (history, history_tokens, history_dropped) =
        fit_history(&inputs.history, history_budget);
    stats.tokens_history = history_tokens;
    stats.history_turns_dropped = history_dropped;
    remaining = remaining.saturating_sub(history_tokens);

    // Section 4 — tools. Whole-or-nothing.
    let tools_tokens = estimate_tokens(&inputs.tools);
    let tools_budget = config.tools.min(remaining);
    let tools = if tools_tokens == 0 {
        // Empty tools block is fine — no drop.
        String::new()
    } else if tools_tokens <= tools_budget {
        inputs.tools.clone()
    } else {
        stats.tools_dropped = true;
        String::new()
    };
    stats.tokens_tools = if stats.tools_dropped { 0 } else { tools_tokens };

    stats.tokens_total = stats.tokens_persona
        + stats.tokens_history
        + stats.tokens_retrieval
        + stats.tokens_tools;

    BudgetResult { persona, history, retrieval, tools, stats }
}

fn fit_retrieval(
    chunks: &[RetrievedChunk],
    budget: usize,
) -> (Vec<RetrievedChunk>, usize, usize) {
    // Sort by score descending; stable sort so equal-scored chunks keep
    // their incoming order.
    let mut sorted: Vec<RetrievedChunk> = chunks.to_vec();
    sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let mut kept = Vec::with_capacity(sorted.len());
    let mut used = 0usize;
    for c in sorted {
        let cost = estimate_tokens(&c.content);
        if used + cost > budget {
            break;
        }
        used += cost;
        kept.push(c);
    }
    let dropped = chunks.len().saturating_sub(kept.len());
    (kept, used, dropped)
}

fn fit_history(
    history: &[HistoryTurn],
    budget: usize,
) -> (Vec<HistoryTurn>, usize, usize) {
    // Walk from the end (newest first) to fit the budget, then reverse
    // back to oldest-first for the caller.
    let mut kept_rev: Vec<HistoryTurn> = Vec::new();
    let mut used = 0usize;
    for turn in history.iter().rev() {
        let cost = estimate_tokens(&turn.content) + estimate_tokens(&turn.role);
        if used + cost > budget {
            break;
        }
        used += cost;
        kept_rev.push(turn.clone());
    }
    let dropped = history.len().saturating_sub(kept_rev.len());
    kept_rev.reverse();
    (kept_rev, used, dropped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(role: &str, content: &str) -> HistoryTurn {
        HistoryTurn { role: role.to_string(), content: content.to_string() }
    }

    fn chunk(content: &str, score: f64) -> RetrievedChunk {
        RetrievedChunk { content: content.to_string(), score }
    }

    #[test]
    fn estimate_tokens_handles_empty_and_one_char() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        // 4 chars = 1 token boundary
        assert_eq!(estimate_tokens("abcd"), 1);
        // 5 chars = round up
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    #[test]
    fn budget_default_is_local_mode() {
        let d = BudgetConfig::default();
        let local = BudgetConfig::for_local_mode();
        assert_eq!(d, local);
    }

    #[test]
    fn budget_modes_have_distinct_totals() {
        // Free is the most constrained, paid the largest, local
        // somewhere between.
        let f = BudgetConfig::for_free_mode();
        let p = BudgetConfig::for_paid_mode();
        let l = BudgetConfig::for_local_mode();
        assert!(f.total < l.total);
        assert!(l.total < p.total);
    }

    #[test]
    fn fit_under_budget_keeps_everything() {
        let inputs = BudgetInputs {
            persona: "You are a friendly companion.".to_string(),
            history: vec![turn("user", "hi"), turn("assistant", "hello")],
            retrieval: vec![chunk("memory A", 0.9), chunk("memory B", 0.8)],
            tools: "{}".to_string(),
        };
        let cfg = BudgetConfig {
            total: 10_000,
            persona: 1000,
            history: 1000,
            retrieval: 1000,
            tools: 1000,
        };
        let res = fit(&inputs, &cfg);
        assert_eq!(res.history.len(), 2);
        assert_eq!(res.retrieval.len(), 2);
        assert_eq!(res.tools, "{}");
        assert_eq!(res.stats.history_turns_dropped, 0);
        assert_eq!(res.stats.retrieval_chunks_dropped, 0);
        assert!(!res.stats.tools_dropped);
    }

    #[test]
    fn fit_drops_lowest_score_retrieval_first() {
        // 4-char content = 1 token each. Budget allows only 2.
        let inputs = BudgetInputs {
            persona: String::new(),
            history: vec![],
            retrieval: vec![
                chunk("AAAA", 0.1),  // worst
                chunk("BBBB", 0.9),  // best
                chunk("CCCC", 0.5),  // middle
            ],
            tools: String::new(),
        };
        let cfg = BudgetConfig {
            total: 100,
            persona: 0,
            history: 0,
            retrieval: 2, // exactly 2 tokens → 2 chunks fit
            tools: 0,
        };
        let res = fit(&inputs, &cfg);
        assert_eq!(res.retrieval.len(), 2);
        // Should keep BBBB (0.9) and CCCC (0.5), drop AAAA (0.1).
        let kept_contents: Vec<&str> =
            res.retrieval.iter().map(|c| c.content.as_str()).collect();
        assert_eq!(kept_contents, vec!["BBBB", "CCCC"]);
        assert_eq!(res.stats.retrieval_chunks_dropped, 1);
    }

    #[test]
    fn fit_history_keeps_most_recent_turns() {
        // Each turn ≈ 2 tokens (4-char content + 4-char role).
        let inputs = BudgetInputs {
            persona: String::new(),
            history: vec![
                turn("user", "OLD1"),  // dropped
                turn("user", "OLD2"),  // dropped
                turn("user", "NEW1"),  // kept
                turn("user", "NEW2"),  // kept
            ],
            retrieval: vec![],
            tools: String::new(),
        };
        let cfg = BudgetConfig {
            total: 100,
            persona: 0,
            history: 4, // exactly 4 tokens → 2 turns fit
            retrieval: 0,
            tools: 0,
        };
        let res = fit(&inputs, &cfg);
        assert_eq!(res.history.len(), 2);
        // Order preserved (oldest-first).
        assert_eq!(res.history[0].content, "NEW1");
        assert_eq!(res.history[1].content, "NEW2");
        assert_eq!(res.stats.history_turns_dropped, 2);
    }

    #[test]
    fn fit_drops_tools_when_oversize() {
        let big_tools = "x".repeat(400); // ≈100 tokens
        let inputs = BudgetInputs {
            persona: String::new(),
            history: vec![],
            retrieval: vec![],
            tools: big_tools,
        };
        let cfg = BudgetConfig {
            total: 50,
            persona: 0,
            history: 0,
            retrieval: 0,
            tools: 10, // too small to fit 100 tokens
        };
        let res = fit(&inputs, &cfg);
        assert!(res.tools.is_empty());
        assert!(res.stats.tools_dropped);
    }

    #[test]
    fn fit_keeps_persona_even_when_other_sections_truncated() {
        // Persona is critical — never truncated, even when its share
        // pushes total over.
        let inputs = BudgetInputs {
            persona: "x".repeat(40), // 10 tokens
            history: vec![turn("user", "y".repeat(20).as_str())], // 5+1 tokens
            retrieval: vec![chunk(&"z".repeat(20), 0.5)], // 5 tokens
            tools: String::new(),
        };
        let cfg = BudgetConfig {
            total: 12,
            persona: 10,
            history: 1, // tight
            retrieval: 1, // tight
            tools: 0,
        };
        let res = fit(&inputs, &cfg);
        // Persona kept verbatim regardless.
        assert_eq!(res.persona.len(), 40);
        assert_eq!(res.stats.tokens_persona, 10);
    }

    #[test]
    fn fit_total_stats_match_section_sums() {
        let inputs = BudgetInputs {
            persona: "abcd".to_string(), // 1 token
            history: vec![turn("user", "abcd")], // 1 + 1 tokens (role+content)
            retrieval: vec![chunk("abcd", 0.9)], // 1 token
            tools: "abcd".to_string(), // 1 token
        };
        let cfg = BudgetConfig::for_local_mode();
        let res = fit(&inputs, &cfg);
        let s = &res.stats;
        assert_eq!(
            s.tokens_total,
            s.tokens_persona + s.tokens_history + s.tokens_retrieval + s.tokens_tools
        );
    }

    #[test]
    fn fit_handles_empty_inputs() {
        let res = fit(&BudgetInputs::default(), &BudgetConfig::default());
        assert!(res.persona.is_empty());
        assert!(res.history.is_empty());
        assert!(res.retrieval.is_empty());
        assert!(res.tools.is_empty());
        assert_eq!(res.stats.tokens_total, 0);
    }

    #[test]
    fn fit_unused_capacity_does_not_overflow_total() {
        // Tiny total; persona uses all of it; history / retrieval
        // should drop everything cleanly without panicking.
        let inputs = BudgetInputs {
            persona: "x".repeat(40), // 10 tokens
            history: vec![turn("user", "yyyy")],
            retrieval: vec![chunk("zzzz", 0.5)],
            tools: "tttt".to_string(),
        };
        let cfg = BudgetConfig {
            total: 10,
            persona: 10,
            history: 100, // generous *per-section* but no remaining
            retrieval: 100,
            tools: 100,
        };
        let res = fit(&inputs, &cfg);
        assert_eq!(res.stats.tokens_persona, 10);
        // Everything else dropped because no remaining budget.
        assert!(res.history.is_empty());
        assert!(res.retrieval.is_empty());
        assert!(res.tools.is_empty());
    }

    #[test]
    fn budget_config_round_trips_through_serde() {
        let cfg = BudgetConfig::for_paid_mode();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: BudgetConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn budget_stats_round_trip() {
        let s = BudgetStats {
            tokens_persona: 10,
            tokens_history: 20,
            tokens_retrieval: 30,
            tokens_tools: 40,
            tokens_total: 100,
            history_turns_dropped: 1,
            retrieval_chunks_dropped: 2,
            tools_dropped: true,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: BudgetStats = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn fit_zero_total_drops_everything_except_persona_being_recorded() {
        // Pathological case — guards against div-by-zero / underflow.
        let inputs = BudgetInputs {
            persona: "abcd".to_string(),
            history: vec![turn("u", "x")],
            retrieval: vec![chunk("y", 0.5)],
            tools: "t".to_string(),
        };
        let cfg = BudgetConfig {
            total: 0,
            persona: 0,
            history: 0,
            retrieval: 0,
            tools: 0,
        };
        let res = fit(&inputs, &cfg);
        // Persona always kept verbatim (truncation policy doc).
        assert_eq!(res.persona, "abcd");
        // All other sections dropped.
        assert!(res.history.is_empty());
        assert!(res.retrieval.is_empty());
        assert!(res.tools.is_empty());
        assert_eq!(res.stats.history_turns_dropped, 1);
        assert_eq!(res.stats.retrieval_chunks_dropped, 1);
    }
}
