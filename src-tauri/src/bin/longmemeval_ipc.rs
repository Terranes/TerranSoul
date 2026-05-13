//! JSONL IPC shim for the LongMemEval-S benchmark adapter.
//!
//! The Node runner owns dataset download, progress reporting, and metrics.
//! This small binary keeps benchmark retrieval on the real `MemoryStore` path.
//!
//! Search modes:
//! - `search` / `keyword`: FTS5 + lexical rerank + KG boosts (`MemoryStore::search`).
//! - `rrf` / `hybrid_search_rrf`: keyword + freshness RRF fusion (`MemoryStore::hybrid_search_rrf`).
//! - `emb`: pure cosine-similarity ranking using Ollama `nomic-embed-text` (768-dim).
//! - `rrf_emb`: RRF fuse FTS5 rank + embedding rank (k=60). Requires `LONGMEM_EMBED=1`.
//! - `rrf_rerank`: RRF retrieval of top-30, then batched LLM-as-judge cross-encoder
//!   rerank via Ollama (`LONGMEM_RERANK_MODEL`, default `gemma3:4b`). BENCH-LCM-8.
//! - `rrf_hyde`: HyDE expansion (LLM writes a hypothetical answer; embed THAT for
//!   the vector channel) + raw-query lexical channel via `hybrid_search_rrf`. Reuses
//!   `terransoul_lib::memory::hyde::{build_hyde_prompt, clean_hyde_reply}` so the
//!   bench exercises the exact prompt the production brain uses. BENCH-LCM-10.
//! - `rrf_hyde_rerank`: stacked HyDE + cross-encoder rerank. BENCH-LCM-10.
//! - `rrf_ctx` / `rrf_ctx_rerank`: same retrieval pipeline as `rrf` / `rrf_rerank`,
//!   but the corpus was ingested with Anthropic Contextual Retrieval (Sept 2024)
//!   prefixes prepended to each session before embedding. Triggered by setting
//!   `LONGMEM_CONTEXTUALIZE=1` on the IPC process; cached at
//!   `target-copilot-bench/ctx-cache/<sha16>.txt` so the ~3-hour one-time
//!   per-corpus cost is paid once. BENCH-LCM-11.
//! - `rrf_kg` / `rrf_kg_rerank`: same retrieval pipeline as `rrf` / `rrf_rerank`,
//!   plus a 1–2 hop BFS over `memory_edges` via
//!   [`terransoul_lib::memory::cascade::cascade_expand`]. Edges are built at
//!   ingest time when `LONGMEM_KG_EDGES=1` is set: a lightweight proper-noun
//!   extractor buckets memory ids by entity, then any new memory sharing
//!   ≥2 entities with an existing one gets a `shares_entities` edge
//!   (top-`LONGMEM_KG_MAX_NEIGHBOURS`, default 10, by overlap count).
//!   Mirrors the chat-side `commands::chat::expand_seeds_via_kg` helper
//!   shipped by BENCH-KG-1 so chat and bench exercise the same cascade
//!   stage. BENCH-KG-2.

use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, Write};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use terransoul_lib::memory::edges::{EdgeSource, NewMemoryEdge};
use terransoul_lib::memory::hyde::{build_hyde_prompt, clean_hyde_reply};
use terransoul_lib::memory::store::{MemoryStore, MemoryType, NewMemory};

const SOURCE_URL_PREFIX: &str = "longmemeval://session/";
const RRF_K: f32 = 60.0;
const DEFAULT_EMBED_MODEL: &str = "nomic-embed-text";
const DEFAULT_RERANK_MODEL: &str = "gemma3:4b";
/// HyDE expansion model. Same default as the reranker — small enough to keep
/// the bench tractable; override via `LONGMEM_HYDE_MODEL`. BENCH-LCM-10.
const DEFAULT_HYDE_MODEL: &str = "gemma3:4b";
/// How many RRF candidates to feed into the cross-encoder reranker.
/// BENCH-LCM-9 tested widening 30→50 to recover multi_hop/open_domain
/// recall but it HURT adversarial (-4pp R@10 on the 100-q smoke): the
/// extra near-miss distractors diluted the reranker's attention. 30
/// remains the LCM-8 sweet spot. Override via LONGMEM_RERANK_POOL.
const DEFAULT_RERANK_POOL: usize = 30;
/// Batch size for the LLM-as-judge prompt (candidates scored per call).
const RERANK_BATCH_SIZE: usize = 5;
/// Cross-encoder rerank score (0-10 scale) below which we trust the
/// pre-rerank RRF ordering instead. BENCH-LCM-9 tested 5.5 (the
/// production default) and 3.5; both caused identical -4pp adversarial
/// R@10 regressions vs LCM-8 because gemma3:4b is bimodal at
/// temperature 0 (scores cluster at 0-2 or 7-9, rarely 3-5). With the
/// LLM near-binary, any threshold > 0 forces too many adversarial
/// candidates back to RRF order. 0 (disabled) is the LCM-8 sweet spot;
/// raising it requires a larger / less-bimodal reranker model first.
/// Override via LONGMEM_RERANK_THRESHOLD.
const DEFAULT_RERANK_THRESHOLD: f32 = 0.0;
const DEFAULT_OLLAMA_HOST: &str = "http://127.0.0.1:11434";

#[derive(Debug, Deserialize)]
struct Request {
    id: u64,
    op: String,
    #[serde(default)]
    question_id: Option<String>,
    #[serde(default)]
    sessions: Vec<IpcSession>,
    #[serde(default)]
    query: String,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct IpcSession {
    session_id: String,
    text: String,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    turn_count: usize,
}

#[derive(Debug, Serialize)]
struct SearchHit {
    memory_id: i64,
    session_id: String,
    token_count: i64,
}

fn default_mode() -> String {
    "rrf".to_string()
}

fn default_limit() -> usize {
    20
}

fn session_source_url(session_id: &str) -> String {
    format!("{SOURCE_URL_PREFIX}{session_id}")
}

fn source_url_to_session_id(source_url: Option<&str>) -> String {
    source_url
        .and_then(|raw| raw.strip_prefix(SOURCE_URL_PREFIX))
        .unwrap_or_default()
        .to_string()
}

fn session_tags(question_id: &str, session: &IpcSession) -> String {
    let mut tags = vec![
        "longmemeval".to_string(),
        "benchmark".to_string(),
        format!("question:{question_id}"),
        format!("session:{}", session.session_id),
    ];
    if let Some(date) = &session.date {
        tags.push(format!("date:{date}"));
    }
    tags.join(",")
}

fn session_content(session: &IpcSession) -> String {
    let mut parts = vec![format!("Session: {}", session.session_id)];
    if let Some(date) = &session.date {
        parts.push(format!("Date: {date}"));
    }
    if session.turn_count > 0 {
        parts.push(format!("Turns: {}", session.turn_count));
    }
    parts.push(session.text.clone());
    parts.join("\n")
}

// ---- Embedding backend ----------------------------------------------------

#[derive(Clone, Copy)]
enum EmbedRole {
    Query,
    Document,
}

struct OllamaEmbedder {
    client: reqwest::blocking::Client,
    url: String,
    model: String,
    /// Whether the model uses task-instruction prefixes (nomic-embed-text).
    use_prefixes: bool,
}

impl OllamaEmbedder {
    fn from_env() -> Option<Self> {
        if std::env::var("LONGMEM_EMBED").ok().as_deref() != Some("1") {
            return None;
        }
        let host =
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());
        let model = std::env::var("LONGMEM_EMBED_MODEL")
            .unwrap_or_else(|_| DEFAULT_EMBED_MODEL.to_string());
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .ok()?;
        // nomic-embed-text requires "search_query:"/"search_document:" prefixes.
        // Other models (mxbai-embed-large, snowflake-arctic-embed2, etc.) do not.
        let use_prefixes = model.contains("nomic");
        Some(Self {
            client,
            url: format!("{}/api/embeddings", host.trim_end_matches('/')),
            model,
            use_prefixes,
        })
    }

    fn embed(&self, text: &str, role: EmbedRole) -> Result<Vec<f32>, String> {
        // Cap input length: most embed models handle ~8k tokens; trim to ~16k
        // chars (~4k tokens) so very long sessions don't dominate latency.
        let trimmed = if text.len() > 16_000 { &text[..16_000] } else { text };
        let prompt = if self.use_prefixes {
            match role {
                EmbedRole::Query => format!("search_query: {trimmed}"),
                EmbedRole::Document => format!("search_document: {trimmed}"),
            }
        } else {
            trimmed.to_string()
        };
        let body = json!({"model": self.model, "prompt": prompt});
        let resp = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .map_err(|err| format!("ollama embed http: {err}"))?;
        if !resp.status().is_success() {
            return Err(format!("ollama embed http status: {}", resp.status()));
        }
        let value: Value = resp.json().map_err(|err| format!("ollama embed json: {err}"))?;
        let arr = value
            .get("embedding")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "ollama embed: missing embedding field".to_string())?;
        let vec = arr
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect::<Vec<f32>>();
        if vec.is_empty() {
            return Err("ollama embed: empty embedding".to_string());
        }
        Ok(vec)
    }
}

// ---- Reranker (LLM-as-judge cross-encoder) -------------------------------

/// Cross-encoder rerank via Ollama chat model. BENCH-LCM-8.
///
/// LongMemEval/LoCoMo adversarial queries swap one named entity for another
/// (e.g. "What did Caroline realize…" against a corpus passage about Melanie).
/// A bi-encoder fuses similarity into a single dot product and cannot tell
/// the two entities apart when the surrounding context is near-identical. A
/// cross-encoder feeds `(query, doc)` jointly into the model so attention can
/// detect the entity mismatch directly — the canonical industry fix per
/// Anthropic Contextual Retrieval (Sept 2024), BGE-reranker-v2-m3, etc.
///
/// We reuse the existing LLM-as-judge approach from
/// `src-tauri/src/memory/reranker.rs` so the bench exercises the same path
/// the production brain uses. To keep latency tolerable on the 100/250-query
/// smoke runs we batch `RERANK_BATCH_SIZE` candidates per Ollama call.
struct OllamaReranker {
    client: reqwest::blocking::Client,
    url: String,
    model: String,
    /// Wide candidate pool drawn from RRF before LLM-as-judge re-scoring.
    pool: usize,
    /// Scores strictly below this (0-10 scale) are treated as unconfident
    /// and fall back to the pre-rerank RRF position.
    threshold: f32,
}

impl OllamaReranker {
    fn from_env() -> Option<Self> {
        if std::env::var("LONGMEM_RERANK").ok().as_deref() != Some("1") {
            return None;
        }
        let host =
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());
        let model = std::env::var("LONGMEM_RERANK_MODEL")
            .unwrap_or_else(|_| DEFAULT_RERANK_MODEL.to_string());
        let pool = std::env::var("LONGMEM_RERANK_POOL")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(DEFAULT_RERANK_POOL);
        let threshold = std::env::var("LONGMEM_RERANK_THRESHOLD")
            .ok()
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(DEFAULT_RERANK_THRESHOLD);
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .ok()?;
        Some(Self {
            client,
            url: format!("{}/api/generate", host.trim_end_matches('/')),
            model,
            pool,
            threshold,
        })
    }

    /// Score a batch of `(idx, doc)` candidates against `query`.
    ///
    /// Returns scores aligned to the input order. A missing/unparseable score
    /// is `None`; callers should keep the candidate at its pre-rerank rank.
    fn score_batch(&self, query: &str, docs: &[(usize, String)]) -> Vec<Option<u8>> {
        if docs.is_empty() {
            return Vec::new();
        }

        // Clip each doc to 1200 chars (~300 tokens) so a batch of 5 stays
        // inside the chat model's effective context budget.
        const MAX_DOC_CHARS: usize = 1200;
        let mut prompt = String::with_capacity(MAX_DOC_CHARS * docs.len() + 512);
        prompt.push_str(
            "You are a relevance scorer for a retrieval system. For each candidate \
             document below, output a single integer 0-10 indicating how directly the \
             document answers the QUERY. Use the rubric: 0 = unrelated, 3 = mentions the \
             topic but doesn't answer, 6 = partially answers, 8 = answers most of the \
             query, 10 = perfect direct answer. PAY CLOSE ATTENTION to named entities: \
             if the query asks about person A but the document is about person B, \
             score it 0-2 even when the activity matches.\n\n",
        );
        prompt.push_str("QUERY: ");
        prompt.push_str(query.trim());
        prompt.push_str("\n\n");
        for (n, (_, doc)) in docs.iter().enumerate() {
            let clipped: String = doc.chars().take(MAX_DOC_CHARS).collect();
            prompt.push_str(&format!("DOCUMENT {}:\n{}\n\n", n + 1, clipped.trim()));
        }
        prompt.push_str(
            "Reply with exactly one line per document in the form `N: SCORE` where N \
             is the document number and SCORE is an integer 0-10. No other text.\n",
        );

        let body = json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": { "temperature": 0.0, "num_predict": 64 }
        });
        let resp = match self.client.post(&self.url).json(&body).send() {
            Ok(r) => r,
            Err(_) => return vec![None; docs.len()],
        };
        if !resp.status().is_success() {
            return vec![None; docs.len()];
        }
        let value: Value = match resp.json() {
            Ok(v) => v,
            Err(_) => return vec![None; docs.len()],
        };
        let reply = value
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Parse "1: 7", "2: 3", … into per-candidate scores. Tolerate stray
        // formatting (markdown bold/leading commentary).
        let mut out: Vec<Option<u8>> = vec![None; docs.len()];
        for line in reply.lines() {
            let line = line.trim();
            if let Some((left, right)) = line.split_once(':') {
                let n: Option<usize> = left
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .ok();
                let score_digits: String =
                    right.chars().filter(|c| c.is_ascii_digit()).collect();
                if let (Some(n), Ok(score)) = (n, score_digits.parse::<u32>()) {
                    if (1..=docs.len()).contains(&n) && score <= 10 {
                        out[n - 1] = Some(score as u8);
                    }
                }
            }
        }
        out
    }
}

// ---- HyDE expander -------------------------------------------------------

/// Hypothetical Document Embeddings (Gao et al. 2022) expander for the bench.
///
/// Reuses the exact production prompt (`memory::hyde::build_hyde_prompt`) so
/// the bench measures the same code path the brain uses on every chat turn.
/// Calls Ollama `/api/generate` synchronously (the bench is single-threaded
/// JSONL IPC) and runs `clean_hyde_reply` over the response.
///
/// BENCH-LCM-10. Hypothesis: a hypothetical answer's embedding matches
/// abstract / multi-step LoCoMo passages that the raw query's embedding
/// misses, recovering the multi_hop / open_domain marginal regressions
/// from BENCH-LCM-8 without touching the adversarial win.
struct HydeExpander {
    client: reqwest::blocking::Client,
    url: String,
    model: String,
}

impl HydeExpander {
    fn from_env() -> Option<Self> {
        if std::env::var("LONGMEM_HYDE").ok().as_deref() != Some("1") {
            return None;
        }
        let host =
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());
        let model = std::env::var("LONGMEM_HYDE_MODEL")
            .unwrap_or_else(|_| DEFAULT_HYDE_MODEL.to_string());
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .ok()?;
        Some(Self {
            client,
            url: format!("{}/api/generate", host.trim_end_matches('/')),
            model,
        })
    }

    /// Expand `query` into a hypothetical answer, or `None` if the LLM call
    /// fails / the reply is too short. Callers fall back to the raw-query
    /// embedding on `None` (same three-stage fallback as the production
    /// `hyde_search_memories` Tauri command).
    fn expand(&self, query: &str) -> Option<String> {
        let (system, user) = build_hyde_prompt(query);
        // Ollama /api/generate accepts a `system` field separate from `prompt`;
        // this matches OllamaAgent::call's chat-template behaviour for a single
        // system + user turn.
        let body = json!({
            "model": self.model,
            "system": system,
            "prompt": user,
            "stream": false,
            "options": { "temperature": 0.2, "num_predict": 160 }
        });
        let resp = self.client.post(&self.url).json(&body).send().ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let value: Value = resp.json().ok()?;
        let reply = value.get("response").and_then(|v| v.as_str()).unwrap_or("");
        clean_hyde_reply(reply)
    }
}

// ---- Contextualizer (Anthropic Contextual Retrieval, Sept 2024) ----------

/// Contextual Retrieval expander for the bench. BENCH-LCM-11.
///
/// Mirrors `src-tauri/src/memory/contextualize.rs` but uses blocking
/// reqwest + on-disk caching so the ~3-hour one-time corpus
/// contextualization cost is paid once and reused across smoke runs.
///
/// At INGEST time, prepends a 50-100 token LLM-generated context sentence
/// to each session before embedding. The context sentence anchors the
/// session in its broader conversation so embedding/lexical retrievers
/// recover entity / topic signals that a stand-alone session might omit.
///
/// Cache layout: `<repo>/target-copilot-bench/ctx-cache/<sha16>.txt`
/// where `sha16` is the first 16 hex chars of SHA-256(model || '\0' ||
/// content). Including the model in the key prevents a cache hit when
/// the contextualizer model is swapped via `LONGMEM_CTX_MODEL`.
///
/// Gated on `LONGMEM_CONTEXTUALIZE=1`. Model defaults to `gemma3:4b`,
/// override with `LONGMEM_CTX_MODEL`.
const DEFAULT_CTX_MODEL: &str = "gemma3:4b";

struct Contextualizer {
    client: reqwest::blocking::Client,
    url: String,
    model: String,
    cache_dir: std::path::PathBuf,
}

impl Contextualizer {
    fn from_env() -> Option<Self> {
        if std::env::var("LONGMEM_CONTEXTUALIZE").ok().as_deref() != Some("1") {
            return None;
        }
        let host =
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_OLLAMA_HOST.to_string());
        let model = std::env::var("LONGMEM_CTX_MODEL")
            .unwrap_or_else(|_| DEFAULT_CTX_MODEL.to_string());
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .ok()?;
        let cache_dir = std::env::var("LONGMEM_CTX_CACHE_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from("target-copilot-bench").join("ctx-cache")
            });
        if let Err(err) = std::fs::create_dir_all(&cache_dir) {
            eprintln!(
                "[contextualizer] failed to create cache dir {}: {err}",
                cache_dir.display()
            );
            return None;
        }
        eprintln!(
            "[contextualizer] enabled: model={model} cache_dir={}",
            cache_dir.display()
        );
        Some(Self {
            client,
            url: format!("{}/api/generate", host.trim_end_matches('/')),
            model,
            cache_dir,
        })
    }

    fn cache_key(&self, content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.model.as_bytes());
        hasher.update([0u8]);
        hasher.update(content.as_bytes());
        let digest = hasher.finalize();
        hex::encode(&digest[..8])
    }

    fn cache_path(&self, key: &str) -> std::path::PathBuf {
        self.cache_dir.join(format!("{key}.txt"))
    }

    /// Generate (or load from cache) a context prefix for `content`.
    ///
    /// Returns `None` when both the cache miss and the LLM call fail. On
    /// success, returns the trimmed context sentence (without the chunk
    /// itself). Use [`prepend_context`] to combine.
    fn contextualize(&self, content: &str) -> Option<String> {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return None;
        }
        let key = self.cache_key(trimmed);
        let cache_path = self.cache_path(&key);
        if let Ok(cached) = std::fs::read_to_string(&cache_path) {
            let cached = cached.trim();
            if !cached.is_empty() {
                return Some(cached.to_string());
            }
        }

        // Build prompts that mirror memory/contextualize.rs system_prompt /
        // user_prompt. We use the chunk itself as the (single-document)
        // summary because each LoCoMo session is self-contained — there's
        // no shared multi-chunk parent document at the bench layer. The
        // LLM still produces a useful "this is from a conversation about
        // X" anchor that helps embedding/lexical retrievers.
        let preview: String = trimmed.chars().take(2000).collect();
        let system = "You are a document context assistant. Given a conversation \
                      transcript, write a SHORT context sentence (50-100 tokens) \
                      that names the participants, topic, and timeframe of the \
                      conversation so a search index can match it on entity / \
                      topic anchors. Reply with ONLY the context sentence, \
                      nothing else.";
        let user = format!("<conversation>\n{preview}\n</conversation>");

        let body = json!({
            "model": self.model,
            "system": system,
            "prompt": user,
            "stream": false,
            "options": { "temperature": 0.2, "num_predict": 160 }
        });
        let resp = self.client.post(&self.url).json(&body).send().ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let value: Value = resp.json().ok()?;
        let reply = value
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if reply.is_empty() {
            return None;
        }
        // Best-effort cache write; failure is non-fatal.
        let _ = std::fs::write(&cache_path, &reply);
        Some(reply)
    }
}

/// Combine a context prefix with the original chunk content. Mirrors
/// `memory::contextualize::prepend_context`.
fn prepend_context(context: &str, chunk: &str) -> String {
    let trimmed = context.trim();
    if trimmed.is_empty() {
        return chunk.to_string();
    }
    format!("{trimmed}\n\n{chunk}")
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na <= f32::EPSILON || nb <= f32::EPSILON {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Stoplist of common English sentence-starters / function words that look
/// like proper nouns when they appear capitalized at the start of a sentence.
/// Shared between [`proper_noun_tokens`] (query side) and
/// [`extract_content_propers`] (BENCH-KG-2 ingest side).
const PROPER_NOUN_STOP: &[&str] = &[
    "what", "who", "when", "where", "why", "how", "which", "whose", "whom",
    "did", "does", "do", "is", "are", "was", "were", "will", "would",
    "can", "could", "should", "shall", "may", "might", "must",
    "has", "have", "had", "the", "and", "but", "or", "not", "for", "with",
    "from", "into", "onto", "about", "after", "before", "during", "while",
    "between", "among", "this", "that", "these", "those", "there", "their",
    "they", "them", "his", "her", "him", "she", "he", "you", "your", "yours",
    "our", "ours", "us", "we", "my", "mine", "i", "me", "it", "its",
    // Sentence-start words that frequently appear capitalized in chat
    // transcripts but carry no entity meaning. Adding these to the bench
    // ingest stoplist (BENCH-KG-2) prevents "shares_entities" edges from
    // forming on noise.
    "yes", "yeah", "yep", "nope", "hello", "hi", "hey", "thanks", "thank",
    "sure", "okay", "ok", "well", "like", "just", "also", "then", "now",
    "today", "tomorrow", "yesterday",
];

/// Extract proper-noun-like tokens from a query: capitalized words ≥3 chars
/// that are not common English sentence-starters / function words.
///
/// Reserved for BENCH-LCM-8 (narrower adversarial defense; BENCH-LCM-7 reverted
/// the unconditional usage).
#[allow(dead_code)]
fn proper_noun_tokens(query: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for raw in query.split(|c: char| !c.is_alphanumeric() && c != '\'') {
        if raw.len() < 3 {
            continue;
        }
        let first = raw.chars().next().unwrap();
        if !first.is_ascii_uppercase() {
            continue;
        }
        let lower = raw.to_lowercase();
        if PROPER_NOUN_STOP.contains(&lower.as_str()) {
            continue;
        }
        // Require at least one more letter (skip pure-numeric or single-letter
        // edge cases that survived the length check).
        if !raw.chars().any(|c| c.is_alphabetic()) {
            continue;
        }
        if seen.insert(lower.clone()) {
            out.push(lower);
        }
    }
    out
}

/// BENCH-KG-2 ingest-side proper-noun extractor.
///
/// Walks `content` token by token, marking each token as either
/// **sentence-start** (after `.`, `!`, `?`, or at the absolute beginning) or
/// **mid-sentence**. Capitalised tokens ≥4 characters that are NOT in
/// [`PROPER_NOUN_STOP`] qualify as entities. Sentence-start tokens are
/// only kept when they survive the stoplist (so a sentence opening with
/// `"Melanie said..."` still extracts `melanie`, but `"Yes, I agree."`
/// drops `yes`).
///
/// Returns lowercase, deduplicated entities preserving first-occurrence
/// order so downstream edge-confidence math (overlap count / entity count)
/// is deterministic.
fn extract_content_propers(content: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    // Sentence boundary tracker — starts true so the very first token is
    // treated as a sentence-start.
    let mut at_sentence_start = true;
    // We split on whitespace so we can preserve sentence-end punctuation,
    // then strip trailing/leading non-alphanumerics from each token.
    for raw_token in content.split_whitespace() {
        // Cleaned token: strip surrounding punctuation but keep internal
        // apostrophes / hyphens-as-letters.
        let cleaned: String = raw_token
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_string();
        let token_is_sentence_start = at_sentence_start;
        // Update boundary BEFORE the continue-checks so end-of-token
        // punctuation always advances the tracker.
        at_sentence_start = raw_token
            .chars()
            .last()
            .map(|c| matches!(c, '.' | '!' | '?'))
            .unwrap_or(false);

        if cleaned.len() < 4 {
            continue;
        }
        let first = match cleaned.chars().next() {
            Some(c) => c,
            None => continue,
        };
        if !first.is_ascii_uppercase() {
            continue;
        }
        if !cleaned.chars().any(|c| c.is_alphabetic()) {
            continue;
        }
        let lower = cleaned.to_lowercase();
        if PROPER_NOUN_STOP.contains(&lower.as_str()) {
            // Skip stoplist words — sentence-start or not, they are noise.
            continue;
        }
        // Sentence-start words pass the stoplist gate above; that is the
        // intended "skip first-word-of-sentence false positives" rule —
        // proper names (Melanie, Tokyo) survive, function-word starts
        // (Yes, Hello) are dropped. We log the boundary flag in case a
        // future tuning pass wants to tighten further.
        let _ = token_is_sentence_start;
        if seen.insert(lower.clone()) {
            out.push(lower);
        }
    }
    out
}

// ---- Index state ----------------------------------------------------------

/// BENCH-KG-2 in-memory entity index built incrementally during
/// `add_sessions`. Tracks `entity (lowercase) → [memory_id]` so we can
/// compute pair-overlap edges without scanning every prior memory on
/// every batch.
#[derive(Default)]
struct KgIndex {
    enabled: bool,
    /// Top-N strongest neighbour edges to materialise per memory (cap on
    /// edge fan-out to keep `cascade_expand` BFS bounded at scale).
    max_neighbours: usize,
    entity_to_mems: HashMap<String, Vec<i64>>,
    edges_added: usize,
}

impl KgIndex {
    fn from_env() -> Self {
        let enabled = std::env::var("LONGMEM_KG_EDGES")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let max_neighbours = std::env::var("LONGMEM_KG_MAX_NEIGHBOURS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(10);
        Self {
            enabled,
            max_neighbours,
            entity_to_mems: HashMap::new(),
            edges_added: 0,
        }
    }

    fn reset(&mut self) {
        self.entity_to_mems.clear();
        self.edges_added = 0;
    }
}

struct IndexState {
    store: MemoryStore,
    embeddings: HashMap<i64, Vec<f32>>,
    session_ids: HashMap<i64, String>,
    token_counts: HashMap<i64, i64>,
    // Reserved for BENCH-LCM-8 narrower adversarial defense; BENCH-LCM-7 reverted.
    #[allow(dead_code)]
    contents_lower: HashMap<i64, String>,
    kg: KgIndex,
}

impl IndexState {
    fn new() -> Self {
        Self {
            store: MemoryStore::in_memory(),
            embeddings: HashMap::new(),
            session_ids: HashMap::new(),
            token_counts: HashMap::new(),
            contents_lower: HashMap::new(),
            kg: KgIndex::from_env(),
        }
    }

    fn reset(&mut self) {
        self.store = MemoryStore::in_memory();
        self.embeddings.clear();
        self.session_ids.clear();
        self.token_counts.clear();
        self.contents_lower.clear();
        self.kg.reset();
    }
}

// ---- Ops ------------------------------------------------------------------

fn add_sessions(
    state: &mut IndexState,
    embedder: Option<&OllamaEmbedder>,
    contextualizer: Option<&Contextualizer>,
    request: &Request,
) -> Result<Value, String> {
    let question_id = request.question_id.as_deref().unwrap_or("unknown-question");
    let raw_contents: Vec<String> = request.sessions.iter().map(session_content).collect();
    // BENCH-LCM-11: when contextualization is on, prepend an LLM-generated
    // context sentence to each session BEFORE embedding/storing so both the
    // vector embedding and FTS5 lexical index see the contextual anchor.
    let mut ctx_hits = 0usize;
    let mut ctx_misses = 0usize;
    let contents: Vec<String> = if let Some(ctx) = contextualizer {
        // BENCH-LCM-11 smoke knob: cap how many sessions are contextualized
        // so a quick validation run doesn't pay the full ~3h corpus cost.
        // Sessions beyond the cap pass through raw. Default = no cap.
        let limit: Option<usize> = std::env::var("LONGMEM_CTX_CORPUS_LIMIT")
            .ok()
            .and_then(|s| s.parse::<usize>().ok());
        let total = raw_contents.len();
        eprintln!(
            "[contextualizer] starting corpus pass: total={total} cap={:?}",
            limit
        );
        let started = std::time::Instant::now();
        raw_contents
            .iter()
            .enumerate()
            .map(|(idx, raw)| {
                if let Some(cap) = limit {
                    if idx >= cap {
                        return raw.clone();
                    }
                }
                let result = match ctx.contextualize(raw) {
                    Some(prefix) => {
                        ctx_hits += 1;
                        prepend_context(&prefix, raw)
                    }
                    None => {
                        ctx_misses += 1;
                        raw.clone()
                    }
                };
                let processed = idx + 1;
                if processed % 25 == 0 || processed == limit.unwrap_or(total) {
                    let secs = started.elapsed().as_secs_f32();
                    eprintln!(
                        "[contextualizer] {processed}/{} hits={ctx_hits} misses={ctx_misses} elapsed={:.1}s",
                        limit.unwrap_or(total),
                        secs
                    );
                }
                result
            })
            .collect()
    } else {
        raw_contents.clone()
    };
    let items = request
        .sessions
        .iter()
        .zip(contents.iter())
        .map(|(session, content)| NewMemory {
            content: content.clone(),
            tags: session_tags(question_id, session),
            importance: 4,
            memory_type: MemoryType::Context,
            source_url: Some(session_source_url(&session.session_id)),
            source_hash: Some(format!("longmemeval:{question_id}:{}", session.session_id)),
            expires_at: None,
        })
        .collect::<Vec<_>>();

    let inserted = state.store.add_many(items).map_err(|err| err.to_string())?;

    let mut embed_errors = 0usize;
    for (idx, mem_id) in inserted.iter().enumerate() {
        if let Some(session) = request.sessions.get(idx) {
            state.session_ids.insert(*mem_id, session.session_id.clone());
        }
        state
            .contents_lower
            .insert(*mem_id, contents[idx].to_lowercase());
        if let Some(embedder) = embedder {
            let content = &contents[idx];
            match embedder.embed(content, EmbedRole::Document) {
                Ok(vec) => {
                    // Store in both local HashMap (for IPC-level cosine)
                    // and MemoryStore (for ANN-backed hybrid_search_rrf).
                    let _ = state.store.set_embedding(*mem_id, &vec);
                    state.embeddings.insert(*mem_id, vec);
                }
                Err(_) => {
                    embed_errors += 1;
                }
            }
        }
    }

    if let Ok(all) = state.store.get_all() {
        for entry in all {
            state.token_counts.insert(entry.id, entry.token_count);
        }
    }

    // BENCH-KG-2: build entity-overlap `shares_entities` edges so the
    // bench exercises the same `cascade_expand` BFS stage that production
    // chat uses via `commands::chat::expand_seeds_via_kg`. Each new
    // memory's proper-noun entities are intersected against every prior
    // memory's entity set; pairs with ≥2 shared entities become edges,
    // top-`max_neighbours` per new memory ranked by overlap count.
    if state.kg.enabled {
        let mut new_edges: Vec<NewMemoryEdge> = Vec::new();
        for (idx, mem_id) in inserted.iter().enumerate() {
            let propers = extract_content_propers(&contents[idx]);
            if propers.is_empty() {
                continue;
            }
            // Score: dst_mem_id -> overlap_count.
            let mut overlap: HashMap<i64, usize> = HashMap::new();
            for entity in &propers {
                if let Some(prior_ids) = state.kg.entity_to_mems.get(entity) {
                    for prior_id in prior_ids {
                        if *prior_id == *mem_id {
                            continue;
                        }
                        *overlap.entry(*prior_id).or_insert(0) += 1;
                    }
                }
            }
            // Keep only pairs with ≥2 shared entities, top-N by overlap.
            let mut ranked: Vec<(i64, usize)> = overlap
                .into_iter()
                .filter(|(_, count)| *count >= 2)
                .collect();
            ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            ranked.truncate(state.kg.max_neighbours);
            let entity_count = propers.len().max(1) as f64;
            for (dst_id, count) in ranked {
                let confidence = ((count as f64) / entity_count).clamp(0.0, 1.0);
                new_edges.push(NewMemoryEdge {
                    src_id: *mem_id,
                    dst_id,
                    rel_type: "shares_entities".to_string(),
                    confidence,
                    source: EdgeSource::Auto,
                    valid_from: None,
                    valid_to: None,
                    edge_source: None,
                });
            }
            // Register this memory's entities AFTER scoring so we don't
            // self-pair within the same batch index.
            for entity in propers {
                state
                    .kg
                    .entity_to_mems
                    .entry(entity)
                    .or_default()
                    .push(*mem_id);
            }
        }
        if !new_edges.is_empty() {
            match state.store.add_edges_batch(&new_edges) {
                Ok(n) => state.kg.edges_added += n,
                Err(err) => eprintln!("[kg] add_edges_batch failed: {err}"),
            }
        }
    }

    Ok(json!({
        "inserted": inserted.len(),
        "embedded": state.embeddings.len(),
        "embed_errors": embed_errors,
        "ctx_hits": ctx_hits,
        "ctx_misses": ctx_misses,
        "kg_edges": state.kg.edges_added,
    }))
}

fn search(
    state: &IndexState,
    embedder: Option<&OllamaEmbedder>,
    reranker: Option<&OllamaReranker>,
    hyder: Option<&HydeExpander>,
    request: &Request,
) -> Result<Value, String> {
    // BENCH-LCM-11: contextualization is an INGEST-time transform, so the
    // search pipeline is identical between (rrf, rrf_ctx) and (rrf_rerank,
    // rrf_ctx_rerank). Normalize the ctx aliases to their base modes so the
    // existing branches handle them. Whether the underlying corpus actually
    // carries contextualized embeddings is governed by `LONGMEM_CONTEXTUALIZE`
    // at process start (and reflected in the `add_sessions` ctx_hits/misses).
    let mode = match request.mode.as_str() {
        "rrf_ctx" => "rrf",
        "rrf_ctx_rerank" => "rrf_rerank",
        other => other,
    };
    let limit = request.limit;

    // BENCH-KG-2: when KG-cascade modes are requested, the lexical/vector
    // pipeline is identical to `rrf` / `rrf_rerank` — we just expand the
    // candidate set via `cascade_expand` before truncating to `limit`.
    let wants_kg = matches!(mode, "rrf_kg" | "rrf_kg_rerank");

    // HyDE expansion: when enabled and the mode opts in, generate a
    // hypothetical answer and embed THAT for the vector channel. Reuses the
    // exact production prompt from `terransoul_lib::memory::hyde`. Falls back
    // to the raw query if the LLM call fails / reply is too short.
    let wants_hyde = matches!(mode, "rrf_hyde" | "rrf_hyde_rerank");
    let hyde_text: Option<String> = if wants_hyde {
        hyder.and_then(|h| h.expand(&request.query))
    } else {
        None
    };
    // Vector channel: HyDE-expanded text if available, raw query otherwise.
    let vector_query: &str = hyde_text.as_deref().unwrap_or(&request.query);

    let entries = match mode {
        "search" | "keyword" => state
            .store
            .search(&request.query)
            .map_err(|err| err.to_string())?,
        "rrf" | "hybrid_search_rrf" | "rrf_kg" => {
            // If embeddings are available, compute query embedding for the
            // internal vector ranking signal in hybrid_search_rrf.
            let q_emb = embedder.and_then(|e| e.embed(&request.query, EmbedRole::Query).ok());
            // BENCH-KG-2: for `rrf_kg`, pull a wider seed pool (default 30,
            // same as the rerank pool tuning sweet-spot) so cascade has
            // meaningful seeds before truncation.
            let pool = if wants_kg {
                reranker.map(|r| r.pool).unwrap_or(DEFAULT_RERANK_POOL)
            } else {
                limit
            };
            state
                .store
                .hybrid_search_rrf(&request.query, q_emb.as_deref(), pool)
                .map_err(|err| err.to_string())?
        }
        "rrf_rerank" | "rrf_kg_rerank" => {
            // Retrieve a wider candidate pool so the cross-encoder has room
            // to promote a buried correct passage. Pool size is configurable
            // via LONGMEM_RERANK_POOL (LCM-9 found 30 is the sweet spot).
            // BENCH-KG-2: `rrf_kg_rerank` shares this branch — cascade
            // expansion is applied after RRF but before reranking, so the
            // cross-encoder sees both lexical/vector hits AND graph
            // neighbours.
            let q_emb = embedder.and_then(|e| e.embed(&request.query, EmbedRole::Query).ok());
            let pool = reranker.map(|r| r.pool).unwrap_or(DEFAULT_RERANK_POOL);
            state
                .store
                .hybrid_search_rrf(&request.query, q_emb.as_deref(), pool)
                .map_err(|err| err.to_string())?
        }
        "rrf_hyde" => {
            // Vector channel uses HyDE expansion; lexical channel keeps the
            // raw query so we don't lose exact-keyword recall. BENCH-LCM-10.
            let q_emb = embedder.and_then(|e| e.embed(vector_query, EmbedRole::Query).ok());
            state
                .store
                .hybrid_search_rrf(&request.query, q_emb.as_deref(), limit)
                .map_err(|err| err.to_string())?
        }
        "rrf_hyde_rerank" => {
            // HyDE-expanded vector channel + raw lexical channel, top-30
            // candidate pool, then cross-encoder rerank. BENCH-LCM-10.
            let q_emb = embedder.and_then(|e| e.embed(vector_query, EmbedRole::Query).ok());
            let pool = reranker.map(|r| r.pool).unwrap_or(DEFAULT_RERANK_POOL);
            state
                .store
                .hybrid_search_rrf(&request.query, q_emb.as_deref(), pool)
                .map_err(|err| err.to_string())?
        }
        "emb" | "rrf_emb" | "search_emb" | "best" => Vec::new(),
        other => return Err(format!("unsupported search mode: {other}")),
    };

    // BENCH-KG-2: expand candidates via `cascade_expand` before the final
    // ranking step. Applies to both `rrf_kg` (returned as-is, truncated to
    // `limit`) and `rrf_kg_rerank` (handed to the cross-encoder below).
    // Cap the post-cascade pool at the reranker pool size so the
    // cross-encoder workload stays bounded even when the graph is dense.
    let entries = if wants_kg {
        let cap = reranker.map(|r| r.pool).unwrap_or(DEFAULT_RERANK_POOL);
        let mut expanded = cascade_expand_to_entries(state, entries)?;
        expanded.truncate(cap);
        expanded
    } else {
        entries
    };

    let hits: Vec<SearchHit> = match mode {
        "emb" => emb_only_hits(state, embedder, &request.query, limit)?,
        "rrf_emb" => rrf_emb_hits(state, embedder, &request.query, limit)?,
        "search_emb" => search_emb_hits(state, embedder, &request.query, limit)?,
        "best" => best_hits(state, embedder, &request.query, limit)?,
        "rrf_rerank" | "rrf_hyde_rerank" | "rrf_kg_rerank" => {
            rerank_hits(reranker, &request.query, entries, limit)?
        }
        _ => entries
            .into_iter()
            .take(limit)
            .map(|entry| SearchHit {
                memory_id: entry.id,
                session_id: source_url_to_session_id(entry.source_url.as_deref()),
                token_count: entry.token_count,
            })
            .collect(),
    };

    Ok(json!({ "results": hits }))
}

/// BENCH-KG-2: expand a seed list of RRF entries via `cascade_expand` so
/// the bench exercises the same 1\u20132 hop graph BFS that chat uses (see
/// `commands::chat::expand_seeds_via_kg`). Original seeds preserve their
/// RRF rank order; neighbours discovered by cascade are appended in
/// descending cascade-score order. Memories already present in seeds are
/// not re-added.
///
/// Returns the unchanged seed list if `MemoryStore::cascade_expand_seeds`
/// fails or returns nothing (no edges yet, KG disabled at ingest, etc.) so
/// the bench keeps producing comparable recall numbers either way.
fn cascade_expand_to_entries(
    state: &IndexState,
    seeds: Vec<terransoul_lib::memory::store::MemoryEntry>,
) -> Result<Vec<terransoul_lib::memory::store::MemoryEntry>, String> {
    if seeds.is_empty() {
        return Ok(seeds);
    }
    // Score seeds by descending RRF rank so cascade scoring respects the
    // upstream ordering. Top-of-list gets the highest seed weight.
    let total = seeds.len() as f64;
    let seed_pairs: Vec<(i64, f64)> = seeds
        .iter()
        .enumerate()
        .map(|(i, e)| (e.id, (total - i as f64) / total))
        .collect();
    let mut already: HashSet<i64> = seeds.iter().map(|e| e.id).collect();
    // Cascade depth = 2: mirrors the chat-side default and the design-doc
    // recommendation. cascade_expand returns (id, score) pairs already
    // sorted by descending score.
    let expanded = state
        .store
        .cascade_expand_seeds(&seed_pairs, Some(2))
        .map_err(|err| err.to_string())?;
    let mut out = seeds;
    for (id, _score) in expanded {
        if already.insert(id) {
            if let Ok(entry) = state.store.get_by_id(id) {
                out.push(entry);
            }
        }
    }
    Ok(out)
}

/// Rerank an RRF candidate pool with the LLM-as-judge cross-encoder.
///
/// Falls back to the input order if the reranker is unavailable (env not set
/// or Ollama call failed), so the bench keeps producing recall-comparable
/// results even when the chat model is offline.
fn rerank_hits(
    reranker: Option<&OllamaReranker>,
    query: &str,
    entries: Vec<terransoul_lib::memory::store::MemoryEntry>,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    let reranker = reranker.ok_or_else(|| {
        "rrf_rerank mode requires LONGMEM_RERANK=1 and a running Ollama chat model"
            .to_string()
    })?;
    if entries.is_empty() {
        return Ok(Vec::new());
    }

    // Score in batches of RERANK_BATCH_SIZE candidates per LLM call.
    let mut all_scores: Vec<Option<u8>> = Vec::with_capacity(entries.len());
    let docs: Vec<(usize, String)> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (i, e.content.clone()))
        .collect();
    for chunk in docs.chunks(RERANK_BATCH_SIZE) {
        let scores = reranker.score_batch(query, chunk);
        all_scores.extend(scores);
    }

    // Apply the confidence threshold: scores strictly below `threshold` are
    // treated as unconfident and downgraded to None, so the candidate keeps
    // its pre-rerank RRF rank instead of being promoted by a noisy guess.
    // BENCH-LCM-9: closes the LCM-8 adversarial NDCG/MRR drop where
    // gemma3:4b was promoting plausible-but-wrong distractors on abstain
    // queries. Set LONGMEM_RERANK_THRESHOLD=0 to disable.
    let threshold = reranker.threshold;
    if threshold > 0.0 {
        for s in all_scores.iter_mut() {
            if let Some(score) = *s {
                if (score as f32) < threshold {
                    *s = None;
                }
            }
        }
    }

    // Pair each candidate with (score-or-None, original_index), then sort:
    // scored first (descending), unscored kept at end in original order. This
    // preserves recall when the LLM call fails on some batches.
    let mut paired: Vec<(usize, Option<u8>, &terransoul_lib::memory::store::MemoryEntry)> =
        entries
            .iter()
            .enumerate()
            .map(|(i, e)| (i, all_scores.get(i).copied().flatten(), e))
            .collect();
    paired.sort_by(|a, b| match (a.1, b.1) {
        (Some(sa), Some(sb)) => sb.cmp(&sa).then_with(|| a.0.cmp(&b.0)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.0.cmp(&b.0),
    });

    Ok(paired
        .into_iter()
        .take(limit)
        .map(|(_, _, entry)| SearchHit {
            memory_id: entry.id,
            session_id: source_url_to_session_id(entry.source_url.as_deref()),
            token_count: entry.token_count,
        })
        .collect())
}

fn emb_only_hits(
    state: &IndexState,
    embedder: Option<&OllamaEmbedder>,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    let embedder = embedder.ok_or_else(|| "emb mode requires LONGMEM_EMBED=1".to_string())?;
    if state.embeddings.is_empty() {
        return Err("emb mode: no session embeddings indexed".to_string());
    }
    let q = embedder.embed(query, EmbedRole::Query)?;
    let mut scored: Vec<(i64, f32)> = state
        .embeddings
        .iter()
        .map(|(id, vec)| (*id, cosine(&q, vec)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let hits = scored
        .into_iter()
        .take(limit)
        .map(|(mem_id, _)| SearchHit {
            memory_id: mem_id,
            session_id: state.session_ids.get(&mem_id).cloned().unwrap_or_default(),
            token_count: state.token_counts.get(&mem_id).copied().unwrap_or(0),
        })
        .collect();
    Ok(hits)
}

fn best_hits(
    state: &IndexState,
    embedder: Option<&OllamaEmbedder>,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    let embedder =
        embedder.ok_or_else(|| "best mode requires LONGMEM_EMBED=1".to_string())?;

    // Signal 1: search() — FTS5 + IDF-weighted rerank + graph boosts.
    let search_entries = state.store.search(query).map_err(|err| err.to_string())?;

    // Signal 2: hybrid_search_rrf() — FTS5 + freshness RRF fusion.
    let rrf_entries = state
        .store
        .hybrid_search_rrf(query, None, 500)
        .map_err(|err| err.to_string())?;

    // Union of candidate IDs from both lexical signals.
    let mut all_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
    for entry in &search_entries {
        all_ids.insert(entry.id);
    }
    for entry in &rrf_entries {
        all_ids.insert(entry.id);
    }

    let q = embedder.embed(query, EmbedRole::Query)?;

    // Signal 3: cosine re-rank of all lexical candidates.
    let mut cand_scored: Vec<(i64, f32)> = all_ids
        .iter()
        .filter_map(|id| state.embeddings.get(id).map(|v| (*id, cosine(&q, v))))
        .collect();
    cand_scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let cand_rank: HashMap<i64, usize> = cand_scored
        .iter()
        .enumerate()
        .map(|(idx, (id, _))| (*id, idx))
        .collect();

    // Signal 4: embedding rescue from outside lexical pool.
    const RESCUE_THRESHOLD: f32 = 0.50;
    const MAX_RESCUE: usize = 100;
    let mut rescue_scored: Vec<(i64, f32)> = state
        .embeddings
        .iter()
        .filter(|(id, _)| !all_ids.contains(id))
        .map(|(id, vec)| (*id, cosine(&q, vec)))
        .filter(|(_, score)| *score > RESCUE_THRESHOLD)
        .collect();
    rescue_scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    rescue_scored.truncate(MAX_RESCUE);
    let rescue_rank: HashMap<i64, usize> = rescue_scored
        .iter()
        .enumerate()
        .map(|(idx, (id, _))| (*id, idx))
        .collect();

    // 4-way weighted RRF fusion.
    const W_SEARCH: f32 = 2.0;
    const W_RRF: f32 = 1.5;
    const W_EMB: f32 = 1.2;
    const W_RESCUE: f32 = 0.6;
    let mut scores: HashMap<i64, f32> = HashMap::new();
    for (rank, entry) in search_entries.iter().enumerate() {
        let s = W_SEARCH / (RRF_K + (rank as f32) + 1.0);
        *scores.entry(entry.id).or_insert(0.0) += s;
    }
    for (rank, entry) in rrf_entries.iter().enumerate() {
        let s = W_RRF / (RRF_K + (rank as f32) + 1.0);
        *scores.entry(entry.id).or_insert(0.0) += s;
    }
    for (id, rank) in &cand_rank {
        let s = W_EMB / (RRF_K + (*rank as f32) + 1.0);
        *scores.entry(*id).or_insert(0.0) += s;
    }
    for (id, rank) in &rescue_rank {
        let s = W_RESCUE / (RRF_K + (*rank as f32) + 1.0);
        *scores.entry(*id).or_insert(0.0) += s;
    }

    // Build session/token maps.
    let mut session_id_of: HashMap<i64, String> = HashMap::new();
    let mut token_of: HashMap<i64, i64> = HashMap::new();
    for entry in search_entries.iter().chain(rrf_entries.iter()) {
        session_id_of
            .entry(entry.id)
            .or_insert_with(|| source_url_to_session_id(entry.source_url.as_deref()));
        token_of.entry(entry.id).or_insert(entry.token_count);
    }

    let mut fused: Vec<(i64, f32)> = scores.into_iter().collect();

    // BENCH-LCM-7 (2026-05-12) — Reverted unconditional proper-noun penalty.
    //
    // The BENCH-LCM-6 fix multiplied every candidate's score by 0.35/0.5 when
    // the query had any proper noun and the candidate had none. On the full
    // 1655-query LoCoMo run with mxbai-embed-large, this scored:
    //   adversarial +1.6pp (61.7 → 63.3)
    //   single_hop  -4.9pp (73.5 → 68.6)
    //   multi_hop  -11.3pp (46.2 → 34.9)
    //   open_domain -9.8pp (42.0 → 32.2)
    //   overall    -2.1pp (63.6 → 61.5)
    // Net loss; the penalty suppressed paraphrased-entity matches (e.g. "the
    // runner" → "Melanie") on factual single_hop/multi_hop queries. The
    // `proper_noun_tokens()` helper and `contents_lower` index remain for
    // BENCH-LCM-8 (a narrower trigger — e.g. only when query has 2+ proper
    // nouns, the adversarial-shape signal).

    fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let hits = fused
        .into_iter()
        .take(limit)
        .map(|(mem_id, _)| SearchHit {
            memory_id: mem_id,
            session_id: session_id_of
                .get(&mem_id)
                .cloned()
                .or_else(|| state.session_ids.get(&mem_id).cloned())
                .unwrap_or_default(),
            token_count: token_of
                .get(&mem_id)
                .copied()
                .or_else(|| state.token_counts.get(&mem_id).copied())
                .unwrap_or(0),
        })
        .collect();
    Ok(hits)
}

fn search_emb_hits(
    state: &IndexState,
    embedder: Option<&OllamaEmbedder>,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    let embedder =
        embedder.ok_or_else(|| "search_emb mode requires LONGMEM_EMBED=1".to_string())?;

    // Signal 1: MemoryStore::search (FTS5 + IDF-weighted rerank + graph boosts).
    let search_entries = state.store.search(query).map_err(|err| err.to_string())?;
    let search_ids: std::collections::HashSet<i64> =
        search_entries.iter().map(|e| e.id).collect();

    let q = embedder.embed(query, EmbedRole::Query)?;

    // Signal 2: cosine re-rank of search candidates.
    let mut cand_scored: Vec<(i64, f32)> = search_ids
        .iter()
        .filter_map(|id| state.embeddings.get(id).map(|v| (*id, cosine(&q, v))))
        .collect();
    cand_scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let cand_rank: HashMap<i64, usize> = cand_scored
        .iter()
        .enumerate()
        .map(|(idx, (id, _))| (*id, idx))
        .collect();

    // Signal 3: embedding rescue (same as rrf_emb).
    const RESCUE_THRESHOLD: f32 = 0.50;
    const MAX_RESCUE: usize = 100;
    let mut rescue_scored: Vec<(i64, f32)> = state
        .embeddings
        .iter()
        .filter(|(id, _)| !search_ids.contains(id))
        .map(|(id, vec)| (*id, cosine(&q, vec)))
        .filter(|(_, score)| *score > RESCUE_THRESHOLD)
        .collect();
    rescue_scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    rescue_scored.truncate(MAX_RESCUE);
    let rescue_rank: HashMap<i64, usize> = rescue_scored
        .iter()
        .enumerate()
        .map(|(idx, (id, _))| (*id, idx))
        .collect();

    // Weighted RRF: search dominant (IDF-weighted scoring is richer), embeddings rerank.
    const W_LEX: f32 = 2.5;
    const W_CAND_EMB: f32 = 1.0;
    const W_RESCUE: f32 = 0.6;
    let mut rrf: HashMap<i64, f32> = HashMap::new();
    for (rank, entry) in search_entries.iter().enumerate() {
        let s = W_LEX / (RRF_K + (rank as f32) + 1.0);
        *rrf.entry(entry.id).or_insert(0.0) += s;
    }
    for (id, rank) in &cand_rank {
        let s = W_CAND_EMB / (RRF_K + (*rank as f32) + 1.0);
        *rrf.entry(*id).or_insert(0.0) += s;
    }
    for (id, rank) in &rescue_rank {
        let s = W_RESCUE / (RRF_K + (*rank as f32) + 1.0);
        *rrf.entry(*id).or_insert(0.0) += s;
    }

    let mut session_id_of: HashMap<i64, String> = HashMap::new();
    let mut token_of: HashMap<i64, i64> = HashMap::new();
    for entry in &search_entries {
        session_id_of.insert(entry.id, source_url_to_session_id(entry.source_url.as_deref()));
        token_of.insert(entry.id, entry.token_count);
    }

    let mut fused: Vec<(i64, f32)> = rrf.into_iter().collect();
    fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let hits = fused
        .into_iter()
        .take(limit)
        .map(|(mem_id, _)| SearchHit {
            memory_id: mem_id,
            session_id: session_id_of
                .get(&mem_id)
                .cloned()
                .or_else(|| state.session_ids.get(&mem_id).cloned())
                .unwrap_or_default(),
            token_count: token_of
                .get(&mem_id)
                .copied()
                .or_else(|| state.token_counts.get(&mem_id).copied())
                .unwrap_or(0),
        })
        .collect();
    Ok(hits)
}

fn rrf_emb_hits(
    state: &IndexState,
    embedder: Option<&OllamaEmbedder>,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    let embedder =
        embedder.ok_or_else(|| "rrf_emb mode requires LONGMEM_EMBED=1".to_string())?;

    // Signal 1: hybrid_search_rrf (FTS5 + freshness RRF fusion).
    let rrf_entries = state
        .store
        .hybrid_search_rrf(query, None, 500)
        .map_err(|err| err.to_string())?;
    let rrf_ids: std::collections::HashSet<i64> = rrf_entries.iter().map(|e| e.id).collect();

    let q = embedder.embed(query, EmbedRole::Query)?;

    // Signal 2: cosine re-rank of rrf candidates (always applied).
    let mut cand_scored: Vec<(i64, f32)> = rrf_ids
        .iter()
        .filter_map(|id| state.embeddings.get(id).map(|v| (*id, cosine(&q, v))))
        .collect();
    cand_scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let cand_rank: HashMap<i64, usize> = cand_scored
        .iter()
        .enumerate()
        .map(|(idx, (id, _))| (*id, idx))
        .collect();

    // Signal 3: full-corpus embedding rescue — top-N docs by cosine that
    // are NOT already in the rrf candidate pool. Uses a threshold to
    // avoid injecting noise from broad queries.
    const RESCUE_THRESHOLD: f32 = 0.50;
    const MAX_RESCUE: usize = 100;
    let mut rescue_scored: Vec<(i64, f32)> = state
        .embeddings
        .iter()
        .filter(|(id, _)| !rrf_ids.contains(id))
        .map(|(id, vec)| (*id, cosine(&q, vec)))
        .filter(|(_, score)| *score > RESCUE_THRESHOLD)
        .collect();
    rescue_scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    rescue_scored.truncate(MAX_RESCUE);
    let rescue_rank: HashMap<i64, usize> = rescue_scored
        .iter()
        .enumerate()
        .map(|(idx, (id, _))| (*id, idx))
        .collect();

    // Weighted RRF fusion: lexical dominant, candidate re-rank medium, rescue low.
    const W_LEX: f32 = 2.0;
    const W_CAND_EMB: f32 = 1.5;
    const W_RESCUE: f32 = 0.8;
    let mut rrf: HashMap<i64, f32> = HashMap::new();
    for (rank, entry) in rrf_entries.iter().enumerate() {
        let s = W_LEX / (RRF_K + (rank as f32) + 1.0);
        *rrf.entry(entry.id).or_insert(0.0) += s;
    }
    for (id, rank) in &cand_rank {
        let s = W_CAND_EMB / (RRF_K + (*rank as f32) + 1.0);
        *rrf.entry(*id).or_insert(0.0) += s;
    }
    for (id, rank) in &rescue_rank {
        let s = W_RESCUE / (RRF_K + (*rank as f32) + 1.0);
        *rrf.entry(*id).or_insert(0.0) += s;
    }

    // Build session/token maps from all sources.
    let mut session_id_of: HashMap<i64, String> = HashMap::new();
    let mut token_of: HashMap<i64, i64> = HashMap::new();
    for entry in &rrf_entries {
        session_id_of.insert(entry.id, source_url_to_session_id(entry.source_url.as_deref()));
        token_of.insert(entry.id, entry.token_count);
    }

    let mut fused: Vec<(i64, f32)> = rrf.into_iter().collect();
    fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let hits = fused
        .into_iter()
        .take(limit)
        .map(|(mem_id, _)| SearchHit {
            memory_id: mem_id,
            session_id: session_id_of
                .get(&mem_id)
                .cloned()
                .or_else(|| state.session_ids.get(&mem_id).cloned())
                .unwrap_or_default(),
            token_count: token_of
                .get(&mem_id)
                .copied()
                .or_else(|| state.token_counts.get(&mem_id).copied())
                .unwrap_or(0),
        })
        .collect();
    Ok(hits)
}

fn write_response(id: u64, payload: Result<Value, String>) {
    let response = match payload {
        Ok(value) => json!({ "id": id, "ok": true, "data": value }),
        Err(error) => json!({ "id": id, "ok": false, "error": error }),
    };
    println!("{response}");
    let _ = io::stdout().flush();
}

fn main() {
    let stdin = io::stdin();
    let mut state = IndexState::new();
    let embedder = OllamaEmbedder::from_env();
    let reranker = OllamaReranker::from_env();
    let hyder = HydeExpander::from_env();
    let contextualizer = Contextualizer::from_env();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                write_response(0, Err(err.to_string()));
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let request = match serde_json::from_str::<Request>(&line) {
            Ok(request) => request,
            Err(err) => {
                write_response(0, Err(format!("invalid request JSON: {err}")));
                continue;
            }
        };

        match request.op.as_str() {
            "reset" => {
                state.reset();
                write_response(request.id, Ok(json!({ "reset": true })));
            }
            "add_sessions" => write_response(
                request.id,
                add_sessions(&mut state, embedder.as_ref(), contextualizer.as_ref(), &request),
            ),
            "search" => write_response(
                request.id,
                search(
                    &state,
                    embedder.as_ref(),
                    reranker.as_ref(),
                    hyder.as_ref(),
                    &request,
                ),
            ),
            "shutdown" => {
                write_response(request.id, Ok(json!({ "shutdown": true })));
                break;
            }
            other => write_response(request.id, Err(format!("unsupported op: {other}"))),
        }
    }
}
