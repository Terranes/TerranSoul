use tauri::{AppHandle, Emitter, Manager, State};

use crate::brain::{
    self, BrainMode, FreeProvider, IntentDecision, ModelRecommendation, OllamaStatus, SystemInfo,
};
use crate::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderHealthInfo {
    pub id: String,
    pub display_name: String,
    pub is_healthy: bool,
    pub is_rate_limited: bool,
    pub requests_sent: u64,
    pub remaining_requests: Option<u64>,
    pub remaining_tokens: Option<u64>,
    pub latency_ms: Option<u64>,
}

/// Return hardware information about the host machine.
#[tauri::command]
pub async fn get_system_info() -> SystemInfo {
    brain::collect_system_info()
}

/// Return the path where Ollama stores downloaded models.
#[tauri::command]
pub async fn get_ollama_models_dir() -> String {
    brain::ollama_models_dir()
}

/// Return disk space information for the drive containing the given path.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_disk_space(path: String) -> Result<brain::DiskInfo, String> {
    brain::disk_info_for_path(&path).ok_or_else(|| format!("No disk found for path: {path}"))
}

/// List all mounted drives / partitions with available and total space.
#[tauri::command]
pub async fn list_drives() -> Vec<brain::DiskInfo> {
    brain::list_drives()
}

/// Read a bundled documentation file by relative path (e.g. `"docs/brain-advanced-design.md"`).
///
/// Returns the file contents as a string. This allows the frontend to
/// display shipped documentation without a network request.
#[tauri::command(rename_all = "camelCase")]
pub async fn read_bundled_doc(app: AppHandle, relative_path: String) -> Result<String, String> {
    // Sanitise: reject path traversal.
    if relative_path.contains("..") {
        return Err("Path traversal not allowed".to_string());
    }
    let resource_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
    let full_path = resource_dir.join(&relative_path);
    std::fs::read_to_string(&full_path).map_err(|e| format!("Cannot read {relative_path}: {e}"))
}

/// Return a ranked list of model recommendations based on available RAM.
///
/// Resolution order:
/// 1. Cached online catalogue (previously fetched via `refresh_model_catalogue`)
/// 2. Bundled `docs/brain-advanced-design.md` (§26)
/// 3. Hardcoded fallback in `model_recommender.rs`
#[tauri::command]
pub async fn recommend_brain_models(app: AppHandle) -> Vec<ModelRecommendation> {
    let info = brain::collect_system_info();

    // 1. Try cached online catalogue (freshest data).
    if let Ok(cache_dir) = app.path().app_cache_dir() {
        if let Some(catalogue) = brain::load_cached_catalogue(&cache_dir) {
            return brain::recommend_from_catalogue(info.total_ram_mb, &catalogue);
        }
    }

    // 2. Try the bundled design doc (works in production builds).
    if let Ok(resource_dir) = app.path().resource_dir() {
        let doc_path = resource_dir.join("docs").join("brain-advanced-design.md");
        if let Ok(markdown) = std::fs::read_to_string(&doc_path) {
            if let Some(catalogue) = brain::parse_catalogue(&markdown) {
                return brain::recommend_from_catalogue(info.total_ram_mb, &catalogue);
            }
        }
    }

    // 2b. Dev fallback: check workspace root (resource_dir points to target/
    //     during `cargo tauri dev`, but docs/ lives at the project root).
    {
        let dev_doc = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("docs")
            .join("brain-advanced-design.md");
        if let Ok(markdown) = std::fs::read_to_string(&dev_doc) {
            if let Some(catalogue) = brain::parse_catalogue(&markdown) {
                return brain::recommend_from_catalogue(info.total_ram_mb, &catalogue);
            }
        }
    }

    // 3. Hardcoded fallback.
    brain::recommend(info.total_ram_mb)
}

/// Fetch the latest model catalogue from the upstream repository.
///
/// The catalogue is cached locally so subsequent `recommend_brain_models`
/// calls use the fresh data without another network request.
/// Returns the number of models in the refreshed catalogue.
#[tauri::command]
pub async fn refresh_model_catalogue(app: AppHandle) -> Result<usize, String> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("cache dir unavailable: {e}"))?;

    let catalogue = brain::fetch_online_catalogue(&cache_dir).await?;
    let count = catalogue.local_models.len() + catalogue.cloud_models.len();
    Ok(count)
}

/// Check whether the local Ollama service is running.
#[tauri::command]
pub async fn check_ollama_status(state: State<'_, AppState>) -> Result<OllamaStatus, String> {
    let client = &state.ollama_client;
    Ok(brain::check_status(client, brain::ollama_agent::OLLAMA_BASE_URL).await)
}

/// Detect Ollama installation status (binary on disk + service responding).
#[tauri::command]
pub async fn detect_ollama_install() -> brain::ollama_lifecycle::OllamaInstallStatus {
    brain::ollama_lifecycle::detect_ollama().await
}

/// Try to start the Ollama service if it's installed but not running.
/// Polls for up to `timeout_secs` (default 15) for the API to respond.
#[tauri::command(rename_all = "camelCase")]
pub async fn start_ollama_service(timeout_secs: Option<u64>) -> Result<bool, String> {
    brain::ollama_lifecycle::start_ollama(timeout_secs.unwrap_or(15)).await
}

/// Download and install Ollama from the official site.
/// Emits `ollama-install-progress` events with `{ phase: string, percent: u32 }`.
#[tauri::command]
pub async fn install_ollama(app: AppHandle) -> Result<String, String> {
    let app_for_progress = app.clone();
    brain::ollama_lifecycle::install_ollama(move |phase, percent| {
        let _ = app_for_progress.emit(
            "ollama-install-progress",
            serde_json::json!({ "phase": phase, "percent": percent }),
        );
    })
    .await
}

/// List all Ollama models installed on this machine.
#[tauri::command]
pub async fn get_ollama_models(
    state: State<'_, AppState>,
) -> Result<Vec<brain::ollama_agent::OllamaModelEntry>, String> {
    let client = &state.ollama_client;
    Ok(brain::list_models(client, brain::ollama_agent::OLLAMA_BASE_URL).await)
}

/// Pull an Ollama model from the registry (downloads it locally).
/// Emits `ollama-pull-progress` events with live download percentage.
#[tauri::command(rename_all = "camelCase")]
pub async fn pull_ollama_model(
    app: AppHandle,
    model_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let client = &state.ollama_client;

    let app_handle = app.clone();
    brain::pull_model_with_progress(
        client,
        brain::ollama_agent::OLLAMA_BASE_URL,
        &model_name,
        move |progress| {
            let _ = app_handle.emit("ollama-pull-progress", &progress);
        },
    )
    .await?;

    // Track the pulled model so factory reset can remove it.
    {
        let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        let tag = format!("ollama_model:{model_name}");
        settings.track_auto_configured(&tag);
        crate::settings::config_store::save(&state.data_dir, &settings)?;
    }

    Ok(())
}

/// Check whether the local LM Studio service is running.
#[tauri::command(rename_all = "camelCase")]
pub async fn check_lm_studio_status(
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<brain::LmStudioStatus, String> {
    Ok(brain::lm_studio::check_status(base_url.as_deref(), api_key.as_deref()).await)
}

/// List all LM Studio models available to the local server.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_lm_studio_models(
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<Vec<brain::LmStudioModelEntry>, String> {
    brain::lm_studio::list_models(base_url.as_deref(), api_key.as_deref()).await
}

/// Download a model through LM Studio's native runtime API.
#[tauri::command(rename_all = "camelCase")]
pub async fn download_lm_studio_model(
    model: String,
    base_url: Option<String>,
    api_key: Option<String>,
    quantization: Option<String>,
) -> Result<brain::LmStudioDownloadStatus, String> {
    brain::lm_studio::download_model(
        base_url.as_deref(),
        api_key.as_deref(),
        &model,
        quantization.as_deref(),
    )
    .await
}

/// Get download progress for a previously started LM Studio download.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_lm_studio_download_status(
    job_id: String,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<brain::LmStudioDownloadStatus, String> {
    brain::lm_studio::download_status(base_url.as_deref(), api_key.as_deref(), &job_id).await
}

/// Explicitly load a downloaded LM Studio model into memory.
#[tauri::command(rename_all = "camelCase")]
pub async fn load_lm_studio_model(
    model: String,
    base_url: Option<String>,
    api_key: Option<String>,
    context_length: Option<u32>,
) -> Result<brain::LmStudioLoadResult, String> {
    brain::lm_studio::load_model(
        base_url.as_deref(),
        api_key.as_deref(),
        &model,
        context_length,
    )
    .await
}

/// Unload one LM Studio model instance by instance id.
#[tauri::command(rename_all = "camelCase")]
pub async fn unload_lm_studio_model(
    instance_id: String,
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<brain::LmStudioUnloadResult, String> {
    brain::lm_studio::unload_model(base_url.as_deref(), api_key.as_deref(), &instance_id).await
}

/// Set the active brain model. Persists the choice to disk.
/// After calling this, subsequent chat messages will be routed through Ollama.
#[tauri::command(rename_all = "camelCase")]
pub async fn set_active_brain(
    model_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    brain::save_brain(&state.data_dir, &model_name)?;
    let mut brain = state.active_brain.lock().map_err(|e| e.to_string())?;
    *brain = Some(model_name);
    Ok(())
}

/// Return the name of the currently active brain model, or null if none is set.
#[tauri::command]
pub async fn get_active_brain(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let brain = state.active_brain.lock().map_err(|e| e.to_string())?;
    Ok(brain.clone())
}

/// Clear the active brain, reverting chat to the built-in stub agent.
#[tauri::command]
pub async fn clear_active_brain(state: State<'_, AppState>) -> Result<(), String> {
    brain::clear_brain(&state.data_dir)?;
    let mut brain = state.active_brain.lock().map_err(|e| e.to_string())?;
    *brain = None;
    Ok(())
}

// ── Three-Tier Brain Commands ────────────────────────────────────────────────

/// Return the curated list of free LLM API providers.
#[tauri::command]
pub async fn list_free_providers() -> Vec<FreeProvider> {
    brain::free_provider_catalogue()
}

/// Return the current brain mode configuration.
#[tauri::command]
pub async fn get_brain_mode(state: State<'_, AppState>) -> Result<Option<BrainMode>, String> {
    let mode = state.brain_mode.lock().map_err(|e| e.to_string())?;
    Ok(mode.clone())
}

/// Set the brain mode (free API, paid API, or local Ollama).
/// Persists to disk and updates in-memory state.
#[tauri::command]
pub async fn set_brain_mode(mode: BrainMode, state: State<'_, AppState>) -> Result<(), String> {
    brain::brain_config::save(&state.data_dir, &mode)?;

    // Also update the legacy active_brain field for backwards compatibility
    // with existing streaming/chat code
    match &mode {
        BrainMode::LocalOllama { model } => {
            let mut brain = state.active_brain.lock().map_err(|e| e.to_string())?;
            *brain = Some(model.clone());
        }
        BrainMode::LocalLmStudio { .. } => {
            let mut brain = state.active_brain.lock().map_err(|e| e.to_string())?;
            *brain = None;
        }
        _ => {
            // For free/paid API modes, clear the Ollama active brain
            // since streaming will route through the OpenAI client
            let mut brain = state.active_brain.lock().map_err(|e| e.to_string())?;
            *brain = None;
        }
    }

    {
        let mut brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?;
        *brain_mode = Some(mode);
    }

    // The chosen embedding model and any "model X doesn't support
    // embeddings" memo from `brain::ollama_agent` are tied to whichever
    // brain we were just using. Reset them so the next /api/embed call
    // probes /api/tags again instead of carrying a stale "unsupported"
    // verdict across a brain switch.
    crate::brain::ollama_agent::clear_embed_caches().await;
    crate::brain::cloud_embeddings::clear_cloud_embed_cache().await;
    // Mode change may pick a different model with different classification
    // behaviour — drop cached intent decisions so the next turn re-asks.
    crate::brain::intent_classifier::clear_cache();

    Ok(())
}

/// Diagnostic command — return the current state of the embedding cache
/// (chosen model, age, list of models that returned 501/400 from
/// `/api/embed`). Useful for the brain debugging panel.
#[tauri::command]
pub async fn get_embed_cache_status() -> brain::ollama_agent::EmbedCacheSnapshot {
    brain::ollama_agent::embed_cache_snapshot().await
}

/// Diagnostic command — clear the embedding caches, forcing the next
/// `/api/embed` call to re-probe `/api/tags`. The frontend should call
/// this after the user installs `nomic-embed-text` so vector RAG comes
/// back online without a restart.
#[tauri::command]
pub async fn reset_embed_cache() -> Result<(), String> {
    brain::ollama_agent::clear_embed_caches().await;
    crate::brain::cloud_embeddings::clear_cloud_embed_cache().await;
    Ok(())
}

// ── Provider Rotator Commands ────────────────────────────────────────────────

/// Return health and rate-limit status for all free API providers.
#[tauri::command]
pub async fn health_check_providers(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderHealthInfo>, String> {
    let rotator = state.provider_rotator.lock().map_err(|e| e.to_string())?;
    Ok(rotator
        .providers
        .values()
        .map(|s| ProviderHealthInfo {
            id: s.provider.id.clone(),
            display_name: s.provider.display_name.clone(),
            is_healthy: s.is_healthy,
            is_rate_limited: s.is_rate_limited,
            requests_sent: s.requests_sent,
            remaining_requests: s.remaining_requests,
            remaining_tokens: s.remaining_tokens,
            latency_ms: s.latency.map(|d| d.as_millis() as u64),
        })
        .collect())
}

/// Return the next healthy, non-rate-limited provider id (fastest first).
#[tauri::command]
pub async fn get_next_provider(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let mut rotator = state.provider_rotator.lock().map_err(|e| e.to_string())?;
    Ok(rotator.next_healthy_provider().map(|p| p.id.clone()))
}

// ── Brain Selection Snapshot ────────────────────────────────────────────────

/// Return a typed snapshot of every active brain selection (provider, embedding,
/// memory, search, storage, agents, RAG quality).
///
/// This is the operational answer to the question
/// *"how does the LLM know which component to use?"* surfaced to the
/// Brain hub UI ("Active selection" panel). See
/// `docs/brain-advanced-design.md` § 20 for the full decision matrix.
#[tauri::command]
pub async fn get_brain_selection(
    state: State<'_, AppState>,
) -> Result<brain::BrainSelection, String> {
    // (1) Provider — read brain_mode + legacy fallback + ask the rotator
    // who it would currently pick (without committing to a request).
    let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?.clone();
    let legacy_active_brain = state
        .active_brain
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let rotator_pick: Option<(String, bool)> = {
        let mut rotator = state.provider_rotator.lock().map_err(|e| e.to_string())?;
        rotator
            .next_healthy_provider()
            .map(|p| (p.id.clone(), true))
    };

    // (2) Embedding — only meaningful in Local Ollama mode. Use the
    // resolver cache (filled by previous /api/embed calls); when nothing
    // is cached we conservatively report "unavailable" rather than probe
    // synchronously here.
    let embed_snapshot = brain::ollama_agent::embed_cache_snapshot().await;
    let embedding_available = embed_snapshot.chosen_model.is_some();
    let embedding_preferred_model = embed_snapshot
        .chosen_model
        .clone()
        .unwrap_or_else(|| "nomic-embed-text".to_string());

    // (3) Memory — read live SQLite stats.
    let memory_snapshot = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        let stats = store.stats().map_err(|e| e.to_string())?;
        brain::MemorySelection {
            total: stats.total,
            short_count: stats.short,
            working_count: stats.working,
            long_count: stats.long,
            embedded_count: stats.embedded,
            // Schema is migrated through V5 (see memory/migrations.rs).
            schema_version: 5,
        }
    };

    // (4) Storage backend — derived from compile-time features. SQLite is
    // always the runtime default in the shipped binary; alternative
    // backends (postgres / mssql / cassandra) are gated behind cargo
    // features and selected via StorageConfig at startup.
    let storage_snapshot = brain::StorageSelection {
        backend: "sqlite".to_string(),
        is_local: true,
        schema_label: "V6 — memory_edges + temporal validity".to_string(),
    };

    // (5) Agents — read the orchestrator roster. The default routing
    // target ("auto" → default_agent_id) is currently hardcoded to
    // "stub" inside AgentOrchestrator::new; we surface the same fact
    // here so the UI explains it.
    let agents_snapshot = brain::AgentSelection {
        registered: vec!["stub".to_string()],
        default_agent_id: "stub".to_string(),
    };

    let rotator_pick_ref = rotator_pick.as_ref().map(|(id, h)| (id.as_str(), *h));

    Ok(brain::BrainSelection::from_parts(
        brain_mode.as_ref(),
        legacy_active_brain.as_deref(),
        rotator_pick_ref,
        embedding_available,
        &embedding_preferred_model,
        memory_snapshot,
        storage_snapshot,
        agents_snapshot,
    ))
}

// ── Intent Classification ────────────────────────────────────────────────────

/// Classify a user message into a structured intent decision via the
/// configured brain (Free → Paid → Local Ollama → Local LM Studio).
///
/// Replaces three brittle regex detectors that used to live in
/// `src/stores/conversation.ts`. See the `brain::intent_classifier`
/// module docs for the full rationale and JSON schema.
///
/// Returns `IntentDecision::Unknown` when no brain is configured, the
/// classifier times out, or the LLM emits malformed JSON. The frontend
/// is expected to fall back to the install-all path on `Unknown` so
/// future turns have a working classifier.
#[tauri::command]
pub async fn classify_intent(
    text: String,
    state: State<'_, AppState>,
) -> Result<IntentDecision, String> {
    let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?.clone();
    let decision =
        brain::classify_user_intent(&text, brain_mode.as_ref(), &state.provider_rotator).await;
    Ok(decision)
}

/// Factory-reset the brain: undo auto-configured components (brain, voice,
/// quests, Docker containers, Ollama models, MCP configs) based on the
/// `auto_configured` list in AppSettings, clear all memories, and revert to
/// the first-launch state.
/// This is irreversible — the frontend must confirm with the user.
#[tauri::command]
pub async fn factory_reset_brain(state: State<'_, AppState>) -> Result<(), String> {
    // Read which components were auto-configured.
    let auto_configured: Vec<String> = {
        let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        settings.auto_configured.clone()
    };

    // 1. Clear brain config (only if auto-configured by TerranSoul).
    if auto_configured.contains(&"brain".to_string()) {
        brain::brain_config::clear(&state.data_dir)?;
        brain::clear_brain(&state.data_dir)?;
        {
            let mut mode = state.brain_mode.lock().map_err(|e| e.to_string())?;
            *mode = None;
        }
        {
            let mut ab = state.active_brain.lock().map_err(|e| e.to_string())?;
            *ab = None;
        }
    }

    // 2. Clear voice config (if auto-configured).
    if auto_configured.contains(&"voice".to_string()) {
        crate::voice::config_store::clear(&state.data_dir)?;
        let mut vc = state.voice_config.lock().map_err(|e| e.to_string())?;
        *vc = crate::voice::VoiceConfig::default();
    }

    // 3. Clear quest tracker (if auto-configured).
    if auto_configured.contains(&"quests".to_string()) {
        let path = state.data_dir.join("quest_tracker.json");
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("clear quest tracker: {e}"))?;
        }
    }

    // 4. Remove Docker container + volume (if auto-configured).
    if auto_configured.contains(&"docker_container".to_string()) {
        let preference = {
            let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
            settings.preferred_container_runtime
        };
        // Best-effort: resolve runtime and remove container. Ignore errors
        // (Docker may not be running or container may already be removed).
        if let Ok(runtime) = crate::container::resolve_runtime(preference).await {
            let _ = brain::docker_ollama::remove_ollama_container_for(runtime).await;
        }
    }

    // 5. Delete auto-pulled Ollama models (best-effort, requires running Ollama).
    {
        let models_to_remove: Vec<String> = auto_configured
            .iter()
            .filter_map(|tag| tag.strip_prefix("ollama_model:").map(|m| m.to_string()))
            .collect();
        if !models_to_remove.is_empty() {
            let client = &state.ollama_client;
            for model in &models_to_remove {
                let _ =
                    brain::delete_model(client, brain::ollama_agent::OLLAMA_BASE_URL, model).await;
            }
        }
    }

    // 6. Remove MCP config entries from external tools (best-effort).
    if auto_configured.contains(&"mcp_vscode".to_string()) {
        // VS Code config is workspace-relative; remove from the running directory.
        if let Ok(cwd) = std::env::current_dir() {
            let _ = crate::ai_integrations::mcp::auto_setup::remove_vscode_config(&cwd);
        }
    }
    if auto_configured.contains(&"mcp_claude".to_string()) {
        let _ = crate::ai_integrations::mcp::auto_setup::remove_claude_config();
    }
    if auto_configured.contains(&"mcp_codex".to_string()) {
        let _ = crate::ai_integrations::mcp::auto_setup::remove_codex_config();
    }

    // 7. Clear ALL memories.
    {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store.delete_all().map_err(|e| e.to_string())?;
    }

    // 8. Clear embedding + intent caches.
    brain::ollama_agent::clear_embed_caches().await;
    crate::brain::cloud_embeddings::clear_cloud_embed_cache().await;
    brain::intent_classifier::clear_cache();

    // 9. Reset provider rotator health tracking.
    {
        let mut rotator = state.provider_rotator.lock().map_err(|e| e.to_string())?;
        *rotator = brain::ProviderRotator::new();
    }

    // 10. Reset conversation history.
    {
        let mut conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.clear();
    }

    // 11. Reset first_launch_complete and auto_configured in settings.
    {
        let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        settings.first_launch_complete = false;
        settings.auto_configured.clear();
        crate::settings::config_store::save(&state.data_dir, &settings)?;
    }

    Ok(())
}

// ── Model update check + auto-cleanup ──────────────────────────────────────

/// Result returned by `check_model_updates`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelUpdateInfo {
    pub has_update: bool,
    pub current_model: String,
    pub recommended_model: String,
    pub recommended_display: String,
}

/// Compare the currently active local model against the online catalogue's
/// top-pick for this hardware. Returns whether a better model is available
/// and what it is. Does NOT check the dismissed list — the frontend handles
/// that so it can persist dismissals without extra IPC.
#[tauri::command]
pub async fn check_model_updates(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ModelUpdateInfo, String> {
    // Refresh catalogue (best-effort — falls back to cached/bundled).
    let _ = brain::fetch_online_catalogue(&app.path().app_cache_dir().map_err(|e| e.to_string())?)
        .await;

    let info = brain::collect_system_info();

    // Resolve recommendations (same 3-tier as recommend_brain_models).
    let recs = {
        if let Ok(cache_dir) = app.path().app_cache_dir() {
            if let Some(cat) = brain::load_cached_catalogue(&cache_dir) {
                brain::recommend_from_catalogue(info.total_ram_mb, &cat)
            } else {
                brain::recommend(info.total_ram_mb)
            }
        } else {
            brain::recommend(info.total_ram_mb)
        }
    };

    let top = recs.iter().find(|r| r.is_top_pick && !r.is_cloud);

    let current_model = {
        let mode = state.brain_mode.lock().map_err(|e| e.to_string())?;
        match mode.as_ref() {
            Some(brain::BrainMode::LocalOllama { model }) => model.clone(),
            _ => String::new(),
        }
    };

    let (recommended_model, recommended_display) = match top {
        Some(r) => (r.model_tag.clone(), r.display_name.clone()),
        None => (String::new(), String::new()),
    };

    let has_update = !recommended_model.is_empty()
        && !current_model.is_empty()
        && recommended_model != current_model;

    Ok(ModelUpdateInfo {
        has_update,
        current_model,
        recommended_model,
        recommended_display,
    })
}

/// Delete an Ollama model by tag (e.g. `"gemma3:4b"`). Also removes the
/// corresponding `ollama_model:*` entry from `auto_configured` in settings.
#[tauri::command]
pub async fn delete_ollama_model(
    model_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let client = &state.ollama_client;
    brain::delete_model(client, brain::ollama_agent::OLLAMA_BASE_URL, &model_name).await?;

    // Remove from auto_configured tracking.
    {
        let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        let tag = format!("ollama_model:{model_name}");
        settings.auto_configured.retain(|t| t != &tag);
        crate::settings::config_store::save(&state.data_dir, &settings)?;
    }

    Ok(())
}

/// Persist a dismissed model update tag so the upgrade quest is not shown again.
#[tauri::command]
pub async fn dismiss_model_update(
    model_tag: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    if !settings.dismissed_model_updates.contains(&model_tag) {
        settings.dismissed_model_updates.push(model_tag);
    }
    crate::settings::config_store::save(&state.data_dir, &settings)?;
    Ok(())
}

/// Update the `last_update_check_date` in settings. The frontend sends
/// today's ISO date string (`YYYY-MM-DD`) so we don't need a `chrono` dep.
#[tauri::command]
pub async fn mark_update_check_done(
    date: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    settings.last_update_check_date = date;
    crate::settings::config_store::save(&state.data_dir, &settings)?;
    Ok(())
}

/// Return the last update check date and the dismissed model list so the
/// frontend can decide whether to run a check without extra round-trips.
#[tauri::command]
pub async fn get_update_check_state(
    state: State<'_, AppState>,
) -> Result<(String, Vec<String>), String> {
    let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    Ok((
        settings.last_update_check_date.clone(),
        settings.dismissed_model_updates.clone(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_system_info_returns_valid_info() {
        let info = get_system_info().await;
        assert!(info.total_ram_mb > 0);
        assert!(info.cpu_cores > 0);
    }

    #[tokio::test]
    async fn recommend_brain_models_returns_at_least_one() {
        // Call the underlying function directly (AppHandle unavailable in unit tests).
        let info = brain::collect_system_info();
        let recs = brain::recommend(info.total_ram_mb);
        assert!(!recs.is_empty());
    }

    #[tokio::test]
    async fn recommend_brain_models_has_exactly_one_top_pick() {
        let info = brain::collect_system_info();
        let recs = brain::recommend(info.total_ram_mb);
        let top = recs.iter().filter(|m| m.is_top_pick).count();
        assert_eq!(top, 1);
    }

    #[tokio::test]
    async fn list_free_providers_not_empty() {
        let providers = list_free_providers().await;
        assert!(!providers.is_empty());
        assert!(providers.iter().any(|p| p.id == "groq"));
    }

    #[tokio::test]
    async fn list_free_providers_all_have_https() {
        let providers = list_free_providers().await;
        for p in &providers {
            assert!(
                p.base_url.starts_with("https://"),
                "{} base_url should be HTTPS",
                p.id
            );
        }
    }
}
