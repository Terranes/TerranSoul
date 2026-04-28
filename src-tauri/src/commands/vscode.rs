//! Tauri commands for VS Code workspace surfacing — Chunk 15.10.
//!
//! All commands operate on `<data_dir>/vscode-windows.json`, which is
//! managed by [`crate::vscode_workspace`]. Command bodies translate
//! Rust errors to the `Result<_, String>` shape Tauri expects.

use tauri::State;

use crate::vscode_workspace::{
    self, OpenOutcome, VsCodeWindow,
};
use crate::AppState;

/// Open `target_path` in VS Code, focusing an existing window when
/// possible. See `vscode_workspace::open_project` for the resolver
/// algorithm. Used by the Control Panel's primary button and by the
/// voice / chat intent `vscode.open_project`.
#[tauri::command]
pub async fn vscode_open_project(
    state: State<'_, AppState>,
    target_path: String,
) -> Result<OpenOutcome, String> {
    let target = std::path::PathBuf::from(&target_path);
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        vscode_workspace::open_project(&data_dir, &target)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("internal join error: {e}"))?
}

/// Snapshot the registry of currently-known VS Code windows. Dead
/// PIDs are pruned before the snapshot is returned so the UI never
/// sees stale entries.
#[tauri::command]
pub async fn vscode_list_known_windows(
    state: State<'_, AppState>,
) -> Result<Vec<VsCodeWindow>, String> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || vscode_workspace::list_known_windows(&data_dir))
        .await
        .map_err(|e| format!("internal join error: {e}"))
}

/// Drop a registry entry by PID (e.g. after the user closed VS Code
/// via Task Manager and the registry got out of sync). Idempotent.
#[tauri::command]
pub async fn vscode_forget_window(
    state: State<'_, AppState>,
    pid: u32,
) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        vscode_workspace::forget_window(&data_dir, pid)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("internal join error: {e}"))?
}
