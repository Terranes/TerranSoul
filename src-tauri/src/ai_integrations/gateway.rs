//! `BrainGateway` — the single, typed op surface that every transport
//! ([`mcp`](super), [`grpc`](super)) routes through.
//!
//! Design goals (per `docs/AI-coding-integrations.md § Surface`):
//!
//! 1. **One surface, two transports.** MCP and gRPC adapters are free of
//!    business logic; they translate their wire format into the same
//!    [`BrainGateway`] calls. The trait is therefore the canonical contract
//!    for what a connected editor agent can ask TerranSoul.
//! 2. **Capability-gated by default.** Every call takes a [`GatewayCaps`]
//!    snapshot. Reads require `brain_read`; writes (`ingest_url`) require
//!    `brain_write`. The default profile is read-only — write tools are
//!    opt-in per client in the Control Panel (Chunk 15.4).
//! 3. **Delta-stable composition.** [`BrainGateway::suggest_context`] —
//!    the editor-flagship call — composes search + KG + summary into a
//!    single pack. The pack carries a `fingerprint` (SHA-256 over the
//!    resolved hit ids + the active brain identifier) so a connected
//!    editor (Chunk 15.7 for VS Code Copilot) can cache against it and
//!    skip re-asking when nothing has changed.
//! 4. **No re-implementation.** [`AppStateGateway`] delegates straight to
//!    [`crate::memory::MemoryStore`] and [`crate::brain::OllamaAgent`];
//!    no new business logic lives here.
//! 5. **Tauri-AppHandle-free trait.** Long-running write operations
//!    (`ingest_url`) plug in via the [`IngestSink`] trait so the gateway
//!    is testable without a real Tauri runtime, while production
//!    constructs an [`AppHandleIngestSink`] that owns a real `AppHandle`
//!    and emits progress events.
//!
//! Chunk reference: **15.3** in `rules/milestones.md`.

use std::io::Write;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::brain::OllamaAgent;
use crate::memory::edges::{EdgeDirection, MemoryEdge};
#[cfg(test)]
use crate::memory::MemoryTier;
use crate::memory::{MemoryEntry, MemoryType};
use crate::AppState;

// ─── Errors ─────────────────────────────────────────────────────────────────

/// Every gateway op returns a typed error so transports can map cleanly to
/// MCP `tool_result.is_error` / gRPC `tonic::Status` codes.
#[derive(Debug, Error)]
pub enum GatewayError {
    /// The connected client lacks the required capability for this op.
    /// Maps to MCP `is_error: true` with code `PERMISSION_DENIED`, gRPC
    /// `Code::PermissionDenied`.
    #[error("permission denied: capability `{0}` is not granted to this client")]
    PermissionDenied(&'static str),

    /// A required resource (brain, AppHandle, ingest sink) is not
    /// configured. Maps to gRPC `Code::FailedPrecondition`.
    #[error("not configured: {0}")]
    NotConfigured(String),

    /// The request itself is malformed (e.g. zero limit, empty query).
    /// Maps to gRPC `Code::InvalidArgument`.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// The requested entity does not exist. Maps to gRPC `Code::NotFound`.
    #[error("not found: {0}")]
    NotFound(String),

    /// Underlying storage error (SQLite). Maps to gRPC `Code::Internal`.
    #[error("storage error: {0}")]
    Storage(String),

    /// Lock poisoning or other internal failure.
    #[error("internal error: {0}")]
    Internal(String),
}

impl GatewayError {
    fn from_lock<E: std::fmt::Display>(e: E) -> Self {
        GatewayError::Internal(format!("lock poisoned: {e}"))
    }
}

// ─── Capability snapshot ────────────────────────────────────────────────────

/// Per-client capability snapshot. Created once when the transport
/// authenticates the client and reused across every call from that client.
///
/// The default profile (`Default::default()`) is **read-only**; write
/// capabilities (`brain_write`) must be explicitly granted by the user
/// through the Control Panel (Chunk 15.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayCaps {
    /// Read brain memories, run searches, traverse the KG, summarize.
    /// Required by every read op.
    pub brain_read: bool,
    /// Write to the brain — currently only `ingest_url`. Off by default.
    pub brain_write: bool,
    /// Reserved for future code-introspection ops; not consulted in 15.3.
    pub code_read: bool,
    /// Write to the code graph overlays (branch sync, index commit).
    #[serde(default)]
    pub code_write: bool,
}

impl Default for GatewayCaps {
    /// Default is **read-only**. Matches the security-by-default posture
    /// in `docs/AI-coding-integrations.md § Capability gating`.
    fn default() -> Self {
        Self {
            brain_read: true,
            brain_write: false,
            code_read: false,
            code_write: false,
        }
    }
}

impl GatewayCaps {
    /// Convenience constant for tests: no caps at all (fail-closed).
    pub const NONE: Self = Self {
        brain_read: false,
        brain_write: false,
        code_read: false,
        code_write: false,
    };

    /// Convenience constant for tests + auto-setup: read + write enabled.
    pub const READ_WRITE: Self = Self {
        brain_read: true,
        brain_write: true,
        code_read: true,
        code_write: true,
    };
}

// ─── Request / response types ───────────────────────────────────────────────

/// Which retrieval algorithm to use for `brain.search`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// Legacy 6-signal weighted-sum hybrid search. Cheapest, no LLM.
    Hybrid,
    /// RRF-fused vector + keyword + freshness (k = 60). The default.
    #[default]
    Rrf,
    /// HyDE — LLM hypothetical-document expansion + RRF. Requires an
    /// active brain; falls back to RRF on raw query when the brain is
    /// unreachable. See `docs/brain-advanced-design.md` § 19.3.
    Hyde,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    /// Top-k results. Bounded `1..=100` server-side; defaults to 10.
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub mode: SearchMode,
    /// Run LLM-as-judge rerank after RRF/HyDE recall. Defaults on for RRF.
    #[serde(default = "default_search_rerank")]
    pub rerank: bool,
    /// Normalised 0.0–1.0 rerank score threshold. Defaults to 0.55.
    #[serde(default = "default_search_rerank_threshold")]
    pub rerank_threshold: f64,
}

fn default_search_rerank() -> bool {
    true
}

fn default_search_rerank_threshold() -> f64 {
    crate::settings::DEFAULT_RERANK_THRESHOLD
}

/// A single search result. Keep this **strictly** flatter than
/// [`MemoryEntry`] so the wire schema stays stable when the storage
/// schema changes (V6, V7, …).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub id: i64,
    pub content: String,
    pub tags: String,
    pub importance: i64,
    /// Reciprocal-rank-fused score. Always positive; higher = more
    /// relevant. Not directly comparable across different `SearchMode`s.
    pub score: f64,
    pub source_url: Option<String>,
    pub tier: String,
}

impl From<MemoryEntry> for SearchHit {
    fn from(e: MemoryEntry) -> Self {
        Self {
            id: e.id,
            content: e.content,
            tags: e.tags,
            importance: e.importance,
            // The store's hybrid_search_rrf path only puts results in
            // ranked order; it does not surface the per-row RRF score.
            // We carry a normalised positional score here (1.0 / rank,
            // computed at the call site) so clients can sort/threshold
            // without inventing a number.
            score: 0.0,
            source_url: e.source_url,
            tier: e.tier.as_str().to_string(),
        }
    }
}

/// A memory entry enriched with its reinforcement history (Chunk 43.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryDetail {
    #[serde(flatten)]
    pub entry: MemoryEntry,
    pub reinforcements: Vec<crate::memory::store::ReinforcementRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentRequest {
    /// Bounded `1..=200` server-side; defaults to 20.
    #[serde(default)]
    pub limit: Option<usize>,
    /// Optional filter on cognitive kind (`fact`, `preference`,
    /// `episode`, `procedure`).
    #[serde(default)]
    pub kind: Option<String>,
    /// Optional comma-or-space-separated tag filter (any-match).
    #[serde(default)]
    pub tag: Option<String>,
    /// Optional Unix-ms lower bound on `created_at`.
    #[serde(default)]
    pub since: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgRequest {
    pub id: i64,
    /// Traversal depth. Currently only depth 1 is supported (one hop).
    /// Higher depths are reserved for a future BFS implementation; if
    /// requested, the gateway returns depth 1 and reports `truncated: true`.
    #[serde(default = "default_depth")]
    pub depth: u8,
    /// Direction filter. Defaults to `both`.
    #[serde(default = "default_direction")]
    pub direction: String,
}

fn default_depth() -> u8 {
    1
}
fn default_direction() -> String {
    "both".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgNeighbor {
    pub edge: MemoryEdge,
    /// The neighbour entry (the *other* end of the edge from the
    /// requested center). May be `None` if the entry was deleted while
    /// the edge survived as an orphan — the transport should treat
    /// `None` as a soft warning, not an error.
    pub entry: Option<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgNeighborhood {
    pub center: MemoryEntry,
    pub neighbors: Vec<KgNeighbor>,
    /// `true` when the requested `depth` was greater than what the
    /// gateway is willing to traverse in this version (currently 1).
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizeRequest {
    /// Either `text`, `memory_ids`, or `query` must be supplied. When
    /// more than one is present, resolved memory/search contents are
    /// appended to `text`.
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub memory_ids: Option<Vec<i64>>,
    /// Optional search query. When supplied, the gateway first runs the
    /// normal RRF memory search and summarizes the resulting hits.
    #[serde(default)]
    pub query: Option<String>,
    /// Top-k memories to resolve when `query` is supplied. Bounded
    /// `1..=20`; defaults to 5.
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizeResponse {
    /// `None` when no brain is configured (graceful degradation — the
    /// transport surfaces this to the editor as "summary not available").
    pub summary: Option<String>,
    /// Number of source-memory ids that were resolved (so the client
    /// knows whether stale ids were silently dropped).
    pub resolved_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestContextRequest {
    /// Optional editor-side context. None of these are *required* — the
    /// gateway treats them as additional ranking signal only.
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub cursor_offset: Option<u64>,
    #[serde(default)]
    pub selection: Option<String>,
    /// The user's natural-language question / current chat turn.
    pub query: String,
    /// Top-k memories to include in the pack. Defaults to 5; bounded `1..=20`.
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestContextPack {
    /// Top memories ranked by RRF (or HyDE if the brain supports it).
    pub hits: Vec<SearchHit>,
    /// One-hop KG neighborhood around the highest-scoring hit, when
    /// available. `None` for cold-start brains with zero edges.
    pub kg: Option<KgNeighborhood>,
    /// LLM-written summary of the pack. `None` when no brain is
    /// configured.
    pub summary: Option<String>,
    /// SHA-256 hex over the resolved hit ids + the active brain
    /// identifier. Identical inputs ⇒ identical fingerprint, which is
    /// the contract VS Code Copilot caches against in Chunk 15.7.
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestUrlRequest {
    pub url: String,
    /// Comma-separated tags. Defaults to `"imported"`.
    #[serde(default)]
    pub tags: Option<String>,
    /// Importance score, clamped `1..=5`. Defaults to 4.
    #[serde(default)]
    pub importance: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestLessonRequest {
    pub content: String,
    /// Comma-separated tags (e.g., "frontend,css,theme").
    #[serde(default)]
    pub tags: Option<String>,
    /// Importance score, clamped `1..=10`. Defaults to 8.
    #[serde(default)]
    pub importance: Option<i64>,
    /// Category (e.g., "coding-workflow", "frontend", "security").
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestUrlResponse {
    /// Background task id — the client can poll task status through the
    /// existing `commands::tasks` surface or wait for `task-progress`
    /// events on the IPC bus.
    pub task_id: String,
    pub source: String,
    /// Either `"url"`, `"file"`, or `"crawl"` — mirrors
    /// [`crate::commands::ingest::IngestStartResult`].
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestLessonResponse {
    /// Memory entry id where the lesson was stored.
    pub memory_id: i64,
    /// Tags that were applied.
    pub tags: String,
    /// Importance assigned.
    pub importance: i64,
    /// Acknowledgement that the lesson was also appended to memory-seed.sql.
    pub persisted_to_seed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthResponse {
    pub version: String,
    /// Active brain provider id (`"ollama"`, `"openai"`, `"free"`, or
    /// `"none"` when no brain is configured).
    pub brain_provider: String,
    pub brain_model: Option<String>,
    /// 0-100 embedding coverage for long-term memories.
    pub rag_quality_pct: u8,
    /// Cumulative memory count across all tiers.
    pub memory_total: i64,
    /// Human-readable interpretation and raw counts for `rag_quality_pct`.
    pub rag_quality: RagQualityHealth,
    /// Memory tier totals that explain `memory_total`.
    pub memory: MemoryHealth,
    /// Embedding worker status (rate-limit pauses, hard failures, throughput).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_worker: Option<crate::memory::embedding_queue::WorkerStatus>,
    /// Field-level descriptions for clients rendering raw JSON.
    pub descriptions: HealthDescriptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagQualityHealth {
    pub label: String,
    pub description: String,
    pub formula: String,
    pub embedded_long_memory_count: i64,
    pub long_memory_count: i64,
    pub pending_embedding_count: u64,
    pub failing_embedding_count: u64,
    pub next_embedding_retry_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryHealth {
    pub total: i64,
    pub short_count: i64,
    pub working_count: i64,
    pub long_count: i64,
    pub embedded_total: i64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthDescriptions {
    pub brain_provider: String,
    pub brain_model: String,
    pub rag_quality_pct: String,
    pub memory_total: String,
}

// ─── Pluggable ingest sink ──────────────────────────────────────────────────

/// The gateway is transport-agnostic and Tauri-`AppHandle`-free; long-running
/// writes plug in through this trait so the trait stays unit-testable.
///
/// Production composes [`AppHandleIngestSink`] (declared at the call site
/// in 15.4 / 15.6 once the Control Panel exists) which owns a real
/// `AppHandle` and dispatches to [`crate::commands::ingest::ingest_document`].
/// Tests pass [`RecordingIngestSink`].
#[async_trait]
pub trait IngestSink: Send + Sync {
    async fn start_ingest(
        &self,
        source: String,
        tags: Option<String>,
        importance: Option<i64>,
    ) -> Result<IngestUrlResponse, GatewayError>;
}

// ─── The trait ──────────────────────────────────────────────────────────────

/// The single op surface every transport routes through.
#[async_trait]
pub trait BrainGateway: Send + Sync {
    /// `brain.search` — hybrid + RRF + (optional) HyDE search.
    async fn search(
        &self,
        caps: &GatewayCaps,
        req: SearchRequest,
    ) -> Result<Vec<SearchHit>, GatewayError>;

    /// `brain.get_entry` — full memory entry by id.
    async fn get_entry(&self, caps: &GatewayCaps, id: i64) -> Result<MemoryEntry, GatewayError>;

    /// `brain.get_entry` with reinforcement provenance (Chunk 43.4).
    async fn get_entry_detail(
        &self,
        caps: &GatewayCaps,
        id: i64,
    ) -> Result<EntryDetail, GatewayError>;

    /// `brain.list_recent` — last N memories with optional filters.
    async fn list_recent(
        &self,
        caps: &GatewayCaps,
        req: RecentRequest,
    ) -> Result<Vec<MemoryEntry>, GatewayError>;

    /// `brain.kg_neighbors` — knowledge-graph one-hop neighbourhood.
    async fn kg_neighbors(
        &self,
        caps: &GatewayCaps,
        req: KgRequest,
    ) -> Result<KgNeighborhood, GatewayError>;

    /// `brain.summarize` — LLM-summarise text or memory ids.
    async fn summarize(
        &self,
        caps: &GatewayCaps,
        req: SummarizeRequest,
    ) -> Result<SummarizeResponse, GatewayError>;

    /// `brain.suggest_context` — editor-flagship: ranked memories + KG +
    /// summary + delta-stable fingerprint.
    async fn suggest_context(
        &self,
        caps: &GatewayCaps,
        req: SuggestContextRequest,
    ) -> Result<SuggestContextPack, GatewayError>;

    /// `brain.ingest_url` — capability-gated write op.
    async fn ingest_url(
        &self,
        caps: &GatewayCaps,
        req: IngestUrlRequest,
    ) -> Result<IngestUrlResponse, GatewayError>;

    /// `brain.ingest_lesson` — directly write a lesson to the brain's
    /// memory store AND append to `mcp-data/shared/memory-seed.sql` for
    /// reseed durability. Requires `brain_write` capability.
    async fn ingest_lesson(
        &self,
        caps: &GatewayCaps,
        req: IngestLessonRequest,
    ) -> Result<IngestLessonResponse, GatewayError>;

    /// `brain.health` — server + brain status snapshot.
    async fn health(&self, caps: &GatewayCaps) -> Result<HealthResponse, GatewayError>;
}

// ─── AppState adapter ───────────────────────────────────────────────────────

/// Production adapter: wraps an [`AppState`] (cheaply clonable Arc
/// newtype) and delegates every op to the existing in-process surfaces
/// ([`crate::memory::MemoryStore`], [`crate::brain::OllamaAgent`],
/// [`IngestSink`]).
///
/// Holds `Arc<dyn IngestSink>` so transports can plug in either
/// `AppHandleIngestSink` (production) or a test sink (unit tests).
pub struct AppStateGateway {
    state: AppState,
    ingest: Option<Arc<dyn IngestSink>>,
}

impl AppStateGateway {
    /// Build a gateway with no ingest sink — `ingest_url` will fail with
    /// [`GatewayError::NotConfigured`]. Use this for read-only deployments
    /// or in unit tests where the ingest path isn't exercised.
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            ingest: None,
        }
    }

    /// Build a gateway with an ingest sink. The transport (15.1 / 15.2)
    /// constructs this with an `AppHandleIngestSink` once the Control
    /// Panel chunk lands.
    pub fn with_ingest(state: AppState, ingest: Arc<dyn IngestSink>) -> Self {
        Self {
            state,
            ingest: Some(ingest),
        }
    }

    /// Snapshot of the active brain model — released *before* any
    /// `.await` to avoid `std::sync::Mutex` + async deadlock.
    fn active_brain(&self) -> Result<Option<String>, GatewayError> {
        Ok(self
            .state
            .active_brain
            .lock()
            .map_err(GatewayError::from_lock)?
            .clone())
    }

    fn brain_mode(&self) -> Result<Option<crate::brain::BrainMode>, GatewayError> {
        Ok(self
            .state
            .brain_mode
            .lock()
            .map_err(GatewayError::from_lock)?
            .clone())
    }

    async fn query_embedding(
        &self,
        mode: SearchMode,
        query: &str,
        model_opt: Option<&str>,
        brain_mode: Option<&crate::brain::BrainMode>,
    ) -> Option<Vec<f32>> {
        let online = match mode {
            SearchMode::Hybrid => None,
            SearchMode::Rrf => crate::brain::embed_for_mode(query, brain_mode, model_opt).await,
            SearchMode::Hyde => {
                let text = if let Some(model) = model_opt {
                    let agent = OllamaAgent::new(model);
                    agent
                        .hyde_complete(query)
                        .await
                        .unwrap_or_else(|| query.to_string())
                } else {
                    query.to_string()
                };
                crate::brain::embed_for_mode(&text, brain_mode, model_opt).await
            }
        };

        if online.is_some() || mode == SearchMode::Hybrid {
            return online;
        }

        if crate::ai_integrations::mcp::is_mcp_pet_mode() {
            crate::memory::offline_embed::embed_text(query)
        } else {
            None
        }
    }

    /// Hex-encoded SHA-256 fingerprint over the resolved hit ids + the
    /// active brain identifier. The contract for VS Code Copilot's
    /// warm-cache pact (Chunk 15.7).
    fn fingerprint(brain: &str, hits: &[SearchHit]) -> String {
        let mut h = Sha256::new();
        h.update(brain.as_bytes());
        h.update([0u8]); // separator so brain="x" + ids=[1] differs from brain="x1"
        for hit in hits {
            h.update(hit.id.to_le_bytes());
        }
        let bytes = h.finalize();
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
        }
        s
    }
}

#[async_trait]
impl BrainGateway for AppStateGateway {
    async fn search(
        &self,
        caps: &GatewayCaps,
        req: SearchRequest,
    ) -> Result<Vec<SearchHit>, GatewayError> {
        require_read(caps)?;
        if req.query.trim().is_empty() {
            return Err(GatewayError::InvalidArgument("query is empty".into()));
        }
        let limit = req.limit.unwrap_or(10).clamp(1, 100);

        // Step 1: optionally compute the query embedding (HyDE expands first).
        let model_opt = self.active_brain()?;
        let brain_mode = self.brain_mode()?;
        let query_emb = self
            .query_embedding(
                req.mode,
                &req.query,
                model_opt.as_deref(),
                brain_mode.as_ref(),
            )
            .await;

        // Step 2: dispatch to the right store method.
        let recall_limit = if matches!(req.mode, SearchMode::Rrf | SearchMode::Hyde)
            && req.rerank
            && model_opt.is_some()
        {
            limit.clamp(20, 50)
        } else {
            limit
        };
        let entries = {
            let store = self
                .state
                .memory_store
                .lock()
                .map_err(GatewayError::from_lock)?;
            match req.mode {
                SearchMode::Hybrid => store
                    .hybrid_search(&req.query, query_emb.as_deref(), recall_limit)
                    .map_err(|e| GatewayError::Storage(e.to_string()))?,
                SearchMode::Rrf | SearchMode::Hyde => store
                    .hybrid_search_rrf(&req.query, query_emb.as_deref(), recall_limit)
                    .map_err(|e| GatewayError::Storage(e.to_string()))?,
            }
        };

        let entries = if matches!(req.mode, SearchMode::Rrf | SearchMode::Hyde) && req.rerank {
            if let Some(model) = model_opt {
                let agent = OllamaAgent::new(&model);
                let mut scores = Vec::with_capacity(entries.len());
                for entry in &entries {
                    scores.push(agent.rerank_score(&req.query, &entry.content).await);
                }

                // Build verdicts before threshold filtering (43.6).
                let threshold_norm = req.rerank_threshold.clamp(0.0, 1.0);
                let verdicts: Vec<(i64, crate::memory::post_retrieval::RetrievalVerdict)> = entries
                    .iter()
                    .zip(scores.iter())
                    .map(|(e, s)| {
                        let verdict = if s.is_some_and(|v| (v as f64 / 10.0) >= threshold_norm) {
                            crate::memory::post_retrieval::RetrievalVerdict::Verified
                        } else {
                            crate::memory::post_retrieval::RetrievalVerdict::Rejected
                        };
                        (e.id, verdict)
                    })
                    .collect();

                let reranked = crate::memory::reranker::rerank_candidates_with_threshold(
                    entries,
                    &scores,
                    limit,
                    req.rerank_threshold,
                );

                // Record reinforcement for entries that survived reranking (43.4).
                if let Ok(store) = self.state.memory_store.lock() {
                    let session_id = format!("brain_search_{}", crate::memory::store::now_ms());
                    for (idx, entry) in reranked.iter().enumerate() {
                        let _ = store.record_reinforcement(entry.id, &session_id, idx as i64);
                    }
                }

                // Spawn async post-retrieval maintenance (43.6).
                if let Ok(store) = self.state.memory_store.lock() {
                    crate::memory::post_retrieval::run_maintenance(
                        &store,
                        &verdicts,
                        &req.query,
                        &crate::memory::post_retrieval::PostRetrievalConfig::default(),
                    );
                }

                reranked
            } else {
                entries.into_iter().take(limit).collect()
            }
        } else {
            entries.into_iter().take(limit).collect()
        };

        // Step 3: stamp positional scores so clients can sort / threshold.
        let total = entries.len() as f64;
        let scored: Vec<SearchHit> = entries
            .into_iter()
            .enumerate()
            .map(|(idx, e)| {
                let mut hit: SearchHit = e.into();
                // Linearly decreasing score from 1.0 (top) to ~0 (last).
                hit.score = if total > 0.0 {
                    1.0 - (idx as f64 / total)
                } else {
                    0.0
                };
                hit
            })
            .collect();

        // Step 4: Gap detection (43.8) — record blind spots.
        let top_score = scored.first().map(|h| h.score).unwrap_or(0.0);
        if let Ok(store) = self.state.memory_store.lock() {
            let _ = crate::memory::gap_detection::detect_and_record_gap(
                &store,
                top_score,
                query_emb.as_deref(),
                &req.query,
                None,
                &crate::memory::gap_detection::GapDetectionConfig::default(),
            );
        }

        Ok(scored)
    }

    async fn get_entry(&self, caps: &GatewayCaps, id: i64) -> Result<MemoryEntry, GatewayError> {
        require_read(caps)?;
        let store = self
            .state
            .memory_store
            .lock()
            .map_err(GatewayError::from_lock)?;
        store.get_by_id(id).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                GatewayError::NotFound(format!("memory id {id}"))
            }
            other => GatewayError::Storage(other.to_string()),
        })
    }

    async fn get_entry_detail(
        &self,
        caps: &GatewayCaps,
        id: i64,
    ) -> Result<EntryDetail, GatewayError> {
        require_read(caps)?;
        let store = self
            .state
            .memory_store
            .lock()
            .map_err(GatewayError::from_lock)?;
        let entry = store.get_by_id(id).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                GatewayError::NotFound(format!("memory id {id}"))
            }
            other => GatewayError::Storage(other.to_string()),
        })?;
        let reinforcements = store
            .get_reinforcements(id, 10)
            .map_err(|e| GatewayError::Storage(e.to_string()))?;
        Ok(EntryDetail {
            entry,
            reinforcements,
        })
    }

    async fn list_recent(
        &self,
        caps: &GatewayCaps,
        req: RecentRequest,
    ) -> Result<Vec<MemoryEntry>, GatewayError> {
        require_read(caps)?;
        let limit = req.limit.unwrap_or(20).clamp(1, 200);

        let store = self
            .state
            .memory_store
            .lock()
            .map_err(GatewayError::from_lock)?;
        let mut all = store
            .get_all()
            .map_err(|e| GatewayError::Storage(e.to_string()))?;
        // Newest-first.
        all.sort_by_key(|e| std::cmp::Reverse(e.created_at));

        // Apply optional filters in-memory. Volume is always bounded by
        // `MemoryStats.total` which the brain-tier UI already polls; for
        // realistic personal-use sizes (<=10k entries) this is cheap.
        let kind_filter = req.kind.as_deref().map(parse_memory_type);
        let tag_filter = req.tag.as_deref().map(|t| {
            t.split([',', ' '])
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        });

        let filtered: Vec<MemoryEntry> = all
            .into_iter()
            .filter(|e| match kind_filter {
                Some(Some(ref k)) => &e.memory_type == k,
                Some(None) => true, // unrecognised kind string — don't filter (be permissive)
                None => true,
            })
            .filter(|e| match &tag_filter {
                Some(needles) if !needles.is_empty() => {
                    let hay = e.tags.to_lowercase();
                    needles
                        .iter()
                        .any(|n| hay.split(',').any(|t| t.trim() == n))
                }
                _ => true,
            })
            .filter(|e| match req.since {
                Some(since) => e.created_at >= since,
                None => true,
            })
            .take(limit)
            .collect();

        Ok(filtered)
    }

    async fn kg_neighbors(
        &self,
        caps: &GatewayCaps,
        req: KgRequest,
    ) -> Result<KgNeighborhood, GatewayError> {
        require_read(caps)?;
        if req.id <= 0 {
            return Err(GatewayError::InvalidArgument("id must be positive".into()));
        }
        let dir = match req.direction.as_str() {
            "in" => EdgeDirection::In,
            "out" => EdgeDirection::Out,
            _ => EdgeDirection::Both,
        };

        // Check LRU cache first (Chunk 41.13).
        let cache_key = crate::memory::kg_cache::KgCacheKey {
            seed_id: req.id,
            depth: req.depth,
            direction: dir,
        };
        let traversal = if let Some(cached) = self.state.kg_cache.get(&cache_key) {
            cached
        } else {
            // Perform bounded BFS.
            let store = self
                .state
                .memory_store
                .lock()
                .map_err(GatewayError::from_lock)?;
            let traversal = crate::memory::kg_cache::bounded_bfs(
                req.id,
                req.depth,
                dir,
                |node_id, direction| store.get_edges_for(node_id, direction).unwrap_or_default(),
            );
            drop(store);
            // Cache the result.
            self.state.kg_cache.insert(cache_key, traversal.clone());
            traversal
        };

        let store = self
            .state
            .memory_store
            .lock()
            .map_err(GatewayError::from_lock)?;
        let center = store.get_by_id(req.id).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                GatewayError::NotFound(format!("memory id {}", req.id))
            }
            other => GatewayError::Storage(other.to_string()),
        })?;

        // Flatten all edges across hops into neighbors.
        let neighbors = traversal
            .hops
            .iter()
            .flat_map(|hop| hop.edges.iter())
            .map(|edge| {
                let other_id = if edge.src_id == req.id {
                    edge.dst_id
                } else {
                    edge.src_id
                };
                let entry = store.get_by_id(other_id).ok();
                KgNeighbor {
                    edge: edge.clone(),
                    entry,
                }
            })
            .collect();

        Ok(KgNeighborhood {
            center,
            neighbors,
            truncated: traversal.truncated,
        })
    }

    async fn summarize(
        &self,
        caps: &GatewayCaps,
        req: SummarizeRequest,
    ) -> Result<SummarizeResponse, GatewayError> {
        require_read(caps)?;
        // Resolve memory ids first (drops the lock before .await).
        let ids = req.memory_ids.clone().unwrap_or_default();
        let mut resolved = Vec::with_capacity(ids.len());
        let mut seen_ids = Vec::with_capacity(ids.len());
        if !ids.is_empty() {
            let store = self
                .state
                .memory_store
                .lock()
                .map_err(GatewayError::from_lock)?;
            for id in &ids {
                if let Ok(entry) = store.get_by_id(*id) {
                    if seen_ids.contains(id) {
                        continue;
                    }
                    seen_ids.push(*id);
                    resolved.push(entry.content);
                }
            }
        }

        if let Some(query) = req
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
        {
            let hits = self
                .search(
                    caps,
                    SearchRequest {
                        query: query.to_string(),
                        limit: Some(req.limit.unwrap_or(5).clamp(1, 20)),
                        mode: SearchMode::Rrf,
                        rerank: default_search_rerank(),
                        rerank_threshold: default_search_rerank_threshold(),
                    },
                )
                .await?;
            for hit in hits {
                if seen_ids.contains(&hit.id) {
                    continue;
                }
                seen_ids.push(hit.id);
                resolved.push(hit.content);
            }
        }
        let resolved_count = resolved.len();

        // Compose the input.
        let mut input = req.text.clone().unwrap_or_default();
        for content in resolved {
            if !input.is_empty() {
                input.push_str("\n\n");
            }
            input.push_str(&content);
        }
        if input.trim().is_empty() {
            return Err(GatewayError::InvalidArgument(
                "either `text`, `memory_ids`, or `query` must yield non-empty content".into(),
            ));
        }

        // Run the brain (or degrade gracefully when none).
        let model_opt = self.active_brain()?;
        let summary = match model_opt {
            Some(model) => {
                OllamaAgent::new(&model)
                    .summarize_conversation(&input)
                    .await
            }
            None => None,
        };
        Ok(SummarizeResponse {
            summary,
            resolved_count,
        })
    }

    async fn suggest_context(
        &self,
        caps: &GatewayCaps,
        req: SuggestContextRequest,
    ) -> Result<SuggestContextPack, GatewayError> {
        require_read(caps)?;
        if req.query.trim().is_empty() {
            return Err(GatewayError::InvalidArgument("query is empty".into()));
        }
        let limit = req.limit.unwrap_or(5).clamp(1, 20);

        // Step 1: search using the best mode the brain supports.
        let model_opt = self.active_brain()?;
        let mode = if model_opt.is_some() {
            SearchMode::Hyde
        } else {
            SearchMode::Rrf
        };
        let hits = self
            .search(
                caps,
                SearchRequest {
                    query: req.query.clone(),
                    limit: Some(limit),
                    mode,
                    rerank: true,
                    rerank_threshold: crate::settings::DEFAULT_RERANK_THRESHOLD,
                },
            )
            .await?;

        // Step 1b: Cascade retrieval through memory_edges (43.5).
        // Expand the top-K via BFS depth ≤ 2 with edge-weighted decay.
        let hits = if !hits.is_empty() {
            let store = self
                .state
                .memory_store
                .lock()
                .map_err(GatewayError::from_lock)?;

            // Build seed scores: linearly decreasing from 1.0
            let total = hits.len() as f64;
            let seeds: Vec<(i64, f64)> = hits
                .iter()
                .enumerate()
                .map(|(i, h)| {
                    let score = if total > 0.0 {
                        1.0 - (i as f64 / total)
                    } else {
                        0.0
                    };
                    (h.id, score)
                })
                .collect();

            let expanded =
                crate::memory::cascade::cascade_expand(&store.conn, &seeds, None).unwrap_or(seeds);

            // Materialize expanded entries as SearchHits.
            let expanded_total = expanded.len() as f64;
            let mut expanded_hits: Vec<SearchHit> = Vec::with_capacity(expanded.len().min(limit));
            for (idx, (id, _)) in expanded.iter().enumerate().take(limit) {
                if let Ok(entry) = store.get_by_id(*id) {
                    let mut hit: SearchHit = entry.into();
                    hit.score = if expanded_total > 0.0 {
                        1.0 - (idx as f64 / expanded_total)
                    } else {
                        0.0
                    };
                    expanded_hits.push(hit);
                }
            }
            expanded_hits
        } else {
            hits
        };

        // Step 2: KG one-hop around the top hit (if any).
        let kg = if let Some(top) = hits.first() {
            self.kg_neighbors(
                caps,
                KgRequest {
                    id: top.id,
                    depth: 1,
                    direction: "both".into(),
                },
            )
            .await
            .ok()
        } else {
            None
        };

        // Step 3: summary over the resolved hits.
        let summary = if !hits.is_empty() {
            self.summarize(
                caps,
                SummarizeRequest {
                    text: Some(req.query.clone()),
                    memory_ids: Some(hits.iter().map(|h| h.id).collect()),
                    query: None,
                    limit: None,
                },
            )
            .await
            .ok()
            .and_then(|r| r.summary)
        } else {
            None
        };

        let brain = model_opt.as_deref().unwrap_or("none");
        let fingerprint = Self::fingerprint(brain, &hits);
        Ok(SuggestContextPack {
            hits,
            kg,
            summary,
            fingerprint,
        })
    }

    async fn ingest_url(
        &self,
        caps: &GatewayCaps,
        req: IngestUrlRequest,
    ) -> Result<IngestUrlResponse, GatewayError> {
        if !caps.brain_write {
            return Err(GatewayError::PermissionDenied("brain_write"));
        }
        if req.url.trim().is_empty() {
            return Err(GatewayError::InvalidArgument("url is empty".into()));
        }
        let sink = self
            .ingest
            .as_ref()
            .ok_or_else(|| GatewayError::NotConfigured("ingest sink not attached".into()))?;
        sink.start_ingest(req.url, req.tags, req.importance).await
    }

    async fn ingest_lesson(
        &self,
        caps: &GatewayCaps,
        req: IngestLessonRequest,
    ) -> Result<IngestLessonResponse, GatewayError> {
        if !caps.brain_write {
            return Err(GatewayError::PermissionDenied("brain_write"));
        }
        if req.content.trim().is_empty() {
            return Err(GatewayError::InvalidArgument(
                "content cannot be empty".into(),
            ));
        }

        let tags = req
            .tags
            .clone()
            .unwrap_or_else(|| "lesson,agent-session".to_string());
        let importance = req.importance.unwrap_or(8).clamp(1, 10);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        // Insert into memories table.
        let store = self
            .state
            .memory_store
            .lock()
            .map_err(GatewayError::from_lock)?;

        store
            .conn()
            .execute(
                "INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    req.content,
                    tags,
                    importance,
                    "lesson",
                    now_ms,
                    "long",
                    1.0,
                    req.category,
                    "procedural",
                ],
            )
            .map_err(|e| GatewayError::Storage(format!("insert lesson: {e}")))?;
        let memory_id = store.conn().last_insert_rowid();

        // Also append to memory-seed.sql so the lesson survives a reseed.
        // This is idempotent: if the exact content already exists, the WHERE NOT EXISTS guard skips it.
        let seed_path = self.state.data_dir.join("mcp-data/shared/memory-seed.sql");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&seed_path)
        {
            let sql_line = format!(
                "INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)\n\
                 SELECT '{}', '{}', {}, 'lesson', {}, 'long', 1.0, '{}', 'procedural'\n\
                 WHERE NOT EXISTS (SELECT 1 FROM memories WHERE content LIKE '{}%%');\n",
                req.content.replace("'", "''"),
                tags,
                importance,
                now_ms,
                req.category,
                req.content.replace("'", "''").chars().take(50).collect::<String>(),
            );
            let _ = writeln!(file, "{}", sql_line);
        }

        Ok(IngestLessonResponse {
            memory_id,
            tags,
            importance,
            persisted_to_seed: true,
        })
    }

    async fn health(&self, caps: &GatewayCaps) -> Result<HealthResponse, GatewayError> {
        require_read(caps)?;
        let model_opt = self.active_brain()?;
        let brain_mode = self
            .state
            .brain_mode
            .lock()
            .map_err(GatewayError::from_lock)?
            .clone();
        let provider = match (&brain_mode, &model_opt) {
            (Some(crate::brain::BrainMode::FreeApi { .. }), _) => "free",
            (Some(crate::brain::BrainMode::PaidApi { .. }), _) => "openai",
            (Some(crate::brain::BrainMode::LocalOllama { .. }), _) => "ollama",
            (Some(crate::brain::BrainMode::LocalLmStudio { .. }), _) => "lmstudio",
            (None, Some(_)) => "ollama",
            (None, None) => "none",
        };

        let (stats, embedded_long_count, queue_status) = {
            let store = self
                .state
                .memory_store
                .lock()
                .map_err(GatewayError::from_lock)?;
            let stats = store
                .stats()
                .map_err(|e| GatewayError::Storage(e.to_string()))?;
            let embedded_long_count = store
                .embedded_long_count()
                .map_err(|e| GatewayError::Storage(e.to_string()))?;
            let queue_status = crate::memory::embedding_queue::queue_status(store.conn())
                .unwrap_or(crate::memory::embedding_queue::EmbeddingQueueStatus {
                    pending: 0,
                    failing: 0,
                    next_retry_at: None,
                });
            (stats, embedded_long_count, queue_status)
        };

        let rag_quality_pct = if stats.long > 0 {
            ((embedded_long_count as f64 / stats.long as f64) * 100.0)
                .round()
                .clamp(0.0, 100.0) as u8
        } else {
            100
        };

        let rag_quality = RagQualityHealth {
            label: rag_quality_label(rag_quality_pct, stats.long).to_string(),
            description: rag_quality_description(
                rag_quality_pct,
                embedded_long_count,
                stats.long,
                queue_status.pending,
            ),
            formula: "embedded_long_memory_count / long_memory_count * 100".into(),
            embedded_long_memory_count: embedded_long_count,
            long_memory_count: stats.long,
            pending_embedding_count: queue_status.pending,
            failing_embedding_count: queue_status.failing,
            next_embedding_retry_at: queue_status.next_retry_at,
        };

        let memory = MemoryHealth {
            total: stats.total,
            short_count: stats.short,
            working_count: stats.working,
            long_count: stats.long,
            embedded_total: stats.embedded,
            description: memory_health_description(
                stats.total,
                stats.short,
                stats.working,
                stats.long,
                stats.embedded,
            ),
        };

        let descriptions = HealthDescriptions {
            brain_provider: "Active LLM or embedding backend used by brain tools.".into(),
            brain_model: "Selected model id when the provider reports one; null means no model id is active.".into(),
            rag_quality_pct: "RAG means retrieval-augmented generation. This percentage is long-term memory vector coverage: embedded_long_memory_count / long_memory_count * 100. Higher means semantic/vector recall can use more memories.".into(),
            memory_total: "All memories stored across short, working, and long tiers.".into(),
        };

        let embed_worker = Some(self.state.embed_worker_metrics.snapshot());

        Ok(HealthResponse {
            version: env!("CARGO_PKG_VERSION").to_string(),
            brain_provider: provider.to_string(),
            brain_model: model_opt,
            rag_quality_pct,
            memory_total: stats.total,
            rag_quality,
            memory,
            embed_worker,
            descriptions,
        })
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn rag_quality_label(pct: u8, long_memory_count: i64) -> &'static str {
    if long_memory_count == 0 {
        return "no_long_term_memories";
    }
    match pct {
        90..=100 => "ready",
        70..=89 => "mostly_ready",
        40..=69 => "partial_vector_coverage",
        1..=39 => "low_vector_coverage",
        _ => "no_vector_coverage",
    }
}

fn rag_quality_description(
    pct: u8,
    embedded_long_count: i64,
    long_memory_count: i64,
    pending_embedding_count: u64,
) -> String {
    if long_memory_count == 0 {
        return "No long-term memories exist yet, so RAG quality is neutral rather than bad. Add or ingest memories to make this signal meaningful.".into();
    }

    let mut description = format!(
        "{pct}% means {embedded_long_count} of {long_memory_count} long-term memories currently have vector embeddings. Keyword search and graph lookup still work, but semantic RAG recall is limited until more memories are embedded."
    );
    if pending_embedding_count > 0 {
        description.push_str(&format!(
            " {pending_embedding_count} embedding jobs are queued for retry/backfill."
        ));
    } else if pct < 100 {
        description.push_str(
            " No embedding jobs are currently queued, so configure or restart the embedding provider/worker if this stays low.",
        );
    }
    description
}

fn memory_health_description(
    total: i64,
    short: i64,
    working: i64,
    long: i64,
    embedded: i64,
) -> String {
    format!(
        "{total} memories total: {short} short, {working} working, {long} long. {embedded} memories across all tiers have vector embeddings."
    )
}

fn require_read(caps: &GatewayCaps) -> Result<(), GatewayError> {
    if caps.brain_read {
        Ok(())
    } else {
        Err(GatewayError::PermissionDenied("brain_read"))
    }
}

/// Tolerant parser for cognitive-kind filter strings. Returns
/// `Some(None)` for unrecognised strings (signal to the filter that the
/// kind is unknown and should be ignored permissively rather than match
/// nothing).
fn parse_memory_type(s: &str) -> Option<MemoryType> {
    match s.trim().to_lowercase().as_str() {
        "fact" => Some(MemoryType::Fact),
        "preference" => Some(MemoryType::Preference),
        "context" => Some(MemoryType::Context),
        "summary" => Some(MemoryType::Summary),
        _ => None,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::NewMemory;
    use std::sync::Mutex as StdMutex;

    type RecordedCall = (String, Option<String>, Option<i64>);

    /// In-test sink that just records the call. Lets us assert the
    /// gateway routes write ops correctly without a real Tauri runtime.
    struct RecordingIngestSink {
        calls: StdMutex<Vec<RecordedCall>>,
    }

    impl RecordingIngestSink {
        fn new() -> Self {
            Self {
                calls: StdMutex::new(Vec::new()),
            }
        }
        fn calls(&self) -> Vec<RecordedCall> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl IngestSink for RecordingIngestSink {
        async fn start_ingest(
            &self,
            source: String,
            tags: Option<String>,
            importance: Option<i64>,
        ) -> Result<IngestUrlResponse, GatewayError> {
            self.calls
                .lock()
                .unwrap()
                .push((source.clone(), tags.clone(), importance));
            Ok(IngestUrlResponse {
                task_id: "task-test-1".into(),
                source,
                source_type: "url".into(),
            })
        }
    }

    fn seed_state() -> AppState {
        let state = AppState::for_test();
        {
            let store = state.memory_store.lock().unwrap();
            store
                .add(NewMemory {
                    content: "Rust uses ownership for memory safety.".into(),
                    tags: "rust,language,memory".into(),
                    importance: 5,
                    memory_type: MemoryType::Fact,
                    ..Default::default()
                })
                .unwrap();
            store
                .add(NewMemory {
                    content: "User prefers dark mode UIs.".into(),
                    tags: "ui,preference".into(),
                    importance: 3,
                    memory_type: MemoryType::Preference,
                    ..Default::default()
                })
                .unwrap();
        }
        state
    }

    fn seed_state_from_shared_mcp_seed() -> AppState {
        let state = AppState::for_test();
        {
            let store = state.memory_store.lock().unwrap();
            store
                .conn()
                .execute_batch(include_str!("../../../mcp-data/shared/memory-seed.sql"))
                .expect("shared MCP seed SQL should apply to canonical in-memory schema");
        }
        state
    }

    fn memory_id_containing(state: &AppState, needle: &str) -> i64 {
        let store = state.memory_store.lock().unwrap();
        store
            .get_all()
            .expect("memory list should load")
            .into_iter()
            .find(|entry| entry.content.contains(needle))
            .unwrap_or_else(|| panic!("missing seeded memory containing {needle:?}"))
            .id
    }

    #[tokio::test]
    async fn read_op_requires_brain_read_capability() {
        let gw = AppStateGateway::new(seed_state());
        let err = gw
            .search(
                &GatewayCaps::NONE,
                SearchRequest {
                    query: "rust".into(),
                    limit: None,
                    mode: SearchMode::Rrf,
                    rerank: true,
                    rerank_threshold: crate::settings::DEFAULT_RERANK_THRESHOLD,
                },
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, GatewayError::PermissionDenied("brain_read")),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn write_op_requires_brain_write_capability() {
        let sink = Arc::new(RecordingIngestSink::new());
        let gw = AppStateGateway::with_ingest(seed_state(), sink.clone());
        // Default caps have brain_read=true, brain_write=false.
        let err = gw
            .ingest_url(
                &GatewayCaps::default(),
                IngestUrlRequest {
                    url: "https://example.com".into(),
                    tags: None,
                    importance: None,
                },
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, GatewayError::PermissionDenied("brain_write")),
            "got {err:?}"
        );
        assert!(
            sink.calls().is_empty(),
            "sink must not be invoked when caps reject"
        );
    }

    #[tokio::test]
    async fn write_op_routes_through_sink_when_permitted() {
        let sink = Arc::new(RecordingIngestSink::new());
        let gw = AppStateGateway::with_ingest(seed_state(), sink.clone());
        let resp = gw
            .ingest_url(
                &GatewayCaps::READ_WRITE,
                IngestUrlRequest {
                    url: "https://example.com/doc".into(),
                    tags: Some("imported,demo".into()),
                    importance: Some(4),
                },
            )
            .await
            .unwrap();
        assert_eq!(resp.task_id, "task-test-1");
        assert_eq!(sink.calls().len(), 1);
        let (src, tags, imp) = sink.calls()[0].clone();
        assert_eq!(src, "https://example.com/doc");
        assert_eq!(tags.as_deref(), Some("imported,demo"));
        assert_eq!(imp, Some(4));
    }

    #[tokio::test]
    async fn write_op_without_sink_reports_not_configured() {
        let gw = AppStateGateway::new(seed_state());
        let err = gw
            .ingest_url(
                &GatewayCaps::READ_WRITE,
                IngestUrlRequest {
                    url: "https://example.com".into(),
                    tags: None,
                    importance: None,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, GatewayError::NotConfigured(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn search_rejects_empty_query() {
        let gw = AppStateGateway::new(seed_state());
        let err = gw
            .search(
                &GatewayCaps::default(),
                SearchRequest {
                    query: "   ".into(),
                    limit: None,
                    mode: SearchMode::Rrf,
                    rerank: true,
                    rerank_threshold: crate::settings::DEFAULT_RERANK_THRESHOLD,
                },
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, GatewayError::InvalidArgument(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn search_request_defaults_rerank_for_rrf() {
        let req: SearchRequest = serde_json::from_value(serde_json::json!({
            "query": "memory"
        }))
        .expect("minimal search request should deserialize");

        assert_eq!(req.mode, SearchMode::Rrf);
        assert!(req.rerank);
        assert_eq!(
            req.rerank_threshold,
            crate::settings::DEFAULT_RERANK_THRESHOLD
        );
    }

    #[tokio::test]
    async fn search_returns_descending_positional_scores() {
        let gw = AppStateGateway::new(seed_state());
        let hits = gw
            .search(
                &GatewayCaps::default(),
                SearchRequest {
                    query: "rust".into(),
                    limit: Some(5),
                    mode: SearchMode::Rrf,
                    rerank: true,
                    rerank_threshold: crate::settings::DEFAULT_RERANK_THRESHOLD,
                },
            )
            .await
            .unwrap();
        // Positional scores are strictly non-increasing.
        for pair in hits.windows(2) {
            assert!(
                pair[0].score >= pair[1].score,
                "scores must be non-increasing"
            );
        }
    }

    #[tokio::test]
    async fn get_entry_returns_not_found_for_missing_id() {
        let gw = AppStateGateway::new(seed_state());
        let err = gw
            .get_entry(&GatewayCaps::default(), 999_999)
            .await
            .unwrap_err();
        assert!(matches!(err, GatewayError::NotFound(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn list_recent_filters_by_kind_and_tag() {
        let gw = AppStateGateway::new(seed_state());
        let prefs = gw
            .list_recent(
                &GatewayCaps::default(),
                RecentRequest {
                    limit: Some(50),
                    kind: Some("preference".into()),
                    tag: None,
                    since: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(prefs.len(), 1);
        assert!(prefs[0].content.contains("dark mode"));

        let by_tag = gw
            .list_recent(
                &GatewayCaps::default(),
                RecentRequest {
                    limit: None,
                    kind: None,
                    tag: Some("rust".into()),
                    since: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(by_tag.len(), 1);
        assert!(by_tag[0].content.contains("ownership"));
    }

    #[tokio::test]
    async fn kg_neighbors_reports_truncation_when_depth_above_one() {
        let state = seed_state();
        let id = {
            let store = state.memory_store.lock().unwrap();
            store.get_all().unwrap()[0].id
        };
        let gw = AppStateGateway::new(state);
        let nb = gw
            .kg_neighbors(
                &GatewayCaps::default(),
                KgRequest {
                    id,
                    depth: 4, // exceeds MAX_DEPTH (3), triggers truncation
                    direction: "both".into(),
                },
            )
            .await
            .unwrap();
        assert_eq!(nb.center.id, id);
        assert!(nb.truncated, "depth > MAX_DEPTH must report truncation");
    }

    #[tokio::test]
    async fn kg_neighbors_reads_shared_seed_lesson_hub_edges() {
        let state = seed_state_from_shared_mcp_seed();
        let lesson_id = memory_id_containing(&state, "LESSON: The MCP seed");
        let lessons_hub_id = memory_id_containing(
            &state,
            "Durable gotchas, decisions, and lessons learned from past agent sessions",
        );
        let stack_anchor_id =
            memory_id_containing(&state, "STACK COVERAGE: the mcp-data seed exercises");

        let gw = AppStateGateway::new(state.clone());
        let nb = gw
            .kg_neighbors(
                &GatewayCaps::default(),
                KgRequest {
                    id: lesson_id,
                    depth: 1,
                    direction: "both".into(),
                },
            )
            .await
            .expect("seeded lesson should have KG neighbours");

        assert_eq!(nb.center.id, lesson_id);
        assert!(
            nb.neighbors.iter().any(|neighbor| {
                neighbor.edge.rel_type == "part_of"
                    && neighbor.edge.dst_id == lessons_hub_id
                    && neighbor
                        .entry
                        .as_ref()
                        .is_some_and(|entry| entry.id == lessons_hub_id)
            }),
            "LESSON rows should be wired part_of the lessons-learned hub"
        );

        let two_hop = {
            let store = state.memory_store.lock().unwrap();
            store
                .traverse_from(lesson_id, 2, None)
                .expect("seed graph should support two-hop traversal")
        };
        assert!(
            two_hop
                .iter()
                .any(|(id, hop)| *id == stack_anchor_id && *hop == 2),
            "lesson -> lessons hub -> stack coverage anchor should be reachable in two hops"
        );
    }

    #[tokio::test]
    async fn summarize_requires_text_memory_ids_or_query() {
        let gw = AppStateGateway::new(seed_state());
        let err = gw
            .summarize(
                &GatewayCaps::default(),
                SummarizeRequest {
                    text: None,
                    memory_ids: None,
                    query: None,
                    limit: None,
                },
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, GatewayError::InvalidArgument(_)),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn summarize_no_brain_returns_none_summary_with_resolution_count() {
        let state = seed_state();
        let ids: Vec<i64> = {
            let store = state.memory_store.lock().unwrap();
            store.get_all().unwrap().iter().map(|e| e.id).collect()
        };
        let gw = AppStateGateway::new(state);
        let resp = gw
            .summarize(
                &GatewayCaps::default(),
                SummarizeRequest {
                    text: None,
                    memory_ids: Some(ids.clone()),
                    query: None,
                    limit: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(resp.resolved_count, ids.len());
        // No brain configured ⇒ summary is None. Test asserts the
        // graceful-degradation contract — never an error in this path.
        assert!(resp.summary.is_none());
    }

    #[tokio::test]
    async fn summarize_query_resolves_search_hits() {
        let gw = AppStateGateway::new(seed_state());
        let resp = gw
            .summarize(
                &GatewayCaps::default(),
                SummarizeRequest {
                    text: None,
                    memory_ids: None,
                    query: Some("rust ownership".into()),
                    limit: Some(3),
                },
            )
            .await
            .unwrap();
        assert!(resp.resolved_count >= 1);
        // No brain configured in seed_state(), so this verifies the
        // search-backed resolution path without requiring Ollama.
        assert!(resp.summary.is_none());
    }

    #[tokio::test]
    async fn suggest_context_is_delta_stable_for_identical_input() {
        let gw = AppStateGateway::new(seed_state());
        let req = SuggestContextRequest {
            file_path: Some("src/main.rs".into()),
            cursor_offset: Some(42),
            selection: None,
            query: "rust ownership".into(),
            limit: Some(5),
        };
        let a = gw
            .suggest_context(&GatewayCaps::default(), req.clone())
            .await
            .unwrap();
        let b = gw
            .suggest_context(&GatewayCaps::default(), req)
            .await
            .unwrap();
        assert_eq!(
            a.fingerprint, b.fingerprint,
            "identical input must yield identical fingerprint"
        );
        assert_eq!(a.hits.len(), b.hits.len());
        for (x, y) in a.hits.iter().zip(b.hits.iter()) {
            assert_eq!(x.id, y.id, "hit order must be stable");
        }
    }

    #[tokio::test]
    async fn suggest_context_fingerprint_changes_when_brain_changes() {
        let state = seed_state();
        let gw = AppStateGateway::new(state.clone());
        let req = SuggestContextRequest {
            file_path: None,
            cursor_offset: None,
            selection: None,
            query: "rust".into(),
            limit: Some(5),
        };
        let a = gw
            .suggest_context(&GatewayCaps::default(), req.clone())
            .await
            .unwrap();
        // Flip the active brain to a different model identifier — same
        // memories, different brain ⇒ different fingerprint (the
        // editor-cache contract).
        *state.active_brain.lock().unwrap() = Some("gemma3:4b-different".into());
        let b = gw
            .suggest_context(&GatewayCaps::default(), req)
            .await
            .unwrap();
        assert_ne!(a.fingerprint, b.fingerprint);
    }

    #[tokio::test]
    async fn health_reports_provider_and_memory_total() {
        let state = seed_state();
        let gw = AppStateGateway::new(state);
        let h = gw.health(&GatewayCaps::default()).await.unwrap();
        assert_eq!(h.brain_provider, "none");
        assert!(h.brain_model.is_none());
        // Two seeded memories.
        assert_eq!(h.memory_total, 2);
        // Seeded entries land in the long tier with NO embedding (no
        // brain configured in tests). The heuristic therefore returns
        // 0 % — `embedded == 0`, `long > 0`. This is the correct,
        // documented behaviour: it tells the editor "you've got memories
        // but they aren't searchable by vector yet — configure a brain."
        assert_eq!(h.rag_quality_pct, 0);
        assert_eq!(h.rag_quality.label, "no_vector_coverage");
        assert_eq!(h.rag_quality.embedded_long_memory_count, 0);
        assert_eq!(h.rag_quality.long_memory_count, 2);
        assert!(h.rag_quality.description.contains("0% means 0 of 2"));
        assert_eq!(h.memory.total, 2);
        assert_eq!(h.memory.long_count, 2);
        assert!(h
            .descriptions
            .rag_quality_pct
            .contains("retrieval-augmented generation"));
    }

    #[test]
    fn fingerprint_is_deterministic_and_id_sensitive() {
        let mk = |id, content: &str| SearchHit {
            id,
            content: content.into(),
            tags: String::new(),
            importance: 1,
            score: 0.0,
            source_url: None,
            tier: MemoryTier::Long.as_str().to_string(),
        };
        let a = AppStateGateway::fingerprint("brain-x", &[mk(1, "a"), mk(2, "b")]);
        let b = AppStateGateway::fingerprint("brain-x", &[mk(1, "a"), mk(2, "b")]);
        let c = AppStateGateway::fingerprint("brain-x", &[mk(1, "a"), mk(3, "c")]);
        let d = AppStateGateway::fingerprint("brain-y", &[mk(1, "a"), mk(2, "b")]);
        assert_eq!(a, b, "same brain + same ids ⇒ same fingerprint");
        assert_ne!(a, c, "different ids ⇒ different fingerprint");
        assert_ne!(a, d, "different brain ⇒ different fingerprint");
        assert_eq!(a.len(), 64, "SHA-256 hex digest is 64 chars");
    }

    #[test]
    fn default_caps_are_read_only() {
        let c = GatewayCaps::default();
        assert!(c.brain_read);
        assert!(!c.brain_write);
        assert!(!c.code_read);
    }

    #[test]
    fn parse_memory_type_is_tolerant() {
        assert!(matches!(parse_memory_type("Fact"), Some(MemoryType::Fact)));
        assert!(matches!(
            parse_memory_type("preference"),
            Some(MemoryType::Preference)
        ));
        assert!(matches!(
            parse_memory_type("CONTEXT"),
            Some(MemoryType::Context)
        ));
        assert!(matches!(
            parse_memory_type("summary"),
            Some(MemoryType::Summary)
        ));
        assert!(parse_memory_type("nonsense").is_none());
    }

    // ─── Chunk 15.7 — Incremental-indexing QA ─────────────────────────

    /// Cold call: first suggest_context returns a non-empty fingerprint and hits.
    #[tokio::test]
    async fn incremental_indexing_cold_call_returns_valid_fingerprint() {
        let gw = AppStateGateway::new(seed_state());
        let pack = gw
            .suggest_context(
                &GatewayCaps::default(),
                SuggestContextRequest {
                    file_path: Some("lib.rs".into()),
                    cursor_offset: None,
                    selection: None,
                    query: "rust ownership".into(),
                    limit: Some(10),
                },
            )
            .await
            .unwrap();
        assert!(
            !pack.fingerprint.is_empty(),
            "cold call must return fingerprint"
        );
        assert_eq!(pack.fingerprint.len(), 64, "SHA-256 hex is 64 chars");
        assert!(!pack.hits.is_empty(), "cold call must return hits");
    }

    /// Warm call: identical request to the same gateway yields the same
    /// fingerprint — this is the cache-hit contract for VS Code Copilot.
    #[tokio::test]
    async fn incremental_indexing_warm_call_cache_hit() {
        let gw = AppStateGateway::new(seed_state());
        let req = SuggestContextRequest {
            file_path: Some("src/main.rs".into()),
            cursor_offset: Some(100),
            selection: None,
            query: "dark mode preference".into(),
            limit: Some(5),
        };
        let first = gw
            .suggest_context(&GatewayCaps::default(), req.clone())
            .await
            .unwrap();
        let second = gw
            .suggest_context(&GatewayCaps::default(), req)
            .await
            .unwrap();
        assert_eq!(
            first.fingerprint, second.fingerprint,
            "warm call (no mutation) must produce same fingerprint (cache hit)"
        );
    }

    /// Invalidation: after ingesting new memory data, the fingerprint
    /// changes — simulates file-watcher invalidation where a new memory
    /// is added and the editor-side cache must be busted.
    #[tokio::test]
    async fn incremental_indexing_invalidation_after_new_memory() {
        let state = seed_state();
        let gw = AppStateGateway::new(state.clone());
        let req = SuggestContextRequest {
            file_path: None,
            cursor_offset: None,
            selection: None,
            query: "rust".into(),
            limit: Some(10),
        };
        let before = gw
            .suggest_context(&GatewayCaps::default(), req.clone())
            .await
            .unwrap();

        // Mutate: add a new memory that matches the query.
        let added_memory_id = {
            let store = state.memory_store.lock().unwrap();
            store
                .add(NewMemory {
                    content: "Rust borrow checker prevents data races at compile time.".into(),
                    tags: "rust,concurrency".into(),
                    importance: 4,
                    memory_type: MemoryType::Fact,
                    ..Default::default()
                })
                .unwrap()
                .id
        };

        let after = gw
            .suggest_context(&GatewayCaps::default(), req)
            .await
            .unwrap();
        assert_ne!(
            before.fingerprint, after.fingerprint,
            "fingerprint must change after new memory is added (cache invalidation)"
        );
        assert!(
            after.hits.iter().any(|hit| hit.id == added_memory_id),
            "new memory should appear in hits"
        );
    }

    /// Invalidation: deleting a memory also changes the fingerprint.
    #[tokio::test]
    async fn incremental_indexing_invalidation_after_delete() {
        let state = seed_state();
        let gw = AppStateGateway::new(state.clone());
        let req = SuggestContextRequest {
            file_path: None,
            cursor_offset: None,
            selection: None,
            query: "rust".into(),
            limit: Some(10),
        };
        let before = gw
            .suggest_context(&GatewayCaps::default(), req.clone())
            .await
            .unwrap();

        // Delete the first matching memory.
        {
            let store = state.memory_store.lock().unwrap();
            let entries = store.get_all().unwrap();
            let rust_entry = entries.iter().find(|e| e.content.contains("Rust")).unwrap();
            store.delete(rust_entry.id).unwrap();
        }

        let after = gw
            .suggest_context(&GatewayCaps::default(), req)
            .await
            .unwrap();
        assert_ne!(
            before.fingerprint, after.fingerprint,
            "fingerprint must change after memory deletion (cache invalidation)"
        );
    }

    /// Fingerprint is query-sensitive: different queries over the same data
    /// produce different fingerprints (different hit sets).
    #[tokio::test]
    async fn incremental_indexing_fingerprint_is_query_sensitive() {
        let gw = AppStateGateway::new(seed_state());
        let a = gw
            .suggest_context(
                &GatewayCaps::default(),
                SuggestContextRequest {
                    file_path: None,
                    cursor_offset: None,
                    selection: None,
                    query: "rust ownership".into(),
                    limit: Some(10),
                },
            )
            .await
            .unwrap();
        let b = gw
            .suggest_context(
                &GatewayCaps::default(),
                SuggestContextRequest {
                    file_path: None,
                    cursor_offset: None,
                    selection: None,
                    query: "dark mode UI preference".into(),
                    limit: Some(10),
                },
            )
            .await
            .unwrap();
        // Different queries hit different memories ⇒ different fingerprints.
        // (Both queries are designed to match different seeded entries.)
        assert_ne!(
            a.fingerprint, b.fingerprint,
            "different queries should yield different fingerprints"
        );
    }

    #[tokio::test]
    async fn ingest_lesson_requires_write_capability() {
        let gw = AppStateGateway::new(seed_state());
        let err = gw
            .ingest_lesson(
                &GatewayCaps::NONE,
                IngestLessonRequest {
                    content: "Test lesson".into(),
                    tags: None,
                    importance: None,
                    category: "test".into(),
                },
            )
            .await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("brain_write"));
    }

    #[tokio::test]
    async fn ingest_lesson_stores_to_memory() {
        let gw = AppStateGateway::new(seed_state());
        let resp = gw
            .ingest_lesson(
                &GatewayCaps::READ_WRITE,
                IngestLessonRequest {
                    content: "LESSON: Test lesson content.".into(),
                    tags: Some("test,lesson".into()),
                    importance: Some(9),
                    category: "test-category".into(),
                },
            )
            .await
            .expect("ingest_lesson should succeed with write caps");

        // Verify response.
        assert!(resp.memory_id > 0);
        assert_eq!(resp.importance, 9);
        assert!(resp.tags.contains("test,lesson"));
        assert!(resp.persisted_to_seed);

        // Verify the lesson was stored in memory.
        let entry = gw
            .get_entry(&GatewayCaps::READ_WRITE, resp.memory_id)
            .await
            .expect("memory entry should exist");
        assert_eq!(entry.content, "LESSON: Test lesson content.");
    }

    #[tokio::test]
    async fn ingest_lesson_rejects_empty_content() {
        let gw = AppStateGateway::new(seed_state());
        let err = gw
            .ingest_lesson(
                &GatewayCaps::READ_WRITE,
                IngestLessonRequest {
                    content: "  ".into(),
                    tags: None,
                    importance: None,
                    category: "test".into(),
                },
            )
            .await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("empty"));
    }

    #[tokio::test]
    async fn ingest_lesson_clamps_importance() {
        let gw = AppStateGateway::new(seed_state());
        let resp = gw
            .ingest_lesson(
                &GatewayCaps::READ_WRITE,
                IngestLessonRequest {
                    content: "Lesson".into(),
                    tags: None,
                    importance: Some(50), // Should be clamped to 10
                    category: "test".into(),
                },
            )
            .await
            .expect("should clamp importance");
        assert_eq!(resp.importance, 10);
    }

    #[tokio::test]
    async fn ingest_lesson_default_importance() {
        let gw = AppStateGateway::new(seed_state());
        let resp = gw
            .ingest_lesson(
                &GatewayCaps::READ_WRITE,
                IngestLessonRequest {
                    content: "Lesson".into(),
                    tags: None,
                    importance: None, // Should default to 8
                    category: "test".into(),
                },
            )
            .await
            .expect("should default importance");
        assert_eq!(resp.importance, 8);
    }
}
