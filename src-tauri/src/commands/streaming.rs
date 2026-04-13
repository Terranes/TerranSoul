use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use crate::AppState;
use crate::brain::ollama_agent::{ChatMessage, OLLAMA_BASE_URL};
use crate::brain::openai_client::{OpenAiClient, OpenAiMessage};
use crate::brain::brain_config::BrainMode;

/// A single streamed chunk emitted via Tauri events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChunk {
    /// The text content of this chunk.
    pub text: String,
    /// Whether this is the final chunk (stream ended).
    pub done: bool,
}

/// Ollama streaming response shape — each line of the NDJSON stream.
#[derive(Debug, Deserialize)]
struct OllamaStreamChunk {
    message: Option<OllamaStreamMessage>,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamMessage {
    content: String,
}

/// Stream a chat response, routing through the configured BrainMode:
/// - FreeApi / PaidApi → OpenAI-compatible SSE streaming
/// - LocalOllama → Ollama NDJSON streaming
/// - No config → stub agent fallback
#[tauri::command]
pub async fn send_message_stream(
    message: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("Message cannot be empty".to_string());
    }

    // Add user message to conversation
    let user_msg = crate::commands::chat::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "user".to_string(),
        content: message.clone(),
        agent_name: None,
        sentiment: None,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    };

    {
        let mut conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.push(user_msg);
    }

    // Determine routing: check brain_mode first, then fall back to legacy active_brain
    let brain_mode: Option<BrainMode> = {
        state.brain_mode.lock().map_err(|e| e.to_string())?.clone()
    };

    let legacy_model: Option<String> = {
        state.active_brain.lock().map_err(|e| e.to_string())?.clone()
    };

    // Build conversation history (last 20 messages)
    let history: Vec<(String, String)> = {
        let conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .rev()
            .take(20)
            .rev()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    };

    match brain_mode {
        Some(BrainMode::FreeApi { provider_id, api_key }) => {
            stream_openai_api(&app_handle, &state, &message, &history, &provider_id, api_key.as_deref(), None).await
        }
        Some(BrainMode::PaidApi { provider: _, api_key, model, base_url }) => {
            stream_openai_api(&app_handle, &state, &message, &history, "paid", Some(&api_key), Some((&base_url, &model))).await
        }
        Some(BrainMode::LocalOllama { model }) => {
            stream_ollama(&app_handle, &state, &message, &history, &model).await
        }
        None => {
            // Check legacy active_brain
            if let Some(model) = legacy_model {
                stream_ollama(&app_handle, &state, &message, &history, &model).await
            } else {
                // No brain — emit stub response
                emit_stub_response(&app_handle, &state, &message)
            }
        }
    }
}

/// Stream via an OpenAI-compatible API (used for FreeApi and PaidApi modes).
async fn stream_openai_api(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    _message: &str,
    history: &[(String, String)],
    provider_id: &str,
    api_key: Option<&str>,
    paid_override: Option<(&str, &str)>, // (base_url, model) for paid API
) -> Result<(), String> {
    // Resolve base_url and model
    let (base_url, model) = if let Some((url, mdl)) = paid_override {
        (url.to_string(), mdl.to_string())
    } else {
        // Look up free provider
        let provider = crate::brain::get_free_provider(provider_id)
            .ok_or_else(|| format!("Unknown free provider: {provider_id}"))?;
        (provider.base_url, provider.model)
    };

    let client = OpenAiClient::new(&base_url, &model, api_key);

    // Build OpenAI message array
    let mut messages = vec![OpenAiMessage {
        role: "system".to_string(),
        content: super::chat::SYSTEM_PROMPT_FOR_STREAMING.to_string(),
    }];
    for (role, content) in history {
        messages.push(OpenAiMessage {
            role: role.clone(),
            content: content.clone(),
        });
    }

    // Stream with callback
    let app = app_handle.clone();
    let result = client
        .chat_stream(messages, move |chunk_text| {
            let _ = app.emit(
                "llm-chunk",
                LlmChunk {
                    text: chunk_text.to_string(),
                    done: false,
                },
            );
        })
        .await;

    match result {
        Ok(full_response) => {
            let _ = app_handle.emit("llm-chunk", LlmChunk { text: String::new(), done: true });
            store_assistant_message(state, &full_response, &model)?;
            Ok(())
        }
        Err(e) => {
            let _ = app_handle.emit("llm-chunk", LlmChunk { text: String::new(), done: true });
            Err(format!("Free API error: {e}"))
        }
    }
}

/// Stream via local Ollama (NDJSON format).
async fn stream_ollama(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    _message: &str,
    history: &[(String, String)],
    model: &str,
) -> Result<(), String> {
    // Build Ollama message array
    let system_msg = ChatMessage {
        role: "system".to_string(),
        content: super::chat::SYSTEM_PROMPT_FOR_STREAMING.to_string(),
    };
    let mut messages = vec![system_msg];
    for (role, content) in history {
        messages.push(ChatMessage {
            role: role.clone(),
            content: content.clone(),
        });
    }

    let url = format!("{OLLAMA_BASE_URL}/api/chat");
    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
    });

    let client = &state.ollama_client;
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama not reachable: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Ollama returned HTTP {}", resp.status()));
    }

    let mut full_response = String::new();
    let mut stream = resp.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk_result) = stream.next().await {
        let bytes = chunk_result.map_err(|e| format!("stream error: {e}"))?;
        let text = String::from_utf8_lossy(&bytes);

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(parsed) = serde_json::from_str::<OllamaStreamChunk>(line) {
                if let Some(msg) = &parsed.message {
                    if !msg.content.is_empty() {
                        full_response.push_str(&msg.content);
                        let _ = app_handle.emit(
                            "llm-chunk",
                            LlmChunk {
                                text: msg.content.clone(),
                                done: false,
                            },
                        );
                    }
                }
                if parsed.done {
                    let _ = app_handle.emit(
                        "llm-chunk",
                        LlmChunk {
                            text: String::new(),
                            done: true,
                        },
                    );
                }
            }
        }
    }

    store_assistant_message(state, &full_response, model)?;
    Ok(())
}

/// Emit a stub response when no brain is configured.
fn emit_stub_response(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    message: &str,
) -> Result<(), String> {
    let stub_text = format!("I hear you! You said: \"{message}\". I'm still learning, but I'm always here to listen and help!");
    let _ = app_handle.emit("llm-chunk", LlmChunk { text: stub_text.clone(), done: false });
    let _ = app_handle.emit("llm-chunk", LlmChunk { text: String::new(), done: true });

    let assistant_msg = crate::commands::chat::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: stub_text,
        agent_name: Some("TerranSoul".to_string()),
        sentiment: Some("neutral".to_string()),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    };
    let mut conv = state.conversation.lock().map_err(|e| e.to_string())?;
    conv.push(assistant_msg);
    Ok(())
}

/// Store the completed assistant message in the conversation.
fn store_assistant_message(
    state: &AppState,
    full_response: &str,
    model: &str,
) -> Result<(), String> {
    let sentiment = crate::brain::OllamaAgent::infer_sentiment_static(full_response);
    let sentiment_label = match sentiment {
        crate::agent::stub_agent::Sentiment::Happy => "happy",
        crate::agent::stub_agent::Sentiment::Sad => "sad",
        crate::agent::stub_agent::Sentiment::Neutral => "neutral",
    };
    let assistant_msg = crate::commands::chat::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: full_response.to_string(),
        agent_name: Some(model.to_string()),
        sentiment: Some(sentiment_label.to_string()),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    };
    let mut conv = state.conversation.lock().map_err(|e| e.to_string())?;
    conv.push(assistant_msg);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_chunk_serializes() {
        let chunk = LlmChunk {
            text: "Hello".to_string(),
            done: false,
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("false"));
    }

    #[test]
    fn llm_chunk_done_flag() {
        let chunk = LlmChunk {
            text: String::new(),
            done: true,
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("true"));
    }

    #[test]
    fn ollama_stream_chunk_deserializes() {
        let json = r#"{"message":{"role":"assistant","content":"Hi"},"done":false}"#;
        let parsed: OllamaStreamChunk = serde_json::from_str(json).unwrap();
        assert!(!parsed.done);
        assert_eq!(parsed.message.unwrap().content, "Hi");
    }

    #[test]
    fn ollama_stream_chunk_done() {
        let json = r#"{"message":{"role":"assistant","content":""},"done":true}"#;
        let parsed: OllamaStreamChunk = serde_json::from_str(json).unwrap();
        assert!(parsed.done);
    }

    #[test]
    fn brain_mode_routes_free_api() {
        // Verify BrainMode::FreeApi can be pattern-matched for routing
        let mode = BrainMode::FreeApi {
            provider_id: "groq".to_string(),
            api_key: None,
        };
        match &mode {
            BrainMode::FreeApi { provider_id, .. } => assert_eq!(provider_id, "groq"),
            _ => panic!("expected FreeApi"),
        }
    }

    #[test]
    fn brain_mode_routes_paid_api() {
        let mode = BrainMode::PaidApi {
            provider: "openai".to_string(),
            api_key: "sk-test".to_string(),
            model: "gpt-4o".to_string(),
            base_url: "https://api.openai.com".to_string(),
        };
        match &mode {
            BrainMode::PaidApi { model, .. } => assert_eq!(model, "gpt-4o"),
            _ => panic!("expected PaidApi"),
        }
    }

    #[test]
    fn brain_mode_routes_local_ollama() {
        let mode = BrainMode::LocalOllama {
            model: "gemma3:4b".to_string(),
        };
        match &mode {
            BrainMode::LocalOllama { model } => assert_eq!(model, "gemma3:4b"),
            _ => panic!("expected LocalOllama"),
        }
    }
}
