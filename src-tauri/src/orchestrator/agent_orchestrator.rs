/// Agent orchestrator — routes messages to the appropriate agent.
/// Phase 1 uses only the StubAgent; future phases will plug in real agents here.
use crate::agent::stub_agent::StubAgent;

pub struct AgentOrchestrator;

impl AgentOrchestrator {
    pub fn new() -> Self {
        Self
    }

    pub async fn dispatch(&self, agent_id: &str, message: &str) -> (String, String) {
        let agent = StubAgent::new(agent_id);
        let (content, _sentiment) = agent.respond(message).await;
        (agent.name().to_string(), content)
    }
}

impl Default for AgentOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}
