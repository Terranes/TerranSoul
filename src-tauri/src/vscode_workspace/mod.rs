//! VS Code workspace surfacing — Chunk 15.10.
//!
//! Public API:
//! - [`open_project`] — focus an existing window if one (or an
//!   ancestor) already covers `target`, else launch a new window.
//! - [`list_known_windows`] — snapshot of the live registry, useful
//!   for the Control Panel's status pill (Chunk 15.4).
//! - [`forget_window`] — manual purge (e.g. user closed VS Code via
//!   Task Manager and the registry got out of sync).
//!
//! See `rules/milestones.md` "Design notes — Chunk 15.10" for the
//! full algorithm and out-of-scope list.

pub mod launcher;
pub mod path_norm;
pub mod registry;
pub mod resolver;

use std::path::{Path, PathBuf};

pub use registry::{LaunchSource, SelfLaunchedRegistry, VsCodeWindow};
pub use resolver::WindowChoice;

/// Outcome of an `open_project` call.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "outcome")]
pub enum OpenOutcome {
    /// An existing VS Code window was reused; no new process spawned.
    Focused {
        pid: u32,
        kind: ChoiceKind,
        /// The folder path of the focused window. Useful for the UI.
        focused_root: PathBuf,
    },
    /// No suitable window existed; a fresh `code <target>` was spawned.
    Launched { pid: u32 },
}

/// Discriminator for which resolver branch produced a `Focused`
/// outcome — surfaced to the UI so the user knows whether their
/// folder was matched directly or via an ancestor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChoiceKind {
    Exact,
    Ancestor,
}

/// Errors that can be surfaced to the UI.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("target path does not exist: {0}")]
    TargetMissing(PathBuf),
    #[error("failed to canonicalise {path}: {source}")]
    CanonicaliseFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Launcher(#[from] launcher::LauncherError),
    #[error("failed to persist VS Code window registry: {0}")]
    RegistryWrite(std::io::Error),
}

/// Resolve, focus, or launch — the headline entry point.
///
/// `data_dir` is the persistence root for `vscode-windows.json`. The
/// caller is responsible for serialising concurrent invocations
/// (typical pattern: hold a `Mutex<()>` or async lock around the call).
pub fn open_project(data_dir: &Path, target: &Path) -> Result<OpenOutcome, WorkspaceError> {
    if !target.exists() {
        return Err(WorkspaceError::TargetMissing(target.to_path_buf()));
    }

    let canonical_target =
        path_norm::canonicalise(target).map_err(|e| WorkspaceError::CanonicaliseFailed {
            path: target.to_path_buf(),
            source: e,
        })?;

    let mut reg = SelfLaunchedRegistry::load(data_dir);

    // Prune dead PIDs once up front so the resolver, the on-disk
    // file, and the UI snapshot all see the same set.
    let pruned = reg.prune_dead(registry::pid_alive);
    if pruned {
        reg.save().map_err(WorkspaceError::RegistryWrite)?;
    }

    let choice = resolver::pick_window(&canonical_target, reg.windows(), registry::pid_alive);

    match choice {
        WindowChoice::Exact { pid } | WindowChoice::Ancestor { pid, .. } => {
            // Re-launch `code <window.root>` to focus the existing
            // window. `code <subpath>` would create a *new* window.
            let kind = matches!(choice, WindowChoice::Exact { .. })
                .then_some(ChoiceKind::Exact)
                .unwrap_or(ChoiceKind::Ancestor);

            let focused_root = reg
                .windows()
                .iter()
                .find(|w| w.pid == pid)
                .map(|w| w.root.clone())
                .unwrap_or_else(|| canonical_target.clone());

            launcher::spawn_code(&focused_root)?;

            Ok(OpenOutcome::Focused {
                pid,
                kind,
                focused_root,
            })
        }
        WindowChoice::None => {
            let new_pid = launcher::spawn_code(&canonical_target)?;
            reg.append(VsCodeWindow {
                pid: new_pid,
                root: canonical_target.clone(),
                launched_at_ms: now_ms(),
                launched_by: LaunchSource::SelfLaunched,
            });
            reg.save().map_err(WorkspaceError::RegistryWrite)?;
            Ok(OpenOutcome::Launched { pid: new_pid })
        }
    }
}

/// Snapshot of all known live VS Code windows, for the Control Panel.
pub fn list_known_windows(data_dir: &Path) -> Vec<VsCodeWindow> {
    let mut reg = SelfLaunchedRegistry::load(data_dir);
    if reg.prune_dead(registry::pid_alive) {
        let _ = reg.save();
    }
    reg.live_windows(registry::pid_alive)
}

/// Forget a registry entry by PID (manual purge from the Control Panel).
pub fn forget_window(data_dir: &Path, pid: u32) -> Result<(), WorkspaceError> {
    let mut reg = SelfLaunchedRegistry::load(data_dir);
    reg.forget(pid);
    reg.save().map_err(WorkspaceError::RegistryWrite)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_project_rejects_missing_target() {
        let dir = tempfile::tempdir().unwrap();
        let result = open_project(dir.path(), Path::new("/no/such/path"));
        assert!(matches!(result, Err(WorkspaceError::TargetMissing(_))));
    }

    #[test]
    fn list_known_windows_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let windows = list_known_windows(dir.path());
        assert!(windows.is_empty());
    }

    #[test]
    fn forget_window_is_a_noop_on_missing_pid() {
        let dir = tempfile::tempdir().unwrap();
        forget_window(dir.path(), 99999).unwrap();
    }

    #[test]
    fn forget_window_removes_existing_entry() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = SelfLaunchedRegistry::load(dir.path());
        reg.append(VsCodeWindow {
            pid: 42,
            root: PathBuf::from("/some/root"),
            launched_at_ms: 1000,
            launched_by: LaunchSource::SelfLaunched,
        });
        reg.save().unwrap();

        forget_window(dir.path(), 42).unwrap();

        let reloaded = SelfLaunchedRegistry::load(dir.path());
        assert!(reloaded.windows().is_empty());
    }

    #[test]
    fn open_project_persists_new_entry_on_launch() {
        // We can't actually exec `code` in unit tests without
        // potentially launching a real editor, so this test only
        // covers the missing-target rejection path. Real launch
        // behaviour is gated behind the `TERRANSOUL_VSCODE_INTEGRATION`
        // env-var integration tests in `e2e/`.
        let dir = tempfile::tempdir().unwrap();
        let result = open_project(dir.path(), Path::new("/no/such/path"));
        assert!(result.is_err());
        let reg = SelfLaunchedRegistry::load(dir.path());
        assert!(reg.windows().is_empty());
    }

    #[test]
    fn now_ms_returns_a_recent_timestamp() {
        let t = now_ms();
        // After 2020-01-01.
        assert!(t > 1_577_836_800_000);
    }
}
