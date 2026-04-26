//! MCP bearer-token management.
//!
//! The token lives in `<data_dir>/mcp-token.txt`. On Unix, the file is
//! created with `0600` permissions so only the owning user can read it.
//! The token is auto-generated on first use and can be regenerated via
//! the `mcp_regenerate_token` Tauri command.

use std::fs;
use std::path::{Path, PathBuf};

/// Path to the MCP bearer token file.
pub fn token_path(data_dir: &Path) -> PathBuf {
    data_dir.join("mcp-token.txt")
}

/// Load the existing token or create a new one.
pub fn load_or_create(data_dir: &Path) -> Result<String, String> {
    let path = token_path(data_dir);
    if path.exists() {
        fs::read_to_string(&path)
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("failed to read MCP token: {e}"))
    } else {
        write_new(data_dir)
    }
}

/// Regenerate the token (overwrite if exists).
pub fn regenerate(data_dir: &Path) -> Result<String, String> {
    write_new(data_dir)
}

fn write_new(data_dir: &Path) -> Result<String, String> {
    let path = token_path(data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create MCP token directory: {e}"))?;
    }
    let token = generate();
    fs::write(&path, &token).map_err(|e| format!("failed to write MCP token: {e}"))?;
    set_restrictive_permissions(&path)?;
    Ok(token)
}

fn generate() -> String {
    use sha2::{Digest, Sha256};
    let id = uuid::Uuid::new_v4();
    let hash = Sha256::digest(id.as_bytes());
    hex::encode(hash)
}

#[cfg(unix)]
fn set_restrictive_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("failed to set MCP token permissions: {e}"))
}

#[cfg(not(unix))]
fn set_restrictive_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_64_hex_chars() {
        let token = generate();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generate_is_unique() {
        let a = generate();
        let b = generate();
        assert_ne!(a, b);
    }

    #[test]
    fn load_or_create_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let token1 = load_or_create(dir.path()).unwrap();
        let token2 = load_or_create(dir.path()).unwrap();
        assert_eq!(token1, token2, "should load the same token");
    }

    #[test]
    fn regenerate_produces_new_token() {
        let dir = tempfile::tempdir().unwrap();
        let token1 = load_or_create(dir.path()).unwrap();
        let token2 = regenerate(dir.path()).unwrap();
        assert_ne!(token1, token2, "should produce a new token");
    }
}
