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

        // ─── Code-intelligence tools (native symbol index) ────────────
        "code_query" | "code_context" | "code_impact" | "code_rename" => {
            dispatch_code_tool(caps, tool_name, args, app_state).await
        }

        _ => Err(format!("unknown tool: {tool_name}")),
    }
}

// ─── Code-intelligence tool definitions (native) ───────────────────────────

/// The 3 code-intelligence tools backed by the native symbol index.
fn code_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "code_query",
            "description": "Search the code symbol index by name or file. Returns symbols with file/line, kind, and parent context. Process-grouped when clusters exist.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "symbol": { "type": "string", "description": "Symbol name to search for (exact or prefix match)" },
                    "file": { "type": "string", "description": "File path to list symbols in (alternative to symbol search)" },
                    "repo": { "type": "string", "description": "Repository path filter (defaults to first indexed repo)" },
                    "limit": { "type": "integer", "description": "Max results (default: 20)" }
                }
            }
        }),
        json!({
            "name": "code_context",
            "description": "360-degree view of a symbol: its definition, incoming callers, outgoing callees, cluster membership, and processes it participates in.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "symbol": { "type": "string", "description": "Symbol name to get context for" },
                    "repo": { "type": "string", "description": "Repository path filter (defaults to first indexed repo)" }
                },
                "required": ["symbol"]
            }
        }),
        json!({
            "name": "code_impact",
            "description": "Compute the blast-radius of changing a symbol: BFS along incoming call edges, grouped by depth. Shows which functions/files would be affected.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "symbol": { "type": "string", "description": "Symbol name to analyze impact for" },
                    "depth": { "type": "integer", "description": "Max BFS depth (default: 5)" },
                    "repo": { "type": "string", "description": "Repository path filter (defaults to first indexed repo)" }
                },
                "required": ["symbol"]
            }
        }),
        json!({
            "name": "code_rename",
            "description": "Rename a symbol across the codebase. Returns an edit plan with graph-resolved (high confidence) and text-search (lower confidence) edits. Use dry_run=true to preview without applying.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "symbol": { "type": "string", "description": "Current symbol name to rename" },
                    "new_name": { "type": "string", "description": "New name for the symbol" },
                    "dry_run": { "type": "boolean", "description": "If true, return edit plan without applying (default: true)" },
                    "repo": { "type": "string", "description": "Repository path (defaults to first indexed repo)" }
                },
                "required": ["symbol", "new_name"]
            }
        }),
    ]
}

/// Dispatch a code-intelligence tool call using the native symbol index.
async fn dispatch_code_tool(
    caps: &GatewayCaps,
    tool_name: &str,
    args: &Value,
    app_state: Option<&AppState>,
) -> Result<String, String> {
    if !caps.code_read {
        return Err(
            "permission denied: capability `code_read` is not granted to this client".into(),
        );
    }

    let state =
        app_state.ok_or_else(|| "code tools require app state: use --mcp-app mode".to_string())?;

    let data_dir = state.data_dir.clone();
    let conn = crate::coding::symbol_index::open_db(&data_dir)
        .map_err(|e| format!("code index not available: {e}"))?;

    // Resolve the repo_id — use `repo` arg if provided, else first indexed repo.
    let repo_id: i64 = if let Some(repo_path) = args["repo"].as_str() {
        conn.query_row(
            "SELECT id FROM code_repos WHERE path = ?1",
            rusqlite::params![repo_path],
            |r| r.get(0),
        )
        .map_err(|_| format!("repo not indexed: {repo_path}"))?
    } else {
        conn.query_row(
            "SELECT id FROM code_repos ORDER BY indexed_at DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "no repos indexed yet — run code_index_repo first".to_string())?
    };

    match tool_name {
        "code_query" => {
            let limit = args["limit"].as_u64().unwrap_or(20) as usize;

            let results = if let Some(symbol_name) = args["symbol"].as_str() {
                crate::coding::symbol_index::query_symbols_by_name(&conn, repo_id, symbol_name)
                    .map_err(|e| e.to_string())?
            } else if let Some(file_path) = args["file"].as_str() {
                crate::coding::symbol_index::query_symbols_in_file(&conn, repo_id, file_path)
                    .map_err(|e| e.to_string())?
            } else {
                return Err("provide either `symbol` or `file` parameter".into());
            };

            let truncated: Vec<_> = results.into_iter().take(limit).collect();
            serde_json::to_string(&truncated).map_err(|e| e.to_string())
        }

        "code_context" => {
            let symbol_name = args["symbol"]
                .as_str()
                .ok_or_else(|| "missing required param: symbol".to_string())?;

            let call_graph = crate::coding::resolver::call_graph(&conn, repo_id, symbol_name)
                .map_err(|e| e.to_string())?;

            // Try to find cluster membership
            let cluster: Option<crate::coding::processes::Cluster> = {
                let clusters =
                    crate::coding::processes::list_clusters(&conn, repo_id).unwrap_or_default();
                // Look up the symbol's DB id for cluster matching
                let sym_id: Option<i64> = conn
                    .query_row(
                        "SELECT id FROM code_symbols WHERE repo_id = ?1 AND name = ?2 LIMIT 1",
                        rusqlite::params![repo_id, symbol_name],
                        |r| r.get(0),
                    )
                    .ok();
                if let Some(id) = sym_id {
                    clusters.into_iter().find(|c| c.symbol_ids.contains(&id))
                } else {
                    None
                }
            };

            // Find processes this symbol participates in
            let processes: Vec<_> = {
                let all_procs =
                    crate::coding::processes::list_processes(&conn, repo_id).unwrap_or_default();
                all_procs
                    .into_iter()
                    .filter(|p| {
                        p.entry_point == symbol_name
                            || p.steps.iter().any(|s| s.name == symbol_name)
                    })
                    .collect()
            };

            let result = serde_json::json!({
                "call_graph": call_graph,
                "cluster": cluster,
                "processes": processes,
            });
            serde_json::to_string(&result).map_err(|e| e.to_string())
        }

        "code_impact" => {
            let symbol_name = args["symbol"]
                .as_str()
                .ok_or_else(|| "missing required param: symbol".to_string())?;
            let max_depth = args["depth"].as_u64().unwrap_or(5) as u32;

            // Get the call graph (incoming edges = direct callers)
            let call_graph = crate::coding::resolver::call_graph(&conn, repo_id, symbol_name)
                .map_err(|e| e.to_string())?;

            // BFS over incoming callers to find transitive impact
            let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
            visited.insert(symbol_name.to_string());
            let mut frontier: Vec<String> = call_graph
                .incoming
                .iter()
                .map(|e| e.symbol_name.clone())
                .collect();
            let mut depth_groups: Vec<Value> = Vec::new();

            // Depth 0 = direct callers
            let direct: Vec<_> = call_graph
                .incoming
                .iter()
                .map(|e| {
                    json!({
                        "symbol": e.symbol_name,
                        "file": e.file,
                        "line": e.line,
                    })
                })
                .collect();
            if !direct.is_empty() {
                depth_groups.push(json!({ "depth": 1, "affected": direct }));
            }

            // Transitive callers up to max_depth
            for depth in 2..=max_depth {
                let mut next_frontier = Vec::new();
                let mut affected_at_depth = Vec::new();

                for caller in &frontier {
                    if !visited.insert(caller.clone()) {
                        continue;
                    }
                    if let Ok(cg) = crate::coding::resolver::call_graph(&conn, repo_id, caller) {
                        for edge in &cg.incoming {
                            if !visited.contains(&edge.symbol_name) {
                                affected_at_depth.push(json!({
                                    "symbol": edge.symbol_name,
                                    "file": edge.file,
                                    "line": edge.line,
                                }));
                                next_frontier.push(edge.symbol_name.clone());
                            }
                        }
                    }
                }

                if !affected_at_depth.is_empty() {
                    depth_groups.push(json!({ "depth": depth, "affected": affected_at_depth }));
                }
                frontier = next_frontier;
                if frontier.is_empty() {
                    break;
                }
            }

            let result = json!({
                "symbol": symbol_name,
                "total_affected": visited.len() - 1,
                "by_depth": depth_groups,
            });
            serde_json::to_string(&result).map_err(|e| e.to_string())
        }

        "code_rename" => {
            let symbol_name = args["symbol"]
                .as_str()
                .ok_or_else(|| "missing required param: symbol".to_string())?;
            let new_name = args["new_name"]
                .as_str()
                .ok_or_else(|| "missing required param: new_name".to_string())?;
            let dry_run = args["dry_run"].as_bool().unwrap_or(true);

            // Resolve repo path from ID
            let repo_path: String = conn
                .query_row(
                    "SELECT path FROM code_repos WHERE id = ?1",
                    rusqlite::params![repo_id],
                    |r| r.get(0),
                )
                .map_err(|e| format!("cannot find repo path: {e}"))?;

            let result = crate::coding::rename::rename_symbol(
                &data_dir,
                std::path::Path::new(&repo_path),
                symbol_name,
                new_name,
                dry_run,
            )
            .map_err(|e| e.to_string())?;

            serde_json::to_string(&result).map_err(|e| e.to_string())
        }

        _ => Err(format!("unknown code tool: {tool_name}")),
    }
}

// ─── MCP Resources ──────────────────────────────────────────────────────────

/// Resource URI templates exposed via `resources/list`.
pub fn resource_definitions(app_state: Option<&AppState>) -> Vec<Value> {
    let _ = app_state; // reserved for future dynamic resource enumeration
    vec![
        json!({
            "uri": "terransoul://repos",
            "name": "Indexed Repositories",
            "description": "List all repos in the code intelligence index.",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "terransoul://clusters",
            "name": "Code Clusters",
            "description": "Functional clusters (community-detected modules) for the default repo.",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "terransoul://processes",
            "name": "Execution Processes",
            "description": "Entry-point to call-chain execution flows for the default repo.",
            "mimeType": "application/json"
        }),
    ]
}

/// Read a resource by URI. Returns the resource contents.
pub fn read_resource(uri: &str, app_state: Option<&AppState>) -> Result<Value, String> {
    let state = app_state.ok_or_else(|| "resources require app state".to_string())?;
    let data_dir = &state.data_dir;

    let conn = crate::coding::symbol_index::open_db(data_dir)
        .map_err(|e| format!("code index not available: {e}"))?;

    match uri {
        "terransoul://repos" => {
            let mut stmt = conn
                .prepare(
                    "SELECT id, path, label, indexed_at FROM code_repos ORDER BY indexed_at DESC",
                )
                .map_err(|e| e.to_string())?;
            let repos: Vec<Value> = stmt
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, i64>(0)?,
                        "path": row.get::<_, String>(1)?,
                        "label": row.get::<_, String>(2)?,
                        "indexed_at": row.get::<_, String>(3)?,
                    }))
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(json!({ "repos": repos }))
        }

        "terransoul://clusters" => {
            let repo_id: i64 = conn
                .query_row(
                    "SELECT id FROM code_repos ORDER BY indexed_at DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| "no repos indexed".to_string())?;
            let clusters = crate::coding::processes::list_clusters(&conn, repo_id)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(&clusters).map_err(|e| e.to_string())
        }

        "terransoul://processes" => {
            let repo_id: i64 = conn
                .query_row(
                    "SELECT id FROM code_repos ORDER BY indexed_at DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| "no repos indexed".to_string())?;
            let processes = crate::coding::processes::list_processes(&conn, repo_id)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(&processes).map_err(|e| e.to_string())
        }

        _ => Err(format!("unknown resource URI: {uri}")),
    }
}

// ─── MCP Prompts ────────────────────────────────────────────────────────────

/// Prompt templates exposed via `prompts/list`.
pub fn prompt_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "detect_impact",
            "description": "Analyze the blast-radius of changing a symbol. Returns a structured impact report with affected callers grouped by depth.",
            "arguments": [
                {
                    "name": "symbol",
                    "description": "The symbol name to analyze",
                    "required": true
                }
            ]
        }),
        json!({
            "name": "generate_map",
            "description": "Generate a high-level architecture map of the codebase showing clusters, entry points, and key execution flows.",
            "arguments": [
                {
                    "name": "repo",
                    "description": "Repository path (optional, defaults to most recently indexed)",
                    "required": false
                }
            ]
        }),
    ]
}

/// Execute a prompt template, returning messages for the LLM.
pub fn get_prompt(name: &str, args: &Value, app_state: Option<&AppState>) -> Result<Value, String> {
    let state = app_state.ok_or_else(|| "prompts require app state".to_string())?;
    let data_dir = &state.data_dir;

    let conn = crate::coding::symbol_index::open_db(data_dir)
        .map_err(|e| format!("code index not available: {e}"))?;

    let repo_id: i64 = if let Some(repo_path) = args["repo"].as_str() {
        conn.query_row(
            "SELECT id FROM code_repos WHERE path = ?1",
            rusqlite::params![repo_path],
            |r| r.get(0),
        )
        .map_err(|_| format!("repo not indexed: {repo_path}"))?
    } else {
        conn.query_row(
            "SELECT id FROM code_repos ORDER BY indexed_at DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "no repos indexed".to_string())?
    };

    match name {
        "detect_impact" => {
            let symbol = args["symbol"]
                .as_str()
                .ok_or_else(|| "missing required argument: symbol".to_string())?;

            let call_graph = crate::coding::resolver::call_graph(&conn, repo_id, symbol)
                .map_err(|e| e.to_string())?;

            let callers_desc = call_graph
                .incoming
                .iter()
                .map(|e| format!("- `{}` in {} (line {})", e.symbol_name, e.file, e.line))
                .collect::<Vec<_>>()
                .join("\n");

            let callees_desc = call_graph
                .outgoing
                .iter()
                .map(|e| format!("- `{}` in {} (line {})", e.symbol_name, e.file, e.line))
                .collect::<Vec<_>>()
                .join("\n");

            let prompt_text = format!(
                "Analyze the impact of changing the symbol `{symbol}`.\n\n\
                 ## Direct Callers ({} functions call this):\n{callers_desc}\n\n\
                 ## Direct Callees ({} functions called by this):\n{callees_desc}\n\n\
                 Please assess:\n\
                 1. Which callers are most likely to break if this symbol's signature changes?\n\
                 2. What tests should be run?\n\
                 3. Are there any circular dependencies to watch out for?",
                call_graph.incoming.len(),
                call_graph.outgoing.len(),
            );

            Ok(json!({
                "messages": [
                    { "role": "user", "content": { "type": "text", "text": prompt_text } }
                ]
            }))
        }

        "generate_map" => {
            let clusters =
                crate::coding::processes::list_clusters(&conn, repo_id).unwrap_or_default();
            let processes =
                crate::coding::processes::list_processes(&conn, repo_id).unwrap_or_default();

            let cluster_desc = clusters
                .iter()
                .map(|c| format!("- **{}** ({} symbols)", c.label, c.size))
                .collect::<Vec<_>>()
                .join("\n");

            let process_desc = processes
                .iter()
                .take(10)
                .map(|p| {
                    let steps: Vec<_> = p.steps.iter().take(5).map(|s| s.name.as_str()).collect();
                    format!("- `{}` → {}", p.entry_point, steps.join(" → "))
                })
                .collect::<Vec<_>>()
                .join("\n");

            let prompt_text = format!(
                "Generate a high-level architecture map for this codebase.\n\n\
                 ## Functional Clusters ({} detected):\n{cluster_desc}\n\n\
                 ## Key Execution Flows ({} processes):\n{process_desc}\n\n\
                 Please produce:\n\
                 1. A Mermaid diagram showing cluster relationships\n\
                 2. A brief description of each cluster's responsibility\n\
                 3. The main data/control flow through the system",
                clusters.len(),
                processes.len(),
            );

            Ok(json!({
                "messages": [
                    { "role": "user", "content": { "type": "text", "text": prompt_text } }
                ]
            }))
        }

        _ => Err(format!("unknown prompt: {name}")),
    }
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
    fn definitions_has_12_tools_with_code_read() {
        let caps = GatewayCaps::READ_WRITE;
        let defs = definitions(&caps);
        assert_eq!(defs.len(), 12);
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
        let expected = ["code_query", "code_context", "code_impact", "code_rename"];
        assert_eq!(code_names, expected);
    }

    #[tokio::test]
    async fn code_tools_denied_without_code_read() {
        let caps = GatewayCaps::default(); // code_read = false
        let args = serde_json::json!({"symbol": "test"});
        let result = dispatch_code_tool(&caps, "code_query", &args, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("permission denied"));
    }

    #[tokio::test]
    async fn code_tools_error_without_app_state() {
        let caps = GatewayCaps::READ_WRITE;
        let args = serde_json::json!({"symbol": "test"});
        let result = dispatch_code_tool(&caps, "code_query", &args, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("code tools require app state"));
    }

    #[test]
    fn resource_definitions_has_3_entries() {
        let defs = resource_definitions(None);
        assert_eq!(defs.len(), 3);
        let uris: Vec<&str> = defs.iter().map(|d| d["uri"].as_str().unwrap()).collect();
        assert_eq!(
            uris,
            [
                "terransoul://repos",
                "terransoul://clusters",
                "terransoul://processes"
            ]
        );
    }

    #[test]
    fn prompt_definitions_has_2_entries() {
        let defs = prompt_definitions();
        assert_eq!(defs.len(), 2);
        let names: Vec<&str> = defs.iter().map(|d| d["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["detect_impact", "generate_map"]);
    }
}
