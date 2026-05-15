pub mod ann_flush;
pub mod ann_index;
pub mod audit;
pub mod auto_learn;
pub mod auto_tag;
pub mod backend;
pub mod brain_maintenance;
pub mod brain_memory;
pub mod cap;
pub mod cascade;
pub mod chunking;
pub mod cognitive_kind;
pub mod confidence_decay;
pub mod conflicts;
pub mod consolidation;
pub mod context_pack;
pub mod contextualize;
pub mod crag;
pub mod crdt_sync;
pub mod disk_backed_ann;
pub mod edge_conflict_scan;
pub mod edge_crdt_sync;
pub mod edges;
pub mod embedding_queue;
pub mod eviction;
pub mod fusion;
pub mod gap_detection;
pub mod graph_page;
pub mod graph_paging;
pub mod graph_rag;
pub mod hyde;
pub mod instruction_slices;
pub mod ivf_pq;
pub mod judgment;
pub mod kg_cache;
pub mod late_chunking;
pub mod matryoshka;
pub mod metrics;
#[cfg(not(feature = "native-ann"))]
pub mod mobile_ann;
pub mod negative;
pub mod obsidian_export;
pub mod obsidian_sync;
pub mod offline_embed;
pub mod platform;
pub mod post_retrieval;
pub mod privacy;
pub mod query_intent;
pub mod refine;
pub mod reflection;
pub mod replay;
pub mod reranker;
pub mod schema;
pub mod search_cache;
pub mod seed_migrations;
pub mod shard_backpressure;
pub mod shard_router;
pub mod sharded_retrieval;
#[cfg(feature = "time-shards")]
pub mod shards;
pub mod snapshot;
pub mod sources;
#[cfg(feature = "repo-rag")]
pub mod repo_ingest;
#[cfg(feature = "repo-rag")]
pub mod repo_oauth;
pub mod store;
pub mod tag_vocabulary;
pub mod temporal;
pub mod versioning;
pub mod wiki;

#[cfg(feature = "cassandra")]
pub mod cassandra;
#[cfg(feature = "mssql")]
pub mod mssql;
#[cfg(feature = "postgres")]
pub mod postgres;

pub use auto_learn::{evaluate as evaluate_auto_learn, AutoLearnDecision, AutoLearnPolicy};
pub use backend::{StorageBackend, StorageConfig, StorageError, StorageResult};
pub use cognitive_kind::{classify as classify_cognitive_kind, CognitiveKind};
pub use context_pack::format_retrieved_context_pack;
pub use edges::{
    format_memories_for_extraction, normalise_rel_type, parse_llm_edges, EdgeDirection, EdgeSource,
    EdgeStats, MemoryEdge, NewMemoryEdge, COMMON_RELATION_TYPES,
};
pub use store::{
    bytes_to_embedding, cosine_similarity, embedding_to_bytes, MemoryCleanupReport, MemoryEntry,
    MemoryStats, MemoryStore, MemoryTier, MemoryType, MemoryUpdate, NewMemory,
};

#[cfg(feature = "cassandra")]
pub use cassandra::CassandraBackend;
#[cfg(feature = "mssql")]
pub use mssql::MssqlBackend;
#[cfg(feature = "postgres")]
pub use postgres::PostgresBackend;
