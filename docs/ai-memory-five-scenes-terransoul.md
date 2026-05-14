# AI Memory in Five Scenes — Where TerranSoul Sits on the Curve

> **Inspired by** cognee's [*AI memory in five scenes*](https://www.cognee.ai/blog/fundamentals/ai-memory-in-five-scenes)
> (Vasilije Markovic et al., 2025). The cognee article walks an unfamiliar
> reader through the progression **base LLM → classic RAG → graph-aware
> RAG → hybrid memory at scale** by retelling it as five everyday
> scenes. This document maps each of those five scenes onto **what
> TerranSoul actually ships today**, with citations into the codebase and
> the architecture deep dive.
>
> No prose, imagery, or examples from the original article are copied.
> Credit and license context are recorded in
> [`CREDITS.md`](../CREDITS.md) per the project's credits rule.

The cognee article is a great five-minute primer on *why* "AI memory"
isn't a single thing. TerranSoul's Hybrid RAG brain was designed against
exactly the same progression of failure modes that the article walks
through, so the five-scenes lens is a useful way to explain — to a new
contributor or a non-technical user — *which scene TerranSoul lives in*
and *what infrastructure is required to live there*.

**Short answer:** TerranSoul is built for **Scene 5**. The brain
deliberately fuses vector recall (Scene 3), per-user/preference context
(Scene 2), and a typed knowledge graph + temporal memory (Scene 4) so a
single chat turn answers like the senior librarian from Scene 1 rather
than the keyword-blind catalogue clerk.

---

## Quick map: the five scenes ↔ TerranSoul

| Scene (cognee) | What it isolates | Where TerranSoul implements it |
|---|---|---|
| **1. Kid in a library** — base LLM vs catalogue clerk vs senior librarian | Three kinds of "knowledge worker" the user can choose from | The three-mode brain — see [Brain Modes](#scene-1-the-three-employees--three-brain-modes) below |
| **2. High schooler at the movies** — preference-aware context construction | Why retrieval needs *per-user* signals, not just raw catalogue dump | Persona + observation + temporal-recency layer ([Scene 2](#scene-2-the-personal-movie-guru--persona--user-context)) |
| **3. College student preparing for an exam** — chunk → embed → semantic search | Standard "vector RAG over my notes" | HNSW vector + semantic chunking + Contextual Retrieval ([Scene 3](#scene-3-the-exam-prep-tool--vector-rag-over-your-corpus)) |
| **4. Junior IT engineer job hunt** — entity/relation extraction → GraphRAG | Why filters / multi-hop reasoning need a **typed knowledge graph** | `memory_edges` + entity resolution + multi-hop traversal ([Scene 4](#scene-4-the-job-hunt--graphrag-over-typed-entities)) |
| **5. AI startup CTO at a party** — hybrid memory at scale (speed, quality, measurability) | The production challenges nobody can skip | Sharded HNSW + RRF + caching + telemetry + bench harness ([Scene 5](#scene-5-the-cto-at-the-party--hybrid-memory-in-production)) |

---

## Scene 1: The three employees → three brain modes

The article's library scene compares three workers a 12-year-old could
ask for help: the eloquent newcomer (a base LLM with no place-knowledge),
the screen-bound search clerk (basic retrieval, no judgment), and the
senior librarian who *is* the library. The point is that each one is the
right tool for some questions and the wrong tool for others.

TerranSoul exposes the same trade-off explicitly through its **three
brain modes**, so the user can choose where they sit on the
privacy / cost / quality / latency triangle:

| Brain mode | "Library employee" analog | Reality in TerranSoul |
|---|---|---|
| **Free API** (Pollinations / OpenRouter / Gemini free) | The eloquent newcomer — broad and free, no local context until you give it some | Zero-config first-launch path; chat works before any setup |
| **Paid API** (OpenAI / Anthropic / Groq with your key) | The screen-bound clerk plus a smart manager — accurate, fast, costs per question | Best raw answer quality; cloud-side embeddings for RAG |
| **Local Ollama** (`mxbai-embed-large` for vectors, your chosen chat model) | The senior librarian who *lives* in the library — fully offline, no leaks | Default for privacy-sensitive corpora and air-gapped use |

All three modes plug into the **same** memory store, persona, and
retrieval pipeline, so switching providers does not lose your library.
That's the whole point of TerranSoul's `BrainGateway` abstraction
(`src-tauri/src/ai_integrations/gateway.rs`) — provider swap, brain
unchanged.

> Code: `src-tauri/src/brain/`, `src-tauri/src/ai_integrations/gateway.rs`. Doc: [`docs/brain-advanced-design.md` § 4](brain-advanced-design.md#4-brain-modes--three-pluggable-backends-same-surface).

---

## Scene 2: The "personal movie guru" → persona + user context

The article's streaming-service scene is about a bot that doesn't dump
the whole catalogue at the LLM — it constructs a *cleaner* context using
what it already knows about you (genres you like, what's available on
your plan, tone preferences). The lesson: retrieval needs **personal
signals**, not just topical similarity.

TerranSoul's brain composes this kind of personal context from
**multiple first-class layers**, all of which feed every chat turn:

- **Persona traits** (`src-tauri/src/persona/`) — long-lived facts
  about the user and their stated preferences, with drift detection.
- **Observation history** — what files, projects, and people the user
  actually interacts with bias retrieval toward the active workspace.
- **Recency / decay layer** (`src-tauri/src/memory/temporal.rs`,
  `confidence_decay.rs`) — newer facts override older ones; decayed
  rows fall in the score formula.
- **Cognitive-kind axis** (`src-tauri/src/memory/cognitive_kind.rs`) —
  the same query is reranked differently for `episodic` ("what did I
  do") vs `semantic` ("what is true") vs `procedural` ("how do I")
  vs `judgment` ("what did I decide") intents.
- **Privacy ACL** (`src-tauri/src/memory/privacy.rs`) — `private` rows
  never leave the device, `paired` rows sync to your own devices,
  `hive` rows can be shared with a relay. The user's context is
  always assembled with the most-private layer winning.

The result is closer to the cognee article's "personal movie guru" than
to a raw vector dump: top-k is **scored**, **diversified per session**,
**capped on noisy clusters**, and **annotated with provenance** before
it lands in the prompt.

> Doc: [`docs/persona-design.md`](persona-design.md), [`docs/brain-advanced-design.md` § 3.5](brain-advanced-design.md#35-cognitive-memory-axes-episodic--semantic--procedural--judgment).

---

## Scene 3: The exam-prep tool → vector RAG over your corpus

The article's exam scene is the textbook RAG explanation: split notes
into chunks, embed each chunk, embed the question, return the top-k by
semantic similarity, and append to the prompt. The article is honest
that this works well for "what did Prof. Miller say about X" — and
collapses for "which examples were *not* in the textbook".

TerranSoul ships the strong version of this baseline:

| Capability | TerranSoul implementation |
|---|---|
| Semantic chunking | `src-tauri/src/memory/chunking.rs` (uses the `text-splitter` crate) |
| Late chunking + Anthropic-style **Contextual Retrieval** | `src-tauri/src/memory/late_chunking.rs`, `contextualize.rs` (gated on `LONGMEM_CONTEXTUALIZE=1`) |
| Vector embeddings | `mxbai-embed-large` (1024-d, default since BENCH-LCM-5) or `nomic-embed-text` (768-d, fallback) on Ollama; cloud `/v1/embeddings` for paid/free modes |
| HNSW ANN index | `src-tauri/src/memory/ann_index.rs` (`usearch` 2.x), per-shard files at `vectors/<tier>__<kind>.usearch` |
| Mobile / fallback search | `src-tauri/src/memory/mobile_ann.rs`, `offline_embed.rs` (deterministic 256-dim hashing for headless seeding) |
| FTS5 lexical retriever | rare-term + acronym weighting with broad-term caps (rescues exact identifiers vector misses) |
| Negative / "not in textbook" filter | `src-tauri/src/memory/negative.rs` |

**Where TerranSoul agrees with the article:** vector RAG alone is the
right tool for direct factual recall over a corpus you've ingested.

**Where TerranSoul goes further:** the article's "the model can't hold
all of it in its memory anyway" failure is exactly why our RAG harness
returns a **scored, threshold-cut, deduplicated** top-k rather than a
greedy concatenation, and why the `[LONG-TERM MEMORY]` block has a
strict token budget enforced by `src-tauri/src/memory/context_pack.rs`.

> Doc: [`docs/brain-advanced-design.md` § 4 RAG retrieval pipeline](brain-advanced-design.md#3-rag-retrieval-pipeline--what-happens-on-every-chat-turn).

---

## Scene 4: The job hunt → GraphRAG over typed entities

The article's most important scene. The job-hunting candidate wants
*"remote-first roles in Series B startups with 20–50 employees that use
Python and React and are not in fintech"*. Plain RAG returns plausible
text containing the keywords; **GraphRAG** extracts typed entities
(`Company`, `Job`, `Skill`, `FundingStage`, `Industry`,
`LocationPolicy`, `TeamSize`) and edges (`employs`, `requires`,
`operates_in`, `is_stage`, …), then retrieves the **subgraph** that
satisfies the constraints — with citations back to the source passages.

TerranSoul's V5 schema implements exactly this layer, natively in
SQLite, alongside the vector store:

| GraphRAG capability (cognee article) | TerranSoul implementation |
|---|---|
| Typed entities | `memories.cognitive_kind` (episodic / semantic / procedural / judgment) + extracted entity rows; entity resolution (Phase 6) deduplicates `Bill G.` / `William Gates` style splits |
| Typed, directional, confidence-weighted edges | `memory_edges` table — `src_id`, `dst_id`, `rel_type`, `confidence`; see canonical schema in `src-tauri/src/memory/schema.rs` |
| Edge extraction from text | `src-tauri/src/memory/auto_tag.rs`, `graph_rag.rs`, plus LLM-assisted "Extract from session" |
| Subgraph retrieval / multi-hop | `multi_hop_search_memories` Tauri command; `src-tauri/src/memory/graph_rag.rs` and `graph_page.rs` |
| KG neighbour boost on vector hits | gated `enable_kg_boost` setting, exposed via the MCP `brain_kg_neighbors` tool |
| Conflict / contradiction resolution | `src-tauri/src/memory/conflicts.rs` — LLM-as-judge resolves *"X was deprecated"* vs *"X is recommended"* across versions |
| Append-only versioning | `src-tauri/src/memory/versioning.rs` (V8) — every edit is a new row, prior rows kept for audit |

The article's job-hunt example —
`(Stage = Series B) ∧ (TeamSize ∈ [20,50]) ∧ (Skills ⊇ {Python, React}) ∧ (Industry ≠ Fintech) ∧ (RemoteFirst = true)` —
maps onto a TerranSoul query that fuses:

1. a vector hit on the job-description chunk,
2. one-hop traversal across `requires` / `operates_in` / `is_stage`
   edges in `memory_edges`,
3. a negative filter from `negative.rs` for the `Industry = Fintech`
   exclusion,
4. and the supporting source passage attached as the citation.

That is what the cognee article calls a "subgraph with receipts", and
it is the same shape the senior librarian gave the kid back in Scene 1.

> Doc: [`docs/brain-advanced-design.md` § 6 Knowledge Graph Vision](brain-advanced-design.md#6-knowledge-graph-vision).

---

## Scene 5: The CTO at the party → hybrid memory in production

The article's final scene names the production reality nobody at a party
wants to hear: hybrid GraphRAG is the *right* architecture, but you only
get to keep it if you also solve **speed**, **quality**, and
**measurability**. TerranSoul has a concrete answer for each.

### Speed — sub-second retrieval at scale

| cognee challenge | TerranSoul lever |
|---|---|
| Combine vector + inverted index + subgraph filters in <1 s | RRF fusion at `k=60` over 4 retrievers (`vector`, `lexical FTS5`, `KG-seeded`, `freshness`) — `src-tauri/src/memory/fusion.rs` |
| Pre-compute hot subgraphs / cache aggressively | `src-tauri/src/memory/kg_cache.rs`, `search_cache.rs`, `graph_paging.rs` |
| Route queries (RAG vs GraphRAG) with a lightweight classifier | `src-tauri/src/memory/query_intent.rs` + the cognitive-kind classifier (`cognitive_kind.rs`) |
| Degrade gracefully under tight budgets | `src-tauri/src/memory/shard_backpressure.rs`, mobile fallback `mobile_ann.rs`, "skip RAG for greetings" fast path |
| Scale past in-process limits | per-shard HNSW (15 logical shards = 3 tiers × 5 cognitive_kinds) with a persisted coarse centroid router (`vectors/shard_router.json`); IVF-PQ disk-backed shards (`disk_backed_ann.rs`) for >100M |

### Quality — meaningful entity types and evolving schemas

The article warns that auto-generated ontologies sound great until you
need domain-specific relationships. TerranSoul takes the iterative
approach the article recommends: a minimal vocabulary of edge types in
`memory_edges`, **temporal attributes** (`decay_score`, version
snapshots), **event-based** updates via `auto_learn.rs` /
`reflection.rs`, and **append-only versioning** so old queries keep
working as the schema evolves.

For knowledge that should never silently drift, the
[`docs/cap-profile.md`](cap-profile.md) work introduces **per-memory
CAP tags** — `personal` / `scratch` rows favour Availability (CRDT,
write-never-blocks), while `legal` / `financial` / `shared-team` rows
favour Consistency (consensus before commit). This is TerranSoul's
answer to the article's "versioning thousands of entities and their
evolving relationships without breaking existing queries".

### Measurability — bench, telemetry, golden sets

The article's measurability point is the one most projects skip.
TerranSoul ships:

- **Retrieval-quality bench** — `scripts/locomo-mteb.mjs` runs the full
  1976-query LoCoMo MTEB slice against four canonical pipelines
  (`rrf`, `rrf_rerank`, `rrf_hyde`, `rrf_hyde_rerank`); see
  [`docs/locomo-mteb-adapter.md`](locomo-mteb-adapter.md) and
  [`docs/longmemeval-s-adapter.md`](longmemeval-s-adapter.md).
- **Token-efficiency bench** — `BENCH-AM-4`: 91.4 % token savings
  vs full-context paste at R@10 63.6 % on the agentmemory case set
  (`benchmark/terransoul/agentmemory-quality/`).
- **Operational telemetry** — `src-tauri/src/memory/metrics.rs` plus
  `brain_health` MCP tool surfacing `router_health`, `disk_ann_health`,
  index ages, shard staleness.
- **Availability SLO** — five-nines target tracked in
  [`docs/availability-slo.md`](availability-slo.md) with in-process
  uptime telemetry and a chaos test (`RESILIENCE-1`).

The published numbers are honest about wins and ties: LongMemEval-S
retrieval-only at **R@5 99.2 %, R@10 99.6 %, NDCG@10 91.3 %, MRR 92.6 %**
beats agentmemory and MemPalace on the comparable slice; HyDE is wired
**per-query-class** (gated on for abstract/multi-hop/temporal,
gated off for under-specified open-ended) because LCM-10 showed
+2.9 pp on temporal_reasoning *and* −1.6 pp on open_domain — a
golden example of the article's "measure, don't assume" rule.

---

## What this means for a new contributor

If you are joining TerranSoul and trying to figure out *which file* to
touch for a memory feature, the five-scenes lens is the fastest map:

- **You're improving how chunks are split or embedded?** → Scene 3.
  Touch `src-tauri/src/memory/chunking.rs`,
  `late_chunking.rs`, or the embedder selection in
  `src-tauri/src/brain/`.
- **You're adding a new entity / relation type?** → Scene 4. Touch
  `src-tauri/src/memory/edges.rs`, `graph_rag.rs`, and the edge-extractor
  in `auto_tag.rs`. Add the type to the canonical schema in
  `schema.rs` and a numbered seed migration in `mcp-data/shared/migrations/`.
- **You're tuning per-user / per-persona retrieval?** → Scene 2. Touch
  `src-tauri/src/persona/`, `src-tauri/src/memory/cognitive_kind.rs`,
  `query_intent.rs`.
- **You're shipping a new bench, cache, or SLO?** → Scene 5. Touch
  `src-tauri/src/memory/metrics.rs`, the `BENCH-*` chunks in
  `rules/milestones.md`, and the docs under `docs/` matching your
  axis (speed / quality / measurability).
- **You're working on the brain mode itself?** → Scene 1. Touch
  `src-tauri/src/brain/` and the `BrainGateway` adapters in
  `src-tauri/src/ai_integrations/gateway.rs`.

Cross-references for full context:

- Architecture deep dive: [`docs/brain-advanced-design.md`](brain-advanced-design.md)
- Why hybrid (vs vector-only) RAG: [README "Why Hybrid RAG" section](../README.md#why-hybrid-rag-vector--knowledge-graph--temporal-memory)
- Knowledge architecture: [`docs/project-knowledge-architecture.md`](project-knowledge-architecture.md)
- Cross-instance / hive sharing: [`docs/hive-protocol.md`](hive-protocol.md)
- Companion ecosystem (Hermes / OpenClaw / Temporal-style runner): [README "Companion AI Ecosystem"](../README.md#companion-ai-ecosystem)
- Source attribution: [`CREDITS.md`](../CREDITS.md)

---

## Why credit the article

Cognee's article is a *narrative* contribution, not a code contribution.
It does not ship runtime behaviour into TerranSoul — but it gave us a
crisp, beginner-friendly five-scene framing that we used to write this
README pointer and to organise the new-contributor map above. Per the
project's credits rule, that kind of design influence is exactly what
[`CREDITS.md`](../CREDITS.md) is for. The cognee article is
acknowledged there alongside the QuarkAndCode "GraphRAG vs Vector RAG"
post that informed the existing "Why Hybrid RAG" section, the agentmemory
+ Graphify pattern, the "Stop Calling It Memory" critique, and the other
sources whose thinking shaped the brain.
