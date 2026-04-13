use tauri::State;

use crate::brain::{
    self, ModelRecommendation, OllamaStatus, SystemInfo,
    BrainMode, FreeProvider,
};
use crate::AppState;

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
#[tauri::command]
pub async fn pull_ollama_model(
    model_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let client = &state.ollama_client;
    brain::pull_model(client, brain::ollama_agent::OLLAMA_BASE_URL, &model_name).await
}

/// Set the active brain model. Persists the choice to disk.
/// After calling this, subsequent chat messages will be routed through Ollama.
#[tauri::command]
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

    let mut brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?;
    *brain_mode = Some(mode);
    Ok(())
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
