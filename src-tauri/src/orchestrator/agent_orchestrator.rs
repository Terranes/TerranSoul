/// Agent orchestrator — routes messages to the appropriate agent.
/// Uses the `AgentProvider` trait for pluggable agent implementations.
use std::collections::HashMap;
use std::sync::Arc;

use crate::agent::stub_agent::StubAgent;
use crate::agent::AgentProvider;

pub struct AgentOrchestrator {
    agents: HashMap<String, Arc<dyn AgentProvider>>,
    default_agent_id: String,
}

impl AgentOrchestrator {
    pub fn new() -> Self {
        let mut agents: HashMap<String, Arc<dyn AgentProvider>> = HashMap::new();
        let stub = Arc::new(StubAgent::new("stub"));
        agents.insert("stub".to_string(), stub);

        Self {
            agents,
            default_agent_id: "stub".to_string(),
        }
    }

    pub fn register(&mut self, agent: Arc<dyn AgentProvider>) {
        self.agents.insert(agent.id().to_string(), agent);
    }

    pub fn get_agent(&self, agent_id: &str) -> Option<Arc<dyn AgentProvider>> {
        self.agents.get(agent_id).cloned()
    }

    pub async fn dispatch(
        &self,
        agent_id: &str,
        message: &str,
    ) -> Result<(String, String), String> {
        let id = if agent_id.is_empty() || agent_id == "auto" {
            &self.default_agent_id
        } else {
            agent_id
        };

        let agent = self
            .agents
            .get(id)
            .ok_or_else(|| format!("Agent '{}' not found", id))?;

        let (content, _sentiment) = agent.respond(message).await;
        Ok((agent.name().to_string(), content))
    }

    pub async fn health_check(&self, agent_id: &str) -> Result<bool, String> {
        let agent = self
            .agents
            .get(agent_id)
            .ok_or_else(|| format!("Agent '{}' not found", agent_id))?;
        Ok(agent.health_check().await)
    }

    pub fn list_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    /// Find agents whose capability tags contain **all** of the `required` tags.
    /// Returns agent IDs in arbitrary order. If no agent matches, the list is empty.
    pub fn agents_with_capabilities(&self, required: &[&str]) -> Vec<String> {
        self.agents
            .iter()
            .filter(|(_, agent)| {
                let caps = agent.capabilities();
                required.iter().all(|req| caps.iter().any(|c| c == req))
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Dispatch a message to the first agent that satisfies the `required` capability tags.
    /// Falls back to the default agent if none match.
    pub async fn dispatch_by_capability(
        &self,
        required: &[&str],
        message: &str,
    ) -> Result<(String, String), String> {
        let matching = self.agents_with_capabilities(required);
        let agent_id = matching
            .first()
            .map(|s| s.as_str())
            .unwrap_or(&self.default_agent_id);
        self.dispatch(agent_id, message).await
    }
}

impl Default for AgentOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::stub_agent::Sentiment;
    use async_trait::async_trait;

    struct MockAgent {
        agent_id: String,
        agent_name: String,
        response: String,
        healthy: bool,
        caps: Vec<String>,
    }

    #[async_trait]
    impl AgentProvider for MockAgent {
        fn id(&self) -> &str {
            &self.agent_id
        }
        fn name(&self) -> &str {
            &self.agent_name
        }
        fn capabilities(&self) -> &[String] {
            &self.caps
        }
        async fn respond(&self, _message: &str) -> (String, Sentiment) {
            (self.response.clone(), Sentiment::Neutral)
        }
        async fn health_check(&self) -> bool {
            self.healthy
        }
    }

    #[tokio::test]
    async fn dispatch_default_agent() {
        let orchestrator = AgentOrchestrator::new();
        let result = orchestrator.dispatch("auto", "hello").await;
        assert!(result.is_ok());
        let (name, content) = result.unwrap();
        assert_eq!(name, "TerranSoul");
        assert!(!content.is_empty());
    }

    #[tokio::test]
    async fn dispatch_unknown_agent_returns_error() {
        let orchestrator = AgentOrchestrator::new();
        let result = orchestrator.dispatch("nonexistent", "hello").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn dispatch_registered_agent() {
        let mut orchestrator = AgentOrchestrator::new();
        orchestrator.register(Arc::new(MockAgent {
            agent_id: "mock".to_string(),
            agent_name: "MockBot".to_string(),
            response: "Mock response".to_string(),
            healthy: true,
            caps: vec![],
        }));
        let result = orchestrator.dispatch("mock", "test").await;
        assert!(result.is_ok());
        let (name, content) = result.unwrap();
        assert_eq!(name, "MockBot");
        assert_eq!(content, "Mock response");
    }

    #[tokio::test]
    async fn health_check_healthy_agent() {
        let mut orchestrator = AgentOrchestrator::new();
        orchestrator.register(Arc::new(MockAgent {
            agent_id: "healthy".to_string(),
            agent_name: "Healthy".to_string(),
            response: String::new(),
            healthy: true,
            caps: vec![],
        }));
        let result = orchestrator.health_check("healthy").await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn health_check_unhealthy_agent() {
        let mut orchestrator = AgentOrchestrator::new();
        orchestrator.register(Arc::new(MockAgent {
            agent_id: "unhealthy".to_string(),
            agent_name: "Unhealthy".to_string(),
            response: String::new(),
            healthy: false,
            caps: vec![],
        }));
        let result = orchestrator.health_check("unhealthy").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn health_check_missing_agent() {
        let orchestrator = AgentOrchestrator::new();
        let result = orchestrator.health_check("missing").await;
        assert!(result.is_err());
    }

    #[test]
    fn list_agents_includes_default() {
        let orchestrator = AgentOrchestrator::new();
        let agents = orchestrator.list_agents();
        assert!(agents.contains(&"stub".to_string()));
    }

    #[test]
    fn list_agents_includes_registered() {
        let mut orchestrator = AgentOrchestrator::new();
        orchestrator.register(Arc::new(MockAgent {
            agent_id: "custom".to_string(),
            agent_name: "Custom".to_string(),
            response: String::new(),
            healthy: true,
            caps: vec![],
        }));
        let agents = orchestrator.list_agents();
        assert!(agents.contains(&"stub".to_string()));
        assert!(agents.contains(&"custom".to_string()));
    }

    #[test]
    fn agents_with_capabilities_empty_when_no_match() {
        let orchestrator = AgentOrchestrator::new();
        let result = orchestrator.agents_with_capabilities(&["code", "plan"]);
        assert!(result.is_empty());
    }

    #[test]
    fn agents_with_capabilities_finds_matching_agent() {
        let mut orchestrator = AgentOrchestrator::new();
        orchestrator.register(Arc::new(MockAgent {
            agent_id: "coder".to_string(),
            agent_name: "Coder".to_string(),
            response: String::new(),
            healthy: true,
            caps: vec!["code".into(), "plan".into(), "implement".into()],
        }));
        let result = orchestrator.agents_with_capabilities(&["code", "plan"]);
        assert_eq!(result, vec!["coder"]);
    }

    #[test]
    fn agents_with_capabilities_requires_all_tags() {
        let mut orchestrator = AgentOrchestrator::new();
        orchestrator.register(Arc::new(MockAgent {
            agent_id: "partial".to_string(),
            agent_name: "Partial".to_string(),
            response: String::new(),
            healthy: true,
            caps: vec!["code".into()], // missing "review"
        }));
        let result = orchestrator.agents_with_capabilities(&["code", "review"]);
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn dispatch_by_capability_uses_matching_agent() {
        let mut orchestrator = AgentOrchestrator::new();
        orchestrator.register(Arc::new(MockAgent {
            agent_id: "reviewer".to_string(),
            agent_name: "Reviewer".to_string(),
            response: "review done".to_string(),
            healthy: true,
            caps: vec!["code".into(), "review".into()],
        }));
        let (name, content) = orchestrator
            .dispatch_by_capability(&["code", "review"], "check this")
            .await
            .unwrap();
        assert_eq!(name, "Reviewer");
        assert_eq!(content, "review done");
    }

    #[tokio::test]
    async fn dispatch_by_capability_falls_back_to_default() {
        let orchestrator = AgentOrchestrator::new();
        let (name, _) = orchestrator
            .dispatch_by_capability(&["nonexistent"], "hello")
            .await
            .unwrap();
        assert_eq!(name, "TerranSoul");
    }
}
