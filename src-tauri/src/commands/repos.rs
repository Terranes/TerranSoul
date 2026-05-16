//! BRAIN-REPO-RAG-1b-i — Tauri command surface for per-repo ingest.
//!
//! Three commands that thin-wrap [`crate::memory::repo_ingest`] and update
//! `memory_sources.last_synced_at` on success.
//!
//! BRAIN-REPO-RAG-1b-ii-b: progress events flow through [`TauriIngestSink`]
//! and an embedding pass runs after the clone/walk/chunk stage, populating
//! the per-repo HNSW index at `<data_dir>/repos/<source_id>/vectors.usearch`.

#![cfg(feature = "repo-rag")]

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};

use crate::brain::cloud_embeddings::embed_for_mode;
use crate::memory::repo_ingest::{
    self, IngestPhase, IngestProgress, IngestSink, RepoEmbedStats, RepoIngestOptions,
    RepoIngestStats, RepoManifest,
};
use crate::memory::sources;
use crate::tasks::manager::{TaskKind, TaskProgressEvent, TaskStatus};
use crate::AppState;

/// 1024 matches the mxbai-embed-large default in
/// `model_recommender.rs`. Cloud paid/free providers also return 1024-dim
/// vectors via `cloud_embeddings::embed_for_mode`.
const REPO_EMBED_DIMENSIONS: usize = 1024;

/// IngestSink adapter that re-emits each [`IngestProgress`] as a
/// `"task-progress"` Tauri event so the Memory panel can stream phase
/// updates. Mirrors the Supertonic-1c emitter pattern in
/// `commands/ingest.rs`.
struct TauriIngestSink {
    app: AppHandle,
    task_id: String,
}

impl IngestSink for TauriIngestSink {
    fn progress(&self, event: IngestProgress) {
        let percent = event
            .processed
            .checked_mul(100)
            .and_then(|n| n.checked_div(event.total))
            .unwrap_or(0)
            .min(100) as u8;
        let status = if matches!(event.phase, IngestPhase::Done) {
            TaskStatus::Completed
        } else {
            TaskStatus::Running
        };
        // BRAIN-REPO-RAG-2b: every event also fills `log_line` so the
        // frontend's debug log can render a per-file scroll history with
        // explicit skip reasons (no silent skips).
        let log_line = match event.phase {
            IngestPhase::Skip => Some(format!(
                "skip[{}]: {}",
                event.skip_reason.unwrap_or("unknown"),
                event.message
            )),
            IngestPhase::Summary => Some(format!("summary: {}", event.message)),
            _ => {
                let phase = event.phase.as_str();
                if event.message.is_empty() {
                    Some(format!("{phase} ({}/{})", event.processed, event.total))
                } else {
                    Some(format!(
                        "{phase} ({}/{}): {}",
                        event.processed, event.total, event.message
                    ))
                }
            }
        };
        let payload = TaskProgressEvent {
            id: self.task_id.clone(),
            kind: TaskKind::Custom,
            status,
            progress: percent,
            description: format!("{}: {}", event.phase.as_str(), event.message),
            processed_items: event.processed as usize,
            total_items: event.total as usize,
            error: None,
            log_line,
        };
        let _ = self.app.emit("task-progress", payload);
    }
}

/// Tauri-facing request payload for `repo_add_source` and `repo_sync`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSyncRequest {
    pub source_id: String,
    pub repo_url: String,
    pub repo_ref: Option<String>,
    #[serde(default)]
    pub include_globs: Vec<String>,
    #[serde(default)]
    pub exclude_globs: Vec<String>,
    pub max_file_bytes: Option<u64>,
}

impl From<RepoSyncRequest> for RepoIngestOptions {
    fn from(r: RepoSyncRequest) -> Self {
        RepoIngestOptions {
            source_id: r.source_id,
            repo_url: r.repo_url,
            repo_ref: r.repo_ref,
            include_globs: r.include_globs,
            exclude_globs: r.exclude_globs,
            max_file_bytes: r.max_file_bytes,
        }
    }
}

/// Friendly response shape: full manifest is verbose, callers typically just
/// want the stats + head_commit + path for UI.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSyncResponse {
    pub source_id: String,
    pub head_commit: Option<String>,
    pub last_synced_at: u64,
    pub stats: RepoIngestStats,
    pub embed_stats: RepoEmbedStats,
    pub repo_path: PathBuf,
}

impl From<(RepoManifest, RepoEmbedStats, PathBuf)> for RepoSyncResponse {
    fn from((m, e, p): (RepoManifest, RepoEmbedStats, PathBuf)) -> Self {
        RepoSyncResponse {
            source_id: m.source_id,
            head_commit: m.head_commit,
            last_synced_at: m.last_synced_at,
            stats: m.stats,
            embed_stats: e,
            repo_path: p,
        }
    }
}

/// Create the `memory_sources` row (kind='repo') and run the first sync.
#[tauri::command]
pub async fn repo_add_source(
    request: RepoSyncRequest,
    label: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<RepoSyncResponse, String> {
    let data_dir = state.data_dir.clone();
    // Register the source row first so the UI sees it during ingest.
    {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        sources::create_source(
            store.conn(),
            &request.source_id,
            sources::MemorySourceKind::Repo,
            &label,
            Some(&request.repo_url),
            request.repo_ref.as_deref(),
        )
        .map_err(|e| e.to_string())?;
    }
    sync_inner(&data_dir, request, state, app).await
}

/// Re-clone an existing source and rebuild its `repo_chunks` rows.
#[tauri::command]
pub async fn repo_sync(
    request: RepoSyncRequest,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<RepoSyncResponse, String> {
    let data_dir = state.data_dir.clone();
    sync_inner(&data_dir, request, state, app).await
}

/// Delete the `<data_dir>/repos/<source_id>/` directory **and** the
/// corresponding `memory_sources` row. Idempotent.
#[tauri::command]
pub async fn repo_remove_source(
    source_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let data_dir = state.data_dir.clone();
    let removed_files = repo_ingest::remove_repo(&data_dir, &source_id).map_err(|e| e.to_string())?;
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let removed_row = sources::delete_source(store.conn(), &source_id).map_err(|e| e.to_string())?;
    Ok(removed_files || removed_row)
}

async fn sync_inner(
    data_dir: &std::path::Path,
    request: RepoSyncRequest,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<RepoSyncResponse, String> {
    let source_id = request.source_id.clone();
    let options: RepoIngestOptions = request.into();
    let data_dir_owned = data_dir.to_path_buf();
    let task_id = format!("repo-sync:{source_id}");

    // Snapshot brain config so the blocking task can drive embeddings
    // without holding the AppState mutex across awaits.
    let brain_mode = state.brain_mode.lock().ok().and_then(|g| g.clone());
    let active_brain = state.active_brain.lock().ok().and_then(|g| g.clone());

    let manifest_source_id = source_id.clone();
    let manifest_app = app.clone();
    let manifest_task_id = task_id.clone();
    let manifest = tokio::task::spawn_blocking(move || {
        let sink = TauriIngestSink {
            app: manifest_app,
            task_id: manifest_task_id,
        };
        repo_ingest::ingest_repo_with(&data_dir_owned, options, &sink)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(|e| e.to_string())?;

    // Embedding pass: each chunk calls `embed_for_mode` through the current
    // tokio runtime. Falls back to a no-op embedder when no brain is
    // configured so the ingest still completes successfully.
    let data_dir_owned = data_dir.to_path_buf();
    let embed_source_id = manifest_source_id.clone();
    let embed_app = app.clone();
    let embed_task_id = task_id.clone();
    let handle = tokio::runtime::Handle::current();
    let embed_stats = tokio::task::spawn_blocking(move || {
        let sink = TauriIngestSink {
            app: embed_app,
            task_id: embed_task_id,
        };
        let brain_mode_ref = brain_mode.as_ref();
        let active_brain_ref = active_brain.as_deref();
        let embed_fn = |text: &str| -> Option<Vec<f32>> {
            brain_mode_ref?;
            handle.block_on(embed_for_mode(text, brain_mode_ref, active_brain_ref))
        };
        repo_ingest::embed_repo_with_fn(
            &data_dir_owned,
            &embed_source_id,
            REPO_EMBED_DIMENSIONS,
            embed_fn,
            &sink,
        )
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(|e| e.to_string())?;

    {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        let _ = sources::touch_synced(store.conn(), &source_id);
    }
    let path = repo_ingest::repo_root(data_dir, &source_id);
    Ok((manifest, embed_stats, path).into())
}

// --- BRAIN-REPO-RAG-1c-a: source-scoped retrieval commands ---------------

/// Run a single-source hybrid search (vector + keyword + recency, RRF k=60)
/// against `<data_dir>/repos/<source_id>/memories.db`. The query is embedded
/// via [`embed_for_mode`] using the current brain mode; if no brain is
/// configured, the vector signal is skipped and only the keyword + recency
/// rankings contribute.
#[tauri::command]
pub async fn repo_search(
    source_id: String,
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::memory::repo_ingest::RepoSearchHit>, String> {
    repo_ingest::validate_source_id(&source_id).map_err(|e| e.to_string())?;
    let data_dir = state.data_dir.clone();
    let limit = limit.unwrap_or(10).max(1);

    let brain_mode = state.brain_mode.lock().ok().and_then(|g| g.clone());
    let active_brain = state.active_brain.lock().ok().and_then(|g| g.clone());

    // Best-effort query embedding.
    let query_emb = if brain_mode.is_some() {
        embed_for_mode(&query, brain_mode.as_ref(), active_brain.as_deref()).await
    } else {
        None
    };

    // Best-effort ANN search, scoped to the per-repo HNSW.
    let ann_data_dir = data_dir.clone();
    let ann_source_id = source_id.clone();
    let ann_emb = query_emb.clone();
    let ann_matches: Vec<(i64, f32)> = tokio::task::spawn_blocking(move || {
        let Some(emb) = ann_emb else {
            return Vec::new();
        };
        let root = repo_ingest::repo_root(&ann_data_dir, &ann_source_id);
        let Ok(index) = crate::memory::ann_index::AnnIndex::open(&root, emb.len()) else {
            return Vec::new();
        };
        index.search(&emb, (limit * 5).max(20)).unwrap_or_default()
    })
    .await
    .map_err(|e| format!("join error: {e}"))?;

    let search_data_dir = data_dir;
    let search_source_id = source_id;
    let query_owned = query;
    let hits = tokio::task::spawn_blocking(
        move || -> Result<Vec<crate::memory::repo_ingest::RepoSearchHit>, String> {
            let store = repo_ingest::RepoStore::open(
                &repo_ingest::db_path(&search_data_dir, &search_source_id),
                &search_source_id,
            )
            .map_err(|e| e.to_string())?;
            store
                .hybrid_search(&query_owned, query_emb.as_deref(), &ann_matches, limit)
                .map_err(|e| e.to_string())
        },
    )
    .await
    .map_err(|e| format!("join error: {e}"))??;
    Ok(hits)
}

/// Return distinct `file_path` values indexed for `source_id`, sorted.
#[tauri::command]
pub async fn repo_list_files(
    source_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    repo_ingest::validate_source_id(&source_id).map_err(|e| e.to_string())?;
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || -> Result<Vec<String>, String> {
        let store = repo_ingest::RepoStore::open(
            &repo_ingest::db_path(&data_dir, &source_id),
            &source_id,
        )
        .map_err(|e| e.to_string())?;
        store.list_files().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Read a file from the per-repo checkout. Prefers reading directly from
/// `<repo_root>/checkout/<file_path>` (accurate full content); falls back
/// to reassembling chunks from the per-repo SQLite when the checkout has
/// been removed. Rejects any path containing `..` or absolute components.
#[tauri::command]
pub async fn repo_read_file(
    source_id: String,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    repo_ingest::validate_source_id(&source_id).map_err(|e| e.to_string())?;
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || -> Result<String, String> {
        if file_path.is_empty()
            || file_path.contains("..")
            || file_path.starts_with('/')
            || file_path.starts_with('\\')
            || PathBuf::from(&file_path).is_absolute()
        {
            return Err("invalid file_path".to_string());
        }
        let checkout = repo_ingest::checkout_dir(&data_dir, &source_id);
        let full = checkout.join(&file_path);
        // Guard against symlink / weird relative components.
        match full.canonicalize() {
            Ok(canon) => {
                let canon_root = checkout.canonicalize().unwrap_or_else(|_| checkout.clone());
                if !canon.starts_with(&canon_root) {
                    return Err("path escapes checkout".to_string());
                }
                std::fs::read_to_string(&canon).map_err(|e| e.to_string())
            }
            Err(_) => {
                // Fallback: stitch from chunks.
                let store = repo_ingest::RepoStore::open(
                    &repo_ingest::db_path(&data_dir, &source_id),
                    &source_id,
                )
                .map_err(|e| e.to_string())?;
                store
                    .read_file(&file_path)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("file not found: {file_path}"))
            }
        }
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

// ─── BRAIN-REPO-RAG-1d: repo_map + repo_signatures ─────────────────────

/// Render an Aider-style repository map for a repo memory source, bounded by
/// a token budget. The map ranks files by tree-sitter chunk count (importance
/// proxy) and surfaces each file's top symbols with a short signature preview.
#[tauri::command]
pub async fn repo_map(
    source_id: String,
    budget_tokens: Option<usize>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    repo_ingest::validate_source_id(&source_id).map_err(|e| e.to_string())?;
    let data_dir = state.data_dir.clone();
    let budget = budget_tokens.unwrap_or(1024).clamp(64, 16_384);
    tokio::task::spawn_blocking(move || -> Result<String, String> {
        let store = repo_ingest::RepoStore::open(
            &repo_ingest::db_path(&data_dir, &source_id),
            &source_id,
        )
        .map_err(|e| e.to_string())?;
        store.build_repo_map(budget).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Tree-sitter signature-only preview for one file inside a repo source.
#[tauri::command]
pub async fn repo_signatures(
    source_id: String,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    repo_ingest::validate_source_id(&source_id).map_err(|e| e.to_string())?;
    if file_path.is_empty()
        || file_path.contains("..")
        || file_path.starts_with('/')
        || file_path.starts_with('\\')
        || PathBuf::from(&file_path).is_absolute()
    {
        return Err("invalid file_path".to_string());
    }
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || -> Result<String, String> {
        let store = repo_ingest::RepoStore::open(
            &repo_ingest::db_path(&data_dir, &source_id),
            &source_id,
        )
        .map_err(|e| e.to_string())?;
        store
            .build_file_signatures(&file_path)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

// ─── BRAIN-REPO-RAG-1e: OAuth device flow for private repos ────────────

use crate::coding::{
    self as coding_mod, DeviceCodeResponse, DevicePollResult, OAuthDeviceConfig,
};
use crate::memory::repo_oauth;

/// Public-facing OAuth status. Never includes the access token itself.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RepoOAuthStatus {
    pub linked: bool,
    pub token_type: String,
    pub scope: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub expired: bool,
}

impl RepoOAuthStatus {
    fn from_token(tok: Option<&repo_oauth::RepoOAuthToken>, now: u64) -> Self {
        match tok {
            Some(t) => Self {
                linked: !t.access_token.is_empty(),
                token_type: t.token_type.clone(),
                scope: t.scope.clone(),
                created_at: t.created_at,
                expires_at: t.expires_at,
                expired: t.is_expired(now),
            },
            None => Self {
                linked: false,
                token_type: String::new(),
                scope: String::new(),
                created_at: 0,
                expires_at: None,
                expired: false,
            },
        }
    }
}

/// Step 1: kick off the GitHub OAuth Device Flow. Returns the
/// `verification_uri` + `user_code` the frontend must surface to the
/// user. Reuses the existing self-improve scaffolding in
/// `crate::coding::github::request_device_code`.
#[tauri::command]
pub async fn repo_oauth_github_start(
    scopes: Option<String>,
) -> Result<DeviceCodeResponse, String> {
    let mut cfg = OAuthDeviceConfig::default();
    if let Some(s) = scopes.filter(|s| !s.trim().is_empty()) {
        cfg.scopes = s;
    }
    let client = reqwest::Client::new();
    coding_mod::request_device_code(&client, &cfg).await
}

/// Step 2: poll for the access token. On success, persists the token
/// to `<data_dir>/oauth/github.json` with FS-permission hardening (see
/// `crate::memory::repo_oauth::save_token`).
#[tauri::command]
pub async fn repo_oauth_github_poll(
    device_code: String,
    state: State<'_, AppState>,
) -> Result<DevicePollResult, String> {
    let client = reqwest::Client::new();
    let cfg = OAuthDeviceConfig::default();
    let result = coding_mod::poll_for_token(&client, &cfg, &device_code).await?;
    if let DevicePollResult::Success {
        access_token,
        token_type,
        scope,
    } = &result
    {
        let tok = repo_oauth::RepoOAuthToken::from_success(
            access_token.clone(),
            token_type.clone(),
            scope.clone(),
            None,
        );
        let data_dir = state.data_dir.clone();
        let tok_for_blocking = tok.clone();
        tokio::task::spawn_blocking(move || repo_oauth::save_token(&data_dir, &tok_for_blocking))
            .await
            .map_err(|e| format!("join error: {e}"))??;
    }
    Ok(result)
}

/// Returns the current binding status without ever exposing the token.
#[tauri::command]
pub async fn repo_oauth_github_status(
    state: State<'_, AppState>,
) -> Result<RepoOAuthStatus, String> {
    let data_dir = state.data_dir.clone();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let tok = tokio::task::spawn_blocking(move || repo_oauth::load_token(&data_dir))
        .await
        .map_err(|e| format!("join error: {e}"))?;
    Ok(RepoOAuthStatus::from_token(tok.as_ref(), now))
}

/// Drop the persisted token so subsequent clones fall back to anonymous.
#[tauri::command]
pub async fn repo_oauth_github_clear(state: State<'_, AppState>) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || repo_oauth::clear_token(&data_dir))
        .await
        .map_err(|e| format!("join error: {e}"))?
}


// ─── BRAIN-REPO-RAG-2a: cross-source knowledge-graph projection ─────────

/// Graph-ready node carrying source provenance. `graph_id` is a stable
/// unique numeric id usable as a d3-force node id: self memories use their
/// positive `memory.id` directly, repo chunks are assigned negative ids
/// during the projection so they never collide with self ids and stay
/// within JS 2^53-safe range.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossSourceGraphNode {
    pub graph_id: i64,
    pub source_id: String,
    pub source_label: String,
    pub local_id: i64,
    pub content: String,
    pub file_path: Option<String>,
    pub parent_symbol: Option<String>,
    pub created_at: i64,
}

/// Bundle returned by `cross_source_graph_nodes`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossSourceGraph {
    pub nodes: Vec<CrossSourceGraphNode>,
    /// `(source_id, label, count)` tuples for the legend / source filter.
    pub per_source_counts: Vec<(String, String, usize)>,
}

/// BRAIN-REPO-RAG-2a: unified node list for the cross-source knowledge
/// graph. Walks every registered `memory_sources` row and projects up
/// to `per_source_limit` recent chunks per repo into the same wire
/// shape used by self memories. The personal brain is fed by the
/// caller (the frontend already has the `MemoryEntry` list); this
/// command only returns the per-repo projections plus a registry of
/// source labels so the graph legend can render the `📦 owner/repo`
/// badges.
#[tauri::command]
pub async fn cross_source_graph_nodes(
    per_source_limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<CrossSourceGraph, String> {
    let limit = per_source_limit.unwrap_or(64).clamp(1, 512);
    let data_dir = state.data_dir.clone();
    // List repo sources synchronously from the main brain's
    // `memory_sources` table; the lock is short-lived so this is safe
    // outside `spawn_blocking`.
    let sources_rows = {
        let store = state
            .memory_store
            .lock()
            .map_err(|e| format!("memory store lock: {e}"))?;
        crate::memory::sources::list_sources(store.conn())
            .map_err(|e| format!("list_sources failed: {e}"))?
    };
    tokio::task::spawn_blocking(move || -> Result<CrossSourceGraph, String> {
        let mut nodes: Vec<CrossSourceGraphNode> = Vec::new();
        let mut per_source_counts: Vec<(String, String, usize)> = Vec::new();
        // Stable, collision-free negative id space: we hand out ids
        // `next_neg = -1, -2, -3, ...` as we walk repos, so any positive
        // self memory.id can never clash.
        let mut next_neg: i64 = -1;

        for src in sources_rows {
            // Skip the personal source — the frontend already owns those.
            if src.id == "self" {
                continue;
            }
            let db_file = repo_ingest::db_path(&data_dir, &src.id);
            if !db_file.exists() {
                per_source_counts.push((src.id.clone(), src.label.clone(), 0));
                continue;
            }
            let store = match repo_ingest::RepoStore::open(&db_file, &src.id) {
                Ok(s) => s,
                Err(_) => {
                    per_source_counts.push((src.id.clone(), src.label.clone(), 0));
                    continue;
                }
            };
            let chunks = store.recent_chunks(limit).map_err(|e| e.to_string())?;
            let count = chunks.len();
            for c in chunks {
                let graph_id = next_neg;
                next_neg = next_neg.saturating_sub(1);
                nodes.push(CrossSourceGraphNode {
                    graph_id,
                    source_id: src.id.clone(),
                    source_label: src.label.clone(),
                    local_id: c.id,
                    content: c.content,
                    file_path: Some(c.file_path),
                    parent_symbol: c.parent_symbol,
                    created_at: c.created_at,
                });
            }
            per_source_counts.push((src.id, src.label, count));
        }

        Ok(CrossSourceGraph {
            nodes,
            per_source_counts,
        })
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}
