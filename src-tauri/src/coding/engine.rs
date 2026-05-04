//! Self-improve engine — the autonomous coding loop driver.
//!
//! Loop lifecycle:
//! 1. User toggles self-improve on (gated through warning dialog).
//! 2. Engine spawns a Tokio task that detects the workspace git repo,
//!    reads `rules/milestones.md`, picks the next `not-started` chunk,
//!    runs the configured Coding LLM through a checkpointed
//!    planner/coder/reviewer/apply/test/stage DAG, persists progress,
//!    and emits `self-improve-progress` Tauri events for the live UI.
//! 3. On error or pause, the task exits gracefully; on next app launch
//!    the engine auto-resumes if `enabled = true`.
//!
//! Resilience: the task lives behind an [`AtomicBool`] cancellation flag.
//! The only way to stop the loop is to flip self-improve to disabled.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::Mutex as TokioMutex;
use tokio::task::JoinHandle;

use crate::brain::openai_client::OpenAiMessage;

use super::apply_file::{self, FileBlock};
use super::client;
use super::dag_runner;
use super::git_ops;
use super::github;
use super::metrics::MetricsLog;
use super::milestones::{next_not_started, parse_chunks, ChunkRow};
use super::repo::{
    detect_repo, feature_branch_name, guess_repo_root, sanitize_branch_segment, RepoState,
};
use super::reviewer::{self, ReviewVerdict, ReviewerConfig};
use super::test_runner::{self, TestRunConfig};
use super::worktree;
use super::{CodingLlmConfig, CodingLlmProvider};

/// Maximum chunks the loop will attempt in a single session before
/// idling. Keeps things bounded in case the planner stalls.
const MAX_CYCLES: usize = 50;

/// Idle delay between cycles when the loop has nothing to do (e.g. all
/// chunks complete) — keeps CPU usage at zero while remaining responsive
/// to milestones.md edits.
const IDLE_SLEEP_SECS: u64 = 30;

/// Event payload emitted to the frontend for live UI updates.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProgressEvent {
    /// Stable phase identifier for grouping in the UI (e.g. `"plan"`,
    /// `"branch"`, `"complete"`, `"error"`).
    pub phase: String,
    /// Free-text human-readable status line.
    pub message: String,
    /// 0–100 progress within the current chunk.
    pub progress: u8,
    /// Optional chunk id this event relates to.
    pub chunk_id: Option<String>,
    /// Severity for the UI activity-feed coloring.
    /// One of `info`, `success`, `warn`, `error`.
    pub level: String,
}

impl ProgressEvent {
    fn info(phase: &str, message: impl Into<String>) -> Self {
        Self {
            phase: phase.to_string(),
            message: message.into(),
            progress: 0,
            chunk_id: None,
            level: "info".to_string(),
        }
    }
    fn success(phase: &str, message: impl Into<String>) -> Self {
        Self {
            phase: phase.to_string(),
            message: message.into(),
            progress: 100,
            chunk_id: None,
            level: "success".to_string(),
        }
    }
    fn error(phase: &str, message: impl Into<String>) -> Self {
        Self {
            phase: phase.to_string(),
            message: message.into(),
            progress: 0,
            chunk_id: None,
            level: "error".to_string(),
        }
    }
    fn with_chunk(mut self, id: &str) -> Self {
        self.chunk_id = Some(id.to_string());
        self
    }
    fn with_progress(mut self, p: u8) -> Self {
        self.progress = p.min(100);
        self
    }
}

/// In-memory engine handle stored on `AppState`. Holds the cancellation
/// flag and join handle for the running loop, if any.
#[derive(Default)]
pub struct SelfImproveEngine {
    pub running: AtomicBool,
    pub cancel: Arc<AtomicBool>,
    pub task: TokioMutex<Option<JoinHandle<()>>>,
}

impl SelfImproveEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Request cancellation. The loop checks the flag between cycles.
    pub async fn request_stop(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        let mut slot = self.task.lock().await;
        if let Some(handle) = slot.take() {
            // Best-effort: wait briefly for graceful shutdown.
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
        }
        self.running.store(false, Ordering::Relaxed);
    }
}

/// Read milestones.md from `repo_root/rules/milestones.md`.
fn read_milestones(repo_root: &std::path::Path) -> Result<String, String> {
    let path = repo_root.join("rules").join("milestones.md");
    std::fs::read_to_string(&path).map_err(|e| format!("read milestones: {e}"))
}

/// Build the planner system + user messages.
///
/// Delegates to [`crate::coding::prompting::CodingPrompt`] so the planner
/// inherits the project-wide XML structure, role description, negative
/// constraints, and error-handling guidance shared by every coding
/// workflow. The reply is expected inside a `<plan>` tag — see
/// [`crate::coding::prompting::OutputShape::NumberedPlan`].
///
/// The autonomous loop never asks the LLM to *apply* changes — that
/// gating lives in a future chunk so this layer remains safe.
fn planner_prompt(
    repo: &RepoState,
    chunk: &ChunkRow,
    workflow_cfg: &crate::coding::CodingWorkflowConfig,
) -> Vec<OpenAiMessage> {
    planner_prompt_for_attempt(repo, chunk, workflow_cfg, None)
}

fn repair_planner_prompt(
    repo: &RepoState,
    chunk: &ChunkRow,
    workflow_cfg: &crate::coding::CodingWorkflowConfig,
    failure_context: &str,
) -> Vec<OpenAiMessage> {
    planner_prompt_for_attempt(repo, chunk, workflow_cfg, Some(failure_context))
}

fn planner_prompt_for_attempt(
    repo: &RepoState,
    chunk: &ChunkRow,
    workflow_cfg: &crate::coding::CodingWorkflowConfig,
    failure_context: Option<&str>,
) -> Vec<OpenAiMessage> {
    let retry_block = failure_context
        .map(|context| {
            format!(
                "\n<failed_test_gate>\n{}\n</failed_test_gate>\n\
                 This is the only automatic retry. Produce a repair plan that directly addresses the failed test gate and preserves any already-correct parts of the first attempt.\n",
                context.trim()
            )
        })
        .unwrap_or_default();
    let user_task = format!(
        "Plan the implementation for one chunk from TerranSoul's milestones.\n\
         \n\
         <repository>\n  \
           <root>{}</root>\n  \
           <branch>{}</branch>\n  \
           <clean>{}</clean>\n\
         </repository>\n\
         <chunk>\n  \
           <id>{}</id>\n  \
           <title>{}</title>\n\
         </chunk>\n\
         {retry_block}\
         \n\
         Produce the plan now. Do not output code — only the file list and \
         reasoning. Consult the supplied <documents> for project rules and \
         conventions before deciding which files to touch.",
        repo.root.as_deref().unwrap_or("unknown"),
        repo.current_branch.as_deref().unwrap_or("unknown"),
        repo.clean,
        chunk.id,
        chunk.title,
    );

    let prompt = crate::coding::prompting::CodingPrompt {
        role: crate::coding::workflow::default_coding_role(),
        task: user_task,
        negative_constraints: {
            let mut c = crate::coding::workflow::default_negative_constraints();
            c.push("Do not produce code in this reply — output the plan only.".to_string());
            c
        },
        documents: load_planner_context(repo, workflow_cfg),
        output: crate::coding::prompting::OutputShape::NumberedPlan { max_steps: 8 },
        example: None,
        assistant_prefill: Some("<analysis>".to_string()),
        error_handling: crate::coding::workflow::default_error_handling(),
    };
    prompt.build()
}

/// Load `rules/`, `instructions/`, `docs/`, and explicit files via the
/// shared workflow loader. Centralising here means a single change to
/// `CodingWorkflowConfig` propagates to both the self-improve planner
/// and the reusable `run_coding_task` runner.
fn load_planner_context(
    repo: &RepoState,
    workflow_cfg: &crate::coding::CodingWorkflowConfig,
) -> Vec<crate::coding::prompting::DocSnippet> {
    load_context_for_target_paths(repo, workflow_cfg, &[])
}

fn load_context_for_target_paths(
    repo: &RepoState,
    workflow_cfg: &crate::coding::CodingWorkflowConfig,
    target_paths: &[String],
) -> Vec<crate::coding::prompting::DocSnippet> {
    let Some(root) = repo.root.as_deref() else {
        return Vec::new();
    };
    crate::coding::workflow::load_workflow_context_for_paths(
        std::path::Path::new(root),
        workflow_cfg,
        true,
        true,
        true,
        target_paths,
    )
}

/// Run a single planning cycle for one chunk. Emits progress events at
/// each phase. Returns the produced plan text on success.
async fn plan_one_chunk<R: Runtime>(
    app: &AppHandle<R>,
    config: &CodingLlmConfig,
    repo: &RepoState,
    chunk: &ChunkRow,
    metrics: &MetricsLog,
    workflow_cfg: &crate::coding::CodingWorkflowConfig,
    retry_context: Option<&str>,
) -> Result<String, String> {
    let provider = provider_label(&config.provider);
    let started_at = metrics.record_start(&chunk.id, &chunk.title, provider, &config.model);

    emit(
        app,
        ProgressEvent::info(
            "plan",
            format!("Planning chunk {}: {}", chunk.id, chunk.title),
        )
        .with_chunk(&chunk.id)
        .with_progress(10),
    );

    let target_branch = feature_branch_name(&chunk.id);
    emit(
        app,
        ProgressEvent::info("branch", format!("Target branch: {target_branch}"))
            .with_chunk(&chunk.id)
            .with_progress(25),
    );

    let openai = client::client_from(config);
    let messages = match retry_context {
        Some(context) => repair_planner_prompt(repo, chunk, workflow_cfg, context),
        None => planner_prompt(repo, chunk, workflow_cfg),
    };
    match openai.chat_with_usage(messages).await {
        Ok((plan, usage)) => {
            let token_usage = match usage {
                Some(u) => crate::coding::metrics::TokenUsage {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                },
                None => crate::coding::metrics::TokenUsage::default(),
            };
            metrics.record_outcome(
                started_at,
                &chunk.id,
                &chunk.title,
                provider,
                &config.model,
                true,
                plan.len(),
                None,
                token_usage,
            );
            emit(
                app,
                ProgressEvent::success("plan", format!("Plan ready ({} chars)", plan.len()))
                    .with_chunk(&chunk.id)
                    .with_progress(90),
            );
            Ok(plan)
        }
        Err(e) => {
            metrics.record_outcome(
                started_at,
                &chunk.id,
                &chunk.title,
                provider,
                &config.model,
                false,
                0,
                Some(&e),
                crate::coding::metrics::TokenUsage::default(),
            );
            Err(e)
        }
    }
}

/// Build the implementation prompt that asks the Coding LLM for
/// explicit `<file path="...">...</file>` blocks. The reply is reviewed
/// before any block is written to disk.
fn coder_prompt(
    repo: &RepoState,
    chunk: &ChunkRow,
    plan: &str,
    workflow_cfg: &crate::coding::CodingWorkflowConfig,
) -> Vec<OpenAiMessage> {
    let user_task = format!(
        "Implement exactly one TerranSoul milestone chunk using the approved plan.\n\
         \n\
         <repository>\n  \
           <root>{}</root>\n  \
           <branch>{}</branch>\n  \
           <clean>{}</clean>\n\
         </repository>\n\
         <chunk>\n  \
           <id>{}</id>\n  \
           <title>{}</title>\n\
         </chunk>\n\
         <approved_plan>\n{}\n</approved_plan>\n\
         \n\
         Output only complete file replacement blocks in this exact form:\n\
         <file path=\"repo/relative/path.ext\">full file contents</file>\n\
         Return one block per file you need to create or replace. Do not use markdown fences.\n\
         Do not include files that do not need changes.",
        repo.root.as_deref().unwrap_or("unknown"),
        repo.current_branch.as_deref().unwrap_or("unknown"),
        repo.clean,
        chunk.id,
        chunk.title,
        plan.trim(),
    );

    let mut constraints = crate::coding::workflow::default_negative_constraints();
    constraints.push(
        "Do not output patches or partial snippets; each <file> block must contain the complete final file contents."
            .to_string(),
    );
    constraints.push(
        "Do not modify generated, vendored, lockfile, target/, node_modules/, or test-output files unless the chunk explicitly requires it."
            .to_string(),
    );

    let target_paths = extract_plan_path_hints(plan);
    let prompt = crate::coding::prompting::CodingPrompt {
        role: crate::coding::workflow::default_coding_role(),
        task: user_task,
        negative_constraints: constraints,
        documents: load_context_for_target_paths(repo, workflow_cfg, &target_paths),
        output: crate::coding::prompting::OutputShape::Prose,
        example: Some(
            "<file path=\"src/example.ts\">\nexport const value = 1\n</file>".to_string(),
        ),
        assistant_prefill: None,
        error_handling: crate::coding::workflow::default_error_handling(),
    };
    prompt.build()
}

fn extract_plan_path_hints(plan: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for raw_token in plan.split_whitespace() {
        let candidate = raw_token
            .trim_matches(|character: char| {
                matches!(
                    character,
                    '`' | '"' | '\'' | ',' | ';' | ':' | '.' | '(' | ')' | '[' | ']' | '{' | '}'
                )
            })
            .trim_start_matches("./")
            .replace('\\', "/");
        if !is_probable_repo_path(&candidate) || paths.contains(&candidate) {
            continue;
        }
        paths.push(candidate);
    }
    paths
}

fn is_probable_repo_path(candidate: &str) -> bool {
    !candidate.starts_with("http")
        && candidate.contains('/')
        && candidate.contains('.')
        && !candidate.contains("..")
}

/// End-to-end execution gate result for a single chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExecutionGateResult {
    applied_paths: Vec<String>,
    test_summary: String,
    isolated_patch_path: Option<String>,
    applied_isolated_patch_path: Option<String>,
}

#[derive(Debug)]
struct ExecutionWorkspace {
    repo: RepoState,
    repo_root: PathBuf,
    temporary_worktree: Option<worktree::TemporaryWorktree>,
}

impl ExecutionWorkspace {
    fn active(repo: &RepoState, repo_root: &Path) -> Self {
        Self {
            repo: repo.clone(),
            repo_root: repo_root.to_path_buf(),
            temporary_worktree: None,
        }
    }

    fn isolated(temporary_worktree: worktree::TemporaryWorktree) -> Self {
        let repo_root = temporary_worktree.path().to_path_buf();
        Self {
            repo: detect_repo(&repo_root),
            repo_root,
            temporary_worktree: Some(temporary_worktree),
        }
    }

    fn is_isolated(&self) -> bool {
        self.temporary_worktree.is_some()
    }

    fn cached_diff(&self) -> Result<String, String> {
        match &self.temporary_worktree {
            Some(temporary_worktree) => temporary_worktree.cached_diff(),
            None => Ok(String::new()),
        }
    }
}

const DAG_NODE_PLAN: &str = "planner";
const DAG_NODE_CODE: &str = "coder";
const DAG_NODE_REVIEW: &str = "reviewer";
const DAG_NODE_APPLY: &str = "apply";
const DAG_NODE_TEST: &str = "tester";
const DAG_NODE_STAGE: &str = "stage";
const TEST_FAILURE_RETRY_MARKER: &str = "retryable test failure";

#[derive(Debug, Default)]
struct ChunkDagState {
    plan: Option<String>,
    blocks: Vec<FileBlock>,
    snapshot: Option<FileSnapshot>,
    applied_paths: Vec<String>,
    test_summary: Option<String>,
}

fn self_improve_execution_graph() -> dag_runner::WorkflowGraph {
    fn node(id: &str, label: &str, capabilities: &[&str]) -> dag_runner::DagNode {
        dag_runner::DagNode {
            id: id.to_string(),
            label: label.to_string(),
            capabilities: capabilities.iter().map(|cap| (*cap).to_string()).collect(),
        }
    }
    fn edge(from: &str, to: &str) -> dag_runner::DagEdge {
        dag_runner::DagEdge {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    dag_runner::WorkflowGraph {
        nodes: vec![
            node(DAG_NODE_PLAN, "Planner", &["llm_call"]),
            node(DAG_NODE_CODE, "Coder", &["llm_call"]),
            node(DAG_NODE_REVIEW, "Reviewer", &["llm_call", "review"]),
            node(DAG_NODE_APPLY, "Apply", &["file_write"]),
            node(DAG_NODE_TEST, "Tester", &["test_run"]),
            node(DAG_NODE_STAGE, "Stage", &["git_write"]),
        ],
        edges: vec![
            edge(DAG_NODE_PLAN, DAG_NODE_CODE),
            edge(DAG_NODE_CODE, DAG_NODE_REVIEW),
            edge(DAG_NODE_REVIEW, DAG_NODE_APPLY),
            edge(DAG_NODE_APPLY, DAG_NODE_TEST),
            edge(DAG_NODE_TEST, DAG_NODE_STAGE),
        ],
    }
}

fn self_improve_dag_config() -> dag_runner::DagRunnerConfig {
    dag_runner::DagRunnerConfig {
        max_parallel: 2,
        skip_on_failure: true,
        available_capabilities: vec![
            "llm_call".to_string(),
            "review".to_string(),
            "file_write".to_string(),
            "test_run".to_string(),
            "git_write".to_string(),
        ],
    }
}

fn format_dag_failure(result: &dag_runner::DagRunResult) -> String {
    let failed = result
        .results
        .iter()
        .filter(|node| matches!(node.status, dag_runner::NodeStatus::Failed))
        .map(|node| format!("{}: {}", node.node_id, node.message))
        .collect::<Vec<_>>()
        .join("; ");
    let skipped = if result.skipped_nodes.is_empty() {
        String::new()
    } else {
        format!(" skipped: {}", result.skipped_nodes.join(", "))
    };
    if failed.is_empty() {
        format!("DAG failed with no failing node.{skipped}")
    } else {
        format!("DAG failed: {failed}.{skipped}")
    }
}

/// Plan -> code -> review -> apply -> test -> stage for one chunk,
/// orchestrated through the coding DAG runner.
#[allow(clippy::too_many_arguments)]
async fn execute_chunk_dag<R: Runtime>(
    app: &AppHandle<R>,
    config: &CodingLlmConfig,
    repo: &RepoState,
    repo_root: &Path,
    chunk: &ChunkRow,
    metrics: &MetricsLog,
    workflow_cfg: &crate::coding::CodingWorkflowConfig,
    worktree_dir: Option<&str>,
    retry_context: Option<String>,
    cancel: &Arc<AtomicBool>,
) -> Result<ExecutionGateResult, String> {
    let original_repo_root = repo_root.to_path_buf();
    let execution_workspace =
        prepare_execution_workspace(app, repo, repo_root, chunk, worktree_dir)?;
    let execution_repo_root = execution_workspace.repo_root.clone();
    let execution_repo = execution_workspace.repo.clone();

    let graph = self_improve_execution_graph();
    let dag_config = self_improve_dag_config();
    dag_runner::validate_graph(&graph, &dag_config).map_err(|e| e.to_string())?;
    let layers = dag_runner::compute_layers(&graph)
        .into_iter()
        .map(|layer| layer.join("+"))
        .collect::<Vec<_>>()
        .join(" -> ");
    emit(
        app,
        ProgressEvent::info("dag", format!("Running coding DAG: {layers}"))
            .with_chunk(&chunk.id)
            .with_progress(8),
    );

    let state = Arc::new(TokioMutex::new(ChunkDagState::default()));
    let app_handle = app.clone();
    let config = config.clone();
    let repo = execution_repo.clone();
    let repo_root = execution_repo_root.clone();
    let chunk = chunk.clone();
    let metrics = metrics.clone();
    let workflow_cfg = workflow_cfg.clone();
    let cancel = cancel.clone();

    let result = dag_runner::execute_dag_async(&graph, &dag_config, |node_id| {
        let app = app_handle.clone();
        let config = config.clone();
        let repo = repo.clone();
        let repo_root = repo_root.clone();
        let chunk = chunk.clone();
        let metrics = metrics.clone();
        let workflow_cfg = workflow_cfg.clone();
        let cancel = cancel.clone();
        let state = state.clone();
        let retry_context = retry_context.clone();

        async move {
            if cancel.load(Ordering::Relaxed) {
                return Err(format!("cancelled before {node_id}"));
            }
            match node_id.as_str() {
                DAG_NODE_PLAN => {
                    let plan = plan_one_chunk(
                        &app,
                        &config,
                        &repo,
                        &chunk,
                        &metrics,
                        &workflow_cfg,
                        retry_context.as_deref(),
                    )
                    .await?;
                    state.lock().await.plan = Some(plan);
                    Ok("plan ready".to_string())
                }
                DAG_NODE_CODE => {
                    emit(
                        &app,
                        ProgressEvent::info("code", "Generating file blocks for execution gate")
                            .with_chunk(&chunk.id)
                            .with_progress(35),
                    );
                    let plan = state
                        .lock()
                        .await
                        .plan
                        .clone()
                        .ok_or_else(|| "planner produced no plan".to_string())?;
                    let reply = client::client_from(&config)
                        .chat(coder_prompt(&repo, &chunk, &plan, &workflow_cfg))
                        .await?;
                    let blocks = apply_file::parse_file_blocks(&reply);
                    if blocks.is_empty() {
                        return Err("coding LLM returned no <file path=...> blocks".to_string());
                    }
                    let count = blocks.len();
                    state.lock().await.blocks = blocks;
                    Ok(format!("generated {count} file block(s)"))
                }
                DAG_NODE_REVIEW => {
                    let blocks = state.lock().await.blocks.clone();
                    let preview = preview_diff(&repo_root, &blocks)?;
                    emit(
                        &app,
                        ProgressEvent::info(
                            "review",
                            format!("Reviewing {} file block(s)", blocks.len()),
                        )
                        .with_chunk(&chunk.id)
                        .with_progress(45),
                    );
                    let review_task = reviewer::build_review_task(
                        &format!("self-improve-review-{}", chunk.id),
                        &preview,
                        Vec::new(),
                    );
                    let review = crate::coding::workflow::run_coding_task(
                        &config,
                        &review_task,
                        Some(&workflow_cfg),
                    )
                    .await?;
                    let review_result = reviewer::parse_review_result(&review.payload)
                        .ok_or_else(|| "reviewer returned invalid JSON".to_string())?;
                    match reviewer::decide(&review_result, &ReviewerConfig::default()) {
                        ReviewVerdict::Accept => Ok("review accepted".to_string()),
                        ReviewVerdict::Reject { reason, .. } => {
                            Err(format!("review rejected diff: {reason}"))
                        }
                    }
                }
                DAG_NODE_APPLY => {
                    let blocks = state.lock().await.blocks.clone();
                    let snapshot = FileSnapshot::capture(&repo_root, &blocks)?;
                    emit(
                        &app,
                        ProgressEvent::info(
                            "apply",
                            format!("Applying {} reviewed file block(s)", blocks.len()),
                        )
                        .with_chunk(&chunk.id)
                        .with_progress(60),
                    );
                    let applied = apply_file::apply_blocks(&repo_root, &blocks, false);
                    if !applied.rejected.is_empty() {
                        snapshot.restore(&repo_root)?;
                        return Err(format_apply_rejections(&applied.rejected));
                    }
                    let paths: Vec<String> =
                        applied.applied.iter().map(|a| a.path.clone()).collect();
                    let mut guard = state.lock().await;
                    guard.snapshot = Some(snapshot);
                    guard.applied_paths = paths.clone();
                    Ok(format!("applied {} file(s)", paths.len()))
                }
                DAG_NODE_TEST => {
                    emit(
                        &app,
                        ProgressEvent::info("test", "Running autonomous execution test gate")
                            .with_chunk(&chunk.id)
                            .with_progress(75),
                    );
                    let tests =
                        test_runner::run_tests(&TestRunConfig::default_ci_gate(repo_root.clone()))
                            .await;
                    let summary = summarize_tests(&tests);
                    if !tests.all_green {
                        if let Some(snapshot) = state.lock().await.snapshot.clone() {
                            snapshot.restore(&repo_root)?;
                        }
                        let details = format_test_failure_for_retry(&tests);
                        return Err(format!(
                            "{TEST_FAILURE_RETRY_MARKER}: tests failed; restored touched files: {summary}\n{details}"
                        ));
                    }
                    state.lock().await.test_summary = Some(summary.clone());
                    Ok(summary)
                }
                DAG_NODE_STAGE => {
                    let paths = state.lock().await.applied_paths.clone();
                    stage_paths(&repo_root, &paths)?;
                    Ok(format!("staged {} file(s)", paths.len()))
                }
                _ => Err(format!("unknown DAG node: {node_id}")),
            }
        }
    })
    .await;

    if !result.all_success {
        return Err(format_dag_failure(&result));
    }

    let final_state = state.lock().await;
    let applied_paths = final_state.applied_paths.clone();
    let test_summary = final_state
        .test_summary
        .clone()
        .unwrap_or_else(|| "no tests recorded".to_string());
    drop(final_state);

    let (isolated_patch_path, applied_isolated_patch_path) = if execution_workspace.is_isolated() {
        let patch = execution_workspace.cached_diff()?;
        if patch.trim().is_empty() {
            (None, None)
        } else {
            let patch_path = write_isolated_patch(&original_repo_root, &chunk.id, &patch)?;
            apply_isolated_patch_to_working_branch(&original_repo_root, Path::new(&patch_path))?;
            stage_paths(&original_repo_root, &applied_paths)?;
            (Some(patch_path.clone()), Some(patch_path))
        }
    } else {
        (None, None)
    };
    Ok(ExecutionGateResult {
        applied_paths,
        test_summary,
        isolated_patch_path,
        applied_isolated_patch_path,
    })
}

#[allow(clippy::too_many_arguments)]
async fn execute_chunk_dag_with_retry<R: Runtime>(
    app: &AppHandle<R>,
    config: &CodingLlmConfig,
    repo: &RepoState,
    repo_root: &Path,
    chunk: &ChunkRow,
    metrics: &MetricsLog,
    workflow_cfg: &crate::coding::CodingWorkflowConfig,
    worktree_dir: Option<&str>,
    cancel: &Arc<AtomicBool>,
) -> Result<ExecutionGateResult, String> {
    let first = execute_chunk_dag(
        app,
        config,
        repo,
        repo_root,
        chunk,
        metrics,
        workflow_cfg,
        worktree_dir,
        None,
        cancel,
    )
    .await;

    let first_error = match first {
        Ok(result) => return Ok(result),
        Err(error) => error,
    };

    if !is_retryable_test_failure(&first_error) || cancel.load(Ordering::Relaxed) {
        return Err(first_error);
    }

    emit(
        app,
        ProgressEvent::info(
            "retry",
            "Test gate failed; retrying once with a repair planner prompt",
        )
        .with_chunk(&chunk.id)
        .with_progress(82),
    );

    execute_chunk_dag(
        app,
        config,
        repo,
        repo_root,
        chunk,
        metrics,
        workflow_cfg,
        worktree_dir,
        Some(first_error.clone()),
        cancel,
    )
    .await
    .map_err(|retry_error| format!("{first_error}\nRetry failed: {retry_error}"))
}

fn is_retryable_test_failure(message: &str) -> bool {
    message.contains(TEST_FAILURE_RETRY_MARKER)
}

fn prepare_execution_workspace<R: Runtime>(
    app: &AppHandle<R>,
    repo: &RepoState,
    repo_root: &Path,
    chunk: &ChunkRow,
    worktree_dir: Option<&str>,
) -> Result<ExecutionWorkspace, String> {
    if git_ops::working_tree_clean(repo_root) {
        return Ok(ExecutionWorkspace::active(repo, repo_root));
    }

    emit(
        app,
        ProgressEvent::info(
            "worktree",
            "Working tree is dirty; using a temporary git worktree for autonomous apply/test",
        )
        .with_chunk(&chunk.id)
        .with_progress(6),
    );
    let base_dir = worktree_dir
        .filter(|s| !s.is_empty())
        .map(std::path::Path::new);
    let temporary_worktree = worktree::create_worktree_in(repo_root, &chunk.id, base_dir)?;
    emit(
        app,
        ProgressEvent::info(
            "worktree",
            format!(
                "Temporary worktree ready at {}",
                temporary_worktree.path().display()
            ),
        )
        .with_chunk(&chunk.id)
        .with_progress(7),
    );
    Ok(ExecutionWorkspace::isolated(temporary_worktree))
}

fn write_isolated_patch(repo_root: &Path, chunk_id: &str, patch: &str) -> Result<String, String> {
    let patch_dir = repo_root
        .join("target")
        .join("terransoul-self-improve")
        .join("patches");
    std::fs::create_dir_all(&patch_dir)
        .map_err(|error| format!("create isolated patch dir: {error}"))?;
    let patch_path = patch_dir.join(format!(
        "{}-isolated.patch",
        sanitize_branch_segment(chunk_id)
    ));
    std::fs::write(&patch_path, patch)
        .map_err(|error| format!("write isolated patch {}: {error}", patch_path.display()))?;
    Ok(patch_path.to_string_lossy().to_string())
}

fn apply_isolated_patch_to_working_branch(
    repo_root: &Path,
    patch_path: &Path,
) -> Result<(), String> {
    let out = Command::new("git")
        .arg("apply")
        .arg("--whitespace=nowarn")
        .arg(patch_path)
        .current_dir(repo_root)
        .output()
        .map_err(|error| format!("git apply unavailable: {error}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let details = if stderr.is_empty() { stdout } else { stderr };
    Err(if details.is_empty() {
        format!("git apply failed for {}", patch_path.display())
    } else {
        format!("git apply failed for {}: {details}", patch_path.display())
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileSnapshot {
    entries: Vec<SnapshotEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotEntry {
    path: String,
    original: Option<String>,
}

impl FileSnapshot {
    fn capture(repo_root: &Path, blocks: &[FileBlock]) -> Result<Self, String> {
        let mut entries = Vec::with_capacity(blocks.len());
        for block in blocks {
            apply_file::validate_path(repo_root, &block.path)?;
            let path = repo_root.join(&block.path);
            let original = match std::fs::read_to_string(&path) {
                Ok(body) => Some(body),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(format!("snapshot {}: {e}", block.path)),
            };
            entries.push(SnapshotEntry {
                path: block.path.clone(),
                original,
            });
        }
        Ok(Self { entries })
    }

    fn restore(&self, repo_root: &Path) -> Result<(), String> {
        for entry in &self.entries {
            let path = repo_root.join(&entry.path);
            match &entry.original {
                Some(body) => {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| format!("restore parent {}: {e}", parent.display()))?;
                    }
                    std::fs::write(&path, body)
                        .map_err(|e| format!("restore {}: {e}", entry.path))?;
                }
                None => match std::fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(format!("remove created {}: {e}", entry.path)),
                },
            }
        }
        Ok(())
    }
}

fn preview_diff(repo_root: &Path, blocks: &[FileBlock]) -> Result<String, String> {
    let mut diff = String::new();
    for block in blocks {
        apply_file::validate_path(repo_root, &block.path)?;
        let old = std::fs::read_to_string(repo_root.join(&block.path)).unwrap_or_default();
        diff.push_str(&format!("diff --git a/{0} b/{0}\n", block.path));
        diff.push_str(&format!("--- a/{}\n+++ b/{}\n", block.path, block.path));
        diff.push_str("@@ full-file replacement @@\n");
        for line in old.lines() {
            diff.push('-');
            diff.push_str(line);
            diff.push('\n');
        }
        for line in block.content.lines() {
            diff.push('+');
            diff.push_str(line);
            diff.push('\n');
        }
    }
    Ok(diff)
}

fn stage_paths(repo_root: &Path, paths: &[String]) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut cmd = Command::new("git");
    cmd.arg("add").arg("--").args(paths).current_dir(repo_root);
    let out = cmd.output().map_err(|e| format!("git add: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        "git add failed".to_string()
    } else {
        format!("git add failed: {stderr}")
    })
}

fn summarize_tests(result: &test_runner::TestRunResult) -> String {
    result
        .suites
        .iter()
        .map(|suite| format!("{}={:?}/{}", suite.name, suite.status, suite.attempts))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_test_failure_for_retry(result: &test_runner::TestRunResult) -> String {
    result
        .suites
        .iter()
        .filter(|suite| !suite.status.is_green())
        .map(|suite| {
            let stdout = suite.stdout_tail.trim();
            let stderr = suite.stderr_tail.trim();
            let spawn_error = suite.spawn_error.as_deref().unwrap_or("").trim();
            format!(
                "suite={} status={:?} attempts={} exit={:?}\nstdout_tail:\n{}\nstderr_tail:\n{}\nspawn_error:\n{}",
                suite.name,
                suite.status,
                suite.attempts,
                suite.exit_code,
                if stdout.is_empty() { "<empty>" } else { stdout },
                if stderr.is_empty() { "<empty>" } else { stderr },
                if spawn_error.is_empty() { "<none>" } else { spawn_error },
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn format_apply_rejections(rejections: &[apply_file::ApplyRejection]) -> String {
    let joined = rejections
        .iter()
        .map(|rejection| format!("{}: {}", rejection.path, rejection.reason))
        .collect::<Vec<_>>()
        .join("; ");
    format!("apply rejected file block(s): {joined}")
}

fn format_execution_success(chunk_id: &str, result: &ExecutionGateResult) -> String {
    match (&result.isolated_patch_path, &result.applied_isolated_patch_path) {
        (Some(patch_path), Some(applied_path)) => format!(
            "Chunk {chunk_id} validated {} file(s) in a temporary worktree; patch saved at {patch_path} and applied from {applied_path}; tests: {}",
            result.applied_paths.len(),
            result.test_summary
        ),
        (Some(patch_path), None) => format!(
            "Chunk {chunk_id} validated {} file(s) in a temporary worktree; patch saved at {patch_path}; tests: {}",
            result.applied_paths.len(),
            result.test_summary
        ),
        (None, _) => format!(
            "Chunk {chunk_id} applied {} file(s); tests: {}",
            result.applied_paths.len(),
            result.test_summary
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompletionArchiveResult {
    modified_paths: Vec<String>,
    next_chunk_id: Option<String>,
}

fn archive_completed_chunk(
    repo_root: &Path,
    chunk: &ChunkRow,
    result: &ExecutionGateResult,
) -> Result<CompletionArchiveResult, String> {
    let rules_dir = repo_root.join("rules");
    let milestones_path = rules_dir.join("milestones.md");
    let completion_log_path = rules_dir.join("completion-log.md");
    let date = current_utc_date();

    let milestones = std::fs::read_to_string(&milestones_path)
        .map_err(|error| format!("read milestones for archive: {error}"))?;
    let (without_chunk, removed) = remove_chunk_row(&milestones, &chunk.id);
    if !removed {
        return Err(format!("chunk {} was not found in milestones.md", chunk.id));
    }
    let without_empty_phases = remove_empty_phase_sections(&without_chunk);
    let remaining_chunks = parse_chunks(&without_empty_phases);
    let next_chunk = next_not_started(&remaining_chunks).cloned();
    let updated_milestones = update_next_chunk_section(&without_empty_phases, next_chunk.as_ref());
    atomic_write_string(&milestones_path, &updated_milestones)?;

    let completion_log = std::fs::read_to_string(&completion_log_path)
        .map_err(|error| format!("read completion log for archive: {error}"))?;
    let updated_completion_log = insert_completion_log_entry(&completion_log, chunk, result, &date);
    atomic_write_string(&completion_log_path, &updated_completion_log)?;

    Ok(CompletionArchiveResult {
        modified_paths: vec![
            "rules/milestones.md".to_string(),
            "rules/completion-log.md".to_string(),
        ],
        next_chunk_id: next_chunk.map(|chunk| chunk.id),
    })
}

fn remove_chunk_row(markdown: &str, chunk_id: &str) -> (String, bool) {
    let mut removed = false;
    let mut next = String::new();
    for line in markdown.split_inclusive('\n') {
        if table_row_id(line).as_deref() == Some(chunk_id) {
            removed = true;
            continue;
        }
        next.push_str(line);
    }
    (next, removed)
}

fn table_row_id(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }
    let inner = trimmed.trim_start_matches('|').trim_end_matches('|');
    let first = inner.split('|').next()?.trim();
    if first.chars().any(|character| character.is_ascii_digit()) {
        Some(first.to_string())
    } else {
        None
    }
}

fn remove_empty_phase_sections(markdown: &str) -> String {
    let mut output = String::new();
    let mut section = String::new();
    let mut in_phase = false;

    for line in markdown.split_inclusive('\n') {
        if line.starts_with("## Phase ") {
            flush_phase_section(&mut output, &mut section, in_phase);
            in_phase = true;
        }

        if in_phase {
            section.push_str(line);
        } else {
            output.push_str(line);
        }
    }
    flush_phase_section(&mut output, &mut section, in_phase);
    output
}

fn flush_phase_section(output: &mut String, section: &mut String, in_phase: bool) {
    if !in_phase || !parse_chunks(section).is_empty() {
        output.push_str(section);
    }
    section.clear();
}

fn update_next_chunk_section(markdown: &str, next_chunk: Option<&ChunkRow>) -> String {
    let Some(start) = markdown.find("## Next Chunk") else {
        return markdown.to_string();
    };
    let after_heading = start + "## Next Chunk".len();
    let Some(end_relative) = markdown[after_heading..].find("\n---") else {
        return markdown.to_string();
    };
    let end = after_heading + end_relative;
    let next_text = match next_chunk {
        Some(chunk) => format!(
            "**Chunk {} — {}.**",
            chunk.id,
            clean_chunk_title(&chunk.title).trim_end_matches('.')
        ),
        None => "No remaining chunks. All phases complete. Check `rules/backlog.md` for future work ideas.".to_string(),
    };
    format!(
        "{}## Next Chunk\n\n{}\n{}",
        &markdown[..start],
        next_text,
        &markdown[end..]
    )
}

fn insert_completion_log_entry(
    markdown: &str,
    chunk: &ChunkRow,
    result: &ExecutionGateResult,
    date: &str,
) -> String {
    let title = format!("Chunk {} — {}", chunk.id, clean_chunk_title(&chunk.title));
    let anchor = completion_log_anchor(&title);
    let toc_row = format!("| [{title}](#{anchor}) | {date} |\n");
    let mut next = if markdown.contains(&toc_row) {
        markdown.to_string()
    } else if let Some(index) = markdown.find("|-------|------|\n") {
        let insert_at = index + "|-------|------|\n".len();
        let mut updated = String::with_capacity(markdown.len() + toc_row.len());
        updated.push_str(&markdown[..insert_at]);
        updated.push_str(&toc_row);
        updated.push_str(&markdown[insert_at..]);
        updated
    } else {
        markdown.to_string()
    };

    let entry = completion_log_entry(&title, chunk, result, date);
    if next.contains(&format!("## {title}\n")) {
        return next;
    }

    let search_start = next.find("## Table of Contents").unwrap_or(0);
    if let Some(relative) = next[search_start..].find("\n---\n\n## ") {
        let insert_at = search_start + relative + "\n---\n\n".len();
        next.insert_str(insert_at, &entry);
        next
    } else {
        if !next.ends_with('\n') {
            next.push('\n');
        }
        next.push_str("\n---\n\n");
        next.push_str(&entry);
        next
    }
}

fn completion_log_entry(
    title: &str,
    chunk: &ChunkRow,
    result: &ExecutionGateResult,
    date: &str,
) -> String {
    let files = if result.applied_paths.is_empty() {
        "- No applied paths were reported by the execution gate.".to_string()
    } else {
        result
            .applied_paths
            .iter()
            .map(|path| format!("- `{path}`"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let isolated_note = match (
        result.isolated_patch_path.as_ref(),
        result.applied_isolated_patch_path.as_ref(),
    ) {
        (Some(path), Some(applied_path)) => format!(
            "- Execution used a temporary worktree; the validated patch was saved at `{path}` and applied to the active checkout from `{applied_path}`."
        ),
        (Some(path), None) => format!(
            "- Execution used a temporary worktree; the validated patch was saved at `{path}`."
        ),
        (None, _) => "- Execution applied changes directly in the active workspace.".to_string(),
    };

    format!(
        "## {title}\n\n\
         **Status:** Complete\n\
         **Date:** {date}\n\
         **Phase:** Self-improve autonomous loop\n\n\
         **Goal:** Complete milestone chunk `{}` — {}.\n\n\
         **Architecture:**\n\
         - Completed by the self-improve DAG after planner, coder, reviewer, apply, test, and stage gates passed.\n\
         - Archived automatically by the self-improve engine so the chunk will not repeat on the next cycle.\n\
         {isolated_note}\n\n\
         **Files modified:**\n\
         {files}\n\n\
         **Validation:**\n\
         - Autonomous test gate — passed: {}\n\n\
         ---\n\n",
        chunk.id,
        clean_chunk_title(&chunk.title),
        result.test_summary,
    )
}

fn clean_chunk_title(title: &str) -> String {
    title.replace("**", "").trim().to_string()
}

fn completion_log_anchor(title: &str) -> String {
    let mut anchor = String::new();
    let mut last_was_dash = false;
    for character in title.to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            anchor.push(character);
            last_was_dash = false;
        } else if !last_was_dash {
            anchor.push('-');
            last_was_dash = true;
        }
    }
    anchor.trim_matches('-').to_string()
}

fn atomic_write_string(path: &Path, content: &str) -> Result<(), String> {
    let tmp = path.with_extension("md.tmp");
    std::fs::write(&tmp, content).map_err(|error| format!("write {}: {error}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|error| format!("rename {}: {error}", path.display()))
}

fn current_utc_date() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year as i32, month as u32, day as u32)
}

fn provider_label(provider: &CodingLlmProvider) -> &'static str {
    match provider {
        CodingLlmProvider::Anthropic => "anthropic",
        CodingLlmProvider::Openai => "openai",
        CodingLlmProvider::Deepseek => "deepseek",
        CodingLlmProvider::Custom => "custom",
    }
}

fn emit<R: Runtime>(app: &AppHandle<R>, event: ProgressEvent) {
    let _ = app.emit("self-improve-progress", event);
}

/// Spawn the autonomous loop.
///
/// `repo_hint` is the directory the engine should *start* searching for a
/// git repo; in production the caller passes the app data dir or a saved
/// workspace path. The engine walks upward from there to find a real
/// TerranSoul checkout.
pub async fn start<R: Runtime>(
    app: AppHandle<R>,
    engine: Arc<SelfImproveEngine>,
    config: CodingLlmConfig,
    workflow_cfg: crate::coding::CodingWorkflowConfig,
    worktree_dir: Option<String>,
    repo_hint: PathBuf,
) {
    if engine.running.swap(true, Ordering::Relaxed) {
        emit(
            &app,
            ProgressEvent::info("idle", "Loop already running — ignoring start request"),
        );
        return;
    }
    engine.cancel.store(false, Ordering::Relaxed);
    let cancel = engine.cancel.clone();
    let engine_for_task = engine.clone();
    let metrics = MetricsLog::new(&repo_hint);
    let data_dir = repo_hint.clone();
    let github_cfg = github::load_github_config(&data_dir);

    let handle = tokio::spawn(async move {
        emit(
            &app,
            ProgressEvent::info("startup", "Self-improve loop starting…").with_progress(0),
        );

        // Discover the workspace root once at startup.
        let repo_root = guess_repo_root(&repo_hint);
        let mut repo = detect_repo(&repo_root);
        if !repo.is_git_repo {
            emit(
                &app,
                ProgressEvent::error(
                    "repo",
                    format!(
                        "Not a git repository at {}. Self-improve will idle until \
                         a TerranSoul checkout is detected.",
                        repo_root.display()
                    ),
                ),
            );
        } else {
            emit(
                &app,
                ProgressEvent::info(
                    "repo",
                    format!(
                        "Bound to repo {} on branch {}",
                        repo.root.as_deref().unwrap_or("?"),
                        repo.current_branch.as_deref().unwrap_or("?"),
                    ),
                )
                .with_progress(5),
            );

            // Step 1 — pull latest from the configured base branch and
            // (when conflicts arise) ask the Coding LLM to resolve them.
            // Failures here are surfaced but never fatal — the loop
            // continues with whatever local state exists.
            let base_branch = github_cfg
                .as_ref()
                .map(|g| g.default_base.clone())
                .unwrap_or_else(|| "main".to_string());
            emit(
                &app,
                ProgressEvent::info("pull", format!("Pulling latest origin/{base_branch}…"))
                    .with_progress(8),
            );
            let pull = git_ops::pull_main(&repo_root, &base_branch, Some(&config)).await;
            if pull.merged {
                emit(
                    &app,
                    ProgressEvent::success("pull", pull.message.clone()).with_progress(12),
                );
            } else {
                emit(&app, ProgressEvent::error("pull", pull.message.clone()));
            }
        }

        let mut completion_pr_opened = false;

        for cycle in 0..MAX_CYCLES {
            if cancel.load(Ordering::Relaxed) {
                emit(
                    &app,
                    ProgressEvent::info("stopped", "Self-improve disabled — exiting loop"),
                );
                break;
            }

            // Re-read milestones each cycle — the user may have edited it.
            let chunks = match read_milestones(&repo_root) {
                Ok(md) => parse_chunks(&md),
                Err(e) => {
                    emit(&app, ProgressEvent::error("milestones", e));
                    sleep_cancellable(&cancel, IDLE_SLEEP_SECS).await;
                    continue;
                }
            };

            let next = match next_not_started(&chunks) {
                Some(c) => c.clone(),
                None => {
                    emit(
                        &app,
                        ProgressEvent::info(
                            "idle",
                            "All chunks complete — idling. Loop will resume \
                             when a new not-started chunk appears in milestones.md.",
                        ),
                    );
                    // Once per "all done" transition, attempt to open a
                    // completion PR if the user has configured GitHub.
                    if !completion_pr_opened {
                        if let Some(cfg) = github_cfg.as_ref() {
                            try_open_completion_pr(&app, cfg, &repo_root).await;
                        }
                        completion_pr_opened = true;
                    }
                    sleep_cancellable(&cancel, IDLE_SLEEP_SECS).await;
                    continue;
                }
            };

            // A new chunk appeared — reset the PR-opened latch so future
            // completions trigger a fresh PR.
            completion_pr_opened = false;

            emit(
                &app,
                ProgressEvent::info("cycle", format!("Cycle {}: chunk {}", cycle + 1, next.id))
                    .with_chunk(&next.id),
            );

            // Refresh repo state before each plan in case the user
            // switched branches manually between cycles.
            repo = detect_repo(&repo_root);

            match execute_chunk_dag_with_retry(
                &app,
                &config,
                &repo,
                &repo_root,
                &next,
                &metrics,
                &workflow_cfg,
                worktree_dir.as_deref(),
                &cancel,
            )
            .await
            {
                Ok(result) => {
                    let success_message = format_execution_success(&next.id, &result);
                    match archive_completed_chunk(&repo_root, &next, &result) {
                        Ok(archive) => {
                            if let Err(error) = stage_paths(&repo_root, &archive.modified_paths) {
                                emit(
                                    &app,
                                    ProgressEvent::error(
                                        "archive",
                                        format!(
                                            "Archived chunk {}, but staging failed: {error}",
                                            next.id
                                        ),
                                    )
                                    .with_chunk(&next.id),
                                );
                            }
                            let next_message = archive
                                .next_chunk_id
                                .map(|id| format!(" next chunk: {id}"))
                                .unwrap_or_else(|| " all milestone chunks complete".to_string());
                            emit(
                                &app,
                                ProgressEvent::success(
                                    "complete",
                                    format!("{success_message}; archived.{next_message}"),
                                )
                                .with_chunk(&next.id)
                                .with_progress(100),
                            );
                        }
                        Err(error) => emit(
                            &app,
                            ProgressEvent::error(
                                "archive",
                                format!(
                                    "Chunk {} passed, but archive update failed: {error}",
                                    next.id
                                ),
                            )
                            .with_chunk(&next.id),
                        ),
                    }
                }
                Err(e) => emit(
                    &app,
                    ProgressEvent::error(
                        "gate",
                        format!("Execution DAG for chunk {} failed: {}", next.id, e),
                    )
                    .with_chunk(&next.id),
                ),
            }

            // Pause between cycles so we don't spam the LLM API. The
            // sleep is cancellable so disable acts immediately.
            sleep_cancellable(&cancel, IDLE_SLEEP_SECS).await;
        }

        emit(
            &app,
            ProgressEvent::info("exit", "Self-improve loop exited"),
        );
        // `data_dir` retained for future use (per-loop state files);
        // the conversation_learning hook fires from the chat command
        // pipeline, not inside this loop.
        let _ = data_dir;
        engine_for_task.running.store(false, Ordering::Relaxed);
    });

    let mut slot = engine.task.lock().await;
    *slot = Some(handle);
}

/// Attempt to open (or update) a completion Pull Request after the loop
/// has finished every chunk in `milestones.md`. Failures are emitted to
/// the UI but never crash the loop.
async fn try_open_completion_pr<R: Runtime>(
    app: &AppHandle<R>,
    cfg: &github::GitHubConfig,
    repo_root: &std::path::Path,
) {
    if !cfg.is_complete() {
        emit(
            app,
            ProgressEvent::info(
                "pr",
                "All chunks complete, but GitHub config is incomplete — skipping PR.",
            ),
        );
        return;
    }
    let head_branch = match git_ops::current_branch(repo_root) {
        Some(b) => b,
        None => {
            emit(
                app,
                ProgressEvent::error("pr", "Cannot open PR from detached HEAD"),
            );
            return;
        }
    };
    if head_branch == cfg.default_base {
        emit(
            app,
            ProgressEvent::info(
                "pr",
                format!("Currently on {head_branch}; nothing to PR against itself."),
            ),
        );
        return;
    }
    emit(
        app,
        ProgressEvent::info("pr", format!("Opening PR for {head_branch}…")).with_progress(50),
    );
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let title = format!("self-improve: complete autonomous chunks ({head_branch})");
    let body = "All `not-started` chunks in `rules/milestones.md` have been planned by \
                the TerranSoul self-improve loop. Please review and merge.\n\n\
                — Opened automatically by the self-improve engine."
        .to_string();
    match github::open_or_update_pr(&client, cfg, &head_branch, &title, &body).await {
        Ok(pr) => emit(
            app,
            ProgressEvent::success(
                "pr",
                format!(
                    "PR #{} {} ({})",
                    pr.number,
                    if pr.created { "opened" } else { "already open" },
                    pr.html_url
                ),
            )
            .with_progress(100),
        ),
        Err(e) => emit(
            app,
            ProgressEvent::error("pr", format!("PR open failed: {e}")),
        ),
    }
}

/// Sleep for `secs` seconds, returning early if cancellation is requested.
async fn sleep_cancellable(cancel: &Arc<AtomicBool>, secs: u64) {
    let deadline = std::time::Instant::now() + Duration::from_secs(secs);
    while std::time::Instant::now() < deadline {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::CodingLlmProvider;

    #[test]
    fn planner_prompt_includes_chunk_id_and_title() {
        let repo = RepoState {
            is_git_repo: true,
            root: Some("/tmp".to_string()),
            current_branch: Some("master".to_string()),
            remote_url: None,
            clean: true,
        };
        let chunk = ChunkRow {
            id: "25.4".to_string(),
            title: "Autonomous loop MVP".to_string(),
            status: "not-started".to_string(),
        };
        let msgs = planner_prompt(
            &repo,
            &chunk,
            &crate::coding::CodingWorkflowConfig::default(),
        );
        // system + user + assistant-prefill (rule 6 from prompting-rules).
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].role, "system");
        assert_eq!(msgs[1].role, "user");
        assert_eq!(msgs[2].role, "assistant");
        // System prompt carries the role + thinking-protocol scaffolding.
        assert!(msgs[0].content.contains("<role>"));
        assert!(msgs[0].content.contains("<thinking_protocol>"));
        assert!(msgs[0].content.contains("<plan>"));
        // User prompt carries the chunk id + title verbatim.
        assert!(msgs[1].content.contains("25.4"));
        assert!(msgs[1].content.contains("Autonomous loop MVP"));
        // Pre-fill steers the model into the analysis tag.
        assert_eq!(msgs[2].content, "<analysis>");
    }

    #[test]
    fn repair_planner_prompt_includes_failed_test_gate() {
        let repo = RepoState {
            is_git_repo: true,
            root: Some("/tmp".to_string()),
            current_branch: Some("master".to_string()),
            remote_url: None,
            clean: true,
        };
        let chunk = ChunkRow {
            id: "32.3".to_string(),
            title: "Self-improve chunk completion + retry".to_string(),
            status: "not-started".to_string(),
        };
        let msgs = repair_planner_prompt(
            &repo,
            &chunk,
            &crate::coding::CodingWorkflowConfig::default(),
            "vitest=Fail/2\nexpected archive row",
        );

        assert!(msgs[1].content.contains("<failed_test_gate>"));
        assert!(msgs[1].content.contains("vitest=Fail/2"));
        assert!(msgs[1].content.contains("only automatic retry"));
    }

    #[test]
    fn coder_prompt_requires_path_file_blocks() {
        let repo = RepoState {
            is_git_repo: true,
            root: Some("/tmp".to_string()),
            current_branch: Some("feature/self-improve-28-11".to_string()),
            remote_url: None,
            clean: true,
        };
        let chunk = ChunkRow {
            id: "28.11".to_string(),
            title: "Apply/review/test execution gate".to_string(),
            status: "not-started".to_string(),
        };
        let msgs = coder_prompt(
            &repo,
            &chunk,
            "1. Touch engine.rs",
            &crate::coding::CodingWorkflowConfig::default(),
        );
        assert_eq!(msgs.len(), 2);
        assert!(msgs[1].content.contains("<approved_plan>"));
        assert!(msgs[1]
            .content
            .contains("<file path=\"repo/relative/path.ext\">"));
        assert!(msgs[1].content.contains("complete file replacement blocks"));
    }

    #[test]
    fn extract_plan_path_hints_finds_repo_paths() {
        let paths = extract_plan_path_hints(
            "1. Update `src-tauri/src/coding/workflow.rs`.\n2. Add tests in src/App.test.ts; ignore https://example.com/docs.",
        );

        assert_eq!(
            paths,
            vec![
                "src-tauri/src/coding/workflow.rs".to_string(),
                "src/App.test.ts".to_string(),
            ]
        );
    }

    #[test]
    fn preview_diff_renders_full_file_replacement() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "pub fn old() {}\n").unwrap();
        let blocks = vec![FileBlock {
            path: "src/lib.rs".to_string(),
            content: "pub fn new() {}\n".to_string(),
        }];
        let diff = preview_diff(dir.path(), &blocks).unwrap();
        assert!(diff.contains("diff --git a/src/lib.rs b/src/lib.rs"));
        assert!(diff.contains("-pub fn old() {}"));
        assert!(diff.contains("+pub fn new() {}"));
    }

    #[test]
    fn file_snapshot_restores_existing_and_removes_created() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "old").unwrap();
        let blocks = vec![
            FileBlock {
                path: "src/lib.rs".to_string(),
                content: "new".to_string(),
            },
            FileBlock {
                path: "src/new.rs".to_string(),
                content: "created".to_string(),
            },
        ];
        let snapshot = FileSnapshot::capture(dir.path(), &blocks).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "new").unwrap();
        std::fs::write(dir.path().join("src/new.rs"), "created").unwrap();
        snapshot.restore(dir.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("src/lib.rs")).unwrap(),
            "old"
        );
        assert!(!dir.path().join("src/new.rs").exists());
    }

    #[test]
    fn summarize_tests_reports_suite_statuses() {
        let result = test_runner::TestRunResult {
            suites: vec![test_runner::SuiteResult {
                name: "vitest".to_string(),
                status: test_runner::SuiteStatus::Pass,
                attempts: 1,
                duration_ms: 10,
                exit_code: Some(0),
                stdout_tail: String::new(),
                stderr_tail: String::new(),
                spawn_error: None,
            }],
            all_green: true,
            total_duration_ms: 10,
            flaky_suites: Vec::new(),
        };
        assert_eq!(summarize_tests(&result), "vitest=Pass/1");
    }

    #[test]
    fn write_isolated_patch_uses_target_patch_dir() {
        let dir = tempfile::tempdir().unwrap();
        let patch_path = write_isolated_patch(dir.path(), "28.13", "diff --git a/a b/a\n").unwrap();

        assert!(patch_path.ends_with("28.13-isolated.patch"));
        assert!(patch_path.contains("terransoul-self-improve"));
        assert_eq!(
            std::fs::read_to_string(&patch_path).unwrap(),
            "diff --git a/a b/a\n"
        );
    }

    #[test]
    fn format_execution_success_mentions_isolated_patch() {
        let result = ExecutionGateResult {
            applied_paths: vec!["src/lib.rs".to_string()],
            test_summary: "vitest=Pass/1".to_string(),
            isolated_patch_path: Some(
                "target/terransoul-self-improve/patches/28.13.patch".to_string(),
            ),
            applied_isolated_patch_path: Some(
                "target/terransoul-self-improve/patches/28.13.patch".to_string(),
            ),
        };

        let message = format_execution_success("28.13", &result);

        assert!(message.contains("temporary worktree"));
        assert!(message.contains("patch saved"));
        assert!(message.contains("applied from"));
        assert!(message.contains("vitest=Pass/1"));
    }

    #[test]
    fn apply_isolated_patch_to_working_branch_modifies_temp_repo() {
        let dir = tempfile::tempdir().unwrap();
        run_git_for_test(dir.path(), &["init"]).unwrap();
        std::fs::write(dir.path().join("tracked.txt"), "base\n").unwrap();
        let patch_path = dir.path().join("change.patch");
        std::fs::write(
            &patch_path,
            "diff --git a/tracked.txt b/tracked.txt\n--- a/tracked.txt\n+++ b/tracked.txt\n@@ -1 +1 @@\n-base\n+patched\n",
        )
        .unwrap();

        apply_isolated_patch_to_working_branch(dir.path(), &patch_path).unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.path().join("tracked.txt")).unwrap(),
            "patched\n"
        );
    }

    #[test]
    fn archive_completed_chunk_updates_milestones_and_completion_log() {
        let dir = tempfile::tempdir().unwrap();
        let rules_dir = dir.path().join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::write(
            rules_dir.join("milestones.md"),
            "# TerranSoul — Milestones\n\n---\n\n## Next Chunk\n\n**Chunk 32.3 — Self-improve chunk completion + retry.**\n\n---\n\n## Phase 32 — Self Improve\n\n| ID | Status | Title | Goal |\n|---|---|---|---|\n| 32.3 | not-started | Self-improve chunk completion + retry | Archive successful chunks. |\n| 32.4 | not-started | Self-improve isolated patch auto-merge | Apply isolated patches. |\n\n---\n",
        )
        .unwrap();
        std::fs::write(
            rules_dir.join("completion-log.md"),
            "# TerranSoul — Completion Log\n\n---\n\n## Table of Contents\n\n| Entry | Date |\n|-------|------|\n| [Old](#old) | 2026-01-01 |\n\n---\n\n## Old\n\nDone.\n",
        )
        .unwrap();
        let chunk = ChunkRow {
            id: "32.3".to_string(),
            title: "Self-improve chunk completion + retry".to_string(),
            status: "not-started".to_string(),
        };
        let gate = ExecutionGateResult {
            applied_paths: vec!["src-tauri/src/coding/engine.rs".to_string()],
            test_summary: "cargo test=Pass/1".to_string(),
            isolated_patch_path: None,
            applied_isolated_patch_path: None,
        };

        let archive = archive_completed_chunk(dir.path(), &chunk, &gate).unwrap();

        assert_eq!(archive.next_chunk_id.as_deref(), Some("32.4"));
        assert_eq!(
            archive.modified_paths,
            vec![
                "rules/milestones.md".to_string(),
                "rules/completion-log.md".to_string(),
            ]
        );
        let milestones = std::fs::read_to_string(rules_dir.join("milestones.md")).unwrap();
        assert!(!milestones.contains("| 32.3 |"));
        assert!(milestones.contains("**Chunk 32.4 — Self-improve isolated patch auto-merge.**"));
        let log = std::fs::read_to_string(rules_dir.join("completion-log.md")).unwrap();
        assert!(log.contains("Chunk 32.3 — Self-improve chunk completion + retry"));
        assert!(log.contains("src-tauri/src/coding/engine.rs"));
        assert!(log.contains("Autonomous test gate — passed: cargo test=Pass/1"));
    }

    #[test]
    fn civil_from_days_formats_unix_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_819), (2027, 1, 1));
    }

    #[test]
    fn self_improve_execution_graph_is_linear_safe_gate() {
        let graph = self_improve_execution_graph();
        let config = self_improve_dag_config();
        dag_runner::validate_graph(&graph, &config).unwrap();
        let layers = dag_runner::compute_layers(&graph);
        assert_eq!(
            layers,
            vec![
                vec![DAG_NODE_PLAN.to_string()],
                vec![DAG_NODE_CODE.to_string()],
                vec![DAG_NODE_REVIEW.to_string()],
                vec![DAG_NODE_APPLY.to_string()],
                vec![DAG_NODE_TEST.to_string()],
                vec![DAG_NODE_STAGE.to_string()],
            ]
        );
    }

    #[test]
    fn self_improve_dag_config_declares_required_capabilities() {
        let config = self_improve_dag_config();
        for expected in ["llm_call", "review", "file_write", "test_run", "git_write"] {
            assert!(config
                .available_capabilities
                .contains(&expected.to_string()));
        }
        assert!(config.skip_on_failure);
        assert_eq!(config.max_parallel, 2);
    }

    #[test]
    fn format_dag_failure_includes_failed_and_skipped_nodes() {
        let result = dag_runner::DagRunResult {
            results: vec![
                dag_runner::NodeResult {
                    node_id: DAG_NODE_REVIEW.to_string(),
                    status: dag_runner::NodeStatus::Failed,
                    message: "review rejected diff".to_string(),
                    duration_ms: 1,
                },
                dag_runner::NodeResult {
                    node_id: DAG_NODE_APPLY.to_string(),
                    status: dag_runner::NodeStatus::Skipped,
                    message: "Skipped: predecessor failed".to_string(),
                    duration_ms: 0,
                },
            ],
            all_success: false,
            total_duration_ms: 1,
            failed_nodes: vec![DAG_NODE_REVIEW.to_string()],
            skipped_nodes: vec![DAG_NODE_APPLY.to_string()],
        };
        let message = format_dag_failure(&result);
        assert!(message.contains("reviewer: review rejected diff"));
        assert!(message.contains("skipped: apply"));
    }

    #[test]
    fn engine_starts_not_running() {
        let engine = SelfImproveEngine::new();
        assert!(!engine.is_running());
    }

    #[tokio::test]
    async fn request_stop_clears_running_flag() {
        let engine = Arc::new(SelfImproveEngine::new());
        engine.running.store(true, Ordering::Relaxed);
        engine.request_stop().await;
        assert!(!engine.is_running());
        assert!(engine.cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn progress_event_helpers_set_level() {
        let info = ProgressEvent::info("p", "m");
        assert_eq!(info.level, "info");
        let s = ProgressEvent::success("p", "m");
        assert_eq!(s.level, "success");
        let e = ProgressEvent::error("p", "m");
        assert_eq!(e.level, "error");
    }

    #[test]
    fn with_chunk_and_progress_chain() {
        let e = ProgressEvent::info("p", "m")
            .with_chunk("25.4")
            .with_progress(75);
        assert_eq!(e.chunk_id.as_deref(), Some("25.4"));
        assert_eq!(e.progress, 75);
    }

    fn run_git_for_test(cwd: &Path, args: &[&str]) -> Result<(), String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(|error| format!("git unavailable: {error}"))?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            stderr
        })
    }

    #[test]
    fn coding_llm_config_can_be_passed_to_engine_signature() {
        // Compile-time check only — confirms the public API accepts the
        // persisted config without an extra adapter.
        let _cfg = CodingLlmConfig {
            provider: CodingLlmProvider::Anthropic,
            model: "x".into(),
            base_url: "http://x".into(),
            api_key: "y".into(),
        };
    }
}
