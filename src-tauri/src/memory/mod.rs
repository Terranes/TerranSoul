pub mod backend;
pub mod brain_memory;
pub mod cognitive_kind;
pub mod edges;
pub mod migrations;
pub mod store;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "mssql")]
pub mod mssql;
#[cfg(feature = "cassandra")]
pub mod cassandra;

pub use backend::{StorageBackend, StorageConfig, StorageError, StorageResult};
pub use cognitive_kind::{classify as classify_cognitive_kind, CognitiveKind};
pub use edges::{
    EdgeDirection, EdgeSource, EdgeStats, MemoryEdge, NewMemoryEdge,
    COMMON_RELATION_TYPES, normalise_rel_type, parse_llm_edges,
    format_memories_for_extraction,
};
pub use store::{
    MemoryEntry, MemoryStats, MemoryStore, MemoryTier, MemoryType, MemoryUpdate, NewMemory,
    bytes_to_embedding, cosine_similarity, embedding_to_bytes,
};

#[cfg(feature = "postgres")]
pub use postgres::PostgresBackend;
#[cfg(feature = "mssql")]
pub use mssql::MssqlBackend;
#[cfg(feature = "cassandra")]
pub use cassandra::CassandraBackend;
