//! Gap detection (Chunk 43.8).
//!
//! When the best retrieval score after `hybrid_search_rrf` falls below a
//! threshold while the query embedding has a meaningful norm, the system
//! persists a `memory_gaps` row — signalling that the knowledge base has
//! a blind spot for this query.

use rusqlite::{params, Result as SqlResult};
use serde::{Deserialize, Serialize};

use super::store::MemoryStore;

/// Configuration for gap detection thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapDetectionConfig {
    /// Maximum top-result score to consider a "gap" (default 0.3).
    pub score_threshold: f64,
    /// Minimum L2 norm of the query embedding to consider meaningful (default 0.7).
    pub norm_threshold: f64,
}

impl Default for GapDetectionConfig {
    fn default() -> Self {
        Self {
            score_threshold: 0.3,
            norm_threshold: 0.7,
        }
    }
}

/// A recorded memory gap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGap {
    pub id: i64,
    pub context_snippet: String,
    pub session_id: Option<String>,
    pub ts: i64,
}

/// Compute the L2 norm of an embedding vector.
pub fn embedding_norm(embedding: &[f32]) -> f64 {
    let sum_sq: f64 = embedding.iter().map(|&v| (v as f64) * (v as f64)).sum();
    sum_sq.sqrt()
}

/// Check whether a gap should be recorded and persist it if so.
///
/// Returns `true` if a gap was recorded.
pub fn detect_and_record_gap(
    store: &MemoryStore,
    top_score: f64,
    query_embedding: Option<&[f32]>,
    context_snippet: &str,
    session_id: Option<&str>,
    config: &GapDetectionConfig,
) -> SqlResult<bool> {
    let norm = match query_embedding {
        Some(emb) => embedding_norm(emb),
        None => return Ok(false),
    };

    if top_score >= config.score_threshold || norm < config.norm_threshold {
        return Ok(false);
    }

    let now = super::store::now_ms();
    let emb_bytes: Option<Vec<u8>> =
        query_embedding.map(|emb| emb.iter().flat_map(|f| f.to_le_bytes()).collect());

    store.conn.execute(
        "INSERT INTO memory_gaps (query_embedding, context_snippet, session_id, ts)
         VALUES (?1, ?2, ?3, ?4)",
        params![emb_bytes, context_snippet, session_id, now],
    )?;

    Ok(true)
}

/// List recent memory gaps, newest first.
pub fn list_recent_gaps(store: &MemoryStore, limit: usize) -> SqlResult<Vec<MemoryGap>> {
    let mut stmt = store.conn.prepare_cached(
        "SELECT id, context_snippet, session_id, ts
         FROM memory_gaps
         ORDER BY ts DESC
         LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(MemoryGap {
            id: row.get(0)?,
            context_snippet: row.get(1)?,
            session_id: row.get(2)?,
            ts: row.get(3)?,
        })
    })?;

    rows.collect()
}

/// Delete a gap by ID (e.g. after it's been addressed).
pub fn dismiss_gap(store: &MemoryStore, gap_id: i64) -> SqlResult<bool> {
    let changed = store
        .conn
        .execute("DELETE FROM memory_gaps WHERE id = ?1", params![gap_id])?;
    Ok(changed > 0)
}

/// Count total gaps.
pub fn gap_count(store: &MemoryStore) -> SqlResult<i64> {
    store
        .conn
        .query_row("SELECT COUNT(*) FROM memory_gaps", [], |row| row.get(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::MemoryStore;

    #[test]
    fn embedding_norm_unit_vector() {
        let v = vec![1.0f32, 0.0, 0.0];
        assert!((embedding_norm(&v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn embedding_norm_multi() {
        let v = vec![3.0f32, 4.0];
        assert!((embedding_norm(&v) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn no_gap_when_score_above_threshold() {
        let store = MemoryStore::in_memory();
        let config = GapDetectionConfig::default();
        let emb = vec![1.0f32; 768]; // norm >> 0.7
        let result =
            detect_and_record_gap(&store, 0.5, Some(&emb), "test query", None, &config).unwrap();
        assert!(!result);
        assert_eq!(gap_count(&store).unwrap(), 0);
    }

    #[test]
    fn no_gap_when_norm_below_threshold() {
        let store = MemoryStore::in_memory();
        let config = GapDetectionConfig::default();
        let emb = vec![0.01f32; 10]; // norm ~ 0.032
        let result =
            detect_and_record_gap(&store, 0.1, Some(&emb), "test query", None, &config).unwrap();
        assert!(!result);
    }

    #[test]
    fn no_gap_without_embedding() {
        let store = MemoryStore::in_memory();
        let config = GapDetectionConfig::default();
        let result = detect_and_record_gap(&store, 0.1, None, "test query", None, &config).unwrap();
        assert!(!result);
    }

    #[test]
    fn records_gap_when_conditions_met() {
        let store = MemoryStore::in_memory();
        let config = GapDetectionConfig::default();
        let emb = vec![1.0f32; 768]; // norm >> 0.7
        let result = detect_and_record_gap(
            &store,
            0.1,
            Some(&emb),
            "quantum physics",
            Some("session-1"),
            &config,
        )
        .unwrap();
        assert!(result);
        assert_eq!(gap_count(&store).unwrap(), 1);

        let gaps = list_recent_gaps(&store, 10).unwrap();
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].context_snippet, "quantum physics");
        assert_eq!(gaps[0].session_id.as_deref(), Some("session-1"));
    }

    #[test]
    fn list_recent_respects_limit() {
        let store = MemoryStore::in_memory();
        let config = GapDetectionConfig::default();
        let emb = vec![1.0f32; 768];
        for i in 0..5 {
            detect_and_record_gap(&store, 0.1, Some(&emb), &format!("gap {i}"), None, &config)
                .unwrap();
        }
        let gaps = list_recent_gaps(&store, 3).unwrap();
        assert_eq!(gaps.len(), 3);
    }

    #[test]
    fn dismiss_gap_removes_it() {
        let store = MemoryStore::in_memory();
        let config = GapDetectionConfig::default();
        let emb = vec![1.0f32; 768];
        detect_and_record_gap(&store, 0.1, Some(&emb), "test", None, &config).unwrap();
        let gaps = list_recent_gaps(&store, 10).unwrap();
        assert_eq!(gaps.len(), 1);
        let removed = dismiss_gap(&store, gaps[0].id).unwrap();
        assert!(removed);
        assert_eq!(gap_count(&store).unwrap(), 0);
    }

    #[test]
    fn dismiss_nonexistent_returns_false() {
        let store = MemoryStore::in_memory();
        assert!(!dismiss_gap(&store, 9999).unwrap());
    }
}
