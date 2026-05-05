//! Persisted judgment-rules artefact.
//!
//! Judgments are memories tagged `judgment:*` that hold user-defined rules,
//! heuristics, or value-statements the LLM should follow during chat.
//! They are stored in the regular memories table but classified as
//! [`CognitiveKind::Judgment`] and injected into the system prompt as a
//! dedicated `[JUDGMENT RULES]` block.

use crate::memory::cognitive_kind::{classify, CognitiveKind};
use crate::memory::store::{MemoryEntry, MemoryStore, MemoryType, NewMemory};

/// Insert a new judgment rule into the memory store.
///
/// The `judgment` tag prefix is added automatically if not already present.
pub fn add_judgment(
    store: &MemoryStore,
    content: &str,
    tags: &str,
    importance: i64,
) -> Result<MemoryEntry, String> {
    let mut final_tags = normalise_judgment_tags(tags);
    if final_tags.is_empty() {
        final_tags = "judgment".to_string();
    }
    store
        .add(NewMemory {
            content: content.to_string(),
            tags: final_tags,
            importance: importance.clamp(1, 5),
            memory_type: MemoryType::Fact,
            ..Default::default()
        })
        .map_err(|e| e.to_string())
}

/// List all memories classified as `CognitiveKind::Judgment`.
pub fn list_judgments(store: &MemoryStore) -> Vec<MemoryEntry> {
    store
        .get_all()
        .unwrap_or_default()
        .into_iter()
        .filter(|e| classify(&e.memory_type, &e.tags, &e.content) == CognitiveKind::Judgment)
        .collect()
}

/// Search judgments relevant to a query using keyword match.
/// Returns top-N judgment entries sorted by relevance (importance-weighted).
pub fn apply_judgments(store: &MemoryStore, query: &str, limit: usize) -> Vec<MemoryEntry> {
    let all = list_judgments(store);
    if all.is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();

    // Score each judgment by keyword overlap with the query
    let mut scored: Vec<(f64, MemoryEntry)> = all
        .into_iter()
        .map(|entry| {
            let content_lower = entry.content.to_lowercase();
            let word_hits = query_words
                .iter()
                .filter(|w| w.len() > 2 && content_lower.contains(*w))
                .count();
            // Base relevance from word overlap + importance boost
            let score = word_hits as f64 + (entry.importance as f64 * 0.1);
            (score, entry)
        })
        .collect();

    // Sort descending by score
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    scored
        .into_iter()
        .take(limit)
        .map(|(_, entry)| entry)
        .collect()
}

/// Format judgment entries into the system-prompt injection block.
pub fn format_judgment_block(judgments: &[MemoryEntry]) -> String {
    if judgments.is_empty() {
        return String::new();
    }
    let entries: String = judgments
        .iter()
        .map(|j| format!("- {}", j.content))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "\n\n[JUDGMENT RULES]\n\
The following user-defined rules and heuristics apply to this conversation:\n\
{entries}\n\
[/JUDGMENT RULES]"
    )
}

/// Ensure the tag string contains the `judgment` prefix.
fn normalise_judgment_tags(tags: &str) -> String {
    let lower = tags.to_lowercase();
    let has_judgment = lower
        .split([',', ' ', '\n', '\t'])
        .any(|t| t.trim().starts_with("judgment"));
    if has_judgment {
        tags.to_string()
    } else if tags.trim().is_empty() {
        "judgment".to_string()
    } else {
        format!("judgment,{tags}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::MemoryStore;

    fn test_store() -> MemoryStore {
        MemoryStore::in_memory()
    }

    #[test]
    fn add_judgment_adds_tag() {
        let store = test_store();
        let entry = add_judgment(&store, "Always prefer short answers", "style", 4).unwrap();
        assert!(entry.tags.contains("judgment"));
        assert!(entry.tags.contains("style"));
    }

    #[test]
    fn add_judgment_no_duplicate_tag() {
        let store = test_store();
        let entry = add_judgment(&store, "Never use emojis", "judgment:communication", 3).unwrap();
        // Should not double-add
        let tag_count = entry.tags.to_lowercase().matches("judgment").count();
        assert_eq!(tag_count, 1);
    }

    #[test]
    fn list_judgments_filters_correctly() {
        let store = test_store();
        // Add a judgment
        add_judgment(&store, "Be concise", "", 3).unwrap();
        // Add a non-judgment memory
        store
            .add(NewMemory {
                content: "User likes Rust".to_string(),
                tags: "preference".to_string(),
                importance: 3,
                memory_type: MemoryType::Preference,
                ..Default::default()
            })
            .unwrap();

        let judgments = list_judgments(&store);
        assert_eq!(judgments.len(), 1);
        assert!(judgments[0].content.contains("Be concise"));
    }

    #[test]
    fn apply_judgments_returns_relevant() {
        let store = test_store();
        add_judgment(&store, "Always use formal tone in emails", "", 4).unwrap();
        add_judgment(&store, "Prefer short code comments", "", 3).unwrap();
        add_judgment(&store, "Never reveal private data", "", 5).unwrap();

        let results = apply_judgments(&store, "writing an email to a client", 2);
        assert!(!results.is_empty());
        assert!(results.len() <= 2);
    }

    #[test]
    fn format_judgment_block_empty_returns_empty() {
        assert_eq!(format_judgment_block(&[]), String::new());
    }

    #[test]
    fn format_judgment_block_non_empty() {
        let store = test_store();
        let entry = add_judgment(&store, "Stay brief", "", 3).unwrap();
        let block = format_judgment_block(&[entry]);
        assert!(block.contains("[JUDGMENT RULES]"));
        assert!(block.contains("Stay brief"));
        assert!(block.contains("[/JUDGMENT RULES]"));
    }
}
