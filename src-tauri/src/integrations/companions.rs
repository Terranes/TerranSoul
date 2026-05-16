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
    /// Optional `owner/repo` slug for the upstream GitHub repository.
    ///
    /// When present, the marketplace can poll
    /// `https://api.github.com/repos/{owner}/{repo}/releases/latest` to
    /// surface an "Update available" badge — the NuGet-style update flow
    /// requested in the 2026-05-14 marketplace update.
    /// `None` means the companion has no upstream release feed we know
    /// how to read (e.g. an internal-only or non-GitHub project).
    pub github_repo: Option<String>,
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
        //
        // The winget manifest (`NousResearch.HermesDesktop`) is not yet
        // accepted into `microsoft/winget-pkgs` (per the upstream README
        // as of 2026-05-14), so we install directly from the GitHub
        // Releases NSIS `.exe` and detect via the user-scope install path.
        CompanionApp {
            id: "hermes-desktop".into(),
            display_name: "Hermes Desktop".into(),
            role: "Deep-research GUI".into(),
            official_url: "https://github.com/fathah/hermes-desktop".into(),
            windows_install: Some(ShellCommand::new(
                "powershell",
                [
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    "$ErrorActionPreference='Stop'; $r=Invoke-RestMethod -Uri 'https://api.github.com/repos/fathah/hermes-desktop/releases/latest' -Headers @{'User-Agent'='TerranSoul'}; $a=$r.assets | Where-Object { $_.name -like '*setup*.exe' -and $_.name -notlike '*blockmap*' } | Select-Object -First 1; if(-not $a){throw 'No Windows setup.exe in latest release'}; $dest=Join-Path $env:TEMP $a.name; Invoke-WebRequest -Uri $a.browser_download_url -OutFile $dest -UseBasicParsing; Start-Process -FilePath $dest -ArgumentList '/S' -Wait",
                ],
                "Downloads the latest Hermes Desktop NSIS installer from GitHub Releases (fathah/hermes-desktop) and runs a silent /S install into %LOCALAPPDATA%. Reused for both initial install and updates.",
            )),
            macos_install: None,
            linux_install: None,
            detect: Some(ShellCommand::new(
                "powershell",
                [
                    "-NoProfile",
                    "-Command",
                    "$p=Join-Path $env:LOCALAPPDATA 'Programs\\hermes-desktop\\hermes-agent.exe'; if(Test-Path $p){ (Get-Item $p).VersionInfo.ProductVersion } else { exit 1 }",
                ],
                "Reads ProductVersion from the installed Hermes Desktop executable under %LOCALAPPDATA%; exit 0 means installed.",
            )),
            // Hermes Desktop installs to %LOCALAPPDATA% (user scope) so no UAC.
            requires_elevation: false,
            github_repo: Some("fathah/hermes-desktop".into()),
        },
        // Hermes Agent — Python CLI; already a TerranSoul MCP consumer.
        CompanionApp {
            id: "hermes-agent".into(),
            display_name: "Hermes Agent".into(),
            role: "Deep-research CLI (MCP consumer)".into(),
            official_url: "https://github.com/NousResearch/hermes-agent".into(),
            windows_install: Some(ShellCommand::new(
                "powershell",
                ["-NoProfile", "-Command", "irm https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.ps1 | iex"],
                "Runs the official Hermes Agent Windows installer (downloads uv, Python, ripgrep, etc.).",
            )),
            macos_install: Some(ShellCommand::new(
                "bash",
                ["-c", "curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh | bash"],
                "Runs the official Hermes Agent installer for macOS/Linux.",
            )),
            linux_install: Some(ShellCommand::new(
                "bash",
                ["-c", "curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh | bash"],
                "Runs the official Hermes Agent installer for Linux.",
            )),
            detect: Some(ShellCommand::new(
                "hermes",
                ["--version"],
                "Runs the Hermes CLI; exit 0 means installed.",
            )),
            // The installer handles its own elevation when needed.
            requires_elevation: false,
            github_repo: Some("NousResearch/hermes-agent".into()),
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
            github_repo: Some("openclaw/openclaw".into()),
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

// ---------------------------------------------------------------------------
// Update checks (NuGet-style "update available" badge in the marketplace).
//
// Approach: when a companion declares `github_repo`, we hit the public
// `GET /repos/{owner}/{repo}/releases/latest` endpoint, parse the
// `tag_name`, normalise it to a numeric `MAJOR.MINOR.PATCH[.BUILD]`
// version, and compare against the version extracted from the detect
// command's stdout. No background polling \u2014 the Tauri command runs on
// explicit user click, mirroring `companions_detect_one`.
// ---------------------------------------------------------------------------

/// Minimal projection of a GitHub Releases-API response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LatestReleaseInfo {
    /// Normalised numeric version (e.g. `"0.3.7"`).
    pub version: String,
    /// Raw upstream tag (e.g. `"v0.3.7"`).
    pub tag: String,
    /// Browser-facing release page URL.
    pub html_url: String,
}

/// Outcome of an update check: combines local detect state with the
/// upstream release info so the UI can render one of:
/// - "Update available: vX \u2192 vY"   (`update_available: true`)
/// - "Up to date (vX)"                  (`update_available: false`)
/// - "Latest: vY (install to track)"    (`installed_version: None`)
/// - "Unknown" branches when either side fails.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateCheckResult {
    pub id: String,
    /// Trimmed version string parsed from the detect command's stdout,
    /// `None` if the companion is not installed or detect failed.
    pub installed_version: Option<String>,
    /// Latest release info from GitHub, `None` if no `github_repo` is
    /// registered or the API call failed.
    pub latest: Option<LatestReleaseInfo>,
    /// `true` only when both versions are known **and** latest > installed.
    pub update_available: bool,
    /// Free-form note shown beneath the badge (e.g. error reason).
    pub note: Option<String>,
}

/// Extract the first numeric `MAJOR.MINOR[.PATCH[.BUILD]]` substring from
/// arbitrary text. Returns `None` when no version-like token is found.
///
/// Examples:
/// - `"Hermes Agent v0.13.0 (2026.5.7)"` \u2192 `"0.13.0"`
/// - `"v0.3.7"`                          \u2192 `"0.3.7"`
/// - `"hermes 1.2"`                      \u2192 `"1.2"`
/// - `"no version here"`                 \u2192 `None`
pub fn parse_version(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            // Read digits + dots, but require at least one dot to qualify.
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let slice = &text[start..i];
            // Trim trailing dot if present (e.g. "1.2." \u2192 "1.2").
            let trimmed = slice.trim_end_matches('.');
            if trimmed.contains('.')
                && trimmed.split('.').all(|seg| !seg.is_empty() && seg.chars().all(|c| c.is_ascii_digit()))
            {
                return Some(trimmed.to_string());
            }
        } else {
            i += 1;
        }
    }
    None
}

/// Compare two numeric dotted versions. Returns `true` when `latest > installed`.
///
/// Segments are compared numerically left-to-right; missing trailing
/// segments on either side are treated as `0`.
pub fn is_newer(installed: &str, latest: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|seg| seg.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let a = parse(installed);
    let b = parse(latest);
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if y > x {
            return true;
        }
        if y < x {
            return false;
        }
    }
    false
}

/// Fetch the latest release for `owner/repo` from GitHub's public API.
///
/// Caller controls when this fires (no background polling). Network and
/// JSON errors surface as `Err`. The response is intentionally minimal so
/// the marketplace UI does not depend on the full release payload.
pub async fn fetch_latest_github_release(repo: &str) -> Result<LatestReleaseInfo, String> {
    use std::time::Duration;
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client: {e}"))?;
    let resp = client
        .get(&url)
        .header("User-Agent", "TerranSoul-CompanionRegistry")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("GitHub API request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GitHub API HTTP {} for {repo}", resp.status()));
    }
    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub release JSON: {e}"))?;
    let tag = value
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "release missing tag_name".to_string())?
        .to_string();
    let html_url = value
        .get("html_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("https://github.com/{repo}/releases/latest"));
    let version = parse_version(&tag).unwrap_or_else(|| tag.trim_start_matches('v').to_string());
    Ok(LatestReleaseInfo {
        version,
        tag,
        html_url,
    })
}

/// Run the detect command and the GitHub release lookup, then produce a
/// merged update-check result for `id`.
///
/// `runner` is the same hook used by [`detect_status_with`] so unit tests
/// can drive both halves without touching the network.
pub async fn check_update_with<F, Fut>(
    app: &CompanionApp,
    runner: F,
    fetch_latest: impl FnOnce(&str) -> Fut,
) -> UpdateCheckResult
where
    F: FnOnce(&ShellCommand) -> Result<DetectOutput, String>,
    Fut: std::future::Future<Output = Result<LatestReleaseInfo, String>>,
{
    let detect = detect_status_with(app, runner);
    let installed_version = match &detect {
        DetectStatus::Installed { version } => version.as_deref().and_then(parse_version),
        _ => None,
    };
    let mut note: Option<String> = None;
    let latest = match app.github_repo.as_deref() {
        Some(repo) => match fetch_latest(repo).await {
            Ok(info) => Some(info),
            Err(e) => {
                note = Some(format!("Update check failed: {e}"));
                None
            }
        },
        None => {
            note = Some("No upstream release feed registered".into());
            None
        }
    };
    let update_available = match (&installed_version, &latest) {
        (Some(i), Some(l)) => is_newer(i, &l.version),
        _ => false,
    };
    UpdateCheckResult {
        id: app.id.clone(),
        installed_version,
        latest,
        update_available,
        note,
    }
}

/// Production wrapper around [`check_update_with`] using
/// [`spawn_detect_status_real`] and the live GitHub API.
pub async fn check_update_real(app: &CompanionApp) -> UpdateCheckResult {
    check_update_with(app, spawn_detect_status_real, |repo| {
        let repo = repo.to_string();
        async move { fetch_latest_github_release(&repo).await }
    })
    .await
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
        // OpenClaw requires UAC on Windows (machine-wide winget install).
        let app = get("openclaw-cli").expect("openclaw-cli in registry");
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
    fn hermes_desktop_install_uses_github_releases_powershell() {
        // The winget manifest is not yet accepted upstream; the
        // marketplace must install via the GitHub Releases NSIS .exe.
        let app = get("hermes-desktop").expect("hermes-desktop in registry");
        let cmd = app
            .windows_install
            .as_ref()
            .expect("Windows installer registered");
        assert_eq!(cmd.program, "powershell");
        let joined = cmd.args.join(" ");
        assert!(
            joined.contains("api.github.com/repos/fathah/hermes-desktop/releases/latest"),
            "install script must hit GitHub Releases API"
        );
        assert!(joined.contains("/S"), "must use NSIS silent install flag");
        // Hermes Desktop installs to %LOCALAPPDATA% \u2014 no UAC.
        assert!(!app.requires_elevation);
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
            github_repo: None,
        };
        let status = detect_status_with(&app, |_| {
            panic!("runner must not be called when no detect command is registered")
        });
        assert!(matches!(status, DetectStatus::Unknown { .. }));
    }

    #[test]
    fn parse_version_extracts_first_dotted_token() {
        assert_eq!(parse_version("v0.3.7"), Some("0.3.7".into()));
        assert_eq!(
            parse_version("Hermes Agent v0.13.0 (2026.5.7)"),
            Some("0.13.0".into())
        );
        assert_eq!(parse_version("hermes 1.2"), Some("1.2".into()));
        assert_eq!(parse_version("no version here"), None);
        assert_eq!(parse_version("build 42"), None); // no dot \u2192 not a version
    }

    #[test]
    fn is_newer_compares_segments_numerically() {
        assert!(is_newer("0.3.6", "0.3.7"));
        assert!(is_newer("0.3.7", "0.4.0"));
        assert!(is_newer("0.9.0", "0.10.0")); // numeric, not lexicographic
        assert!(!is_newer("0.3.7", "0.3.7"));
        assert!(!is_newer("1.0.0", "0.9.9"));
        // Missing trailing segments treated as 0.
        assert!(is_newer("1.0", "1.0.1"));
        assert!(!is_newer("1.0.0", "1.0"));
    }

    #[tokio::test]
    async fn check_update_with_flags_update_when_latest_is_newer() {
        let app = get("hermes-desktop").unwrap();
        let result = check_update_with(
            &app,
            |_| {
                Ok(DetectOutput {
                    exit_code: 0,
                    stdout_first_line: Some("0.3.6".into()),
                })
            },
            |_repo| async {
                Ok(LatestReleaseInfo {
                    version: "0.3.7".into(),
                    tag: "v0.3.7".into(),
                    html_url: "https://github.com/fathah/hermes-desktop/releases/tag/v0.3.7".into(),
                })
            },
        )
        .await;
        assert_eq!(result.installed_version.as_deref(), Some("0.3.6"));
        assert_eq!(result.latest.as_ref().unwrap().version, "0.3.7");
        assert!(result.update_available);
    }

    #[tokio::test]
    async fn check_update_with_reports_up_to_date_when_equal() {
        let app = get("hermes-desktop").unwrap();
        let result = check_update_with(
            &app,
            |_| {
                Ok(DetectOutput {
                    exit_code: 0,
                    stdout_first_line: Some("0.3.7".into()),
                })
            },
            |_repo| async {
                Ok(LatestReleaseInfo {
                    version: "0.3.7".into(),
                    tag: "v0.3.7".into(),
                    html_url: "https://github.com/fathah/hermes-desktop/releases/tag/v0.3.7".into(),
                })
            },
        )
        .await;
        assert!(!result.update_available);
    }

    #[tokio::test]
    async fn check_update_with_handles_missing_install() {
        let app = get("hermes-desktop").unwrap();
        let result = check_update_with(
            &app,
            |_| {
                Ok(DetectOutput {
                    exit_code: 1,
                    stdout_first_line: None,
                })
            },
            |_repo| async {
                Ok(LatestReleaseInfo {
                    version: "0.3.7".into(),
                    tag: "v0.3.7".into(),
                    html_url: "https://example/".into(),
                })
            },
        )
        .await;
        assert_eq!(result.installed_version, None);
        // Latest is known but install is missing \u2192 not an "update", it's an install.
        assert!(!result.update_available);
        assert!(result.latest.is_some());
    }

    #[tokio::test]
    async fn check_update_with_surfaces_api_failure_in_note() {
        let app = get("hermes-desktop").unwrap();
        let result = check_update_with(
            &app,
            |_| {
                Ok(DetectOutput {
                    exit_code: 0,
                    stdout_first_line: Some("0.3.7".into()),
                })
            },
            |_repo| async { Err("HTTP 503".into()) },
        )
        .await;
        assert!(result.latest.is_none());
        assert!(result.note.as_deref().unwrap_or("").contains("HTTP 503"));
        assert!(!result.update_available);
    }
}
