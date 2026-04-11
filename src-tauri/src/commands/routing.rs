use serde::Serialize;
use tauri::State;

use crate::routing::command_envelope::CommandResult;
use crate::routing::permission::PermissionPolicy;
use crate::AppState;

/// Summary of a pending command exposed to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct PendingCommandInfo {
    pub command_id: String,
    pub origin_device: String,
    pub command_type: String,
    pub payload: serde_json::Value,
}

/// List pending commands that need user approval.
#[tauri::command]
pub async fn list_pending_commands(state: State<'_, AppState>) -> Result<Vec<PendingCommandInfo>, String> {
    let router = state.command_router.lock().await;
    let pending: Vec<PendingCommandInfo> = router
        .pending_commands()
        .iter()
        .map(|e| PendingCommandInfo {
            command_id: e.command_id.clone(),
            origin_device: e.origin_device.clone(),
            command_type: e.command_type.clone(),
            payload: e.payload.clone(),
        })
        .collect();
    Ok(pending)
}

/// Approve a pending command. Optionally remember the device (auto-allow future commands).
#[tauri::command]
pub async fn approve_remote_command(
    command_id: String,
    remember: bool,
    state: State<'_, AppState>,
) -> Result<CommandResult, String> {
    let mut router = state.command_router.lock().await;
    router
        .approve_command(&command_id, remember)
        .ok_or_else(|| format!("command {} not found or already processed", command_id))
}

/// Deny a pending command. Optionally block the device.
#[tauri::command]
pub async fn deny_remote_command(
    command_id: String,
    block: bool,
    state: State<'_, AppState>,
) -> Result<CommandResult, String> {
    let mut router = state.command_router.lock().await;
    router
        .deny_command(&command_id, block)
        .ok_or_else(|| format!("command {} not found or already processed", command_id))
}

/// Set the permission policy for a device.
#[tauri::command]
pub async fn set_device_permission(
    device_id: String,
    policy: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let p = match policy.as_str() {
        "allow" => PermissionPolicy::Allow,
        "deny" => PermissionPolicy::Deny,
        "ask" => PermissionPolicy::Ask,
        other => return Err(format!("invalid policy: {other} (expected allow/deny/ask)")),
    };
    let mut router = state.command_router.lock().await;
    router.permissions_mut().set_policy(&device_id, p);
    Ok(())
}

/// Get current device permission policies.
#[tauri::command]
pub async fn get_device_permissions(
    state: State<'_, AppState>,
) -> Result<Vec<(String, String)>, String> {
    let router = state.command_router.lock().await;
    let policies = router
        .permissions()
        .all_policies()
        .into_iter()
        .map(|(id, p)| {
            let p_str = match p {
                PermissionPolicy::Allow => "allow",
                PermissionPolicy::Deny => "deny",
                PermissionPolicy::Ask => "ask",
            };
            (id, p_str.to_string())
        })
        .collect();
    Ok(policies)
}
