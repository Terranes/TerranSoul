//! OpenAI-compatible client wrapper for the dedicated Coding LLM.
//!
//! Thin shim over [`crate::brain::openai_client::OpenAiClient`] that adapts
//! a [`super::CodingLlmConfig`] into a usable client. Kept separate so the
//! self-improve engine can construct one without reaching into the chat
//! brain plumbing.

use crate::brain::openai_client::{OpenAiClient, OpenAiMessage};

use super::CodingLlmConfig;

/// Construct an [`OpenAiClient`] from the persisted coding-LLM config.
///
/// When `api_key` is empty (e.g. local Ollama on `127.0.0.1:11434/v1`,
/// which does not require auth), the bearer-auth header is omitted —
/// otherwise some local servers reject the request with 400 / 401.
pub fn client_from(config: &CodingLlmConfig) -> OpenAiClient {
    let key = if config.api_key.trim().is_empty() {
        None
    } else {
        Some(config.api_key.as_str())
    };
    OpenAiClient::new(&config.base_url, &config.model, key)
}

/// Result of a reachability test. Surfaced in the BrainView "Test
/// connection" button — distinguishes between transport failures (DNS,
/// TLS, timeout) and HTTP errors (401 bad key, 404 wrong model).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReachabilityResult {
    pub ok: bool,
    /// One-line summary suitable for an inline status pill.
    pub summary: String,
    /// Optional detailed body (model-id reply, error text).
    pub detail: Option<String>,
}

/// Probe the configured coding LLM by sending a one-token completion.
///
/// We deliberately use the chat-completions endpoint with a `"hi"` prompt
/// rather than a hypothetical `/v1/models` call because the latter is not
/// universal across OpenAI-compatible providers (DeepSeek/Anthropic
/// proxies, Groq Lite). A successful chat round-trip proves the key,
/// model name, and base URL are all valid.
pub async fn test_reachability(config: &CodingLlmConfig) -> ReachabilityResult {
    let client = client_from(config);
    let messages = vec![OpenAiMessage {
        role: "user".to_string(),
        content: "Reply with the single word 'ok'.".to_string(),
    }];
    match client.chat(messages).await {
        Ok(reply) => {
            let trimmed = reply.trim();
            ReachabilityResult {
                ok: true,
                summary: format!("✓ Reachable — {} replied", config.model),
                detail: Some(trimmed.chars().take(120).collect()),
            }
        }
        Err(e) => ReachabilityResult {
            ok: false,
            summary: "✗ Unreachable".to_string(),
            detail: Some(e),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::CodingLlmProvider;

    #[test]
    fn client_from_plumbs_base_url_and_model() {
        let cfg = CodingLlmConfig {
            provider: CodingLlmProvider::Anthropic,
            model: "claude-sonnet-4-5".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            api_key: "sk-test".to_string(),
        };
        let client = client_from(&cfg);
        assert_eq!(client.model(), "claude-sonnet-4-5");
        assert!(client.base_url().contains("anthropic"));
    }

    #[tokio::test]
    async fn reachability_fails_gracefully_on_bad_url() {
        // Use a clearly-invalid URL so this test is deterministic and
        // doesn't depend on any external service.
        let cfg = CodingLlmConfig {
            provider: CodingLlmProvider::Custom,
            model: "test".to_string(),
            base_url: "http://127.0.0.1:1".to_string(),
            api_key: "x".to_string(),
        };
        let result = test_reachability(&cfg).await;
        assert!(!result.ok);
        assert!(result.summary.contains("Unreachable"));
    }

    /// End-to-end smoke test: spin up a minimal OpenAI-compatible stub
    /// HTTP server using axum and verify the reachability probe round-trips.
    /// This proves the engine can actually talk to a Coding LLM provider —
    /// it's the closest thing to a real integration test that can run in
    /// CI without external network access.
    #[tokio::test]
    async fn reachability_succeeds_against_stub_chat_completions_server() {
        use axum::{routing::post, Json, Router};

        // Stub handler returns a single-message chat-completion response
        // matching the OpenAI v1 schema that `OpenAiClient::chat` expects.
        async fn chat_handler(Json(_body): Json<serde_json::Value>) -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "choices": [
                    { "message": { "role": "assistant", "content": "ok" } }
                ]
            }))
        }

        let app = Router::new().route("/v1/chat/completions", post(chat_handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Give axum a moment to begin accepting.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let cfg = CodingLlmConfig {
            provider: CodingLlmProvider::Custom,
            model: "stub-model".to_string(),
            base_url: format!("http://{addr}"),
            api_key: "test-key".to_string(),
        };
        let result = test_reachability(&cfg).await;
        assert!(result.ok, "expected ok=true, got {result:?}");
        assert!(result.summary.contains("Reachable"));
        assert_eq!(result.detail.as_deref(), Some("ok"));

        server.abort();
    }
}
