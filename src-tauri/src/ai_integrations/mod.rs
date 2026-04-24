//! AI Coding Integrations — expose TerranSoul's brain to other AI coding
//! assistants (GitHub Copilot, Claude Desktop, Codex, Cursor, …) over MCP
//! and gRPC.
//!
//! Architectural reference: [`docs/AI-coding-integrations.md`](../../../docs/AI-coding-integrations.md).
//!
//! ## Module layout
//!
//! ```text
//! ai_integrations/
//! ├── gateway.rs   — the typed BrainGateway trait + AppStateGateway adapter
//! ├── mcp/         — MCP transport (Chunk 15.1, not yet shipped)
//! └── grpc/        — gRPC transport (Chunk 15.2, not yet shipped)
//! ```
//!
//! Both transports route to a single [`gateway::BrainGateway`] trait so the
//! op surface can never drift between MCP and gRPC.
//!
//! Chunk reference: **15.3** in `rules/milestones.md`.

pub mod gateway;

pub use gateway::{
    AppStateGateway, BrainGateway, GatewayCaps, GatewayError, HealthResponse, IngestSink,
    IngestUrlRequest, IngestUrlResponse, KgNeighbor, KgNeighborhood, KgRequest, RecentRequest,
    SearchHit, SearchMode, SearchRequest, SuggestContextPack, SuggestContextRequest,
    SummarizeRequest, SummarizeResponse,
};
