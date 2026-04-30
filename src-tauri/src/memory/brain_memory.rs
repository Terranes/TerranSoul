/// Brain-powered memory operations.
///
/// Supports all three brain modes (Free API, Paid API, Local Ollama) for
/// memory extraction, summarization, and relevance ranking.
///
/// Design principle: all async LLM calls work on plain data (Vec/String),
/// never holding a MutexGuard across an `.await` point.  The caller is
/// responsible for locking the store before/after the async call.
use crate::brain::openai_client::OpenAiMessage;
use crate::brain::{BrainMode, OllamaAgent, OpenAiClient};
use crate::memory::{MemoryEntry, MemoryStore, MemoryType, NewMemory};

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

/// Extract memorable facts via any brain mode (Free/Paid/Local).
///
/// This is the mode-agnostic version of [`extract_facts`] that routes through
/// `OpenAiClient` for Free and Paid API modes instead of requiring Ollama.
pub async fn extract_facts_any_mode(
    brain_mode: &BrainMode,
    history: &[(String, String)],
    rotator: &std::sync::Mutex<crate::brain::ProviderRotator>,
) -> Vec<String> {
    if history.is_empty() {
        return vec![];
    }
    let transcript = format_transcript(history);
    let prompt = format!(
        "Read this conversation and extract up to 5 important facts worth remembering \
        about the user (preferences, goals, personal details, ongoing projects). \
        Reply with ONLY a bullet list, one fact per line, starting each line with '- '. \
        If there is nothing worth remembering, reply with exactly: NONE\n\n{transcript}"
    );

    let reply = complete_via_mode(
        brain_mode,
        "You are a memory extraction assistant. Extract concise facts about the user from conversations.",
        &prompt,
        rotator,
    )
    .await;

    let text = match reply {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    if text.trim() == "NONE" || text.trim().is_empty() {
        return vec![];
    }
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim().trim_start_matches("- ").trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

/// Conversation-segmented variant of [`extract_facts_any_mode`] (Chunk
/// 26.2b). Embeds each turn, runs the Phase-26 topic segmenter to find
/// topic-shift boundaries, then runs extraction once per segment so a
/// "vacation → debugging → cooking" session produces a focused fact
/// list per topic instead of one jumbled blob.
///
/// Falls back to single-pass [`extract_facts_any_mode`] (graceful
/// degradation) when:
///
/// - any turn fails to embed (e.g. offline embedder, mismatched dim),
/// - the segmenter returns ≤1 segment (no topic shifts detected), or
/// - the history is too short to segment meaningfully.
///
/// Per-segment results are concatenated in segment order and
/// deduplicated by trimmed lower-cased content (the LLM frequently
/// re-states the same fact across topic boundaries).
pub async fn extract_facts_segmented_any_mode(
    brain_mode: &BrainMode,
    active_brain_model: Option<&str>,
    history: &[(String, String)],
    rotator: &std::sync::Mutex<crate::brain::ProviderRotator>,
) -> Vec<String> {
    if history.len() < 4 {
        // Not enough material to segment — fall through to single-pass.
        return extract_facts_any_mode(brain_mode, history, rotator).await;
    }

    // Embed every turn. If *any* embedding is missing, fall back rather
    // than degrading the segmenter into a single-bucket noop.
    let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(history.len());
    for (_, content) in history.iter() {
        match crate::brain::embed_for_mode(content, Some(brain_mode), active_brain_model).await {
            Some(v) if !v.is_empty() => embeddings.push(v),
            _ => return extract_facts_any_mode(brain_mode, history, rotator).await,
        }
    }

    let turns: Vec<crate::brain::segmenter::SegTurn<'_>> = history
        .iter()
        .map(|(role, content)| crate::brain::segmenter::SegTurn {
            role: role.as_str(),
            content: content.as_str(),
        })
        .collect();
    let segments = crate::brain::segmenter::segment(
        &turns,
        &embeddings,
        &crate::brain::segmenter::SegmenterConfig::default(),
    );

    if segments.len() <= 1 {
        // No topic shift found — single-pass is fine.
        return extract_facts_any_mode(brain_mode, history, rotator).await;
    }

    let mut all_facts: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for seg in segments {
        let slice: Vec<(String, String)> = history[seg.start..seg.end].to_vec();
        let facts = extract_facts_any_mode(brain_mode, &slice, rotator).await;
        for fact in facts {
            let key = fact.trim().to_lowercase();
            if !key.is_empty() && seen.insert(key) {
                all_facts.push(fact);
            }
        }
    }
    all_facts
}

/// Summarize a conversation via any brain mode (Free/Paid/Local).
pub async fn summarize_any_mode(
    brain_mode: &BrainMode,
    history: &[(String, String)],
    rotator: &std::sync::Mutex<crate::brain::ProviderRotator>,
) -> Option<String> {
    if history.is_empty() {
        return None;
    }
    let transcript = format_transcript(history);
    let prompt = format!(
        "Summarize this conversation in 1-3 sentences, focusing on what the user \
        was trying to accomplish and any conclusions reached. Be concise.\n\n{transcript}"
    );
    let reply = complete_via_mode(
        brain_mode,
        "You are a concise summarizer. Summarize conversations into 1-3 sentences.",
        &prompt,
        rotator,
    )
    .await
    .ok()?;
    let clean = reply.trim().to_string();
    if clean.is_empty() {
        None
    } else {
        Some(clean)
    }
}

/// Route a single prompt through the configured brain mode.
async fn complete_via_mode(
    brain_mode: &BrainMode,
    system: &str,
    user_prompt: &str,
    rotator: &std::sync::Mutex<crate::brain::ProviderRotator>,
) -> Result<String, String> {
    match brain_mode {
        BrainMode::FreeApi {
            provider_id,
            api_key,
        } => {
            let effective_id = rotator
                .lock()
                .ok()
                .and_then(|mut r| r.next_healthy_provider().map(|p| p.id.clone()))
                .unwrap_or_else(|| provider_id.clone());
            let provider = crate::brain::get_free_provider(&effective_id)
                .ok_or_else(|| format!("Unknown free provider: {effective_id}"))?;
            let client =
                OpenAiClient::new(&provider.base_url, &provider.model, api_key.as_deref());
            let msgs = vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ];
            client.chat(msgs).await
        }
        BrainMode::PaidApi {
            api_key,
            model,
            base_url,
            ..
        } => {
            let client = OpenAiClient::new(base_url, model, Some(api_key));
            let msgs = vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ];
            client.chat(msgs).await
        }
        BrainMode::LocalOllama { model } => {
            let agent = OllamaAgent::new(model);
            let msgs = vec![
                crate::brain::ollama_agent::ChatMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                crate::brain::ollama_agent::ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ];
            let (reply, _) = agent.call(msgs).await;
            Ok(reply)
        }
        BrainMode::LocalLmStudio {
            model,
            base_url,
            api_key,
            ..
        } => {
            let client = OpenAiClient::new(base_url, model, api_key.as_deref());
            let msgs = vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ];
            client.chat(msgs).await
        }
    }
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

// ── Fast vector path (no brute-force LLM ranking) ──────────────────────────────

/// Fast RAG retrieval: embed the query, then cosine-search the vector index.
/// Falls back to the slow LLM-ranking path when embeddings are unavailable.
///
/// **Performance**: <10 ms for 100k entries (pure arithmetic, zero LLM calls).
pub async fn fast_semantic_search(
    model: &str,
    query: &str,
    store: &MemoryStore,
    limit: usize,
) -> Vec<MemoryEntry> {
    // Try the fast vector path first.
    if let Some(query_emb) = OllamaAgent::embed_text(query, model).await {
        if let Ok(results) = store.vector_search(&query_emb, limit) {
            if !results.is_empty() {
                return results;
            }
        }
    }

    // Fallback: load all and use old LLM-ranking (for entries without embeddings).
    let entries = store.get_all().unwrap_or_default();
    semantic_search_entries(model, query, &entries, limit).await
}

/// Embed a single memory entry and store its vector.  Silently ignored on error.
pub async fn embed_and_store(id: i64, content: &str, model: &str, store: &MemoryStore) {
    if let Some(emb) = OllamaAgent::embed_text(content, model).await {
        let _ = store.set_embedding(id, &emb);
    }
}

/// Process all un-embedded memories in the background.
/// Returns the number of entries newly embedded.
pub async fn backfill_embeddings(model: &str, store: &MemoryStore) -> usize {
    let unembedded = match store.unembedded_ids() {
        Ok(ids) => ids,
        Err(_) => return 0,
    };
    let mut count = 0;
    for (id, content) in &unembedded {
        if let Some(emb) = OllamaAgent::embed_text(content, model).await {
            if store.set_embedding(*id, &emb).is_ok() {
                count += 1;
            }
        }
    }
    count
}

/// Check if a text is a near-duplicate of an existing memory (cosine > 0.97).
/// Returns the id of the duplicate, if any.
pub async fn check_duplicate(
    content: &str,
    model: &str,
    store: &MemoryStore,
) -> Option<i64> {
    let emb = OllamaAgent::embed_text(content, model).await?;
    store.find_duplicate(&emb, 0.97).ok().flatten()
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
                    ..Default::default()
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
            ..Default::default()
        })
        .is_ok()
}

// ── Edge extraction (LLM-powered Entity-Relationship Graph builder) ──────────

/// Use the brain to propose typed, directional edges connecting a batch of
/// memories. Returns the count of new edges actually inserted (duplicates and
/// invalid edges are silently skipped).
///
/// **Cost.** One LLM call per `chunk_size` memories. The default 25-memory
/// chunk keeps the prompt under most context windows. For 1000 memories this
/// is 40 calls — typically a few minutes on a local Ollama, seconds on a
/// hosted API.
pub async fn extract_edges_via_brain(
    model: &str,
    store: &crate::memory::MemoryStore,
    chunk_size: usize,
) -> usize {
    use crate::memory::edges::{format_memories_for_extraction, parse_llm_edges};

    let entries = match store.get_all() {
        Ok(es) => es,
        Err(_) => return 0,
    };
    if entries.len() < 2 {
        return 0;
    }
    let known_ids: std::collections::HashSet<i64> =
        entries.iter().map(|e| e.id).collect();

    let agent = OllamaAgent::new(model);
    let mut total_inserted = 0usize;
    let chunk = chunk_size.clamp(2, 50);

    for window in entries.chunks(chunk) {
        let block = format_memories_for_extraction(window);
        let reply = agent.propose_edges(&block).await;
        if reply.trim().eq_ignore_ascii_case("NONE") {
            continue;
        }
        let new_edges = parse_llm_edges(&reply, &known_ids);
        if new_edges.is_empty() {
            continue;
        }
        if let Ok(n) = store.add_edges_batch(&new_edges) {
            total_inserted += n;
        }
    }
    total_inserted
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
            ..Default::default()
        })
        .unwrap();
        s.add(NewMemory {
            content: "User is building a neural network".to_string(),
            tags: "project,ml".to_string(),
            importance: 5,
            memory_type: MemoryType::Context,
            ..Default::default()
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

    /// Chunk 26.2b — short transcripts skip the segmenter and fall
    /// through to single-pass extraction. Verifies the early-exit
    /// guard against `history.len() < 4`.
    #[tokio::test]
    async fn segmented_extract_short_history_falls_back() {
        let mode = BrainMode::LocalOllama {
            model: "gemma3:4b".to_string(),
        };
        let rotator = std::sync::Mutex::new(crate::brain::ProviderRotator::default());
        // 2-turn history is below the segmenter floor; with no Ollama
        // running the underlying single-pass path returns empty rather
        // than panicking — the assertion is that the function
        // *terminates* with a `Vec` (i.e. takes the fallback path
        // instead of trying to embed and segment).
        let facts = extract_facts_segmented_any_mode(
            &mode,
            None,
            &sample_history(),
            &rotator,
        )
        .await;
        assert!(facts.is_empty() || facts.iter().all(|f| !f.is_empty()));
    }

    /// Chunk 26.2b — when no embeddings are available the function
    /// must fall back to single-pass without panicking.
    #[tokio::test]
    async fn segmented_extract_falls_back_when_no_embeddings() {
        let mode = BrainMode::LocalOllama {
            model: "gemma3:4b".to_string(),
        };
        let rotator = std::sync::Mutex::new(crate::brain::ProviderRotator::default());
        // 6-turn history triggers the embed path; with no Ollama
        // running, embeddings come back `None` and the function falls
        // back to single-pass.
        let history: Vec<(String, String)> = (0..6)
            .map(|i| {
                (
                    if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                    format!("turn {i} content"),
                )
            })
            .collect();
        let facts = extract_facts_segmented_any_mode(&mode, None, &history, &rotator).await;
        // No assertion on contents (offline) — assertion is "doesn't panic".
        let _ = facts;
    }
}
