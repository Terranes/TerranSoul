//! MCP JSON-RPC 2.0 router over HTTP (Streamable HTTP transport).
//!
//! Implements the MCP 2025-11-05 protocol surface on axum:
//! - `POST /mcp` — JSON-RPC 2.0 requests (initialize, tools/list, tools/call, ping)
//! - Notifications (no `id` field) return 202 Accepted
//! - Bearer-token authentication on every request
//!
//! Reference: <https://modelcontextprotocol.io/specification/2025-11-05/basic/transports#streamable-http>

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::ai_integrations::gateway::*;
use crate::AppState;

use super::activity::McpActivityReporter;
use super::compliance_gate::{self, ComplianceState};
use super::hooks::IndexStalenessTracker;
use super::tools;

// ─── Shared state for the axum router ───────────────────────────────────────

#[derive(Clone)]
pub struct McpRouterState {
    pub gw: Arc<dyn BrainGateway>,
    pub caps: GatewayCaps,
    pub token: String,
    pub lan_public_read_only: bool,
    pub port: u16,
    pub seed_loaded: bool,
    pub activity: Option<McpActivityReporter>,
    /// Full app state for native code-intelligence tools that need the code index.
    pub app_state: Option<AppState>,
    /// Tracks git HEAD for staleness detection on post-tool-use hooks.
    pub staleness_tracker: Arc<Mutex<IndexStalenessTracker>>,
    /// Per-session compliance tracking — enforces MCP preflight + milestone hygiene.
    pub compliance: ComplianceState,
}

// ─── JSON-RPC 2.0 types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Serialize)]
pub(crate) struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Serialize)]
pub(crate) struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

impl JsonRpcResponse {
    pub(crate) fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub(crate) fn err(id: Value, code: i64, message: String) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

/// Dispatch a JSON-RPC method/params pair to the brain gateway.
/// Shared between the HTTP transport ([`handle_request`]) and the
/// stdio transport ([`super::stdio`]).
///
/// Authentication is the caller's responsibility — this function does
/// **not** check tokens. HTTP enforces the bearer header before
/// calling; stdio runs in a trusted parent-child relationship and
/// skips auth by design (canonical MCP stdio behaviour).
pub(crate) async fn dispatch_method(
    gw: &dyn BrainGateway,
    caps: &GatewayCaps,
    method: &str,
    params: Value,
    id: Value,
) -> JsonRpcResponse {
    dispatch_method_with_state(gw, caps, method, params, id, None).await
}

/// Extended dispatch that accepts an optional `AppState` for code tools.
pub(crate) async fn dispatch_method_with_state(
    gw: &dyn BrainGateway,
    caps: &GatewayCaps,
    method: &str,
    params: Value,
    id: Value,
    app_state: Option<&AppState>,
) -> JsonRpcResponse {
    match method {
        "initialize" => {
            let (build_mode, server_name) = if super::is_mcp_pet_mode() {
                ("mcp", "terransoul-brain-mcp")
            } else if super::is_dev_build() {
                ("dev", "terransoul-brain-dev")
            } else {
                ("release", "terransoul-brain")
            };
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "prompts": {}
                },
                "serverInfo": {
                    "name": server_name,
                    "version": env!("CARGO_PKG_VERSION"),
                    "buildMode": build_mode
                }
            });
            JsonRpcResponse::ok(id, result)
        }

        "tools/list" => {
            let result = json!({ "tools": tools::definitions(caps) });
            JsonRpcResponse::ok(id, result)
        }

        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));

            match tools::dispatch(gw, caps, name, &args, app_state).await {
                Ok(text) => {
                    let result = json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    });
                    JsonRpcResponse::ok(id, result)
                }
                Err(e) => {
                    let result = json!({
                        "content": [{ "type": "text", "text": e }],
                        "isError": true
                    });
                    JsonRpcResponse::ok(id, result)
                }
            }
        }

        "resources/list" => {
            let result = json!({ "resources": tools::resource_definitions(app_state) });
            JsonRpcResponse::ok(id, result)
        }

        "resources/read" => {
            let uri = params["uri"].as_str().unwrap_or("");
            match tools::read_resource(uri, app_state) {
                Ok(contents) => {
                    let text = serde_json::to_string(&contents).unwrap_or_default();
                    let result = json!({
                        "contents": [{ "uri": uri, "mimeType": "application/json", "text": text }]
                    });
                    JsonRpcResponse::ok(id, result)
                }
                Err(e) => JsonRpcResponse::err(id, -32602, e),
            }
        }

        "prompts/list" => {
            let result = json!({ "prompts": tools::prompt_definitions() });
            JsonRpcResponse::ok(id, result)
        }

        "prompts/get" => {
            let name = params["name"].as_str().unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));
            match tools::get_prompt(name, &args, app_state) {
                Ok(messages) => JsonRpcResponse::ok(id, messages),
                Err(e) => JsonRpcResponse::err(id, -32602, e),
            }
        }

        "ping" => JsonRpcResponse::ok(id, json!({})),

        _ => JsonRpcResponse::err(id, -32601, format!("method not found: {method}")),
    }
}

// ─── Router construction ────────────────────────────────────────────────────

/// Build the axum router for the MCP Streamable HTTP transport.
pub fn build(state: McpRouterState) -> Router {
    Router::new()
        .route("/mcp", post(handle_request))
        .route("/health", get(handle_health))
        .route("/hooks/pre_tool", post(handle_pre_tool_hook))
        .route("/hooks/post_tool", post(handle_post_tool_hook))
        .route("/status", get(handle_status))
        .with_state(state)
}

// ─── Request handler ────────────────────────────────────────────────────────

async fn handle_request(
    State(state): State<McpRouterState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    // Parse JSON body.
    let req: JsonRpcRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            let resp = JsonRpcResponse::err(Value::Null, -32700, format!("parse error: {e}"));
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    let response_id = req.id.clone().unwrap_or(Value::Null);

    let authed = validate_auth(&headers, &state.token);
    let public_allowed = state.lan_public_read_only && is_public_read_only_request(&req);

    // Bearer-token authentication. Public LAN mode may expose a restricted,
    // read-only subset without the token.
    if !authed && !public_allowed {
        let resp = JsonRpcResponse::err(response_id, -32000, "unauthorized".into());
        return (StatusCode::UNAUTHORIZED, Json(resp)).into_response();
    }

    // Notifications (no id) — dispatch to hook handlers, then acknowledge.
    if req.id.is_none() {
        let params = req.params.unwrap_or(Value::Null);

        // Handle compliance signals before general notifications.
        if req.method == "compliance/signal" {
            if let Some(signal) = params.get("signal").and_then(|v| v.as_str()) {
                let compliance = state.compliance.clone();
                let signal_owned = signal.to_string();
                tokio::spawn(async move {
                    compliance_gate::on_compliance_signal(&compliance, &signal_owned).await;
                });
            }
            return StatusCode::ACCEPTED.into_response();
        }

        // Fire-and-forget notification handling
        super::hooks::handle_notification(
            &req.method,
            &params,
            state.app_state.as_ref(),
            &state.staleness_tracker,
        );
        return StatusCode::ACCEPTED.into_response();
    }

    let id = req.id.unwrap_or(Value::Null);

    let params = req.params.unwrap_or(Value::Null);
    let tool_activity = if req.method == "tools/call" {
        let tool_name = params["name"].as_str().unwrap_or("").to_string();
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or(Value::Object(Default::default()));
        if !tool_name.is_empty() {
            if let Some(activity) = &state.activity {
                activity.tool_started(&tool_name, &args);
            }
            Some((tool_name, args, state.activity.clone()))
        } else {
            None
        }
    } else {
        None
    };

    let effective_caps = if authed {
        state.caps
    } else {
        GatewayCaps::default()
    };

    let mut resp = dispatch_method_with_state(
        state.gw.as_ref(),
        &effective_caps,
        &req.method,
        params,
        id,
        state.app_state.as_ref(),
    )
    .await;

    // ─── Compliance gate: track tool calls + annotate responses ──────────
    if let Some((ref tool_name, ref args, _)) = tool_activity {
        // Handle signal param on brain_session_checklist
        if tool_name == "brain_session_checklist" {
            if let Some(signal) = args.get("signal").and_then(|v| v.as_str()) {
                compliance_gate::on_compliance_signal(&state.compliance, signal).await;
            }
            // Always replace the response with the full checklist
            let checklist = compliance_gate::get_checklist(&state.compliance).await;
            resp = JsonRpcResponse::ok(
                resp.id.clone(),
                json!({
                    "content": [{ "type": "text", "text": checklist }]
                }),
            );
        } else {
            compliance_gate::on_tool_called(&state.compliance, tool_name, args).await;

            // Annotate successful tool responses with compliance reminders
            if resp.error.is_none() {
                if let Some(annotation) =
                    compliance_gate::get_response_annotation(&state.compliance).await
                {
                    // Append compliance reminder to the text content of the tool result
                    if let Some(result) = resp.result.as_mut() {
                        if let Some(content) = result.get_mut("content") {
                            if let Some(arr) = content.as_array_mut() {
                                arr.push(json!({
                                    "type": "text",
                                    "text": annotation
                                }));
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some((tool_name, _args, Some(activity))) = tool_activity {
        let is_error = resp
            .result
            .as_ref()
            .and_then(|result| result.get("isError"))
            .and_then(Value::as_bool)
            .unwrap_or(resp.error.is_some());
        if is_error {
            let message = resp
                .result
                .as_ref()
                .and_then(|result| result.get("content"))
                .and_then(Value::as_array)
                .and_then(|content| content.first())
                .and_then(|item| item.get("text"))
                .and_then(Value::as_str)
                .or_else(|| resp.error.as_ref().map(|error| error.message.as_str()))
                .unwrap_or("unknown MCP tool error");
            activity.tool_failed(&tool_name, message);
        } else {
            activity.tool_finished(&tool_name);
        }
    }

    (StatusCode::OK, Json(resp)).into_response()
}

// ─── Health endpoint (no auth required) ─────────────────────────────────────

/// Unauthenticated health check for agent auto-discovery.
///
/// Returns `200 OK` with basic brain status so agents (and the tray
/// status page) can verify the server is running without needing the
/// bearer token first. No API keys or sensitive data are exposed.
///
/// Includes `Access-Control-Allow-Origin: *` so the built-in tray status
/// page (served from `about:blank`, origin `null`) can fetch it.
async fn handle_health(State(state): State<McpRouterState>) -> Response {
    let metrics = crate::memory::metrics::METRICS.snapshot();
    let body = match state.gw.health(&state.caps).await {
        Ok(h) => json!({
            "status": "ok",
            "port": state.port,
            "brain_provider": h.brain_provider,
            "brain_model": h.brain_model,
            "rag_quality_pct": h.rag_quality_pct,
            "memory_total": h.memory_total,
            "rag_quality": h.rag_quality,
            "memory": h.memory,
            "descriptions": h.descriptions,
            "metrics": metrics,
        }),
        Err(_) => json!({
            "status": "ok",
            "port": state.port,
        }),
    };

    (
        StatusCode::OK,
        [(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")],
        Json(body),
    )
        .into_response()
}

// ─── Status endpoint (auth required) ────────────────────────────────────────

/// `GET /status` — bearer-authenticated live snapshot of the running
/// MCP server. Lets the user (or any agent) monitor RAG/memory health
/// without speaking JSON-RPC.
async fn handle_status(State(state): State<McpRouterState>, headers: HeaderMap) -> Response {
    if !validate_auth(&headers, &state.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }

    let (build_mode, server_name) = if super::is_mcp_pet_mode() {
        ("mcp", "terransoul-brain-mcp")
    } else if super::is_dev_build() {
        ("dev", "terransoul-brain-dev")
    } else {
        ("release", "terransoul-brain")
    };

    let health = match state.gw.health(&state.caps).await {
        Ok(h) => json!({
            "version": h.version,
            "brain_provider": h.brain_provider,
            "brain_model": h.brain_model,
            "rag_quality_pct": h.rag_quality_pct,
            "memory_total": h.memory_total,
            "rag_quality": h.rag_quality,
            "memory": h.memory,
            "descriptions": h.descriptions,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    };

    let body = json!({
        "name": server_name,
        "version": env!("CARGO_PKG_VERSION"),
        "buildMode": build_mode,
        "petMode": super::is_mcp_pet_mode(),
        "actual_port": state.port,
        "seed_loaded": state.seed_loaded,
        "health": health,
    });

    (StatusCode::OK, Json(body)).into_response()
}

// ─── Hook endpoints ─────────────────────────────────────────────────────────

async fn handle_pre_tool_hook(
    State(state): State<McpRouterState>,
    headers: HeaderMap,
    Json(req): Json<super::hooks::PreToolUseRequest>,
) -> Response {
    if !validate_auth(&headers, &state.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    let resp = super::hooks::handle_pre_tool_use(&req, state.app_state.as_ref());
    (StatusCode::OK, Json(json!(resp))).into_response()
}

async fn handle_post_tool_hook(
    State(state): State<McpRouterState>,
    headers: HeaderMap,
    Json(req): Json<super::hooks::PostToolUseRequest>,
) -> Response {
    if !validate_auth(&headers, &state.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    let mut tracker = state.staleness_tracker.lock().await;
    let resp = super::hooks::handle_post_tool_use(&req, &mut tracker, state.app_state.as_ref());
    (StatusCode::OK, Json(json!(resp))).into_response()
}

/// Validate the `Authorization: Bearer <token>` header.
fn validate_auth(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|t| t == expected)
}

fn is_public_read_only_request(req: &JsonRpcRequest) -> bool {
    match req.method.as_str() {
        "initialize" | "ping" => true,
        "tools/list" => true,
        "tools/call" => req
            .params
            .as_ref()
            .and_then(|params| params.get("name"))
            .and_then(Value::as_str)
            .is_some_and(is_public_tool_name),
        _ => false,
    }
}

fn is_public_tool_name(name: &str) -> bool {
    matches!(
        name,
        "brain_search"
            | "brain_get_entry"
            | "brain_list_recent"
            | "brain_kg_neighbors"
            | "brain_summarize"
            | "brain_suggest_context"
            | "brain_health"
            | "brain_failover_status"
            | "brain_wiki_audit"
            | "brain_wiki_spotlight"
            | "brain_wiki_serendipity"
            | "brain_wiki_revisit"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_auth_accepts_correct_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer abc123".parse().unwrap());
        assert!(validate_auth(&headers, "abc123"));
    }

    #[test]
    fn validate_auth_rejects_wrong_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer wrong".parse().unwrap());
        assert!(!validate_auth(&headers, "abc123"));
    }

    #[test]
    fn validate_auth_rejects_missing_header() {
        let headers = HeaderMap::new();
        assert!(!validate_auth(&headers, "abc123"));
    }

    #[test]
    fn validate_auth_rejects_non_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Basic abc123".parse().unwrap());
        assert!(!validate_auth(&headers, "abc123"));
    }

    #[test]
    fn public_read_only_request_allows_brain_search() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::from(1)),
            method: "tools/call".into(),
            params: Some(json!({ "name": "brain_search", "arguments": { "query": "hello" } })),
        };
        assert!(is_public_read_only_request(&req));
    }

    #[test]
    fn public_read_only_request_allows_brain_wiki_audit() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::from(1)),
            method: "tools/call".into(),
            params: Some(json!({ "name": "brain_wiki_audit", "arguments": { "limit": 10 } })),
        };
        assert!(is_public_read_only_request(&req));
    }

    #[test]
    fn public_read_only_request_denies_ingest() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::from(1)),
            method: "tools/call".into(),
            params: Some(
                json!({ "name": "brain_ingest_url", "arguments": { "url": "https://example.com" } }),
            ),
        };
        assert!(!is_public_read_only_request(&req));
    }

    #[test]
    fn public_read_only_request_denies_brain_wiki_digest_text() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::from(1)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "brain_wiki_digest_text",
                "arguments": { "content": "private note" }
            })),
        };
        assert!(!is_public_read_only_request(&req));
    }

    #[test]
    fn json_rpc_response_serializes_ok() {
        let resp = JsonRpcResponse::ok(Value::from(1), json!({"status": "ok"}));
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains("\"result\""));
        assert!(!s.contains("\"error\""));
    }

    #[test]
    fn json_rpc_response_serializes_err() {
        let resp = JsonRpcResponse::err(Value::from(1), -32601, "not found".into());
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains("\"error\""));
        assert!(!s.contains("\"result\""));
    }
}
