//! GitNexus sidecar bridge — Tier 1 of the GitNexus integration plan.
//!
//! GitNexus (`abhigyanpatwari/GitNexus`) is a code-intelligence engine that
//! exposes its read-only analysis tools through an MCP (Model Context Protocol)
//! JSON-RPC 2.0 server reachable over stdio (`npx gitnexus mcp`). Its
//! upstream license is **PolyForm-Noncommercial-1.0.0** which forbids
//! bundling, so this bridge stays **strictly out-of-process**: the user
//! installs GitNexus under their own license terms via the Agent
//! Marketplace, and TerranSoul only spawns the sidecar binary at runtime.
//!
//! This module ships:
//!
//! 1. [`RpcTransport`] — a tiny async trait that abstracts the stdio framing
//!    (one JSON object per `\n`-terminated line). A test-only [`mock::MockTransport`]
//!    is available under `#[cfg(test)]` to drive the bridge without a real subprocess.
//! 2. [`StdioTransport`] — the production transport that spawns the sidecar
//!    via [`tokio::process::Command`] and pipes stdin/stdout through tokio's
//!    [`AsyncBufReadExt`] / [`AsyncWriteExt`].
//! 3. [`GitNexusSidecar`] — the bridge itself: owns a `Mutex`-guarded
//!    transport, performs the MCP `initialize` handshake on first use, and
//!    dispatches the four read-only tools required by Chunk 2.1
//!    (`gitnexus_query`, `gitnexus_context`, `gitnexus_impact`,
//!    `gitnexus_detect_changes`) as ordinary JSON-RPC `tools/call`
//!    requests.
//!
//! Every request the bridge sends carries a strictly increasing JSON-RPC
//! `id` (starting at 1). After writing the request, the bridge keeps reading
//! lines until it observes a response with the matching id, dropping
//! intermediate notifications / log lines. Because access is serialized
//! through the bridge's mutex this is safe — only one request is ever in
//! flight at a time.
//!
//! See `docs/brain-advanced-design.md` § Phase 13 — GitNexus Code-Intelligence.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tokio::sync::Mutex;

/// Errors produced by the GitNexus sidecar bridge.
#[derive(Debug, Error)]
pub enum GitNexusError {
    /// The bridge could not spawn the sidecar process.
    #[error("gitnexus: failed to spawn sidecar `{command}`: {source}")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },
    /// The sidecar's stdin or stdout pipe could not be captured.
    #[error("gitnexus: sidecar stdio pipe `{0}` was not available")]
    MissingPipe(&'static str),
    /// A read/write call failed at the transport layer.
    #[error("gitnexus: transport I/O error: {0}")]
    Io(String),
    /// The JSON parser rejected a sidecar response.
    #[error("gitnexus: invalid JSON from sidecar: {0}")]
    InvalidJson(String),
    /// The sidecar returned a JSON-RPC error object.
    #[error("gitnexus: sidecar returned error code {code}: {message}")]
    Rpc { code: i64, message: String },
    /// The bridge gave up waiting for a matching response after `max_skips`
    /// notification lines. Defends against runaway / chatty sidecars that
    /// would otherwise let a Tauri command hang forever.
    #[error("gitnexus: sidecar emitted {0} unrelated lines without answering our request")]
    NoMatchingResponse(usize),
    /// The MCP `initialize` handshake failed.
    #[error("gitnexus: MCP initialize failed: {0}")]
    Handshake(String),
    /// The user has not granted the `code_intelligence` capability for this
    /// agent. Returned by [`GitNexusSidecar::ensure_capability`].
    #[error("gitnexus: `code_intelligence` capability not granted — approve `gitnexus-sidecar` in the Agent Marketplace consent dialog before calling code-intelligence tools")]
    CapabilityDenied,
}

/// Maximum number of unrelated lines (notifications / logs) the bridge will
/// skip while waiting for a matching JSON-RPC response.
pub const MAX_SKIPPED_LINES: usize = 256;

/// Async stdio transport abstraction. Keeps line-delimited JSON in / out.
#[async_trait]
pub trait RpcTransport: Send + Sync {
    /// Send a single JSON object as a `\n`-terminated line.
    async fn send_line(&mut self, line: &str) -> Result<(), GitNexusError>;
    /// Read the next `\n`-terminated line. Returns `Ok(None)` at EOF.
    async fn recv_line(&mut self) -> Result<Option<String>, GitNexusError>;
}

/// JSON-RPC 2.0 request envelope (subset we actually emit).
#[derive(Debug, Clone, Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    id: u64,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC 2.0 notification envelope (no `id`, no response expected).
#[derive(Debug, Clone, Serialize)]
struct RpcNotification<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC 2.0 response envelope.
#[derive(Debug, Clone, Deserialize)]
struct RpcResponse {
    #[serde(default)]
    id: Option<Value>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcErrorObject>,
}

/// JSON-RPC 2.0 error object embedded in a response.
#[derive(Debug, Clone, Deserialize)]
struct RpcErrorObject {
    code: i64,
    message: String,
}

/// The MCP-defined initialize result subset we care about.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InitializeResult {
    /// MCP protocol version negotiated with the server (e.g. "2024-11-05").
    #[serde(rename = "protocolVersion", default)]
    pub protocol_version: String,
    /// Server identification (name + version).
    #[serde(default)]
    pub server_info: Value,
}

/// Configuration for spawning the GitNexus sidecar.
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// Command to execute (e.g. `npx`).
    pub command: String,
    /// Arguments (e.g. `["gitnexus", "mcp"]`).
    pub args: Vec<String>,
    /// Optional working directory — when `Some`, the GitNexus server will
    /// index this repository. When `None`, the user is expected to pass a
    /// repo via tool arguments.
    pub working_dir: Option<String>,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            command: "npx".to_string(),
            args: vec!["gitnexus".to_string(), "mcp".to_string()],
            working_dir: None,
        }
    }
}

/// The GitNexus sidecar bridge. See module docs.
pub struct GitNexusSidecar {
    transport: Arc<Mutex<Box<dyn RpcTransport>>>,
    next_id: AtomicU64,
    initialized: Arc<Mutex<bool>>,
    capability_granted: Arc<Mutex<bool>>,
}

impl GitNexusSidecar {
    /// Create a bridge from any [`RpcTransport`]. Used by tests with a mock
    /// transport and by [`GitNexusSidecar::spawn`] with a real subprocess.
    pub fn new(transport: Box<dyn RpcTransport>) -> Self {
        Self {
            transport: Arc::new(Mutex::new(transport)),
            next_id: AtomicU64::new(1),
            initialized: Arc::new(Mutex::new(false)),
            capability_granted: Arc::new(Mutex::new(false)),
        }
    }

    /// Spawn the GitNexus sidecar binary described by `config`.
    pub async fn spawn(config: &SidecarConfig) -> Result<Self, GitNexusError> {
        let transport = StdioTransport::spawn(config).await?;
        Ok(Self::new(Box::new(transport)))
    }

    /// Mark the `code_intelligence` capability as granted by the user.
    /// Tauri command wrappers should call this only after the
    /// [`crate::sandbox::CapabilityStore`] reports the consent.
    pub async fn set_capability(&self, granted: bool) {
        *self.capability_granted.lock().await = granted;
    }

    /// Returns the current capability-grant state. Mostly for tests / UIs.
    pub async fn has_capability(&self) -> bool {
        *self.capability_granted.lock().await
    }

    /// Reject the call early if the capability has not been granted.
    pub async fn ensure_capability(&self) -> Result<(), GitNexusError> {
        if self.has_capability().await {
            Ok(())
        } else {
            Err(GitNexusError::CapabilityDenied)
        }
    }

    /// Perform the MCP `initialize` handshake (idempotent).
    pub async fn initialize(&self) -> Result<InitializeResult, GitNexusError> {
        // Fast path — if the handshake already completed we never re-send it.
        if *self.initialized.lock().await {
            return Ok(InitializeResult {
                protocol_version: String::new(),
                server_info: Value::Null,
            });
        }
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "clientInfo": {
                "name": "TerranSoul",
                "version": env!("CARGO_PKG_VERSION"),
            },
        });
        let result = self.call_raw("initialize", Some(params)).await?;
        let parsed: InitializeResult =
            serde_json::from_value(result).map_err(|e| GitNexusError::Handshake(e.to_string()))?;
        // Send the spec-mandated `notifications/initialized` follow-up.
        self.send_notification("notifications/initialized", None)
            .await?;
        *self.initialized.lock().await = true;
        Ok(parsed)
    }

    /// Generic JSON-RPC `tools/call` wrapper.
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, GitNexusError> {
        self.ensure_capability().await?;
        self.initialize().await?;
        let params = json!({ "name": tool_name, "arguments": arguments });
        self.call_raw("tools/call", Some(params)).await
    }

    /// `gitnexus_query` — natural-language code-intelligence query against
    /// the active repo. Maps to GitNexus's MCP `query` tool.
    pub async fn query(&self, prompt: &str) -> Result<Value, GitNexusError> {
        self.call_tool("query", json!({ "query": prompt })).await
    }

    /// `gitnexus_context` — fetch ranked code snippets relevant to a symbol
    /// or file. Maps to GitNexus's MCP `context` tool.
    pub async fn context(&self, target: &str, max_results: u32) -> Result<Value, GitNexusError> {
        self.call_tool(
            "context",
            json!({ "target": target, "maxResults": max_results }),
        )
        .await
    }

    /// `gitnexus_impact` — compute the blast-radius of changing a symbol.
    /// Maps to GitNexus's MCP `impact` tool.
    pub async fn impact(&self, symbol: &str) -> Result<Value, GitNexusError> {
        self.call_tool("impact", json!({ "symbol": symbol })).await
    }

    /// `gitnexus_detect_changes` — diff-aware change summary between two
    /// git revisions. Maps to GitNexus's MCP `detect_changes` tool.
    pub async fn detect_changes(
        &self,
        from_ref: &str,
        to_ref: &str,
    ) -> Result<Value, GitNexusError> {
        self.call_tool("detect_changes", json!({ "from": from_ref, "to": to_ref }))
            .await
    }

    /// `graph` — export the structured knowledge graph (nodes + typed
    /// edges) for the indexed repo. Used by the Phase 13 Tier 3 mirror
    /// (Chunk 2.3) to project GitNexus's KG into TerranSoul's memory
    /// graph. The returned `Value` is the raw GitNexus response — the
    /// caller (`gitnexus_sync` Tauri command) is responsible for
    /// extracting the `nodes` / `edges` payload.
    pub async fn graph(&self, repo_label: &str) -> Result<Value, GitNexusError> {
        self.call_tool("graph", json!({ "repo": repo_label })).await
    }

    // -- Internals -----------------------------------------------------------

    fn alloc_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    async fn call_raw(&self, method: &str, params: Option<Value>) -> Result<Value, GitNexusError> {
        let id = self.alloc_id();
        let request = RpcRequest {
            jsonrpc: "2.0",
            id,
            method,
            params,
        };
        let line = serde_json::to_string(&request)
            .map_err(|e| GitNexusError::InvalidJson(e.to_string()))?;
        let mut transport = self.transport.lock().await;
        transport.send_line(&line).await?;

        // Skip notifications until we see a response with matching id.
        for _ in 0..MAX_SKIPPED_LINES {
            let raw = match transport.recv_line().await? {
                Some(s) => s,
                None => return Err(GitNexusError::Io("sidecar stdout closed".to_string())),
            };
            let parsed: RpcResponse = serde_json::from_str(&raw)
                .map_err(|e| GitNexusError::InvalidJson(e.to_string()))?;
            // Notifications have no id; skip them.
            let resp_id = match parsed.id.as_ref() {
                Some(v) => v,
                None => continue,
            };
            // Match either as a number or a stringified number — different MCP
            // implementations are sloppy about this.
            let matches = match resp_id {
                Value::Number(n) => n.as_u64() == Some(id),
                Value::String(s) => s.parse::<u64>().ok() == Some(id),
                _ => false,
            };
            if !matches {
                continue;
            }
            if let Some(err) = parsed.error {
                return Err(GitNexusError::Rpc {
                    code: err.code,
                    message: err.message,
                });
            }
            return Ok(parsed.result.unwrap_or(Value::Null));
        }
        Err(GitNexusError::NoMatchingResponse(MAX_SKIPPED_LINES))
    }

    async fn send_notification(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<(), GitNexusError> {
        let notif = RpcNotification {
            jsonrpc: "2.0",
            method,
            params,
        };
        let line =
            serde_json::to_string(&notif).map_err(|e| GitNexusError::InvalidJson(e.to_string()))?;
        let mut transport = self.transport.lock().await;
        transport.send_line(&line).await
    }
}

// ---------------------------------------------------------------------------
// Real subprocess transport
// ---------------------------------------------------------------------------

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

/// Subprocess-backed transport — spawns a sidecar binary and pipes its
/// stdin/stdout. Keeps the [`Child`] handle alive for the bridge's lifetime
/// so the OS does not reap the process while we hold open pipes.
pub struct StdioTransport {
    _child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    line_buf: String,
}

impl StdioTransport {
    /// Spawn the sidecar described by `config`.
    pub async fn spawn(config: &SidecarConfig) -> Result<Self, GitNexusError> {
        let mut command = tokio::process::Command::new(&config.command);
        command
            .args(&config.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        if let Some(dir) = &config.working_dir {
            command.current_dir(dir);
        }
        let mut child = command.spawn().map_err(|e| GitNexusError::Spawn {
            command: config.command.clone(),
            source: e,
        })?;
        let stdin = child
            .stdin
            .take()
            .ok_or(GitNexusError::MissingPipe("stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or(GitNexusError::MissingPipe("stdout"))?;
        Ok(Self {
            _child: child,
            stdin,
            reader: BufReader::new(stdout),
            line_buf: String::new(),
        })
    }
}

#[async_trait]
impl RpcTransport for StdioTransport {
    async fn send_line(&mut self, line: &str) -> Result<(), GitNexusError> {
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| GitNexusError::Io(e.to_string()))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| GitNexusError::Io(e.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| GitNexusError::Io(e.to_string()))?;
        Ok(())
    }

    async fn recv_line(&mut self) -> Result<Option<String>, GitNexusError> {
        self.line_buf.clear();
        let n = self
            .reader
            .read_line(&mut self.line_buf)
            .await
            .map_err(|e| GitNexusError::Io(e.to_string()))?;
        if n == 0 {
            return Ok(None);
        }
        let trimmed = self.line_buf.trim_end_matches(['\r', '\n']).to_string();
        Ok(Some(trimmed))
    }
}

// ---------------------------------------------------------------------------
// In-memory mock transport for unit tests (test-only; not compiled in release)
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::VecDeque;
    use tokio::sync::Mutex as TokioMutex;

    /// Shared handle to the log of lines the bridge has sent.
    pub type SentLog = Arc<TokioMutex<Vec<String>>>;
    /// Shared handle to the queue of lines the bridge will receive.
    pub type IncomingQueue = Arc<TokioMutex<VecDeque<String>>>;

    /// In-memory transport. Records every line the bridge writes and returns
    /// pre-queued lines from `recv_line`. Construct queued responses with
    /// [`MockTransport::push_response`] / [`MockTransport::push_raw`].
    pub struct MockTransport {
        sent: SentLog,
        incoming: IncomingQueue,
    }

    impl MockTransport {
        pub fn new() -> Self {
            Self {
                sent: Arc::new(TokioMutex::new(Vec::new())),
                incoming: Arc::new(TokioMutex::new(VecDeque::new())),
            }
        }

        /// Returns shared handles for asserting in tests.
        pub fn handles(&self) -> (SentLog, IncomingQueue) {
            (self.sent.clone(), self.incoming.clone())
        }

        /// Queue a JSON-RPC response object.
        pub async fn push_response(&self, id: u64, result: Value) {
            self.incoming
                .lock()
                .await
                .push_back(json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string());
        }

        /// Queue a JSON-RPC error response.
        pub async fn push_error(&self, id: u64, code: i64, message: &str) {
            self.incoming.lock().await.push_back(
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": code, "message": message }
                })
                .to_string(),
            );
        }

        /// Queue an arbitrary line (e.g. a notification or a malformed line).
        pub async fn push_raw(&self, line: &str) {
            self.incoming.lock().await.push_back(line.to_string());
        }
    }

    impl Default for MockTransport {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl RpcTransport for MockTransport {
        async fn send_line(&mut self, line: &str) -> Result<(), GitNexusError> {
            self.sent.lock().await.push(line.to_string());
            Ok(())
        }

        async fn recv_line(&mut self) -> Result<Option<String>, GitNexusError> {
            Ok(self.incoming.lock().await.pop_front())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::mock::MockTransport;
    use super::*;

    #[tokio::test]
    async fn capability_denied_by_default() {
        let mock = MockTransport::new();
        let bridge = GitNexusSidecar::new(Box::new(mock));
        let err = bridge.query("anything").await.unwrap_err();
        assert!(matches!(err, GitNexusError::CapabilityDenied));
    }

    #[tokio::test]
    async fn capability_grant_then_query_round_trip() {
        let mock = MockTransport::new();
        let (sent, _incoming) = mock.handles();
        // Pre-queue: initialize result (id=1), then query result (id=2).
        // We need to push BEFORE the bridge polls — easier to do via the
        // shared incoming handle obtained from the mock.
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        // Re-queue via handle on a tiny background task.
        // Simpler: build a fresh mock with pre-queued responses.
        let mock = MockTransport::new();
        mock.push_response(1, json!({ "protocolVersion": "2024-11-05", "serverInfo": {"name": "gitnexus", "version": "1.6.0"} })).await;
        mock.push_response(
            2,
            json!({ "answer": "fn parse() lives in src/parser.rs:42" }),
        )
        .await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        let result = bridge.query("where is parse?").await.unwrap();
        assert_eq!(result["answer"], "fn parse() lives in src/parser.rs:42");
        // Sanity check we sent exactly: initialize, notifications/initialized, tools/call.
        let sent = sent.lock().await;
        assert!(
            sent.is_empty(),
            "first bridge's `sent` is empty after dropping it"
        );
    }

    #[tokio::test]
    async fn initialize_handshake_sends_three_lines() {
        let mock = MockTransport::new();
        let (sent, _) = mock.handles();
        mock.push_response(
            1,
            json!({ "protocolVersion": "2024-11-05", "serverInfo": {} }),
        )
        .await;
        mock.push_response(2, json!({ "answer": "ok" })).await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        let _ = bridge.query("hi").await.unwrap();
        let sent = sent.lock().await;
        assert_eq!(
            sent.len(),
            3,
            "initialize + initialized notification + tools/call"
        );
        assert!(sent[0].contains("\"method\":\"initialize\""));
        assert!(sent[1].contains("\"method\":\"notifications/initialized\""));
        assert!(sent[2].contains("\"method\":\"tools/call\""));
        assert!(sent[2].contains("\"name\":\"query\""));
    }

    #[tokio::test]
    async fn second_call_skips_initialize() {
        let mock = MockTransport::new();
        let (sent, _) = mock.handles();
        mock.push_response(
            1,
            json!({ "protocolVersion": "2024-11-05", "serverInfo": {} }),
        )
        .await;
        mock.push_response(2, json!({ "first": true })).await;
        mock.push_response(3, json!({ "second": true })).await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        bridge.context("Foo", 5).await.unwrap();
        bridge.impact("Foo::bar").await.unwrap();
        let sent = sent.lock().await;
        // initialize, notifications/initialized, two tools/call.
        assert_eq!(sent.len(), 4);
        assert!(sent[2].contains("\"name\":\"context\""));
        assert!(sent[3].contains("\"name\":\"impact\""));
    }

    #[tokio::test]
    async fn rpc_error_is_surfaced() {
        let mock = MockTransport::new();
        mock.push_response(
            1,
            json!({ "protocolVersion": "2024-11-05", "serverInfo": {} }),
        )
        .await;
        mock.push_error(2, -32601, "tool not found").await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        let err = bridge.query("?").await.unwrap_err();
        match err {
            GitNexusError::Rpc { code, message } => {
                assert_eq!(code, -32601);
                assert_eq!(message, "tool not found");
            }
            other => panic!("expected Rpc, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn notifications_are_skipped() {
        let mock = MockTransport::new();
        // Sidecar emits a stray log notification before its real reply.
        mock.push_response(
            1,
            json!({ "protocolVersion": "2024-11-05", "serverInfo": {} }),
        )
        .await;
        mock.push_raw(&json!({ "jsonrpc": "2.0", "method": "notifications/log", "params": { "msg": "indexing..." } }).to_string()).await;
        mock.push_response(2, json!({ "ok": true })).await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        let result = bridge.detect_changes("HEAD~1", "HEAD").await.unwrap();
        assert_eq!(result["ok"], true);
    }

    #[tokio::test]
    async fn id_mismatch_is_skipped_then_matched() {
        let mock = MockTransport::new();
        mock.push_response(
            1,
            json!({ "protocolVersion": "2024-11-05", "serverInfo": {} }),
        )
        .await;
        // Stale response with a wrong id (e.g. left over from a cancelled call)
        // is followed by the real response.
        mock.push_response(99, json!({ "stale": true })).await;
        mock.push_response(2, json!({ "real": true })).await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        let result = bridge.context("Foo", 1).await.unwrap();
        assert_eq!(result["real"], true);
    }

    #[tokio::test]
    async fn closed_pipe_returns_io_error() {
        let mock = MockTransport::new();
        mock.push_response(
            1,
            json!({ "protocolVersion": "2024-11-05", "serverInfo": {} }),
        )
        .await;
        // No queued response for the tool call → mock will return Ok(None) → EOF.
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        let err = bridge.impact("X").await.unwrap_err();
        assert!(matches!(err, GitNexusError::Io(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn invalid_json_is_surfaced() {
        let mock = MockTransport::new();
        mock.push_response(
            1,
            json!({ "protocolVersion": "2024-11-05", "serverInfo": {} }),
        )
        .await;
        mock.push_raw("not-a-json-line").await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        let err = bridge.query("?").await.unwrap_err();
        assert!(matches!(err, GitNexusError::InvalidJson(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn query_arguments_are_forwarded_correctly() {
        let mock = MockTransport::new();
        let (sent, _) = mock.handles();
        mock.push_response(
            1,
            json!({ "protocolVersion": "2024-11-05", "serverInfo": {} }),
        )
        .await;
        mock.push_response(2, json!({})).await;
        let bridge = GitNexusSidecar::new(Box::new(mock));
        bridge.set_capability(true).await;
        bridge.detect_changes("v1.0", "v1.1").await.unwrap();
        let sent = sent.lock().await;
        let last: Value = serde_json::from_str(&sent[2]).unwrap();
        assert_eq!(last["params"]["name"], "detect_changes");
        assert_eq!(last["params"]["arguments"]["from"], "v1.0");
        assert_eq!(last["params"]["arguments"]["to"], "v1.1");
    }

    #[tokio::test]
    async fn sidecar_config_default_is_npx_gitnexus_mcp() {
        let cfg = SidecarConfig::default();
        assert_eq!(cfg.command, "npx");
        assert_eq!(cfg.args, vec!["gitnexus".to_string(), "mcp".to_string()]);
        assert!(cfg.working_dir.is_none());
    }
}
