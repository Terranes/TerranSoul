use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

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
                content: "You extract typed relationships between memories. Reply with JSON lines only.".to_string(),
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
            ChatMessage { role: "system".to_string(), content: system },
            ChatMessage { role: "user".to_string(),   content: user },
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
            ChatMessage { role: "system".to_string(), content: system },
            ChatMessage { role: "user".to_string(),   content: user },
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
        let (system, user) = crate::persona::extract::build_persona_prompt(snippets);
        let msgs = vec![
            ChatMessage { role: "system".to_string(), content: system },
            ChatMessage { role: "user".to_string(),   content: user },
        ];
        let (reply, _) = self.call(msgs).await;
        crate::persona::extract::parse_persona_reply(&reply)
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

        // 3. Bound every embed call so a hung daemon can't stall the chat
        //    pipeline forever.
        let client = match Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
        {
            Ok(c) => c,
            Err(_) => return None,
        };

        let body = serde_json::json!({
            "model": embed_model,
            "input": text,
        });

        let resp = match client
            .post(format!("{OLLAMA_BASE_URL}/api/embed"))
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
        let vec: Vec<f32> = arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect();
        if vec.is_empty() { None } else { Some(vec) }
    }

    /// Check if a model name is available locally in Ollama.
    /// Result is cached for 60 s by [`resolve_embed_model`].
    async fn model_exists(name: &str) -> bool {
        let client = match Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };
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

fn embed_model_cache() -> &'static Mutex<Option<EmbedModelChoice>> {
    static CACHE: OnceLock<Mutex<Option<EmbedModelChoice>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn unsupported_models() -> &'static Mutex<HashSet<String>> {
    static SET: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SET.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Resolve which model to pass to `/api/embed`, caching the choice for
/// 60 seconds. Prefers `nomic-embed-text` when it's installed locally;
/// otherwise falls back to the chat-model hint (which may itself be
/// unsupported — in which case `embed_text` will mark it as such on the
/// next call and refuse to retry).
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

    // Cache miss / expired → probe once.
    let chosen = if OllamaAgent::model_exists(PREFERRED_EMBED_MODEL).await {
        PREFERRED_EMBED_MODEL.to_string()
    } else {
        model_hint.to_string()
    };

    let mut guard = cache.lock().await;
    *guard = Some(EmbedModelChoice {
        model: chosen.clone(),
        chosen_at: Instant::now(),
    });
    chosen
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
    let unsupported: Vec<String> = unsupported_models()
        .lock()
        .await
        .iter()
        .cloned()
        .collect();
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

    // ── Embedding cache resilience tests ─────────────────────────────

    #[tokio::test]
    async fn embed_text_short_circuits_on_empty_input() {
        // No HTTP request should be made for empty input — proves that
        // empty messages can't accidentally hammer /api/embed.
        clear_embed_caches().await;
        assert!(OllamaAgent::embed_text("", "gemma3:4b").await.is_none());
        assert!(OllamaAgent::embed_text("   \n\t  ", "gemma3:4b").await.is_none());
    }

    #[tokio::test]
    async fn unsupported_model_is_remembered_and_short_circuits() {
        clear_embed_caches().await;
        // Mark a fake model as unsupported.
        mark_unsupported("fake-model:1b", 501).await;
        assert!(is_known_unsupported("fake-model:1b").await);

        // A second mark for the same model is a no-op (no log spam).
        mark_unsupported("fake-model:1b", 501).await;

        // embed_text must short-circuit and never hit the network.
        assert!(
            OllamaAgent::embed_text("hello world", "fake-model:1b")
                .await
                .is_none()
        );

        // Snapshot exposes the unsupported model.
        let snap = embed_cache_snapshot().await;
        assert!(snap.unsupported.iter().any(|m| m == "fake-model:1b"));
    }

    #[tokio::test]
    async fn clear_embed_caches_forgets_unsupported_models() {
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
        // No Ollama running on this port — must return None gracefully
        // rather than panic or hang. The 15-s client timeout protects us.
        clear_embed_caches().await;
        // Use a chat-model name so resolve_embed_model picks it as the
        // fallback (model_exists("nomic-embed-text") will fail too).
        let result = OllamaAgent::embed_text("hello", "definitely-not-installed:1b").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn embed_cache_snapshot_serializable() {
        clear_embed_caches().await;
        mark_unsupported("snap-test:3b", 501).await;
        let snap = embed_cache_snapshot().await;
        // Must round-trip through serde for the Tauri command surface.
        let json = serde_json::to_string(&snap).expect("snapshot serializes");
        assert!(json.contains("snap-test:3b"));
        clear_embed_caches().await;
    }
}
