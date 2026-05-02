pub mod ann_index;
pub mod auto_learn;
pub mod auto_tag;
pub mod backend;
pub mod brain_maintenance;
pub mod brain_memory;
pub mod chunking;
pub mod code_rag;
pub mod cognitive_kind;
pub mod conflicts;
pub mod consolidation;
pub mod contextualize;
pub mod crag;
pub mod edge_conflict_scan;
pub mod edges;
pub mod fusion;
pub mod gitnexus_mirror;
pub mod graph_rag;
pub mod hyde;
pub mod late_chunking;
pub mod matryoshka;
pub mod migrations;
pub mod obsidian_export;
pub mod obsidian_sync;
pub mod query_intent;
pub mod replay;
pub mod reranker;
pub mod store;
pub mod tag_vocabulary;
pub mod temporal;
pub mod versioning;

#[cfg(feature = "cassandra")]
pub mod cassandra;
#[cfg(feature = "mssql")]
pub mod mssql;
#[cfg(feature = "postgres")]
pub mod postgres;

pub use auto_learn::{evaluate as evaluate_auto_learn, AutoLearnDecision, AutoLearnPolicy};
pub use backend::{StorageBackend, StorageConfig, StorageError, StorageResult};
pub use cognitive_kind::{classify as classify_cognitive_kind, CognitiveKind};
pub use edges::{
    format_memories_for_extraction, normalise_rel_type, parse_llm_edges, EdgeDirection, EdgeSource,
    EdgeStats, MemoryEdge, NewMemoryEdge, COMMON_RELATION_TYPES,
};
pub use store::{
    bytes_to_embedding, cosine_similarity, embedding_to_bytes, MemoryEntry, MemoryStats,
    MemoryStore, MemoryTier, MemoryType, MemoryUpdate, NewMemory,
};

#[cfg(feature = "cassandra")]
pub use cassandra::CassandraBackend;
#[cfg(feature = "mssql")]
pub use mssql::MssqlBackend;
#[cfg(feature = "postgres")]
pub use postgres::PostgresBackend;
