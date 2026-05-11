//! Integration tests for the MCP server.
//!
//! These tests start a real HTTP server on an ephemeral port, make
//! JSON-RPC requests via reqwest, and validate the MCP protocol flow.

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::ai_integrations::mcp;
    use crate::AppState;

    /// Helper: start the MCP server on port 0 (OS-assigned) and return
    /// (handle, base_url, token).
    async fn start_test_server() -> (mcp::McpServerHandle, String, String) {
        let state = AppState::for_test();
        let token = "test-token-abc123".to_string();

        // Use port 0 to let the OS pick an available port.
        // Our start_server binds to a specific port, so use a high
        // ephemeral port that's unlikely to conflict.
        let port = portpicker();
        let handle = mcp::start_server(state, port, token.clone(), false, false)
            .await
            .expect("MCP server should start");

        // Give the server a tick to start accepting connections.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let url = format!("http://127.0.0.1:{}/mcp", handle.port);
        (handle, url, token)
    }

    fn health_url(mcp_url: &str) -> String {
        format!("{}/health", mcp_url.trim_end_matches("/mcp"))
    }

    fn status_url(mcp_url: &str) -> String {
        format!("{}/status", mcp_url.trim_end_matches("/mcp"))
    }

    /// Pick a random high port that's likely free.
    fn portpicker() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    }

    /// Send a JSON-RPC request to the MCP server.
    async fn rpc(url: &str, token: &str, body: Value) -> (u16, Value) {
        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .expect("request should succeed");
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        (status, body)
    }

    #[tokio::test]
    async fn initialize_returns_server_info() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {}
            }),
        )
        .await;

        assert_eq!(status, 200);
        assert_eq!(body["jsonrpc"], "2.0");
        assert_eq!(body["id"], 1);
        // In debug/test builds the name is "terransoul-brain-dev",
        // in release builds it's "terransoul-brain".
        let name = body["result"]["serverInfo"]["name"].as_str().unwrap();
        assert!(
            name == "terransoul-brain" || name == "terransoul-brain-dev",
            "unexpected server name: {name}"
        );
        assert!(body["result"]["capabilities"]["tools"].is_object());

        handle.stop();
    }

    #[tokio::test]
    async fn tools_list_returns_28_tools() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            }),
        )
        .await;

        assert_eq!(status, 200);
        let tools = body["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 35);

        // Verify the first tool has the expected structure.
        assert_eq!(tools[0]["name"], "brain_search");
        assert!(tools[0]["inputSchema"].is_object());

        // Verify Knowledge Wiki tools are present before code tools.
        assert_eq!(tools[11]["name"], "brain_wiki_audit");
        assert_eq!(tools[15]["name"], "brain_wiki_digest_text");
        assert_eq!(tools[16]["name"], "brain_review_gaps");
        assert_eq!(tools[17]["name"], "brain_session_checklist");

        // Verify code tools are present.
        assert_eq!(tools[18]["name"], "code_query");
        assert_eq!(tools[21]["name"], "code_rename");
        assert_eq!(tools[31]["name"], "code_branch_sync");
        assert_eq!(tools[32]["name"], "code_index_commit");
        assert_eq!(tools[33]["name"], "code_branch_diff");
        assert_eq!(tools[34]["name"], "code_group_drift");

        handle.stop();
    }

    #[tokio::test]
    async fn tools_call_brain_health() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "brain_health",
                    "arguments": {}
                }
            }),
        )
        .await;

        assert_eq!(status, 200);
        let content = &body["result"]["content"][0];
        assert_eq!(content["type"], "text");
        // Parse the text as JSON and check health fields.
        let health: Value = serde_json::from_str(content["text"].as_str().unwrap()).unwrap();
        assert!(health["version"].is_string());
        assert!(health["brain_provider"].is_string());

        handle.stop();
    }

    #[tokio::test]
    async fn tools_call_brain_search_empty_brain() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {
                    "name": "brain_search",
                    "arguments": { "query": "test" }
                }
            }),
        )
        .await;

        assert_eq!(status, 200);
        let content = &body["result"]["content"][0];
        assert_eq!(content["type"], "text");
        // Empty brain returns empty array.
        let results: Vec<Value> = serde_json::from_str(content["text"].as_str().unwrap()).unwrap();
        assert!(results.is_empty());

        handle.stop();
    }

    #[tokio::test]
    async fn tools_call_brain_wiki_audit_empty_brain() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 41,
                "method": "tools/call",
                "params": {
                    "name": "brain_wiki_audit",
                    "arguments": { "limit": 5 }
                }
            }),
        )
        .await;

        assert_eq!(status, 200);
        let content = &body["result"]["content"][0];
        assert_eq!(content["type"], "text");
        let report: Value = serde_json::from_str(content["text"].as_str().unwrap()).unwrap();
        assert_eq!(report["total_memories"], 0);
        assert!(report["orphan_ids"].as_array().unwrap().is_empty());
        assert!(report["stale_ids"].as_array().unwrap().is_empty());

        handle.stop();
    }

    #[tokio::test]
    async fn tools_call_brain_wiki_digest_text_dedups_content() {
        let (handle, url, token) = start_test_server().await;

        let request = |id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": {
                    "name": "brain_wiki_digest_text",
                    "arguments": {
                        "content": "MCP wiki digest integration test note.",
                        "source_url": "mcp-test://wiki-digest",
                        "tags": "test,wiki",
                        "importance": 4
                    }
                }
            })
        };

        let (first_status, first_body) = rpc(&url, &token, request(42)).await;
        assert_eq!(first_status, 200);
        let first_text = first_body["result"]["content"][0]["text"].as_str().unwrap();
        let first_result: Value = serde_json::from_str(first_text).unwrap();
        assert_eq!(first_result["kind"], "ingested");

        let (second_status, second_body) = rpc(&url, &token, request(43)).await;
        assert_eq!(second_status, 200);
        let second_text = second_body["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let second_result: Value = serde_json::from_str(second_text).unwrap();
        assert_eq!(second_result["kind"], "skipped");

        handle.stop();
    }

    #[tokio::test]
    async fn ping_returns_empty_result() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "ping"
            }),
        )
        .await;

        assert_eq!(status, 200);
        assert_eq!(body["result"], json!({}));

        handle.stop();
    }

    #[tokio::test]
    async fn unauthorized_without_token() {
        let (handle, url, _token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            "wrong-token",
            json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "ping"
            }),
        )
        .await;

        assert_eq!(status, 401);
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("unauthorized"));

        handle.stop();
    }

    #[tokio::test]
    async fn health_returns_ok_without_auth() {
        let (handle, url, _token) = start_test_server().await;

        let resp = reqwest::Client::new()
            .get(health_url(&url))
            .send()
            .await
            .expect("health request should succeed");
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.expect("health body should be JSON");

        assert_eq!(status, 200);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["port"], handle.port);

        handle.stop();
    }

    #[tokio::test]
    async fn status_includes_actual_port_and_seed_loaded() {
        let (handle, url, token) = start_test_server().await;

        let resp = reqwest::Client::new()
            .get(status_url(&url))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .expect("status request should succeed");
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.expect("status body should be JSON");

        assert_eq!(status, 200);
        assert_eq!(body["actual_port"], handle.port);
        assert!(body["seed_loaded"].is_boolean());
        assert!(body["health"].is_object());

        handle.stop();
    }

    #[tokio::test]
    async fn mcp_rejects_missing_auth_header() {
        let (handle, url, _token) = start_test_server().await;

        let resp = reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&json!({ "jsonrpc": "2.0", "id": 16, "method": "ping" }))
            .send()
            .await
            .expect("request should succeed");
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.expect("error body should be JSON");

        assert_eq!(status, 401);
        assert_eq!(body["error"]["message"], "unauthorized");

        handle.stop();
    }

    #[tokio::test]
    async fn mcp_notification_rejects_missing_auth_header() {
        let (handle, url, _token) = start_test_server().await;

        let resp = reqwest::Client::new()
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }))
            .send()
            .await
            .expect("request should succeed");
        let status = resp.status().as_u16();
        let body: Value = resp.json().await.expect("error body should be JSON");

        assert_eq!(status, 401);
        assert_eq!(body["error"]["message"], "unauthorized");

        handle.stop();
    }

    #[tokio::test]
    async fn notification_returns_202() {
        let (handle, url, token) = start_test_server().await;

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 202);

        handle.stop();
    }

    #[tokio::test]
    async fn unknown_method_returns_error() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "nonexistent/method"
            }),
        )
        .await;

        assert_eq!(status, 200);
        assert_eq!(body["error"]["code"], -32601);

        handle.stop();
    }

    #[tokio::test]
    async fn tools_call_unknown_tool() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "tools/call",
                "params": {
                    "name": "nonexistent_tool",
                    "arguments": {}
                }
            }),
        )
        .await;

        assert_eq!(status, 200);
        // Unknown tool returns isError: true in tool result.
        assert_eq!(body["result"]["isError"], true);

        handle.stop();
    }

    #[tokio::test]
    async fn server_stop_is_graceful() {
        let (handle, url, token) = start_test_server().await;

        // Verify server is responding.
        let (status, _) = rpc(
            &url,
            &token,
            json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}),
        )
        .await;
        assert_eq!(status, 200);

        // Stop the server.
        handle.stop();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle.task).await;

        // Server should no longer accept connections.
        let client = reqwest::Client::new();
        let result = client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}))
            .send()
            .await;
        assert!(result.is_err(), "server should be stopped");
    }

    // ─── Code tool integration tests ─────────────────────────────────────

    #[tokio::test]
    async fn code_query_returns_structured_error_when_no_sidecar() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 10,
                "method": "tools/call",
                "params": {
                    "name": "code_query",
                    "arguments": { "prompt": "find main function" }
                }
            }),
        )
        .await;

        assert_eq!(status, 200);
        assert_eq!(body["result"]["isError"], true);
        let text = body["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            text.contains("code_intelligence")
                || text.contains("sidecar not configured")
                || text.contains("no repos indexed yet"),
            "expected code-tool setup error, got: {text}"
        );

        handle.stop();
    }

    #[tokio::test]
    async fn code_context_returns_error_without_sidecar() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 11,
                "method": "tools/call",
                "params": {
                    "name": "code_context",
                    "arguments": { "target": "AppState" }
                }
            }),
        )
        .await;

        assert_eq!(status, 200);
        assert_eq!(body["result"]["isError"], true);

        handle.stop();
    }

    #[tokio::test]
    async fn code_impact_returns_error_without_sidecar() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 12,
                "method": "tools/call",
                "params": {
                    "name": "code_impact",
                    "arguments": { "symbol": "run" }
                }
            }),
        )
        .await;

        assert_eq!(status, 200);
        assert_eq!(body["result"]["isError"], true);

        handle.stop();
    }

    #[tokio::test]
    async fn code_detect_changes_returns_error_without_sidecar() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 13,
                "method": "tools/call",
                "params": {
                    "name": "code_detect_changes",
                    "arguments": { "from_ref": "main", "to_ref": "HEAD" }
                }
            }),
        )
        .await;

        assert_eq!(status, 200);
        assert_eq!(body["result"]["isError"], true);

        handle.stop();
    }

    #[tokio::test]
    async fn code_graph_sync_returns_error_without_sidecar() {
        let (handle, url, token) = start_test_server().await;

        let (status, body) = rpc(
            &url,
            &token,
            json!({
                "jsonrpc": "2.0",
                "id": 14,
                "method": "tools/call",
                "params": {
                    "name": "code_graph_sync",
                    "arguments": { "repo_label": "test-repo" }
                }
            }),
        )
        .await;

        assert_eq!(status, 200);
        assert_eq!(body["result"]["isError"], true);

        handle.stop();
    }
}
