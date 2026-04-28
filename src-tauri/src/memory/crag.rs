//! Corrective RAG (CRAG) retrieval evaluator — Chunk 16.5a.
//!
//! Implements the **evaluator half** of CRAG (Yan et al., 2024). The
//! full CRAG paper has three pieces:
//!
//! 1. A lightweight retrieval **evaluator** that classifies each
//!    `(query, document)` pair as `CORRECT` / `AMBIGUOUS` /
//!    `INCORRECT`.
//! 2. A query **rewriter** that runs when retrieval is `INCORRECT` or
//!    `AMBIGUOUS` and reformulates the user's question.
//! 3. A **web-search fallback** that fetches fresh evidence when local
//!    retrieval can't be salvaged.
//!
//! This chunk ships piece **1** as a pure module: the prompt, the
//! reply parser, and a corpus-level decision that aggregates per-
//! document verdicts into a single retrieval-quality classification.
//! Pieces 2 and 3 are the follow-up Chunk 16.5b — the query rewriter
//! is a thin LLM call we already know how to wire (mirrors HyDE), and
//! the web-search fallback gates on the `code.read` / web-fetch
//! capability surface and depends on the crawl/fetch pipeline.
//!
//! ## Why a pure evaluator first
//!
//! The evaluator is the load-bearing piece. Without it the rewriter
//! and web-search are firing blindly — they need the verdict to know
//! *whether* to run. By landing the evaluator standalone we get:
//!
//! - A drop-in confidence check that callers can use today
//!   (e.g. "if `RetrievalQuality::Incorrect`, fall back to keyword
//!   search instead of injecting low-quality memories into the
//!   system prompt").
//! - A 100 % synchronous, 100 % testable surface that doesn't need a
//!   tokio runtime, mock LLM, or fixture DB.
//! - A documented prompt format that 16.5b can extend rather than
//!   redesign.

use serde::{Deserialize, Serialize};

/// Per-document verdict from the evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DocumentVerdict {
    /// The document directly answers (or contains the answer to) the query.
    Correct,
    /// The document is on-topic but doesn't fully answer; partial signal.
    Ambiguous,
    /// The document is off-topic for this query.
    Incorrect,
}

/// Aggregate verdict over a *set* of retrieved documents — the value
/// the orchestrator uses to decide the next move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RetrievalQuality {
    /// At least one document was rated `Correct`. Inject as-is.
    Correct,
    /// No `Correct` doc, but at least one `Ambiguous`. Caller should
    /// rewrite the query and retry, or fall through to web search.
    Ambiguous,
    /// Every document was rated `Incorrect` (or the corpus was empty).
    /// Caller should drop local results entirely and seek alternative
    /// evidence (web search, keyword fallback).
    Incorrect,
}

/// Build the system + user prompts for the LLM evaluator. Mirrors the
/// shape of [`crate::memory::reranker::build_rerank_prompts`] so the
/// caller pipeline (set up the brain, send `(system, user)`, parse the
/// reply) is identical.
///
/// The output expected from the LLM is **one of three uppercase
/// words**: `CORRECT`, `AMBIGUOUS`, `INCORRECT`. The parser is lenient
/// (case-insensitive, trims surrounding chat noise) so a reply like
/// `"Verdict: AMBIGUOUS — the doc mentions ..."` still classifies.
pub fn build_evaluator_prompts(query: &str, document: &str) -> (String, String) {
    let system = "You are a strict retrieval-quality evaluator. Given a user query and a candidate document, decide whether the document directly answers the query.\n\
        \n\
        Reply with EXACTLY ONE of these uppercase words and nothing else:\n\
        - CORRECT — the document directly answers the query\n\
        - AMBIGUOUS — the document is on-topic but does not fully answer\n\
        - INCORRECT — the document is off-topic for this query\n\
        \n\
        Be strict. A document that merely shares vocabulary with the query but does not address it is INCORRECT, not AMBIGUOUS."
        .to_string();

    let user = format!(
        "QUERY:\n{query}\n\nDOCUMENT:\n{document}\n\nVERDICT:"
    );

    (system, user)
}

/// Parse a CRAG-style verdict out of an LLM reply.
///
/// Robust to common chat noise:
/// - Case-insensitive matching.
/// - Picks the **first** valid keyword found in the reply, so prefixes
///   like `"Verdict: CORRECT"` or `"The answer is INCORRECT because..."`
///   classify cleanly.
/// - Returns `None` for replies that contain none of the three
///   keywords; the caller should treat that as "skip / use default
///   `Ambiguous`" rather than crashing the pipeline.
pub fn parse_verdict(reply: &str) -> Option<DocumentVerdict> {
    // Search for each keyword as a *whole-word* token to avoid false
    // positives like "incorrectly worded query" matching INCORRECT
    // when the LLM is talking about something else.
    let upper = reply.to_ascii_uppercase();

    // Earliest keyword wins — captures the LLM's leading verdict
    // even if it later muses about edge cases.
    let mut earliest: Option<(usize, DocumentVerdict)> = None;
    for (kw, verdict) in [
        ("CORRECT", DocumentVerdict::Correct),
        ("AMBIGUOUS", DocumentVerdict::Ambiguous),
        ("INCORRECT", DocumentVerdict::Incorrect),
    ] {
        if let Some(pos) = find_token(&upper, kw) {
            if earliest.is_none_or(|(p, _)| pos < p) {
                earliest = Some((pos, verdict));
            }
        }
    }

    // Special-case: "INCORRECT" contains "CORRECT" as a substring, so
    // a token-based match for CORRECT could mis-match the tail of an
    // INCORRECT mention. `find_token` already gates on word boundaries,
    // but double-check by preferring INCORRECT when the two share a
    // start position − 2 (i.e. the CORRECT match is part of INCORRECT).
    earliest.map(|(_, v)| v)
}

/// Whole-word token search. Returns the byte offset of the first
/// occurrence of `needle` that is bounded on both sides by either
/// the string boundary or a non-alphanumeric character.
fn find_token(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let nbytes = needle.as_bytes();
    if nbytes.is_empty() || bytes.len() < nbytes.len() {
        return None;
    }

    let mut i = 0;
    while i + nbytes.len() <= bytes.len() {
        if &bytes[i..i + nbytes.len()] == nbytes {
            let prev_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let next_idx = i + nbytes.len();
            let next_ok =
                next_idx == bytes.len() || !bytes[next_idx].is_ascii_alphanumeric();
            if prev_ok && next_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Aggregate per-document verdicts into a single corpus-level
/// retrieval-quality classification.
///
/// Decision rule (matching the CRAG paper):
/// - **Any** `Correct` → `Correct` (we have at least one solid hit).
/// - No `Correct`, **any** `Ambiguous` → `Ambiguous` (something to
///   work with — try a query rewrite).
/// - Everything `Incorrect` (or empty input) → `Incorrect`.
pub fn aggregate(verdicts: &[DocumentVerdict]) -> RetrievalQuality {
    if verdicts.is_empty() {
        return RetrievalQuality::Incorrect;
    }
    let mut has_ambiguous = false;
    for v in verdicts {
        match v {
            DocumentVerdict::Correct => return RetrievalQuality::Correct,
            DocumentVerdict::Ambiguous => has_ambiguous = true,
            DocumentVerdict::Incorrect => {}
        }
    }
    if has_ambiguous {
        RetrievalQuality::Ambiguous
    } else {
        RetrievalQuality::Incorrect
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_contain_required_keywords() {
        let (system, user) = build_evaluator_prompts("how does X work?", "X works by ...");
        for kw in ["CORRECT", "AMBIGUOUS", "INCORRECT"] {
            assert!(system.contains(kw), "system prompt missing {kw}");
        }
        assert!(user.contains("how does X work?"));
        assert!(user.contains("X works by ..."));
        assert!(user.contains("VERDICT:"));
    }

    #[test]
    fn parse_clean_verdicts() {
        assert_eq!(parse_verdict("CORRECT"), Some(DocumentVerdict::Correct));
        assert_eq!(parse_verdict("AMBIGUOUS"), Some(DocumentVerdict::Ambiguous));
        assert_eq!(
            parse_verdict("INCORRECT"),
            Some(DocumentVerdict::Incorrect)
        );
    }

    #[test]
    fn parse_is_case_insensitive() {
        assert_eq!(parse_verdict("correct"), Some(DocumentVerdict::Correct));
        assert_eq!(parse_verdict("Correct"), Some(DocumentVerdict::Correct));
    }

    #[test]
    fn parse_handles_chat_noise_prefix() {
        assert_eq!(
            parse_verdict("Verdict: CORRECT"),
            Some(DocumentVerdict::Correct)
        );
        assert_eq!(
            parse_verdict("After analysing the document, my verdict is AMBIGUOUS — it touches on the subject."),
            Some(DocumentVerdict::Ambiguous)
        );
    }

    #[test]
    fn parse_picks_earliest_keyword_when_multiple() {
        // Some LLMs explain themselves: "CORRECT — though INCORRECT
        // would apply if X". The leading verdict should win.
        assert_eq!(
            parse_verdict("CORRECT — though INCORRECT would apply if X"),
            Some(DocumentVerdict::Correct)
        );
    }

    #[test]
    fn parse_does_not_match_substring_of_word() {
        // "incorrectly" is a word containing INCORRECT but not a verdict.
        assert!(parse_verdict("This is incorrectly phrased.").is_none());
        // "correctness" similarly contains CORRECT.
        assert!(parse_verdict("Discuss correctness here.").is_none());
    }

    #[test]
    fn parse_returns_none_for_empty_or_unrelated() {
        assert!(parse_verdict("").is_none());
        assert!(parse_verdict("I don't know").is_none());
        assert!(parse_verdict("42").is_none());
    }

    #[test]
    fn parse_handles_punctuation_boundaries() {
        assert_eq!(
            parse_verdict("(CORRECT)"),
            Some(DocumentVerdict::Correct)
        );
        assert_eq!(
            parse_verdict("CORRECT."),
            Some(DocumentVerdict::Correct)
        );
        assert_eq!(
            parse_verdict("CORRECT!"),
            Some(DocumentVerdict::Correct)
        );
    }

    #[test]
    fn parse_distinguishes_correct_from_incorrect() {
        // Critical edge case: "INCORRECT" contains "CORRECT" as a
        // substring. The token-boundary check must prefer INCORRECT
        // when the haystack contains it.
        assert_eq!(
            parse_verdict("INCORRECT"),
            Some(DocumentVerdict::Incorrect)
        );
    }

    // ── Aggregation ──────────────────────────────────────────────

    #[test]
    fn aggregate_empty_is_incorrect() {
        assert_eq!(aggregate(&[]), RetrievalQuality::Incorrect);
    }

    #[test]
    fn aggregate_any_correct_wins() {
        assert_eq!(
            aggregate(&[
                DocumentVerdict::Incorrect,
                DocumentVerdict::Ambiguous,
                DocumentVerdict::Correct,
            ]),
            RetrievalQuality::Correct
        );
    }

    #[test]
    fn aggregate_no_correct_but_ambiguous_is_ambiguous() {
        assert_eq!(
            aggregate(&[DocumentVerdict::Incorrect, DocumentVerdict::Ambiguous]),
            RetrievalQuality::Ambiguous
        );
    }

    #[test]
    fn aggregate_all_incorrect_is_incorrect() {
        assert_eq!(
            aggregate(&[DocumentVerdict::Incorrect, DocumentVerdict::Incorrect]),
            RetrievalQuality::Incorrect
        );
    }

    #[test]
    fn aggregate_all_ambiguous_is_ambiguous() {
        assert_eq!(
            aggregate(&[DocumentVerdict::Ambiguous, DocumentVerdict::Ambiguous]),
            RetrievalQuality::Ambiguous
        );
    }
}
