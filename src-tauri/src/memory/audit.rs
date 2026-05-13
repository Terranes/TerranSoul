//! Memory audit provenance view (Chunk 33B.4).
//!
//! Joins a memory entry with its version history and incident KG edges,
//! including compact summaries for neighboring memories.

use rusqlite::Result as SqlResult;
use serde::{Deserialize, Serialize};

use super::edges::{EdgeDirection, MemoryEdge};
use super::store::{MemoryEntry, MemoryStore};
use super::versioning::{get_history, MemoryVersion};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEdgeDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditNeighbor {
    pub id: i64,
    pub content: String,
    pub tags: String,
    pub importance: i64,
    pub memory_type: String,
    pub tier: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEdge {
    pub edge: MemoryEdge,
    pub direction: AuditEdgeDirection,
    pub neighbor: Option<AuditNeighbor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProvenance {
    pub entry: MemoryEntry,
    pub versions: Vec<MemoryVersion>,
    pub edges: Vec<AuditEdge>,
    pub version_count: usize,
    pub edge_count: usize,
}

pub fn get_memory_provenance(store: &MemoryStore, memory_id: i64) -> SqlResult<MemoryProvenance> {
    let entry = store.get_by_id(memory_id)?;
    let versions = get_history(store.conn(), memory_id)?;
    let raw_edges = store.get_edges_for(memory_id, EdgeDirection::Both)?;
    let mut edges = Vec::with_capacity(raw_edges.len());

    for edge in raw_edges {
        let (direction, neighbor_id) = if edge.src_id == memory_id {
            (AuditEdgeDirection::Outgoing, edge.dst_id)
        } else {
            (AuditEdgeDirection::Incoming, edge.src_id)
        };
        let neighbor = store.get_by_id(neighbor_id).ok().map(neighbor_from_entry);
        edges.push(AuditEdge {
            edge,
            direction,
            neighbor,
        });
    }

    let version_count = versions.len();
    let edge_count = edges.len();

    Ok(MemoryProvenance {
        entry,
        versions,
        edges,
        version_count,
        edge_count,
    })
}

fn neighbor_from_entry(entry: MemoryEntry) -> AuditNeighbor {
    AuditNeighbor {
        id: entry.id,
        content: entry.content,
        tags: entry.tags,
        importance: entry.importance,
        memory_type: entry.memory_type.as_str().to_string(),
        tier: entry.tier.as_str().to_string(),
        created_at: entry.created_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{EdgeSource, MemoryType, NewMemory, NewMemoryEdge};

    fn new_memory(content: &str, tags: &str) -> NewMemory {
        NewMemory {
            content: content.to_string(),
            tags: tags.to_string(),
            importance: 3,
            memory_type: MemoryType::Fact,
            source_url: None,
            source_hash: None,
            expires_at: None,
            created_at: None,
        }
    }

    #[test]
    fn provenance_returns_entry_versions_and_joined_edges() {
        let store = MemoryStore::in_memory();
        let first = store
            .add(new_memory("first memory", "project:one"))
            .unwrap();
        let second = store
            .add(new_memory("second memory", "project:two"))
            .unwrap();

        store
            .update(
                first.id,
                crate::memory::MemoryUpdate {
                    content: Some("first memory edited".to_string()),
                    tags: None,
                    importance: None,
                    memory_type: None,
                },
            )
            .unwrap();

        store
            .add_edge(NewMemoryEdge {
                src_id: first.id,
                dst_id: second.id,
                rel_type: "supports".to_string(),
                confidence: 0.9,
                source: EdgeSource::Llm,
                valid_from: None,
                valid_to: None,
                edge_source: Some("test".to_string()),
            })
            .unwrap();

        let provenance = get_memory_provenance(&store, first.id).unwrap();
        assert_eq!(provenance.entry.content, "first memory edited");
        assert_eq!(provenance.version_count, 1);
        assert_eq!(provenance.versions[0].content, "first memory");
        assert_eq!(provenance.edge_count, 1);
        assert!(matches!(
            provenance.edges[0].direction,
            AuditEdgeDirection::Outgoing
        ));
        let neighbor = provenance.edges[0].neighbor.as_ref().unwrap();
        assert_eq!(neighbor.id, second.id);
        assert_eq!(neighbor.content, "second memory");
    }

    #[test]
    fn provenance_marks_incoming_edges() {
        let store = MemoryStore::in_memory();
        let source = store.add(new_memory("source", "a")).unwrap();
        let target = store.add(new_memory("target", "b")).unwrap();
        store
            .add_edge(NewMemoryEdge {
                src_id: source.id,
                dst_id: target.id,
                rel_type: "derived_from".to_string(),
                confidence: 1.0,
                source: EdgeSource::User,
                valid_from: None,
                valid_to: None,
                edge_source: None,
            })
            .unwrap();

        let provenance = get_memory_provenance(&store, target.id).unwrap();
        assert_eq!(provenance.edge_count, 1);
        assert!(matches!(
            provenance.edges[0].direction,
            AuditEdgeDirection::Incoming
        ));
        assert_eq!(provenance.edges[0].neighbor.as_ref().unwrap().id, source.id);
    }

    #[test]
    fn provenance_missing_memory_returns_error() {
        let store = MemoryStore::in_memory();
        let err = get_memory_provenance(&store, 999).unwrap_err();
        assert_eq!(err, rusqlite::Error::QueryReturnedNoRows);
    }
}
