//! LAN knowledge-sharing via UDP broadcast discovery.
//!
//! Allows a TerranSoul instance to:
//! 1. Advertise its MCP brain on the local network (host mode)
//! 2. Discover other TerranSoul brains on the LAN (client mode)
//! 3. Connect to a remote brain and query it via HTTP MCP protocol
//!
//! ## Discovery protocol
//! - Hosts send periodic UDP broadcast on port 7424
//! - Payload: 4-byte magic "TSBL" + JSON `DiscoveryAnnouncement`
//! - Clients listen on the same port for announcements
//! - Token is NOT broadcast — shared out-of-band (QR, email, verbal)
//!
//! ## Security model
//! - Discovery only reveals existence/metadata (name, port, memory count)
//! - Hosts may require a bearer token or explicitly allow public read-only access
//! - Token mode shares credentials out-of-band (displayed in UI, QR code, etc.)
//! - Read-only by default; host can optionally allow writes

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::sync::watch;

/// UDP port used for LAN brain discovery broadcasts.
pub const DISCOVERY_PORT: u16 = 7424;

/// Magic header to identify TerranSoul discovery packets.
const MAGIC: &[u8; 4] = b"TSBL";

/// Broadcast interval in seconds.
const BROADCAST_INTERVAL_SECS: u64 = 5;

/// A discovered remote brain on the LAN.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoveredBrain {
    /// Human-readable brain display name.
    pub brain_name: String,
    /// Host IP address (from the UDP source address).
    pub host: String,
    /// MCP server port.
    pub port: u16,
    /// Brain provider type (e.g., "free", "ollama", "openai").
    pub provider: String,
    /// Number of memories in the remote brain.
    pub memory_count: u32,
    /// Whether the remote brain is read-only.
    pub read_only: bool,
    /// Hostname of the advertising machine.
    pub hostname: String,
    /// Whether a bearer token is required to connect.
    pub token_required: bool,
}

/// UDP broadcast payload (JSON-serialized after the MAGIC header).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoveryAnnouncement {
    /// Protocol version.
    pub version: u8,
    /// Display name for the shared brain.
    pub brain_name: String,
    /// MCP HTTP port to connect to.
    pub mcp_port: u16,
    /// Brain provider info.
    pub provider: String,
    /// Number of memories.
    pub memory_count: u32,
    /// Read-only flag.
    pub read_only: bool,
    /// Machine hostname.
    pub hostname: String,
    /// Whether bearer auth is required for querying.
    pub token_required: bool,
}

/// A connection to a remote TerranSoul brain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteBrainConnection {
    /// Unique connection ID.
    pub id: String,
    /// Remote host address.
    pub host: String,
    /// Remote MCP port.
    pub port: u16,
    /// Bearer token for authentication when required.
    pub token: Option<String>,
    /// Whether the remote host expects a bearer token.
    pub token_required: bool,
    /// Display name of the remote brain.
    pub brain_name: String,
    /// Whether the connection is active.
    pub connected: bool,
}

/// Search result from a remote brain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSearchResult {
    pub id: i64,
    pub content: String,
    pub tags: Option<String>,
    pub importance: i64,
    pub score: f64,
    pub source_url: Option<String>,
    pub tier: String,
}

/// Response from a remote brain health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteBrainHealth {
    pub version: String,
    pub brain_provider: Option<String>,
    pub brain_model: Option<String>,
    pub rag_quality_pct: Option<u32>,
    pub memory_total: Option<u64>,
}

/// Handle to the UDP broadcast advertiser.
pub struct LanShareAdvertiser {
    shutdown: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl LanShareAdvertiser {
    /// Start broadcasting this TerranSoul brain on the LAN.
    pub async fn start(
        brain_name: &str,
        port: u16,
        provider: &str,
        memory_count: u32,
        read_only: bool,
        token_required: bool,
    ) -> Result<Self, String> {
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".to_string());

        let announcement = DiscoveryAnnouncement {
            version: 1,
            brain_name: brain_name.to_string(),
            mcp_port: port,
            provider: provider.to_string(),
            memory_count,
            read_only,
            hostname,
            token_required,
        };

        let payload_json =
            serde_json::to_vec(&announcement).map_err(|e| format!("Serialize: {e}"))?;
        let mut packet = Vec::with_capacity(MAGIC.len() + payload_json.len());
        packet.extend_from_slice(MAGIC);
        packet.extend_from_slice(&payload_json);

        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("Bind UDP: {e}"))?;
        socket
            .set_broadcast(true)
            .map_err(|e| format!("Set broadcast: {e}"))?;

        let broadcast_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::BROADCAST, DISCOVERY_PORT));

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

        let task = tokio::spawn(async move {
            loop {
                // Send first, then wait — ensures immediate broadcast on start.
                let _ = socket.send_to(&packet, broadcast_addr).await;
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(BROADCAST_INTERVAL_SECS)) => {}
                    _ = async {
                        while !*shutdown_rx.borrow_and_update() {
                            shutdown_rx.changed().await.ok();
                        }
                    } => {
                        break;
                    }
                }
            }
        });

        Ok(Self {
            shutdown: shutdown_tx,
            task,
        })
    }

    /// Stop advertising and shut down the broadcast task.
    pub fn stop(self) {
        let _ = self.shutdown.send(true);
        self.task.abort();
    }
}

/// Handle to the UDP discovery listener.
pub struct LanShareBrowser {
    shutdown: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
    discovered: Arc<std::sync::Mutex<Vec<DiscoveredBrain>>>,
}

impl LanShareBrowser {
    /// Start listening for TerranSoul brain broadcasts on the LAN.
    pub async fn start() -> Result<Self, String> {
        let socket = UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DISCOVERY_PORT,
        )))
        .await
        .map_err(|e| format!("Bind discovery listener on port {DISCOVERY_PORT}: {e}"))?;

        socket
            .set_broadcast(true)
            .map_err(|e| format!("Set broadcast on listener: {e}"))?;

        let discovered: Arc<std::sync::Mutex<Vec<DiscoveredBrain>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let discovered_clone = discovered.clone();

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

        let task = tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                tokio::select! {
                    result = socket.recv_from(&mut buf) => {
                        match result {
                            Ok((len, addr)) => {
                                if let Some(brain) = parse_discovery_packet(&buf[..len], addr) {
                                    let mut guard = discovered_clone.lock().unwrap_or_else(|e| e.into_inner());
                                    // Update or insert (deduplicate by host+port).
                                    if let Some(existing) = guard.iter_mut().find(|b| b.host == brain.host && b.port == brain.port) {
                                        *existing = brain;
                                    } else {
                                        guard.push(brain);
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    _ = async {
                        while !*shutdown_rx.borrow_and_update() {
                            shutdown_rx.changed().await.ok();
                        }
                    } => {
                        break;
                    }
                }
            }
        });

        Ok(Self {
            shutdown: shutdown_tx,
            task,
            discovered,
        })
    }

    /// Get all currently discovered brains.
    pub fn collect_discovered(&self) -> Vec<DiscoveredBrain> {
        self.discovered
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Stop listening.
    pub fn stop(self) {
        let _ = self.shutdown.send(true);
        self.task.abort();
    }
}

/// Parse a UDP discovery packet into a DiscoveredBrain.
fn parse_discovery_packet(data: &[u8], addr: SocketAddr) -> Option<DiscoveredBrain> {
    if data.len() < MAGIC.len() + 2 {
        return None;
    }
    if &data[..MAGIC.len()] != MAGIC {
        return None;
    }

    let json_data = &data[MAGIC.len()..];
    let announcement: DiscoveryAnnouncement = serde_json::from_slice(json_data).ok()?;

    // Ignore unsupported protocol versions.
    if announcement.version != 1 {
        return None;
    }

    Some(DiscoveredBrain {
        brain_name: announcement.brain_name,
        host: addr.ip().to_string(),
        port: announcement.mcp_port,
        provider: announcement.provider,
        memory_count: announcement.memory_count,
        read_only: announcement.read_only,
        hostname: announcement.hostname,
        token_required: announcement.token_required,
    })
}

/// HTTP client for querying a remote TerranSoul brain via MCP protocol.
pub struct RemoteBrainClient {
    client: reqwest::Client,
    base_url: String,
    token: Option<String>,
}

impl RemoteBrainClient {
    pub fn new(host: &str, port: u16, token: Option<&str>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(5))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: format!("http://{}:{}", host, port),
            token: token.map(ToOwned::to_owned),
        }
    }

    fn with_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(token) = self.token.as_deref() {
            builder.header("Authorization", format!("Bearer {token}"))
        } else {
            builder
        }
    }

    /// Check health of the remote brain.
    pub async fn health(&self) -> Result<RemoteBrainHealth, String> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map_err(|e| format!("Remote health request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Remote health returned {}", resp.status()));
        }

        resp.json::<RemoteBrainHealth>()
            .await
            .map_err(|e| format!("Parse remote health: {e}"))
    }

    /// Search the remote brain using MCP JSON-RPC protocol.
    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<RemoteSearchResult>, String> {
        let rpc_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "brain_search",
                "arguments": {
                    "query": query,
                    "limit": limit,
                    "mode": "rrf",
                    "rerank": false
                }
            }
        });

        let resp = self
            .with_auth(
                self.client
                    .post(format!("{}/mcp", self.base_url))
                    .header("Content-Type", "application/json")
                    .json(&rpc_body),
            )
            .send()
            .await
            .map_err(|e| format!("Remote search request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Remote search returned {}", resp.status()));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse remote search response: {e}"))?;

        // Extract results from JSON-RPC response.
        // MCP tools/call returns { result: { content: [{ type: "text", text: "..." }] } }
        let text = body
            .pointer("/result/content/0/text")
            .and_then(|v| v.as_str())
            .unwrap_or("[]");

        serde_json::from_str::<Vec<RemoteSearchResult>>(text)
            .map_err(|e| format!("Parse search results: {e}"))
    }

    /// Get a specific memory entry from the remote brain.
    pub async fn get_entry(&self, id: i64) -> Result<serde_json::Value, String> {
        let rpc_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "brain_get_entry",
                "arguments": {
                    "id": id
                }
            }
        });

        let resp = self
            .with_auth(
                self.client
                    .post(format!("{}/mcp", self.base_url))
                    .header("Content-Type", "application/json")
                    .json(&rpc_body),
            )
            .send()
            .await
            .map_err(|e| format!("Remote get_entry request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Remote get_entry returned {}", resp.status()));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse remote get_entry response: {e}"))?;

        let text = body
            .pointer("/result/content/0/text")
            .and_then(|v| v.as_str())
            .unwrap_or("{}");

        serde_json::from_str(text).map_err(|e| format!("Parse entry: {e}"))
    }

    /// Ingest a URL into the remote brain (requires write access on host).
    pub async fn ingest_url(
        &self,
        source: &str,
        tags: Option<&str>,
        importance: Option<i64>,
    ) -> Result<serde_json::Value, String> {
        let mut args = serde_json::json!({ "source": source });
        if let Some(t) = tags {
            args["tags"] = serde_json::Value::String(t.to_string());
        }
        if let Some(i) = importance {
            args["importance"] = serde_json::Value::Number(i.into());
        }

        let rpc_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "brain_ingest_url",
                "arguments": args
            }
        });

        let resp = self
            .with_auth(
                self.client
                    .post(format!("{}/mcp", self.base_url))
                    .header("Content-Type", "application/json")
                    .json(&rpc_body),
            )
            .send()
            .await
            .map_err(|e| format!("Remote ingest request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Remote ingest returned {}", resp.status()));
        }

        resp.json()
            .await
            .map_err(|e| format!("Parse remote ingest response: {e}"))
    }
}

/// State for the LAN sharing system.
pub struct LanShareState {
    /// Active advertiser (when hosting).
    pub advertiser: Option<LanShareAdvertiser>,
    /// Active browser (when discovering).
    pub browser: Option<LanShareBrowser>,
    /// Connected remote brains.
    pub connections: HashMap<String, Arc<RemoteBrainClient>>,
    /// Connection metadata for serialization.
    pub connection_info: HashMap<String, RemoteBrainConnection>,
}

impl LanShareState {
    pub fn new() -> Self {
        Self {
            advertiser: None,
            browser: None,
            connections: HashMap::new(),
            connection_info: HashMap::new(),
        }
    }
}

impl Default for LanShareState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovered_brain_serializes() {
        let brain = DiscoveredBrain {
            brain_name: "HR Company Rules".to_string(),
            host: "192.168.1.100".to_string(),
            port: 7421,
            provider: "free".to_string(),
            memory_count: 150,
            read_only: true,
            hostname: "HR-PC".to_string(),
            token_required: true,
        };
        let json = serde_json::to_string(&brain).unwrap();
        assert!(json.contains("HR Company Rules"));
        assert!(json.contains("192.168.1.100"));
    }

    #[test]
    fn remote_brain_client_builds_url() {
        let client = RemoteBrainClient::new("192.168.1.100", 7421, Some("test-token"));
        assert_eq!(client.base_url, "http://192.168.1.100:7421");
        assert_eq!(client.token.as_deref(), Some("test-token"));
    }

    #[test]
    fn lan_share_state_default() {
        let state = LanShareState::new();
        assert!(state.advertiser.is_none());
        assert!(state.browser.is_none());
        assert!(state.connections.is_empty());
    }

    #[test]
    fn parse_valid_discovery_packet() {
        let announcement = DiscoveryAnnouncement {
            version: 1,
            brain_name: "Test Brain".to_string(),
            mcp_port: 7421,
            provider: "free".to_string(),
            memory_count: 42,
            read_only: true,
            hostname: "test-pc".to_string(),
            token_required: true,
        };
        let json = serde_json::to_vec(&announcement).unwrap();
        let mut packet = Vec::new();
        packet.extend_from_slice(MAGIC);
        packet.extend_from_slice(&json);

        let addr: SocketAddr = "192.168.1.50:12345".parse().unwrap();
        let brain = parse_discovery_packet(&packet, addr).unwrap();

        assert_eq!(brain.brain_name, "Test Brain");
        assert_eq!(brain.host, "192.168.1.50");
        assert_eq!(brain.port, 7421);
        assert_eq!(brain.memory_count, 42);
        assert!(brain.read_only);
    }

    #[test]
    fn parse_invalid_magic_returns_none() {
        let data = b"XXXX{\"version\":1}";
        let addr: SocketAddr = "192.168.1.50:12345".parse().unwrap();
        assert!(parse_discovery_packet(data, addr).is_none());
    }

    #[test]
    fn parse_too_short_returns_none() {
        let data = b"TSB";
        let addr: SocketAddr = "192.168.1.50:12345".parse().unwrap();
        assert!(parse_discovery_packet(data, addr).is_none());
    }

    #[test]
    fn parse_unsupported_version_returns_none() {
        let announcement = DiscoveryAnnouncement {
            version: 99,
            brain_name: "Future Brain".to_string(),
            mcp_port: 7421,
            provider: "quantum".to_string(),
            memory_count: 0,
            read_only: true,
            hostname: "future-pc".to_string(),
            token_required: false,
        };
        let json = serde_json::to_vec(&announcement).unwrap();
        let mut packet = Vec::new();
        packet.extend_from_slice(MAGIC);
        packet.extend_from_slice(&json);

        let addr: SocketAddr = "192.168.1.50:12345".parse().unwrap();
        assert!(parse_discovery_packet(&packet, addr).is_none());
    }
}
