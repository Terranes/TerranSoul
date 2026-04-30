//! Tauri commands for the plugin system.

use tauri::State;

use crate::plugins::{
    self, InstalledPlugin, PluginHostStatus, PluginManifest,
    CommandEntry, SlashCommandEntry, ContributedTheme,
};
use crate::plugins::host::CommandResult;
use crate::AppState;

#[tauri::command]
pub async fn plugin_install(
    json: String,
    app_state: State<'_, AppState>,
) -> Result<InstalledPlugin, String> {
    let manifest = plugins::parse_plugin_manifest(&json).map_err(|e| e.to_string())?;
    app_state.plugin_host.install(manifest).await
}

#[tauri::command]
pub async fn plugin_activate(
    plugin_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    app_state.plugin_host.activate(&plugin_id).await
}

#[tauri::command]
pub async fn plugin_deactivate(
    plugin_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    app_state.plugin_host.deactivate(&plugin_id).await
}

#[tauri::command]
pub async fn plugin_uninstall(
    plugin_id: String,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    app_state.plugin_host.uninstall(&plugin_id).await
}

#[tauri::command]
pub async fn plugin_list(
    app_state: State<'_, AppState>,
) -> Result<Vec<InstalledPlugin>, String> {
    Ok(app_state.plugin_host.list_plugins().await)
}

#[tauri::command]
pub async fn plugin_get(
    plugin_id: String,
    app_state: State<'_, AppState>,
) -> Result<Option<InstalledPlugin>, String> {
    Ok(app_state.plugin_host.get_plugin(&plugin_id).await)
}

#[tauri::command]
pub async fn plugin_list_commands(
    app_state: State<'_, AppState>,
) -> Result<Vec<CommandEntry>, String> {
    Ok(app_state.plugin_host.list_commands().await)
}

#[tauri::command]
pub async fn plugin_list_slash_commands(
    app_state: State<'_, AppState>,
) -> Result<Vec<SlashCommandEntry>, String> {
    Ok(app_state.plugin_host.list_slash_commands().await)
}

#[tauri::command]
pub async fn plugin_list_themes(
    app_state: State<'_, AppState>,
) -> Result<Vec<ContributedTheme>, String> {
    Ok(app_state.plugin_host.list_themes().await)
}

#[tauri::command]
pub async fn plugin_get_setting(
    key: String,
    app_state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    Ok(app_state.plugin_host.get_setting(&key).await)
}

#[tauri::command]
pub async fn plugin_set_setting(
    key: String,
    value: serde_json::Value,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    app_state.plugin_host.set_setting(&key, value).await;
    Ok(())
}

#[tauri::command]
pub async fn plugin_host_status(
    app_state: State<'_, AppState>,
) -> Result<PluginHostStatus, String> {
    Ok(app_state.plugin_host.status().await)
}

#[tauri::command]
pub async fn plugin_parse_manifest(
    json: String,
) -> Result<PluginManifest, String> {
    plugins::parse_plugin_manifest(&json).map_err(|e| e.to_string())
}

/// Invoke a contributed command by id (Chunk 22.4).
///
/// Returns a [`CommandResult`] echoing the command's metadata. Full
/// execution lands in Chunk 22.7.
#[tauri::command]
pub async fn plugin_invoke_command(
    command_id: String,
    args: Option<serde_json::Value>,
    app_state: State<'_, AppState>,
) -> Result<CommandResult, String> {
    app_state
        .plugin_host
        .invoke_command(&command_id, args)
        .await
}

/// Invoke a slash-command by its name (without `/`) (Chunk 22.4).
#[tauri::command]
pub async fn plugin_invoke_slash_command(
    name: String,
    args: Option<serde_json::Value>,
    app_state: State<'_, AppState>,
) -> Result<CommandResult, String> {
    app_state
        .plugin_host
        .invoke_slash_command(&name, args)
        .await
}
