use async_trait::async_trait;
use tokio::time::{sleep, Duration};

use super::AgentProvider;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Sentiment {
    Happy,
    Neutral,
    Sad,
}

pub struct StubAgent {
    id: String,
}

impl StubAgent {
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }

    fn classify(&self, message: &str) -> (String, Sentiment) {
        let lower = message.to_lowercase();

        if lower.contains("hello") || lower.contains("hi") || lower.starts_with("hey") {
            (
                "Hello! I'm TerranSoul, your AI assistant. How can I help you today?".to_string(),
                Sentiment::Happy,
            )
        } else if lower.contains("sad") || lower.contains("bad") || lower.contains("hate") {
            (
                format!("I'm sorry to hear that. I understand you said: '{}'. I'm here for you!", message),
                Sentiment::Sad,
            )
        } else if lower.contains("happy") || lower.contains("great") || lower.contains("awesome") {
            (
                format!("That's wonderful! You said: '{}'. I'm glad things are going well!", message),
                Sentiment::Happy,
            )
        } else {
            (
                format!("I understand you said: '{}'. I'm still learning, but I'm here to help!", message),
                Sentiment::Neutral,
            )
        }
    }
}

#[async_trait]
impl AgentProvider for StubAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        match self.id.as_str() {
            "stub" => "TerranSoul",
            other => other,
        }
    }

    async fn respond(&self, message: &str) -> (String, Sentiment) {
        // Simulate network / model latency
        let delay_ms = 500 + (message.len() as u64 % 500);
        sleep(Duration::from_millis(delay_ms)).await;

        self.classify(message)
    }

    async fn health_check(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_returns_terransoul_for_stub_id() {
        let agent = StubAgent::new("stub");
        assert_eq!(agent.name(), "TerranSoul");
    }

    #[test]
    fn name_returns_custom_id() {
        let agent = StubAgent::new("custom-agent");
        assert_eq!(agent.name(), "custom-agent");
    }

    #[tokio::test]
    async fn respond_hello_returns_happy() {
        let agent = StubAgent::new("stub");
        let (response, sentiment) = agent.respond("hello").await;
        assert!(response.contains("TerranSoul"));
        assert!(matches!(sentiment, Sentiment::Happy));
    }

    #[tokio::test]
    async fn respond_hi_returns_happy() {
        let agent = StubAgent::new("stub");
        let (response, sentiment) = agent.respond("Hi there").await;
        assert!(response.contains("TerranSoul"));
        assert!(matches!(sentiment, Sentiment::Happy));
    }

    #[tokio::test]
    async fn respond_sad_returns_sad() {
        let agent = StubAgent::new("stub");
        let (response, sentiment) = agent.respond("I am sad today").await;
        assert!(response.contains("I'm sorry"));
        assert!(matches!(sentiment, Sentiment::Sad));
    }

    #[tokio::test]
    async fn respond_happy_returns_happy() {
        let agent = StubAgent::new("stub");
        let (response, sentiment) = agent.respond("I am happy!").await;
        assert!(response.contains("wonderful"));
        assert!(matches!(sentiment, Sentiment::Happy));
    }

    #[tokio::test]
    async fn respond_neutral_returns_neutral() {
        let agent = StubAgent::new("stub");
        let (response, sentiment) = agent.respond("Tell me about the weather").await;
        assert!(response.contains("still learning"));
        assert!(matches!(sentiment, Sentiment::Neutral));
    }
}
