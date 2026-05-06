//! Knowledge-wiki orchestration over the existing memory store.
//!
//! Implements the operations described in
//! [`docs/llm-wiki-pattern-application.md`] — the TerranSoul-native
//! application of Karpathy's *LLM Wiki* (Apr 2026 gist + Mar 2025
//! *append-and-review* note) and `safishamsi/graphify` (MIT, v0.7.6).
//!
//! All five operations reuse V15 schema columns — there is no schema
//! migration. The module is pure orchestration over [`MemoryStore`],
//! [`MemoryEdge`], and [`memory_communities`].
//!
//! Surfaced to the chat box and Brain UI as TerranSoul-native verbs:
//!
//! | Verb        | Backend function                |
//! | ----------- | ------------------------------- |
//! | `/digest`   | [`ensure_source_dedup`]         |
//! | `/ponder`   | [`audit_report`]                |
//! | `/spotlight`| [`god_nodes`]                   |
//! | `/serendipity` | [`surprising_connections`]   |
//! | `/revisit`  | [`append_and_review_queue`]     |
//!
//! Names are deliberately not graphify clones nor Karpathy verb clones.

use rusqlite::{params, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use super::conflicts::{ConflictStatus, MemoryConflict};
use super::edges::{EdgeSource, MemoryEdge};
use super::store::{MemoryEntry, MemoryStore};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ── 1. Confidence rubric ───────────────────────────────────────────────

/// Discrete confidence label for an edge — TerranSoul's mapping of
/// graphify's `EXTRACTED / INFERRED / AMBIGUOUS` rubric onto our existing
/// `(EdgeSource, confidence: f64)` pair.
///
/// Pure function — see `confidence_label()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLabel {
    /// User assertion or deterministic extraction (confidence 1.0).
    Extracted,
    /// Strong LLM inference (≥ 0.85).
    InferredStrong,
    /// Reasonable LLM inference (0.65 ≤ x < 0.85).
    InferredWeak,
    /// Ambiguous — flag for human review (< 0.65).
    Ambiguous,
}

impl ConfidenceLabel {
    pub fn as_str(self) -> &'static str {
        match self {
            ConfidenceLabel::Extracted => "extracted",
            ConfidenceLabel::InferredStrong => "inferred_strong",
            ConfidenceLabel::InferredWeak => "inferred_weak",
            ConfidenceLabel::Ambiguous => "ambiguous",
        }
    }
}

/// Translate `(EdgeSource, confidence)` into a discrete rubric label.
///
/// Mirrors graphify's grading rubric without copying its code:
/// User/Auto provenance is `Extracted` regardless of float value (the
/// numeric 1.0 is only the default; explicit user/auto edges are
/// definitive). LLM-proposed edges fall into one of three buckets by
/// confidence threshold.
pub fn confidence_label(edge: &MemoryEdge) -> ConfidenceLabel {
    match edge.source {
        EdgeSource::User | EdgeSource::Auto => ConfidenceLabel::Extracted,
        EdgeSource::Llm => {
            if edge.confidence >= 0.95 {
                ConfidenceLabel::Extracted
            } else if edge.confidence >= 0.85 {
                ConfidenceLabel::InferredStrong
            } else if edge.confidence >= 0.65 {
                ConfidenceLabel::InferredWeak
            } else {
                ConfidenceLabel::Ambiguous
            }
        }
    }
}

// ── 2. Source dedup (`/digest`) ─────────────────────────────────────────

/// Outcome of an attempt to ingest a piece of content with a known
/// source URL — either the row already existed (identified by
/// `source_hash`) or a fresh entry was inserted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceDedupResult {
    /// `source_hash` already present in `memories`. The caller can skip
    /// the embed/edge/extract pipeline.
    Skipped { existing_id: i64 },
    /// New row was inserted; pipeline should proceed.
    Ingested { entry_id: i64 },
}

/// SHA-256 hex digest of the input bytes — mirrors graphify's source
/// fingerprint cache.
pub fn fingerprint(content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(content);
    hex::encode(digest)
}

/// Look up `source_hash` in `memories`; if found, report `Skipped`. Otherwise
/// insert a new memory row and report `Ingested`. The caller is responsible
/// for follow-on work (embedding, edge extraction, contextualisation).
pub fn ensure_source_dedup(
    store: &MemoryStore,
    source_url: Option<&str>,
    content: &str,
    tags: &str,
    importance: i64,
) -> SqlResult<SourceDedupResult> {
    let hash = fingerprint(content.as_bytes());
    if let Some(entry) = store.find_by_source_hash(&hash)? {
        return Ok(SourceDedupResult::Skipped {
            existing_id: entry.id,
        });
    }
    let entry = store.add(super::store::NewMemory {
        content: content.to_string(),
        tags: tags.to_string(),
        importance: importance.clamp(1, 5),
        memory_type: super::store::MemoryType::Fact,
        source_url: source_url.map(|s| s.to_string()),
        source_hash: Some(hash),
        ..Default::default()
    })?;
    Ok(SourceDedupResult::Ingested { entry_id: entry.id })
}

// ── 3. Audit (`/ponder`) ────────────────────────────────────────────────

/// Configuration for a single audit pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Memories with `decay_score` below this are considered stale (default 0.20).
    pub stale_threshold: f64,
    /// Cap on entries returned per category (default 50).
    pub limit: usize,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            stale_threshold: 0.20,
            limit: 50,
        }
    }
}

/// Aggregated brain-health snapshot — the "lint" pass from the LLM-Wiki
/// pattern, surfacing five orthogonal signals in one report.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditReport {
    pub open_conflicts: Vec<MemoryConflict>,
    pub orphan_ids: Vec<i64>,
    pub stale_ids: Vec<i64>,
    pub pending_embeddings: i64,
    pub total_memories: i64,
    pub total_edges: i64,
    pub generated_at: i64,
}

/// Build the audit report. All queries are indexed so this is cheap to
/// run on every BrainView focus.
pub fn audit_report(store: &MemoryStore, cfg: &AuditConfig) -> SqlResult<AuditReport> {
    let conn = store.conn();

    let total_memories: i64 = conn
        .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
        .unwrap_or(0);
    let total_edges: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_edges WHERE valid_to IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let pending_embeddings: i64 = conn
        .query_row("SELECT COUNT(*) FROM pending_embeddings", [], |r| r.get(0))
        .unwrap_or(0);

    // Orphans — long-tier memories with no live edges.
    let mut orphan_ids = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT m.id FROM memories m
             LEFT JOIN memory_edges e
               ON (e.src_id = m.id OR e.dst_id = m.id) AND e.valid_to IS NULL
             WHERE m.tier = 'long' AND e.id IS NULL AND m.valid_to IS NULL
             ORDER BY m.created_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![cfg.limit as i64], |r| r.get::<_, i64>(0))?;
        for row in rows {
            orphan_ids.push(row?);
        }
    }

    // Stale — low decay, low importance, still active.
    let mut stale_ids = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT id FROM memories
             WHERE decay_score < ?1
               AND importance < 4
               AND valid_to IS NULL
               AND COALESCE(protected, 0) = 0
             ORDER BY decay_score ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![cfg.stale_threshold, cfg.limit as i64], |r| {
            r.get::<_, i64>(0)
        })?;
        for row in rows {
            stale_ids.push(row?);
        }
    }

    let open_conflicts = store
        .list_conflicts(Some(&ConflictStatus::Open))
        .unwrap_or_default();

    Ok(AuditReport {
        open_conflicts,
        orphan_ids,
        stale_ids,
        pending_embeddings,
        total_memories,
        total_edges,
        generated_at: now_ms(),
    })
}

// ── 4. God-nodes (`/spotlight`) ─────────────────────────────────────────

/// A "god node" — a memory ranked by the number of live KG edges
/// touching it. graphify's analyse pass prints the top-10; we expose the
/// same in the BrainView.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodNode {
    pub entry: MemoryEntry,
    pub degree: i64,
}

/// Return the `limit` memories with the highest live-edge degree,
/// descending. `WHERE valid_to IS NULL` enforces temporal validity.
pub fn god_nodes(store: &MemoryStore, limit: usize) -> SqlResult<Vec<GodNode>> {
    let conn = store.conn();
    let mut stmt = conn.prepare(
        "SELECT m.id, COUNT(e.id) AS deg
         FROM memories m
         JOIN memory_edges e
           ON (e.src_id = m.id OR e.dst_id = m.id) AND e.valid_to IS NULL
         WHERE m.valid_to IS NULL
         GROUP BY m.id
         ORDER BY deg DESC
         LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![limit as i64], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
        })?
        .collect::<SqlResult<Vec<_>>>()?;

    let mut out = Vec::with_capacity(rows.len());
    for (id, degree) in rows {
        if let Ok(entry) = store.get_by_id(id) {
            out.push(GodNode { entry, degree });
        }
    }
    Ok(out)
}

// ── 5. Surprising connections (`/serendipity`) ──────────────────────────

/// A cross-community edge — two memories that the brain previously placed
/// in distinct communities but a high-confidence edge now bridges. graphify
/// calls these "surprising connections"; we surface them as serendipitous.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurprisingConnection {
    pub edge: MemoryEdge,
    pub src: MemoryEntry,
    pub dst: MemoryEntry,
    pub label: ConfidenceLabel,
}

/// Return up to `limit` cross-community edges, sorted by confidence
/// descending. Requires `memory_communities` to have been populated;
/// otherwise returns an empty vec (caller can call
/// `MemoryStore::detect_communities` first).
pub fn surprising_connections(
    store: &MemoryStore,
    limit: usize,
) -> SqlResult<Vec<SurprisingConnection>> {
    // Build a node → community map from memory_communities.
    let communities = store.load_communities().unwrap_or_default();
    if communities.is_empty() {
        return Ok(Vec::new());
    }
    let mut node_to_comm: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
    for c in &communities {
        for &m in &c.member_ids {
            node_to_comm.insert(m, c.id);
        }
    }

    // Walk all live edges and pick the cross-community ones.
    let conn = store.conn();
    let mut stmt = conn.prepare(
        "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at,
                valid_from, valid_to, edge_source
         FROM memory_edges
         WHERE valid_to IS NULL AND confidence >= 0.65
         ORDER BY confidence DESC, created_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(MemoryEdge {
            id: r.get(0)?,
            src_id: r.get(1)?,
            dst_id: r.get(2)?,
            rel_type: r.get(3)?,
            confidence: r.get(4)?,
            source: EdgeSource::parse(&r.get::<_, String>(5)?),
            created_at: r.get(6)?,
            valid_from: r.get(7).ok(),
            valid_to: r.get(8).ok(),
            edge_source: r.get(9).ok(),
        })
    })?;

    let mut seen_pair: HashSet<(i64, i64)> = HashSet::new();
    let mut out = Vec::with_capacity(limit);
    for row in rows {
        let edge = row?;
        let (Some(&ca), Some(&cb)) = (
            node_to_comm.get(&edge.src_id),
            node_to_comm.get(&edge.dst_id),
        ) else {
            continue;
        };
        if ca == cb {
            continue;
        }
        let key = if edge.src_id <= edge.dst_id {
            (edge.src_id, edge.dst_id)
        } else {
            (edge.dst_id, edge.src_id)
        };
        if !seen_pair.insert(key) {
            continue;
        }
        let (Ok(src), Ok(dst)) = (store.get_by_id(edge.src_id), store.get_by_id(edge.dst_id))
        else {
            continue;
        };
        let label = confidence_label(&edge);
        out.push(SurprisingConnection {
            edge,
            src,
            dst,
            label,
        });
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}

// ── 6. Append-and-review queue (`/revisit`) ─────────────────────────────

/// One entry in the prioritised review queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewItem {
    pub entry: MemoryEntry,
    /// Lower = more in need of review (sinking to the bottom of the note
    /// in Karpathy's *append-and-review* metaphor).
    pub gravity: f64,
}

/// Compute the gravity score for a single memory. Pure, used by tests.
pub fn gravity_score(entry: &MemoryEntry, now: i64) -> f64 {
    let last = entry.last_accessed.unwrap_or(entry.created_at);
    let age_hours = ((now - last).max(0) as f64) / 3_600_000.0;
    // Older = more "sunk" (lower); higher importance = stays afloat;
    // higher decay = stays current.
    let recency_term = 1.0 / (1.0 + age_hours / 24.0); // 1.0 today, ~0.5 a day old, ~0.04 a month old
    let importance_term = 0.20 * (entry.importance as f64);
    let decay_term = 0.20 * entry.decay_score;
    recency_term + importance_term + decay_term
}

/// Return the `limit` memories most in need of review (lowest gravity).
/// Excludes protected and short-term entries.
pub fn append_and_review_queue(store: &MemoryStore, limit: usize) -> SqlResult<Vec<ReviewItem>> {
    let now = now_ms();
    let conn = store.conn();
    // Pull a wider slice (10× the requested limit, capped) and rank in Rust
    // for clarity — at 1M rows the limit is bounded by the index on
    // `tier` + `decay_score`.
    let slice = (limit.max(20) * 10).min(2000) as i64;
    let mut stmt = conn.prepare(
        "SELECT id
         FROM memories
         WHERE valid_to IS NULL
           AND tier IN ('long', 'working')
           AND COALESCE(protected, 0) = 0
         ORDER BY decay_score ASC,
                  CASE WHEN last_accessed IS NULL THEN 0 ELSE 1 END ASC,
                  last_accessed ASC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![slice], |r| r.get::<_, i64>(0))?;

    let mut scored: Vec<ReviewItem> = rows
        .filter_map(|r| r.ok())
        .filter_map(|id| store.get_by_id(id).ok())
        .map(|entry| {
            let gravity = gravity_score(&entry, now);
            ReviewItem { entry, gravity }
        })
        .collect();
    scored.sort_by(|a, b| {
        a.gravity
            .partial_cmp(&b.gravity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(limit);
    Ok(scored)
}

// ── Tests (modeled on graphify's tests/) ────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::edges::NewMemoryEdge;
    use crate::memory::store::{MemoryStore, MemoryType, NewMemory};

    fn open_store() -> MemoryStore {
        MemoryStore::in_memory()
    }

    fn add_basic(store: &MemoryStore, content: &str) -> i64 {
        store
            .add(NewMemory {
                content: content.to_string(),
                tags: "test".into(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .expect("add memory")
            .id
    }

    /// graphify `test_confidence.py` — verify rubric thresholds.
    #[test]
    fn confidence_label_matches_rubric() {
        let mk = |source: EdgeSource, c: f64| MemoryEdge {
            id: 0,
            src_id: 1,
            dst_id: 2,
            rel_type: "related_to".into(),
            confidence: c,
            source,
            created_at: 0,
            valid_from: None,
            valid_to: None,
            edge_source: None,
        };
        assert_eq!(
            confidence_label(&mk(EdgeSource::User, 1.0)),
            ConfidenceLabel::Extracted
        );
        assert_eq!(
            confidence_label(&mk(EdgeSource::Auto, 0.10)),
            ConfidenceLabel::Extracted,
            "Auto provenance should be Extracted regardless of float"
        );
        assert_eq!(
            confidence_label(&mk(EdgeSource::Llm, 0.96)),
            ConfidenceLabel::Extracted
        );
        assert_eq!(
            confidence_label(&mk(EdgeSource::Llm, 0.90)),
            ConfidenceLabel::InferredStrong
        );
        assert_eq!(
            confidence_label(&mk(EdgeSource::Llm, 0.70)),
            ConfidenceLabel::InferredWeak
        );
        assert_eq!(
            confidence_label(&mk(EdgeSource::Llm, 0.40)),
            ConfidenceLabel::Ambiguous
        );
    }

    /// graphify `test_cache.py` + `test_dedup.py` — same content twice → one row.
    #[test]
    fn dedup_skips_identical_source() {
        let store = open_store();
        let r1 = ensure_source_dedup(
            &store,
            Some("https://example.com/a"),
            "Mars has two moons.",
            "astronomy",
            3,
        )
        .expect("first ingest");
        let r2 = ensure_source_dedup(
            &store,
            Some("https://example.com/a"),
            "Mars has two moons.",
            "astronomy",
            3,
        )
        .expect("second ingest");
        let SourceDedupResult::Ingested { entry_id: id1 } = r1 else {
            panic!("expected Ingested for first call, got {:?}", r1)
        };
        let SourceDedupResult::Skipped { existing_id: id2 } = r2 else {
            panic!("expected Skipped for second call, got {:?}", r2)
        };
        assert_eq!(id1, id2, "dedup must point at the original row");
        let all = store.get_all().expect("get_all");
        assert_eq!(all.len(), 1, "exactly one row should exist after dedup");
    }

    /// graphify `test_incremental.py` — different content under same URL inserts a new row.
    #[test]
    fn incremental_reingest_updates_only_changed() {
        let store = open_store();
        ensure_source_dedup(&store, Some("u"), "v1 of the doc", "doc", 3).unwrap();
        let r2 =
            ensure_source_dedup(&store, Some("u"), "v2 of the doc — updated", "doc", 3).unwrap();
        assert!(matches!(r2, SourceDedupResult::Ingested { .. }));
        assert_eq!(store.get_all().unwrap().len(), 2);
    }

    /// graphify `test_validate.py` — orphan detection.
    #[test]
    fn audit_detects_orphans_and_open_conflicts() {
        let store = open_store();
        let a = add_basic(&store, "A is a fact.");
        let b = add_basic(&store, "B is a fact.");
        let _orphan = add_basic(&store, "Orphan with no edges.");
        store
            .add_edge(NewMemoryEdge {
                src_id: a,
                dst_id: b,
                rel_type: "related_to".into(),
                confidence: 1.0,
                source: EdgeSource::User,
                valid_from: None,
                valid_to: None,
                edge_source: None,
            })
            .expect("add edge");
        store
            .add_conflict(a, b, "test conflict")
            .expect("add conflict");

        let report = audit_report(&store, &AuditConfig::default()).expect("audit");
        assert_eq!(report.total_memories, 3);
        assert_eq!(report.total_edges, 1);
        assert_eq!(report.open_conflicts.len(), 1);
        assert_eq!(report.orphan_ids.len(), 1, "exactly one orphan expected");
    }

    /// graphify `test_analyze.py` — top-connected memories.
    #[test]
    fn god_nodes_returns_top_connected() {
        let store = open_store();
        let hub = add_basic(&store, "Hub memory.");
        for i in 0..5 {
            let leaf = add_basic(&store, &format!("Leaf {i}"));
            store
                .add_edge(NewMemoryEdge {
                    src_id: hub,
                    dst_id: leaf,
                    rel_type: "related_to".into(),
                    confidence: 1.0,
                    source: EdgeSource::User,
                    valid_from: None,
                    valid_to: None,
                    edge_source: None,
                })
                .unwrap();
        }
        let nodes = god_nodes(&store, 3).expect("god nodes");
        assert!(!nodes.is_empty());
        assert_eq!(nodes[0].entry.id, hub, "hub should rank first");
        assert_eq!(nodes[0].degree, 5);
    }

    /// graphify `test_analyze.py` — surprising = cross-community edges.
    #[test]
    fn surprising_connections_crosses_communities() {
        let store = open_store();
        store.ensure_communities_schema().unwrap();
        let a = add_basic(&store, "Member of cluster A.");
        let b = add_basic(&store, "Member of cluster B.");
        // Manually seed two distinct communities, then a bridging edge.
        store
            .save_communities(&[
                super::super::graph_rag::Community {
                    id: 0,
                    level: 0,
                    member_ids: vec![a],
                    summary: None,
                    embedding: None,
                    updated_at: 0,
                },
                super::super::graph_rag::Community {
                    id: 0,
                    level: 0,
                    member_ids: vec![b],
                    summary: None,
                    embedding: None,
                    updated_at: 0,
                },
            ])
            .unwrap();
        store
            .add_edge(NewMemoryEdge {
                src_id: a,
                dst_id: b,
                rel_type: "related_to".into(),
                confidence: 0.92,
                source: EdgeSource::Llm,
                valid_from: None,
                valid_to: None,
                edge_source: None,
            })
            .unwrap();
        let hits = surprising_connections(&store, 5).expect("surprises");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].label, ConfidenceLabel::InferredStrong);
    }

    /// Karpathy *append-and-review note* — ordering by gravity.
    #[test]
    fn revisit_queue_orders_by_gravity_ascending() {
        let store = open_store();
        let stale_id = add_basic(&store, "Stale claim — sinks to the bottom.");
        let fresh_id = add_basic(&store, "Fresh claim — floats at the top.");
        // Force decay on the stale row.
        store
            .conn()
            .execute(
                "UPDATE memories SET decay_score = 0.05, last_accessed = 0 WHERE id = ?1",
                params![stale_id],
            )
            .unwrap();
        store
            .conn()
            .execute(
                "UPDATE memories SET decay_score = 0.95, last_accessed = ?1 WHERE id = ?2",
                params![now_ms(), fresh_id],
            )
            .unwrap();

        let queue = append_and_review_queue(&store, 5).expect("revisit");
        assert!(!queue.is_empty());
        assert_eq!(
            queue[0].entry.id, stale_id,
            "stale row should be first (lowest gravity)"
        );
        assert!(queue[0].gravity < queue.last().unwrap().gravity);
    }

    /// Fingerprint stability test — same bytes → same hex.
    #[test]
    fn fingerprint_is_stable_and_unique() {
        let a = fingerprint(b"hello world");
        let b = fingerprint(b"hello world");
        let c = fingerprint(b"hello world!");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64, "SHA-256 hex is 64 chars");
    }
}
