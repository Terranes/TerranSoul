use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// A single message in the OpenAI chat completions format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiMessage {
    pub role: String,
    pub content: String,
}

/// Request body for `/v1/chat/completions`.
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    stream: bool,
}

/// Delta content from a streaming SSE chunk.
#[derive(Debug, Deserialize)]
struct SseDelta {
    content: Option<String>,
}

/// A single choice in a streaming SSE response.
#[derive(Debug, Deserialize)]
struct SseChoice {
    delta: SseDelta,
}

/// A streaming SSE response chunk from the OpenAI-compatible API.
#[derive(Debug, Deserialize)]
struct SseChunkResponse {
    choices: Vec<SseChoice>,
}

/// Non-streaming response from `/v1/chat/completions`.
#[derive(Debug, Deserialize)]
struct NonStreamChoice {
    message: OpenAiMessage,
}

/// Token usage block returned by OpenAI-compatible providers under
/// the `usage` key. All fields are optional — local providers (Ollama,
/// LM Studio) often omit one or both.
#[derive(Debug, Default, Deserialize, Clone, Copy)]
pub struct ChatCompletionUsage {
    #[serde(default)]
    pub prompt_tokens: Option<u64>,
    #[serde(default)]
    pub completion_tokens: Option<u64>,
}

/// Non-streaming full response.
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<NonStreamChoice>,
    #[serde(default)]
    usage: Option<ChatCompletionUsage>,
}

/// A generic OpenAI-compatible chat client that works with any provider
/// using the standard `/v1/chat/completions` endpoint.
///
/// Supports both streaming (SSE) and non-streaming modes.
pub struct OpenAiClient {
    base_url: String,
    model: String,
    api_key: Option<String>,
    client: Client,
}

impl OpenAiClient {
    /// Create a new client for the given provider.
    ///
    /// `base_url` should NOT include the `/v1/chat/completions` path — that is
    /// appended automatically.
    pub fn new(base_url: &str, model: &str, api_key: Option<&str>) -> Self {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key: api_key.map(|s| s.to_string()),
            client,
        }
    }

    /// Build the full completions URL.
    ///
    /// Tolerant of a `/v1` suffix that the caller may have pasted (e.g.
    /// "https://api.openai.com/v1") — the suffix is stripped so we never
    /// produce a doubled `/v1/v1/chat/completions` URL.
    fn completions_url(&self) -> String {
        let trimmed = self.base_url.trim_end_matches('/');
        let normalised = trimmed.strip_suffix("/v1").unwrap_or(trimmed);
        format!("{normalised}/v1/chat/completions")
    }

    /// Send a non-streaming chat completion request and return the full reply.
    pub async fn chat(&self, messages: Vec<OpenAiMessage>) -> Result<String, String> {
        self.chat_with_usage(messages).await.map(|(reply, _)| reply)
    }

    /// Send a non-streaming chat completion request and return both the
    /// reply and any token usage the provider reported. Returns
    /// `Ok((reply, None))` for providers that don't expose usage
    /// metadata (most local servers).
    ///
    /// Used by the self-improve metrics path (Chunk 28.7) to record
    /// real per-run token counts and dollar cost. Plain [`Self::chat`]
    /// forwards to this and discards the usage block.
    pub async fn chat_with_usage(
        &self,
        messages: Vec<OpenAiMessage>,
    ) -> Result<(String, Option<ChatCompletionUsage>), String> {
        let body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            stream: false,
        };

        let mut req = self.client.post(self.completions_url()).json(&body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req.send().await.map_err(|e| format!("request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body_text}"));
        }

        let parsed: ChatCompletionResponse = resp
            .json()
            .await
            .map_err(|e| format!("failed to parse response: {e}"))?;

        let usage = parsed.usage;
        let reply = parsed
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| "no choices in response".to_string())?;
        Ok((reply, usage))
    }

    /// Stream a chat completion, calling `on_chunk` for each text delta.
    ///
    /// Returns the full concatenated response text.
    pub async fn chat_stream<F>(
        &self,
        messages: Vec<OpenAiMessage>,
        mut on_chunk: F,
    ) -> Result<String, String>
    where
        F: FnMut(&str),
    {
        let body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            stream: true,
        };

        let mut req = self.client.post(self.completions_url()).json(&body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req.send().await.map_err(|e| format!("request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body_text}"));
        }

        let mut full_text = String::new();
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let bytes = chunk_result.map_err(|e| format!("stream error: {e}"))?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            // Process complete SSE lines from the buffer
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                // SSE format: "data: {...}" or "data: [DONE]"
                if let Some(data) = line.strip_prefix("data: ") {
                    if data.trim() == "[DONE]" {
                        return Ok(full_text);
                    }

                    if let Ok(parsed) = serde_json::from_str::<SseChunkResponse>(data) {
                        for choice in &parsed.choices {
                            if let Some(content) = &choice.delta.content {
                                if !content.is_empty() {
                                    full_text.push_str(content);
                                    on_chunk(content);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(full_text)
    }

    /// Return the model this client is configured to use.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Return the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completions_url_builds_correctly() {
        let client = OpenAiClient::new("https://api.groq.com/openai", "llama-3.3-70b", None);
        assert_eq!(
            client.completions_url(),
            "https://api.groq.com/openai/v1/chat/completions"
        );
    }

    #[test]
    fn completions_url_strips_trailing_slash() {
        let client =
            OpenAiClient::new("https://api.groq.com/openai/", "llama-3.3-70b", None);
        assert_eq!(
            client.completions_url(),
            "https://api.groq.com/openai/v1/chat/completions"
        );
    }

    #[test]
    fn completions_url_does_not_double_v1_suffix() {
        // Users frequently paste "/v1" into the base URL because every
        // OpenAI-compatible doc lists endpoints rooted at "/v1/...".
        // Both forms must produce the correct completions URL.
        let with_v1 = OpenAiClient::new("http://127.0.0.1:11434/v1", "gemma3:4b", None);
        assert_eq!(
            with_v1.completions_url(),
            "http://127.0.0.1:11434/v1/chat/completions"
        );
        let with_v1_slash = OpenAiClient::new("http://127.0.0.1:11434/v1/", "gemma3:4b", None);
        assert_eq!(
            with_v1_slash.completions_url(),
            "http://127.0.0.1:11434/v1/chat/completions"
        );
    }

    #[test]
    fn model_accessor() {
        let client = OpenAiClient::new("https://api.test.com", "my-model", None);
        assert_eq!(client.model(), "my-model");
    }

    #[test]
    fn base_url_accessor() {
        let client = OpenAiClient::new("https://api.test.com/", "m", None);
        assert_eq!(client.base_url(), "https://api.test.com");
    }

    #[test]
    fn openai_message_serializes() {
        let msg = OpenAiMessage {
            role: "user".into(),
            content: "Hello".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn openai_message_roundtrip() {
        let msg = OpenAiMessage {
            role: "assistant".into(),
            content: "Hi there!".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: OpenAiMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "assistant");
        assert_eq!(parsed.content, "Hi there!");
    }

    #[test]
    fn sse_chunk_deserializes() {
        let json = r#"{"id":"1","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let parsed: SseChunkResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.choices.len(), 1);
        assert_eq!(
            parsed.choices[0].delta.content.as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn sse_chunk_empty_delta() {
        let json = r#"{"choices":[{"delta":{},"index":0}]}"#;
        let parsed: SseChunkResponse = serde_json::from_str(json).unwrap();
        assert!(parsed.choices[0].delta.content.is_none());
    }

    #[test]
    fn non_stream_response_deserializes() {
        let json = r#"{"id":"1","object":"chat.completion","choices":[{"index":0,"message":{"role":"assistant","content":"World"},"finish_reason":"stop"}]}"#;
        let parsed: ChatCompletionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.choices[0].message.content, "World");
    }

    #[tokio::test]
    async fn chat_fails_gracefully_when_no_server() {
        let client = OpenAiClient::new("http://127.0.0.1:19998", "test-model", None);
        let msgs = vec![OpenAiMessage {
            role: "user".into(),
            content: "hi".into(),
        }];
        let result = client.chat(msgs).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("request failed"));
    }

    #[tokio::test]
    async fn chat_stream_fails_gracefully_when_no_server() {
        let client = OpenAiClient::new("http://127.0.0.1:19998", "test-model", None);
        let msgs = vec![OpenAiMessage {
            role: "user".into(),
            content: "hi".into(),
        }];
        let result = client.chat_stream(msgs, |_| {}).await;
        assert!(result.is_err());
    }
}
