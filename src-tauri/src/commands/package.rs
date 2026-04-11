use serde::Serialize;
use crate::package_manager;

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
