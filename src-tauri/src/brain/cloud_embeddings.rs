//! Cloud embedding API — OpenAI-compatible `/v1/embeddings` dispatch.
//!
//! When the brain mode is `FreeApi` or `PaidApi`, the existing
//! `OllamaAgent::embed_text` can't help because Ollama isn't the
//! active provider. This module fills the gap by calling the standard
//! OpenAI-compatible embeddings endpoint that many cloud providers
//! expose.
//!
//! **Resilience contract** — mirrors `OllamaAgent::embed_text`:
//! - Returns `Option<Vec<f32>>` — never panics, never errors.
//! - Caches per-provider "unsupported" status so we don't retry
//!   providers that 404/501 on `/v1/embeddings`.
//! - 15 s timeout per call.
//!
//! Chunk 16.9 — see `docs/brain-advanced-design.md` §16 Phase 4.

use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;

use super::brain_config::BrainMode;

// ── Default embedding models per provider ─────────────────────────────────────

/// Default embedding model to use for OpenAI-compatible paid APIs.
const OPENAI_EMBED_MODEL: &str = "text-embedding-3-small";

/// Map a free-provider ID to an embedding model name, if that provider
/// supports embeddings. Returns `None` for providers with no embed endpoint.
fn free_provider_embed_model(provider_id: &str) -> Option<&'static str> {
    match provider_id {
        "mistral" => Some("mistral-embed"),
        "github-models" => Some("text-embedding-3-small"),
        "siliconflow" => Some("BAAI/bge-m3"),
        "nvidia-nim" => Some("nvidia/nv-embedqa-e5-v5"),
        _ => None, // pollinations, groq, cerebras, openrouter — no embed API
    }
}

/// Map a paid-provider name to a default embedding model.
/// The user can override via the embed model selector in settings (future).
fn paid_provider_embed_model(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "voyage-3-lite", // Anthropic recommends Voyage; same key
        "mistral" => "mistral-embed",
        _ => OPENAI_EMBED_MODEL, // openai, groq-paid, azure, etc.
    }
}

// ── Unsupported-provider cache ────────────────────────────────────────────────

static CLOUD_UNSUPPORTED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn unsupported_set() -> &'static Mutex<HashSet<String>> {
    CLOUD_UNSUPPORTED.get_or_init(|| Mutex::new(HashSet::new()))
}

async fn is_cloud_unsupported(key: &str) -> bool {
    unsupported_set().lock().await.contains(key)
}

async fn mark_cloud_unsupported(key: &str) {
    unsupported_set().lock().await.insert(key.to_string());
}

/// Clear the cloud embed unsupported cache (called on brain mode switch).
pub async fn clear_cloud_embed_cache() {
    unsupported_set().lock().await.clear();
}

// ── OpenAI-compatible embeddings response ─────────────────────────────────────

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingObject>,
}

#[derive(Deserialize)]
struct EmbeddingObject {
    embedding: Vec<f32>,
}

// ── Core embed function ───────────────────────────────────────────────────────

/// Call an OpenAI-compatible `/v1/embeddings` endpoint.
///
/// Returns `None` on any failure — network, auth, unsupported model, etc.
async fn embed_text_openai(
    text: &str,
    base_url: &str,
    model: &str,
    api_key: Option<&str>,
) -> Option<Vec<f32>> {
    if text.trim().is_empty() {
        return None;
    }

    let cache_key = format!("{base_url}::{model}");
    if is_cloud_unsupported(&cache_key).await {
        return None;
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .ok()?;

    let url = format!("{}/v1/embeddings", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "input": text,
    });

    let mut req = client.post(&url).json(&body);
    if let Some(key) = api_key {
        req = req.bearer_auth(key);
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(_) => return None,
    };

    if !resp.status().is_success() {
        mark_cloud_unsupported(&cache_key).await;
        return None;
    }

    let parsed: EmbeddingResponse = resp.json().await.ok()?;
    let vec = parsed.data.into_iter().next()?.embedding;
    if vec.is_empty() {
        None
    } else {
        Some(vec)
    }
}

// ── Unified dispatcher ────────────────────────────────────────────────────────

/// Generate an embedding using the best available provider for the
/// current brain mode:
///
/// - `LocalOllama` → delegates to `OllamaAgent::embed_text`
/// - `PaidApi` → calls the provider's `/v1/embeddings`
/// - `FreeApi` → calls the free provider's embed endpoint (if supported)
/// - `None` (no brain configured) → returns `None`
///
/// This is the single entry point all Tauri commands should use
/// instead of calling `OllamaAgent::embed_text` directly.
pub async fn embed_for_mode(
    text: &str,
    brain_mode: Option<&BrainMode>,
    active_brain: Option<&str>,
) -> Option<Vec<f32>> {
    match brain_mode {
        Some(BrainMode::LocalOllama { model }) => super::OllamaAgent::embed_text(text, model).await,
        Some(BrainMode::LocalLmStudio {
            model,
            base_url,
            api_key,
            embedding_model,
        }) => {
            let embed_model = embedding_model.as_deref().unwrap_or(model);
            embed_text_openai(text, base_url, embed_model, api_key.as_deref()).await
        }
        Some(BrainMode::PaidApi {
            base_url,
            api_key,
            provider,
            ..
        }) => {
            let embed_model = paid_provider_embed_model(provider);
            embed_text_openai(text, base_url, embed_model, Some(api_key)).await
        }
        Some(BrainMode::FreeApi {
            provider_id,
            api_key,
            ..
        }) => {
            // Only try if this provider has an embed endpoint.
            let embed_model = free_provider_embed_model(provider_id)?;
            let provider = super::get_free_provider(provider_id)?;
            embed_text_openai(text, &provider.base_url, embed_model, api_key.as_deref()).await
        }
        None => {
            // Legacy fallback: if active_brain is set, use Ollama.
            if let Some(model) = active_brain {
                super::OllamaAgent::embed_text(text, model).await
            } else {
                None
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_provider_embed_model_known() {
        assert_eq!(free_provider_embed_model("mistral"), Some("mistral-embed"));
        assert_eq!(
            free_provider_embed_model("github-models"),
            Some("text-embedding-3-small")
        );
        assert_eq!(
            free_provider_embed_model("siliconflow"),
            Some("BAAI/bge-m3")
        );
        assert_eq!(
            free_provider_embed_model("nvidia-nim"),
            Some("nvidia/nv-embedqa-e5-v5")
        );
    }

    #[test]
    fn free_provider_embed_model_unsupported() {
        assert!(free_provider_embed_model("pollinations").is_none());
        assert!(free_provider_embed_model("groq").is_none());
        assert!(free_provider_embed_model("cerebras").is_none());
        assert!(free_provider_embed_model("openrouter").is_none());
    }

    #[test]
    fn paid_provider_embed_model_defaults() {
        assert_eq!(paid_provider_embed_model("openai"), OPENAI_EMBED_MODEL);
        assert_eq!(paid_provider_embed_model("anthropic"), "voyage-3-lite");
        assert_eq!(paid_provider_embed_model("mistral"), "mistral-embed");
        assert_eq!(
            paid_provider_embed_model("unknown-provider"),
            OPENAI_EMBED_MODEL
        );
    }

    #[tokio::test]
    async fn embed_for_mode_none_returns_none() {
        let result = embed_for_mode("hello world", None, None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn embed_for_mode_free_unsupported_provider_returns_none() {
        // Pollinations has no embed API → should return None.
        let mode = BrainMode::FreeApi {
            provider_id: "pollinations".to_string(),
            api_key: None,
            model: None,
        };
        let result = embed_for_mode("hello world", Some(&mode), None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn embed_for_mode_empty_text_returns_none() {
        let mode = BrainMode::PaidApi {
            provider: "openai".to_string(),
            api_key: "sk-test".to_string(),
            model: "gpt-4o".to_string(),
            base_url: "https://api.openai.com".to_string(),
        };
        let result = embed_for_mode("   ", Some(&mode), None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn embed_for_mode_paid_no_server_returns_none() {
        // No server listening → should gracefully return None.
        let mode = BrainMode::PaidApi {
            provider: "openai".to_string(),
            api_key: "sk-test".to_string(),
            model: "gpt-4o".to_string(),
            base_url: "http://127.0.0.1:19999".to_string(),
        };
        let result = embed_for_mode("hello world", Some(&mode), None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn cloud_unsupported_cache_round_trip() {
        let key = "test-cache-key";
        assert!(!is_cloud_unsupported(key).await);
        mark_cloud_unsupported(key).await;
        assert!(is_cloud_unsupported(key).await);
        clear_cloud_embed_cache().await;
        assert!(!is_cloud_unsupported(key).await);
    }
}
