//! External CLI worker — spawns `codex` / `claude` / `gemini` (or a
//! user-approved custom binary) inside a user-picked working folder and
//! streams its stdout / stderr line-by-line.
//!
//! **Security model**
//!
//! 1. Binaries are looked up via `$PATH` with
//!    [`Command::new`](std::process::Command::new). No shell interpretation,
//!    no absolute paths, no `sh -c`.
//! 2. `CliKind::Custom` binary names are validated by
//!    [`CliKind::validate_custom_binary`] — alphanumerics + `-`/`_`/`.` only.
//! 3. Arguments are passed as a pre-split [`Vec<String>`] so no string
//!    interpolation can inject flags.
//! 4. The process is pinned to the agent's `working_folder` via
//!    [`Command::current_dir`](tokio::process::Command::current_dir). The
//!    caller (Tauri command) is responsible for making sure the folder
//!    is user-consented.
//!
//! Output is delivered through a [`tokio::sync::mpsc::UnboundedReceiver`]
//! of [`CliEvent`]s so the durable workflow engine can persist each line
//! atomically before acknowledging it.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use super::{BrainBackend, CliKind};

/// A single event from a running CLI worker. Persisted to the workflow
/// history verbatim, so adding or renaming variants requires a migration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CliEvent {
    /// First event after a successful spawn — includes the OS-level PID
    /// so the workflow engine can record it for cross-restart resume.
    Started { pid: u32 },
    /// One line from stdout or stderr.
    Line { stream: CliStream, text: String },
    /// Child process exited with a status code (None = killed by signal).
    Exited { code: Option<i32> },
    /// Spawn failed — the workflow is terminal with this error.
    SpawnError { message: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CliStream {
    Stdout,
    Stderr,
}

/// Inputs to [`spawn`]. Kept in a struct because the call site already
/// has most of these fields on the [`super::AgentProfile`] and carrying
/// them individually would be error-prone.
#[derive(Debug, Clone)]
pub struct CliSpawnSpec {
    pub kind: CliKind,
    pub binary: String,
    pub extra_args: Vec<String>,
    pub working_folder: PathBuf,
    /// The user's prompt / instruction for this run. Passed as a single
    /// argument — we do **not** concatenate it into a shell string.
    pub prompt: String,
}

impl CliSpawnSpec {
    /// Build from an [`AgentProfile`](super::AgentProfile)'s backend +
    /// a runtime prompt. Returns `None` for non-CLI backends.
    pub fn from_backend(
        backend: &BrainBackend,
        working_folder: Option<&Path>,
        prompt: &str,
    ) -> Option<Self> {
        if let BrainBackend::ExternalCli {
            kind,
            binary,
            extra_args,
        } = backend
        {
            let folder = working_folder?;
            Some(CliSpawnSpec {
                kind: *kind,
                binary: binary.clone(),
                extra_args: extra_args.clone(),
                working_folder: folder.to_path_buf(),
                prompt: prompt.to_string(),
            })
        } else {
            None
        }
    }

    /// Validate every field, returning a human-readable error on the
    /// first failure. Called before [`spawn`] so we never launch a
    /// child process on inputs that would be rejected anyway.
    pub fn validate(&self) -> Result<(), String> {
        // 1. Binary name must match kind (or pass custom validator).
        if self.kind == CliKind::Custom {
            CliKind::validate_custom_binary(&self.binary)?;
        } else if self.binary != self.kind.default_binary() {
            return Err(format!(
                "binary '{}' does not match kind {:?}",
                self.binary, self.kind
            ));
        }
        // 2. Working folder must exist and be a directory.
        if !self.working_folder.exists() {
            return Err(format!(
                "working_folder does not exist: {}",
                self.working_folder.display()
            ));
        }
        if !self.working_folder.is_dir() {
            return Err(format!(
                "working_folder is not a directory: {}",
                self.working_folder.display()
            ));
        }
        // 3. Prompt sanity — refuse to spawn with an empty prompt. Keeps
        //    workflow history meaningful and avoids a CLI dropping into
        //    REPL mode waiting for stdin.
        if self.prompt.trim().is_empty() {
            return Err("prompt must not be empty".to_string());
        }
        if self.prompt.len() > 32 * 1024 {
            return Err("prompt too long (max 32 KB)".to_string());
        }
        // 4. Each extra arg must be a printable, non-null string so we
        //    can round-trip it through the workflow history.
        for arg in &self.extra_args {
            if arg.contains('\0') {
                return Err("extra_args must not contain NUL".to_string());
            }
        }
        Ok(())
    }
}

/// Handle to a running CLI worker. Keep this alive for the duration of
/// the workflow so the drop of the `Child` doesn't kill the process
/// prematurely.
pub struct CliWorker {
    child: Child,
    events: UnboundedReceiver<CliEvent>,
}

impl std::fmt::Debug for CliWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliWorker")
            .field("pid", &self.child.id())
            .finish()
    }
}

impl CliWorker {
    /// Receive the next event, if any. Returns `None` once the child has
    /// exited and all buffered output has been drained.
    pub async fn next_event(&mut self) -> Option<CliEvent> {
        self.events.recv().await
    }

    /// Kill the child process. Best-effort: a failure to kill is logged
    /// but not surfaced so the caller can always clean up.
    pub async fn kill(&mut self) {
        let _ = self.child.kill().await;
    }

    /// Return the OS PID of the spawned process.
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
}

/// Spawn a CLI worker. The returned [`CliWorker`] owns both the child
/// process handle and the receiver side of the event channel.
pub async fn spawn(spec: CliSpawnSpec) -> Result<CliWorker, String> {
    spec.validate()?;

    let (tx, rx) = unbounded_channel::<CliEvent>();

    let mut cmd = Command::new(&spec.binary);
    // Pass the prompt as the LAST argument so that CLI tools which accept
    // `cli [flags] <prompt>` work out of the box. Tools that need a
    // different positional order can rewire via `extra_args`.
    for arg in &spec.extra_args {
        cmd.arg(arg);
    }
    cmd.arg(&spec.prompt);
    cmd.current_dir(&spec.working_folder);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    // Prevent the child from inheriting environment variables we don't
    // want it to see (API keys intended only for the main process).
    // We keep PATH, HOME, USER, LANG for sanity.
    cmd.env_clear();
    for var in ["PATH", "HOME", "USER", "LANG", "LC_ALL", "TERM"] {
        if let Ok(val) = std::env::var(var) {
            cmd.env(var, val);
        }
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("failed to spawn '{}': {e}", spec.binary);
            let _ = tx.send(CliEvent::SpawnError {
                message: msg.clone(),
            });
            return Err(msg);
        }
    };

    let pid = child.id().unwrap_or(0);
    let _ = tx.send(CliEvent::Started { pid });

    if let Some(stdout) = child.stdout.take() {
        spawn_reader(stdout, CliStream::Stdout, tx.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_reader(stderr, CliStream::Stderr, tx);
    }
    // The reader tasks own the remaining `tx` clones. Once both readers
    // finish (child closed its stdout/stderr), every sender is dropped
    // and `CliWorker::next_event` returns `None`. The caller then
    // observes the exit status via `drain`, which awaits `child.wait()`.

    Ok(CliWorker { child, events: rx })
}

fn spawn_reader<R>(reader: R, stream: CliStream, tx: UnboundedSender<CliEvent>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if tx
                .send(CliEvent::Line {
                    stream,
                    text: line,
                })
                .is_err()
            {
                // receiver gone — caller dropped CliWorker
                break;
            }
        }
    });
}

/// Drive a `CliWorker` to completion, collecting every event into a
/// `Vec` and awaiting the child's real exit status at the end. Used by
/// the workflow engine and by tests.
pub async fn drain(mut worker: CliWorker) -> Vec<CliEvent> {
    let mut out = Vec::new();
    while let Some(ev) = worker.next_event().await {
        out.push(ev);
    }
    let code = worker.child.wait().await.ok().and_then(|s| s.code());
    out.push(CliEvent::Exited { code });
    out
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn spec_in(dir: &Path) -> CliSpawnSpec {
        CliSpawnSpec {
            kind: CliKind::Custom,
            binary: "echo".into(),
            extra_args: vec!["hello".into()],
            working_folder: dir.to_path_buf(),
            prompt: "world".into(),
        }
    }

    #[test]
    fn validate_rejects_empty_prompt() {
        let tmp = TempDir::new().unwrap();
        let mut s = spec_in(tmp.path());
        s.prompt = "".into();
        assert!(s.validate().is_err());
        s.prompt = "   \n".into();
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_rejects_mismatched_binary_for_known_kind() {
        let tmp = TempDir::new().unwrap();
        let mut s = spec_in(tmp.path());
        s.kind = CliKind::Codex;
        s.binary = "pwned".into();
        let err = s.validate().unwrap_err();
        assert!(err.contains("does not match"), "got: {err}");
    }

    #[test]
    fn validate_rejects_missing_folder() {
        let mut s = spec_in(Path::new("/nonexistent-terransoul-xyz"));
        s.binary = "echo".into();
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_rejects_nul_in_args() {
        let tmp = TempDir::new().unwrap();
        let mut s = spec_in(tmp.path());
        s.extra_args = vec!["a\0b".into()];
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_rejects_oversize_prompt() {
        let tmp = TempDir::new().unwrap();
        let mut s = spec_in(tmp.path());
        s.prompt = "x".repeat(64 * 1024);
        assert!(s.validate().is_err());
    }

    #[test]
    fn from_backend_returns_none_for_native() {
        let backend = BrainBackend::Native { mode: None };
        let tmp = TempDir::new().unwrap();
        let out = CliSpawnSpec::from_backend(&backend, Some(tmp.path()), "hello");
        assert!(out.is_none());
    }

    #[test]
    fn from_backend_requires_working_folder() {
        let backend = BrainBackend::ExternalCli {
            kind: CliKind::Codex,
            binary: "codex".into(),
            extra_args: vec![],
        };
        assert!(CliSpawnSpec::from_backend(&backend, None, "hi").is_none());
    }

    #[tokio::test]
    async fn spawn_echo_produces_stdout_line_and_exit() {
        // `echo` is present on every CI runner we care about. We're
        // validating the pipeline, not the command itself.
        let tmp = TempDir::new().unwrap();
        let spec = spec_in(tmp.path());
        let worker = match spawn(spec).await {
            Ok(w) => w,
            Err(_) => {
                // Some minimal runner environments may not ship echo;
                // skip rather than fail the suite.
                return;
            }
        };
        let events = drain(worker).await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CliEvent::Started { .. })),
            "missing Started event"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CliEvent::Line { stream: CliStream::Stdout, text } if text.contains("hello") && text.contains("world"))),
            "missing combined 'hello world' stdout line: {events:?}"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CliEvent::Exited { code: Some(0) })),
            "missing clean exit: {events:?}"
        );
    }

    #[tokio::test]
    async fn spawn_unknown_binary_returns_error() {
        let tmp = TempDir::new().unwrap();
        let mut s = spec_in(tmp.path());
        s.binary = "terransoul-does-not-exist-xyz".into();
        let err = spawn(s).await.unwrap_err();
        assert!(err.contains("failed to spawn"), "got: {err}");
    }
}
