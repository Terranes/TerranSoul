//! Tauri commands for the gRPC Brain server — Phase 24.3.
//!
//! Manages the lifecycle of the `brain.v1` gRPC server with optional
//! mTLS when LAN mode is enabled.

use std::net::SocketAddr;
use std::sync::Arc;

use tauri::State;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::ai_integrations::gateway::{AppStateGateway, BrainGateway, GatewayCaps};
use crate::ai_integrations::grpc;
use crate::AppState;

use serde::{Deserialize, Serialize};

/// Handle to a running gRPC server. Stored in `AppStateInner.grpc_server`.
pub struct GrpcServerHandle {
    shutdown_tx: watch::Sender<bool>,
    pub task: JoinHandle<()>,
    pub port: u16,
    pub tls_enabled: bool,
}

impl GrpcServerHandle {
    /// Signal the server to shut down gracefully.
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

/// Status returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcServerStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub tls_enabled: bool,
    pub lan_exposed: bool,
}

/// Start the gRPC Brain server.
///
/// - When `lan_enabled`: binds to `0.0.0.0:7422`, uses mTLS with the
///   pairing CA cert for client verification.
/// - When not: binds to `127.0.0.1:7422`, plaintext (loopback is safe).
#[tauri::command]
pub async fn grpc_server_start(state: State<'_, AppState>) -> Result<GrpcServerStatus, String> {
    let mut guard = state.grpc_server.lock().await;

    // Already running — return current status.
    if let Some(ref handle) = *guard {
        return Ok(GrpcServerStatus {
            running: true,
            port: Some(handle.port),
            tls_enabled: handle.tls_enabled,
            lan_exposed: handle.tls_enabled,
        });
    }

    let lan_enabled = state
        .app_settings
        .lock()
        .map_err(|e| e.to_string())?
        .lan_enabled;

    let port: u16 = 7422;
    let ip: [u8; 4] = if lan_enabled {
        [0, 0, 0, 0]
    } else {
        [127, 0, 0, 1]
    };
    let addr = SocketAddr::from((ip, port));

    // Build TLS config if LAN mode.
    let tls = if lan_enabled {
        // Lazily init pairing manager to get certs.
        {
            let mut pmgr = state.pairing_manager.lock().map_err(|e| e.to_string())?;
            if pmgr.is_none() {
                let mgr = crate::network::pairing::PairingManager::load_or_create(&state.data_dir)?;
                *pmgr = Some(mgr);
            }
        }

        let pairing_mgr = state.pairing_manager.lock().map_err(|e| e.to_string())?;
        let mgr = pairing_mgr.as_ref().unwrap();
        let ca_cert_pem = mgr.ca_cert_pem().to_string();

        // Issue a server cert from the same CA for TLS.
        let (server_cert_pem, server_key_pem) = mgr.issue_server_cert()?;

        let tls_cfg = grpc::tls_config_from_pem(
            server_cert_pem.as_bytes(),
            server_key_pem.as_bytes(),
            Some(ca_cert_pem.as_bytes()),
        );
        Some(tls_cfg)
    } else {
        None
    };

    let tls_enabled = tls.is_some();

    let gw: Arc<dyn BrainGateway> = Arc::new(AppStateGateway::new(state.inner().clone()));
    let caps = GatewayCaps::default();
    let app_state_clone = state.inner().clone();

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    let task = tokio::spawn(async move {
        let shutdown_fut = async move {
            let _ = shutdown_rx.wait_for(|v| *v).await;
        };
        if let Err(e) =
            grpc::serve_with_shutdown(addr, gw, caps, tls, shutdown_fut, Some(app_state_clone))
                .await
        {
            eprintln!("[grpc] server error: {e}");
        }
    });

    let handle = GrpcServerHandle {
        shutdown_tx,
        task,
        port,
        tls_enabled,
    };

    let status = GrpcServerStatus {
        running: true,
        port: Some(handle.port),
        tls_enabled: handle.tls_enabled,
        lan_exposed: handle.tls_enabled,
    };

    *guard = Some(handle);
    Ok(status)
}

/// Stop the gRPC Brain server.
#[tauri::command]
pub async fn grpc_server_stop(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.grpc_server.lock().await;
    if let Some(handle) = guard.take() {
        handle.stop();
        // Give it a moment to shut down gracefully.
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle.task).await;
    }
    Ok(())
}

/// Get gRPC server status.
#[tauri::command]
pub async fn grpc_server_status(state: State<'_, AppState>) -> Result<GrpcServerStatus, String> {
    let guard = state.grpc_server.lock().await;
    match guard.as_ref() {
        Some(handle) => Ok(GrpcServerStatus {
            running: true,
            port: Some(handle.port),
            tls_enabled: handle.tls_enabled,
            lan_exposed: handle.tls_enabled,
        }),
        None => Ok(GrpcServerStatus {
            running: false,
            port: None,
            tls_enabled: false,
            lan_exposed: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grpc_server_handle_stop_does_not_panic() {
        let (tx, _rx) = watch::channel(false);
        let task = tokio::runtime::Runtime::new()
            .unwrap()
            .spawn(async { /* no-op */ });
        let handle = GrpcServerHandle {
            shutdown_tx: tx,
            task,
            port: 7422,
            tls_enabled: false,
        };
        handle.stop(); // Should not panic.
    }
}
