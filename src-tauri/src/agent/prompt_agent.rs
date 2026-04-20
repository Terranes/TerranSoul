use async_trait::async_trait;

use super::AgentProvider;
use super::stub_agent::Sentiment;

/// A prompt-based agent whose personality is defined entirely by a system prompt.
///
/// Imported from agency-agents or any other prompt collection. When TerranSoul
/// adds real LLM integration, `PromptAgent` will inject `system_prompt` as the
/// system message before every conversation turn. Until then it echoes the prompt
/// header so the UI can confirm which agent is active.
pub struct PromptAgent {
    /// Unique agent id (lowercase, hyphenated, e.g. "engineering-frontend-developer").
    id: String,
    /// Human-readable display name (e.g. "Frontend Developer").
    display_name: String,
    /// Full system prompt content.
    system_prompt: String,
    /// Optional emoji icon.
    emoji: Option<String>,
    /// Division this agent belongs to (e.g. "engineering", "marketing").
    division: Option<String>,
}

impl PromptAgent {
    /// Create a new `PromptAgent`.
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        system_prompt: impl Into<String>,
        emoji: Option<String>,
        division: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            system_prompt: system_prompt.into(),
            emoji,
            division,
        }
    }

    /// Returns the system prompt content for injection into an upstream LLM.
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// Returns the emoji for this agent, if any.
    pub fn emoji(&self) -> Option<&str> {
        self.emoji.as_deref()
    }

    /// Returns the division this agent belongs to, if any.
    pub fn division(&self) -> Option<&str> {
        self.division.as_deref()
    }

    /// Builds a short acknowledgement message shown while LLM integration is pending.
    fn activation_message(&self) -> String {
        let icon = self.emoji.as_deref().unwrap_or("🤖");
        let div = self
            .division
            .as_deref()
            .map(|d| format!(" ({} division)", d))
            .unwrap_or_default();
        format!(
            "{icon} **{}**{div} is active.\n\n\
             I'm ready to help. My full expertise is loaded — once TerranSoul's \
             LLM integration is wired up I'll respond in character. \
             What would you like to work on?",
            self.display_name
        )
    }
}

#[async_trait]
impl AgentProvider for PromptAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.display_name
    }

    async fn respond(&self, message: &str) -> (String, Sentiment) {
        // Phase 1: LLM not yet integrated — acknowledge activation or echo back.
        // TODO(Phase 2): replace with real LLM call using `self.system_prompt` as
        // the system message and `message` as the user turn.
        let lower = message.to_lowercase();
        if lower.contains("hello")
            || lower.contains("hi")
            || lower.starts_with("hey")
            || lower.contains("activate")
            || lower.contains("start")
        {
            (self.activation_message(), Sentiment::Happy)
        } else {
            let icon = self.emoji.as_deref().unwrap_or("🤖");
            (
                format!(
                    "{icon} **{}** received: \"{}\"\n\n\
                     *(LLM integration coming in Phase 2 — \
                     the system prompt for this agent is loaded and ready.)*",
                    self.display_name, message
                ),
                Sentiment::Neutral,
            )
        }
    }

    async fn health_check(&self) -> bool {
        // Prompt agents are always healthy — no external process to ping.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent() -> PromptAgent {
        PromptAgent::new(
            "engineering-frontend-developer",
            "Frontend Developer",
            "You are a frontend developer expert...",
            Some("🖥️".to_string()),
            Some("engineering".to_string()),
        )
    }

    #[test]
    fn id_returns_correct_value() {
        let agent = make_agent();
        assert_eq!(agent.id(), "engineering-frontend-developer");
    }

    #[test]
    fn name_returns_display_name() {
        let agent = make_agent();
        assert_eq!(agent.name(), "Frontend Developer");
    }

    #[test]
    fn system_prompt_is_stored() {
        let agent = make_agent();
        assert!(agent.system_prompt().contains("frontend developer"));
    }

    #[test]
    fn emoji_is_returned() {
        let agent = make_agent();
        assert_eq!(agent.emoji(), Some("🖥️"));
    }

    #[test]
    fn division_is_returned() {
        let agent = make_agent();
        assert_eq!(agent.division(), Some("engineering"));
    }

    #[tokio::test]
    async fn respond_hello_returns_happy() {
        let agent = make_agent();
        let (msg, sentiment) = agent.respond("hello").await;
        assert!(matches!(sentiment, Sentiment::Happy));
        assert!(msg.contains("Frontend Developer"));
    }

    #[tokio::test]
    async fn respond_unknown_returns_neutral() {
        let agent = make_agent();
        let (msg, sentiment) = agent.respond("build me a react component").await;
        assert!(matches!(sentiment, Sentiment::Neutral));
        assert!(msg.contains("Frontend Developer"));
    }

    #[tokio::test]
    async fn health_check_always_true() {
        let agent = make_agent();
        assert!(agent.health_check().await);
    }
}
