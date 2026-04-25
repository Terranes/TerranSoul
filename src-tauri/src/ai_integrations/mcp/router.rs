//! MCP JSON-RPC 2.0 router over HTTP (Streamable HTTP transport).
//!
//! Implements the MCP 2025-11-05 protocol surface on axum:
//! - `POST /mcp` — JSON-RPC 2.0 requests (initialize, tools/list, tools/call, ping)
//! - Notifications (no `id` field) return 202 Accepted
//! - Bearer-token authentication on every request
//!
//! Reference: <https://modelcontextprotocol.io/specification/2025-11-25/basic/transports#streamable-http>

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::ai_integrations::gateway::*;

use super::tools;

// ─── Shared state for the axum router ───────────────────────────────────────

#[derive(Clone)]
pub struct McpRouterState {
    pub gw: Arc<dyn BrainGateway>,
    pub caps: GatewayCaps,
    pub token: String,
}

// ─── JSON-RPC 2.0 types ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl JsonRpcResponse {
    fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    fn err(id: Value, code: i64, message: String) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

// ─── Router construction ────────────────────────────────────────────────────

/// Build the axum router for the MCP Streamable HTTP transport.
pub fn build(state: McpRouterState) -> Router {
    Router::new()
        .route("/mcp", post(handle_request))
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
            let resp = JsonRpcResponse::err(
                Value::Null,
                -32700,
                format!("parse error: {e}"),
            );
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    // Notifications (no id) — acknowledge without a body.
    if req.id.is_none() {
        return StatusCode::ACCEPTED.into_response();
    }

    let id = req.id.unwrap_or(Value::Null);

    // Bearer-token authentication.
    if !validate_auth(&headers, &state.token) {
        let resp = JsonRpcResponse::err(id, -32000, "unauthorized".into());
        return (StatusCode::UNAUTHORIZED, Json(resp)).into_response();
    }

    let params = req.params.unwrap_or(Value::Null);

    let resp = match req.method.as_str() {
        "initialize" => {
            let build_mode = if super::is_dev_build() { "dev" } else { "release" };
            let server_name = if super::is_dev_build() {
                "terransoul-brain-dev"
            } else {
                "terransoul-brain"
            };
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": server_name,
                    "version": env!("CARGO_PKG_VERSION"),
                    "buildMode": build_mode
                }
            });
            JsonRpcResponse::ok(id, result)
        }

        "tools/list" => {
            let result = json!({ "tools": tools::definitions() });
            JsonRpcResponse::ok(id, result)
        }

        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));

            match tools::dispatch(state.gw.as_ref(), &state.caps, name, &args).await {
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

        "ping" => JsonRpcResponse::ok(id, json!({})),

        _ => JsonRpcResponse::err(
            id,
            -32601,
            format!("method not found: {}", req.method),
        ),
    };

    (StatusCode::OK, Json(resp)).into_response()
}

/// Validate the `Authorization: Bearer <token>` header.
fn validate_auth(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|t| t == expected)
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
