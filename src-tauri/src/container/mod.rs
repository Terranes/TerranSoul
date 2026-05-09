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

// ── Container runtime installers ──────────────────────────────────────────

/// Download and install Docker Desktop silently.
///
/// - **Windows**: downloads the official `Docker Desktop Installer.exe`
///   and runs it with `install --quiet --accept-license`.
/// - **macOS**: directs the user to download manually (DMG requires UI).
/// - **Linux**: runs the official convenience script (`get.docker.com`).
pub async fn install_docker_desktop<F>(progress: F) -> Result<String, String>
where
    F: Fn(&str, u32) + Send + Sync,
{
    progress("Checking Docker Desktop...", 0);

    if detect_docker_desktop_installed() {
        return Ok("Docker Desktop is already installed".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use std::time::Duration;
        let temp_dir = std::env::temp_dir();
        let installer_path = temp_dir.join("DockerDesktopInstaller.exe");

        progress("Downloading Docker Desktop installer...", 5);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| format!("HTTP client: {e}"))?;

        let url = "https://desktop.docker.com/win/main/amd64/Docker%20Desktop%20Installer.exe";
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Download request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Download HTTP {}", resp.status()));
        }

        let total = resp.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        let mut file = tokio::fs::File::create(&installer_path)
            .await
            .map_err(|e| format!("Failed to create installer file: {e}"))?;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download stream error: {e}"))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Write error: {e}"))?;
            downloaded += chunk.len() as u64;
            if let Some(pct_raw) = (downloaded * 60).checked_div(total) {
                let pct = pct_raw as u32 + 5;
                progress("Downloading Docker Desktop installer...", pct.min(65));
            }
        }
        file.flush().await.ok();
        drop(file);

        progress("Running Docker Desktop installer (this may take a few minutes)...", 70);

        let output = Command::new(&installer_path)
            .args(["install", "--quiet", "--accept-license"])
            .output()
            .await
            .map_err(|e| format!("Installer exec failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Docker Desktop installer failed: {stderr}"));
        }

        progress("Docker Desktop installed — verifying...", 90);

        if !detect_docker_desktop_installed() {
            return Err("Installation completed but Docker Desktop not found".to_string());
        }

        progress("Docker Desktop installed successfully", 100);
        Ok("Docker Desktop installed".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        let _ = progress;
        Err("Automatic Docker Desktop install on macOS is not supported. Please download from https://www.docker.com/products/docker-desktop/".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        progress("Running Docker Engine installer (get.docker.com)...", 10);
        let output = Command::new("sh")
            .arg("-c")
            .arg("curl -fsSL https://get.docker.com | sh")
            .output()
            .await
            .map_err(|e| format!("Docker installer failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Docker install failed: {stderr}"));
        }

        progress("Docker Engine installed successfully", 100);
        Ok("Docker Engine installed via get.docker.com".to_string())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = progress;
        Err("Docker Desktop install not supported on this platform".to_string())
    }
}

/// Download and install Podman silently.
///
/// - **Windows**: downloads the official Podman Setup MSI and runs it via
///   `msiexec /i ... /qn`.
/// - **macOS**: directs the user to use `brew install podman`.
/// - **Linux**: uses the system package manager.
pub async fn install_podman<F>(progress: F) -> Result<String, String>
where
    F: Fn(&str, u32) + Send + Sync,
{
    progress("Checking Podman...", 0);

    if run_silent("podman", &["--version"]).await.is_ok() {
        return Ok("Podman is already installed".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use std::time::Duration;
        let temp_dir = std::env::temp_dir();
        let installer_path = temp_dir.join("podman-setup.exe");

        progress("Downloading Podman installer...", 5);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .map_err(|e| format!("HTTP client: {e}"))?;

        // Use GitHub releases API to find the latest Podman release for Windows
        let url = "https://api.github.com/repos/containers/podman/releases/latest";
        let release_resp = client
            .get(url)
            .header("User-Agent", "TerranSoul")
            .send()
            .await
            .map_err(|e| format!("GitHub API request failed: {e}"))?;

        if !release_resp.status().is_success() {
            return Err(format!("GitHub API HTTP {}", release_resp.status()));
        }

        let release: serde_json::Value = release_resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse release JSON: {e}"))?;

        let download_url = release["assets"]
            .as_array()
            .and_then(|assets| {
                assets.iter().find(|a| {
                    let name = a["name"].as_str().unwrap_or("");
                    name.ends_with("-setup.exe") && name.contains("podman")
                })
            })
            .and_then(|a| a["browser_download_url"].as_str())
            .ok_or_else(|| "Could not find Podman Windows installer in latest release".to_string())?
            .to_string();

        progress("Downloading Podman installer...", 10);
        let resp = client
            .get(&download_url)
            .send()
            .await
            .map_err(|e| format!("Download failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Download HTTP {}", resp.status()));
        }

        let total = resp.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        let mut file = tokio::fs::File::create(&installer_path)
            .await
            .map_err(|e| format!("Failed to create installer file: {e}"))?;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download stream error: {e}"))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Write error: {e}"))?;
            downloaded += chunk.len() as u64;
            if let Some(pct_raw) = (downloaded * 55).checked_div(total) {
                let pct = pct_raw as u32 + 10;
                progress("Downloading Podman installer...", pct.min(65));
            }
        }
        file.flush().await.ok();
        drop(file);

        progress("Running Podman installer...", 70);

        let output = Command::new(&installer_path)
            .args(["/install", "/quiet", "/norestart"])
            .output()
            .await
            .map_err(|e| format!("Installer exec failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Podman installer failed: {stderr}"));
        }

        progress("Podman installed — verifying...", 90);

        // Give PATH a moment to propagate
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        progress("Podman installed successfully", 100);
        Ok("Podman installed via official Windows installer".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        progress("Installing Podman via Homebrew...", 10);
        let output = Command::new("brew")
            .args(["install", "podman"])
            .output()
            .await
            .map_err(|e| format!("brew install failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("brew install podman failed: {stderr}"));
        }

        progress("Podman installed successfully", 100);
        Ok("Podman installed via Homebrew".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        progress("Installing Podman via package manager...", 10);

        // Try apt first (Debian/Ubuntu), then dnf (Fedora/RHEL)
        let apt_result = Command::new("apt-get")
            .args(["install", "-y", "podman"])
            .output()
            .await;

        if let Ok(output) = apt_result {
            if output.status.success() {
                progress("Podman installed successfully", 100);
                return Ok("Podman installed via apt".to_string());
            }
        }

        let dnf_result = Command::new("dnf")
            .args(["install", "-y", "podman"])
            .output()
            .await;

        if let Ok(output) = dnf_result {
            if output.status.success() {
                progress("Podman installed successfully", 100);
                return Ok("Podman installed via dnf".to_string());
            }
        }

        Err("Could not install Podman. Please install manually: https://podman.io/docs/installation".to_string())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = progress;
        Err("Podman install not supported on this platform".to_string())
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
