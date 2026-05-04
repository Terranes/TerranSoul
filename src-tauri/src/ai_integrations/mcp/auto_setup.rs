//! Auto-setup writers for external AI coding assistants.
//!
//! Each writer is a pure function of `(config_path, transport_url, token)`
//! so it can be unit-tested. The pattern:
//!
//! 1. Read existing config (preserve other servers/entries).
//! 2. Upsert the `terransoul-brain` entry.
//! 3. Atomically write via temp-file + rename.
//!
//! Supported clients: VS Code Copilot, Claude Desktop, Codex CLI.

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Result type for auto-setup operations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SetupResult {
    /// Whether the config was written successfully.
    pub success: bool,
    /// The path that was written (or would have been written).
    pub config_path: String,
    /// Human-readable message.
    pub message: String,
}

/// The entry name we write in every client's config.
const ENTRY_NAME: &str = "terransoul-brain";
const ENTRY_NAME_DEV: &str = "terransoul-brain-dev";

/// Returns the MCP entry name for the current build profile.
pub fn entry_name() -> &'static str {
    if super::is_dev_build() {
        ENTRY_NAME_DEV
    } else {
        ENTRY_NAME
    }
}

// ─── Path resolution ────────────────────────────────────────────────

/// VS Code per-workspace MCP config path.
/// `workspace_root` is the project root where `.vscode/` lives.
pub fn vscode_mcp_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".vscode").join("mcp.json")
}

/// Claude Desktop config path (platform-specific).
pub fn claude_desktop_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        dirs::config_dir().map(|d| d.join("Claude").join("claude_desktop_config.json"))
    }
    #[cfg(target_os = "macos")]
    {
        dirs::config_dir().map(|d| d.join("Claude").join("claude_desktop_config.json"))
    }
    #[cfg(target_os = "linux")]
    {
        dirs::config_dir().map(|d| d.join("Claude").join("claude_desktop_config.json"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

/// Codex CLI config path.
pub fn codex_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join(".codex").join("config.json"))
}

/// Cursor IDE MCP config path.
pub fn cursor_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join(".cursor").join("mcp.json"))
}

/// OpenCode config path.
pub fn opencode_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("opencode").join("config.json"))
}

// ─── Config builders (pure functions) ───────────────────────────────

/// Build the MCP server entry for VS Code / Copilot.
///
/// VS Code `mcp.json` format:
/// ```json
/// { "servers": { "terransoul-brain": { "type": "http", "url": "...", "headers": {...} } } }
/// ```
pub fn build_vscode_entry(url: &str, token: &str) -> Value {
    json!({
        "type": "http",
        "url": url,
        "headers": {
            "Authorization": format!("Bearer {token}")
        }
    })
}

/// Build the MCP server entry for Claude Desktop.
///
/// Claude Desktop format:
/// ```json
/// { "mcpServers": { "terransoul-brain": { "url": "...", "headers": {...} } } }
/// ```
pub fn build_claude_entry(url: &str, token: &str) -> Value {
    json!({
        "url": url,
        "headers": {
            "Authorization": format!("Bearer {token}")
        }
    })
}

/// Build the MCP server entry for Codex CLI.
pub fn build_codex_entry(url: &str, token: &str) -> Value {
    json!({
        "url": url,
        "token": token
    })
}

// ─── Stdio entry builders (Chunk 15.9) ──────────────────────────────

/// Build a VS Code MCP entry for the **stdio** transport.
///
/// VS Code stdio format:
/// ```json
/// { "servers": { "terransoul-brain": { "type": "stdio", "command": "...", "args": [...] } } }
/// ```
pub fn build_vscode_stdio_entry(exe_path: &str) -> Value {
    json!({
        "type": "stdio",
        "command": exe_path,
        "args": ["--mcp-stdio"]
    })
}

/// Build a Claude Desktop MCP entry for the **stdio** transport.
pub fn build_claude_stdio_entry(exe_path: &str) -> Value {
    json!({
        "command": exe_path,
        "args": ["--mcp-stdio"]
    })
}

/// Build a Codex CLI MCP entry for the **stdio** transport.
pub fn build_codex_stdio_entry(exe_path: &str) -> Value {
    json!({
        "command": exe_path,
        "args": ["--mcp-stdio"]
    })
}

// ─── Writers ────────────────────────────────────────────────────────

/// Upsert `entry` under `parent_key` → [`entry_name()`] inside the
/// JSON object at `path`, creating the file (and `parent_key`) if
/// missing. Used by every writer below to share the same upsert /
/// atomic-write logic across HTTP and stdio transports.
fn upsert_entry(path: &Path, parent_key: &str, entry: Value) -> Result<(), String> {
    let mut config = read_json_or_empty(path)?;

    if config.get(parent_key).is_none() {
        config
            .as_object_mut()
            .ok_or("config is not a JSON object")?
            .insert(parent_key.to_string(), json!({}));
    }

    config[parent_key][entry_name()] = entry;
    atomic_write_json(path, &config)
}

/// Write the VS Code `.vscode/mcp.json` file.
///
/// Merges into existing config if present, preserving other servers.
pub fn write_vscode_config(
    workspace_root: &Path,
    url: &str,
    token: &str,
) -> Result<SetupResult, String> {
    let path = vscode_mcp_path(workspace_root);
    upsert_entry(&path, "servers", build_vscode_entry(url, token))?;
    Ok(SetupResult {
        success: true,
        config_path: path.display().to_string(),
        message: "VS Code MCP config written. Restart VS Code to activate. \
             Test with: @workspace use terransoul-brain to search memories"
            .to_string(),
    })
}

/// Write the VS Code `.vscode/mcp.json` file using the **stdio**
/// transport (Chunk 15.9). `exe_path` is the absolute path to the
/// `terransoul` executable that will be spawned with `--mcp-stdio`.
pub fn write_vscode_stdio_config(
    workspace_root: &Path,
    exe_path: &str,
) -> Result<SetupResult, String> {
    let path = vscode_mcp_path(workspace_root);
    upsert_entry(&path, "servers", build_vscode_stdio_entry(exe_path))?;
    Ok(SetupResult {
        success: true,
        config_path: path.display().to_string(),
        message: "VS Code MCP config written (stdio transport). Restart VS Code to activate."
            .to_string(),
    })
}

/// Write the Claude Desktop config.
///
/// Merges into existing config, preserving other MCP servers.
pub fn write_claude_config(url: &str, token: &str) -> Result<SetupResult, String> {
    let path =
        claude_desktop_config_path().ok_or("could not determine Claude Desktop config path")?;
    upsert_entry(&path, "mcpServers", build_claude_entry(url, token))?;
    Ok(SetupResult {
        success: true,
        config_path: path.display().to_string(),
        message: "Claude Desktop config written. Restart Claude to activate.".to_string(),
    })
}

/// Write the Claude Desktop config using the **stdio** transport.
pub fn write_claude_stdio_config(exe_path: &str) -> Result<SetupResult, String> {
    let path =
        claude_desktop_config_path().ok_or("could not determine Claude Desktop config path")?;
    upsert_entry(&path, "mcpServers", build_claude_stdio_entry(exe_path))?;
    Ok(SetupResult {
        success: true,
        config_path: path.display().to_string(),
        message: "Claude Desktop config written (stdio transport). Restart Claude to activate."
            .to_string(),
    })
}

/// Write the Codex CLI config.
pub fn write_codex_config(url: &str, token: &str) -> Result<SetupResult, String> {
    let path = codex_config_path().ok_or("could not determine Codex config path")?;
    upsert_entry(&path, "mcpServers", build_codex_entry(url, token))?;
    Ok(SetupResult {
        success: true,
        config_path: path.display().to_string(),
        message: "Codex CLI config written.".to_string(),
    })
}

/// Write the Codex CLI config using the **stdio** transport.
pub fn write_codex_stdio_config(exe_path: &str) -> Result<SetupResult, String> {
    let path = codex_config_path().ok_or("could not determine Codex config path")?;
    upsert_entry(&path, "mcpServers", build_codex_stdio_entry(exe_path))?;
    Ok(SetupResult {
        success: true,
        config_path: path.display().to_string(),
        message: "Codex CLI config written (stdio transport).".to_string(),
    })
}

/// Write the Cursor IDE MCP config (HTTP transport).
///
/// Cursor uses the same format as VS Code (`servers` → entry).
pub fn write_cursor_config(url: &str, token: &str) -> Result<SetupResult, String> {
    let path = cursor_config_path().ok_or("could not determine Cursor config path")?;
    upsert_entry(&path, "servers", build_vscode_entry(url, token))?;
    Ok(SetupResult {
        success: true,
        config_path: path.display().to_string(),
        message: "Cursor MCP config written.".to_string(),
    })
}

/// Write the OpenCode config (HTTP transport).
///
/// OpenCode uses `mcpServers` format like Claude/Codex.
pub fn write_opencode_config(url: &str, token: &str) -> Result<SetupResult, String> {
    let path = opencode_config_path().ok_or("could not determine OpenCode config path")?;
    upsert_entry(&path, "mcpServers", build_claude_entry(url, token))?;
    Ok(SetupResult {
        success: true,
        config_path: path.display().to_string(),
        message: "OpenCode config written.".to_string(),
    })
}

// ─── Removers ───────────────────────────────────────────────────────

/// Run the auto-setup flow: detect all known AI editor config locations
/// and write TerranSoul MCP entries into each one found.
///
/// `workspace_root` is used for VS Code's `.vscode/mcp.json`.
/// Returns a list of results for each client attempted.
pub fn setup_all_clients(
    workspace_root: &Path,
    url: &str,
    token: &str,
) -> Vec<SetupResult> {
    let mut results = Vec::new();

    // VS Code (workspace-local)
    let vscode_dir = workspace_root.join(".vscode");
    if vscode_dir.is_dir() {
        match write_vscode_config(workspace_root, url, token) {
            Ok(r) => results.push(r),
            Err(e) => results.push(SetupResult {
                success: false,
                config_path: vscode_mcp_path(workspace_root).display().to_string(),
                message: format!("VS Code: {e}"),
            }),
        }
    }

    // Claude Desktop
    if claude_desktop_config_path().is_some() {
        match write_claude_config(url, token) {
            Ok(r) => results.push(r),
            Err(e) => results.push(SetupResult {
                success: false,
                config_path: claude_desktop_config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                message: format!("Claude Desktop: {e}"),
            }),
        }
    }

    // Codex CLI
    if let Some(codex_path) = codex_config_path() {
        let parent_exists = codex_path.parent().is_some_and(|p| p.is_dir());
        if parent_exists {
            match write_codex_config(url, token) {
                Ok(r) => results.push(r),
                Err(e) => results.push(SetupResult {
                    success: false,
                    config_path: codex_path.display().to_string(),
                    message: format!("Codex CLI: {e}"),
                }),
            }
        }
    }

    // Cursor IDE
    if let Some(cursor_path) = cursor_config_path() {
        let parent_exists = cursor_path.parent().is_some_and(|p| p.is_dir());
        if parent_exists {
            match write_cursor_config(url, token) {
                Ok(r) => results.push(r),
                Err(e) => results.push(SetupResult {
                    success: false,
                    config_path: cursor_path.display().to_string(),
                    message: format!("Cursor: {e}"),
                }),
            }
        }
    }

    // OpenCode
    if let Some(opencode_path) = opencode_config_path() {
        let parent_exists = opencode_path.parent().is_some_and(|p| p.is_dir());
        if parent_exists {
            match write_opencode_config(url, token) {
                Ok(r) => results.push(r),
                Err(e) => results.push(SetupResult {
                    success: false,
                    config_path: opencode_path.display().to_string(),
                    message: format!("OpenCode: {e}"),
                }),
            }
        }
    }

    results
}

/// Remove the terransoul-brain entry from VS Code config.
pub fn remove_vscode_config(workspace_root: &Path) -> Result<SetupResult, String> {
    let path = vscode_mcp_path(workspace_root);
    remove_entry_from_json(&path, &["servers", entry_name()])
}

/// Remove the terransoul-brain entry from Claude Desktop config.
pub fn remove_claude_config() -> Result<SetupResult, String> {
    let path =
        claude_desktop_config_path().ok_or("could not determine Claude Desktop config path")?;
    remove_entry_from_json(&path, &["mcpServers", entry_name()])
}

/// Remove the terransoul-brain entry from Codex CLI config.
pub fn remove_codex_config() -> Result<SetupResult, String> {
    let path = codex_config_path().ok_or("could not determine Codex config path")?;
    remove_entry_from_json(&path, &["mcpServers", entry_name()])
}

// ─── Helpers ────────────────────────────────────────────────────────

/// Read a JSON file, returning an empty object `{}` if the file doesn't exist.
fn read_json_or_empty(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let raw =
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    // Strip JSON comments (VS Code supports JSONC)
    let stripped = strip_json_comments(&raw);
    serde_json::from_str(&stripped).map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

/// Write JSON to a file atomically (temp file + rename).
fn atomic_write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directory {}: {e}", parent.display()))?;
    }

    let pretty = serde_json::to_string_pretty(value)
        .map_err(|e| format!("failed to serialize JSON: {e}"))?;

    let tmp = path.with_extension("tmp");
    fs::write(&tmp, pretty.as_bytes())
        .map_err(|e| format!("failed to write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        format!(
            "failed to rename {} → {}: {e}",
            tmp.display(),
            path.display()
        )
    })?;

    Ok(())
}

/// Remove a nested key from a JSON file.
fn remove_entry_from_json(path: &Path, keys: &[&str]) -> Result<SetupResult, String> {
    if !path.exists() {
        return Ok(SetupResult {
            success: true,
            config_path: path.display().to_string(),
            message: "Config file does not exist — nothing to remove.".to_string(),
        });
    }

    let mut config = read_json_or_empty(path)?;

    // Navigate to parent, then remove the last key
    if keys.len() >= 2 {
        let parent_keys = &keys[..keys.len() - 1];
        let target_key = keys[keys.len() - 1];

        let mut current = &mut config;
        for &k in parent_keys {
            match current.get_mut(k) {
                Some(v) => current = v,
                None => {
                    return Ok(SetupResult {
                        success: true,
                        config_path: path.display().to_string(),
                        message: format!("Entry '{target_key}' not found — nothing to remove."),
                    });
                }
            }
        }

        if let Some(obj) = current.as_object_mut() {
            obj.remove(target_key);
        }
    }

    atomic_write_json(path, &config)?;

    Ok(SetupResult {
        success: true,
        config_path: path.display().to_string(),
        message: format!("Removed '{}' from {}", entry_name(), path.display()),
    })
}

/// Strip single-line (`//`) and multi-line (`/* */`) comments from JSON.
/// This handles the JSONC format used by VS Code config files.
fn strip_json_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escape_next = false;

    while let Some(ch) = chars.next() {
        if escape_next {
            result.push(ch);
            escape_next = false;
            continue;
        }

        if in_string {
            result.push(ch);
            if ch == '\\' {
                escape_next = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                result.push(ch);
            }
            '/' => {
                if chars.peek() == Some(&'/') {
                    // Single-line comment — skip until newline
                    chars.next(); // consume second /
                    for c in chars.by_ref() {
                        if c == '\n' {
                            result.push('\n');
                            break;
                        }
                    }
                } else if chars.peek() == Some(&'*') {
                    // Multi-line comment — skip until */
                    chars.next(); // consume *
                    let mut prev = ' ';
                    for c in chars.by_ref() {
                        if prev == '*' && c == '/' {
                            break;
                        }
                        // Preserve newlines for line-count stability
                        if c == '\n' {
                            result.push('\n');
                        }
                        prev = c;
                    }
                } else {
                    result.push(ch);
                }
            }
            _ => result.push(ch),
        }
    }

    result
}

// ─── List installed clients ────────────────────────────────────────

/// Describes which clients are configured and where.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClientStatus {
    pub client: String,
    pub configured: bool,
    pub config_path: Option<String>,
}

/// Check which clients have terransoul-brain configured.
pub fn list_client_status(workspace_root: &Path) -> Vec<ClientStatus> {
    let mut results = Vec::new();

    // VS Code
    let vscode_path = vscode_mcp_path(workspace_root);
    let vscode_configured = check_entry_exists(&vscode_path, &["servers", entry_name()]);
    results.push(ClientStatus {
        client: "VS Code / Copilot".to_string(),
        configured: vscode_configured,
        config_path: Some(vscode_path.display().to_string()),
    });

    // Claude Desktop
    if let Some(claude_path) = claude_desktop_config_path() {
        let configured = check_entry_exists(&claude_path, &["mcpServers", entry_name()]);
        results.push(ClientStatus {
            client: "Claude Desktop".to_string(),
            configured,
            config_path: Some(claude_path.display().to_string()),
        });
    }

    // Codex CLI
    if let Some(codex_path) = codex_config_path() {
        let configured = check_entry_exists(&codex_path, &["mcpServers", entry_name()]);
        results.push(ClientStatus {
            client: "Codex CLI".to_string(),
            configured,
            config_path: Some(codex_path.display().to_string()),
        });
    }

    // Cursor IDE
    if let Some(cursor_path) = cursor_config_path() {
        let configured = check_entry_exists(&cursor_path, &["servers", entry_name()]);
        results.push(ClientStatus {
            client: "Cursor".to_string(),
            configured,
            config_path: Some(cursor_path.display().to_string()),
        });
    }

    // OpenCode
    if let Some(opencode_path) = opencode_config_path() {
        let configured = check_entry_exists(&opencode_path, &["mcpServers", entry_name()]);
        results.push(ClientStatus {
            client: "OpenCode".to_string(),
            configured,
            config_path: Some(opencode_path.display().to_string()),
        });
    }

    results
}

fn check_entry_exists(path: &Path, keys: &[&str]) -> bool {
    let config = match read_json_or_empty(path) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let mut current = &config;
    for &k in keys {
        match current.get(k) {
            Some(v) => current = v,
            None => return false,
        }
    }
    true
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const TEST_URL: &str = "http://127.0.0.1:7421/mcp";
    const TEST_TOKEN: &str = "abc123def456";

    #[test]
    fn vscode_entry_has_correct_structure() {
        let entry = build_vscode_entry(TEST_URL, TEST_TOKEN);
        assert_eq!(entry["type"], "http");
        assert_eq!(entry["url"], TEST_URL);
        assert!(entry["headers"]["Authorization"]
            .as_str()
            .unwrap()
            .starts_with("Bearer "));
    }

    #[test]
    fn claude_entry_has_correct_structure() {
        let entry = build_claude_entry(TEST_URL, TEST_TOKEN);
        assert_eq!(entry["url"], TEST_URL);
        assert!(entry["headers"]["Authorization"]
            .as_str()
            .unwrap()
            .contains(TEST_TOKEN));
    }

    #[test]
    fn codex_entry_has_correct_structure() {
        let entry = build_codex_entry(TEST_URL, TEST_TOKEN);
        assert_eq!(entry["url"], TEST_URL);
        assert_eq!(entry["token"], TEST_TOKEN);
    }

    // ─── Stdio entry builder + writer tests (Chunk 15.9) ───────────

    const TEST_EXE: &str = "/usr/local/bin/terransoul";

    #[test]
    fn vscode_stdio_entry_has_correct_structure() {
        let entry = build_vscode_stdio_entry(TEST_EXE);
        assert_eq!(entry["type"], "stdio");
        assert_eq!(entry["command"], TEST_EXE);
        assert_eq!(entry["args"], json!(["--mcp-stdio"]));
        // No URL/token leaked into stdio entry.
        assert!(entry.get("url").is_none());
        assert!(entry.get("headers").is_none());
    }

    #[test]
    fn claude_stdio_entry_has_correct_structure() {
        let entry = build_claude_stdio_entry(TEST_EXE);
        assert_eq!(entry["command"], TEST_EXE);
        assert_eq!(entry["args"], json!(["--mcp-stdio"]));
    }

    #[test]
    fn codex_stdio_entry_has_correct_structure() {
        let entry = build_codex_stdio_entry(TEST_EXE);
        assert_eq!(entry["command"], TEST_EXE);
        assert_eq!(entry["args"], json!(["--mcp-stdio"]));
    }

    #[test]
    fn write_vscode_stdio_creates_new_config() {
        let tmp = TempDir::new().unwrap();
        let result = write_vscode_stdio_config(tmp.path(), TEST_EXE).unwrap();
        assert!(result.success);

        let config: Value =
            serde_json::from_str(&fs::read_to_string(vscode_mcp_path(tmp.path())).unwrap())
                .unwrap();
        assert_eq!(config["servers"][entry_name()]["type"], "stdio");
        assert_eq!(config["servers"][entry_name()]["command"], TEST_EXE);
    }

    #[test]
    fn stdio_writer_overwrites_previous_http_entry() {
        let tmp = TempDir::new().unwrap();
        // First wire up the HTTP transport, then switch to stdio —
        // simulating a user toggling the Control Panel's transport
        // picker. The terransoul-brain entry should now be the stdio
        // form, with no leftover URL field.
        write_vscode_config(tmp.path(), TEST_URL, TEST_TOKEN).unwrap();
        write_vscode_stdio_config(tmp.path(), TEST_EXE).unwrap();

        let config: Value =
            serde_json::from_str(&fs::read_to_string(vscode_mcp_path(tmp.path())).unwrap())
                .unwrap();
        let entry = &config["servers"][entry_name()];
        assert_eq!(entry["type"], "stdio");
        assert_eq!(entry["command"], TEST_EXE);
        assert!(entry.get("url").is_none(), "stale http url leaked: {entry}");
        assert!(
            entry.get("headers").is_none(),
            "stale headers leaked: {entry}"
        );
    }

    #[test]
    fn write_vscode_creates_new_config() {
        let tmp = TempDir::new().unwrap();
        let result = write_vscode_config(tmp.path(), TEST_URL, TEST_TOKEN).unwrap();
        assert!(result.success);

        let config: Value =
            serde_json::from_str(&fs::read_to_string(vscode_mcp_path(tmp.path())).unwrap())
                .unwrap();
        assert_eq!(config["servers"][entry_name()]["type"], "http");
        assert_eq!(config["servers"][entry_name()]["url"], TEST_URL);
    }

    #[test]
    fn write_vscode_preserves_existing_servers() {
        let tmp = TempDir::new().unwrap();
        let vscode_dir = tmp.path().join(".vscode");
        fs::create_dir_all(&vscode_dir).unwrap();

        // Write existing config with another server
        let existing = json!({
            "servers": {
                "other-server": { "type": "stdio", "command": "some-cmd" }
            }
        });
        fs::write(
            vscode_dir.join("mcp.json"),
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .unwrap();

        write_vscode_config(tmp.path(), TEST_URL, TEST_TOKEN).unwrap();

        let config: Value =
            serde_json::from_str(&fs::read_to_string(vscode_mcp_path(tmp.path())).unwrap())
                .unwrap();
        // Both servers should exist
        assert!(config["servers"]["other-server"].is_object());
        assert!(config["servers"][entry_name()].is_object());
    }

    #[test]
    fn write_vscode_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        write_vscode_config(tmp.path(), TEST_URL, TEST_TOKEN).unwrap();
        write_vscode_config(tmp.path(), TEST_URL, "new-token").unwrap();

        let config: Value =
            serde_json::from_str(&fs::read_to_string(vscode_mcp_path(tmp.path())).unwrap())
                .unwrap();
        // Should have new token, only one entry
        assert!(config["servers"][entry_name()]["headers"]["Authorization"]
            .as_str()
            .unwrap()
            .contains("new-token"));
        let servers = config["servers"].as_object().unwrap();
        assert_eq!(servers.len(), 1);
    }

    #[test]
    fn remove_vscode_config_removes_entry() {
        let tmp = TempDir::new().unwrap();
        write_vscode_config(tmp.path(), TEST_URL, TEST_TOKEN).unwrap();
        remove_vscode_config(tmp.path()).unwrap();

        let config: Value =
            serde_json::from_str(&fs::read_to_string(vscode_mcp_path(tmp.path())).unwrap())
                .unwrap();
        assert!(!config["servers"]
            .as_object()
            .unwrap()
            .contains_key(entry_name()));
    }

    #[test]
    fn remove_nonexistent_is_ok() {
        let tmp = TempDir::new().unwrap();
        let result = remove_vscode_config(tmp.path()).unwrap();
        assert!(result.success);
    }

    #[test]
    fn strip_comments_handles_jsonc() {
        let input = r#"{
  // This is a comment
  "servers": {
    /* block comment */
    "test": { "type": "http" }
  }
}"#;
        let stripped = strip_json_comments(input);
        let parsed: Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(parsed["servers"]["test"]["type"], "http");
    }

    #[test]
    fn strip_comments_preserves_urls() {
        // Ensure // inside strings is not stripped
        let input = r#"{ "url": "http://localhost:7421/mcp" }"#;
        let stripped = strip_json_comments(input);
        let parsed: Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(parsed["url"], "http://localhost:7421/mcp");
    }

    #[test]
    fn list_client_status_detects_configured() {
        let tmp = TempDir::new().unwrap();
        write_vscode_config(tmp.path(), TEST_URL, TEST_TOKEN).unwrap();

        let statuses = list_client_status(tmp.path());
        let vscode = statuses
            .iter()
            .find(|s| s.client.contains("VS Code"))
            .unwrap();
        assert!(vscode.configured);
    }

    #[test]
    fn list_client_status_detects_unconfigured() {
        let tmp = TempDir::new().unwrap();
        let statuses = list_client_status(tmp.path());
        let vscode = statuses
            .iter()
            .find(|s| s.client.contains("VS Code"))
            .unwrap();
        assert!(!vscode.configured);
    }

    #[test]
    fn write_claude_creates_config() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("claude_desktop_config.json");

        // Use the write helper directly with a known path
        let mut config = json!({});
        config
            .as_object_mut()
            .unwrap()
            .insert("mcpServers".to_string(), json!({}));
        config["mcpServers"][entry_name()] = build_claude_entry(TEST_URL, TEST_TOKEN);
        atomic_write_json(&path, &config).unwrap();

        let read_back: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(read_back["mcpServers"][entry_name()]["url"], TEST_URL);
    }

    #[test]
    fn atomic_write_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let deep_path = tmp.path().join("a").join("b").join("c.json");
        atomic_write_json(&deep_path, &json!({"test": true})).unwrap();
        assert!(deep_path.exists());
    }
}
