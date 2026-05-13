//! Tauri command surface for the companion-AI detect-and-link registry.
//!
//! Chunk reference: **INTEGRATE-1** in `rules/milestones.md`.
//!
//! Detection runs only on explicit user click (the frontend calls
//! [`companions_detect_one`] when the user opens the integrations panel
//! or a quest sub-card). [`companions_run_guided_install`] always wraps
//! the install command in an OS-elevated terminal — the operating system
//! prompts for consent, not TerranSoul.

use crate::integrations::companions::{
    self, CompanionApp, CompanionOs, DetectStatus, GuidedInstallOutcome,
};

/// Return the entire companion registry (purely synthesised, no I/O).
#[tauri::command]
pub async fn companions_list() -> Result<Vec<CompanionApp>, String> {
    Ok(companions::default_registry())
}

/// Detect whether a single companion is installed on this machine.
///
/// Runs the registered detect command via `std::process::Command`. The
/// caller is expected to invoke this only in response to explicit user
/// action; there is no background polling.
#[tauri::command]
pub async fn companions_detect_one(id: String) -> Result<DetectStatus, String> {
    let Some(app) = companions::get(&id) else {
        return Ok(DetectStatus::Unknown {
            reason: format!("unknown companion id: {id}"),
        });
    };
    Ok(companions::detect_status_with(
        &app,
        companions::spawn_detect_status_real,
    ))
}

/// Open the companion's official upstream URL in the user's default browser.
///
/// This is the safe fallback when no install command is registered for the
/// current OS, or when the user explicitly wants to read the upstream docs
/// before installing.
#[tauri::command]
pub async fn companions_open_install_page(
    id: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let Some(app) = companions::get(&id) else {
        return Err(format!("unknown companion id: {id}"));
    };
    use tauri_plugin_shell::ShellExt;
    app_handle
        .shell()
        .open(&app.official_url, None)
        .map_err(|e| format!("failed to open install page: {e}"))
}

/// Build the guided-install plan for the current OS and, if applicable,
/// spawn an OS-elevated terminal that runs the install command.
///
/// The returned [`GuidedInstallOutcome`] always describes exactly what
/// happened — it is intended for the quest UI to surface to the user.
/// No silent installs: when the OS supports it, elevation is required
/// even for direct-install commands so the user always sees a terminal.
#[tauri::command]
pub async fn companions_run_guided_install(id: String) -> Result<GuidedInstallOutcome, String> {
    let os = CompanionOs::current();
    let plan = companions::plan_guided_install_by_id(&id, os);
    match &plan {
        GuidedInstallOutcome::RequiresElevation { program, args, .. } => {
            spawn_elevated_terminal(program, args)?;
        }
        GuidedInstallOutcome::DirectInstall { program, args, .. } => {
            spawn_visible_terminal(program, args)?;
        }
        GuidedInstallOutcome::NoInstallerForOs { .. } | GuidedInstallOutcome::Unknown { .. } => {
            // Nothing to launch — the UI will show the URL fallback.
        }
    }
    Ok(plan)
}

/// Spawn an OS-elevated terminal that runs `program args`. The OS shows
/// the elevation prompt (UAC on Windows, `osascript ... with administrator
/// privileges` on macOS, `pkexec` on Linux). TerranSoul never asks for
/// credentials directly.
fn spawn_elevated_terminal(program: &str, args: &[String]) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // PowerShell `Start-Process -Verb RunAs` triggers the UAC prompt.
        let arg_list = ps_args(args);
        let ps_cmd = format!(
            "Start-Process -FilePath '{}' -ArgumentList {} -Verb RunAs",
            escape_single_quotes(program),
            arg_list
        );
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_cmd])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("failed to spawn elevated PowerShell: {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        let joined = std::iter::once(program.to_string())
            .chain(args.iter().cloned())
            .map(|s| s.replace('"', "\\\""))
            .collect::<Vec<_>>()
            .join(" ");
        let script = format!(
            "do shell script \"{}\" with administrator privileges",
            joined
        );
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("failed to spawn osascript: {e}"))
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        // Linux: pkexec is the cross-DE elevation prompt.
        std::process::Command::new("pkexec")
            .arg(program)
            .args(args)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("failed to spawn pkexec: {e}"))
    }
}

/// Spawn a non-elevated, but **visible**, terminal that runs `program args`.
/// Visibility is the consent gate when no elevation is needed — the user
/// still sees the command run, can cancel, and never has a silent install.
fn spawn_visible_terminal(program: &str, args: &[String]) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // cmd /K keeps the window open so the user can see the result.
        let mut cmdline = vec![program.to_string()];
        cmdline.extend(args.iter().cloned());
        std::process::Command::new("cmd")
            .args(["/C", "start", "cmd", "/K"])
            .args(cmdline)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("failed to spawn visible terminal: {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        let joined = std::iter::once(program.to_string())
            .chain(args.iter().cloned())
            .map(|s| s.replace('"', "\\\""))
            .collect::<Vec<_>>()
            .join(" ");
        let script = format!("tell application \"Terminal\" to do script \"{}\"", joined);
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("failed to spawn macOS terminal: {e}"))
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        // Best-effort: xterm if present, otherwise raw spawn.
        if std::process::Command::new("xterm").arg("-version").output().is_ok() {
            let mut full = vec!["-e".to_string(), program.to_string()];
            full.extend(args.iter().cloned());
            std::process::Command::new("xterm")
                .args(full)
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("failed to spawn xterm: {e}"))
        } else {
            std::process::Command::new(program)
                .args(args)
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("failed to spawn install command: {e}"))
        }
    }
}

#[cfg(target_os = "windows")]
fn ps_args(args: &[String]) -> String {
    if args.is_empty() {
        // Start-Process requires at least one arg in -ArgumentList; pass an empty placeholder.
        return "@()".into();
    }
    let quoted: Vec<String> = args
        .iter()
        .map(|a| format!("'{}'", escape_single_quotes(a)))
        .collect();
    quoted.join(",")
}

#[cfg(target_os = "windows")]
fn escape_single_quotes(s: &str) -> String {
    s.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn companions_list_returns_registry() {
        let list = companions_list().await.expect("list");
        assert!(list.iter().any(|c| c.id == "hermes-desktop"));
        assert!(list.iter().any(|c| c.id == "hermes-agent"));
        assert!(list.iter().any(|c| c.id == "openclaw-cli"));
    }

    #[tokio::test]
    async fn companions_detect_unknown_id_returns_unknown_status() {
        let status = companions_detect_one("nope-not-real".into())
            .await
            .expect("ok");
        assert!(matches!(status, DetectStatus::Unknown { .. }));
    }
}
