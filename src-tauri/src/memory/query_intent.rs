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
            CognitiveKind::Negative => self.semantic, // negatives boosted as semantic
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

/// Whether HyDE (Hypothetical Document Embeddings) is likely to help
/// retrieval for a query of this intent class.
///
/// BENCH-CHAT-PARITY-2 / BENCH-LCM-10 — HyDE is a *per-query-class*
/// expansion: writing a hypothetical answer first sharpens cosine
/// similarity for queries whose target documents live in a different
/// surface form (multi-hop reasoning, temporal episodes, abstract
/// concepts), but **hurts** factoid / open-domain lookups where the
/// hypothetical drifts away from the literal answer span.
///
/// Mapping from the existing [`QueryIntent`] taxonomy:
/// - `Episodic` → temporal class (HyDE writes a plausible episode) → **on**
/// - `Semantic` → abstract / multi-hop reasoning class → **on**
/// - `Procedural` → single-shot how-to with deterministic span → **off**
/// - `Factual` → factoid lookup (drift hurts) → **off**
/// - `Unknown` → no signal, skip HyDE to keep TTFT down → **off**
///
/// Pure function — no I/O, microseconds.
pub fn hyde_recommended(intent: QueryIntent) -> bool {
    matches!(intent, QueryIntent::Episodic | QueryIntent::Semantic)
}

/// Convenience gate: classify `query` and return whether HyDE should
/// fire **given** the brain is available. Returns `false` when
/// `brain_available` is `false`, regardless of class — HyDE needs an LLM
/// hop.
///
/// Cloud-streaming chat (`commands::streaming::stream_cloud`) calls this
/// before deciding to invoke `OllamaAgent::hyde_complete`. The
/// VRAM-safe local-Ollama streaming path and Self-RAG loop do not call
/// this — they use the sync retrieval helper directly.
pub fn should_run_hyde(query: &str, brain_available: bool) -> bool {
    if !brain_available {
        return false;
    }
    hyde_recommended(classify_query(query).intent)
}

// ---------------------------------------------------------------------------
// GRAPHRAG-1c — Query scope classification (global vs local routing)
// ---------------------------------------------------------------------------

/// Query scope — determines which retrieval path to use.
///
/// - `Global` → route to high-level community summaries (top hierarchy
///   levels). Best for broad, thematic questions ("What topics have we
///   discussed?", "Give me an overview of X").
/// - `Local` → route to entity-walk + hybrid_search_rrf. Best for specific
///   factual lookups ("What did Alice say about the budget?").
/// - `Mixed` → use the standard dual-level RRF fusion (community + entity).
///   Default when scope cannot be determined confidently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QueryScope {
    /// Broad thematic/summary queries → community summaries.
    Global,
    /// Specific entity/fact lookups → entity-walk + RRF.
    Local,
    /// Ambiguous or multi-faceted → dual-level fusion (default).
    #[default]
    Mixed,
}

impl QueryScope {
    pub fn as_str(self) -> &'static str {
        match self {
            QueryScope::Global => "global",
            QueryScope::Local => "local",
            QueryScope::Mixed => "mixed",
        }
    }
}

/// Lower-case prefixes/keywords that indicate global (broad/thematic) scope.
const GLOBAL_INDICATORS: &[&str] = &[
    "summarize ",
    "summarise ",
    "overview of ",
    "what topics ",
    "what themes ",
    "what categories ",
    "what are the main ",
    "what have we discussed ",
    "what do you know about everything",
    "big picture ",
    "high-level ",
    "high level ",
    "broadly ",
    "in general ",
    "overall ",
    "give me a summary ",
    "list all topics ",
    "list all categories ",
    "what are all the ",
];

/// Lower-case prefixes/keywords that indicate local (specific/entity) scope.
const LOCAL_INDICATORS: &[&str] = &[
    "what did ",
    "who said ",
    "where is ",
    "when did ",
    "where did ",
    "tell me about ",
    "find ",
    "look up ",
    "what is the name ",
    "what was the ",
    "specifically ",
    "exactly ",
    "detail about ",
    "details about ",
    "details on ",
];

/// Classify the scope of a query (global / local / mixed).
///
/// Pure heuristic — no LLM, microsecond latency. Checks for global
/// indicators first (broad/thematic signals), then local indicators
/// (specific entity/fact signals). If both or neither fire, returns Mixed.
pub fn classify_scope(query: &str) -> QueryScope {
    let raw = query.trim();
    if raw.is_empty() {
        return QueryScope::Mixed;
    }
    let lower = format!(" {} ", raw.to_lowercase());
    let trimmed = lower.trim_start();

    let global_score: usize = GLOBAL_INDICATORS
        .iter()
        .filter(|ind| trimmed.contains(*ind))
        .count();

    let local_score: usize = LOCAL_INDICATORS
        .iter()
        .filter(|ind| trimmed.contains(*ind))
        .count();

    if global_score > 0 && local_score == 0 {
        QueryScope::Global
    } else if local_score > 0 && global_score == 0 {
        QueryScope::Local
    } else {
        // Both or neither — fall back to mixed (standard dual-level RRF).
        QueryScope::Mixed
    }
}

/// Full classification result including both intent and scope axes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullClassification {
    pub intent: IntentClassification,
    pub scope: QueryScope,
}

/// Classify both intent and scope in a single call.
pub fn classify_full(query: &str) -> FullClassification {
    FullClassification {
        intent: classify_query(query),
        scope: classify_scope(query),
    }
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

    // ------------------------------------------------------------------
    // BENCH-CHAT-PARITY-2 — HyDE per-query-class gating
    // ------------------------------------------------------------------

    #[test]
    fn hyde_recommended_for_semantic_and_episodic_only() {
        assert!(hyde_recommended(QueryIntent::Semantic));
        assert!(hyde_recommended(QueryIntent::Episodic));
        assert!(!hyde_recommended(QueryIntent::Procedural));
        assert!(!hyde_recommended(QueryIntent::Factual));
        assert!(!hyde_recommended(QueryIntent::Unknown));
    }

    #[test]
    fn should_run_hyde_requires_brain_available() {
        // Semantic class would normally fire HyDE, but without a brain
        // we cannot run the LLM hop → gate is off.
        assert!(!should_run_hyde("Explain how RRF fusion works", false));
        assert!(should_run_hyde("Explain how RRF fusion works", true));
    }

    #[test]
    fn should_run_hyde_off_for_factoid_lookups() {
        // BENCH-LCM-10: HyDE hurts open / factoid queries — gate must be off.
        assert!(!should_run_hyde("What is HNSW?", true));
        assert!(!should_run_hyde("Who is the author of MotionGPT?", true));
        assert!(!should_run_hyde("How do I install Ollama?", true));
    }

    #[test]
    fn should_run_hyde_on_for_temporal_and_abstract() {
        // BENCH-LCM-10: HyDE helps temporal/abstract/multi-hop classes.
        assert!(should_run_hyde(
            "What did Alice tell me yesterday about lunch?",
            true
        ));
        assert!(should_run_hyde("Why does HyDE improve retrieval?", true));
        assert!(should_run_hyde(
            "Compare RRF fusion against pure cosine retrieval",
            true
        ));
    }

    #[test]
    fn should_run_hyde_off_for_unclassified_queries() {
        // No signal at all → Unknown → skip HyDE to protect TTFT.
        assert!(!should_run_hyde("foo bar baz", true));
        assert!(!should_run_hyde("", true));
    }

    // ------------------------------------------------------------------
    // GRAPHRAG-1c — Scope classification tests
    // ------------------------------------------------------------------

    #[test]
    fn scope_empty_is_mixed() {
        assert_eq!(classify_scope(""), QueryScope::Mixed);
        assert_eq!(classify_scope("   "), QueryScope::Mixed);
    }

    #[test]
    fn scope_summarize_is_global() {
        assert_eq!(
            classify_scope("Summarize everything we've discussed"),
            QueryScope::Global
        );
    }

    #[test]
    fn scope_overview_is_global() {
        assert_eq!(
            classify_scope("Give me an overview of our conversations"),
            QueryScope::Global
        );
    }

    #[test]
    fn scope_what_topics_is_global() {
        assert_eq!(
            classify_scope("What topics have we covered so far?"),
            QueryScope::Global
        );
    }

    #[test]
    fn scope_high_level_is_global() {
        assert_eq!(
            classify_scope("Give me a high-level view of the project"),
            QueryScope::Global
        );
    }

    #[test]
    fn scope_what_did_is_local() {
        assert_eq!(
            classify_scope("What did Alice say about the budget?"),
            QueryScope::Local
        );
    }

    #[test]
    fn scope_find_is_local() {
        assert_eq!(
            classify_scope("Find the recipe for chocolate cake"),
            QueryScope::Local
        );
    }

    #[test]
    fn scope_tell_me_about_is_local() {
        assert_eq!(
            classify_scope("Tell me about the meeting notes from Friday"),
            QueryScope::Local
        );
    }

    #[test]
    fn scope_specifically_is_local() {
        assert_eq!(
            classify_scope("What specifically did we decide about pricing?"),
            QueryScope::Local
        );
    }

    #[test]
    fn scope_ambiguous_is_mixed() {
        // No global or local indicators → mixed.
        assert_eq!(
            classify_scope("How does the brain system work?"),
            QueryScope::Mixed
        );
    }

    #[test]
    fn scope_no_signal_is_mixed() {
        assert_eq!(classify_scope("hello world"), QueryScope::Mixed);
    }

    #[test]
    fn scope_serde_roundtrip() {
        let scope = QueryScope::Global;
        let json = serde_json::to_string(&scope).unwrap();
        let deser: QueryScope = serde_json::from_str(&json).unwrap();
        assert_eq!(scope, deser);
        assert!(json.contains("global"));
    }

    #[test]
    fn scope_as_str_stable() {
        assert_eq!(QueryScope::Global.as_str(), "global");
        assert_eq!(QueryScope::Local.as_str(), "local");
        assert_eq!(QueryScope::Mixed.as_str(), "mixed");
    }

    #[test]
    fn full_classification_combines_both_axes() {
        let full = classify_full("Summarize everything about the project");
        assert_eq!(full.scope, QueryScope::Global);
        // "Summarize" prefix also matches semantic intent.
        assert_eq!(full.intent.intent, QueryIntent::Semantic);
    }

    #[test]
    fn full_classification_local_episodic() {
        let full = classify_full("When did we discuss the budget?");
        assert_eq!(full.scope, QueryScope::Local);
        assert_eq!(full.intent.intent, QueryIntent::Episodic);
    }
}
