use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use sha2::{Sha256, Digest};
use tauri::{AppHandle, Emitter, Manager, State};
use crate::AppState;
use crate::memory::{MemoryType, NewMemory};
use crate::tasks::manager::{TaskKind, TaskStatus, TaskProgressEvent, CrawlCheckpoint, IngestCheckpoint};

/// Result of starting an ingestion task.
#[derive(Debug, serde::Serialize, Clone)]
pub struct IngestStartResult {
    pub task_id: String,
    pub source: String,
    pub source_type: String,
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
        "crawl" => format!("Crawling {}", source_trimmed.strip_prefix("crawl:").unwrap_or(&source_trimmed)),
        "url" => format!("Importing {}", source_trimmed),
        _ => format!("Reading {}", std::path::Path::new(&source_trimmed).file_name()
            .and_then(|n| n.to_str()).unwrap_or(&source_trimmed)),
    };

    let kind = if source_type == "crawl" { TaskKind::Crawl } else { TaskKind::Ingest };

    let task_id = {
        let mut mgr = state.task_manager.lock().await;
        mgr.create_task(kind.clone(), &description, &source_trimmed)
    };

    let _ = app_handle.emit("task-progress", TaskProgressEvent {
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
        mgr.get_cancel_flag(&task_id).unwrap_or_else(|| Arc::new(std::sync::atomic::AtomicBool::new(false)))
    };

    let task_id_clone = task_id.clone();
    let source_clone = source_trimmed.clone();
    let kind_clone = kind.clone();
    let app = app_handle.clone();

    tokio::spawn(async move {
        let app_state = app.state::<AppState>();
        let result = run_ingest_task(
            &task_id_clone, &source_clone, &tags, importance,
            &cancel_flag, &app, &app_state,
        ).await;

        let mut mgr = app_state.task_manager.lock().await;
        match result {
            Ok((chunks, total_chars)) => {
                mgr.complete_task(&task_id_clone);
                let _ = app.emit("task-progress", TaskProgressEvent {
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
                    let prog = mgr.get_task(&task_id_clone).map(|t| t.progress).unwrap_or(0);
                    let _ = app.emit("task-progress", TaskProgressEvent {
                        id: task_id_clone, kind: kind_clone,
                        status: TaskStatus::Paused, progress: prog,
                        description: "Paused — exceeded 30 min. Resume to continue.".to_string(),
                        processed_items: 0, total_items: 0,
                        error: Some(e),
                    });
                } else {
                    mgr.fail_task(&task_id_clone, &e);
                    let _ = app.emit("task-progress", TaskProgressEvent {
                        id: task_id_clone, kind: kind_clone,
                        status: TaskStatus::Failed, progress: 0,
                        description: "Failed".to_string(),
                        processed_items: 0, total_items: 0,
                        error: Some(e),
                    });
                }
            }
        }
    });

    Ok(IngestStartResult { task_id, source: source_trimmed, source_type: source_type.to_string() })
}

/// Cancel a running ingest/crawl task.
#[tauri::command]
pub async fn cancel_ingest_task(
    task_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut mgr = state.task_manager.lock().await;
    let task = mgr.cancel_task(&task_id)
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
        let task = mgr.resume_task(&task_id)
            .ok_or_else(|| format!("Task {} not found", task_id))?;
        let _ = app_handle.emit("task-progress", TaskProgressEvent::from(&task));
        (task.source, task.kind)
    };

    let cancel_flag = {
        let mgr = state.task_manager.lock().await;
        mgr.get_cancel_flag(&task_id).unwrap_or_else(|| Arc::new(std::sync::atomic::AtomicBool::new(false)))
    };

    let task_id_clone = task_id.clone();
    let kind_clone = kind.clone();
    let app = app_handle.clone();

    tokio::spawn(async move {
        let app_state = app.state::<AppState>();
        let result = run_ingest_task(
            &task_id_clone, &source, "imported", 4,
            &cancel_flag, &app, &app_state,
        ).await;

        let mut mgr = app_state.task_manager.lock().await;
        match result {
            Ok((chunks, total_chars)) => {
                mgr.complete_task(&task_id_clone);
                let _ = app.emit("task-progress", TaskProgressEvent {
                    id: task_id_clone, kind: kind_clone,
                    status: TaskStatus::Completed, progress: 100,
                    description: format!("Done! {} chunks from {} chars", chunks, total_chars),
                    processed_items: chunks, total_items: chunks, error: None,
                });
            }
            Err(e) => {
                if !e.contains("cancelled") {
                    mgr.fail_task(&task_id_clone, &e);
                    let _ = app.emit("task-progress", TaskProgressEvent {
                        id: task_id_clone, kind: kind_clone,
                        status: TaskStatus::Failed, progress: 0,
                        description: "Failed".to_string(),
                        processed_items: 0, total_items: 0, error: Some(e),
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
    task_id: &str, source: &str, tags: &str, importance: i64,
    cancel_flag: &Arc<std::sync::atomic::AtomicBool>,
    app: &AppHandle, state: &State<'_, AppState>,
) -> Result<(usize, usize), String> {
    emit_progress(app, task_id, 5, "Fetching content…", 0, 0);

    if cancel_flag.load(Ordering::Relaxed) {
        return Err("Task cancelled".to_string());
    }

    let (text, source_url) = if source.starts_with("crawl:") {
        let url = source.strip_prefix("crawl:").unwrap().trim();
        let crawled = crawl_website_with_progress(url, 2, 20, task_id, cancel_flag, app, state).await?;
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
            emit_progress(app, task_id, 100, "Content unchanged — skipped", 0, 0);
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
    // Flatten to (text, heading) pairs so the rest of the pipeline works.
    let chunks: Vec<(String, Option<String>)> = semantic_chunks
        .into_iter()
        .map(|c| (c.text, c.heading))
        .collect();

    emit_progress(app, task_id, 30, &format!("Chunked into {} pieces", chunk_count), 0, chunk_count);

    // ── Contextual Retrieval (Anthropic 2024, Chunk 16.2) ──────────────
    // When enabled, generate a document summary once, then prepend a
    // 50–100 token context prefix to each chunk before storing.
    let contextual_retrieval_enabled = state
        .app_settings
        .lock()
        .map(|s| s.contextual_retrieval)
        .unwrap_or(false);

    let doc_summary = if contextual_retrieval_enabled {
        let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?.clone();
        if let Some(mode) = brain_mode {
            emit_progress(app, task_id, 32, "Generating document summary for contextual retrieval…", 0, chunk_count);
            crate::memory::contextualize::generate_doc_summary(&text, &mode).await
        } else {
            None
        }
    } else {
        None
    };

    // Store chunks with progress
    let mut created = 0usize;
    for (i, (chunk_text, chunk_heading)) in chunks.iter().enumerate() {
        if cancel_flag.load(Ordering::Relaxed) {
            let plain: Vec<String> = chunks.iter().map(|(t, _)| t.clone()).collect();
            save_ingest_checkpoint(state, task_id, source, tags, importance, &plain, i).await;
            return Err("Task cancelled".to_string());
        }

        // Check 30-min timeout
        {
            let mut mgr = state.task_manager.lock().await;
            let progress = (30 + (i * 50 / chunk_count.max(1))) as u8;
            if let Some(task) = mgr.update_progress(task_id, progress, i, chunk_count) {
                if task.status == TaskStatus::Paused {
                    let plain: Vec<String> = chunks.iter().map(|(t, _)| t.clone()).collect();
                    save_ingest_checkpoint(state, task_id, source, tags, importance, &plain, i).await;
                    return Err("Auto-paused: exceeded 30-minute limit".to_string());
                }
            }
        }

        if chunk_text.trim().len() < 10 { continue; }
        let mut chunk_tags = if chunk_count > 1 {
            format!("{},chunk-{}/{}", tags, i + 1, chunk_count)
        } else {
            tags.to_string()
        };
        // Propagate Markdown heading as a tag when available.
        if let Some(heading) = chunk_heading {
            let slug: String = heading.chars()
                .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
                .collect::<String>();
            let slug = slug.trim_matches('-');
            if !slug.is_empty() {
                chunk_tags = format!("{chunk_tags},section:{slug}");
            }
        }

        // Contextual Retrieval: prepend document-level context to the chunk.
        let final_content = if let Some(ref summary) = doc_summary {
            let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?.clone();
            if let Some(mode) = brain_mode {
                if let Some(ctx) = crate::memory::contextualize::contextualise_chunk(summary, chunk_text, &mode).await {
                    crate::memory::contextualize::prepend_context(&ctx, chunk_text)
                } else {
                    chunk_text.clone()
                }
            } else {
                chunk_text.clone()
            }
        } else {
            chunk_text.clone()
        };

        let result = {
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            store.add(NewMemory {
                content: final_content, tags: chunk_tags,
                importance, memory_type: MemoryType::Fact,
                source_url: Some(source_url.clone()),
                source_hash: Some(source_hash.clone()),
                ..Default::default()
            })
        };
        if result.is_ok() { created += 1; }

        let progress = (30 + ((i + 1) * 50 / chunk_count.max(1))) as u8;
        emit_progress(app, task_id, progress, &format!("Stored {}/{} chunks", i + 1, chunk_count), i + 1, chunk_count);
    }

    // Embed (best effort)
    emit_progress(app, task_id, 85, "Generating embeddings…", created, chunk_count);

    let model_opt = state.active_brain.lock().map_err(|e| e.to_string())?.clone();
    if let Some(model) = model_opt {
        let recent: Vec<crate::memory::MemoryEntry> = {
            let store = state.memory_store.lock().map_err(|e| e.to_string())?;
            store.get_all().unwrap_or_default().into_iter().take(created).collect()
        };
        for (i, entry) in recent.iter().enumerate() {
            if cancel_flag.load(Ordering::Relaxed) { break; }
            if let Some(emb) = crate::brain::OllamaAgent::embed_text(&entry.content, &model).await {
                if let Ok(s) = state.memory_store.lock() {
                    let _ = s.set_embedding(entry.id, &emb);
                }
            }
            let progress = (85 + ((i + 1) * 15 / recent.len().max(1))) as u8;
            emit_progress(app, task_id, progress, &format!("Embedded {}/{}", i + 1, recent.len()), i + 1, recent.len());
        }
    }

    Ok((created, total_chars))
}

async fn save_ingest_checkpoint(
    state: &State<'_, AppState>, task_id: &str,
    source: &str, tags: &str, importance: i64,
    chunks: &[String], next_index: usize,
) {
    let checkpoint = serde_json::to_string(&IngestCheckpoint {
        source: source.to_string(), tags: tags.to_string(),
        importance, chunks: chunks[next_index..].to_vec(), next_chunk_index: next_index,
    }).unwrap_or_default();
    let mut mgr = state.task_manager.lock().await;
    mgr.save_checkpoint(task_id, &checkpoint);
}

fn emit_progress(app: &AppHandle, task_id: &str, progress: u8, desc: &str, processed: usize, total: usize) {
    let _ = app.emit("task-progress", TaskProgressEvent {
        id: task_id.to_string(), kind: TaskKind::Ingest,
        status: TaskStatus::Running, progress,
        description: desc.to_string(),
        processed_items: processed, total_items: total, error: None,
    });
}

// ── File reading ───────────────────────────────────────────────────────────────

fn read_local_file(path: &str) -> Result<String, String> {
    let path = std::path::Path::new(path);
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
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
                    let clean: String = text.chars()
                        .filter(|c| c.is_ascii_graphic() || c.is_ascii_whitespace()).collect();
                    if !clean.trim().is_empty() { text_parts.push(clean); }
                }
            }
        }
    }
    if text_parts.is_empty() {
        return Err("Could not extract text from PDF. The PDF may use image-based content.".to_string());
    }
    Ok(text_parts.join(" "))
}

// ── URL fetching ───────────────────────────────────────────────────────────────

async fn fetch_url(url: &str, state: &State<'_, AppState>) -> Result<String, String> {
    validate_url(url)?;
    let response = state.ollama_client
        .get(url).header("User-Agent", "TerranSoul/0.1 DocumentIngester")
        .send().await.map_err(|e| format!("Failed to fetch URL: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}: {}", response.status(), url));
    }
    let ct = response.headers().get("content-type")
        .and_then(|v| v.to_str().ok()).unwrap_or("").to_lowercase();
    let body = response.text().await.map_err(|e| format!("Failed to read body: {e}"))?;
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
    let skip_set: HashSet<&str> = ["script", "style", "noscript", "nav", "footer", "header"].iter().copied().collect();
    let mut text_parts: Vec<String> = Vec::new();
    for sel_str in &["article", "main", "[role=main]", ".content", "#content"] {
        if let Ok(selector) = Selector::parse(sel_str) {
            for element in document.select(&selector) {
                let text: String = element.text().collect::<Vec<_>>().join(" ");
                if text.len() > 100 { text_parts.push(text); }
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
        if t.is_empty() { if !prev_nl { result.push('\n'); prev_nl = true; } }
        else { result.push_str(t); result.push('\n'); prev_nl = false; }
    }
    result.trim().to_string()
}

// ── Web crawling with progress ─────────────────────────────────────────────────

async fn crawl_website_with_progress(
    start_url: &str, max_depth: usize, max_pages: usize,
    task_id: &str, cancel_flag: &Arc<std::sync::atomic::AtomicBool>,
    app: &AppHandle, state: &State<'_, AppState>,
) -> Result<String, String> {
    use scraper::{Html, Selector};
    use std::collections::VecDeque;

    validate_url(start_url)?;
    let base_url = url::Url::parse(start_url).map_err(|e| format!("Invalid URL: {e}"))?;
    let base_domain = base_url.host_str()
        .ok_or_else(|| "Cannot determine domain".to_string())?.to_string();

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    let mut all_text: Vec<String> = Vec::new();
    queue.push_back((start_url.to_string(), 0));

    let link_selector = Selector::parse("a[href]")
        .map_err(|_| "Failed to create link selector".to_string())?;

    while let Some((url, depth)) = queue.pop_front() {
        if visited.len() >= max_pages || visited.contains(&url) { continue; }

        if cancel_flag.load(Ordering::Relaxed) {
            save_crawl_checkpoint(state, task_id, &visited, &queue, &all_text, &base_domain, max_depth, max_pages).await;
            return Err("Task cancelled".to_string());
        }

        // Check 30-min timeout
        {
            let mut mgr = state.task_manager.lock().await;
            let progress = ((visited.len() * 25) / max_pages.max(1)) as u8;
            if let Some(task) = mgr.update_progress(task_id, progress, visited.len(), max_pages) {
                if task.status == TaskStatus::Paused {
                    save_crawl_checkpoint(state, task_id, &visited, &queue, &all_text, &base_domain, max_depth, max_pages).await;
                    return Err("Auto-paused: exceeded 30-minute limit".to_string());
                }
            }
        }

        visited.insert(url.clone());

        emit_progress(app, task_id,
            ((visited.len() * 25) / max_pages.max(1)) as u8,
            &format!("Crawling {}/{}: {}", visited.len(), max_pages, truncate_url(&url)),
            visited.len(), max_pages);

        let response = match state.ollama_client
            .get(&url).header("User-Agent", "TerranSoul/0.1 WebCrawler")
            .send().await
        {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };
        let body = match response.text().await { Ok(b) => b, Err(_) => continue };
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
    state: &State<'_, AppState>, task_id: &str,
    visited: &HashSet<String>, queue: &std::collections::VecDeque<(String, usize)>,
    collected_text: &[String], base_domain: &str, max_depth: usize, max_pages: usize,
) {
    let checkpoint = serde_json::to_string(&CrawlCheckpoint {
        visited: visited.iter().cloned().collect(),
        queue: queue.iter().cloned().collect(),
        collected_text: collected_text.to_vec(),
        base_domain: base_domain.to_string(),
        max_depth, max_pages,
    }).unwrap_or_default();
    let mut mgr = state.task_manager.lock().await;
    mgr.save_checkpoint(task_id, &checkpoint);
}

fn truncate_url(url: &str) -> String {
    if url.len() > 60 { format!("{}…", &url[..57]) } else { url.to_string() }
}

// ── URL validation (SSRF prevention) ───────────────────────────────────────────

fn validate_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(format!("Only http/https URLs are supported, got: {}", parsed.scheme()));
    }
    let host = parsed.host_str().ok_or_else(|| "URL has no host".to_string())?;
    let blocked = ["localhost", "127.0.0.1", "0.0.0.0", "[::1]", "169.254."];
    for b in &blocked {
        if host.starts_with(b) {
            return Err(format!("Access to internal/private addresses is blocked: {host}"));
        }
    }
    if host.starts_with("10.") || host.starts_with("192.168.") {
        return Err(format!("Access to private network addresses is blocked: {host}"));
    }
    if host.starts_with("172.") {
        if let Some(second) = host.split('.').nth(1) {
            if let Ok(n) = second.parse::<u8>() {
                if (16..=31).contains(&n) {
                    return Err(format!("Access to private network addresses is blocked: {host}"));
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
    if words.len() <= target_words { return vec![text.to_string()]; }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < words.len() {
        let end = (start + target_words).min(words.len());
        chunks.push(words[start..end].join(" "));
        if end >= words.len() { break; }
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
        let text = (0..100).map(|i| format!("w{i}")).collect::<Vec<_>>().join(" ");
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
        use sha2::{Sha256, Digest};
        let text = "Rule 14.3: Family law filings require 30-day notice.";
        let hash1 = hex::encode(Sha256::digest(text.as_bytes()));
        let hash2 = hex::encode(Sha256::digest(text.as_bytes()));
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn sha256_hash_changes_with_content() {
        use sha2::{Sha256, Digest};
        let hash_v1 = hex::encode(Sha256::digest(b"30-day deadline"));
        let hash_v2 = hex::encode(Sha256::digest(b"21-day deadline"));
        assert_ne!(hash_v1, hash_v2);
    }
}
