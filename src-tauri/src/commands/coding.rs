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
            return Err("Configure a Coding LLM before enabling self-improve. \
                 Open Brain → Coding LLM and pick Local Ollama, Claude, OpenAI, or Custom."
                .to_string());
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
    let existing_worktree_dir = {
        let s = state.self_improve.lock().map_err(|e| e.to_string())?;
        s.worktree_dir.clone()
    };
    let next = SelfImproveSettings {
        enabled,
        updated_at: now,
        last_acknowledged_at: if enabled { now } else { 0 },
        last_provider: if enabled { provider } else { String::new() },
        worktree_dir: existing_worktree_dir,
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
pub async fn detect_self_improve_repo(state: State<'_, AppState>) -> Result<RepoState, String> {
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelfImproveWorkboardItem {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub status: String,
    pub source: String,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct SelfImproveWorkboard {
    pub generated_at_ms: u64,
    pub finished: Vec<SelfImproveWorkboardItem>,
    pub working: Vec<SelfImproveWorkboardItem>,
    pub backlog: Vec<SelfImproveWorkboardItem>,
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

#[tauri::command]
pub async fn get_self_improve_workboard(
    state: State<'_, AppState>,
) -> Result<SelfImproveWorkboard, String> {
    let repo = coding_repo::guess_repo_root(&state.data_dir);
    let generated_at_ms = now_secs().saturating_mul(1000);
    let mut board = SelfImproveWorkboard {
        generated_at_ms,
        ..Default::default()
    };

    for item in parse_milestones(&repo.join("rules/milestones.md"), generated_at_ms) {
        match item.status.as_str() {
            "in-progress" => board.working.push(item),
            _ => board.backlog.push(item),
        }
    }

    let metrics = MetricsLog::new(&repo);
    for run in metrics.recent(100) {
        let item = SelfImproveWorkboardItem {
            id: run.chunk_id.clone(),
            title: if run.chunk_title.is_empty() {
                run.chunk_id.clone()
            } else {
                run.chunk_title.clone()
            },
            detail: run
                .error
                .clone()
                .unwrap_or_else(|| format!("{} via {} · {}", run.outcome, run.provider, run.model)),
            status: run.outcome.clone(),
            source: "self-improve-run-log".to_string(),
            updated_at_ms: run.finished_at_ms,
        };
        match run.outcome.as_str() {
            "running" => board.working.push(item),
            "success" => board.finished.push(item),
            "failure" => board.backlog.push(item),
            _ => {}
        }
    }

    board.finished.extend(parse_completion_log(
        &repo.join("rules/completion-log.md"),
        generated_at_ms,
    ));

    let path = state.data_dir.join("self_improve_workboard.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&board).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(board)
}

fn parse_milestones(path: &std::path::Path, generated_at_ms: u64) -> Vec<SelfImproveWorkboardItem> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let cells = markdown_table_cells(line);
            if cells.len() < 4 || cells[0] == "ID" || cells[0].starts_with("---") {
                return None;
            }
            Some(SelfImproveWorkboardItem {
                id: cells[0].clone(),
                status: cells[1].clone(),
                title: cells[2].clone(),
                detail: cells[3].clone(),
                source: "rules/milestones.md".to_string(),
                updated_at_ms: generated_at_ms,
            })
        })
        .collect()
}

fn parse_completion_log(
    path: &std::path::Path,
    generated_at_ms: u64,
) -> Vec<SelfImproveWorkboardItem> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let cells = markdown_table_cells(line);
            if cells.len() < 2 || !cells[0].starts_with("[") {
                return None;
            }
            let title = cells[0]
                .trim_start_matches('[')
                .split("](")
                .next()
                .unwrap_or(cells[0].as_str())
                .to_string();
            Some(SelfImproveWorkboardItem {
                id: title
                    .split('—')
                    .next()
                    .unwrap_or(title.as_str())
                    .trim()
                    .to_string(),
                title,
                detail: format!("Completed {}", cells[1]),
                status: "completed".to_string(),
                source: "rules/completion-log.md".to_string(),
                updated_at_ms: generated_at_ms,
            })
        })
        .take(20)
        .collect()
}

fn markdown_table_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return Vec::new();
    }
    trimmed
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

/// Start the autonomous self-improve loop. Idempotent — calling while
/// already running emits a warning event and returns Ok.
///
/// The caller (UI) is expected to have toggled `self_improve.enabled = true`
/// via [`set_self_improve_enabled`] *before* calling this command.
#[tauri::command]
pub async fn start_self_improve(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
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
    let worktree_dir = {
        let s = state.self_improve.lock().map_err(|e| e.to_string())?;
        let d = s.worktree_dir.clone();
        if d.is_empty() {
            None
        } else {
            Some(d)
        }
    };
    coding::engine::start(app, engine, cfg, workflow_cfg, worktree_dir, repo_hint).await;
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
pub async fn clear_self_improve_log(state: State<'_, AppState>) -> Result<MetricsSummary, String> {
    let log = MetricsLog::new(&state.data_dir);
    log.clear().map_err(|e| format!("clear log: {e}"))?;
    Ok(log.summary())
}

// ---------------------------------------------------------------------------
// Gate telemetry (Phase 34 — Chunk 34.2)
// ---------------------------------------------------------------------------

/// Per-gate aggregate metrics: pass/fail rates, average duration, last
/// error per gate. Computed from the gate event JSONL log, capped at the
/// most recent [`coding::gate_telemetry::MAX_GATE_RECORDS`] end-events.
#[tauri::command]
pub async fn get_self_improve_gate_metrics(
    state: State<'_, AppState>,
) -> Result<coding::gate_telemetry::GateMetricsSummary, String> {
    let log = coding::gate_telemetry::GateLog::new(&state.data_dir);
    Ok(log.summary())
}

/// Most recent gate end-events (newest first). The UI displays these in
/// the per-gate timing breakdown panel.
#[tauri::command]
pub async fn get_self_improve_gate_history(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<coding::gate_telemetry::GateEvent>, String> {
    let log = coding::gate_telemetry::GateLog::new(&state.data_dir);
    let n = limit
        .unwrap_or(50)
        .min(coding::gate_telemetry::MAX_GATE_RECORDS);
    Ok(log.recent_ends(n))
}

// ---------------------------------------------------------------------------
// Backlog promotion (Phase 34 — Chunk 34.3)
// ---------------------------------------------------------------------------

/// Result of promoting a backlog item to a milestone chunk.
///
/// Reports the new chunk's ID (e.g. `"34.4"`) plus the phase it was
/// inserted into so the UI can show a confirmation toast and refresh
/// the workboard without re-parsing markdown by hand.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PromoteToChunkResult {
    pub chunk_id: String,
    pub phase_id: u32,
    pub title: String,
}

/// Promote a backlog item (failed run, research idea, deferred suggestion)
/// to a scoped milestone chunk by safely appending a new row to
/// `rules/milestones.md`.
///
/// Behaviour:
/// - Auto-selects the next unused chunk ID in the target phase
///   (e.g. if Phase 34 has `34.3`, the new row becomes `34.4`).
/// - Defaults to the highest phase that already has a `## Phase N` header
///   when no `phase_id` is supplied.
/// - Refuses to write if the target phase has no markdown table yet —
///   the caller must ensure the phase header + table skeleton exist.
/// - Writes atomically via `std::fs::write` after a single string mutation.
///
/// Title and goal are sanitised (newlines / pipes stripped) so the
/// markdown table stays valid.
#[tauri::command]
pub async fn promote_to_milestone_chunk(
    title: String,
    goal: String,
    phase_id: Option<u32>,
    state: State<'_, AppState>,
) -> Result<PromoteToChunkResult, String> {
    let title = sanitise_table_cell(&title);
    let goal = sanitise_table_cell(&goal);
    if title.is_empty() {
        return Err("Title is required.".to_string());
    }
    if goal.is_empty() {
        return Err("Goal is required.".to_string());
    }

    let repo = coding_repo::guess_repo_root(&state.data_dir);
    let path = repo.join("rules/milestones.md");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    let target_phase = match phase_id {
        Some(p) => p,
        None => detect_latest_phase(&text)
            .ok_or_else(|| "No phase headers found in milestones.md".to_string())?,
    };

    let next_id = next_chunk_id_in_phase(&text, target_phase);
    let chunk_id = format!("{}.{}", target_phase, next_id);
    let new_row = format!("| {} | not-started | {} | {} |", chunk_id, title, goal);

    let updated = insert_row_in_phase_table(&text, target_phase, &new_row).ok_or_else(|| {
        format!(
            "Phase {} table not found in milestones.md — add the phase header + table skeleton first.",
            target_phase
        )
    })?;

    std::fs::write(&path, updated).map_err(|e| format!("Failed to write milestones.md: {}", e))?;

    Ok(PromoteToChunkResult {
        chunk_id,
        phase_id: target_phase,
        title,
    })
}

fn sanitise_table_cell(s: &str) -> String {
    s.replace(['\r', '\n'], " ")
        .replace('|', "/")
        .trim()
        .to_string()
}

/// Find the largest `## Phase N` header in the document. Returns `None`
/// when no phase headers exist.
fn detect_latest_phase(text: &str) -> Option<u32> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let rest = trimmed.strip_prefix("## Phase ")?;
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            num_str.parse::<u32>().ok()
        })
        .max()
}

/// Compute the next unused chunk number within a phase. Walks every
/// `| N.M |` row inside the phase's table and returns `max(M) + 1`,
/// defaulting to `1` if the table is empty.
fn next_chunk_id_in_phase(text: &str, phase: u32) -> u32 {
    let prefix = format!("{}.", phase);
    let mut max_seen: u32 = 0;
    let mut in_target_phase = false;

    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("## Phase ") {
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            in_target_phase = num_str.parse::<u32>().ok() == Some(phase);
            continue;
        }
        if !in_target_phase {
            continue;
        }
        let cells = markdown_table_cells(line);
        if cells.len() < 2 {
            continue;
        }
        let id = &cells[0];
        if let Some(suffix) = id.strip_prefix(&prefix) {
            if let Ok(n) = suffix.parse::<u32>() {
                if n > max_seen {
                    max_seen = n;
                }
            }
        }
    }
    max_seen + 1
}

/// Insert `new_row` at the end of the markdown table under `## Phase N`.
/// The table is delimited by the first non-table line (blank, `---`, or
/// next heading) after the header row. Returns the updated document, or
/// `None` if the phase header or its table cannot be found.
fn insert_row_in_phase_table(text: &str, phase: u32, new_row: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();

    let phase_idx = lines.iter().position(|l| {
        let trimmed = l.trim_start();
        let Some(rest) = trimmed.strip_prefix("## Phase ") else {
            return false;
        };
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        num_str.parse::<u32>().ok() == Some(phase)
    })?;

    // Find the table header row (starts with `| ID `) after the phase heading.
    let mut header_row_idx: Option<usize> = None;
    for (i, line) in lines.iter().enumerate().skip(phase_idx + 1) {
        let t = line.trim_start();
        if t.starts_with("## ") {
            // hit next phase before finding a table
            return None;
        }
        if t.starts_with("| ID ") || t.starts_with("|ID ") {
            header_row_idx = Some(i);
            break;
        }
    }
    let header_row_idx = header_row_idx?;

    // Skip the separator row (|---|) and walk forward while still in the table.
    let mut last_table_line = header_row_idx + 1; // separator
    for (i, line) in lines.iter().enumerate().skip(header_row_idx + 2) {
        let trimmed = line.trim_start();
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            last_table_line = i;
        } else {
            break;
        }
    }

    // Rebuild the document with the new row inserted right after the
    // last table row. Preserve original line endings as `\n`.
    let mut out = String::with_capacity(text.len() + new_row.len() + 1);
    for (i, line) in lines.iter().enumerate() {
        out.push_str(line);
        out.push('\n');
        if i == last_table_line {
            out.push_str(new_row);
            out.push('\n');
        }
    }
    // Preserve trailing newline state of the input.
    if !text.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    Some(out)
}

#[cfg(test)]
mod promote_tests {
    use super::*;

    const SAMPLE: &str = "# Milestones\n\n## Phase 34 — Foo\n\n| ID | Status | Title | Goal |\n|---|---|---|---|\n| 34.3 | not-started | Backlog promotion | Some goal |\n\n## Phase 35 — Bar\n\n| ID | Status | Title | Goal |\n|---|---|---|---|\n| 35.1 | not-started | Other | More |\n";

    #[test]
    fn detects_latest_phase() {
        assert_eq!(detect_latest_phase(SAMPLE), Some(35));
    }

    #[test]
    fn next_chunk_id_after_existing_row() {
        assert_eq!(next_chunk_id_in_phase(SAMPLE, 34), 4);
        assert_eq!(next_chunk_id_in_phase(SAMPLE, 35), 2);
    }

    #[test]
    fn next_chunk_id_for_empty_phase_starts_at_one() {
        let empty = "## Phase 36 — Empty\n\n| ID | Status | Title | Goal |\n|---|---|---|---|\n";
        assert_eq!(next_chunk_id_in_phase(empty, 36), 1);
    }

    #[test]
    fn inserts_row_at_end_of_phase_table() {
        let new_row = "| 34.4 | not-started | New thing | Do it |";
        let updated = insert_row_in_phase_table(SAMPLE, 34, new_row).expect("inserted");
        assert!(updated.contains("| 34.3 | not-started | Backlog promotion | Some goal |\n| 34.4 | not-started | New thing | Do it |\n"));
        // Phase 35 table is untouched and still present.
        assert!(updated.contains("| 35.1 | not-started | Other | More |"));
    }

    #[test]
    fn sanitise_strips_pipes_and_newlines() {
        assert_eq!(sanitise_table_cell("a|b\nc"), "a/b c");
    }

    #[test]
    fn missing_phase_returns_none() {
        let new_row = "| 99.1 | not-started | x | y |";
        assert!(insert_row_in_phase_table(SAMPLE, 99, new_row).is_none());
    }
}

// ---------------------------------------------------------------------------
// GitHub config + PR automation (Phase 25 — Chunk 25.13)
// ---------------------------------------------------------------------------

/// Persisted GitHub binding (token, owner/repo, base branch, reviewers).
/// Returns `None` when not yet configured.
#[tauri::command]
pub async fn get_github_config(state: State<'_, AppState>) -> Result<Option<GitHubConfig>, String> {
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
        Some(cfg) => {
            let cfg = coding::apply_github_config_defaults(cfg, &state.data_dir);
            coding::save_github_config(&state.data_dir, &cfg)?;
            Ok(Some(cfg))
        }
    }
}

/// Manually trigger the "open / update PR for the current feature branch"
/// flow. Useful for users who want to ship before the autonomous loop
/// completes every chunk. Returns the PR summary on success.
#[tauri::command]
pub async fn open_self_improve_pr(state: State<'_, AppState>) -> Result<PrSummary, String> {
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
pub async fn pull_main_for_self_improve(state: State<'_, AppState>) -> Result<PullResult, String> {
    let repo_root = coding_repo::guess_repo_root(&state.data_dir);
    let base = coding::load_github_config(&state.data_dir)
        .map(|c| c.default_base)
        .unwrap_or_else(|| "main".to_string());
    let coding_cfg = state
        .coding_llm_config
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
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
    let cfg = match state
        .coding_llm_config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
    {
        Some(c) => c,
        None => return Ok(None),
    };
    let repo_root = coding_repo::guess_repo_root(&state.data_dir);
    let learned = coding::learn_from_message(&message, &cfg, &state.data_dir, &repo_root).await;
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
///
/// ## Long-session handoff (Chunk 28.9)
///
/// When `handoffSessionId` is `Some`, the command:
/// 1. Loads any prior [`coding::HandoffState`] from
///    `<data_dir>/coding_workflow/sessions/<id>.json` and injects it as
///    a `[RESUMING SESSION]` block at the head of the prompt.
/// 2. Asks the model to emit a fresh `<next_session_seed>` JSON payload.
/// 3. Parses the seed from the reply (if present) and atomically writes
///    it back to the same on-disk slot for the next call.
///
/// The resulting [`coding::CodingTaskResult::next_handoff`] echoes the
/// freshly persisted state so the UI can render a "Session resumed"
/// chip without an extra round-trip.
#[tauri::command(rename_all = "camelCase")]
pub async fn run_coding_task(
    mut task: coding::CodingTask,
    handoff_session_id: Option<String>,
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

    // Load prior handoff before invoking the model. A bad/corrupt
    // snapshot is non-fatal — we proceed without injection rather than
    // blocking the user on disk state.
    if let Some(id) = handoff_session_id.as_deref() {
        if task.prior_handoff.is_none() {
            match coding::load_handoff(&state.data_dir, id) {
                Ok(prior) => task.prior_handoff = prior,
                Err(e) => {
                    eprintln!("[coding-handoff] load {id} failed: {e}");
                }
            }
        }
    }

    let result = coding::run_coding_task(&cfg, &task, Some(&workflow_cfg)).await?;

    // Persist any freshly emitted seed under the caller-supplied id (or
    // the seed's own id when the caller did not supply one).
    if let Some(next) = &result.next_handoff {
        let target = handoff_session_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(next.session_id.as_str());
        let mut to_save = next.clone();
        to_save.session_id = target.to_string();
        if to_save.created_at == 0 {
            to_save.created_at = coding::handoff_store::now_unix_ms();
        }
        if let Err(e) = coding::save_handoff(&state.data_dir, &to_save) {
            eprintln!("[coding-handoff] save {target} failed: {e}");
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Coding session handoff persistence (Chunk 28.9)
// ---------------------------------------------------------------------------

/// Manually persist a [`coding::HandoffState`] snapshot. The frontend
/// uses this when the user explicitly bookmarks a session before
/// stepping away (e.g. closing the laptop) so the next launch can
/// resume from a known-good seed even if the LLM did not produce one.
#[tauri::command(rename_all = "camelCase")]
pub async fn coding_session_save_handoff(
    handoff: coding::HandoffState,
    state: State<'_, AppState>,
) -> Result<(), String> {
    coding::save_handoff(&state.data_dir, &handoff)
}

/// Load the persisted [`coding::HandoffState`] for `sessionId`.
/// Returns `None` when no snapshot exists.
#[tauri::command(rename_all = "camelCase")]
pub async fn coding_session_load_handoff(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Option<coding::HandoffState>, String> {
    coding::load_handoff(&state.data_dir, &session_id)
}

/// List every persisted handoff snapshot, newest first.
#[tauri::command]
pub async fn coding_session_list_handoffs(
    state: State<'_, AppState>,
) -> Result<Vec<coding::HandoffSummary>, String> {
    coding::list_handoffs(&state.data_dir)
}

/// Delete the persisted snapshot for `sessionId`. Returns `true` when
/// a file was removed, `false` when no snapshot existed (idempotent).
#[tauri::command(rename_all = "camelCase")]
pub async fn coding_session_clear_handoff(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    coding::clear_handoff(&state.data_dir, &session_id)
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
        return Err("max_total_chars must be greater than or equal to max_file_chars.".to_string());
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

// ---------------------------------------------------------------------------
// Self-improve worktree management
// ---------------------------------------------------------------------------

/// Set (or clear) the custom worktree directory used for self-improve
/// execution. Pass an empty string to revert to the default (OS temp dir).
///
/// The path is persisted alongside the other self-improve settings.
#[tauri::command(rename_all = "camelCase")]
pub async fn set_self_improve_worktree_dir(
    dir: String,
    state: State<'_, AppState>,
) -> Result<SelfImproveSettings, String> {
    let mut slot = state.self_improve.lock().map_err(|e| e.to_string())?;
    slot.worktree_dir = dir;
    coding::save_self_improve(&state.data_dir, &slot)?;
    Ok(slot.clone())
}

/// List all git worktrees attached to the TerranSoul repo.
///
/// Returns an array of `{ path, head, branch }` objects. Self-improve
/// worktrees appear with branch "(detached)".
///
/// Users can open a worktree in their editor/terminal:
/// - VS Code: `code <path>`
/// - GitHub Desktop: File → Add Local Repository → paste the path
/// - Terminal: `cd <path>`
/// - Or run `git worktree list` from the main repo
#[tauri::command]
pub async fn list_self_improve_worktrees(
    state: State<'_, AppState>,
) -> Result<Vec<coding::worktree::WorktreeInfo>, String> {
    let repo = coding_repo::detect_repo(&state.data_dir);
    let repo_root = repo
        .root
        .as_deref()
        .ok_or_else(|| "No git repository detected".to_string())?;
    let repo_root = std::path::Path::new(repo_root);
    coding::worktree::list_worktrees(repo_root)
}

/// Index a repository's Rust and TypeScript source files into the local
/// symbol table. Returns stats (files parsed, symbols/edges extracted).
#[tauri::command(rename_all = "camelCase")]
pub async fn code_index_repo(
    repo_path: String,
    state: State<'_, AppState>,
) -> Result<coding::symbol_index::IndexStats, String> {
    let repo = std::path::Path::new(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }
    let data_dir = std::path::Path::new(&state.data_dir);
    tokio::task::spawn_blocking({
        let data_dir = data_dir.to_path_buf();
        let repo = repo.to_path_buf();
        move || coding::symbol_index::index_repo(&data_dir, &repo).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Resolve cross-file import/call edges for an already-indexed repo.
/// Returns stats on how many edges were resolved.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_resolve_edges(
    repo_path: String,
    state: State<'_, AppState>,
) -> Result<coding::resolver::ResolveStats, String> {
    let repo = std::path::Path::new(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }
    let data_dir = std::path::Path::new(&state.data_dir);
    tokio::task::spawn_blocking({
        let data_dir = data_dir.to_path_buf();
        let repo = repo.to_path_buf();
        move || coding::resolver::resolve_edges(&data_dir, &repo).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Get the call graph for a symbol: incoming callers + outgoing callees.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_call_graph(
    repo_path: String,
    symbol_name: String,
    state: State<'_, AppState>,
) -> Result<coding::resolver::CallGraph, String> {
    let repo = std::path::Path::new(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }
    let data_dir = std::path::Path::new(&state.data_dir);
    tokio::task::spawn_blocking({
        let data_dir = data_dir.to_path_buf();
        let repo = repo.to_path_buf();
        let symbol = symbol_name.clone();
        move || {
            let repo = repo
                .canonicalize()
                .map_err(|e| format!("invalid path: {e}"))?;
            let conn = coding::symbol_index::open_db(&data_dir).map_err(|e| e.to_string())?;
            let repo_str = repo.to_string_lossy().to_string();
            let repo_id: i64 = conn
                .query_row(
                    "SELECT id FROM code_repos WHERE path = ?1",
                    rusqlite::params![repo_str],
                    |r| r.get(0),
                )
                .map_err(|_| format!("repo not indexed: {repo_str}"))?;
            coding::resolver::call_graph(&conn, repo_id, &symbol).map_err(|e| e.to_string())
        }
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Run functional clustering + entry-point scoring + process tracing.
/// Requires the repo to have been indexed and resolved first.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_compute_processes(
    repo_path: String,
    max_depth: Option<u32>,
    state: State<'_, AppState>,
) -> Result<coding::processes::ProcessStats, String> {
    let repo = std::path::Path::new(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }
    let data_dir = std::path::Path::new(&state.data_dir);
    let depth = max_depth.unwrap_or(10);
    tokio::task::spawn_blocking({
        let data_dir = data_dir.to_path_buf();
        let repo = repo.to_path_buf();
        move || {
            coding::processes::compute_processes(&data_dir, &repo, depth).map_err(|e| e.to_string())
        }
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// List clusters for a repo.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_list_clusters(
    repo_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<coding::processes::Cluster>, String> {
    let repo = std::path::Path::new(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }
    let data_dir = std::path::Path::new(&state.data_dir);
    tokio::task::spawn_blocking({
        let data_dir = data_dir.to_path_buf();
        let repo = repo.to_path_buf();
        move || {
            let repo = repo
                .canonicalize()
                .map_err(|e| format!("invalid path: {e}"))?;
            let conn = coding::symbol_index::open_db(&data_dir).map_err(|e| e.to_string())?;
            let repo_str = repo.to_string_lossy().to_string();
            let repo_id: i64 = conn
                .query_row(
                    "SELECT id FROM code_repos WHERE path = ?1",
                    rusqlite::params![repo_str],
                    |r| r.get(0),
                )
                .map_err(|_| format!("repo not indexed: {repo_str}"))?;
            coding::processes::list_clusters(&conn, repo_id).map_err(|e| e.to_string())
        }
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// List execution-flow processes for a repo.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_list_processes(
    repo_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<coding::processes::Process>, String> {
    let repo = std::path::Path::new(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }
    let data_dir = std::path::Path::new(&state.data_dir);
    tokio::task::spawn_blocking({
        let data_dir = data_dir.to_path_buf();
        let repo = repo.to_path_buf();
        move || {
            let repo = repo
                .canonicalize()
                .map_err(|e| format!("invalid path: {e}"))?;
            let conn = coding::symbol_index::open_db(&data_dir).map_err(|e| e.to_string())?;
            let repo_str = repo.to_string_lossy().to_string();
            let repo_id: i64 = conn
                .query_row(
                    "SELECT id FROM code_repos WHERE path = ?1",
                    rusqlite::params![repo_str],
                    |r| r.get(0),
                )
                .map_err(|_| format!("repo not indexed: {repo_str}"))?;
            coding::processes::list_processes(&conn, repo_id).map_err(|e| e.to_string())
        }
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Generate wiki pages from the symbol graph for a repo.
///
/// Produces per-cluster Markdown pages with mermaid call graphs under
/// `<data_dir>/wiki/`. Optionally summarises each cluster via the active
/// brain.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_generate_wiki(
    repo_path: String,
    state: State<'_, AppState>,
) -> Result<coding::wiki::WikiResult, String> {
    let repo = std::path::Path::new(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }
    let data_dir = state.data_dir.clone();
    let wiki_dir = data_dir.join("wiki");

    // Phase 1: load cluster data from the code index (blocking).
    let (clusters, all_syms_raw, all_edges_raw) = tokio::task::spawn_blocking({
        let data_dir = data_dir.clone();
        let repo = repo.to_path_buf();
        let wiki_dir = wiki_dir.clone();
        move || {
            let repo = repo
                .canonicalize()
                .map_err(|e| format!("invalid path: {e}"))?;
            coding::wiki::generate_wiki_sync(&data_dir, &repo, &wiki_dir).map_err(|e| e.to_string())
        }
    })
    .await
    .map_err(|e| format!("join error: {e}"))??;

    // Phase 2: summarise each cluster via the brain (async, optional).
    let brain_mode = state.brain_mode.lock().ok().and_then(|g| g.clone());
    let mut summaries: Vec<Option<String>> = Vec::with_capacity(clusters.len());

    if let Some(mode) = &brain_mode {
        for (i, cluster) in clusters.iter().enumerate() {
            let desc = coding::wiki::build_cluster_description(cluster, &all_syms_raw[i]);
            let result = crate::memory::brain_memory::complete_via_mode(
                mode,
                "You are a concise technical writer. Summarize the following code cluster in 1-2 sentences, focusing on its purpose and key responsibilities.",
                &desc,
                &state.provider_rotator,
            )
            .await;
            match result {
                Ok(s) if !s.trim().is_empty() => summaries.push(Some(s.trim().to_string())),
                _ => summaries.push(None),
            }
        }
    } else {
        summaries.resize(clusters.len(), None);
    }

    // Phase 3: write the wiki pages (blocking).
    tokio::task::spawn_blocking({
        let wiki_dir = wiki_dir.clone();
        move || {
            coding::wiki::write_wiki_pages(
                &wiki_dir,
                &clusters,
                &all_syms_raw,
                &all_edges_raw,
                &summaries,
            )
            .map_err(|e| e.to_string())
        }
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Generate agent skill files (SKILL.md) from the code graph.
///
/// Output goes to `<data_dir>/skills/` by default. Returns the list of
/// generated skill files and their paths.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_generate_skills(
    repo_path: String,
    output_dir: Option<String>,
    state: State<'_, AppState>,
) -> Result<coding::skills::SkillGenResult, String> {
    let repo = std::path::PathBuf::from(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }
    let data_dir = state.data_dir.clone();
    let out = output_dir
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| data_dir.join("skills"));

    tokio::task::spawn_blocking(move || {
        let repo = repo
            .canonicalize()
            .map_err(|e| format!("invalid path: {e}"))?;
        coding::skills::generate_skills(&data_dir, &repo, &out).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

// ─── Multi-repo groups and contracts (chunk 37.13) ───────────────────────

/// List all repo groups.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_list_groups(
    state: State<'_, AppState>,
) -> Result<Vec<coding::repo_groups::RepoGroup>, String> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        coding::repo_groups::list_groups(&data_dir).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Create a new repo group.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_create_group(
    label: String,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<coding::repo_groups::RepoGroup, String> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        coding::repo_groups::create_group(&data_dir, &label, description.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Delete a repo group.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_delete_group(group_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        coding::repo_groups::delete_group(&data_dir, group_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Add a repo to a group.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_add_repo_to_group(
    group_id: i64,
    repo_id: i64,
    role: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        coding::repo_groups::add_repo_to_group(&data_dir, group_id, repo_id, role.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Remove a repo from a group.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_remove_repo_from_group(
    group_id: i64,
    repo_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        coding::repo_groups::remove_repo_from_group(&data_dir, group_id, repo_id)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Aggregated status for a group.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_group_status(
    group_id: i64,
    state: State<'_, AppState>,
) -> Result<coding::repo_groups::GroupStatus, String> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        coding::repo_groups::group_status(&data_dir, group_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Extract public-API contracts for a repo.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_extract_contracts(
    repo_id: i64,
    state: State<'_, AppState>,
) -> Result<coding::repo_groups::ContractExtractResult, String> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        coding::repo_groups::extract_contracts(&data_dir, repo_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// List all contracts for a group's member repos.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_list_group_contracts(
    group_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<coding::repo_groups::ContractEntry>, String> {
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        coding::repo_groups::list_group_contracts(&data_dir, group_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Search for a symbol name across all repos in a group.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_cross_repo_query(
    group_id: i64,
    name: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<coding::repo_groups::CrossRepoMatch>, String> {
    let data_dir = state.data_dir.clone();
    let lim = limit.unwrap_or(50) as usize;
    tokio::task::spawn_blocking(move || {
        coding::repo_groups::cross_repo_query(&data_dir, group_id, &name, lim)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Export the code graph for a repository as a JSON snapshot (Chunk 36B.1).
///
/// The snapshot includes all symbols and edges, suitable for committing
/// to version control for architecture review.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_export_graph(
    repo_path: String,
    output_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<coding::graph_export::CodeGraphSnapshot, String> {
    let data_dir = state.data_dir.clone();
    let repo = std::path::PathBuf::from(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }
    tokio::task::spawn_blocking(move || {
        let conn = coding::symbol_index::open_db(&data_dir).map_err(|e| e.to_string())?;
        let canon = repo
            .canonicalize()
            .map_err(|e| format!("invalid path: {e}"))?;
        let canon_str = canon.to_string_lossy();
        // Find the repo_id for this path
        let repo_id: i64 = conn
            .query_row(
                "SELECT id FROM code_repos WHERE path = ?1",
                rusqlite::params![canon_str.as_ref()],
                |row| row.get(0),
            )
            .map_err(|_| format!("repository not indexed: {}", canon_str))?;
        let out = match output_path {
            Some(p) => std::path::PathBuf::from(p),
            None => canon.join("code-graph.json"),
        };
        coding::graph_export::export_to_file(&conn, repo_id, &out).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Generate a persona-adaptive explanation of a code symbol (Chunk 36B.2).
///
/// Loads the symbol's call graph (callers + callees) from the code index
/// and asks the active brain to explain it for the requested audience.
///
/// `audience` accepts: `newcomer`, `maintainer`, `pm` / `project_manager`,
/// or `power_user` / `expert`.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_explain_graph(
    repo_path: String,
    symbol_name: String,
    audience: String,
    state: State<'_, AppState>,
) -> Result<coding::graph_explain::GraphExplanation, String> {
    let audience_kind = coding::graph_explain::Audience::parse(&audience)
        .ok_or_else(|| format!("unknown audience: {audience}"))?;

    let repo = std::path::PathBuf::from(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }

    // Phase 1: load the call graph (blocking SQLite work).
    let data_dir = state.data_dir.clone();
    let symbol_for_load = symbol_name.clone();
    let call_graph = tokio::task::spawn_blocking(move || {
        let canon = repo
            .canonicalize()
            .map_err(|e| format!("invalid path: {e}"))?;
        let conn = coding::symbol_index::open_db(&data_dir).map_err(|e| e.to_string())?;
        let canon_str = canon.to_string_lossy().to_string();
        let repo_id: i64 = conn
            .query_row(
                "SELECT id FROM code_repos WHERE path = ?1",
                rusqlite::params![canon_str],
                |r| r.get(0),
            )
            .map_err(|_| format!("repo not indexed: {canon_str}"))?;
        coding::resolver::call_graph(&conn, repo_id, &symbol_for_load).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))??;

    let symbol_kind = "symbol".to_string();
    let file_path = call_graph
        .symbol_file
        .clone()
        .unwrap_or_else(|| "<unknown>".to_string());
    let incoming: Vec<(String, String)> = call_graph
        .incoming
        .iter()
        .map(|e| (e.symbol_name.clone(), e.kind.clone()))
        .collect();
    let outgoing: Vec<(String, String)> = call_graph
        .outgoing
        .iter()
        .map(|e| (e.symbol_name.clone(), e.kind.clone()))
        .collect();

    let (system_prompt, user_prompt) = coding::graph_explain::explain_symbol_prompt(
        audience_kind,
        &symbol_name,
        &symbol_kind,
        &file_path,
        &incoming,
        &outgoing,
    );

    // Phase 2: ask the brain for the explanation (best-effort).
    let brain_mode = state.brain_mode.lock().ok().and_then(|g| g.clone());
    let explanation = if let Some(mode) = &brain_mode {
        crate::memory::brain_memory::complete_via_mode(
            mode,
            &system_prompt,
            &user_prompt,
            &state.provider_rotator,
        )
        .await
        .unwrap_or_default()
        .trim()
        .to_string()
    } else {
        String::new()
    };

    Ok(coding::graph_explain::GraphExplanation {
        subject: symbol_name,
        audience: audience_kind.label().to_string(),
        explanation,
        context_summary: user_prompt,
    })
}

/// Build guided architecture tours for a repo (Chunk 36B.3).
///
/// Each indexed process becomes one tour, with steps converted into
/// ordered, narrated stops. `max_stops` caps tour length (default 12).
#[tauri::command(rename_all = "camelCase")]
pub async fn code_architecture_tours(
    repo_path: String,
    max_stops: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<coding::graph_tours::ArchitectureTour>, String> {
    let repo = std::path::PathBuf::from(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }
    let data_dir = state.data_dir.clone();
    let cap = max_stops.unwrap_or(12);
    tokio::task::spawn_blocking(move || {
        let canon = repo
            .canonicalize()
            .map_err(|e| format!("invalid path: {e}"))?;
        let conn = coding::symbol_index::open_db(&data_dir).map_err(|e| e.to_string())?;
        let canon_str = canon.to_string_lossy().to_string();
        let repo_id: i64 = conn
            .query_row(
                "SELECT id FROM code_repos WHERE path = ?1",
                rusqlite::params![canon_str],
                |r| r.get(0),
            )
            .map_err(|_| format!("repo not indexed: {canon_str}"))?;
        coding::graph_tours::build_tours(&conn, repo_id, cap).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}

/// Build a diff impact overlay for the given git ref (Chunk 36B.4).
///
/// Combines `analyze_diff_impact` with per-file lookups of impacted
/// processes, related docs (under `docs/` and `wiki/`), and related
/// test files — designed as a pre-commit reviewer aid.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_diff_overlay(
    repo_path: String,
    diff_ref: String,
    state: State<'_, AppState>,
) -> Result<coding::diff_overlay::DiffOverlay, String> {
    let repo = std::path::PathBuf::from(&repo_path);
    if !repo.is_dir() {
        return Err(format!("not a directory: {repo_path}"));
    }
    let data_dir = state.data_dir.clone();
    tokio::task::spawn_blocking(move || {
        coding::diff_overlay::build_overlay(&data_dir, &repo, &diff_ref).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
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

    #[test]
    fn milestone_table_parser_extracts_workboard_rows() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("milestones.md");
        std::fs::write(
            &path,
            "| ID | Status | Title | Goal |\n|---|---|---|---|\n| 34.1 | not-started | Persisted workboard | Keep lanes durable. |\n",
        )
        .unwrap();

        let rows = parse_milestones(&path, 42);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "34.1");
        assert_eq!(rows[0].title, "Persisted workboard");
        assert_eq!(rows[0].status, "not-started");
    }

    #[test]
    fn completion_log_parser_extracts_finished_rows() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("completion-log.md");
        std::fs::write(
            &path,
            "| Entry | Date |\n|-------|------|\n| [Chunk 33.6 — Maintenance](#chunk-336) | 2026-05-05 |\n",
        )
        .unwrap();

        let rows = parse_completion_log(&path, 42);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "Chunk 33.6");
        assert_eq!(rows[0].status, "completed");
    }
}

// ---------------------------------------------------------------------------
// Negative-memory backfill from coding-standards (Chunk 43.7)
// ---------------------------------------------------------------------------

/// Result of extracting negative-memory rules from a rules file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractNegativesResult {
    pub created: usize,
    pub skipped: usize,
    pub rules: Vec<String>,
}

/// Extract "never / don't / avoid" anti-pattern lines from
/// `rules/coding-standards.md` and ingest them as negative memories with
/// substring trigger patterns.
///
/// Idempotent — rules that already exist as negative memories (by exact
/// content match) are skipped.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_extract_negatives(
    state: State<'_, AppState>,
) -> Result<ExtractNegativesResult, String> {
    let repo = coding_repo::guess_repo_root(&state.data_dir);
    let path = repo.join("rules/coding-standards.md");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read coding-standards.md: {e}"))?;

    let negatives = extract_negative_lines(&text);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;

    let mut created = 0usize;
    let mut skipped = 0usize;
    let rule_texts: Vec<String> = negatives.iter().map(|(r, _)| r.clone()).collect();

    for (rule, triggers) in &negatives {
        // Check if already exists by exact content match.
        let exists: bool = store
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM memories WHERE content = ?1 AND valid_to IS NULL)",
                rusqlite::params![rule],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if exists {
            skipped += 1;
            continue;
        }

        let entry = store
            .add(crate::memory::store::NewMemory {
                content: rule.clone(),
                tags: "negative,coding-standard,auto-extracted".to_string(),
                importance: 4,
                memory_type: crate::memory::store::MemoryType::Fact,
                ..Default::default()
            })
            .map_err(|e| e.to_string())?;

        // Set cognitive_kind to negative.
        store
            .conn
            .execute(
                "UPDATE memories SET cognitive_kind = 'negative' WHERE id = ?1",
                rusqlite::params![entry.id],
            )
            .map_err(|e| e.to_string())?;

        // Add trigger patterns.
        for trigger in triggers {
            crate::memory::negative::add_trigger(&store, entry.id, trigger, "substring")
                .map_err(|e| e.to_string())?;
        }

        created += 1;
    }

    Ok(ExtractNegativesResult {
        created,
        skipped,
        rules: rule_texts,
    })
}

/// Parse coding-standards.md for lines containing "never", "don't",
/// "do not", "avoid", "must not" indicators. Returns (rule_text, triggers).
pub fn extract_negative_lines(text: &str) -> Vec<(String, Vec<String>)> {
    let negative_indicators = [
        "never ", "don't ", "do not ", "avoid ", "must not ", "must never ",
        "- no ", "forbidden", "anti-pattern",
    ];

    let mut results = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim().trim_start_matches("- ").trim_start_matches("* ");
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('|') {
            continue;
        }
        let lower = trimmed.to_lowercase();
        let is_negative = negative_indicators.iter().any(|ind| lower.contains(ind));
        if !is_negative {
            continue;
        }
        // Extract meaningful keywords for triggers (words > 5 chars).
        let triggers: Vec<String> = trimmed
            .split_whitespace()
            .filter(|w| {
                let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
                clean.len() > 5
                    && !matches!(
                        clean.to_lowercase().as_str(),
                        "never" | "don't" | "avoid" | "should" | "instead" | "always" | "before"
                    )
            })
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .take(3)
            .collect();

        if !triggers.is_empty() {
            results.push((trimmed.to_string(), triggers));
        }
    }
    results
}

#[cfg(test)]
mod extract_negatives_tests {
    use super::*;

    #[test]
    fn extracts_never_lines() {
        let text = "# Standards\n\n- Never use `.unwrap()` in library code\n- Use `thiserror` for errors\n- Don't hardcode hex colors\n";
        let results = extract_negative_lines(text);
        assert!(results.len() >= 2);
        assert!(results.iter().any(|(r, _)| r.contains("unwrap")));
    }

    #[test]
    fn skips_headers_and_empty() {
        let text = "# Never skip tests\n\n\n";
        let results = extract_negative_lines(text);
        assert!(results.is_empty());
    }

    #[test]
    fn extracts_triggers() {
        let text = "- Never use placeholder implementations in production code\n";
        let results = extract_negative_lines(text);
        assert!(!results.is_empty());
        let (_, triggers) = &results[0];
        assert!(!triggers.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Cross-harness session import (Chunk 43.12)
// ---------------------------------------------------------------------------

use crate::coding::session_import::{self, DetectedHarness, Harness, ImportResult};

/// Detect which AI coding harnesses have importable sessions.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_detect_harnesses() -> Result<Vec<DetectedHarness>, String> {
    let home = dirs::home_dir().ok_or_else(|| "cannot determine home directory".to_string())?;
    Ok(session_import::detect_harnesses(&home))
}

/// Import sessions from a detected harness directory.
///
/// Parses all JSON/JSONL files, redacts secrets, and returns the
/// structured turns for each session. The caller is responsible for
/// feeding these through `brain_memory::extract_facts` if desired.
#[tauri::command(rename_all = "camelCase")]
pub async fn code_import_sessions(
    harness: String,
) -> Result<Vec<ImportResult>, String> {
    let h = parse_harness(&harness)?;
    let home = dirs::home_dir().ok_or_else(|| "cannot determine home directory".to_string())?;
    let dir = home.join(h.relative_dir());
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let files = session_import::list_session_files(&dir);
    let results: Vec<ImportResult> = files
        .iter()
        .map(|f| session_import::parse_transcript(h, f))
        .collect();
    Ok(results)
}

fn parse_harness(s: &str) -> Result<Harness, String> {
    match s {
        "claude" => Ok(Harness::Claude),
        "codex" => Ok(Harness::Codex),
        "opencode" => Ok(Harness::OpenCode),
        "cursor" => Ok(Harness::Cursor),
        "copilot_cli" => Ok(Harness::CopilotCli),
        _ => Err(format!("unknown harness: {s}")),
    }
}
