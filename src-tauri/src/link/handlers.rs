//! Soul Link message handlers + background receive loop (Chunk 17.5b).
//!
//! Dispatches inbound `LinkMessage`s by `kind` field. The `memory_sync`
//! handler applies CRDT deltas and optionally responds with local deltas
//! for bidirectional sync. `edge_sync` handles KG edge CRDT replication
//! (Chunk 42.5).

use super::{LinkMessage, LinkStatus, PeerAddr};
use crate::memory::crdt_sync::SyncDelta;
use crate::memory::edge_crdt_sync::EdgeSyncDelta;
use crate::memory::embedding_queue;
use crate::AppState;

/// Dispatch an inbound LinkMessage to the appropriate handler.
///
/// Returns an optional response message to send back to the peer.
pub async fn dispatch_link_message(
    msg: LinkMessage,
    state: &AppState,
) -> Result<Option<LinkMessage>, String> {
    match msg.kind.as_str() {
        "memory_sync" => handle_memory_sync(&msg, state).await,
        "memory_sync_request" => handle_memory_sync_request(&msg, state).await,
        "edge_sync" => handle_edge_sync(&msg, state).await,
        "ping" => Ok(Some(LinkMessage {
            id: uuid::Uuid::new_v4().to_string(),
            origin: get_device_id(state),
            target: msg.origin.clone(),
            kind: "pong".into(),
            payload: serde_json::json!({}),
        })),
        _ => Ok(None), // Unknown kind — ignore silently.
    }
}

/// Handle inbound memory_sync deltas from a peer.
///
/// Applies the deltas via LWW, logs the sync, and responds with local
/// deltas that the peer doesn't have yet (bidirectional push).
async fn handle_memory_sync(
    msg: &LinkMessage,
    state: &AppState,
) -> Result<Option<LinkMessage>, String> {
    let deltas: Vec<SyncDelta> = serde_json::from_value(msg.payload.clone())
        .map_err(|e| format!("invalid memory_sync payload: {e}"))?;

    let local_device_id = get_device_id(state);
    let response_deltas: Vec<SyncDelta>;

    {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        let since = store
            .last_sync_time(&msg.origin)
            .map_err(|e| e.to_string())?
            .unwrap_or(0);
        response_deltas = store
            .compute_sync_deltas(since, &local_device_id)
            .map_err(|e| e.to_string())?;

        let result = store
            .apply_sync_deltas(&deltas, &local_device_id)
            .map_err(|e| e.to_string())?;
        let total = result.inserted + result.updated + result.soft_closed;
        store
            .log_sync(&msg.origin, "inbound", total)
            .map_err(|e| e.to_string())?;

        // Enqueue newly synced entries for embedding so the ANN index
        // picks them up on the next embed worker tick (multi-device fix).
        if total > 0 {
            let _ = embedding_queue::backfill_queue(store.conn());
        }
    }

    if response_deltas.is_empty() {
        return Ok(None);
    }

    // Respond with our deltas.
    Ok(Some(LinkMessage {
        id: uuid::Uuid::new_v4().to_string(),
        origin: local_device_id,
        target: msg.origin.clone(),
        kind: "memory_sync".into(),
        payload: serde_json::to_value(&response_deltas).unwrap_or(serde_json::json!([])),
    }))
}

/// Handle a sync request — peer is asking for our deltas since a timestamp.
async fn handle_memory_sync_request(
    msg: &LinkMessage,
    state: &AppState,
) -> Result<Option<LinkMessage>, String> {
    let since: i64 = msg
        .payload
        .get("since")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let local_device_id = get_device_id(state);
    let deltas = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        store
            .compute_sync_deltas(since, &local_device_id)
            .map_err(|e| e.to_string())?
    };

    if deltas.is_empty() {
        return Ok(None);
    }

    Ok(Some(LinkMessage {
        id: uuid::Uuid::new_v4().to_string(),
        origin: local_device_id,
        target: msg.origin.clone(),
        kind: "memory_sync".into(),
        payload: serde_json::to_value(&deltas).unwrap_or(serde_json::json!([])),
    }))
}

/// Handle inbound edge_sync deltas from a peer (Chunk 42.5).
///
/// Applies edge CRDT deltas via HLC-based LWW and responds with local
/// edge deltas the peer doesn't have.
async fn handle_edge_sync(
    msg: &LinkMessage,
    state: &AppState,
) -> Result<Option<LinkMessage>, String> {
    let deltas: Vec<EdgeSyncDelta> = serde_json::from_value(msg.payload.clone())
        .map_err(|e| format!("invalid edge_sync payload: {e}"))?;

    let local_device_id = get_device_id(state);
    let response_deltas: Vec<EdgeSyncDelta>;

    {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        // Use HLC=0 to get all edge deltas for now (cursor-based in future).
        response_deltas = store
            .compute_edge_sync_deltas(0, &local_device_id)
            .map_err(|e| e.to_string())?;

        store
            .apply_edge_sync_deltas(&deltas, &local_device_id)
            .map_err(|e| e.to_string())?;
    }

    if response_deltas.is_empty() {
        return Ok(None);
    }

    Ok(Some(LinkMessage {
        id: uuid::Uuid::new_v4().to_string(),
        origin: local_device_id,
        target: msg.origin.clone(),
        kind: "edge_sync".into(),
        payload: serde_json::to_value(&response_deltas).unwrap_or(serde_json::json!([])),
    }))
}

/// Start the background receive loop for Soul Link messages.
///
/// This spawns a tokio task that:
/// 1. Calls `mgr.recv()` in a loop.
/// 2. Dispatches each message via `dispatch_link_message`.
/// 3. Sends back any response messages.
/// 4. On disconnect, triggers auto-sync on reconnect.
///
/// The task exits when the link disconnects and reconnection fails,
/// or when the link manager is dropped.
pub fn start_receive_loop(state: AppState) {
    tokio::spawn(async move {
        if let Err(e) = trigger_sync(&state).await {
            eprintln!("[soul-link] initial memory sync failed: {e}");
        }

        loop {
            let msg = {
                let mgr = state.link_manager.lock().await;
                if mgr.status() != LinkStatus::Connected {
                    break;
                }
                mgr.recv().await
            };

            match msg {
                Ok(message) => match dispatch_link_message(message, &state).await {
                    Ok(Some(response)) => {
                        let mgr = state.link_manager.lock().await;
                        let _ = mgr.send(&response).await;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("[soul-link] handler error: {e}");
                    }
                },
                Err(_) => {
                    if let Err(e) = reconnect_current_peer(&state).await {
                        eprintln!("[soul-link] reconnect failed: {e}");
                        break;
                    }
                    if let Err(e) = trigger_sync(&state).await {
                        eprintln!("[soul-link] memory sync after reconnect failed: {e}");
                    }
                }
            }
        }
    });
}

/// Trigger a full memory + edge sync with the connected peer (Chunk 42.5).
///
/// Sends all local memory deltas since the last sync as a `memory_sync`
/// message, then sends edge deltas as an `edge_sync` message.
pub async fn trigger_sync(state: &AppState) -> Result<(), String> {
    let peer_device_id = {
        let mgr = state.link_manager.lock().await;
        match mgr.connected_peer().await {
            Some(peer) => peer.device_id,
            None => return Err("No peer connected".into()),
        }
    };

    let local_device_id = get_device_id(state);
    let (memory_deltas, edge_deltas) = {
        let store = state.memory_store.lock().map_err(|e| e.to_string())?;
        let since = store
            .last_sync_time(&peer_device_id)
            .unwrap_or(Some(0))
            .unwrap_or(0);
        let mem = store
            .compute_sync_deltas(since, &local_device_id)
            .map_err(|e| e.to_string())?;
        // Edge deltas: use HLC=0 for initial sync (full push).
        let edges = store
            .compute_edge_sync_deltas(0, &local_device_id)
            .map_err(|e| e.to_string())?;
        (mem, edges)
    };

    let mgr = state.link_manager.lock().await;

    // Send memory deltas.
    if !memory_deltas.is_empty() {
        let msg = LinkMessage {
            id: uuid::Uuid::new_v4().to_string(),
            origin: local_device_id.clone(),
            target: peer_device_id.clone(),
            kind: "memory_sync".into(),
            payload: serde_json::to_value(&memory_deltas).unwrap_or(serde_json::json!([])),
        };
        mgr.send(&msg).await?;
    }

    // Send edge deltas.
    if !edge_deltas.is_empty() {
        let msg = LinkMessage {
            id: uuid::Uuid::new_v4().to_string(),
            origin: local_device_id.clone(),
            target: peer_device_id.clone(),
            kind: "edge_sync".into(),
            payload: serde_json::to_value(&edge_deltas).unwrap_or(serde_json::json!([])),
        };
        mgr.send(&msg).await?;
    }

    // Log outbound sync.
    drop(mgr);
    let store = state.memory_store.lock().map_err(|e| e.to_string())?;
    let _ = store.log_sync(
        &peer_device_id,
        "outbound",
        memory_deltas.len() + edge_deltas.len(),
    );
    Ok(())
}

async fn reconnect_current_peer(state: &AppState) -> Result<(), String> {
    let peer = {
        let mgr = state.link_manager.lock().await;
        mgr.connected_peer()
            .await
            .ok_or_else(|| "No peer connected".to_string())?
    };
    let addr = parse_peer_addr(&peer.addr)?;
    let mgr = state.link_manager.lock().await;
    mgr.reconnect(&addr).await
}

fn parse_peer_addr(addr: &str) -> Result<PeerAddr, String> {
    let (host, port) = addr
        .rsplit_once(':')
        .ok_or_else(|| format!("invalid peer address: {addr}"))?;
    let port = port
        .parse::<u16>()
        .map_err(|e| format!("invalid peer address port: {e}"))?;
    Ok(PeerAddr {
        host: host.to_string(),
        port,
    })
}

fn get_device_id(state: &AppState) -> String {
    state
        .device_identity
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|id| id.device_id.clone()))
        .unwrap_or_else(|| "unknown".into())
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_returns_none_for_unknown_kind() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let state = AppState::for_test();
        let msg = LinkMessage {
            id: "1".into(),
            origin: "peer-a".into(),
            target: "local".into(),
            kind: "unknown_kind".into(),
            payload: serde_json::json!({}),
        };
        let result = rt.block_on(dispatch_link_message(msg, &state));
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn dispatch_ping_returns_pong() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let state = AppState::for_test();
        let msg = LinkMessage {
            id: "2".into(),
            origin: "peer-a".into(),
            target: "local".into(),
            kind: "ping".into(),
            payload: serde_json::json!({}),
        };
        let result = rt.block_on(dispatch_link_message(msg, &state)).unwrap();
        assert!(result.is_some());
        let resp = result.unwrap();
        assert_eq!(resp.kind, "pong");
        assert_eq!(resp.target, "peer-a");
    }

    #[test]
    fn dispatch_memory_sync_applies_deltas() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let state = AppState::for_test();

        let deltas = vec![SyncDelta {
            key: crate::memory::crdt_sync::SyncKey {
                content_hash: Some("test-hash-sync".into()),
                source_url: None,
                content_prefix: None,
                created_at: 100,
            },
            operation: crate::memory::crdt_sync::SyncOp::Upsert,
            content: "Synced from peer".into(),
            tags: "synced".into(),
            importance: 3,
            memory_type: "fact".into(),
            created_at: 100,
            updated_at: 2000,
            origin_device: "peer-a".into(),
            source_url: None,
            source_hash: Some("test-hash-sync".into()),
            hlc_counter: 0,
        }];

        let msg = LinkMessage {
            id: "3".into(),
            origin: "peer-a".into(),
            target: "local".into(),
            kind: "memory_sync".into(),
            payload: serde_json::to_value(&deltas).unwrap(),
        };

        let _result = rt.block_on(dispatch_link_message(msg, &state)).unwrap();
        // Response may or may not exist (depends on whether we have deltas to send back).
        // But the memory should be inserted.
        let store = state.memory_store.lock().unwrap();
        let all = store.get_all().unwrap();
        assert!(
            all.iter().any(|e| e.content == "Synced from peer"),
            "synced memory should be inserted"
        );
    }

    #[test]
    fn dispatch_memory_sync_request_returns_deltas() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let state = AppState::for_test();

        // Add a local memory.
        {
            let store = state.memory_store.lock().unwrap();
            store
                .add(crate::memory::NewMemory {
                    content: "Local fact".into(),
                    tags: "test".into(),
                    importance: 3,
                    memory_type: crate::memory::MemoryType::Fact,
                    ..Default::default()
                })
                .unwrap();
        }

        let msg = LinkMessage {
            id: "4".into(),
            origin: "peer-b".into(),
            target: "local".into(),
            kind: "memory_sync_request".into(),
            payload: serde_json::json!({ "since": 0 }),
        };

        let result = rt.block_on(dispatch_link_message(msg, &state)).unwrap();
        assert!(result.is_some());
        let resp = result.unwrap();
        assert_eq!(resp.kind, "memory_sync");
        let resp_deltas: Vec<SyncDelta> = serde_json::from_value(resp.payload).unwrap();
        assert!(!resp_deltas.is_empty());
    }

    #[test]
    fn dispatch_memory_sync_replies_with_pre_apply_local_deltas() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let state = AppState::for_test();

        {
            let store = state.memory_store.lock().unwrap();
            store
                .add(crate::memory::NewMemory {
                    content: "Local unsynced fact".into(),
                    tags: "local".into(),
                    importance: 3,
                    memory_type: crate::memory::MemoryType::Fact,
                    source_hash: Some("local-unsynced".into()),
                    ..Default::default()
                })
                .unwrap();
        }

        let deltas = vec![SyncDelta {
            key: crate::memory::crdt_sync::SyncKey {
                content_hash: Some("remote-unsynced".into()),
                source_url: None,
                content_prefix: None,
                created_at: 100,
            },
            operation: crate::memory::crdt_sync::SyncOp::Upsert,
            content: "Remote fact from peer".into(),
            tags: "remote".into(),
            importance: 3,
            memory_type: "fact".into(),
            created_at: 100,
            updated_at: 2000,
            origin_device: "peer-a".into(),
            source_url: None,
            source_hash: Some("remote-unsynced".into()),
            hlc_counter: 0,
        }];

        let msg = LinkMessage {
            id: "5".into(),
            origin: "peer-a".into(),
            target: "local".into(),
            kind: "memory_sync".into(),
            payload: serde_json::to_value(&deltas).unwrap(),
        };

        let response = rt.block_on(dispatch_link_message(msg, &state)).unwrap();
        let response = response.expect("local deltas should be returned to peer");
        let response_deltas: Vec<SyncDelta> = serde_json::from_value(response.payload).unwrap();
        assert!(response_deltas
            .iter()
            .any(|delta| delta.content == "Local unsynced fact"));
        assert!(!response_deltas
            .iter()
            .any(|delta| delta.content == "Remote fact from peer"));
    }

    #[test]
    fn memory_sync_request_returns_none_when_peer_is_current() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let state = AppState::for_test();
        let msg = LinkMessage {
            id: "6".into(),
            origin: "peer-b".into(),
            target: "local".into(),
            kind: "memory_sync_request".into(),
            payload: serde_json::json!({ "since": i64::MAX }),
        };

        let result = rt.block_on(dispatch_link_message(msg, &state)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn parse_peer_addr_splits_host_and_port() {
        let addr = parse_peer_addr("127.0.0.1:7422").unwrap();
        assert_eq!(addr.host, "127.0.0.1");
        assert_eq!(addr.port, 7422);
        assert!(parse_peer_addr("missing-port").is_err());
    }

    #[test]
    fn dispatch_edge_sync_applies_edge_deltas() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let state = AppState::for_test();

        // First insert memories so edge FK targets exist.
        {
            let store = state.memory_store.lock().unwrap();
            store
                .add(crate::memory::store::NewMemory {
                    content: "Memory A".into(),
                    tags: "test".into(),
                    importance: 3,
                    memory_type: crate::memory::store::MemoryType::Fact,
                    ..Default::default()
                })
                .unwrap();
            store
                .add(crate::memory::store::NewMemory {
                    content: "Memory B".into(),
                    tags: "test".into(),
                    importance: 3,
                    memory_type: crate::memory::store::MemoryType::Fact,
                    ..Default::default()
                })
                .unwrap();
        }

        let deltas = vec![EdgeSyncDelta {
            src_id: 1,
            dst_id: 2,
            rel_type: "related_to".into(),
            confidence: 0.9,
            source: "llm".into(),
            created_at: 1000,
            valid_from: None,
            valid_to: None,
            edge_source: None,
            origin_device: "peer-a".into(),
            hlc_counter: 5,
        }];

        let msg = LinkMessage {
            id: "7".into(),
            origin: "peer-a".into(),
            target: "local".into(),
            kind: "edge_sync".into(),
            payload: serde_json::to_value(&deltas).unwrap(),
        };

        let result = rt.block_on(dispatch_link_message(msg, &state)).unwrap();
        // Should respond with local edge deltas (empty since we have none).
        assert!(result.is_none());

        // Verify edge was inserted.
        let store = state.memory_store.lock().unwrap();
        let edges = store
            .get_edges_for(1, crate::memory::edges::EdgeDirection::Both)
            .unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].rel_type, "related_to");
    }
}
