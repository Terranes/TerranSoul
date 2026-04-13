use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use crate::AppState;
use crate::brain::ollama_agent::{ChatMessage, OLLAMA_BASE_URL};

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

/// Stream a chat response from Ollama, emitting `llm-chunk` events to the frontend.
/// Falls back to the stub agent response if no brain is configured.
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

    // Check if a brain model is configured
    let model_opt: Option<String> = {
        state.active_brain.lock().map_err(|e| e.to_string())?.clone()
    };

    let Some(model) = model_opt else {
        // No brain — emit stub response as single chunk
        let stub_text = format!("I hear you! You said: \"{message}\". I'm still learning, but I'm always here to listen and help!");
        let _ = app_handle.emit("llm-chunk", LlmChunk { text: stub_text.clone(), done: false });
        let _ = app_handle.emit("llm-chunk", LlmChunk { text: String::new(), done: true });

        // Add assistant message to conversation
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
        return Ok(());
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

    // Build Ollama message array
    let system_msg = ChatMessage {
        role: "system".to_string(),
        content: super::chat::SYSTEM_PROMPT_FOR_STREAMING.to_string(),
    };
    let mut messages = vec![system_msg];
    for (role, content) in &history {
        messages.push(ChatMessage {
            role: role.clone(),
            content: content.clone(),
        });
    }
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: message,
    });

    // Stream from Ollama
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

        // Ollama streams NDJSON — each line is a JSON object
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

    // Add complete assistant message to conversation
    let sentiment = crate::brain::OllamaAgent::infer_sentiment_static(&full_response);
    // Convert sentiment to string using the same mapping as chat.rs
    let sentiment_label = match sentiment {
        crate::agent::stub_agent::Sentiment::Happy => "happy",
        crate::agent::stub_agent::Sentiment::Sad => "sad",
        crate::agent::stub_agent::Sentiment::Neutral => "neutral",
    };
    let assistant_msg = crate::commands::chat::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: full_response,
        agent_name: Some(model),
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
}
