//! Job distribution — dispatches work items to local or remote workers.
//!
//! A `JobDispatcher` holds a list of advertised local capabilities and can:
//! - Execute a job locally if capabilities match ("self-job")
//! - Submit to the hive relay for remote execution otherwise
//!
//! Workers can also poll the relay for jobs they can fulfil.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::protocol::{Bundle, Capability, EdgeDelta, JobSpec, MemoryDelta};

#[cfg(test)]
use super::protocol::ShareScope;

/// Result of a completed job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: String,
    pub worker_id: String,
    /// Result memories produced by the job.
    pub memory_deltas: Vec<MemoryDelta>,
    /// Result edges produced by the job.
    pub edge_deltas: Vec<EdgeDelta>,
}

impl JobResult {
    /// Convert into a Bundle for transmission.
    pub fn into_bundle(self, hlc_from: u64, hlc_to: u64) -> Bundle {
        Bundle {
            bundle_id: format!("job-result-{}", self.job_id),
            hlc_from,
            hlc_to,
            memory_deltas: self.memory_deltas,
            edge_deltas: self.edge_deltas,
        }
    }
}

/// Capability matcher — checks if a set of advertised capabilities satisfies
/// a job's requirements.
pub fn capabilities_match(advertised: &[Capability], required: &[Capability]) -> bool {
    let available: HashSet<(&str, &str)> = advertised
        .iter()
        .map(|c| (c.kind.as_str(), c.value.as_str()))
        .collect();

    required
        .iter()
        .all(|r| available.contains(&(r.kind.as_str(), r.value.as_str())))
}

/// A local job handler — trait for executing jobs on this device.
#[async_trait::async_trait]
pub trait JobHandler: Send + Sync {
    /// The job type this handler can process (e.g., "embed_chunks").
    fn job_type(&self) -> &str;

    /// Required capabilities for this handler to operate.
    fn capabilities(&self) -> Vec<Capability>;

    /// Execute the job and return results.
    async fn execute(&self, input: &[u8]) -> Result<JobResult, String>;
}

/// Job dispatcher — routes jobs locally or to the relay.
pub struct JobDispatcher {
    device_id: String,
    local_capabilities: Vec<Capability>,
    handlers: Vec<Box<dyn JobHandler>>,
}

impl JobDispatcher {
    /// Create a new dispatcher for the given device.
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            local_capabilities: Vec::new(),
            handlers: Vec::new(),
        }
    }

    /// Register a local job handler and advertise its capabilities.
    pub fn register_handler(&mut self, handler: Box<dyn JobHandler>) {
        for cap in handler.capabilities() {
            if !self.local_capabilities.contains(&cap) {
                self.local_capabilities.push(cap);
            }
        }
        self.handlers.push(handler);
    }

    /// All capabilities this device advertises.
    pub fn advertised_capabilities(&self) -> &[Capability] {
        &self.local_capabilities
    }

    /// Check if a job can be executed locally.
    pub fn can_execute_locally(&self, job: &JobSpec) -> bool {
        // Must have a handler for the job type AND satisfy all capability requirements
        let has_handler = self.handlers.iter().any(|h| h.job_type() == job.job_type);
        has_handler && capabilities_match(&self.local_capabilities, &job.capabilities)
    }

    /// Execute a job locally (self-job). Returns `Err` if no local handler matches.
    pub async fn execute_local(&self, job: &JobSpec) -> Result<JobResult, String> {
        let handler = self
            .handlers
            .iter()
            .find(|h| h.job_type() == job.job_type)
            .ok_or_else(|| format!("No local handler for job type '{}'", job.job_type))?;

        if !capabilities_match(&self.local_capabilities, &job.capabilities) {
            return Err("Local capabilities do not satisfy job requirements".into());
        }

        handler.execute(&job.input).await
    }

    /// Decide routing: local execution or remote dispatch.
    ///
    /// Returns `Ok(Some(result))` for local execution,
    /// `Ok(None)` if the job should be sent to the relay.
    pub async fn dispatch(&self, job: &JobSpec) -> Result<Option<JobResult>, String> {
        if self.can_execute_locally(job) {
            let result = self.execute_local(job).await?;
            Ok(Some(result))
        } else {
            // Job should be submitted to relay for remote execution
            Ok(None)
        }
    }

    /// The device ID for this dispatcher.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}

/// A synthetic embedding job handler for testing.
/// In production, this would call the actual Ollama embedding API.
#[cfg(test)]
pub struct MockEmbedHandler {
    device_id: String,
}

#[cfg(test)]
impl MockEmbedHandler {
    pub fn new(device_id: &str) -> Self {
        Self {
            device_id: device_id.to_string(),
        }
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl JobHandler for MockEmbedHandler {
    fn job_type(&self) -> &str {
        "embed_chunks"
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability {
            kind: "embedding_model".into(),
            value: "nomic-embed-text".into(),
        }]
    }

    async fn execute(&self, input: &[u8]) -> Result<JobResult, String> {
        // Parse input as a list of (content_hash, text) pairs
        let chunks: Vec<(String, String)> =
            serde_json::from_slice(input).map_err(|e| format!("Invalid input: {e}"))?;

        // Produce synthetic memory deltas (in real impl, these would be embedding vectors)
        let memory_deltas: Vec<MemoryDelta> = chunks
            .into_iter()
            .map(|(hash, _text)| MemoryDelta {
                content_hash: hash,
                operation: "upsert".into(),
                content: "embedded".into(),
                tags: "embedding".into(),
                importance: 3,
                memory_type: "fact".into(),
                cognitive_kind: None,
                created_at: 0,
                updated_at: 0,
                hlc_counter: 0,
                origin_device: self.device_id.clone(),
                share_scope: ShareScope::Hive,
                source_url: None,
                source_hash: None,
                context_prefix: None,
                valid_to: None,
            })
            .collect();

        Ok(JobResult {
            job_id: "test-job".into(),
            worker_id: self.device_id.clone(),
            memory_deltas,
            edge_deltas: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_match_exact() {
        let advertised = vec![
            Capability {
                kind: "embedding_model".into(),
                value: "nomic-embed-text".into(),
            },
            Capability {
                kind: "gpu".into(),
                value: "cuda".into(),
            },
        ];
        let required = vec![Capability {
            kind: "embedding_model".into(),
            value: "nomic-embed-text".into(),
        }];
        assert!(capabilities_match(&advertised, &required));
    }

    #[test]
    fn capabilities_match_subset() {
        let advertised = vec![
            Capability {
                kind: "embedding_model".into(),
                value: "nomic-embed-text".into(),
            },
            Capability {
                kind: "brain_mode".into(),
                value: "ollama".into(),
            },
        ];
        let required = vec![
            Capability {
                kind: "embedding_model".into(),
                value: "nomic-embed-text".into(),
            },
            Capability {
                kind: "brain_mode".into(),
                value: "ollama".into(),
            },
        ];
        assert!(capabilities_match(&advertised, &required));
    }

    #[test]
    fn capabilities_mismatch() {
        let advertised = vec![Capability {
            kind: "embedding_model".into(),
            value: "nomic-embed-text".into(),
        }];
        let required = vec![Capability {
            kind: "gpu".into(),
            value: "cuda".into(),
        }];
        assert!(!capabilities_match(&advertised, &required));
    }

    #[test]
    fn empty_requirements_always_match() {
        let advertised = vec![Capability {
            kind: "anything".into(),
            value: "value".into(),
        }];
        assert!(capabilities_match(&advertised, &[]));
    }

    #[test]
    fn empty_advertised_fails_nonempty_requirements() {
        let required = vec![Capability {
            kind: "gpu".into(),
            value: "cuda".into(),
        }];
        assert!(!capabilities_match(&[], &required));
    }

    #[tokio::test]
    async fn dispatcher_routes_locally_when_capable() {
        let mut dispatcher = JobDispatcher::new("device-a".into());
        dispatcher.register_handler(Box::new(MockEmbedHandler::new("device-a")));

        let job = JobSpec {
            job_id: "job-001".into(),
            job_type: "embed_chunks".into(),
            capabilities: vec![Capability {
                kind: "embedding_model".into(),
                value: "nomic-embed-text".into(),
            }],
            input: serde_json::to_vec(&vec![("hash1".to_string(), "hello".to_string())]).unwrap(),
            timeout_ms: 30_000,
            max_retries: 3,
        };

        assert!(dispatcher.can_execute_locally(&job));
        let result = dispatcher.dispatch(&job).await.unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.memory_deltas.len(), 1);
        assert_eq!(result.memory_deltas[0].content_hash, "hash1");
    }

    #[tokio::test]
    async fn dispatcher_routes_to_relay_when_not_capable() {
        let dispatcher = JobDispatcher::new("device-b".into());

        let job = JobSpec {
            job_id: "job-002".into(),
            job_type: "embed_chunks".into(),
            capabilities: vec![Capability {
                kind: "embedding_model".into(),
                value: "nomic-embed-text".into(),
            }],
            input: vec![],
            timeout_ms: 30_000,
            max_retries: 3,
        };

        assert!(!dispatcher.can_execute_locally(&job));
        let result = dispatcher.dispatch(&job).await.unwrap();
        assert!(result.is_none()); // Should go to relay
    }

    #[tokio::test]
    async fn job_result_converts_to_bundle() {
        let result = JobResult {
            job_id: "job-003".into(),
            worker_id: "device-a".into(),
            memory_deltas: vec![MemoryDelta {
                content_hash: "abc".into(),
                operation: "upsert".into(),
                content: "test".into(),
                tags: "".into(),
                importance: 3,
                memory_type: "fact".into(),
                cognitive_kind: None,
                created_at: 100,
                updated_at: 100,
                hlc_counter: 50,
                origin_device: "device-a".into(),
                share_scope: ShareScope::Hive,
                source_url: None,
                source_hash: None,
                context_prefix: None,
                valid_to: None,
            }],
            edge_deltas: Vec::new(),
        };

        let bundle = result.into_bundle(10, 50);
        assert_eq!(bundle.bundle_id, "job-result-job-003");
        assert_eq!(bundle.hlc_from, 10);
        assert_eq!(bundle.hlc_to, 50);
        assert_eq!(bundle.memory_deltas.len(), 1);
    }
}
