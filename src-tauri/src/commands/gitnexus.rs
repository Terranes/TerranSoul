//! Tauri commands for the GitNexus sidecar bridge (Chunk 2.1).
//!
//! These are the four read-only code-intelligence tools that Phase 13 Tier 1
//! exposes to the frontend. Every command:
//!
//! 1. Resolves the user's `code_intelligence` consent for the
//!    `gitnexus-sidecar` agent from the [`crate::sandbox::CapabilityStore`].
//! 2. Lazily spawns the sidecar (`npx gitnexus mcp` by default — overridable
//!    via [`configure_gitnexus_sidecar`]) on first use and caches the handle
//!    in [`crate::AppState::gitnexus_sidecar`].
//! 3. Forwards the call to the bridge and returns the JSON-RPC `result`
//!    payload as a `serde_json::Value` so the frontend can render whatever
//!    shape GitNexus chose for its tool response.
//!
//! Sidecar processes are kept alive for the lifetime of `AppState`. The
//! [`tokio::process::Command::kill_on_drop`] flag set by
//! [`crate::agent::gitnexus_sidecar::StdioTransport::spawn`] guarantees the
//! child process is reaped when TerranSoul exits.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

use crate::agent::gitnexus_sidecar::{
    GitNexusError, GitNexusSidecar, SidecarConfig,
};
use crate::memory::gitnexus_mirror::{self, KgPayload, MirrorReport};
use crate::sandbox::Capability;
use crate::AppState;

/// The agent name the user must approve `code_intelligence` for.
pub const GITNEXUS_AGENT: &str = "gitnexus-sidecar";

/// Frontend-facing configuration mirror of [`SidecarConfig`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitNexusConfigDto {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub working_dir: Option<String>,
}

impl From<GitNexusConfigDto> for SidecarConfig {
    fn from(dto: GitNexusConfigDto) -> Self {
        Self {
            command: dto.command,
            args: dto.args,
            working_dir: dto.working_dir,
        }
    }
}

impl From<SidecarConfig> for GitNexusConfigDto {
    fn from(cfg: SidecarConfig) -> Self {
        Self {
            command: cfg.command,
            args: cfg.args,
            working_dir: cfg.working_dir,
        }
    }
}

/// Replace the sidecar configuration. The currently-running sidecar (if any)
/// is dropped, which kills its child process. The next tool call will spawn
/// a fresh sidecar with the new config.
#[tauri::command(rename_all = "camelCase")]
pub async fn configure_gitnexus_sidecar(
    config: GitNexusConfigDto,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut cfg_guard = state.gitnexus_config.lock().await;
    *cfg_guard = config.into();
    // Drop the current sidecar so the next call respawns with the new config.
    let mut sidecar_guard = state.gitnexus_sidecar.lock().await;
    *sidecar_guard = None;
    Ok(())
}

/// Read the current sidecar configuration.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_gitnexus_sidecar_config(
    state: State<'_, AppState>,
) -> Result<GitNexusConfigDto, String> {
    let cfg = state.gitnexus_config.lock().await.clone();
    Ok(cfg.into())
}

/// Returns whether a sidecar handle has been spawned and the
/// `code_intelligence` capability has been granted.
#[tauri::command(rename_all = "camelCase")]
pub async fn gitnexus_sidecar_status(state: State<'_, AppState>) -> Result<SidecarStatus, String> {
    let granted = {
        let cap = state.capability_store.lock().await;
        cap.has_capability(GITNEXUS_AGENT, &Capability::CodeIntelligence)
    };
    let running = state.gitnexus_sidecar.lock().await.is_some();
    Ok(SidecarStatus {
        capability_granted: granted,
        running,
    })
}

/// Status of the GitNexus sidecar, exposed to the BrainView UI.
#[derive(Debug, Clone, Serialize)]
pub struct SidecarStatus {
    pub capability_granted: bool,
    pub running: bool,
}

/// `gitnexus_query` — natural-language code-intelligence query.
#[tauri::command(rename_all = "camelCase")]
pub async fn gitnexus_query(
    prompt: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let bridge = ensure_bridge(&state).await.map_err(stringify)?;
    bridge.query(&prompt).await.map_err(stringify)
}

/// `gitnexus_context` — fetch ranked code snippets for a symbol or file.
#[tauri::command(rename_all = "camelCase")]
pub async fn gitnexus_context(
    target: String,
    max_results: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let bridge = ensure_bridge(&state).await.map_err(stringify)?;
    bridge
        .context(&target, max_results.unwrap_or(10))
        .await
        .map_err(stringify)
}

/// `gitnexus_impact` — compute the blast-radius of changing a symbol.
#[tauri::command(rename_all = "camelCase")]
pub async fn gitnexus_impact(
    symbol: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let bridge = ensure_bridge(&state).await.map_err(stringify)?;
    bridge.impact(&symbol).await.map_err(stringify)
}

/// `gitnexus_detect_changes` — diff-aware change summary between two refs.
#[tauri::command(rename_all = "camelCase")]
pub async fn gitnexus_detect_changes(
    from_ref: String,
    to_ref: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let bridge = ensure_bridge(&state).await.map_err(stringify)?;
    bridge
        .detect_changes(&from_ref, &to_ref)
        .await
        .map_err(stringify)
}

/// Lazily spawn the sidecar (or return the cached handle), apply the
/// current capability grant from the `CapabilityStore`, and return a
/// shared bridge handle.
async fn ensure_bridge(state: &State<'_, AppState>) -> Result<Arc<GitNexusSidecar>, GitNexusError> {
    // Refresh capability state every call — the user may toggle consent
    // between calls and we need to honour the latest decision.
    let granted = {
        let cap = state.capability_store.lock().await;
        cap.has_capability(GITNEXUS_AGENT, &Capability::CodeIntelligence)
    };
    if !granted {
        return Err(GitNexusError::CapabilityDenied);
    }

    // Fast path — return the cached bridge if we already have one.
    {
        let guard = state.gitnexus_sidecar.lock().await;
        if let Some(b) = guard.as_ref() {
            b.set_capability(true).await;
            return Ok(b.clone());
        }
    }

    // Slow path — spawn a fresh sidecar under the configured command.
    let cfg = state.gitnexus_config.lock().await.clone();
    let bridge = GitNexusSidecar::spawn(&cfg).await?;
    bridge.set_capability(true).await;
    let arc = Arc::new(bridge);
    let mut guard = state.gitnexus_sidecar.lock().await;
    *guard = Some(arc.clone());
    Ok(arc)
}

fn stringify(e: GitNexusError) -> String {
    e.to_string()
}

/// `gitnexus_sync` — Tier 3: mirror GitNexus's structured knowledge
/// graph into the SQLite memory store under the
/// `gitnexus:<repo_label>` `edge_source` provenance.
///
/// Strictly opt-in: the frontend invokes this only when the user
/// asks. Never runs at startup. Returns a [`MirrorReport`] with
/// inserted-vs-reused counts so the UI can show meaningful feedback.
///
/// `kg_payload` is optional. When present, it bypasses the sidecar
/// call entirely — useful for tests and for clients that fetched the
/// KG out-of-band. When omitted, the command calls the sidecar's
/// `graph` MCP tool and parses the response.
#[tauri::command(rename_all = "camelCase")]
pub async fn gitnexus_sync(
    repo_label: String,
    kg_payload: Option<KgPayload>,
    state: State<'_, AppState>,
) -> Result<MirrorReport, String> {
    if repo_label.trim().is_empty() {
        return Err("gitnexus_sync: repoLabel must be non-empty".into());
    }

    // Resolve the KG payload — either from the caller or by talking
    // to the sidecar.
    let payload = match kg_payload {
        Some(p) => p,
        None => {
            let bridge = ensure_bridge(&state).await.map_err(stringify)?;
            let raw = bridge.graph(&repo_label).await.map_err(stringify)?;
            extract_kg_payload(&raw).map_err(|e| {
                format!("gitnexus_sync: could not parse `graph` response: {e}")
            })?
        }
    };

    let store = state
        .memory_store
        .lock()
        .map_err(|e| format!("gitnexus_sync: memory store lock poisoned: {e}"))?;
    gitnexus_mirror::mirror_kg(&store, &repo_label, &payload)
}

/// `gitnexus_unmirror` — remove every edge previously inserted by
/// [`gitnexus_sync`] for the given `repo_label`. Memory nodes are
/// preserved. Returns the number of edges deleted.
#[tauri::command(rename_all = "camelCase")]
pub async fn gitnexus_unmirror(
    repo_label: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    if repo_label.trim().is_empty() {
        return Err("gitnexus_unmirror: repoLabel must be non-empty".into());
    }
    let store = state
        .memory_store
        .lock()
        .map_err(|e| format!("gitnexus_unmirror: memory store lock poisoned: {e}"))?;
    gitnexus_mirror::unmirror(&store, &repo_label)
}

/// One mirrored repo as surfaced by [`gitnexus_list_mirrors`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitNexusMirrorSummary {
    /// Full `edge_source` value (e.g. `gitnexus:repo:foo/bar@abc`).
    pub edge_source: String,
    /// Caller-supplied scope without the `gitnexus:` prefix
    /// (e.g. `repo:foo/bar@abc`). Pass this to `gitnexus_unmirror`.
    pub scope: String,
    /// Number of mirrored edges currently in the store.
    pub edge_count: i64,
    /// Wall-clock time (Unix-ms) of the most-recent edge insertion
    /// for this scope — i.e. when the repo was last synced.
    pub last_synced_at: i64,
}

/// `gitnexus_list_mirrors` — enumerate every repo currently mirrored
/// from GitNexus, ordered by most-recent-sync first.
///
/// Powers the Phase 13 Tier 4 BrainView "Code knowledge" panel
/// (Chunk 2.4): each row is a clickable mirror with edge count,
/// last-sync timestamp, and a one-button rollback path.
#[tauri::command]
pub async fn gitnexus_list_mirrors(
    state: State<'_, AppState>,
) -> Result<Vec<GitNexusMirrorSummary>, String> {
    let store = state
        .memory_store
        .lock()
        .map_err(|e| format!("gitnexus_list_mirrors: memory store lock poisoned: {e}"))?;
    let rows = store
        .list_external_mirrors(&format!(
            "{}%",
            crate::memory::gitnexus_mirror::GITNEXUS_EDGE_SOURCE_PREFIX
        ))
        .map_err(|e| format!("gitnexus_list_mirrors: {e}"))?;
    let prefix = crate::memory::gitnexus_mirror::GITNEXUS_EDGE_SOURCE_PREFIX;
    Ok(rows
        .into_iter()
        .map(|(edge_source, edge_count, last_synced_at)| {
            let scope = edge_source
                .strip_prefix(prefix)
                .unwrap_or(&edge_source)
                .to_string();
            GitNexusMirrorSummary {
                edge_source,
                scope,
                edge_count,
                last_synced_at,
            }
        })
        .collect())
}

/// Pull a [`KgPayload`] out of the raw `graph` MCP response.
///
/// Tries three shapes in order:
/// 1. Top-level `{ "nodes": [...], "edges": [...] }`.
/// 2. Nested under `{ "graph": { "nodes": [...], "edges": [...] } }`.
/// 3. MCP `tools/call` content envelope `{ "content": [{ "text": "<json>" }] }`
///    where the embedded text is the JSON KG.
fn extract_kg_payload(raw: &Value) -> Result<KgPayload, String> {
    // Shape 1.
    if raw.get("nodes").is_some() || raw.get("edges").is_some() {
        return serde_json::from_value(raw.clone())
            .map_err(|e| e.to_string());
    }
    // Shape 2.
    if let Some(graph) = raw.get("graph") {
        return serde_json::from_value(graph.clone())
            .map_err(|e| e.to_string());
    }
    // Shape 3 — MCP-standard text content.
    if let Some(arr) = raw.get("content").and_then(|c| c.as_array()) {
        for item in arr {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                if let Ok(p) = serde_json::from_str::<KgPayload>(text) {
                    return Ok(p);
                }
            }
        }
    }
    Err(format!(
        "no recognised graph shape (nodes/edges, graph.*, or content[].text); got keys: {:?}",
        raw.as_object().map(|o| o.keys().collect::<Vec<_>>())
    ))
}

#[cfg(test)]
mod tests {
    //! These tests exercise the **bridge** behind the commands directly with
    //! the in-memory mock transport — Tauri commands themselves are thin
    //! wrappers and would otherwise require a `MockRuntime` harness for very
    //! little additional coverage.

    use crate::agent::gitnexus_sidecar::{mock::MockTransport, GitNexusError, GitNexusSidecar};
    use serde_json::json;

    #[tokio::test]
    async fn bridge_rejects_when_capability_not_granted() {
        let mock = MockTransport::new();
        let bridge = GitNexusSidecar::new(Box::new(mock));
        // Default state: capability NOT granted.
        let err = bridge.query("hello").await.unwrap_err();
        assert!(matches!(err, GitNexusError::CapabilityDenied));
    }

    #[tokio::test]
    async fn full_query_round_trip_when_capability_granted() {
        let mock = MockTransport::new();
        mock.push_response(
            1,
            json!({"protocolVersion": "2024-11-05", "serverInfo": {"name": "gitnexus", "version": "1.6.0"}}),
        )
        .await;
        mock.push_response(2, json!({"answer": "fn parse() at src/main.rs:10"}))
            .await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        let result = bridge.query("where is parse?").await.unwrap();
        assert_eq!(result["answer"], "fn parse() at src/main.rs:10");
    }

    #[tokio::test]
    async fn context_passes_max_results_argument() {
        let mock = MockTransport::new();
        let (sent, _) = mock.handles();
        mock.push_response(1, json!({"protocolVersion": "2024-11-05", "serverInfo": {}}))
            .await;
        mock.push_response(2, json!({"snippets": []})).await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        bridge.context("Foo::bar", 7).await.unwrap();
        let sent = sent.lock().await;
        let call: serde_json::Value = serde_json::from_str(&sent[2]).unwrap();
        assert_eq!(call["params"]["arguments"]["maxResults"], 7);
    }

    #[tokio::test]
    async fn impact_propagates_rpc_error() {
        let mock = MockTransport::new();
        mock.push_response(1, json!({"protocolVersion": "2024-11-05", "serverInfo": {}}))
            .await;
        mock.push_error(2, -32602, "missing argument: symbol").await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        let err = bridge.impact("").await.unwrap_err();
        assert!(matches!(err, GitNexusError::Rpc { code: -32602, .. }));
    }

    // ── Chunk 2.3 — KG mirror response-extraction tests ──────────────

    use super::extract_kg_payload;

    #[test]
    fn extract_kg_payload_top_level_shape() {
        let raw = json!({
            "nodes": [{"id": "n1", "label": "a", "kind": "module"}],
            "edges": [{"src": "n1", "dst": "n1", "type": "CONTAINS"}]
        });
        let p = extract_kg_payload(&raw).unwrap();
        assert_eq!(p.nodes.len(), 1);
        assert_eq!(p.edges.len(), 1);
        assert_eq!(p.edges[0].rel_type, "CONTAINS");
    }

    #[test]
    fn extract_kg_payload_nested_under_graph() {
        let raw = json!({
            "graph": {
                "nodes": [{"id": "n1"}],
                "edges": []
            }
        });
        let p = extract_kg_payload(&raw).unwrap();
        assert_eq!(p.nodes.len(), 1);
    }

    #[test]
    fn extract_kg_payload_mcp_content_envelope() {
        let inner = r#"{"nodes":[{"id":"n1"}],"edges":[]}"#;
        let raw = json!({
            "content": [{"type": "text", "text": inner}]
        });
        let p = extract_kg_payload(&raw).unwrap();
        assert_eq!(p.nodes.len(), 1);
    }

    #[test]
    fn extract_kg_payload_rejects_unknown_shape() {
        let raw = json!({ "stuff": 123 });
        assert!(extract_kg_payload(&raw).is_err());
    }

    #[tokio::test]
    async fn graph_method_calls_correct_tool() {
        let mock = MockTransport::new();
        let (sent, _) = mock.handles();
        mock.push_response(
            1,
            json!({"protocolVersion": "2024-11-05", "serverInfo": {}}),
        )
        .await;
        mock.push_response(2, json!({"nodes": [], "edges": []}))
            .await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        let _ = bridge.graph("repo:foo/bar@abc").await.unwrap();
        let sent = sent.lock().await;
        let call: serde_json::Value = serde_json::from_str(&sent[2]).unwrap();
        assert_eq!(call["params"]["name"], "graph");
        assert_eq!(call["params"]["arguments"]["repo"], "repo:foo/bar@abc");
    }
}
