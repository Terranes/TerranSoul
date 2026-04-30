//! Cross-platform launcher for the `code` CLI.
//!
//! Spawns `code <path>` detached so the child VS Code process keeps
//! running after TerranSoul exits. Captures the spawned PID for the
//! self-launched registry.

use std::path::Path;
use std::process::{Command, Stdio};

/// Name of the VS Code launcher binary on every platform. Could
/// become a parameter later (`code-insiders`, `vscodium`, `cursor`).
#[cfg(not(windows))]
const CODE_BIN: &str = "code";

/// On Windows the `code` shipped on PATH is actually a `.cmd` shim,
/// so `Command::new("code")` fails with "program not found" unless we
/// route through `cmd /C`. The shim's PID isn't useful for tracking
/// the long-running VS Code window anyway, but we capture *some* PID
/// for the registry — see `LauncherError::CommandFailed` for fallback.
#[cfg(windows)]
const CODE_BIN: &str = "code.cmd";

/// Errors the launcher can surface to the UI.
#[derive(Debug, thiserror::Error)]
pub enum LauncherError {
    #[error(
        "VS Code's `code` CLI was not found on PATH. \
         Open VS Code → Cmd/Ctrl+Shift+P → 'Shell Command: Install code in PATH', then retry."
    )]
    CodeBinaryNotFound,
    #[error("failed to spawn `code {target}`: {source}")]
    SpawnFailed {
        target: String,
        #[source]
        source: std::io::Error,
    },
}

/// Spawn `code <target>` detached from TerranSoul's lifecycle.
///
/// Returns the PID of the spawned process. On Windows this is the PID
/// of the `code.cmd` shim, which exits quickly after launching the
/// real VS Code window — that's still useful: a dead PID in the
/// registry just falls through to "launch new", which is the correct
/// behaviour. The Unix path returns the actual VS Code PID.
pub fn spawn_code(target: &Path) -> Result<u32, LauncherError> {
    let mut cmd = Command::new(CODE_BIN);
    cmd.arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // Note: `Child` does not wait on drop, so the child keeps running
    // after we exit this function. That gives us "fire and forget"
    // semantics without needing a custom pre_exec / setsid path.

    let child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            LauncherError::CodeBinaryNotFound
        } else {
            LauncherError::SpawnFailed {
                target: target.display().to_string(),
                source: e,
            }
        }
    })?;

    let pid = child.id();
    // Explicitly leak the Child so we don't even hold a `Drop` guard
    // around it; the OS reclaims the zombie when VS Code exits.
    std::mem::forget(child);
    Ok(pid)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::path::PathBuf;

    /// We can't really test the success path without VS Code being
    /// installed, and gating on `which code` would be flaky in CI.
    /// We *can* test the failure path by setting `PATH` to an empty
    /// dir (Unix) — Windows `cmd.exe` resolution is harder to hijack,
    /// so we skip that branch there.
    #[test]
    #[cfg(unix)]
    fn missing_binary_returns_friendly_error() {
        let original = std::env::var("PATH").ok();
        // SAFETY: tests run single-threaded by default for this case;
        // we restore PATH before returning.
        unsafe {
            std::env::set_var("PATH", "/nonexistent");
        }

        let result = spawn_code(&PathBuf::from("/tmp"));
        // Restore PATH first so a panic above doesn't leak.
        unsafe {
            match original {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }

        match result {
            Err(LauncherError::CodeBinaryNotFound) => {}
            other => panic!("expected CodeBinaryNotFound, got {other:?}"),
        }
    }

    #[test]
    fn launcher_error_messages_are_actionable() {
        let err = LauncherError::CodeBinaryNotFound;
        let msg = err.to_string();
        assert!(msg.contains("Install code in PATH"));
    }
}
