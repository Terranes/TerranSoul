use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::OnceLock;
#[cfg(test)]
use std::sync::RwLock as StdRwLock;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::agent::stub_agent::Sentiment;
use crate::agent::AgentProvider;
use crate::memory::late_chunking::CharSpan;

pub const OLLAMA_BASE_URL: &str = "http://127.0.0.1:11434";

// ── Chat-model warm-keeper (see docs/brain-advanced-design.md § 16 Phase 4) ──
//
// On consumer GPUs the chat model (e.g. `gemma4:e4b` ~10.6 GB) and the embed
// model (`nomic-embed-text` ~0.6 GB) cannot reliably co-reside. Any embed
// call evicts the chat model, and the next user reply pays a 5-15 s reload
// cost. We register the active chat model in a process-wide cell so that
// **every** embed call (app, MCP, gRPC, CRAG, Self-RAG, …) can fire a
// fire-and-forget re-warm immediately afterwards, ensuring the next chat
// turn finds the chat model warm.
fn chat_model_for_warmup() -> &'static RwLock<Option<String>> {
    static CELL: OnceLock<RwLock<Option<String>>> = OnceLock::new();
    CELL.get_or_init(|| RwLock::new(None))
}

/// Register the active chat model so embed calls can re-warm it after
/// running. Called from app startup, MCP server startup, and on every
/// brain-mode change.
pub fn set_chat_model_for_warmup(model: &str) {
    if let Ok(mut guard) = chat_model_for_warmup().write() {
        *guard = Some(model.to_string());
    }
}

/// Snapshot of the registered chat model, or `None` if unset.
pub fn registered_chat_model_for_warmup() -> Option<String> {
    chat_model_for_warmup().read().ok().and_then(|g| g.clone())
}

/// Spawn a fire-and-forget request that loads the registered chat model
/// into VRAM with a long `keep_alive`. Returns immediately. Used after
/// every embed call to undo the model swap.
fn spawn_chat_model_rewarm(reason: &'static str) {
    let Some(model) = registered_chat_model_for_warmup() else {
        return;
    };
    tokio::spawn(async move {
        let Ok(client) = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
        else {
            return;
        };
        // 1-token real chat forces Ollama to actually load the weights
        // into VRAM. An empty `messages: []` body sometimes no-ops.
        let body = serde_json::json!({
            "model": model,
            "messages": [{ "role": "user", "content": " " }],
            "options": { "num_predict": 1, "num_ctx": 2048, "num_batch": 512 },
            "stream": false,
            "keep_alive": "30m",
        });
        let started = Instant::now();
        match client
            .post(ollama_api_url("/api/chat"))
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                eprintln!(
                    "[chat-rewarm:{reason}] {} status={} {}ms",
                    model,
                    resp.status(),
                    started.elapsed().as_millis()
                );
            }
            Err(e) => {
                eprintln!("[chat-rewarm:{reason}] {model} skipped: {e}");
            }
        }
    });
}

fn ollama_api_url(path: &str) -> String {
    format!("{}{}", ollama_base_url(), path)
}

fn ollama_base_url() -> String {
    #[cfg(test)]
    {
        if let Some(base_url) = test_ollama_base_url()
            .read()
            .expect("test Ollama base URL lock poisoned")
            .clone()
        {
            return base_url;
        }
    }

    OLLAMA_BASE_URL.to_string()
}

#[cfg(test)]
fn test_ollama_base_url() -> &'static StdRwLock<Option<String>> {
    static TEST_BASE_URL: OnceLock<StdRwLock<Option<String>>> = OnceLock::new();
    TEST_BASE_URL.get_or_init(|| StdRwLock::new(None))
}

#[cfg(test)]
struct TestOllamaBaseUrlGuard {
    previous: Option<String>,
}

#[cfg(test)]
impl Drop for TestOllamaBaseUrlGuard {
    fn drop(&mut self) {
        *test_ollama_base_url()
            .write()
            .expect("test Ollama base URL lock poisoned") = self.previous.take();
    }
}

#[cfg(test)]
fn use_test_ollama_base_url(base_url: &str) -> TestOllamaBaseUrlGuard {
    let mut guard = test_ollama_base_url()
        .write()
        .expect("test Ollama base URL lock poisoned");
    let previous = guard.replace(base_url.to_string());
    TestOllamaBaseUrlGuard { previous }
}

/// System prompt injected into every Ollama conversation.
const SYSTEM_PROMPT: &str = r#"You are TerranSoul, a friendly AI companion with a 3D character avatar. You live inside the TerranSoul desktop app and serve as the user's intelligent assistant.

Your capabilities:
- Helpful conversation and answering questions on any topic
- Recommending AI tools and software based on the user's needs
- Guiding users through installing packages via the TerranSoul Package Manager

Available extensions you can recommend:
- **OpenClaw Bridge** (built-in plugin: "openclaw-bridge"): An OpenClaw-style tool bridge for `/openclaw read`, `/openclaw fetch`, and `/openclaw chat` workflows. Great for users who want TerranSoul to coordinate an external tool runtime while preserving plugin capability consent.
- **Claude Cowork** (model tag: "claude-cowork"): A collaborative AI workspace powered by Anthropic's Claude. Perfect for document analysis, long-context reasoning, and team workflows.
- **stub-agent**: The built-in lightweight agent. Always available offline.

When recommending an extension or package, mention its name and briefly explain why it suits the user's request. Keep responses concise and warm."#;

// ── Ollama API types ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    /// Disable Gemma 4 / Qwen 3 built-in thinking so generated tokens go to
    /// `content` instead of being consumed by internal reasoning.
    #[serde(skip_serializing_if = "Option::is_none")]
    think: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptions>,
    /// Keep the model loaded in VRAM for 30 minutes between requests.
    /// Without this, Ollama uses the default 5-minute keep-alive and any
    /// other model load (e.g. embedding) will evict the chat model,
    /// adding 10-20s to the next reply on consumer GPUs.
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_alive: Option<String>,
}

#[derive(Serialize)]
struct ChatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_ctx: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// Prompt-processing batch size. Larger = faster prompt eval at the
    /// cost of slightly more VRAM. 512 is the Ollama default; we set it
    /// explicitly so it stays consistent across all request sites.
    #[serde(skip_serializing_if = "Option::is_none")]
    num_batch: Option<u32>,
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

/// Per-token embedding response from an Ollama-compatible long-context
/// embedder. Standard Ollama embedders usually return one pooled vector;
/// late chunking only activates when the response contains one vector per
/// token plus character spans or token text that can be aligned back to the
/// original document.
#[derive(Debug, Clone, PartialEq)]
pub struct OllamaTokenEmbeddings {
    pub model: String,
    pub token_embeddings: Vec<Vec<f32>>,
    pub token_char_spans: Vec<CharSpan>,
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
    /// Always returns Neutral — real emotion comes from `<anim>` stream tags.
    fn infer_sentiment(_text: &str) -> Sentiment {
        Sentiment::Neutral
    }

    /// Sentiment fallback for the non-streaming path.
    ///
    /// The LLM decides emotion via `<anim>` tags in the streaming pipeline;
    /// this static fallback always returns `Neutral` so keyword heuristics
    /// never override the LLM's judgment.
    pub fn infer_sentiment_static(_text: &str) -> Sentiment {
        Sentiment::Neutral
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
            format!(
                "{SYSTEM_PROMPT}{}",
                crate::memory::format_retrieved_context_pack(&format!("- {mem_block}"))
            )
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
    pub(crate) async fn call(&self, messages: Vec<ChatMessage>) -> (String, Sentiment) {
        let body = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
            think: Some(false),
            options: Some(ChatOptions {
                num_predict: Some(150),
                num_ctx: Some(2048),
                temperature: Some(0.7),
                num_batch: Some(512),
            }),
            keep_alive: Some("30m".to_string()),
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

    /// Ask the brain to propose typed, directional edges for a batch of memories.
    ///
    /// Returns the raw LLM reply (one JSON-line edge per row, or `NONE`). The
    /// caller is responsible for parsing via `crate::memory::parse_llm_edges`,
    /// which validates ids, de-duplicates self-loops, and clamps confidence.
    pub async fn propose_edges(&self, memories_block: &str) -> String {
        let prompt = format!(
            "You are building a knowledge graph. Read the following memories \
            (each tagged with its id) and propose typed, directional relationships \
            between them. Use one of these relation types where possible: \
            related_to, contains, cites, governs, part_of, depends_on, supersedes, \
            contradicts, derived_from, mentions, located_in, studies, prefers, \
            knows, owns, mother_of, child_of. \
            Only propose edges you are reasonably confident about. \
            Reply with ONE JSON object per line, no surrounding prose, in this exact form: \
            {{\"src_id\": <id>, \"dst_id\": <id>, \"rel_type\": \"<type>\", \"confidence\": <0.0-1.0>}} \
            If there are no good edges in this batch, reply with exactly: NONE\n\n\
            MEMORIES:\n{memories_block}"
        );
        let msgs = vec![
            ChatMessage {
                role: "system".to_string(),
                content:
                    "You extract typed relationships between memories. Reply with JSON lines only."
                        .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];
        let (reply, _) = self.call(msgs).await;
        reply
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
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
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
                content:
                    "You are a concise summarizer. Summarize conversations into 1-3 sentences."
                        .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let (reply, _) = self.call(msgs).await;
        let clean = reply.trim().to_string();
        if clean.is_empty() {
            None
        } else {
            Some(clean)
        }
    }

    /// Generate a **hypothetical answer** for a HyDE retrieval query.
    ///
    /// HyDE (Gao et al., 2022) embeds an LLM-written hypothetical answer
    /// instead of the raw query, dramatically improving recall on cold
    /// or abstract queries. Returns `None` when the brain is unreachable
    /// or the reply is too short to carry retrieval signal — in both
    /// cases the caller should fall back to embedding the raw query.
    ///
    /// Prompt construction + reply cleaning live in
    /// [`crate::memory::hyde`] so they can be unit-tested without the
    /// network.
    pub async fn hyde_complete(&self, query: &str) -> Option<String> {
        if query.trim().is_empty() {
            return None;
        }
        let (system, user) = crate::memory::hyde::build_hyde_prompt(query);
        let msgs = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user,
            },
        ];
        let (reply, _) = self.call(msgs).await;
        crate::memory::hyde::clean_hyde_reply(&reply)
    }

    /// LLM-as-judge **cross-encoder rerank score** for a single
    /// `(query, document)` pair on a 0–10 integer scale.
    ///
    /// This is the per-candidate primitive consumed by the
    /// `rerank_search_memories` Tauri command. Returns `None` if the
    /// brain is unreachable or replies with an unparseable score —
    /// the caller treats `None` as "skip", and
    /// [`crate::memory::reranker::rerank_candidates`] keeps unscored
    /// candidates rather than dropping them, so a flaky brain never
    /// silently loses recall.
    ///
    /// Prompt construction + reply parsing live in
    /// [`crate::memory::reranker`] so they can be unit-tested without
    /// the network. See `docs/brain-advanced-design.md` § 16 Phase 6 /
    /// § 19.2 row 10.
    pub async fn rerank_score(&self, query: &str, document: &str) -> Option<u8> {
        if query.trim().is_empty() || document.trim().is_empty() {
            return None;
        }
        let (system, user) = crate::memory::reranker::build_rerank_prompt(query, document);
        let msgs = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user,
            },
        ];
        let (reply, _) = self.call(msgs).await;
        crate::memory::reranker::parse_rerank_score(&reply)
    }

    /// Master-Echo persona suggestion (Chunk 14.2 of `docs/persona-design.md`).
    ///
    /// Reads the rendered `(system, user)` pair from
    /// [`crate::persona::extract::build_persona_prompt`], asks the active
    /// brain for a one-shot reply, then parses it via
    /// [`crate::persona::extract::parse_persona_reply`]. Returns `None`
    /// when the brain is unreachable, the reply is empty, or the parsed
    /// candidate is missing required fields — in every failure mode the
    /// caller surfaces a "couldn't suggest a persona right now" message
    /// rather than silently writing garbage.
    ///
    /// Prompt construction + reply parsing live in
    /// [`crate::persona::extract`] so they can be unit-tested without
    /// the network — same shape as `hyde` and `reranker`.
    pub async fn propose_persona(
        &self,
        snippets: &[crate::persona::extract::PromptSnippet],
    ) -> Option<crate::persona::extract::PersonaCandidate> {
        self.propose_persona_with_hints(snippets, None).await
    }

    /// Hint-aware variant of [`Self::propose_persona`] (Chunk 14.6).
    /// `prosody_hints` is an already-rendered single-line block from
    /// [`crate::persona::prosody::render_prosody_block`]. Pass `None`
    /// when ASR is not configured or no signal was strong enough.
    pub async fn propose_persona_with_hints(
        &self,
        snippets: &[crate::persona::extract::PromptSnippet],
        prosody_hints: Option<&str>,
    ) -> Option<crate::persona::extract::PersonaCandidate> {
        let (system, user) =
            crate::persona::extract::build_persona_prompt_with_hints(snippets, prosody_hints);
        let msgs = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user,
            },
        ];
        let (reply, _) = self.call(msgs).await;
        crate::persona::extract::parse_persona_reply(&reply)
    }

    // ── Persona drift detection (Chunk 14.8) ─────────────────────────────

    /// Check whether the user's `personal:*` memories still align with the
    /// active persona traits. Returns `None` when the brain reply can't be
    /// parsed (caller should treat this as "no drift detected").
    pub async fn check_persona_drift(
        &self,
        persona_json: &str,
        personal_memories: &[(String, String)],
    ) -> Option<crate::persona::drift::DriftReport> {
        let (system, user) =
            crate::persona::drift::build_drift_prompt(persona_json, personal_memories);
        let msgs = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user,
            },
        ];
        let (reply, _) = self.call(msgs).await;
        crate::persona::drift::parse_drift_reply(&reply)
    }

    // ── Contradiction detection (Chunk 17.2) ───────────────────────────────

    /// Ask the LLM whether two memory statements contradict each other.
    /// Returns `None` when the brain reply can't be parsed (caller should
    /// treat this as "no contradiction detected").
    pub async fn check_contradiction(
        &self,
        content_a: &str,
        content_b: &str,
    ) -> Option<crate::memory::conflicts::ContradictionResult> {
        let (system, user) =
            crate::memory::conflicts::build_contradiction_prompt(content_a, content_b);
        let msgs = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user,
            },
        ];
        let (reply, _) = self.call(msgs).await;
        crate::memory::conflicts::parse_contradiction_reply(&reply)
    }

    // ── Embedding ──────────────────────────────────────────────────────────

    /// Generate a vector embedding for `text` via Ollama `/api/embed`.
    ///
    /// **Resilience contract** — this function is called on every chat
    /// message (RAG injection), every memory add, and every ingest chunk.
    /// It MUST never panic and MUST short-circuit cheaply when the local
    /// Ollama daemon does not support embeddings for the active model
    /// (the common cause of repeated `501 Not Implemented` /
    /// `400 model does not support embeddings` errors). Two caches make
    /// that possible:
    ///
    /// 1. `embed_model_cache` — chosen embedding model + 60 s TTL so we
    ///    don't hammer `/api/tags` on every call.
    /// 2. `unsupported_models` — process-lifetime allow-list of models
    ///    that previously returned a non-success status; subsequent
    ///    embed calls for those models return `None` immediately.
    ///
    /// Returns `None` (never an `Err`) so callers can fall back to the
    /// keyword / LLM-ranking path without breaking the chat flow.
    pub async fn embed_text(text: &str, model_hint: &str) -> Option<Vec<f32>> {
        // Refuse work on empty input rather than wasting an HTTP round-trip.
        if text.trim().is_empty() {
            return None;
        }

        // 1. Resolve which embedding model to use (cached for 60 s).
        let embed_model = resolve_embed_model(model_hint).await;

        // 2. Fast path: this model is already known to be unsupported.
        if is_known_unsupported(&embed_model).await {
            return None;
        }

        // 3. Tight timeout: nomic-embed-text responds in ~55 ms when warm.
        //    If the embed model is cold (not loaded), Ollama would swap out
        //    the chat model to load it, then the chat call must reload the
        //    chat model — two model swaps costing 10-20 s total.  A 500 ms
        //    cap lets warm embeds through while gracefully falling back to
        //    keyword-only hybrid search when a model swap would be needed.
        let client = match Client::builder()
            .timeout(Duration::from_millis(500))
            .build()
        {
            Ok(c) => c,
            Err(_) => return None,
        };

        let body = serde_json::json!({
            "model": embed_model,
            "input": text,
            // Unload the embed model immediately after this single embed so
            // the chat model can stay resident in VRAM. Without this, the
            // embed model stays loaded for 5 min by default and the next
            // chat reply pays a 10-20 s reload cost on consumer GPUs.
            "keep_alive": 0,
        });

        let resp = match client
            .post(ollama_api_url("/api/embed"))
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return None, // network error — let caller fall back
        };

        if !resp.status().is_success() {
            // A non-success status from /api/embed for this exact model
            // means the model genuinely cannot embed (most chat models
            // return 501 / 400). Cache that fact so we never retry it
            // for the lifetime of the process — the cache is cleared
            // when the user switches to a working embedding model via
            // `clear_embed_caches`.
            let status = resp.status().as_u16();
            mark_unsupported(&embed_model, status).await;
            return None;
        }

        let json: serde_json::Value = resp.json().await.ok()?;
        // Ollama returns { "embeddings": [[...]] }
        let arr = json.get("embeddings")?.as_array()?.first()?.as_array()?;
        let vec: Vec<f32> = arr
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();
        if vec.is_empty() {
            None
        } else {
            // Re-warm the chat model so the next user turn doesn't pay
            // the 5-15 s VRAM swap caused by this embed call.
            spawn_chat_model_rewarm("embed_text");
            Some(vec)
        }
    }

    /// Generate embeddings for a batch of texts via Ollama `/api/embed`.
    ///
    /// Ollama 0.4+ supports `"input": ["text1", "text2", …]` and returns
    /// `"embeddings": [[…], […], …]`. This batches up to `batch_size` texts
    /// in a single HTTP call, dramatically reducing round-trip overhead for
    /// bulk ingest.
    ///
    /// Returns a `Vec<Option<Vec<f32>>>` where each element corresponds to
    /// the input at the same index. `None` means the embedding failed for
    /// that specific text (or the entire call failed, in which case all
    /// entries are `None`).
    pub async fn embed_text_batch(
        texts: &[&str],
        model_hint: &str,
        batch_size: usize,
    ) -> Vec<Option<Vec<f32>>> {
        if texts.is_empty() {
            return vec![];
        }

        let embed_model = resolve_embed_model(model_hint).await;

        if is_known_unsupported(&embed_model).await {
            return vec![None; texts.len()];
        }

        let client = match Client::builder().timeout(Duration::from_secs(60)).build() {
            Ok(c) => c,
            Err(_) => return vec![None; texts.len()],
        };

        let mut results = Vec::with_capacity(texts.len());

        for chunk in texts.chunks(batch_size) {
            // Filter out empty texts but track indices for reassembly.
            let (indices, batch_texts): (Vec<usize>, Vec<&str>) = chunk
                .iter()
                .enumerate()
                .filter(|(_, t)| !t.trim().is_empty())
                .map(|(i, t)| (i, *t))
                .unzip();

            if batch_texts.is_empty() {
                results.extend(std::iter::repeat_n(None, chunk.len()));
                continue;
            }

            let body = serde_json::json!({
                "model": embed_model,
                "input": batch_texts,
                // Unload the embed model immediately after the batch so the
                // chat model isn’t evicted from VRAM by lingering keep-alive.
                "keep_alive": 0,
            });

            let resp = match client
                .post(ollama_api_url("/api/embed"))
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(_) => {
                    results.extend(std::iter::repeat_n(None, chunk.len()));
                    continue;
                }
            };

            if !resp.status().is_success() {
                let status = resp.status().as_u16();
                mark_unsupported(&embed_model, status).await;
                results.extend(std::iter::repeat_n(None, chunk.len()));
                continue;
            }

            let json: serde_json::Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => {
                    results.extend(std::iter::repeat_n(None, chunk.len()));
                    continue;
                }
            };

            let embeddings = json
                .get("embeddings")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            // Re-assemble: put embeddings back at their original indices.
            let mut chunk_results = vec![None; chunk.len()];
            for (slot, emb_val) in indices.iter().zip(embeddings.iter()) {
                if let Some(arr) = emb_val.as_array() {
                    let vec: Vec<f32> = arr
                        .iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect();
                    if !vec.is_empty() {
                        chunk_results[*slot] = Some(vec);
                    }
                }
            }
            results.extend(chunk_results);
        }

        // Re-warm the chat model so the next user turn doesn't pay the
        // 5-15 s VRAM swap caused by this batch embed call.
        if results.iter().any(|r| r.is_some()) {
            spawn_chat_model_rewarm("embed_text_batch");
        }

        results
    }

    /// Generate per-token embeddings for a whole document via Ollama.
    ///
    /// This is intentionally best-effort. If the selected model or Ollama
    /// build returns the standard pooled `/api/embed` shape, the parser returns
    /// `None` and callers should fall back to per-chunk embeddings.
    pub async fn embed_tokens(text: &str, model_hint: &str) -> Option<OllamaTokenEmbeddings> {
        if text.trim().is_empty() {
            return None;
        }

        let embed_model = resolve_late_chunk_model(model_hint).await;
        if is_known_unsupported(&embed_model).await {
            return None;
        }

        let client = match Client::builder().timeout(Duration::from_secs(60)).build() {
            Ok(c) => c,
            Err(_) => return None,
        };

        let body = serde_json::json!({
            "model": embed_model,
            "input": text,
            "truncate": false,
            "options": {
                "truncate": false
            },
            // Unload immediately after late-chunk embedding so the chat
            // model keeps its VRAM residency.
            "keep_alive": 0,
        });

        let resp = match client
            .post(ollama_api_url("/api/embed"))
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return None,
        };

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            mark_unsupported(&embed_model, status).await;
            return None;
        }

        let json: serde_json::Value = resp.json().await.ok()?;
        let (token_embeddings, token_char_spans) = parse_token_embedding_response(&json, text)?;
        // Re-warm the chat model so the next user turn doesn't pay the
        // 5-15 s VRAM swap caused by this late-chunk embed call.
        spawn_chat_model_rewarm("embed_tokens");
        Some(OllamaTokenEmbeddings {
            model: embed_model,
            token_embeddings,
            token_char_spans,
        })
    }

    /// Check if a model name is available locally in Ollama.
    /// Result is cached for 60 s by [`resolve_embed_model`].
    /// Tight 1 s timeout: this runs on the hot chat path on cache miss
    /// (every 60 s in steady state), so we never want it to block longer
    /// than the chat itself takes to start.
    async fn model_exists(name: &str) -> bool {
        let client = match Client::builder().timeout(Duration::from_secs(1)).build() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let resp = client.get(ollama_api_url("/api/tags")).send().await;
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
                content:
                    "You select the most relevant memories from a list. Reply with numbers only."
                        .to_string(),
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

// ── Embedding capability cache (resilience for /api/embed) ────────────────────
//
// Two caches let `OllamaAgent::embed_text` short-circuit cheaply when the
// local Ollama daemon would otherwise return repeated 501/400 errors:
//
// * `EMBED_MODEL_CACHE` remembers which model name we settled on, with a
//   60-second TTL so we don't re-probe `/api/tags` on every chat message.
// * `UNSUPPORTED_MODELS` lists models that previously returned a non-success
//   status from `/api/embed`. Most chat models (Llama, Phi, Gemma, …) do not
//   implement embeddings and return `501 Not Implemented`; once we've seen
//   that once we MUST stop retrying for the lifetime of the process.
//
// Both caches are cleared by `clear_embed_caches()` whenever the user
// switches brain mode or installs a new embedding model.

#[derive(Clone)]
struct EmbedModelChoice {
    model: String,
    chosen_at: Instant,
}

const EMBED_MODEL_TTL: Duration = Duration::from_secs(60);
const PREFERRED_EMBED_MODEL: &str = "nomic-embed-text";

/// Ordered fallback chain of dedicated embedding models, tried in this order
/// when [`PREFERRED_EMBED_MODEL`] is unavailable. Per
/// `docs/brain-advanced-design.md` §4 resilience notes (Chunk 16.9b).
///
/// Each entry must be the **bare model name** as published in the Ollama
/// library (no `:tag` suffix) — `model_exists` matches by name prefix.
/// Order is from most-recommended (768d, fast, well-tested) → larger /
/// alternative (1024d, slower) → tiny last-resort.
const EMBED_MODEL_FALLBACKS: &[&str] = &[
    "mxbai-embed-large",      // 1024d, strong general-purpose
    "snowflake-arctic-embed", // 1024d / 768d depending on tag
    "bge-m3",                 // 1024d, multilingual
    "all-minilm",             // 384d, tiny last-resort
];

/// Candidate local embedders that are plausible long-context / token-vector
/// providers. Most current Ollama builds still return pooled vectors only;
/// the late-chunking parser verifies the response shape before using it.
const LATE_CHUNK_MODEL_FALLBACKS: &[&str] = &[
    "jina-embeddings-v3",
    "bge-m3",
    "mxbai-embed-large",
    PREFERRED_EMBED_MODEL,
];

fn embed_model_cache() -> &'static Mutex<Option<EmbedModelChoice>> {
    static CACHE: OnceLock<Mutex<Option<EmbedModelChoice>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn unsupported_models() -> &'static Mutex<HashSet<String>> {
    static SET: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SET.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Resolve which model to pass to `/api/embed`, caching the choice for
/// 60 seconds.
///
/// Resolution chain (Chunk 16.9b — embedding-model fallback):
///
/// 1. **`nomic-embed-text`** — preferred (768d, fast, well-tested).
/// 2. **Fallback chain** in [`EMBED_MODEL_FALLBACKS`] order — `mxbai`,
///    `snowflake-arctic`, `bge-m3`, `all-minilm`. The first one present
///    in `/api/tags` wins. Models already in the unsupported set are
///    skipped.
/// 3. **`model_hint`** — the active chat model (probably non-embedding;
///    `embed_text` will mark it unsupported on the first call).
///
/// When every dedicated embedder is unavailable AND the chat model can't
/// embed either, the caller (memory store) falls back to keyword-only
/// search — see `docs/brain-advanced-design.md` §4.
async fn resolve_embed_model(model_hint: &str) -> String {
    let cache = embed_model_cache();
    {
        let guard = cache.lock().await;
        if let Some(choice) = guard.as_ref() {
            if choice.chosen_at.elapsed() < EMBED_MODEL_TTL {
                return choice.model.clone();
            }
        }
    }

    // Cache miss / expired → walk the fallback chain.
    let chosen = pick_embed_model(model_hint).await;

    let mut guard = cache.lock().await;
    *guard = Some(EmbedModelChoice {
        model: chosen.clone(),
        chosen_at: Instant::now(),
    });
    chosen
}

/// Walk the embed-model resolution chain. Pure helper so tests can drive
/// it without mocking the cache layer.
async fn pick_embed_model(model_hint: &str) -> String {
    // Preferred first.
    if !is_known_unsupported(PREFERRED_EMBED_MODEL).await
        && OllamaAgent::model_exists(PREFERRED_EMBED_MODEL).await
    {
        return PREFERRED_EMBED_MODEL.to_string();
    }
    // Then dedicated fallbacks.
    for candidate in EMBED_MODEL_FALLBACKS {
        if is_known_unsupported(candidate).await {
            continue;
        }
        if OllamaAgent::model_exists(candidate).await {
            return (*candidate).to_string();
        }
    }
    // Last resort — the active chat model. Likely won't support embed,
    // but we let `embed_text` discover that and mark it unsupported.
    model_hint.to_string()
}

async fn resolve_late_chunk_model(model_hint: &str) -> String {
    for candidate in LATE_CHUNK_MODEL_FALLBACKS {
        if is_known_unsupported(candidate).await {
            continue;
        }
        if OllamaAgent::model_exists(candidate).await {
            return (*candidate).to_string();
        }
    }
    resolve_embed_model(model_hint).await
}

fn parse_token_embedding_response(
    json: &serde_json::Value,
    input: &str,
) -> Option<(Vec<Vec<f32>>, Vec<CharSpan>)> {
    let token_embeddings = parse_token_embedding_matrix(json)?;
    if token_embeddings.len() < 2 {
        return None;
    }
    let token_char_spans = parse_token_char_spans(json, input)?;
    if token_char_spans.len() != token_embeddings.len() {
        return None;
    }
    Some((token_embeddings, token_char_spans))
}

fn parse_token_embedding_matrix(json: &serde_json::Value) -> Option<Vec<Vec<f32>>> {
    if let Some(matrix) = json
        .get("token_embeddings")
        .and_then(parse_embedding_matrix)
    {
        return Some(matrix);
    }

    let embeddings = json.get("embeddings")?.as_array()?;
    let first = embeddings.first()?.as_array()?;
    if first.first().is_some_and(|value| value.is_array()) {
        return parse_embedding_matrix(embeddings.first()?);
    }
    if embeddings.len() > 1 {
        return parse_embedding_matrix(json.get("embeddings")?);
    }
    None
}

fn parse_embedding_matrix(value: &serde_json::Value) -> Option<Vec<Vec<f32>>> {
    let rows = value.as_array()?;
    let mut matrix = Vec::with_capacity(rows.len());
    for row in rows {
        let values = row.as_array()?;
        let vector: Vec<f32> = values
            .iter()
            .map(|value| value.as_f64().map(|number| number as f32))
            .collect::<Option<Vec<_>>>()?;
        if vector.is_empty() {
            return None;
        }
        matrix.push(vector);
    }
    Some(matrix)
}

fn parse_token_char_spans(json: &serde_json::Value, input: &str) -> Option<Vec<CharSpan>> {
    for key in ["token_spans", "token_offsets", "offsets"] {
        if let Some(spans) = json.get(key).and_then(parse_offset_spans) {
            return Some(spans);
        }
    }

    let tokens = first_batched_array(json.get("tokens")?)?;
    if tokens.first().is_some_and(|value| value.is_object()) {
        return parse_object_token_spans(tokens);
    }

    let token_texts: Vec<String> = tokens
        .iter()
        .map(|value| value.as_str().map(normalise_token_text))
        .collect::<Option<Vec<_>>>()?;
    infer_token_spans_from_text(input, &token_texts)
}

fn first_batched_array(value: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    let arr = value.as_array()?;
    if arr.first().is_some_and(|first| first.is_array()) {
        arr.first()?.as_array()
    } else {
        Some(arr)
    }
}

fn parse_offset_spans(value: &serde_json::Value) -> Option<Vec<CharSpan>> {
    let offsets = offset_span_array(value)?;
    offsets
        .iter()
        .map(|value| {
            if let Some(arr) = value.as_array() {
                let start = arr.first()?.as_u64()? as usize;
                let end = arr.get(1)?.as_u64()? as usize;
                return Some(CharSpan::new(start, end));
            }
            parse_object_span(value)
        })
        .collect()
}

fn offset_span_array(value: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    let arr = value.as_array()?;
    let first = arr.first()?;
    let first_inner = first.as_array();
    if first_inner
        .and_then(|inner| inner.first())
        .is_some_and(|value| value.is_array() || value.is_object())
    {
        first_inner
    } else {
        Some(arr)
    }
}

fn parse_object_token_spans(tokens: &[serde_json::Value]) -> Option<Vec<CharSpan>> {
    tokens.iter().map(parse_object_span).collect()
}

fn parse_object_span(value: &serde_json::Value) -> Option<CharSpan> {
    let start = value
        .get("start")
        .or_else(|| value.get("start_offset"))
        .or_else(|| value.get("offset_start"))
        .and_then(|value| value.as_u64())? as usize;
    let end = value
        .get("end")
        .or_else(|| value.get("end_offset"))
        .or_else(|| value.get("offset_end"))
        .and_then(|value| value.as_u64())? as usize;
    Some(CharSpan::new(start, end))
}

fn normalise_token_text(token: &str) -> String {
    token.replace(['▁', 'Ġ'], " ").replace('Ċ', "\n")
}

fn infer_token_spans_from_text(input: &str, tokens: &[String]) -> Option<Vec<CharSpan>> {
    let mut spans = Vec::with_capacity(tokens.len());
    let mut cursor = 0usize;
    for token in tokens {
        let token = normalise_token_text(token);
        if token.is_empty() {
            spans.push(CharSpan::new(cursor, cursor));
            continue;
        }
        let haystack = input.get(cursor..)?;
        let (relative, matched_len) = if let Some(relative) = haystack.find(&token) {
            (relative, token.len())
        } else {
            let trimmed = token.trim_start();
            if trimmed.is_empty() {
                spans.push(CharSpan::new(cursor, cursor));
                continue;
            }
            let relative = haystack.find(trimmed)?;
            (relative, trimmed.len())
        };
        let start = cursor + relative;
        let end = start + matched_len;
        spans.push(CharSpan::new(start, end));
        cursor = end;
    }
    Some(spans)
}

async fn is_known_unsupported(model: &str) -> bool {
    unsupported_models().lock().await.contains(model)
}

async fn mark_unsupported(model: &str, status: u16) {
    let inserted = unsupported_models().lock().await.insert(model.to_string());
    if inserted {
        // Log once per model so the user can see why embeddings are off
        // without their chat scrolling filling with retry warnings.
        eprintln!(
            "[brain] Ollama model '{model}' returned HTTP {status} from /api/embed; \
             disabling vector embeddings for this model. Install \
             `nomic-embed-text` (`ollama pull nomic-embed-text`) to re-enable \
             fast vector RAG."
        );
        // Force the model-choice cache to expire so the next call has a
        // chance to upgrade to nomic-embed-text if the user installs it.
        *embed_model_cache().lock().await = None;
    }
}

/// Reset both embedding caches. Call this whenever the brain mode or
/// active model changes so the next `embed_text` call re-probes
/// `/api/tags` and forgets about previously-unsupported models.
pub async fn clear_embed_caches() {
    *embed_model_cache().lock().await = None;
    unsupported_models().lock().await.clear();
}

/// Snapshot of the embedding cache state — for diagnostics and tests.
#[derive(Debug, Clone, Serialize)]
pub struct EmbedCacheSnapshot {
    pub chosen_model: Option<String>,
    pub chosen_age_secs: Option<u64>,
    pub unsupported: Vec<String>,
}

pub async fn embed_cache_snapshot() -> EmbedCacheSnapshot {
    let chosen = embed_model_cache().lock().await.clone();
    let unsupported: Vec<String> = unsupported_models().lock().await.iter().cloned().collect();
    EmbedCacheSnapshot {
        chosen_model: chosen.as_ref().map(|c| c.model.clone()),
        chosen_age_secs: chosen.as_ref().map(|c| c.chosen_at.elapsed().as_secs()),
        unsupported,
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

/// Progress event emitted during `pull_model_with_progress`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullProgress {
    /// Human-readable status line from Ollama (e.g. "pulling manifest").
    pub status: String,
    /// Layer/file digest being downloaded (empty for non-layer steps).
    #[serde(default)]
    pub digest: String,
    /// Total bytes for this layer (0 when unknown).
    #[serde(default)]
    pub total: u64,
    /// Bytes completed for this layer.
    #[serde(default)]
    pub completed: u64,
    /// Overall percentage (0–100) computed across all layers.
    pub percent: u8,
}

/// Pull an Ollama model, consuming the streaming progress response.
pub async fn pull_model(client: &Client, base_url: &str, model_name: &str) -> Result<(), String> {
    pull_model_with_progress(client, base_url, model_name, |_| {}).await
}

/// Pull an Ollama model with a per-event progress callback.
///
/// Ollama's `/api/pull` returns NDJSON lines with fields:
/// `{ "status": "…", "digest": "…", "total": N, "completed": N }`
pub async fn pull_model_with_progress<F>(
    client: &Client,
    base_url: &str,
    model_name: &str,
    on_progress: F,
) -> Result<(), String>
where
    F: Fn(PullProgress),
{
    #[derive(Serialize)]
    struct PullRequest<'a> {
        name: &'a str,
        stream: bool,
    }

    #[derive(Deserialize)]
    struct PullLine {
        #[serde(default)]
        status: String,
        #[serde(default)]
        digest: String,
        #[serde(default)]
        total: u64,
        #[serde(default)]
        completed: u64,
        #[serde(default)]
        error: Option<String>,
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
        return Err(format!("Ollama pull failed with status {}", resp.status()));
    }

    // Track per-layer progress so we can compute an overall percentage.
    // Ollama sends separate total/completed for each layer digest.
    let mut layer_totals: std::collections::HashMap<String, (u64, u64)> =
        std::collections::HashMap::new();
    let mut buf = String::new();
    let mut saw_success = false;

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("stream error: {e}"))?;
        buf.push_str(&String::from_utf8_lossy(&bytes));

        // Process complete lines from the buffer.
        while let Some(newline_pos) = buf.find('\n') {
            let line: String = buf.drain(..=newline_pos).collect();
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(parsed) = serde_json::from_str::<PullLine>(trimmed) else {
                continue;
            };

            // Ollama surfaces errors inline.
            if let Some(err) = parsed.error {
                return Err(format!("Ollama pull error: {err}"));
            }

            if parsed.status == "success" {
                saw_success = true;
            }

            // Update per-layer tracking.
            if !parsed.digest.is_empty() && parsed.total > 0 {
                layer_totals.insert(parsed.digest.clone(), (parsed.total, parsed.completed));
            }

            // Compute overall percentage across all known layers.
            let (sum_total, sum_done) = layer_totals
                .values()
                .fold((0u64, 0u64), |(t, d), (lt, ld)| (t + lt, d + ld));
            let percent = if sum_total > 0 {
                ((sum_done as f64 / sum_total as f64) * 100.0).min(100.0) as u8
            } else if parsed.status == "success" {
                100
            } else {
                0
            };

            on_progress(PullProgress {
                status: parsed.status,
                digest: parsed.digest,
                total: parsed.total,
                completed: parsed.completed,
                percent,
            });
        }
    }

    if !saw_success {
        return Err("Ollama pull stream ended without a success status — \
             the download may have been interrupted or the model may not exist"
            .to_string());
    }

    Ok(())
}

/// Delete an Ollama model from the local registry.
pub async fn delete_model(client: &Client, base_url: &str, model_name: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct DeleteRequest<'a> {
        name: &'a str,
    }

    let url = format!("{base_url}/api/delete");
    let resp = client
        .delete(&url)
        .json(&DeleteRequest { name: model_name })
        .send()
        .await
        .map_err(|e| format!("Ollama not reachable: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Ollama delete failed with status {}",
            resp.status()
        ));
    }

    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_token_embedding_response_accepts_token_embeddings_with_offsets() {
        let json = serde_json::json!({
            "token_embeddings": [[1.0, 0.0], [0.0, 1.0]],
            "offsets": [[0, 5], [5, 11]]
        });
        let (embeddings, spans) = parse_token_embedding_response(&json, "Hello world").unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(spans, vec![CharSpan::new(0, 5), CharSpan::new(5, 11)]);
    }

    #[test]
    fn parse_token_embedding_response_accepts_batched_embeddings_with_token_text() {
        let json = serde_json::json!({
            "embeddings": [[[1.0, 0.0], [0.0, 1.0]]],
            "tokens": [["Hello", " world"]]
        });
        let (embeddings, spans) = parse_token_embedding_response(&json, "Hello world").unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(spans, vec![CharSpan::new(0, 5), CharSpan::new(5, 11)]);
    }

    #[test]
    fn parse_token_embedding_response_rejects_pooled_embedding_response() {
        let json = serde_json::json!({
            "embeddings": [[0.1, 0.2, 0.3]]
        });
        assert!(parse_token_embedding_response(&json, "Hello world").is_none());
    }

    #[test]
    fn infer_token_spans_handles_sentencepiece_space_marker() {
        let spans = infer_token_spans_from_text(
            "Hello world",
            &["Hello".to_string(), "▁world".to_string()],
        )
        .unwrap();
        assert_eq!(spans, vec![CharSpan::new(0, 5), CharSpan::new(5, 11)]);
    }

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
    fn infer_sentiment_always_neutral() {
        // Keyword-based sentiment was removed — the LLM decides emotion
        // via `<anim>` tags in the streaming pipeline. The static fallback
        // always returns Neutral.
        assert_eq!(
            OllamaAgent::infer_sentiment("I'm sorry, I can't help with that."),
            Sentiment::Neutral
        );
        assert_eq!(
            OllamaAgent::infer_sentiment("This is a wonderful explanation!"),
            Sentiment::Neutral
        );
        assert_eq!(
            OllamaAgent::infer_sentiment("Hi there! I'm happy to help you with that."),
            Sentiment::Neutral
        );
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
        assert!(msgs[0].content.contains("RETRIEVED CONTEXT"));
        assert!(msgs[0].content.contains("not an exhaustive transcript"));
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

    // ── Embedding cache resilience tests ─────────────────────────────

    /// Serialization guard for tests that mutate the shared embed caches.
    /// All cache-related tests must hold this lock to avoid cross-test
    /// races (the caches are process-global statics).
    static EMBED_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[tokio::test]
    async fn embed_text_short_circuits_on_empty_input() {
        let _guard = EMBED_TEST_LOCK.lock().await;
        // No HTTP request should be made for empty input — proves that
        // empty messages can't accidentally hammer /api/embed.
        clear_embed_caches().await;
        assert!(OllamaAgent::embed_text("", "gemma3:4b").await.is_none());
        assert!(OllamaAgent::embed_text("   \n\t  ", "gemma3:4b")
            .await
            .is_none());
    }

    #[tokio::test]
    async fn unsupported_model_is_remembered_and_short_circuits() {
        let _guard = EMBED_TEST_LOCK.lock().await;
        let _url_guard = use_test_ollama_base_url("http://127.0.0.1:19999");
        clear_embed_caches().await;
        // Mark a fake model as unsupported.
        mark_unsupported("fake-model:1b", 501).await;
        assert!(is_known_unsupported("fake-model:1b").await);

        // A second mark for the same model is a no-op (no log spam).
        mark_unsupported("fake-model:1b", 501).await;

        // embed_text must short-circuit and never hit the network.
        assert!(OllamaAgent::embed_text("hello world", "fake-model:1b")
            .await
            .is_none());

        // Snapshot exposes the unsupported model.
        let snap = embed_cache_snapshot().await;
        assert!(snap.unsupported.iter().any(|m| m == "fake-model:1b"));
    }

    #[tokio::test]
    async fn clear_embed_caches_forgets_unsupported_models() {
        let _guard = EMBED_TEST_LOCK.lock().await;
        clear_embed_caches().await;
        mark_unsupported("forget-me:7b", 400).await;
        assert!(is_known_unsupported("forget-me:7b").await);

        clear_embed_caches().await;

        assert!(!is_known_unsupported("forget-me:7b").await);
        let snap = embed_cache_snapshot().await;
        assert!(snap.chosen_model.is_none());
        assert!(!snap.unsupported.iter().any(|m| m == "forget-me:7b"));
    }

    #[tokio::test]
    async fn embed_text_returns_none_when_daemon_unreachable() {
        let _guard = EMBED_TEST_LOCK.lock().await;
        let _url_guard = use_test_ollama_base_url("http://127.0.0.1:19999");
        // No Ollama running on this test URL — must return None gracefully
        // rather than panic or hang. The tight client timeout protects us.
        clear_embed_caches().await;
        // Use a chat-model name so resolve_embed_model picks it as the
        // fallback (model_exists("nomic-embed-text") will fail too).
        let result = OllamaAgent::embed_text("hello", "definitely-not-installed:1b").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn embed_text_batch_returns_none_per_item_when_unreachable() {
        let _guard = EMBED_TEST_LOCK.lock().await;
        let _url_guard = use_test_ollama_base_url("http://127.0.0.1:19999");
        clear_embed_caches().await;
        let texts = vec!["hello", "world", ""];
        let results = OllamaAgent::embed_text_batch(&texts, "definitely-not-installed:1b", 2).await;
        assert_eq!(results.len(), 3);
        // All should be None since the isolated test URL is unreachable.
        for r in &results {
            assert!(r.is_none());
        }
    }

    #[tokio::test]
    async fn embed_text_batch_handles_empty_input() {
        let results = OllamaAgent::embed_text_batch(&[], "gemma3:4b", 32).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn embed_cache_snapshot_serializable() {
        let _guard = EMBED_TEST_LOCK.lock().await;
        clear_embed_caches().await;
        mark_unsupported("snap-test:3b", 501).await;
        let snap = embed_cache_snapshot().await;
        // Must round-trip through serde for the Tauri command surface.
        let json = serde_json::to_string(&snap).expect("snapshot serializes");
        assert!(json.contains("snap-test:3b"));
        clear_embed_caches().await;
    }

    // ---- Chunk 16.9b — fallback chain tests ------------------------------

    #[tokio::test]
    async fn fallback_chain_falls_through_to_hint_when_nothing_installed() {
        let _guard = EMBED_TEST_LOCK.lock().await;
        let _url_guard = use_test_ollama_base_url("http://127.0.0.1:19999");
        clear_embed_caches().await;
        // No Ollama daemon (or none of the embed models present) —
        // resolution should yield the model_hint as the last resort.
        let chosen = pick_embed_model("my-chat-model:7b").await;
        assert_eq!(chosen, "my-chat-model:7b");
    }

    #[tokio::test]
    async fn fallback_chain_skips_known_unsupported_preferred() {
        let _guard = EMBED_TEST_LOCK.lock().await;
        let _url_guard = use_test_ollama_base_url("http://127.0.0.1:19999");
        clear_embed_caches().await;
        // Mark the preferred model as unsupported. Resolution must NOT
        // pick it even if it's somehow reachable.
        mark_unsupported(PREFERRED_EMBED_MODEL, 501).await;
        let chosen = pick_embed_model("hint-model:7b").await;
        // Without a real Ollama, all fallbacks miss → returns hint.
        assert_eq!(chosen, "hint-model:7b");
        // The preferred model must never have been picked.
        assert_ne!(chosen, PREFERRED_EMBED_MODEL);
        clear_embed_caches().await;
    }

    #[tokio::test]
    async fn fallback_chain_constants_are_well_formed() {
        // Enforce the contract: every fallback is a bare model name (no
        // `:tag`, no whitespace) and the preferred model isn't duplicated.
        for name in EMBED_MODEL_FALLBACKS {
            assert!(!name.is_empty(), "fallback name must not be empty");
            assert!(!name.contains(':'), "fallback {name} must not include :tag");
            assert!(
                !name.chars().any(char::is_whitespace),
                "fallback {name} must not contain whitespace"
            );
            assert_ne!(
                *name, PREFERRED_EMBED_MODEL,
                "fallback list must not contain the preferred model"
            );
        }
        // De-duplication.
        let mut seen: HashSet<&&str> = HashSet::new();
        for name in EMBED_MODEL_FALLBACKS {
            assert!(seen.insert(name), "duplicate fallback model: {name}");
        }
    }

    #[tokio::test]
    async fn fallback_chain_skips_unsupported_fallbacks() {
        let _guard = EMBED_TEST_LOCK.lock().await;
        let _url_guard = use_test_ollama_base_url("http://127.0.0.1:19999");
        clear_embed_caches().await;
        // Mark every fallback as unsupported. Resolution must walk past
        // them all and land on the chat-model hint.
        mark_unsupported(PREFERRED_EMBED_MODEL, 501).await;
        for name in EMBED_MODEL_FALLBACKS {
            mark_unsupported(name, 501).await;
        }
        let chosen = pick_embed_model("hint-only:7b").await;
        assert_eq!(chosen, "hint-only:7b");
        clear_embed_caches().await;
    }
}
