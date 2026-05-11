# Billion-Scale Retrieval & Graph — Design

> Status: **Phases 1, 4, 5, and cross-cutting complete.** Honest scoping doc.
> Phases 2 and 3 remain roadmap. Land in priority order; each phase is
> independently shippable.
> Last updated: 2026-05-11

## Why this exists

TerranSoul's brain currently scales comfortably to ~10 million long-term memories
on a single workstation (SQLite + a single `usearch` HNSW index with optional
`i8` / `b1` quantization, plus a 6-signal hybrid search and an RRF fusion path).
Past ~100M the single-index design starts to fall over:

- Build time for one global HNSW becomes hours, and a single corrupt write can
  invalidate the whole index.
- RAM usage of a single index dominates the working set even with `i8` quant
  (≈ 768 B/vec × 100M = 76.8 GB).
- Hybrid search latency degrades because keyword + recency + decay signals all
  scan one large table.
- The 2D graph viewport tries to materialize every node into JS memory, which
  is impossible past ~50k nodes on Canvas2D (and the screen has only ~2M pixels
  anyway).

This doc captures the path to **1B records, single machine** for both
chat/MCP retrieval and the graph viewport.

## Non-goals

- Distributed clusters / multi-host sharding. Local-first is the product.
- Replacing SQLite as the source of truth. We add indexes around it.
- Real-time interactive rendering of a billion nodes simultaneously. That is
  physically impossible at any zoom level — the screen does not have a billion
  pixels. The graph UX is always **viewport + LOD streaming**.

## Honest physical limits

| Resource | 1B vectors @ 768 dim | Notes |
|---|---|---|
| Raw `f32` | 3.07 TB | Disk-only territory. |
| `i8` quant | 768 GB | Still too big for typical RAM. |
| `b1` quant | 96 GB | Fits high-end workstation RAM; recall loss real. |
| PQ (m=96, nbits=8) | 96 GB → ~96 B/vec | Industry standard for >100M. |
| OPQ + IVF + PQ | ~24–48 GB hot working set | DiskANN / FAISS-IVFPQ class. |

Implication: at 1B scale, **the index does not live entirely in RAM**, and
**a single index is not appropriate**. We need sharding + a coarse router.

## Phase plan

### Phase 1 — Foundation (this PR)

Land additive scaffolds and one user-visible win. No breaking changes.

1. **Shard-aware retrieval scaffold** (`memory/sharded_retrieval.rs`)
   - `ShardKey` derived from `(tier, cognitive_kind)` — 3 × 4 = 12 logical
     shards out of the box, all still living in the same SQLite for now.
   - `ShardedHybridSearch::search(query, query_emb, k)`: runs top-K per shard
     via the existing `hybrid_search_rrf`, then merges results across shards
     with RRF (k=60).
   - Cap on rerank pool (default 50) — never rerank thousands of candidates;
     pruning happens before the LLM-as-judge stage.
   - Opt-in via brain config flag. Default off until performance is verified.
2. **Paged graph endpoint** (`commands/memory.rs::memory_graph_page`)
   - Inputs: optional focus id, zoom level (`overview` / `cluster` / `detail`),
     node limit (default 2 000, max 10 000).
   - Returns: nodes ranked by `degree desc, importance desc, recency desc` plus
     edges that touch the returned node set.
   - For overview zoom: aggregates by `cognitive_kind` into supernodes; edges
     between supernodes are aggregated link counts.
   - Backend never returns more than `limit` nodes — the frontend cannot stall
     by asking for "everything".
3. **WebGL graph renderer** in `src/components/MemoryGraph.vue`
   - Sigma.js WebGL backend with the Obsidian-style chrome already in place.
   - Pulls a page from `memory_graph_page` on viewport change / zoom / focus.
   - Falls back to in-memory props when the host doesn't expose the new
     command (preserves Vitest / non-Tauri usage).
4. **Design doc** (this file) + milestone breakdown in `rules/milestones.md`.

### Phase 2 — Sharded HNSW (single-process) (✅ Shipped — Chunk 48.2/48.3/48.8/48.9)

- ✅ One `usearch` index per `ShardKey`, files under
  `<app-data>/vectors/<shard>.usearch`, each with its own quantization sidecar.
- ✅ Coarse router: a tiny IVF-style centroid lookup built from a 1% sample of
  embeddings to predict the top-`p` shards per query.
- ✅ Router persistence + reload: `shard_router.json` now stores centroid
  vectors + shard mapping so `load_shard_router()` can hydrate a queryable
  router across restarts instead of always rebuilding.
- ✅ `ShardedHybridSearch` consults the router → vector top-K per shard →
  RRF merge → keyword/decay/recency reranker on the union.
- ✅ `MemoryStore::rebalance_shards()` is available.
- ✅ Dedicated router refresh policy:
  - time trigger (stale/missing) + volume trigger (mutation delta),
  - cooldown-gated to avoid repeated on-query rebuild bursts,
  - forced scheduled refresh via maintenance `AnnCompact` path.

### Phase 3 — Disk-backed ANN (PQ / IVF-PQ) (◐ Kickoff shipped — Chunk 49.1)

- ✅ Kickoff planner surface (`memory/disk_backed_ann.rs` +
  `MemoryStore::disk_ann_plan`) reports per-shard migration candidates by
  threshold and ANN index-file presence for deterministic rollout sequencing.

- For shards > N million entries, build IVF-PQ indexes (target m=96, nbits=8)
  using a Rust binding to FAISS or `usearch`'s pq mode.
- Memory-map shard files; only the active centroid lists are paged into RAM.
- Refresh PQ codebooks during nightly compaction.

### Phase 4 — Keyword + recency scale-out (✅ Shipped — Chunk 48.5)

- ✅ FTS5 external-content virtual table `memories_fts` with unicode61
  tokenizer, auto-sync INSERT/UPDATE/DELETE triggers, and
  `keyword_candidate_ids_fts5()` fast path with INSTR fallback.
- ✅ Schema V21 creates FTS5 table per-shard (via `ensure_v21_fts5()`).
- Store `last_accessed` and `decay_score` in a covering index so recency
  signals never scan the full table.

### Phase 5 — Graph at 1B (✅ Shipped — Chunk 48.6)

- ✅ Composite covering indexes `(src_id, rel_type)` and `(dst_id, rel_type)`
  on `memory_edges` enable O(log n) filtered adjacency queries.
- ✅ `memory_graph_clusters` pre-aggregated table refreshed during
  `AnnCompact` maintenance job.
- ✅ `memory/graph_paging.rs` module with `get_edges_paged()`,
  `get_top_degree_nodes()`, `get_graph_clusters()`, `graph_totals()`.
- ✅ `memory_graph_page` command uses paged adjacency fast path for
  detail zoom + focus_id (loads only neighbourhood, not entire graph).
- Frontend never holds more than ~5 000 nodes simultaneously. Cluster
  supernodes at overview zoom, expand on focus.

## Cross-cutting rules (✅ Shipped — Chunk 48.7)

- **No global locks during search.** Per-shard `RwLock`, parallel queries via
  `rayon` or `tokio::task::spawn_blocking`.
- **Bounded rerank.** LLM-as-judge runs on at most `rerank_cap` (default 50)
  candidates regardless of `limit`.
- **Hot-cache.** ✅ Last N query → top-K results cached for ≤ 60 s to absorb
  duplicate chat-loop calls. (`search_cache.rs`, `DEFAULT_TTL_MS = 60_000`.)
- **Backpressure.** ✅ Ingest jobs that would push a shard past
  `shard_max_entries` (default 2M per logical shard) are rejected with
  `BackpressureError`; callers must split/rebalance.
  (`memory/shard_backpressure.rs`, `DEFAULT_SHARD_MAX_ENTRIES = 2_000_000`.)
- **No silent fallbacks.** ✅ If a shard's index is missing/corrupt, the
  `shard_health_summary()` reports it through `HealthResponse.shard_health`
  in the `brain_health` MCP tool; it does not pretend the result set is
  complete. (`ai_integrations/gateway.rs`.)

## Phase 1 acceptance

- [x] `memory/sharded_retrieval.rs` exists with unit tests covering shard
      key derivation, top-K-per-shard, and RRF merge.
- [x] `commands/memory.rs::memory_graph_page` exists with unit tests for
      limit enforcement and supernode aggregation.
- [x] `MemoryGraph.vue` renders via sigma.js when available, with Canvas2D
      fallback for environments without WebGL (Vitest / jsdom).
- [x] Existing test suites stay green: `npx vitest run`, `cargo test --lib`.
- [x] Design doc + milestone entry committed.

## References (credit in `CREDITS.md` when applicable)

- [brain-advanced-design.md](brain-advanced-design.md) — canonical brain/memory
  architecture (this doc extends its §16 Scaling Roadmap Phase 7).
- [brain-advanced-design-mappings-features.md](brain-advanced-design-mappings-features.md)
  — per-feature chatbox audit that maps billion-scale infra to daily use.
- Cormack et al., "Reciprocal Rank Fusion outperforms Condorcet and individual
  rank learning methods", SIGIR 2009.
- Subramanya et al., "DiskANN: Fast Accurate Billion-point Nearest Neighbor
  Search on a Single Node", NeurIPS 2019.
- Jégou, Douze, Schmid, "Product quantization for nearest neighbor search",
  IEEE TPAMI 2011.
- Anthropic, "Contextual Retrieval", 2024.
- `unum-cloud/usearch` and `facebookresearch/faiss` for the index implementations.
- `jacomyal/sigma.js` for the WebGL graph renderer used in the viewport.
