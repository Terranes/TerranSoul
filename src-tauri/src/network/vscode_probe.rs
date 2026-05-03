//! VS Code / Copilot session probe — FS wrapper (Chunk 24.5b).
//!
//! Wraps the pure parser from `network::vscode_log` (24.5a) with filesystem
//! I/O to locate and read the latest Copilot Chat log file on the current OS.
//!
//! ## Path resolution per OS
//!
//! - **Windows:** `%APPDATA%/Code/User`
//! - **macOS:** `~/Library/Application Support/Code/User`
//! - **Linux:** `~/.config/Code/User`
//!
//! Within that directory, the log lives at:
//! `logs/<latest-date>/window<N>/exthost/GitHub.copilot-chat/Copilot-Chat.log`
//!
//! We pick the most-recently modified file when multiple windows exist.

use std::path::{Path, PathBuf};

use super::vscode_log::{self, CopilotLogSummary};

/// Resolve the VS Code user-data directory for the current OS.
///
/// Returns `None` if the expected environment variable or home directory
/// is not set (e.g. headless CI environments).
pub fn vscode_user_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|appdata| PathBuf::from(appdata).join("Code").join("User"))
    }
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| {
            h.join("Library")
                .join("Application Support")
                .join("Code")
                .join("User")
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        dirs::home_dir().map(|h| h.join(".config").join("Code").join("User"))
    }
}

/// Find the most-recently modified Copilot Chat log file within the
/// VS Code `logs/` directory.
///
/// The structure is `logs/<date>/window<N>/exthost/GitHub.copilot-chat/Copilot-Chat.log`.
/// We walk date folders in reverse (newest first) and window folders
/// by modification time.
pub fn find_latest_copilot_log(user_data: &Path) -> Option<PathBuf> {
    let logs_dir = user_data.join("logs");
    if !logs_dir.is_dir() {
        return None;
    }

    // Collect date folders sorted reverse-alphabetically (newest date first).
    let mut date_dirs: Vec<PathBuf> = std::fs::read_dir(&logs_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();
    date_dirs.sort_unstable_by(|a, b| b.cmp(a));

    for date_dir in date_dirs {
        // Within each date folder, find window<N> sub-dirs.
        let mut candidates: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&date_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let window_dir = entry.path();
                if !window_dir.is_dir() {
                    continue;
                }
                let log_path = window_dir
                    .join("exthost")
                    .join("GitHub.copilot-chat")
                    .join("Copilot-Chat.log");
                if log_path.is_file() {
                    if let Ok(meta) = std::fs::metadata(&log_path) {
                        if let Ok(modified) = meta.modified() {
                            candidates.push((log_path, modified));
                        }
                    }
                }
            }
        }

        // Pick the most-recently modified log in this date folder.
        if !candidates.is_empty() {
            candidates.sort_unstable_by_key(|c| std::cmp::Reverse(c.1));
            return Some(candidates.into_iter().next().unwrap().0);
        }
    }

    None
}

/// Read and summarise the latest Copilot Chat log.
///
/// Returns `None` if no log file could be located (VS Code not installed,
/// Copilot extension not present, or no sessions yet).
pub async fn probe_copilot_session() -> Option<CopilotLogSummary> {
    let user_data = vscode_user_data_dir()?;
    let log_path = find_latest_copilot_log(&user_data)?;
    let contents = tokio::fs::read_to_string(&log_path).await.ok()?;
    Some(vscode_log::summarise_log(&contents))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vscode_user_data_dir_returns_some_on_desktop() {
        // On a normal desktop dev machine this should resolve.
        // In CI without $HOME or %APPDATA% it may be None — that's fine.
        let dir = vscode_user_data_dir();
        if let Some(ref d) = dir {
            // Should end in "User" component.
            assert_eq!(d.file_name().and_then(|f| f.to_str()), Some("User"));
        }
    }

    #[test]
    fn find_latest_copilot_log_returns_none_for_nonexistent() {
        let fake = PathBuf::from("/nonexistent/path/that/does/not/exist");
        assert_eq!(find_latest_copilot_log(&fake), None);
    }

    #[tokio::test]
    async fn probe_copilot_session_returns_option() {
        // Just exercises the happy path or None depending on environment.
        let result = probe_copilot_session().await;
        // On dev machines with VS Code + Copilot, this should be Some.
        // On CI, it's None. Either way — no panic.
        let _ = result;
    }
}
