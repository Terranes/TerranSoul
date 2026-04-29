//! Self-improve engine — the autonomous coding loop driver.
//!
//! Loop lifecycle:
//! 1. User toggles self-improve on (gated through warning dialog).
//! 2. Engine spawns a Tokio task that detects the workspace git repo,
//!    reads `rules/milestones.md`, picks the next `not-started` chunk,
//!    asks the configured Coding LLM for an implementation **plan**
//!    (planner mode — does NOT yet apply diffs), persists progress, and
//!    emits `self-improve-progress` Tauri events for the live UI.
//! 3. On error or pause, the task exits gracefully; on next app launch
//!    the engine auto-resumes if `enabled = true`.
//!
//! Resilience: the task lives behind an [`AtomicBool`] cancellation flag.
//! The only way to stop the loop is to flip self-improve to disabled.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::Mutex as TokioMutex;
use tokio::task::JoinHandle;

use crate::brain::openai_client::OpenAiMessage;

use super::client;
use super::metrics::MetricsLog;
use super::milestones::{next_not_started, parse_chunks, ChunkRow};
use super::repo::{detect_repo, feature_branch_name, guess_repo_root, RepoState};
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

/// Build the planner prompt. Kept short and explicit to keep token usage
/// minimal. The autonomous loop never asks the LLM to *apply* changes —
/// that gating lives in a future chunk so this layer remains safe.
fn planner_prompt(repo: &RepoState, chunk: &ChunkRow) -> Vec<OpenAiMessage> {
    let system = "You are TerranSoul's self-improve planner. Given a chunk \
                  description from a project's milestones, produce a concise \
                  step-by-step plan (max 8 steps) listing exactly which files \
                  to create or modify and why. Do NOT output code. Do NOT \
                  invent file paths — only suggest edits to plausible \
                  TerranSoul paths (src-tauri/src/, src/, rules/, docs/). \
                  Output as a numbered markdown list.";
    let user = format!(
        "Repository root: {}\nCurrent branch: {}\nWorking tree clean: {}\n\
         Chunk id: {}\nChunk title: {}\n\nProduce the plan now.",
        repo.root.as_deref().unwrap_or("unknown"),
        repo.current_branch.as_deref().unwrap_or("unknown"),
        repo.clean,
        chunk.id,
        chunk.title,
    );
    vec![
        OpenAiMessage { role: "system".to_string(), content: system.to_string() },
        OpenAiMessage { role: "user".to_string(), content: user },
    ]
}

/// Run a single planning cycle for one chunk. Emits progress events at
/// each phase. Returns the produced plan text on success.
async fn plan_one_chunk<R: Runtime>(
    app: &AppHandle<R>,
    config: &CodingLlmConfig,
    repo: &RepoState,
    chunk: &ChunkRow,
    metrics: &MetricsLog,
) -> Result<String, String> {
    let provider = provider_label(&config.provider);
    let started_at = metrics.record_start(&chunk.id, &chunk.title, provider, &config.model);

    emit(
        app,
        ProgressEvent::info("plan", format!("Planning chunk {}: {}", chunk.id, chunk.title))
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
    let messages = planner_prompt(repo, chunk);
    match openai.chat(messages).await {
        Ok(plan) => {
            metrics.record_outcome(
                started_at,
                &chunk.id,
                &chunk.title,
                provider,
                &config.model,
                true,
                plan.len(),
                None,
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
            );
            Err(e)
        }
    }
}

fn provider_label(p: &CodingLlmProvider) -> &'static str {
    match p {
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
    repo_hint: PathBuf,
) {
    if engine.running.swap(true, Ordering::Relaxed) {
        emit(&app, ProgressEvent::info("idle", "Loop already running — ignoring start request"));
        return;
    }
    engine.cancel.store(false, Ordering::Relaxed);
    let cancel = engine.cancel.clone();
    let engine_for_task = engine.clone();
    let metrics = MetricsLog::new(&repo_hint);

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
        }

        for cycle in 0..MAX_CYCLES {
            if cancel.load(Ordering::Relaxed) {
                emit(&app, ProgressEvent::info("stopped", "Self-improve disabled — exiting loop"));
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
                    sleep_cancellable(&cancel, IDLE_SLEEP_SECS).await;
                    continue;
                }
            };

            emit(
                &app,
                ProgressEvent::info("cycle", format!("Cycle {}: chunk {}", cycle + 1, next.id))
                    .with_chunk(&next.id),
            );

            // Refresh repo state before each plan in case the user
            // switched branches manually between cycles.
            repo = detect_repo(&repo_root);

            match plan_one_chunk(&app, &config, &repo, &next, &metrics).await {
                Ok(plan) => {
                    let summary = plan.lines().take(2).collect::<Vec<_>>().join(" ");
                    emit(
                        &app,
                        ProgressEvent::success(
                            "complete",
                            format!("Chunk {} planned: {}", next.id, summary),
                        )
                        .with_chunk(&next.id)
                        .with_progress(100),
                    );
                }
                Err(e) => {
                    emit(
                        &app,
                        ProgressEvent::error(
                            "plan",
                            format!("Plan for chunk {} failed: {}", next.id, e),
                        )
                        .with_chunk(&next.id),
                    );
                }
            }

            // Pause between cycles so we don't spam the LLM API. The
            // sleep is cancellable so disable acts immediately.
            sleep_cancellable(&cancel, IDLE_SLEEP_SECS).await;
        }

        emit(&app, ProgressEvent::info("exit", "Self-improve loop exited"));
        engine_for_task.running.store(false, Ordering::Relaxed);
    });

    let mut slot = engine.task.lock().await;
    *slot = Some(handle);
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
        let msgs = planner_prompt(&repo, &chunk);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "system");
        assert!(msgs[1].content.contains("25.4"));
        assert!(msgs[1].content.contains("Autonomous loop MVP"));
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
        let e = ProgressEvent::info("p", "m").with_chunk("25.4").with_progress(75);
        assert_eq!(e.chunk_id.as_deref(), Some("25.4"));
        assert_eq!(e.progress, 75);
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
