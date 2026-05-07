//! Tauri commands for loading extensible prompt commands from
//! `.terransoul/prompts/` directories.
//!
//! Scans multiple directories for `.md` files and returns their names
//! and content so the frontend can register them as slash commands.
//!
//! ## Mode gating
//!
//! Commands can specify a `mode` in YAML frontmatter to restrict when
//! they appear in the command picker:
//!
//! ```yaml
//! ---
//! mode: coding
//! ---
//! ```
//!
//! Supported modes:
//! - `all` (default) — always visible
//! - `coding` — only visible when coding workflow / self-improve is active
//! - `companion` — only visible when in normal companion chat mode

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::State;

use crate::AppState;

/// Mode restriction for a prompt command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PromptMode {
    /// Always available (default when no frontmatter is specified).
    #[default]
    All,
    /// Only available when coding workflow / self-improve is active.
    Coding,
    /// Only available in normal companion chat mode.
    Companion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptFile {
    pub name: String,
    pub content: String,
    pub path: String,
    /// Mode restriction parsed from YAML frontmatter.
    pub mode: PromptMode,
}

/// Parse YAML frontmatter from a prompt file's content.
///
/// Expects the standard `---` delimited block at the top of the file.
/// Returns the extracted `PromptMode` and the content body (after the
/// closing `---`).
fn parse_frontmatter(content: &str) -> (PromptMode, String) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (PromptMode::All, content.to_string());
    }

    // Find the closing `---` (skip the first line).
    let after_open = &trimmed[3..];
    let rest = after_open.trim_start_matches(['\r', '\n']);
    if let Some(close_idx) = rest.find("\n---") {
        let frontmatter_block = &rest[..close_idx];
        let body_start = close_idx + 4; // "\n---"
        let body = rest[body_start..].trim_start_matches(['\r', '\n']);

        // Parse the `mode:` field from frontmatter.
        let mode = parse_mode_from_frontmatter(frontmatter_block);
        (mode, body.to_string())
    } else {
        // No closing delimiter — treat entire content as body.
        (PromptMode::All, content.to_string())
    }
}

/// Extract the `mode` value from raw frontmatter text.
fn parse_mode_from_frontmatter(frontmatter: &str) -> PromptMode {
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("mode:") {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            return match value.to_lowercase().as_str() {
                "coding" => PromptMode::Coding,
                "companion" => PromptMode::Companion,
                _ => PromptMode::All,
            };
        }
    }
    PromptMode::All
}

/// Scan a directory for `.md` files and collect them as prompt commands.
fn scan_prompt_dir(dir: &Path) -> Vec<PromptFile> {
    let mut results = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if ext != "md" {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if name.is_empty() {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            let (mode, body) = parse_frontmatter(&content);
            results.push(PromptFile {
                name,
                content: body,
                path: path.display().to_string(),
                mode,
            });
        }
    }
    results
}

/// List all available prompt commands by scanning known directories.
///
/// Directories scanned (in order, later entries override earlier):
/// 1. `<data_dir>/prompts/` — user's personal prompts
/// 2. `.terransoul/prompts/` — workspace-local prompts
/// 3. `.github/prompts/` — Copilot-style prompts (reused)
///
/// Each `.md` file becomes a slash command named after the file stem.
#[tauri::command]
pub async fn list_prompt_commands(state: State<'_, AppState>) -> Result<Vec<PromptFile>, String> {
    let mut commands = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    // Directories to scan (last wins for duplicate names).
    let dirs = prompt_dirs(&state.data_dir);

    for dir in &dirs {
        for file in scan_prompt_dir(dir) {
            seen_names.insert(file.name.clone());
            // Remove older entry with same name (last dir wins).
            commands.retain(|f: &PromptFile| f.name != file.name);
            commands.push(file);
        }
    }

    // Also include .terransoul/prompts/ from the working directory
    // (covers the case where data_dir differs from cwd).
    let cwd_prompts = std::env::current_dir()
        .unwrap_or_default()
        .join(".terransoul")
        .join("prompts");
    if cwd_prompts.exists() {
        for file in scan_prompt_dir(&cwd_prompts) {
            if !seen_names.contains(&file.name) {
                seen_names.insert(file.name.clone());
                commands.push(file);
            }
        }
    }

    commands.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(commands)
}

/// Save (create or update) a prompt command file.
///
/// Writes to `<data_dir>/prompts/<name>.md`. The name must be a valid
/// filename slug (alphanumeric, hyphens, underscores).
#[tauri::command]
pub async fn save_prompt_command(
    state: State<'_, AppState>,
    name: String,
    content: String,
) -> Result<PromptFile, String> {
    validate_prompt_name(&name)?;
    let dir = state.data_dir.join("prompts");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create prompts dir: {e}"))?;
    let path = dir.join(format!("{name}.md"));
    std::fs::write(&path, &content)
        .map_err(|e| format!("Failed to write prompt file: {e}"))?;
    let (mode, body) = parse_frontmatter(&content);
    Ok(PromptFile {
        name,
        content: body,
        path: path.display().to_string(),
        mode,
    })
}

/// Delete a prompt command file by name.
///
/// Only deletes from `<data_dir>/prompts/`. Workspace-local prompts
/// (`.terransoul/prompts/`) are not deleted via this command.
#[tauri::command]
pub async fn delete_prompt_command(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    validate_prompt_name(&name)?;
    let path = state.data_dir.join("prompts").join(format!("{name}.md"));
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete prompt file: {e}"))?;
    }
    Ok(())
}

/// Validate that a prompt command name is a safe filename slug.
fn validate_prompt_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Prompt command name cannot be empty".to_string());
    }
    if name.len() > 64 {
        return Err("Prompt command name too long (max 64 chars)".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            "Prompt command name must contain only alphanumeric characters, hyphens, or underscores"
                .to_string(),
        );
    }
    Ok(())
}

/// Resolve the set of directories to scan for prompt commands.
fn prompt_dirs(data_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. User's data dir prompts
    let user_prompts = data_dir.join("prompts");
    if user_prompts.exists() {
        dirs.push(user_prompts);
    }

    // 2. Workspace .github/prompts (Copilot convention, reused)
    let cwd = std::env::current_dir().unwrap_or_default();
    let github_prompts = cwd.join(".github").join("prompts");
    if github_prompts.exists() {
        dirs.push(github_prompts);
    }

    // 3. Workspace .terransoul/prompts (TerranSoul convention)
    let ts_prompts = cwd.join(".terransoul").join("prompts");
    if ts_prompts.exists() {
        dirs.push(ts_prompts);
    }

    dirs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_prompt_dir_finds_md_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        fs::write(dir.join("hello-world.md"), "# Hello\nThis is a test prompt.").unwrap();
        fs::write(dir.join("setup.md"), "Setup instructions here.").unwrap();
        fs::write(dir.join("ignore.txt"), "Not a prompt.").unwrap();

        let results = scan_prompt_dir(dir);
        assert_eq!(results.len(), 2);

        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"hello-world"));
        assert!(names.contains(&"setup"));
    }

    #[test]
    fn scan_prompt_dir_empty_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let results = scan_prompt_dir(tmp.path());
        assert!(results.is_empty());
    }

    #[test]
    fn scan_prompt_dir_missing_dir_returns_empty() {
        let results = scan_prompt_dir(Path::new("/nonexistent/dir/that/does/not/exist"));
        assert!(results.is_empty());
    }

    #[test]
    fn validate_prompt_name_accepts_valid() {
        assert!(validate_prompt_name("hello-world").is_ok());
        assert!(validate_prompt_name("setup_prereqs").is_ok());
        assert!(validate_prompt_name("Review123").is_ok());
    }

    #[test]
    fn validate_prompt_name_rejects_invalid() {
        assert!(validate_prompt_name("").is_err());
        assert!(validate_prompt_name("has spaces").is_err());
        assert!(validate_prompt_name("path/traversal").is_err());
        assert!(validate_prompt_name("../escape").is_err());
        assert!(validate_prompt_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn parse_frontmatter_extracts_coding_mode() {
        let content = "---\nmode: coding\n---\n# Setup\nDo setup things.";
        let (mode, body) = parse_frontmatter(content);
        assert_eq!(mode, PromptMode::Coding);
        assert!(body.starts_with("# Setup"));
    }

    #[test]
    fn parse_frontmatter_extracts_companion_mode() {
        let content = "---\nmode: companion\n---\nHello world.";
        let (mode, body) = parse_frontmatter(content);
        assert_eq!(mode, PromptMode::Companion);
        assert_eq!(body, "Hello world.");
    }

    #[test]
    fn parse_frontmatter_defaults_to_all_when_missing() {
        let content = "# No frontmatter\nJust a prompt.";
        let (mode, body) = parse_frontmatter(content);
        assert_eq!(mode, PromptMode::All);
        assert_eq!(body, content);
    }

    #[test]
    fn parse_frontmatter_defaults_to_all_when_no_mode_field() {
        let content = "---\ntitle: My Prompt\n---\nBody here.";
        let (mode, body) = parse_frontmatter(content);
        assert_eq!(mode, PromptMode::All);
        assert_eq!(body, "Body here.");
    }

    #[test]
    fn scan_prompt_dir_parses_mode_from_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        fs::write(
            dir.join("coding-cmd.md"),
            "---\nmode: coding\n---\n# Coding\nDo coding.",
        )
        .unwrap();
        fs::write(dir.join("general.md"), "# General\nAlways available.").unwrap();

        let results = scan_prompt_dir(dir);
        assert_eq!(results.len(), 2);

        let coding = results.iter().find(|f| f.name == "coding-cmd").unwrap();
        assert_eq!(coding.mode, PromptMode::Coding);
        assert!(coding.content.starts_with("# Coding"));

        let general = results.iter().find(|f| f.name == "general").unwrap();
        assert_eq!(general.mode, PromptMode::All);
    }
}
