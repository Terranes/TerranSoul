//! LWW-Map CRDT sync for cross-device memory merge (Chunk 17.5).
//!
//! Implements a Last-Write-Wins element dictionary keyed on
//! `(source_hash, source_url)` with `updated_at` as the timestamp and
//! `origin_device` as a tiebreaker.
//!
//! # Protocol
//!
//! 1. Device A computes deltas since last sync with B (`compute_deltas`).
//! 2. Sends deltas over Soul Link as `kind: "memory_sync"`.
//! 3. Device B applies deltas (`apply_deltas`) — LWW resolves conflicts.
//! 4. B responds with its own deltas (bidirectional push).
//!
//! Entries without `source_hash` are keyed on `(content_prefix, created_at)`
//! to handle legacy/unindexed memories.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Result as SqlResult};
use serde::{Deserialize, Serialize};

use super::store::{MemoryEntry, MemoryStore};

// ─── Types ─────────────────────────────────────────────────────────────

/// A globally-unique key for deduplicating memories across devices.
///
/// Equality is determined by `content_hash` (primary) or
/// `(content_prefix, created_at)` as fallback for legacy entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncKey {
    /// SHA-256 content hash (primary dedup signal).
    pub content_hash: Option<String>,
    /// Origin URL (secondary dedup for ingested docs).
    pub source_url: Option<String>,
    /// Fallback: first 64 chars of content + created_at for legacy entries.
    pub content_prefix: Option<String>,
    pub created_at: i64,
}

impl PartialEq for SyncKey {
    fn eq(&self, other: &Self) -> bool {
        match (&self.content_hash, &other.content_hash) {
            (Some(a), Some(b)) => a == b,
            _ => self.content_prefix == other.content_prefix && self.created_at == other.created_at,
        }
    }
}

impl Eq for SyncKey {}

impl std::hash::Hash for SyncKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if let Some(h) = &self.content_hash {
            h.hash(state);
        } else {
            self.content_prefix.hash(state);
            self.created_at.hash(state);
        }
    }
}

impl SyncKey {
    /// Derive the sync key from a memory entry.
    pub fn from_entry(entry: &MemoryEntry) -> Self {
        Self {
            content_hash: entry.source_hash.clone(),
            source_url: entry.source_url.clone(),
            content_prefix: if entry.source_hash.is_none() {
                Some(entry.content.chars().take(64).collect())
            } else {
                None
            },
            created_at: entry.created_at,
        }
    }
}

/// A single delta record for sync — represents one memory's state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDelta {
    pub key: SyncKey,
    pub operation: SyncOp,
    pub content: String,
    pub tags: String,
    pub importance: i64,
    pub memory_type: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub origin_device: String,
    pub source_url: Option<String>,
    pub source_hash: Option<String>,
}

/// Operation type for a sync delta.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncOp {
    /// Insert or update (LWW — highest updated_at wins).
    Upsert,
    /// Soft-close (valid_to set).
    SoftClose { valid_to: i64 },
}

/// Result of applying deltas — reports what happened.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApplyResult {
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub soft_closed: usize,
}

// ─── Delta Computation ─────────────────────────────────────────────────

impl MemoryStore {
    /// Compute deltas (entries modified since `since_timestamp`) for syncing
    /// to a peer device.
    ///
    /// If `since_timestamp` is 0, returns ALL non-expired entries (full sync).
    pub fn compute_sync_deltas(
        &self,
        since_timestamp: i64,
        local_device_id: &str,
    ) -> SqlResult<Vec<SyncDelta>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at,
                    updated_at, origin_device, source_url, source_hash, valid_to
             FROM memories
             WHERE COALESCE(updated_at, created_at) >= ?1",
        )?;

        let rows = stmt.query_map(params![since_timestamp], |row| {
            let updated_at: Option<i64> = row.get(6)?;
            let origin_device: Option<String> = row.get(7)?;
            let created_at: i64 = row.get(5)?;
            let valid_to: Option<i64> = row.get(10)?;

            let content: String = row.get(1)?;
            let source_hash: Option<String> = row.get(9)?;
            let source_url: Option<String> = row.get(8)?;

            let key = SyncKey {
                content_hash: source_hash.clone(),
                source_url: source_url.clone(),
                content_prefix: if source_hash.is_none() {
                    Some(content.chars().take(64).collect())
                } else {
                    None
                },
                created_at,
            };

            let operation = match valid_to {
                Some(vt) => SyncOp::SoftClose { valid_to: vt },
                None => SyncOp::Upsert,
            };

            Ok(SyncDelta {
                key,
                operation,
                content,
                tags: row.get(2)?,
                importance: row.get(3)?,
                memory_type: row.get::<_, String>(4)?,
                created_at,
                updated_at: updated_at.unwrap_or(created_at),
                origin_device: origin_device.unwrap_or_else(|| local_device_id.to_string()),
                source_url,
                source_hash,
            })
        })?;

        rows.collect()
    }

    /// Apply inbound deltas from a peer device using LWW conflict resolution.
    ///
    /// For each delta:
    /// 1. If no local entry matches the key → insert.
    /// 2. If a local entry exists and the delta has a higher `updated_at` → update.
    /// 3. If `updated_at` is equal → lexicographic `origin_device` tiebreaker (higher wins).
    /// 4. Otherwise → skip (local is newer).
    pub fn apply_sync_deltas(
        &self,
        deltas: &[SyncDelta],
        local_device_id: &str,
    ) -> SqlResult<ApplyResult> {
        let conn = self.conn();
        let mut result = ApplyResult::default();

        // Build index of existing entries by sync key.
        let existing = self.build_sync_index()?;

        for delta in deltas {
            match existing.get(&delta.key) {
                None => {
                    // No local match — insert.
                    match &delta.operation {
                        SyncOp::Upsert => {
                            conn.execute(
                                "INSERT INTO memories (content, tags, importance, memory_type,
                                    created_at, tier, decay_score, token_count, source_url,
                                    source_hash, updated_at, origin_device)
                                 VALUES (?1, ?2, ?3, ?4, ?5, 'long', 1.0, 0, ?6, ?7, ?8, ?9)",
                                params![
                                    delta.content,
                                    delta.tags,
                                    delta.importance,
                                    delta.memory_type,
                                    delta.created_at,
                                    delta.source_url,
                                    delta.source_hash,
                                    delta.updated_at,
                                    delta.origin_device,
                                ],
                            )?;
                            result.inserted += 1;
                        }
                        SyncOp::SoftClose { valid_to } => {
                            // Insert as already-closed.
                            conn.execute(
                                "INSERT INTO memories (content, tags, importance, memory_type,
                                    created_at, tier, decay_score, token_count, source_url,
                                    source_hash, updated_at, origin_device, valid_to)
                                 VALUES (?1, ?2, ?3, ?4, ?5, 'long', 1.0, 0, ?6, ?7, ?8, ?9, ?10)",
                                params![
                                    delta.content,
                                    delta.tags,
                                    delta.importance,
                                    delta.memory_type,
                                    delta.created_at,
                                    delta.source_url,
                                    delta.source_hash,
                                    delta.updated_at,
                                    delta.origin_device,
                                    valid_to,
                                ],
                            )?;
                            result.soft_closed += 1;
                        }
                    }
                }
                Some(local_entry) => {
                    // Existing entry — apply LWW.
                    let local_ts = local_entry.updated_at.unwrap_or(local_entry.created_at);
                    let local_device = local_entry
                        .origin_device
                        .as_deref()
                        .unwrap_or(local_device_id);

                    if delta.updated_at > local_ts
                        || (delta.updated_at == local_ts
                            && *delta.origin_device > *local_device)
                    {
                        // Remote wins — update local.
                        match &delta.operation {
                            SyncOp::Upsert => {
                                conn.execute(
                                    "UPDATE memories SET content = ?1, tags = ?2, importance = ?3,
                                        memory_type = ?4, updated_at = ?5, origin_device = ?6,
                                        source_url = ?7, source_hash = ?8
                                     WHERE id = ?9",
                                    params![
                                        delta.content,
                                        delta.tags,
                                        delta.importance,
                                        delta.memory_type,
                                        delta.updated_at,
                                        delta.origin_device,
                                        delta.source_url,
                                        delta.source_hash,
                                        local_entry.id,
                                    ],
                                )?;
                                result.updated += 1;
                            }
                            SyncOp::SoftClose { valid_to } => {
                                conn.execute(
                                    "UPDATE memories SET valid_to = ?1, updated_at = ?2,
                                        origin_device = ?3 WHERE id = ?4",
                                    params![
                                        valid_to,
                                        delta.updated_at,
                                        delta.origin_device,
                                        local_entry.id,
                                    ],
                                )?;
                                result.soft_closed += 1;
                            }
                        }
                    } else {
                        result.skipped += 1;
                    }
                }
            }
        }

        Ok(result)
    }

    /// Record a sync event in the audit log.
    pub fn log_sync(
        &self,
        peer_device: &str,
        direction: &str,
        entry_count: usize,
    ) -> SqlResult<()> {
        let conn = self.conn();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        conn.execute(
            "INSERT INTO sync_log (peer_device, direction, entry_count, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![peer_device, direction, entry_count as i64, now],
        )?;
        Ok(())
    }

    /// Get the timestamp of the last successful sync with a peer.
    pub fn last_sync_time(&self, peer_device: &str) -> SqlResult<Option<i64>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT MAX(timestamp) FROM sync_log WHERE peer_device = ?1",
            params![peer_device],
            |row| row.get(0),
        )
    }

    /// Build an index mapping SyncKey → MemoryEntry for all active memories.
    fn build_sync_index(&self) -> SqlResult<HashMap<SyncKey, MemoryEntry>> {
        let entries = self.get_all()?;
        let mut map = HashMap::with_capacity(entries.len());
        for entry in entries {
            let key = SyncKey::from_entry(&entry);
            map.insert(key, entry);
        }
        Ok(map)
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::{MemoryType, NewMemory};
    use std::path::Path;

    fn make_store() -> MemoryStore {
        MemoryStore::new(Path::new(":memory:"))
    }

    fn add_memory(store: &MemoryStore, content: &str, hash: Option<&str>) -> i64 {
        store
            .add(NewMemory {
                content: content.into(),
                tags: "test".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                source_hash: hash.map(|s| s.into()),
                ..Default::default()
            })
            .unwrap()
            .id
    }

    #[test]
    fn compute_deltas_returns_all_entries_since_zero() {
        let store = make_store();
        add_memory(&store, "Hello world", Some("hash1"));
        add_memory(&store, "Second entry", Some("hash2"));

        let deltas = store.compute_sync_deltas(0, "device-a").unwrap();
        assert_eq!(deltas.len(), 2);
        assert!(deltas.iter().all(|d| d.operation == SyncOp::Upsert));
    }

    #[test]
    fn apply_deltas_inserts_new_entries() {
        let store = make_store();
        let deltas = vec![SyncDelta {
            key: SyncKey {
                content_hash: Some("remote-hash".into()),
                source_url: None,
                content_prefix: None,
                created_at: 1000,
            },
            operation: SyncOp::Upsert,
            content: "Remote memory".into(),
            tags: "synced".into(),
            importance: 4,
            memory_type: "fact".into(),
            created_at: 1000,
            updated_at: 2000,
            origin_device: "device-b".into(),
            source_url: None,
            source_hash: Some("remote-hash".into()),
        }];

        let result = store.apply_sync_deltas(&deltas, "device-a").unwrap();
        assert_eq!(result.inserted, 1);
        assert_eq!(result.skipped, 0);

        let all = store.get_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].content, "Remote memory");
        assert_eq!(all[0].origin_device.as_deref(), Some("device-b"));
    }

    #[test]
    fn apply_deltas_lww_remote_wins_when_newer() {
        let store = make_store();
        let id = add_memory(&store, "Old content", Some("shared-hash"));

        // Set local updated_at to 1000.
        let conn = store.conn();
        conn.execute(
            "UPDATE memories SET updated_at = 1000, origin_device = 'device-a' WHERE id = ?1",
            params![id],
        )
        .unwrap();

        // Remote delta has updated_at = 2000 (newer).
        let deltas = vec![SyncDelta {
            key: SyncKey {
                content_hash: Some("shared-hash".into()),
                source_url: None,
                content_prefix: None,
                created_at: 100,
            },
            operation: SyncOp::Upsert,
            content: "Updated content from remote".into(),
            tags: "synced".into(),
            importance: 5,
            memory_type: "fact".into(),
            created_at: 100,
            updated_at: 2000,
            origin_device: "device-b".into(),
            source_url: None,
            source_hash: Some("shared-hash".into()),
        }];

        let result = store.apply_sync_deltas(&deltas, "device-a").unwrap();
        assert_eq!(result.updated, 1);

        let all = store.get_all().unwrap();
        assert_eq!(all[0].content, "Updated content from remote");
    }

    #[test]
    fn apply_deltas_lww_local_wins_when_newer() {
        let store = make_store();
        let id = add_memory(&store, "Fresh local content", Some("shared-hash"));

        // Set local updated_at to 5000 (newer than remote).
        let conn = store.conn();
        conn.execute(
            "UPDATE memories SET updated_at = 5000, origin_device = 'device-a' WHERE id = ?1",
            params![id],
        )
        .unwrap();

        // Remote delta has updated_at = 2000 (older).
        let deltas = vec![SyncDelta {
            key: SyncKey {
                content_hash: Some("shared-hash".into()),
                source_url: None,
                content_prefix: None,
                created_at: 100,
            },
            operation: SyncOp::Upsert,
            content: "Stale remote content".into(),
            tags: "synced".into(),
            importance: 2,
            memory_type: "fact".into(),
            created_at: 100,
            updated_at: 2000,
            origin_device: "device-b".into(),
            source_url: None,
            source_hash: Some("shared-hash".into()),
        }];

        let result = store.apply_sync_deltas(&deltas, "device-a").unwrap();
        assert_eq!(result.skipped, 1);

        let all = store.get_all().unwrap();
        assert_eq!(all[0].content, "Fresh local content");
    }

    #[test]
    fn apply_deltas_tiebreaker_uses_device_id() {
        let store = make_store();
        let id = add_memory(&store, "Original", Some("shared-hash"));

        // Same updated_at, but device-b > device-a lexicographically.
        let conn = store.conn();
        conn.execute(
            "UPDATE memories SET updated_at = 1000, origin_device = 'device-a' WHERE id = ?1",
            params![id],
        )
        .unwrap();

        let deltas = vec![SyncDelta {
            key: SyncKey {
                content_hash: Some("shared-hash".into()),
                source_url: None,
                content_prefix: None,
                created_at: 100,
            },
            operation: SyncOp::Upsert,
            content: "Device B content".into(),
            tags: "synced".into(),
            importance: 3,
            memory_type: "fact".into(),
            created_at: 100,
            updated_at: 1000, // same timestamp
            origin_device: "device-b".into(), // "device-b" > "device-a"
            source_url: None,
            source_hash: Some("shared-hash".into()),
        }];

        let result = store.apply_sync_deltas(&deltas, "device-a").unwrap();
        assert_eq!(result.updated, 1);

        let all = store.get_all().unwrap();
        assert_eq!(all[0].content, "Device B content");
    }

    #[test]
    fn apply_deltas_soft_close() {
        let store = make_store();
        let id = add_memory(&store, "Will be closed", Some("close-hash"));

        // Set a known updated_at that's older than the remote delta.
        let conn = store.conn();
        conn.execute(
            "UPDATE memories SET updated_at = 1000, origin_device = 'device-a' WHERE id = ?1",
            params![id],
        )
        .unwrap();

        let deltas = vec![SyncDelta {
            key: SyncKey {
                content_hash: Some("close-hash".into()),
                source_url: None,
                content_prefix: None,
                created_at: 100,
            },
            operation: SyncOp::SoftClose { valid_to: 9999 },
            content: "Will be closed".into(),
            tags: "test".into(),
            importance: 3,
            memory_type: "fact".into(),
            created_at: 100,
            updated_at: 5000,
            origin_device: "device-b".into(),
            source_url: None,
            source_hash: Some("close-hash".into()),
        }];

        let result = store.apply_sync_deltas(&deltas, "device-a").unwrap();
        assert_eq!(result.soft_closed, 1);

        let all = store.get_all().unwrap();
        assert_eq!(all[0].valid_to, Some(9999));
    }

    #[test]
    fn log_sync_and_last_sync_time() {
        let store = make_store();
        // Ensure schema.
        store.conn().execute_batch(
            "CREATE TABLE IF NOT EXISTS sync_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                peer_device TEXT NOT NULL,
                direction TEXT NOT NULL,
                entry_count INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            )"
        ).unwrap();

        store.log_sync("device-b", "outbound", 5).unwrap();
        let ts = store.last_sync_time("device-b").unwrap();
        assert!(ts.is_some());
        assert!(ts.unwrap() > 0);

        let ts_other = store.last_sync_time("device-c").unwrap();
        assert!(ts_other.is_none());
    }

    #[test]
    fn roundtrip_compute_and_apply() {
        let store_a = make_store();
        add_memory(&store_a, "Memory from A", Some("hash-a"));

        // Compute deltas from A.
        let deltas = store_a.compute_sync_deltas(0, "device-a").unwrap();
        assert_eq!(deltas.len(), 1);

        // Apply to B.
        let store_b = make_store();
        let result = store_b.apply_sync_deltas(&deltas, "device-b").unwrap();
        assert_eq!(result.inserted, 1);

        let all_b = store_b.get_all().unwrap();
        assert_eq!(all_b[0].content, "Memory from A");
    }
}
