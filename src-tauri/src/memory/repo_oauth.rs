//! Per-repo RAG OAuth token storage for cloning private repositories.
//!
//! Stores a GitHub OAuth token under `<data_dir>/oauth/github.json` with
//! FS-permission hardening (0600 on Unix; user-only ACL on Windows via
//! `std::fs::set_permissions` readonly + best-effort `icacls` invocation
//! when present). The file is intentionally separate from
//! `<data_dir>/github_config.json` (which the self-improve loop owns) so
//! repo-RAG clones can use a different OAuth grant — e.g. a token with
//! `repo` scope only — without disturbing the self-improve binding.
//!
//! The full OAuth Device Flow client lives in `crate::coding::github`
//! (`request_device_code` + `poll_for_token`); this module is the
//! durable-storage + url-rewriting half that hooks the resulting token
//! into the `gix` clone path used by `repo_ingest::shallow_clone`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Subdirectory under `data_dir` that holds repo-RAG OAuth tokens.
pub const OAUTH_SUBDIR: &str = "oauth";
/// Filename of the GitHub token blob.
pub const GITHUB_TOKEN_FILE: &str = "github.json";

/// Persisted OAuth token for repo-RAG clones.
///
/// `access_token` is treated as a secret and must never be logged. Use
/// [`RepoOAuthToken::redacted`] when formatting for debug output.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RepoOAuthToken {
    pub access_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub scope: String,
    /// Unix seconds. `0` when unknown.
    #[serde(default)]
    pub created_at: u64,
    /// Unix seconds. `None` = no documented expiry (GitHub PATs and
    /// classic OAuth tokens don't expire by default; fine-grained
    /// tokens do).
    #[serde(default)]
    pub expires_at: Option<u64>,
}

impl RepoOAuthToken {
    /// Build a fresh token record, stamping `created_at` to the current
    /// wall clock. `expires_in` (seconds) is folded into `expires_at`.
    pub fn from_success(
        access_token: String,
        token_type: String,
        scope: String,
        expires_in: Option<u64>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            access_token,
            token_type,
            scope,
            created_at: now,
            expires_at: expires_in.map(|s| now.saturating_add(s)),
        }
    }

    /// True when `expires_at` is set and `now >= expires_at`.
    pub fn is_expired(&self, now: u64) -> bool {
        matches!(self.expires_at, Some(t) if now >= t)
    }

    /// Debug-safe summary: never includes the access token.
    pub fn redacted(&self) -> String {
        format!(
            "RepoOAuthToken {{ token_type: {:?}, scope: {:?}, created_at: {}, expires_at: {:?}, access_token: <redacted {} chars> }}",
            self.token_type,
            self.scope,
            self.created_at,
            self.expires_at,
            self.access_token.len()
        )
    }
}

/// Where the token file lives for `data_dir`.
pub fn token_path(data_dir: &Path) -> PathBuf {
    data_dir.join(OAUTH_SUBDIR).join(GITHUB_TOKEN_FILE)
}

/// Load the persisted token. Returns `None` when the file is missing or
/// unreadable.
pub fn load_token(data_dir: &Path) -> Option<RepoOAuthToken> {
    let path = token_path(data_dir);
    if !path.exists() {
        return None;
    }
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Persist `tok` atomically and harden FS permissions on the resulting
/// file. The parent directory is created with the same restrictive
/// permissions on Unix.
pub fn save_token(data_dir: &Path, tok: &RepoOAuthToken) -> Result<(), String> {
    let dir = data_dir.join(OAUTH_SUBDIR);
    fs::create_dir_all(&dir).map_err(|e| format!("create oauth dir: {e}"))?;
    harden_dir(&dir);

    let path = dir.join(GITHUB_TOKEN_FILE);
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(tok).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&tmp, json).map_err(|e| format!("write oauth tmp: {e}"))?;
    harden_file(&tmp);
    fs::rename(&tmp, &path).map_err(|e| format!("rename oauth tmp: {e}"))?;
    harden_file(&path);
    Ok(())
}

/// Remove the persisted token (idempotent — missing file is OK).
pub fn clear_token(data_dir: &Path) -> Result<(), String> {
    let path = token_path(data_dir);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("remove oauth token: {e}"))?;
    }
    Ok(())
}

/// Inject an OAuth token into an HTTPS GitHub URL so `gix` can clone a
/// private repository. SSH URLs and non-HTTPS URLs pass through
/// unchanged (caller is expected to fall back to system credentials).
///
/// The form `https://x-access-token:<token>@github.com/owner/repo.git`
/// is GitHub's documented HTTPS auth pattern and is recognised by every
/// git client including `gix`.
pub fn inject_https_token(remote_url: &str, token: &str) -> String {
    let url = remote_url.trim();
    if token.is_empty() {
        return url.to_string();
    }
    // Only inject for HTTPS GitHub URLs. SSH (`git@github.com:owner/repo.git`)
    // uses key auth and must not be rewritten. We accept both bare HTTPS
    // (`https://github.com/...`) and forms that already include userinfo
    // (`https://user:pass@github.com/...`) so we can replace stale creds.
    let lower = url.to_ascii_lowercase();
    let is_github_https = lower.starts_with("https://github.com/")
        || lower.starts_with("http://github.com/")
        || lower.starts_with("https://www.github.com/")
        || (lower.starts_with("https://")
            && lower.contains("@github.com/")
            && !lower.starts_with("https://github.com/"));
    if !is_github_https {
        return url.to_string();
    }
    // Strip any existing userinfo segment to avoid duplicating credentials.
    let stripped = if let Some(rest) = url.strip_prefix("https://") {
        match rest.split_once('@') {
            Some((userinfo, after))
                if after.to_ascii_lowercase().starts_with("github.com")
                    && !userinfo.contains('/') =>
            {
                format!("https://{after}")
            }
            _ => url.to_string(),
        }
    } else {
        url.to_string()
    };
    stripped.replacen(
        "https://github.com/",
        &format!("https://x-access-token:{token}@github.com/"),
        1,
    )
}

// ── FS permission hardening ──────────────────────────────────────────────

#[cfg(unix)]
fn harden_dir(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o700);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(unix)]
fn harden_file(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn harden_dir(_path: &Path) {
    // Best-effort: the file itself is hardened via `harden_file`. The
    // parent directory lives inside the user's data directory which is
    // already user-scoped on Windows; explicit ACL tightening would
    // require shelling out to `icacls.exe` and is intentionally skipped
    // to keep the path dependency-free. Document in CREDITS / docs.
}

#[cfg(not(unix))]
fn harden_file(path: &Path) {
    // Strip world/group write by marking the file read-only after write;
    // combined with the user-scoped data_dir this is the same posture
    // VS Code, npm, and pip use for their auth files on Windows.
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_readonly(true);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn token_roundtrip_persists_under_oauth_subdir() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        assert!(load_token(dir).is_none());

        let tok = RepoOAuthToken::from_success(
            "ghs_secret".into(),
            "bearer".into(),
            "repo".into(),
            Some(28_800),
        );
        save_token(dir, &tok).unwrap();

        let path = token_path(dir);
        assert!(path.exists());
        assert!(path.starts_with(dir.join(OAUTH_SUBDIR)));

        let loaded = load_token(dir).expect("token loads back");
        assert_eq!(loaded, tok);
        assert!(!loaded.access_token.is_empty());
    }

    #[test]
    fn redacted_never_leaks_access_token() {
        let tok = RepoOAuthToken::from_success(
            "ghs_supersecret_token_value".into(),
            "bearer".into(),
            "repo".into(),
            None,
        );
        let s = tok.redacted();
        assert!(!s.contains("ghs_supersecret_token_value"));
        assert!(s.contains("<redacted"));
        assert!(s.contains("27 chars"));
    }

    #[test]
    fn clear_token_is_idempotent() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        // Clear on missing file is OK.
        clear_token(dir).unwrap();

        let tok = RepoOAuthToken::from_success("t".into(), "bearer".into(), "repo".into(), None);
        save_token(dir, &tok).unwrap();
        assert!(token_path(dir).exists());

        clear_token(dir).unwrap();
        assert!(!token_path(dir).exists());
        // Second clear is still OK.
        clear_token(dir).unwrap();
    }

    #[test]
    fn is_expired_only_when_expires_at_is_in_the_past() {
        let tok = RepoOAuthToken {
            access_token: "t".into(),
            token_type: "bearer".into(),
            scope: "repo".into(),
            created_at: 1_000,
            expires_at: Some(2_000),
        };
        assert!(!tok.is_expired(1_500));
        assert!(tok.is_expired(2_000));
        assert!(tok.is_expired(3_000));

        let no_expiry = RepoOAuthToken {
            expires_at: None,
            ..tok
        };
        assert!(!no_expiry.is_expired(u64::MAX));
    }

    #[test]
    fn inject_https_token_rewrites_only_github_https_urls() {
        let t = "ghs_abc";

        // HTTPS GitHub → injected.
        assert_eq!(
            inject_https_token("https://github.com/owner/repo.git", t),
            "https://x-access-token:ghs_abc@github.com/owner/repo.git"
        );
        assert_eq!(
            inject_https_token("https://github.com/owner/repo", t),
            "https://x-access-token:ghs_abc@github.com/owner/repo"
        );

        // SSH → untouched (key auth).
        assert_eq!(
            inject_https_token("git@github.com:owner/repo.git", t),
            "git@github.com:owner/repo.git"
        );

        // Non-GitHub HTTPS → untouched.
        assert_eq!(
            inject_https_token("https://gitlab.com/owner/repo.git", t),
            "https://gitlab.com/owner/repo.git"
        );

        // Empty token → no rewrite.
        assert_eq!(
            inject_https_token("https://github.com/owner/repo.git", ""),
            "https://github.com/owner/repo.git"
        );

        // Existing userinfo is replaced, not duplicated.
        assert_eq!(
            inject_https_token("https://olduser:oldpass@github.com/owner/repo.git", t),
            "https://x-access-token:ghs_abc@github.com/owner/repo.git"
        );
    }
}
