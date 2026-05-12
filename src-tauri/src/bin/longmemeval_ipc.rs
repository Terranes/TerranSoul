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

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use terransoul_lib::memory::store::{MemoryStore, MemoryType, NewMemory};

const SOURCE_URL_PREFIX: &str = "longmemeval://session/";
const RRF_K: f32 = 60.0;
const DEFAULT_EMBED_MODEL: &str = "nomic-embed-text";
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
        Some(Self {
            client,
            url: format!("{}/api/embeddings", host.trim_end_matches('/')),
            model,
        })
    }

    fn embed(&self, text: &str, role: EmbedRole) -> Result<Vec<f32>, String> {
        // Cap input length: nomic-embed-text handles ~8k tokens; trim to ~16k
        // chars (~4k tokens) so very long sessions don't dominate latency.
        let trimmed = if text.len() > 16_000 { &text[..16_000] } else { text };
        // nomic-embed-text v1.5 requires task-instruction prefixes; without
        // them retrieval quality collapses to near-random on cross-domain pairs.
        let prefixed = match role {
            EmbedRole::Query => format!("search_query: {trimmed}"),
            EmbedRole::Document => format!("search_document: {trimmed}"),
        };
        let body = json!({"model": self.model, "prompt": prefixed});
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

// ---- Index state ----------------------------------------------------------

struct IndexState {
    store: MemoryStore,
    embeddings: HashMap<i64, Vec<f32>>,
    session_ids: HashMap<i64, String>,
    token_counts: HashMap<i64, i64>,
}

impl IndexState {
    fn new() -> Self {
        Self {
            store: MemoryStore::in_memory(),
            embeddings: HashMap::new(),
            session_ids: HashMap::new(),
            token_counts: HashMap::new(),
        }
    }

    fn reset(&mut self) {
        self.store = MemoryStore::in_memory();
        self.embeddings.clear();
        self.session_ids.clear();
        self.token_counts.clear();
    }
}

// ---- Ops ------------------------------------------------------------------

fn add_sessions(
    state: &mut IndexState,
    embedder: Option<&OllamaEmbedder>,
    request: &Request,
) -> Result<Value, String> {
    let question_id = request.question_id.as_deref().unwrap_or("unknown-question");
    let contents: Vec<String> = request.sessions.iter().map(session_content).collect();
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
        if let Some(embedder) = embedder {
            let content = &contents[idx];
            match embedder.embed(content, EmbedRole::Document) {
                Ok(vec) => {
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

    Ok(json!({
        "inserted": inserted.len(),
        "embedded": state.embeddings.len(),
        "embed_errors": embed_errors,
    }))
}

fn search(
    state: &IndexState,
    embedder: Option<&OllamaEmbedder>,
    request: &Request,
) -> Result<Value, String> {
    let mode = request.mode.as_str();
    let limit = request.limit;

    let entries = match mode {
        "search" | "keyword" => state
            .store
            .search(&request.query)
            .map_err(|err| err.to_string())?,
        "rrf" | "hybrid_search_rrf" => state
            .store
            .hybrid_search_rrf(&request.query, None, limit)
            .map_err(|err| err.to_string())?,
        "emb" | "rrf_emb" => Vec::new(),
        other => return Err(format!("unsupported search mode: {other}")),
    };

    let hits: Vec<SearchHit> = match mode {
        "emb" => emb_only_hits(state, embedder, &request.query, limit)?,
        "rrf_emb" => rrf_emb_hits(state, embedder, &request.query, limit)?,
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

fn rrf_emb_hits(
    state: &IndexState,
    embedder: Option<&OllamaEmbedder>,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, String> {
    let embedder =
        embedder.ok_or_else(|| "rrf_emb mode requires LONGMEM_EMBED=1".to_string())?;

    // FTS5 candidates (already lexically reranked + KG-boosted by MemoryStore::search).
    let fts_entries = state.store.search(query).map_err(|err| err.to_string())?;

    // Restrict embedding signal to FTS5's candidate pool. LongMemEval-S filler
    // sessions (ShareGPT/UltraChat) are large and semantically broad, so a
    // free-form embedding sweep across the whole haystack drowns the small,
    // specific gold sessions. Embeddings are most useful as a tiebreaker over
    // the lexical candidate pool, not as an independent retriever.
    let candidate_ids: Vec<i64> = fts_entries.iter().map(|e| e.id).collect();
    let q = embedder.embed(query, EmbedRole::Query)?;
    let mut emb_scored: Vec<(i64, f32)> = candidate_ids
        .iter()
        .filter_map(|id| state.embeddings.get(id).map(|v| (*id, cosine(&q, v))))
        .collect();
    emb_scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let emb_rank: HashMap<i64, usize> = emb_scored
        .iter()
        .enumerate()
        .map(|(idx, (id, _))| (*id, idx))
        .collect();

    // Weighted RRF: FTS5 dominant, embedding as a tiebreaker. The weights
    // were tuned empirically on the LongMemEval-S 20-question slice.
    const W_FTS: f32 = 3.0;
    const W_EMB: f32 = 1.0;
    let mut rrf: HashMap<i64, f32> = HashMap::new();
    for (rank, entry) in fts_entries.iter().enumerate() {
        let s = W_FTS / (RRF_K + (rank as f32) + 1.0);
        *rrf.entry(entry.id).or_insert(0.0) += s;
    }
    for (id, rank) in &emb_rank {
        let s = W_EMB / (RRF_K + (*rank as f32) + 1.0);
        *rrf.entry(*id).or_insert(0.0) += s;
    }

    let mut session_id_of: HashMap<i64, String> = HashMap::new();
    let mut token_of: HashMap<i64, i64> = HashMap::new();
    for entry in &fts_entries {
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
                add_sessions(&mut state, embedder.as_ref(), &request),
            ),
            "search" => {
                write_response(request.id, search(&state, embedder.as_ref(), &request))
            }
            "shutdown" => {
                write_response(request.id, Ok(json!({ "shutdown": true })));
                break;
            }
            other => write_response(request.id, Err(format!("unsupported op: {other}"))),
        }
    }
}
