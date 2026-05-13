//! 2P-Set CRDT sync for knowledge-graph edges (Chunk 42.4).
//!
//! Edges form an Observed-Remove-Set keyed by `(src_id, dst_id, rel_type)`.
//! The `valid_to` column is the natural tombstone:
//! - `valid_to = None` → edge is live (in the add-set).
//! - `valid_to = Some(ts)` → edge is tombstoned (in the remove-set).
//!
//! Sync uses the same HLC-based LWW resolution as memory rows:
//! - Higher `(hlc_counter, origin_device)` wins.
//! - Re-adds after a remove are valid if the add's HLC > remove's HLC.
//!
//! Compaction: edges with `valid_to < now - retention` are hard-deleted
//! during the maintenance scheduler to bound storage growth.

use rusqlite::{params, Result as SqlResult};
use serde::{Deserialize, Serialize};

use super::store::MemoryStore;

/// Default retention period for tombstoned edges: 30 days in milliseconds.
const TOMBSTONE_RETENTION_MS: i64 = 30 * 24 * 60 * 60 * 1000;

/// A single edge delta for cross-device sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeSyncDelta {
    /// Source memory content_hash or id-based key.
    pub src_id: i64,
    pub dst_id: i64,
    pub rel_type: String,
    pub confidence: f64,
    pub source: String,
    pub created_at: i64,
    pub valid_from: Option<i64>,
    /// `Some(ts)` means this edge was removed (tombstoned) at `ts`.
    pub valid_to: Option<i64>,
    pub edge_source: Option<String>,
    pub origin_device: String,
    pub hlc_counter: u64,
}

/// Result of applying edge deltas.
#[derive(Debug, Clone, Default)]
pub struct EdgeApplyResult {
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub tombstoned: usize,
}

impl MemoryStore {
    /// Compute edge sync deltas since a given HLC counter for a device.
    ///
    /// Returns all edges whose `hlc_counter > since_hlc`.
    pub fn compute_edge_sync_deltas(
        &self,
        since_hlc: u64,
        local_device_id: &str,
    ) -> SqlResult<Vec<EdgeSyncDelta>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT src_id, dst_id, rel_type, confidence, source, created_at,
                    valid_from, valid_to, edge_source, origin_device, hlc_counter
             FROM memory_edges
             WHERE hlc_counter > ?1",
        )?;
        let rows = stmt.query_map(params![since_hlc as i64], |row| {
            Ok(EdgeSyncDelta {
                src_id: row.get(0)?,
                dst_id: row.get(1)?,
                rel_type: row.get(2)?,
                confidence: row.get(3)?,
                source: row.get(4)?,
                created_at: row.get(5)?,
                valid_from: row.get(6)?,
                valid_to: row.get(7)?,
                edge_source: row.get(8)?,
                origin_device: row
                    .get::<_, Option<String>>(9)?
                    .unwrap_or_else(|| local_device_id.to_string()),
                hlc_counter: row.get::<_, i64>(10)? as u64,
            })
        })?;
        rows.collect()
    }

    /// Apply edge sync deltas from a remote device.
    ///
    /// Resolution: for each `(src_id, dst_id, rel_type)`:
    /// - If no local edge exists → insert.
    /// - If local edge exists → HLC-based LWW:
    ///   - Higher `(hlc_counter, origin_device)` wins.
    ///   - Winner's `valid_to` state takes effect (tombstone or live).
    pub fn apply_edge_sync_deltas(
        &self,
        deltas: &[EdgeSyncDelta],
        _local_device_id: &str,
    ) -> SqlResult<EdgeApplyResult> {
        let conn = self.conn();
        let mut result = EdgeApplyResult::default();

        for delta in deltas {
            // Check if a local edge with the same key exists.
            let local = conn
                .query_row(
                    "SELECT id, hlc_counter, origin_device, valid_to
                     FROM memory_edges
                     WHERE src_id = ?1 AND dst_id = ?2 AND rel_type = ?3",
                    params![delta.src_id, delta.dst_id, delta.rel_type],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)? as u64,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<i64>>(3)?,
                        ))
                    },
                )
                .optional()?;

            match local {
                None => {
                    // No local edge — insert.
                    conn.execute(
                        "INSERT INTO memory_edges
                            (src_id, dst_id, rel_type, confidence, source,
                             created_at, valid_from, valid_to, edge_source,
                             origin_device, hlc_counter)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                        params![
                            delta.src_id,
                            delta.dst_id,
                            delta.rel_type,
                            delta.confidence,
                            delta.source,
                            delta.created_at,
                            delta.valid_from,
                            delta.valid_to,
                            delta.edge_source,
                            delta.origin_device,
                            delta.hlc_counter as i64,
                        ],
                    )?;
                    if delta.valid_to.is_some() {
                        result.tombstoned += 1;
                    } else {
                        result.inserted += 1;
                    }
                }
                Some((local_id, local_hlc, local_device, _local_valid_to)) => {
                    let local_dev = local_device.as_deref().unwrap_or(_local_device_id);

                    // HLC-based LWW: (hlc_counter, origin_device) total order.
                    let remote_wins = delta.hlc_counter > local_hlc
                        || (delta.hlc_counter == local_hlc && *delta.origin_device > *local_dev);

                    if remote_wins {
                        conn.execute(
                            "UPDATE memory_edges SET
                                confidence = ?1, source = ?2, valid_from = ?3,
                                valid_to = ?4, edge_source = ?5,
                                origin_device = ?6, hlc_counter = ?7
                             WHERE id = ?8",
                            params![
                                delta.confidence,
                                delta.source,
                                delta.valid_from,
                                delta.valid_to,
                                delta.edge_source,
                                delta.origin_device,
                                delta.hlc_counter as i64,
                                local_id,
                            ],
                        )?;
                        if delta.valid_to.is_some() {
                            result.tombstoned += 1;
                        } else {
                            result.updated += 1;
                        }
                    } else {
                        result.skipped += 1;
                    }
                }
            }
        }

        Ok(result)
    }

    /// Hard-delete edges that have been tombstoned for longer than the
    /// retention period. Returns the number of edges purged.
    ///
    /// Call this from the maintenance scheduler to bound storage growth.
    pub fn compact_tombstoned_edges(&self) -> SqlResult<usize> {
        self.compact_tombstoned_edges_with_retention(TOMBSTONE_RETENTION_MS)
    }

    /// Compact with a custom retention period (for testing).
    pub fn compact_tombstoned_edges_with_retention(&self, retention_ms: i64) -> SqlResult<usize> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let cutoff = now - retention_ms;
        let deleted = self.conn().execute(
            "DELETE FROM memory_edges WHERE valid_to IS NOT NULL AND valid_to < ?1",
            params![cutoff],
        )?;
        Ok(deleted)
    }
}

use rusqlite::OptionalExtension;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::MemoryStore;
    use std::path::Path;

    fn make_store() -> MemoryStore {
        MemoryStore::new(Path::new(":memory:"))
    }

    /// Insert a dummy memory so edges have valid FK targets.
    fn add_memories(store: &MemoryStore) -> (i64, i64) {
        use crate::memory::store::{MemoryType, NewMemory};
        let a = store
            .add(NewMemory {
                content: "Memory A".into(),
                tags: "test".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap()
            .id;
        let b = store
            .add(NewMemory {
                content: "Memory B".into(),
                tags: "test".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap()
            .id;
        (a, b)
    }

    #[test]
    fn insert_edge_via_sync() {
        let store = make_store();
        let (a, b) = add_memories(&store);

        let deltas = vec![EdgeSyncDelta {
            src_id: a,
            dst_id: b,
            rel_type: "related_to".into(),
            confidence: 0.9,
            source: "llm".into(),
            created_at: 1000,
            valid_from: None,
            valid_to: None,
            edge_source: None,
            origin_device: "dev-a".into(),
            hlc_counter: 5,
        }];

        let result = store.apply_edge_sync_deltas(&deltas, "dev-b").unwrap();
        assert_eq!(result.inserted, 1);
        assert_eq!(result.skipped, 0);
    }

    #[test]
    fn hlc_remote_wins_updates_edge() {
        let store = make_store();
        let (a, b) = add_memories(&store);

        // Insert local edge with HLC=3.
        store.conn().execute(
            "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, origin_device, hlc_counter)
             VALUES (?1, ?2, 'related_to', 0.5, 'user', 1000, 'dev-a', 3)",
            params![a, b],
        ).unwrap();

        // Remote delta with HLC=10 should win.
        let deltas = vec![EdgeSyncDelta {
            src_id: a,
            dst_id: b,
            rel_type: "related_to".into(),
            confidence: 0.95,
            source: "llm".into(),
            created_at: 1000,
            valid_from: None,
            valid_to: None,
            edge_source: None,
            origin_device: "dev-b".into(),
            hlc_counter: 10,
        }];

        let result = store.apply_edge_sync_deltas(&deltas, "dev-a").unwrap();
        assert_eq!(result.updated, 1);

        let conf: f64 = store
            .conn()
            .query_row(
                "SELECT confidence FROM memory_edges WHERE src_id=?1 AND dst_id=?2",
                params![a, b],
                |r| r.get(0),
            )
            .unwrap();
        assert!((conf - 0.95).abs() < 0.001);
    }

    #[test]
    fn hlc_local_wins_skips_delta() {
        let store = make_store();
        let (a, b) = add_memories(&store);

        // Local edge with HLC=20.
        store.conn().execute(
            "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, origin_device, hlc_counter)
             VALUES (?1, ?2, 'related_to', 0.8, 'user', 1000, 'dev-a', 20)",
            params![a, b],
        ).unwrap();

        // Remote with HLC=5 should lose.
        let deltas = vec![EdgeSyncDelta {
            src_id: a,
            dst_id: b,
            rel_type: "related_to".into(),
            confidence: 0.3,
            source: "auto".into(),
            created_at: 500,
            valid_from: None,
            valid_to: None,
            edge_source: None,
            origin_device: "dev-b".into(),
            hlc_counter: 5,
        }];

        let result = store.apply_edge_sync_deltas(&deltas, "dev-a").unwrap();
        assert_eq!(result.skipped, 1);
    }

    #[test]
    fn tombstone_and_re_add_converges() {
        let store_a = make_store();
        let store_b = make_store();
        let (a1, b1) = add_memories(&store_a);
        let (a2, b2) = add_memories(&store_b);
        // Use same IDs for both stores.
        assert_eq!((a1, b1), (a2, b2));

        // Device A adds edge at HLC=1.
        store_a.conn().execute(
            "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, origin_device, hlc_counter)
             VALUES (?1, ?2, 'cites', 1.0, 'user', 1000, 'dev-a', 1)",
            params![a1, b1],
        ).unwrap();
        store_b.conn().execute(
            "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, origin_device, hlc_counter)
             VALUES (?1, ?2, 'cites', 1.0, 'user', 1000, 'dev-a', 1)",
            params![a2, b2],
        ).unwrap();

        // Device B tombstones at HLC=5.
        store_b
            .conn()
            .execute(
                "UPDATE memory_edges SET valid_to = 2000, hlc_counter = 5, origin_device = 'dev-b'
             WHERE src_id = ?1 AND dst_id = ?2 AND rel_type = 'cites'",
                params![a2, b2],
            )
            .unwrap();

        // Device A re-adds at HLC=10 (should win over tombstone).
        store_a
            .conn()
            .execute(
                "UPDATE memory_edges SET valid_to = NULL, hlc_counter = 10, origin_device = 'dev-a'
             WHERE src_id = ?1 AND dst_id = ?2 AND rel_type = 'cites'",
                params![a1, b1],
            )
            .unwrap();

        // Sync B→A: tombstone should be ignored (A's HLC=10 > B's HLC=5).
        let deltas_b = store_b.compute_edge_sync_deltas(0, "dev-b").unwrap();
        let result_a = store_a.apply_edge_sync_deltas(&deltas_b, "dev-a").unwrap();
        assert_eq!(result_a.skipped, 1);

        // Sync A→B: re-add should win (HLC=10 > 5).
        let deltas_a = store_a.compute_edge_sync_deltas(0, "dev-a").unwrap();
        let result_b = store_b.apply_edge_sync_deltas(&deltas_a, "dev-b").unwrap();
        assert_eq!(result_b.updated, 1);

        // Both should have the edge live (valid_to = NULL).
        let a_valid_to: Option<i64> = store_a.conn().query_row(
            "SELECT valid_to FROM memory_edges WHERE src_id=?1 AND dst_id=?2 AND rel_type='cites'",
            params![a1, b1],
            |r| r.get(0),
        ).unwrap();
        let b_valid_to: Option<i64> = store_b.conn().query_row(
            "SELECT valid_to FROM memory_edges WHERE src_id=?1 AND dst_id=?2 AND rel_type='cites'",
            params![a2, b2],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(a_valid_to, None);
        assert_eq!(b_valid_to, None);
    }

    #[test]
    fn compact_tombstoned_edges_removes_old() {
        let store = make_store();
        let (a, b) = add_memories(&store);

        // Insert an edge tombstoned 60 days ago.
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let old_tombstone = now_ms - 60 * 24 * 60 * 60 * 1000; // 60 days ago

        store.conn().execute(
            "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, valid_to, hlc_counter)
             VALUES (?1, ?2, 'old_edge', 1.0, 'user', 1000, ?3, 1)",
            params![a, b, old_tombstone],
        ).unwrap();

        // Insert a recently tombstoned edge (5 days ago).
        let recent_tombstone = now_ms - 5 * 24 * 60 * 60 * 1000;
        store.conn().execute(
            "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, valid_to, hlc_counter)
             VALUES (?1, ?2, 'recent_edge', 1.0, 'user', 1000, ?3, 2)",
            params![a, b, recent_tombstone],
        ).unwrap();

        let purged = store.compact_tombstoned_edges().unwrap();
        assert_eq!(purged, 1); // Only the 60-day-old edge.

        let remaining: i64 = store
            .conn()
            .query_row("SELECT COUNT(*) FROM memory_edges", [], |r| r.get(0))
            .unwrap();
        assert_eq!(remaining, 1); // The recent one survives.
    }

    #[test]
    fn compute_deltas_filters_by_hlc() {
        let store = make_store();
        let (a, b) = add_memories(&store);

        // Two edges with different HLCs.
        store.conn().execute(
            "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, origin_device, hlc_counter)
             VALUES (?1, ?2, 'r1', 1.0, 'user', 1000, 'dev-a', 3)",
            params![a, b],
        ).unwrap();
        store.conn().execute(
            "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, origin_device, hlc_counter)
             VALUES (?1, ?2, 'r2', 1.0, 'user', 2000, 'dev-a', 7)",
            params![a, b],
        ).unwrap();

        // Only edges with hlc > 5 should be returned.
        let deltas = store.compute_edge_sync_deltas(5, "dev-a").unwrap();
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].rel_type, "r2");
    }
}
