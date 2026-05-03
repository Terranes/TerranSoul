pub mod handlers;
pub mod manager;
pub mod quic;
pub mod ws;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── Shared types ──────────────────────────────────────────────────────

/// Status of the peer link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// A framed message sent between paired devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkMessage {
    /// Unique message id.
    pub id: String,
    /// Originating device_id.
    pub origin: String,
    /// Target device_id (or "*" for broadcast).
    pub target: String,
    /// Message kind — e.g. "chat", "sync", "ping".
    pub kind: String,
    /// Opaque JSON payload.
    pub payload: serde_json::Value,
}

/// Summary of a connected peer, safe to expose to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkPeer {
    pub device_id: String,
    pub name: String,
    pub addr: String,
}

/// Connection address info.
#[derive(Debug, Clone)]
pub struct PeerAddr {
    pub host: String,
    pub port: u16,
}

impl std::fmt::Display for PeerAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

// ── Transport trait ───────────────────────────────────────────────────

/// Abstraction over a transport layer (QUIC primary, WebSocket fallback).
/// Implementations handle framing, TLS, and I/O — callers only see
/// [`LinkMessage`] values.
#[async_trait]
pub trait LinkTransport: Send + Sync {
    /// Human-readable name of the transport (e.g. "QUIC", "WebSocket").
    fn name(&self) -> &str;

    /// Start listening on the given port.
    /// Returns the actual port bound (useful when `port == 0`).
    async fn listen(&self, port: u16) -> Result<u16, String>;

    /// Connect to a remote peer at the given address.
    async fn connect(&self, addr: &PeerAddr) -> Result<(), String>;

    /// Send a message to the currently connected peer.
    async fn send(&self, msg: &LinkMessage) -> Result<(), String>;

    /// Block until the next message arrives (or the connection drops).
    async fn recv(&self) -> Result<LinkMessage, String>;

    /// Gracefully close the connection.
    async fn close(&self) -> Result<(), String>;

    /// Current status.
    fn status(&self) -> LinkStatus;
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_message_json_roundtrip() {
        let msg = LinkMessage {
            id: "msg-1".to_string(),
            origin: "device-a".to_string(),
            target: "device-b".to_string(),
            kind: "chat".to_string(),
            payload: serde_json::json!({"text": "hello"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let restored: LinkMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "msg-1");
        assert_eq!(restored.origin, "device-a");
        assert_eq!(restored.target, "device-b");
        assert_eq!(restored.kind, "chat");
        assert_eq!(restored.payload["text"], "hello");
    }

    #[test]
    fn link_message_broadcast_target() {
        let msg = LinkMessage {
            id: "msg-2".to_string(),
            origin: "device-a".to_string(),
            target: "*".to_string(),
            kind: "sync".to_string(),
            payload: serde_json::json!(null),
        };
        assert_eq!(msg.target, "*");
    }

    #[test]
    fn link_status_serde_roundtrip() {
        for status in [
            LinkStatus::Disconnected,
            LinkStatus::Connecting,
            LinkStatus::Connected,
            LinkStatus::Reconnecting,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let restored: LinkStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, status);
        }
    }

    #[test]
    fn link_status_serialises_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&LinkStatus::Disconnected).unwrap(),
            "\"disconnected\""
        );
        assert_eq!(
            serde_json::to_string(&LinkStatus::Reconnecting).unwrap(),
            "\"reconnecting\""
        );
    }

    #[test]
    fn link_peer_json_roundtrip() {
        let peer = LinkPeer {
            device_id: "d-1".to_string(),
            name: "Phone".to_string(),
            addr: "192.168.1.5:4433".to_string(),
        };
        let json = serde_json::to_string(&peer).unwrap();
        let restored: LinkPeer = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.device_id, "d-1");
        assert_eq!(restored.name, "Phone");
        assert_eq!(restored.addr, "192.168.1.5:4433");
    }

    #[test]
    fn peer_addr_display() {
        let addr = PeerAddr {
            host: "10.0.0.1".to_string(),
            port: 4433,
        };
        assert_eq!(addr.to_string(), "10.0.0.1:4433");
    }
}
