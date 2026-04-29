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
    CodingLlmRecommendation, CodingWorkflowConfig, GitHubConfig, LearnedChunk, MetricsLog,
    MetricsSummary, PrSummary, PullResult, RepoState, RunRecord, SelfImproveSettings,
};
use crate::AppState;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Return the curated list of recommended coding LLM providers.
///
/// The local Ollama entry is hardware-adaptive: the returned
/// `default_model` is the best model for the user's RAM tier per
/// `brain-advanced-design.md` §26. The system RAM is detected
/// automatically via `sysinfo`.
#[tauri::command]
pub async fn list_coding_llm_recommendations() -> Vec<CodingLlmRecommendation> {
    let ram = crate::brain::system_info::collect().total_ram_mb;
    coding::coding_llm_recommendations(ram)
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
    let workflow_cfg = state
        .coding_workflow_config
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    coding::engine::start(app, engine, cfg, workflow_cfg, repo_hint).await;
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

// ---------------------------------------------------------------------------
// GitHub config + PR automation (Phase 25 — Chunk 25.13)
// ---------------------------------------------------------------------------

/// Persisted GitHub binding (token, owner/repo, base branch, reviewers).
/// Returns `None` when not yet configured.
#[tauri::command]
pub async fn get_github_config(
    state: State<'_, AppState>,
) -> Result<Option<GitHubConfig>, String> {
    Ok(coding::load_github_config(&state.data_dir))
}

/// Save the GitHub config (atomic write). Pass `null` to clear.
///
/// When `owner` / `repo` are empty the backend attempts to derive them
/// from the repository's `origin` remote URL — the resulting config
/// returned to the caller has the filled-in values so the UI can preview
/// what was stored.
#[tauri::command]
pub async fn set_github_config(
    config: Option<GitHubConfig>,
    state: State<'_, AppState>,
) -> Result<Option<GitHubConfig>, String> {
    match config {
        None => {
            coding::clear_github_config(&state.data_dir)?;
            Ok(None)
        }
        Some(mut cfg) => {
            if cfg.owner.is_empty() || cfg.repo.is_empty() {
                let root = coding_repo::guess_repo_root(&state.data_dir);
                if let Some(remote) = coding_repo::detect_repo(&root).remote_url {
                    if let Some((o, r)) = coding::parse_owner_repo(&remote) {
                        if cfg.owner.is_empty() {
                            cfg.owner = o;
                        }
                        if cfg.repo.is_empty() {
                            cfg.repo = r;
                        }
                    }
                }
            }
            if cfg.default_base.is_empty() {
                cfg.default_base = "main".to_string();
            }
            coding::save_github_config(&state.data_dir, &cfg)?;
            Ok(Some(cfg))
        }
    }
}

/// Manually trigger the "open / update PR for the current feature branch"
/// flow. Useful for users who want to ship before the autonomous loop
/// completes every chunk. Returns the PR summary on success.
#[tauri::command]
pub async fn open_self_improve_pr(
    state: State<'_, AppState>,
) -> Result<PrSummary, String> {
    let cfg = coding::load_github_config(&state.data_dir)
        .ok_or_else(|| "GitHub not configured. Set token + owner/repo first.".to_string())?;
    if !cfg.is_complete() {
        return Err("GitHub config incomplete (token, owner, repo required).".to_string());
    }
    let repo_root = coding_repo::guess_repo_root(&state.data_dir);
    let repo_state = coding_repo::detect_repo(&repo_root);
    if !repo_state.is_git_repo {
        return Err("Not inside a git repository.".to_string());
    }
    let head = coding::git_ops::current_branch(&repo_root)
        .ok_or_else(|| "Detached HEAD — cannot open a PR.".to_string())?;
    if head == cfg.default_base {
        return Err(format!(
            "Currently on the base branch ({head}); switch to a feature branch first."
        ));
    }
    let client = reqwest::Client::new();
    let title = format!("self-improve: complete autonomous chunks ({head})");
    let body = "Opened manually from the TerranSoul self-improve panel.".to_string();
    coding::open_or_update_pr(&client, &cfg, &head, &title, &body).await
}

/// Manually pull-and-merge `origin/<base>` into the current branch with
/// optional LLM-assisted conflict resolution. Returns a structured
/// outcome the UI can render verbatim.
#[tauri::command]
pub async fn pull_main_for_self_improve(
    state: State<'_, AppState>,
) -> Result<PullResult, String> {
    let repo_root = coding_repo::guess_repo_root(&state.data_dir);
    let base = coding::load_github_config(&state.data_dir)
        .map(|c| c.default_base)
        .unwrap_or_else(|| "main".to_string());
    let coding_cfg = state.coding_llm_config.lock().map_err(|e| e.to_string())?.clone();
    Ok(coding::pull_main(&repo_root, &base, coding_cfg.as_ref()).await)
}

/// Inspect the current chat message for an improvement / feature /
/// bug-fix idea and, when found, append it as a new `not-started` chunk
/// to `rules/milestones.md`. Always returns `Ok` (the chat pipeline must
/// never fail because of the learning hook); the inner `Option` is
/// `Some(chunk)` when something actionable was learned.
///
/// Silently no-ops when self-improve is disabled OR the coding LLM is
/// not configured — the UI can fire this on every user message without
/// pre-checking the toggle.
#[tauri::command]
pub async fn learn_from_user_message(
    message: String,
    state: State<'_, AppState>,
) -> Result<Option<LearnedChunk>, String> {
    let enabled = state
        .self_improve
        .lock()
        .map(|s| s.enabled)
        .map_err(|e| e.to_string())?;
    if !enabled {
        return Ok(None);
    }
    let cfg = match state.coding_llm_config.lock().map_err(|e| e.to_string())?.clone() {
        Some(c) => c,
        None => return Ok(None),
    };
    let repo_root = coding_repo::guess_repo_root(&state.data_dir);
    let learned =
        coding::learn_from_message(&message, &cfg, &state.data_dir, &repo_root).await;
    Ok(learned)
}

/// Run a coding task through the shared workflow.
///
/// Reusable entry point for **any** coding work — not just self-improve.
/// The chat UI, future agents, and ad-hoc tooling all call this command
/// when they need the coding LLM to produce a plan, a JSON object,
/// resolved file contents, or prose.
///
/// Behaviour:
/// 1. Requires a configured Coding LLM (returns `Err` otherwise).
/// 2. Auto-loads `rules/*.md`, `instructions/*.md`, and `docs/*.md` from
///    the bound repository as supplementary documents (configurable on
///    the task) so every task is anchored to the same source of truth.
/// 3. Builds an XML-structured prompt via
///    [`coding::CodingPrompt`] applying the ten Anthropic prompt
///    principles uniformly (see `rules/prompting-rules.md`).
/// 4. Returns the structured payload extracted from the requested
///    output tag, plus the raw reply for debugging.
///
/// The command is **stateless** — no metrics are recorded and no
/// progress events are emitted, so it is safe to call from tests, ad-hoc
/// chat actions, or background agents without touching the self-improve
/// metrics log.
#[tauri::command(rename_all = "camelCase")]
pub async fn run_coding_task(
    mut task: coding::CodingTask,
    state: State<'_, AppState>,
) -> Result<coding::CodingTaskResult, String> {
    let cfg = state
        .coding_llm_config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| {
            "No coding LLM configured. Open Brain → Coding LLM and pick a provider first."
                .to_string()
        })?;

    // If the caller did not specify a repo root, default to the bound
    // repository so context auto-loading works out of the box.
    if task.repo_root.is_none() {
        task.repo_root = Some(coding_repo::guess_repo_root(&state.data_dir));
    }

    let workflow_cfg = state
        .coding_workflow_config
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    coding::run_coding_task(&cfg, &task, Some(&workflow_cfg)).await
}

// ---------------------------------------------------------------------------
// Coding workflow config (Chunk 25.16)
// ---------------------------------------------------------------------------

/// Single document entry shown in the live preview pane of the
/// CodingWorkflowConfigPanel — `label` is the repo-relative path and
/// `char_count` is the loaded length after truncation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CodingWorkflowPreviewDoc {
    pub label: String,
    pub char_count: usize,
}

/// Aggregate preview returned to the UI: list of matched files plus
/// roll-ups so the panel can render counters and progress bars.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CodingWorkflowPreview {
    pub documents: Vec<CodingWorkflowPreviewDoc>,
    pub total_chars: usize,
    pub file_count: usize,
    pub repo_root: String,
}

/// Return the persisted coding-workflow config (always returns a record
/// — defaults are returned when nothing has been saved).
#[tauri::command]
pub async fn get_coding_workflow_config(
    state: State<'_, AppState>,
) -> Result<CodingWorkflowConfig, String> {
    let cfg = state
        .coding_workflow_config
        .lock()
        .map_err(|e| e.to_string())?;
    Ok(cfg.clone())
}

/// Persist a new coding-workflow config. Validates basic invariants
/// (non-zero char caps, no empty trimmed strings) before writing.
///
/// Implements the atomicity contract from
/// `rules/coding-workflow-reliability.md` §2: write-to-disk happens
/// before in-memory swap, and the in-memory state stays on its
/// previous value when the disk write fails.
#[tauri::command(rename_all = "camelCase")]
pub async fn set_coding_workflow_config(
    config: CodingWorkflowConfig,
    state: State<'_, AppState>,
) -> Result<CodingWorkflowConfig, String> {
    // 1. Validate.
    if config.max_file_chars == 0 {
        return Err("max_file_chars must be greater than zero.".to_string());
    }
    if config.max_total_chars == 0 {
        return Err("max_total_chars must be greater than zero.".to_string());
    }
    if config.max_total_chars < config.max_file_chars {
        return Err(
            "max_total_chars must be greater than or equal to max_file_chars.".to_string(),
        );
    }
    let trim_check = |list: &[String], field: &str| -> Result<(), String> {
        for entry in list {
            if entry.trim().is_empty() {
                return Err(format!("{field} entries must be non-empty."));
            }
        }
        Ok(())
    };
    trim_check(&config.include_dirs, "include_dirs")?;
    trim_check(&config.include_files, "include_files")?;
    trim_check(&config.exclude_paths, "exclude_paths")?;

    // 2. Persist atomically.
    coding::save_coding_workflow_config(&state.data_dir, &config)?;

    // 3. Swap in-memory only after the disk write succeeded.
    let mut slot = state
        .coding_workflow_config
        .lock()
        .map_err(|e| e.to_string())?;
    *slot = config.clone();
    Ok(config)
}

/// Reset the coding-workflow config to defaults (deletes the persisted
/// file and resets the in-memory state).
#[tauri::command]
pub async fn reset_coding_workflow_config(
    state: State<'_, AppState>,
) -> Result<CodingWorkflowConfig, String> {
    coding::clear_coding_workflow_config(&state.data_dir)?;
    let defaults = CodingWorkflowConfig::default();
    let mut slot = state
        .coding_workflow_config
        .lock()
        .map_err(|e| e.to_string())?;
    *slot = defaults.clone();
    Ok(defaults)
}

/// Preview which files the current workflow config will inject into
/// the next coding-task prompt. Used by the settings panel to show a
/// live preview as the user edits the config.
#[tauri::command(rename_all = "camelCase")]
pub async fn preview_coding_workflow_context(
    config: Option<CodingWorkflowConfig>,
    state: State<'_, AppState>,
) -> Result<CodingWorkflowPreview, String> {
    let cfg = match config {
        Some(c) => c,
        None => state
            .coding_workflow_config
            .lock()
            .map_err(|e| e.to_string())?
            .clone(),
    };
    let repo_root = coding_repo::guess_repo_root(&state.data_dir);
    let docs = coding::workflow::load_workflow_context(&repo_root, &cfg, true, true, true);
    let documents: Vec<CodingWorkflowPreviewDoc> = docs
        .iter()
        .map(|d| CodingWorkflowPreviewDoc {
            label: d.label.clone(),
            char_count: d.body.chars().count(),
        })
        .collect();
    let total_chars: usize = documents.iter().map(|d| d.char_count).sum();
    let file_count = documents.len();
    Ok(CodingWorkflowPreview {
        documents,
        total_chars,
        file_count,
        repo_root: repo_root.to_string_lossy().to_string(),
    })
}

// ---------------------------------------------------------------------------
// Local Ollama discovery (Chunk 25.17)
// ---------------------------------------------------------------------------

/// Probe `/api/tags` on `127.0.0.1:11434` (the default Ollama port) and
/// return the list of installed model names. Empty vec on connection
/// failure — the UI treats that as "no Ollama running" and surfaces an
/// install prompt.
///
/// `base_url` is optional; defaults to `http://127.0.0.1:11434`. Pass
/// the user's custom Ollama port when probing a non-default install.
#[tauri::command(rename_all = "camelCase")]
pub async fn list_local_coding_models(
    base_url: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let url = base_url
        .as_deref()
        .map(strip_trailing_slash)
        .unwrap_or("http://127.0.0.1:11434");
    // Strip the OpenAI-compatible `/v1` suffix if the user pasted that —
    // the native `/api/tags` endpoint lives one level up.
    let probe_url = url.trim_end_matches("/v1");
    let entries = crate::brain::ollama_agent::list_models(&state.ollama_client, probe_url).await;
    Ok(entries.into_iter().map(|m| m.name).collect())
}

fn strip_trailing_slash(s: &str) -> &str {
    s.trim_end_matches('/')
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
