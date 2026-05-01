/// WebSocket + TLS fallback transport using `tokio-tungstenite`.
///
/// This is the fallback when QUIC is not available (e.g. restrictive NATs).
/// Messages are JSON text frames.
use std::net::SocketAddr;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message as WsMsg;

use super::{LinkMessage, LinkStatus, LinkTransport, PeerAddr};

type WsSinkBox = Box<
    dyn futures_util::Sink<WsMsg, Error = tokio_tungstenite::tungstenite::Error> + Send + Unpin,
>;
type WsStreamBox = Box<
    dyn futures_util::Stream<Item = Result<WsMsg, tokio_tungstenite::tungstenite::Error>>
        + Send
        + Unpin,
>;

pub struct WsTransport {
    status: Mutex<LinkStatus>,
    listener: Mutex<Option<TcpListener>>,
    ws_sink: Mutex<Option<WsSinkBox>>,
    ws_stream: Mutex<Option<WsStreamBox>>,
}

impl WsTransport {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(LinkStatus::Disconnected),
            listener: Mutex::new(None),
            ws_sink: Mutex::new(None),
            ws_stream: Mutex::new(None),
        }
    }
}

impl Default for WsTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LinkTransport for WsTransport {
    fn name(&self) -> &str {
        "WebSocket"
    }

    async fn listen(&self, port: u16) -> Result<u16, String> {
        let addr: SocketAddr = format!("0.0.0.0:{port}")
            .parse()
            .map_err(|e: std::net::AddrParseError| e.to_string())?;
        let listener = TcpListener::bind(addr).await.map_err(|e| e.to_string())?;
        let bound_port = listener.local_addr().map_err(|e| e.to_string())?.port();
        *self.listener.lock().await = Some(listener);
        *self.status.lock().await = LinkStatus::Connecting;
        Ok(bound_port)
    }

    async fn connect(&self, addr: &PeerAddr) -> Result<(), String> {
        *self.status.lock().await = LinkStatus::Connecting;

        let url = format!("ws://{}:{}", addr.host, addr.port);
        let (ws, _response) = tokio_tungstenite::connect_async(&url)
            .await
            .map_err(|e| format!("WS connect: {e}"))?;

        let (sink, stream) = ws.split();
        *self.ws_sink.lock().await = Some(Box::new(sink));
        *self.ws_stream.lock().await = Some(Box::new(stream));
        *self.status.lock().await = LinkStatus::Connected;
        Ok(())
    }

    async fn send(&self, msg: &LinkMessage) -> Result<(), String> {
        let json = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        let mut lock = self.ws_sink.lock().await;
        let sink = lock.as_mut().ok_or("not connected")?;
        sink.send(WsMsg::Text(json.into()))
            .await
            .map_err(|e| e.to_string())
    }

    async fn recv(&self) -> Result<LinkMessage, String> {
        let mut lock = self.ws_stream.lock().await;
        let stream = lock.as_mut().ok_or("not connected")?;

        loop {
            match stream.next().await {
                Some(Ok(WsMsg::Text(text))) => {
                    return serde_json::from_str(&text).map_err(|e| e.to_string());
                }
                Some(Ok(WsMsg::Close(_))) | None => {
                    return Err("connection closed".to_string());
                }
                Some(Ok(_)) => {
                    // Skip ping/pong/binary frames
                    continue;
                }
                Some(Err(e)) => {
                    return Err(format!("WS recv: {e}"));
                }
            }
        }
    }

    async fn close(&self) -> Result<(), String> {
        if let Some(mut sink) = self.ws_sink.lock().await.take() {
            let _ = sink.send(WsMsg::Close(None)).await;
        }
        *self.ws_stream.lock().await = None;
        *self.listener.lock().await = None;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ws_transport_starts_disconnected() {
        let transport = WsTransport::new();
        assert_eq!(transport.status(), LinkStatus::Disconnected);
    }

    #[tokio::test]
    async fn ws_transport_name() {
        let transport = WsTransport::new();
        assert_eq!(transport.name(), "WebSocket");
    }

    #[tokio::test]
    async fn ws_listen_binds_port() {
        let transport = WsTransport::new();
        let port = transport.listen(0).await.unwrap();
        assert!(port > 0);
        assert_eq!(transport.status(), LinkStatus::Connecting);
        transport.close().await.unwrap();
    }

    #[tokio::test]
    async fn ws_send_without_connection_fails() {
        let transport = WsTransport::new();
        let msg = LinkMessage {
            id: "x".into(),
            origin: "a".into(),
            target: "b".into(),
            kind: "ping".into(),
            payload: serde_json::json!(null),
        };
        let result = transport.send(&msg).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not connected"));
    }

    #[tokio::test]
    async fn ws_recv_without_connection_fails() {
        let transport = WsTransport::new();
        let result = transport.recv().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not connected"));
    }

    #[tokio::test]
    async fn ws_close_from_disconnected_is_ok() {
        let transport = WsTransport::new();
        let result = transport.close().await;
        assert!(result.is_ok());
        assert_eq!(transport.status(), LinkStatus::Disconnected);
    }
}
