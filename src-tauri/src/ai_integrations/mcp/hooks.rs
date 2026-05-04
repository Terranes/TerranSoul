//! Editor pre/post-tool-use hooks (Chunk 31.8).
//!
//! Handles MCP notifications and HTTP endpoints for AI coding editors
//! (Claude Code, Cursor, etc.) to signal before and after tool calls.
//!
//! - **Pre-hook**: enriches a search query with cluster + process context
//! - **Post-hook**: detects if a git commit happened and triggers background re-indexing

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::AppState;

// ─── Types ──────────────────────────────────────────────────────────────────

/// Pre-tool-use hook request.
#[derive(Debug, Clone, Deserialize)]
pub struct PreToolUseRequest {
    /// The tool name about to be called.
    pub tool_name: String,
    /// The query or primary argument for search-like tools.
    pub query: Option<String>,
    /// Current file the editor is focused on.
    pub file_path: Option<String>,
}

/// Pre-tool-use hook response with enriched context.
#[derive(Debug, Clone, Serialize)]
pub struct PreToolUseResponse {
    /// Enriched query (original + cluster/process context appended).
    pub enriched_query: Option<String>,
    /// Relevant clusters for the current file.
    pub clusters: Vec<String>,
    /// Relevant processes for the current file.
    pub processes: Vec<String>,
}

/// Post-tool-use hook request.
#[derive(Debug, Clone, Deserialize)]
pub struct PostToolUseRequest {
    /// The tool name that was called.
    pub tool_name: String,
    /// Whether the tool call succeeded.
    pub success: bool,
    /// The repo path to check for staleness.
    pub repo_path: Option<String>,
}

/// Post-tool-use hook response.
#[derive(Debug, Clone, Serialize)]
pub struct PostToolUseResponse {
    /// Whether a re-index was triggered.
    pub reindex_triggered: bool,
    /// Message about what happened.
    pub message: String,
}

/// Tracks the last known git HEAD for staleness detection.
#[derive(Debug, Clone, Default)]
pub struct IndexStalenessTracker {
    /// Map of repo_path → last known HEAD commit hash.
    last_known_head: std::collections::HashMap<String, String>,
}

impl IndexStalenessTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the repo HEAD has changed since we last saw it.
    pub fn is_stale(&mut self, repo_path: &str) -> bool {
        let current_head = read_git_head(repo_path);
        let Some(head) = current_head else {
            return false;
        };

        if let Some(last) = self.last_known_head.get(repo_path) {
            if *last != head {
                self.last_known_head.insert(repo_path.to_string(), head);
                return true;
            }
            false
        } else {
            self.last_known_head.insert(repo_path.to_string(), head);
            false // First time seeing this repo, not "stale"
        }
    }
}

/// Read the current git HEAD commit hash for a repo.
fn read_git_head(repo_path: &str) -> Option<String> {
    let head_path = std::path::Path::new(repo_path).join(".git/HEAD");
    let content = std::fs::read_to_string(&head_path).ok()?;
    let content = content.trim();

    if let Some(ref_path) = content.strip_prefix("ref: ") {
        // Read the actual commit from the ref file
        let ref_file = std::path::Path::new(repo_path).join(".git").join(ref_path);
        std::fs::read_to_string(ref_file)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        // Detached HEAD — content is the commit hash
        Some(content.to_string())
    }
}

// ─── Hook dispatch ──────────────────────────────────────────────────────────

/// Handle a pre-tool-use notification/request.
/// Enriches search queries with cluster + process context from the symbol index.
pub fn handle_pre_tool_use(
    req: &PreToolUseRequest,
    app_state: Option<&AppState>,
) -> PreToolUseResponse {
    let state = match app_state {
        Some(s) => s,
        None => {
            return PreToolUseResponse {
                enriched_query: req.query.clone(),
                clusters: vec![],
                processes: vec![],
            }
        }
    };

    let data_dir = &state.data_dir;
    let conn = match crate::coding::symbol_index::open_db(data_dir) {
        Ok(c) => c,
        Err(_) => {
            return PreToolUseResponse {
                enriched_query: req.query.clone(),
                clusters: vec![],
                processes: vec![],
            }
        }
    };

    // Get the default repo
    let repo_id: i64 = match conn.query_row(
        "SELECT id FROM code_repos ORDER BY indexed_at DESC LIMIT 1",
        [],
        |r| r.get(0),
    ) {
        Ok(id) => id,
        Err(_) => {
            return PreToolUseResponse {
                enriched_query: req.query.clone(),
                clusters: vec![],
                processes: vec![],
            }
        }
    };

    let mut cluster_labels = Vec::new();
    let mut process_names = Vec::new();

    // If a file_path is provided, find the clusters/processes that contain symbols from that file
    if let Some(file_path) = &req.file_path {
        let rel_path = file_path.replace('\\', "/");

        // Find symbols in this file
        let symbols = crate::coding::symbol_index::query_symbols_in_file(&conn, repo_id, &rel_path)
            .unwrap_or_default();

        if !symbols.is_empty() {
            // Get clusters
            let clusters =
                crate::coding::processes::list_clusters(&conn, repo_id).unwrap_or_default();

            // Get symbol IDs for this file
            let sym_ids: Vec<i64> = conn
                .prepare("SELECT id FROM code_symbols WHERE repo_id = ?1 AND file = ?2")
                .ok()
                .map(|mut stmt| {
                    stmt.query_map(rusqlite::params![repo_id, rel_path], |r| r.get(0))
                        .map(|rows| rows.filter_map(|r| r.ok()).collect())
                        .unwrap_or_default()
                })
                .unwrap_or_default();

            for cluster in &clusters {
                if sym_ids.iter().any(|id| cluster.symbol_ids.contains(id)) {
                    cluster_labels.push(cluster.label.clone());
                }
            }

            // Get processes
            let processes =
                crate::coding::processes::list_processes(&conn, repo_id).unwrap_or_default();
            for proc in &processes {
                if proc
                    .steps
                    .iter()
                    .any(|s| symbols.iter().any(|sym| sym.name == s.name))
                {
                    process_names.push(proc.entry_point.clone());
                }
            }
        }
    }

    // Enrich the query with context
    let enriched_query = req.query.as_ref().map(|q| {
        if cluster_labels.is_empty() && process_names.is_empty() {
            return q.clone();
        }
        let mut enriched = q.clone();
        if !cluster_labels.is_empty() {
            enriched.push_str(&format!(
                " [relevant clusters: {}]",
                cluster_labels.join(", ")
            ));
        }
        if !process_names.is_empty() {
            enriched.push_str(&format!(
                " [relevant processes: {}]",
                process_names.join(", ")
            ));
        }
        enriched
    });

    PreToolUseResponse {
        enriched_query,
        clusters: cluster_labels,
        processes: process_names,
    }
}

/// Handle a post-tool-use notification/request.
/// Checks if git HEAD changed (indicating a commit) and triggers re-indexing.
pub fn handle_post_tool_use(
    req: &PostToolUseRequest,
    tracker: &mut IndexStalenessTracker,
    app_state: Option<&AppState>,
) -> PostToolUseResponse {
    let state = match app_state {
        Some(s) => s,
        None => {
            return PostToolUseResponse {
                reindex_triggered: false,
                message: "no app state available".into(),
            }
        }
    };

    // Determine repo path
    let repo_path = if let Some(rp) = &req.repo_path {
        rp.clone()
    } else {
        // Try to find from indexed repos
        let data_dir = &state.data_dir;
        let conn = match crate::coding::symbol_index::open_db(data_dir) {
            Ok(c) => c,
            Err(_) => {
                return PostToolUseResponse {
                    reindex_triggered: false,
                    message: "code index not available".into(),
                }
            }
        };
        match conn.query_row(
            "SELECT path FROM code_repos ORDER BY indexed_at DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        ) {
            Ok(p) => p,
            Err(_) => {
                return PostToolUseResponse {
                    reindex_triggered: false,
                    message: "no repos indexed".into(),
                }
            }
        }
    };

    // Check if HEAD changed
    if !tracker.is_stale(&repo_path) {
        return PostToolUseResponse {
            reindex_triggered: false,
            message: "index is up-to-date".into(),
        };
    }

    // Trigger background re-index
    let data_dir = state.data_dir.clone();
    let repo_path_buf = PathBuf::from(&repo_path);
    tokio::spawn(async move {
        let _ = tokio::task::spawn_blocking(move || {
            let _ = crate::coding::symbol_index::index_repo(&data_dir, &repo_path_buf);
        })
        .await;
    });

    PostToolUseResponse {
        reindex_triggered: true,
        message: format!("re-indexing triggered for {repo_path} (HEAD changed)"),
    }
}

/// Handle an MCP notification method (no response expected).
/// Returns `true` if the notification was recognized and handled.
pub fn handle_notification(
    method: &str,
    params: &Value,
    app_state: Option<&AppState>,
    tracker: &Arc<Mutex<IndexStalenessTracker>>,
) -> bool {
    match method {
        "notifications/tools/pre_use" | "editor/preToolUse" => {
            let req: PreToolUseRequest = match serde_json::from_value(params.clone()) {
                Ok(r) => r,
                Err(_) => return false,
            };
            let _response = handle_pre_tool_use(&req, app_state);
            true
        }
        "notifications/tools/post_use" | "editor/postToolUse" => {
            let req: PostToolUseRequest = match serde_json::from_value(params.clone()) {
                Ok(r) => r,
                Err(_) => return false,
            };
            // Need to lock the tracker — spawn a task to avoid blocking
            let tracker = tracker.clone();
            let app_state_clone = app_state.cloned();
            tokio::spawn(async move {
                let mut guard = tracker.lock().await;
                let _response = handle_post_tool_use(&req, &mut guard, app_state_clone.as_ref());
            });
            true
        }
        _ => false,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staleness_tracker_first_check_not_stale() {
        let mut tracker = IndexStalenessTracker::new();
        // No .git dir → always not stale
        let is_stale = tracker.is_stale("/nonexistent/repo");
        assert!(!is_stale);
    }

    #[test]
    fn pre_hook_without_state_returns_original_query() {
        let req = PreToolUseRequest {
            tool_name: "brain_search".into(),
            query: Some("find the auth module".into()),
            file_path: None,
        };
        let resp = handle_pre_tool_use(&req, None);
        assert_eq!(resp.enriched_query, Some("find the auth module".into()));
        assert!(resp.clusters.is_empty());
    }

    #[test]
    fn post_hook_without_state_returns_no_reindex() {
        let req = PostToolUseRequest {
            tool_name: "brain_search".into(),
            success: true,
            repo_path: None,
        };
        let mut tracker = IndexStalenessTracker::new();
        let resp = handle_post_tool_use(&req, &mut tracker, None);
        assert!(!resp.reindex_triggered);
    }

    #[test]
    fn read_git_head_returns_none_for_nonexistent() {
        let result = read_git_head("/nonexistent/path/xyz");
        assert!(result.is_none());
    }
}
