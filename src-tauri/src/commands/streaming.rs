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

/// A structured animation command deserialized from `<anim>` JSON blocks.
/// Emitted as `llm-animation` Tauri events — the frontend receives typed data
/// instead of parsing raw text tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationCommand {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emotion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<String>,
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

// ── Streaming tag parser ──────────────────────────────────────────────────────

/// State-machine parser that extracts `<anim>{"emotion":"happy"}</anim>` blocks
/// from a stream of text chunks. Returns clean text and deserialized
/// [`AnimationCommand`]s — no regex needed on the frontend.
struct StreamTagParser {
    buffer: String,
    in_anim_block: bool,
    anim_buffer: String,
    strip_next_newline: bool,
}

impl StreamTagParser {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            in_anim_block: false,
            anim_buffer: String::new(),
            strip_next_newline: false,
        }
    }

    /// Feed a chunk of text. Returns `(clean_text, animation_commands)`.
    fn feed(&mut self, chunk: &str) -> (String, Vec<AnimationCommand>) {
        self.buffer.push_str(chunk);

        // Strip leading newline left over from a previous </anim> boundary.
        if self.strip_next_newline && !self.buffer.is_empty() {
            if self.buffer.starts_with("\r\n") {
                self.buffer = self.buffer[2..].to_string();
            } else if self.buffer.starts_with('\n') {
                self.buffer = self.buffer[1..].to_string();
            }
            self.strip_next_newline = false;
        }

        let mut text_out = String::new();
        let mut commands: Vec<AnimationCommand> = Vec::new();

        loop {
            if self.in_anim_block {
                if let Some(end) = self.buffer.find("</anim>") {
                    let json_part = self.buffer[..end].to_string();
                    self.anim_buffer.push_str(&json_part);
                    if let Ok(cmd) = serde_json::from_str::<AnimationCommand>(self.anim_buffer.trim()) {
                        commands.push(cmd);
                    }
                    self.anim_buffer.clear();
                    self.in_anim_block = false;
                    self.buffer = self.buffer[end + "</anim>".len()..].to_string();
                    // Strip one trailing newline so it doesn't leak into chat text.
                    if self.buffer.starts_with('\n') {
                        self.buffer = self.buffer[1..].to_string();
                    } else if self.buffer.starts_with("\r\n") {
                        self.buffer = self.buffer[2..].to_string();
                    } else if self.buffer.is_empty() {
                        self.strip_next_newline = true;
                    }
                } else {
                    // End tag not yet seen — hold back any partial `</anim>` prefix.
                    let hold = partial_prefix_len(&self.buffer, "</anim>");
                    let safe = self.buffer.len() - hold;
                    self.anim_buffer.push_str(&self.buffer[..safe]);
                    self.buffer = self.buffer[safe..].to_string();
                    break;
                }
            } else if let Some(start) = self.buffer.find("<anim>") {
                text_out.push_str(&self.buffer[..start]);
                self.buffer = self.buffer[start + "<anim>".len()..].to_string();
                self.in_anim_block = true;
            } else {
                // Hold back any partial `<anim>` prefix at the end.
                let hold = partial_prefix_len(&self.buffer, "<anim>");
                let safe = self.buffer.len() - hold;
                text_out.push_str(&self.buffer[..safe]);
                self.buffer = self.buffer[safe..].to_string();
                break;
            }
        }

        (text_out, commands)
    }

    /// Flush remaining buffered content (call when the stream ends).
    fn flush(&mut self) -> (String, Vec<AnimationCommand>) {
        let remaining = std::mem::take(&mut self.buffer);
        let anim_remaining = std::mem::take(&mut self.anim_buffer);
        self.in_anim_block = false;
        self.strip_next_newline = false;
        // If we were mid-anim-block, the content is malformed — emit as text.
        if !anim_remaining.is_empty() {
            return (format!("{anim_remaining}{remaining}"), Vec::new());
        }
        (remaining, Vec::new())
    }
}

/// How many bytes at the end of `buffer` could be the start of `tag`.
fn partial_prefix_len(buffer: &str, tag: &str) -> usize {
    let tag_bytes = tag.as_bytes();
    let buf_bytes = buffer.as_bytes();
    for len in (1..tag_bytes.len()).rev() {
        if len <= buf_bytes.len() && buf_bytes[buf_bytes.len() - len..] == tag_bytes[..len] {
            return len;
        }
    }
    0
}

/// Strip `<anim>...</anim>` blocks from a completed response (for storage).
fn strip_anim_blocks(input: &str) -> String {
    let mut result = String::new();
    let mut remaining = input;
    while let Some(start) = remaining.find("<anim>") {
        result.push_str(&remaining[..start]);
        remaining = &remaining[start + "<anim>".len()..];
        if let Some(end) = remaining.find("</anim>") {
            remaining = &remaining[end + "</anim>".len()..];
            // Skip one trailing newline.
            if remaining.starts_with('\n') {
                remaining = &remaining[1..];
            } else if remaining.starts_with("\r\n") {
                remaining = &remaining[2..];
            }
        }
    }
    result.push_str(remaining);
    result.trim().to_string()
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
            // Check rotator for a healthy provider, falling back to configured one
            let effective_provider_id = {
                let mut rotator = state.provider_rotator.lock().map_err(|e| e.to_string())?;
                rotator.next_healthy_provider()
                    .map(|p| p.id.clone())
                    .unwrap_or(provider_id.clone())
            };
            stream_openai_api(&app_handle, &state, &message, &history, &effective_provider_id, api_key.as_deref(), None).await
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

    // Stream with callback — parser separates text from <anim> blocks.
    let app = app_handle.clone();
    let parser = std::sync::Arc::new(std::sync::Mutex::new(StreamTagParser::new()));
    let parser_cb = std::sync::Arc::clone(&parser);
    let result = client
        .chat_stream(messages, move |chunk_text| {
            let mut p = parser_cb.lock().unwrap();
            let (clean_text, anim_cmds) = p.feed(chunk_text);
            if !clean_text.is_empty() {
                let _ = app.emit("llm-chunk", LlmChunk { text: clean_text, done: false });
            }
            for cmd in anim_cmds {
                let _ = app.emit("llm-animation", cmd);
            }
        })
        .await;

    match result {
        Ok(full_response) => {
            // Flush any remaining buffered content from the parser.
            {
                let mut p = parser.lock().unwrap();
                let (remaining_text, remaining_cmds) = p.flush();
                if !remaining_text.is_empty() {
                    let _ = app_handle.emit("llm-chunk", LlmChunk { text: remaining_text, done: false });
                }
                for cmd in remaining_cmds {
                    let _ = app_handle.emit("llm-animation", cmd);
                }
            }
            let _ = app_handle.emit("llm-chunk", LlmChunk { text: String::new(), done: true });
            // Record successful request in rotator
            {
                let mut rotator = state.provider_rotator.lock().map_err(|e| e.to_string())?;
                rotator.providers.entry(provider_id.to_string()).and_modify(|s| {
                    s.requests_sent += 1;
                });
            }
            store_assistant_message(state, &strip_anim_blocks(&full_response), &model)?;
            Ok(())
        }
        Err(e) => {
            let _ = app_handle.emit("llm-chunk", LlmChunk { text: String::new(), done: true });
            // Record rate limit if applicable
            let err_lower = e.to_string().to_lowercase();
            if err_lower.contains("429") || err_lower.contains("rate limit") {
                let mut rotator = state.provider_rotator.lock().map_err(|er| er.to_string())?;
                rotator.record_rate_limit(provider_id);
                if rotator.all_exhausted() {
                    let _ = app_handle.emit("providers-exhausted", ());
                }
            }
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
    // ── Local semantic-search RAG: retrieve relevant memories ───────────
    let mut system_prompt = super::chat::SYSTEM_PROMPT_FOR_STREAMING.to_string();

    // Extract the user's latest message for RAG query
    let user_query = history
        .iter()
        .rev()
        .find(|(role, _)| role == "user")
        .map(|(_, content)| content.as_str())
        .unwrap_or(_message);

    // Fast vector path: embed query → cosine search → <10ms.
    // Falls back to LLM ranking only when no embeddings exist yet.
    //
    // Step 1: Try vector search (lock held briefly, no await).
    let vector_results: Vec<crate::memory::MemoryEntry> = {
        let query_emb = crate::brain::OllamaAgent::embed_text(user_query, model).await;
        match (query_emb, state.memory_store.lock()) {
            (Some(emb), Ok(store)) => store.vector_search(&emb, 5).unwrap_or_default(),
            _ => vec![],
        }
    };

    // Step 2: If vector search returned nothing, try legacy LLM-rank fallback.
    let relevant = if !vector_results.is_empty() {
        vector_results
    } else {
        let entries: Vec<crate::memory::MemoryEntry> = {
            match state.memory_store.lock() {
                Ok(mem_store) => mem_store.get_all().unwrap_or_default(),
                Err(_) => vec![],
            }
        };
        if entries.is_empty() {
            vec![]
        } else {
            crate::memory::brain_memory::semantic_search_entries(
                model,
                user_query,
                &entries,
                5,
            )
            .await
        }
    };

    if !relevant.is_empty() {
        let memory_block: String = relevant
            .iter()
            .map(|e| format!("- {}", e.content))
            .collect::<Vec<_>>()
            .join("\n");
        system_prompt.push_str(&format!(
            "\n\n[LONG-TERM MEMORY]\nThe following facts from your memory are relevant to this conversation:\n{memory_block}\n[/LONG-TERM MEMORY]"
        ));
    }

    // Build Ollama message array
    let system_msg = ChatMessage {
        role: "system".to_string(),
        content: system_prompt,
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
    let mut parser = StreamTagParser::new();
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
                        let (clean_text, anim_cmds) = parser.feed(&msg.content);
                        if !clean_text.is_empty() {
                            let _ = app_handle.emit(
                                "llm-chunk",
                                LlmChunk { text: clean_text, done: false },
                            );
                        }
                        for cmd in anim_cmds {
                            let _ = app_handle.emit("llm-animation", cmd);
                        }
                    }
                }
                if parsed.done {
                    let (remaining_text, remaining_cmds) = parser.flush();
                    if !remaining_text.is_empty() {
                        let _ = app_handle.emit(
                            "llm-chunk",
                            LlmChunk { text: remaining_text, done: false },
                        );
                    }
                    for cmd in remaining_cmds {
                        let _ = app_handle.emit("llm-animation", cmd);
                    }
                    let _ = app_handle.emit(
                        "llm-chunk",
                        LlmChunk { text: String::new(), done: true },
                    );
                }
            }
        }
    }

    store_assistant_message(state, &strip_anim_blocks(&full_response), model)?;
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

    // ── AnimationCommand tests ────────────────────────────────────────────────

    #[test]
    fn animation_command_serializes() {
        let cmd = AnimationCommand {
            emotion: Some("happy".to_string()),
            motion: None,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("happy"));
        // "motion" key should be absent when None (skip_serializing_if)
        assert!(!json.contains(r#""motion""#));
    }

    #[test]
    fn animation_command_deserializes() {
        let json = r#"{"emotion":"happy","motion":"wave"}"#;
        let cmd: AnimationCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd.emotion, Some("happy".to_string()));
        assert_eq!(cmd.motion, Some("wave".to_string()));
    }

    #[test]
    fn animation_command_partial_fields() {
        let json = r#"{"emotion":"sad"}"#;
        let cmd: AnimationCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd.emotion, Some("sad".to_string()));
        assert_eq!(cmd.motion, None);
    }

    // ── StreamTagParser tests ─────────────────────────────────────────────────

    #[test]
    fn stream_tag_parser_basic() {
        let mut parser = StreamTagParser::new();
        let (text, cmds) = parser.feed(r#"<anim>{"emotion":"happy"}</anim>Hello!"#);
        assert_eq!(text, "Hello!");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].emotion, Some("happy".to_string()));
    }

    #[test]
    fn stream_tag_parser_no_anim() {
        let mut parser = StreamTagParser::new();
        let (text, cmds) = parser.feed("Just plain text");
        assert_eq!(text, "Just plain text");
        assert!(cmds.is_empty());
    }

    #[test]
    fn stream_tag_parser_split_across_chunks() {
        let mut parser = StreamTagParser::new();
        let (t1, c1) = parser.feed(r#"<anim>{"emot"#);
        assert_eq!(t1, "");
        assert!(c1.is_empty());

        let (t2, c2) = parser.feed(r#"ion":"happy"}</anim>Hi!"#);
        assert_eq!(t2, "Hi!");
        assert_eq!(c2.len(), 1);
        assert_eq!(c2[0].emotion, Some("happy".to_string()));
    }

    #[test]
    fn stream_tag_parser_partial_open_tag() {
        let mut parser = StreamTagParser::new();
        let (t1, c1) = parser.feed("Hello <ani");
        assert_eq!(t1, "Hello ");
        assert!(c1.is_empty());

        let (t2, _) = parser.flush();
        assert_eq!(t2, "<ani");
    }

    #[test]
    fn stream_tag_parser_flush_mid_anim() {
        let mut parser = StreamTagParser::new();
        parser.feed(r#"<anim>{"emotion":"ha"#);
        let (flushed, cmds) = parser.flush();
        assert!(flushed.contains("emotion"));
        assert!(cmds.is_empty());
    }

    #[test]
    fn stream_tag_parser_mixed_text_and_anim() {
        let mut parser = StreamTagParser::new();
        let (text, cmds) = parser.feed(
            r#"Before <anim>{"emotion":"sad","motion":"nod"}</anim>After"#,
        );
        assert_eq!(text, "Before After");
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].emotion, Some("sad".to_string()));
        assert_eq!(cmds[0].motion, Some("nod".to_string()));
    }

    #[test]
    fn stream_tag_parser_strips_newline_after_anim() {
        let mut parser = StreamTagParser::new();
        let (text, cmds) =
            parser.feed("<anim>{\"emotion\":\"happy\"}</anim>\nHello!");
        assert_eq!(text, "Hello!");
        assert_eq!(cmds.len(), 1);
    }

    #[test]
    fn stream_tag_parser_strips_newline_across_chunks() {
        let mut parser = StreamTagParser::new();
        let (t1, c1) = parser.feed("<anim>{\"emotion\":\"happy\"}</anim>");
        assert_eq!(t1, "");
        assert_eq!(c1.len(), 1);

        let (t2, _) = parser.feed("\nHello!");
        assert_eq!(t2, "Hello!");
    }

    // ── strip_anim_blocks tests ───────────────────────────────────────────────

    #[test]
    fn strip_anim_blocks_basic() {
        let input = "<anim>{\"emotion\":\"happy\"}</anim>\nHello world!";
        assert_eq!(strip_anim_blocks(input), "Hello world!");
    }

    #[test]
    fn strip_anim_blocks_no_anim() {
        assert_eq!(strip_anim_blocks("Just text"), "Just text");
    }

    #[test]
    fn strip_anim_blocks_multiple() {
        let input =
            "<anim>{\"emotion\":\"happy\"}</anim>\nHi! <anim>{\"motion\":\"wave\"}</anim>\nBye!";
        assert_eq!(strip_anim_blocks(input), "Hi! Bye!");
    }

    #[test]
    fn partial_prefix_len_matches() {
        assert_eq!(partial_prefix_len("hello<", "<anim>"), 1);
        assert_eq!(partial_prefix_len("hello<an", "<anim>"), 3);
        assert_eq!(partial_prefix_len("hello<anim", "<anim>"), 5);
        assert_eq!(partial_prefix_len("hello", "<anim>"), 0);
        assert_eq!(partial_prefix_len("", "<anim>"), 0);
    }
}
