//! gRPC service implementation for the Hive relay.

use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

use crate::db::RelayDb;
use crate::proto::hive_relay_server::HiveRelay;
use crate::proto::{
    ClaimJobRequest, ClaimJobResponse, CompleteJobRequest, Empty, HealthResponse, HiveEnvelope,
    SubmitResponse, SubscribeRequest,
};
use crate::verify::verify_envelope;

/// The relay service state.
pub struct HiveRelayService {
    db: RelayDb,
    /// Broadcast channel for real-time envelope delivery to subscribers.
    tx: broadcast::Sender<HiveEnvelope>,
}

impl HiveRelayService {
    /// Create a new relay service.
    pub fn new(db: RelayDb) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { db, tx }
    }

    /// Get an Arc-wrapped instance for use with Tonic.
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

type StreamResult = Pin<Box<dyn tokio_stream::Stream<Item = Result<HiveEnvelope, Status>> + Send>>;

#[tonic::async_trait]
impl HiveRelay for Arc<HiveRelayService> {
    async fn submit(
        &self,
        request: Request<HiveEnvelope>,
    ) -> Result<Response<SubmitResponse>, Status> {
        let envelope = request.into_inner();

        // 1. Verify Ed25519 signature
        verify_envelope(&envelope).map_err(Status::unauthenticated)?;

        // 2. Replay protection — check HLC watermark
        let current_watermark = self
            .db
            .get_hlc_watermark(&envelope.sender_id)
            .await
            .map_err(|e| Status::internal(format!("DB error: {e}")))?;

        if envelope.hlc_counter <= current_watermark {
            return Ok(Response::new(SubmitResponse {
                accepted: false,
                error: format!(
                    "Replay rejected: hlc {} <= watermark {}",
                    envelope.hlc_counter, current_watermark
                ),
            }));
        }

        // 3. Update HLC watermark
        self.db
            .update_hlc_watermark(&envelope.sender_id, envelope.hlc_counter)
            .await
            .map_err(|e| Status::internal(format!("DB error: {e}")))?;

        // 4. Handle by message type
        let msg_type = envelope.msg_type;
        match msg_type {
            0 => {
                // BUNDLE — persist
                // Extract bundle_id from payload (best-effort; use hlc as fallback key)
                let bundle_id = format!("{}-{}", envelope.sender_id, envelope.hlc_counter);
                self.db
                    .store_bundle(
                        &envelope.sender_id,
                        &bundle_id,
                        envelope.hlc_counter,
                        &envelope.payload,
                        &envelope.signature,
                    )
                    .await
                    .map_err(|e| Status::internal(format!("DB error: {e}")))?;
            }
            1 => {
                // OP — ephemeral, just broadcast
            }
            2 => {
                // JOB — enqueue
                let job_id = format!("job-{}-{}", envelope.sender_id, envelope.hlc_counter);
                self.db
                    .enqueue_job(
                        &job_id,
                        &envelope.sender_id,
                        &envelope.payload,
                        &envelope.signature,
                    )
                    .await
                    .map_err(|e| Status::internal(format!("DB error: {e}")))?;
            }
            _ => {
                return Ok(Response::new(SubmitResponse {
                    accepted: false,
                    error: format!("Unknown msg_type: {msg_type}"),
                }));
            }
        }

        // 5. Broadcast to all subscribers
        let _ = self.tx.send(envelope);

        Ok(Response::new(SubmitResponse {
            accepted: true,
            error: String::new(),
        }))
    }

    type SubscribeStream = StreamResult;

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let req = request.into_inner();
        let since_hlc = req.since_hlc;

        // Send historical bundles first, then switch to live stream
        let historical = self
            .db
            .get_bundles_since(since_hlc, 1000)
            .await
            .map_err(|e| Status::internal(format!("DB error: {e}")))?;

        let historical_envelopes: Vec<HiveEnvelope> = historical
            .into_iter()
            .map(|b| HiveEnvelope {
                version: 1,
                msg_type: 0, // BUNDLE
                sender_id: b.sender_id,
                sender_pubkey: Vec::new(), // Not stored separately; verifiable from signature
                timestamp: 0,
                hlc_counter: b.hlc_counter as u64,
                payload: b.payload,
                signature: b.signature,
                compressed: false,
            })
            .collect();

        let rx = self.tx.subscribe();
        let live_stream = BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(envelope) => Some(Ok(envelope)),
            Err(_) => None, // Lagged — skip missed messages
        });

        let historical_stream = tokio_stream::iter(historical_envelopes.into_iter().map(Ok));
        let combined = historical_stream.chain(live_stream);

        Ok(Response::new(Box::pin(combined)))
    }

    async fn claim_job(
        &self,
        request: Request<ClaimJobRequest>,
    ) -> Result<Response<ClaimJobResponse>, Status> {
        let req = request.into_inner();

        let job = self
            .db
            .claim_job(&req.worker_id)
            .await
            .map_err(|e| Status::internal(format!("DB error: {e}")))?;

        match job {
            Some(stored) => {
                let envelope = HiveEnvelope {
                    version: 1,
                    msg_type: 2, // JOB
                    sender_id: stored.sender_id,
                    sender_pubkey: Vec::new(),
                    timestamp: 0,
                    hlc_counter: 0,
                    payload: stored.payload,
                    signature: stored.signature,
                    compressed: false,
                };
                Ok(Response::new(ClaimJobResponse {
                    found: true,
                    job_envelope: Some(envelope),
                }))
            }
            None => Ok(Response::new(ClaimJobResponse {
                found: false,
                job_envelope: None,
            })),
        }
    }

    async fn complete_job(
        &self,
        request: Request<CompleteJobRequest>,
    ) -> Result<Response<SubmitResponse>, Status> {
        let req = request.into_inner();

        let completed = self
            .db
            .complete_job(&req.job_id, &req.worker_id)
            .await
            .map_err(|e| Status::internal(format!("DB error: {e}")))?;

        if !completed {
            return Ok(Response::new(SubmitResponse {
                accepted: false,
                error: "Job not found or not claimed by this worker".into(),
            }));
        }

        // Forward the result bundle to subscribers
        if let Some(result_envelope) = req.result_envelope {
            let _ = self.tx.send(result_envelope);
        }

        Ok(Response::new(SubmitResponse {
            accepted: true,
            error: String::new(),
        }))
    }

    async fn health(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<HealthResponse>, Status> {
        let pending = self
            .db
            .pending_job_count()
            .await
            .map_err(|e| Status::internal(format!("DB error: {e}")))?;

        Ok(Response::new(HealthResponse {
            version: "0.1.0".into(),
            connected_devices: self.tx.receiver_count() as u64,
            pending_jobs: pending,
        }))
    }
}
