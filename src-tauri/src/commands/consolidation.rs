//! Tauri commands for sleep-time memory consolidation (Chunk 16.7).

use crate::memory::consolidation::{ConsolidationConfig, ConsolidationResult};
use crate::AppState;

/// Trigger a full consolidation run with the given session IDs.
#[tauri::command]
pub async fn run_sleep_consolidation(
    state: tauri::State<'_, AppState>,
    session_ids: Vec<String>,
    config: Option<ConsolidationConfig>,
) -> Result<ConsolidationResult, String> {
    let cfg = config.unwrap_or_default();
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    Ok(crate::memory::consolidation::run_consolidation(
        &store,
        &session_ids,
        &cfg,
    ))
}

/// Record a user interaction (resets the idle timer).
#[tauri::command]
pub fn touch_activity(state: tauri::State<'_, AppState>) {
    state.activity_tracker.touch();
}

/// Get current idle status.
#[tauri::command]
pub fn get_idle_status(
    state: tauri::State<'_, AppState>,
    threshold_ms: Option<i64>,
) -> Result<serde_json::Value, String> {
    let threshold = threshold_ms.unwrap_or(crate::memory::consolidation::DEFAULT_IDLE_THRESHOLD_MS);
    Ok(serde_json::json!({
        "idle_ms": state.activity_tracker.idle_ms(),
        "is_idle": state.activity_tracker.is_idle(threshold),
        "last_activity": state.activity_tracker.last_activity(),
        "threshold_ms": threshold,
    }))
}
