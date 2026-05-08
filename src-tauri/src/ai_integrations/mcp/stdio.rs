//! MCP stdio transport — Chunk 15.9.
//!
//! Newline-delimited JSON-RPC 2.0 over stdin/stdout. This is the
//! **canonical** MCP transport per the 2024-11-05 spec and is the
//! default transport for editors like Claude Desktop, the VS Code MCP
//! extension, and Codex CLI.
//!
//! ## Protocol framing
//!
//! Per the MCP stdio specification, each JSON-RPC message is a single
//! JSON object on its own line, terminated by `\n`. There are no
//! Content-Length / LSP-style headers — just newline delimiters.
//!
//! ## Authentication
//!
//! Stdio runs in a trusted parent–child relationship: the editor
//! spawns `terransoul --mcp-stdio` as its own subprocess, so anything
//! that can read/write the pipes already has the user's privileges.
//! We therefore **do not** validate bearer tokens on stdio — that
//! matches canonical MCP behaviour (Claude Desktop, VS Code's MCP
//! extension, etc. never pass tokens to stdio servers).
//!
//! The HTTP transport ([`super::router`]) keeps bearer-token auth
//! because loopback HTTP is reachable by any other local process.
//!
//! ## Logging
//!
//! `stdout` is reserved for JSON-RPC responses. All diagnostic /
//! error logging from the stdio shim must go to `stderr` so it does
//! not corrupt the protocol stream.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};

use crate::ai_integrations::gateway::{
    AppStateGateway, BrainGateway, GatewayCaps, GatewayError, IngestSink, IngestUrlResponse,
};

use super::router::{dispatch_method, JsonRpcRequest, JsonRpcResponse};

/// Run the stdio JSON-RPC loop until stdin closes.
///
/// Reads newline-delimited JSON requests from `reader`, dispatches
/// each through [`dispatch_method`], and writes the response (also
/// newline-delimited) to `writer`. Notifications (requests without
/// an `id` field) produce no output, matching the JSON-RPC 2.0 spec.
///
/// This function does not return until EOF on the reader. It logs
/// fatal errors to `stderr` and continues serving the next request
/// on parse failures.
pub async fn run_loop<R, W>(
    gw: Arc<dyn BrainGateway>,
    caps: GatewayCaps,
    reader: R,
    mut writer: W,
) -> std::io::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            // EOF — peer closed stdin, time to exit.
            break;
        }
        let trimmed = line.trim();
        let line_for_error = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::err(
                    Value::Null,
                    -32700,
                    format!("parse error on line '{}': {}", line_for_error, e),
                );
                write_response(&mut writer, &resp).await?;
                continue;
            }
        };

        // Notifications (no id) are fire-and-forget per JSON-RPC 2.0.
        let Some(id) = req.id.clone() else {
            continue;
        };

        let params = req.params.unwrap_or(Value::Null);
        let resp = dispatch_method(gw.as_ref(), &caps, &req.method, params, id).await;
        if let Ok(v) = serde_json::to_value(&resp) {
            if v.get("error").is_some_and(|e| !e.is_null()) {
                eprintln!("MCP stdio dispatch error for method '{}': {}", req.method, v["error"]);
            }
        }
        write_response(&mut writer, &resp).await?;
    }

    Ok(())
}

async fn write_response<W>(writer: &mut W, resp: &JsonRpcResponse) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut bytes = serde_json::to_vec(resp).map_err(|e| {
        std::io::Error::other(format!("Failed to serialize JSON-RPC response: {}", e))
    })?;
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
}

/// Convenience entry point: build an [`AppStateGateway`] from an
/// [`AppState`] and run the stdio loop on the process's actual
/// stdin/stdout. Used by `terransoul --mcp-stdio`.
pub async fn run_with_state(state: crate::AppState) -> std::io::Result<()> {
    let ingest_sink = Arc::new(StdioIngestSink {
        state: state.clone(),
    });
    let gw: Arc<dyn BrainGateway> = Arc::new(AppStateGateway::with_ingest(state, ingest_sink));
    let caps = super::transport_caps();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    run_loop(gw, caps, stdin, stdout).await
}

#[derive(Clone)]
struct StdioIngestSink {
    state: crate::AppState,
}

#[async_trait]
impl IngestSink for StdioIngestSink {
    async fn start_ingest(
        &self,
        source: String,
        tags: Option<String>,
        importance: Option<i64>,
    ) -> Result<IngestUrlResponse, GatewayError> {
        let result = crate::commands::ingest::ingest_document_silent(
            source,
            tags,
            importance,
            self.state.clone(),
        )
        .await
        .map_err(|error| GatewayError::Internal(format!("ingest_document_silent: {error}")))?;

        Ok(IngestUrlResponse {
            task_id: result.task_id,
            source: result.source,
            source_type: result.source_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_integrations::gateway::AppStateGateway;
    use crate::AppState;
    use serde_json::json;
    use tokio::io::AsyncReadExt;

    /// Drive the stdio loop with an in-memory pipe pair: write
    /// `requests` to the reader side, then read the response stream.
    async fn run_with_inputs(requests: &[Value]) -> Vec<Value> {
        let state = AppState::for_test();
        let gw: Arc<dyn BrainGateway> = Arc::new(AppStateGateway::new(state));

        // Build the input — newline-delimited JSON.
        let mut input = String::new();
        for req in requests {
            input.push_str(&serde_json::to_string(req).unwrap());
            input.push('\n');
        }

        let reader = std::io::Cursor::new(input.into_bytes());
        let mut output: Vec<u8> = Vec::new();

        run_loop(gw, GatewayCaps::default(), reader, &mut output)
            .await
            .expect("loop ran cleanly");

        // Parse newline-delimited responses.
        let s = String::from_utf8(output).unwrap();
        s.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    #[tokio::test]
    async fn initialize_returns_server_info() {
        let resps = run_with_inputs(&[json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        })])
        .await;

        assert_eq!(resps.len(), 1);
        assert_eq!(resps[0]["jsonrpc"], "2.0");
        assert_eq!(resps[0]["id"], 1);
        let name = resps[0]["result"]["serverInfo"]["name"].as_str().unwrap();
        assert!(
            name == "terransoul-brain" || name == "terransoul-brain-dev",
            "unexpected server name: {name}"
        );
        assert_eq!(resps[0]["result"]["protocolVersion"], "2024-11-05");
    }

    #[tokio::test]
    async fn tools_list_returns_brain_tools() {
        let resps = run_with_inputs(&[json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        })])
        .await;

        assert_eq!(resps.len(), 1);
        let tools = resps[0]["result"]["tools"].as_array().expect("tools array");
        assert!(!tools.is_empty(), "expected at least one tool");
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"brain_search"));
        assert!(names.contains(&"brain_health"));
    }

    #[tokio::test]
    async fn ping_returns_empty_result() {
        let resps = run_with_inputs(&[json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "ping"
        })])
        .await;

        assert_eq!(resps.len(), 1);
        assert_eq!(resps[0]["id"], 3);
        assert!(resps[0]["result"].is_object());
    }

    #[tokio::test]
    async fn notification_produces_no_response() {
        // No id → notification. Followed by a real request to confirm
        // the loop kept processing.
        let resps = run_with_inputs(&[
            json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            }),
            json!({
                "jsonrpc": "2.0",
                "id": 99,
                "method": "ping"
            }),
        ])
        .await;

        assert_eq!(resps.len(), 1, "notification should not produce output");
        assert_eq!(resps[0]["id"], 99);
    }

    #[tokio::test]
    async fn unknown_method_returns_error() {
        let resps = run_with_inputs(&[json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "nonsense/method"
        })])
        .await;

        assert_eq!(resps.len(), 1);
        assert_eq!(resps[0]["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn parse_error_keeps_loop_alive() {
        let state = AppState::for_test();
        let gw: Arc<dyn BrainGateway> = Arc::new(AppStateGateway::new(state));

        // Mix garbage with a valid request.
        let input = b"not json\n{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"ping\"}\n".to_vec();
        let reader = std::io::Cursor::new(input);
        let mut output: Vec<u8> = Vec::new();

        run_loop(gw, GatewayCaps::default(), reader, &mut output)
            .await
            .unwrap();

        let s = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 2, "expected one parse-error + one ping reply");

        let parse_err: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parse_err["error"]["code"], -32700);
        assert!(parse_err["id"].is_null());
        assert!(
            parse_err["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("parse error on line 'not json':"),
            "unexpected parse error message: {}",
            parse_err["error"]["message"]
        );

        let ping_reply: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(ping_reply["id"], 7);
        assert!(ping_reply["result"].is_object());
    }

    #[tokio::test]
    async fn empty_input_exits_cleanly() {
        let state = AppState::for_test();
        let gw: Arc<dyn BrainGateway> = Arc::new(AppStateGateway::new(state));

        let reader = std::io::Cursor::new(Vec::<u8>::new());
        let mut output: Vec<u8> = Vec::new();

        run_loop(gw, GatewayCaps::default(), reader, &mut output)
            .await
            .unwrap();

        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn multiple_requests_are_processed_in_order() {
        let resps = run_with_inputs(&[
            json!({"jsonrpc":"2.0","id":"a","method":"ping"}),
            json!({"jsonrpc":"2.0","id":"b","method":"ping"}),
            json!({"jsonrpc":"2.0","id":"c","method":"ping"}),
        ])
        .await;

        assert_eq!(resps.len(), 3);
        assert_eq!(resps[0]["id"], "a");
        assert_eq!(resps[1]["id"], "b");
        assert_eq!(resps[2]["id"], "c");
    }

    /// Smoke test for the AsyncWrite pathway with a real tokio pipe.
    #[tokio::test]
    async fn run_loop_works_with_tokio_pipes() {
        let state = AppState::for_test();
        let gw: Arc<dyn BrainGateway> = Arc::new(AppStateGateway::new(state));

        let (client_tx, server_rx) = tokio::io::duplex(4096);
        let (server_tx, mut client_rx) = tokio::io::duplex(4096);

        let task = tokio::spawn(run_loop(gw, GatewayCaps::default(), server_rx, server_tx));

        // Drop client_tx after writing so the server sees EOF.
        let mut client_tx = client_tx;
        let req = br#"{"jsonrpc":"2.0","id":42,"method":"ping"}
"#;
        client_tx.write_all(req).await.unwrap();
        drop(client_tx);

        let mut buf = Vec::new();
        client_rx.read_to_end(&mut buf).await.unwrap();
        task.await.unwrap().unwrap();

        let s = String::from_utf8(buf).unwrap();
        let resp: Value = serde_json::from_str(s.trim()).unwrap();
        assert_eq!(resp["id"], 42);
    }
}
