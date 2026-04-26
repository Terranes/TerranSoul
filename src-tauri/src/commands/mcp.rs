//! Tauri commands for the MCP server (Chunk 15.1).
//!
//! Four commands: start, stop, status, regenerate-token.

use tauri::State;

use crate::ai_integrations::mcp;
use crate::AppState;

/// Status snapshot returned by `mcp_server_start` and `mcp_server_status`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub token: Option<String>,
    /// Whether this is a dev/debug build (uses separate port range).
    pub is_dev: bool,
}

/// Start the MCP HTTP server on the default port for this build profile.
///
/// - **Release** builds default to port **7421**.
/// - **Dev** builds default to port **7422**.
///
/// If the port is already taken (e.g. the other build profile is running),
/// the server tries consecutive fallback ports before failing.
///
/// Returns the server status including the actual bound port and bearer
/// token (so the frontend can display it in the Control Panel or copy
/// to clipboard). If the server is already running, returns the existing
/// status.
#[tauri::command]
pub async fn mcp_server_start(state: State<'_, AppState>) -> Result<McpServerStatus, String> {
    let mut guard = state.mcp_server.lock().await;

    // Already running — return current status.
    if let Some(ref handle) = *guard {
        return Ok(McpServerStatus {
            running: true,
            port: Some(handle.port),
            token: Some(handle.token.clone()),
            is_dev: mcp::is_dev_build(),
        });
    }

    let token = mcp::auth::load_or_create(&state.data_dir)?;
    let port = mcp::default_port();
    let handle =
        mcp::start_server(state.inner().clone(), port, token).await?;

    let status = McpServerStatus {
        running: true,
        port: Some(handle.port),
        token: Some(handle.token.clone()),
        is_dev: mcp::is_dev_build(),
    };

    *guard = Some(handle);
    Ok(status)
}

/// Stop the MCP HTTP server.
#[tauri::command]
pub async fn mcp_server_stop(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.mcp_server.lock().await;
    if let Some(handle) = guard.take() {
        handle.stop();
        // Give the server a moment to drain, but don't block forever.
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            handle.task,
        )
        .await;
    }
    Ok(())
}

/// Query whether the MCP server is running and its port/token.
#[tauri::command]
pub async fn mcp_server_status(state: State<'_, AppState>) -> Result<McpServerStatus, String> {
    let guard = state.mcp_server.lock().await;
    match &*guard {
        Some(handle) => Ok(McpServerStatus {
            running: true,
            port: Some(handle.port),
            token: Some(handle.token.clone()),
            is_dev: mcp::is_dev_build(),
        }),
        None => Ok(McpServerStatus {
            running: false,
            port: None,
            token: None,
            is_dev: mcp::is_dev_build(),
        }),
    }
}

/// Regenerate the MCP bearer token. If the server is running it must be
/// restarted for the new token to take effect.
#[tauri::command]
pub async fn mcp_regenerate_token(state: State<'_, AppState>) -> Result<String, String> {
    mcp::auth::regenerate(&state.data_dir)
}
