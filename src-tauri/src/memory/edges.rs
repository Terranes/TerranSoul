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
///
/// **Temporal validity (V6):** `valid_from` and `valid_to` are
/// optional Unix-ms timestamps that bound when the edge is considered
/// true. `valid_from = None` means "valid since the beginning of
/// time", `valid_to = None` means "still valid". The interval is
/// **right-exclusive**: an edge with `valid_from = t1, valid_to = t2`
/// is true for `t in [t1, t2)`. See [`MemoryEdge::is_valid_at`].
///
/// This shape lets the brain answer point-in-time queries ("what was
/// true on date X?") and represents superseded facts as a
/// non-destructive update — close the previous edge with
/// `valid_to = now` and insert a new one with `valid_from = now`.
/// Implements the Zep / Graphiti temporal-KG pattern, 2024.
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
    /// Inclusive lower bound of the temporal validity interval, or
    /// `None` for "valid since the beginning of time".
    #[serde(default)]
    pub valid_from: Option<i64>,
    /// Exclusive upper bound of the temporal validity interval, or
    /// `None` for "still valid".
    #[serde(default)]
    pub valid_to: Option<i64>,
    /// Optional external knowledge-graph provenance (V7).
    ///
    /// `None` for native TerranSoul edges (the default). Mirrors of
    /// external KGs use `<system>:<scope>` strings — for example
    /// `gitnexus:repo:owner/name@sha`. See
    /// [`crate::memory::gitnexus_mirror`] for the canonical format used
    /// by the GitNexus integration (Phase 13 Tier 3).
    #[serde(default)]
    pub edge_source: Option<String>,
}

impl MemoryEdge {
    /// Returns `true` if this edge is in its validity interval at
    /// the given Unix-ms timestamp `t`. The interval is right-exclusive:
    /// an edge with `valid_to == Some(t)` is **not** valid at `t`.
    ///
    /// `valid_from = None` (open lower bound) is treated as "always
    /// has been valid"; `valid_to = None` (open upper bound) is
    /// treated as "still valid". Pure function — used by both the
    /// in-memory filter on `get_edges_for_at` and unit tests.
    pub fn is_valid_at(&self, t: i64) -> bool {
        let lower_ok = self.valid_from.is_none_or(|from| from <= t);
        let upper_ok = self.valid_to.is_none_or(|to| t < to);
        lower_ok && upper_ok
    }
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
    /// Optional inclusive lower bound of the temporal validity interval.
    /// Omit (`None`) for "valid since the beginning of time".
    #[serde(default)]
    pub valid_from: Option<i64>,
    /// Optional exclusive upper bound of the temporal validity interval.
    /// Omit (`None`) for "still valid".
    #[serde(default)]
    pub valid_to: Option<i64>,
    /// Optional external knowledge-graph provenance (V7). See
    /// [`MemoryEdge::edge_source`] for the format.
    #[serde(default)]
    pub edge_source: Option<String>,
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
            "INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, valid_from, valid_to, edge_source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![e.src_id, e.dst_id, rel, confidence, source, now, e.valid_from, e.valid_to, e.edge_source],
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
                "INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, valid_from, valid_to, edge_source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            for e in edges {
                if e.src_id == e.dst_id {
                    continue;
                }
                let rel = normalise_rel_type(&e.rel_type);
                let confidence = e.confidence.clamp(0.0, 1.0);
                let source = e.source.as_str();
                let n = stmt.execute(params![
                    e.src_id, e.dst_id, rel, confidence, source, now,
                    e.valid_from, e.valid_to, e.edge_source,
                ])?;
                inserted += n;
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    /// Get an edge by its primary key.
    pub fn get_edge(&self, id: i64) -> SqlResult<MemoryEdge> {
        self.conn().query_row(
            "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at, valid_from, valid_to, edge_source
             FROM memory_edges WHERE id = ?1",
            params![id],
            row_to_edge,
        )
    }

    fn get_edge_unique(&self, src: i64, dst: i64, rel: &str) -> SqlResult<MemoryEdge> {
        self.conn().query_row(
            "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at, valid_from, valid_to, edge_source
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

    /// Delete every edge whose `edge_source` matches the given value.
    ///
    /// Used by external KG mirrors (e.g. the GitNexus Tier-3 sync) to
    /// undo a previous mirror without touching native or LLM-extracted
    /// edges. The match is exact: pass the same `edge_source` string
    /// that was used when inserting the edges (typically
    /// `gitnexus:repo:owner/name@sha`). Returns the number of rows
    /// deleted.
    pub fn delete_edges_by_edge_source(&self, edge_source: &str) -> SqlResult<usize> {
        let n = self.conn().execute(
            "DELETE FROM memory_edges WHERE edge_source = ?1",
            params![edge_source],
        )?;
        Ok(n)
    }

    /// Return all edges in the graph (ordered by id).
    pub fn list_edges(&self) -> SqlResult<Vec<MemoryEdge>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at, valid_from, valid_to, edge_source
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
                "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at, valid_from, valid_to, edge_source
                 FROM memory_edges WHERE src_id = ?1 ORDER BY confidence DESC, id ASC",
                vec![memory_id],
            ),
            EdgeDirection::In => (
                "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at, valid_from, valid_to, edge_source
                 FROM memory_edges WHERE dst_id = ?1 ORDER BY confidence DESC, id ASC",
                vec![memory_id],
            ),
            EdgeDirection::Both => (
                "SELECT id, src_id, dst_id, rel_type, confidence, source, created_at, valid_from, valid_to, edge_source
                 FROM memory_edges WHERE src_id = ?1 OR dst_id = ?1
                 ORDER BY confidence DESC, id ASC",
                vec![memory_id],
            ),
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(args), row_to_edge)?;
        rows.collect()
    }

    /// Point-in-time variant of [`MemoryStore::get_edges_for`].
    ///
    /// When `valid_at` is `Some(t)`, the result is filtered to edges
    /// that are *valid at the Unix-ms timestamp `t`* — i.e. their
    /// `valid_from` (if set) is ≤ `t` and their `valid_to` (if set)
    /// is > `t`. When `valid_at` is `None`, behaviour is identical to
    /// `get_edges_for`: every edge is returned regardless of temporal
    /// validity, preserving full backward compatibility for callers
    /// that don't yet pass a timestamp.
    ///
    /// Implementation: defers to `get_edges_for` and then applies the
    /// pure [`MemoryEdge::is_valid_at`] filter in Rust. The added
    /// `idx_edges_valid_to` index from V6 still pays off in the
    /// future when we push the predicate into SQL, but the in-memory
    /// filter keeps this implementation simple and exhaustively
    /// unit-testable today (graphs that fit in one SQLite page).
    pub fn get_edges_for_at(
        &self,
        memory_id: i64,
        direction: EdgeDirection,
        valid_at: Option<i64>,
    ) -> SqlResult<Vec<MemoryEdge>> {
        let edges = self.get_edges_for(memory_id, direction)?;
        Ok(match valid_at {
            None => edges,
            Some(t) => edges.into_iter().filter(|e| e.is_valid_at(t)).collect(),
        })
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
        valid_from: row.get(7)?,
        valid_to: row.get(8)?,
        edge_source: row.get(9)?,
    })
}

impl MemoryStore {
    /// Close an edge's validity interval at timestamp `t`.
    ///
    /// Sets `valid_to = t` on the edge with the given id and returns
    /// the number of rows updated (`0` if the id was unknown). Does
    /// **not** mutate `valid_from`. Idempotent: re-closing an edge
    /// that already has `valid_to == Some(t)` is a no-op semantically
    /// (the row count is still `1`, but the column value is
    /// unchanged).
    ///
    /// This is the primary mutation used to record a *superseded
    /// fact* in the temporal KG. Pair with [`MemoryStore::add_edge`]
    /// using `valid_from = Some(t)` on the replacement edge to express
    /// "the previous edge was true until `t`, the new edge is true
    /// from `t` onwards".
    pub fn close_edge(&self, edge_id: i64, valid_to: i64) -> SqlResult<usize> {
        self.conn().execute(
            "UPDATE memory_edges SET valid_to = ?1 WHERE id = ?2",
            params![valid_to, edge_id],
        )
    }
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
                // The LLM extractor doesn't currently propose temporal
                // bounds; treat extracted edges as "valid since insert".
                // Future work can enrich the prompt to elicit explicit
                // valid_from / valid_to fields.
                valid_from: None,
                valid_to: None,
                edge_source: None,
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
                source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None })
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
            source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None });
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
            source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None };
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
                source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None })
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
            NewMemoryEdge { src_id: a, dst_id: b, rel_type: "rel".into(), confidence: 1.0, source: EdgeSource::Llm, valid_from: None, valid_to: None, edge_source: None },
            NewMemoryEdge { src_id: a, dst_id: b, rel_type: "rel".into(), confidence: 1.0, source: EdgeSource::Llm, valid_from: None, valid_to: None, edge_source: None },  // dup
            NewMemoryEdge { src_id: a, dst_id: a, rel_type: "self".into(), confidence: 1.0, source: EdgeSource::Llm, valid_from: None, valid_to: None, edge_source: None }, // self-loop
            NewMemoryEdge { src_id: b, dst_id: c, rel_type: "rel".into(), confidence: 0.5, source: EdgeSource::Llm, valid_from: None, valid_to: None, edge_source: None },
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
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "r1".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: b, dst_id: c, rel_type: "r2".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();
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
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "r1".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: c, dst_id: b, rel_type: "r2".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();
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
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "n".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: b, dst_id: c, rel_type: "n".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: c, dst_id: d, rel_type: "n".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: d, dst_id: a, rel_type: "cycle".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();

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
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "good".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: c, rel_type: "bad".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();
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
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "cites".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: c, rel_type: "cites".into(), confidence: 1.0, source: EdgeSource::Llm, valid_from: None, valid_to: None, edge_source: None }).unwrap();
        store.add_edge(NewMemoryEdge { src_id: b, dst_id: c, rel_type: "related_to".into(), confidence: 1.0, source: EdgeSource::Llm, valid_from: None, valid_to: None, edge_source: None }).unwrap();

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
                source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None })
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
        store.add_edge(NewMemoryEdge { src_id: a, dst_id: b, rel_type: "rel".into(), confidence: 1.0, source: EdgeSource::User, valid_from: None, valid_to: None, edge_source: None }).unwrap();
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

    // ── V6 — Temporal knowledge-graph tests ───────────────────────────

    /// Helper: build a temporal edge between two fresh memories.
    fn add_temporal(store: &MemoryStore, rel: &str,
                    valid_from: Option<i64>, valid_to: Option<i64>) -> MemoryEdge {
        let a = make_memory(store, &format!("A-{rel}-{:?}-{:?}", valid_from, valid_to));
        let b = make_memory(store, &format!("B-{rel}-{:?}-{:?}", valid_from, valid_to));
        store.add_edge(NewMemoryEdge {
            src_id: a, dst_id: b,
            rel_type: rel.into(),
            confidence: 1.0,
            source: EdgeSource::User,
            valid_from, valid_to,
            edge_source: None,
        }).unwrap()
    }

    #[test]
    fn is_valid_at_open_open_interval_always_true() {
        let e = add_temporal(&MemoryStore::in_memory(), "rel", None, None);
        assert!(e.is_valid_at(0));
        assert!(e.is_valid_at(i64::MAX));
        assert!(e.is_valid_at(-1));
    }

    #[test]
    fn is_valid_at_respects_lower_bound_inclusive() {
        let e = add_temporal(&MemoryStore::in_memory(), "rel", Some(100), None);
        assert!(!e.is_valid_at(99));
        assert!(e.is_valid_at(100), "valid_from is inclusive");
        assert!(e.is_valid_at(1_000_000));
    }

    #[test]
    fn is_valid_at_respects_upper_bound_exclusive() {
        let e = add_temporal(&MemoryStore::in_memory(), "rel", None, Some(200));
        assert!(e.is_valid_at(0));
        assert!(e.is_valid_at(199));
        assert!(!e.is_valid_at(200), "valid_to is exclusive");
        assert!(!e.is_valid_at(201));
    }

    #[test]
    fn is_valid_at_closed_interval() {
        let e = add_temporal(&MemoryStore::in_memory(), "rel", Some(100), Some(200));
        assert!(!e.is_valid_at(99));
        assert!(e.is_valid_at(100));
        assert!(e.is_valid_at(150));
        assert!(e.is_valid_at(199));
        assert!(!e.is_valid_at(200));
    }

    #[test]
    fn add_edge_persists_validity_columns() {
        let store = MemoryStore::in_memory();
        let e = add_temporal(&store, "loved", Some(1_000), Some(2_000));
        let reloaded = store.get_edge(e.id).unwrap();
        assert_eq!(reloaded.valid_from, Some(1_000));
        assert_eq!(reloaded.valid_to,   Some(2_000));
    }

    #[test]
    fn list_edges_round_trips_validity_columns() {
        let store = MemoryStore::in_memory();
        add_temporal(&store, "rel", None, None);
        add_temporal(&store, "rel", Some(50), None);
        add_temporal(&store, "rel", Some(50), Some(150));
        let all = store.list_edges().unwrap();
        assert_eq!(all.len(), 3);
        let intervals: Vec<(Option<i64>, Option<i64>)> =
            all.iter().map(|e| (e.valid_from, e.valid_to)).collect();
        assert!(intervals.contains(&(None, None)));
        assert!(intervals.contains(&(Some(50), None)));
        assert!(intervals.contains(&(Some(50), Some(150))));
    }

    #[test]
    fn get_edges_for_at_with_none_returns_all_edges() {
        let store = MemoryStore::in_memory();
        let src = make_memory(&store, "src");
        let d1 = make_memory(&store, "d1");
        let d2 = make_memory(&store, "d2");
        // closed past edge + open present edge
        store.add_edge(NewMemoryEdge {
            src_id: src, dst_id: d1, rel_type: "rel".into(),
            confidence: 1.0, source: EdgeSource::User,
            valid_from: Some(0), valid_to: Some(100),
            edge_source: None,
        }).unwrap();
        store.add_edge(NewMemoryEdge {
            src_id: src, dst_id: d2, rel_type: "rel".into(),
            confidence: 1.0, source: EdgeSource::User,
            valid_from: Some(100), valid_to: None,
            edge_source: None,
        }).unwrap();
        // valid_at = None preserves legacy behaviour: returns both.
        let all = store.get_edges_for_at(src, EdgeDirection::Out, None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn get_edges_for_at_filters_to_point_in_time() {
        let store = MemoryStore::in_memory();
        let src = make_memory(&store, "src");
        let d1 = make_memory(&store, "d1");
        let d2 = make_memory(&store, "d2");
        let d3 = make_memory(&store, "d3");
        // Past closed: [0, 100)
        let past = store.add_edge(NewMemoryEdge {
            src_id: src, dst_id: d1, rel_type: "rel".into(),
            confidence: 1.0, source: EdgeSource::User,
            valid_from: Some(0), valid_to: Some(100),
            edge_source: None,
        }).unwrap();
        // Present open-ended: [100, ∞)
        let present = store.add_edge(NewMemoryEdge {
            src_id: src, dst_id: d2, rel_type: "rel".into(),
            confidence: 1.0, source: EdgeSource::User,
            valid_from: Some(100), valid_to: None,
            edge_source: None,
        }).unwrap();
        // Always: (-∞, ∞)
        let always = store.add_edge(NewMemoryEdge {
            src_id: src, dst_id: d3, rel_type: "rel".into(),
            confidence: 1.0, source: EdgeSource::User,
            valid_from: None, valid_to: None,
            edge_source: None,
        }).unwrap();

        // Before everything started: only "always" survives.
        let at_neg = store.get_edges_for_at(src, EdgeDirection::Out, Some(-1)).unwrap();
        let ids_neg: HashSet<i64> = at_neg.iter().map(|e| e.id).collect();
        assert_eq!(ids_neg, [always.id].into_iter().collect());

        // Mid past window: "past" + "always"; "present" hasn't started.
        let at_50 = store.get_edges_for_at(src, EdgeDirection::Out, Some(50)).unwrap();
        let ids_50: HashSet<i64> = at_50.iter().map(|e| e.id).collect();
        assert_eq!(ids_50, [past.id, always.id].into_iter().collect());

        // Right at boundary 100: "past" closes (exclusive), "present" opens (inclusive).
        let at_100 = store.get_edges_for_at(src, EdgeDirection::Out, Some(100)).unwrap();
        let ids_100: HashSet<i64> = at_100.iter().map(|e| e.id).collect();
        assert_eq!(ids_100, [present.id, always.id].into_iter().collect());

        // Far future: "present" + "always".
        let at_huge = store.get_edges_for_at(src, EdgeDirection::Out, Some(1_000_000)).unwrap();
        let ids_huge: HashSet<i64> = at_huge.iter().map(|e| e.id).collect();
        assert_eq!(ids_huge, [present.id, always.id].into_iter().collect());
    }

    #[test]
    fn close_edge_sets_valid_to_and_returns_row_count() {
        let store = MemoryStore::in_memory();
        let e = add_temporal(&store, "rel", None, None);
        assert_eq!(e.valid_to, None);

        let updated = store.close_edge(e.id, 500).unwrap();
        assert_eq!(updated, 1);

        let reloaded = store.get_edge(e.id).unwrap();
        assert_eq!(reloaded.valid_to, Some(500));
        // valid_from is preserved (NULL → None).
        assert_eq!(reloaded.valid_from, None);
    }

    #[test]
    fn close_edge_unknown_id_returns_zero() {
        let store = MemoryStore::in_memory();
        let n = store.close_edge(99_999, 1).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn close_edge_then_query_at_drops_the_closed_edge() {
        let store = MemoryStore::in_memory();
        let src = make_memory(&store, "src");
        let dst = make_memory(&store, "dst");
        let e = store.add_edge(NewMemoryEdge {
            src_id: src, dst_id: dst, rel_type: "rel".into(),
            confidence: 1.0, source: EdgeSource::User,
            valid_from: Some(0), valid_to: None,
            edge_source: None,
        }).unwrap();

        // Before close: visible at t=500.
        let before = store.get_edges_for_at(src, EdgeDirection::Out, Some(500)).unwrap();
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].id, e.id);

        // Close at t=400 (valid_to=400 means edge ends just before 400).
        store.close_edge(e.id, 400).unwrap();

        // After close: gone at t=500, still present at t=399.
        let at_500 = store.get_edges_for_at(src, EdgeDirection::Out, Some(500)).unwrap();
        assert!(at_500.is_empty(), "closed edge must not appear after valid_to");
        let at_399 = store.get_edges_for_at(src, EdgeDirection::Out, Some(399)).unwrap();
        assert_eq!(at_399.len(), 1, "closed edge stays visible inside its window");
    }

    #[test]
    fn supersession_pattern_close_then_replace_returns_one_edge_per_timepoint() {
        // Models the "fact X changed value at time T" pattern: close the
        // old edge, insert a new one with valid_from=T. Any single
        // point-in-time query should return exactly one edge.
        let store = MemoryStore::in_memory();
        let person = make_memory(&store, "person:Alice");
        let role_old = make_memory(&store, "role:engineer");
        let role_new = make_memory(&store, "role:manager");

        // Original fact: Alice was engineer from t=100 onwards.
        let old = store.add_edge(NewMemoryEdge {
            src_id: person, dst_id: role_old, rel_type: "has_role".into(),
            confidence: 1.0, source: EdgeSource::User,
            valid_from: Some(100), valid_to: None,
            edge_source: None,
        }).unwrap();

        // At t=500 she was promoted: close old, open new.
        store.close_edge(old.id, 500).unwrap();
        store.add_edge(NewMemoryEdge {
            src_id: person, dst_id: role_new, rel_type: "has_role".into(),
            confidence: 1.0, source: EdgeSource::User,
            valid_from: Some(500), valid_to: None,
            edge_source: None,
        }).unwrap();

        // Mid-old: returns engineer.
        let at_300 = store.get_edges_for_at(person, EdgeDirection::Out, Some(300)).unwrap();
        assert_eq!(at_300.len(), 1);
        assert_eq!(at_300[0].dst_id, role_old);

        // Boundary at 500: engineer ended (exclusive), manager started (inclusive).
        let at_500 = store.get_edges_for_at(person, EdgeDirection::Out, Some(500)).unwrap();
        assert_eq!(at_500.len(), 1);
        assert_eq!(at_500[0].dst_id, role_new);

        // Later: still manager.
        let at_999 = store.get_edges_for_at(person, EdgeDirection::Out, Some(999)).unwrap();
        assert_eq!(at_999.len(), 1);
        assert_eq!(at_999[0].dst_id, role_new);

        // Without time filter: both edges visible (full history).
        let all = store.get_edges_for_at(person, EdgeDirection::Out, None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn legacy_get_edges_for_returns_temporal_edges_unfiltered() {
        // The legacy non-temporal API must keep returning every edge
        // regardless of validity intervals — proves backward compatibility.
        let store = MemoryStore::in_memory();
        let src = make_memory(&store, "src");
        let dst = make_memory(&store, "dst");
        store.add_edge(NewMemoryEdge {
            src_id: src, dst_id: dst, rel_type: "rel".into(),
            confidence: 1.0, source: EdgeSource::User,
            valid_from: Some(0), valid_to: Some(1),  // closed in the distant past
            edge_source: None,
        }).unwrap();
        let legacy = store.get_edges_for(src, EdgeDirection::Out).unwrap();
        assert_eq!(legacy.len(), 1, "legacy API must ignore temporal bounds");
    }
}
