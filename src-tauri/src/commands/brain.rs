use tauri::State;

use crate::brain::{
    self, ModelRecommendation, OllamaStatus, SystemInfo,
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
}
