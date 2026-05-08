use tauri::State;

use crate::memory::wiki::{
    append_and_review_queue, audit_report, ensure_source_dedup, god_nodes, surprising_connections,
    AuditConfig, AuditReport, GodNode, ReviewItem, SourceDedupResult, SurprisingConnection,
};
use crate::AppState;

fn clamp_limit(limit: Option<usize>, default: usize, max: usize) -> usize {
    limit.unwrap_or(default).clamp(1, max)
}

/// Brain wiki audit — the chat/UI equivalent of an LLM-wiki lint pass.
#[tauri::command]
pub async fn brain_wiki_audit(
    limit: Option<usize>,
    stale_threshold: Option<f64>,
    state: State<'_, AppState>,
) -> Result<AuditReport, String> {
    let cfg = AuditConfig {
        limit: clamp_limit(limit, 50, 200),
        stale_threshold: stale_threshold.unwrap_or(0.20).clamp(0.0, 1.0),
    };
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    audit_report(&store, &cfg).map_err(|e| e.to_string())
}

/// Brain wiki spotlight — most-connected memories (graphify-style god nodes).
#[tauri::command]
pub async fn brain_wiki_spotlight(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<GodNode>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    god_nodes(&store, clamp_limit(limit, 10, 100)).map_err(|e| e.to_string())
}

/// Brain wiki serendipity — high-confidence cross-community edges.
#[tauri::command]
pub async fn brain_wiki_serendipity(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<SurprisingConnection>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    surprising_connections(&store, clamp_limit(limit, 10, 100)).map_err(|e| e.to_string())
}

/// Brain wiki revisit queue — append-and-review style sinking notes.
#[tauri::command]
pub async fn brain_wiki_revisit(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<ReviewItem>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    append_and_review_queue(&store, clamp_limit(limit, 12, 100)).map_err(|e| e.to_string())
}

/// Text digest with source-hash dedup. URL fetching remains in `brain_ingest_url`;
/// this command handles pasted notes, file snippets, and future MCP resources.
#[tauri::command]
pub async fn brain_wiki_digest_text(
    content: String,
    source_url: Option<String>,
    tags: Option<String>,
    importance: Option<i64>,
    state: State<'_, AppState>,
) -> Result<SourceDedupResult, String> {
    let body = content.trim();
    if body.is_empty() {
        return Err("content cannot be empty".into());
    }
    let tags = tags.unwrap_or_else(|| "wiki:digest".into());
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    ensure_source_dedup(
        &store,
        source_url.as_deref(),
        body,
        &tags,
        importance.unwrap_or(3),
    )
    .map_err(|e| e.to_string())
}
