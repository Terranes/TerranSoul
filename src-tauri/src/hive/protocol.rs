//! Wire types for the Hive protocol (version 1).
//!
//! See `docs/hive-protocol.md` for the full specification.

use serde::{Deserialize, Serialize};

/// Protocol version.
pub const HIVE_PROTOCOL_VERSION: u8 = 1;

/// Maximum bundle payload size (64 MiB).
pub const MAX_BUNDLE_SIZE: usize = 64 * 1024 * 1024;

/// Maximum memories per bundle.
pub const MAX_MEMORIES_PER_BUNDLE: usize = 10_000;

/// Maximum edges per bundle.
pub const MAX_EDGES_PER_BUNDLE: usize = 50_000;

/// Default OP queue depth per device on the relay.
pub const DEFAULT_OP_QUEUE_DEPTH: usize = 1000;

// ─── Envelope ───────────────────────────────────────────────────────────────

/// Top-level message type discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MsgType {
    Bundle = 0,
    Op = 1,
    Job = 2,
}

/// Share scope for individual memories.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareScope {
    /// Never leaves the device.
    #[default]
    Private,
    /// Syncs only between paired own-devices.
    Paired,
    /// Can be uploaded to a hive relay.
    Hive,
}

/// The outer envelope wrapping every Hive message.
///
/// Fields are serialised with MessagePack for the wire format.
/// The `signature` covers `(version ∥ msg_type ∥ sender_id ∥ timestamp ∥ hlc_counter ∥ payload)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveEnvelope {
    pub version: u8,
    pub msg_type: MsgType,
    pub sender_id: String,
    /// Ed25519 verifying (public) key — 32 bytes.
    pub sender_pubkey: Vec<u8>,
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
    /// Hybrid Logical Clock counter for causal ordering.
    pub hlc_counter: u64,
    /// Serialised message body (MessagePack).
    pub payload: Vec<u8>,
    /// Ed25519 signature — 64 bytes.
    pub signature: Vec<u8>,
    /// Whether `payload` is LZ4-compressed.
    pub compressed: bool,
}

// ─── BUNDLE ─────────────────────────────────────────────────────────────────

/// A signed batch of memory deltas and edge deltas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub bundle_id: String,
    /// HLC range covered: receiver can request "since" hlc_from.
    pub hlc_from: u64,
    pub hlc_to: u64,
    pub memory_deltas: Vec<MemoryDelta>,
    pub edge_deltas: Vec<EdgeDelta>,
}

/// A single memory CRDT delta within a bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDelta {
    /// SHA-256 of content, used as cross-device stable identifier.
    pub content_hash: String,
    /// "upsert" or "soft_close".
    pub operation: String,
    pub content: String,
    pub tags: String,
    pub importance: u8,
    pub memory_type: String,
    pub cognitive_kind: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub hlc_counter: u64,
    pub origin_device: String,
    pub share_scope: ShareScope,
    pub source_url: Option<String>,
    pub source_hash: Option<String>,
    /// Contextual retrieval prefix (Anthropic 2024).
    pub context_prefix: Option<String>,
    /// Non-None means soft-closed at this timestamp.
    pub valid_to: Option<u64>,
}

/// A single edge CRDT delta within a bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDelta {
    /// Content hash of the source memory.
    pub src_content_hash: String,
    /// Content hash of the destination memory.
    pub dst_content_hash: String,
    pub rel_type: String,
    pub confidence: f64,
    /// "user" | "llm" | "auto"
    pub source: String,
    pub created_at: u64,
    pub valid_from: Option<u64>,
    /// Some = tombstoned at this timestamp.
    pub valid_to: Option<u64>,
    pub edge_source: Option<String>,
    pub origin_device: String,
    pub hlc_counter: u64,
}

// ─── OP ─────────────────────────────────────────────────────────────────────

/// A single CRDT operation (low-latency, ephemeral).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Op {
    pub target: OpTarget,
    pub delta: OpDelta,
}

/// Discriminant for an OP target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpTarget {
    Memory,
    Edge,
}

/// Payload of a single OP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpDelta {
    Memory(MemoryDelta),
    Edge(EdgeDelta),
}

// ─── JOB ────────────────────────────────────────────────────────────────────

/// A distributable work item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub job_id: String,
    /// e.g. "embed_chunks", "summarize", "extract_edges"
    pub job_type: String,
    /// Required worker capabilities.
    pub capabilities: Vec<Capability>,
    /// Opaque input payload (serialised by the originator).
    pub input: Vec<u8>,
    /// Timeout in milliseconds before the job returns to the queue.
    pub timeout_ms: u64,
    /// Maximum retry attempts.
    pub max_retries: u8,
}

/// Current status of a job on the relay.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum JobStatus {
    Pending,
    Claimed {
        worker_id: String,
        claimed_at: u64,
    },
    Completed {
        worker_id: String,
        completed_at: u64,
    },
    Failed {
        reason: String,
    },
}

/// A capability declaration (kind + value pair).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// Category: "brain_mode", "gpu", "embedding_model", etc.
    pub kind: String,
    /// Specific value: "ollama", "cuda", "nomic-embed-text", etc.
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_scope_default_is_private() {
        assert_eq!(ShareScope::default(), ShareScope::Private);
    }

    #[test]
    fn msg_type_repr() {
        assert_eq!(MsgType::Bundle as u8, 0);
        assert_eq!(MsgType::Op as u8, 1);
        assert_eq!(MsgType::Job as u8, 2);
    }

    #[test]
    fn bundle_serde_roundtrip() {
        let bundle = Bundle {
            bundle_id: "test-bundle-001".into(),
            hlc_from: 100,
            hlc_to: 200,
            memory_deltas: vec![MemoryDelta {
                content_hash: "abc123".into(),
                operation: "upsert".into(),
                content: "Hello world".into(),
                tags: "test".into(),
                importance: 3,
                memory_type: "fact".into(),
                cognitive_kind: Some("episodic".into()),
                created_at: 1000,
                updated_at: 1000,
                hlc_counter: 150,
                origin_device: "device-a".into(),
                share_scope: ShareScope::Hive,
                source_url: None,
                source_hash: None,
                context_prefix: None,
                valid_to: None,
            }],
            edge_deltas: vec![],
        };

        let json = serde_json::to_string(&bundle).unwrap();
        let decoded: Bundle = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.bundle_id, "test-bundle-001");
        assert_eq!(decoded.memory_deltas.len(), 1);
        assert_eq!(decoded.memory_deltas[0].share_scope, ShareScope::Hive);
    }

    #[test]
    fn job_spec_serde_roundtrip() {
        let job = JobSpec {
            job_id: "job-001".into(),
            job_type: "embed_chunks".into(),
            capabilities: vec![Capability {
                kind: "embedding_model".into(),
                value: "nomic-embed-text".into(),
            }],
            input: vec![1, 2, 3, 4],
            timeout_ms: 30_000,
            max_retries: 3,
        };

        let json = serde_json::to_string(&job).unwrap();
        let decoded: JobSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.job_id, "job-001");
        assert_eq!(decoded.capabilities.len(), 1);
        assert_eq!(decoded.capabilities[0].value, "nomic-embed-text");
    }

    #[test]
    fn envelope_structure() {
        let envelope = HiveEnvelope {
            version: HIVE_PROTOCOL_VERSION,
            msg_type: MsgType::Bundle,
            sender_id: "device-a".into(),
            sender_pubkey: vec![0u8; 32],
            timestamp: 1_700_000_000_000,
            hlc_counter: 42,
            payload: vec![],
            signature: vec![0u8; 64],
            compressed: false,
        };

        assert_eq!(envelope.version, 1);
        assert_eq!(envelope.msg_type, MsgType::Bundle);
        assert_eq!(envelope.sender_pubkey.len(), 32);
        assert_eq!(envelope.signature.len(), 64);
    }
}
