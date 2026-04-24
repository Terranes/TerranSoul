//! Cross-encoder-style **reranking** for top-k RAG candidates.
//!
//! ## Why rerank
//!
//! `MemoryStore::hybrid_search_rrf` returns the top-k candidates fused
//! from independent **bi-encoder** retrievers (vector cosine, keyword,
//! freshness). Bi-encoders embed the query and the document
//! independently, then compare with a single dot product — fast at
//! retrieval time but lossy: subtle phrase-level interactions between
//! query and document are flattened into one number.
//!
//! A **cross-encoder** instead feeds `(query, document)` *together*
//! into a model and outputs a single relevance score. The query and
//! document tokens attend to each other directly, so phrase-level
//! interactions are preserved. This is much higher precision but too
//! expensive to run over the whole corpus, so the standard pipeline is
//!
//! ```text
//!   bi-encoder hybrid retrieval (top-k = 30..50)
//!         │
//!         ▼
//!   cross-encoder rerank (top-N = 5..10) ──► prompt context
//! ```
//!
//! ## Why "LLM-as-judge" rather than a dedicated reranker model
//!
//! The seminal cross-encoder choices (BGE-reranker-v2-m3, Cohere
//! Rerank 3, mxbai-rerank) are *separate* models you'd have to
//! download alongside the chat model. For a single-user desktop app
//! that already has a capable chat brain configured (Free / Paid /
//! Local Ollama), it is more honest to **reuse the active brain as the
//! reranker** by asking it to score each `(query, document)` pair on
//! a 0–10 relevance scale ("LLM-as-judge", widely used in 2024 RAG
//! eval pipelines and as a pragmatic reranker fallback).
//!
//! Trade-offs vs. a dedicated reranker model:
//!
//! * ✅ No extra model download, no extra RAM, works in all three
//!   brain modes including the cold-start Free API.
//! * ✅ Quality is competitive when the chat model is decent
//!   (Llama-3-8B+, Qwen-2.5+, or any cloud model).
//! * ⚠ Slower than a tiny BGE reranker — N+1 round-trips for N
//!   candidates. We mitigate by capping `rerank_k` (default 20) and by
//!   running scores **sequentially within a single command** so we
//!   stay under the LLM provider's typical rate limits.
//!
//! When TerranSoul later ships a dedicated reranker (BGE / mxbai), the
//! interface in this module — `(query, document) -> Option<f32>` — is
//! exactly the same, so swapping the backend is a one-line change in
//! the Tauri command.
//!
//! See `docs/brain-advanced-design.md` § 16 Phase 6 / § 19.2 row 10.

use crate::memory::MemoryEntry;

/// Build the LLM-as-judge prompt for a single `(query, document)` pair.
///
/// Returns `(system, user)` so the caller can pass them straight to
/// `OllamaAgent::call`. The prompt deliberately:
///
/// * Asks for an integer 0–10 score (not a free-text justification),
///   making the reply parseable with a single regex pass.
/// * Includes a short rubric so models without rich instruction
///   following still produce calibrated scores.
/// * Caps the document at the first 1500 chars so an unusually long
///   memory doesn't blow the context window of small local models.
pub fn build_rerank_prompt(query: &str, document: &str) -> (String, String) {
    let system = "You are a relevance scorer for a retrieval system. Read the user's \
        QUERY and the candidate DOCUMENT, then output a single integer from 0 to 10 \
        indicating how directly the document answers the query. \
        Use this rubric: 0 = unrelated, 3 = mentions the topic but doesn't answer, \
        6 = partially answers, 8 = answers most of the query, 10 = perfect direct \
        answer. Reply with ONLY the integer and nothing else."
        .to_string();

    // Truncate the document to keep small local models within their
    // context budget. 1500 chars is roughly 400 tokens — comfortable.
    const MAX_DOC_CHARS: usize = 1500;
    let doc_clipped: String = document.chars().take(MAX_DOC_CHARS).collect();

    let user = format!(
        "QUERY: {q}\n\nDOCUMENT: {d}\n\nScore (0-10):",
        q = query.trim(),
        d = doc_clipped.trim(),
    );

    (system, user)
}

/// Parse a 0–10 integer score from an LLM reply.
///
/// Robust to common chat noise:
/// * Strips whitespace, trailing punctuation, and surrounding text.
/// * Accepts replies of the form `"7"`, `"7."`, `"Score: 7"`,
///   `"7 out of 10"`, `"**7**"`.
/// * Returns the **first** integer in 0..=10. Anything outside the
///   valid range — or a reply with no integers at all — yields
///   `None`, which the caller should treat as "skip this candidate"
///   (the candidate keeps its pre-rerank position rather than being
///   dropped, see [`rerank_candidates`]).
pub fn parse_rerank_score(reply: &str) -> Option<u8> {
    let mut digits = String::new();
    for ch in reply.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            // Stop at the first non-digit *after* we've started
            // collecting digits, so "10/10" parses as 10 rather than
            // running into "1010".
            //
            // (Continued in the matching `else` branch below.)
        } else if !digits.is_empty() {
            break;
        }
    }

    if digits.is_empty() {
        return None;
    }

    // Cap at three digits to defend against "1000" → 100; we'll still
    // reject anything > 10 below.
    let clipped = &digits[..digits.len().min(3)];
    let parsed: u32 = clipped.parse().ok()?;
    if parsed <= 10 {
        Some(parsed as u8)
    } else {
        None
    }
}

/// Re-order `candidates` according to the per-candidate scores in
/// `scores`, breaking ties by the candidate's *original* position
/// (which already encodes the bi-encoder hybrid ranking).
///
/// `scores[i]` is `Some(s)` if the reranker successfully scored
/// `candidates[i]`, or `None` if the LLM call failed / produced an
/// unparseable reply. Unscored candidates are kept in the result but
/// fall *below* every successfully-scored candidate, preserving
/// recall: the user never silently loses a hit because the reranker
/// was flaky.
///
/// Returns the top `limit` reranked candidates. Pure function — no
/// I/O, no LLM calls — so it is fully unit-tested below.
pub fn rerank_candidates(
    candidates: Vec<MemoryEntry>,
    scores: &[Option<u8>],
    limit: usize,
) -> Vec<MemoryEntry> {
    if limit == 0 || candidates.is_empty() {
        return Vec::new();
    }
    debug_assert_eq!(candidates.len(), scores.len(),
        "rerank_candidates: scores must align 1:1 with candidates");

    // Pair each candidate with (score-or-None, original_index).
    let mut indexed: Vec<(usize, Option<u8>, MemoryEntry)> = candidates
        .into_iter()
        .enumerate()
        .map(|(idx, entry)| {
            let score = scores.get(idx).copied().flatten();
            (idx, score, entry)
        })
        .collect();

    // Sort: scored candidates first (descending by score),
    // then unscored, ties broken by original index ascending.
    indexed.sort_by(|a, b| {
        match (a.1, b.1) {
            (Some(sa), Some(sb)) => sb.cmp(&sa).then_with(|| a.0.cmp(&b.0)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.0.cmp(&b.0),
        }
    });

    indexed
        .into_iter()
        .take(limit)
        .map(|(_, _, entry)| entry)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryTier, MemoryType};

    fn entry(id: i64, content: &str) -> MemoryEntry {
        MemoryEntry {
            id,
            content: content.to_string(),
            tags: String::new(),
            importance: 3,
            memory_type: MemoryType::Fact,
            created_at: 0,
            last_accessed: None,
            access_count: 0,
            embedding: None,
            tier: MemoryTier::Long,
            decay_score: 1.0,
            session_id: None,
            parent_id: None,
            token_count: 0,
            source_url: None,
            source_hash: None,
            expires_at: None,
            valid_to: None,
        }
    }

    // ── Prompt construction ──────────────────────────────────────────

    #[test]
    fn build_prompt_includes_query_doc_and_rubric() {
        let (system, user) = build_rerank_prompt("What is RAG?", "RAG combines retrieval and generation.");
        assert!(system.contains("0 to 10"));
        assert!(system.contains("rubric"));
        assert!(system.contains("ONLY the integer"));
        assert!(user.contains("QUERY: What is RAG?"));
        assert!(user.contains("DOCUMENT: RAG combines"));
        assert!(user.trim_end().ends_with("Score (0-10):"));
    }

    #[test]
    fn build_prompt_truncates_long_documents() {
        let long = "x".repeat(5000);
        let (_system, user) = build_rerank_prompt("q", &long);
        // The document portion must not contain the full 5000 x's.
        let xs = user.matches('x').count();
        assert!(xs <= 1500, "expected document truncated to <=1500 chars, got {xs}");
    }

    #[test]
    fn build_prompt_trims_query_and_doc_whitespace() {
        let (_system, user) = build_rerank_prompt("  q  ", "  d  ");
        assert!(user.contains("QUERY: q"));
        assert!(user.contains("DOCUMENT: d"));
    }

    // ── Score parsing ────────────────────────────────────────────────

    #[test]
    fn parse_score_bare_integer() {
        assert_eq!(parse_rerank_score("7"), Some(7));
        assert_eq!(parse_rerank_score("0"), Some(0));
        assert_eq!(parse_rerank_score("10"), Some(10));
    }

    #[test]
    fn parse_score_with_punctuation_and_whitespace() {
        assert_eq!(parse_rerank_score("  7. "), Some(7));
        assert_eq!(parse_rerank_score("**8**"), Some(8));
        assert_eq!(parse_rerank_score("Score: 6"), Some(6));
        assert_eq!(parse_rerank_score("7 out of 10"), Some(7));
    }

    #[test]
    fn parse_score_rejects_out_of_range() {
        assert_eq!(parse_rerank_score("11"), None);
        assert_eq!(parse_rerank_score("100"), None);
        assert_eq!(parse_rerank_score("1000"), None);
    }

    #[test]
    fn parse_score_returns_none_on_no_digits() {
        assert_eq!(parse_rerank_score(""), None);
        assert_eq!(parse_rerank_score("I don't know"), None);
        assert_eq!(parse_rerank_score("   "), None);
    }

    // ── Reorder logic ────────────────────────────────────────────────

    #[test]
    fn rerank_zero_limit_returns_empty() {
        let cands = vec![entry(1, "a")];
        let scores = vec![Some(5)];
        assert!(rerank_candidates(cands, &scores, 0).is_empty());
    }

    #[test]
    fn rerank_empty_candidates_returns_empty() {
        let scores: Vec<Option<u8>> = vec![];
        assert!(rerank_candidates(vec![], &scores, 5).is_empty());
    }

    #[test]
    fn rerank_sorts_by_score_descending() {
        let cands = vec![entry(1, "low"), entry(2, "high"), entry(3, "mid")];
        let scores = vec![Some(2), Some(9), Some(5)];
        let out = rerank_candidates(cands, &scores, 3);
        assert_eq!(out.iter().map(|e| e.id).collect::<Vec<_>>(), vec![2, 3, 1]);
    }

    #[test]
    fn rerank_breaks_score_ties_by_original_index() {
        // Two candidates with the same score → the earlier original
        // position (which encodes the bi-encoder ranking) wins.
        let cands = vec![entry(1, "first"), entry(2, "second"), entry(3, "third")];
        let scores = vec![Some(7), Some(7), Some(3)];
        let out = rerank_candidates(cands, &scores, 3);
        assert_eq!(out.iter().map(|e| e.id).collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn rerank_unscored_candidates_kept_below_scored_ones() {
        // The reranker failed to score id=2; it must NOT be silently
        // dropped — it slots in below every successfully-scored
        // candidate but still appears in the output.
        let cands = vec![entry(1, "scored low"), entry(2, "unscored"), entry(3, "scored high")];
        let scores = vec![Some(2), None, Some(9)];
        let out = rerank_candidates(cands, &scores, 3);
        assert_eq!(out.iter().map(|e| e.id).collect::<Vec<_>>(), vec![3, 1, 2]);
    }

    #[test]
    fn rerank_all_unscored_preserves_original_order() {
        let cands = vec![entry(7, "a"), entry(8, "b"), entry(9, "c")];
        let scores = vec![None, None, None];
        let out = rerank_candidates(cands, &scores, 3);
        assert_eq!(out.iter().map(|e| e.id).collect::<Vec<_>>(), vec![7, 8, 9]);
    }

    #[test]
    fn rerank_truncates_to_limit() {
        let cands = (0..10).map(|i| entry(i, "x")).collect::<Vec<_>>();
        let scores: Vec<Option<u8>> = (0..10).map(|i| Some(i as u8)).collect();
        let out = rerank_candidates(cands, &scores, 3);
        // Top 3 by score (descending) are 9, 8, 7.
        assert_eq!(out.iter().map(|e| e.id).collect::<Vec<_>>(), vec![9, 8, 7]);
    }
}
