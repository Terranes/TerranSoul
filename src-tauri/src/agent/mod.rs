pub mod stub_agent;

use async_trait::async_trait;
use stub_agent::Sentiment;

#[async_trait]
pub trait AgentProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    async fn respond(&self, message: &str) -> (String, Sentiment);
    async fn health_check(&self) -> bool;
}
