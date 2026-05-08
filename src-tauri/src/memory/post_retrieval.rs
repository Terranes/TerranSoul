//! Post-retrieval maintenance background task (Chunk 43.6).
//!
//! Triggered after an LLM-as-judge reranking verdict. Runs asynchronously
//! via `tokio::spawn` so the retrieval path is not blocked.
//!
//! Actions:
//! - Strengthen `related_to` edges between co-relevant pairs
//! - Bump confidence (+0.05, cap 1.0) on verified entries
//! - Decay confidence (−0.02) on rejected entries
//! - Log a `memory_gaps` row when no hits match
//! - Refresh tag inference every N runs (placeholder for future integration)

use std::sync::Mutex;

use super::store::MemoryStore;

/// Config for post-retrieval maintenance actions.
#[derive(Debug, Clone, Copy)]
pub struct PostRetrievalConfig {
    /// Confidence bump for verified (reranker-passed) entries.
    pub confidence_bump: f64,
    /// Confidence penalty for rejected entries.
    pub confidence_penalty: f64,
    /// Maximum confidence value after bumps.
    pub confidence_cap: f64,
    /// Minimum confidence value after penalties.
    pub confidence_floor: f64,
}

impl Default for PostRetrievalConfig {
    fn default() -> Self {
        Self {
            confidence_bump: 0.05,
            confidence_penalty: 0.02,
            confidence_cap: 1.0,
            confidence_floor: 0.0,
        }
    }
}

/// Verdict for a single retrieval result from the LLM-as-judge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalVerdict {
    /// Entry was confirmed relevant by the reranker.
    Verified,
    /// Entry was rejected by the reranker.
    Rejected,
}

/// Spawn the post-retrieval maintenance task.
///
/// `store` must be a cheaply-clonable handle wrapping a `Mutex<MemoryStore>`
/// (e.g. `Arc<AppStateInner>` accessed as `&Mutex<MemoryStore>`).
///
/// `verdicts` maps `(memory_id, verdict)` for each candidate that was
/// scored. `query` is the original search query (used for gap logging).
pub fn spawn_post_retrieval<S: std::ops::Deref<Target = Mutex<MemoryStore>> + Send + Sync + 'static>(
    store: S,
    verdicts: Vec<(i64, RetrievalVerdict)>,
    query: String,
    config: PostRetrievalConfig,
) {
    tokio::spawn(async move {
        if let Ok(store) = store.lock() {
            run_maintenance(&store, &verdicts, &query, &config);
        }
    });
}

/// Synchronous maintenance logic (testable without tokio).
pub fn run_maintenance(
    store: &MemoryStore,
    verdicts: &[(i64, RetrievalVerdict)],
    query: &str,
    config: &PostRetrievalConfig,
) {
    let verified_ids: Vec<i64> = verdicts
        .iter()
        .filter(|(_, v)| *v == RetrievalVerdict::Verified)
        .map(|(id, _)| *id)
        .collect();

    let rejected_ids: Vec<i64> = verdicts
        .iter()
        .filter(|(_, v)| *v == RetrievalVerdict::Rejected)
        .map(|(id, _)| *id)
        .collect();

    // Bump confidence on verified entries.
    for &id in &verified_ids {
        let _ = store.conn.execute(
            "UPDATE memories SET confidence = MIN(confidence + ?1, ?2) WHERE id = ?3",
            rusqlite::params![config.confidence_bump, config.confidence_cap, id],
        );
    }

    // Decay confidence on rejected entries.
    for &id in &rejected_ids {
        let _ = store.conn.execute(
            "UPDATE memories SET confidence = MAX(confidence - ?1, ?2) WHERE id = ?3",
            rusqlite::params![config.confidence_penalty, config.confidence_floor, id],
        );
    }

    // Strengthen related_to edges between co-relevant (verified) pairs.
    if verified_ids.len() >= 2 {
        let now = super::store::now_ms();
        for i in 0..verified_ids.len() {
            for j in (i + 1)..verified_ids.len() {
                let _ = store.conn.execute(
                    "INSERT INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
                     VALUES (?1, ?2, 'related_to', 0.5, 'auto', ?3)
                     ON CONFLICT(src_id, dst_id, rel_type) DO UPDATE SET
                       confidence = MIN(confidence + 0.05, 1.0)",
                    rusqlite::params![verified_ids[i], verified_ids[j], now],
                );
            }
        }
    }

    // Log gap when no hits matched.
    if verified_ids.is_empty() && !verdicts.is_empty() {
        let now = super::store::now_ms();
        let _ = store.conn.execute(
            "INSERT INTO memory_gaps (context_snippet, ts) VALUES (?1, ?2)",
            rusqlite::params![query, now],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::{MemoryStore, NewMemory, MemoryType};

    fn setup() -> MemoryStore {
        let store = MemoryStore::in_memory();
        store
            .add(NewMemory {
                content: "fact A".to_string(),
                tags: "test".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        store
            .add(NewMemory {
                content: "fact B".to_string(),
                tags: "test".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        store
            .add(NewMemory {
                content: "fact C".to_string(),
                tags: "test".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        store
    }

    #[test]
    fn verified_bumps_confidence() {
        let store = setup();
        let cfg = PostRetrievalConfig::default();
        let verdicts = vec![(1, RetrievalVerdict::Verified)];

        run_maintenance(&store, &verdicts, "test query", &cfg);

        let entry = store.get_by_id(1).unwrap();
        assert!(
            (entry.confidence - 1.0).abs() < f64::EPSILON,
            "confidence should be capped at 1.0, got {}",
            entry.confidence
        );
    }

    #[test]
    fn rejected_decays_confidence() {
        let store = setup();
        let cfg = PostRetrievalConfig::default();
        let verdicts = vec![(1, RetrievalVerdict::Rejected)];

        run_maintenance(&store, &verdicts, "test query", &cfg);

        let entry = store.get_by_id(1).unwrap();
        assert!(
            (entry.confidence - 0.98).abs() < 0.001,
            "confidence should be 0.98 after -0.02 penalty, got {}",
            entry.confidence
        );
    }

    #[test]
    fn co_relevant_pairs_get_edges() {
        let store = setup();
        let cfg = PostRetrievalConfig::default();
        let verdicts = vec![
            (1, RetrievalVerdict::Verified),
            (2, RetrievalVerdict::Verified),
        ];

        run_maintenance(&store, &verdicts, "test query", &cfg);

        let edge_count: i64 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM memory_edges WHERE rel_type = 'related_to'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(edge_count, 1, "should create one edge between co-relevant pair");
    }

    #[test]
    fn no_hits_logs_gap() {
        let store = setup();
        let cfg = PostRetrievalConfig::default();
        let verdicts = vec![
            (1, RetrievalVerdict::Rejected),
            (2, RetrievalVerdict::Rejected),
        ];

        run_maintenance(&store, &verdicts, "unfound query", &cfg);

        let gap_count: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM memory_gaps", [], |row| row.get(0))
            .unwrap();
        assert_eq!(gap_count, 1, "should log a gap when all rejected");

        let snippet: String = store
            .conn
            .query_row(
                "SELECT context_snippet FROM memory_gaps LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(snippet, "unfound query");
    }

    #[test]
    fn co_relevant_edge_strengthens_on_repeat() {
        let store = setup();
        let cfg = PostRetrievalConfig::default();
        let verdicts = vec![
            (1, RetrievalVerdict::Verified),
            (2, RetrievalVerdict::Verified),
        ];

        // Run twice
        run_maintenance(&store, &verdicts, "q1", &cfg);
        run_maintenance(&store, &verdicts, "q2", &cfg);

        let confidence: f64 = store
            .conn
            .query_row(
                "SELECT confidence FROM memory_edges WHERE src_id = 1 AND dst_id = 2",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            (confidence - 0.55).abs() < 0.01,
            "edge confidence should strengthen to 0.55, got {confidence}"
        );
    }

    #[test]
    fn empty_verdicts_no_ops() {
        let store = setup();
        let cfg = PostRetrievalConfig::default();
        run_maintenance(&store, &[], "empty", &cfg);

        // Nothing should have changed
        let entry = store.get_by_id(1).unwrap();
        assert!((entry.confidence - 1.0).abs() < f64::EPSILON);
    }
}
