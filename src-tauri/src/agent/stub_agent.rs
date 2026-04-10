use tokio::time::{sleep, Duration};

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

    pub fn name(&self) -> &str {
        match self.id.as_str() {
            "stub" => "TerranSoul",
            other => other,
        }
    }

    pub async fn respond(&self, message: &str) -> (String, Sentiment) {
        // Simulate network / model latency
        let delay_ms = 500 + (message.len() as u64 % 500);
        sleep(Duration::from_millis(delay_ms)).await;

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
