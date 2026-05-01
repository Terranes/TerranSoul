//! Ollama installation + lifecycle management.
//!
//! Provides commands to:
//! - Detect whether Ollama is installed (binary on disk)
//! - Start the Ollama service if installed but not running
//! - Download + install Ollama from the official site
//!
//! Used by the FirstLaunchWizard to make local-LLM setup zero-click on
//! systems where Ollama is not yet installed.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use super::ollama_agent::OLLAMA_BASE_URL;

/// Status of the Ollama installation on this machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaInstallStatus {
    /// `true` if the `ollama` binary was found on PATH or in a known location.
    pub installed: bool,
    /// `true` if the HTTP API at `localhost:11434/api/tags` responds.
    pub running: bool,
    /// Path to the binary if found (for diagnostics / logging).
    pub binary_path: Option<String>,
}

/// Find the Ollama binary on this machine.
///
/// Checks (in order):
/// 1. `ollama` on PATH (via `where`/`which`)
/// 2. Common install locations (Windows / macOS / Linux)
fn find_ollama_binary() -> Option<PathBuf> {
    // Try PATH lookup first.
    #[cfg(target_os = "windows")]
    let path_cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let path_cmd = "which";

    if let Ok(output) = std::process::Command::new(path_cmd).arg("ollama").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first_line) = stdout.lines().next() {
                let p = PathBuf::from(first_line.trim());
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }

    // Platform-specific common install locations.
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local)
                .join("Programs")
                .join("Ollama")
                .join("ollama.exe");
            if p.exists() {
                return Some(p);
            }
        }
        if let Ok(prog) = std::env::var("ProgramFiles") {
            let p = PathBuf::from(prog).join("Ollama").join("ollama.exe");
            if p.exists() {
                return Some(p);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        for candidate in [
            "/Applications/Ollama.app/Contents/Resources/ollama",
            "/usr/local/bin/ollama",
        ] {
            let p = PathBuf::from(candidate);
            if p.exists() {
                return Some(p);
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        for candidate in ["/usr/local/bin/ollama", "/usr/bin/ollama"] {
            let p = PathBuf::from(candidate);
            if p.exists() {
                return Some(p);
            }
        }
    }

    None
}

/// Check if the Ollama HTTP API responds.
async fn is_ollama_responding(client: &reqwest::Client) -> bool {
    let url = format!("{OLLAMA_BASE_URL}/api/tags");
    matches!(
        tokio::time::timeout(Duration::from_secs(2), client.get(&url).send()).await,
        Ok(Ok(resp)) if resp.status().is_success()
    )
}

/// Detect whether Ollama is installed and/or running.
pub async fn detect_ollama() -> OllamaInstallStatus {
    let binary = find_ollama_binary();
    let installed = binary.is_some();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    let running = is_ollama_responding(&client).await;
    OllamaInstallStatus {
        installed,
        running,
        binary_path: binary.map(|p| p.to_string_lossy().into_owned()),
    }
}

/// Attempt to start the Ollama service if it's installed but not running.
///
/// Spawns `ollama serve` in the background (detached from the parent).
/// Polls the HTTP API for up to `timeout_secs` to confirm startup.
///
/// Returns `Ok(true)` if Ollama is running by the end of the timeout,
/// `Ok(false)` if we couldn't start it (no binary), or `Err` on a real error.
pub async fn start_ollama(timeout_secs: u64) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| format!("HTTP client: {e}"))?;

    // Already running? Done.
    if is_ollama_responding(&client).await {
        return Ok(true);
    }

    let binary = find_ollama_binary().ok_or_else(|| "Ollama binary not found".to_string())?;

    // Spawn `ollama serve` detached. We don't keep the handle — the user can
    // stop it via Task Manager / `ollama` CLI.
    let mut cmd = Command::new(&binary);
    cmd.arg("serve");

    #[cfg(target_os = "windows")]
    {
        // CREATE_NO_WINDOW (0x08000000) — don't pop a console window.
        // DETACHED_PROCESS (0x00000008) — don't tie the child to our console.
        cmd.creation_flags(0x0800_0008);
    }

    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn ollama serve: {e}"))?;

    // Detach: don't wait. The child runs independently.
    let _ = child.id();

    // Poll for startup.
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < timeout_secs {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if is_ollama_responding(&client).await {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Download + install Ollama from the official site.
///
/// Platform behavior:
/// - **Windows**: Downloads `OllamaSetup.exe` and runs it silently (`/SILENT`).
/// - **macOS**: Returns an error directing the user to download the .dmg.
/// - **Linux**: Runs the official `curl | sh` installer.
///
/// Emits progress events on `app` channel `ollama-install-progress` with
/// JSON payload `{ "phase": String, "percent": u32 }`.
pub async fn install_ollama<F>(progress: F) -> Result<String, String>
where
    F: Fn(&str, u32) + Send + Sync,
{
    progress("Starting Ollama installer download...", 0);

    #[cfg(target_os = "windows")]
    {
        let temp_dir = std::env::temp_dir();
        let installer_path = temp_dir.join("OllamaSetup.exe");

        progress("Downloading Ollama installer...", 5);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| format!("HTTP client: {e}"))?;

        let url = "https://ollama.com/download/OllamaSetup.exe";
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

        let mut file = tokio::fs::File::create(&installer_path)
            .await
            .map_err(|e| format!("Failed to create installer file: {e}"))?;

        use tokio::io::AsyncWriteExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download stream error: {e}"))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Write error: {e}"))?;
            downloaded += chunk.len() as u64;
            if let Some(pct_raw) = (downloaded * 60).checked_div(total) {
                let pct = pct_raw as u32 + 5; // 5-65%
                progress("Downloading Ollama installer...", pct.min(65));
            }
        }
        file.flush().await.ok();
        drop(file);

        progress("Running Ollama installer (this may take a minute)...", 70);

        // Run installer silently. Inno Setup uses /VERYSILENT for no UI.
        let mut cmd = Command::new(&installer_path);
        cmd.arg("/VERYSILENT")
            .arg("/SUPPRESSMSGBOXES")
            .arg("/NORESTART");
        let output = cmd
            .output()
            .await
            .map_err(|e| format!("Installer exec failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Installer exited with error: {stderr}"));
        }

        progress("Installation complete — verifying...", 90);

        // Verify the binary now exists.
        if find_ollama_binary().is_none() {
            return Err(
                "Installation completed but Ollama binary not found on PATH or known locations"
                    .to_string(),
            );
        }

        progress("Ollama installed successfully", 100);
        Ok("Ollama installed via OllamaSetup.exe".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        let _ = progress;
        Err("Automatic install on macOS is not supported. Please download Ollama from https://ollama.com/download".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        progress("Running Ollama installer (curl | sh)...", 10);
        let output = Command::new("sh")
            .arg("-c")
            .arg("curl -fsSL https://ollama.com/install.sh | sh")
            .output()
            .await
            .map_err(|e| format!("Installer exec failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Installer failed: {stderr}"));
        }

        progress("Ollama installed successfully", 100);
        Ok("Ollama installed via official Linux installer".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_binary_returns_option() {
        // Just verify it doesn't panic.
        let _ = find_ollama_binary();
    }

    #[tokio::test]
    async fn detect_returns_consistent_status() {
        let status = detect_ollama().await;
        // installed == binary_path.is_some() always
        assert_eq!(status.installed, status.binary_path.is_some());
    }
}
