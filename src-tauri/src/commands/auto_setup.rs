//! Tauri commands for auto-setup of external AI coding assistant configs.

use tauri::State;

use crate::ai_integrations::mcp::auto_setup::{self, ClientStatus, SetupResult};
use crate::ai_integrations::mcp::{self, DEFAULT_PORT};
use crate::AppState;

/// Build the MCP server URL from the current state.
fn mcp_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

/// Get the current MCP token and port from state.
async fn get_mcp_info(state: &AppState) -> Result<(String, u16), String> {
    let token = mcp::auth::load_or_create(&state.data_dir)?;
    let guard = state.mcp_server.lock().await;
    let port = guard.as_ref().map(|h| h.port).unwrap_or(DEFAULT_PORT);
    Ok((token, port))
}

/// Set up VS Code / Copilot MCP integration.
///
/// Writes `.vscode/mcp.json` in the given workspace root with the
/// `terransoul-brain` entry pointing at the running MCP server.
#[tauri::command]
pub async fn setup_vscode_mcp(
    state: State<'_, AppState>,
    workspace_root: String,
) -> Result<SetupResult, String> {
    let (token, port) = get_mcp_info(&state).await?;
    let url = mcp_url(port);
    auto_setup::write_vscode_config(
        std::path::Path::new(&workspace_root),
        &url,
        &token,
    )
}

/// Set up Claude Desktop MCP integration.
///
/// Writes the `mcpServers.terransoul-brain` entry in the platform-specific
/// Claude Desktop config file.
#[tauri::command]
pub async fn setup_claude_mcp(
    state: State<'_, AppState>,
) -> Result<SetupResult, String> {
    let (token, port) = get_mcp_info(&state).await?;
    let url = mcp_url(port);
    auto_setup::write_claude_config(&url, &token)
}

/// Set up Codex CLI MCP integration.
#[tauri::command]
pub async fn setup_codex_mcp(
    state: State<'_, AppState>,
) -> Result<SetupResult, String> {
    let (token, port) = get_mcp_info(&state).await?;
    let url = mcp_url(port);
    auto_setup::write_codex_config(&url, &token)
}

/// Remove the `terransoul-brain` entry from VS Code config.
#[tauri::command]
pub async fn remove_vscode_mcp(workspace_root: String) -> Result<SetupResult, String> {
    auto_setup::remove_vscode_config(std::path::Path::new(&workspace_root))
}

/// Remove the `terransoul-brain` entry from Claude Desktop config.
#[tauri::command]
pub async fn remove_claude_mcp() -> Result<SetupResult, String> {
    auto_setup::remove_claude_config()
}

/// Remove the `terransoul-brain` entry from Codex CLI config.
#[tauri::command]
pub async fn remove_codex_mcp() -> Result<SetupResult, String> {
    auto_setup::remove_codex_config()
}

/// List all supported clients and their setup status.
#[tauri::command]
pub async fn list_mcp_clients(workspace_root: String) -> Result<Vec<ClientStatus>, String> {
    Ok(auto_setup::list_client_status(std::path::Path::new(
        &workspace_root,
    )))
}
