use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn load_vrm(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut vrm = state.vrm_path.lock().map_err(|e| e.to_string())?;
    *vrm = Some(path);
    Ok(())
}
