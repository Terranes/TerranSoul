use tauri::State;

use crate::memory::{MemoryEntry, MemoryUpdate, NewMemory};
use crate::AppState;

const BYTES_PER_MB: f64 = 1024.0 * 1024.0;
const BYTES_PER_GB: f64 = 1024.0 * BYTES_PER_MB;
const DEFAULT_MAX_MEMORY_BYTES_FALLBACK: u64 =
    (crate::settings::DEFAULT_MAX_MEMORY_GB * BYTES_PER_GB) as u64;
const DEFAULT_MAX_MEMORY_CACHE_BYTES_FALLBACK: u64 =
    (crate::settings::DEFAULT_MAX_MEMORY_MB * BYTES_PER_MB) as u64;

fn configured_memory_limit_bytes(state: &AppState) -> u64 {
    state
        .app_settings
        .lock()
        .map(|s| s.max_memory_bytes())
        .unwrap_or(DEFAULT_MAX_MEMORY_BYTES_FALLBACK)
}

fn configured_memory_cache_limit_bytes(state: &AppState) -> u64 {
    state
        .app_settings
        .lock()
        .map(|s| s.max_memory_cache_bytes())
        .unwrap_or(DEFAULT_MAX_MEMORY_CACHE_BYTES_FALLBACK)
}

fn enforce_configured_memory_limit(
    state: &AppState,
) -> Result<crate::memory::MemoryCleanupReport, String> {
    let max_bytes = configured_memory_limit_bytes(state);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .enforce_size_limit(max_bytes)
        .map_err(|e| e.to_string())
}

/// Read brain_mode + active_brain from state for use by `embed_for_mode`.
/// Returns `(Option<BrainMode>, Option<String>)`.
fn read_embed_context(state: &AppState) -> (Option<crate::brain::BrainMode>, Option<String>) {
    let brain_mode = state.brain_mode.lock().ok().and_then(|g| g.clone());
    let active_brain = state.active_brain.lock().ok().and_then(|g| g.clone());
    (brain_mode, active_brain)
}

/// Embed text using the current brain mode (cloud or local).
async fn embed(state: &AppState, text: &str) -> Option<Vec<f32>> {
    let (brain_mode, active_brain) = read_embed_context(state);
    crate::brain::embed_for_mode(text, brain_mode.as_ref(), active_brain.as_deref()).await
}

/// Add a new long-term memory.
/// Automatically generates a vector embedding when a brain is configured.
/// When `AppSettings.auto_tag` is enabled and a brain is active, runs an
/// LLM pass to classify the content into curated-prefix tags and merges
/// them with the user-supplied tags (chunk 18.1).
#[tauri::command]
pub async fn add_memory(
    content: String,
    tags: String,
    importance: i64,
    memory_type: String,
    state: State<'_, AppState>,
) -> Result<MemoryEntry, String> {
    add_memory_inner(content, tags, importance, memory_type, &state).await
}

async fn add_memory_inner(
    mut content: String,
    mut tags: String,
    mut importance: i64,
    mut memory_type: String,
    state: &AppState,
) -> Result<MemoryEntry, String> {
    state.plugin_host.activate_for_memory_tags(&tags).await;
    let pre_hooks = state
        .plugin_host
        .run_memory_hooks(
            crate::plugins::MemoryHookStage::PreStore,
            crate::plugins::MemoryHookPayload {
                stage: crate::plugins::MemoryHookStage::PreStore,
                content,
                tags,
                importance,
                memory_type,
                entry_id: None,
            },
        )
        .await;
    for error in &pre_hooks.errors {
        eprintln!("[plugins] PreStore memory hook failed: {error}");
    }
    content = pre_hooks.payload.content;
    tags = pre_hooks.payload.tags;
    importance = pre_hooks.payload.importance;
    memory_type = pre_hooks.payload.memory_type;

    let mt = serde_json::from_value(serde_json::Value::String(memory_type))
        .unwrap_or(crate::memory::MemoryType::Fact);

    let mut entry = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store
            .add(NewMemory {
                content: content.clone(),
                tags: tags.clone(),
                importance,
                memory_type: mt,
                ..Default::default()
            })
            .map_err(|e| e.to_string())?
    }; // lock released before await

    // Best-effort embedding — non-blocking, silently ignored on failure.
    // Chunk 16.9: uses cloud embedding API when brain mode is FreeApi/PaidApi.
    if let Some(emb) = embed(state, &content).await {
        // Find near-duplicate while holding the lock, then release.
        let dup_info: Option<(i64, String)> = {
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            let _ = store.set_embedding(entry.id, &emb);

            // Contradiction detection (Chunk 17.2): check for near-duplicate.
            store
                .find_duplicate(&emb, 0.85)
                .ok()
                .flatten()
                .filter(|&dup_id| dup_id != entry.id)
                .and_then(|dup_id| {
                    store
                        .get_by_id(dup_id)
                        .ok()
                        .map(|dup| (dup_id, dup.content))
                })
        }; // lock released here

        // If a near-duplicate exists, ask the LLM whether the two
        // statements contradict. If so, open a MemoryConflict row.
        if let Some((dup_id, dup_content)) = dup_info {
            let model_opt = state
                .active_brain
                .lock()
                .map_err(|e| e.to_string())?
                .clone();
            if let Some(ref model) = model_opt {
                let agent = crate::brain::OllamaAgent::new(model);
                if let Some(result) = agent.check_contradiction(&dup_content, &content).await {
                    if result.contradicts {
                        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
                        let _ = store.add_conflict(dup_id, entry.id, &result.reason);
                    }
                }
            }
        }
    }

    // Auto-tag via LLM when the setting is enabled and a brain is configured.
    let auto_tag_enabled = state
        .app_settings
        .lock()
        .map(|s| s.auto_tag)
        .unwrap_or(false);
    if auto_tag_enabled {
        let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?.clone();
        if let Some(mode) = brain_mode {
            let auto_tags = crate::memory::auto_tag::auto_tag_content(&content, &mode).await;
            if !auto_tags.is_empty() {
                let merged = crate::memory::auto_tag::merge_tags(&tags, &auto_tags);
                let store = state.memory_store.lock().map_err(|e| e.to_string())?;
                if let Ok(updated) = store.update(
                    entry.id,
                    MemoryUpdate {
                        tags: Some(merged),
                        content: None,
                        importance: None,
                        memory_type: None,
                    },
                ) {
                    entry = updated;
                }
            }
        }
    }

    state
        .plugin_host
        .activate_for_memory_tags(&entry.tags)
        .await;
    let post_hooks = state
        .plugin_host
        .run_memory_hooks(
            crate::plugins::MemoryHookStage::PostStore,
            crate::plugins::MemoryHookPayload {
                stage: crate::plugins::MemoryHookStage::PostStore,
                content: entry.content.clone(),
                tags: entry.tags.clone(),
                importance: entry.importance,
                memory_type: entry.memory_type.as_str().to_string(),
                entry_id: Some(entry.id),
            },
        )
        .await;
    for error in &post_hooks.errors {
        eprintln!("[plugins] PostStore memory hook failed: {error}");
    }

    let _ = enforce_configured_memory_limit(state);
    Ok(entry)
}

/// Return all stored memories.
#[tauri::command]
pub async fn get_memories(state: State<'_, AppState>) -> Result<Vec<MemoryEntry>, String> {
    let max_bytes = configured_memory_limit_bytes(&state);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .get_all_within_storage_bytes(max_bytes)
        .map_err(|e| e.to_string())
}

/// Search memories by keyword.
#[tauri::command]
pub async fn search_memories(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEntry>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.search(&query).map_err(|e| e.to_string())
}

/// Update fields on an existing memory.
#[tauri::command]
pub async fn update_memory(
    id: i64,
    content: Option<String>,
    tags: Option<String>,
    importance: Option<i64>,
    memory_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<MemoryEntry, String> {
    let mt = memory_type.and_then(|s| serde_json::from_value(serde_json::Value::String(s)).ok());
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .update(
            id,
            MemoryUpdate {
                content,
                tags,
                importance,
                memory_type: mt,
            },
        )
        .map_err(|e| e.to_string())
}

/// Delete a memory by id.
#[tauri::command]
pub async fn delete_memory(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.delete(id).map_err(|e| e.to_string())
}

/// Return the N most relevant long-term memories for a given message.
/// Used internally; also exposed for debugging.
#[tauri::command]
pub async fn get_relevant_memories(
    message: String,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    Ok(store.relevant_for(&message, limit))
}

/// Return the current short-term memory window (last N conversation messages).
#[tauri::command]
pub async fn get_short_term_memory(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::commands::chat::Message>, String> {
    let window = limit.unwrap_or(20);
    let conv = state.conversation.lock().map_err(|e| e.to_string())?;
    let messages: Vec<_> = conv.iter().rev().take(window).rev().cloned().collect();
    Ok(messages)
}

// ── Brain-powered memory commands ────────────────────────────────────────────

/// Use the active brain to extract memorable facts from the current session
/// and store them automatically.  Returns the number of new memories saved.
#[tauri::command]
pub async fn extract_memories_from_session(state: State<'_, AppState>) -> Result<usize, String> {
    // Read brain_mode first — works for Free/Paid/Local.
    let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?.clone();

    let history: Vec<(String, String)> = {
        let conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    }; // lock released before await

    let facts = if let Some(mode) = brain_mode {
        // Chunk 26.2b — route through the topic-segmenter so a chat
        // that wandered across topics produces a focused per-topic
        // fact list instead of one jumbled blob. Falls back to
        // single-pass extraction when embeddings are unavailable or
        // no topic shift is detected.
        let active_model = state
            .active_brain
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        crate::memory::brain_memory::extract_facts_segmented_any_mode(
            &mode,
            active_model.as_deref(),
            &history,
            &state.provider_rotator,
        )
        .await
    } else {
        // Legacy path: check active_brain for Ollama model name.
        let model = state
            .active_brain
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;
        crate::memory::brain_memory::extract_facts(&model, &history).await
    };

    let count = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        crate::memory::brain_memory::save_facts(&facts, &store)
    };

    // Chunk 26.3 — auto-fire edge extraction so newly-learned facts
    // immediately participate in the typed-edge knowledge graph instead of
    // waiting for the user to manually trigger `extract_edges_via_brain`.
    // Gated by `auto_extract_edges` (default on); skipped silently when
    // disabled, when no facts were saved this round, or when no Ollama
    // model is configured (edge extraction currently requires `active_brain`,
    // see `extract_edges_via_brain` below). A failure in this follow-up
    // never surfaces to the caller — the primary fact-save already
    // succeeded and the next learn cycle will retry edge extraction.
    if count > 0 {
        let auto_edges = state
            .app_settings
            .lock()
            .map(|s| s.auto_extract_edges)
            .unwrap_or(true);
        let active_model = state
            .active_brain
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        if auto_edges {
            if let Some(model) = active_model {
                let _ = run_edge_extraction(&model, &state, 25).await;
            }
        }
    }

    let _ = enforce_configured_memory_limit(&state);
    Ok(count)
}

/// Inner helper used by both [`extract_edges_via_brain`] (manual command)
/// and the auto-fire path inside [`extract_memories_from_session`]
/// (Chunk 26.3). Snapshots all memories under a short lock, runs the LLM
/// edge proposer in chunked batches without holding the lock, and re-locks
/// briefly to insert each batch. Returns the count of new edges inserted.
async fn run_edge_extraction(
    model: &str,
    state: &State<'_, AppState>,
    chunk: usize,
) -> Result<usize, String> {
    let entries: Vec<MemoryEntry> = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store.get_all().map_err(|e| e.to_string())?
    };
    let chunk = chunk.clamp(2, 50);
    if entries.len() < 2 {
        return Ok(0);
    }
    let known_ids: std::collections::HashSet<i64> = entries.iter().map(|e| e.id).collect();
    let agent = crate::brain::OllamaAgent::new(model);
    let mut total_inserted = 0usize;

    for window in entries.chunks(chunk) {
        let block = crate::memory::format_memories_for_extraction(window);
        let reply = agent.propose_edges(&block).await;
        if reply.trim().eq_ignore_ascii_case("NONE") {
            continue;
        }
        let new_edges = crate::memory::parse_llm_edges(&reply, &known_ids);
        if new_edges.is_empty() {
            continue;
        }
        // Collect endpoints for cache invalidation (Chunk 41.13).
        let endpoints: Vec<i64> = new_edges
            .iter()
            .flat_map(|e| [e.src_id, e.dst_id])
            .collect();
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        if let Ok(n) = store.add_edges_batch(&new_edges) {
            total_inserted += n;
        }
        drop(store);
        state.kg_cache.invalidate(&endpoints);
    }
    Ok(total_inserted)
}

/// Use the active brain to summarize the current session into a single memory entry.
#[tauri::command]
pub async fn summarize_session(state: State<'_, AppState>) -> Result<String, String> {
    let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?.clone();

    let history: Vec<(String, String)> = {
        let conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    }; // lock released before await

    let summary = if let Some(mode) = brain_mode {
        crate::memory::brain_memory::summarize_any_mode(&mode, &history, &state.provider_rotator)
            .await
    } else {
        let model = state
            .active_brain
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;
        crate::memory::brain_memory::summarize(&model, &history).await
    };

    let summary = summary.ok_or_else(|| "Session is empty or brain is unreachable.".to_string())?;

    {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        crate::memory::brain_memory::save_summary(&summary, &store);
    }
    let _ = enforce_configured_memory_limit(&state);
    Ok(summary)
}

/// Reflect on the current chat session, extracting facts and saving a
/// provenance-linked summary memory.
#[tauri::command]
pub async fn reflect_on_session(
    state: State<'_, AppState>,
) -> Result<crate::memory::reflection::SessionReflectionReport, String> {
    let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?.clone();

    let history: Vec<(String, String)> = {
        let conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    };
    if history.is_empty() {
        return Err("Session is empty.".to_string());
    }

    let facts = if let Some(mode) = brain_mode.clone() {
        let active_model = state
            .active_brain
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        crate::memory::brain_memory::extract_facts_segmented_any_mode(
            &mode,
            active_model.as_deref(),
            &history,
            &state.provider_rotator,
        )
        .await
    } else {
        let model = state
            .active_brain
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;
        crate::memory::brain_memory::extract_facts(&model, &history).await
    };

    let summary = if let Some(mode) = brain_mode {
        crate::memory::brain_memory::summarize_any_mode(&mode, &history, &state.provider_rotator)
            .await
    } else {
        let model = state
            .active_brain
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;
        crate::memory::brain_memory::summarize(&model, &history).await
    }
    .ok_or_else(|| "Session is empty or brain is unreachable.".to_string())?;

    let report = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        crate::memory::reflection::persist_session_reflection(&store, &history, &facts, &summary)?
    };

    if let Some(embedding) = embed(&state, &report.summary).await {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        let _ = store.set_embedding(report.reflection_id, &embedding);
    }

    let _ = enforce_configured_memory_limit(&state);
    Ok(report)
}

/// Walk historical session summaries and re-run fact extraction on each.
///
/// **Chunk 26.4** — Earlier versions of TerranSoul saved
/// `MemoryType::Summary` paragraphs but did not extract structured facts
/// from them. This command backfills those facts using the same
/// segmented extractor that auto-fires after every chat
/// (Chunk 26.2b). Emits `brain-replay-progress` events with a
/// [`crate::memory::replay::ReplayProgress`] payload so the UI can show
/// a progress bar.
///
/// `since_timestamp_ms`: only replay summaries created at or after this
/// Unix-millisecond timestamp. `None` means replay all summaries.
///
/// `dry_run`: when `true`, the LLM is invoked but the resulting facts
/// are **not** persisted. Useful for "preview how many memories this
/// would create" before committing.
///
/// `max_summaries`: hard cap on the number of summaries replayed. `None`
/// = no cap.
#[tauri::command]
pub async fn replay_extract_history(
    since_timestamp_ms: Option<i64>,
    dry_run: Option<bool>,
    max_summaries: Option<usize>,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<crate::memory::replay::ReplayProgress, String> {
    use tauri::Emitter;

    let config = crate::memory::replay::ReplayConfig {
        since_timestamp_ms,
        dry_run: dry_run.unwrap_or(false),
        max_summaries,
    };

    let brain_mode = state
        .brain_mode
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;

    let active_model = state
        .active_brain
        .lock()
        .ok()
        .and_then(|guard| guard.clone());

    // Snapshot all entries under a short lock, then filter without it.
    let all = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store.get_all().map_err(|e| e.to_string())?
    };
    let summaries = crate::memory::replay::select_summaries(&all, &config);
    let total = summaries.len();

    let mut progress = crate::memory::replay::ReplayProgress {
        processed: 0,
        total,
        new_memories: 0,
        current_summary_created_at: None,
        current_summary_id: None,
        done: total == 0,
    };

    // Always emit at least one event so the UI sees the total even when
    // there are no summaries to process.
    let _ = app_handle.emit("brain-replay-progress", &progress);

    for summary in &summaries {
        let history = crate::memory::replay::synthetic_history_from_summary(&summary.content);
        let facts = crate::memory::brain_memory::extract_facts_segmented_any_mode(
            &brain_mode,
            active_model.as_deref(),
            &history,
            &state.provider_rotator,
        )
        .await;

        let saved = if config.dry_run {
            facts.len()
        } else {
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            crate::memory::brain_memory::save_facts(&facts, &store)
        };

        progress = crate::memory::replay::next_progress(&progress, summary, saved, total);
        let _ = app_handle.emit("brain-replay-progress", &progress);
    }

    let _ = enforce_configured_memory_limit(&state);
    Ok(progress)
}

/// Use the active brain to perform a semantic search across stored memories.
/// Uses fast vector search when embeddings exist, falls back to LLM ranking,
/// then to keyword search if no brain is available.
#[tauri::command]
pub async fn semantic_search_memories(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::memory::MemoryEntry>, String> {
    let limit = limit.unwrap_or(10);

    // Fast path: embed query → cosine search (no LLM call needed).
    // Chunk 16.9: uses cloud embedding when brain mode is FreeApi/PaidApi.
    if let Some(query_emb) = embed(&state, &query).await {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        let results = store
            .vector_search(&query_emb, limit)
            .map_err(|e| e.to_string())?;
        if !results.is_empty() {
            return Ok(results);
        }
    }

    // Fallback: brute-force LLM ranking (requires active_brain for Ollama).
    let model_opt = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    if let Some(model) = model_opt {
        let entries: Vec<crate::memory::MemoryEntry> = {
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            store.get_all().map_err(|e| e.to_string())?
        };
        let results =
            crate::memory::brain_memory::semantic_search_entries(&model, &query, &entries, limit)
                .await;
        Ok(results)
    } else {
        // No brain — keyword fallback.
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        let entries = store.get_all().map_err(|e| e.to_string())?;
        Ok(entries
            .into_iter()
            .filter(|e| {
                let q = query.to_lowercase();
                e.content.to_lowercase().contains(&q) || e.tags.to_lowercase().contains(&q)
            })
            .take(limit)
            .collect())
    }
}

/// Generate embeddings for all memories that don't have one yet.
/// Returns the number of entries newly embedded.
#[tauri::command]
pub async fn backfill_embeddings(state: State<'_, AppState>) -> Result<usize, String> {
    // Verify at least one embed source is available.
    let (brain_mode, active_brain) = read_embed_context(&state);
    if brain_mode.is_none() && active_brain.is_none() {
        return Err("No brain configured. Set up a brain first.".to_string());
    }

    let unembedded: Vec<(i64, String)> = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store.unembedded_ids().map_err(|e| e.to_string())?
    };

    let mut count = 0usize;
    for (id, content) in &unembedded {
        if let Some(emb) = embed(&state, content).await {
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            if store.set_embedding(*id, &emb).is_ok() {
                count += 1;
            }
        }
    }
    Ok(count)
}

/// Backfill `embedding_model_id` on existing memories that have embeddings
/// but no model metadata. Called after switching to V16 schema (Chunk 41.8).
/// Optionally copies embeddings into the `memory_embeddings` side table.
#[tauri::command]
pub async fn backfill_embedding_model_id(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<usize, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .backfill_embedding_model(&model_id)
        .map_err(|e| e.to_string())
}

/// Rebuild the ANN index with a new quantization mode (Chunk 41.9).
/// Valid modes: "f32" (default), "i8" (4× compression), "b1" (binary).
/// Returns the number of vectors re-indexed.
#[tauri::command]
pub async fn set_ann_quantization(
    state: State<'_, AppState>,
    mode: String,
) -> Result<usize, String> {
    let quant = crate::memory::ann_index::EmbeddingQuantization::from_str_lossy(&mode);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.rebuild_ann_quantized(quant)
}

/// Compact the ANN index by rebuilding from live long-term memories (Chunk 41.11).
///
/// Removes tombstones from the HNSW graph. Only runs if fragmentation
/// exceeds the threshold (20%). Returns the vector count after compaction,
/// or 0 if compaction was not needed or no index exists.
#[tauri::command]
pub async fn compact_ann(
    state: State<'_, AppState>,
    force: Option<bool>,
) -> Result<usize, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    if !force.unwrap_or(false) && !store.ann_needs_compaction() {
        return Ok(0);
    }
    store.compact_ann()
}

/// Return the current database schema version and storage status.
#[tauri::command]
pub async fn get_schema_info(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let version = store.schema_version();
    let count = store.count();
    let unembedded = store.unembedded_ids().map_err(|e| e.to_string())?.len();

    Ok(serde_json::json!({
        "schema_version": version,
        "target_version": crate::memory::schema::CANONICAL_SCHEMA_VERSION,
        "total_memories": count,
        "unembedded_count": unembedded,
        "embedded_count": count as usize - unembedded,
        "db_engine": "SQLite (WAL mode)",
        "columns": {
            "id": "INTEGER PRIMARY KEY AUTOINCREMENT",
            "content": "TEXT NOT NULL — the memory text",
            "tags": "TEXT — comma-separated tags",
            "importance": "INTEGER 1-5 — priority ranking",
            "memory_type": "TEXT — fact|preference|context|summary",
            "created_at": "INTEGER — Unix timestamp (ms)",
            "last_accessed": "INTEGER — last RAG hit timestamp",
            "access_count": "INTEGER — times retrieved by RAG",
            "embedding": "BLOB — 768-dim f32 vector (little-endian)",
            "source_url": "TEXT — origin URL for ingested documents",
            "source_hash": "TEXT — content hash for dedup/staleness",
            "expires_at": "INTEGER — TTL for auto-expiry",
            "tier": "TEXT — short|working|long",
            "decay_score": "REAL — 0.0-1.0 freshness weight",
            "session_id": "TEXT — session grouping",
            "parent_id": "INTEGER — parent memory (for summaries)",
            "token_count": "INTEGER — estimated token count"
        }
    }))
}

// ── Tiered memory commands ───────────────────────────────────────────────────

/// Hybrid search combining vector similarity, keywords, recency, importance, and decay.
/// The primary RAG retrieval method — scales to 1M+ entries.
#[tauri::command]
pub async fn hybrid_search_memories(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEntry>, String> {
    let limit = limit.unwrap_or(10);

    // Chunk 16.9: use cloud embedding when brain mode is FreeApi/PaidApi.
    let query_emb = embed(&state, &query).await;

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .hybrid_search(&query, query_emb.as_deref(), limit)
        .map_err(|e| e.to_string())
}

/// RRF-fused hybrid search — combines independent vector / keyword / freshness
/// retrievers via Reciprocal Rank Fusion (k = 60). Robust to score-scale
/// mismatch between retrievers; recommended over [`hybrid_search_memories`]
/// when retrievers may disagree on absolute score magnitudes.
///
/// Implements §16 Phase 6 / §19.2 row 2 of `docs/brain-advanced-design.md`.
#[tauri::command]
pub async fn hybrid_search_memories_rrf(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEntry>, String> {
    let limit = limit.unwrap_or(10);

    let query_emb = embed(&state, &query).await;

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .hybrid_search_rrf(&query, query_emb.as_deref(), limit)
        .map_err(|e| e.to_string())
}

/// **HyDE** — Hypothetical Document Embeddings retrieval (Gao et al., 2022).
///
/// Asks the active brain to write a plausible 1-3 sentence answer to
/// `query`, embeds *that* hypothetical answer, then runs RRF-fused
/// hybrid search using the hypothetical embedding instead of the raw
/// query embedding. The hypothetical lives in the same distribution as
/// real documents, so cosine similarity becomes much sharper on cold,
/// abstract or one-word queries.
///
/// Graceful fallback chain:
/// 1. No brain configured → falls back to keyword + freshness ranking
///    via [`MemoryStore::hybrid_search_rrf`] with no embedding.
/// 2. Brain configured but `hyde_complete` returns `None` (network
///    failure, empty reply) → falls back to embedding the raw query.
/// 3. Embedding step returns `None` → falls back to keyword + freshness.
///
/// Returns the same `Vec<MemoryEntry>` shape as the other search
/// commands so it is a drop-in replacement for callers that want
/// HyDE-quality retrieval without changing their result-handling code.
///
/// Implements §16 Phase 6 / §19.2 row 4 of `docs/brain-advanced-design.md`.
#[tauri::command]
pub async fn hyde_search_memories(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEntry>, String> {
    let limit = limit.unwrap_or(10);

    let model_opt = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    // Step 1+2: HyDE expansion → embed the hypothetical (or fall back to
    // embedding the raw query if expansion fails).
    let query_emb = if let Some(model) = model_opt {
        let agent = crate::brain::OllamaAgent::new(&model);
        let hypothetical = agent.hyde_complete(&query).await;
        let text_to_embed = hypothetical.as_deref().unwrap_or(query.as_str());
        embed(&state, text_to_embed).await
    } else {
        embed(&state, &query).await
    };

    // Step 3: RRF-fused retrieval with the (possibly hypothetical) embedding.
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .hybrid_search_rrf(&query, query_emb.as_deref(), limit)
        .map_err(|e| e.to_string())
}

/// **Matryoshka two-stage vector search** (Chunk 16.8).
///
/// Brute-force vector search where stage 1 scores every candidate
/// against a *truncated* query embedding (default 256-dim), then
/// stage 2 re-ranks the top `fast_top_k` survivors with the full-dim
/// embedding. Returns the top `limit` entries by full-dim cosine
/// similarity.
///
/// Why bother when we already have an ANN index? The ANN index is
/// optional (rebuilds lazily, may be unavailable on first start, may
/// have dimension mismatch after model swap). When the brute-force
/// fallback path runs — which it does for every user on a cold first
/// query — Matryoshka cuts the per-candidate cost roughly 3× with a
/// negligible recall hit.
///
/// Arguments:
/// - `query` — natural-language search text. Embedded via the same
///   path as the other search commands (cloud, Ollama, or no-op).
/// - `limit` — final result count (default 10).
/// - `fast_dim` — truncation dimension for stage 1 (default 256).
///   Set to 0 to use the module default.
/// - `fast_top_k` — survivors carried into the full-dim re-ranker
///   (default `limit * 5`, capped at the candidate-set size).
///
/// Returns the same `Vec<MemoryEntry>` shape as the other search
/// commands so it is a drop-in replacement.
///
/// Implements §16 Phase 6 / §19.2 row 11 of `docs/brain-advanced-design.md`.
#[tauri::command]
pub async fn matryoshka_search_memories(
    query: String,
    limit: Option<usize>,
    fast_dim: Option<usize>,
    fast_top_k: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEntry>, String> {
    let limit = limit.unwrap_or(10).max(1);
    let fast_dim = match fast_dim {
        Some(0) | None => crate::memory::matryoshka::DEFAULT_FAST_DIM,
        Some(n) => n,
    };
    let fast_top_k = fast_top_k.unwrap_or(limit.saturating_mul(5)).max(limit);

    let Some(query_emb) = embed(&state, &query).await else {
        // No embedding available (no brain configured, or daemon
        // unreachable). Return empty results — the caller can fall
        // back to keyword search.
        return Ok(vec![]);
    };

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let all_with_emb = store.get_with_embeddings().map_err(|e| e.to_string())?;
    let candidates: Vec<(i64, Vec<f32>)> = all_with_emb
        .iter()
        .filter_map(|e| Some((e.id, e.embedding.clone()?)))
        .collect();

    let scored = crate::memory::matryoshka::two_stage_search(
        &query_emb,
        &candidates,
        fast_dim,
        fast_top_k,
        limit,
    );

    // Map scored ids back to full entries, preserving order.
    let mut results = Vec::with_capacity(scored.len());
    for s in scored {
        if let Some(entry) = all_with_emb.iter().find(|e| e.id == s.id) {
            results.push(entry.clone());
        }
    }
    Ok(results)
}

/// **Cross-encoder rerank** over RRF-fused hybrid search (LLM-as-judge style).
///
/// Two-stage retrieval pipeline:
///
/// 1. **Recall stage** — `hybrid_search_rrf` returns the top
///    `candidates_k` candidates (default 20, bounded `limit..=50`).
/// 2. **Precision stage** — the active brain scores each candidate on
///    a 0–10 relevance scale via [`OllamaAgent::rerank_score`], and
///    [`crate::memory::reranker::rerank_candidates`] re-orders them.
///
/// Unscored candidates (LLM call failed or unparseable reply) are
/// kept in the result but ranked below every successfully-scored
/// candidate, so a flaky reranker never silently loses recall.
///
/// When **no brain is configured**, the rerank stage is skipped and
/// the command behaves exactly like `hybrid_search_memories_rrf` —
/// callers can adopt this command unconditionally without breaking
/// the cold-start (no-LLM) experience.
///
/// Implements § 16 Phase 6 / § 19.2 row 10 of `docs/brain-advanced-design.md`.
#[tauri::command]
pub async fn rerank_search_memories(
    query: String,
    limit: Option<usize>,
    candidates_k: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEntry>, String> {
    let limit = limit.unwrap_or(10).max(1);
    // Bound the recall stage so the LLM round-trip count stays sane.
    // We always pull at least `limit` so rerank can't shrink the result.
    let candidates_k = candidates_k.unwrap_or(20).clamp(limit, 50);

    // Stage 1 — RRF-fused recall.
    let model_opt = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let query_emb = embed(&state, &query).await;

    let candidates: Vec<MemoryEntry> = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store
            .hybrid_search_rrf(&query, query_emb.as_deref(), candidates_k)
            .map_err(|e| e.to_string())?
    }; // release the store lock before any LLM await

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    // Stage 2 — LLM-as-judge rerank. If no brain is configured, return
    // the recall list truncated to `limit` so the command is still useful.
    let Some(model) = model_opt else {
        return Ok(candidates.into_iter().take(limit).collect());
    };

    let agent = crate::brain::OllamaAgent::new(&model);
    // Score sequentially to stay under provider rate limits.
    let mut scores: Vec<Option<u8>> = Vec::with_capacity(candidates.len());
    for cand in &candidates {
        scores.push(agent.rerank_score(&query, &cand.content).await);
    }

    Ok(crate::memory::reranker::rerank_candidates(
        candidates, &scores, limit,
    ))
}

/// Get memory statistics per tier.
#[tauri::command]
pub async fn get_memory_stats(
    state: State<'_, AppState>,
) -> Result<crate::memory::MemoryStats, String> {
    let cache_limit_bytes = configured_memory_cache_limit_bytes(&state);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let mut stats = store.stats().map_err(|e| e.to_string())?;
    stats.cache_bytes = store
        .active_cache_bytes(cache_limit_bytes)
        .map_err(|e| e.to_string())?;
    Ok(stats)
}

/// Get per-operation latency metrics (p50/p95/p99) for all memory CRUD and
/// retrieval operations (Phase 41.2). Returns a JSON object with one key per
/// operation name.
#[tauri::command]
pub async fn get_memory_metrics() -> Result<serde_json::Value, String> {
    let snap = crate::memory::metrics::METRICS.snapshot();
    serde_json::to_value(snap).map_err(|e| e.to_string())
}

/// Return search cache statistics (hit rate, entries, generation).
#[tauri::command]
pub async fn get_search_cache_stats() -> Result<crate::memory::search_cache::CacheStats, String> {
    Ok(crate::memory::search_cache::SEARCH_CACHE.stats())
}

/// Enforce the configured maximum memory/RAG storage cap immediately.
#[tauri::command]
pub async fn enforce_memory_storage_limit(
    state: State<'_, AppState>,
) -> Result<crate::memory::MemoryCleanupReport, String> {
    enforce_configured_memory_limit(&state)
}

/// Apply time-based decay to long-term memories. Returns count of updated entries.
#[tauri::command]
pub async fn apply_memory_decay(state: State<'_, AppState>) -> Result<usize, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.apply_decay().map_err(|e| e.to_string())
}

/// Auto-promote frequently accessed working-tier entries to long-tier.
///
/// Pure access-pattern heuristic — no LLM. An entry is promoted when both
/// `access_count >= min_access_count` (default 5) and `last_accessed` is
/// within the last `window_days` days (default 7). Returns the IDs that
/// were promoted (in ascending order). Idempotent.
///
/// Maps to `docs/brain-advanced-design.md` § 16 Phase 5
/// "Auto-promotion based on access patterns" (chunk 17.1).
#[tauri::command]
pub async fn auto_promote_memories(
    min_access_count: Option<i64>,
    window_days: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<i64>, String> {
    let min = min_access_count.unwrap_or(5);
    let win = window_days.unwrap_or(7);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .auto_promote_to_long(min, win)
        .map_err(|e| e.to_string())
}

/// Auto-adjust memory importance based on access patterns (Chunk 17.4).
///
/// * Hot entries (`access_count >= hot_threshold`, default 10) get +1
///   importance (capped at 5). Their access_count is then reset to 0.
/// * Cold entries (`access_count == 0` for `cold_days`, default 30) get
///   −1 importance (floored at 1).
///
/// Each adjustment is audited via `memory_versions` (V8 schema).
/// Returns `{ boosted, demoted }`.
///
/// Maps to `docs/brain-advanced-design.md` §16 Phase 5 (chunk 17.4).
#[tauri::command]
pub async fn adjust_memory_importance(
    hot_threshold: Option<i64>,
    cold_days: Option<i64>,
    state: State<'_, AppState>,
) -> Result<ImportanceAdjustResult, String> {
    let hot = hot_threshold.unwrap_or(10);
    let cold = cold_days.unwrap_or(30);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let (boosted, demoted) = store
        .adjust_importance_by_access(hot, cold)
        .map_err(|e| e.to_string())?;
    Ok(ImportanceAdjustResult { boosted, demoted })
}

/// Result of an importance auto-adjustment run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportanceAdjustResult {
    pub boosted: usize,
    pub demoted: usize,
}

/// Per-memory tag-validation report (chunk 18.4).
///
/// Returned by [`audit_memory_tags`] for BrainView's "review tags" panel.
#[derive(serde::Serialize, Debug, Clone)]
pub struct MemoryTagAudit {
    pub memory_id: i64,
    /// `tag` text exactly as stored, paired with the human-readable
    /// reason it was flagged. Acceptable tags are not included.
    pub flagged: Vec<TagAuditFlag>,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct TagAuditFlag {
    pub tag: String,
    pub reason: String,
}

// ── Contradiction resolution (Chunk 17.2) ───────────────────────────────────

/// List all memory conflicts, optionally filtered by status.
/// Defaults to listing only "open" conflicts when no filter is provided.
#[tauri::command]
pub async fn list_memory_conflicts(
    status: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::memory::conflicts::MemoryConflict>, String> {
    let filter = status.map(|s| crate::memory::conflicts::ConflictStatus::parse(&s));
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .list_conflicts(filter.as_ref())
        .map_err(|e| e.to_string())
}

/// Resolve a memory conflict by picking a winner. The loser is soft-closed
/// via `valid_to` — never deleted.
#[tauri::command]
pub async fn resolve_memory_conflict(
    conflict_id: i64,
    winner_id: i64,
    state: State<'_, AppState>,
) -> Result<crate::memory::conflicts::MemoryConflict, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .resolve_conflict(conflict_id, winner_id)
        .map_err(|e| e.to_string())
}

/// Dismiss a memory conflict (user says "not a real conflict").
#[tauri::command]
pub async fn dismiss_memory_conflict(
    conflict_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .dismiss_conflict(conflict_id)
        .map_err(|e| e.to_string())
}

/// Return the number of open (unresolved) memory conflicts.
#[tauri::command]
pub async fn count_memory_conflicts(state: State<'_, AppState>) -> Result<i64, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.count_open_conflicts().map_err(|e| e.to_string())
}

/// Scan connected memories for contradictions (Chunk 17.6).
///
/// Iterates edges with positive relation types (supports, implies,
/// related_to, etc.) and asks the LLM whether the connected pair
/// actually contradicts. When found, inserts a "contradicts" edge
/// and opens a MemoryConflict row.
///
/// `max_pairs` caps the scan to avoid runaway LLM calls (default 50).
/// Intended to be triggered on user idle from the frontend.
#[tauri::command]
pub async fn scan_edge_conflicts(
    max_pairs: Option<usize>,
    state: State<'_, AppState>,
) -> Result<crate::memory::edge_conflict_scan::EdgeConflictScanResult, String> {
    let model = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No brain configured — edge conflict scan requires an LLM.".to_string())?;

    let max_pairs = max_pairs.unwrap_or(50).min(200);

    // Phase 1: collect candidate pairs while holding the lock.
    let candidates = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        crate::memory::edge_conflict_scan::collect_scan_candidates(&store, max_pairs)
    };

    // Phase 2: run LLM checks without holding the lock.
    let agent = crate::brain::OllamaAgent::new(&model);
    let mut result = crate::memory::edge_conflict_scan::EdgeConflictScanResult {
        pairs_scanned: 0,
        conflicts_found: 0,
        pairs_skipped: candidates.skipped,
    };

    for (src_id, dst_id, content_a, content_b) in &candidates.pairs {
        result.pairs_scanned += 1;
        if let Some(cr) = agent.check_contradiction(content_a, content_b).await {
            if cr.contradicts {
                // Phase 3: write results, re-acquiring the lock per write.
                let store = state.memory_store.lock().map_err(|e| e.to_string())?;
                crate::memory::edge_conflict_scan::record_contradiction(
                    &store, *src_id, *dst_id, &cr.reason,
                );
                result.conflicts_found += 1;
            }
        }
    }

    Ok(result)
}

/// Walk every memory, validate its `tags` against the curated vocabulary,
/// and return only the rows that have at least one non-conforming tag.
///
/// This is the read-only surface BrainView calls to render its
/// "review tags" warning. The write path is **not** affected — ingest
/// always accepts tags, this is purely an audit lens.
///
/// Maps to `docs/brain-advanced-design.md` § 16 Phase 2 row "Tag-prefix
/// convention enforcement" (chunk 18.4).
#[tauri::command]
pub async fn audit_memory_tags(state: State<'_, AppState>) -> Result<Vec<MemoryTagAudit>, String> {
    use crate::memory::tag_vocabulary::{validate_csv, NonConformingReason, TagValidation};

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let entries = store.get_all().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for entry in entries {
        let mut flagged = Vec::new();
        for (raw, verdict) in entry
            .tags
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .zip(validate_csv(&entry.tags))
        {
            if let TagValidation::NonConforming { reason } = verdict {
                let reason_str = match reason {
                    NonConformingReason::UnknownPrefix(p) => format!("Unknown prefix `{}` (use one of: personal, domain, project, tool, code, external, session, quest)", p),
                    NonConformingReason::MissingPrefix => "Missing `<prefix>:<value>` shape — consider adding a curated prefix".to_string(),
                    NonConformingReason::EmptyValue { prefix } => format!("Empty value after `{}:`", prefix),
                    NonConformingReason::Empty => "Empty tag".to_string(),
                };
                flagged.push(TagAuditFlag {
                    tag: raw.to_string(),
                    reason: reason_str,
                });
            }
        }
        if !flagged.is_empty() {
            out.push(MemoryTagAudit {
                memory_id: entry.id,
                flagged,
            });
        }
    }
    Ok(out)
}

/// Garbage-collect decayed low-importance memories. Returns count removed.
#[tauri::command]
pub async fn gc_memories(
    threshold: Option<f64>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let threshold = threshold.unwrap_or(0.05);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let decayed = store.gc_decayed(threshold).map_err(|e| e.to_string())?;
    drop(store);
    let capped = enforce_configured_memory_limit(&state)?;
    Ok(decayed + capped.deleted)
}

/// Promote a working memory to long-term storage.
#[tauri::command]
pub async fn promote_memory(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .promote(id, crate::memory::MemoryTier::Long)
        .map_err(|e| e.to_string())
}

/// Get memories filtered by tier.
#[tauri::command]
pub async fn get_memories_by_tier(
    tier: String,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEntry>, String> {
    let tier = crate::memory::MemoryTier::from_str(&tier);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.get_by_tier(&tier).map_err(|e| e.to_string())
}

// ── Entity-Relationship Graph commands (Phase 12 / V5 schema) ────────────────

use crate::memory::{
    EdgeDirection, EdgeSource, EdgeStats, MemoryEdge, NewMemoryEdge, COMMON_RELATION_TYPES,
};

/// Insert (or fetch existing) typed edge between two memories.
/// `confidence` is clamped to [0.0, 1.0]; `source` defaults to `"user"`.
///
/// **Temporal validity (V6).** `valid_from` / `valid_to` are optional
/// Unix-ms timestamps bounding the edge's truthiness window.
/// Omit both for the legacy "always valid" semantics.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn add_memory_edge(
    src_id: i64,
    dst_id: i64,
    rel_type: String,
    confidence: Option<f64>,
    source: Option<String>,
    valid_from: Option<i64>,
    valid_to: Option<i64>,
    state: State<'_, AppState>,
) -> Result<MemoryEdge, String> {
    let edge = NewMemoryEdge {
        src_id,
        dst_id,
        rel_type,
        confidence: confidence.unwrap_or(1.0),
        source: source.as_deref().map(EdgeSource::parse).unwrap_or_default(),
        valid_from,
        valid_to,
        edge_source: None,
    };
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let result = store.add_edge(edge).map_err(|e| e.to_string())?;
    drop(store);
    // Invalidate KG cache for affected endpoints (Chunk 41.13).
    state.kg_cache.invalidate(&[src_id, dst_id]);
    Ok(result)
}

/// Close an edge's validity interval at the given Unix-ms timestamp.
///
/// V6 temporal-KG mutation: marks an existing edge as no longer
/// valid from `valid_to` onwards. Returns `1` if a row was updated,
/// `0` if no edge with `edge_id` exists. Idempotent — replaying the
/// call with the same `valid_to` is a no-op semantically.
#[tauri::command]
pub async fn close_memory_edge(
    edge_id: i64,
    valid_to: i64,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .close_edge(edge_id, valid_to)
        .map_err(|e| e.to_string())
}

/// Delete an edge by primary key.
#[tauri::command]
pub async fn delete_memory_edge(edge_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    // Look up endpoints before deletion for cache invalidation (Chunk 41.13).
    let endpoints: Option<(i64, i64)> = store
        .get_edge_by_id(edge_id)
        .ok()
        .map(|e| (e.src_id, e.dst_id));
    store.delete_edge(edge_id).map_err(|e| e.to_string())?;
    drop(store);
    if let Some((src, dst)) = endpoints {
        state.kg_cache.invalidate(&[src, dst]);
    }
    Ok(())
}

/// List all edges in the graph.
#[tauri::command]
pub async fn list_memory_edges(state: State<'_, AppState>) -> Result<Vec<MemoryEdge>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.list_edges().map_err(|e| e.to_string())
}

/// Get edges incident to a specific memory.
/// `direction` accepts `"in"`, `"out"`, or `"both"` (default).
#[tauri::command]
pub async fn get_edges_for_memory(
    memory_id: i64,
    direction: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEdge>, String> {
    let dir = match direction.as_deref().unwrap_or("both") {
        "in" => EdgeDirection::In,
        "out" => EdgeDirection::Out,
        _ => EdgeDirection::Both,
    };
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .get_edges_for(memory_id, dir)
        .map_err(|e| e.to_string())
}

/// Aggregate graph statistics (total edges, by relation type, by source,
/// connected memories).
#[tauri::command]
pub async fn get_edge_stats(state: State<'_, AppState>) -> Result<EdgeStats, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.edge_stats().map_err(|e| e.to_string())
}

/// Curated relation-type vocabulary the UI shows in the edge picker.
/// Backend accepts any string; this list is just a UX hint.
#[tauri::command]
pub async fn list_relation_types() -> Result<Vec<String>, String> {
    Ok(COMMON_RELATION_TYPES
        .iter()
        .map(|s| s.to_string())
        .collect())
}

/// Use the active brain to scan memories in batches and propose typed edges
/// between them. Returns the count of new edges inserted (duplicates skipped).
/// Requires a configured brain — returns an error otherwise.
#[tauri::command]
pub async fn extract_edges_via_brain(
    chunk_size: Option<usize>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let model = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;

    let chunk = chunk_size.unwrap_or(25);
    run_edge_extraction(&model, &state, chunk).await
}

/// Multi-hop hybrid search: vector + keyword + graph traversal.
/// `hops` (default 1) controls how far to walk from each direct hit.
#[tauri::command]
pub async fn multi_hop_search_memories(
    query: String,
    limit: Option<usize>,
    hops: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEntry>, String> {
    let limit = limit.unwrap_or(10);
    let hops = hops.unwrap_or(1).min(3); // hard cap to avoid runaway expansion
    let query_emb = embed(&state, &query).await;
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .hybrid_search_with_graph(&query, query_emb.as_deref(), limit, hops)
        .map_err(|e| e.to_string())
}

// ── Auto-Learn Policy Commands ───────────────────────────────────────────────

/// Return the user's current auto-learn policy from `AppSettings`.
///
/// See `docs/brain-advanced-design.md` § 21 ("How Daily Conversation Updates
/// the Brain") for how the frontend uses this to decide when to call
/// `extract_memories_from_session` automatically.
#[tauri::command]
pub async fn get_auto_learn_policy(
    state: State<'_, AppState>,
) -> Result<crate::memory::AutoLearnPolicy, String> {
    let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.auto_learn_policy)
}

/// Persist a new auto-learn policy. The policy is validated by the
/// `evaluate_auto_learn` decision function, which clamps zero-or-negative
/// thresholds at runtime — invalid values are accepted but neutralised
/// rather than rejected.
#[tauri::command(rename_all = "camelCase")]
pub async fn set_auto_learn_policy(
    policy: crate::memory::AutoLearnPolicy,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        settings.auto_learn_policy = policy;
        crate::settings::config_store::save(&state.data_dir, &settings)?;
    }
    Ok(())
}

/// Pure decision query — given the current session counters, return what
/// the auto-learner *would* do right now without actually firing it.
/// The frontend calls this after every assistant turn to decide whether
/// to invoke `extract_memories_from_session`.
#[tauri::command(rename_all = "camelCase")]
pub async fn evaluate_auto_learn(
    total_turns: u32,
    last_autolearn_turn: Option<u32>,
    state: State<'_, AppState>,
) -> Result<crate::memory::auto_learn::AutoLearnDecisionDto, String> {
    let policy = {
        let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        settings.auto_learn_policy
    };
    let decision = crate::memory::evaluate_auto_learn(policy, total_turns, last_autolearn_turn);
    Ok(crate::memory::auto_learn::AutoLearnDecisionDto::from(
        decision,
    ))
}

// ── Obsidian vault export (Chunk 18.5) ───────────────────────────────

/// Export all long-tier memories to an Obsidian vault directory.
///
/// Creates `<vault_dir>/TerranSoul/` and writes one `.md` file per long-tier
/// memory with YAML frontmatter. Idempotent — skips files whose mtime is >=
/// the memory's last modification.
#[tauri::command(rename_all = "camelCase")]
pub async fn export_to_obsidian(
    vault_dir: String,
    state: State<'_, AppState>,
) -> Result<crate::memory::obsidian_export::ExportReport, String> {
    let entries = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store.get_all().map_err(|e| e.to_string())?
    };
    let path = std::path::Path::new(&vault_dir);
    if !path.exists() {
        return Err(format!("Vault directory does not exist: {vault_dir}"));
    }
    let layout = state
        .app_settings
        .lock()
        .map_err(|e| e.to_string())?
        .obsidian_layout;
    crate::memory::obsidian_export::export_to_vault_with_layout(path, &entries, layout)
}

// ── Bidirectional Obsidian sync (Chunk 17.7) ─────────────────────────

/// Run a single bidirectional sync cycle with an Obsidian vault.
///
/// Exports new/changed memories → vault and imports vault edits → DB.
/// LWW conflict resolution: whichever side has the newer timestamp wins.
#[tauri::command(rename_all = "camelCase")]
pub async fn obsidian_sync(
    vault_dir: String,
    state: State<'_, AppState>,
) -> Result<crate::memory::obsidian_sync::SyncReport, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let path = std::path::Path::new(&vault_dir);
    if !path.exists() {
        return Err(format!("Vault directory does not exist: {vault_dir}"));
    }
    crate::memory::obsidian_sync::sync_bidirectional(path, &store)
}

/// Start background file-watching for bidirectional Obsidian sync.
///
/// Returns immediately. Changes in the vault will be auto-synced with
/// a 1-second debounce. Call `obsidian_sync_stop` to stop watching.
#[tauri::command(rename_all = "camelCase")]
pub async fn obsidian_sync_start(
    vault_dir: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(&vault_dir);
    if !path.exists() {
        return Err(format!("Vault directory does not exist: {vault_dir}"));
    }
    let app_state = state.inner().clone();
    let watcher = crate::memory::obsidian_sync::ObsidianWatcher::start_with_state(path, app_state)
        .map_err(|e| format!("start watcher: {e}"))?;
    let mut guard = state.obsidian_watcher.lock().await;
    if let Some(old) = guard.take() {
        old.stop();
    }
    *guard = Some(watcher);
    Ok(())
}

/// Stop background Obsidian sync file-watching.
#[tauri::command(rename_all = "camelCase")]
pub async fn obsidian_sync_stop(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.obsidian_watcher.lock().await;
    if let Some(watcher) = guard.take() {
        watcher.stop();
    }
    Ok(())
}

// ── GraphRAG community detection + dual-level search (Chunk 16.6) ────

/// Run community detection on the memory knowledge graph and store results.
#[tauri::command(rename_all = "camelCase")]
pub async fn graph_rag_detect_communities(state: State<'_, AppState>) -> Result<usize, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let communities = store
        .detect_and_store_communities()
        .map_err(|e| format!("detect communities: {e}"))?;
    Ok(communities.len())
}

/// Dual-level GraphRAG search: entity + community retrieval fused via RRF.
#[tauri::command(rename_all = "camelCase")]
pub async fn graph_rag_search(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::memory::store::MemoryEntry>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let results = store
        .graph_rag_search(&query, None, limit.unwrap_or(10))
        .map_err(|e| format!("graph_rag_search: {e}"))?;
    // Fetch full entries for the result ids.
    let mut entries = Vec::with_capacity(results.len());
    for (id, _score) in results {
        if let Ok(entry) = store.get_by_id(id) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

// ── Temporal reasoning queries (Chunk 17.3) ──────────────────────────

/// Result of a temporal query: the parsed time range + matching memories.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TemporalQueryResult {
    /// The resolved time range, or `None` if no time expression was detected.
    pub time_range: Option<crate::memory::temporal::TimeRange>,
    /// Memories whose `created_at` falls within the time range.
    /// When no time range is detected, falls back to keyword search over all memories.
    pub memories: Vec<crate::memory::MemoryEntry>,
}

/// Query memories within a natural-language time range.
///
/// Examples: *"what did I learn last month about X?"*, *"have my preferences
/// shifted since April?"*, *"show yesterday's memories"*.
///
/// The `question` is parsed for time expressions (last N days, since date,
/// between dates, today, yesterday, etc.). Memories within the range are
/// returned, optionally keyword-filtered by any non-time-related terms.
#[tauri::command(rename_all = "camelCase")]
pub async fn temporal_query(
    question: String,
    state: State<'_, AppState>,
) -> Result<TemporalQueryResult, String> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let time_range = crate::memory::temporal::parse_time_range(&question, now_ms);

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;

    let memories = match time_range {
        Some(range) => {
            let all = store.get_all().map_err(|e| e.to_string())?;
            all.into_iter()
                .filter(|m| m.created_at >= range.start_ms && m.created_at < range.end_ms)
                .collect()
        }
        None => {
            // No time range detected — fall back to keyword search
            store.search(&question).map_err(|e| e.to_string())?
        }
    };

    Ok(TemporalQueryResult {
        time_range,
        memories,
    })
}

// ── Daily brief quest (Chunk 33B.3) ──────────────────────────────────

/// Result of the daily brief query.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DailyBriefResult {
    /// Memories matching the brief search (time-filtered + RRF-ranked).
    pub memories: Vec<crate::memory::MemoryEntry>,
    /// The time window used (last 24 h from now).
    pub time_range: crate::memory::temporal::TimeRange,
    /// Number of total memories in the time range before RRF filtering.
    pub total_in_range: usize,
}

/// Run the daily brief quest query: retrieves memories from the last 24 h
/// that match "overdue OR upcoming OR commitment" via hybrid_search_rrf,
/// scoped to the temporal window. Returns up to `limit` results ranked by
/// relevance within the recent window.
#[tauri::command(rename_all = "camelCase")]
pub async fn daily_brief_query(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<DailyBriefResult, String> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let range = crate::memory::temporal::TimeRange {
        start_ms: now_ms - 86_400_000, // 24 hours ago
        end_ms: now_ms,
    };

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;

    // Get all memories in the last 24 h.
    let all = store.get_all().map_err(|e| e.to_string())?;
    let recent: Vec<crate::memory::MemoryEntry> = all
        .into_iter()
        .filter(|m| m.created_at >= range.start_ms && m.created_at < range.end_ms)
        .collect();

    let total_in_range = recent.len();

    if recent.is_empty() {
        return Ok(DailyBriefResult {
            memories: vec![],
            time_range: range,
            total_in_range: 0,
        });
    }

    // Run hybrid_search_rrf for relevance ranking with the brief keywords.
    let brief_query = "overdue OR upcoming OR commitment";
    let lim = limit.unwrap_or(10).clamp(1, 50);

    let ranked = store
        .hybrid_search_rrf(brief_query, None, lim)
        .map_err(|e| e.to_string())?;

    // Intersect: only keep RRF results that are within the time range.
    let in_range: Vec<crate::memory::MemoryEntry> = ranked
        .into_iter()
        .filter(|m| m.created_at >= range.start_ms && m.created_at < range.end_ms)
        .collect();

    // If no RRF results fall in range, return the most recent entries by recency.
    let memories = if in_range.is_empty() {
        let mut fallback = recent;
        fallback.sort_by_key(|m| std::cmp::Reverse(m.created_at));
        fallback.truncate(lim);
        fallback
    } else {
        in_range
    };

    Ok(DailyBriefResult {
        memories,
        time_range: range,
        total_in_range,
    })
}

/// Get the full version history for a memory entry (chunk 16.12).
///
/// Returns all previous snapshots ordered oldest-first. An empty list means
/// the memory has never been edited (or the schema is pre-V8).
#[tauri::command(rename_all = "camelCase")]
pub async fn get_memory_history(
    memory_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<crate::memory::versioning::MemoryVersion>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    crate::memory::versioning::get_history(store.conn(), memory_id).map_err(|e| e.to_string())
}

/// Return the joined provenance tree for one memory entry (Chunk 33B.4).
///
/// Includes the current entry, version snapshots, and incident graph edges
/// joined to compact neighboring memory summaries.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_memory_provenance(
    memory_id: i64,
    state: State<'_, AppState>,
) -> Result<crate::memory::audit::MemoryProvenance, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    crate::memory::audit::get_memory_provenance(&store, memory_id).map_err(|e| e.to_string())
}

/// Delete **all** persisted data: memories, brain config, voice config,
/// persona, quest tracker, conversation history, and app settings.
/// Reverts the app to a fresh-install state. Only the device identity
/// key is preserved (for P2P re-linking).
/// This is irreversible — the frontend must confirm with the user.
#[tauri::command]
pub async fn clear_all_data(state: State<'_, AppState>) -> Result<(), String> {
    // 1. Clear all memories, edges, conflicts, versions.
    {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store.delete_all().map_err(|e| e.to_string())?;
    }

    // 2. Clear brain configuration.
    crate::brain::brain_config::clear(&state.data_dir)?;
    crate::brain::clear_brain(&state.data_dir)?;
    {
        let mut mode = state.brain_mode.lock().map_err(|e| e.to_string())?;
        *mode = None;
    }
    {
        let mut ab = state.active_brain.lock().map_err(|e| e.to_string())?;
        *ab = None;
    }

    // 3. Clear embedding + intent caches.
    crate::brain::ollama_agent::clear_embed_caches().await;
    crate::brain::cloud_embeddings::clear_cloud_embed_cache().await;
    crate::brain::intent_classifier::clear_cache();

    // 4. Reset provider rotator.
    {
        let mut rotator = state.provider_rotator.lock().map_err(|e| e.to_string())?;
        *rotator = crate::brain::ProviderRotator::new();
    }

    // 5. Clear voice configuration.
    crate::voice::config_store::clear(&state.data_dir)?;
    {
        let mut vc = state.voice_config.lock().map_err(|e| e.to_string())?;
        *vc = crate::voice::VoiceConfig::default();
    }

    // 6. Clear quest tracker.
    let quest_path = state.data_dir.join("quest_tracker.json");
    if quest_path.exists() {
        let _ = std::fs::remove_file(&quest_path);
    }

    // 7. Clear persona (traits, expressions, motions).
    let persona_dir = state.data_dir.join("persona");
    if persona_dir.exists() {
        let _ = std::fs::remove_dir_all(&persona_dir);
    }
    {
        let mut pb = state.persona_block.lock().map_err(|e| e.to_string())?;
        pb.clear();
    }

    // 8. Clear conversation history.
    {
        let mut conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.clear();
    }

    // 9. Reset app settings to defaults.
    {
        let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        *settings = crate::settings::AppSettings::default();
        crate::settings::config_store::save(&state.data_dir, &settings)?;
    }

    // 10. Delete ANN vector index file.
    let ann_path = state.data_dir.join("vectors.usearch");
    if ann_path.exists() {
        let _ = std::fs::remove_file(&ann_path);
    }

    // 11. Delete MCP token.
    let mcp_path = state.data_dir.join("mcp-token.txt");
    if mcp_path.exists() {
        let _ = std::fs::remove_file(&mcp_path);
    }

    Ok(())
}

// ─── Judgment Rules Commands ────────────────────────────────────────────

/// Add a new judgment rule. Auto-tags with `judgment` if missing.
#[tauri::command]
pub async fn judgment_add(
    content: String,
    tags: String,
    importance: i64,
    state: State<'_, AppState>,
) -> Result<MemoryEntry, String> {
    let entry = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        crate::memory::judgment::add_judgment(&store, &content, &tags, importance)?
    };

    // Best-effort embedding
    if let Some(emb) = embed(&state, &content).await {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        let _ = store.set_embedding(entry.id, &emb);
    }

    Ok(entry)
}

/// List all persisted judgment rules.
#[tauri::command]
pub async fn judgment_list(state: State<'_, AppState>) -> Result<Vec<MemoryEntry>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    Ok(crate::memory::judgment::list_judgments(&store))
}

/// Search for judgment rules relevant to a query and return top-N.
#[tauri::command]
pub async fn judgment_apply(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEntry>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    Ok(crate::memory::judgment::apply_judgments(
        &store,
        &query,
        limit.unwrap_or(5),
    ))
}

#[cfg(all(test, feature = "wasm-sandbox"))]
mod tests {
    use super::*;
    use crate::package_manager::manifest::InstallMethod;
    use crate::plugins::manifest::{
        ActivationEvent, ContributedMemoryHook, Contributions, MemoryHookStage, PluginKind,
        PluginManifest,
    };

    fn memory_hook_wasm(output_json: &str) -> Vec<u8> {
        use wasm_encoder::{
            CodeSection, ConstExpr, DataSection, ExportKind, ExportSection, Function,
            FunctionSection, Instruction, MemorySection, MemoryType, Module, TypeSection, ValType,
        };

        const OUTPUT_OFFSET: u64 = 1024;
        let packed = ((OUTPUT_OFFSET << 32) | output_json.len() as u64) as i64;
        let mut module = Module::new();
        let mut types = TypeSection::new();
        types
            .ty()
            .function([ValType::I32, ValType::I32], [ValType::I64]);
        module.section(&types);

        let mut functions = FunctionSection::new();
        functions.function(0);
        module.section(&functions);

        let mut memories = MemorySection::new();
        memories.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&memories);

        let mut exports = ExportSection::new();
        exports.export("memory", ExportKind::Memory, 0);
        exports.export("memory_hook", ExportKind::Func, 0);
        module.section(&exports);

        let mut code = CodeSection::new();
        let mut function = Function::new([]);
        function.instruction(&Instruction::I64Const(packed));
        function.instruction(&Instruction::End);
        code.function(&function);
        module.section(&code);

        let mut data = DataSection::new();
        data.active(
            0,
            &ConstExpr::i32_const(OUTPUT_OFFSET as i32),
            output_json.as_bytes().iter().copied(),
        );
        module.section(&data);
        module.finish()
    }

    fn memory_plugin_manifest(wasm_path: String) -> PluginManifest {
        PluginManifest {
            id: "memory-tag-rewriter".into(),
            display_name: "Memory Tag Rewriter".into(),
            version: "1.0.0".into(),
            description: "Rewrites tags before memory storage".into(),
            kind: PluginKind::MemoryProcessor,
            install_method: InstallMethod::Wasm { url: wasm_path },
            capabilities: Vec::new(),
            activation_events: vec![ActivationEvent::OnMemoryTag { tag: "seed".into() }],
            contributes: Contributions {
                memory_hooks: vec![ContributedMemoryHook {
                    id: "memory-tag-rewriter.pre-store".into(),
                    stage: MemoryHookStage::PreStore,
                    description: "Normalize memory tags".into(),
                }],
                ..Default::default()
            },
            system_requirements: None,
            api_version: 1,
            homepage: None,
            license: None,
            author: None,
            icon: None,
            publisher: None,
            signature: None,
            sha256: None,
            dependencies: Vec::new(),
        }
    }

    #[tokio::test]
    async fn add_memory_applies_prestore_wasm_tag_rewriter() {
        let state = AppState::for_test();
        let temp_dir = tempfile::tempdir().unwrap();
        let wasm_path = temp_dir.path().join("tag-rewriter.wasm");
        std::fs::write(
            &wasm_path,
            memory_hook_wasm(
                r#"{"content":"Hooked memory","tags":"auto:seed, project:test","importance":5}"#,
            ),
        )
        .unwrap();

        state
            .plugin_host
            .install(memory_plugin_manifest(
                wasm_path.to_string_lossy().to_string(),
            ))
            .await
            .unwrap();

        let entry = add_memory_inner(
            "Original memory".into(),
            "seed:raw".into(),
            2,
            "fact".into(),
            &state,
        )
        .await
        .unwrap();

        assert_eq!(entry.content, "Hooked memory");
        assert_eq!(entry.tags, "auto:seed, project:test");
        assert_eq!(entry.importance, 5);

        let stored = {
            let store = state.memory_store.lock().unwrap();
            store.get_by_id(entry.id).unwrap()
        };
        assert_eq!(stored.content, "Hooked memory");
        assert_eq!(stored.tags, "auto:seed, project:test");
    }
}
