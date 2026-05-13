//! Tauri commands for the safety classifier (Chunk 43.10).

use tauri::State;

use crate::coding::safety::{self, Action, SafetyConfig, SafetyDecisionRecord};
use crate::AppState;

/// Request permission for an action. Returns `true` if auto-approved (Tier 1).
#[tauri::command(rename_all = "camelCase")]
pub async fn safety_request_permission(
    action: String,
    reason: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let action = parse_action(&action)?;
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let config = SafetyConfig::default();
    safety::request_permission(&store.conn, action, &config, &reason).map_err(|e| e.to_string())
}

/// List recent safety decisions.
#[tauri::command(rename_all = "camelCase")]
pub async fn safety_list_decisions(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<SafetyDecisionRecord>, String> {
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let n = limit.unwrap_or(50).min(200);
    safety::list_decisions(&store.conn, n).map_err(|e| e.to_string())
}

/// Check if an action qualifies for Tier 1 promotion.
#[tauri::command(rename_all = "camelCase")]
pub async fn safety_check_promotion(
    action: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let action = parse_action(&action)?;
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let config = SafetyConfig::default();
    safety::check_promotion(&store.conn, action, &config).map_err(|e| e.to_string())
}

fn parse_action(s: &str) -> Result<Action, String> {
    match s {
        "read" => Ok(Action::Read),
        "write" => Ok(Action::Write),
        "run_tests" => Ok(Action::RunTests),
        "create_branch" => Ok(Action::CreateBranch),
        "push_remote" => Ok(Action::PushRemote),
        "open_pr" => Ok(Action::OpenPr),
        "merge_pr" => Ok(Action::MergePr),
        "run_shell" => Ok(Action::RunShell),
        "send_email" => Ok(Action::SendEmail),
        "install_package" => Ok(Action::InstallPackage),
        "delete_file" => Ok(Action::DeleteFile),
        "drop_table" => Ok(Action::DropTable),
        _ => Err(format!("unknown action: {s}")),
    }
}
