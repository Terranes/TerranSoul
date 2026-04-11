use serde::Serialize;
use tauri::State;
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct AgentSearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub homepage: Option<String>,
}

#[tauri::command]
pub async fn start_registry_server(state: State<'_, AppState>) -> Result<u16, String> {
    let mut handle_guard = state.registry_server_handle.lock().await;
    if let Some((port, _)) = handle_guard.as_ref() {
        return Ok(*port);
    }
    let (port, handle) = crate::registry_server::start_server()
        .await
        .map_err(|e| e.to_string())?;
    let http_registry = crate::registry_server::HttpRegistry::new(port);
    *state.package_registry.lock().await = Box::new(http_registry);
    *handle_guard = Some((port, handle));
    Ok(port)
}

#[tauri::command]
pub async fn stop_registry_server(state: State<'_, AppState>) -> Result<(), String> {
    let mut handle_guard = state.registry_server_handle.lock().await;
    if let Some((_, handle)) = handle_guard.take() {
        handle.abort();
    }
    Ok(())
}

#[tauri::command]
pub async fn get_registry_server_port(state: State<'_, AppState>) -> Result<Option<u16>, String> {
    let handle_guard = state.registry_server_handle.lock().await;
    Ok(handle_guard.as_ref().map(|(port, _)| *port))
}

#[tauri::command]
pub async fn search_agents(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<AgentSearchResult>, String> {
    let registry = state.package_registry.lock().await;
    let manifests = registry.search(&query).await.map_err(|e| e.to_string())?;
    let results = manifests
        .into_iter()
        .map(|m| {
            let capabilities = m
                .capabilities
                .iter()
                .map(|c| {
                    serde_json::to_value(c)
                        .unwrap_or_default()
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string()
                })
                .collect();
            AgentSearchResult {
                name: m.name,
                version: m.version,
                description: m.description,
                capabilities,
                homepage: m.homepage,
            }
        })
        .collect();
    Ok(results)
}
