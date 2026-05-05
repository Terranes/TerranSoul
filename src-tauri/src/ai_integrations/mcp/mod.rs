//! MCP server — Chunk 15.1.
//!
//! Exposes TerranSoul's brain to AI coding assistants (GitHub Copilot,
//! Claude Desktop, Cursor, Codex, …) via an MCP-compatible HTTP server
//! on `127.0.0.1:7421`.
//!
//! Architecture:
//! - **Transport**: Streamable HTTP (POST `/mcp`) on axum — the
//!   milestones-endorsed fallback when `rmcp`'s SSE combo isn't needed.
//! - **Protocol**: JSON-RPC 2.0 per MCP 2024-11-05 spec.
//! - **Auth**: Bearer token from `<data_dir>/mcp-token.txt`.
//! - **Ops**: 8 tools routed to [`crate::ai_integrations::gateway::BrainGateway`].
//!
//! The stdio transport ([`stdio`]) reuses the same dispatch surface
//! over newline-delimited JSON-RPC on stdin/stdout. See Chunk 15.9.

pub mod activity;
pub mod auth;
pub mod auto_setup;
pub mod hooks;
pub mod router;
pub mod self_host;
pub mod stdio;
pub mod tools;

#[cfg(test)]
mod integration_tests;

use std::sync::Arc;

use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::ai_integrations::gateway::{AppStateGateway, GatewayCaps};
use crate::AppState;

use router::McpRouterState;

/// Default port for the MCP HTTP server (release builds).
pub const DEFAULT_PORT: u16 = 7421;

/// Default port for the MCP HTTP server (dev builds).
/// Separate from release so both can run simultaneously without conflict.
pub const DEFAULT_DEV_PORT: u16 = 7422;

/// Number of fallback ports to try if the primary port is taken.
const PORT_FALLBACK_RANGE: u16 = 10;

/// Returns the default MCP port for the current build profile.
/// Debug/dev builds use 7422, release builds use 7421.
pub fn default_port() -> u16 {
    if cfg!(debug_assertions) {
        DEFAULT_DEV_PORT
    } else {
        DEFAULT_PORT
    }
}

/// Whether this is a dev/debug build.
pub fn is_dev_build() -> bool {
    cfg!(debug_assertions)
}

/// Runtime flag set when the process is running as the headless
/// `--mcp-http` server (a.k.a. "MCP pet mode"). When true, the
/// initialize handshake reports `serverInfo.name = "terransoul-brain-mcp"`
/// and `buildMode = "mcp"` instead of the dev/release labels — so
/// external agents (Copilot, Codex, Claude Code, Clawcode, etc.) can
/// tell at a glance that they are talking to the repo-local headless
/// server, not a running app.
static MCP_PET_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Mark the current process as running in MCP pet mode. Called by
/// `terransoul_lib::run_http_server` before the server starts.
pub fn enable_mcp_pet_mode() {
    MCP_PET_MODE.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the current process is running as the headless MCP server.
pub fn is_mcp_pet_mode() -> bool {
    MCP_PET_MODE.load(std::sync::atomic::Ordering::Relaxed)
}

fn seed_loaded_from_state(state: &AppState) -> bool {
    if !is_mcp_pet_mode() {
        return false;
    }
    state
        .memory_store
        .lock()
        .ok()
        .and_then(|store| store.stats().ok())
        .is_some_and(|stats| stats.total > 0)
}

/// Handle to a running MCP server. Stored in
/// `AppStateInner.mcp_server`.
pub struct McpServerHandle {
    shutdown: watch::Sender<bool>,
    /// Public so the stop command can await graceful shutdown.
    pub task: JoinHandle<()>,
    pub port: u16,
    pub token: String,
}

impl McpServerHandle {
    /// Signal the server to shut down gracefully.
    pub fn stop(&self) {
        let _ = self.shutdown.send(true);
    }
}

/// Start the MCP HTTP server on the given port.
///
/// If the port is already in use, tries up to [`PORT_FALLBACK_RANGE`]
/// consecutive ports (e.g. 7421, 7423, 7425…) before failing. This
/// prevents dev and release builds from stealing each other's port.
///
/// When `lan_enabled` is `true`, binds to `0.0.0.0` (all interfaces)
/// instead of `127.0.0.1` (loopback only).
///
/// Returns a handle that can be stored in `AppState.mcp_server` and
/// used to query status or stop the server.
pub async fn start_server(
    state: AppState,
    port: u16,
    token: String,
    lan_enabled: bool,
) -> Result<McpServerHandle, String> {
    start_server_with_activity(state, port, token, lan_enabled, None).await
}

/// Start the MCP HTTP server and optionally emit live activity events to
/// the Tauri frontend.
pub async fn start_server_with_activity(
    state: AppState,
    port: u16,
    token: String,
    lan_enabled: bool,
    app: Option<tauri::AppHandle>,
) -> Result<McpServerHandle, String> {
    let activity = activity::McpActivityReporter::new(state.clone(), app);
    activity.startup(format!("Starting MCP brain server on port {port}."));
    let gw = Arc::new(AppStateGateway::new(state.clone()))
        as Arc<dyn crate::ai_integrations::gateway::BrainGateway>;
    let caps = GatewayCaps {
        brain_read: true,
        brain_write: false,
        code_read: true,
    };

    // Try the requested port first, then fallbacks.
    let mut last_err = String::new();
    let mut bound_port = port;
    let mut listener_opt = None;

    for offset in 0..=PORT_FALLBACK_RANGE {
        let try_port = port.saturating_add(offset);
        let ip = if lan_enabled {
            [0, 0, 0, 0]
        } else {
            [127, 0, 0, 1]
        };
        let addr = std::net::SocketAddr::from((ip, try_port));
        match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => {
                bound_port = try_port;
                listener_opt = Some(l);
                break;
            }
            Err(e) => {
                last_err = format!("port {try_port}: {e}");
                // If this is AddrInUse, try next; otherwise it's a real error
                if e.kind() != std::io::ErrorKind::AddrInUse {
                    let message = format!("failed to bind MCP server on {addr}: {e}");
                    activity.failed(message.clone());
                    return Err(message);
                }
            }
        }
    }

    let listener = listener_opt.ok_or_else(|| {
        let message = format!(
            "failed to bind MCP server: ports {port}–{} all in use ({last_err})",
            port.saturating_add(PORT_FALLBACK_RANGE)
        );
        activity.failed(message.clone());
        message
    })?;

    if bound_port != port {
        eprintln!("[mcp] primary port {port} in use, bound to fallback port {bound_port}");
    }

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    activity.ready(bound_port);
    let server_activity = activity.clone();
    let seed_loaded = seed_loaded_from_state(&state);
    let router_state = McpRouterState {
        gw,
        caps,
        token: token.clone(),
        port: bound_port,
        seed_loaded,
        activity: Some(activity.clone()),
        app_state: Some(state),
        staleness_tracker: Arc::new(tokio::sync::Mutex::new(hooks::IndexStalenessTracker::new())),
    };
    let app = router::build(router_state);

    let task = tokio::spawn(async move {
        let server = axum::serve(listener, app);
        tokio::select! {
            result = server => {
                if let Err(e) = result {
                    server_activity.failed(format!("MCP server error: {e}"));
                    eprintln!("[mcp] server error: {e}");
                }
            }
            _ = async {
                while !*shutdown_rx.borrow_and_update() {
                    shutdown_rx.changed().await.ok();
                }
            } => {
                // Graceful shutdown requested.
            }
        }
    });

    Ok(McpServerHandle {
        shutdown: shutdown_tx,
        task,
        port: bound_port,
        token,
    })
}
