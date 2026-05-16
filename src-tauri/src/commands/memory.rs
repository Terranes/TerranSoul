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

    let facts = if let Some(mode) = brain_mode.clone() {
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

    let brain_mode_for_refine = brain_mode;

    // CHAT-PARITY-4 (2026-05-13): snapshot the pre-save high-water mark
    // so we can enumerate facts inserted during this turn for the
    // `auto_detect_conflicts` post-pass below. Captured BEFORE the save
    // block so a row inserted by save_facts / save_facts_refined satisfies
    // `id > pre_max_id`. SELECT MAX(id) on a fresh table returns NULL ⇒
    // we map to 0 so the comparison still works for the first ever fact.
    let pre_max_id: i64 = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store
            .conn()
            .query_row(
                "SELECT COALESCE(MAX(id), 0) FROM memories",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0)
    };

    let count = if let Some(mode) = brain_mode_for_refine.as_ref() {
        // Brain available → refine each fact through the LLM so we
        // update existing knowledge instead of accumulating duplicates.
        // Falls back to plain insert per-fact when the LLM is
        // unreachable, so we never silently lose extracted facts.
        let stats = crate::memory::refine::save_facts_refined(
            &facts,
            mode,
            &state.provider_rotator,
            &state.memory_store,
        )
        .await;
        stats.total_writes()
    } else {
        // No brain configured (e.g. legacy Ollama-only path during
        // first-boot bootstrap) → original blind-insert behaviour.
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        crate::memory::brain_memory::save_facts(&facts, &store)
    };

    // CHAT-PARITY-4 (2026-05-13): auto-detect contradictions on
    // chat-extracted facts when the user opted in via
    // `AppSettings.auto_detect_conflicts` (default `false`). For each
    // fact inserted in this turn (id > `pre_max_id`), embed the content,
    // ask the store for a high-cosine (≥ 0.85) near-duplicate, then ask
    // the active brain whether the new fact actually CONTRADICTS the
    // existing one. If yes, the pure helper `record_contradiction_if`
    // opens a `memory_conflicts` row so the user can pick a winner.
    //
    // The explicit `add_memory` Tauri command has had unconditional
    // auto-detect since Chunk 17.2 — that path is for user-driven memory
    // creation. This block extends the same pattern to chat-extracted
    // facts but keeps it opt-in because each near-duplicate ingest costs
    // one LLM round-trip and chat sessions typically extract many facts.
    //
    // Best-effort throughout: every step is silently skipped on lock
    // poisoning, embedding failure, or brain unreachability so the
    // primary fact-save count stays authoritative.
    if count > 0 {
        let auto_detect = state
            .app_settings
            .lock()
            .map(|s| s.auto_detect_conflicts)
            .unwrap_or(false);
        let active_model = state
            .active_brain
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        if auto_detect {
            if let Some(ref model) = active_model {
                let new_rows: Vec<(i64, String)> = (|| -> Vec<(i64, String)> {
                    let Ok(s) = state.memory_store.lock() else {
                        return Vec::new();
                    };
                    let Ok(mut stmt) = s.conn().prepare(
                        "SELECT id, content FROM memories WHERE id > ?1 ORDER BY id ASC",
                    ) else {
                        return Vec::new();
                    };
                    let Ok(rows) = stmt.query_map(rusqlite::params![pre_max_id], |r| {
                        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
                    }) else {
                        return Vec::new();
                    };
                    rows.filter_map(|r| r.ok()).collect()
                })();
                let agent = crate::brain::OllamaAgent::new(model);
                for (new_id, new_content) in new_rows {
                    let Some(emb) = embed(&state, &new_content).await else {
                        continue;
                    };
                    let dup_opt = match state.memory_store.lock() {
                        Ok(s) => s
                            .find_duplicate(&emb, 0.85)
                            .ok()
                            .flatten()
                            .filter(|&dup_id| dup_id != new_id)
                            .and_then(|dup_id| {
                                s.get_by_id(dup_id).ok().map(|d| (dup_id, d.content))
                            }),
                        Err(_) => None,
                    };
                    let Some((dup_id, dup_content)) = dup_opt else {
                        continue;
                    };
                    let Some(verdict) =
                        agent.check_contradiction(&dup_content, &new_content).await
                    else {
                        continue;
                    };
                    if let Ok(s) = state.memory_store.lock() {
                        let _ = crate::memory::conflicts::record_contradiction_if(
                            &s, new_id, dup_id, &verdict,
                        );
                    }
                }
            }
        }
    }

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

    // GRAPHRAG-1b — auto-fire structured entity/relationship extraction.
    // Gated by `graph_extract_enabled` (default off). When enabled, newly
    // saved facts are scanned for named entities and typed relationships
    // via the active brain provider. Results are materialized as entity
    // memories + edges. Failures are swallowed — primary save succeeded.
    if count > 0 {
        let graph_extract = state
            .app_settings
            .lock()
            .map(|s| s.graph_extract_enabled)
            .unwrap_or(false);
        if graph_extract {
            let brain_mode = state.brain_mode.lock().ok().and_then(|g| g.clone());
            if let Some(mode) = brain_mode {
                // Snapshot the most recent N entries under a short lock.
                let recent: Vec<MemoryEntry> = {
                    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
                    let all = store.get_all().unwrap_or_default();
                    all.into_iter().rev().take(count).collect()
                };
                // Run extraction per entry: LLM call without lock, store
                // materialisation under brief lock per entry.
                let _ = run_graph_extraction(&recent, &mode, &state).await;
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

/// GRAPHRAG-1b: Run structured entity/relationship extraction on a batch of
/// memory entries. For each entry, calls the LLM to extract entities +
/// relationships, then materialises them (entity memories + edges) under a
/// brief store lock per entry. Idempotent: entries that already have
/// extraction edges are skipped.
async fn run_graph_extraction(
    entries: &[MemoryEntry],
    brain_mode: &crate::brain::BrainMode,
    state: &State<'_, AppState>,
) -> Result<crate::memory::extraction::ExtractionReport, String> {
    use crate::memory::extraction::{
        build_extraction_prompt, materialise_edges, materialise_entities,
        parse_extraction_response, ExtractionReport,
    };

    let mut total = ExtractionReport {
        entities_found: 0,
        entities_created: 0,
        entities_deduplicated: 0,
        relationships_found: 0,
        edges_created: 0,
        source_edges_created: 0,
    };

    for entry in entries {
        // Skip very short entries.
        if entry.content.len() < 20 {
            continue;
        }

        // Check idempotency under a short lock.
        {
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            let has_edges: bool = store
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM memory_edges WHERE src_id = ?1 AND edge_source = 'graphrag:extraction'",
                    [entry.id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0)
                > 0;
            if has_edges {
                continue;
            }
        }

        // LLM call — no lock held.
        let user_prompt = build_extraction_prompt(&entry.content);
        let reply = crate::memory::brain_memory::complete_via_mode(
            brain_mode,
            crate::memory::extraction::EXTRACTION_SYSTEM_PROMPT,
            &user_prompt,
            &state.provider_rotator,
        )
        .await;

        let reply = match reply {
            Ok(r) => r,
            Err(_) => continue,
        };

        let result = match parse_extraction_response(&reply) {
            Some(r) => r,
            None => continue,
        };

        if result.entities.is_empty() {
            continue;
        }

        total.entities_found += result.entities.len();
        total.relationships_found += result.relationships.len();

        // Materialise under a brief lock.
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;

        let existing_count = result
            .entities
            .iter()
            .filter(|e| {
                let h = crate::memory::extraction::entity_source_hash(&e.name);
                store.find_by_source_hash(&h).ok().flatten().is_some()
            })
            .count();

        let name_to_id = materialise_entities(&store, &result.entities, entry.id);
        total.entities_created += result.entities.len().saturating_sub(existing_count);
        total.entities_deduplicated += existing_count;

        let (edges, source_edges) =
            materialise_edges(&store, &result.relationships, &name_to_id, entry.id);
        total.edges_created += edges;
        total.source_edges_created += source_edges;

        // KG cache invalidation.
        let endpoints: Vec<i64> = name_to_id.values().copied().collect();
        drop(store);
        state.kg_cache.invalidate(&endpoints);
    }

    Ok(total)
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
pub async fn compact_ann(state: State<'_, AppState>, force: Option<bool>) -> Result<usize, String> {
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct CompactMemoryResult {
    pub id: i64,
    pub rank: usize,
    pub title: String,
    pub preview: String,
    pub tags: String,
    pub importance: i64,
    pub memory_type: String,
    pub tier: String,
    pub created_at: i64,
    pub updated_at: Option<i64>,
    pub session_id: Option<String>,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProgressiveMemorySearchResponse {
    pub compact: Vec<CompactMemoryResult>,
    pub expanded: Vec<MemoryEntry>,
}

/// Progressive disclosure search: return compact ranked metadata first, and
/// optionally expand selected IDs into full `MemoryEntry` payloads.
#[tauri::command]
pub async fn progressive_search_memories(
    query: String,
    limit: Option<usize>,
    expand_ids: Option<Vec<i64>>,
    state: State<'_, AppState>,
) -> Result<ProgressiveMemorySearchResponse, String> {
    let limit = limit.unwrap_or(10).clamp(1, 50);
    let query_emb = embed(&state, &query).await;

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let results = store
        .hybrid_search_rrf(&query, query_emb.as_deref(), limit)
        .map_err(|e| e.to_string())?;

    let compact = results
        .iter()
        .enumerate()
        .map(|(idx, entry)| compact_result(entry, idx + 1))
        .collect();

    let expanded = expand_ids
        .unwrap_or_default()
        .into_iter()
        .take(20)
        .filter_map(|id| store.get_by_id(id).ok())
        .collect();

    Ok(ProgressiveMemorySearchResponse { compact, expanded })
}

fn compact_result(entry: &MemoryEntry, rank: usize) -> CompactMemoryResult {
    CompactMemoryResult {
        id: entry.id,
        rank,
        title: compact_text(&entry.content, 80),
        preview: compact_text(&entry.content, 240),
        tags: entry.tags.clone(),
        importance: entry.importance,
        memory_type: entry.memory_type.as_str().to_string(),
        tier: entry.tier.as_str().to_string(),
        created_at: entry.created_at,
        updated_at: entry.updated_at,
        session_id: entry.session_id.clone(),
        parent_id: entry.parent_id,
    }
}

fn compact_text(content: &str, max_chars: usize) -> String {
    let clean = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() <= max_chars {
        return clean;
    }
    let mut text = clean
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    text.push_str("...");
    text
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

/// Update mutable fields of an existing edge in-place.
///
/// Any combination of `rel_type`, `confidence`, and `source` may be omitted
/// (`None`) to leave that field unchanged. Returns the refreshed edge.
#[tauri::command]
pub async fn update_memory_edge(
    edge_id: i64,
    rel_type: Option<String>,
    confidence: Option<f64>,
    source: Option<String>,
    state: State<'_, AppState>,
) -> Result<MemoryEdge, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let src_parsed = source.as_deref().map(EdgeSource::parse);
    let edge = store
        .update_edge(edge_id, rel_type.as_deref(), confidence, src_parsed)
        .map_err(|e| e.to_string())?;
    drop(store);
    state.kg_cache.invalidate(&[edge.src_id, edge.dst_id]);
    Ok(edge)
}

/// Detach a memory from the graph by deleting every incident edge.
///
/// Returns the count of edges removed. Useful for the "Detach all" UX action
/// on the graph node-detail panel — the memory itself remains intact.
#[tauri::command]
pub async fn detach_memory_node(
    memory_id: i64,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    // Collect neighbour ids before deleting so we can invalidate KG cache.
    let neighbours: Vec<i64> = store
        .get_edges_for(memory_id, EdgeDirection::Both)
        .map_err(|e| e.to_string())?
        .into_iter()
        .flat_map(|e| [e.src_id, e.dst_id])
        .collect();
    let removed = store
        .delete_edges_for_memory(memory_id)
        .map_err(|e| e.to_string())?;
    drop(store);
    let mut affected: Vec<i64> = neighbours;
    affected.push(memory_id);
    state.kg_cache.invalidate(&affected);
    Ok(removed)
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
///
/// `level` is the optional hierarchy depth (GRAPHRAG-1a). Pass `None` to
/// consider community summaries at every level; pass `Some(n)` to
/// restrict the community-side ranking to communities at level `n`.
#[tauri::command(rename_all = "camelCase")]
pub async fn graph_rag_search(
    query: String,
    limit: Option<usize>,
    level: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::memory::store::MemoryEntry>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let results = store
        .graph_rag_search_at_level(&query, None, limit.unwrap_or(10), level)
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

/// Scope-routed GraphRAG retrieval (GRAPHRAG-1c).
///
/// Classifies the query scope automatically (or uses an explicit override)
/// and routes to the best retrieval path:
/// - `global` → community summaries at the top hierarchy level
/// - `local` → entity-walk + cascade expansion
/// - `mixed` → standard dual-level RRF fusion
#[tauri::command(rename_all = "camelCase")]
pub async fn graph_rag_search_routed(
    query: String,
    limit: Option<usize>,
    scope_override: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::memory::store::MemoryEntry>, String> {
    use crate::memory::query_intent::{classify_scope, QueryScope};

    let scope = match scope_override.as_deref() {
        Some("global") => QueryScope::Global,
        Some("local") => QueryScope::Local,
        Some("mixed") => QueryScope::Mixed,
        _ => classify_scope(&query),
    };

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let results = store
        .graph_rag_search_routed(&query, None, limit.unwrap_or(10), scope)
        .map_err(|e| format!("graph_rag_search_routed: {e}"))?;
    let mut entries = Vec::with_capacity(results.len());
    for (id, _score) in results {
        if let Ok(entry) = store.get_by_id(id) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

/// Result of a [`graph_rag_build_hierarchy`] run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphRagHierarchyReport {
    /// Total communities persisted across all levels.
    pub total_communities: usize,
    /// Highest hierarchy level produced (`0` when only the base level
    /// was detected; up to
    /// [`crate::memory::graph_rag::MAX_HIERARCHY_LEVELS`]`- 1`).
    pub max_level: i32,
    /// Number of communities per level, indexed by level.
    pub per_level_counts: Vec<usize>,
    /// How many communities had a fresh LLM summary generated this run.
    pub summaries_generated: usize,
    /// How many communities carried an existing summary forward (same
    /// member set as a previous run, no LLM call needed).
    pub summaries_carried_over: usize,
    /// How many communities still have no summary (because the active
    /// brain mode was unavailable, refused, or the call errored).
    pub summaries_skipped: usize,
}

/// Build a hierarchical community structure (GRAPHRAG-1a).
///
/// Recurses [`MemoryStore::detect_and_store_hierarchy`] to populate
/// `memory_communities` at levels `0..max_levels` (capped by
/// [`crate::memory::graph_rag::MAX_HIERARCHY_LEVELS`]). Then, for every
/// community that lacks a summary, asks the active brain provider to
/// write a 1-3 sentence description of the cluster. Idempotent — a
/// second call on the same graph reuses existing summaries whose member
/// set has not changed.
#[tauri::command(rename_all = "camelCase")]
pub async fn graph_rag_build_hierarchy(
    max_levels: Option<usize>,
    state: State<'_, AppState>,
) -> Result<GraphRagHierarchyReport, String> {
    let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?.clone();
    let target_levels = max_levels
        .unwrap_or(crate::memory::graph_rag::MAX_HIERARCHY_LEVELS)
        .clamp(1, crate::memory::graph_rag::MAX_HIERARCHY_LEVELS);

    // Detect + persist the hierarchy first (carrying any existing summaries
    // whose member set is unchanged).
    let communities = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store
            .detect_and_store_hierarchy(target_levels)
            .map_err(|e| format!("detect hierarchy: {e}"))?
    };

    let max_level = communities.iter().map(|c| c.level).max().unwrap_or(0);
    let level_count = (max_level as usize) + 1;
    let mut per_level_counts = vec![0_usize; level_count];
    for c in &communities {
        let idx = c.level as usize;
        if idx < per_level_counts.len() {
            per_level_counts[idx] += 1;
        }
    }

    let carried_over = communities
        .iter()
        .filter(|c| c.summary.is_some())
        .count();
    let mut generated = 0_usize;
    let mut skipped = 0_usize;

    // Generate summaries for any community without one. We grab a fresh
    // snapshot of (id, level, member_ids) and the per-id contents now so
    // the brain call below doesn't hold the memory_store lock across an
    // .await.
    let pending: Vec<(i64, i32, Vec<i64>)> = communities
        .iter()
        .filter(|c| c.summary.is_none())
        .map(|c| (c.id, c.level, c.member_ids.clone()))
        .collect();

    for (community_id, level, member_ids) in pending {
        let snippets: Vec<String> = {
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            let mut out = Vec::with_capacity(member_ids.len());
            for mid in &member_ids {
                if let Ok(entry) = store.get_by_id(*mid) {
                    out.push(format!(
                        "- {}",
                        entry.content.chars().take(280).collect::<String>()
                    ));
                }
            }
            out
        };

        if snippets.is_empty() {
            skipped += 1;
            continue;
        }

        let summary = match brain_mode.as_ref() {
            Some(mode) => {
                let system = "You are a concise summarizer. Produce 1-3 sentences \
                              describing the shared theme of the listed memories. \
                              Do not enumerate the items; capture the cluster's \
                              gist.";
                let prompt = format!(
                    "Hierarchy level: {}\nMember count: {}\nMemories:\n{}",
                    level,
                    member_ids.len(),
                    snippets.join("\n")
                );
                match crate::memory::brain_memory::complete_via_mode(
                    mode,
                    system,
                    &prompt,
                    &state.provider_rotator,
                )
                .await
                {
                    Ok(reply) => {
                        let trimmed = reply.trim().to_string();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed)
                        }
                    }
                    Err(_) => None,
                }
            }
            None => None,
        };

        match summary {
            Some(text) => {
                let store = state.memory_store.lock().map_err(|e| e.to_string())?;
                store
                    .set_community_summary(community_id, &text, None)
                    .map_err(|e| format!("set summary: {e}"))?;
                generated += 1;
            }
            None => {
                skipped += 1;
            }
        }
    }

    Ok(GraphRagHierarchyReport {
        total_communities: communities.len(),
        max_level,
        per_level_counts,
        summaries_generated: generated,
        summaries_carried_over: carried_over,
        summaries_skipped: skipped,
    })
}

/// Result of a [`graph_extract_entities`] run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphExtractReport {
    pub entities_found: usize,
    pub entities_created: usize,
    pub entities_deduplicated: usize,
    pub relationships_found: usize,
    pub edges_created: usize,
    pub source_edges_created: usize,
}

/// GRAPHRAG-1b: Run structured entity / relationship extraction on recent
/// memories (or all if `limit` is `None`). Requires an active brain mode.
///
/// Idempotent — memories that already have `graphrag:extraction` edges are
/// skipped. Entities are deduplicated by normalised name (`source_hash`).
#[tauri::command(rename_all = "camelCase")]
pub async fn graph_extract_entities(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<GraphExtractReport, String> {
    let brain_mode = state
        .brain_mode
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No brain configured. Enable a brain provider first.".to_string())?;

    let entries: Vec<MemoryEntry> = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        let all = store.get_all().map_err(|e| e.to_string())?;
        match limit {
            Some(n) => all.into_iter().rev().take(n).collect(),
            None => all,
        }
    };

    let report = run_graph_extraction(&entries, &brain_mode, &state).await?;

    Ok(GraphExtractReport {
        entities_found: report.entities_found,
        entities_created: report.entities_created,
        entities_deduplicated: report.entities_deduplicated,
        relationships_found: report.relationships_found,
        edges_created: report.edges_created,
        source_edges_created: report.source_edges_created,
    })
}

// ── (legacy) ─────────────────────────────────────────────────────────

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

/// Return a paged, LOD view of the memory knowledge graph.
///
/// The frontend graph viewport cannot render more than a few thousand
/// nodes at once even with a WebGL renderer; at billion-node scale it
/// cannot even hold them in JS memory. This command returns at most
/// `limit` (default 2 000, hard cap 10 000) nodes plus the edges that
/// connect them, with three LOD modes:
///
/// * `"overview"` — one supernode per cognitive kind.
/// * `"cluster"` — real nodes inside `focus_kind`, supernodes for the rest.
/// * `"detail"`  — real nodes near `focus_id`, ranked by degree / importance.
///
/// See `docs/billion-scale-retrieval-design.md` Phase 1 + Phase 5.
///
/// At scale, this command avoids loading the entire graph into memory:
/// - **Detail zoom with focus_id**: uses paged adjacency (covering indexes)
///   to fetch only the focus node's neighbourhood.
/// - **Overview zoom**: uses pre-aggregated `memory_graph_clusters` when
///   available, falling back to full scan for fresh databases.
/// - **Cluster zoom** / no focus: falls back to the Phase 1 pure function
///   `build_graph_page` (loads all entries + edges).
#[tauri::command]
pub async fn memory_graph_page(
    focus_id: Option<i64>,
    focus_kind: Option<String>,
    zoom: Option<String>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<crate::memory::graph_page::GraphPageResponse, String> {
    use crate::memory::graph_page::{
        build_graph_page, GraphEdge, GraphNode, GraphPageRequest, GraphPageResponse, GraphZoom,
        DEFAULT_GRAPH_LIMIT, MAX_GRAPH_NODES,
    };
    use crate::memory::CognitiveKind;

    let zoom = match zoom.as_deref().unwrap_or("detail") {
        "overview" => GraphZoom::Overview,
        "cluster" => GraphZoom::Cluster,
        _ => GraphZoom::Detail,
    };
    let focus_kind = focus_kind.as_deref().and_then(|s| match s {
        "episodic" => Some(CognitiveKind::Episodic),
        "semantic" => Some(CognitiveKind::Semantic),
        "procedural" => Some(CognitiveKind::Procedural),
        "judgment" => Some(CognitiveKind::Judgment),
        "negative" => Some(CognitiveKind::Negative),
        _ => None,
    });
    let limit_val = limit
        .unwrap_or(DEFAULT_GRAPH_LIMIT)
        .clamp(1, MAX_GRAPH_NODES);

    // ── Fast path: Detail zoom with a focus node (paged adjacency) ──
    if let (GraphZoom::Detail, Some(fid)) = (&zoom, focus_id) {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        let page = store
            .get_edges_paged(fid, limit_val, 0, None)
            .map_err(|e| e.to_string())?;
        let (total_nodes, total_edges) = store.graph_totals().map_err(|e| e.to_string())?;

        // Collect neighbour ids from edges.
        let mut neighbour_ids: Vec<i64> = Vec::with_capacity(page.edges.len() + 1);
        neighbour_ids.push(fid);
        for e in &page.edges {
            let other = if e.src_id == fid { e.dst_id } else { e.src_id };
            if !neighbour_ids.contains(&other) {
                neighbour_ids.push(other);
            }
        }

        // Load only the focus node + its immediate neighbours.
        let entries = store
            .get_entries_by_ids(&neighbour_ids)
            .map_err(|e| e.to_string())?;

        // Build response.
        let mut nodes: Vec<GraphNode> = entries
            .iter()
            .map(|entry| {
                let kind = crate::memory::cognitive_kind::classify(
                    &entry.memory_type,
                    &entry.tags,
                    &entry.content,
                );
                let degree = page
                    .edges
                    .iter()
                    .filter(|e| e.src_id == entry.id || e.dst_id == entry.id)
                    .count() as i64;
                GraphNode {
                    id: format!("m-{}", entry.id),
                    label: entry.content.chars().take(80).collect(),
                    kind: kind.as_str().to_string(),
                    tier: entry.tier.as_str().to_string(),
                    importance: entry.importance,
                    degree,
                    is_supernode: false,
                    count: 1,
                    origin_device: entry.origin_device.clone().unwrap_or_default(),
                }
            })
            .collect();
        nodes.sort_by_key(|n| std::cmp::Reverse(n.degree));

        let edges: Vec<GraphEdge> = page
            .edges
            .iter()
            .map(|e| GraphEdge {
                source: format!("m-{}", e.src_id),
                target: format!("m-{}", e.dst_id),
                rel_type: e.rel_type.clone(),
                weight: e.confidence,
            })
            .collect();

        return Ok(GraphPageResponse {
            nodes,
            edges,
            total_nodes,
            total_edges,
            truncated: page.has_more,
            zoom: GraphZoom::Detail,
        });
    }

    // ── Fallback path: load all entries + edges (Phase 1 behaviour) ──
    let (entries, edges) = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        let entries = store.get_all().map_err(|e| e.to_string())?;
        let edges = store.list_edges().map_err(|e| e.to_string())?;
        (entries, edges)
    };

    let req = GraphPageRequest {
        focus_id,
        focus_kind,
        zoom,
        limit,
    };
    Ok(build_graph_page(&entries, &edges, &req))
}

/// Preview deterministic disk-backed ANN migration candidates without writing sidecars.
#[tauri::command]
pub async fn disk_ann_plan_preview(
    threshold: Option<usize>,
    state: State<'_, AppState>,
) -> Result<crate::memory::disk_backed_ann::DiskAnnPlan, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .disk_ann_plan(
            threshold.unwrap_or(crate::memory::disk_backed_ann::DEFAULT_DISK_ANN_ENTRY_THRESHOLD),
        )
        .map_err(|e| e.to_string())
}

/// Return disk-backed ANN migration readiness: eligible candidates, sidecars present,
/// and missing sidecar count.
#[tauri::command]
pub async fn disk_ann_migration_status(
    threshold: Option<usize>,
    state: State<'_, AppState>,
) -> Result<crate::memory::disk_backed_ann::DiskAnnHealthSummary, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .disk_ann_health_summary(
            threshold.unwrap_or(crate::memory::disk_backed_ann::DEFAULT_DISK_ANN_ENTRY_THRESHOLD),
        )
        .map_err(|e| e.to_string())
}

/// Execute one disk-backed ANN migration batch by writing IVF-PQ sidecars for
/// top eligible shards.
#[tauri::command]
pub async fn run_disk_ann_migration(
    threshold: Option<usize>,
    max_shards: Option<usize>,
    state: State<'_, AppState>,
) -> Result<crate::memory::disk_backed_ann::DiskAnnMigrationReport, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .run_disk_ann_migration_job(
            threshold.unwrap_or(crate::memory::disk_backed_ann::DEFAULT_DISK_ANN_ENTRY_THRESHOLD),
            max_shards
                .unwrap_or(crate::memory::disk_backed_ann::DEFAULT_DISK_ANN_MAX_SHARDS_PER_RUN),
        )
        .map_err(|e| e.to_string())
}

/// Build IVF-PQ indexes for shards that have planned sidecars.
/// This is the Phase 3 execution step: trains coarse + PQ codebooks,
/// encodes all shard vectors, and writes binary IVF-PQ index files.
#[tauri::command]
pub async fn build_ivf_pq_indexes(
    max_shards: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::memory::ivf_pq::IvfPqBuildStats>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .build_ivf_pq_indexes(
            max_shards
                .unwrap_or(crate::memory::disk_backed_ann::DEFAULT_DISK_ANN_MAX_SHARDS_PER_RUN),
        )
        .map_err(|e| e.to_string())
}

// ── Chunk 50.1 — Shard health, router health, and graph observability ─────────

/// Per-shard capacity and index health summary (shard backpressure, Chunk 48.7).
/// Returns health status for every shard that has at least one entry.
/// `max_entries` sets the per-shard capacity threshold (default: 2 million).
#[tauri::command]
pub async fn shard_health(
    max_entries: Option<i64>,
    state: State<'_, AppState>,
) -> Result<crate::memory::shard_backpressure::ShardHealthSummary, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .shard_health_summary(
            max_entries.unwrap_or(crate::memory::shard_backpressure::DEFAULT_SHARD_MAX_ENTRIES),
        )
        .map_err(|e| e.to_string())
}

/// Coarse shard router health metadata (Chunk 48.3).
/// Reports built-at timestamp, centroid count, staleness, and refresh eligibility.
#[tauri::command]
pub async fn router_health(
    state: State<'_, AppState>,
) -> Result<crate::memory::shard_router::RouterHealth, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.router_health_summary()
}

/// Explicitly rebuild the coarse shard router from a 1% sample of embeddings.
/// Useful after large bulk ingests or when the router is stale.
/// Returns the number of centroids added.
#[tauri::command]
pub async fn rebuild_shard_router(state: State<'_, AppState>) -> Result<usize, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.build_shard_router()
}

/// Rebalance all shard ANN indices from live embeddings.
/// Use this after a large migration or schema change to rebuild all per-shard HNSW indices.
/// Returns the total vector count across all shards.
#[tauri::command]
pub async fn rebalance_ann_shards(state: State<'_, AppState>) -> Result<usize, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.rebalance_shards()
}

/// Refresh the `memory_graph_clusters` pre-aggregated stats table.
/// Called automatically during nightly compaction; call explicitly after large ingests.
/// Returns the number of cluster rows written.
#[tauri::command]
pub async fn refresh_graph_clusters(state: State<'_, AppState>) -> Result<usize, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.refresh_graph_clusters().map_err(|e| e.to_string())
}

/// Return the top-K memory nodes ranked by graph degree (in + out edges).
/// Optionally filter by cognitive_kind (e.g. "semantic", "episodic").
#[tauri::command]
pub async fn get_top_degree_nodes(
    kind: Option<String>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::memory::graph_paging::DegreeNode>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .get_top_degree_nodes(kind.as_deref(), limit.unwrap_or(20))
        .map_err(|e| e.to_string())
}

/// Return total node and edge counts for the graph overview.
#[tauri::command]
pub async fn graph_totals(state: State<'_, AppState>) -> Result<(i64, i64), String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.graph_totals().map_err(|e| e.to_string())
}

/// MEM-DRILLDOWN-1 — provenance drill-down. Walk `derived_from` edges
/// OUT from a memory and return the full chain of source memories with
/// their depth from the root.
#[tauri::command]
pub async fn memory_drilldown(
    memory_id: i64,
    max_depth: Option<usize>,
    state: State<'_, AppState>,
) -> Result<crate::memory::drilldown::SourceChain, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .source_chain(memory_id, max_depth)
        .map_err(|e| e.to_string())
}

/// CTX-OFFLOAD-1a — fetch an offloaded verbose tool-output payload from
/// the sidecar `memory_offload_payloads` table. Returns the raw bytes
/// alongside their metadata so the caller can decide how to re-inflate
/// the content into the agent context. Errors with `"not found"` when
/// no payload row exists for the given memory id.
#[tauri::command]
pub async fn memory_drilldown_payload(
    memory_id: i64,
    state: State<'_, AppState>,
) -> Result<crate::memory::offload_payload::OffloadPayload, String> {
    if memory_id <= 0 {
        return Err("memory_id must be positive".into());
    }
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let payload = store
        .get_offload_payload(memory_id)
        .map_err(|e| e.to_string())?;
    payload.ok_or_else(|| format!("not found: offload payload for memory id {}", memory_id))
}

/// CTX-OFFLOAD-1b — total bytes stored across all offloaded tool-output
/// payloads. Used by the "Context Compression" skill-tree quest to detect
/// whether the agent runtime has offloaded any verbose tool output yet.
#[tauri::command]
pub async fn memory_offload_payload_total_bytes(
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .offload_payload_total_bytes()
        .map_err(|e| e.to_string())
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
