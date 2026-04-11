/// Brain-powered memory operations.
///
/// Reuses the active Ollama model to provide semantic understanding for
/// memory extraction, summarization, and relevance ranking.
///
/// Design principle: all async LLM calls work on plain data (Vec/String),
/// never holding a MutexGuard across an `.await` point.  The caller is
/// responsible for locking the store before/after the async call.
use crate::brain::OllamaAgent;
use crate::memory::{MemoryEntry, MemoryType, NewMemory};

/// Format a flat list of (role, content) pairs into a readable transcript.
pub fn format_transcript(history: &[(String, String)]) -> String {
    history
        .iter()
        .map(|(role, content)| format!("{}: {}", role.to_uppercase(), content))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Async LLM operations (no store interaction) ────────────────────────────────

/// Use the brain to extract memorable facts from a conversation transcript.
///
/// Returns a list of short fact strings. Empty when nothing is worth
/// remembering or when Ollama is unreachable.
pub async fn extract_facts(model: &str, history: &[(String, String)]) -> Vec<String> {
    if history.is_empty() {
        return vec![];
    }
    let transcript = format_transcript(history);
    OllamaAgent::new(model).extract_memories(&transcript).await
}

/// Use the brain to produce a one-paragraph summary of a conversation.
///
/// Returns `None` when the conversation is empty or Ollama is unreachable.
pub async fn summarize(model: &str, history: &[(String, String)]) -> Option<String> {
    if history.is_empty() {
        return None;
    }
    let transcript = format_transcript(history);
    OllamaAgent::new(model)
        .summarize_conversation(&transcript)
        .await
}

/// Use the brain to rank `entries` by relevance to `query`.
///
/// Returns a filtered, re-ordered subset of entries.
/// Falls back to a simple keyword filter when Ollama is unreachable.
pub async fn semantic_search_entries(
    model: &str,
    query: &str,
    entries: &[MemoryEntry],
    limit: usize,
) -> Vec<MemoryEntry> {
    if entries.is_empty() {
        return vec![];
    }

    let candidates: Vec<(i64, String)> = entries
        .iter()
        .map(|e| (e.id, e.content.clone()))
        .collect();

    let agent = OllamaAgent::new(model);
    let relevant_ids = agent
        .semantic_relevant_ids(query, &candidates, limit)
        .await;

    if relevant_ids.is_empty() {
        // Keyword fallback.
        let q = query.to_lowercase();
        return entries
            .iter()
            .filter(|e| {
                e.content.to_lowercase().contains(&q) || e.tags.to_lowercase().contains(&q)
            })
            .take(limit)
            .cloned()
            .collect();
    }

    entries
        .iter()
        .filter(|e| relevant_ids.contains(&e.id))
        .cloned()
        .collect()
}

// ── Sync store operations (no async) ──────────────────────────────────────────

/// Save a list of fact strings into the store.  Returns the count stored.
pub fn save_facts(facts: &[String], store: &crate::memory::MemoryStore) -> usize {
    facts
        .iter()
        .filter(|f| f.len() >= 5)
        .filter_map(|fact| {
            store
                .add(NewMemory {
                    content: fact.clone(),
                    tags: "auto-extracted".to_string(),
                    importance: 3,
                    memory_type: MemoryType::Fact,
                })
                .ok()
        })
        .count()
}

/// Save a conversation summary into the store.
pub fn save_summary(summary: &str, store: &crate::memory::MemoryStore) -> bool {
    store
        .add(NewMemory {
            content: summary.to_string(),
            tags: "session-summary".to_string(),
            importance: 4,
            memory_type: MemoryType::Summary,
        })
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryStore, NewMemory, MemoryType};

    fn sample_history() -> Vec<(String, String)> {
        vec![
            ("user".to_string(), "I mostly code in Python".to_string()),
            ("assistant".to_string(), "Great choice!".to_string()),
        ]
    }

    fn store_with_entries() -> MemoryStore {
        let s = MemoryStore::in_memory();
        s.add(NewMemory {
            content: "User prefers Python".to_string(),
            tags: "language".to_string(),
            importance: 4,
            memory_type: MemoryType::Preference,
        })
        .unwrap();
        s.add(NewMemory {
            content: "User is building a neural network".to_string(),
            tags: "project,ml".to_string(),
            importance: 5,
            memory_type: MemoryType::Context,
        })
        .unwrap();
        s
    }

    #[test]
    fn format_transcript_uppercases_roles() {
        let t = format_transcript(&sample_history());
        assert!(t.contains("USER: I mostly code in Python"));
        assert!(t.contains("ASSISTANT: Great choice!"));
    }

    #[test]
    fn save_facts_filters_short_strings() {
        let store = MemoryStore::in_memory();
        let facts = vec!["ok".to_string(), "User likes dark mode".to_string()];
        let n = save_facts(&facts, &store);
        assert_eq!(n, 1);
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn save_summary_stores_entry() {
        let store = MemoryStore::in_memory();
        let ok = save_summary("User discussed Python and ML projects.", &store);
        assert!(ok);
        assert_eq!(store.count(), 1);
        let entry = store.get_all().unwrap()[0].clone();
        assert_eq!(entry.memory_type, MemoryType::Summary);
        assert_eq!(entry.importance, 4);
    }

    #[tokio::test]
    async fn extract_facts_empty_history_returns_empty() {
        let facts = extract_facts("gemma3:4b", &[]).await;
        assert!(facts.is_empty());
    }

    #[tokio::test]
    async fn summarize_empty_history_returns_none() {
        let result = summarize("gemma3:4b", &[]).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn semantic_search_empty_entries_returns_empty() {
        let result = semantic_search_entries("gemma3:4b", "Python", &[], 5).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn semantic_search_falls_back_to_keyword_when_ollama_unavailable() {
        // Ollama won't be running in CI; should fall back to keyword filter.
        let store = store_with_entries();
        let entries = store.get_all().unwrap();
        let results =
            semantic_search_entries("gemma3:4b", "Python", &entries, 5).await;
        // Keyword fallback should find the "User prefers Python" entry.
        assert!(results.iter().any(|e| e.content.contains("Python")));
    }
}
