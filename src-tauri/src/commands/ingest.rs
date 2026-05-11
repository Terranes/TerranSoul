use crate::brain::BrainMode;
use crate::memory::late_chunking::CharSpan;
use crate::memory::{MemoryType, NewMemory};
use crate::tasks::manager::{
    CrawlCheckpoint, IngestCheckpoint, TaskKind, TaskProgressEvent, TaskStatus,
};
use crate::AppState;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

/// Result of starting an ingestion task.
#[derive(Debug, serde::Serialize, Clone)]
pub struct IngestStartResult {
    pub task_id: String,
    pub source: String,
    pub source_type: String,
}

#[derive(Debug, Clone)]
struct IngestChunk {
    text: String,
    heading: Option<String>,
    char_span: Option<CharSpan>,
}

#[derive(Clone)]
enum IngestProgressEmitter {
    Tauri(AppHandle),
    Silent,
}

impl IngestProgressEmitter {
    fn emit(&self, event: TaskProgressEvent) {
        if let Self::Tauri(app) = self {
            let _ = app.emit("task-progress", event);
        }
    }
}

/// Ingest a document from a local file path, URL, or web crawl.
/// Runs as a background task with progress events.
#[tauri::command]
pub async fn ingest_document(
    source: String,
    tags: Option<String>,
    importance: Option<i64>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<IngestStartResult, String> {
    start_ingest(
        source,
        tags,
        importance,
        Some(app_handle),
        state.inner().clone(),
    )
    .await
}

/// Start ingestion without a Tauri [`AppHandle`]. Used by MCP stdio, where
/// there is no WebView event channel but the same persistent AppState is
/// available.
pub async fn ingest_document_silent(
    source: String,
    tags: Option<String>,
    importance: Option<i64>,
    state: AppState,
) -> Result<IngestStartResult, String> {
    start_ingest(source, tags, importance, None, state).await
}

async fn start_ingest(
    source: String,
    tags: Option<String>,
    importance: Option<i64>,
    app_handle: Option<AppHandle>,
    state: AppState,
) -> Result<IngestStartResult, String> {
    let tags = tags.unwrap_or_else(|| "imported".to_string());
    let importance = importance.unwrap_or(4).clamp(1, 5);
    let source_trimmed = source.trim().to_string();

    let source_type = if source_trimmed.starts_with("crawl:") {
        "crawl"
    } else if source_trimmed.starts_with("http://") || source_trimmed.starts_with("https://") {
        "url"
    } else {
        "file"
    };

    let description = match source_type {
        "crawl" => format!(
            "Crawling {}",
            source_trimmed
                .strip_prefix("crawl:")
                .unwrap_or(&source_trimmed)
        ),
        "url" => format!("Importing {}", source_trimmed),
        _ => format!(
            "Reading {}",
            std::path::Path::new(&source_trimmed)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&source_trimmed)
        ),
    };

    let kind = if source_type == "crawl" {
        TaskKind::Crawl
    } else {
        TaskKind::Ingest
    };

    let task_id = {
        let mut mgr = state.task_manager.lock().await;
        mgr.create_task(kind.clone(), &description, &source_trimmed)
    };

    let emitter = app_handle
        .map(IngestProgressEmitter::Tauri)
        .unwrap_or(IngestProgressEmitter::Silent);
    emitter.emit(TaskProgressEvent {
        id: task_id.clone(),
        kind: kind.clone(),
        status: TaskStatus::Running,
        progress: 0,
        description: description.clone(),
        processed_items: 0,
        total_items: 0,
        error: None,
    });

    let cancel_flag = {
        let mgr = state.task_manager.lock().await;
        mgr.get_cancel_flag(&task_id)
            .unwrap_or_else(|| Arc::new(std::sync::atomic::AtomicBool::new(false)))
    };

    let task_id_clone = task_id.clone();
    let source_clone = source_trimmed.clone();
    let kind_clone = kind.clone();
    let emitter_clone = emitter.clone();
    let state_clone = state.clone();

    // Acquire a concurrency permit so at most N ingest tasks run their
    // embedding phase simultaneously. This prevents stampeding Ollama when
    // many files are ingested at once (e.g. doc-sync of 56 files).
    let semaphore = state.ingest_semaphore.clone();
    tokio::spawn(async move {
        let _permit = semaphore.acquire().await;
        let result = run_ingest_task(
            &task_id_clone,
            &source_clone,
            &tags,
            importance,
            &cancel_flag,
            &emitter_clone,
            &state_clone,
        )
        .await;

        let mut mgr = state_clone.task_manager.lock().await;
        match result {
            Ok((chunks, total_chars)) => {
                mgr.complete_task(&task_id_clone);
                emitter_clone.emit(TaskProgressEvent {
                    id: task_id_clone,
                    kind: kind_clone,
                    status: TaskStatus::Completed,
                    progress: 100,
                    description: format!("Done! {} chunks from {} chars", chunks, total_chars),
                    processed_items: chunks,
                    total_items: chunks,
                    error: None,
                });
            }
            Err(e) => {
                if e.contains("cancelled") {
                    // Already handled
                } else if e.contains("Auto-paused") {
                    mgr.pause_task(&task_id_clone);
                    let prog = mgr
                        .get_task(&task_id_clone)
                        .map(|t| t.progress)
                        .unwrap_or(0);
                    emitter_clone.emit(TaskProgressEvent {
                        id: task_id_clone,
                        kind: kind_clone,
                        status: TaskStatus::Paused,
                        progress: prog,
                        description: "Paused — exceeded 30 min. Resume to continue.".to_string(),
                        processed_items: 0,
                        total_items: 0,
                        error: Some(e),
                    });
                } else {
                    mgr.fail_task(&task_id_clone, &e);
                    emitter_clone.emit(TaskProgressEvent {
                        id: task_id_clone,
                        kind: kind_clone,
                        status: TaskStatus::Failed,
                        progress: 0,
                        description: "Failed".to_string(),
                        processed_items: 0,
                        total_items: 0,
                        error: Some(e),
                    });
                }
            }
        }
    });

    Ok(IngestStartResult {
        task_id,
        source: source_trimmed,
        source_type: source_type.to_string(),
    })
}

/// Cancel a running ingest/crawl task.
#[tauri::command]
pub async fn cancel_ingest_task(
    task_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut mgr = state.task_manager.lock().await;
    let task = mgr
        .cancel_task(&task_id)
        .ok_or_else(|| format!("Task {} not found", task_id))?;
    let _ = app_handle.emit("task-progress", TaskProgressEvent::from(&task));
    Ok(())
}

/// Resume a paused ingest/crawl task.
#[tauri::command]
pub async fn resume_ingest_task(
    task_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (source, kind) = {
        let mut mgr = state.task_manager.lock().await;
        let task = mgr
            .resume_task(&task_id)
            .ok_or_else(|| format!("Task {} not found", task_id))?;
        let _ = app_handle.emit("task-progress", TaskProgressEvent::from(&task));
        (task.source, task.kind)
    };

    let cancel_flag = {
        let mgr = state.task_manager.lock().await;
        mgr.get_cancel_flag(&task_id)
            .unwrap_or_else(|| Arc::new(std::sync::atomic::AtomicBool::new(false)))
    };

    let task_id_clone = task_id.clone();
    let kind_clone = kind.clone();
    let app = app_handle.clone();

    tokio::spawn(async move {
        let app_state = app.state::<AppState>().inner().clone();
        let emitter = IngestProgressEmitter::Tauri(app.clone());
        let result = run_ingest_task(
            &task_id_clone,
            &source,
            "imported",
            4,
            &cancel_flag,
            &emitter,
            &app_state,
        )
        .await;

        let mut mgr = app_state.task_manager.lock().await;
        match result {
            Ok((chunks, total_chars)) => {
                mgr.complete_task(&task_id_clone);
                emitter.emit(TaskProgressEvent {
                    id: task_id_clone,
                    kind: kind_clone,
                    status: TaskStatus::Completed,
                    progress: 100,
                    description: format!("Done! {} chunks from {} chars", chunks, total_chars),
                    processed_items: chunks,
                    total_items: chunks,
                    error: None,
                });
            }
            Err(e) => {
                if !e.contains("cancelled") {
                    mgr.fail_task(&task_id_clone, &e);
                    emitter.emit(TaskProgressEvent {
                        id: task_id_clone,
                        kind: kind_clone,
                        status: TaskStatus::Failed,
                        progress: 0,
                        description: "Failed".to_string(),
                        processed_items: 0,
                        total_items: 0,
                        error: Some(e),
                    });
                }
            }
        }
    });

    Ok(())
}

/// Get all tasks (active, paused, completed).
#[tauri::command]
pub async fn get_all_tasks(
    state: State<'_, AppState>,
) -> Result<Vec<crate::tasks::manager::TaskInfo>, String> {
    let mgr = state.task_manager.lock().await;
    Ok(mgr.list_tasks())
}

// ── Core ingestion logic (runs in background) ──────────────────────────────────

async fn run_ingest_task(
    task_id: &str,
    source: &str,
    tags: &str,
    importance: i64,
    cancel_flag: &Arc<std::sync::atomic::AtomicBool>,
    emitter: &IngestProgressEmitter,
    state: &AppState,
) -> Result<(usize, usize), String> {
    emit_progress(emitter, task_id, 5, "Fetching content…", 0, 0);

    if cancel_flag.load(Ordering::Relaxed) {
        return Err("Task cancelled".to_string());
    }

    let (text, source_url) = if source.starts_with("crawl:") {
        let url = source.strip_prefix("crawl:").unwrap().trim();
        let crawled =
            crawl_website_with_progress(url, 2, 20, task_id, cancel_flag, emitter, state).await?;
        (crawled, url.to_string())
    } else if source.starts_with("http://") || source.starts_with("https://") {
        let text = fetch_url(source, state).await?;
        (text, source.to_string())
    } else {
        let text = read_local_file(source)?;
        (text, source.to_string())
    };

    if text.trim().is_empty() {
        return Err("Document is empty or could not be parsed.".to_string());
    }

    // Compute content hash for dedup / staleness detection.
    let source_hash = {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        hex::encode(hasher.finalize())
    };

    // Check if we already have content from this source with the same hash → skip.
    {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        if let Ok(Some(existing)) = store.find_by_source_hash(&source_hash) {
            // Content unchanged — skip re-ingest.
            emit_progress(emitter, task_id, 100, "Content unchanged — skipped", 0, 0);
            return Ok((0, existing.token_count as usize));
        }
        // Content changed or new — delete any stale entries from the same source URL.
        let _ = store.delete_by_source_url(&source_url);
    }

    if cancel_flag.load(Ordering::Relaxed) {
        return Err("Task cancelled".to_string());
    }

    let total_chars = text.len();

    // ── Semantic chunking (Chunk 16.11) ────────────────────────────────
    // Use MarkdownSplitter for .md files and HTML-sourced content;
    // TextSplitter for everything else.  Both respect sentence /
    // paragraph / heading boundaries.
    let is_markdown = source_url.ends_with(".md")
        || source_url.ends_with(".markdown")
        || text.starts_with("# ")
        || text.contains("\n## ");
    let raw_chunks = if is_markdown {
        crate::memory::chunking::split_markdown(&text, crate::memory::chunking::DEFAULT_CHUNK_CHARS)
    } else {
        crate::memory::chunking::split_text(&text, crate::memory::chunking::DEFAULT_CHUNK_CHARS)
    };
    let semantic_chunks = crate::memory::chunking::dedup_chunks(raw_chunks);
    let chunk_count = semantic_chunks.len();
    let chunks = build_ingest_chunks(&text, semantic_chunks);

    emit_progress(
        emitter,
        task_id,
        30,
        &format!("Chunked into {} pieces", chunk_count),
        0,
        chunk_count,
    );

    // ── Contextual Retrieval (Anthropic 2024, Chunk 16.2) ──────────────
    // When enabled, generate a document summary once, then prepend a
    // 50–100 token context prefix to each chunk before storing.
    let contextual_retrieval_enabled = state
        .app_settings
        .lock()
        .map(|s| s.contextual_retrieval)
        .unwrap_or(false);
    let late_chunking_enabled = state
        .app_settings
        .lock()
        .map(|s| s.late_chunking)
        .unwrap_or(false);

    let brain_mode = state.brain_mode.lock().ok().and_then(|g| g.clone());
    let active_brain = state.active_brain.lock().ok().and_then(|g| g.clone());

    let doc_summary = if contextual_retrieval_enabled {
        if let Some(mode) = brain_mode.clone() {
            emit_progress(
                emitter,
                task_id,
                32,
                "Generating document summary for contextual retrieval…",
                0,
                chunk_count,
            );
            crate::memory::contextualize::generate_doc_summary(&text, &mode).await
        } else {
            None
        }
    } else {
        None
    };

    let late_chunk_embeddings = if late_chunking_enabled {
        if let Some(model_hint) =
            local_ollama_model_hint(brain_mode.as_ref(), active_brain.as_deref())
        {
            emit_progress(
                emitter,
                task_id,
                34,
                "Generating whole-document token embeddings for late chunking…",
                0,
                chunk_count,
            );
            late_chunk_embeddings_for_chunks(&text, &chunks, &model_hint).await
        } else {
            None
        }
    } else {
        None
    };

    // Store chunks with progress
    let mut created = 0usize;
    let mut created_entries: Vec<(i64, String, bool)> = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        if cancel_flag.load(Ordering::Relaxed) {
            let plain: Vec<String> = chunks.iter().map(|chunk| chunk.text.clone()).collect();
            save_ingest_checkpoint(state, task_id, source, tags, importance, &plain, i).await;
            return Err("Task cancelled".to_string());
        }

        // Check 30-min timeout
        {
            let mut mgr = state.task_manager.lock().await;
            let progress = (30 + (i * 50 / chunk_count.max(1))) as u8;
            if let Some(task) = mgr.update_progress(task_id, progress, i, chunk_count) {
                if task.status == TaskStatus::Paused {
                    let plain: Vec<String> =
                        chunks.iter().map(|chunk| chunk.text.clone()).collect();
                    save_ingest_checkpoint(state, task_id, source, tags, importance, &plain, i)
                        .await;
                    return Err("Auto-paused: exceeded 30-minute limit".to_string());
                }
            }
        }

        if chunk.text.trim().len() < 10 {
            continue;
        }
        let mut chunk_tags = if chunk_count > 1 {
            format!("{},chunk-{}/{}", tags, i + 1, chunk_count)
        } else {
            tags.to_string()
        };
        // Propagate Markdown heading as a tag when available.
        if let Some(heading) = &chunk.heading {
            let slug: String = heading
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect::<String>();
            let slug = slug.trim_matches('-');
            if !slug.is_empty() {
                chunk_tags = format!("{chunk_tags},section:{slug}");
            }
        }

        // Contextual Retrieval: prepend document-level context to the chunk.
        let final_content = if let Some(ref summary) = doc_summary {
            if let Some(mode) = brain_mode.clone() {
                if let Some(ctx) =
                    crate::memory::contextualize::contextualise_chunk(summary, &chunk.text, &mode)
                        .await
                {
                    crate::memory::contextualize::prepend_context(&ctx, &chunk.text)
                } else {
                    chunk.text.clone()
                }
            } else {
                chunk.text.clone()
            }
        } else {
            chunk.text.clone()
        };

        let result = {
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            store.add(NewMemory {
                content: final_content,
                tags: chunk_tags,
                importance,
                memory_type: MemoryType::Fact,
                source_url: Some(source_url.clone()),
                source_hash: Some(source_hash.clone()),
                ..Default::default()
            })
        };
        if let Ok(entry) = result {
            let mut embedded = false;
            if let Some(Some(embedding)) = late_chunk_embeddings.as_ref().and_then(|v| v.get(i)) {
                if let Ok(store) = state.memory_store.lock() {
                    embedded = store.set_embedding(entry.id, embedding).is_ok();
                }
            }
            created_entries.push((entry.id, entry.content.clone(), embedded));
            created += 1;
        }

        let progress = (30 + ((i + 1) * 50 / chunk_count.max(1))) as u8;
        emit_progress(
            emitter,
            task_id,
            progress,
            &format!("Stored {}/{} chunks", i + 1, chunk_count),
            i + 1,
            chunk_count,
        );
    }

    if created > 0 {
        if let Some(source_guide) = build_source_guide(&source_url, &text, &chunks) {
            let result = {
                let store = state.memory_store.lock().map_err(|e| e.to_string())?;
                store.add(NewMemory {
                    content: source_guide,
                    tags: build_source_guide_tags(tags, &source_url),
                    importance: (importance + 1).min(5),
                    memory_type: MemoryType::Summary,
                    source_url: Some(source_url.clone()),
                    source_hash: Some(source_hash.clone()),
                    ..Default::default()
                })
            };
            if let Ok(entry) = result {
                created_entries.push((entry.id, entry.content.clone(), false));
            }
        }
    }

    // Embed (best effort)
    emit_progress(
        emitter,
        task_id,
        85,
        "Generating embeddings…",
        created_entries.len(),
        created_entries.len(),
    );

    if brain_mode.is_some() || active_brain.is_some() {
        let pending_embeddings: Vec<(i64, String)> = created_entries
            .iter()
            .filter(|(_, _, embedded)| !*embedded)
            .map(|(id, content, _)| (*id, content.clone()))
            .collect();

        // Batch embed in groups of 32 (Ollama /api/embed + OpenAI both
        // support array input). This is ≥10× faster than sequential
        // single-text calls because it amortises TCP/TLS overhead and
        // lets the model batch GPU work.
        let texts: Vec<&str> = pending_embeddings.iter().map(|(_, c)| c.as_str()).collect();

        let embeddings = crate::brain::embed_batch_for_mode(
            &texts,
            brain_mode.as_ref(),
            active_brain.as_deref(),
            None, // default batch_size = 32
        )
        .await;

        let mut embedded_count = 0usize;
        let mut failed_ids: Vec<i64> = Vec::new();
        for (i, emb_opt) in embeddings.into_iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }
            if let Some(emb) = emb_opt {
                if let Ok(s) = state.memory_store.lock() {
                    let _ = s.set_embedding(pending_embeddings[i].0, &emb);
                    embedded_count += 1;
                }
            } else {
                // Embedding failed — enqueue for retry by the
                // self-healing background worker (Chunk 38.2).
                failed_ids.push(pending_embeddings[i].0);
            }
            // Emit progress every 32 items or on last item.
            if (i + 1) % 32 == 0 || i + 1 == pending_embeddings.len() {
                let progress = (85 + ((i + 1) * 15 / pending_embeddings.len().max(1))) as u8;
                emit_progress(
                    emitter,
                    task_id,
                    progress,
                    &format!("Embedded {}/{}", embedded_count, pending_embeddings.len()),
                    i + 1,
                    pending_embeddings.len(),
                );
            }
        }

        // Enqueue all failed embeddings for retry. The background worker
        // will pick them up on its next 10-s tick.
        if !failed_ids.is_empty() {
            if let Ok(s) = state.memory_store.lock() {
                let _ = crate::memory::embedding_queue::enqueue_many(s.conn(), &failed_ids);
            }
        }
    }

    if let Some(model) = auto_edge_extraction_model(
        state
            .app_settings
            .lock()
            .map(|settings| settings.auto_extract_edges)
            .unwrap_or(true),
        active_brain.as_deref(),
        created_entries.len(),
    ) {
        emit_progress(
            emitter,
            task_id,
            99,
            "Extracting memory graph edges…",
            created_entries.len(),
            created_entries.len(),
        );
        let _ = run_ingest_edge_extraction(state, &model, 25).await;
    }

    Ok((created, total_chars))
}

fn auto_edge_extraction_model(
    auto_extract_edges: bool,
    active_brain: Option<&str>,
    created_entries: usize,
) -> Option<String> {
    if !auto_extract_edges || created_entries == 0 {
        return None;
    }
    active_brain.map(str::to_string)
}

async fn run_ingest_edge_extraction(
    state: &AppState,
    model: &str,
    chunk_size: usize,
) -> Result<usize, String> {
    let entries: Vec<crate::memory::MemoryEntry> = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store.get_all().map_err(|e| e.to_string())?
    };
    if entries.len() < 2 {
        return Ok(0);
    }

    let known_ids: HashSet<i64> = entries.iter().map(|entry| entry.id).collect();
    let agent = crate::brain::OllamaAgent::new(model);
    let chunk_size = chunk_size.clamp(2, 50);
    let mut total_inserted = 0usize;

    for window in entries.chunks(chunk_size) {
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
        if let Ok(inserted) = store.add_edges_batch(&new_edges) {
            total_inserted += inserted;
        }
    }

    Ok(total_inserted)
}

fn build_ingest_chunks(
    document: &str,
    chunks: Vec<crate::memory::chunking::Chunk>,
) -> Vec<IngestChunk> {
    let spans = locate_chunk_char_spans(document, &chunks);
    chunks
        .into_iter()
        .zip(spans)
        .map(|(chunk, char_span)| IngestChunk {
            text: chunk.text,
            heading: chunk.heading,
            char_span,
        })
        .collect()
}

fn build_source_guide(source_url: &str, text: &str, chunks: &[IngestChunk]) -> Option<String> {
    let preview = compact_preview(text, 560);
    if preview.is_empty() {
        return None;
    }

    let title = source_label(source_url);
    let headings = source_headings(chunks, 8);
    let topics = top_source_terms(text, 8);
    let estimated_tokens = text.chars().count().div_ceil(4);

    let mut lines = vec![
        "[DOCUMENT SOURCE GUIDE]".to_string(),
        format!("Source: {title}"),
        format!(
            "Length: {} chunk(s), approximately {estimated_tokens} tokens.",
            chunks.len()
        ),
        format!("Synopsis: {preview}"),
    ];

    if !headings.is_empty() {
        lines.push(format!("Key sections: {}", headings.join("; ")));
    }
    if !topics.is_empty() {
        lines.push(format!("Key topics: {}", topics.join(", ")));
    }

    lines.push(
        "Best use: broad overview, outline, and source-selection questions. Exact quotes or details should be grounded in original chunks from this source when those chunks are available."
            .to_string(),
    );

    let questions = source_guide_questions(&title, &headings);
    if !questions.is_empty() {
        lines.push(format!("Starter questions: {}", questions.join(" | ")));
    }

    lines.push("[/DOCUMENT SOURCE GUIDE]".to_string());
    Some(truncate_chars(&lines.join("\n"), 1_800))
}

fn build_source_guide_tags(tags: &str, source_url: &str) -> String {
    let mut parts: Vec<String> = tags
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    parts.push("source-guide".to_string());
    parts.push("document-summary".to_string());
    parts.push(format!("source:{}", source_slug(source_url)));
    parts.join(",")
}

fn source_label(source_url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(source_url) {
        if matches!(parsed.scheme(), "http" | "https") {
            let host = parsed.host_str().unwrap_or("document");
            let name = parsed
                .path_segments()
                .and_then(|mut segments| segments.next_back())
                .filter(|segment| !segment.is_empty())
                .unwrap_or(host);
            return if name == host {
                host.to_string()
            } else {
                format!("{name} ({host})")
            };
        }
    }

    source_url
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("document")
        .to_string()
}

fn source_slug(source_url: &str) -> String {
    let label = source_label(source_url);
    let mut slug = String::with_capacity(label.len().min(64));
    let mut previous_dash = false;
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
        if slug.len() >= 64 {
            break;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "document".to_string()
    } else {
        slug
    }
}

fn source_headings(chunks: &[IngestChunk], limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    chunks
        .iter()
        .filter_map(|chunk| chunk.heading.as_deref())
        .map(str::trim)
        .filter(|heading| !heading.is_empty())
        .filter(|heading| seen.insert(heading.to_ascii_lowercase()))
        .take(limit)
        .map(|heading| truncate_chars(heading, 80))
        .collect()
}

fn top_source_terms(text: &str, limit: usize) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "about", "after", "again", "also", "because", "before", "being", "between", "could",
        "during", "every", "first", "from", "have", "into", "more", "most", "only", "other",
        "over", "same", "should", "such", "than", "that", "their", "there", "these", "they",
        "this", "through", "under", "using", "when", "where", "which", "while", "with", "would",
        "your",
    ];

    let mut counts: HashMap<String, usize> = HashMap::new();
    for raw in text.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        let word = raw.to_ascii_lowercase();
        if word.len() < 4 || STOP_WORDS.contains(&word.as_str()) {
            continue;
        }
        *counts.entry(word).or_insert(0) += 1;
    }

    let mut terms: Vec<(String, usize)> = counts.into_iter().collect();
    terms.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    terms
        .into_iter()
        .take(limit)
        .map(|(term, _)| term)
        .collect()
}

fn source_guide_questions(title: &str, headings: &[String]) -> Vec<String> {
    let mut questions = vec![
        format!("What are the main takeaways from {title}?"),
        format!("What details in {title} should I verify against the original source?"),
    ];
    if let Some(first_heading) = headings.first() {
        questions.push(format!("What does {title} say about {first_heading}?"));
    }
    if headings.len() > 1 {
        questions.push(format!(
            "How do the sections on {} and {} relate?",
            headings[0], headings[1]
        ));
    }
    questions.truncate(4);
    questions
}

fn compact_preview(text: &str, max_chars: usize) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_chars(collapsed.trim(), max_chars)
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut result: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    result.push_str("...");
    result
}

fn locate_chunk_char_spans(
    document: &str,
    chunks: &[crate::memory::chunking::Chunk],
) -> Vec<Option<CharSpan>> {
    let mut cursor = 0usize;
    chunks
        .iter()
        .map(|chunk| {
            let text = chunk.text.trim();
            if text.is_empty() {
                return None;
            }
            if let Some(relative) = document.get(cursor..).and_then(|tail| tail.find(text)) {
                let start = cursor + relative;
                let end = start + text.len();
                cursor = end;
                return Some(CharSpan::new(start, end));
            }
            document
                .find(text)
                .map(|start| CharSpan::new(start, start + text.len()))
        })
        .collect()
}

async fn late_chunk_embeddings_for_chunks(
    document: &str,
    chunks: &[IngestChunk],
    model_hint: &str,
) -> Option<Vec<Option<Vec<f32>>>> {
    let token_response = crate::brain::OllamaAgent::embed_tokens(document, model_hint).await?;
    let pooled = pool_late_chunk_embeddings(
        chunks,
        &token_response.token_embeddings,
        &token_response.token_char_spans,
    );
    if pooled.iter().any(Option::is_some) {
        Some(pooled)
    } else {
        None
    }
}

fn pool_late_chunk_embeddings(
    chunks: &[IngestChunk],
    token_embeddings: &[Vec<f32>],
    token_char_spans: &[CharSpan],
) -> Vec<Option<Vec<f32>>> {
    let chunk_char_spans: Vec<Option<CharSpan>> =
        chunks.iter().map(|chunk| chunk.char_span).collect();
    let token_spans = crate::memory::late_chunking::token_spans_for_char_spans(
        token_char_spans,
        &chunk_char_spans,
    );
    crate::memory::late_chunking::pool_chunks(token_embeddings, &token_spans)
}

fn local_ollama_model_hint(
    brain_mode: Option<&BrainMode>,
    active_brain: Option<&str>,
) -> Option<String> {
    match brain_mode {
        Some(BrainMode::LocalOllama { model }) => Some(model.clone()),
        Some(_) => None,
        None => active_brain.map(str::to_string),
    }
}

async fn save_ingest_checkpoint(
    state: &AppState,
    task_id: &str,
    source: &str,
    tags: &str,
    importance: i64,
    chunks: &[String],
    next_index: usize,
) {
    let checkpoint = serde_json::to_string(&IngestCheckpoint {
        source: source.to_string(),
        tags: tags.to_string(),
        importance,
        chunks: chunks[next_index..].to_vec(),
        next_chunk_index: next_index,
    })
    .unwrap_or_default();
    let mut mgr = state.task_manager.lock().await;
    mgr.save_checkpoint(task_id, &checkpoint);
}

fn emit_progress(
    emitter: &IngestProgressEmitter,
    task_id: &str,
    progress: u8,
    desc: &str,
    processed: usize,
    total: usize,
) {
    emitter.emit(TaskProgressEvent {
        id: task_id.to_string(),
        kind: TaskKind::Ingest,
        status: TaskStatus::Running,
        progress,
        description: desc.to_string(),
        processed_items: processed,
        total_items: total,
        error: None,
    });
}

// ── File reading ───────────────────────────────────────────────────────────────

fn read_local_file(path: &str) -> Result<String, String> {
    let path = std::path::Path::new(path);
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "md" | "txt" | "csv" | "json" | "xml" | "html" | "htm" | "log" | "rst" | "adoc" => {
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))
        }
        "pdf" => extract_pdf_text(path),
        _ => std::fs::read_to_string(path)
            .map_err(|_| format!("Unsupported or binary file format: .{ext}")),
    }
}

fn extract_pdf_text(path: &std::path::Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read PDF: {e}"))?;
    let content = String::from_utf8_lossy(&bytes);
    let mut text_parts: Vec<String> = Vec::new();
    for bt_block in content.split("BT") {
        if let Some(et_pos) = bt_block.find("ET") {
            let block = &bt_block[..et_pos];
            for part in block.split('(') {
                if let Some(end) = part.find(')') {
                    let text = &part[..end];
                    let clean: String = text
                        .chars()
                        .filter(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
                        .collect();
                    if !clean.trim().is_empty() {
                        text_parts.push(clean);
                    }
                }
            }
        }
    }
    if text_parts.is_empty() {
        return Err(
            "Could not extract text from PDF. The PDF may use image-based content.".to_string(),
        );
    }
    Ok(text_parts.join(" "))
}

// ── URL fetching ───────────────────────────────────────────────────────────────

async fn fetch_url(url: &str, state: &AppState) -> Result<String, String> {
    validate_url(url)?;
    let response = state
        .ollama_client
        .get(url)
        .header("User-Agent", "TerranSoul/0.1 DocumentIngester")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch URL: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}: {}", response.status(), url));
    }
    let ct = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read body: {e}"))?;
    if ct.contains("text/html") || ct.contains("application/xhtml") {
        Ok(extract_text_from_html(&body))
    } else {
        Ok(body)
    }
}

// ── HTML extraction ────────────────────────────────────────────────────────────

fn extract_text_from_html(html: &str) -> String {
    use scraper::{Html, Selector};
    let document = Html::parse_document(html);
    let skip_set: HashSet<&str> = ["script", "style", "noscript", "nav", "footer", "header"]
        .iter()
        .copied()
        .collect();
    let mut text_parts: Vec<String> = Vec::new();
    for sel_str in &["article", "main", "[role=main]", ".content", "#content"] {
        if let Ok(selector) = Selector::parse(sel_str) {
            for element in document.select(&selector) {
                let text: String = element.text().collect::<Vec<_>>().join(" ");
                if text.len() > 100 {
                    text_parts.push(text);
                }
            }
        }
    }
    if text_parts.is_empty() {
        // Remove text from script/style/etc by selecting them out first
        let mut skip_text: HashSet<String> = HashSet::new();
        for skip_tag in &skip_set {
            if let Ok(sel) = Selector::parse(skip_tag) {
                for el in document.select(&sel) {
                    for t in el.text() {
                        skip_text.insert(t.to_string());
                    }
                }
            }
        }
        if let Ok(body_sel) = Selector::parse("body") {
            for element in document.select(&body_sel) {
                for t in element.text() {
                    let trimmed = t.trim();
                    if !trimmed.is_empty() && !skip_text.contains(t) {
                        text_parts.push(trimmed.to_string());
                    }
                }
            }
        }
    }
    let raw = text_parts.join("\n");
    let mut result = String::with_capacity(raw.len());
    let mut prev_nl = false;
    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() {
            if !prev_nl {
                result.push('\n');
                prev_nl = true;
            }
        } else {
            result.push_str(t);
            result.push('\n');
            prev_nl = false;
        }
    }
    result.trim().to_string()
}

// ── Web crawling with progress ─────────────────────────────────────────────────

async fn crawl_website_with_progress(
    start_url: &str,
    max_depth: usize,
    max_pages: usize,
    task_id: &str,
    cancel_flag: &Arc<std::sync::atomic::AtomicBool>,
    emitter: &IngestProgressEmitter,
    state: &AppState,
) -> Result<String, String> {
    use scraper::{Html, Selector};
    use std::collections::VecDeque;

    validate_url(start_url)?;
    let base_url = url::Url::parse(start_url).map_err(|e| format!("Invalid URL: {e}"))?;
    let base_domain = base_url
        .host_str()
        .ok_or_else(|| "Cannot determine domain".to_string())?
        .to_string();

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    let mut all_text: Vec<String> = Vec::new();
    queue.push_back((start_url.to_string(), 0));

    let link_selector =
        Selector::parse("a[href]").map_err(|_| "Failed to create link selector".to_string())?;

    while let Some((url, depth)) = queue.pop_front() {
        if visited.len() >= max_pages || visited.contains(&url) {
            continue;
        }

        if cancel_flag.load(Ordering::Relaxed) {
            save_crawl_checkpoint(
                state,
                task_id,
                &visited,
                &queue,
                &all_text,
                &base_domain,
                max_depth,
                max_pages,
            )
            .await;
            return Err("Task cancelled".to_string());
        }

        // Check 30-min timeout
        {
            let mut mgr = state.task_manager.lock().await;
            let progress = ((visited.len() * 25) / max_pages.max(1)) as u8;
            if let Some(task) = mgr.update_progress(task_id, progress, visited.len(), max_pages) {
                if task.status == TaskStatus::Paused {
                    save_crawl_checkpoint(
                        state,
                        task_id,
                        &visited,
                        &queue,
                        &all_text,
                        &base_domain,
                        max_depth,
                        max_pages,
                    )
                    .await;
                    return Err("Auto-paused: exceeded 30-minute limit".to_string());
                }
            }
        }

        visited.insert(url.clone());

        emitter.emit(TaskProgressEvent {
            id: task_id.to_string(),
            kind: TaskKind::Crawl,
            status: TaskStatus::Running,
            progress: ((visited.len() * 25) / max_pages.max(1)) as u8,
            description: format!(
                "Crawling {}/{} (depth {}/{}): {}",
                visited.len(),
                max_pages,
                depth,
                max_depth,
                truncate_url(&url)
            ),
            processed_items: visited.len(),
            total_items: max_pages,
            error: None,
        });

        let response = match state
            .ollama_client
            .get(&url)
            .header("User-Agent", "TerranSoul/0.1 WebCrawler")
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };
        let body = match response.text().await {
            Ok(b) => b,
            Err(_) => continue,
        };
        let page_text = extract_text_from_html(&body);
        if !page_text.is_empty() {
            all_text.push(format!("--- Source: {} ---\n{}", url, page_text));
        }

        if depth < max_depth {
            let document = Html::parse_document(&body);
            for element in document.select(&link_selector) {
                if let Some(href) = element.value().attr("href") {
                    if let Ok(resolved) = base_url.join(href) {
                        if resolved.host_str() == Some(&base_domain)
                            && (resolved.scheme() == "http" || resolved.scheme() == "https")
                            && !visited.contains(resolved.as_str())
                        {
                            queue.push_back((resolved.to_string(), depth + 1));
                        }
                    }
                }
            }
        }
    }

    if all_text.is_empty() {
        return Err(format!("Could not extract any text from {start_url}"));
    }
    Ok(all_text.join("\n\n"))
}

#[allow(clippy::too_many_arguments)]
async fn save_crawl_checkpoint(
    state: &AppState,
    task_id: &str,
    visited: &HashSet<String>,
    queue: &std::collections::VecDeque<(String, usize)>,
    collected_text: &[String],
    base_domain: &str,
    max_depth: usize,
    max_pages: usize,
) {
    let checkpoint = serde_json::to_string(&CrawlCheckpoint {
        visited: visited.iter().cloned().collect(),
        queue: queue.iter().cloned().collect(),
        collected_text: collected_text.to_vec(),
        base_domain: base_domain.to_string(),
        max_depth,
        max_pages,
    })
    .unwrap_or_default();
    let mut mgr = state.task_manager.lock().await;
    mgr.save_checkpoint(task_id, &checkpoint);
}

fn truncate_url(url: &str) -> String {
    if url.len() > 60 {
        format!("{}…", &url[..57])
    } else {
        url.to_string()
    }
}

// ── URL validation (SSRF prevention) ───────────────────────────────────────────

fn validate_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(format!(
            "Only http/https URLs are supported, got: {}",
            parsed.scheme()
        ));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "URL has no host".to_string())?;
    let blocked = ["localhost", "127.0.0.1", "0.0.0.0", "[::1]", "169.254."];
    for b in &blocked {
        if host.starts_with(b) {
            return Err(format!(
                "Access to internal/private addresses is blocked: {host}"
            ));
        }
    }
    if host.starts_with("10.") || host.starts_with("192.168.") {
        return Err(format!(
            "Access to private network addresses is blocked: {host}"
        ));
    }
    if host.starts_with("172.") {
        if let Some(second) = host.split('.').nth(1) {
            if let Ok(n) = second.parse::<u8>() {
                if (16..=31).contains(&n) {
                    return Err(format!(
                        "Access to private network addresses is blocked: {host}"
                    ));
                }
            }
        }
    }
    Ok(())
}

// ── Text chunking (legacy — superseded by memory::chunking, Chunk 16.11) ───

/// Naive word-count splitter.  Superseded by `memory::chunking::split_text`
/// / `split_markdown` which use semantic boundary detection.  Kept for
/// the resume-from-checkpoint path that stores pre-split `Vec<String>`.
#[allow(dead_code)]
fn chunk_text(text: &str, target_words: usize, overlap_words: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= target_words {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < words.len() {
        let end = (start + target_words).min(words.len());
        chunks.push(words[start..end].join(" "));
        if end >= words.len() {
            break;
        }
        start = end.saturating_sub(overlap_words);
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_text_single() {
        assert_eq!(chunk_text("Hello world", 800, 100).len(), 1);
    }

    #[test]
    fn chunk_text_splits() {
        let text = (0..100)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(chunk_text(&text, 30, 5).len() > 1);
    }

    #[test]
    fn validate_url_blocks_private() {
        assert!(validate_url("http://localhost/x").is_err());
        assert!(validate_url("http://127.0.0.1/x").is_err());
        assert!(validate_url("http://192.168.1.1/x").is_err());
        assert!(validate_url("http://10.0.0.1/x").is_err());
    }

    #[test]
    fn validate_url_allows_public() {
        assert!(validate_url("https://example.com").is_ok());
    }

    #[test]
    fn extract_html_basic() {
        let html = "<html><body><p>Hello</p><script>x()</script></body></html>";
        let text = extract_text_from_html(html);
        assert!(text.contains("Hello"));
        assert!(!text.contains("x()"));
    }

    #[test]
    fn read_file_not_found() {
        assert!(read_local_file("/nonexistent/file.md").is_err());
    }

    #[test]
    fn truncate_url_works() {
        assert_eq!(truncate_url("https://x.com"), "https://x.com");
        let long = "https://example.com/very/long/path/that/exceeds/sixty/characters/easily";
        assert!(truncate_url(long).ends_with('…'));
    }

    #[test]
    fn sha256_hash_deterministic() {
        use sha2::{Digest, Sha256};
        let text = "Rule 14.3: Family law filings require 30-day notice.";
        let hash1 = hex::encode(Sha256::digest(text.as_bytes()));
        let hash2 = hex::encode(Sha256::digest(text.as_bytes()));
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn sha256_hash_changes_with_content() {
        use sha2::{Digest, Sha256};
        let hash_v1 = hex::encode(Sha256::digest(b"30-day deadline"));
        let hash_v2 = hex::encode(Sha256::digest(b"21-day deadline"));
        assert_ne!(hash_v1, hash_v2);
    }

    #[test]
    fn source_guide_is_compact_and_grounded() {
        let text = "# Overview\n\nPrivacy rules require notice before collection. \
                    # Retention\n\nRetention rules describe deletion windows. "
            .repeat(30);
        let chunks = vec![
            IngestChunk {
                text: "Privacy rules require notice before collection.".to_string(),
                heading: Some("Overview".to_string()),
                char_span: None,
            },
            IngestChunk {
                text: "Retention rules describe deletion windows.".to_string(),
                heading: Some("Retention".to_string()),
                char_span: None,
            },
        ];

        let guide = build_source_guide("C:\\docs\\privacy-policy.md", &text, &chunks).unwrap();

        assert!(guide.contains("[DOCUMENT SOURCE GUIDE]"));
        assert!(guide.contains("Source: privacy-policy.md"));
        assert!(guide.contains("Key sections: Overview; Retention"));
        assert!(guide.contains("Key topics:"));
        assert!(guide.contains("Starter questions:"));
        assert!(guide.len() <= 1_800);
    }

    #[test]
    fn source_guide_tags_include_safe_source_slug() {
        let tags = build_source_guide_tags("imported,law", "https://example.com/My Doc.pdf");

        assert!(tags.contains("imported"));
        assert!(tags.contains("law"));
        assert!(tags.contains("source-guide"));
        assert!(tags.contains("document-summary"));
        assert!(!tags.contains(' '));
    }

    #[test]
    fn top_source_terms_filters_common_words() {
        let terms = top_source_terms(
            "privacy privacy privacy and the the collection retention retention",
            3,
        );

        assert_eq!(terms[0], "privacy");
        assert!(terms.contains(&"retention".to_string()));
        assert!(!terms.contains(&"the".to_string()));
    }

    #[test]
    fn locate_chunk_char_spans_follows_document_order() {
        let document = "alpha beta alpha gamma";
        let chunks = vec![
            crate::memory::chunking::Chunk {
                index: 0,
                text: "alpha beta".to_string(),
                hash: "a".to_string(),
                heading: None,
            },
            crate::memory::chunking::Chunk {
                index: 1,
                text: "alpha gamma".to_string(),
                hash: "b".to_string(),
                heading: None,
            },
        ];
        let spans = locate_chunk_char_spans(document, &chunks);
        assert_eq!(
            spans,
            vec![Some(CharSpan::new(0, 10)), Some(CharSpan::new(11, 22))]
        );
    }

    #[test]
    fn pool_late_chunk_embeddings_aligns_chunks_to_tokens() {
        let chunks = vec![
            IngestChunk {
                text: "alpha beta".to_string(),
                heading: None,
                char_span: Some(CharSpan::new(0, 10)),
            },
            IngestChunk {
                text: "gamma".to_string(),
                heading: None,
                char_span: Some(CharSpan::new(11, 16)),
            },
        ];
        let token_embeddings = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 0.0]];
        let token_spans = vec![
            CharSpan::new(0, 5),
            CharSpan::new(6, 10),
            CharSpan::new(11, 16),
        ];
        let pooled = pool_late_chunk_embeddings(&chunks, &token_embeddings, &token_spans);
        assert_eq!(pooled.len(), 2);
        assert!(pooled[0].is_some());
        assert!(pooled[1].is_some());
        assert!((pooled[1].as_ref().unwrap()[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn local_ollama_model_hint_only_uses_ollama_modes() {
        let local = BrainMode::LocalOllama {
            model: "gemma3:4b".to_string(),
        };
        assert_eq!(
            local_ollama_model_hint(Some(&local), Some("legacy")),
            Some("gemma3:4b".to_string())
        );
        let paid = BrainMode::PaidApi {
            provider: "openai".to_string(),
            api_key: "sk".to_string(),
            model: "gpt-4o".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        };
        assert_eq!(local_ollama_model_hint(Some(&paid), Some("legacy")), None);
        assert_eq!(
            local_ollama_model_hint(None, Some("legacy-model")),
            Some("legacy-model".to_string())
        );
    }

    #[test]
    fn auto_edge_extraction_model_requires_setting_model_and_created_entries() {
        assert_eq!(
            auto_edge_extraction_model(true, Some("gemma3:4b"), 2),
            Some("gemma3:4b".to_string())
        );
        assert_eq!(
            auto_edge_extraction_model(false, Some("gemma3:4b"), 2),
            None
        );
        assert_eq!(auto_edge_extraction_model(true, None, 2), None);
        assert_eq!(auto_edge_extraction_model(true, Some("gemma3:4b"), 0), None);
    }
}
