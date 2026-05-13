//! Detect-and-link registry for verified companion AI applications.
//!
//! This registry is the **single source of truth** consumed by:
//! - the Companion AI marketplace UI (browse / detect / install button), and
//! - the chat-driven quest hook (the chat layer triggers the
//!   `companion-ecosystem` quest, which renders an Install button that
//!   calls [`crate::commands::companions::companions_run_guided_install`]).
//!
//! See `rules/milestones.md` Phase INTEGRATE for the verified scope:
//! - **Hermes Desktop** (`fathah/hermes-desktop`) — Electron GUI for Hermes Agent.
//! - **Hermes Agent** (`NousResearch/hermes-agent`) — Python CLI, MCP-config consumer.
//! - **OpenClaw** (`openclaw/openclaw`) — already integrated as the
//!   `openclaw-bridge` plugin; the registry surfaces upstream-CLI install state
//!   so the plugin can offer guided install.
//!
//! Temporal.io is intentionally **not** in this registry — per the
//! milestone, it is a workflow-engine design reference, not an integration.
//!
//! Detection is on-demand only: the [`detect_status`] helper spawns the
//! registered detect command and returns its result. No background polling.
//! Installation goes through [`plan_guided_install`] which always returns
//! [`GuidedInstallOutcome::RequiresElevation`] for OSes that support it,
//! letting the caller spawn an OS-elevated terminal.

use serde::{Deserialize, Serialize};

/// Operating systems for which a companion may register an installer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CompanionOs {
    Windows,
    MacOs,
    Linux,
}

impl CompanionOs {
    /// Returns the OS this build targets.
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(target_os = "macos")]
        {
            Self::MacOs
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            Self::Linux
        }
    }
}

/// A single shell invocation: a program and its argument vector.
///
/// Kept as `program` + `args` (not a joined command string) so the OS shell
/// never has to re-parse user-provided text — eliminates the
/// shell-injection class of bugs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShellCommand {
    pub program: String,
    pub args: Vec<String>,
    /// Human-readable summary, shown in the quest UI before the user clicks
    /// Install. Should describe both **what** the command does and **why**
    /// elevation is required.
    pub description: String,
}

impl ShellCommand {
    pub fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            description: description.into(),
        }
    }
}

/// One companion AI application in the registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompanionApp {
    /// Stable kebab-case identifier (e.g. `hermes-desktop`).
    pub id: String,
    /// User-facing name (e.g. `Hermes Desktop`).
    pub display_name: String,
    /// One-line role label shown in the quest UI (e.g. "Deep-research GUI").
    pub role: String,
    /// Canonical upstream homepage / project URL.
    pub official_url: String,
    /// Optional Windows install command.
    pub windows_install: Option<ShellCommand>,
    /// Optional macOS install command.
    pub macos_install: Option<ShellCommand>,
    /// Optional Linux install command.
    pub linux_install: Option<ShellCommand>,
    /// Optional detect command. Exit code `0` means "installed".
    pub detect: Option<ShellCommand>,
    /// When `true`, install MUST be wrapped in an OS elevation prompt
    /// before the registered command runs. This is the mandatory consent
    /// gate from the Phase INTEGRATE install policy.
    pub requires_elevation: bool,
}

impl CompanionApp {
    /// Returns the install command for the supplied OS, if any.
    pub fn install_for(&self, os: CompanionOs) -> Option<&ShellCommand> {
        match os {
            CompanionOs::Windows => self.windows_install.as_ref(),
            CompanionOs::MacOs => self.macos_install.as_ref(),
            CompanionOs::Linux => self.linux_install.as_ref(),
        }
    }
}

/// Result of [`detect_status`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum DetectStatus {
    /// Detect command exited 0. `version` carries the trimmed first stdout
    /// line when available.
    Installed { version: Option<String> },
    /// Detect command ran but exited non-zero.
    NotInstalled,
    /// No detect command registered, or the command could not be spawned.
    Unknown { reason: String },
}

/// Plan returned by [`plan_guided_install`] — describes the user-visible
/// install action without yet performing it.
///
/// The `RequiresElevation` variant is the canonical happy path: the caller
/// (a Tauri command) spawns an OS-elevated terminal that runs `program
/// args`. The OS itself shows the UAC / sudo prompt; TerranSoul never asks
/// for credentials.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum GuidedInstallOutcome {
    /// Install will run only after the OS elevation prompt is confirmed.
    RequiresElevation {
        program: String,
        args: Vec<String>,
        description: String,
        os: CompanionOs,
    },
    /// The registered install command does not need elevation (rare —
    /// reserved for user-scope package managers). Caller may still wrap it
    /// in a visible terminal, but no UAC prompt will appear.
    DirectInstall {
        program: String,
        args: Vec<String>,
        description: String,
        os: CompanionOs,
    },
    /// No install command is registered for this OS. UI should fall back
    /// to opening [`CompanionApp::official_url`] in the browser.
    NoInstallerForOs { os: CompanionOs },
    /// Companion id was not in the registry.
    Unknown { id: String },
}

/// Build the canonical companion registry.
///
/// This is a pure function with no side effects so it can be called from
/// both Tauri commands and unit tests without setup.
pub fn default_registry() -> Vec<CompanionApp> {
    vec![
        // Hermes Desktop — Electron GUI for Hermes Agent (deep research).
        CompanionApp {
            id: "hermes-desktop".into(),
            display_name: "Hermes Desktop".into(),
            role: "Deep-research GUI".into(),
            official_url: "https://github.com/fathah/hermes-desktop".into(),
            windows_install: Some(ShellCommand::new(
                "winget",
                ["install", "--id", "NousResearch.HermesDesktop", "--exact"],
                "Installs Hermes Desktop machine-wide via the Windows Package Manager. Requires UAC.",
            )),
            macos_install: None,
            linux_install: None,
            detect: Some(ShellCommand::new(
                "winget",
                ["list", "--id", "NousResearch.HermesDesktop", "--exact"],
                "Lists Hermes Desktop via winget; exit 0 means installed.",
            )),
            requires_elevation: true,
        },
        // Hermes Agent — Python CLI; already a TerranSoul MCP consumer.
        CompanionApp {
            id: "hermes-agent".into(),
            display_name: "Hermes Agent".into(),
            role: "Deep-research CLI (MCP consumer)".into(),
            official_url: "https://github.com/NousResearch/hermes-agent".into(),
            windows_install: Some(ShellCommand::new(
                "py",
                ["-m", "pip", "install", "--upgrade", "nous-hermes-agent"],
                "Installs the Hermes Agent CLI into the active Python via pip.",
            )),
            macos_install: Some(ShellCommand::new(
                "python3",
                ["-m", "pip", "install", "--upgrade", "nous-hermes-agent"],
                "Installs the Hermes Agent CLI into the active Python via pip.",
            )),
            linux_install: Some(ShellCommand::new(
                "python3",
                ["-m", "pip", "install", "--upgrade", "nous-hermes-agent"],
                "Installs the Hermes Agent CLI into the active Python via pip.",
            )),
            detect: Some(ShellCommand::new(
                "hermes-agent",
                ["--version"],
                "Runs the Hermes Agent CLI; exit 0 means installed.",
            )),
            // pip user-scope install does NOT need OS elevation.
            requires_elevation: false,
        },
        // OpenClaw — the upstream CLI behind the openclaw-bridge plugin.
        CompanionApp {
            id: "openclaw-cli".into(),
            display_name: "OpenClaw".into(),
            role: "Tool-use agent CLI (active openclaw-bridge plugin)".into(),
            official_url: "https://github.com/openclaw/openclaw".into(),
            windows_install: Some(ShellCommand::new(
                "winget",
                ["install", "--id", "OpenClaw.OpenClaw", "--exact"],
                "Installs OpenClaw machine-wide via the Windows Package Manager. Requires UAC.",
            )),
            macos_install: Some(ShellCommand::new(
                "brew",
                ["install", "openclaw"],
                "Installs OpenClaw via Homebrew (user scope, no sudo).",
            )),
            linux_install: Some(ShellCommand::new(
                "pipx",
                ["install", "openclaw"],
                "Installs OpenClaw into an isolated pipx env (no sudo).",
            )),
            detect: Some(ShellCommand::new(
                "openclaw",
                ["--version"],
                "Runs the OpenClaw CLI; exit 0 means installed.",
            )),
            requires_elevation: true,
        },
    ]
}

/// Return the registry entry whose [`CompanionApp::id`] matches `id`.
pub fn get(id: &str) -> Option<CompanionApp> {
    default_registry().into_iter().find(|c| c.id == id)
}

/// On-demand detection. Spawns the registered detect command and waits
/// for it to finish. Caller must ensure this only runs in response to an
/// explicit user click (per the Phase INTEGRATE install policy).
///
/// `runner` lets tests inject a fake process runner; production callers
/// pass [`spawn_detect_status_real`].
pub fn detect_status_with<F>(app: &CompanionApp, runner: F) -> DetectStatus
where
    F: FnOnce(&ShellCommand) -> Result<DetectOutput, String>,
{
    let Some(cmd) = app.detect.as_ref() else {
        return DetectStatus::Unknown {
            reason: "no detect command registered".into(),
        };
    };
    match runner(cmd) {
        Ok(output) if output.exit_code == 0 => DetectStatus::Installed {
            version: output
                .stdout_first_line
                .filter(|line| !line.trim().is_empty()),
        },
        Ok(_) => DetectStatus::NotInstalled,
        Err(reason) => DetectStatus::Unknown { reason },
    }
}

/// Minimal output captured by a detect-command runner.
#[derive(Debug, Clone, Default)]
pub struct DetectOutput {
    pub exit_code: i32,
    pub stdout_first_line: Option<String>,
}

/// Production runner: spawn the command via `std::process::Command`.
pub fn spawn_detect_status_real(cmd: &ShellCommand) -> Result<DetectOutput, String> {
    let output = std::process::Command::new(&cmd.program)
        .args(&cmd.args)
        .output()
        .map_err(|e| format!("spawn failed: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().map(|s| s.trim().to_string());
    Ok(DetectOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout_first_line: first_line,
    })
}

/// Build the install plan for `app` on `os` without spawning anything.
///
/// This is the pure planner used by the Tauri command and exercised by the
/// UAC-flag unit test. The caller is responsible for actually launching an
/// OS-elevated terminal when the result is
/// [`GuidedInstallOutcome::RequiresElevation`].
pub fn plan_guided_install(app: &CompanionApp, os: CompanionOs) -> GuidedInstallOutcome {
    let Some(cmd) = app.install_for(os) else {
        return GuidedInstallOutcome::NoInstallerForOs { os };
    };
    if app.requires_elevation {
        GuidedInstallOutcome::RequiresElevation {
            program: cmd.program.clone(),
            args: cmd.args.clone(),
            description: cmd.description.clone(),
            os,
        }
    } else {
        GuidedInstallOutcome::DirectInstall {
            program: cmd.program.clone(),
            args: cmd.args.clone(),
            description: cmd.description.clone(),
            os,
        }
    }
}

/// Convenience: look up `id` then build the install plan for `os`.
pub fn plan_guided_install_by_id(id: &str, os: CompanionOs) -> GuidedInstallOutcome {
    match get(id) {
        Some(app) => plan_guided_install(&app, os),
        None => GuidedInstallOutcome::Unknown { id: id.into() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_shape_is_stable() {
        let registry = default_registry();
        // All three verified companions must be present, in a stable order.
        let ids: Vec<&str> = registry.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["hermes-desktop", "hermes-agent", "openclaw-cli"]);
        // Every entry must have a non-empty display name, role, and URL.
        for app in &registry {
            assert!(!app.display_name.is_empty(), "{} display_name", app.id);
            assert!(!app.role.is_empty(), "{} role", app.id);
            assert!(
                app.official_url.starts_with("https://"),
                "{} url not https",
                app.id
            );
            // Each entry must at least register one OS install command.
            assert!(
                app.windows_install.is_some()
                    || app.macos_install.is_some()
                    || app.linux_install.is_some(),
                "{} has no install command",
                app.id
            );
        }
    }

    #[test]
    fn requires_elevation_flag_produces_requires_elevation_variant() {
        // Hermes Desktop requires UAC on Windows.
        let app = get("hermes-desktop").expect("hermes-desktop in registry");
        assert!(app.requires_elevation);
        let plan = plan_guided_install(&app, CompanionOs::Windows);
        match plan {
            GuidedInstallOutcome::RequiresElevation { program, os, .. } => {
                assert_eq!(program, "winget");
                assert_eq!(os, CompanionOs::Windows);
            }
            other => panic!("expected RequiresElevation, got {other:?}"),
        }
    }

    #[test]
    fn non_elevated_companion_produces_direct_install_variant() {
        // Hermes Agent installs via user-scope pip — no UAC needed.
        let app = get("hermes-agent").expect("hermes-agent in registry");
        assert!(!app.requires_elevation);
        let plan = plan_guided_install(&app, CompanionOs::Linux);
        assert!(
            matches!(plan, GuidedInstallOutcome::DirectInstall { .. }),
            "non-elevated companion must not produce RequiresElevation",
        );
    }

    #[test]
    fn missing_installer_returns_no_installer_for_os() {
        // Hermes Desktop has no macOS / Linux installer in the registry yet.
        let app = get("hermes-desktop").unwrap();
        let plan = plan_guided_install(&app, CompanionOs::Linux);
        assert_eq!(
            plan,
            GuidedInstallOutcome::NoInstallerForOs {
                os: CompanionOs::Linux
            }
        );
    }

    #[test]
    fn unknown_id_returns_unknown_variant() {
        let plan = plan_guided_install_by_id("does-not-exist", CompanionOs::current());
        assert!(matches!(plan, GuidedInstallOutcome::Unknown { .. }));
    }

    #[test]
    fn detect_status_uses_injected_runner() {
        let app = get("hermes-agent").unwrap();
        // Installed branch: exit 0 with version string.
        let installed = detect_status_with(&app, |_| {
            Ok(DetectOutput {
                exit_code: 0,
                stdout_first_line: Some("hermes-agent 1.2.3".into()),
            })
        });
        assert_eq!(
            installed,
            DetectStatus::Installed {
                version: Some("hermes-agent 1.2.3".into())
            }
        );

        // NotInstalled branch: non-zero exit.
        let missing = detect_status_with(&app, |_| {
            Ok(DetectOutput {
                exit_code: 127,
                stdout_first_line: None,
            })
        });
        assert_eq!(missing, DetectStatus::NotInstalled);

        // Unknown branch: spawn error.
        let unknown = detect_status_with(&app, |_| Err("spawn failed: no such file".into()));
        assert!(matches!(unknown, DetectStatus::Unknown { .. }));
    }

    #[test]
    fn detect_status_missing_detect_command_is_unknown() {
        let app = CompanionApp {
            id: "no-detect".into(),
            display_name: "No Detect".into(),
            role: "test fixture".into(),
            official_url: "https://example.invalid/".into(),
            windows_install: None,
            macos_install: None,
            linux_install: None,
            detect: None,
            requires_elevation: false,
        };
        let status = detect_status_with(&app, |_| {
            panic!("runner must not be called when no detect command is registered")
        });
        assert!(matches!(status, DetectStatus::Unknown { .. }));
    }
}
