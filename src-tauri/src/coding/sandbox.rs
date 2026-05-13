//! Coding workflow sandbox guards (chunk 47.5).
//!
//! Provides two safety checks used by coding workflows:
//! - `SecretsDenylist`: path-pattern denylist for sensitive files.
//! - `shell_preflight`: best-effort shell command token scan.

use std::path::{Component, Path, PathBuf};

use glob::Pattern;

/// Default denylist patterns for coding workflows.
pub const DEFAULT_DENIED_PATTERNS: [&str; 6] = [
    "**/.env",
    "**/.env.*",
    "**/secrets/**",
    "**/*.pem",
    "**/id_rsa*",
    "**/*.key",
];

#[derive(Debug, Clone)]
pub struct SecretsDenylist {
    patterns: Vec<Pattern>,
}

impl Default for SecretsDenylist {
    fn default() -> Self {
        Self::new(
            DEFAULT_DENIED_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        )
        .expect("default denylist patterns must compile")
    }
}

impl SecretsDenylist {
    pub fn new(patterns: Vec<String>) -> Result<Self, String> {
        let mut compiled = Vec::new();
        for raw in patterns {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            compiled.push(
                Pattern::new(trimmed)
                    .map_err(|e| format!("invalid denylist pattern `{trimmed}`: {e}"))?,
            );
            // Root-level convenience for `**/foo` style patterns.
            if let Some(stripped) = trimmed.strip_prefix("**/") {
                if !stripped.is_empty() {
                    compiled.push(
                        Pattern::new(stripped)
                            .map_err(|e| format!("invalid denylist pattern `{stripped}`: {e}"))?,
                    );
                }
            }
        }
        Ok(Self { patterns: compiled })
    }

    /// Check a repository-relative path (e.g. `src/main.rs`).
    pub fn is_rel_path_denied(&self, rel: &str) -> bool {
        let rel_norm = rel.replace('\\', "/").trim_start_matches("./").to_string();
        if rel_norm.is_empty() {
            return false;
        }
        let file_name = Path::new(&rel_norm)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.patterns
            .iter()
            .any(|p| p.matches(&rel_norm) || (!file_name.is_empty() && p.matches(&file_name)))
    }

    fn is_abs_or_rel_denied(&self, worktree: &Path, candidate: &Path) -> bool {
        let abs = if candidate.is_absolute() {
            normalize_path(candidate)
        } else {
            normalize_path(&worktree.join(candidate))
        };
        let abs_norm = abs.to_string_lossy().replace('\\', "/");
        let rel_norm = abs.strip_prefix(worktree).ok().map(|p| {
            p.to_string_lossy()
                .replace('\\', "/")
                .trim_start_matches("./")
                .to_string()
        });
        let file_name = abs
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        self.patterns.iter().any(|p| {
            p.matches(&abs_norm)
                || rel_norm.as_ref().is_some_and(|r| p.matches(r))
                || (!file_name.is_empty() && p.matches(&file_name))
        })
    }
}

/// Best-effort shell command pre-flight.
///
/// Parses command text with `shlex::split`, extracts path-like tokens
/// (contains `/` or `\\`, starts with `.` or `~`), resolves paths relative
/// to `worktree`, then rejects commands touching denylisted paths.
pub fn shell_preflight(worktree: &Path, cmd: &str) -> Result<(), String> {
    let tokens =
        shlex::split(cmd).ok_or_else(|| "shell preflight: command parse failed".to_string())?;
    let denylist = SecretsDenylist::default();

    for token in tokens {
        if !is_path_like_token(&token) {
            continue;
        }
        let resolved = resolve_token_path(worktree, &token);
        if denylist.is_abs_or_rel_denied(worktree, &resolved) {
            return Err(format!(
                "shell preflight rejected token `{token}`: matches secrets denylist"
            ));
        }
    }

    Ok(())
}

fn is_path_like_token(token: &str) -> bool {
    token.contains('/') || token.contains('\\') || token.starts_with('.') || token.starts_with('~')
}

fn resolve_token_path(worktree: &Path, token: &str) -> PathBuf {
    if let Some(stripped) = token.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return normalize_path(&home.join(stripped));
        }
    }
    let p = PathBuf::from(token);
    if p.is_absolute() {
        normalize_path(&p)
    } else {
        normalize_path(&worktree.join(p))
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                let _ = out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_denylist_hits_each_pattern_family() {
        let d = SecretsDenylist::default();
        assert!(d.is_rel_path_denied(".env"));
        assert!(d.is_rel_path_denied(".env.local"));
        assert!(d.is_rel_path_denied("config/secrets/token.txt"));
        assert!(d.is_rel_path_denied("certs/server.pem"));
        assert!(d.is_rel_path_denied(".ssh/id_rsa"));
        assert!(d.is_rel_path_denied("keys/private.key"));
    }

    #[test]
    fn shell_preflight_rejects_dotenv() {
        let root = tempfile::tempdir().unwrap();
        let err = shell_preflight(root.path(), "cat ./.env").unwrap_err();
        assert!(err.contains("rejected token"));
    }

    #[test]
    fn shell_preflight_rejects_home_private_key() {
        let root = tempfile::tempdir().unwrap();
        let err = shell_preflight(root.path(), "cat ~/.ssh/id_rsa").unwrap_err();
        assert!(err.contains("rejected token"));
    }

    #[test]
    fn shell_preflight_allows_safe_workspace_file() {
        let root = tempfile::tempdir().unwrap();
        let ok = shell_preflight(root.path(), "cat ./README.md");
        assert!(ok.is_ok());
    }
}
