use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::agent::stub_agent::Sentiment;
use crate::agent::AgentProvider;

pub const OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434";

/// System prompt injected into every Ollama conversation.
///
/// Gives the brain its TerranSoul identity and knowledge of available packages.
const SYSTEM_PROMPT: &str = r#"You are TerranSoul, a friendly AI companion with a 3D character avatar. You live inside the TerranSoul desktop app and serve as the user's intelligent assistant.

Your capabilities:
- Helpful conversation and answering questions on any topic
- Recommending AI tools and software based on the user's needs
- Guiding users through installing packages via the TerranSoul Package Manager

Available packages you can recommend:
- **OpenClaw** (model tag: "openclaw-bridge"): An open-source AI interface that connects to powerful language model APIs. Great for users who want cloud-based AI alongside local models.
- **Claude Cowork** (model tag: "claude-cowork"): A collaborative AI workspace powered by Anthropic's Claude. Perfect for document analysis, long-context reasoning, and team workflows.
- **stub-agent**: The built-in lightweight agent. Always available offline.

When recommending a package, mention its name and briefly explain why it suits the user's request. Keep responses concise and warm. You can suggest that the user visit the Package Manager tab to install recommended tools."#;

/// Request body for Ollama's chat completion endpoint.
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

/// A single message in an Ollama conversation.
#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Response from Ollama's /api/chat endpoint (non-streaming).
#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

/// Response from Ollama's /api/tags endpoint.
#[derive(Deserialize)]
pub struct TagsResponse {
    pub models: Vec<OllamaModelEntry>,
}

/// Entry in Ollama's model list.
#[derive(Deserialize, Serialize, Clone)]
pub struct OllamaModelEntry {
    pub name: String,
    pub size: u64,
}

/// Status of the local Ollama service.
#[derive(Serialize)]
pub struct OllamaStatus {
    pub running: bool,
    pub model_count: usize,
}

/// An AI agent backed by a locally running Ollama language model.
///
/// Routes all messages to the specified Ollama model via the REST API.
/// Falls back to a helpful error message when Ollama is unreachable.
pub struct OllamaAgent {
    model: String,
    base_url: String,
    client: Client,
}

impl OllamaAgent {
    /// Create an agent that talks to the given Ollama model.
    pub fn new(model: &str) -> Self {
        Self::with_url(model, OLLAMA_BASE_URL)
    }

    /// Create an agent with a custom Ollama base URL (useful for tests).
    pub fn with_url(model: &str, base_url: &str) -> Self {
        OllamaAgent {
            model: model.to_string(),
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    fn chat_url(&self) -> String {
        format!("{}/api/chat", self.base_url)
    }

    fn tags_url(&self) -> String {
        format!("{}/api/tags", self.base_url)
    }

    /// Infer a simple sentiment from the response text.
    fn infer_sentiment(text: &str) -> Sentiment {
        let lower = text.to_lowercase();
        if lower.contains("sorry")
            || lower.contains("unfortunate")
            || lower.contains("can't help")
            || lower.contains("cannot help")
        {
            Sentiment::Sad
        } else if lower.contains("great")
            || lower.contains("excellent")
            || lower.contains("happy to")
            || lower.contains("glad to")
            || lower.contains("wonderful")
            || lower.contains("sure!")
            || lower.contains("of course!")
        {
            Sentiment::Happy
        } else {
            Sentiment::Neutral
        }
    }
}

#[async_trait]
impl AgentProvider for OllamaAgent {
    fn id(&self) -> &str {
        "ollama"
    }

    fn name(&self) -> &str {
        &self.model
    }

    async fn respond(&self, message: &str) -> (String, Sentiment) {
        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: message.to_string(),
                },
            ],
            stream: false,
        };

        match self
            .client
            .post(&self.chat_url())
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<ChatResponse>().await {
                    Ok(chat) => {
                        let sentiment = Self::infer_sentiment(&chat.message.content);
                        (chat.message.content, sentiment)
                    }
                    Err(e) => (
                        format!("I received a response but couldn't parse it ({}). Please try again.", e),
                        Sentiment::Neutral,
                    ),
                }
            }
            Ok(resp) => {
                let status = resp.status();
                (
                    format!(
                        "My brain returned an error (HTTP {}). Is the model '{}' installed? Try: `ollama pull {}`",
                        status, self.model, self.model
                    ),
                    Sentiment::Sad,
                )
            }
            Err(_) => (
                "My brain (Ollama) is not reachable right now. Please make sure Ollama is running: https://ollama.ai".to_string(),
                Sentiment::Sad,
            ),
        }
    }

    async fn health_check(&self) -> bool {
        self.client
            .get(&self.tags_url())
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

/// Check whether the local Ollama service is running.
pub async fn check_status(client: &Client, base_url: &str) -> OllamaStatus {
    let url = format!("{base_url}/api/tags");
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let model_count = resp
                .json::<TagsResponse>()
                .await
                .map(|t| t.models.len())
                .unwrap_or(0);
            OllamaStatus {
                running: true,
                model_count,
            }
        }
        _ => OllamaStatus {
            running: false,
            model_count: 0,
        },
    }
}

/// List all locally installed Ollama models.
pub async fn list_models(client: &Client, base_url: &str) -> Vec<OllamaModelEntry> {
    let url = format!("{base_url}/api/tags");
    client
        .get(&url)
        .send()
        .await
        .ok()
        .and_then(|r| {
            if r.status().is_success() {
                Some(r)
            } else {
                None
            }
        })
        .and_then(|r| futures_util::executor::block_on(r.json::<TagsResponse>()).ok())
        .map(|t| t.models)
        .unwrap_or_default()
}

/// Pull an Ollama model, consuming the streaming response.
/// Resolves when the download is complete or on error.
pub async fn pull_model(client: &Client, base_url: &str, model_name: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct PullRequest<'a> {
        name: &'a str,
        stream: bool,
    }

    let url = format!("{base_url}/api/pull");
    let resp = client
        .post(&url)
        .json(&PullRequest {
            name: model_name,
            stream: true,
        })
        .send()
        .await
        .map_err(|e| format!("Ollama not reachable: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Ollama pull failed with status {}",
            resp.status()
        ));
    }

    // Drain the streaming response (each line is a JSON progress update).
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        chunk.map_err(|e| format!("stream error: {e}"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ollama_agent_id() {
        let agent = OllamaAgent::new("gemma3:4b");
        assert_eq!(agent.id(), "ollama");
    }

    #[test]
    fn ollama_agent_name_is_model() {
        let agent = OllamaAgent::new("gemma3:4b");
        assert_eq!(agent.name(), "gemma3:4b");
    }

    #[test]
    fn infer_sentiment_sorry_is_sad() {
        assert_eq!(OllamaAgent::infer_sentiment("I'm sorry, I can't help with that."), Sentiment::Sad);
    }

    #[test]
    fn infer_sentiment_happy_keywords() {
        assert_eq!(OllamaAgent::infer_sentiment("I'm happy to help you with that!"), Sentiment::Happy);
        assert_eq!(OllamaAgent::infer_sentiment("Of course! Let me explain."), Sentiment::Happy);
    }

    #[test]
    fn infer_sentiment_neutral_default() {
        assert_eq!(OllamaAgent::infer_sentiment("The capital of France is Paris."), Sentiment::Neutral);
    }

    #[tokio::test]
    async fn health_check_fails_gracefully_when_no_server() {
        // Port 19999 is almost certainly not running Ollama.
        let agent = OllamaAgent::with_url("gemma3:4b", "http://127.0.0.1:19999");
        let healthy = agent.health_check().await;
        assert!(!healthy, "should return false when Ollama is unreachable");
    }

    #[tokio::test]
    async fn respond_returns_helpful_error_when_no_server() {
        let agent = OllamaAgent::with_url("gemma3:4b", "http://127.0.0.1:19999");
        let (response, sentiment) = agent.respond("hello").await;
        assert!(
            response.contains("not reachable") || response.contains("ollama.ai"),
            "unexpected response: {response}"
        );
        assert_eq!(sentiment, Sentiment::Sad);
    }

    #[tokio::test]
    async fn check_status_returns_not_running_when_no_server() {
        let client = Client::new();
        let status = check_status(&client, "http://127.0.0.1:19999").await;
        assert!(!status.running);
        assert_eq!(status.model_count, 0);
    }
}
