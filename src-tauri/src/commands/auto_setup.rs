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
    let result =
        auto_setup::write_vscode_config(std::path::Path::new(&workspace_root), &url, &token)?;

    // Track auto-configured MCP entry.
    {
        let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        settings.track_auto_configured("mcp_vscode");
        crate::settings::config_store::save(&state.data_dir, &settings)?;
    }

    Ok(result)
}

/// Set up Claude Desktop MCP integration.
///
/// Writes the `mcpServers.terransoul-brain` entry in the platform-specific
/// Claude Desktop config file.
#[tauri::command]
pub async fn setup_claude_mcp(state: State<'_, AppState>) -> Result<SetupResult, String> {
    let (token, port) = get_mcp_info(&state).await?;
    let url = mcp_url(port);
    let result = auto_setup::write_claude_config(&url, &token)?;

    {
        let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        settings.track_auto_configured("mcp_claude");
        crate::settings::config_store::save(&state.data_dir, &settings)?;
    }

    Ok(result)
}

/// Set up Codex CLI MCP integration.
#[tauri::command]
pub async fn setup_codex_mcp(state: State<'_, AppState>) -> Result<SetupResult, String> {
    let (token, port) = get_mcp_info(&state).await?;
    let url = mcp_url(port);
    let result = auto_setup::write_codex_config(&url, &token)?;

    {
        let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
        settings.track_auto_configured("mcp_codex");
        crate::settings::config_store::save(&state.data_dir, &settings)?;
    }

    Ok(result)
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

// ─── Stdio transport commands (Chunk 15.9) ──────────────────────────

/// Resolve the path to the running TerranSoul executable. Used by the
/// stdio auto-setup writers so editors know which binary to spawn.
fn current_exe_path() -> Result<String, String> {
    std::env::current_exe()
        .map_err(|e| format!("could not resolve current executable: {e}"))
        .map(|p| p.display().to_string())
}

/// Track an auto-configured MCP entry (separate key per client so the
/// quest tracker can detect *which* clients are wired up).
fn track_auto_configured(state: &AppState, key: &str) -> Result<(), String> {
    let mut settings = state.app_settings.lock().map_err(|e| e.to_string())?;
    settings.track_auto_configured(key);
    crate::settings::config_store::save(&state.data_dir, &settings)?;
    Ok(())
}

/// Set up VS Code / Copilot MCP integration over the **stdio**
/// transport. Writes `.vscode/mcp.json` with `command: <exe> --mcp-stdio`
/// instead of an HTTP URL. See Chunk 15.9.
#[tauri::command]
pub async fn setup_vscode_mcp_stdio(
    state: State<'_, AppState>,
    workspace_root: String,
) -> Result<SetupResult, String> {
    let exe = current_exe_path()?;
    let result =
        auto_setup::write_vscode_stdio_config(std::path::Path::new(&workspace_root), &exe)?;
    track_auto_configured(&state, "mcp_vscode_stdio")?;
    Ok(result)
}

/// Set up Claude Desktop MCP integration over the **stdio** transport.
#[tauri::command]
pub async fn setup_claude_mcp_stdio(state: State<'_, AppState>) -> Result<SetupResult, String> {
    let exe = current_exe_path()?;
    let result = auto_setup::write_claude_stdio_config(&exe)?;
    track_auto_configured(&state, "mcp_claude_stdio")?;
    Ok(result)
}

/// Set up Codex CLI MCP integration over the **stdio** transport.
#[tauri::command]
pub async fn setup_codex_mcp_stdio(state: State<'_, AppState>) -> Result<SetupResult, String> {
    let exe = current_exe_path()?;
    let result = auto_setup::write_codex_stdio_config(&exe)?;
    track_auto_configured(&state, "mcp_codex_stdio")?;
    Ok(result)
}

// ─── Dev prerequisites check (Chunk: setup-prerequisites) ───────────

/// Status of a single development prerequisite.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrereqStatus {
    pub name: String,
    pub ok: bool,
    pub version: Option<String>,
    pub install_hint: String,
}

/// Check all development prerequisites and return their status.
///
/// This is designed to be called from the frontend First Launch Wizard
/// or from the companion's chat (e.g., "check my dev setup").
#[tauri::command]
pub async fn check_prerequisites() -> Result<Vec<PrereqStatus>, String> {
    let mut results = Vec::new();

    // Node.js ≥ 20
    let node = std::process::Command::new("node")
        .arg("-v")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    let node_ok = node.as_ref().is_some_and(|v| {
        v.trim_start_matches('v')
            .split('.')
            .next()
            .and_then(|m| m.parse::<u32>().ok())
            .is_some_and(|major| major >= 20)
    });
    results.push(PrereqStatus {
        name: "Node.js ≥ 20".into(),
        ok: node_ok,
        version: node,
        install_hint: if cfg!(target_os = "windows") {
            "winget install OpenJS.NodeJS.LTS".into()
        } else {
            "brew install node@20".into()
        },
    });

    // Rust
    let rustc = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    results.push(PrereqStatus {
        name: "Rust (stable)".into(),
        ok: rustc.is_some(),
        version: rustc,
        install_hint: if cfg!(target_os = "windows") {
            "winget install Rustlang.Rustup".into()
        } else {
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh".into()
        },
    });

    // Tauri CLI
    let tauri_ver = std::process::Command::new("cargo")
        .args(["tauri", "--version"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    results.push(PrereqStatus {
        name: "Tauri CLI".into(),
        ok: tauri_ver.is_some(),
        version: tauri_ver,
        install_hint: "cargo install tauri-cli".into(),
    });

    // WebView2 (Windows only)
    #[cfg(target_os = "windows")]
    {
        let webview2 = std::process::Command::new("reg")
            .args([
                "query",
                r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
                "/v", "pv",
            ])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                if s.contains("0.0.0.0") { None }
                else { Some(s) }
            });
        results.push(PrereqStatus {
            name: "WebView2 Runtime".into(),
            ok: webview2.is_some(),
            version: webview2.and_then(|s| {
                s.lines()
                    .find(|l| l.contains("pv"))
                    .and_then(|l| l.split_whitespace().last())
                    .map(|v| v.to_string())
            }),
            install_hint: "winget install Microsoft.EdgeWebView2Runtime".into(),
        });
    }

    Ok(results)
}

/// Run the cross-platform setup script with --auto flag.
/// Returns stdout from the script execution.
#[tauri::command]
pub async fn run_setup_auto() -> Result<String, String> {
    let script_path = std::env::current_dir()
        .unwrap_or_default()
        .join("scripts")
        .join("setup-prerequisites.mjs");

    let output = std::process::Command::new("node")
        .args([script_path.to_string_lossy().as_ref(), "--auto"])
        .output()
        .map_err(|e| format!("Failed to run setup script: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Ok(format!("{stdout}\n{stderr}"))
    }
}
