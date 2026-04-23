//! Tauri commands for the multi-agent roster — Chunk 1.5.
//!
//! The roster itself lives on disk under `<data_dir>/agents/`. This
//! module exposes CRUD + workflow driving on top of it, and computes the
//! RAM-aware concurrency cap that the frontend consults before
//! activating an agent.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime, State};

use crate::agents::cli_worker::{spawn, CliEvent, CliSpawnSpec, CliStream};
use crate::agents::{default_agent, fresh_id, AgentProfile, AgentRoster, BrainBackend};
#[cfg(test)]
use crate::agents::{AgentBackendKind, CliKind};
use crate::brain::ram_budget::{
    compute_max_concurrent_agents, estimate_agent_mb, free_ram_mb, AgentFootprint, RamCap,
};
use crate::workflows::{WorkflowEngine, WorkflowEventKind, WorkflowStatus, WorkflowSummary};
use crate::AppState;

/// Payload the frontend posts when creating a new agent. `id` is
/// optional — if omitted we generate a fresh UUID-ish id.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAgentRequest {
    pub id: Option<String>,
    pub display_name: String,
    pub vrm_model_id: String,
    pub brain_backend: BrainBackend,
    #[serde(default)]
    pub working_folder: Option<PathBuf>,
}

/// List every agent profile on disk, sorted by last-active desc. If the
/// roster is empty, a "default" agent is lazily created so the app
/// always has at least one usable persona.
#[tauri::command]
pub async fn roster_list(state: State<'_, AppState>) -> Result<Vec<AgentProfile>, String> {
    let roster = AgentRoster::open(&state.data_dir);
    let mut agents = roster.list();
    if agents.is_empty() {
        // Lazy-init on first run so the app is never "agent-less".
        let default_vrm = state
            .app_settings
            .lock()
            .map_err(|e| e.to_string())?
            .selected_model_id
            .clone();
        let default_vrm = if default_vrm.trim().is_empty() {
            "annabelle".to_string()
        } else {
            default_vrm
        };
        let default = default_agent(&default_vrm);
        roster.create(default.clone())?;
        agents.push(default);
    }
    Ok(agents)
}

/// Create a new agent. Fails if [`crate::agents::MAX_AGENTS`] is reached,
/// the ID already exists, or the profile fails validation (see
/// [`AgentProfile::validate`]).
#[tauri::command]
pub async fn roster_create(
    request: CreateAgentRequest,
    state: State<'_, AppState>,
) -> Result<AgentProfile, String> {
    let roster = AgentRoster::open(&state.data_dir);
    let id = request.id.unwrap_or_else(fresh_id);
    let now = crate::agents::roster::now_secs();
    let profile = AgentProfile {
        id,
        display_name: request.display_name,
        vrm_model_id: request.vrm_model_id,
        brain_backend: request.brain_backend,
        working_folder: request.working_folder,
        created_at: now,
        last_active_at: now,
    };
    roster.create(profile.clone())?;
    Ok(profile)
}

/// Delete an agent. Idempotent — deleting a non-existent agent is Ok(()).
#[tauri::command]
pub async fn roster_delete(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let roster = AgentRoster::open(&state.data_dir);
    roster.delete(&id)
}

/// Switch to the given agent. Persists the choice to disk and bumps the
/// agent's `last_active_at` so `list_agents` sorts it first.
#[tauri::command]
pub async fn roster_switch(id: String, state: State<'_, AppState>) -> Result<AgentProfile, String> {
    let roster = AgentRoster::open(&state.data_dir);
    roster.set_current_agent(&id)?;
    roster.touch(&id)?;
    roster.get(&id)
}

/// Read the persisted "current agent" pointer, if any.
#[tauri::command]
pub async fn roster_get_current(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let roster = AgentRoster::open(&state.data_dir);
    roster.current_agent_id()
}

/// Set the working folder for an external-CLI agent. Validates that the
/// folder exists before saving so the roster never carries a dangling
/// path.
#[tauri::command]
pub async fn roster_set_working_folder(
    id: String,
    folder: PathBuf,
    state: State<'_, AppState>,
) -> Result<AgentProfile, String> {
    if !folder.exists() {
        return Err(format!(
            "working folder does not exist: {}",
            folder.display()
        ));
    }
    if !folder.is_dir() {
        return Err(format!(
            "working folder is not a directory: {}",
            folder.display()
        ));
    }
    let roster = AgentRoster::open(&state.data_dir);
    let mut profile = roster.get(&id)?;
    profile.working_folder = Some(folder);
    roster.update(&profile)?;
    Ok(profile)
}

// ── RAM-cap command ──────────────────────────────────────────────────────

/// Report on how many agents may run simultaneously given current free
/// RAM and the candidate footprints of each agent. Called every time
/// the agent picker opens so the user gets a live number.
#[tauri::command]
pub async fn roster_get_ram_cap(state: State<'_, AppState>) -> Result<AgentRamCapReport, String> {
    let roster = AgentRoster::open(&state.data_dir);
    let agents = roster.list();
    let footprints: Vec<AgentFootprint> = agents
        .iter()
        .map(|a| {
            let kind = a.brain_backend.discriminant();
            // For Ollama-backed agents we'd ideally look up the model
            // size, but we don't want to block on the CLI. Use a
            // conservative 2 GB default.
            let ollama_mb = a.brain_backend.ollama_model().map(|_| 2_000_u64);
            AgentFootprint {
                agent_id: a.id.clone(),
                kind,
                estimated_mb: estimate_agent_mb(kind, ollama_mb),
            }
        })
        .collect();
    let free_mb = free_ram_mb();
    let cap = compute_max_concurrent_agents(free_mb, &footprints);
    Ok(AgentRamCapReport {
        cap,
        footprints,
    })
}

/// Public result shape for [`get_agent_ram_cap`].
#[derive(Debug, Clone, Serialize)]
pub struct AgentRamCapReport {
    pub cap: RamCap,
    pub footprints: Vec<AgentFootprint>,
}

// ── Workflow-driven CLI execution ────────────────────────────────────────

/// Frontend request to run an external-CLI agent on a prompt.
#[derive(Debug, Clone, Deserialize)]
pub struct StartCliWorkflowRequest {
    pub agent_id: String,
    pub prompt: String,
}

/// Result of starting a workflow — contains the workflow id the frontend
/// uses to subscribe to updates via [`query_workflow_status`].
#[derive(Debug, Clone, Serialize)]
pub struct StartCliWorkflowResult {
    pub workflow_id: String,
}

/// Start a CLI workflow for the given agent + prompt. Spawns a detached
/// task that appends `ActivityScheduled`/`Heartbeat`/`ActivityCompleted`/
/// `Failed`/`Completed` events to the durable log and emits the same
/// events to the frontend via the `agent-cli-output` channel.
#[tauri::command]
pub async fn roster_start_cli_workflow<R: Runtime>(
    request: StartCliWorkflowRequest,
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<StartCliWorkflowResult, String> {
    let roster = AgentRoster::open(&state.data_dir);
    let profile = roster.get(&request.agent_id)?;

    // Only ExternalCli backends use workflows today. Native brain routes
    // through the existing streaming pipeline.
    let spec = CliSpawnSpec::from_backend(
        &profile.brain_backend,
        profile.working_folder.as_deref(),
        &request.prompt,
    )
    .ok_or_else(|| {
        "agent has no external CLI backend with a working folder configured".to_string()
    })?;
    spec.validate()?;

    // Enforce the RAM cap at the moment of activation.
    let report = roster_get_ram_cap(state.clone()).await?;
    let active_workflows = state
        .workflow_engine
        .lock()
        .await
        .attached_count()
        .await;
    if active_workflows >= report.cap.cap {
        return Err(format!(
            "RAM cap reached — {} workflows already running (cap: {}). Cancel one before starting a new workflow.",
            active_workflows, report.cap.cap
        ));
    }

    let engine = state.workflow_engine.lock().await.handle();
    let workflow_id = engine
        .start(
            "cli_run",
            serde_json::json!({
                "agent_id": profile.id,
                "binary": spec.binary,
                "working_folder": spec.working_folder,
                "prompt": spec.prompt,
            }),
        )
        .await?;

    // Drive the worker in a detached task so the command returns
    // immediately. The task owns the engine handle + AppHandle; if the
    // app shuts down, the engine's event log preserves progress and the
    // RAM-cap calculation will report the workflow as `Resuming` on the
    // next boot.
    let wf_id = workflow_id.clone();
    let app_handle = app.clone();
    tokio::spawn(async move {
        drive_cli_workflow(engine, wf_id, spec, app_handle).await;
    });

    Ok(StartCliWorkflowResult { workflow_id })
}

async fn drive_cli_workflow<R: Runtime>(
    engine: WorkflowEngine,
    workflow_id: String,
    spec: CliSpawnSpec,
    app: AppHandle<R>,
) {
    let mut worker = match spawn(spec).await {
        Ok(w) => w,
        Err(e) => {
            let _ = engine.fail(&workflow_id, &e).await;
            let _ = app.emit(
                "agent-cli-output",
                AgentCliEvent {
                    workflow_id: workflow_id.clone(),
                    event: CliEvent::SpawnError { message: e },
                },
            );
            return;
        }
    };

    // The cli_worker emits a Started event on the channel — forward it
    // both to the history and to the UI.
    let mut child_exit_code: Option<i32> = None;
    while let Some(ev) = worker.next_event().await {
        // Fan out to UI.
        let _ = app.emit(
            "agent-cli-output",
            AgentCliEvent {
                workflow_id: workflow_id.clone(),
                event: ev.clone(),
            },
        );
        // Record to durable log.
        match &ev {
            CliEvent::Started { pid } => {
                let _ = engine
                    .append(
                        &workflow_id,
                        WorkflowEventKind::ActivityScheduled {
                            activity_id: format!("pid-{pid}"),
                            kind: "cli_spawn".into(),
                        },
                    )
                    .await;
            }
            CliEvent::Line { stream, text } => {
                let _ = engine
                    .heartbeat(
                        &workflow_id,
                        Some(format!(
                            "{}: {}",
                            if *stream == CliStream::Stderr { "stderr" } else { "stdout" },
                            truncate(text, 512)
                        )),
                    )
                    .await;
            }
            CliEvent::Exited { code } => {
                child_exit_code = *code;
            }
            CliEvent::SpawnError { message } => {
                let _ = engine.fail(&workflow_id, message).await;
                return;
            }
        }
    }

    // Channel closed — reap the child and finalise the workflow.
    match child_exit_code {
        Some(0) => {
            let _ = engine
                .complete(
                    &workflow_id,
                    serde_json::json!({"exit_code": 0}),
                )
                .await;
        }
        Some(code) => {
            let _ = engine
                .fail(&workflow_id, &format!("CLI exited with status {code}"))
                .await;
        }
        None => {
            // Process ended without a status — could be signal-killed.
            let _ = engine
                .fail(&workflow_id, "CLI exited without status code")
                .await;
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentCliEvent {
    pub workflow_id: String,
    pub event: CliEvent,
}

/// Query the current status + full history of a workflow.
#[tauri::command]
pub async fn roster_query_workflow(
    workflow_id: String,
    state: State<'_, AppState>,
) -> Result<WorkflowStatusReport, String> {
    let engine = state.workflow_engine.lock().await;
    let status = engine.status(&workflow_id).await?.ok_or_else(|| {
        format!("workflow {workflow_id} not found")
    })?;
    let history = engine.history(&workflow_id).await?;
    Ok(WorkflowStatusReport { status, history })
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStatusReport {
    pub status: WorkflowStatus,
    pub history: Vec<crate::workflows::WorkflowEvent>,
}

/// Cancel a workflow — appends a `Cancelled` event. Idempotent.
#[tauri::command]
pub async fn roster_cancel_workflow(
    workflow_id: String,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let engine = state.workflow_engine.lock().await;
    engine
        .cancel(
            &workflow_id,
            reason.as_deref().unwrap_or("user cancelled"),
        )
        .await
}

/// List recent workflows (terminal + pending) across all agents.
#[tauri::command]
pub async fn roster_list_workflows(
    state: State<'_, AppState>,
) -> Result<Vec<WorkflowSummary>, String> {
    let engine = state.workflow_engine.lock().await;
    engine.list_all().await
}

/// List only workflows whose last event is non-terminal. Used on startup
/// so the UI can show "these were running when you quit".
#[tauri::command]
pub async fn roster_list_pending_workflows(
    state: State<'_, AppState>,
) -> Result<Vec<WorkflowSummary>, String> {
    let engine = state.workflow_engine.lock().await;
    engine.list_pending().await
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_cli_request_deserializes() {
        let j = r#"{"agent_id":"beta","prompt":"hi"}"#;
        let req: StartCliWorkflowRequest = serde_json::from_str(j).unwrap();
        assert_eq!(req.agent_id, "beta");
    }

    #[test]
    fn create_agent_request_round_trip() {
        let j = serde_json::json!({
            "display_name": "Coder",
            "vrm_model_id": "m58",
            "brain_backend": {
                "kind": "external_cli",
                "data": {
                    "kind": "codex",
                    "binary": "codex",
                    "extra_args": ["--json"]
                }
            }
        });
        let req: CreateAgentRequest = serde_json::from_value(j).unwrap();
        match req.brain_backend {
            BrainBackend::ExternalCli { kind, binary, .. } => {
                assert_eq!(kind, CliKind::Codex);
                assert_eq!(binary, "codex");
            }
            _ => panic!("wrong backend discriminant"),
        }
    }

    #[test]
    fn truncate_handles_short_and_long() {
        assert_eq!(truncate("abc", 10), "abc");
        assert_eq!(truncate(&"x".repeat(20), 5), "xxxxx…");
    }

    #[test]
    fn agent_backend_kind_discriminant() {
        let cli = BrainBackend::ExternalCli {
            kind: CliKind::Claude,
            binary: "claude".into(),
            extra_args: vec![],
        };
        assert_eq!(cli.discriminant(), AgentBackendKind::ExternalCli);
        let native = BrainBackend::Native { mode: None };
        assert_eq!(native.discriminant(), AgentBackendKind::NativeApi);
    }
}
