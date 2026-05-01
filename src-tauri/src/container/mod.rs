//! Container runtime abstraction — supports both Docker (Docker Desktop /
//! Docker Engine) and Podman so users on machines that cannot install Docker
//! Desktop for compliance reasons can still run the Ollama container quest.
//!
//! Detection precedence (when `preferred_container_runtime` is `Auto`):
//!   1. Docker CLI present **and** `docker info` succeeds → use **Docker**.
//!   2. Else `docker --version` succeeds (CLI installed but daemon stopped) → **Docker**
//!      (we will offer to start Docker Desktop).
//!   3. Else `podman --version` succeeds → use **Podman**.
//!   4. Else → no runtime detected; the frontend should prompt the user to
//!      install one.
//!
//! Users can override the auto-pick via the `preferred_container_runtime`
//! setting which is persisted in `AppSettings`.

use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;

/// Which container runtime to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

impl ContainerRuntime {
    /// Underlying CLI binary name.
    pub fn binary(self) -> &'static str {
        match self {
            ContainerRuntime::Docker => "docker",
            ContainerRuntime::Podman => "podman",
        }
    }

    /// Human-readable label for UI surfaces.
    pub fn label(self) -> &'static str {
        match self {
            ContainerRuntime::Docker => "Docker",
            ContainerRuntime::Podman => "Podman",
        }
    }
}

/// User preference for which runtime to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuntimePreference {
    /// Auto-detect: prefer Docker if available, else Podman.
    #[default]
    Auto,
    /// Force Docker (fail if absent).
    Docker,
    /// Force Podman (fail if absent).
    Podman,
}

impl From<RuntimePreference> for Option<ContainerRuntime> {
    fn from(p: RuntimePreference) -> Self {
        match p {
            RuntimePreference::Auto => None,
            RuntimePreference::Docker => Some(ContainerRuntime::Docker),
            RuntimePreference::Podman => Some(ContainerRuntime::Podman),
        }
    }
}

/// Detection report exposed to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeDetection {
    /// Whether the `docker` CLI is installed.
    pub docker_cli: bool,
    /// Whether the Docker daemon is responsive.
    pub docker_daemon: bool,
    /// Whether Docker Desktop is installed (helps decide whether to offer
    /// "start Docker Desktop").
    pub docker_desktop_installed: bool,
    /// Whether the `podman` CLI is installed.
    pub podman_cli: bool,
    /// Whether the Podman engine appears to work (`podman info` succeeds).
    /// On Linux native this is essentially always true once the CLI is
    /// installed; on macOS/Windows it requires `podman machine start`.
    pub podman_working: bool,
    /// The auto-picked runtime given the current state, if any.
    pub auto_pick: Option<ContainerRuntime>,
    /// Whether both runtimes are available (so the UI may show a picker).
    pub both_available: bool,
}

/// Probe both runtimes and return a structured detection report.
pub async fn detect_runtimes() -> RuntimeDetection {
    let docker_cli = run_silent("docker", &["--version"]).await.is_ok();
    let docker_daemon = if docker_cli {
        run_silent("docker", &["info"]).await.is_ok()
    } else {
        false
    };
    let docker_desktop_installed = detect_docker_desktop_installed();

    let podman_cli = run_silent("podman", &["--version"]).await.is_ok();
    let podman_working = if podman_cli {
        run_silent("podman", &["info"]).await.is_ok()
    } else {
        false
    };

    let auto_pick = if docker_cli {
        Some(ContainerRuntime::Docker)
    } else if podman_cli {
        Some(ContainerRuntime::Podman)
    } else {
        None
    };

    let both_available = docker_cli && podman_cli;

    RuntimeDetection {
        docker_cli,
        docker_daemon,
        docker_desktop_installed,
        podman_cli,
        podman_working,
        auto_pick,
        both_available,
    }
}

/// Resolve a concrete runtime from a user preference, returning a clear
/// error if the chosen runtime is unavailable.
pub async fn resolve_runtime(preference: RuntimePreference) -> Result<ContainerRuntime, String> {
    let detection = detect_runtimes().await;
    match preference {
        RuntimePreference::Docker => {
            if detection.docker_cli {
                Ok(ContainerRuntime::Docker)
            } else {
                Err("Docker is set as preferred runtime but the `docker` CLI is not installed."
                    .to_string())
            }
        }
        RuntimePreference::Podman => {
            if detection.podman_cli {
                Ok(ContainerRuntime::Podman)
            } else {
                Err("Podman is set as preferred runtime but the `podman` CLI is not installed."
                    .to_string())
            }
        }
        RuntimePreference::Auto => detection.auto_pick.ok_or_else(|| {
            "No container runtime detected. Install Docker Desktop (https://www.docker.com/products/docker-desktop/) or Podman (https://podman.io/)."
                .to_string()
        }),
    }
}

// ── Internal helpers (shared with brain::docker_ollama) ───────────────────

pub(crate) async fn run_silent(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}

/// Detect if Docker Desktop is installed (platform-specific file checks).
pub fn detect_docker_desktop_installed() -> bool {
    #[cfg(target_os = "windows")]
    {
        std::path::Path::new(r"C:\Program Files\Docker\Docker\Docker Desktop.exe").exists()
            || std::path::Path::new(r"C:\Program Files (x86)\Docker\Docker\Docker Desktop.exe")
                .exists()
    }

    #[cfg(target_os = "macos")]
    {
        std::path::Path::new("/Applications/Docker.app").exists()
    }

    #[cfg(target_os = "linux")]
    {
        std::path::Path::new("/opt/docker-desktop").exists()
            || std::path::Path::new("/usr/bin/dockerd").exists()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_names_match_runtime() {
        assert_eq!(ContainerRuntime::Docker.binary(), "docker");
        assert_eq!(ContainerRuntime::Podman.binary(), "podman");
    }

    #[test]
    fn labels_are_human_readable() {
        assert_eq!(ContainerRuntime::Docker.label(), "Docker");
        assert_eq!(ContainerRuntime::Podman.label(), "Podman");
    }

    #[test]
    fn runtime_preference_default_is_auto() {
        assert_eq!(RuntimePreference::default(), RuntimePreference::Auto);
    }

    #[test]
    fn preference_to_optional_runtime() {
        assert_eq!(
            Option::<ContainerRuntime>::from(RuntimePreference::Auto),
            None
        );
        assert_eq!(
            Option::<ContainerRuntime>::from(RuntimePreference::Docker),
            Some(ContainerRuntime::Docker)
        );
        assert_eq!(
            Option::<ContainerRuntime>::from(RuntimePreference::Podman),
            Some(ContainerRuntime::Podman)
        );
    }

    #[tokio::test]
    async fn detect_runtimes_returns_well_formed_struct() {
        let d = detect_runtimes().await;
        // Every field is read; auto_pick must agree with cli detection.
        let _ = d.docker_desktop_installed;
        let _ = d.docker_daemon;
        let _ = d.podman_working;
        if !d.docker_cli && !d.podman_cli {
            assert!(d.auto_pick.is_none());
            assert!(!d.both_available);
        }
        if d.docker_cli {
            assert_eq!(d.auto_pick, Some(ContainerRuntime::Docker));
        } else if d.podman_cli {
            assert_eq!(d.auto_pick, Some(ContainerRuntime::Podman));
        }
        assert_eq!(d.both_available, d.docker_cli && d.podman_cli);
    }

    #[tokio::test]
    async fn resolve_runtime_explicit_docker_errors_when_missing() {
        // We can't guarantee Docker is missing in CI, but when it IS missing
        // we expect a clear error rather than a panic. Either branch is fine.
        let result = resolve_runtime(RuntimePreference::Docker).await;
        match result {
            Ok(rt) => assert_eq!(rt, ContainerRuntime::Docker),
            Err(msg) => assert!(msg.to_lowercase().contains("docker")),
        }
    }

    #[tokio::test]
    async fn resolve_runtime_explicit_podman_errors_when_missing() {
        let result = resolve_runtime(RuntimePreference::Podman).await;
        match result {
            Ok(rt) => assert_eq!(rt, ContainerRuntime::Podman),
            Err(msg) => assert!(msg.to_lowercase().contains("podman")),
        }
    }

    #[tokio::test]
    async fn auto_preference_returns_runtime_or_install_hint() {
        match resolve_runtime(RuntimePreference::Auto).await {
            Ok(rt) => assert!(matches!(
                rt,
                ContainerRuntime::Docker | ContainerRuntime::Podman
            )),
            Err(msg) => {
                let lower = msg.to_lowercase();
                assert!(lower.contains("docker") && lower.contains("podman"));
            }
        }
    }
}
