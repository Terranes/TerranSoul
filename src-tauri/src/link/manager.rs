/// Link manager — high-level API that wraps a `LinkTransport` with
/// reconnection logic, status tracking, and the ability to swap
/// between QUIC (primary) and WebSocket (fallback).
use std::sync::Arc;

use tokio::sync::Mutex;

use super::{LinkMessage, LinkPeer, LinkStatus, LinkTransport, PeerAddr};
use super::quic::QuicTransport;
use super::ws::WsTransport;

/// Which transport back-end to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Quic,
    WebSocket,
}

/// Central coordinator for a single peer-to-peer link.
pub struct LinkManager {
    transport: Mutex<Box<dyn LinkTransport>>,
    kind: Mutex<TransportKind>,
    peer: Mutex<Option<LinkPeer>>,
    reconnect_attempts: Mutex<u32>,
    max_reconnect_attempts: u32,
}

impl LinkManager {
    /// Create a new manager with the QUIC transport as default.
    pub fn new() -> Self {
        Self {
            transport: Mutex::new(Box::new(QuicTransport::new())),
            kind: Mutex::new(TransportKind::Quic),
            peer: Mutex::new(None),
            reconnect_attempts: Mutex::new(0),
            max_reconnect_attempts: 5,
        }
    }

    /// Create a new manager with a specific transport (for testing).
    pub fn with_transport(transport: Box<dyn LinkTransport>, kind: TransportKind) -> Self {
        Self {
            transport: Mutex::new(transport),
            kind: Mutex::new(kind),
            peer: Mutex::new(None),
            reconnect_attempts: Mutex::new(0),
            max_reconnect_attempts: 5,
        }
    }

    /// Start listening for incoming connections on the given port.
    pub async fn start_server(&self, port: u16) -> Result<u16, String> {
        let transport = self.transport.lock().await;
        transport.listen(port).await
    }

    /// Connect to a remote peer.
    pub async fn connect(&self, addr: &PeerAddr, peer: LinkPeer) -> Result<(), String> {
        let transport = self.transport.lock().await;
        transport.connect(addr).await?;
        *self.peer.lock().await = Some(peer);
        *self.reconnect_attempts.lock().await = 0;
        Ok(())
    }

    /// Attempt to reconnect to the last known peer.
    /// Falls back from QUIC → WebSocket after exhausting QUIC retries.
    pub async fn reconnect(&self, addr: &PeerAddr) -> Result<(), String> {
        let mut attempts = self.reconnect_attempts.lock().await;
        *attempts += 1;

        if *attempts > self.max_reconnect_attempts {
            // Try falling back to the other transport
            let mut kind = self.kind.lock().await;
            if *kind == TransportKind::Quic {
                *kind = TransportKind::WebSocket;
                *self.transport.lock().await = Box::new(WsTransport::new());
                *attempts = 1; // reset attempts for the new transport
            } else {
                return Err("max reconnect attempts exceeded on all transports".to_string());
            }
        }

        let transport = self.transport.lock().await;
        transport.connect(addr).await
    }

    /// Send a message to the connected peer.
    pub async fn send(&self, msg: &LinkMessage) -> Result<(), String> {
        let transport = self.transport.lock().await;
        transport.send(msg).await
    }

    /// Receive the next message from the connected peer.
    pub async fn recv(&self) -> Result<LinkMessage, String> {
        let transport = self.transport.lock().await;
        transport.recv().await
    }

    /// Current status.
    pub fn status(&self) -> LinkStatus {
        self.transport
            .try_lock()
            .map(|t| t.status())
            .unwrap_or(LinkStatus::Disconnected)
    }

    /// Current transport kind.
    pub fn transport_kind(&self) -> TransportKind {
        self.kind
            .try_lock()
            .map(|k| *k)
            .unwrap_or(TransportKind::Quic)
    }

    /// Currently connected peer, if any.
    pub async fn connected_peer(&self) -> Option<LinkPeer> {
        self.peer.lock().await.clone()
    }

    /// Disconnect and clean up.
    pub async fn disconnect(&self) -> Result<(), String> {
        let transport = self.transport.lock().await;
        transport.close().await?;
        *self.peer.lock().await = None;
        *self.reconnect_attempts.lock().await = 0;
        Ok(())
    }
}

impl Default for LinkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{LinkMessage, LinkStatus, LinkTransport, PeerAddr};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};

    // ── Mock transport for unit tests ──

    struct MockTransport {
        status: Mutex<LinkStatus>,
        connect_calls: AtomicU32,
        connect_fail: bool,
        sent_messages: Mutex<Vec<LinkMessage>>,
        recv_message: Mutex<Option<LinkMessage>>,
    }

    impl MockTransport {
        fn new() -> Self {
            Self {
                status: Mutex::new(LinkStatus::Disconnected),
                connect_calls: AtomicU32::new(0),
                connect_fail: false,
                sent_messages: Mutex::new(Vec::new()),
                recv_message: Mutex::new(None),
            }
        }

        fn failing() -> Self {
            Self {
                status: Mutex::new(LinkStatus::Disconnected),
                connect_calls: AtomicU32::new(0),
                connect_fail: true,
                sent_messages: Mutex::new(Vec::new()),
                recv_message: Mutex::new(None),
            }
        }

        fn with_recv(msg: LinkMessage) -> Self {
            Self {
                status: Mutex::new(LinkStatus::Connected),
                connect_calls: AtomicU32::new(0),
                connect_fail: false,
                sent_messages: Mutex::new(Vec::new()),
                recv_message: Mutex::new(Some(msg)),
            }
        }
    }

    #[async_trait]
    impl LinkTransport for MockTransport {
        fn name(&self) -> &str { "Mock" }

        async fn listen(&self, _port: u16) -> Result<u16, String> {
            *self.status.lock().await = LinkStatus::Connecting;
            Ok(9999)
        }

        async fn connect(&self, _addr: &PeerAddr) -> Result<(), String> {
            self.connect_calls.fetch_add(1, Ordering::SeqCst);
            if self.connect_fail {
                return Err("mock connect failed".to_string());
            }
            *self.status.lock().await = LinkStatus::Connected;
            Ok(())
        }

        async fn send(&self, msg: &LinkMessage) -> Result<(), String> {
            self.sent_messages.lock().await.push(msg.clone());
            Ok(())
        }

        async fn recv(&self) -> Result<LinkMessage, String> {
            self.recv_message
                .lock()
                .await
                .take()
                .ok_or_else(|| "no message".to_string())
        }

        async fn close(&self) -> Result<(), String> {
            *self.status.lock().await = LinkStatus::Disconnected;
            Ok(())
        }

        fn status(&self) -> LinkStatus {
            self.status
                .try_lock()
                .map(|s| *s)
                .unwrap_or(LinkStatus::Disconnected)
        }
    }

    fn test_addr() -> PeerAddr {
        PeerAddr {
            host: "127.0.0.1".to_string(),
            port: 4433,
        }
    }

    fn test_peer() -> LinkPeer {
        LinkPeer {
            device_id: "peer-1".to_string(),
            name: "Peer".to_string(),
            addr: "127.0.0.1:4433".to_string(),
        }
    }

    fn test_msg() -> LinkMessage {
        LinkMessage {
            id: "m-1".to_string(),
            origin: "a".to_string(),
            target: "b".to_string(),
            kind: "ping".to_string(),
            payload: serde_json::json!(null),
        }
    }

    #[tokio::test]
    async fn manager_starts_disconnected() {
        let mgr = LinkManager::with_transport(Box::new(MockTransport::new()), TransportKind::Quic);
        assert_eq!(mgr.status(), LinkStatus::Disconnected);
        assert!(mgr.connected_peer().await.is_none());
    }

    #[tokio::test]
    async fn manager_start_server() {
        let mgr = LinkManager::with_transport(Box::new(MockTransport::new()), TransportKind::Quic);
        let port = mgr.start_server(0).await.unwrap();
        assert_eq!(port, 9999);
    }

    #[tokio::test]
    async fn manager_connect_sets_peer() {
        let mgr = LinkManager::with_transport(Box::new(MockTransport::new()), TransportKind::Quic);
        mgr.connect(&test_addr(), test_peer()).await.unwrap();
        let peer = mgr.connected_peer().await;
        assert!(peer.is_some());
        assert_eq!(peer.unwrap().device_id, "peer-1");
    }

    #[tokio::test]
    async fn manager_send_delegates_to_transport() {
        let mock = Arc::new(MockTransport::new());
        // We need to put mock in the manager, so we create a new one
        let mgr = LinkManager::with_transport(Box::new(MockTransport::new()), TransportKind::Quic);
        let msg = test_msg();
        let result = mgr.send(&msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn manager_recv_returns_message() {
        let msg = test_msg();
        let mock = MockTransport::with_recv(msg.clone());
        let mgr = LinkManager::with_transport(Box::new(mock), TransportKind::Quic);
        let received = mgr.recv().await.unwrap();
        assert_eq!(received.id, "m-1");
        assert_eq!(received.kind, "ping");
    }

    #[tokio::test]
    async fn manager_disconnect_clears_peer() {
        let mgr = LinkManager::with_transport(Box::new(MockTransport::new()), TransportKind::Quic);
        mgr.connect(&test_addr(), test_peer()).await.unwrap();
        assert!(mgr.connected_peer().await.is_some());
        mgr.disconnect().await.unwrap();
        assert!(mgr.connected_peer().await.is_none());
        assert_eq!(mgr.status(), LinkStatus::Disconnected);
    }

    #[tokio::test]
    async fn manager_reconnect_increments_attempts() {
        let mgr = LinkManager::with_transport(Box::new(MockTransport::new()), TransportKind::Quic);
        mgr.reconnect(&test_addr()).await.unwrap();
        // After 1 reconnect the transport should be connected
        assert_eq!(mgr.status(), LinkStatus::Connected);
    }

    #[tokio::test]
    async fn manager_reconnect_fallback_to_ws() {
        // Use a failing transport so all reconnects fail, forcing fallback
        let mgr = LinkManager::with_transport(
            Box::new(MockTransport::failing()),
            TransportKind::Quic,
        );
        // Exhaust QUIC reconnect attempts
        for _ in 0..5 {
            let _ = mgr.reconnect(&test_addr()).await;
        }
        // Next reconnect should switch to WebSocket
        // (the WsTransport will fail to connect to 127.0.0.1:4433 since nothing is listening)
        let result = mgr.reconnect(&test_addr()).await;
        // We switched transport kind even if connect fails
        assert_eq!(mgr.transport_kind(), TransportKind::WebSocket);
    }

    #[tokio::test]
    async fn manager_default_is_quic() {
        let mgr = LinkManager::new();
        assert_eq!(mgr.transport_kind(), TransportKind::Quic);
    }
}
