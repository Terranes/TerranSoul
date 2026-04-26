//! MCP tool definitions and dispatch.
//!
//! Defines the 8 brain tools exposed via MCP (matching the
//! `BrainGateway` trait surface) and dispatches JSON-RPC `tools/call`
//! requests to the gateway.

use serde_json::{json, Value};

use crate::ai_integrations::gateway::*;

/// Return the static list of MCP tool definitions (name, description,
/// input JSON Schema). Called by the `tools/list` JSON-RPC method.
pub fn definitions() -> Vec<Value> {
    vec![
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
    ]
}

/// Dispatch a `tools/call` request to the appropriate gateway method.
/// Returns `Ok(json_string)` on success or `Err(message)` on failure.
pub async fn dispatch(
    gw: &dyn BrainGateway,
    caps: &GatewayCaps,
    tool_name: &str,
    args: &Value,
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
                direction: args["direction"]
                    .as_str()
                    .unwrap_or("both")
                    .to_string(),
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
            let query = args["query"]
                .as_str()
                .unwrap_or_default()
                .to_string();
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
        _ => Err(format!("unknown tool: {tool_name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_has_8_tools() {
        let defs = definitions();
        assert_eq!(defs.len(), 8);
    }

    #[test]
    fn all_tools_have_name_and_input_schema() {
        for def in definitions() {
            assert!(def["name"].is_string(), "tool missing name: {def}");
            assert!(
                def["inputSchema"].is_object(),
                "tool missing inputSchema: {}",
                def["name"]
            );
        }
    }

    #[test]
    fn tool_names_match_dispatch_arms() {
        let defs = definitions();
        let names: Vec<&str> = defs
            .iter()
            .map(|d| d["name"].as_str().unwrap())
            .collect();
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
}
