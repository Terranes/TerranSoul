//! Cognitive memory classification — the **episodic / semantic / procedural**
//! axis (orthogonal to [`MemoryType`] and [`MemoryTier`]).
//!
//! ## Why a third axis?
//!
//! TerranSoul already carries two memory dimensions:
//!
//! - [`MemoryType`] — **structural** (how the memory was produced):
//!   `fact` / `preference` / `context` / `summary`.
//! - [`MemoryTier`] — **lifecycle** (where the memory lives now):
//!   `short` / `working` / `long`.
//!
//! Cognitive psychology distinguishes a third dimension that neither of those
//! captures cleanly:
//!
//! | Cognitive kind | Description                                              | TerranSoul examples                              |
//! |----------------|----------------------------------------------------------|--------------------------------------------------|
//! | **Episodic**   | Time- and place-anchored personal experiences            | "On April 22nd Alex finished the rust refactor"  |
//! | **Semantic**   | Time-independent general knowledge & stable preferences  | "Rust uses ownership for memory safety", "Alex prefers dark mode" |
//! | **Procedural** | How-to knowledge, learned skills, repeatable routines    | "How to ship a release: bump → tag → push"       |
//!
//! The full design discussion lives in `docs/brain-advanced-design.md`
//! § "Cognitive Memory Axes (Episodic / Semantic / Procedural)".
//!
//! ## Why no schema migration?
//!
//! Following the explicit guidance in the design doc, this module classifies
//! the cognitive kind from `(MemoryType, tags, content)` instead of adding a
//! `cognitive_kind` column. Tag prefixes (`episodic:*`, `semantic:*`,
//! `procedural:*`) override the heuristic, so power users can tag memories
//! authoritatively without waiting for a schema migration. If we later need
//! the column for indexed retrieval, a V6 migration is straightforward.

use serde::{Deserialize, Serialize};

use super::store::MemoryType;

/// Cognitive memory kind — a third axis on top of [`MemoryType`] / [`MemoryTier`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CognitiveKind {
    /// Time- and place-anchored personal experiences.
    Episodic,
    /// Time-independent knowledge and stable preferences.
    #[default]
    Semantic,
    /// How-to knowledge, learned skills, repeatable routines.
    Procedural,
    /// Persisted rules, heuristics, and value judgments the LLM should follow.
    Judgment,
}

impl CognitiveKind {
    /// Stable string serialisation used in tag prefixes and comparisons.
    pub fn as_str(self) -> &'static str {
        match self {
            CognitiveKind::Episodic => "episodic",
            CognitiveKind::Semantic => "semantic",
            CognitiveKind::Procedural => "procedural",
            CognitiveKind::Judgment => "judgment",
        }
    }
}

/// Verbs commonly associated with procedural how-to knowledge.
const PROCEDURAL_VERBS: &[&str] = &[
    "how to",
    "how-to",
    "step ",
    "steps to",
    "first,",
    "next,",
    "finally,",
    "procedure",
    "process for",
    "workflow:",
    "recipe",
    "method:",
    "algorithm:",
];

/// Words/punctuation that strongly suggest an episodic (time-anchored) memory.
const EPISODIC_HINTS: &[&str] = &[
    "yesterday",
    "today",
    "this morning",
    "this afternoon",
    "this evening",
    "last night",
    "last week",
    "last month",
    "earlier today",
    "just now",
    "on monday",
    "on tuesday",
    "on wednesday",
    "on thursday",
    "on friday",
    "on saturday",
    "on sunday",
    " ago",
    "happened",
    "occurred",
    "we met",
    "i met",
    "we visited",
    "i went",
    "we went",
];

/// Classify the cognitive kind of a memory from `(memory_type, tags, content)`.
///
/// Resolution order:
///
/// 1. **Explicit tag** — if `tags` contains `"episodic"`, `"semantic"`, or
///    `"procedural"` (or the same word as a `prefix:value` head, e.g.
///    `"episodic:meeting"`), that wins.
/// 2. **Structural type heuristic** — `Summary` is almost always episodic
///    (it recaps a session); `Preference` is almost always semantic.
/// 3. **Content heuristics** — date-anchored language → episodic; how-to
///    verbs → procedural; everything else → semantic (the safe default).
pub fn classify(memory_type: &MemoryType, tags: &str, content: &str) -> CognitiveKind {
    if let Some(k) = classify_from_tags(tags) {
        return k;
    }
    if let Some(k) = classify_from_type(memory_type) {
        return k;
    }
    classify_from_content(content)
}

/// Look for an explicit `episodic`/`semantic`/`procedural` tag (optionally with
/// `:detail` suffix and arbitrary surrounding whitespace/comma separators).
fn classify_from_tags(tags: &str) -> Option<CognitiveKind> {
    for raw in tags.split([',', ' ', '\n', '\t']) {
        let token = raw.trim().to_lowercase();
        let head = token.split(':').next().unwrap_or(&token);
        match head {
            "episodic" => return Some(CognitiveKind::Episodic),
            "semantic" => return Some(CognitiveKind::Semantic),
            "procedural" => return Some(CognitiveKind::Procedural),
            "judgment" => return Some(CognitiveKind::Judgment),
            _ => {}
        }
    }
    None
}

/// Map [`MemoryType`] variants whose cognitive kind is essentially fixed.
///
/// Returns `None` for `Fact` and `Context` because those can be either
/// episodic or semantic depending on the content (a fact like "the meeting
/// started at 9am" is episodic; a fact like "Mars has two moons" is semantic).
fn classify_from_type(memory_type: &MemoryType) -> Option<CognitiveKind> {
    match memory_type {
        MemoryType::Summary => Some(CognitiveKind::Episodic),
        MemoryType::Preference => Some(CognitiveKind::Semantic),
        MemoryType::Fact | MemoryType::Context => None,
    }
}

/// Lightweight content-based heuristic. Pure function — no I/O, no LLM.
fn classify_from_content(content: &str) -> CognitiveKind {
    let lower = content.to_lowercase();

    // Procedural cues are the strongest signal because how-to language is
    // distinctive ("how to", numbered steps, "first/next/finally").
    if PROCEDURAL_VERBS.iter().any(|verb| lower.contains(verb)) {
        return CognitiveKind::Procedural;
    }
    // Numbered lists ("1. ... 2. ... 3. ...") are also procedural.
    if has_numbered_list(&lower) {
        return CognitiveKind::Procedural;
    }
    // Episodic cues — explicit time anchors.
    if EPISODIC_HINTS.iter().any(|hint| lower.contains(hint)) {
        return CognitiveKind::Episodic;
    }
    // Default to semantic — generic knowledge or stable preference.
    CognitiveKind::Semantic
}

/// Detect a numbered-list shape like `"1. foo 2. bar"` (no regex dep).
fn has_numbered_list(content: &str) -> bool {
    // Look for at least two of "1.", "2.", "3."… within the content.
    let mut count = 0;
    for n in 1..=9 {
        let needle = format!("{n}.");
        if content.contains(&needle) {
            count += 1;
            if count >= 2 {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_episodic_tag_wins() {
        let k = classify(&MemoryType::Fact, "episodic, work", "Mars has two moons");
        assert_eq!(k, CognitiveKind::Episodic);
    }

    #[test]
    fn explicit_semantic_tag_wins_over_episodic_content() {
        let k = classify(
            &MemoryType::Fact,
            "semantic",
            "Yesterday I learned Mars has two moons",
        );
        assert_eq!(k, CognitiveKind::Semantic);
    }

    #[test]
    fn explicit_procedural_tag_wins() {
        let k = classify(&MemoryType::Context, "procedural:release", "bump tag push");
        assert_eq!(k, CognitiveKind::Procedural);
    }

    #[test]
    fn tag_prefix_with_detail_is_recognised() {
        let k = classify(&MemoryType::Fact, "episodic:meeting", "team sync notes");
        assert_eq!(k, CognitiveKind::Episodic);
    }

    #[test]
    fn summary_type_classifies_as_episodic_by_default() {
        let k = classify(&MemoryType::Summary, "", "Discussed the rust refactor.");
        assert_eq!(k, CognitiveKind::Episodic);
    }

    #[test]
    fn preference_type_classifies_as_semantic() {
        let k = classify(&MemoryType::Preference, "", "User prefers dark mode");
        assert_eq!(k, CognitiveKind::Semantic);
    }

    #[test]
    fn fact_with_episodic_time_anchor_is_episodic() {
        let k = classify(
            &MemoryType::Fact,
            "",
            "Yesterday Alex finished the rust refactor",
        );
        assert_eq!(k, CognitiveKind::Episodic);
    }

    #[test]
    fn fact_with_general_knowledge_is_semantic() {
        let k = classify(
            &MemoryType::Fact,
            "",
            "Rust uses ownership for memory safety",
        );
        assert_eq!(k, CognitiveKind::Semantic);
    }

    #[test]
    fn how_to_content_is_procedural() {
        let k = classify(
            &MemoryType::Fact,
            "",
            "How to ship: bump version, tag, push",
        );
        assert_eq!(k, CognitiveKind::Procedural);
    }

    #[test]
    fn numbered_list_is_procedural() {
        let k = classify(
            &MemoryType::Context,
            "",
            "Release process: 1. bump version 2. tag commit 3. push tag",
        );
        assert_eq!(k, CognitiveKind::Procedural);
    }

    #[test]
    fn single_numbered_item_is_not_procedural() {
        let k = classify(&MemoryType::Fact, "", "Item 1. is not a list");
        assert_eq!(k, CognitiveKind::Semantic);
    }

    #[test]
    fn context_with_no_hints_defaults_to_semantic() {
        let k = classify(
            &MemoryType::Context,
            "",
            "Working on the marketplace feature",
        );
        assert_eq!(k, CognitiveKind::Semantic);
    }

    #[test]
    fn as_str_is_stable() {
        assert_eq!(CognitiveKind::Episodic.as_str(), "episodic");
        assert_eq!(CognitiveKind::Semantic.as_str(), "semantic");
        assert_eq!(CognitiveKind::Procedural.as_str(), "procedural");
    }

    #[test]
    fn classify_is_pure_no_panics_on_empty_input() {
        let k = classify(&MemoryType::Fact, "", "");
        assert_eq!(k, CognitiveKind::Semantic);
    }

    #[test]
    fn unrecognised_tags_fall_through_to_heuristic() {
        let k = classify(&MemoryType::Fact, "work, important", "Mars has two moons");
        assert_eq!(k, CognitiveKind::Semantic);
    }

    #[test]
    fn comma_separated_tag_list_works() {
        let k = classify(&MemoryType::Fact, "work,episodic,important", "x");
        assert_eq!(k, CognitiveKind::Episodic);
    }

    #[test]
    fn first_recognised_tag_wins() {
        // When multiple cognitive tags are present (a misuse), the first wins.
        let k = classify(&MemoryType::Fact, "episodic semantic", "x");
        assert_eq!(k, CognitiveKind::Episodic);
    }

    #[test]
    fn case_insensitive_tag_match() {
        let k = classify(&MemoryType::Fact, "Episodic", "x");
        assert_eq!(k, CognitiveKind::Episodic);
    }
}
