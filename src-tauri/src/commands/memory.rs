use tauri::State;

use crate::memory::{MemoryEntry, MemoryUpdate, NewMemory};
use crate::AppState;

/// Read brain_mode + active_brain from state for use by `embed_for_mode`.
/// Returns `(Option<BrainMode>, Option<String>)`.
fn read_embed_context(state: &AppState) -> (Option<crate::brain::BrainMode>, Option<String>) {
    let brain_mode = state
        .brain_mode
        .lock()
        .ok()
        .and_then(|g| g.clone());
    let active_brain = state
        .active_brain
        .lock()
        .ok()
        .and_then(|g| g.clone());
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
    if let Some(emb) = embed(&state, &content).await {
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
                    store.get_by_id(dup_id).ok().map(|dup| (dup_id, dup.content))
                })
        }; // lock released here

        // If a near-duplicate exists, ask the LLM whether the two
        // statements contradict. If so, open a MemoryConflict row.
        if let Some((dup_id, dup_content)) = dup_info {
            let model_opt = state.active_brain.lock().map_err(|e| e.to_string())?.clone();
            if let Some(ref model) = model_opt {
                let agent = crate::brain::OllamaAgent::new(model);
                if let Some(result) = agent
                    .check_contradiction(&dup_content, &content)
                    .await
                {
                    if result.contradicts {
                        let store = state
                            .memory_store
                            .lock()
                            .map_err(|e| e.to_string())?;
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
            let auto_tags =
                crate::memory::auto_tag::auto_tag_content(&content, &mode).await;
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

    Ok(entry)
}

/// Return all stored memories.
#[tauri::command]
pub async fn get_memories(state: State<'_, AppState>) -> Result<Vec<MemoryEntry>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.get_all().map_err(|e| e.to_string())
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
    let mt = memory_type.and_then(|s| {
        serde_json::from_value(serde_json::Value::String(s)).ok()
    });
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
pub async fn delete_memory(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
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
    let conv = state
        .conversation
        .lock()
        .map_err(|e| e.to_string())?;
    let messages: Vec<_> = conv.iter().rev().take(window).rev().cloned().collect();
    Ok(messages)
}

// ── Brain-powered memory commands ────────────────────────────────────────────

/// Use the active brain to extract memorable facts from the current session
/// and store them automatically.  Returns the number of new memories saved.
#[tauri::command]
pub async fn extract_memories_from_session(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let model = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;

    let history: Vec<(String, String)> = {
        let conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    }; // lock released before await

    let facts = crate::memory::brain_memory::extract_facts(&model, &history).await;

    let count = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        crate::memory::brain_memory::save_facts(&facts, &store)
    };
    Ok(count)
}

/// Use the active brain to summarize the current session into a single memory entry.
#[tauri::command]
pub async fn summarize_session(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let model = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "No brain configured. Set up a brain first.".to_string())?;

    let history: Vec<(String, String)> = {
        let conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    }; // lock released before await

    let summary = crate::memory::brain_memory::summarize(&model, &history)
        .await
        .ok_or_else(|| "Session is empty or brain is unreachable.".to_string())?;

    {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        crate::memory::brain_memory::save_summary(&summary, &store);
    }
    Ok(summary)
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
        let results = store.vector_search(&query_emb, limit).map_err(|e| e.to_string())?;
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
        let results = crate::memory::brain_memory::semantic_search_entries(
            &model, &query, &entries, limit,
        )
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
pub async fn backfill_embeddings(
    state: State<'_, AppState>,
) -> Result<usize, String> {
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

/// Return the current database schema version and migration status.
#[tauri::command]
pub async fn get_schema_info(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let version = store.schema_version();
    let count = store.count();
    let unembedded = store.unembedded_ids().map_err(|e| e.to_string())?.len();

    Ok(serde_json::json!({
        "schema_version": version,
        "target_version": crate::memory::migrations::TARGET_VERSION,
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
    store.hybrid_search(&query, query_emb.as_deref(), limit).map_err(|e| e.to_string())
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

    let model_opt = state.active_brain.lock().map_err(|e| e.to_string())?.clone();

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
    let model_opt = state.active_brain.lock().map_err(|e| e.to_string())?.clone();
    let query_emb = embed(&state, &query).await;

    let candidates: Vec<MemoryEntry> = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store
            .hybrid_search_rrf(&query, query_emb.as_deref(), candidates_k)
            .map_err(|e| e.to_string())?
    }; // release the store lock before any LLM await

    // Stage 1.5 — Code-RAG fusion (Chunk 2.2). When the GitNexus sidecar
    // is configured AND the user has granted the `code_intelligence`
    // capability, also dispatch the user query to GitNexus and RRF-fuse
    // its snippets into the candidate set. Failures are swallowed: code
    // intelligence augments recall, it never gates it.
    let candidates = code_rag_fuse(&query, candidates, candidates_k, &state).await;

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

    Ok(crate::memory::reranker::rerank_candidates(candidates, &scores, limit))
}

/// Code-RAG fusion helper for [`rerank_search_memories`] (Chunk 2.2).
///
/// Dispatches `query` to the GitNexus sidecar (when configured + capability
/// granted), normalises the JSON response into pseudo-`MemoryEntry`
/// records, and RRF-fuses them with the existing SQLite recall set. The
/// fused list is truncated to `candidates_k` so the downstream rerank
/// stage's LLM round-trip count stays bounded.
///
/// Failure modes — all silently fall back to returning `db_candidates`
/// unchanged so the user always gets *some* answer:
/// 1. Sidecar handle absent (user never spawned it).
/// 2. Capability not granted (user revoked `code_intelligence`).
/// 3. Sidecar errors (process died, RPC failure, JSON malformed).
/// 4. GitNexus returned a shape we don't recognise (normaliser → empty).
async fn code_rag_fuse(
    query: &str,
    db_candidates: Vec<MemoryEntry>,
    candidates_k: usize,
    state: &State<'_, AppState>,
) -> Vec<MemoryEntry> {
    use crate::commands::gitnexus::GITNEXUS_AGENT;
    use crate::memory::code_rag::gitnexus_response_to_entries;
    use crate::memory::fusion::{reciprocal_rank_fuse, DEFAULT_RRF_K};
    use crate::sandbox::Capability;

    // Cheap pre-check: only proceed when both consent AND a live sidecar
    // exist. This avoids paying the lock + spawn cost when the feature
    // is off (the common case).
    let granted = {
        let cap = state.capability_store.lock().await;
        cap.has_capability(GITNEXUS_AGENT, &Capability::CodeIntelligence)
    };
    if !granted {
        return db_candidates;
    }
    let sidecar = {
        let guard = state.gitnexus_sidecar.lock().await;
        guard.clone()
    };
    let Some(sidecar) = sidecar else {
        return db_candidates;
    };

    // Make sure the bridge knows the capability is on (cheap idempotent).
    sidecar.set_capability(true).await;

    let code_entries = match sidecar.query(query).await {
        Ok(value) => gitnexus_response_to_entries(&value, -1),
        Err(e) => {
            eprintln!("[code-rag] gitnexus query failed: {e}; serving DB-only recall");
            return db_candidates;
        }
    };
    if code_entries.is_empty() {
        return db_candidates;
    }

    // RRF-fuse the two rankings. We RRF on ids only (which are unique
    // across both lists thanks to negative pseudo-ids), then look the
    // entries back up.
    let db_ids: Vec<i64> = db_candidates.iter().map(|e| e.id).collect();
    let code_ids: Vec<i64> = code_entries.iter().map(|e| e.id).collect();
    let fused: Vec<(i64, f64)> =
        reciprocal_rank_fuse(&[db_ids.as_slice(), code_ids.as_slice()], DEFAULT_RRF_K);

    use std::collections::HashMap;
    let mut by_id: HashMap<i64, MemoryEntry> = HashMap::with_capacity(
        db_candidates.len() + code_entries.len(),
    );
    for e in db_candidates {
        by_id.insert(e.id, e);
    }
    for e in code_entries {
        by_id.entry(e.id).or_insert(e);
    }

    fused
        .into_iter()
        .filter_map(|(id, _score)| by_id.remove(&id))
        .take(candidates_k)
        .collect()
}

/// Get memory statistics per tier.
#[tauri::command]
pub async fn get_memory_stats(
    state: State<'_, AppState>,
) -> Result<crate::memory::MemoryStats, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.stats().map_err(|e| e.to_string())
}

/// Apply time-based decay to long-term memories. Returns count of updated entries.
#[tauri::command]
pub async fn apply_memory_decay(
    state: State<'_, AppState>,
) -> Result<usize, String> {
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
    store.auto_promote_to_long(min, win).map_err(|e| e.to_string())
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
pub async fn count_memory_conflicts(
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store
        .count_open_conflicts()
        .map_err(|e| e.to_string())
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
pub async fn audit_memory_tags(
    state: State<'_, AppState>,
) -> Result<Vec<MemoryTagAudit>, String> {
    use crate::memory::tag_vocabulary::{validate_csv, NonConformingReason, TagValidation};

    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let entries = store.get_all().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for entry in entries {
        let mut flagged = Vec::new();
        for (raw, verdict) in entry.tags.split(',').map(str::trim).filter(|t| !t.is_empty()).zip(validate_csv(&entry.tags)) {
            if let TagValidation::NonConforming { reason } = verdict {
                let reason_str = match reason {
                    NonConformingReason::UnknownPrefix(p) => format!("Unknown prefix `{}` (use one of: personal, domain, project, tool, code, external, session, quest)", p),
                    NonConformingReason::MissingPrefix => "Missing `<prefix>:<value>` shape — consider adding a curated prefix".to_string(),
                    NonConformingReason::EmptyValue { prefix } => format!("Empty value after `{}:`", prefix),
                    NonConformingReason::Empty => "Empty tag".to_string(),
                };
                flagged.push(TagAuditFlag { tag: raw.to_string(), reason: reason_str });
            }
        }
        if !flagged.is_empty() {
            out.push(MemoryTagAudit { memory_id: entry.id, flagged });
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
    store.gc_decayed(threshold).map_err(|e| e.to_string())
}

/// Promote a working memory to long-term storage.
#[tauri::command]
pub async fn promote_memory(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.promote(id, crate::memory::MemoryTier::Long).map_err(|e| e.to_string())
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
    EdgeDirection, EdgeSource, EdgeStats, MemoryEdge, NewMemoryEdge,
    COMMON_RELATION_TYPES,
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
    store.add_edge(edge).map_err(|e| e.to_string())
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
    store.close_edge(edge_id, valid_to).map_err(|e| e.to_string())
}

/// Delete an edge by primary key.
#[tauri::command]
pub async fn delete_memory_edge(
    edge_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.delete_edge(edge_id).map_err(|e| e.to_string())
}

/// List all edges in the graph.
#[tauri::command]
pub async fn list_memory_edges(
    state: State<'_, AppState>,
) -> Result<Vec<MemoryEdge>, String> {
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
    store.get_edges_for(memory_id, dir).map_err(|e| e.to_string())
}

/// Aggregate graph statistics (total edges, by relation type, by source,
/// connected memories).
#[tauri::command]
pub async fn get_edge_stats(
    state: State<'_, AppState>,
) -> Result<EdgeStats, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    store.edge_stats().map_err(|e| e.to_string())
}

/// Curated relation-type vocabulary the UI shows in the edge picker.
/// Backend accepts any string; this list is just a UX hint.
#[tauri::command]
pub async fn list_relation_types() -> Result<Vec<String>, String> {
    Ok(COMMON_RELATION_TYPES.iter().map(|s| s.to_string()).collect())
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

    // The store mutex must not be held across `.await`, so we snapshot the
    // memories under a short-lived lock, do all LLM calls without the lock,
    // and re-acquire it briefly to insert each batch of edges.
    let chunk = chunk_size.unwrap_or(25);

    let entries: Vec<MemoryEntry> = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store.get_all().map_err(|e| e.to_string())?
    };
    if entries.len() < 2 {
        return Ok(0);
    }
    let known_ids: std::collections::HashSet<i64> =
        entries.iter().map(|e| e.id).collect();

    let agent = crate::brain::OllamaAgent::new(&model);
    let chunk = chunk.clamp(2, 50);
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
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        if let Ok(n) = store.add_edges_batch(&new_edges) {
            total_inserted += n;
        }
    }
    Ok(total_inserted)
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
    Ok(crate::memory::auto_learn::AutoLearnDecisionDto::from(decision))
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
    crate::memory::obsidian_export::export_to_vault(path, &entries)
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
