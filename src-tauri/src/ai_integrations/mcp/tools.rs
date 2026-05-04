//! MCP tool definitions and dispatch.
//!
//! Defines the brain tools exposed via MCP (matching the
//! `BrainGateway` trait surface) plus code-intelligence tools that
//! delegate to the GitNexus sidecar. Dispatches JSON-RPC `tools/call`
//! requests accordingly.

use serde_json::{json, Value};

use crate::ai_integrations::gateway::*;
use crate::AppState;

/// Return the static list of MCP tool definitions (name, description,
/// input JSON Schema). Called by the `tools/list` JSON-RPC method.
/// When `caps.code_read` is true, includes the code-intelligence tools.
pub fn definitions(caps: &GatewayCaps) -> Vec<Value> {
    let mut defs = vec![
        json!({
            "name": "brain_search",
            "description": "Hybrid + RRF + optional HyDE search over TerranSoul's memories. Returns top-k results with relevance scores.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query text" },
                    "limit": { "type": "integer", "description": "Max results (1-100, default 10)" },
                    "mode": { "type": "string", "enum": ["hybrid", "rrf", "hyde"], "description": "Search mode (default: rrf)" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "brain_get_entry",
            "description": "Retrieve a full memory entry by id, including tags, importance, cognitive kind, and source URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Memory entry id" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "brain_list_recent",
            "description": "List the most recent memories, optionally filtered by cognitive kind, tag, or timestamp.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max results (1-200, default 20)" },
                    "kind": { "type": "string", "description": "Cognitive kind filter (fact, preference, episode, procedure)" },
                    "tag": { "type": "string", "description": "Comma/space-separated tag filter (any match)" },
                    "since": { "type": "integer", "description": "Unix-ms lower bound on created_at" }
                }
            }
        }),
        json!({
            "name": "brain_kg_neighbors",
            "description": "Knowledge-graph one-hop neighbourhood: retrieve typed/directional edges and neighbours around a memory entry.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Center memory entry id" },
                    "depth": { "type": "integer", "description": "Traversal depth (only 1 currently supported)" },
                    "direction": { "type": "string", "enum": ["both", "outgoing", "incoming"], "description": "Edge direction filter (default: both)" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "brain_summarize",
            "description": "LLM-summarize text or memory ids using TerranSoul's active brain.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to summarize" },
                    "memory_ids": { "type": "string", "description": "Comma-separated memory entry ids to summarize" }
                }
            }
        }),
        json!({
            "name": "brain_suggest_context",
            "description": "Flagship call: curated context pack with top memories, KG neighborhood, LLM summary, and delta-stable fingerprint for caching.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural-language query or current chat turn" },
                    "file_path": { "type": "string", "description": "Current file path (ranking signal)" },
                    "selection": { "type": "string", "description": "Selected text (ranking signal)" },
                    "limit": { "type": "integer", "description": "Top-k memories (1-20, default 5)" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "brain_ingest_url",
            "description": "Fetch, extract, chunk, and embed a URL into TerranSoul's brain. Requires write capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "URL to ingest" },
                    "tags": { "type": "string", "description": "Comma-separated tags (default: 'imported')" },
                    "importance": { "type": "integer", "description": "Importance score 1-5 (default: 4)" }
                },
                "required": ["url"]
            }
        }),
        json!({
            "name": "brain_health",
            "description": "TerranSoul brain status: version, active provider, model, RAG quality percentage, memory count.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
    ];

    if caps.code_read {
        defs.extend(code_tool_definitions());
    }

    defs
}

/// Dispatch a `tools/call` request to the appropriate gateway method.
/// Returns `Ok(json_string)` on success or `Err(message)` on failure.
/// `app_state` is required for code-intelligence tools that need the
/// GitNexus sidecar; pass `None` when running in stdio mode.
pub async fn dispatch(
    gw: &dyn BrainGateway,
    caps: &GatewayCaps,
    tool_name: &str,
    args: &Value,
    app_state: Option<&AppState>,
) -> Result<String, String> {
    match tool_name {
        "brain_search" => {
            let query = args["query"].as_str().unwrap_or_default().to_string();
            let limit = args["limit"].as_u64().map(|n| n as usize);
            let mode = match args["mode"].as_str() {
                Some("hybrid") => SearchMode::Hybrid,
                Some("hyde") => SearchMode::Hyde,
                _ => SearchMode::Rrf,
            };
            let req = SearchRequest { query, limit, mode };
            gw.search(caps, req)
                .await
                .map(|hits| serde_json::to_string(&hits).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "brain_get_entry" => {
            let id = args["id"]
                .as_i64()
                .ok_or_else(|| "missing required param: id".to_string())?;
            gw.get_entry(caps, id)
                .await
                .map(|e| serde_json::to_string(&e).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "brain_list_recent" => {
            let req = RecentRequest {
                limit: args["limit"].as_u64().map(|n| n as usize),
                kind: args["kind"].as_str().map(String::from),
                tag: args["tag"].as_str().map(String::from),
                since: args["since"].as_i64(),
            };
            gw.list_recent(caps, req)
                .await
                .map(|entries| serde_json::to_string(&entries).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "brain_kg_neighbors" => {
            let id = args["id"]
                .as_i64()
                .ok_or_else(|| "missing required param: id".to_string())?;
            let req = KgRequest {
                id,
                depth: args["depth"].as_u64().map(|n| n as u8).unwrap_or(1),
                direction: args["direction"].as_str().unwrap_or("both").to_string(),
            };
            gw.kg_neighbors(caps, req)
                .await
                .map(|n| serde_json::to_string(&n).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "brain_summarize" => {
            let ids = args["memory_ids"].as_str().map(|s| {
                s.split(',')
                    .filter_map(|id| id.trim().parse::<i64>().ok())
                    .collect::<Vec<_>>()
            });
            let req = SummarizeRequest {
                text: args["text"].as_str().map(String::from),
                memory_ids: ids,
            };
            gw.summarize(caps, req)
                .await
                .map(|r| serde_json::to_string(&r).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "brain_suggest_context" => {
            let query = args["query"].as_str().unwrap_or_default().to_string();
            let req = SuggestContextRequest {
                query,
                file_path: args["file_path"].as_str().map(String::from),
                cursor_offset: args["cursor_offset"].as_u64(),
                selection: args["selection"].as_str().map(String::from),
                limit: args["limit"].as_u64().map(|n| n as usize),
            };
            gw.suggest_context(caps, req)
                .await
                .map(|p| serde_json::to_string(&p).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "brain_ingest_url" => {
            let url = args["url"]
                .as_str()
                .ok_or_else(|| "missing required param: url".to_string())?
                .to_string();
            let req = IngestUrlRequest {
                url,
                tags: args["tags"].as_str().map(String::from),
                importance: args["importance"].as_i64(),
            };
            gw.ingest_url(caps, req)
                .await
                .map(|r| serde_json::to_string(&r).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "brain_health" => gw
            .health(caps)
            .await
            .map(|h| serde_json::to_string(&h).unwrap_or_default())
            .map_err(|e| e.to_string()),

        // ─── Code-intelligence tools (GitNexus sidecar) ─────────────────
        "code_query" | "code_context" | "code_impact" | "code_detect_changes"
        | "code_graph_sync" => {
            dispatch_code_tool(caps, tool_name, args, app_state).await
        }

        _ => Err(format!("unknown tool: {tool_name}")),
    }
}

// ─── Code-intelligence tool definitions ─────────────────────────────────────

/// The 5 code-intelligence tools exposed when `code_read` is granted.
fn code_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "code_query",
            "description": "Natural-language code-intelligence query via the GitNexus sidecar. Returns ranked code snippets, explanations, and file references.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "prompt": { "type": "string", "description": "Natural-language question about the codebase" }
                },
                "required": ["prompt"]
            }
        }),
        json!({
            "name": "code_context",
            "description": "Fetch ranked code context for a symbol or file path via GitNexus. Returns definitions, usages, and surrounding code.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "Symbol name or file path to get context for" },
                    "max_results": { "type": "integer", "description": "Maximum results (default: 10)" }
                },
                "required": ["target"]
            }
        }),
        json!({
            "name": "code_impact",
            "description": "Compute the blast-radius of changing a symbol: which files, functions, and tests are affected.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "symbol": { "type": "string", "description": "Symbol name to analyze impact for" }
                },
                "required": ["symbol"]
            }
        }),
        json!({
            "name": "code_detect_changes",
            "description": "Diff-aware change summary between two git refs. Identifies new/modified/deleted symbols and affected callers.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from_ref": { "type": "string", "description": "Base git ref (branch, tag, or commit SHA)" },
                    "to_ref": { "type": "string", "description": "Target git ref to compare against" }
                },
                "required": ["from_ref", "to_ref"]
            }
        }),
        json!({
            "name": "code_graph_sync",
            "description": "Trigger a knowledge-graph sync from the GitNexus sidecar into TerranSoul's memory store. Returns a mirror report with inserted/reused counts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_label": { "type": "string", "description": "Repository label for the KG namespace (e.g. 'terransoul')" }
                },
                "required": ["repo_label"]
            }
        }),
    ]
}

/// Dispatch a code-intelligence tool call to the GitNexus sidecar.
async fn dispatch_code_tool(
    caps: &GatewayCaps,
    tool_name: &str,
    args: &Value,
    app_state: Option<&AppState>,
) -> Result<String, String> {
    if !caps.code_read {
        return Err("permission denied: capability `code_read` is not granted to this client".into());
    }

    let state = app_state.ok_or_else(|| {
        "sidecar not configured: code tools require --mcp-app mode. \
         Use `configure_gitnexus_sidecar` via the TerranSoul app first."
            .to_string()
    })?;

    let bridge = ensure_sidecar(state).await?;

    match tool_name {
        "code_query" => {
            let prompt = args["prompt"]
                .as_str()
                .ok_or_else(|| "missing required param: prompt".to_string())?;
            bridge
                .query(prompt)
                .await
                .map(|v| serde_json::to_string(&v).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "code_context" => {
            let target = args["target"]
                .as_str()
                .ok_or_else(|| "missing required param: target".to_string())?;
            let max_results = args["max_results"].as_u64().unwrap_or(10) as u32;
            bridge
                .context(target, max_results)
                .await
                .map(|v| serde_json::to_string(&v).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "code_impact" => {
            let symbol = args["symbol"]
                .as_str()
                .ok_or_else(|| "missing required param: symbol".to_string())?;
            bridge
                .impact(symbol)
                .await
                .map(|v| serde_json::to_string(&v).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "code_detect_changes" => {
            let from_ref = args["from_ref"]
                .as_str()
                .ok_or_else(|| "missing required param: from_ref".to_string())?;
            let to_ref = args["to_ref"]
                .as_str()
                .ok_or_else(|| "missing required param: to_ref".to_string())?;
            bridge
                .detect_changes(from_ref, to_ref)
                .await
                .map(|v| serde_json::to_string(&v).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "code_graph_sync" => {
            let repo_label = args["repo_label"]
                .as_str()
                .ok_or_else(|| "missing required param: repo_label".to_string())?;
            bridge
                .graph(repo_label)
                .await
                .map(|v| serde_json::to_string(&v).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("unknown code tool: {tool_name}")),
    }
}

/// Resolve the GitNexus sidecar bridge from AppState, checking
/// capability and lazily spawning if needed.
async fn ensure_sidecar(
    state: &AppState,
) -> Result<std::sync::Arc<crate::agent::gitnexus_sidecar::GitNexusSidecar>, String> {
    use crate::sandbox::Capability;

    // Check the code_intelligence capability is granted for the sidecar agent.
    let granted = {
        let cap = state.capability_store.lock().await;
        cap.has_capability(
            crate::commands::gitnexus::GITNEXUS_AGENT,
            &Capability::CodeIntelligence,
        )
    };
    if !granted {
        return Err(
            "sidecar not configured: the `code_intelligence` capability has not been granted \
             for the gitnexus-sidecar agent. Grant it via the TerranSoul Control Panel or call \
             `configure_gitnexus_sidecar` first."
                .to_string(),
        );
    }

    // Fast path — return the cached bridge.
    {
        let guard = state.gitnexus_sidecar.lock().await;
        if let Some(b) = guard.as_ref() {
            b.set_capability(true).await;
            return Ok(b.clone());
        }
    }

    // Slow path — spawn a fresh sidecar.
    let cfg = state.gitnexus_config.lock().await.clone();
    let bridge = crate::agent::gitnexus_sidecar::GitNexusSidecar::spawn(&cfg)
        .await
        .map_err(|e| e.to_string())?;
    bridge.set_capability(true).await;
    let arc = std::sync::Arc::new(bridge);
    let mut guard = state.gitnexus_sidecar.lock().await;
    *guard = Some(arc.clone());
    Ok(arc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_has_8_brain_tools_without_code_read() {
        let defs = definitions(&GatewayCaps::default());
        assert_eq!(defs.len(), 8);
    }

    #[test]
    fn definitions_has_13_tools_with_code_read() {
        let caps = GatewayCaps::READ_WRITE;
        let defs = definitions(&caps);
        assert_eq!(defs.len(), 13);
    }

    #[test]
    fn all_tools_have_name_and_input_schema() {
        let caps = GatewayCaps::READ_WRITE;
        for def in definitions(&caps) {
            assert!(def["name"].is_string(), "tool missing name: {def}");
            assert!(
                def["inputSchema"].is_object(),
                "tool missing inputSchema: {}",
                def["name"]
            );
        }
    }

    #[test]
    fn brain_tool_names_match_dispatch_arms() {
        let defs = definitions(&GatewayCaps::default());
        let names: Vec<&str> = defs.iter().map(|d| d["name"].as_str().unwrap()).collect();
        let expected = [
            "brain_search",
            "brain_get_entry",
            "brain_list_recent",
            "brain_kg_neighbors",
            "brain_summarize",
            "brain_suggest_context",
            "brain_ingest_url",
            "brain_health",
        ];
        assert_eq!(names, expected);
    }

    #[test]
    fn code_tool_names_are_correct() {
        let caps = GatewayCaps::READ_WRITE;
        let defs = definitions(&caps);
        let code_names: Vec<&str> = defs[8..]
            .iter()
            .map(|d| d["name"].as_str().unwrap())
            .collect();
        let expected = [
            "code_query",
            "code_context",
            "code_impact",
            "code_detect_changes",
            "code_graph_sync",
        ];
        assert_eq!(code_names, expected);
    }

    #[tokio::test]
    async fn code_tools_denied_without_code_read() {
        let caps = GatewayCaps::default(); // code_read = false
        let args = serde_json::json!({"prompt": "test"});
        let result = dispatch_code_tool(&caps, "code_query", &args, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("permission denied"));
    }

    #[tokio::test]
    async fn code_tools_error_without_app_state() {
        let caps = GatewayCaps::READ_WRITE;
        let args = serde_json::json!({"prompt": "test"});
        let result = dispatch_code_tool(&caps, "code_query", &args, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("sidecar not configured"));
    }
}
