//! HyDE — **Hypothetical Document Embeddings** for cold-query retrieval.
//!
//! Standard vector search embeds the *query* and looks for similar
//! documents. This works well when the query phrasing is close to the
//! way the documents are written, but breaks down on *cold* queries
//! that are abstract, short, or phrased very differently from the
//! corpus (e.g. a one-word lookup against multi-paragraph notes).
//!
//! HyDE (Gao et al., 2022 — *"Precise Zero-Shot Dense Retrieval without
//! Relevance Labels"*) sidesteps this by asking the LLM to **generate
//! a hypothetical answer** to the query and then embedding *that* for
//! the search. The hypothetical answer lives in the same distribution
//! as real documents, so cosine similarity becomes much sharper —
//! the seminal paper reports large recall gains across BEIR, TREC and
//! Mr. TyDi without any fine-tuning.
//!
//! This module is intentionally minimal: it owns only the *prompt
//! template* and the *response cleaner*, so the LLM-call orchestration
//! stays in the Tauri command layer (`crate::commands::memory`) where
//! it can reuse the existing `OllamaAgent` and embedding cache without
//! a circular dependency.
//!
//! See `docs/brain-advanced-design.md` § 19.2 row 4 / § 16 Phase 6.

/// Build the HyDE prompt for the given user query.
///
/// The prompt asks the LLM to write a plausible 1–3 sentence answer
/// **as if it already knew the corpus**. The output is intentionally
/// short so embedding it stays cheap, and is constrained to plain
/// prose so we don't pollute the embedding space with bullet
/// formatting or surrounding chatter.
///
/// The system prompt is returned separately from the user prompt so
/// the caller can pass them straight to [`OllamaAgent::call`] which
/// expects a `Vec<ChatMessage>` of `{role, content}` pairs.
pub fn build_hyde_prompt(query: &str) -> (String, String) {
    let system = "You are a retrieval helper. Given a user question, write a plausible \
        1-3 sentence answer in plain prose (no bullets, no preamble, no markdown). \
        Optimise for matching the writing style of long-form documentation, not chat. \
        If you genuinely have no idea, restate the question as a declarative \
        sentence — never reply with 'I don't know'."
        .to_string();

    let user = format!(
        "Question: {q}\n\nWrite a hypothetical answer.",
        q = query.trim()
    );

    (system, user)
}

/// Clean an LLM reply into a single line of prose suitable for
/// embedding.
///
/// * Strips leading/trailing whitespace.
/// * Removes common chat preambles ("Sure, ...", "Answer: ...",
///   "Here is ...") that the model sometimes emits despite the prompt.
/// * Collapses internal newlines to single spaces — embedding models
///   are insensitive to whitespace and we want a stable byte-string
///   for cache hits.
/// * Returns `None` if the cleaned text is empty or so short that it
///   won't carry any retrieval signal (< 4 chars), in which case the
///   caller should fall back to embedding the raw query.
pub fn clean_hyde_reply(reply: &str) -> Option<String> {
    let trimmed = reply.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Strip a single leading preamble line if present.
    let stripped = strip_preamble(trimmed);

    // Collapse newlines + repeated whitespace.
    let collapsed: String = stripped.split_whitespace().collect::<Vec<_>>().join(" ");

    if collapsed.chars().count() < 4 {
        None
    } else {
        Some(collapsed)
    }
}

/// Strip a common chat preamble (`Sure, ...`, `Answer: ...`, etc.)
/// from the front of an LLM reply. Idempotent and safe on inputs that
/// don't start with a preamble.
fn strip_preamble(text: &str) -> &str {
    const PREAMBLES: &[&str] = &[
        "Sure,",
        "Sure!",
        "Sure.",
        "Of course,",
        "Of course!",
        "Certainly,",
        "Certainly!",
        "Answer:",
        "ANSWER:",
        "Hypothetical answer:",
        "Hypothetical:",
        "Here is a hypothetical answer:",
        "Here's a hypothetical answer:",
        "Here is an answer:",
        "Here's an answer:",
    ];
    for p in PREAMBLES {
        if let Some(rest) = text.strip_prefix(p) {
            return rest.trim_start_matches(|c: char| c.is_whitespace() || c == ':' || c == '-');
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_includes_query_and_constraints() {
        let (system, user) = build_hyde_prompt("What is RAG?");
        // System prompt encodes the formatting constraints.
        assert!(system.contains("plain prose"));
        assert!(system.contains("1-3 sentence"));
        assert!(system.contains("no bullets"));
        // User prompt contains the trimmed query.
        assert!(user.contains("What is RAG?"));
        assert!(user.starts_with("Question:"));
    }

    #[test]
    fn build_prompt_trims_query_whitespace() {
        let (_system, user) = build_hyde_prompt("   spacy query   ");
        assert!(user.contains("Question: spacy query"));
        assert!(!user.contains("Question:    spacy query"));
    }

    #[test]
    fn clean_reply_strips_preamble() {
        let cleaned = clean_hyde_reply(
            "Sure! RAG stands for Retrieval-Augmented Generation, a technique that \
             combines a retriever with a generator.",
        )
        .expect("should produce a hypothetical");
        assert!(cleaned.starts_with("RAG stands for"));
        assert!(!cleaned.to_lowercase().starts_with("sure"));
    }

    #[test]
    fn clean_reply_strips_answer_label() {
        let cleaned =
            clean_hyde_reply("Answer: The court filing deadline is 30 days from service.")
                .expect("should produce a hypothetical");
        assert!(cleaned.starts_with("The court filing deadline"));
    }

    #[test]
    fn clean_reply_collapses_whitespace_and_newlines() {
        let cleaned =
            clean_hyde_reply("foo\n\nbar   baz\n\tqux").expect("should produce a hypothetical");
        assert_eq!(cleaned, "foo bar baz qux");
    }

    #[test]
    fn clean_reply_returns_none_on_empty() {
        assert!(clean_hyde_reply("").is_none());
        assert!(clean_hyde_reply("   \n\t  ").is_none());
    }

    #[test]
    fn clean_reply_returns_none_on_too_short() {
        // Less than 4 chars — no useful retrieval signal.
        assert!(clean_hyde_reply("ok").is_none());
        assert!(clean_hyde_reply("Sure! ok").is_none());
    }

    #[test]
    fn clean_reply_passes_through_clean_text() {
        let input = "The Vietnamese law portal thuvienphapluat.vn publishes statutes \
                     and decrees from the Ministry of Justice.";
        let cleaned = clean_hyde_reply(input).expect("should pass through");
        assert_eq!(cleaned, input);
    }

    #[test]
    fn clean_reply_idempotent_on_already_clean_text() {
        let input = "RAG combines retrieval with generation.";
        let once = clean_hyde_reply(input).unwrap();
        let twice = clean_hyde_reply(&once).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn strip_preamble_is_safe_without_preamble() {
        assert_eq!(strip_preamble("plain text"), "plain text");
    }
}
