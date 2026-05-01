//! gRPC transport for the shared [`BrainGateway`](crate::ai_integrations::gateway::BrainGateway).
//!
//! Chunk 15.2 foundation: a typed `brain.v1` tonic service, conversion layer,
//! mTLS-capable server helpers, and tests. Runtime autostart / control-panel
//! ownership remains with the existing self-hosting commands so the transport
//! can be started explicitly by callers.

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use futures_util::stream;
use futures_util::Stream;
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status};

use crate::ai_integrations::gateway::{
    BrainGateway, GatewayCaps, GatewayError, IngestUrlRequest as GatewayIngestUrlRequest,
    KgRequest as GatewayKgRequest, RecentRequest as GatewayRecentRequest,
    SearchHit as GatewaySearchHit, SearchMode as GatewaySearchMode,
    SearchRequest as GatewaySearchRequest, SuggestContextRequest as GatewaySuggestContextRequest,
    SummarizeRequest as GatewaySummarizeRequest,
};
use crate::memory::MemoryEntry as StoreMemoryEntry;

pub mod proto {
    tonic::include_proto!("terransoul.brain.v1");
}

use proto::brain_server::{Brain, BrainServer};

/// Default loopback endpoint for the typed gRPC transport.
pub const DEFAULT_GRPC_ADDR: &str = "127.0.0.1:7422";

/// Tonic service wrapper. It is cloneable because tonic clones services per
/// connection; the business logic remains in the shared gateway trait object.
#[derive(Clone)]
pub struct BrainGrpcService {
    gateway: Arc<dyn BrainGateway>,
    caps: GatewayCaps,
}

impl BrainGrpcService {
    pub fn new(gateway: Arc<dyn BrainGateway>, caps: GatewayCaps) -> Self {
        Self { gateway, caps }
    }

    pub fn into_server(self) -> BrainServer<Self> {
        BrainServer::new(self)
    }
}

/// Build a tonic server TLS config from PEM-encoded certs. Supplying
/// `client_ca_pem` enables mTLS client-certificate verification.
pub fn tls_config_from_pem(
    server_cert_pem: impl AsRef<[u8]>,
    server_key_pem: impl AsRef<[u8]>,
    client_ca_pem: Option<impl AsRef<[u8]>>,
) -> ServerTlsConfig {
    let identity = Identity::from_pem(server_cert_pem, server_key_pem);
    let cfg = ServerTlsConfig::new().identity(identity);
    match client_ca_pem {
        Some(ca) => cfg.client_ca_root(Certificate::from_pem(ca)),
        None => cfg,
    }
}

/// Serve the gRPC Brain API until `shutdown` resolves.
pub async fn serve_with_shutdown(
    addr: SocketAddr,
    gateway: Arc<dyn BrainGateway>,
    caps: GatewayCaps,
    tls: Option<ServerTlsConfig>,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), tonic::transport::Error> {
    let svc = BrainGrpcService::new(gateway, caps).into_server();
    let builder = Server::builder();
    let builder = if let Some(tls) = tls {
        builder.tls_config(tls)?
    } else {
        builder
    };
    builder
        .add_service(svc)
        .serve_with_shutdown(addr, shutdown)
        .await
}

#[tonic::async_trait]
impl Brain for BrainGrpcService {
    async fn health(
        &self,
        _request: Request<proto::HealthRequest>,
    ) -> Result<Response<proto::HealthResponse>, Status> {
        let response = self
            .gateway
            .health(&self.caps)
            .await
            .map_err(status_from_error)?;
        Ok(Response::new(proto::HealthResponse {
            version: response.version,
            brain_provider: response.brain_provider,
            brain_model: response.brain_model,
            rag_quality_pct: u32::from(response.rag_quality_pct),
            memory_total: response.memory_total,
        }))
    }

    async fn search(
        &self,
        request: Request<proto::SearchRequest>,
    ) -> Result<Response<proto::SearchResponse>, Status> {
        let hits = self
            .gateway
            .search(&self.caps, gateway_search_request(request.into_inner()))
            .await
            .map_err(status_from_error)?;
        Ok(Response::new(proto::SearchResponse {
            hits: hits.into_iter().map(search_hit_to_proto).collect(),
        }))
    }

    type StreamSearchStream = Pin<Box<dyn Stream<Item = Result<proto::SearchHit, Status>> + Send>>;

    async fn stream_search(
        &self,
        request: Request<proto::SearchRequest>,
    ) -> Result<Response<Self::StreamSearchStream>, Status> {
        let hits = self
            .gateway
            .search(&self.caps, gateway_search_request(request.into_inner()))
            .await
            .map_err(status_from_error)?;
        let out = hits.into_iter().map(|hit| Ok(search_hit_to_proto(hit)));
        Ok(Response::new(Box::pin(stream::iter(out))))
    }

    async fn get_entry(
        &self,
        request: Request<proto::GetEntryRequest>,
    ) -> Result<Response<proto::MemoryEntry>, Status> {
        let entry = self
            .gateway
            .get_entry(&self.caps, request.into_inner().id)
            .await
            .map_err(status_from_error)?;
        Ok(Response::new(memory_entry_to_proto(entry)))
    }

    async fn list_recent(
        &self,
        request: Request<proto::RecentRequest>,
    ) -> Result<Response<proto::RecentResponse>, Status> {
        let entries = self
            .gateway
            .list_recent(&self.caps, gateway_recent_request(request.into_inner()))
            .await
            .map_err(status_from_error)?;
        Ok(Response::new(proto::RecentResponse {
            entries: entries.into_iter().map(memory_entry_to_proto).collect(),
        }))
    }

    async fn kg_neighbors(
        &self,
        request: Request<proto::KgRequest>,
    ) -> Result<Response<proto::KgNeighborhood>, Status> {
        let kg = self
            .gateway
            .kg_neighbors(&self.caps, gateway_kg_request(request.into_inner()))
            .await
            .map_err(status_from_error)?;
        Ok(Response::new(kg_to_proto(kg)?))
    }

    async fn summarize(
        &self,
        request: Request<proto::SummarizeRequest>,
    ) -> Result<Response<proto::SummarizeResponse>, Status> {
        let req = request.into_inner();
        let response = self
            .gateway
            .summarize(
                &self.caps,
                GatewaySummarizeRequest {
                    text: req.text,
                    memory_ids: (!req.memory_ids.is_empty()).then_some(req.memory_ids),
                },
            )
            .await
            .map_err(status_from_error)?;
        Ok(Response::new(proto::SummarizeResponse {
            summary: response.summary,
            resolved_count: response.resolved_count as u32,
        }))
    }

    async fn suggest_context(
        &self,
        request: Request<proto::SuggestContextRequest>,
    ) -> Result<Response<proto::SuggestContextResponse>, Status> {
        let req = request.into_inner();
        let response = self
            .gateway
            .suggest_context(
                &self.caps,
                GatewaySuggestContextRequest {
                    file_path: req.file_path,
                    cursor_offset: req.cursor_offset,
                    selection: req.selection,
                    query: req.query,
                    limit: req.limit.map(|v| v as usize),
                },
            )
            .await
            .map_err(status_from_error)?;
        Ok(Response::new(proto::SuggestContextResponse {
            hits: response.hits.into_iter().map(search_hit_to_proto).collect(),
            kg: response.kg.map(kg_to_proto).transpose()?,
            summary: response.summary,
            fingerprint: response.fingerprint,
        }))
    }

    async fn ingest_url(
        &self,
        request: Request<proto::IngestUrlRequest>,
    ) -> Result<Response<proto::IngestUrlResponse>, Status> {
        let req = request.into_inner();
        let response = self
            .gateway
            .ingest_url(
                &self.caps,
                GatewayIngestUrlRequest {
                    url: req.url,
                    tags: req.tags,
                    importance: req.importance,
                },
            )
            .await
            .map_err(status_from_error)?;
        Ok(Response::new(proto::IngestUrlResponse {
            task_id: response.task_id,
            source: response.source,
            source_type: response.source_type,
        }))
    }
}

fn gateway_search_request(req: proto::SearchRequest) -> GatewaySearchRequest {
    GatewaySearchRequest {
        query: req.query,
        limit: req.limit.map(|v| v as usize),
        mode: match proto::SearchMode::try_from(req.mode).unwrap_or(proto::SearchMode::Rrf) {
            proto::SearchMode::Hybrid => GatewaySearchMode::Hybrid,
            proto::SearchMode::Hyde => GatewaySearchMode::Hyde,
            proto::SearchMode::Rrf => GatewaySearchMode::Rrf,
        },
    }
}

fn gateway_recent_request(req: proto::RecentRequest) -> GatewayRecentRequest {
    GatewayRecentRequest {
        limit: req.limit.map(|v| v as usize),
        kind: req.kind,
        tag: req.tag,
        since: req.since,
    }
}

fn gateway_kg_request(req: proto::KgRequest) -> GatewayKgRequest {
    GatewayKgRequest {
        id: req.id,
        depth: if req.depth == 0 {
            1
        } else {
            req.depth.min(u32::from(u8::MAX)) as u8
        },
        direction: if req.direction.trim().is_empty() {
            "both".to_string()
        } else {
            req.direction
        },
    }
}

fn search_hit_to_proto(hit: GatewaySearchHit) -> proto::SearchHit {
    proto::SearchHit {
        id: hit.id,
        content: hit.content,
        tags: hit.tags,
        importance: hit.importance,
        score: hit.score,
        source_url: hit.source_url,
        tier: hit.tier,
    }
}

fn memory_entry_to_proto(entry: StoreMemoryEntry) -> proto::MemoryEntry {
    proto::MemoryEntry {
        id: entry.id,
        content: entry.content,
        tags: entry.tags,
        importance: entry.importance,
        memory_type: entry.memory_type.as_str().to_string(),
        created_at: entry.created_at,
        last_accessed: entry.last_accessed,
        access_count: entry.access_count,
        tier: entry.tier.as_str().to_string(),
        decay_score: entry.decay_score,
        session_id: entry.session_id,
        parent_id: entry.parent_id,
        token_count: entry.token_count,
        source_url: entry.source_url,
        source_hash: entry.source_hash,
        expires_at: entry.expires_at,
        valid_to: entry.valid_to,
    }
}

fn kg_to_proto(
    kg: crate::ai_integrations::gateway::KgNeighborhood,
) -> Result<proto::KgNeighborhood, Status> {
    let neighbors = kg
        .neighbors
        .into_iter()
        .map(|n| {
            serde_json::to_string(&n.edge)
                .map(|edge_json| proto::KgNeighbor {
                    edge_json,
                    entry: n.entry.map(memory_entry_to_proto),
                })
                .map_err(|e| Status::internal(format!("serialize edge: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(proto::KgNeighborhood {
        center: Some(memory_entry_to_proto(kg.center)),
        neighbors,
        truncated: kg.truncated,
    })
}

fn status_from_error(err: GatewayError) -> Status {
    match err {
        GatewayError::PermissionDenied(_) => Status::permission_denied(err.to_string()),
        GatewayError::NotConfigured(_) => Status::failed_precondition(err.to_string()),
        GatewayError::InvalidArgument(_) => Status::invalid_argument(err.to_string()),
        GatewayError::NotFound(_) => Status::not_found(err.to_string()),
        GatewayError::Storage(_) | GatewayError::Internal(_) => Status::internal(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_mode_defaults_to_rrf_for_unknown_wire_value() {
        let req = gateway_search_request(proto::SearchRequest {
            query: "hello".to_string(),
            limit: Some(7),
            mode: 999,
        });
        assert_eq!(req.mode, GatewaySearchMode::Rrf);
        assert_eq!(req.limit, Some(7));
    }

    #[test]
    fn empty_kg_direction_defaults_to_both_and_depth_to_one() {
        let req = gateway_kg_request(proto::KgRequest {
            id: 42,
            depth: 0,
            direction: "  ".to_string(),
        });
        assert_eq!(req.id, 42);
        assert_eq!(req.depth, 1);
        assert_eq!(req.direction, "both");
    }

    #[test]
    fn gateway_errors_map_to_grpc_status_codes() {
        assert_eq!(
            status_from_error(GatewayError::PermissionDenied("brain_read")).code(),
            tonic::Code::PermissionDenied,
        );
        assert_eq!(
            status_from_error(GatewayError::InvalidArgument("bad".to_string())).code(),
            tonic::Code::InvalidArgument,
        );
        assert_eq!(
            status_from_error(GatewayError::NotFound("missing".to_string())).code(),
            tonic::Code::NotFound,
        );
    }

    #[test]
    fn tls_config_accepts_mtls_material() {
        let cfg = tls_config_from_pem(
            b"-----BEGIN CERTIFICATE-----\n-----END CERTIFICATE-----\n".as_slice(),
            b"-----BEGIN PRIVATE KEY-----\n-----END PRIVATE KEY-----\n".as_slice(),
            Some(b"-----BEGIN CERTIFICATE-----\n-----END CERTIFICATE-----\n".as_slice()),
        );
        let debug = format!("{cfg:?}");
        assert_eq!(debug, "ServerTlsConfig");
    }
}
