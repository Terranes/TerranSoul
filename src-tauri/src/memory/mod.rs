pub mod backend;
pub mod brain_memory;
pub mod migrations;
pub mod store;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "mssql")]
pub mod mssql;
#[cfg(feature = "cassandra")]
pub mod cassandra;

pub use backend::{StorageBackend, StorageConfig, StorageError, StorageResult};
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
