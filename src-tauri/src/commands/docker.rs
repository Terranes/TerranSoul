use tauri::State;

use crate::brain::docker_ollama;
use crate::container::{self, RuntimeDetection, RuntimePreference};
use crate::AppState;

/// Check Docker CLI and daemon status.
#[tauri::command]
pub async fn check_docker_status() -> docker_ollama::DockerStatus {
    docker_ollama::check_docker_status().await
}

/// Detect both Docker and Podman in one call. The frontend uses this to
/// show a runtime picker the first time the user starts the local-LLM
/// quest (or whenever both runtimes are available).
#[tauri::command]
pub async fn detect_container_runtimes() -> RuntimeDetection {
    container::detect_runtimes().await
}

/// Read the persisted container-runtime preference.
#[tauri::command]
pub async fn get_runtime_preference(
    state: State<'_, AppState>,
) -> Result<RuntimePreference, String> {
    let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.preferred_container_runtime)
}

/// Persist the user's container-runtime preference (Auto / Docker / Podman).
/// Used by the local-LLM quest UI when the user explicitly picks a runtime.
#[tauri::command(rename_all = "camelCase")]
pub async fn set_runtime_preference(
    preference: RuntimePreference,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    settings.preferred_container_runtime = preference;
    crate::settings::config_store::save(&state.data_dir, &settings)?;
    Ok(())
}

/// Attempt to launch Docker Desktop. Returns immediately after dispatching
/// the launch command — use `wait_for_docker` to poll readiness.
#[tauri::command]
pub async fn start_docker_desktop() -> Result<String, String> {
    docker_ollama::start_docker_desktop().await
}

/// Gracefully quit Docker Desktop to free system memory.
#[tauri::command]
pub async fn stop_docker_desktop() -> Result<String, String> {
    docker_ollama::stop_docker_desktop().await
}

/// Block until Docker daemon is responsive or `timeout_secs` elapses.
#[tauri::command]
pub async fn wait_for_docker(timeout_secs: Option<u64>) -> bool {
    docker_ollama::wait_for_docker_ready(timeout_secs.unwrap_or(90)).await
}

/// Check the Ollama Docker container status.
#[tauri::command]
pub async fn check_ollama_container() -> docker_ollama::OllamaContainerStatus {
    docker_ollama::check_ollama_container().await
}

/// Ensure the Ollama container is running (creates + starts if needed).
/// Automatically enables GPU passthrough when an NVIDIA GPU is detected.
#[tauri::command]
pub async fn ensure_ollama_container() -> Result<String, String> {
    docker_ollama::ensure_ollama_container().await
}

/// Pull a model inside the running Ollama Docker container.
#[tauri::command(rename_all = "camelCase")]
pub async fn docker_pull_model(model_name: String) -> Result<String, String> {
    docker_ollama::docker_pull_model(&model_name).await
}

/// One-click full setup: check Docker → start Desktop → create container → pull model.
/// After success the model is ready for use via `set_brain_mode({ mode: 'local_ollama', model })`.
#[tauri::command(rename_all = "camelCase")]
pub async fn auto_setup_local_llm(
    model_name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let preference = {
        let settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        settings.preferred_container_runtime
    };
    auto_setup_local_llm_with_runtime(model_name, preference, state).await
}

/// Variant of [`auto_setup_local_llm`] that runs against an explicit
/// runtime preference. Lets the frontend bypass the persisted setting (for
/// example when the quest UI offers a one-off "try with Podman" button).
#[tauri::command(rename_all = "camelCase")]
pub async fn auto_setup_local_llm_with_runtime(
    model_name: String,
    preference: RuntimePreference,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let result = docker_ollama::auto_setup_local_llm_with(&model_name, preference).await?;

    // Auto-configure brain mode to LocalOllama with the pulled model
    let mode = crate::brain::BrainMode::LocalOllama {
        model: model_name.clone(),
    };
    crate::brain::brain_config::save(&state.data_dir, &mode)?;

    {
        let mut brain = state.active_brain.lock().map_err(|e| e.to_string())?;
        *brain = Some(model_name);
    }
    {
        let mut brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?;
        *brain_mode = Some(mode);
    }

    Ok(result)
}
