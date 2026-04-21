use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::agent::stub_agent::Sentiment;
use crate::agent::AgentProvider;

pub const OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434";

/// System prompt injected into every Ollama conversation.
const SYSTEM_PROMPT: &str = r#"You are TerranSoul, a friendly AI companion with a 3D character avatar. You live inside the TerranSoul desktop app and serve as the user's intelligent assistant.

Your capabilities:
- Helpful conversation and answering questions on any topic
- Recommending AI tools and software based on the user's needs
- Guiding users through installing packages via the TerranSoul Package Manager

Available packages you can recommend:
- **OpenClaw** (model tag: "openclaw-bridge"): An open-source AI interface that connects to powerful language model APIs. Great for users who want cloud-based AI alongside local models.
- **Claude Cowork** (model tag: "claude-cowork"): A collaborative AI workspace powered by Anthropic's Claude. Perfect for document analysis, long-context reasoning, and team workflows.
- **stub-agent**: The built-in lightweight agent. Always available offline.

When recommending a package, mention its name and briefly explain why it suits the user's request. Keep responses concise and warm."#;

// ── Ollama API types ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct OllamaModelEntry {
    pub name: String,
    pub size: u64,
}

#[derive(Deserialize)]
pub struct TagsResponse {
    pub models: Vec<OllamaModelEntry>,
}

#[derive(Serialize)]
pub struct OllamaStatus {
    pub running: bool,
    pub model_count: usize,
}

// ── OllamaAgent ────────────────────────────────────────────────────────────────

/// An AI agent backed by a locally running Ollama language model.
pub struct OllamaAgent {
    model: String,
    base_url: String,
    client: Client,
}

impl OllamaAgent {
    /// Create an agent that talks to the given Ollama model on localhost.
    pub fn new(model: &str) -> Self {
        Self::with_url(model, OLLAMA_BASE_URL)
    }

    /// Create an agent with a custom Ollama base URL (useful for tests).
    pub fn with_url(model: &str, base_url: &str) -> Self {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| Client::new());
        OllamaAgent {
            model: model.to_string(),
            base_url: base_url.to_string(),
            client,
        }
    }

    fn chat_url(&self) -> String {
        format!("{}/api/chat", self.base_url)
    }

    fn tags_url(&self) -> String {
        format!("{}/api/tags", self.base_url)
    }

    /// Infer a simple sentiment label from the response text.
    fn infer_sentiment(text: &str) -> Sentiment {
        Self::infer_sentiment_static(text)
    }

    /// Static version of sentiment inference, usable without an OllamaAgent instance.
    pub fn infer_sentiment_static(text: &str) -> Sentiment {
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

    /// Build the full message list from system prompt + optional memory block + history + current message.
    fn build_messages(
        &self,
        message: &str,
        history: &[(String, String)],
        memories: &[String],
    ) -> Vec<ChatMessage> {
        let system_content = if memories.is_empty() {
            SYSTEM_PROMPT.to_string()
        } else {
            let mem_block = memories.join("\n- ");
            format!("{SYSTEM_PROMPT}\n\n[LONG-TERM MEMORY]\n- {mem_block}\n[/LONG-TERM MEMORY]")
        };

        let mut msgs = vec![ChatMessage {
            role: "system".to_string(),
            content: system_content,
        }];

        for (role, content) in history {
            msgs.push(ChatMessage {
                role: role.clone(),
                content: content.clone(),
            });
        }

        msgs.push(ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });

        msgs
    }

    /// Send `messages` to Ollama and decode the assistant reply.
    async fn call(&self, messages: Vec<ChatMessage>) -> (String, Sentiment) {
        let body = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
        };

        match self.client.post(self.chat_url()).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<ChatResponse>().await {
                    Ok(chat) => {
                        let sentiment = Self::infer_sentiment(&chat.message.content);
                        (chat.message.content, sentiment)
                    }
                    Err(e) => (
                        format!(
                            "I received a response but couldn't parse it ({}). Please try again.",
                            e
                        ),
                        Sentiment::Neutral,
                    ),
                }
            }
            Ok(resp) => (
                format!(
                    "My brain returned an error (HTTP {}). Is the model '{}' installed? Try: `ollama pull {}`",
                    resp.status(),
                    self.model,
                    self.model
                ),
                Sentiment::Sad,
            ),
            Err(_) => (
                "My brain (Ollama) is not reachable right now. Please make sure Ollama is running: https://ollama.ai"
                    .to_string(),
                Sentiment::Sad,
            ),
        }
    }

    /// Respond with full conversation history and injected long-term memories.
    ///
    /// `history` is a slice of (role, content) pairs ordered oldest-first.
    /// `memories` is a list of long-term memory strings to inject into the system prompt.
    pub async fn respond_contextual(
        &self,
        message: &str,
        history: &[(String, String)],
        memories: &[String],
    ) -> (String, Sentiment) {
        let msgs = self.build_messages(message, history, memories);
        self.call(msgs).await
    }

    /// Ask the brain to extract memorable facts from a conversation.
    ///
    /// Returns a list of short fact strings (one per line) or an empty vec on failure.
    pub async fn extract_memories(&self, conversation_text: &str) -> Vec<String> {
        let prompt = format!(
            "Read this conversation and extract up to 5 important facts worth remembering \
            about the user (preferences, goals, personal details, ongoing projects). \
            Reply with ONLY a bullet list, one fact per line, starting each line with '- '. \
            If there is nothing worth remembering, reply with exactly: NONE\n\n{conversation_text}"
        );

        let msgs = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a memory extraction assistant. Extract concise facts about the user from conversations.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let (reply, _) = self.call(msgs).await;
        if reply.trim() == "NONE" || reply.trim().is_empty() {
            return vec![];
        }

        reply
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim().trim_start_matches("- ").trim();
                if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
            })
            .collect()
    }

    /// Ask the brain to summarize a conversation into a single memory entry.
    pub async fn summarize_conversation(&self, conversation_text: &str) -> Option<String> {
        let prompt = format!(
            "Summarize this conversation in 1-3 sentences, focusing on what the user \
            was trying to accomplish and any conclusions reached. Be concise.\n\n{conversation_text}"
        );

        let msgs = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a concise summarizer. Summarize conversations into 1-3 sentences.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let (reply, _) = self.call(msgs).await;
        let clean = reply.trim().to_string();
        if clean.is_empty() { None } else { Some(clean) }
    }

    // ── Embedding ──────────────────────────────────────────────────────────

    /// Generate a vector embedding for `text` via Ollama `/api/embed`.
    /// Uses `nomic-embed-text` (768-dim) if available, otherwise falls
    /// back to the active chat model.  Returns `None` on any error.
    pub async fn embed_text(text: &str, model_hint: &str) -> Option<Vec<f32>> {
        // Prefer a dedicated embedding model; fall back to chat model.
        let embed_model = if Self::model_exists("nomic-embed-text").await {
            "nomic-embed-text".to_string()
        } else {
            model_hint.to_string()
        };

        let client = Client::new();
        let body = serde_json::json!({
            "model": embed_model,
            "input": text,
        });

        let resp = client
            .post(format!("{OLLAMA_BASE_URL}/api/embed"))
            .json(&body)
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let json: serde_json::Value = resp.json().await.ok()?;
        // Ollama returns { "embeddings": [[...]] }
        let arr = json.get("embeddings")?.as_array()?.first()?.as_array()?;
        let vec: Vec<f32> = arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect();
        if vec.is_empty() { None } else { Some(vec) }
    }

    /// Check if a model name is available locally in Ollama.
    async fn model_exists(name: &str) -> bool {
        let client = Client::new();
        let resp = client
            .get(format!("{OLLAMA_BASE_URL}/api/tags"))
            .send()
            .await;
        match resp {
            Ok(r) => {
                if let Ok(json) = r.json::<serde_json::Value>().await {
                    json.get("models")
                        .and_then(|m| m.as_array())
                        .map(|models| {
                            models.iter().any(|m| {
                                m.get("name")
                                    .and_then(|n| n.as_str())
                                    .map(|n| n.starts_with(name))
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    /// Ask the brain which stored memories are most relevant to a query.
    ///
    /// `candidates` is a list of (id, content) pairs.  
    /// Returns the ids of the top relevant entries.
    pub async fn semantic_relevant_ids(
        &self,
        query: &str,
        candidates: &[(i64, String)],
        limit: usize,
    ) -> Vec<i64> {
        if candidates.is_empty() {
            return vec![];
        }

        let numbered = candidates
            .iter()
            .enumerate()
            .map(|(i, (_, content))| format!("{}. {}", i + 1, content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Given this user query:\n\"{query}\"\n\n\
            Which of the following memories are most relevant? \
            Reply with ONLY the numbers of the top {limit} relevant ones, \
            comma-separated (e.g. \"1,3,5\"). If none are relevant, reply \"NONE\".\n\n{numbered}"
        );

        let msgs = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You select the most relevant memories from a list. Reply with numbers only.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let (reply, _) = self.call(msgs).await;
        if reply.trim() == "NONE" {
            return vec![];
        }

        reply
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .filter(|&n| n >= 1 && n <= candidates.len())
            .take(limit)
            .map(|n| candidates[n - 1].0)
            .collect()
    }
}

// ── AgentProvider trait impl ───────────────────────────────────────────────────

#[async_trait]
impl AgentProvider for OllamaAgent {
    fn id(&self) -> &str {
        "ollama"
    }

    fn name(&self) -> &str {
        &self.model
    }

    async fn respond(&self, message: &str) -> (String, Sentiment) {
        let msgs = self.build_messages(message, &[], &[]);
        self.call(msgs).await
    }

    async fn health_check(&self) -> bool {
        self.client
            .get(self.tags_url())
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

// ── Module-level helpers ───────────────────────────────────────────────────────

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
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<TagsResponse>()
            .await
            .map(|t| t.models)
            .unwrap_or_default(),
        _ => vec![],
    }
}

/// Pull an Ollama model, consuming the streaming progress response.
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

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        chunk.map_err(|e| format!("stream error: {e}"))?;
    }
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

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
        assert_eq!(
            OllamaAgent::infer_sentiment("I'm sorry, I can't help with that."),
            Sentiment::Sad
        );
    }

    #[test]
    fn infer_sentiment_happy_keywords() {
        assert_eq!(
            OllamaAgent::infer_sentiment("I'm happy to help you with that!"),
            Sentiment::Happy
        );
        assert_eq!(
            OllamaAgent::infer_sentiment("Of course! Let me explain."),
            Sentiment::Happy
        );
    }

    #[test]
    fn infer_sentiment_neutral_default() {
        assert_eq!(
            OllamaAgent::infer_sentiment("The capital of France is Paris."),
            Sentiment::Neutral
        );
    }

    #[test]
    fn build_messages_no_history_no_memory() {
        let agent = OllamaAgent::new("gemma3:4b");
        let msgs = agent.build_messages("hello", &[], &[]);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "system");
        assert_eq!(msgs[1].role, "user");
        assert_eq!(msgs[1].content, "hello");
    }

    #[test]
    fn build_messages_injects_memories() {
        let agent = OllamaAgent::new("gemma3:4b");
        let mems = vec!["User likes Python".to_string()];
        let msgs = agent.build_messages("tell me about coding", &[], &mems);
        assert!(msgs[0].content.contains("LONG-TERM MEMORY"));
        assert!(msgs[0].content.contains("User likes Python"));
    }

    #[test]
    fn build_messages_includes_history() {
        let agent = OllamaAgent::new("gemma3:4b");
        let history = vec![
            ("user".to_string(), "previous question".to_string()),
            ("assistant".to_string(), "previous answer".to_string()),
        ];
        let msgs = agent.build_messages("follow-up", &history, &[]);
        // system + 2 history + current user
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[1].content, "previous question");
        assert_eq!(msgs[2].content, "previous answer");
        assert_eq!(msgs[3].content, "follow-up");
    }

    #[tokio::test]
    async fn health_check_fails_gracefully_when_no_server() {
        let agent = OllamaAgent::with_url("gemma3:4b", "http://127.0.0.1:19999");
        assert!(!agent.health_check().await);
    }

    #[tokio::test]
    async fn respond_returns_helpful_error_when_no_server() {
        let agent = OllamaAgent::with_url("gemma3:4b", "http://127.0.0.1:19999");
        let (response, sentiment) = agent.respond("hello").await;
        assert!(
            response.contains("not reachable") || response.contains("ollama.ai"),
            "unexpected: {response}"
        );
        assert_eq!(sentiment, Sentiment::Sad);
    }

    #[tokio::test]
    async fn check_status_not_running_when_no_server() {
        let client = Client::new();
        let status = check_status(&client, "http://127.0.0.1:19999").await;
        assert!(!status.running);
        assert_eq!(status.model_count, 0);
    }
}
