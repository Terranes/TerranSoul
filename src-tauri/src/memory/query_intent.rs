//! Query-intent classifier for retrieval ranking (Chunk 16.6b).
//!
//! Classifies a user query into a [`QueryIntent`] (procedural / episodic /
//! semantic / factual / unknown) and produces per-`CognitiveKind` weight
//! boosts that callers can apply during RRF fusion.
//!
//! Per `docs/brain-advanced-design.md` §3.5.6 "Retrieval ranking by query
//! intent" — pure logic, no LLM call. Heuristic-based on:
//!
//! 1. **Question structure** — interrogative form ("what is", "who is")
//!    suggests factual; "how to/how do I" suggests procedural; "when did"
//!    / "where was" suggests episodic.
//! 2. **Verb cues** — imperative / how-to verbs ("install", "run", "build")
//!    boost procedural.
//! 3. **Time anchors** — "yesterday", "last week", date strings boost
//!    episodic.
//! 4. **Default** — semantic (general knowledge).

use serde::{Deserialize, Serialize};

use super::cognitive_kind::CognitiveKind;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// User query intent — guides retrieval-rank boosting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QueryIntent {
    /// "How do I X?" / step-by-step / installation / configuration.
    Procedural,
    /// "When did we X?" / "What did I say about Y?" — time / event anchored.
    Episodic,
    /// Looking up a definition, identity, factoid (single-shot answer).
    Factual,
    /// Conceptual / explanatory ("explain X", "why does Y work").
    #[default]
    Semantic,
    /// Could not classify with confidence — caller falls back to default RRF.
    Unknown,
}

impl QueryIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            QueryIntent::Procedural => "procedural",
            QueryIntent::Episodic => "episodic",
            QueryIntent::Factual => "factual",
            QueryIntent::Semantic => "semantic",
            QueryIntent::Unknown => "unknown",
        }
    }
}

/// Result of classifying a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentClassification {
    pub intent: QueryIntent,
    /// Confidence 0.0–1.0 from the heuristic.
    pub confidence: f32,
    /// Per-`CognitiveKind` boost multiplier the caller should apply during
    /// RRF fusion. `1.0` = no change; `>1.0` = boost; `<1.0` = penalise.
    pub kind_boosts: KindBoosts,
}

/// Boost multipliers per cognitive kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KindBoosts {
    pub procedural: f32,
    pub episodic: f32,
    pub semantic: f32,
}

impl Default for KindBoosts {
    fn default() -> Self {
        Self {
            procedural: 1.0,
            episodic: 1.0,
            semantic: 1.0,
        }
    }
}

impl KindBoosts {
    /// Boost lookup helper.
    pub fn for_kind(&self, kind: CognitiveKind) -> f32 {
        match kind {
            CognitiveKind::Procedural => self.procedural,
            CognitiveKind::Episodic => self.episodic,
            CognitiveKind::Semantic => self.semantic,
            CognitiveKind::Judgment => self.semantic, // judgments treated as semantic for boosting
        }
    }
}

// ---------------------------------------------------------------------------
// Heuristic vocabulary
// ---------------------------------------------------------------------------

/// Lower-case prefixes that strongly indicate procedural intent.
const PROCEDURAL_PREFIXES: &[&str] = &[
    "how to ",
    "how do i ",
    "how do you ",
    "how can i ",
    "steps to ",
    "show me how ",
    "walk me through ",
    "guide me through ",
    "teach me how ",
];

/// Verbs/keywords that boost procedural confidence anywhere in the query.
const PROCEDURAL_VERBS: &[&str] = &[
    " install ",
    " configure ",
    " build ",
    " deploy ",
    " run ",
    " setup ",
    " set up ",
    " step-by-step",
    " step by step",
    " workflow",
    " procedure",
    " recipe",
];

/// Lower-case prefixes that indicate episodic (time-anchored) intent.
const EPISODIC_PREFIXES: &[&str] = &[
    "when did ",
    "when was ",
    "where did ",
    "where was ",
    "what did i say ",
    "what did we discuss ",
    "what did you say ",
    "remind me when ",
    "remind me what ",
];

/// Time-anchor keywords boosting episodic confidence.
const EPISODIC_TIME_ANCHORS: &[&str] = &[
    "yesterday",
    "today",
    "this morning",
    "this afternoon",
    "tonight",
    "last night",
    "last week",
    "last month",
    "earlier",
    " ago",
    " on monday",
    " on tuesday",
    " on wednesday",
    " on thursday",
    " on friday",
    " on saturday",
    " on sunday",
];

/// Lower-case prefixes indicating factual lookup.
const FACTUAL_PREFIXES: &[&str] = &[
    "what is ",
    "what's ",
    "who is ",
    "who's ",
    "define ",
    "definition of ",
    "meaning of ",
    "what does ",
];

/// Lower-case prefixes indicating semantic / conceptual intent.
const SEMANTIC_PREFIXES: &[&str] = &[
    "why ",
    "explain ",
    "describe ",
    "compare ",
    "summarize ",
    "summarise ",
    "what are the differences ",
];

// ---------------------------------------------------------------------------
// Classifier
// ---------------------------------------------------------------------------

/// Classify a user query string into a [`QueryIntent`] with confidence
/// score and per-kind boost multipliers.
///
/// Pure function — no LLM, no I/O. Always returns within microseconds.
pub fn classify_query(query: &str) -> IntentClassification {
    let raw = query.trim();
    if raw.is_empty() {
        return IntentClassification {
            intent: QueryIntent::Unknown,
            confidence: 0.0,
            kind_boosts: KindBoosts::default(),
        };
    }
    // Pad with leading + trailing space so " on monday" etc. match cleanly.
    let lower = format!(" {} ", raw.to_lowercase());

    // Score each candidate intent.
    let proc_score = score_procedural(&lower);
    let epi_score = score_episodic(&lower);
    let fact_score = score_factual(&lower);
    let sem_score = score_semantic(&lower);

    // Pick the highest non-zero score.
    let candidates = [
        (QueryIntent::Procedural, proc_score),
        (QueryIntent::Episodic, epi_score),
        (QueryIntent::Factual, fact_score),
        (QueryIntent::Semantic, sem_score),
    ];

    let (intent, raw_score) = candidates
        .iter()
        .copied()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((QueryIntent::Unknown, 0.0));

    if raw_score <= 0.0 {
        // No signal at all — hand back Unknown so the caller can fall back
        // to its default ranking.
        return IntentClassification {
            intent: QueryIntent::Unknown,
            confidence: 0.0,
            kind_boosts: KindBoosts::default(),
        };
    }

    // Normalise confidence into 0.0–1.0 (cap at 1.0).
    let confidence = (raw_score / 3.0).min(1.0);
    let kind_boosts = boosts_for_intent(intent);
    IntentClassification {
        intent,
        confidence,
        kind_boosts,
    }
}

fn score_procedural(lower_padded: &str) -> f32 {
    let trimmed = lower_padded.trim_start();
    let mut score = 0.0;
    if PROCEDURAL_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
        score += 2.0;
    }
    for v in PROCEDURAL_VERBS {
        if lower_padded.contains(v) {
            score += 0.5;
        }
    }
    if trimmed.starts_with("how ") {
        score += 0.5;
    }
    score
}

fn score_episodic(lower_padded: &str) -> f32 {
    let trimmed = lower_padded.trim_start();
    let mut score = 0.0;
    if EPISODIC_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
        score += 2.0;
    }
    for a in EPISODIC_TIME_ANCHORS {
        if lower_padded.contains(a) {
            score += 0.6;
        }
    }
    score
}

fn score_factual(lower_padded: &str) -> f32 {
    let trimmed = lower_padded.trim_start();
    let mut score = 0.0;
    if FACTUAL_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
        score += 2.0;
    }
    score
}

fn score_semantic(lower_padded: &str) -> f32 {
    let trimmed = lower_padded.trim_start();
    let mut score = 0.0;
    if SEMANTIC_PREFIXES.iter().any(|p| trimmed.starts_with(p)) {
        score += 1.5;
    }
    // Generic question marks without a more specific signal — weak semantic.
    if trimmed.ends_with('?') {
        score += 0.2;
    }
    score
}

/// Default boost shape per intent. Tunable per docs §3.5.6.
fn boosts_for_intent(intent: QueryIntent) -> KindBoosts {
    match intent {
        QueryIntent::Procedural => KindBoosts {
            procedural: 1.5,
            episodic: 0.8,
            semantic: 1.0,
        },
        QueryIntent::Episodic => KindBoosts {
            procedural: 0.8,
            episodic: 1.6,
            semantic: 0.9,
        },
        QueryIntent::Factual => KindBoosts {
            procedural: 0.9,
            episodic: 0.9,
            semantic: 1.3,
        },
        QueryIntent::Semantic => KindBoosts {
            procedural: 1.0,
            episodic: 0.95,
            semantic: 1.2,
        },
        QueryIntent::Unknown => KindBoosts::default(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_is_unknown() {
        let r = classify_query("   ");
        assert_eq!(r.intent, QueryIntent::Unknown);
        assert_eq!(r.confidence, 0.0);
    }

    #[test]
    fn how_to_is_procedural() {
        let r = classify_query("How to install Ollama on Windows?");
        assert_eq!(r.intent, QueryIntent::Procedural);
        assert!(r.confidence > 0.5);
        assert!(r.kind_boosts.procedural > 1.0);
    }

    #[test]
    fn step_by_step_is_procedural() {
        let r = classify_query("Walk me through configuring the brain");
        assert_eq!(r.intent, QueryIntent::Procedural);
    }

    #[test]
    fn how_do_i_is_procedural() {
        let r = classify_query("How do I set up the local LLM?");
        assert_eq!(r.intent, QueryIntent::Procedural);
    }

    #[test]
    fn when_did_is_episodic() {
        let r = classify_query("When did we last discuss the budget?");
        assert_eq!(r.intent, QueryIntent::Episodic);
        assert!(r.kind_boosts.episodic > 1.0);
    }

    #[test]
    fn yesterday_is_episodic() {
        let r = classify_query("What did Alice tell me yesterday about lunch?");
        assert_eq!(r.intent, QueryIntent::Episodic);
    }

    #[test]
    fn what_is_is_factual() {
        let r = classify_query("What is HNSW indexing?");
        assert_eq!(r.intent, QueryIntent::Factual);
        assert!(r.kind_boosts.semantic > 1.0);
    }

    #[test]
    fn who_is_is_factual() {
        let r = classify_query("Who is the author of MotionGPT?");
        assert_eq!(r.intent, QueryIntent::Factual);
    }

    #[test]
    fn explain_is_semantic() {
        let r = classify_query("Explain how RRF fusion works");
        assert_eq!(r.intent, QueryIntent::Semantic);
        assert!(r.kind_boosts.semantic > 1.0);
    }

    #[test]
    fn why_is_semantic() {
        let r = classify_query("Why does HyDE improve retrieval?");
        assert_eq!(r.intent, QueryIntent::Semantic);
    }

    #[test]
    fn ambiguous_short_query_is_unknown_or_low() {
        let r = classify_query("foo bar baz");
        // No prefixes match — score should be 0 → Unknown.
        assert_eq!(r.intent, QueryIntent::Unknown);
    }

    #[test]
    fn confidence_capped_at_one() {
        let r = classify_query(
            "How to install and configure and build and deploy and run step-by-step workflow",
        );
        assert!(r.confidence <= 1.0);
        assert!(r.confidence > 0.5);
    }

    #[test]
    fn kind_boosts_helper_returns_correct_multiplier() {
        let boosts = KindBoosts {
            procedural: 1.5,
            episodic: 0.8,
            semantic: 1.0,
        };
        assert_eq!(boosts.for_kind(CognitiveKind::Procedural), 1.5);
        assert_eq!(boosts.for_kind(CognitiveKind::Episodic), 0.8);
        assert_eq!(boosts.for_kind(CognitiveKind::Semantic), 1.0);
    }

    #[test]
    fn unknown_intent_returns_neutral_boosts() {
        let r = classify_query("");
        assert_eq!(r.kind_boosts.procedural, 1.0);
        assert_eq!(r.kind_boosts.episodic, 1.0);
        assert_eq!(r.kind_boosts.semantic, 1.0);
    }

    #[test]
    fn intent_serde_roundtrip() {
        let intent = QueryIntent::Procedural;
        let json = serde_json::to_string(&intent).unwrap();
        let deser: QueryIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(intent, deser);
        assert!(json.contains("procedural"));
    }

    #[test]
    fn classification_serde_roundtrip() {
        let r = classify_query("How to install?");
        let json = serde_json::to_string(&r).unwrap();
        let deser: IntentClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.intent, QueryIntent::Procedural);
    }

    #[test]
    fn intent_as_str_stable() {
        assert_eq!(QueryIntent::Procedural.as_str(), "procedural");
        assert_eq!(QueryIntent::Episodic.as_str(), "episodic");
        assert_eq!(QueryIntent::Factual.as_str(), "factual");
        assert_eq!(QueryIntent::Semantic.as_str(), "semantic");
        assert_eq!(QueryIntent::Unknown.as_str(), "unknown");
    }

    #[test]
    fn procedural_beats_factual_on_how_questions() {
        // "What is X?" → factual; "How to X?" → procedural.
        let f = classify_query("What is HNSW?");
        let p = classify_query("How to use HNSW?");
        assert_eq!(f.intent, QueryIntent::Factual);
        assert_eq!(p.intent, QueryIntent::Procedural);
    }

    #[test]
    fn episodic_anchor_overcomes_factual_prefix() {
        // "What did I say yesterday" — has factual-ish "what" but also
        // strong episodic prefix + time anchor.
        let r = classify_query("What did I say yesterday about the deadline?");
        assert_eq!(r.intent, QueryIntent::Episodic);
    }
}
