/// QUIC transport implementation using the `quinn` crate.
///
/// This provides the primary transport layer for TerranSoul Link.
/// Messages are length-prefixed JSON frames over a single bidirectional QUIC stream.
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use quinn::{ClientConfig, Endpoint, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use tokio::sync::Mutex;

use super::{LinkMessage, LinkStatus, LinkTransport, PeerAddr};

/// Generate a self-signed certificate + private key for TLS.
/// Used for local-network QUIC where we trust via device pairing, not PKI.
pub fn generate_self_signed_cert(
) -> Result<(Vec<CertificateDer<'static>>, PrivatePkcs8KeyDer<'static>), String> {
    let cert = rcgen::generate_simple_self_signed(vec!["terransoul.local".to_string()])
        .map_err(|e| format!("cert generation failed: {e}"))?;

    let key_der = PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());

    Ok((vec![cert_der], key_der))
}

/// Build a `quinn::ServerConfig` with a self-signed cert.
pub fn build_server_config() -> Result<ServerConfig, String> {
    let (certs, key) = generate_self_signed_cert()?;

    let crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key.into())
        .map_err(|e| format!("TLS server config: {e}"))?;

    Ok(ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(crypto)
            .map_err(|e| format!("QUIC server config: {e}"))?,
    )))
}

/// Build a `quinn::ClientConfig` that skips server certificate verification
/// (trust is established via device pairing, not PKI).
pub fn build_client_config() -> ClientConfig {
    let crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto).expect("QUIC client config"),
    ))
}

/// Certificate verifier that accepts any server cert.
/// Safe in our threat model: device identity is verified at the application layer.
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

// ── QuicTransport ─────────────────────────────────────────────────────

pub struct QuicTransport {
    status: Mutex<LinkStatus>,
    endpoint: Mutex<Option<Endpoint>>,
    send_stream: Mutex<Option<quinn::SendStream>>,
    recv_stream: Mutex<Option<quinn::RecvStream>>,
}

impl QuicTransport {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(LinkStatus::Disconnected),
            endpoint: Mutex::new(None),
            send_stream: Mutex::new(None),
            recv_stream: Mutex::new(None),
        }
    }
}

impl Default for QuicTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Write a length-prefixed frame (4-byte big-endian length + payload).
async fn write_frame(stream: &mut quinn::SendStream, data: &[u8]) -> Result<(), String> {
    let len = (data.len() as u32).to_be_bytes();
    stream.write_all(&len).await.map_err(|e| e.to_string())?;
    stream.write_all(data).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Read a length-prefixed frame.
async fn read_frame(stream: &mut quinn::RecvStream) -> Result<Vec<u8>, String> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| format!("read len: {e}"))?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > 16 * 1024 * 1024 {
        return Err("frame too large (>16 MiB)".to_string());
    }
    let mut buf = vec![0u8; len];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|e| format!("read payload: {e}"))?;
    Ok(buf)
}

#[async_trait]
impl LinkTransport for QuicTransport {
    fn name(&self) -> &str {
        "QUIC"
    }

    async fn listen(&self, port: u16) -> Result<u16, String> {
        let server_config = build_server_config()?;
        let addr: SocketAddr = format!("0.0.0.0:{port}")
            .parse()
            .map_err(|e: std::net::AddrParseError| e.to_string())?;
        let endpoint = Endpoint::server(server_config, addr).map_err(|e| e.to_string())?;
        let bound_port = endpoint.local_addr().map_err(|e| e.to_string())?.port();
        *self.endpoint.lock().await = Some(endpoint);
        *self.status.lock().await = LinkStatus::Connecting;
        Ok(bound_port)
    }

    async fn connect(&self, addr: &PeerAddr) -> Result<(), String> {
        *self.status.lock().await = LinkStatus::Connecting;

        let mut endpoint =
            Endpoint::client("0.0.0.0:0".parse().unwrap()).map_err(|e| e.to_string())?;
        endpoint.set_default_client_config(build_client_config());

        let remote: SocketAddr = addr
            .to_string()
            .parse()
            .map_err(|e: std::net::AddrParseError| e.to_string())?;
        let connection = endpoint
            .connect(remote, "terransoul.local")
            .map_err(|e| e.to_string())?
            .await
            .map_err(|e| format!("QUIC connect: {e}"))?;

        let (send, recv) = connection
            .open_bi()
            .await
            .map_err(|e| format!("open_bi: {e}"))?;

        *self.send_stream.lock().await = Some(send);
        *self.recv_stream.lock().await = Some(recv);
        *self.endpoint.lock().await = Some(endpoint);
        *self.status.lock().await = LinkStatus::Connected;
        Ok(())
    }

    async fn send(&self, msg: &LinkMessage) -> Result<(), String> {
        let data = serde_json::to_vec(msg).map_err(|e| e.to_string())?;
        let mut lock = self.send_stream.lock().await;
        let stream = lock.as_mut().ok_or("not connected")?;
        write_frame(stream, &data).await
    }

    async fn recv(&self) -> Result<LinkMessage, String> {
        let mut lock = self.recv_stream.lock().await;
        let stream = lock.as_mut().ok_or("not connected")?;
        let data = read_frame(stream).await?;
        serde_json::from_slice(&data).map_err(|e| e.to_string())
    }

    async fn close(&self) -> Result<(), String> {
        // Drop streams
        *self.send_stream.lock().await = None;
        *self.recv_stream.lock().await = None;
        if let Some(ep) = self.endpoint.lock().await.take() {
            ep.close(0u32.into(), b"bye");
        }
        *self.status.lock().await = LinkStatus::Disconnected;
        Ok(())
    }

    fn status(&self) -> LinkStatus {
        // Sync access — we use try_lock and fall back to Disconnected.
        self.status
            .try_lock()
            .map(|s| *s)
            .unwrap_or(LinkStatus::Disconnected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_signed_cert_generation() {
        let result = generate_self_signed_cert();
        assert!(result.is_ok());
        let (certs, _key) = result.unwrap();
        assert_eq!(certs.len(), 1);
    }

    #[test]
    fn server_config_builds_successfully() {
        let result = build_server_config();
        assert!(result.is_ok());
    }

    #[test]
    fn client_config_builds_successfully() {
        let _config = build_client_config();
        // No panic = success
    }

    #[tokio::test]
    async fn quic_transport_starts_disconnected() {
        let transport = QuicTransport::new();
        assert_eq!(transport.status(), LinkStatus::Disconnected);
    }

    #[tokio::test]
    async fn quic_transport_name() {
        let transport = QuicTransport::new();
        assert_eq!(transport.name(), "QUIC");
    }

    #[tokio::test]
    async fn quic_listen_binds_port() {
        let transport = QuicTransport::new();
        let port = transport.listen(0).await.unwrap();
        assert!(port > 0);
        assert_eq!(transport.status(), LinkStatus::Connecting);
        transport.close().await.unwrap();
    }

    #[tokio::test]
    async fn quic_send_without_connection_fails() {
        let transport = QuicTransport::new();
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
    async fn quic_recv_without_connection_fails() {
        let transport = QuicTransport::new();
        let result = transport.recv().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not connected"));
    }

    #[tokio::test]
    async fn quic_close_from_disconnected_is_ok() {
        let transport = QuicTransport::new();
        let result = transport.close().await;
        assert!(result.is_ok());
        assert_eq!(transport.status(), LinkStatus::Disconnected);
    }
}
