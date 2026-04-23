//! Entity-Relationship Graph — typed, directional edges between memories.
//!
//! Promotes the memory store from a tag-based co-occurrence graph to a
//! proper knowledge graph. See `docs/brain-advanced-design.md` §6 for the
//! full design and `migrations.rs` (V5) for the schema.
//!
//! Directional, typed edges enable:
//! - **Multi-hop RAG** — start from a vector hit and walk the graph for
//!   topically connected memories that don't share keywords.
//! - **Provenance** — `source` records whether an edge was asserted by the
//!   user, extracted by the LLM, or derived automatically.
//! - **Idempotent extraction** — `(src_id, dst_id, rel_type)` is unique so
//!   re-running edge extraction never duplicates an edge.

use rusqlite::{params, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

use super::store::{MemoryEntry, MemoryStore, MemoryType, MemoryTier};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Curated relation vocabulary. The schema accepts any string, but the
/// application normalises common types so the UI can colour them consistently
/// and the LLM extractor knows what to emit.
pub const COMMON_RELATION_TYPES: &[&str] = &[
    "related_to",
    "contains",
    "cites",
    "governs",
    "part_of",
    "depends_on",
    "supersedes",
    "contradicts",
    "derived_from",
    "mentions",
    "located_in",
    "studies",
    "prefers",
    "knows",
    "owns",
    "mother_of",
    "child_of",
];

/// Where an edge came from — used for UI provenance and LLM-extraction
/// gating (e.g. "delete only LLM-proposed edges").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeSource {
    /// User explicitly created this edge.
    #[default]
    User,
    /// LLM proposed this edge from `extract_edges`.
    Llm,
    /// Derived automatically (e.g. by tag overlap, ingest pipeline).
    Auto,
}

impl EdgeSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeSource::User => "user",
            EdgeSource::Llm => "llm",
            EdgeSource::Auto => "auto",
        }
    }

    /// Parse a string label back into an `EdgeSource`. Unknown strings fall
    /// back to `User` (the most conservative provenance).
    pub fn parse(s: &str) -> Self {
        match s {
            "llm" => EdgeSource::Llm,
            "auto" => EdgeSource::Auto,
            _ => EdgeSource::User,
        }
    }
}

/// A typed, directional edge between two memories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryEdge {
    pub id: i64,
    pub src_id: i64,
    pub dst_id: i64,
    /// Free-form relation label; see `COMMON_RELATION_TYPES`.
    pub rel_type: String,
    /// LLM-reported confidence in [0.0, 1.0]. User-asserted edges store 1.0.
    pub confidence: f64,
    pub source: EdgeSource,
    pub created_at: i64,
}

/// Fields needed to insert a new edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMemoryEdge {
    pub src_id: i64,
    pub dst_id: i64,
    pub rel_type: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub source: EdgeSource,
}

fn default_confidence() -> f64 { 1.0 }

/// Direction of edge traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeDirection {
    /// Outgoing edges (where memory is the src).
    Out,
    /// Incoming edges (where memory is the dst).
    In,
    /// Both directions.
    Both,
}

/// Aggregated graph statistics.
#[derive(Debug, Clone, Serialize)]
pub struct EdgeStats {
    pub total_edges: i64,
    /// Top relation types by count, descending.
    pub by_rel_type: Vec<(String, i64)>,
    pub by_source: Vec<(String, i64)>,
    /// Number of memories that have at least one edge.
    pub connected_memories: i64,
}

/// Normalise a relation type string: trim, lowercase, replace spaces with `_`.
/// Empty strings collapse to `related_to`.
pub fn normalise_rel_type(input: &str) -> String {
    let cleaned: String = input
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_whitespace() || c == '-' { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if cleaned.is_empty() {
        "related_to".to_string()
    } else {
        cleaned
    }
}

// ── Edge operations on MemoryStore ────────────────────────────────────────────

impl MemoryStore {
    /// Insert a new typed edge between two memories.
    ///
    /// Returns the inserted edge. If `(src_id, dst_id, rel_type)` already
    /// exists the call is a no-op and the existing row is returned (idempotent).
    /// Self-loops (`src_id == dst_id`) are rejected.
    pub fn add_edge(&self, e: NewMemoryEdge) -> SqlResult<MemoryEdge> {
        if e.src_id == e.dst_id {
            return Err(rusqlite::Error::InvalidParameterName(
                "src_id and dst_id must differ (self-loops are not allowed)".to_string(),
            ));
        }
        let rel = normalise_rel_type(&e.rel_type);
        let confidence = e.confidence.clamp(0.0, 1.0);
        let source = e.source.as_str();
        let now = now_ms();

        // Upsert-style insert: if the unique constraint fires, fetch the row.
        self.conn().execute(
            "INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![e.src_id, e.dst_id, rel, confidence, source, now],
        )?;
        // Fetch (whether newly inserted or already-present).
        self.get_edge_unique(e.src_id, e.dst_id, &rel)
    }

    /// Insert a batch of edges in a single transaction. Returns the count of
    /// rows actually inserted (excluding pre-existing duplicates).
    pub fn add_edges_batch(&self, edges: &[NewMemoryEdge]) -> SqlResult<usize> {
        if edges.is_empty() {
            return Ok(0);
        }
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        let now = now_ms();
        let mut inserted = 0usize;
        {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for e in edges {
                if e.src_id == e.dst_id {
                    continue;
                }
                let rel = normalise_rel_type(&e.rel_type);
                let confidence = e.confidence.clamp(0.0, 1.0);
                let source = e.source.as_str();
                let n = stmt.execute(params![e.src_id, e.dst_id, rel, confidence, source, now])?;
                inserted += n;
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    /// Get an edge by its primary key.
    pub fn get_edge(&self, id: i64) -> SqlResult<MemoryEdge> {
        self.conn().query_row(
            "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at
             FROM memory_edges WHERE id = ?1",
            params![id],
            row_to_edge,
        )
    }

    fn get_edge_unique(&self, src: i64, dst: i64, rel: &str) -> SqlResult<MemoryEdge> {
        self.conn().query_row(
            "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at
             FROM memory_edges WHERE src_id = ?1 AND dst_id = ?2 AND rel_type = ?3",
            params![src, dst, rel],
            row_to_edge,
        )
    }

    /// Delete an edge by its primary key.
    pub fn delete_edge(&self, id: i64) -> SqlResult<()> {
        self.conn()
            .execute("DELETE FROM memory_edges WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Delete all edges incident to a memory (in or out).
    /// Normally unnecessary because of `ON DELETE CASCADE`, but exposed for
    /// explicit graph-pruning operations.
    pub fn delete_edges_for_memory(&self, memory_id: i64) -> SqlResult<usize> {
        let n = self.conn().execute(
            "DELETE FROM memory_edges WHERE src_id = ?1 OR dst_id = ?1",
            params![memory_id],
        )?;
        Ok(n)
    }

    /// Return all edges in the graph (ordered by id).
    pub fn list_edges(&self) -> SqlResult<Vec<MemoryEdge>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at
             FROM memory_edges ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], row_to_edge)?;
        rows.collect()
    }

    /// Return edges incident to a memory in the requested direction.
    pub fn get_edges_for(
        &self,
        memory_id: i64,
        direction: EdgeDirection,
    ) -> SqlResult<Vec<MemoryEdge>> {
        let conn = self.conn();
        let (sql, args): (&str, Vec<i64>) = match direction {
            EdgeDirection::Out => (
                "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at
                 FROM memory_edges WHERE src_id = ?1 ORDER BY confidence DESC, id ASC",
                vec![memory_id],
            ),
            EdgeDirection::In => (
                "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at
                 FROM memory_edges WHERE dst_id = ?1 ORDER BY confidence DESC, id ASC",
                vec![memory_id],
            ),
            EdgeDirection::Both => (
                "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at
                 FROM memory_edges WHERE src_id = ?1 OR dst_id = ?1
                 ORDER BY confidence DESC, id ASC",
                vec![memory_id],
            ),
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(args), row_to_edge)?;
        rows.collect()
    }

    /// Count edges and produce aggregate stats.
    pub fn edge_stats(&self) -> SqlResult<EdgeStats> {
        let conn = self.conn();
        let total_edges: i64 =
            conn.query_row("SELECT COUNT(*) FROM memory_edges", [], |r| r.get(0))?;

        let by_rel_type: Vec<(String, i64)> = {
            let mut stmt = conn.prepare(
                "SELECT rel_type, COUNT(*) AS n FROM memory_edges
                 GROUP BY rel_type ORDER BY n DESC, rel_type ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            rows.filter_map(|r| r.ok()).collect()
        };

        let by_source: Vec<(String, i64)> = {
            let mut stmt = conn.prepare(
                "SELECT source, COUNT(*) AS n FROM memory_edges
                 GROUP BY source ORDER BY n DESC, source ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            rows.filter_map(|r| r.ok()).collect()
        };

        let connected_memories: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT id) FROM (
                SELECT src_id AS id FROM memory_edges
                UNION
                SELECT dst_id AS id FROM memory_edges
             )",
            [],
            |r| r.get(0),
        )?;

        Ok(EdgeStats {
            total_edges,
            by_rel_type,
            by_source,
            connected_memories,
        })
    }

    /// Cycle-safe BFS traversal starting from `start_id`. Returns the set of
    /// memory ids reachable within `max_hops` hops, ordered by hop distance
    /// (closest first). The starting memory itself is **excluded** from the
    /// result so the caller can cleanly merge it with vector search hits.
    ///
    /// `rel_filter` (if Some) restricts traversal to edges of those types.
    /// Traversal walks edges in BOTH directions — a knowledge-graph hop is
    /// undirected for retrieval purposes (you want neighbours of an entity
    /// regardless of which way the edge points).
    pub fn traverse_from(
        &self,
        start_id: i64,
        max_hops: usize,
        rel_filter: Option<&[String]>,
    ) -> SqlResult<Vec<(i64, usize)>> {
        if max_hops == 0 {
            return Ok(vec![]);
        }
        let conn = self.conn();
        let mut visited: HashSet<i64> = HashSet::new();
        visited.insert(start_id);
        let mut queue: VecDeque<(i64, usize)> = VecDeque::new();
        queue.push_back((start_id, 0));
        let mut out: Vec<(i64, usize)> = Vec::new();

        // Build the rel-filter clause once.
        let filter_clause = rel_filter
            .filter(|f| !f.is_empty())
            .map(|f| {
                let placeholders = (0..f.len())
                    .map(|i| format!("?{}", i + 2))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(" AND rel_type IN ({placeholders})")
            })
            .unwrap_or_default();

        let sql = format!(
            "SELECT src_id, dst_id FROM memory_edges
             WHERE (src_id = ?1 OR dst_id = ?1){filter_clause}",
        );

        while let Some((node, depth)) = queue.pop_front() {
            if depth >= max_hops {
                continue;
            }
            let mut stmt = conn.prepare(&sql)?;
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            params_vec.push(Box::new(node));
            if let Some(f) = rel_filter {
                if !f.is_empty() {
                    for r in f {
                        params_vec.push(Box::new(normalise_rel_type(r)));
                    }
                }
            }
            let refs: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();
            let rows = stmt.query_map(refs.as_slice(), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?;
            for r in rows.flatten() {
                let neighbour = if r.0 == node { r.1 } else { r.0 };
                if visited.insert(neighbour) {
                    out.push((neighbour, depth + 1));
                    queue.push_back((neighbour, depth + 1));
                }
            }
        }
        Ok(out)
    }

    /// Multi-hop hybrid search: run `hybrid_search`, then for each top-k seed
    /// expand by graph traversal up to `hops` and merge results.
    ///
    /// Re-ranking weights:
    /// - Direct hybrid hits keep their score.
    /// - Graph-expansion hits are scored as `seed_score / (hop + 1)`.
    /// - Duplicates are kept at the highest score seen.
    pub fn hybrid_search_with_graph(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
        hops: usize,
    ) -> SqlResult<Vec<MemoryEntry>> {
        // Step 1 — direct hybrid search. Take a slightly larger seed pool than
        // `limit` so graph expansion has more entry points.
        let seed_pool = limit.max(5);
        let direct = self.hybrid_search(query, query_embedding, seed_pool)?;
        if direct.is_empty() || hops == 0 {
            // No graph expansion needed.
            let mut result = direct;
            result.truncate(limit);
            return Ok(result);
        }

        // Step 2 — assign decreasing scores to the direct hits and walk the
        // graph from each one.
        let mut scored: std::collections::HashMap<i64, (f64, MemoryEntry)> =
            std::collections::HashMap::new();
        // Direct hits get scores 1.0, 0.95, 0.90, ... so the absolute hybrid
        // ordering is preserved when no graph hops contribute.
        for (i, entry) in direct.iter().enumerate() {
            let score = (1.0 - (i as f64) * 0.05).max(0.1);
            scored.insert(entry.id, (score, entry.clone()));
        }

        // Step 3 — expand each direct hit by `hops` hops in the graph.
        for entry in &direct {
            let neighbours = self.traverse_from(entry.id, hops, None)?;
            let seed_score = scored
                .get(&entry.id)
                .map(|(s, _)| *s)
                .unwrap_or(1.0);
            for (nid, hop) in neighbours {
                let neighbour_score = seed_score / (hop as f64 + 1.0);
                if let Ok(neighbour_entry) = self.get_by_id(nid) {
                    scored
                        .entry(nid)
                        .and_modify(|(s, _)| {
                            if neighbour_score > *s {
                                *s = neighbour_score;
                            }
                        })
                        .or_insert((neighbour_score, neighbour_entry));
                }
            }
        }

        // Step 4 — sort by composite score and truncate.
        let mut ranked: Vec<(f64, MemoryEntry)> = scored.into_values().collect();
        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(limit);
        Ok(ranked.into_iter().map(|(_, e)| e).collect())
    }
}

fn row_to_edge(row: &rusqlite::Row<'_>) -> SqlResult<MemoryEdge> {
    Ok(MemoryEdge {
        id: row.get(0)?,
        src_id: row.get(1)?,
        dst_id: row.get(2)?,
        rel_type: row.get(3)?,
        confidence: row.get(4)?,
        source: EdgeSource::parse(&row.get::<_, String>(5).unwrap_or_default()),
        created_at: row.get(6)?,
    })
}

// ── LLM-extraction helpers (pure functions, no DB) ────────────────────────────

/// Parse the LLM's JSON-line edge proposal output into `NewMemoryEdge`s.
///
/// Expected format — one JSON object per line:
/// ```json
/// {"src_id": 12, "dst_id": 14, "rel_type": "contains", "confidence": 0.85}
/// ```
/// Lines that fail to parse, reference unknown ids, or describe self-loops
/// are silently skipped — the LLM is unreliable, the parser is forgiving.
pub fn parse_llm_edges(text: &str, known_ids: &HashSet<i64>) -> Vec<NewMemoryEdge> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim().trim_start_matches("- ").trim();
            if trimmed.is_empty() || !trimmed.starts_with('{') {
                return None;
            }
            #[derive(Deserialize)]
            struct Raw {
                src_id: i64,
                dst_id: i64,
                rel_type: String,
                #[serde(default = "default_confidence_raw")]
                confidence: f64,
            }
            fn default_confidence_raw() -> f64 { 0.7 }

            let parsed: Raw = serde_json::from_str(trimmed).ok()?;
            if parsed.src_id == parsed.dst_id {
                return None;
            }
            if !known_ids.contains(&parsed.src_id) || !known_ids.contains(&parsed.dst_id) {
                return None;
            }
            let rel = normalise_rel_type(&parsed.rel_type);
            if rel.is_empty() {
                return None;
            }
            Some(NewMemoryEdge {
                src_id: parsed.src_id,
                dst_id: parsed.dst_id,
                rel_type: rel,
                confidence: parsed.confidence.clamp(0.0, 1.0),
                source: EdgeSource::Llm,
            })
        })
        .collect()
}

/// Format a slice of memories into a numbered prompt-friendly list.
/// Used by the LLM edge-extraction prompt.
pub fn format_memories_for_extraction(entries: &[MemoryEntry]) -> String {
    entries
        .iter()
        .map(|e| {
            let snippet = if e.content.len() > 200 {
                format!("{}…", &e.content[..200])
            } else {
                e.content.clone()
            };
            let kind = match e.memory_type {
                MemoryType::Fact => "fact",
                MemoryType::Preference => "preference",
                MemoryType::Context => "context",
                MemoryType::Summary => "summary",
            };
            let tier = match e.tier {
                MemoryTier::Short => "short",
                MemoryTier::Working => "working",
                MemoryTier::Long => "long",
            };
            format!("id={} ({kind}, {tier}): {snippet}", e.id)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryStore, NewMemory};

    fn make_memory(store: &MemoryStore, content: &str) -> i64 {
        store
            .add(NewMemory {
                content: content.to_string(),
                tags: "test".to_string(),
                importance: 3,
                ..Default::default()
            })
            .unwrap()
            .id
    }

    #[test]
    fn add_edge_round_trip() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "Family Law");
        let b = make_memory(&store, "Rule 14.3");
        let edge = store
            .add_edge(NewMemoryEdge {
                src_id: a,
                dst_id: b,
                rel_type: "contains".to_string(),
                confidence: 0.9,
                source: EdgeSource::User,
            })
            .unwrap();
        assert_eq!(edge.src_id, a);
        assert_eq!(edge.dst_id, b);
        assert_eq!(edge.rel_type, "contains");
        assert!((edge.confidence - 0.9).abs() < 1e-6);
        assert_eq!(edge.source, EdgeSource::User);
    }

    #[test]
    fn add_edge_rejects_self_loop() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "self");
        let res = store.add_edge(NewMemoryEdge {
            src_id: a,
            dst_id: a,
            rel_type: "related_to".to_string(),
            confidence: 1.0,
            source: EdgeSource::User,
        });
        assert!(res.is_err(), "self-loops must be rejected");
    }

    #[test]
    fn add_edge_is_idempotent() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "A");
        let b = make_memory(&store, "B");
        let new_edge = || NewMemoryEdge {
            src_id: a,
            dst_id: b,
            rel_type: "cites".to_string(),
            confidence: 1.0,
            source: EdgeSource::User,
        };
        let e1 = store.add_edge(new_edge()).unwrap();
        let e2 = store.add_edge(new_edge()).unwrap();
        assert_eq!(e1.id, e2.id, "duplicate insert must return existing edge");
        assert_eq!(store.list_edges().unwrap().len(), 1);
    }

    #[test]
    fn rel_type_is_normalised() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "A");
        let b = make_memory(&store, "B");
        let edge = store
            .add_edge(NewMemoryEdge {
                src_id: a,
                dst_id: b,
                rel_type: "Mother Of".to_string(), // mixed case + space
                confidence: 1.0,
                source: EdgeSource::User,
            })
            .unwrap();
        assert_eq!(edge.rel_type, "mother_of");
    }

    #[test]
    fn add_edges_batch_skips_duplicates_and_self_loops() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "A");
        let b = make_memory(&store, "B");
        let c = make_memory(&store, "C");
        let edges = vec![
            NewMemoryEdge { src_id: a, dst_id: b, rel_type: "rel".into(), confidence: 1.0, source: EdgeSource::Llm },
            NewMemoryEdge { src_id: a, dst_id: b, rel_type: "rel".into(), confidence: 1.0, source: EdgeSource::Llm },  // dup
            NewMemoryEdge { src_id: a, dst_id: a, rel_type: "self".into(), confidence: 1.0, source: EdgeSource::Llm }, // self-loop
            NewMemoryEdge { src_id: b, dst_id: c, rel_type: "rel".into(), confidence: 0.5, source: EdgeSource::Llm },
        ];
        let n = store.add_edges_batch(&edges).unwrap();
        assert_eq!(n, 2, "expected 2 inserts (dup + self-loop dropped)");
        assert_eq!(store.list_edges().unwrap().len(), 2);
    }

    #[test]
    fn cascade_delete_removes_incident_edges() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "A");
        let b = make_memory(&store, "B");
        let c = make_memory(&store, "C");
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "r1".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: b, dst_id: c, rel_type: "r2".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();
        assert_eq!(store.list_edges().unwrap().len(), 2);
        store.delete(b).unwrap();
        // Both edges incident to b must be cascade-deleted.
        assert_eq!(store.list_edges().unwrap().len(), 0);
    }

    #[test]
    fn get_edges_for_directional() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "A");
        let b = make_memory(&store, "B");
        let c = make_memory(&store, "C");
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "r1".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: c, dst_id: b, rel_type: "r2".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();
        assert_eq!(store.get_edges_for(b, EdgeDirection::Out).unwrap().len(), 0);
        assert_eq!(store.get_edges_for(b, EdgeDirection::In).unwrap().len(), 2);
        assert_eq!(store.get_edges_for(b, EdgeDirection::Both).unwrap().len(), 2);
        assert_eq!(store.get_edges_for(a, EdgeDirection::Out).unwrap().len(), 1);
    }

    #[test]
    fn traverse_from_respects_max_hops_and_cycles() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "A");
        let b = make_memory(&store, "B");
        let c = make_memory(&store, "C");
        let d = make_memory(&store, "D");
        // Chain A → B → C → D, plus a cycle D → A
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "n".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: b, dst_id: c, rel_type: "n".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: c, dst_id: d, rel_type: "n".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: d, dst_id: a, rel_type: "cycle".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();

        let one = store.traverse_from(a, 1, None).unwrap();
        // 1-hop neighbours of A are {B, D} (D via the cycle edge).
        let ids: HashSet<i64> = one.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&b));
        assert!(ids.contains(&d));
        assert!(!ids.contains(&c));

        let three = store.traverse_from(a, 3, None).unwrap();
        let ids: HashSet<i64> = three.iter().map(|(id, _)| *id).collect();
        // Within 3 hops we reach B, C, D — but never re-visit A.
        assert!(ids.contains(&b) && ids.contains(&c) && ids.contains(&d));
        assert!(!ids.contains(&a));
    }

    #[test]
    fn traverse_from_filters_rel_type() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "A");
        let b = make_memory(&store, "B");
        let c = make_memory(&store, "C");
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "good".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: c, rel_type: "bad".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();
        let filtered = store
            .traverse_from(a, 2, Some(&["good".to_string()]))
            .unwrap();
        let ids: HashSet<i64> = filtered.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&b));
        assert!(!ids.contains(&c));
    }

    #[test]
    fn edge_stats_are_correct() {
        let store = MemoryStore::in_memory();
        let a = make_memory(&store, "A");
        let b = make_memory(&store, "B");
        let c = make_memory(&store, "C");
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "cites".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: c, rel_type: "cites".into(), confidence: 1.0, source: EdgeSource::Llm }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: b, dst_id: c, rel_type: "related_to".into(), confidence: 1.0, source: EdgeSource::Llm }).unwrap();

        let stats = store.edge_stats().unwrap();
        assert_eq!(stats.total_edges, 3);
        assert_eq!(stats.connected_memories, 3);
        // Top relation type by count is "cites" (2 edges).
        assert_eq!(stats.by_rel_type[0].0, "cites");
        assert_eq!(stats.by_rel_type[0].1, 2);
        // by_source: llm=2, user=1
        let llm = stats.by_source.iter().find(|(s, _)| s == "llm").unwrap().1;
        let user = stats.by_source.iter().find(|(s, _)| s == "user").unwrap().1;
        assert_eq!(llm, 2);
        assert_eq!(user, 1);
    }

    #[test]
    fn parse_llm_edges_skips_invalid() {
        let mut known = HashSet::new();
        known.insert(1i64);
        known.insert(2i64);
        let text = r#"
- {"src_id": 1, "dst_id": 2, "rel_type": "contains", "confidence": 0.8}
{"src_id": 1, "dst_id": 1, "rel_type": "self", "confidence": 1.0}
{"src_id": 999, "dst_id": 2, "rel_type": "unknown", "confidence": 0.5}
not json at all
{"src_id": 2, "dst_id": 1, "rel_type": "Cited By", "confidence": 1.5}
"#;
        let edges = parse_llm_edges(text, &known);
        assert_eq!(edges.len(), 2, "expected 2 valid edges");
        assert_eq!(edges[0].src_id, 1);
        assert_eq!(edges[0].dst_id, 2);
        assert_eq!(edges[0].rel_type, "contains");
        assert_eq!(edges[0].source, EdgeSource::Llm);
        // Confidence clamped to 1.0 and rel_type normalised.
        assert!((edges[1].confidence - 1.0).abs() < 1e-6);
        assert_eq!(edges[1].rel_type, "cited_by");
    }

    #[test]
    fn hybrid_search_with_graph_pulls_in_graph_neighbours() {
        let store = MemoryStore::in_memory();
        // Vector-keyword direct hit: contains the query word.
        let seed = store
            .add(NewMemory {
                content: "Family Law overview document".to_string(),
                tags: "law".into(),
                importance: 5,
                ..Default::default()
            })
            .unwrap()
            .id;
        // Graph-only neighbour: no keyword overlap with the query.
        let neighbour = store
            .add(NewMemory {
                content: "Rule 14.3 thirty day notice".to_string(),
                tags: "rule".into(),
                importance: 5,
                ..Default::default()
            })
            .unwrap()
            .id;
        // Distractor with no edge to the seed.
        let _distractor = store
            .add(NewMemory {
                content: "Cooking pasta recipe".to_string(),
                tags: "food".into(),
                importance: 1,
                ..Default::default()
            })
            .unwrap()
            .id;
        store
            .add_edge(NewMemoryEdge {
                src_id: seed,
                dst_id: neighbour,
                rel_type: "contains".into(),
                confidence: 1.0,
                source: EdgeSource::User,
            })
            .unwrap();
        let direct = store.hybrid_search("Family Law", None, 1).unwrap();
        assert_eq!(direct.len(), 1);
        assert_eq!(direct[0].id, seed);
        // With graph hops the neighbour should show up even though it doesn't
        // contain "Family Law" anywhere.
        let with_graph = store.hybrid_search_with_graph("Family Law", None, 5, 1).unwrap();
        let ids: HashSet<i64> = with_graph.iter().map(|e| e.id).collect();
        assert!(ids.contains(&seed));
        assert!(ids.contains(&neighbour),
            "graph traversal should pull in the neighbour memory");
    }

    #[test]
    fn migration_v5_round_trip() {
        // A fresh in-memory store is at TARGET_VERSION (≥ 5).
        let store = MemoryStore::in_memory();
        assert!(store.schema_version() >= 5);
        // Add a memory + edge, then downgrade to V4 — the edge table should
        // disappear, but the memory must survive.
        let a = make_memory(&store, "A");
        let b = make_memory(&store, "B");
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "rel".into(), confidence: 1.0, source: EdgeSource::User }).unwrap();
        assert_eq!(store.list_edges().unwrap().len(), 1);

        crate::memory::migrations::downgrade_to(store.conn(), 4).unwrap();
        assert_eq!(store.schema_version(), 4);
        // memory_edges no longer exists.
        let exists: Result<i64, _> = store
            .conn()
            .query_row("SELECT COUNT(*) FROM memory_edges", [], |r| r.get(0));
        assert!(exists.is_err(), "memory_edges should not exist at V4");
        // Re-upgrade to latest.
        crate::memory::migrations::migrate_to_latest(store.conn()).unwrap();
        assert!(store.schema_version() >= 5);
        // Edges from before are gone (table was dropped), memories survive.
        assert_eq!(store.list_edges().unwrap().len(), 0);
        assert_eq!(store.count(), 2);
    }

    #[test]
    fn format_memories_for_extraction_truncates_long_content() {
        let store = MemoryStore::in_memory();
        let long = "x".repeat(500);
        store.add(NewMemory { content: long, tags: "t".into(), importance: 3, ..Default::default() }).unwrap();
        let entries = store.get_all().unwrap();
        let s = format_memories_for_extraction(&entries);
        assert!(s.contains('…'));
        assert!(s.len() < 400);
    }
}
