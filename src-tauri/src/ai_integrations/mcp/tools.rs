//! MCP tool definitions and dispatch.
//!
//! Defines the brain tools exposed via MCP (matching the
//! `BrainGateway` trait surface) plus native code-intelligence tools
//! backed by TerranSoul's code index. Dispatches JSON-RPC `tools/call`
//! requests accordingly.

use serde_json::{json, Value};

use crate::ai_integrations::gateway::*;
use crate::memory::wiki::{
    append_and_review_queue, audit_report, ensure_source_dedup, god_nodes, surprising_connections,
    AuditConfig,
};
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
                    "mode": { "type": "string", "enum": ["hybrid", "rrf", "hyde"], "description": "Search mode (default: rrf)" },
                    "rerank": { "type": "boolean", "description": "Run LLM-as-judge rerank for RRF/HyDE when a local brain is available (default: true)" },
                    "rerank_threshold": { "type": "number", "description": "Normalised 0.0-1.0 rerank pruning threshold (default: 0.55)" }
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
            "description": "LLM-summarize direct text, memory ids, or a search query using TerranSoul's active brain. If you have a topic rather than ids, pass query instead of guessing memory_ids.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to summarize" },
                    "memory_ids": { "type": "string", "description": "Comma-separated memory entry ids to summarize" },
                    "query": { "type": "string", "description": "Search query; top matching memories are resolved and summarized" },
                    "limit": { "type": "integer", "description": "Top-k memories when query is supplied (1-20, default 5)" }
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
            "description": "TerranSoul brain status: version, active provider, model, RAG quality, memory count, and descriptions explaining what the numbers mean.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "brain_failover_status",
            "description": "Provider failover status: healthy/rate-limited/unhealthy counts, selected provider, and recent failover events.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "brain_wiki_audit",
            "description": "Knowledge Wiki audit over TerranSoul's memory graph: conflicts, orphans, stale entries, pending embeddings, and graph totals.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max ids returned per audit bucket (1-200, default 50)" },
                    "stale_threshold": { "type": "number", "description": "Decay score below which unprotected low-importance memories are stale (0.0-1.0, default 0.20)" }
                }
            }
        }),
        json!({
            "name": "brain_wiki_spotlight",
            "description": "Knowledge Wiki spotlight: most-connected memories by live graph edge degree.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max memories to return (1-100, default 10)" }
                }
            }
        }),
        json!({
            "name": "brain_wiki_serendipity",
            "description": "Knowledge Wiki serendipity: high-confidence links that bridge distinct memory communities.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max cross-community links to return (1-100, default 10)" }
                }
            }
        }),
        json!({
            "name": "brain_wiki_revisit",
            "description": "Knowledge Wiki revisit queue: unprotected long/working memories ordered by review priority.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max review candidates to return (1-100, default 12)" }
                }
            }
        }),
        json!({
            "name": "brain_wiki_digest_text",
            "description": "Digest pasted text into TerranSoul's Knowledge Wiki with source-hash dedup. Requires write capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "Text content to digest" },
                    "source_url": { "type": "string", "description": "Optional source URL or resource identifier" },
                    "tags": { "type": "string", "description": "Comma-separated tags (default: wiki:digest)" },
                    "importance": { "type": "integer", "description": "Importance score 1-5 (default 3)" }
                },
                "required": ["content"]
            }
        }),
        json!({
            "name": "brain_review_gaps",
            "description": "List recent memory gaps — queries where retrieval found no good match. Returns gaps with context snippet and timestamp.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max gaps to return (1-100, default 20)" },
                    "dismiss": { "type": "integer", "description": "Gap ID to dismiss (deletes the gap). If set, other params are ignored." }
                }
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
/// `app_state` is required for native code-intelligence tools because they
/// open the repo index under the app data directory; pass `None` when running
/// in transports that should expose brain tools only.
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
            let rerank = args["rerank"].as_bool().unwrap_or(true);
            let rerank_threshold = args["rerank_threshold"]
                .as_f64()
                .unwrap_or(crate::settings::DEFAULT_RERANK_THRESHOLD);
            let req = SearchRequest {
                query,
                limit,
                mode,
                rerank,
                rerank_threshold,
            };
            gw.search(caps, req)
                .await
                .map(|hits| serde_json::to_string(&hits).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        "brain_get_entry" => {
            let id = args["id"]
                .as_i64()
                .ok_or_else(|| "missing required param: id".to_string())?;
            gw.get_entry_detail(caps, id)
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
                query: args["query"].as_str().map(String::from),
                limit: args["limit"].as_u64().map(|n| n as usize),
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

        "brain_failover_status" => {
            let Some(state) = app_state else {
                return Err("failover status requires app state".to_string());
            };
            let rotator = state
                .provider_rotator
                .lock()
                .map_err(|e| format!("lock rotator: {e}"))?;
            let summary = rotator.failover_summary();
            serde_json::to_string(&summary).map_err(|e| e.to_string())
        }

        "brain_wiki_audit"
        | "brain_wiki_spotlight"
        | "brain_wiki_serendipity"
        | "brain_wiki_revisit"
        | "brain_wiki_digest_text" => dispatch_brain_wiki_tool(caps, tool_name, args, app_state),

        "brain_review_gaps" => {
            let app = app_state.ok_or("brain_review_gaps requires app state")?;
            let store = app.memory_store.lock().map_err(|e| e.to_string())?;

            // Dismiss mode.
            if let Some(gap_id) = args["dismiss"].as_i64() {
                let removed = crate::memory::gap_detection::dismiss_gap(&store, gap_id)
                    .map_err(|e| e.to_string())?;
                return serde_json::to_string(&serde_json::json!({ "dismissed": removed }))
                    .map_err(|e| e.to_string());
            }

            let limit = clamp_limit(args, 20, 100);
            let gaps = crate::memory::gap_detection::list_recent_gaps(&store, limit)
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&gaps).map_err(|e| e.to_string())
        }

        // ─── Code-intelligence tools (native symbol index) ────────────
        "code_query"
        | "code_context"
        | "code_impact"
        | "code_rename"
        | "code_generate_skills"
        | "code_list_groups"
        | "code_create_group"
        | "code_add_repo_to_group"
        | "code_group_status"
        | "code_extract_contracts"
        | "code_extract_negatives"
        | "code_list_group_contracts"
        | "code_cross_repo_query" => dispatch_code_tool(caps, tool_name, args, app_state).await,

        _ => Err(format!("unknown tool: {tool_name}")),
    }
}

fn clamp_limit(args: &Value, default: usize, max: usize) -> usize {
    args["limit"]
        .as_u64()
        .map(|n| n as usize)
        .unwrap_or(default)
        .clamp(1, max)
}

fn dispatch_brain_wiki_tool(
    caps: &GatewayCaps,
    tool_name: &str,
    args: &Value,
    app_state: Option<&AppState>,
) -> Result<String, String> {
    let state = app_state.ok_or_else(|| "brain wiki tools require app state".to_string())?;
    let store = state
        .memory_store
        .lock()
        .map_err(|error| format!("lock memory store: {error}"))?;

    match tool_name {
        "brain_wiki_audit" => {
            if !caps.brain_read {
                return Err(
                    "permission denied: capability `brain_read` is not granted to this client"
                        .into(),
                );
            }
            let cfg = AuditConfig {
                limit: clamp_limit(args, 50, 200),
                stale_threshold: args["stale_threshold"]
                    .as_f64()
                    .unwrap_or(0.20)
                    .clamp(0.0, 1.0),
            };
            let report = audit_report(&store, &cfg).map_err(|e| e.to_string())?;
            serde_json::to_string(&report).map_err(|e| e.to_string())
        }
        "brain_wiki_spotlight" => {
            if !caps.brain_read {
                return Err(
                    "permission denied: capability `brain_read` is not granted to this client"
                        .into(),
                );
            }
            let nodes = god_nodes(&store, clamp_limit(args, 10, 100)).map_err(|e| e.to_string())?;
            serde_json::to_string(&nodes).map_err(|e| e.to_string())
        }
        "brain_wiki_serendipity" => {
            if !caps.brain_read {
                return Err(
                    "permission denied: capability `brain_read` is not granted to this client"
                        .into(),
                );
            }
            let connections = surprising_connections(&store, clamp_limit(args, 10, 100))
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&connections).map_err(|e| e.to_string())
        }
        "brain_wiki_revisit" => {
            if !caps.brain_read {
                return Err(
                    "permission denied: capability `brain_read` is not granted to this client"
                        .into(),
                );
            }
            let queue = append_and_review_queue(&store, clamp_limit(args, 12, 100))
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&queue).map_err(|e| e.to_string())
        }
        "brain_wiki_digest_text" => {
            if !caps.brain_write {
                return Err(
                    "permission denied: capability `brain_write` is not granted to this client"
                        .into(),
                );
            }
            let content = args["content"]
                .as_str()
                .ok_or_else(|| "missing required param: content".to_string())?
                .trim();
            if content.is_empty() {
                return Err("content cannot be empty".into());
            }
            let tags = args["tags"].as_str().unwrap_or("wiki:digest");
            let result = ensure_source_dedup(
                &store,
                args["source_url"].as_str(),
                content,
                tags,
                args["importance"].as_i64().unwrap_or(3),
            )
            .map_err(|e| e.to_string())?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
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
            "description": "Search the code symbol index. Supports free-text hybrid search (BM25 + graph RRF), exact symbol name lookup, or file listing. Returns symbols with scores and related processes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Free-text search query (hybrid BM25 + graph RRF fusion)" },
                    "symbol": { "type": "string", "description": "Symbol name for exact match lookup" },
                    "file": { "type": "string", "description": "File path to list symbols in" },
                    "repo": { "type": "string", "description": "Repository path filter (defaults to first indexed repo)" },
                    "limit": { "type": "integer", "description": "Max results (default: 20)" },
                    "include_processes": { "type": "boolean", "description": "Include related execution processes in results (default: true)" }
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
            "description": "Compute blast-radius of changing a symbol or analyze a git diff. With 'symbol': BFS along incoming call edges grouped by depth. With 'diff': map changed lines to symbols and surface risk buckets (critical/high/moderate/low).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "symbol": { "type": "string", "description": "Symbol name to analyze impact for (omit if using diff mode)" },
                    "diff": { "type": "string", "description": "Git diff ref/range (e.g. 'HEAD~1', 'main..feature') — analyzes all changed symbols" },
                    "depth": { "type": "integer", "description": "Max BFS depth (default: 5)" },
                    "repo": { "type": "string", "description": "Repository path filter (defaults to first indexed repo)" }
                }
            }
        }),
        json!({
            "name": "code_rename",
            "description": "Rename a symbol across the codebase. Returns an edit plan with graph-resolved (high confidence), heritage (medium), and text-search (lower) edits. Includes file-grouped review payload and summary statistics. Use dry_run=true to preview without applying.",
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
        json!({
            "name": "code_generate_skills",
            "description": "Generate agent-compatible SKILL.md files from the code graph. Produces per-cluster and per-process skills with YAML frontmatter, symbol tables, and mermaid call graphs. Output is written to the specified directory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo": { "type": "string", "description": "Repository path (defaults to first indexed repo)" },
                    "output_dir": { "type": "string", "description": "Output directory for generated skills (defaults to <repo>/.generated-skills)" }
                }
            }
        }),
        // ─── Multi-repo groups and contracts (chunk 37.13) ──────────────
        json!({
            "name": "code_list_groups",
            "description": "List all repo groups with member counts.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "code_create_group",
            "description": "Create a new repo group with a unique label and optional description.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "label":       { "type": "string", "description": "Unique group label" },
                    "description": { "type": "string", "description": "Optional description" }
                },
                "required": ["label"]
            }
        }),
        json!({
            "name": "code_add_repo_to_group",
            "description": "Add an indexed repo to a group with an optional role tag (e.g., 'frontend', 'backend', 'shared'). Idempotent — re-adding updates the role.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "group_id": { "type": "integer" },
                    "repo_id":  { "type": "integer", "description": "Indexed repo id (use code_query to discover)" },
                    "role":     { "type": "string", "description": "Optional role tag" }
                },
                "required": ["group_id", "repo_id"]
            }
        }),
        json!({
            "name": "code_group_status",
            "description": "Aggregated status for a group: members, symbol/contract counts, stalest indexed timestamp.",
            "inputSchema": {
                "type": "object",
                "properties": { "group_id": { "type": "integer" } },
                "required": ["group_id"]
            }
        }),
        json!({
            "name": "code_extract_contracts",
            "description": "Extract public-API contracts from a repo's symbol surface. Captures top-level functions, structs, enums, traits, classes, interfaces, type aliases, and constants with stable signature hashes for change detection.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo":    { "type": "string",  "description": "Repository path (resolves to repo_id)" },
                    "repo_id": { "type": "integer", "description": "Or pass an indexed repo id directly" }
                }
            }
        }),
        json!({
            "name": "code_extract_negatives",
            "description": "Extract anti-pattern rules from rules/coding-standards.md and ingest them as negative memories with substring trigger patterns. Idempotent — existing rules are skipped.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "code_list_group_contracts",
            "description": "List all extracted contracts for the repos in a group.",
            "inputSchema": {
                "type": "object",
                "properties": { "group_id": { "type": "integer" } },
                "required": ["group_id"]
            }
        }),
        json!({
            "name": "code_cross_repo_query",
            "description": "Search for a symbol name across all repos in a group. Each match indicates which repo it lives in and whether it is part of that repo's contract surface.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "group_id": { "type": "integer" },
                    "name":     { "type": "string", "description": "Substring to match (case-insensitive LIKE)" },
                    "limit":    { "type": "integer", "description": "Max matches (1-1000, default 50)" }
                },
                "required": ["group_id", "name"]
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

    // ─── Group/contract tools — no repo_id resolution needed ────────────
    match tool_name {
        "code_list_groups" => {
            let groups =
                crate::coding::repo_groups::list_groups(&data_dir).map_err(|e| e.to_string())?;
            return serde_json::to_string(&groups).map_err(|e| e.to_string());
        }
        "code_create_group" => {
            let label = args["label"]
                .as_str()
                .ok_or_else(|| "missing required arg: label".to_string())?;
            let description = args["description"].as_str();
            let group = crate::coding::repo_groups::create_group(&data_dir, label, description)
                .map_err(|e| e.to_string())?;
            return serde_json::to_string(&group).map_err(|e| e.to_string());
        }
        "code_add_repo_to_group" => {
            let group_id = args["group_id"]
                .as_i64()
                .ok_or_else(|| "missing required arg: group_id".to_string())?;
            let repo_id = args["repo_id"]
                .as_i64()
                .ok_or_else(|| "missing required arg: repo_id".to_string())?;
            let role = args["role"].as_str();
            crate::coding::repo_groups::add_repo_to_group(&data_dir, group_id, repo_id, role)
                .map_err(|e| e.to_string())?;
            return Ok(serde_json::json!({ "ok": true }).to_string());
        }
        "code_group_status" => {
            let group_id = args["group_id"]
                .as_i64()
                .ok_or_else(|| "missing required arg: group_id".to_string())?;
            let status = crate::coding::repo_groups::group_status(&data_dir, group_id)
                .map_err(|e| e.to_string())?;
            return serde_json::to_string(&status).map_err(|e| e.to_string());
        }
        "code_extract_contracts" => {
            // Accept either an explicit repo_id or a `repo` path.
            let repo_id = if let Some(rid) = args["repo_id"].as_i64() {
                rid
            } else if let Some(repo_path) = args["repo"].as_str() {
                conn.query_row(
                    "SELECT id FROM code_repos WHERE path = ?1",
                    rusqlite::params![repo_path],
                    |r| r.get(0),
                )
                .map_err(|_| format!("repo not indexed: {repo_path}"))?
            } else {
                return Err("missing required arg: repo or repo_id".into());
            };
            let result = crate::coding::repo_groups::extract_contracts(&data_dir, repo_id)
                .map_err(|e| e.to_string())?;
            return serde_json::to_string(&result).map_err(|e| e.to_string());
        }
        "code_extract_negatives" => {
            let app = app_state.ok_or("code_extract_negatives requires app state")?;
            let repo_root = crate::coding::repo::guess_repo_root(&app.data_dir);
            let path = repo_root.join("rules/coding-standards.md");
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("read coding-standards.md: {e}"))?;
            let negatives = crate::commands::coding::extract_negative_lines(&text);
            let store = app.memory_store.lock().map_err(|e| e.to_string())?;

            let mut created = 0usize;
            let mut skipped = 0usize;
            let rule_texts: Vec<String> = negatives.iter().map(|(r, _)| r.clone()).collect();

            for (rule, triggers) in &negatives {
                let exists: bool = store
                    .conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM memories WHERE content = ?1 AND valid_to IS NULL)",
                        rusqlite::params![rule],
                        |row| row.get(0),
                    )
                    .map_err(|e| e.to_string())?;
                if exists {
                    skipped += 1;
                    continue;
                }
                let entry = store
                    .add(crate::memory::store::NewMemory {
                        content: rule.clone(),
                        tags: "negative,coding-standard,auto-extracted".to_string(),
                        importance: 4,
                        memory_type: crate::memory::store::MemoryType::Fact,
                        ..Default::default()
                    })
                    .map_err(|e| e.to_string())?;
                store
                    .conn
                    .execute(
                        "UPDATE memories SET cognitive_kind = 'negative' WHERE id = ?1",
                        rusqlite::params![entry.id],
                    )
                    .map_err(|e| e.to_string())?;
                for trigger in triggers {
                    crate::memory::negative::add_trigger(&store, entry.id, trigger, "substring")
                        .map_err(|e| e.to_string())?;
                }
                created += 1;
            }

            let result = serde_json::json!({
                "created": created,
                "skipped": skipped,
                "rules": rule_texts,
            });
            return serde_json::to_string(&result).map_err(|e| e.to_string());
        }
        "code_list_group_contracts" => {
            let group_id = args["group_id"]
                .as_i64()
                .ok_or_else(|| "missing required arg: group_id".to_string())?;
            let contracts = crate::coding::repo_groups::list_group_contracts(&data_dir, group_id)
                .map_err(|e| e.to_string())?;
            return serde_json::to_string(&contracts).map_err(|e| e.to_string());
        }
        "code_cross_repo_query" => {
            let group_id = args["group_id"]
                .as_i64()
                .ok_or_else(|| "missing required arg: group_id".to_string())?;
            let name = args["name"]
                .as_str()
                .ok_or_else(|| "missing required arg: name".to_string())?;
            let limit = args["limit"].as_u64().unwrap_or(50) as usize;
            let matches =
                crate::coding::repo_groups::cross_repo_query(&data_dir, group_id, name, limit)
                    .map_err(|e| e.to_string())?;
            return serde_json::to_string(&matches).map_err(|e| e.to_string());
        }
        _ => {}
    }

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

            // Free-text hybrid search (BM25 + vector + graph RRF).
            if let Some(query_text) = args["query"].as_str() {
                let search_results = crate::coding::code_search::hybrid_code_search_by_repo(
                    &conn, repo_id, query_text, None, limit,
                )
                .map_err(|e| e.to_string())?;
                let response = serde_json::json!({
                    "results": search_results,
                    "mode": "hybrid_rrf",
                });
                return serde_json::to_string(&response).map_err(|e| e.to_string());
            }

            let results = if let Some(symbol_name) = args["symbol"].as_str() {
                crate::coding::symbol_index::query_symbols_by_name(&conn, repo_id, symbol_name)
                    .map_err(|e| e.to_string())?
            } else if let Some(file_path) = args["file"].as_str() {
                crate::coding::symbol_index::query_symbols_in_file(&conn, repo_id, file_path)
                    .map_err(|e| e.to_string())?
            } else {
                return Err("provide `query`, `symbol`, or `file` parameter".into());
            };

            let truncated: Vec<_> = results.into_iter().take(limit).collect();

            // Enrich with process context if clusters exist.
            let process_context = if args["include_processes"].as_bool().unwrap_or(true) {
                let processes =
                    crate::coding::processes::list_processes(&conn, repo_id).unwrap_or_default();
                if processes.is_empty() {
                    None
                } else {
                    // Find processes containing any of the returned symbols.
                    let sym_names: Vec<&str> = truncated.iter().map(|s| s.name.as_str()).collect();
                    let relevant: Vec<_> = processes
                        .into_iter()
                        .filter(|p| {
                            sym_names.contains(&p.entry_point.as_str())
                                || p.steps.iter().any(|s| sym_names.contains(&s.name.as_str()))
                        })
                        .take(5)
                        .collect();
                    if relevant.is_empty() {
                        None
                    } else {
                        Some(relevant)
                    }
                }
            } else {
                None
            };

            let response = serde_json::json!({
                "symbols": truncated,
                "processes": process_context,
            });
            serde_json::to_string(&response).map_err(|e| e.to_string())
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
            let max_depth = args["depth"].as_u64().unwrap_or(5) as u32;

            // Diff mode: analyze all changed symbols from a git ref/range
            if let Some(diff_ref) = args["diff"].as_str() {
                let repo_path: String = conn
                    .query_row(
                        "SELECT path FROM code_repos WHERE id = ?1",
                        rusqlite::params![repo_id],
                        |r| r.get(0),
                    )
                    .map_err(|e| format!("cannot find repo path: {e}"))?;

                let report = crate::coding::diff_impact::analyze_diff_impact(
                    &data_dir,
                    std::path::Path::new(&repo_path),
                    diff_ref,
                    max_depth,
                )
                .map_err(|e| e.to_string())?;

                return serde_json::to_string(&report).map_err(|e| e.to_string());
            }

            // Symbol mode: BFS callers for a single symbol
            let symbol_name = args["symbol"]
                .as_str()
                .ok_or_else(|| "missing required param: symbol or diff".to_string())?;
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

        "code_generate_skills" => {
            let repo_path: String = conn
                .query_row(
                    "SELECT path FROM code_repos WHERE id = ?1",
                    rusqlite::params![repo_id],
                    |r| r.get(0),
                )
                .map_err(|e| format!("cannot find repo path: {e}"))?;

            let output_dir = if let Some(dir) = args["output_dir"].as_str() {
                std::path::PathBuf::from(dir)
            } else {
                std::path::Path::new(&repo_path).join(".generated-skills")
            };

            let result = crate::coding::skills::generate_skills(
                &data_dir,
                std::path::Path::new(&repo_path),
                &output_dir,
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
        json!({
            "uri": "terransoul://schema",
            "name": "Code Index Schema",
            "description": "Summary of indexed symbol kinds, edge kinds, and counts for the default repo.",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "terransoul://context",
            "name": "Repo Context (LLM-ready)",
            "description": "Full repository context pack: top clusters, key entry points, file tree summary. Suitable for injecting into an LLM system prompt.",
            "mimeType": "text/plain"
        }),
        json!({
            "uri": "terransoul://setup",
            "name": "Editor Setup Config",
            "description": "Ready-to-use VS Code MCP configuration JSON for connecting to this TerranSoul instance.",
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

        "terransoul://schema" => {
            let repo_id: i64 = conn
                .query_row(
                    "SELECT id FROM code_repos ORDER BY indexed_at DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| "no repos indexed".to_string())?;

            // Aggregate symbol kinds
            let mut kind_stmt = conn
                .prepare(
                    "SELECT kind, COUNT(*) FROM code_symbols WHERE repo_id = ?1 GROUP BY kind ORDER BY COUNT(*) DESC",
                )
                .map_err(|e| e.to_string())?;
            let symbol_kinds: Vec<Value> = kind_stmt
                .query_map(rusqlite::params![repo_id], |row| {
                    Ok(json!({
                        "kind": row.get::<_, String>(0)?,
                        "count": row.get::<_, i64>(1)?
                    }))
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            // Aggregate edge kinds
            let mut edge_stmt = conn
                .prepare(
                    "SELECT kind, COUNT(*) FROM code_edges WHERE repo_id = ?1 GROUP BY kind ORDER BY COUNT(*) DESC",
                )
                .map_err(|e| e.to_string())?;
            let edge_kinds: Vec<Value> = edge_stmt
                .query_map(rusqlite::params![repo_id], |row| {
                    Ok(json!({
                        "kind": row.get::<_, String>(0)?,
                        "count": row.get::<_, i64>(1)?
                    }))
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            let total_symbols: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM code_symbols WHERE repo_id = ?1",
                    rusqlite::params![repo_id],
                    |r| r.get(0),
                )
                .unwrap_or(0);

            let total_edges: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM code_edges WHERE repo_id = ?1",
                    rusqlite::params![repo_id],
                    |r| r.get(0),
                )
                .unwrap_or(0);

            Ok(json!({
                "repo_id": repo_id,
                "total_symbols": total_symbols,
                "total_edges": total_edges,
                "symbol_kinds": symbol_kinds,
                "edge_kinds": edge_kinds
            }))
        }

        "terransoul://context" => {
            let repo_id: i64 = conn
                .query_row(
                    "SELECT id FROM code_repos ORDER BY indexed_at DESC LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| "no repos indexed".to_string())?;

            let repo_label: String = conn
                .query_row(
                    "SELECT label FROM code_repos WHERE id = ?1",
                    rusqlite::params![repo_id],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| "unknown".to_string());

            let clusters =
                crate::coding::processes::list_clusters(&conn, repo_id).unwrap_or_default();
            let processes =
                crate::coding::processes::list_processes(&conn, repo_id).unwrap_or_default();

            // Build LLM-ready context text
            let mut ctx = format!("# Repository: {repo_label}\n\n");

            ctx.push_str("## Functional Clusters\n");
            for c in &clusters {
                ctx.push_str(&format!("- **{}** ({} symbols)\n", c.label, c.size));
            }

            ctx.push_str("\n## Key Execution Flows\n");
            for p in processes.iter().take(15) {
                let steps: Vec<&str> = p.steps.iter().take(6).map(|s| s.name.as_str()).collect();
                ctx.push_str(&format!("- `{}` → {}\n", p.entry_point, steps.join(" → ")));
            }

            // Top exported symbols
            let mut export_stmt = conn
                .prepare(
                    "SELECT name, kind, file FROM code_symbols \
                     WHERE repo_id = ?1 AND exported = 1 \
                     ORDER BY name LIMIT 30",
                )
                .map_err(|e| e.to_string())?;
            let exports: Vec<(String, String, String)> = export_stmt
                .query_map(rusqlite::params![repo_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            if !exports.is_empty() {
                ctx.push_str("\n## Public API (exported symbols)\n");
                for (name, kind, file) in &exports {
                    ctx.push_str(&format!("- `{name}` ({kind}) in {file}\n"));
                }
            }

            Ok(json!({ "context": ctx }))
        }

        "terransoul://setup" => {
            let setup = generate_editor_setup(state);
            Ok(setup)
        }

        _ => Err(format!("unknown resource URI: {uri}")),
    }
}

// ─── Editor Setup Writer ────────────────────────────────────────────────────

/// Generate VS Code MCP configuration for this TerranSoul instance.
fn generate_editor_setup(state: &AppState) -> Value {
    let token_path = state.data_dir.join("mcp-token.txt");
    let token_hint = if token_path.exists() {
        "token file exists — use content as bearer token"
    } else {
        "token file not found — MCP may not be running"
    };

    // Detect which port we're likely running on
    let port = std::env::var("TERRANSOUL_MCP_PORT").unwrap_or_else(|_| "7421".to_string());

    json!({
        "description": "VS Code MCP configuration for TerranSoul brain server",
        "instructions": "Copy the 'servers' object below into your .vscode/mcp.json file.",
        "token_path": token_path.to_string_lossy(),
        "token_hint": token_hint,
        "servers": {
            "terransoul-brain": {
                "type": "http",
                "url": format!("http://127.0.0.1:{port}/mcp"),
                "headers": {
                    "Authorization": "Bearer ${input:terransoul-token}"
                }
            }
        },
        "inputs": [
            {
                "id": "terransoul-token",
                "type": "promptString",
                "description": "TerranSoul MCP bearer token",
                "password": true
            }
        ],
        "write_path": ".vscode/mcp.json"
    })
}

/// Write the editor setup config to the specified repo path.
/// Called by the `code_setup_writer` tool if invoked.
pub fn write_editor_setup(repo_path: &std::path::Path, state: &AppState) -> Result<String, String> {
    let setup = generate_editor_setup(state);

    let vscode_dir = repo_path.join(".vscode");
    std::fs::create_dir_all(&vscode_dir).map_err(|e| format!("cannot create .vscode/: {e}"))?;

    let mcp_path = vscode_dir.join("mcp.json");

    // Read existing config or start fresh
    let mut config: Value = if mcp_path.exists() {
        let content =
            std::fs::read_to_string(&mcp_path).map_err(|e| format!("cannot read mcp.json: {e}"))?;
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    // Merge our server entry
    let servers = config
        .as_object_mut()
        .ok_or_else(|| "mcp.json root is not an object".to_string())?
        .entry("servers")
        .or_insert_with(|| json!({}));

    if let Some(servers_obj) = servers.as_object_mut() {
        servers_obj.insert(
            "terransoul-brain".to_string(),
            setup["servers"]["terransoul-brain"].clone(),
        );
    }

    // Merge inputs
    let inputs = config
        .as_object_mut()
        .unwrap()
        .entry("inputs")
        .or_insert_with(|| json!([]));

    if let Some(inputs_arr) = inputs.as_array_mut() {
        let has_token_input = inputs_arr
            .iter()
            .any(|i| i["id"].as_str() == Some("terransoul-token"));
        if !has_token_input {
            inputs_arr.push(setup["inputs"][0].clone());
        }
    }

    let output = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("cannot serialize mcp.json: {e}"))?;
    std::fs::write(&mcp_path, &output).map_err(|e| format!("cannot write mcp.json: {e}"))?;

    Ok(format!("Written to {}", mcp_path.display()))
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
        json!({
            "name": "guided_impact",
            "description": "Analyze a git diff for risk: maps changed lines to symbols, classifies by blast-radius, and suggests review priorities.",
            "arguments": [
                {
                    "name": "diff_ref",
                    "description": "Git ref or range (e.g. 'HEAD~1', 'main..feature'). Defaults to 'HEAD~1'.",
                    "required": false
                },
                {
                    "name": "repo",
                    "description": "Repository path (optional)",
                    "required": false
                }
            ]
        }),
        json!({
            "name": "explore_cluster",
            "description": "Deep-dive into a specific functional cluster: its symbols, internal edges, public API, and connected clusters.",
            "arguments": [
                {
                    "name": "cluster",
                    "description": "Cluster label or ID to explore",
                    "required": true
                },
                {
                    "name": "repo",
                    "description": "Repository path (optional)",
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

        "guided_impact" => {
            let diff_ref = args["diff_ref"].as_str().unwrap_or("HEAD~1");

            // Get repo path for git operations
            let repo_path: String = conn
                .query_row(
                    "SELECT path FROM code_repos WHERE id = ?1",
                    rusqlite::params![repo_id],
                    |r| r.get(0),
                )
                .map_err(|e| format!("cannot find repo path: {e}"))?;

            let report = crate::coding::diff_impact::analyze_diff_impact(
                data_dir,
                std::path::Path::new(&repo_path),
                diff_ref,
                5,
            )
            .map_err(|e| e.to_string())?;

            let mut impacts_desc = String::new();
            for impact in &report.impacts {
                let risk_label = match impact.risk {
                    crate::coding::diff_impact::RiskLevel::Critical => "🔴 CRITICAL",
                    crate::coding::diff_impact::RiskLevel::High => "🟠 HIGH",
                    crate::coding::diff_impact::RiskLevel::Moderate => "🟡 MODERATE",
                    crate::coding::diff_impact::RiskLevel::Low => "🟢 LOW",
                };
                impacts_desc.push_str(&format!(
                    "- `{}` ({}) in {} — {risk_label} ({} affected)\n",
                    impact.symbol.name,
                    impact.symbol.kind,
                    impact.symbol.file,
                    impact.affected_count
                ));
            }

            let prompt_text = format!(
                "Review the impact of the latest code changes (diff ref: `{diff_ref}`).\n\n\
                 ## Summary\n\
                 - Files changed: {}\n\
                 - Symbols directly modified: {}\n\
                 - Total transitively affected: {}\n\n\
                 ## Risk Breakdown\n\
                 - Critical: {}\n\
                 - High: {}\n\
                 - Moderate: {}\n\
                 - Low: {}\n\n\
                 ## Changed Symbols (by risk):\n{impacts_desc}\n\
                 Please provide:\n\
                 1. Which changes carry the most risk and why\n\
                 2. Recommended review order (highest risk first)\n\
                 3. Suggested test coverage for the affected paths\n\
                 4. Any potential breaking changes for downstream consumers",
                report.files_changed,
                report.symbols_changed,
                report.total_affected,
                report.risk_summary.critical,
                report.risk_summary.high,
                report.risk_summary.moderate,
                report.risk_summary.low,
            );

            Ok(json!({
                "messages": [
                    { "role": "user", "content": { "type": "text", "text": prompt_text } }
                ]
            }))
        }

        "explore_cluster" => {
            let cluster_label = args["cluster"]
                .as_str()
                .ok_or_else(|| "missing required argument: cluster".to_string())?;

            let clusters =
                crate::coding::processes::list_clusters(&conn, repo_id).unwrap_or_default();

            let target = clusters
                .iter()
                .find(|c| {
                    c.label.eq_ignore_ascii_case(cluster_label) || c.id.to_string() == cluster_label
                })
                .ok_or_else(|| format!("cluster not found: {cluster_label}"))?;

            // Get symbols in this cluster via cluster_members join
            let mut sym_stmt = conn
                .prepare(
                    "SELECT s.name, s.kind, s.file, s.line, s.exported \
                     FROM code_symbols s \
                     JOIN code_cluster_members m ON m.symbol_id = s.id AND m.repo_id = s.repo_id \
                     WHERE s.repo_id = ?1 AND m.cluster_id = ?2 \
                     ORDER BY s.file, s.line",
                )
                .map_err(|e| e.to_string())?;
            let symbols: Vec<(String, String, String, u32, bool)> = sym_stmt
                .query_map(rusqlite::params![repo_id, target.id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, u32>(3)?,
                        row.get::<_, bool>(4)?,
                    ))
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();

            let public_api: Vec<_> = symbols.iter().filter(|s| s.4).collect();
            let internal: Vec<_> = symbols.iter().filter(|s| !s.4).collect();

            let mut prompt_text = format!(
                "Explore the **{}** cluster ({} symbols total).\n\n",
                target.label, target.size
            );

            prompt_text.push_str("## Public API (exported):\n");
            for (name, kind, file, line, _) in &public_api {
                prompt_text.push_str(&format!("- `{name}` ({kind}) — {file}:{line}\n"));
            }

            prompt_text.push_str(&format!("\n## Internal ({} symbols):\n", internal.len()));
            for (name, kind, file, line, _) in internal.iter().take(20) {
                prompt_text.push_str(&format!("- `{name}` ({kind}) — {file}:{line}\n"));
            }
            if internal.len() > 20 {
                prompt_text.push_str(&format!("  ... and {} more\n", internal.len() - 20));
            }

            prompt_text.push_str(
                "\nPlease analyze:\n\
                 1. What is this cluster's single responsibility?\n\
                 2. Is the public API surface appropriate (too wide/narrow)?\n\
                 3. Are there any symbols that seem misplaced (should belong elsewhere)?\n\
                 4. What are the key data flows within this cluster?",
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
        assert_eq!(defs.len(), 15);
    }

    #[test]
    fn definitions_has_21_tools_with_code_read() {
        let caps = GatewayCaps::READ_WRITE;
        let defs = definitions(&caps);
        assert_eq!(defs.len(), 28);
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
            "brain_failover_status",
            "brain_wiki_audit",
            "brain_wiki_spotlight",
            "brain_wiki_serendipity",
            "brain_wiki_revisit",
            "brain_wiki_digest_text",
            "brain_review_gaps",
        ];
        assert_eq!(names, expected);
    }

    #[test]
    fn code_tool_names_are_correct() {
        let caps = GatewayCaps::READ_WRITE;
        let defs = definitions(&caps);
        let code_names: Vec<&str> = defs[15..]
            .iter()
            .map(|d| d["name"].as_str().unwrap())
            .collect();
        let expected = [
            "code_query",
            "code_context",
            "code_impact",
            "code_rename",
            "code_generate_skills",
            "code_list_groups",
            "code_create_group",
            "code_add_repo_to_group",
            "code_group_status",
            "code_extract_contracts",
            "code_extract_negatives",
            "code_list_group_contracts",
            "code_cross_repo_query",
        ];
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
    fn resource_definitions_has_6_entries() {
        let defs = resource_definitions(None);
        assert_eq!(defs.len(), 6);
        let uris: Vec<&str> = defs.iter().map(|d| d["uri"].as_str().unwrap()).collect();
        assert_eq!(
            uris,
            [
                "terransoul://repos",
                "terransoul://clusters",
                "terransoul://processes",
                "terransoul://schema",
                "terransoul://context",
                "terransoul://setup",
            ]
        );
    }

    #[test]
    fn prompt_definitions_has_4_entries() {
        let defs = prompt_definitions();
        assert_eq!(defs.len(), 4);
        let names: Vec<&str> = defs.iter().map(|d| d["name"].as_str().unwrap()).collect();
        assert_eq!(
            names,
            [
                "detect_impact",
                "generate_map",
                "guided_impact",
                "explore_cluster"
            ]
        );
    }
}
