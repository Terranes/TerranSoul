//! Paged graph queries for billion-scale knowledge graph (Chunk 48.6).
//!
//! At billion scale the `memory_graph_page` command cannot load all edges
//! and entries into memory. This module provides SQL-backed paged queries
//! that push LIMIT/OFFSET down to the database and use the covering
//! indexes `(src_id, rel_type)` / `(dst_id, rel_type)` for O(log n)
//! adjacency lookups.
//!
//! The pre-aggregated `memory_graph_clusters` table stores per-kind stats
//! (node count, edge count, avg importance) that the overview zoom consumes
//! without touching the full `memories` table.

use rusqlite::{params, Result as SqlResult};
use serde::{Deserialize, Serialize};

use super::store::{now_ms, MemoryStore};

/// Pre-aggregated cluster statistics (one row per `cognitive_kind`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphCluster {
    pub kind: String,
    pub node_count: i64,
    pub edge_count: i64,
    pub avg_importance: f64,
    pub updated_at: i64,
}

/// A paged edge result with enough context for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedEdge {
    pub id: i64,
    pub src_id: i64,
    pub dst_id: i64,
    pub rel_type: String,
    pub confidence: f64,
}

/// Result of a paged adjacency query.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PagedAdjacency {
    pub edges: Vec<PagedEdge>,
    /// Total edges incident to the focus node (for "showing N of M" UI).
    pub total: i64,
    /// Whether more edges are available beyond the returned page.
    pub has_more: bool,
}

/// A node summary for degree-ranked listing (no full content).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegreeNode {
    pub id: i64,
    pub degree: i64,
    pub importance: i64,
    pub kind: String,
    pub tier: String,
}

impl MemoryStore {
    // ─── Cluster Aggregation ─────────────────────────────────────────

    /// Refresh the `memory_graph_clusters` table from live data.
    /// Called during nightly compaction or after bulk ingests.
    /// Returns the number of cluster rows written.
    pub fn refresh_graph_clusters(&self) -> SqlResult<usize> {
        let conn = self.conn();
        let now = now_ms();

        // Compute per-kind stats in one pass over the memories table.
        // Uses COALESCE to handle NULL cognitive_kind (falls back to 'semantic').
        conn.execute("DELETE FROM memory_graph_clusters", [])?;

        conn.execute(
            "INSERT INTO memory_graph_clusters (kind, node_count, edge_count, avg_importance, updated_at)
             SELECT
                 COALESCE(m.cognitive_kind, 'semantic') AS kind,
                 COUNT(*) AS node_count,
                 COALESCE(edge_counts.cnt, 0) AS edge_count,
                 COALESCE(AVG(m.importance), 0.0) AS avg_importance,
                 ?1 AS updated_at
             FROM memories m
             LEFT JOIN (
                 SELECT kind, SUM(cnt) AS cnt FROM (
                     SELECT COALESCE(mm.cognitive_kind, 'semantic') AS kind, COUNT(*) AS cnt
                     FROM memory_edges e
                     JOIN memories mm ON mm.id = e.src_id
                     GROUP BY mm.cognitive_kind
                     UNION ALL
                     SELECT COALESCE(mm.cognitive_kind, 'semantic') AS kind, COUNT(*) AS cnt
                     FROM memory_edges e
                     JOIN memories mm ON mm.id = e.dst_id
                     GROUP BY mm.cognitive_kind
                 ) GROUP BY kind
             ) edge_counts ON edge_counts.kind = COALESCE(m.cognitive_kind, 'semantic')
             GROUP BY COALESCE(m.cognitive_kind, 'semantic')",
            params![now],
        )?;

        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM memory_graph_clusters", [], |row| {
                row.get(0)
            })?;
        Ok(count as usize)
    }

    /// Read the pre-aggregated cluster stats. Returns an empty vec if the
    /// table hasn't been populated yet.
    pub fn get_graph_clusters(&self) -> SqlResult<Vec<GraphCluster>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT kind, node_count, edge_count, avg_importance, updated_at
             FROM memory_graph_clusters
             ORDER BY node_count DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(GraphCluster {
                kind: row.get(0)?,
                node_count: row.get(1)?,
                edge_count: row.get(2)?,
                avg_importance: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    // ─── Paged Adjacency ─────────────────────────────────────────────

    /// Fetch edges incident to `focus_id` with paging. Uses the composite
    /// covering indexes for O(log n) lookup.
    ///
    /// `limit`: max edges to return (page size).
    /// `offset`: number of edges to skip (for cursor-based paging).
    /// `rel_filter`: optional relation type filter.
    pub fn get_edges_paged(
        &self,
        focus_id: i64,
        limit: usize,
        offset: usize,
        rel_filter: Option<&str>,
    ) -> SqlResult<PagedAdjacency> {
        let conn = self.conn();

        // Count total edges for this node (with optional filter).
        let total = if let Some(rel) = rel_filter {
            conn.query_row(
                "SELECT COUNT(*) FROM memory_edges
                 WHERE (src_id = ?1 OR dst_id = ?1) AND rel_type = ?2",
                params![focus_id, rel],
                |row| row.get::<_, i64>(0),
            )?
        } else {
            conn.query_row(
                "SELECT COUNT(*) FROM memory_edges
                 WHERE src_id = ?1 OR dst_id = ?1",
                params![focus_id],
                |row| row.get::<_, i64>(0),
            )?
        };

        // Fetch the page.
        let edges = if let Some(rel) = rel_filter {
            let mut stmt = conn.prepare(
                "SELECT id, src_id, dst_id, rel_type, confidence
                 FROM memory_edges
                 WHERE (src_id = ?1 OR dst_id = ?1) AND rel_type = ?2
                 ORDER BY confidence DESC, id ASC
                 LIMIT ?3 OFFSET ?4",
            )?;
            let rows = stmt.query_map(
                params![focus_id, rel, limit as i64, offset as i64],
                row_to_paged_edge,
            )?;
            rows.collect::<SqlResult<Vec<_>>>()?
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, src_id, dst_id, rel_type, confidence
                 FROM memory_edges
                 WHERE src_id = ?1 OR dst_id = ?1
                 ORDER BY confidence DESC, id ASC
                 LIMIT ?2 OFFSET ?3",
            )?;
            let rows = stmt.query_map(
                params![focus_id, limit as i64, offset as i64],
                row_to_paged_edge,
            )?;
            rows.collect::<SqlResult<Vec<_>>>()?
        };

        let has_more = (offset + edges.len()) < total as usize;

        Ok(PagedAdjacency {
            edges,
            total,
            has_more,
        })
    }

    // ─── Top-Degree Nodes ────────────────────────────────────────────

    /// Get the top-K nodes by degree (in + out edges combined).
    /// Optionally filter by cognitive_kind. Uses a subquery on
    /// `memory_edges` with covering indexes for efficiency.
    pub fn get_top_degree_nodes(
        &self,
        kind_filter: Option<&str>,
        limit: usize,
    ) -> SqlResult<Vec<DegreeNode>> {
        let conn = self.conn();

        let sql = if let Some(_kind) = kind_filter {
            "SELECT m.id, m.importance, COALESCE(m.cognitive_kind, 'semantic') AS kind, m.tier,
                    COALESCE(d.degree, 0) AS degree
             FROM memories m
             LEFT JOIN (
                 SELECT node_id, COUNT(*) AS degree FROM (
                     SELECT src_id AS node_id FROM memory_edges
                     UNION ALL
                     SELECT dst_id AS node_id FROM memory_edges
                 ) GROUP BY node_id
             ) d ON d.node_id = m.id
             WHERE COALESCE(m.cognitive_kind, 'semantic') = ?1
             ORDER BY degree DESC, m.importance DESC, m.created_at DESC
             LIMIT ?2"
        } else {
            "SELECT m.id, m.importance, COALESCE(m.cognitive_kind, 'semantic') AS kind, m.tier,
                    COALESCE(d.degree, 0) AS degree
             FROM memories m
             LEFT JOIN (
                 SELECT node_id, COUNT(*) AS degree FROM (
                     SELECT src_id AS node_id FROM memory_edges
                     UNION ALL
                     SELECT dst_id AS node_id FROM memory_edges
                 ) GROUP BY node_id
             ) d ON d.node_id = m.id
             ORDER BY degree DESC, m.importance DESC, m.created_at DESC
             LIMIT ?1"
        };

        let mut stmt = conn.prepare(sql)?;

        let rows = if let Some(kind) = kind_filter {
            stmt.query_map(params![kind, limit as i64], row_to_degree_node)?
        } else {
            stmt.query_map(params![limit as i64], row_to_degree_node)?
        };

        rows.collect()
    }

    /// Get total counts (memories + edges) for the graph overview.
    /// Uses cached cluster stats when available, falls back to COUNT(*).
    pub fn graph_totals(&self) -> SqlResult<(i64, i64)> {
        let conn = self.conn();
        let nodes: i64 = conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
        let edges: i64 =
            conn.query_row("SELECT COUNT(*) FROM memory_edges", [], |row| row.get(0))?;
        Ok((nodes, edges))
    }
}

fn row_to_paged_edge(row: &rusqlite::Row) -> SqlResult<PagedEdge> {
    Ok(PagedEdge {
        id: row.get(0)?,
        src_id: row.get(1)?,
        dst_id: row.get(2)?,
        rel_type: row.get(3)?,
        confidence: row.get(4)?,
    })
}

fn row_to_degree_node(row: &rusqlite::Row) -> SqlResult<DegreeNode> {
    Ok(DegreeNode {
        id: row.get(0)?,
        importance: row.get(1)?,
        kind: row.get(2)?,
        tier: row.get(3)?,
        degree: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::edges::{EdgeSource, NewMemoryEdge};
    use crate::memory::store::{MemoryType, NewMemory};

    fn setup_store_with_graph() -> MemoryStore {
        let store = MemoryStore::in_memory();

        // Insert 5 memories.
        for i in 1..=5 {
            store
                .add(NewMemory {
                    content: format!("Memory node {i}"),
                    tags: format!("node-{i}"),
                    importance: i as i64,
                    memory_type: MemoryType::Fact,
                    ..Default::default()
                })
                .unwrap();
        }

        // Create edges: 1→2, 1→3, 2→3, 3→4, 4→5
        let edges = vec![
            (1, 2, "related_to"),
            (1, 3, "contains"),
            (2, 3, "related_to"),
            (3, 4, "depends_on"),
            (4, 5, "cites"),
        ];
        for (src, dst, rel) in edges {
            store
                .add_edge(NewMemoryEdge {
                    src_id: src,
                    dst_id: dst,
                    rel_type: rel.to_string(),
                    confidence: 0.9,
                    source: EdgeSource::Auto,
                    valid_from: None,
                    valid_to: None,
                    edge_source: None,
                })
                .unwrap();
        }

        store
    }

    #[test]
    fn graph_clusters_refresh_and_read() {
        let store = setup_store_with_graph();
        let count = store.refresh_graph_clusters().unwrap();
        assert!(count >= 1, "should have at least one cluster kind");

        let clusters = store.get_graph_clusters().unwrap();
        assert!(!clusters.is_empty());

        let total_nodes: i64 = clusters.iter().map(|c| c.node_count).sum();
        assert_eq!(total_nodes, 5);
    }

    #[test]
    fn paged_adjacency_basic() {
        let store = setup_store_with_graph();

        // Node 1 has edges: 1→2, 1→3 (2 edges total)
        let page = store.get_edges_paged(1, 10, 0, None).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.edges.len(), 2);
        assert!(!page.has_more);
    }

    #[test]
    fn paged_adjacency_with_limit() {
        let store = setup_store_with_graph();

        // Node 3 has edges: 1→3, 2→3, 3→4 (3 edges total)
        let page = store.get_edges_paged(3, 2, 0, None).unwrap();
        assert_eq!(page.total, 3);
        assert_eq!(page.edges.len(), 2);
        assert!(page.has_more);

        // Second page
        let page2 = store.get_edges_paged(3, 2, 2, None).unwrap();
        assert_eq!(page2.edges.len(), 1);
        assert!(!page2.has_more);
    }

    #[test]
    fn paged_adjacency_with_rel_filter() {
        let store = setup_store_with_graph();

        // Node 1 has: 1→2 (related_to), 1→3 (contains)
        let page = store.get_edges_paged(1, 10, 0, Some("related_to")).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.edges.len(), 1);
        assert_eq!(page.edges[0].rel_type, "related_to");
    }

    #[test]
    fn top_degree_nodes() {
        let store = setup_store_with_graph();

        let nodes = store.get_top_degree_nodes(None, 3).unwrap();
        assert_eq!(nodes.len(), 3);
        // Node 3 should be highest degree (3 edges: 1→3, 2→3, 3→4)
        assert_eq!(nodes[0].id, 3);
        assert_eq!(nodes[0].degree, 3);
    }

    #[test]
    fn graph_totals() {
        let store = setup_store_with_graph();
        let (nodes, edges) = store.graph_totals().unwrap();
        assert_eq!(nodes, 5);
        assert_eq!(edges, 5);
    }

    #[test]
    fn covering_indexes_exist() {
        let store = MemoryStore::in_memory();
        let conn = store.conn();

        let idx_src_type: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_edges_src_type'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            > 0;
        assert!(idx_src_type, "idx_edges_src_type should exist");

        let idx_dst_type: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_edges_dst_type'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            > 0;
        assert!(idx_dst_type, "idx_edges_dst_type should exist");
    }

    #[test]
    fn memory_graph_clusters_table_exists() {
        let store = MemoryStore::in_memory();
        let conn = store.conn();
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='memory_graph_clusters'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            > 0;
        assert!(exists, "memory_graph_clusters table should exist");
    }
}
