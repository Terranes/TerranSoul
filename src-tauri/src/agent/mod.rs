pub mod openclaw_agent;
pub mod stub_agent;

use async_trait::async_trait;
use stub_agent::Sentiment;

#[async_trait]
pub trait AgentProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    /// Capability tags this agent supports (e.g. `["code", "plan", "review"]`).
    /// Used by the orchestrator for tag-based routing (chunk 33B.6).
    fn capabilities(&self) -> &[String] {
        &[]
    }
    async fn respond(&self, message: &str) -> (String, Sentiment);
    async fn health_check(&self) -> bool;
}
