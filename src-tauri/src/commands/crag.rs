//! CRAG (Corrective RAG) orchestrator commands — Chunk 16.5b.
//!
//! Wires the CRAG evaluator (16.5a) into a Tauri command that:
//! 1. Retrieves memories via hybrid search
//! 2. Evaluates retrieval quality using the LLM-based CRAG classifier
//! 3. On `Ambiguous` → rewrites the query via LLM, retries retrieval
//! 4. On `Incorrect` → falls back to web search (DuckDuckGo HTML scrape)
//! 5. Returns the filtered, quality-assessed memories for injection

use crate::brain::ollama_agent::{ChatMessage, OLLAMA_BASE_URL};
use crate::memory::crag::{
    aggregate, build_evaluator_prompts, build_rewriter_prompts, build_web_search_url,
    parse_rewritten_query, parse_verdict, DocumentVerdict, RetrievalQuality,
};
use crate::memory::MemoryEntry;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Result of CRAG-enhanced retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CragResult {
    /// The final set of memories deemed relevant.
    pub memories: Vec<MemoryEntry>,
    /// The retrieval quality classification.
    pub quality: String,
    /// Whether a query rewrite was performed.
    pub rewrote_query: bool,
    /// Whether web search fallback was used.
    pub used_web_search: bool,
    /// The rewritten query, if any.
    pub rewritten_query: Option<String>,
}

/// Run CRAG-enhanced retrieval: evaluate → rewrite → web-fallback.
///
/// This command evaluates retrieval quality using the LLM, rewrites
/// the query on `Ambiguous`, and falls back to web search on `Incorrect`.
#[tauri::command]
pub async fn crag_retrieve(
    query: String,
    state: State<'_, AppState>,
) -> Result<CragResult, String> {
    run_crag_retrieve(&query, state.inner()).await
}

/// Testable entry point for CRAG retrieval.
pub async fn run_crag_retrieve(query: &str, state: &AppState) -> Result<CragResult, String> {
    if query.trim().is_empty() {
        return Err("Query cannot be empty".to_string());
    }

    // Determine the model to use for evaluation/rewriting
    let model = get_model(state)?;

    let threshold = state
        .app_settings
        .lock()
        .map(|s| s.relevance_threshold)
        .unwrap_or(crate::settings::DEFAULT_RELEVANCE_THRESHOLD);

    // ── Step 1: Initial retrieval ─────────────────────────────────────
    let query_emb = crate::brain::OllamaAgent::embed_text(query, &model).await;
    let initial_memories: Vec<MemoryEntry> = {
        match state.memory_store.lock() {
            Ok(store) => store
                .hybrid_search_with_threshold(query, query_emb.as_deref(), 5, threshold)
                .unwrap_or_default(),
            Err(_) => vec![],
        }
    };

    // If no memories found, skip evaluation — go straight to web fallback
    if initial_memories.is_empty() {
        return try_web_fallback(query, state).await;
    }

    // ── Step 2: Evaluate each memory with CRAG ────────────────────────
    let mut verdicts = Vec::with_capacity(initial_memories.len());
    for mem in &initial_memories {
        let verdict = evaluate_document(&model, query, &mem.content, state).await;
        verdicts.push(verdict);
    }

    let quality = aggregate(&verdicts);

    match quality {
        RetrievalQuality::Correct => {
            // Filter to only Correct/Ambiguous documents
            let filtered = filter_by_verdicts(&initial_memories, &verdicts);
            Ok(CragResult {
                memories: filtered,
                quality: "correct".to_string(),
                rewrote_query: false,
                used_web_search: false,
                rewritten_query: None,
            })
        }
        RetrievalQuality::Ambiguous => {
            // ── Step 3: Rewrite query and retry ───────────────────────
            let rewritten = rewrite_query(&model, query, state).await;

            if let Some(ref new_query) = rewritten {
                let new_emb =
                    crate::brain::OllamaAgent::embed_text(new_query, &model).await;
                let retry_memories: Vec<MemoryEntry> = {
                    match state.memory_store.lock() {
                        Ok(store) => store
                            .hybrid_search_with_threshold(
                                new_query,
                                new_emb.as_deref(),
                                5,
                                threshold,
                            )
                            .unwrap_or_default(),
                        Err(_) => vec![],
                    }
                };

                if !retry_memories.is_empty() {
                    return Ok(CragResult {
                        memories: retry_memories,
                        quality: "ambiguous_rewritten".to_string(),
                        rewrote_query: true,
                        used_web_search: false,
                        rewritten_query: rewritten,
                    });
                }
            }

            // Rewrite didn't help — return the original ambiguous results
            let filtered = filter_by_verdicts(&initial_memories, &verdicts);
            Ok(CragResult {
                memories: filtered,
                quality: "ambiguous".to_string(),
                rewrote_query: rewritten.is_some(),
                used_web_search: false,
                rewritten_query: rewritten,
            })
        }
        RetrievalQuality::Incorrect => {
            // ── Step 4: Web search fallback ───────────────────────────
            try_web_fallback(query, state).await
        }
    }
}

// ─── Internal helpers ────────────────────────────────────────────────

fn get_model(state: &AppState) -> Result<String, String> {
    use crate::brain::brain_config::BrainMode;
    let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?;
    match brain_mode.as_ref() {
        Some(BrainMode::LocalOllama { model }) => Ok(model.clone()),
        _ => state
            .active_brain
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or_else(|| "CRAG requires a configured brain model".to_string()),
    }
}

/// Call the LLM to evaluate a single (query, document) pair.
async fn evaluate_document(
    model: &str,
    query: &str,
    document: &str,
    state: &AppState,
) -> DocumentVerdict {
    let (system, user) = build_evaluator_prompts(query, document);
    let reply = call_llm_simple(model, &system, &user, state).await;
    match reply {
        Some(r) => parse_verdict(&r).unwrap_or(DocumentVerdict::Ambiguous),
        None => DocumentVerdict::Ambiguous, // default to ambiguous on LLM failure
    }
}

/// Call the LLM to rewrite a query.
async fn rewrite_query(model: &str, original: &str, state: &AppState) -> Option<String> {
    let (system, user) = build_rewriter_prompts(original);
    let reply = call_llm_simple(model, &system, &user, state).await?;
    parse_rewritten_query(&reply)
}

/// Simple non-streaming LLM call via Ollama `/api/chat`.
async fn call_llm_simple(
    model: &str,
    system: &str,
    user: &str,
    state: &AppState,
) -> Option<String> {
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user.to_string(),
        },
    ];

    let url = format!("{OLLAMA_BASE_URL}/api/chat");
    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false,
    });

    let resp = state.ollama_client.post(&url).json(&body).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    json["message"]["content"].as_str().map(|s| s.to_string())
}

/// Attempt web search fallback. Gated on `web_search_enabled` setting.
async fn try_web_fallback(query: &str, state: &AppState) -> Result<CragResult, String> {
    // Check if web search is enabled (capability gate)
    let web_enabled = state
        .app_settings
        .lock()
        .map(|s| s.web_search_enabled)
        .unwrap_or(false);

    if !web_enabled {
        return Ok(CragResult {
            memories: vec![],
            quality: "incorrect".to_string(),
            rewrote_query: false,
            used_web_search: false,
            rewritten_query: None,
        });
    }

    // Fetch from DuckDuckGo
    let search_url = build_web_search_url(query);
    let resp = state
        .ollama_client
        .get(&search_url)
        .header("User-Agent", "TerranSoul/0.1 CragWebFallback")
        .send()
        .await
        .map_err(|e| format!("Web search failed: {e}"))?;

    if !resp.status().is_success() {
        return Ok(CragResult {
            memories: vec![],
            quality: "incorrect".to_string(),
            rewrote_query: false,
            used_web_search: true,
            rewritten_query: None,
        });
    }

    let body = resp.text().await.unwrap_or_default();
    let snippets = extract_search_snippets(&body);

    // Convert snippets into synthetic MemoryEntry objects
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let memories: Vec<MemoryEntry> = snippets
        .into_iter()
        .take(3)
        .map(|snippet| MemoryEntry {
            id: 0,
            content: snippet,
            tier: crate::memory::MemoryTier::Short,
            tags: "web-search".to_string(),
            created_at: now_ms,
            importance: 5,
            source_url: Some(search_url.clone()),
            memory_type: crate::memory::MemoryType::Fact,
            last_accessed: None,
            access_count: 0,
            embedding: None,
            decay_score: 1.0,
            session_id: None,
            parent_id: None,
            token_count: 0,
            source_hash: None,
            expires_at: None,
            valid_to: None,
            obsidian_path: None,
            last_exported: None,
        })
        .collect();

    Ok(CragResult {
        memories,
        quality: "incorrect_web_fallback".to_string(),
        rewrote_query: false,
        used_web_search: true,
        rewritten_query: None,
    })
}

/// Extract text snippets from DuckDuckGo HTML results.
fn extract_search_snippets(html: &str) -> Vec<String> {
    use scraper::{Html, Selector};
    let document = Html::parse_document(html);
    let mut snippets = Vec::new();

    // DuckDuckGo HTML results use class "result__snippet"
    if let Ok(selector) = Selector::parse(".result__snippet") {
        for element in document.select(&selector) {
            let text: String = element.text().collect::<Vec<_>>().join(" ");
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() && trimmed.len() > 20 {
                snippets.push(trimmed);
            }
        }
    }

    // Fallback: try ".result__body" or generic result text
    if snippets.is_empty() {
        if let Ok(selector) = Selector::parse(".result__body, .result__a") {
            for element in document.select(&selector) {
                let text: String = element.text().collect::<Vec<_>>().join(" ");
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() && trimmed.len() > 20 {
                    snippets.push(trimmed);
                }
            }
        }
    }

    snippets
}

/// Filter memories to only those with Correct or Ambiguous verdicts.
fn filter_by_verdicts(memories: &[MemoryEntry], verdicts: &[DocumentVerdict]) -> Vec<MemoryEntry> {
    memories
        .iter()
        .zip(verdicts.iter())
        .filter(|(_, v)| matches!(v, DocumentVerdict::Correct | DocumentVerdict::Ambiguous))
        .map(|(m, _)| m.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_by_verdicts_excludes_incorrect() {
        let make_mem = |content: &str| MemoryEntry {
            id: 0,
            content: content.to_string(),
            tags: String::new(),
            importance: 3,
            memory_type: crate::memory::MemoryType::Fact,
            created_at: 0,
            last_accessed: None,
            access_count: 0,
            embedding: None,
            tier: crate::memory::MemoryTier::Long,
            decay_score: 1.0,
            session_id: None,
            parent_id: None,
            token_count: 0,
            source_url: None,
            source_hash: None,
            expires_at: None,
            valid_to: None,
        obsidian_path: None,
        last_exported: None,
        };
        let mems = vec![make_mem("good"), make_mem("bad"), make_mem("meh")];
        let verdicts = vec![
            DocumentVerdict::Correct,
            DocumentVerdict::Incorrect,
            DocumentVerdict::Ambiguous,
        ];
        let result = filter_by_verdicts(&mems, &verdicts);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "good");
        assert_eq!(result[1].content, "meh");
    }

    #[test]
    fn extract_search_snippets_handles_empty_html() {
        let snippets = extract_search_snippets("<html><body></body></html>");
        assert!(snippets.is_empty());
    }

    #[test]
    fn extract_search_snippets_finds_results() {
        let html = r#"<html><body>
            <div class="result__snippet">This is a useful search result about Rust programming language and its ownership model.</div>
            <div class="result__snippet">Another result with good information about memory safety.</div>
        </body></html>"#;
        let snippets = extract_search_snippets(html);
        assert_eq!(snippets.len(), 2);
        assert!(snippets[0].contains("Rust programming"));
    }
}
