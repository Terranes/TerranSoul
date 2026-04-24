use tauri::State;

use crate::brain::{
    self, ModelRecommendation, OllamaStatus, SystemInfo,
    BrainMode, FreeProvider,
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

/// Return a ranked list of model recommendations based on available RAM.
#[tauri::command]
pub async fn recommend_brain_models() -> Vec<ModelRecommendation> {
    let info = brain::collect_system_info();
    brain::recommend(info.total_ram_mb)
}

/// Check whether the local Ollama service is running.
#[tauri::command]
pub async fn check_ollama_status(state: State<'_, AppState>) -> Result<OllamaStatus, String> {
    let client = &state.ollama_client;
    Ok(brain::check_status(client, brain::ollama_agent::OLLAMA_BASE_URL).await)
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
/// This may take several minutes for large models.
#[tauri::command(rename_all = "camelCase")]
pub async fn pull_ollama_model(
    model_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let client = &state.ollama_client;
    brain::pull_model(client, brain::ollama_agent::OLLAMA_BASE_URL, &model_name).await
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
pub async fn set_brain_mode(
    mode: BrainMode,
    state: State<'_, AppState>,
) -> Result<(), String> {
    brain::brain_config::save(&state.data_dir, &mode)?;

    // Also update the legacy active_brain field for backwards compatibility
    // with existing streaming/chat code
    match &mode {
        BrainMode::LocalOllama { model } => {
            let mut brain = state.active_brain.lock().map_err(|e| e.to_string())?;
            *brain = Some(model.clone());
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
    Ok(())
}

// ── Provider Rotator Commands ────────────────────────────────────────────────

/// Return health and rate-limit status for all free API providers.
#[tauri::command]
pub async fn health_check_providers(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderHealthInfo>, String> {
    let rotator = state.provider_rotator.lock().map_err(|e| e.to_string())?;
    Ok(rotator.providers.values().map(|s| ProviderHealthInfo {
        id: s.provider.id.clone(),
        display_name: s.provider.display_name.clone(),
        is_healthy: s.is_healthy,
        is_rate_limited: s.is_rate_limited,
        requests_sent: s.requests_sent,
        remaining_requests: s.remaining_requests,
        remaining_tokens: s.remaining_tokens,
        latency_ms: s.latency.map(|d| d.as_millis() as u64),
    }).collect())
}

/// Return the next healthy, non-rate-limited provider id (fastest first).
#[tauri::command]
pub async fn get_next_provider(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
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
    let legacy_active_brain = state.active_brain.lock().map_err(|e| e.to_string())?.clone();

    let rotator_pick: Option<(String, bool)> = {
        let mut rotator = state.provider_rotator.lock().map_err(|e| e.to_string())?;
        rotator.next_healthy_provider().map(|p| (p.id.clone(), true))
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
        let recs = recommend_brain_models().await;
        assert!(!recs.is_empty());
    }

    #[tokio::test]
    async fn recommend_brain_models_has_exactly_one_top_pick() {
        let recs = recommend_brain_models().await;
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
