use serde::Serialize;
use std::process::Stdio;
use tokio::process::Command;

use crate::container::{ContainerRuntime, RuntimePreference};

const CONTAINER_NAME: &str = "ollama";
const OLLAMA_IMAGE: &str = "ollama/ollama:latest";

/// Status report returned by `check_docker_status`.
#[derive(Debug, Clone, Serialize)]
pub struct DockerStatus {
    /// Whether the `docker` CLI is found on PATH.
    pub cli_found: bool,
    /// Whether the Docker daemon is responsive (`docker info` succeeds).
    pub daemon_running: bool,
    /// Whether Docker Desktop (or equivalent) is installed.
    pub desktop_installed: bool,
}

/// Status report for the Ollama container.
#[derive(Debug, Clone, Serialize)]
pub struct OllamaContainerStatus {
    /// Whether a container named "ollama" exists at all.
    pub exists: bool,
    /// Whether the container is currently running.
    pub running: bool,
    /// Whether the Ollama HTTP API on 127.0.0.1:11434 is reachable.
    pub api_reachable: bool,
}

/// Result of a setup step, streamed back as events.
#[derive(Debug, Clone, Serialize)]
pub struct SetupProgress {
    pub step: String,
    pub status: String, // "started" | "done" | "error"
    pub detail: String,
}

// ── Docker detection ─────────────────────────────────────────────────────────

/// Check whether Docker CLI is available and the daemon is running.
pub async fn check_docker_status() -> DockerStatus {
    let cli_found = run_silent("docker", &["--version"]).await.is_ok();

    let daemon_running = if cli_found {
        run_silent("docker", &["info"]).await.is_ok()
    } else {
        false
    };

    let desktop_installed = detect_docker_desktop_installed();

    DockerStatus {
        cli_found,
        daemon_running,
        desktop_installed,
    }
}

/// Attempt to launch Docker Desktop (Windows: starts the exe, macOS: `open -a`,
/// Linux: `systemctl start docker`). Returns Ok once the launch command has been
/// dispatched — it does NOT wait for the daemon to become ready.
pub async fn start_docker_desktop() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        // Try known install locations
        let paths = [
            r"C:\Program Files\Docker\Docker\Docker Desktop.exe",
            r"C:\Program Files (x86)\Docker\Docker\Docker Desktop.exe",
        ];
        for path in &paths {
            if std::path::Path::new(path).exists() {
                Command::new(path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| format!("Failed to launch Docker Desktop: {e}"))?;
                return Ok("Docker Desktop launch initiated".to_string());
            }
        }
        // Try via Start-Process in case it's on a custom path
        let status = Command::new("cmd")
            .args(["/C", "start", "", "Docker Desktop"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| format!("Failed to start Docker Desktop: {e}"))?;
        if status.success() {
            return Ok("Docker Desktop launch initiated via cmd".to_string());
        }
        Err("Docker Desktop not found. Please install it from https://www.docker.com/products/docker-desktop/".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .args(["-a", "Docker"])
            .status()
            .await
            .map_err(|e| format!("Failed to open Docker Desktop: {e}"))?;
        if status.success() {
            Ok("Docker Desktop launch initiated".to_string())
        } else {
            Err("Docker Desktop not found. Install from https://www.docker.com/products/docker-desktop/".to_string())
        }
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("systemctl")
            .args(["start", "docker"])
            .status()
            .await
            .map_err(|e| format!("Failed to start docker service: {e}"))?;
        if status.success() {
            Ok("Docker service started".to_string())
        } else {
            Err("Failed to start Docker service. Try: sudo systemctl start docker".to_string())
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported OS for Docker auto-start".to_string())
    }
}

/// Gracefully quit Docker Desktop to free memory after testing.
/// Windows: `taskkill` the "Docker Desktop" process.
/// macOS: `osascript -e 'quit app "Docker"'`.
/// Linux: `systemctl stop docker`.
pub async fn stop_docker_desktop() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let status = Command::new("taskkill")
            .args(["/IM", "Docker Desktop.exe", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(|e| format!("Failed to stop Docker Desktop: {e}"))?;
        if status.success() {
            Ok("Docker Desktop stopped".to_string())
        } else {
            Err("Docker Desktop process not found or could not be stopped".to_string())
        }
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("osascript")
            .args(["-e", "quit app \"Docker\""])
            .status()
            .await
            .map_err(|e| format!("Failed to quit Docker Desktop: {e}"))?;
        if status.success() {
            Ok("Docker Desktop quit".to_string())
        } else {
            Err("Failed to quit Docker Desktop".to_string())
        }
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("systemctl")
            .args(["stop", "docker"])
            .status()
            .await
            .map_err(|e| format!("Failed to stop docker service: {e}"))?;
        if status.success() {
            Ok("Docker service stopped".to_string())
        } else {
            Err("Failed to stop Docker service. Try: sudo systemctl stop docker".to_string())
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err("Unsupported OS for Docker auto-stop".to_string())
    }
}

/// Poll until `docker info` succeeds or the timeout is reached.
/// Returns true if the daemon became ready, false on timeout.
pub async fn wait_for_docker_ready(timeout_secs: u64) -> bool {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        if run_silent("docker", &["info"]).await.is_ok() {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    false
}

// ── Ollama container management ──────────────────────────────────────────────

/// Check the status of the Ollama Docker container.
pub async fn check_ollama_container() -> OllamaContainerStatus {
    check_ollama_container_for(ContainerRuntime::Docker).await
}

/// Variant of [`check_ollama_container`] that targets a specific runtime.
pub async fn check_ollama_container_for(runtime: ContainerRuntime) -> OllamaContainerStatus {
    let bin = runtime.binary();
    let exists = container_exists(bin, CONTAINER_NAME).await;
    let running = if exists {
        container_running(bin, CONTAINER_NAME).await
    } else {
        false
    };
    let api_reachable = check_ollama_api().await;

    OllamaContainerStatus {
        exists,
        running,
        api_reachable,
    }
}

/// Stop and remove the Ollama container and its associated Docker volume.
/// Best-effort: succeeds even if the container doesn't exist.
pub async fn remove_ollama_container() -> Result<String, String> {
    remove_ollama_container_for(ContainerRuntime::Docker).await
}

/// Variant of [`remove_ollama_container`] that targets a specific runtime.
pub async fn remove_ollama_container_for(
    runtime: ContainerRuntime,
) -> Result<String, String> {
    let bin = runtime.binary();
    let mut steps: Vec<String> = Vec::new();

    // Stop the container (ignore errors — may already be stopped or absent).
    let _ = run_command(bin, &["stop", CONTAINER_NAME]).await;

    // Remove the container.
    match run_command(bin, &["rm", "-f", CONTAINER_NAME]).await {
        Ok(_) => steps.push(format!("Removed container '{CONTAINER_NAME}'")),
        Err(_) => steps.push(format!("Container '{CONTAINER_NAME}' not found (already removed)")),
    }

    // Remove the named volume.
    match run_command(bin, &["volume", "rm", "ollama_data"]).await {
        Ok(_) => steps.push("Removed volume 'ollama_data'".to_string()),
        Err(_) => steps.push("Volume 'ollama_data' not found (already removed)".to_string()),
    }

    Ok(steps.join("\n"))
}

/// Ensure the Ollama container is running. Creates it if it doesn't exist,
/// starts it if stopped.  Detects NVIDIA GPU and enables `--gpus all` when
/// available.
pub async fn ensure_ollama_container() -> Result<String, String> {
    ensure_ollama_container_for(ContainerRuntime::Docker).await
}

/// Variant of [`ensure_ollama_container`] that targets a specific runtime.
/// Both Docker and Podman accept the same `run / start / -d` arguments.
pub async fn ensure_ollama_container_for(
    runtime: ContainerRuntime,
) -> Result<String, String> {
    let bin = runtime.binary();
    // If already running and API reachable, nothing to do
    let status = check_ollama_container_for(runtime).await;
    if status.running && status.api_reachable {
        return Ok(format!(
            "Ollama container already running (via {})",
            runtime.label()
        ));
    }

    // If container exists but stopped, start it
    if status.exists && !status.running {
        let out = run_command(bin, &["start", CONTAINER_NAME]).await?;
        // Wait for API
        wait_for_ollama_api(30).await?;
        return Ok(format!(
            "Ollama container started via {}: {out}",
            runtime.label()
        ));
    }

    // Container doesn't exist — create and run
    let has_gpu = detect_nvidia_gpu().await;
    let mut args = vec![
        "run", "-d",
        "--name", CONTAINER_NAME,
        "-p", "11434:11434",
        "-v", "ollama_data:/root/.ollama",
        "--restart", "unless-stopped",
    ];
    if has_gpu {
        // Both Docker and Podman accept `--gpus all` (Podman ≥ 4.1 with the
        // nvidia container toolkit installed). On systems without GPU
        // support this flag is simply omitted.
        args.insert(2, "all");
        args.insert(2, "--gpus");
    }
    args.push(OLLAMA_IMAGE);

    let out = run_command(bin, &args).await?;

    // Wait for the API to become ready
    wait_for_ollama_api(60).await?;

    Ok(format!(
        "Ollama container created via {}{}: {out}",
        runtime.label(),
        if has_gpu { " (GPU enabled)" } else { "" }
    ))
}

/// Pull a model inside the running Ollama container.
/// Uses `<runtime> exec` to run `ollama pull <model>`.
pub async fn docker_pull_model(model: &str) -> Result<String, String> {
    docker_pull_model_for(ContainerRuntime::Docker, model).await
}

/// Variant of [`docker_pull_model`] that targets a specific runtime.
pub async fn docker_pull_model_for(
    runtime: ContainerRuntime,
    model: &str,
) -> Result<String, String> {
    // Validate model name: alphanumeric, hyphens, underscores, colons, slashes, dots
    if model.is_empty() || !model.chars().all(|c| c.is_alphanumeric() || "-_:/.".contains(c)) {
        return Err("Invalid model name".to_string());
    }

    let output = Command::new(runtime.binary())
        .args(["exec", CONTAINER_NAME, "ollama", "pull", model])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to exec in container: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Model pull failed: {stderr}"))
    }
}

/// Full auto-setup: Docker check → start Desktop if needed → create Ollama
/// container → pull the recommended model. Returns a summary string.
pub async fn auto_setup_local_llm(model: &str) -> Result<String, String> {
    auto_setup_local_llm_with(model, RuntimePreference::default()).await
}

/// Variant of [`auto_setup_local_llm`] that respects a runtime preference.
/// When the preference is `Auto` the helper picks Docker first, then Podman
/// — matching the order documented in [`crate::container`].
pub async fn auto_setup_local_llm_with(
    model: &str,
    preference: RuntimePreference,
) -> Result<String, String> {
    let runtime = crate::container::resolve_runtime(preference).await?;
    let mut steps: Vec<String> = Vec::new();
    steps.push(format!("Container runtime: {}", runtime.label()));

    // Step 1: Check daemon (Docker only — Podman is daemon-less on Linux)
    if matches!(runtime, ContainerRuntime::Docker) {
        let docker = check_docker_status().await;
        if !docker.daemon_running {
            if docker.desktop_installed {
                start_docker_desktop().await?;
                steps.push("Docker Desktop launch initiated".to_string());

                if !wait_for_docker_ready(90).await {
                    return Err("Docker daemon did not become ready within 90 seconds. Please start Docker Desktop manually.".to_string());
                }
                steps.push("Docker daemon is now ready".to_string());
            } else {
                return Err(
                    "Docker daemon is not running and Docker Desktop was not found. Install Docker Desktop (https://www.docker.com/products/docker-desktop/) or switch to Podman in Settings."
                        .to_string(),
                );
            }
        } else {
            steps.push("Docker daemon already running".to_string());
        }
    }

    // Step 2: Ensure Ollama container
    let msg = ensure_ollama_container_for(runtime).await?;
    steps.push(msg);

    // Step 3: Validate + pull the model
    if model.is_empty() || !model.chars().all(|c| c.is_alphanumeric() || "-_:/.".contains(c)) {
        return Err("Invalid model name".to_string());
    }
    let pull_msg = docker_pull_model_for(runtime, model).await?;
    steps.push(format!("Model '{model}' pulled: {pull_msg}"));

    Ok(steps.join("\n"))
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/// Run a command silently and return Ok(()) if exit code is 0.
async fn run_silent(program: &str, args: &[&str]) -> Result<(), String> {
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

/// Run a command and capture stdout.
async fn run_command(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run {program}: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{program} failed: {stderr}"))
    }
}

/// Check if a container with the given name exists in the runtime.
async fn container_exists(bin: &str, name: &str) -> bool {
    run_command(bin, &["inspect", "--format", "{{.State.Status}}", name])
        .await
        .is_ok()
}

/// Check if a container with the given name is running.
async fn container_running(bin: &str, name: &str) -> bool {
    run_command(bin, &["inspect", "--format", "{{.State.Status}}", name])
        .await
        .map(|s| s.trim() == "running")
        .unwrap_or(false)
}

/// Try to reach the Ollama API at localhost:11434.
async fn check_ollama_api() -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    client
        .get("http://127.0.0.1:11434/api/tags")
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Wait for the Ollama API to become reachable.
async fn wait_for_ollama_api(timeout_secs: u64) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        if check_ollama_api().await {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    Err("Ollama API did not become reachable within timeout".to_string())
}

/// Detect if an NVIDIA GPU is available (checks `nvidia-smi`).
async fn detect_nvidia_gpu() -> bool {
    run_silent("nvidia-smi", &[]).await.is_ok()
}

/// Detect if Docker Desktop is installed (platform-specific file checks).
fn detect_docker_desktop_installed() -> bool {
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
        // On Linux, Docker Desktop installs to /opt/docker-desktop
        // or the user may have docker engine directly
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

    #[tokio::test]
    async fn check_docker_status_returns_struct() {
        let status = check_docker_status().await;
        // cli_found may be true or false depending on CI environment
        // Just verify the struct is well-formed
        let _ = status.cli_found;
        let _ = status.daemon_running;
        let _ = status.desktop_installed;
    }

    #[test]
    fn detect_desktop_does_not_panic() {
        // Just ensure the function runs without panic
        let _ = detect_docker_desktop_installed();
    }

    #[tokio::test]
    async fn check_ollama_container_returns_struct() {
        let status = check_ollama_container().await;
        let _ = status.exists;
        let _ = status.running;
        let _ = status.api_reachable;
    }

    #[tokio::test]
    async fn docker_pull_model_rejects_empty() {
        let result = docker_pull_model("").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid model name"));
    }

    #[tokio::test]
    async fn docker_pull_model_rejects_injection() {
        let result = docker_pull_model("model; rm -rf /").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn auto_setup_rejects_empty_model() {
        let result = auto_setup_local_llm("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn auto_setup_with_explicit_runtime_rejects_empty_model() {
        let result = auto_setup_local_llm_with("", RuntimePreference::Auto).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn docker_pull_model_for_podman_rejects_empty() {
        // Even when targeting Podman, validation must run before any exec.
        let result = docker_pull_model_for(ContainerRuntime::Podman, "").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid model name"));
    }

    #[tokio::test]
    async fn check_ollama_container_for_podman_returns_struct() {
        // Podman may or may not be installed; we only assert no panic and
        // a well-formed struct.
        let status = check_ollama_container_for(ContainerRuntime::Podman).await;
        let _ = status.exists;
        let _ = status.running;
        let _ = status.api_reachable;
    }

    #[tokio::test]
    async fn stop_docker_desktop_does_not_panic() {
        // stop_docker_desktop may fail if Docker isn't running — that's OK.
        // We only verify it doesn't panic and returns a Result.
        let result = stop_docker_desktop().await;
        let _ = result; // Ok or Err, both are fine
    }
}
