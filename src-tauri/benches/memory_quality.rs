// SPDX-License-Identifier: MIT
//
// memory_quality — TerranSoul retrieval-quality bench. Uses the open-source
// concept-tagged corpus (240 observations / 20 queries, IR metrics) originally
// published by rohitg00/agentmemory as one reference dataset; numbers are
// compared against multiple top-tier memory systems in benchmark/COMPARISON.md.
//
// Reference dataset: https://github.com/rohitg00/agentmemory/blob/main/benchmark/quality-eval.ts
// Fixture builder: scripts/build-memory-quality-fixture.mjs
//
// Run:
//   cd src-tauri
//   cargo bench --bench memory_quality --target-dir ../target-copilot-bench
//
// Environment knobs:
//   TS_BENCH_AM_EMBED   = "none" | "deterministic" (default: deterministic)
//   TS_BENCH_AM_OUT_DIR = override report dir (default: ../target-copilot-bench/bench-results)

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use terransoul_lib::memory::edges::{EdgeSource, NewMemoryEdge};
use terransoul_lib::memory::store::{MemoryStore, MemoryType, NewMemory};

// ── Fixture types (mirror the JSON shape from build-memory-quality-fixture.mjs) ──

#[derive(Debug, Deserialize)]
struct Fixture {
    observations: Vec<Observation>,
    queries: Vec<LabeledQuery>,
    #[serde(default)]
    pinned_commit: String,
    #[serde(default)]
    source: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Observation {
    id: String,
    #[serde(rename = "sessionId")]
    session_id: String,
    timestamp: String,
    #[serde(rename = "type")]
    obs_type: String,
    title: String,
    #[serde(default)]
    subtitle: Option<String>,
    #[serde(default)]
    facts: Vec<String>,
    narrative: String,
    concepts: Vec<String>,
    #[serde(default)]
    files: Vec<String>,
    importance: i64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LabeledQuery {
    query: String,
    #[serde(rename = "relevantObsIds")]
    relevant_obs_ids: Vec<String>,
    description: String,
    category: String,
}

// ── Metrics & report structs ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct QueryMetrics {
    query: String,
    category: String,
    query_tokens: usize,
    retrieved_context_tokens: usize,
    full_context_tokens: usize,
    memory_200_line_tokens: usize,
    savings_vs_full_context_pct: f64,
    savings_vs_memory_200_pct: f64,
    recall_at_5: f64,
    recall_at_10: f64,
    recall_at_20: f64,
    precision_at_5: f64,
    precision_at_10: f64,
    ndcg_at_10: f64,
    mrr: f64,
    relevant_count: usize,
    retrieved_count: usize,
    latency_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
struct SystemMetrics {
    system: String,
    avg_query_tokens: f64,
    avg_retrieved_context_tokens: f64,
    avg_full_context_tokens: f64,
    avg_memory_200_line_tokens: f64,
    avg_savings_vs_full_context_pct: f64,
    avg_savings_vs_memory_200_pct: f64,
    avg_recall_at_5: f64,
    avg_recall_at_10: f64,
    avg_recall_at_20: f64,
    avg_precision_at_5: f64,
    avg_precision_at_10: f64,
    avg_ndcg_at_10: f64,
    avg_mrr: f64,
    avg_latency_ms: f64,
    per_query: Vec<QueryMetrics>,
}

#[derive(Debug, Serialize)]
struct Report {
    benchmark: &'static str,
    upstream_source: String,
    upstream_pinned_commit: String,
    observations: usize,
    queries: usize,
    embedding_mode: String,
    token_estimator: &'static str,
    systems: Vec<SystemMetrics>,
}

// ── IR metrics ──────────────────────────────────────────────────────────────

fn recall(retrieved: &[String], relevant: &HashSet<String>, k: usize) -> f64 {
    if relevant.is_empty() {
        return 1.0;
    }
    let top: HashSet<&String> = retrieved.iter().take(k).collect();
    let hits = relevant.iter().filter(|r| top.contains(r)).count();
    hits as f64 / relevant.len() as f64
}

fn precision(retrieved: &[String], relevant: &HashSet<String>, k: usize) -> f64 {
    let top: Vec<&String> = retrieved.iter().take(k).collect();
    if top.is_empty() {
        return 0.0;
    }
    let hits = top.iter().filter(|r| relevant.contains(**r)).count();
    hits as f64 / top.len() as f64
}

fn ndcg_at_k(retrieved: &[String], relevant: &HashSet<String>, k: usize) -> f64 {
    let actual_rels: Vec<bool> = retrieved
        .iter()
        .take(k)
        .map(|id| relevant.contains(id))
        .collect();
    let ideal_count = relevant.len().min(k);
    let dcg = |rels: &[bool]| -> f64 {
        rels.iter()
            .enumerate()
            .map(|(i, &r)| {
                if r {
                    1.0 / ((i + 2) as f64).log2()
                } else {
                    0.0
                }
            })
            .sum()
    };
    let ideal: Vec<bool> = (0..ideal_count).map(|_| true).collect();
    let idcg = dcg(&ideal);
    if idcg == 0.0 {
        0.0
    } else {
        dcg(&actual_rels) / idcg
    }
}

fn mrr(retrieved: &[String], relevant: &HashSet<String>) -> f64 {
    for (i, id) in retrieved.iter().enumerate() {
        if relevant.contains(id) {
            return 1.0 / (i + 1) as f64;
        }
    }
    0.0
}

fn avg(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

// ── Token efficiency accounting ────────────────────────────────────────────

const MEMORY_200_LINE_LIMIT: usize = 200;

#[derive(Debug, Clone)]
struct TokenBaselines {
    obs_context_tokens: HashMap<String, usize>,
    full_context_tokens: usize,
    memory_200_line_tokens: usize,
}

#[derive(Debug, Clone, Copy)]
struct QueryTokenMetrics {
    query_tokens: usize,
    retrieved_context_tokens: usize,
    full_context_tokens: usize,
    memory_200_line_tokens: usize,
    savings_vs_full_context_pct: f64,
    savings_vs_memory_200_pct: f64,
}

fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

fn memory_200_line(obs: &Observation) -> String {
    let narrative_prefix: String = obs.narrative.chars().take(80).collect();
    format!(
        "- {}: {}... [{}]",
        obs.title,
        narrative_prefix,
        obs.concepts
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn token_baselines(fixture: &Fixture) -> TokenBaselines {
    let obs_context_tokens: HashMap<String, usize> = fixture
        .observations
        .iter()
        .map(|obs| (obs.id.clone(), estimate_tokens(&obs_content(obs))))
        .collect();
    let full_context_tokens = obs_context_tokens.values().sum();
    let memory_200_line_tokens = fixture
        .observations
        .iter()
        .take(MEMORY_200_LINE_LIMIT)
        .map(memory_200_line)
        .map(|line| estimate_tokens(&line))
        .sum();

    TokenBaselines {
        obs_context_tokens,
        full_context_tokens,
        memory_200_line_tokens,
    }
}

fn savings_pct(retrieved_tokens: usize, baseline_tokens: usize) -> f64 {
    if baseline_tokens == 0 {
        0.0
    } else {
        1.0 - (retrieved_tokens as f64 / baseline_tokens as f64)
    }
}

fn query_token_metrics(
    query: &str,
    retrieved: &[String],
    baselines: &TokenBaselines,
) -> QueryTokenMetrics {
    let mut seen = HashSet::new();
    let retrieved_context_tokens = retrieved
        .iter()
        .filter(|id| seen.insert((*id).clone()))
        .filter_map(|id| baselines.obs_context_tokens.get(id))
        .sum();

    QueryTokenMetrics {
        query_tokens: estimate_tokens(query),
        retrieved_context_tokens,
        full_context_tokens: baselines.full_context_tokens,
        memory_200_line_tokens: baselines.memory_200_line_tokens,
        savings_vs_full_context_pct: savings_pct(
            retrieved_context_tokens,
            baselines.full_context_tokens,
        ),
        savings_vs_memory_200_pct: savings_pct(
            retrieved_context_tokens,
            baselines.memory_200_line_tokens,
        ),
    }
}

// ── Deterministic 384-d embedding (matches agentmemory exactly) ─────────────

fn deterministic_embedding(text: &str, dims: usize) -> Vec<f32> {
    let mut arr = vec![0.0f32; dims];
    let lower = text.to_lowercase();
    for word in lower.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if word.len() <= 2 {
            continue;
        }
        let bytes = word.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            let idx = ((b as usize) * 31 + i * 17) % dims;
            arr[idx] += 1.0;
            let idx2 = ((b as usize) * 37 + i * 13 + bytes.len() * 7) % dims;
            arr[idx2] += 0.5;
        }
    }
    let norm: f32 = arr.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut arr {
            *v /= norm;
        }
    }
    arr
}

// ── Build content + tags from an observation ────────────────────────────────

fn obs_content(obs: &Observation) -> String {
    let mut parts = Vec::with_capacity(8);
    parts.push(obs.title.clone());
    if let Some(sub) = &obs.subtitle {
        parts.push(sub.clone());
    }
    parts.push(obs.narrative.clone());
    if !obs.facts.is_empty() {
        parts.push(format!("Facts: {}", obs.facts.join("; ")));
    }
    if !obs.concepts.is_empty() {
        parts.push(format!("Concepts: {}", obs.concepts.join(", ")));
    }
    if !obs.files.is_empty() {
        parts.push(format!("Files: {}", obs.files.join(", ")));
    }
    parts.push(format!("Type: {}", obs.obs_type));
    parts.join("\n")
}

fn obs_tags(obs: &Observation) -> String {
    // Pipe-separated tags — TerranSoul's tag scheme accepts arbitrary strings.
    // Mirror agentmemory's concept-tag ground-truth model.
    let mut tags = obs.concepts.clone();
    tags.push(format!("session:{}", obs.session_id));
    tags.push(format!("kind:{}", obs.obs_type));
    tags.join("|")
}

fn embed_text_for_obs(obs: &Observation) -> String {
    // Identical shape to agentmemory's deterministic embed input:
    //   [title, narrative, ...concepts, ...facts].join(" ")
    let mut s = String::new();
    s.push_str(&obs.title);
    s.push(' ');
    s.push_str(&obs.narrative);
    s.push(' ');
    s.push_str(&obs.concepts.join(" "));
    s.push(' ');
    s.push_str(&obs.facts.join(" "));
    s
}

// ── Ingest fixture into a fresh store ───────────────────────────────────────

struct Ingested {
    store: MemoryStore,
    // ts_id (i64) → obs_id (e.g. "obs_ses_000_00")
    id_to_obs: HashMap<i64, String>,
}

fn ingest(fixture: &Fixture, with_embedding: bool) -> Ingested {
    let store = MemoryStore::in_memory();
    let items: Vec<NewMemory> = fixture
        .observations
        .iter()
        .map(|o| NewMemory {
            content: obs_content(o),
            tags: obs_tags(o),
            importance: o.importance.clamp(1, 5),
            memory_type: MemoryType::Fact,
            // Stash the upstream obs id in source_url so we can recover the
            // ground-truth label after retrieval.
            source_url: Some(format!("am-fixture://{}", o.id)),
            source_hash: None,
            expires_at: None,
        })
        .collect();

    let ids = store
        .add_many(items)
        .expect("add_many should succeed for the agentmemory fixture");
    assert_eq!(
        ids.len(),
        fixture.observations.len(),
        "add_many returned wrong id count",
    );

    let mut id_to_obs = HashMap::with_capacity(ids.len());
    let mut ids_by_concept: HashMap<String, Vec<i64>> = HashMap::new();
    for (ts_id, obs) in ids.iter().zip(fixture.observations.iter()) {
        id_to_obs.insert(*ts_id, obs.id.clone());
        for concept in &obs.concepts {
            ids_by_concept
                .entry(concept.to_lowercase())
                .or_default()
                .push(*ts_id);
        }
        if with_embedding {
            let vec = deterministic_embedding(&embed_text_for_obs(obs), 384);
            store
                .set_embedding(*ts_id, &vec)
                .expect("set_embedding should succeed");
        }
    }

    let mut seen_edges = HashSet::new();
    let mut edges = Vec::new();
    for ids_for_concept in ids_by_concept.values() {
        for (i, &src_id) in ids_for_concept.iter().enumerate() {
            for &dst_id in ids_for_concept.iter().skip(i + 1) {
                if seen_edges.insert((src_id, dst_id)) {
                    edges.push(NewMemoryEdge {
                        src_id,
                        dst_id,
                        rel_type: "shares_concept".to_string(),
                        confidence: 0.9,
                        source: EdgeSource::Auto,
                        valid_from: None,
                        valid_to: None,
                        edge_source: Some("memory-quality-bench:concept".to_string()),
                    });
                }
            }
        }
    }
    store
        .add_edges_batch(&edges)
        .expect("concept graph edges should be inserted");

    Ingested { store, id_to_obs }
}

// ── Per-system evaluators ───────────────────────────────────────────────────

type Retriever<'a> = Box<dyn Fn(&str, Option<&[f32]>) -> Vec<i64> + 'a>;
type ObsIdRetriever<'a> = Box<dyn Fn(&str) -> Vec<String> + 'a>;

fn evaluate(
    system_name: &str,
    fixture: &Fixture,
    ingested: &Ingested,
    token_baselines: &TokenBaselines,
    embed_query: bool,
    retriever: Retriever,
) -> SystemMetrics {
    let mut per_query = Vec::with_capacity(fixture.queries.len());
    for q in &fixture.queries {
        let relevant: HashSet<String> = q.relevant_obs_ids.iter().cloned().collect();
        let q_emb = if embed_query {
            Some(deterministic_embedding(&q.query, 384))
        } else {
            None
        };
        let start = Instant::now();
        let result_ids = retriever(&q.query, q_emb.as_deref());
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
        let retrieved: Vec<String> = result_ids
            .into_iter()
            .filter_map(|id| ingested.id_to_obs.get(&id).cloned())
            .collect();
        let tokens = query_token_metrics(&q.query, &retrieved, token_baselines);
        per_query.push(QueryMetrics {
            query: q.query.clone(),
            category: q.category.clone(),
            query_tokens: tokens.query_tokens,
            retrieved_context_tokens: tokens.retrieved_context_tokens,
            full_context_tokens: tokens.full_context_tokens,
            memory_200_line_tokens: tokens.memory_200_line_tokens,
            savings_vs_full_context_pct: tokens.savings_vs_full_context_pct,
            savings_vs_memory_200_pct: tokens.savings_vs_memory_200_pct,
            recall_at_5: recall(&retrieved, &relevant, 5),
            recall_at_10: recall(&retrieved, &relevant, 10),
            recall_at_20: recall(&retrieved, &relevant, 20),
            precision_at_5: precision(&retrieved, &relevant, 5),
            precision_at_10: precision(&retrieved, &relevant, 10),
            ndcg_at_10: ndcg_at_k(&retrieved, &relevant, 10),
            mrr: mrr(&retrieved, &relevant),
            relevant_count: relevant.len(),
            retrieved_count: retrieved.len(),
            latency_ms,
        });
    }
    SystemMetrics {
        system: system_name.to_string(),
        avg_query_tokens: avg(&per_query
            .iter()
            .map(|q| q.query_tokens as f64)
            .collect::<Vec<_>>()),
        avg_retrieved_context_tokens: avg(&per_query
            .iter()
            .map(|q| q.retrieved_context_tokens as f64)
            .collect::<Vec<_>>()),
        avg_full_context_tokens: avg(&per_query
            .iter()
            .map(|q| q.full_context_tokens as f64)
            .collect::<Vec<_>>()),
        avg_memory_200_line_tokens: avg(&per_query
            .iter()
            .map(|q| q.memory_200_line_tokens as f64)
            .collect::<Vec<_>>()),
        avg_savings_vs_full_context_pct: avg(&per_query
            .iter()
            .map(|q| q.savings_vs_full_context_pct)
            .collect::<Vec<_>>()),
        avg_savings_vs_memory_200_pct: avg(&per_query
            .iter()
            .map(|q| q.savings_vs_memory_200_pct)
            .collect::<Vec<_>>()),
        avg_recall_at_5: avg(&per_query.iter().map(|q| q.recall_at_5).collect::<Vec<_>>()),
        avg_recall_at_10: avg(&per_query.iter().map(|q| q.recall_at_10).collect::<Vec<_>>()),
        avg_recall_at_20: avg(&per_query.iter().map(|q| q.recall_at_20).collect::<Vec<_>>()),
        avg_precision_at_5: avg(&per_query
            .iter()
            .map(|q| q.precision_at_5)
            .collect::<Vec<_>>()),
        avg_precision_at_10: avg(&per_query
            .iter()
            .map(|q| q.precision_at_10)
            .collect::<Vec<_>>()),
        avg_ndcg_at_10: avg(&per_query.iter().map(|q| q.ndcg_at_10).collect::<Vec<_>>()),
        avg_mrr: avg(&per_query.iter().map(|q| q.mrr).collect::<Vec<_>>()),
        avg_latency_ms: avg(&per_query.iter().map(|q| q.latency_ms).collect::<Vec<_>>()),
        per_query,
    }
}

fn evaluate_obs_ids(
    system_name: &str,
    fixture: &Fixture,
    token_baselines: &TokenBaselines,
    retriever: ObsIdRetriever,
) -> SystemMetrics {
    let mut per_query = Vec::with_capacity(fixture.queries.len());
    for q in &fixture.queries {
        let relevant: HashSet<String> = q.relevant_obs_ids.iter().cloned().collect();
        let start = Instant::now();
        let retrieved = retriever(&q.query);
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
        let tokens = query_token_metrics(&q.query, &retrieved, token_baselines);
        per_query.push(QueryMetrics {
            query: q.query.clone(),
            category: q.category.clone(),
            query_tokens: tokens.query_tokens,
            retrieved_context_tokens: tokens.retrieved_context_tokens,
            full_context_tokens: tokens.full_context_tokens,
            memory_200_line_tokens: tokens.memory_200_line_tokens,
            savings_vs_full_context_pct: tokens.savings_vs_full_context_pct,
            savings_vs_memory_200_pct: tokens.savings_vs_memory_200_pct,
            recall_at_5: recall(&retrieved, &relevant, 5),
            recall_at_10: recall(&retrieved, &relevant, 10),
            recall_at_20: recall(&retrieved, &relevant, 20),
            precision_at_5: precision(&retrieved, &relevant, 5),
            precision_at_10: precision(&retrieved, &relevant, 10),
            ndcg_at_10: ndcg_at_k(&retrieved, &relevant, 10),
            mrr: mrr(&retrieved, &relevant),
            relevant_count: relevant.len(),
            retrieved_count: retrieved.len(),
            latency_ms,
        });
    }
    SystemMetrics {
        system: system_name.to_string(),
        avg_query_tokens: avg(&per_query
            .iter()
            .map(|q| q.query_tokens as f64)
            .collect::<Vec<_>>()),
        avg_retrieved_context_tokens: avg(&per_query
            .iter()
            .map(|q| q.retrieved_context_tokens as f64)
            .collect::<Vec<_>>()),
        avg_full_context_tokens: avg(&per_query
            .iter()
            .map(|q| q.full_context_tokens as f64)
            .collect::<Vec<_>>()),
        avg_memory_200_line_tokens: avg(&per_query
            .iter()
            .map(|q| q.memory_200_line_tokens as f64)
            .collect::<Vec<_>>()),
        avg_savings_vs_full_context_pct: avg(&per_query
            .iter()
            .map(|q| q.savings_vs_full_context_pct)
            .collect::<Vec<_>>()),
        avg_savings_vs_memory_200_pct: avg(&per_query
            .iter()
            .map(|q| q.savings_vs_memory_200_pct)
            .collect::<Vec<_>>()),
        avg_recall_at_5: avg(&per_query.iter().map(|q| q.recall_at_5).collect::<Vec<_>>()),
        avg_recall_at_10: avg(&per_query.iter().map(|q| q.recall_at_10).collect::<Vec<_>>()),
        avg_recall_at_20: avg(&per_query.iter().map(|q| q.recall_at_20).collect::<Vec<_>>()),
        avg_precision_at_5: avg(&per_query
            .iter()
            .map(|q| q.precision_at_5)
            .collect::<Vec<_>>()),
        avg_precision_at_10: avg(&per_query
            .iter()
            .map(|q| q.precision_at_10)
            .collect::<Vec<_>>()),
        avg_ndcg_at_10: avg(&per_query.iter().map(|q| q.ndcg_at_10).collect::<Vec<_>>()),
        avg_mrr: avg(&per_query.iter().map(|q| q.mrr).collect::<Vec<_>>()),
        avg_latency_ms: avg(&per_query.iter().map(|q| q.latency_ms).collect::<Vec<_>>()),
        per_query,
    }
}

fn upstream_query_terms(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() > 2)
        .map(String::from)
        .collect()
}

fn eval_builtin_memory(fixture: &Fixture, token_baselines: &TokenBaselines) -> SystemMetrics {
    evaluate_obs_ids(
        "Built-in (CLAUDE.md / grep)",
        fixture,
        token_baselines,
        Box::new(|query| {
            let terms = upstream_query_terms(query);
            let mut scored = Vec::new();
            for obs in &fixture.observations {
                let text = [
                    obs.title.as_str(),
                    obs.narrative.as_str(),
                    &obs.concepts.join(" "),
                    &obs.facts.join(" "),
                ]
                .join(" ")
                .to_lowercase();
                let score = terms
                    .iter()
                    .filter(|term| text.contains(term.as_str()))
                    .count();
                if score > 0 {
                    scored.push((score, obs.id.clone()));
                }
            }
            scored.sort_by_key(|(score, _)| Reverse(*score));
            scored.into_iter().map(|(_, id)| id).take(20).collect()
        }),
    )
}

fn eval_builtin_memory_truncated(
    fixture: &Fixture,
    token_baselines: &TokenBaselines,
) -> SystemMetrics {
    let lines: Vec<String> = fixture.observations.iter().map(memory_200_line).collect();

    evaluate_obs_ids(
        "Built-in (200-line MEMORY.md)",
        fixture,
        token_baselines,
        Box::new(|query| {
            let terms = upstream_query_terms(query);
            let mut scored = Vec::new();
            for (obs, line) in fixture
                .observations
                .iter()
                .zip(lines.iter())
                .take(MEMORY_200_LINE_LIMIT)
            {
                let lower_line = line.to_lowercase();
                let score = terms
                    .iter()
                    .filter(|term| lower_line.contains(term.as_str()))
                    .count();
                if score > 0 {
                    scored.push((score, obs.id.clone()));
                }
            }
            scored.sort_by_key(|(score, _)| Reverse(*score));
            scored.into_iter().map(|(_, id)| id).take(20).collect()
        }),
    )
}

// ── Report writer ───────────────────────────────────────────────────────────

fn pct(x: f64) -> String {
    format!("{:.1}%", x * 100.0)
}

fn token_count_fmt(x: f64) -> String {
    format!("{:.0}", x)
}

fn md_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn write_markdown_report(report: &Report, out_dir: &Path) {
    let md_path = out_dir.join("memory_quality.md");
    let mut s = String::new();
    s.push_str("# TerranSoul vs agentmemory — Quality Bench Parity\n\n");
    s.push_str(&format!(
        "> Generated by `cargo bench --bench memory_quality`.\n> Fixture: {} observations · {} queries · embedding={}\n> Token estimator: {}\n> Upstream pinned commit: `{}`\n\n",
        report.observations,
        report.queries,
        report.embedding_mode,
        report.token_estimator,
        report.upstream_pinned_commit,
    ));
    s.push_str("## Head-to-Head\n\n");
    s.push_str("| System | Recall@5 | Recall@10 | Precision@5 | NDCG@10 | MRR | Latency | Avg retrieved memory tokens | Saved vs full context | Saved vs 200-line |\n");
    s.push_str("|---|---|---|---|---|---|---|---|---|---|\n");
    for sys in &report.systems {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {:.2}ms | {} | {} | {} |\n",
            sys.system,
            pct(sys.avg_recall_at_5),
            pct(sys.avg_recall_at_10),
            pct(sys.avg_precision_at_5),
            pct(sys.avg_ndcg_at_10),
            pct(sys.avg_mrr),
            sys.avg_latency_ms,
            token_count_fmt(sys.avg_retrieved_context_tokens),
            pct(sys.avg_savings_vs_full_context_pct),
            pct(sys.avg_savings_vs_memory_200_pct),
        ));
    }
    if let Some(first) = report.systems.first() {
        s.push_str("\n## Token Efficiency\n\n");
        s.push_str(&format!(
            "The harness estimates tokens with `{}` to match TerranSoul's existing ingest/accounting convention. Full-context paste is every fixture observation rendered as full memory text. The 200-line baseline is the upstream-style MEMORY.md summary capped at 200 observation lines. Retrieved-memory tokens are the full text for each system's top-20 retrieved memories.\n\n",
            report.token_estimator,
        ));
        s.push_str("| Baseline | Tokens per query |\n");
        s.push_str("|---|---|\n");
        s.push_str(&format!(
            "| Full-context paste | {} |\n",
            token_count_fmt(first.avg_full_context_tokens),
        ));
        s.push_str(&format!(
            "| 200-line MEMORY.md | {} |\n\n",
            token_count_fmt(first.avg_memory_200_line_tokens),
        ));

        s.push_str("| System | Avg retrieved memory tokens | Saved vs full context | Saved vs 200-line MEMORY.md |\n");
        s.push_str("|---|---|---|---|\n");
        for sys in &report.systems {
            s.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                sys.system,
                token_count_fmt(sys.avg_retrieved_context_tokens),
                pct(sys.avg_savings_vs_full_context_pct),
                pct(sys.avg_savings_vs_memory_200_pct),
            ));
        }
    }

    s.push_str("\n## Per-Query Token Report\n\n");
    s.push_str("| System | Category | Query | Query tokens | Retrieved memory tokens | Saved vs full context | Saved vs 200-line |\n");
    s.push_str("|---|---|---|---|---|---|---|\n");
    for sys in &report.systems {
        for query in &sys.per_query {
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                md_cell(&sys.system),
                md_cell(&query.category),
                md_cell(&query.query),
                query.query_tokens,
                query.retrieved_context_tokens,
                pct(query.savings_vs_full_context_pct),
                pct(query.savings_vs_memory_200_pct),
            ));
        }
    }

    s.push_str("\n## agentmemory v0.6.0 Reference Numbers\n\n");
    s.push_str("| System | Recall@10 | NDCG@10 | MRR | Source |\n");
    s.push_str("|---|---|---|---|---|\n");
    s.push_str("| Built-in (CLAUDE.md / grep) | 55.8% | 80.3% | 82.5% | upstream QUALITY.md |\n");
    s.push_str("| Built-in (200-line MEMORY.md) | 37.8% | 56.4% | 65.5% | upstream QUALITY.md |\n");
    s.push_str("| BM25-only | 55.9% | 82.7% | 95.5% | upstream QUALITY.md |\n");
    s.push_str("| Dual-stream (BM25+Vector) | 58.6% | 84.7% | 95.4% | upstream QUALITY.md |\n");
    s.push_str(
        "| Triple-stream (BM25+Vector+Graph) | 58.0% | 81.7% | 87.9% | upstream QUALITY.md |\n",
    );
    s.push('\n');
    s.push_str(&format!("Source: {}\n", report.upstream_source));
    fs::write(&md_path, s).expect("failed to write markdown report");
    eprintln!("[memory_quality] wrote {}", md_path.display());
}

// ── Entry point (Cargo bench harness=false) ─────────────────────────────────

fn main() {
    // Locate fixture relative to CARGO_MANIFEST_DIR (src-tauri/).
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()));
    let fixture_path = manifest_dir.join("benches/memory_quality_fixture.json");
    let raw = fs::read_to_string(&fixture_path).unwrap_or_else(|e| {
        panic!(
            "failed to read fixture at {} — run `node scripts/build-memory-quality-fixture.mjs` first: {}",
            fixture_path.display(),
            e
        )
    });
    let fixture: Fixture = serde_json::from_str(&raw).expect("invalid fixture JSON");
    eprintln!(
        "[memory_quality] fixture: {} observations · {} queries · commit {}",
        fixture.observations.len(),
        fixture.queries.len(),
        fixture.pinned_commit,
    );

    let embedding_mode =
        std::env::var("TS_BENCH_AM_EMBED").unwrap_or_else(|_| "deterministic".to_string());
    let with_embedding = embedding_mode != "none";
    eprintln!("[memory_quality] embedding mode = {embedding_mode}");

    let baselines = token_baselines(&fixture);
    let ingested = ingest(&fixture, with_embedding);

    let mut systems = Vec::new();

    // ── Upstream built-in baselines from agentmemory quality-eval.ts ──────
    systems.push(eval_builtin_memory(&fixture, &baselines));
    systems.push(eval_builtin_memory_truncated(&fixture, &baselines));

    // ── System 1: keyword search (TerranSoul `search`) ────────────────────
    // Uses FTS5/keyword path only — matches agentmemory's BM25-only row.
    systems.push(evaluate(
        "TerranSoul keyword-only (search)",
        &fixture,
        &ingested,
        &baselines,
        false,
        Box::new(|q, _| {
            ingested
                .store
                .search(q)
                .map(|v| v.into_iter().map(|e| e.id).collect())
                .unwrap_or_default()
        }),
    ));

    // ── System 2: hybrid_search (6-signal) without vectors ────────────────
    systems.push(evaluate(
        "TerranSoul hybrid_search (no vectors)",
        &fixture,
        &ingested,
        &baselines,
        false,
        Box::new(|q, _| {
            ingested
                .store
                .hybrid_search(q, None, 20)
                .map(|v| v.into_iter().map(|e| e.id).collect())
                .unwrap_or_default()
        }),
    ));

    // ── System 3: hybrid_search with deterministic vectors ────────────────
    if with_embedding {
        systems.push(evaluate(
            "TerranSoul hybrid_search (deterministic vectors)",
            &fixture,
            &ingested,
            &baselines,
            true,
            Box::new(|q, qe| {
                ingested
                    .store
                    .hybrid_search(q, qe, 20)
                    .map(|v| v.into_iter().map(|e| e.id).collect())
                    .unwrap_or_default()
            }),
        ));
    }

    // ── System 4: hybrid_search_rrf without vectors ───────────────────────
    systems.push(evaluate(
        "TerranSoul hybrid_search_rrf (no vectors)",
        &fixture,
        &ingested,
        &baselines,
        false,
        Box::new(|q, _| {
            ingested
                .store
                .hybrid_search_rrf(q, None, 20)
                .map(|v| v.into_iter().map(|e| e.id).collect())
                .unwrap_or_default()
        }),
    ));

    // ── System 5: hybrid_search_rrf with deterministic vectors ────────────
    if with_embedding {
        systems.push(evaluate(
            "TerranSoul hybrid_search_rrf (deterministic vectors)",
            &fixture,
            &ingested,
            &baselines,
            true,
            Box::new(|q, qe| {
                ingested
                    .store
                    .hybrid_search_rrf(q, qe, 20)
                    .map(|v| v.into_iter().map(|e| e.id).collect())
                    .unwrap_or_default()
            }),
        ));
    }

    // ── Print + persist results ───────────────────────────────────────────
    println!("\n=== TerranSoul vs agentmemory quality bench ===");
    println!(
        "{:<55} {:>9} {:>10} {:>10} {:>8} {:>11} {:>11}",
        "system", "R@10", "NDCG@10", "MRR", "ms", "tokens", "saved"
    );
    for sys in &systems {
        println!(
            "{:<55} {:>9} {:>10} {:>10} {:>8.2} {:>11.0} {:>11}",
            sys.system,
            pct(sys.avg_recall_at_10),
            pct(sys.avg_ndcg_at_10),
            pct(sys.avg_mrr),
            sys.avg_latency_ms,
            sys.avg_retrieved_context_tokens,
            pct(sys.avg_savings_vs_full_context_pct),
        );
    }

    let out_dir = std::env::var("TS_BENCH_AM_OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Default: <workspace>/target-copilot-bench/bench-results
            manifest_dir
                .parent()
                .unwrap_or(&manifest_dir)
                .join("target-copilot-bench/bench-results")
        });
    fs::create_dir_all(&out_dir).expect("failed to create bench results dir");

    let report = Report {
        benchmark: "memory_quality",
        upstream_source: fixture.source.clone(),
        upstream_pinned_commit: fixture.pinned_commit.clone(),
        observations: fixture.observations.len(),
        queries: fixture.queries.len(),
        embedding_mode: embedding_mode.clone(),
        token_estimator: "chars.div_ceil(4)",
        systems,
    };
    let json_path = out_dir.join("memory_quality.json");
    fs::write(
        &json_path,
        serde_json::to_string_pretty(&report).expect("serialize report"),
    )
    .expect("failed to write JSON report");
    eprintln!("[memory_quality] wrote {}", json_path.display());
    write_markdown_report(&report, &out_dir);
}
