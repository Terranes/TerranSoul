use serde::Serialize;
use tauri::State;

use crate::package_manager;
use crate::AppState;

/// Frontend-facing manifest summary returned from Tauri commands.
#[derive(Debug, Clone, Serialize)]
pub struct ManifestInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub sensitive_capabilities: Vec<String>,
    pub install_type: String,
    pub ipc_protocol_version: u32,
    pub author: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
}

impl From<&package_manager::AgentManifest> for ManifestInfo {
    fn from(m: &package_manager::AgentManifest) -> Self {
        let capabilities: Vec<String> = m
            .capabilities
            .iter()
            .map(|c| serde_json::to_value(c).unwrap_or_default())
            .map(|v| v.as_str().unwrap_or("unknown").to_string())
            .collect();

        let sensitive_capabilities: Vec<String> = m
            .capabilities
            .iter()
            .filter(|c| c.requires_consent())
            .map(|c| serde_json::to_value(c).unwrap_or_default())
            .map(|v| v.as_str().unwrap_or("unknown").to_string())
            .collect();

        let install_type = match &m.install_method {
            package_manager::InstallMethod::Binary { .. } => "binary",
            package_manager::InstallMethod::Wasm { .. } => "wasm",
            package_manager::InstallMethod::Sidecar { .. } => "sidecar",
        };

        ManifestInfo {
            name: m.name.clone(),
            version: m.version.clone(),
            description: m.description.clone(),
            capabilities,
            sensitive_capabilities,
            install_type: install_type.to_string(),
            ipc_protocol_version: m.ipc_protocol_version,
            author: m.author.clone(),
            license: m.license.clone(),
            homepage: m.homepage.clone(),
        }
    }
}

/// Frontend-facing installed agent summary.
#[derive(Debug, Clone, Serialize)]
pub struct InstalledAgentInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub install_path: String,
}

impl From<&package_manager::InstalledAgent> for InstalledAgentInfo {
    fn from(a: &package_manager::InstalledAgent) -> Self {
        InstalledAgentInfo {
            name: a.name.clone(),
            version: a.version.clone(),
            description: a.description.clone(),
            install_path: a.install_path.clone(),
        }
    }
}

/// Parse and validate a manifest JSON string, returning a summary for the frontend.
#[tauri::command]
pub async fn parse_agent_manifest(json: String) -> Result<ManifestInfo, String> {
    let manifest =
        package_manager::parse_manifest(&json).map_err(|e| e.to_string())?;
    Ok(ManifestInfo::from(&manifest))
}

/// Validate a manifest JSON string, returning Ok(()) if valid.
#[tauri::command]
pub async fn validate_agent_manifest(json: String) -> Result<(), String> {
    package_manager::parse_manifest(&json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Return the supported IPC protocol version range.
#[tauri::command]
pub async fn get_ipc_protocol_range() -> Result<(u32, u32), String> {
    Ok((
        package_manager::MIN_IPC_PROTOCOL_VERSION,
        package_manager::MAX_IPC_PROTOCOL_VERSION,
    ))
}

/// Install an agent from the mock registry.
#[tauri::command]
pub async fn install_agent(
    agent_name: String,
    state: State<'_, AppState>,
) -> Result<InstalledAgentInfo, String> {
    let mut installer = state.package_installer.lock().await;
    let registry = state.package_registry.lock().await;
    let result = installer
        .install(&agent_name, &**registry)
        .await
        .map_err(|e| e.to_string())?;
    Ok(InstalledAgentInfo::from(&result))
}

/// Update an installed agent to the latest version.
#[tauri::command]
pub async fn update_agent(
    agent_name: String,
    state: State<'_, AppState>,
) -> Result<InstalledAgentInfo, String> {
    let mut installer = state.package_installer.lock().await;
    let registry = state.package_registry.lock().await;
    let result = installer
        .update(&agent_name, &**registry)
        .await
        .map_err(|e| e.to_string())?;
    Ok(InstalledAgentInfo::from(&result))
}

/// Remove an installed agent.
#[tauri::command]
pub async fn remove_agent(
    agent_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut installer = state.package_installer.lock().await;
    installer
        .remove(&agent_name)
        .map_err(|e| e.to_string())
}

/// List all installed agents.
#[tauri::command]
pub async fn list_installed_agents(
    state: State<'_, AppState>,
) -> Result<Vec<InstalledAgentInfo>, String> {
    let installer = state.package_installer.lock().await;
    Ok(installer
        .list_installed()
        .iter()
        .map(InstalledAgentInfo::from)
        .collect())
}
