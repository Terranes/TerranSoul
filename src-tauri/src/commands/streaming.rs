use crate::brain::brain_config::BrainMode;
use crate::brain::ollama_agent::{ChatMessage, OLLAMA_BASE_URL};
use crate::brain::openai_client::{OpenAiClient, OpenAiMessage};
use crate::persona::pose_frame::{parse_pose_payload, LlmPoseFrame};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

/// Decoupled event payload routed through the [`spawn_event_pump`] channel.
///
/// The LLM streaming loop calls `tx.send(StreamEvent::…)` (microsecond cost),
/// while a background Tokio task owns the [`tauri::AppHandle`] and emits each
/// event in arrival order. Without this decoupling, every token chunk had to
/// wait for `app.emit()` to JSON-serialise the payload and dispatch it into
/// every subscribed webview (chat + pet overlay + subtitle), which back-
/// pressured `stream.next().await` and slowed perceived token throughput on
/// LocalOllama.
#[derive(Debug)]
enum StreamEvent {
    Chunk(LlmChunk),
    Animation(AnimationCommand),
    Pose(LlmPoseFrame),
}

/// Spawn a background task that drains `rx` and emits Tauri events in order.
///
/// Returns:
/// - the `UnboundedSender<StreamEvent>` used by the streaming loop, and
/// - a [`tokio::task::JoinHandle`] the caller awaits **after dropping the
///   sender** to guarantee every queued event has been emitted before the
///   function returns (so `done:true` is never lost).
///
/// ## Three-stream coalescing
///
/// LocalOllama on a 3080 Ti emits 60+ tokens/sec, and `app.emit()` JSON-
/// serialises + dispatches to every subscribed webview (chat, pet overlay,
/// subtitles). Sending one event per token wastes IPC bandwidth — the
/// webview cannot render faster than vsync (~60 Hz) anyway.
///
/// The pump therefore *coalesces consecutive text chunks*: after waking on
/// `recv().await`, it drains every `Chunk` already queued via `try_recv()`
/// and concatenates their text into a single `llm-chunk` emit. The
/// `Animation` and `Pose` streams are **never coalesced** — each event
/// carries a discrete command/frame that must reach the renderer with its
/// original timing. When a non-Chunk event is encountered mid-drain, the
/// accumulated text is emitted first to preserve global arrival order,
/// then the non-Chunk event is emitted. The terminal `done:true` chunk is
/// also flushed eagerly so the frontend can finalize the message.
fn spawn_event_pump<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> (UnboundedSender<StreamEvent>, tokio::task::JoinHandle<()>) {
    let (tx, mut rx) = unbounded_channel::<StreamEvent>();
    let handle = tokio::spawn(async move {
        // Reusable scratch buffer for coalescing successive text chunks.
        let mut text_buf = String::with_capacity(256);
        let mut done_flag = false;

        // Helper closure-style flush: emit the accumulated text (if any)
        // as a single chunk event. `done` is honoured separately so the
        // terminal frame still arrives.
        let flush_text =
            |buf: &mut String, done: &mut bool, app: &tauri::AppHandle<R>| {
                if !buf.is_empty() || *done {
                    let _ = app.emit(
                        "llm-chunk",
                        LlmChunk {
                            text: std::mem::take(buf),
                            done: *done,
                        },
                    );
                    *done = false;
                }
            };

        while let Some(evt) = rx.recv().await {
            // Process the freshly-received event and any siblings already
            // in the channel queue, in arrival order.
            let mut current = Some(evt);
            loop {
                let evt = match current.take() {
                    Some(e) => e,
                    None => match rx.try_recv() {
                        Ok(e) => e,
                        Err(_) => break, // queue empty → wait for next recv
                    },
                };
                match evt {
                    StreamEvent::Chunk(c) => {
                        if c.done {
                            done_flag = true;
                        }
                        if !c.text.is_empty() {
                            text_buf.push_str(&c.text);
                        }
                        // Flush immediately on `done:true` so the
                        // frontend never waits for the next event to
                        // discover the stream ended.
                        if done_flag {
                            flush_text(&mut text_buf, &mut done_flag, &app);
                        }
                    }
                    StreamEvent::Animation(a) => {
                        // Preserve global order: emit any pending text
                        // before the animation command.
                        flush_text(&mut text_buf, &mut done_flag, &app);
                        let _ = app.emit("llm-animation", a);
                    }
                    StreamEvent::Pose(p) => {
                        flush_text(&mut text_buf, &mut done_flag, &app);
                        let _ = app.emit("llm-pose", p);
                    }
                }
            }
            // Queue drained — flush any leftover text before sleeping
            // again on `recv().await`. This is the natural batching
            // boundary: one emit per producer wake-up.
            flush_text(&mut text_buf, &mut done_flag, &app);
        }
        // Sender dropped — final flush.
        flush_text(&mut text_buf, &mut done_flag, &app);
    });
    (tx, handle)
}

/// Decide whether to skip the RAG pipeline (embed + hybrid search) for a
/// trivial query. Skipping saves 400-1500 ms on cold-embed paths because
/// loading the embed model would otherwise evict the chat model from VRAM.
///
/// Skip when ANY of the following is true:
///   - Memory store is empty (nothing to retrieve)
///   - Query is a short greeting / acknowledgment with no content words
///     (≤ 3 tokens AND every token is shorter than 5 chars), so no
///     keyword would match anyway
///
/// Returns `true` when RAG should be SKIPPED.
pub(crate) fn should_skip_rag(query: &str, memory_count: i64) -> bool {
    if memory_count == 0 {
        return true;
    }
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return true;
    }
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    // Trivial greeting heuristic: very short input where no token is long
    // enough to be a meaningful keyword. The hybrid scorer drops words
    // shorter than 3 chars; words ≤ 5 are usually stop-words for greetings
    // like "Hi", "Hello", "Thanks", "OK", "Yes", "No".
    if tokens.len() <= 3 && tokens.iter().all(|t| t.chars().count() <= 5) {
        return true;
    }
    false
}

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
    /// Emotion intensity in `[0, 1]`. Defaults to 1.0 when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intensity: Option<f32>,
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

/// Outputs from a single [`StreamTagParser::feed`] call.
#[derive(Debug, Default)]
pub struct StreamFeed {
    /// Clean text with all meta-tag blocks stripped.
    pub text: String,
    /// Parsed `<anim>{…}</anim>` blocks.
    pub anim_commands: Vec<AnimationCommand>,
    /// Parsed `<pose>{…}</pose>` blocks (validated + clamped).
    pub pose_frames: Vec<LlmPoseFrame>,
}

/// Which meta-tag block the parser is currently inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Anim,
    Pose,
}

impl BlockKind {
    fn open_tag(self) -> &'static str {
        match self {
            BlockKind::Anim => "<anim>",
            BlockKind::Pose => "<pose>",
        }
    }
    fn close_tag(self) -> &'static str {
        match self {
            BlockKind::Anim => "</anim>",
            BlockKind::Pose => "</pose>",
        }
    }
}

const ALL_BLOCKS: &[BlockKind] = &[BlockKind::Anim, BlockKind::Pose];

/// State-machine parser that extracts `<anim>{…}</anim>` and
/// `<pose>{…}</pose>` blocks from a stream of text chunks. Returns
/// clean text plus typed payloads — no regex needed on the frontend.
pub(crate) struct StreamTagParser {
    buffer: String,
    in_block: Option<BlockKind>,
    block_buffer: String,
    strip_next_newline: bool,
}

impl StreamTagParser {
    pub(crate) fn new() -> Self {
        Self {
            buffer: String::new(),
            in_block: None,
            block_buffer: String::new(),
            strip_next_newline: false,
        }
    }

    /// Feed a chunk of text. Returns clean text plus any complete blocks.
    pub(crate) fn feed(&mut self, chunk: &str) -> StreamFeed {
        self.buffer.push_str(chunk);

        // Strip leading newline left over from a previous </tag> boundary.
        if self.strip_next_newline && !self.buffer.is_empty() {
            if self.buffer.starts_with("\r\n") {
                self.buffer = self.buffer[2..].to_string();
            } else if self.buffer.starts_with('\n') {
                self.buffer = self.buffer[1..].to_string();
            }
            self.strip_next_newline = false;
        }

        let mut out = StreamFeed::default();

        loop {
            if let Some(kind) = self.in_block {
                let close = kind.close_tag();
                if let Some(end) = self.buffer.find(close) {
                    let json_part = self.buffer[..end].to_string();
                    self.block_buffer.push_str(&json_part);
                    let payload = std::mem::take(&mut self.block_buffer);
                    match kind {
                        BlockKind::Anim => {
                            if let Ok(cmd) =
                                serde_json::from_str::<AnimationCommand>(payload.trim())
                            {
                                out.anim_commands.push(cmd);
                            }
                        }
                        BlockKind::Pose => {
                            if let Ok(parsed) = parse_pose_payload(payload.trim()) {
                                out.pose_frames.push(parsed.frame);
                            }
                        }
                    }
                    self.in_block = None;
                    self.buffer = self.buffer[end + close.len()..].to_string();
                    // Strip one trailing newline so it doesn't leak into chat text.
                    if self.buffer.starts_with('\n') {
                        self.buffer = self.buffer[1..].to_string();
                    } else if self.buffer.starts_with("\r\n") {
                        self.buffer = self.buffer[2..].to_string();
                    } else if self.buffer.is_empty() {
                        self.strip_next_newline = true;
                    }
                } else {
                    // Close tag not yet seen — hold back any partial close.
                    let hold = partial_prefix_len(&self.buffer, close);
                    let safe = self.buffer.len() - hold;
                    self.block_buffer.push_str(&self.buffer[..safe]);
                    self.buffer = self.buffer[safe..].to_string();
                    break;
                }
            } else if let Some((start, kind)) = find_earliest_open(&self.buffer) {
                out.text.push_str(&self.buffer[..start]);
                self.buffer = self.buffer[start + kind.open_tag().len()..].to_string();
                self.in_block = Some(kind);
            } else {
                // Hold back any partial open tag at the end.
                let hold = ALL_BLOCKS
                    .iter()
                    .map(|k| partial_prefix_len(&self.buffer, k.open_tag()))
                    .max()
                    .unwrap_or(0);
                let safe = self.buffer.len() - hold;
                out.text.push_str(&self.buffer[..safe]);
                self.buffer = self.buffer[safe..].to_string();
                break;
            }
        }

        out
    }

    /// Flush remaining buffered content (call when the stream ends).
    pub(crate) fn flush(&mut self) -> StreamFeed {
        let remaining = std::mem::take(&mut self.buffer);
        let block_remaining = std::mem::take(&mut self.block_buffer);
        let in_block = self.in_block.take();
        self.strip_next_newline = false;
        let mut out = StreamFeed::default();
        // If we were mid-block, the content is malformed — emit as text
        // (re-prepend the original opener so the user sees what happened).
        if !block_remaining.is_empty() || in_block.is_some() {
            let opener = in_block.map(|k| k.open_tag()).unwrap_or("");
            out.text = format!("{opener}{block_remaining}{remaining}");
        } else {
            out.text = remaining;
        }
        out
    }
}

/// Find the earliest opening tag in `buffer` and return its byte offset
/// and which block kind it is. `None` if no opener is present.
fn find_earliest_open(buffer: &str) -> Option<(usize, BlockKind)> {
    let mut best: Option<(usize, BlockKind)> = None;
    for kind in ALL_BLOCKS {
        if let Some(idx) = buffer.find(kind.open_tag()) {
            match best {
                Some((b, _)) if b <= idx => {}
                _ => best = Some((idx, *kind)),
            }
        }
    }
    best
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

/// Strip `<anim>...</anim>` and `<pose>...</pose>` blocks from a
/// completed response (for storage).
pub(crate) fn strip_anim_blocks(input: &str) -> String {
    let mut result = String::new();
    let mut remaining = input;
    loop {
        let next = ALL_BLOCKS
            .iter()
            .filter_map(|k| remaining.find(k.open_tag()).map(|i| (i, *k)))
            .min_by_key(|(i, _)| *i);
        let Some((start, kind)) = next else { break };
        result.push_str(&remaining[..start]);
        remaining = &remaining[start + kind.open_tag().len()..];
        if let Some(end) = remaining.find(kind.close_tag()) {
            remaining = &remaining[end + kind.close_tag().len()..];
            // Skip one trailing newline.
            if remaining.starts_with('\n') {
                remaining = &remaining[1..];
            } else if remaining.starts_with("\r\n") {
                remaining = &remaining[2..];
            }
        } else {
            // Unclosed block — stop and treat the rest as text.
            result.push_str(remaining);
            return result.trim().to_string();
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
    run_chat_stream(message, &app_handle, state.inner()).await
}

/// Headless / testable entry point for the streaming chat pipeline.
///
/// This contains the entire body of [`send_message_stream`] and is exposed
/// so integration tests can drive the full flow against an in-process
/// [`tauri::test::MockRuntime`] `AppHandle` without going through Tauri IPC.
pub async fn run_chat_stream<R: tauri::Runtime>(
    message: String,
    app_handle: &tauri::AppHandle<R>,
    state: &AppState,
) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("Message cannot be empty".to_string());
    }

    // Mark chat activity so background LocalOllama embeddings stay paused.
    state.mark_chat_activity_now();

    // Add user message to conversation
    let user_msg = crate::commands::chat::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "user".to_string(),
        content: message.clone(),
        agent_name: None,
        agent_id: None,
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
    let brain_mode: Option<BrainMode> =
        { state.brain_mode.lock().map_err(|e| e.to_string())?.clone() };

    let legacy_model: Option<String> = {
        state
            .active_brain
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
    };

    // Build conversation history. LocalOllama/LM Studio use a shorter
    // window (10 messages) so prompt evaluation stays fast — every extra
    // history turn adds prompt-eval latency on consumer GPUs.
    let is_local = matches!(
        brain_mode,
        Some(BrainMode::LocalOllama { .. }) | Some(BrainMode::LocalLmStudio { .. })
    );
    let history_limit = if is_local { 10 } else { 20 };
    let history: Vec<(String, String)> = {
        let conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .rev()
            .take(history_limit)
            .rev()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    };

    match brain_mode {
        Some(BrainMode::FreeApi {
            provider_id,
            api_key,
            model,
        }) => {
            // Build failover chain: ordered list of providers to try
            let failover_chain = {
                let mut rotator = state.provider_rotator.lock().map_err(|e| e.to_string())?;
                let policy = state
                    .failover_policy
                    .lock()
                    .map(|p| p.clone())
                    .unwrap_or_default();
                let constraints = crate::brain::SelectionConstraints::default();
                let mut chain = rotator.select_failover_chain(&constraints, &policy);
                // Ensure the configured provider is first if it's not already in the chain
                if !chain.contains(&provider_id) {
                    chain.insert(0, provider_id.clone());
                }
                chain
            };

            // Retry loop: try each provider in the failover chain
            let mut attempts: Vec<crate::brain::FailoverAttempt> = Vec::new();
            let original_provider_id = failover_chain
                .first()
                .cloned()
                .unwrap_or_else(|| provider_id.clone());

            for candidate_id in &failover_chain {
                let result = stream_openai_api(
                    app_handle,
                    state,
                    &message,
                    &history,
                    candidate_id,
                    api_key.as_deref(),
                    model.as_deref().filter(|_| candidate_id == &provider_id),
                    None,
                )
                .await;

                match result {
                    Ok(()) => {
                        // Emit failover decision if we retried
                        if !attempts.is_empty() {
                            let decision = crate::brain::FailoverDecision {
                                original_provider_id: original_provider_id.clone(),
                                final_provider_id: candidate_id.clone(),
                                attempts: attempts.clone(),
                                succeeded: true,
                            };
                            let _ = app_handle.emit("provider-failover", &decision);
                        }
                        return Ok(());
                    }
                    Err(e) => {
                        let err_lower = e.to_lowercase();
                        let reason =
                            if err_lower.contains("429") || err_lower.contains("rate limit") {
                                // Mark rate-limited with cooldown
                                let policy = state
                                    .failover_policy
                                    .lock()
                                    .map(|p| p.clone())
                                    .unwrap_or_default();
                                let mut rotator =
                                    state.provider_rotator.lock().map_err(|er| er.to_string())?;
                                rotator.record_rate_limit_with_cooldown(
                                    candidate_id,
                                    policy.min_cooldown_secs,
                                );
                                crate::brain::FailoverReason::RateLimit
                            } else if err_lower.contains("context")
                                || err_lower.contains("too long")
                                || err_lower.contains("maximum")
                            {
                                crate::brain::FailoverReason::ContextOverflow {
                                    estimated_tokens: 0,
                                    provider_max: 0,
                                }
                            } else {
                                // Generic unhealthy (timeout, network, 5xx)
                                let mut rotator =
                                    state.provider_rotator.lock().map_err(|er| er.to_string())?;
                                if let Some(status) = rotator.providers.get_mut(candidate_id) {
                                    status.is_healthy = false;
                                }
                                crate::brain::FailoverReason::Unhealthy
                            };

                        attempts.push(crate::brain::FailoverAttempt {
                            provider_id: candidate_id.clone(),
                            reason,
                        });

                        // Continue to next candidate
                    }
                }
            }

            // All candidates exhausted — emit final done signal
            let _ = app_handle.emit(
                "llm-chunk",
                LlmChunk {
                    text: String::new(),
                    done: true,
                },
            );
            let decision = crate::brain::FailoverDecision {
                original_provider_id: original_provider_id.clone(),
                final_provider_id: String::new(),
                attempts: attempts.clone(),
                succeeded: false,
            };
            let _ = app_handle.emit("provider-failover", &decision);
            let _ = app_handle.emit("providers-exhausted", ());
            Err(format!(
                "All providers exhausted after {} attempts",
                attempts.len()
            ))
        }
        Some(BrainMode::PaidApi {
            provider: _,
            api_key,
            model,
            base_url,
        }) => {
            stream_openai_api(
                app_handle,
                state,
                &message,
                &history,
                "paid",
                Some(&api_key),
                None,
                Some((&base_url, &model)),
            )
            .await
        }
        Some(BrainMode::LocalOllama { model }) => {
            stream_ollama(app_handle, state, &message, &history, &model).await
        }
        Some(BrainMode::LocalLmStudio {
            model,
            base_url,
            api_key,
            ..
        }) => {
            stream_openai_api(
                app_handle,
                state,
                &message,
                &history,
                "lm_studio",
                api_key.as_deref(),
                None,
                Some((&base_url, &model)),
            )
            .await
        }
        None => {
            // Check legacy active_brain
            if let Some(model) = legacy_model {
                stream_ollama(app_handle, state, &message, &history, &model).await
            } else {
                // No brain — emit stub response
                emit_stub_response(app_handle, state, &message)
            }
        }
    }
}

/// Stream via an OpenAI-compatible API (used for FreeApi and PaidApi modes).
async fn stream_openai_api<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    state: &AppState,
    _message: &str,
    history: &[(String, String)],
    provider_id: &str,
    api_key: Option<&str>,
    free_model_override: Option<&str>,
    paid_override: Option<(&str, &str)>, // (base_url, model) for paid API
) -> Result<(), String> {
    // Resolve base_url and model
    let (base_url, model) = if let Some((url, mdl)) = paid_override {
        (url.to_string(), mdl.to_string())
    } else {
        // Look up free provider
        let provider = crate::brain::get_free_provider(provider_id)
            .ok_or_else(|| format!("Unknown free provider: {provider_id}"))?;
        (
            provider.base_url,
            free_model_override.unwrap_or(&provider.model).to_string(),
        )
    };

    let client = OpenAiClient::new(&base_url, &model, api_key);

    // ── RAG: retrieve relevant memories via hybrid search ─────────────
    let mut system_prompt = super::chat::SYSTEM_PROMPT_FOR_STREAMING.to_string();

    let user_query = history
        .iter()
        .rev()
        .find(|(role, _)| role == "user")
        .map(|(_, content)| content.as_str())
        .unwrap_or(_message);

    // Hybrid search: combines keyword + recency + importance + decay scoring.
    // Chunk 16.9: use cloud embedding for the vector component so cloud modes
    // get real vector RAG instead of keyword-only retrieval.
    // Memories below the user-tunable relevance threshold are skipped
    // (Chunk 16.1 — see docs/brain-advanced-design.md § 16 Phase 4).
    //
    // RAG gate (perf): skip embed + search for trivial queries to save a
    // cloud round-trip on greetings and acknowledgments.
    let memory_count = state.memory_store.lock().map(|s| s.count()).unwrap_or(0);
    let skip_rag = should_skip_rag(user_query, memory_count);

    let brain_mode = state.brain_mode.lock().ok().and_then(|g| g.clone());
    let active_brain = state.active_brain.lock().ok().and_then(|g| g.clone());
    let query_emb = if skip_rag {
        None
    } else {
        crate::brain::embed_for_mode(user_query, brain_mode.as_ref(), active_brain.as_deref()).await
    };

    let threshold = state
        .app_settings
        .lock()
        .map(|s| s.relevance_threshold)
        .unwrap_or(crate::settings::DEFAULT_RELEVANCE_THRESHOLD);
    let relevant: Vec<crate::memory::MemoryEntry> = if skip_rag {
        vec![]
    } else {
        match state.memory_store.lock() {
            Ok(store) => store
                .hybrid_search_with_threshold(user_query, query_emb.as_deref(), 5, threshold)
                .unwrap_or_default(),
            Err(_) => vec![],
        }
    };

    if !relevant.is_empty() {
        let memory_block: String = relevant
            .iter()
            .map(|e| format!("- [{}] {}", e.tier.as_str(), e.content))
            .collect::<Vec<_>>()
            .join("\n");
        system_prompt.push_str(&crate::memory::format_retrieved_context_pack(&memory_block));
    }

    // ── Persona block (see docs/persona-design.md § 9.1) ──────────────
    // Pushed from the frontend persona store via `set_persona_block`.
    // Empty string means no persona injection.
    if let Ok(persona) = state.persona_block.lock() {
        if !persona.is_empty() {
            system_prompt.push_str(persona.as_str());
        }
    }

    // ── Handoff block (Chunk 23.2b) ──────────────────────────────────
    // Pushed from the frontend agent-roster store on agent switch.
    // **One-shot**: read-and-clear so the new agent is briefed exactly
    // once, then operates on its own thread for subsequent turns.
    if let Ok(mut handoff) = state.handoff_block.lock() {
        if !handoff.is_empty() {
            system_prompt.push_str(handoff.as_str());
            handoff.clear();
        }
    }

    // Build OpenAI message array
    let mut messages = vec![OpenAiMessage {
        role: "system".to_string(),
        content: system_prompt,
    }];
    for (role, content) in history {
        messages.push(OpenAiMessage {
            role: role.clone(),
            content: content.clone(),
        });
    }

    // Stream with callback — parser separates text from <anim> blocks.
    // The callback pushes events into a non-blocking channel so each token
    // returns control to `stream.next().await` immediately; a background
    // pump task owns the AppHandle and emits in arrival order.
    let (tx, pump) = spawn_event_pump(app_handle.clone());
    let parser = std::sync::Arc::new(std::sync::Mutex::new(StreamTagParser::new()));
    let parser_cb = std::sync::Arc::clone(&parser);
    let tx_cb = tx.clone();
    let result = client
        .chat_stream(messages, move |chunk_text| {
            let mut p = parser_cb.lock().unwrap();
            let feed = p.feed(chunk_text);
            if !feed.text.is_empty() {
                let _ = tx_cb.send(StreamEvent::Chunk(LlmChunk {
                    text: feed.text,
                    done: false,
                }));
            }
            for cmd in feed.anim_commands {
                let _ = tx_cb.send(StreamEvent::Animation(cmd));
            }
            for frame in feed.pose_frames {
                let _ = tx_cb.send(StreamEvent::Pose(frame));
            }
        })
        .await;

    match result {
        Ok(full_response) => {
            // Flush any remaining buffered content from the parser.
            {
                let mut p = parser.lock().unwrap();
                let feed = p.flush();
                if !feed.text.is_empty() {
                    let _ = tx.send(StreamEvent::Chunk(LlmChunk {
                        text: feed.text,
                        done: false,
                    }));
                }
                for cmd in feed.anim_commands {
                    let _ = tx.send(StreamEvent::Animation(cmd));
                }
                for frame in feed.pose_frames {
                    let _ = tx.send(StreamEvent::Pose(frame));
                }
            }
            let _ = tx.send(StreamEvent::Chunk(LlmChunk {
                text: String::new(),
                done: true,
            }));
            // Drop sender + await the pump so all events are flushed in
            // arrival order before we return to the frontend.
            drop(tx);
            let _ = pump.await;
            // Record successful request in rotator
            {
                let mut rotator = state.provider_rotator.lock().map_err(|e| e.to_string())?;
                rotator
                    .providers
                    .entry(provider_id.to_string())
                    .and_modify(|s| {
                        s.requests_sent += 1;
                    });
            }
            store_assistant_message(state, &strip_anim_blocks(&full_response), &model)?;
            Ok(())
        }
        Err(e) => {
            // Drain pump before bubbling the error so any buffered chunks
            // emitted before the failure still reach the frontend.
            drop(tx);
            let _ = pump.await;
            // Don't emit done:true here — the caller (retry loop) decides
            // whether to emit it after all attempts are exhausted.
            // Record rate limit if applicable
            let err_lower = e.to_string().to_lowercase();
            if err_lower.contains("429") || err_lower.contains("rate limit") {
                let mut rotator = state.provider_rotator.lock().map_err(|er| er.to_string())?;
                rotator.record_rate_limit(provider_id);
            }
            Err(format!("Free API error: {e}"))
        }
    }
}

/// Stream via local Ollama (NDJSON format).
async fn stream_ollama<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    state: &AppState,
    _message: &str,
    history: &[(String, String)],
    model: &str,
) -> Result<(), String> {
    // ── Local RAG: hybrid search with vector + keyword + decay ────────
    let mut system_prompt = super::chat::SYSTEM_PROMPT_FOR_STREAMING.to_string();

    let user_query = history
        .iter()
        .rev()
        .find(|(role, _)| role == "user")
        .map(|(_, content)| content.as_str())
        .unwrap_or(_message);

    // Hybrid search: keyword + recency + importance + decay scoring.
    // IMPORTANT: For LocalOllama we NEVER call embed_text() in the
    // streaming hot-path. Loading the embedding model (nomic-embed-text)
    // would evict the chat model from VRAM, causing 5-15s cold-start
    // on the next chat generation. Instead we rely on keyword-only
    // hybrid search which is near-instant (<5ms) and avoids any model
    // contention. This guarantees <1s TTFT for ALL messages.
    let memory_count = state.memory_store.lock().map(|s| s.count()).unwrap_or(0);
    // Skip RAG entirely for content-light turns (e.g. "Hi", "ok", "?")
    // — they don't benefit from memory retrieval and any work here adds
    // latency to the first chunk.
    let skip_rag = should_skip_rag(user_query, memory_count);

    let threshold = state
        .app_settings
        .lock()
        .map(|s| s.relevance_threshold)
        .unwrap_or(crate::settings::DEFAULT_RELEVANCE_THRESHOLD);
    let relevant: Vec<crate::memory::MemoryEntry> = if skip_rag {
        vec![]
    } else {
        match state.memory_store.lock() {
            Ok(store) => store
                .hybrid_search_with_threshold(user_query, None, 5, threshold)
                .unwrap_or_default(),
            Err(_) => vec![],
        }
    };

    if !relevant.is_empty() {
        let memory_block: String = relevant
            .iter()
            .map(|e| format!("- [{}] {}", e.tier.as_str(), e.content))
            .collect::<Vec<_>>()
            .join("\n");
        system_prompt.push_str(&crate::memory::format_retrieved_context_pack(&memory_block));
    }

    // ── Persona block (see docs/persona-design.md § 9.1) ──────────────
    if let Ok(persona) = state.persona_block.lock() {
        if !persona.is_empty() {
            system_prompt.push_str(persona.as_str());
        }
    }

    // ── Handoff block (Chunk 23.2b) ──────────────────────────────────
    // One-shot read-and-clear; same pattern as the OpenAI path above.
    if let Ok(mut handoff) = state.handoff_block.lock() {
        if !handoff.is_empty() {
            system_prompt.push_str(handoff.as_str());
            handoff.clear();
        }
    }

    // Build Ollama message array
    let system_msg = ChatMessage {
        role: "system".to_string(),
        content: system_prompt,
    };
    let mut messages = vec![system_msg];
    // Cap individual history messages to avoid a single long response
    // consuming the entire context window. ~200 chars ≈ 50 tokens.
    const MAX_HISTORY_MSG_CHARS: usize = 800;
    for (role, content) in history {
        let trimmed = if content.len() > MAX_HISTORY_MSG_CHARS {
            format!("{}…", &content[..MAX_HISTORY_MSG_CHARS])
        } else {
            content.clone()
        };
        messages.push(ChatMessage {
            role: role.clone(),
            content: trimmed,
        });
    }

    let url = format!("{OLLAMA_BASE_URL}/api/chat");
    // CRITICAL: Use a fixed num_ctx (2048) for ALL Ollama chat requests.
    // Changing num_ctx between requests forces Ollama to reallocate the
    // KV cache and reload the model weights — adding 2-3s per request.
    // 2048 is enough for 10-message history + system prompt + persona.
    let max_tokens: u32 = if skip_rag { 256 } else { 512 };
    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "think": false,
        "keep_alive": "30m",
        "options": {
            "temperature": 0.7,
            "num_ctx": 2048,
            "num_predict": max_tokens,
            "num_batch": 512,
        },
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

    // Decouple emit work from the LLM token loop: a background pump owns
    // the AppHandle and emits events in arrival order, so each token
    // returns control to `stream.next().await` immediately even when
    // multiple webviews are subscribed.
    let (tx, pump) = spawn_event_pump(app_handle.clone());

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
                        let feed = parser.feed(&msg.content);
                        if !feed.text.is_empty() {
                            let _ = tx.send(StreamEvent::Chunk(LlmChunk {
                                text: feed.text,
                                done: false,
                            }));
                        }
                        for cmd in feed.anim_commands {
                            let _ = tx.send(StreamEvent::Animation(cmd));
                        }
                        for frame in feed.pose_frames {
                            let _ = tx.send(StreamEvent::Pose(frame));
                        }
                    }
                }
                if parsed.done {
                    let feed = parser.flush();
                    if !feed.text.is_empty() {
                        let _ = tx.send(StreamEvent::Chunk(LlmChunk {
                            text: feed.text,
                            done: false,
                        }));
                    }
                    for cmd in feed.anim_commands {
                        let _ = tx.send(StreamEvent::Animation(cmd));
                    }
                    for frame in feed.pose_frames {
                        let _ = tx.send(StreamEvent::Pose(frame));
                    }
                    let _ = tx.send(StreamEvent::Chunk(LlmChunk {
                        text: String::new(),
                        done: true,
                    }));
                }
            }
        }
    }

    // Drop sender + await the pump so every queued event is delivered
    // (including the final done:true) before we return.
    drop(tx);
    let _ = pump.await;

    store_assistant_message(state, &strip_anim_blocks(&full_response), model)?;
    Ok(())
}

// ── Self-RAG iterative streaming (Chunk 16.4b) ────────────────────────────────

/// Stream a chat response using Self-RAG iterative refinement.
///
/// The loop:
/// 1. Retrieve memories via hybrid search
/// 2. Prompt LLM with Self-RAG system prompt (asks for reflection tokens)
/// 3. Collect full response, stream tokens to user in real-time
/// 4. Run `SelfRagController::next_step()` on the complete response
/// 5. On `Retrieve` → re-embed, re-retrieve, re-prompt (loop)
/// 6. On `Accept` → emit final cleaned answer
/// 7. On `Reject` → emit graceful refusal
///
/// Only available for LocalOllama mode (needs embeddings for re-retrieval).
#[tauri::command]
pub async fn send_message_stream_self_rag(
    message: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    run_self_rag_stream(message, &app_handle, state.inner()).await
}

/// Testable entry point for the Self-RAG streaming pipeline.
pub async fn run_self_rag_stream<R: tauri::Runtime>(
    message: String,
    app_handle: &tauri::AppHandle<R>,
    state: &AppState,
) -> Result<(), String> {
    use crate::orchestrator::self_rag::{
        Decision, RejectReason, SelfRagController, SELF_RAG_SYSTEM_PROMPT,
    };
    use futures_util::StreamExt;

    if message.trim().is_empty() {
        return Err("Message cannot be empty".to_string());
    }

    // Add user message to conversation
    let user_msg = crate::commands::chat::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "user".to_string(),
        content: message.clone(),
        agent_name: None,
        agent_id: None,
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

    // Self-RAG requires local Ollama (needs embeddings for re-retrieval).
    let model = {
        let brain_mode = state.brain_mode.lock().map_err(|e| e.to_string())?;
        match brain_mode.as_ref() {
            Some(BrainMode::LocalOllama { model }) => model.clone(),
            _ => {
                // Fall back to legacy active_brain for backward compat
                state
                    .active_brain
                    .lock()
                    .map_err(|e| e.to_string())?
                    .clone()
                    .ok_or_else(|| {
                        "Self-RAG requires a local Ollama model to be configured".to_string()
                    })?
            }
        }
    };

    // Build conversation history (10 messages — self-RAG is local-only)
    let history: Vec<(String, String)> = {
        let conv = state.conversation.lock().map_err(|e| e.to_string())?;
        conv.iter()
            .rev()
            .take(10)
            .rev()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect()
    };

    let threshold = state
        .app_settings
        .lock()
        .map(|s| s.relevance_threshold)
        .unwrap_or(crate::settings::DEFAULT_RELEVANCE_THRESHOLD);

    let mut controller = SelfRagController::new();

    // Decouple emit work from the LLM token loop across all Self-RAG
    // iterations: a background pump owns the AppHandle and emits in
    // arrival order so each token returns control immediately.
    let (tx, pump) = spawn_event_pump(app_handle.clone());

    let final_answer: String = loop {
        // ── Step 1: Embed + retrieve ──────────────────────────────────
        let query_emb = crate::brain::OllamaAgent::embed_text(&message, &model).await;

        let relevant: Vec<crate::memory::MemoryEntry> = {
            match state.memory_store.lock() {
                Ok(store) => store
                    .hybrid_search_with_threshold(&message, query_emb.as_deref(), 5, threshold)
                    .unwrap_or_default(),
                Err(_) => vec![],
            }
        };

        // ── Step 2: Build system prompt with RAG + Self-RAG addendum ──
        let mut system_prompt = super::chat::SYSTEM_PROMPT_FOR_STREAMING.to_string();

        if !relevant.is_empty() {
            let memory_block: String = relevant
                .iter()
                .map(|e| format!("- [{}] {}", e.tier.as_str(), e.content))
                .collect::<Vec<_>>()
                .join("\n");
            system_prompt.push_str(&crate::memory::format_retrieved_context_pack(&memory_block));
        }

        // Append Self-RAG reflection instructions
        system_prompt.push_str("\n\n");
        system_prompt.push_str(SELF_RAG_SYSTEM_PROMPT);

        // Persona block
        if let Ok(persona) = state.persona_block.lock() {
            if !persona.is_empty() {
                system_prompt.push_str(persona.as_str());
            }
        }

        // ── Step 3: Stream LLM response ───────────────────────────────
        let system_msg = ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        };
        let mut messages = vec![system_msg];
        for (role, content) in &history {
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
            "think": false,
            "keep_alive": "30m",
            "options": {
                "num_predict": 150,
                "num_ctx": 2048,
                "num_batch": 512,
                "temperature": 0.7,
            },
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
                            let feed = parser.feed(&msg.content);
                            if !feed.text.is_empty() {
                                let _ = tx.send(StreamEvent::Chunk(LlmChunk {
                                    text: feed.text,
                                    done: false,
                                }));
                            }
                            for cmd in feed.anim_commands {
                                let _ = tx.send(StreamEvent::Animation(cmd));
                            }
                            for frame in feed.pose_frames {
                                let _ = tx.send(StreamEvent::Pose(frame));
                            }
                        }
                    }
                    if parsed.done {
                        let feed = parser.flush();
                        if !feed.text.is_empty() {
                            let _ = tx.send(StreamEvent::Chunk(LlmChunk {
                                text: feed.text,
                                done: false,
                            }));
                        }
                        for cmd in feed.anim_commands {
                            let _ = tx.send(StreamEvent::Animation(cmd));
                        }
                        for frame in feed.pose_frames {
                            let _ = tx.send(StreamEvent::Pose(frame));
                        }
                    }
                }
            }
        }

        // ── Step 4: Evaluate via Self-RAG controller ──────────────────
        let decision = controller.next_step(&full_response);

        match decision {
            Decision::Accept { answer } => {
                break answer;
            }
            Decision::Reject { reason } => {
                let refusal = match reason {
                    RejectReason::MaxIterationsExceeded => {
                        "I wasn't able to find a well-supported answer after multiple attempts. Let me know if you'd like me to try a different approach.".to_string()
                    }
                    RejectReason::Unsupported => {
                        "I don't have enough information in my memory to give you a reliable answer to that question.".to_string()
                    }
                };
                let _ = tx.send(StreamEvent::Chunk(LlmChunk {
                    text: refusal.clone(),
                    done: false,
                }));
                break refusal;
            }
            Decision::Retrieve => {
                // Emit a brief indicator that we're re-retrieving
                let _ = tx.send(StreamEvent::Chunk(LlmChunk {
                    text: "\n\n---\n*Refining answer with additional context...*\n\n"
                        .to_string(),
                    done: false,
                }));
                // Loop continues — next iteration re-embeds and re-retrieves
            }
        }
    };

    // ── Final: emit done signal and store message ─────────────────────
    let _ = tx.send(StreamEvent::Chunk(LlmChunk {
        text: String::new(),
        done: true,
    }));

    // Drop the sender + drain the pump so every queued event reaches the
    // frontend before we return.
    drop(tx);
    let _ = pump.await;

    store_assistant_message(state, &strip_anim_blocks(&final_answer), &model)?;

    Ok(())
}

/// Emit a stub response when no brain is configured.
fn emit_stub_response<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    state: &AppState,
    message: &str,
) -> Result<(), String> {
    let stub_text = format!("I hear you! You said: \"{message}\". I'm still learning, but I'm always here to listen and help!");
    let _ = app_handle.emit(
        "llm-chunk",
        LlmChunk {
            text: stub_text.clone(),
            done: false,
        },
    );
    let _ = app_handle.emit(
        "llm-chunk",
        LlmChunk {
            text: String::new(),
            done: true,
        },
    );

    let assistant_msg = crate::commands::chat::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: stub_text,
        agent_name: Some("TerranSoul".to_string()),
        agent_id: None,
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
        agent_id: None,
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
    fn skip_rag_when_memory_empty() {
        assert!(should_skip_rag("Anything goes here", 0));
    }

    #[test]
    fn skip_rag_for_short_greetings() {
        assert!(should_skip_rag("Hi", 1000));
        assert!(should_skip_rag("Hello", 1000));
        assert!(should_skip_rag("Hey", 1000));
        assert!(should_skip_rag("Yes", 1000));
        assert!(should_skip_rag("No", 1000));
        assert!(should_skip_rag("OK", 1000));
        assert!(should_skip_rag("ok", 1000));
        assert!(should_skip_rag("?", 1000));
    }

    #[test]
    fn skip_rag_for_empty_or_whitespace() {
        assert!(should_skip_rag("", 1000));
        assert!(should_skip_rag("   ", 1000));
    }

    #[test]
    fn run_rag_for_real_questions() {
        // Has at least one ≥5-char content word — RAG runs
        assert!(!should_skip_rag("explain Vietnamese contract law", 1000));
        assert!(!should_skip_rag("what is HyDE retrieval", 1000));
        assert!(!should_skip_rag("Hello there friend", 1000));
        assert!(!should_skip_rag("Hi there explain things", 1000));
    }

    /// Verify [`spawn_event_pump`] preserves the text content and order in
    /// which events are pushed by the streaming loop. The pump now
    /// *coalesces* consecutive text chunks (multiple tokens may collapse
    /// into a single `llm-chunk` emit), so we assert on the concatenated
    /// text rather than the emit count. The `done:true` sentinel must
    /// still arrive exactly once, as the final event.
    #[tokio::test]
    async fn event_pump_preserves_order_and_drains_on_close() {
        use tauri::test::{mock_builder, mock_context, noop_assets};
        use tauri::Listener;

        let app = mock_builder()
            .build(mock_context(noop_assets()))
            .expect("mock_builder build");
        let handle = app.handle().clone();

        let received: std::sync::Arc<std::sync::Mutex<Vec<LlmChunk>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let done = std::sync::Arc::new(tokio::sync::Notify::new());

        let received_cb = std::sync::Arc::clone(&received);
        let done_cb = std::sync::Arc::clone(&done);
        handle.listen("llm-chunk", move |event| {
            if let Ok(c) = serde_json::from_str::<LlmChunk>(event.payload()) {
                let is_done = c.done;
                received_cb.lock().unwrap().push(c);
                if is_done {
                    done_cb.notify_one();
                }
            }
        });

        let (tx, pump) = spawn_event_pump(handle.clone());

        // Push 50 text events fast — the pump may coalesce them.
        let mut expected_text = String::new();
        for i in 0..50 {
            let t = format!("t{i}");
            expected_text.push_str(&t);
            tx.send(StreamEvent::Chunk(LlmChunk {
                text: t,
                done: false,
            }))
            .expect("send chunk");
        }
        tx.send(StreamEvent::Chunk(LlmChunk {
            text: String::new(),
            done: true,
        }))
        .expect("send done");

        drop(tx);
        pump.await.expect("pump drain");

        // Wait for the listener thread to observe the done sentinel.
        tokio::select! {
            _ = done.notified() => {}
            _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                panic!("done:true never observed by listener");
            }
        }

        let observed = received.lock().unwrap().clone();
        assert!(!observed.is_empty(), "expected at least one chunk emit");
        // Coalescing should reduce 51 input events to far fewer emits.
        assert!(
            observed.len() <= 51,
            "coalescing should not inflate event count, got {}",
            observed.len()
        );
        // Concatenated text must match input order exactly.
        let joined: String = observed.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(
            joined, expected_text,
            "coalesced text must preserve content + order"
        );
        // done:true must be present exactly once, as the final emit.
        let done_positions: Vec<usize> = observed
            .iter()
            .enumerate()
            .filter(|(_, c)| c.done)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(done_positions, vec![observed.len() - 1]);
    }

    /// Coalescing must collapse a tight burst of token chunks into far
    /// fewer Tauri emits while preserving every byte of text. This is
    /// the perf guard for the three-stream LocalOllama path.
    #[tokio::test]
    async fn event_pump_coalesces_consecutive_chunks() {
        use tauri::test::{mock_builder, mock_context, noop_assets};
        use tauri::Listener;

        let app = mock_builder()
            .build(mock_context(noop_assets()))
            .expect("mock_builder build");
        let handle = app.handle().clone();

        let received: std::sync::Arc<std::sync::Mutex<Vec<LlmChunk>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let received_cb = std::sync::Arc::clone(&received);
        handle.listen("llm-chunk", move |event| {
            if let Ok(c) = serde_json::from_str::<LlmChunk>(event.payload()) {
                received_cb.lock().unwrap().push(c);
            }
        });

        let (tx, pump) = spawn_event_pump(handle.clone());
        // Send a burst that should be in the channel before the pump
        // task wakes — try_recv will drain them into one emit.
        let mut expected = String::new();
        for i in 0..200 {
            let t = format!("[{i}]");
            expected.push_str(&t);
            tx.send(StreamEvent::Chunk(LlmChunk {
                text: t,
                done: false,
            }))
            .expect("send");
        }
        tx.send(StreamEvent::Chunk(LlmChunk {
            text: String::new(),
            done: true,
        }))
        .expect("send done");
        drop(tx);
        pump.await.expect("pump drain");

        // Give the listener thread a moment to drain.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let observed = received.lock().unwrap().clone();
        let joined: String = observed.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(joined, expected, "no text byte may be lost");
        // 200 input chunks ⇒ should collapse to dramatically fewer emits.
        // We allow up to 20 (a generous bound; in practice it is 1-3).
        assert!(
            observed.len() <= 20,
            "expected coalescing to collapse 201 input events into ≤20 emits, got {}",
            observed.len()
        );
    }

    /// Animation events must NOT be coalesced — each command needs to
    /// reach the renderer with its original timing. Verify that two
    /// animation events between text bursts both arrive intact.
    #[tokio::test]
    async fn event_pump_does_not_coalesce_animations() {
        use tauri::test::{mock_builder, mock_context, noop_assets};
        use tauri::Listener;

        let app = mock_builder()
            .build(mock_context(noop_assets()))
            .expect("mock_builder build");
        let handle = app.handle().clone();

        let anims: std::sync::Arc<std::sync::Mutex<Vec<AnimationCommand>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let anims_cb = std::sync::Arc::clone(&anims);
        handle.listen("llm-animation", move |event| {
            if let Ok(c) = serde_json::from_str::<AnimationCommand>(event.payload()) {
                anims_cb.lock().unwrap().push(c);
            }
        });

        let (tx, pump) = spawn_event_pump(handle.clone());
        tx.send(StreamEvent::Chunk(LlmChunk {
            text: "before".to_string(),
            done: false,
        }))
        .unwrap();
        tx.send(StreamEvent::Animation(AnimationCommand {
            emotion: Some("happy".to_string()),
            motion: None,
            intensity: None,
        }))
        .unwrap();
        tx.send(StreamEvent::Chunk(LlmChunk {
            text: "middle".to_string(),
            done: false,
        }))
        .unwrap();
        tx.send(StreamEvent::Animation(AnimationCommand {
            emotion: Some("surprised".to_string()),
            motion: None,
            intensity: None,
        }))
        .unwrap();
        tx.send(StreamEvent::Chunk(LlmChunk {
            text: "after".to_string(),
            done: true,
        }))
        .unwrap();
        drop(tx);
        pump.await.expect("pump drain");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let observed = anims.lock().unwrap().clone();
        assert_eq!(observed.len(), 2, "both animation commands must arrive");
        assert_eq!(observed[0].emotion.as_deref(), Some("happy"));
        assert_eq!(observed[1].emotion.as_deref(), Some("surprised"));
    }

    /// Sending into the pump must be effectively instant relative to the
    /// frontend's emit cost. A backpressured pump would defeat the whole
    /// purpose of the decoupling — this test guards against a regression
    /// where someone replaces the unbounded channel with a bounded one
    /// without also moving emit work off the streaming task.
    #[tokio::test]
    async fn event_pump_send_is_non_blocking() {
        use tauri::test::{mock_builder, mock_context, noop_assets};

        let app = mock_builder()
            .build(mock_context(noop_assets()))
            .expect("mock_builder build");
        let handle = app.handle().clone();
        let (tx, pump) = spawn_event_pump(handle);

        let start = std::time::Instant::now();
        for i in 0..10_000 {
            tx.send(StreamEvent::Chunk(LlmChunk {
                text: format!("t{i}"),
                done: false,
            }))
            .expect("send");
        }
        let send_elapsed = start.elapsed();

        // 10k unbounded sends should complete well under 100 ms even on
        // slow CI; this guards against accidental sync work being added
        // to the producer side of the channel.
        assert!(
            send_elapsed < std::time::Duration::from_millis(100),
            "10k pump sends took {send_elapsed:?} — producer side is no longer non-blocking"
        );

        drop(tx);
        let _ = pump.await;
    }


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
            model: None,
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

    #[test]
    fn brain_mode_routes_local_lm_studio() {
        let mode = BrainMode::LocalLmStudio {
            model: "qwen/qwen3-4b".to_string(),
            base_url: "http://127.0.0.1:1234".to_string(),
            api_key: None,
            embedding_model: None,
        };
        match &mode {
            BrainMode::LocalLmStudio {
                model, base_url, ..
            } => {
                assert_eq!(model, "qwen/qwen3-4b");
                assert_eq!(base_url, "http://127.0.0.1:1234");
            }
            _ => panic!("expected LocalLmStudio"),
        }
    }

    // ── AnimationCommand tests ────────────────────────────────────────────────

    #[test]
    fn animation_command_serializes() {
        let cmd = AnimationCommand {
            emotion: Some("happy".to_string()),
            motion: None,
            intensity: None,
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
        let f = parser.feed(r#"<anim>{"emotion":"happy"}</anim>Hello!"#);
        assert_eq!(f.text, "Hello!");
        assert_eq!(f.anim_commands.len(), 1);
        assert_eq!(f.anim_commands[0].emotion, Some("happy".to_string()));
    }

    #[test]
    fn stream_tag_parser_no_anim() {
        let mut parser = StreamTagParser::new();
        let f = parser.feed("Just plain text");
        assert_eq!(f.text, "Just plain text");
        assert!(f.anim_commands.is_empty());
        assert!(f.pose_frames.is_empty());
    }

    #[test]
    fn stream_tag_parser_split_across_chunks() {
        let mut parser = StreamTagParser::new();
        let f1 = parser.feed(r#"<anim>{"emot"#);
        assert_eq!(f1.text, "");
        assert!(f1.anim_commands.is_empty());

        let f2 = parser.feed(r#"ion":"happy"}</anim>Hi!"#);
        assert_eq!(f2.text, "Hi!");
        assert_eq!(f2.anim_commands.len(), 1);
        assert_eq!(f2.anim_commands[0].emotion, Some("happy".to_string()));
    }

    #[test]
    fn stream_tag_parser_partial_open_tag() {
        let mut parser = StreamTagParser::new();
        let f1 = parser.feed("Hello <ani");
        assert_eq!(f1.text, "Hello ");
        assert!(f1.anim_commands.is_empty());

        let f2 = parser.flush();
        assert_eq!(f2.text, "<ani");
    }

    #[test]
    fn stream_tag_parser_flush_mid_anim() {
        let mut parser = StreamTagParser::new();
        parser.feed(r#"<anim>{"emotion":"ha"#);
        let f = parser.flush();
        assert!(f.text.contains("emotion"));
        assert!(f.anim_commands.is_empty());
    }

    #[test]
    fn stream_tag_parser_mixed_text_and_anim() {
        let mut parser = StreamTagParser::new();
        let f = parser.feed(r#"Before <anim>{"emotion":"sad","motion":"nod"}</anim>After"#);
        assert_eq!(f.text, "Before After");
        assert_eq!(f.anim_commands.len(), 1);
        assert_eq!(f.anim_commands[0].emotion, Some("sad".to_string()));
        assert_eq!(f.anim_commands[0].motion, Some("nod".to_string()));
    }

    #[test]
    fn stream_tag_parser_strips_newline_after_anim() {
        let mut parser = StreamTagParser::new();
        let f = parser.feed("<anim>{\"emotion\":\"happy\"}</anim>\nHello!");
        assert_eq!(f.text, "Hello!");
        assert_eq!(f.anim_commands.len(), 1);
    }

    #[test]
    fn stream_tag_parser_strips_newline_across_chunks() {
        let mut parser = StreamTagParser::new();
        let f1 = parser.feed("<anim>{\"emotion\":\"happy\"}</anim>");
        assert_eq!(f1.text, "");
        assert_eq!(f1.anim_commands.len(), 1);

        let f2 = parser.feed("\nHello!");
        assert_eq!(f2.text, "Hello!");
    }

    // ── <pose> tag tests (Chunk 14.16b2) ──────────────────────────────────────

    #[test]
    fn stream_tag_parser_pose_basic() {
        let mut parser = StreamTagParser::new();
        let f = parser.feed(r#"Hi! <pose>{"head":[0.1,0,0],"spine":[0,0,0.05]}</pose> done."#);
        assert_eq!(f.text, "Hi!  done.");
        assert_eq!(f.pose_frames.len(), 1);
        let frame = &f.pose_frames[0];
        assert_eq!(frame.bones["head"], [0.1, 0.0, 0.0]);
        assert_eq!(frame.bones["spine"], [0.0, 0.0, 0.05]);
    }

    #[test]
    fn stream_tag_parser_pose_split_across_chunks() {
        let mut parser = StreamTagParser::new();
        let f1 = parser.feed(r#"<pose>{"head":[0.2,"#);
        assert_eq!(f1.text, "");
        assert!(f1.pose_frames.is_empty());
        let f2 = parser.feed(r#"0,0]}</pose>OK"#);
        assert_eq!(f2.text, "OK");
        assert_eq!(f2.pose_frames.len(), 1);
        assert_eq!(f2.pose_frames[0].bones["head"], [0.2, 0.0, 0.0]);
    }

    #[test]
    fn stream_tag_parser_pose_clamps_out_of_range() {
        // 5.0 rad must be clamped down to ±0.5 by parse_pose_payload.
        let mut parser = StreamTagParser::new();
        let f = parser.feed(r#"<pose>{"head":[5.0,0,0]}</pose>"#);
        assert_eq!(f.pose_frames.len(), 1);
        assert_eq!(f.pose_frames[0].bones["head"][0], 0.5);
    }

    #[test]
    fn stream_tag_parser_pose_drops_invalid_payload() {
        // Garbage JSON inside <pose> must NOT crash the stream — the
        // block is silently dropped and surrounding text passes through.
        let mut parser = StreamTagParser::new();
        let f = parser.feed("Before <pose>not json</pose>After");
        assert_eq!(f.text, "Before After");
        assert!(f.pose_frames.is_empty());
    }

    #[test]
    fn stream_tag_parser_anim_and_pose_in_same_chunk() {
        let mut parser = StreamTagParser::new();
        let f =
            parser.feed(r#"<anim>{"emotion":"happy"}</anim><pose>{"head":[0.1,0,0]}</pose>Hi!"#);
        assert_eq!(f.text, "Hi!");
        assert_eq!(f.anim_commands.len(), 1);
        assert_eq!(f.pose_frames.len(), 1);
    }

    #[test]
    fn stream_tag_parser_pose_partial_open_held_back() {
        let mut parser = StreamTagParser::new();
        let f1 = parser.feed("Hello <pos");
        assert_eq!(f1.text, "Hello ");
        let f2 = parser.feed("e>{\"head\":[0,0,0]}</pose>");
        assert!(f2.text.is_empty());
        assert_eq!(f2.pose_frames.len(), 1);
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

    #[tokio::test]
    #[ignore = "requires local Ollama with TERRANSOUL_TEST_OLLAMA_MODEL or gemma4:e4b"]
    async fn local_ollama_hi_real_backend_first_chunk_under_1s() {
        use crate::brain::brain_config::BrainMode;
        use crate::brain::ollama_agent::OLLAMA_BASE_URL;
        use crate::AppState;
        use std::sync::{Arc, Mutex as StdMutex};
        use std::time::{Duration, Instant};
        use tauri::test::{mock_builder, mock_context, noop_assets};
        use tauri::{Listener, Manager};

        let model = std::env::var("TERRANSOUL_TEST_OLLAMA_MODEL")
            .unwrap_or_else(|_| "gemma4:e4b".to_string());
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("reqwest client");

        let warm_body = serde_json::json!({
            "model": model,
            "messages": [{ "role": "user", "content": "hi" }],
            "stream": false,
            "think": false,
            "options": { "num_predict": 1, "num_ctx": 2048, "num_batch": 512 },
            "keep_alive": "30m",
        });
        let warm_status = client
            .post(format!("{OLLAMA_BASE_URL}/api/chat"))
            .json(&warm_body)
            .send()
            .await
            .expect("Ollama warm-up request failed")
            .status();
        assert!(warm_status.is_success(), "Ollama warm-up status {warm_status}");

        let app = mock_builder()
            .build(mock_context(noop_assets()))
            .expect("mock_builder build");
        let handle = app.handle().clone();
        let state = AppState::for_test();
        {
            *state.brain_mode.lock().unwrap() = Some(BrainMode::LocalOllama {
                model: model.clone(),
            });
            *state.active_brain.lock().unwrap() = Some(model.clone());
        }
        handle.manage(state);

        let first_text_ms: Arc<StdMutex<Option<u128>>> = Arc::new(StdMutex::new(None));
        let done_signal = Arc::new(tokio::sync::Notify::new());
        let started = Instant::now();
        let first_cb = Arc::clone(&first_text_ms);
        let done_cb = Arc::clone(&done_signal);
        handle.listen("llm-chunk", move |event| {
            if let Ok(chunk) = serde_json::from_str::<LlmChunk>(event.payload()) {
                if !chunk.done && !chunk.text.is_empty() {
                    let mut first = first_cb.lock().unwrap();
                    if first.is_none() {
                        *first = Some(started.elapsed().as_millis());
                    }
                }
                if chunk.done {
                    done_cb.notify_one();
                }
            }
        });

        let state_ref: tauri::State<'_, AppState> = handle.state();
        run_chat_stream("Hi".to_string(), &handle, state_ref.inner())
            .await
            .expect("run_chat_stream");

        tokio::select! {
            _ = done_signal.notified() => {}
            _ = tokio::time::sleep(Duration::from_secs(10)) => panic!("timed out waiting for llm-chunk done"),
        }

        let first = *first_text_ms.lock().unwrap();
        let first = first.expect("expected at least one non-empty llm-chunk");
        eprintln!("[real-backend-hi] first_chunk={first}ms model={model}");
        assert!(
            first < 1_000,
            "first real backend llm-chunk took {first}ms; expected <1000ms"
        );
    }

    // ── Headless end-to-end stream verification (Linux only) ──────────────────
    //
    // These tests prove the entire backend streaming pipeline works on Linux
    // *without* a frontend (no GTK/WebKit/X11) by driving it through Tauri's
    // official MockRuntime. They spin up an in-process axum SSE mock LLM
    // server, listen for the three streams the frontend consumes
    // (`llm-chunk` text, `llm-animation`, `llm-chunk` done sentinel), and
    // assert on the contents of each.
    //
    // OS split per project policy: Linux verifies the **streams** (no GUI
    // available in CI), Windows verifies the **UI** through the existing
    // Playwright suite (`e2e/animation-flow.spec.ts`) which exercises the
    // same three-stream contract through a real browser. The cross-OS
    // smoke for the streaming Pinia store lives in `src/stores/streaming.test.ts`
    // (Vitest).
    #[cfg(target_os = "linux")]
    mod headless_linux {
        use super::*;
        use crate::brain::brain_config::BrainMode;
        use crate::AppState;
        use axum::{
            response::sse::{Event, KeepAlive, Sse},
            routing::post,
            Router,
        };
        use futures_util::stream;
        use std::convert::Infallible;
        use std::sync::Arc;
        use std::sync::Mutex as StdMutex;
        use std::time::Duration;
        use tauri::test::{mock_builder, mock_context, noop_assets};
        use tauri::{Listener, Manager};
        use tokio::net::TcpListener;

        /// SSE handler that streams a representative OpenAI-compatible response
        /// (simplified shape — not byte-for-byte production output) containing
        /// an `<anim>` block, two text deltas, and an end-of-stream `[DONE]`
        /// sentinel. This is the same envelope shape Groq/OpenAI/Pollinations
        /// use for `/v1/chat/completions` streaming.
        async fn mock_openai_sse_handler(
        ) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
            let chunks: Vec<&'static str> = vec![
                r#"{"choices":[{"delta":{"content":"<anim>"}}]}"#,
                r#"{"choices":[{"delta":{"content":"{\"emotion\":\"happy\",\"motion\":\"wave\"}"}}]}"#,
                r#"{"choices":[{"delta":{"content":"</anim>"}}]}"#,
                r#"{"choices":[{"delta":{"content":"Hello"}}]}"#,
                r#"{"choices":[{"delta":{"content":", world!"}}]}"#,
                "[DONE]",
            ];
            let s = stream::iter(
                chunks
                    .into_iter()
                    .map(|c| Ok::<_, Infallible>(Event::default().data(c))),
            );
            Sse::new(s).keep_alive(KeepAlive::default())
        }

        /// Spawn an axum mock LLM server on a random port and return its base URL
        /// (e.g. `http://127.0.0.1:34567`). The `/v1/chat/completions` endpoint
        /// streams the canned response. The listener is bound *before* this
        /// function returns, so connections are immediately accepted — no sleep
        /// needed.
        async fn spawn_mock_openai_server() -> String {
            let app = Router::new().route("/v1/chat/completions", post(mock_openai_sse_handler));
            let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
            let port = listener.local_addr().unwrap().port();
            tokio::spawn(async move {
                let _ = axum::serve(listener, app).await;
            });
            format!("http://127.0.0.1:{port}")
        }

        /// Wait for `done_signal` to fire or fail with a clear timeout error
        /// after `Duration`. Replaces non-deterministic `sleep`-based waits in
        /// the streaming tests.
        async fn await_done(
            notify: &tokio::sync::Notify,
            timeout: Duration,
        ) -> Result<(), &'static str> {
            tokio::select! {
                _ = notify.notified() => Ok(()),
                _ = tokio::time::sleep(timeout) => Err("timed out waiting for done:true llm-chunk"),
            }
        }

        /// End-to-end verification that the OpenAI-compatible streaming path
        /// emits the three event streams the frontend expects:
        ///   1. `llm-chunk` deltas with clean text (no `<anim>` blocks)
        ///   2. `llm-animation` typed AnimationCommand events
        ///   3. final `llm-chunk` with `done: true`
        /// and that the assistant message is persisted into `state.conversation`
        /// with `<anim>` blocks stripped.
        #[tokio::test]
        async fn headless_openai_stream_emits_three_streams() {
            // 1. Spawn the mock LLM server.
            let base_url = spawn_mock_openai_server().await;

            // 2. Build a Tauri app on the MockRuntime — no GTK/WebKit needed.
            let app = mock_builder()
                .build(mock_context(noop_assets()))
                .expect("mock_builder build");
            let handle = app.handle().clone();

            // 3. Manage an `AppState` configured for PaidApi against the mock URL.
            let state = AppState::for_test();
            {
                let mut mode = state.brain_mode.lock().unwrap();
                *mode = Some(BrainMode::PaidApi {
                    provider: "mock".to_string(),
                    api_key: "sk-test".to_string(),
                    model: "mock-model".to_string(),
                    base_url: base_url.clone(),
                });
            }
            handle.manage(state);

            // 4. Subscribe to the three event streams. A Notify fires the moment
            //    we observe the `done:true` sentinel — replaces sleep-based waits.
            let chunks: Arc<StdMutex<Vec<LlmChunk>>> = Arc::new(StdMutex::new(Vec::new()));
            let anims: Arc<StdMutex<Vec<AnimationCommand>>> = Arc::new(StdMutex::new(Vec::new()));
            let done_signal = Arc::new(tokio::sync::Notify::new());
            let chunks_cb = Arc::clone(&chunks);
            let done_cb = Arc::clone(&done_signal);
            handle.listen("llm-chunk", move |event| {
                if let Ok(c) = serde_json::from_str::<LlmChunk>(event.payload()) {
                    let is_done = c.done;
                    chunks_cb.lock().unwrap().push(c);
                    if is_done {
                        done_cb.notify_one();
                    }
                }
            });
            let anims_cb = Arc::clone(&anims);
            handle.listen("llm-animation", move |event| {
                if let Ok(a) = serde_json::from_str::<AnimationCommand>(event.payload()) {
                    anims_cb.lock().unwrap().push(a);
                }
            });

            // 5. Drive the full pipeline through the headless entry point.
            let state_ref: tauri::State<'_, AppState> = handle.state();
            run_chat_stream("hello".to_string(), &handle, state_ref.inner())
                .await
                .expect("run_chat_stream");

            // Block deterministically until the `done:true` chunk has been
            // dispatched to the listener thread.
            await_done(&done_signal, Duration::from_secs(5))
                .await
                .expect("done sentinel");

            // 6. Assertions.
            let chunks_snap = chunks.lock().unwrap().clone();
            let anims_snap = anims.lock().unwrap().clone();

            // ── Stream 1: text deltas ────────────────────────────────────────────
            let text_chunks: Vec<&LlmChunk> = chunks_snap.iter().filter(|c| !c.done).collect();
            assert!(
                !text_chunks.is_empty(),
                "expected at least one text llm-chunk, got none"
            );
            let concatenated: String = text_chunks.iter().map(|c| c.text.as_str()).collect();
            assert!(
                concatenated.contains("Hello, world!"),
                "expected concatenated text to contain 'Hello, world!', got {concatenated:?}"
            );
            assert!(
                !concatenated.contains("<anim>") && !concatenated.contains("</anim>"),
                "anim blocks must be stripped from text stream, got {concatenated:?}"
            );

            // ── Stream 2: animation events ───────────────────────────────────────
            assert!(
                !anims_snap.is_empty(),
                "expected at least one llm-animation event"
            );
            let happy = anims_snap
                .iter()
                .find(|a| a.emotion.as_deref() == Some("happy"))
                .expect("expected an llm-animation with emotion=happy");
            assert_eq!(happy.motion.as_deref(), Some("wave"));

            // ── Stream 3: completion sentinel ────────────────────────────────────
            let last = chunks_snap.last().expect("at least one chunk");
            assert!(
                last.done && last.text.is_empty(),
                "last chunk should be done=true with empty text, got {last:?}"
            );

            // ── Persistence side-effects ─────────────────────────────────────────
            let state_ref: tauri::State<'_, AppState> = handle.state();
            let conv = state_ref.conversation.lock().unwrap();
            assert_eq!(conv.len(), 2, "expected user + assistant message");
            assert_eq!(conv[0].role, "user");
            assert_eq!(conv[0].content, "hello");
            assert_eq!(conv[1].role, "assistant");
            assert!(
                conv[1].content.contains("Hello, world!"),
                "stored assistant content should contain reply text, got {:?}",
                conv[1].content
            );
            assert!(
                !conv[1].content.contains("<anim>"),
                "stored assistant content must have anim blocks stripped"
            );
        }

        /// Verifies that an empty message is rejected before any IO happens —
        /// guarding the one early-return branch of `run_chat_stream`.
        #[tokio::test]
        async fn headless_empty_message_is_rejected() {
            let app = mock_builder()
                .build(mock_context(noop_assets()))
                .expect("mock_builder build");
            let handle = app.handle().clone();
            let state = AppState::for_test();
            handle.manage(state);
            let state_ref: tauri::State<'_, AppState> = handle.state();
            let err = run_chat_stream("   ".to_string(), &handle, state_ref.inner())
                .await
                .expect_err("empty message should error");
            assert!(err.contains("empty"), "got: {err}");
            let state_ref: tauri::State<'_, AppState> = handle.state();
            assert!(
                state_ref.conversation.lock().unwrap().is_empty(),
                "no user message should be stored on early reject"
            );
        }

        /// When no `BrainMode` is configured and no legacy brain is active, the
        /// pipeline must fall through to the stub agent and still emit the same
        /// three-stream contract (so the frontend renders identically).
        #[tokio::test]
        async fn headless_stub_fallback_emits_three_streams() {
            let app = mock_builder()
                .build(mock_context(noop_assets()))
                .expect("mock_builder build");
            let handle = app.handle().clone();
            let state = AppState::for_test();
            handle.manage(state);

            let chunks: Arc<StdMutex<Vec<LlmChunk>>> = Arc::new(StdMutex::new(Vec::new()));
            let done_signal = Arc::new(tokio::sync::Notify::new());
            let chunks_cb = Arc::clone(&chunks);
            let done_cb = Arc::clone(&done_signal);
            handle.listen("llm-chunk", move |event| {
                if let Ok(c) = serde_json::from_str::<LlmChunk>(event.payload()) {
                    let is_done = c.done;
                    chunks_cb.lock().unwrap().push(c);
                    if is_done {
                        done_cb.notify_one();
                    }
                }
            });

            let state_ref: tauri::State<'_, AppState> = handle.state();
            run_chat_stream("ping".to_string(), &handle, state_ref.inner())
                .await
                .expect("run_chat_stream");
            await_done(&done_signal, Duration::from_secs(5))
                .await
                .expect("done sentinel");

            let chunks_snap = chunks.lock().unwrap().clone();
            assert!(
                chunks_snap
                    .iter()
                    .any(|c| !c.done && c.text.contains("ping")),
                "stub should echo the user message back in a non-done chunk"
            );
            assert!(
                chunks_snap
                    .last()
                    .map(|c| c.done && c.text.is_empty())
                    .unwrap_or(false),
                "stub must terminate with done=true sentinel"
            );
        }
    } // end mod headless_linux
}
