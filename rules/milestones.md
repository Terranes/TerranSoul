# TerranSoul — Milestones

> **To continue development, tell the AI agent:**
>
> ```
> Continue
> ```
>
> The agent will read this file, find the next chunk with status `not-started`,
> implement it, update the status to `done`, **remove the row from this file**,
> and log details in `rules/completion-log.md`.

> **ENFORCEMENT RULE — Completed chunks must be archived.**
>
> When a chunk is marked `done`:
> 1. Log full details (date, goal, architecture, files created/modified, test counts) in `rules/completion-log.md`.
> 2. **Remove the done row from this file.** `milestones.md` contains only `not-started` and `in-progress` chunks.
> 3. If an entire phase has no remaining rows, drop the phase heading too.
> 4. Update `Next Chunk` (below) to point to the next `not-started` chunk.
>
> This rule is mandatory for every AI agent session. Never leave done rows
> in milestones.md — the full historical record lives in `completion-log.md`.
>
> **Additional:** If the chunk was derived from reverse-engineering research,
> also clean up `rules/research-reverse-engineering.md` and `rules/backlog.md`.
> See `rules/prompting-rules.md` -> "ENFORCEMENT RULE — Clean Up Reverse-Engineering Research on Chunk Completion".

> **Completed work lives in [`rules/completion-log.md`](completion-log.md).**
> Do not re-list done chunks here. Chunks are recorded there in reverse-chronological order.

---

## Next Chunk

Next up: **Chunk 48.2 — Per-shard `usearch` index files (sharded HNSW)**

---

## Phase 48 — Billion-Scale Retrieval & Graph

> See [`docs/billion-scale-retrieval-design.md`](../docs/billion-scale-retrieval-design.md)
> for the full phased plan, honest physical limits, and acceptance criteria.
> Phase 1 (foundation scaffold + paged-graph command + Lite/WebGL renderer)
> is already complete and recorded in `completion-log.md`.

| Chunk | Status | Title | Notes |
|---|---|---|---|
| 48.2 | not-started | Per-shard `usearch` HNSW indexes | One index file per `ShardKey` under `<app-data>/vectors/<shard>.usearch`, with per-shard quantization sidecar. `ShardedHybridSearch` consults shards in parallel and merges via RRF. Adds `MemoryStore::rebalance_shards()` + background compaction. |
| 48.3 | not-started | Coarse shard router (IVF-style centroids) | Build a small centroid index from a 1% sample of embeddings so each query only probes top-p shards instead of all 15. Stored alongside vectors. Falls back to "probe all" when the router is missing or stale. |
| 48.4 | not-started | Disk-backed ANN (IVF-PQ / DiskANN-class) | For shards over `shard_max_entries` (default 50M) build IVF-PQ indexes (m=96, nbits=8). Memory-map shard files; refresh PQ codebooks during nightly compaction. Gated on the `native-ann` feature. |
| 48.5 | not-started | FTS5 per-shard keyword index | Migrate BM25-lite from SQL `LIKE` to an FTS5 virtual table per shard (or `tantivy` if FTS5 hits limits). Add covering indexes for `last_accessed` / `decay_score` so recency signals never scan the full table. |
| 48.6 | not-started | Paged knowledge graph at 1B | Move KG traversal off in-memory `Vec<MemoryEdge>` onto paged adjacency with covering indexes `(src_id, edge_type)` and `(dst_id, edge_type)`. Pre-aggregated `memory_graph_clusters` table refreshed during compaction. Frontend stays at ≤ 5k visible nodes via existing `memory_graph_page` LOD. |
| 48.7 | not-started | Backpressure + hot-cache + health surface | Reject ingests that would push a shard past `shard_max_entries` (trigger split/rebalance instead of degrading search). Last-N query → top-K cache for ≤ 60 s. Per-shard health (index missing/corrupt/dirty) wired into `brain_health` so the search layer never silently returns partial results. |

---

 



