//! DB-backed verbose tool-output offload (CTX-OFFLOAD-1a).
//!
//! Inspired by Tencent/TencentDB-Agent-Memory's approach of pushing
//! large agent-loop payloads into the durable memory layer instead of
//! the per-session filesystem. The existing `coding::offload` module
//! does filesystem spill (which is fine for one-off shell output) but
//! the brain has no record. This module gives the brain a sidecar
//! payload table keyed by `memory_id` so verbose tool results can be
//! drilled back into via the same `brain_drilldown_payload` surface
//! the rest of the agent uses for provenance walks.
//!
//! Schema lives in `schema.rs` (V23). This module is the
//! `MemoryStore` API + tests only.

use rusqlite::{params, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};

use crate::memory::store::MemoryStore;

/// One row in `memory_offload_payloads`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffloadPayload {
    pub memory_id: i64,
    /// Raw payload bytes. For non-binary content (e.g. JSON tool
    /// output), this is the UTF-8 representation.
    pub payload: Vec<u8>,
    /// Authoritative byte length. Always equal to `payload.len()` when
    /// fetched whole; useful because the matching `memories` row only
    /// records a summary, not the size.
    pub byte_count: i64,
    /// Best-effort MIME hint. Defaults to `text/plain`. Producers
    /// should set `application/json`, `text/markdown`, etc. when
    /// known.
    pub mime_type: String,
    /// Unix-ms when the payload was first written.
    pub created_at: i64,
}

/// Compact summary returned alongside the bytes for transport
/// surfaces (MCP, Tauri) that want to base64-encode the payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffloadPayloadInfo {
    pub memory_id: i64,
    pub byte_count: i64,
    pub mime_type: String,
    pub created_at: i64,
}

impl OffloadPayload {
    pub fn info(&self) -> OffloadPayloadInfo {
        OffloadPayloadInfo {
            memory_id: self.memory_id,
            byte_count: self.byte_count,
            mime_type: self.mime_type.clone(),
            created_at: self.created_at,
        }
    }
}

impl MemoryStore {
    /// Insert or replace the offload payload for `memory_id`. The
    /// caller is responsible for first creating the lightweight
    /// `memories` row carrying the summary; this method does not
    /// validate that the memory exists (the FK on
    /// `memory_offload_payloads.memory_id` will reject orphan writes).
    ///
    /// Stores `payload.len()` as `byte_count` so listing endpoints
    /// don't have to materialise the BLOB.
    pub fn add_offload_payload(
        &self,
        memory_id: i64,
        payload: &[u8],
        mime_type: &str,
    ) -> SqlResult<OffloadPayload> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let byte_count = payload.len() as i64;
        let conn = self.conn();
        conn.execute(
            "INSERT OR REPLACE INTO memory_offload_payloads
                (memory_id, payload, byte_count, mime_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![memory_id, payload, byte_count, mime_type, now],
        )?;
        Ok(OffloadPayload {
            memory_id,
            payload: payload.to_vec(),
            byte_count,
            mime_type: mime_type.to_string(),
            created_at: now,
        })
    }

    /// Fetch the full payload for `memory_id`, or `None` if no row.
    pub fn get_offload_payload(&self, memory_id: i64) -> SqlResult<Option<OffloadPayload>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT memory_id, payload, byte_count, mime_type, created_at
             FROM memory_offload_payloads WHERE memory_id = ?1",
            params![memory_id],
            |row| {
                Ok(OffloadPayload {
                    memory_id: row.get(0)?,
                    payload: row.get(1)?,
                    byte_count: row.get(2)?,
                    mime_type: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .optional()
    }

    /// Cheap "is there a payload?" check — does not materialise the
    /// BLOB. Returns the metadata row only.
    pub fn get_offload_payload_info(
        &self,
        memory_id: i64,
    ) -> SqlResult<Option<OffloadPayloadInfo>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT memory_id, byte_count, mime_type, created_at
             FROM memory_offload_payloads WHERE memory_id = ?1",
            params![memory_id],
            |row| {
                Ok(OffloadPayloadInfo {
                    memory_id: row.get(0)?,
                    byte_count: row.get(1)?,
                    mime_type: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
        .optional()
    }

    /// Delete the offload payload for `memory_id`. Returns true if a
    /// row was actually removed.
    pub fn delete_offload_payload(&self, memory_id: i64) -> SqlResult<bool> {
        let conn = self.conn();
        let n = conn.execute(
            "DELETE FROM memory_offload_payloads WHERE memory_id = ?1",
            params![memory_id],
        )?;
        Ok(n > 0)
    }

    /// Total bytes currently held in the offload table — useful for
    /// the memory-pressure / housekeeping UI.
    pub fn offload_payload_total_bytes(&self) -> SqlResult<i64> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COALESCE(SUM(byte_count), 0) FROM memory_offload_payloads",
            [],
            |row| row.get(0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryType, NewMemory};
    use std::sync::atomic::{AtomicI64, Ordering};

    /// `MemoryStore::add` dedupes by content hash, so each test helper
    /// call must produce a unique content string — otherwise both
    /// "memories" collapse to the same row and the payload table only
    /// gets one entry under the dedup'd id.
    fn make_memory(store: &MemoryStore) -> i64 {
        static COUNTER: AtomicI64 = AtomicI64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        store
            .add(NewMemory {
                content: format!("offload-test-memory-{}", n),
                tags: "tool_output_ref".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap()
            .id
    }

    #[test]
    fn roundtrip_payload() {
        let store = MemoryStore::in_memory();
        let m = make_memory(&store);
        let bytes = b"large tool output payload";
        let written = store
            .add_offload_payload(m, bytes, "text/plain")
            .unwrap();
        assert_eq!(written.byte_count, bytes.len() as i64);
        assert_eq!(written.mime_type, "text/plain");

        let fetched = store.get_offload_payload(m).unwrap().unwrap();
        assert_eq!(fetched.payload, bytes);
        assert_eq!(fetched.byte_count, bytes.len() as i64);
        assert_eq!(fetched.mime_type, "text/plain");
    }

    #[test]
    fn missing_payload_returns_none() {
        let store = MemoryStore::in_memory();
        let m = make_memory(&store);
        assert!(store.get_offload_payload(m).unwrap().is_none());
        assert!(store.get_offload_payload_info(m).unwrap().is_none());
    }

    #[test]
    fn info_avoids_blob_but_matches_full_fetch() {
        let store = MemoryStore::in_memory();
        let m = make_memory(&store);
        let bytes = vec![0u8; 16 * 1024]; // 16 KiB
        store
            .add_offload_payload(m, &bytes, "application/octet-stream")
            .unwrap();
        let info = store.get_offload_payload_info(m).unwrap().unwrap();
        assert_eq!(info.byte_count, 16 * 1024);
        assert_eq!(info.mime_type, "application/octet-stream");
        let full = store.get_offload_payload(m).unwrap().unwrap();
        assert_eq!(full.info().byte_count, info.byte_count);
        assert_eq!(full.info().mime_type, info.mime_type);
        assert_eq!(full.info().created_at, info.created_at);
    }

    #[test]
    fn replace_overwrites_payload_and_updates_size() {
        let store = MemoryStore::in_memory();
        let m = make_memory(&store);
        store.add_offload_payload(m, b"short", "text/plain").unwrap();
        let bigger = b"a much longer payload that replaces the previous one";
        store
            .add_offload_payload(m, bigger, "text/markdown")
            .unwrap();
        let fetched = store.get_offload_payload(m).unwrap().unwrap();
        assert_eq!(fetched.payload, bigger);
        assert_eq!(fetched.byte_count, bigger.len() as i64);
        assert_eq!(fetched.mime_type, "text/markdown");
    }

    #[test]
    fn delete_removes_payload() {
        let store = MemoryStore::in_memory();
        let m = make_memory(&store);
        store.add_offload_payload(m, b"bytes", "text/plain").unwrap();
        assert!(store.delete_offload_payload(m).unwrap());
        assert!(store.get_offload_payload(m).unwrap().is_none());
        // Second delete returns false (idempotent).
        assert!(!store.delete_offload_payload(m).unwrap());
    }

    #[test]
    fn memory_delete_cascades_to_payload() {
        let store = MemoryStore::in_memory();
        let m = make_memory(&store);
        store.add_offload_payload(m, b"bytes", "text/plain").unwrap();
        store.delete(m).unwrap();
        assert!(store.get_offload_payload(m).unwrap().is_none());
    }

    #[test]
    fn total_bytes_sums_across_rows() {
        let store = MemoryStore::in_memory();
        let m1 = make_memory(&store);
        let m2 = make_memory(&store);
        store
            .add_offload_payload(m1, &[0u8; 100], "text/plain")
            .unwrap();
        store
            .add_offload_payload(m2, &[0u8; 250], "text/plain")
            .unwrap();
        assert_eq!(store.offload_payload_total_bytes().unwrap(), 350);
    }
}
