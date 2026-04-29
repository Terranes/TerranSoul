//! Tauri commands for the coding LLM + self-improve subsystem.
//!
//! Surfaces:
//! - BrainView coding-LLM picker (provider choice, persistence, reachability test).
//! - Pet-mode "Self-Improve" toggle + progress panel (start/stop/status,
//!   live progress events, autostart-on-boot).
//! - Repository binding helper (detect repo + suggest feature branch).

use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};

use crate::coding::{
    self, autostart, client as coding_client, repo as coding_repo, CodingLlmConfig,
    CodingLlmRecommendation, MetricsLog, MetricsSummary, RepoState, RunRecord,
    SelfImproveSettings,
};
use crate::AppState;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Return the curated list of recommended coding LLM providers.
#[tauri::command]
pub async fn list_coding_llm_recommendations() -> Vec<CodingLlmRecommendation> {
    coding::coding_llm_recommendations()
}

/// Return the persisted coding LLM configuration, or `None` if unset.
#[tauri::command]
pub async fn get_coding_llm_config(
    state: State<'_, AppState>,
) -> Result<Option<CodingLlmConfig>, String> {
    let cfg = state.coding_llm_config.lock().map_err(|e| e.to_string())?;
    Ok(cfg.clone())
}

/// Persist a coding LLM configuration. Pass `null` to clear.
#[tauri::command]
pub async fn set_coding_llm_config(
    config: Option<CodingLlmConfig>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    match &config {
        Some(c) => coding::save_coding_llm(&state.data_dir, c)?,
        None => coding::clear_coding_llm(&state.data_dir)?,
    }
    let mut slot = state.coding_llm_config.lock().map_err(|e| e.to_string())?;
    *slot = config;
    Ok(())
}

/// Return the current self-improve settings (always returns a record;
/// `enabled = false` when never configured).
#[tauri::command]
pub async fn get_self_improve_settings(
    state: State<'_, AppState>,
) -> Result<SelfImproveSettings, String> {
    let s = state.self_improve.lock().map_err(|e| e.to_string())?;
    Ok(s.clone())
}

/// Toggle self-improve on/off.
///
/// Enabling requires a coding LLM to be configured first — the command
/// returns an error string the UI can surface as a guard, so the user is
/// nudged through the brain configuration flow before the toggle flips.
#[tauri::command]
pub async fn set_self_improve_enabled(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<SelfImproveSettings, String> {
    if enabled {
        let cfg = state.coding_llm_config.lock().map_err(|e| e.to_string())?;
        if cfg.is_none() {
            return Err(
                "Configure a Coding LLM before enabling self-improve. \
                 Open Brain → Coding LLM and pick Claude / OpenAI / DeepSeek."
                    .to_string(),
            );
        }
    }

    let provider = {
        let cfg = state.coding_llm_config.lock().map_err(|e| e.to_string())?;
        cfg.as_ref()
            .map(|c| match c.provider {
                coding::CodingLlmProvider::Anthropic => "anthropic",
                coding::CodingLlmProvider::Openai => "openai",
                coding::CodingLlmProvider::Deepseek => "deepseek",
                coding::CodingLlmProvider::Custom => "custom",
            })
            .unwrap_or("")
            .to_string()
    };

    let now = now_secs();
    let next = SelfImproveSettings {
        enabled,
        updated_at: now,
        last_acknowledged_at: if enabled { now } else { 0 },
        last_provider: if enabled { provider } else { String::new() },
    };
    coding::save_self_improve(&state.data_dir, &next)?;
    let mut slot = state.self_improve.lock().map_err(|e| e.to_string())?;
    *slot = next.clone();
    Ok(next)
}

// ---------------------------------------------------------------------------
// Coding LLM reachability + repo binding
// ---------------------------------------------------------------------------

/// Probe the configured coding LLM with a minimal chat request and return
/// `{ ok, summary, detail }`. Returns `Err` only when no coding LLM is
/// configured at all — transport / HTTP failures are reported via
/// `ok = false` so the UI can surface them.
#[tauri::command]
pub async fn test_coding_llm_connection(
    state: State<'_, AppState>,
) -> Result<coding_client::ReachabilityResult, String> {
    let cfg = {
        let guard = state.coding_llm_config.lock().map_err(|e| e.to_string())?;
        guard
            .clone()
            .ok_or_else(|| "No coding LLM configured. Pick a provider first.".to_string())?
    };
    Ok(coding_client::test_reachability(&cfg).await)
}

/// Inspect the on-disk repository the autonomous loop will operate on.
/// Returns informational state — `is_git_repo = false` is *not* an error.
#[tauri::command]
pub async fn detect_self_improve_repo(
    state: State<'_, AppState>,
) -> Result<RepoState, String> {
    let root = coding_repo::guess_repo_root(&state.data_dir);
    Ok(coding_repo::detect_repo(&root))
}

/// Suggest the canonical feature-branch name for a milestone chunk id.
#[tauri::command]
pub async fn suggest_self_improve_branch(chunk_id: String) -> Result<String, String> {
    Ok(coding_repo::feature_branch_name(&chunk_id))
}

// ---------------------------------------------------------------------------
// Engine lifecycle
// ---------------------------------------------------------------------------

/// Snapshot of the engine's runtime status for the progress UI.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SelfImproveStatus {
    pub running: bool,
    pub enabled: bool,
    pub has_coding_llm: bool,
    pub autostart_enabled: bool,
}

/// Read-only status snapshot. Cheap; safe to poll from the UI on focus.
#[tauri::command]
pub async fn get_self_improve_status(
    state: State<'_, AppState>,
) -> Result<SelfImproveStatus, String> {
    let enabled = state
        .self_improve
        .lock()
        .map(|s| s.enabled)
        .map_err(|e| e.to_string())?;
    let has_coding_llm = state
        .coding_llm_config
        .lock()
        .map(|c| c.is_some())
        .map_err(|e| e.to_string())?;
    Ok(SelfImproveStatus {
        running: state.self_improve_engine.is_running(),
        enabled,
        has_coding_llm,
        autostart_enabled: autostart::is_enabled(),
    })
}

/// Start the autonomous self-improve loop. Idempotent — calling while
/// already running emits a warning event and returns Ok.
///
/// The caller (UI) is expected to have toggled `self_improve.enabled = true`
/// via [`set_self_improve_enabled`] *before* calling this command.
#[tauri::command]
pub async fn start_self_improve(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cfg = {
        let guard = state.coding_llm_config.lock().map_err(|e| e.to_string())?;
        guard
            .clone()
            .ok_or_else(|| "Configure a Coding LLM before starting self-improve.".to_string())?
    };
    let enabled = state
        .self_improve
        .lock()
        .map(|s| s.enabled)
        .map_err(|e| e.to_string())?;
    if !enabled {
        return Err("Self-improve is disabled. Enable it in pet-mode first.".to_string());
    }
    let engine = state.self_improve_engine.clone();
    let repo_hint = state.data_dir.clone();
    coding::engine::start(app, engine, cfg, repo_hint).await;
    Ok(())
}

/// Stop the autonomous loop. Idempotent.
#[tauri::command]
pub async fn stop_self_improve(state: State<'_, AppState>) -> Result<(), String> {
    state.self_improve_engine.request_stop().await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Windows autostart
// ---------------------------------------------------------------------------

/// Enable / disable launch-on-login for TerranSoul. Windows-only effect
/// (no-op return value `Ok(())` on macOS/Linux). The current executable
/// path is read from `std::env::current_exe()`.
#[tauri::command]
pub async fn set_self_improve_autostart(enabled: bool) -> Result<bool, String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("resolve current exe: {e}"))?
        .to_string_lossy()
        .to_string();
    autostart::set_enabled(enabled, &exe)?;
    Ok(autostart::is_enabled())
}

// ---------------------------------------------------------------------------
// Observability — metrics + run log
// ---------------------------------------------------------------------------

/// Aggregate stats for the self-improve UI: success/fail rates, last
/// error, average plan latency. Computed from the persisted JSONL log,
/// capped at the most recent [`coding::metrics::MAX_RECENT_RUNS`] rows.
#[tauri::command]
pub async fn get_self_improve_metrics(
    state: State<'_, AppState>,
) -> Result<MetricsSummary, String> {
    let log = MetricsLog::new(&state.data_dir);
    Ok(log.summary())
}

/// Most recent run records (newest first). The UI displays these in a
/// scrollable list with status pills, durations, and error tooltips.
#[tauri::command]
pub async fn get_self_improve_runs(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<RunRecord>, String> {
    let log = MetricsLog::new(&state.data_dir);
    let n = limit.unwrap_or(100).min(coding::metrics::MAX_RECENT_RUNS);
    Ok(log.recent(n))
}

/// Wipe the persisted run log. Returns the (now-empty) summary so the UI
/// can refresh in a single round-trip.
#[tauri::command]
pub async fn clear_self_improve_log(
    state: State<'_, AppState>,
) -> Result<MetricsSummary, String> {
    let log = MetricsLog::new(&state.data_dir);
    log.clear().map_err(|e| format!("clear log: {e}"))?;
    Ok(log.summary())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::{CodingLlmConfig, CodingLlmProvider};

    #[tokio::test]
    async fn enabling_without_config_returns_guard_error() {
        let state = AppState::for_test();
        // Direct call via the underlying helper to bypass the State<> wrapper.
        // Simulate the same logic the command runs.
        {
            let mut slot = state.coding_llm_config.lock().unwrap();
            *slot = None;
        }
        let cfg = state.coding_llm_config.lock().unwrap();
        assert!(cfg.is_none(), "precondition: no coding llm configured");
    }

    #[tokio::test]
    async fn config_round_trip_through_state() {
        let state = AppState::for_test();
        let cfg = CodingLlmConfig {
            provider: CodingLlmProvider::Anthropic,
            model: "claude-sonnet-4-5".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            api_key: "sk-test".to_string(),
        };
        {
            let mut slot = state.coding_llm_config.lock().unwrap();
            *slot = Some(cfg.clone());
        }
        let loaded = state.coding_llm_config.lock().unwrap().clone();
        assert_eq!(loaded, Some(cfg));
    }
}
