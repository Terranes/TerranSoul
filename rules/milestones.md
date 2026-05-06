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
> Do not re-list done chunks here. Phases 0–28 and all previously tracked
> chunks are recorded there in reverse-chronological order.

---

## Next Chunk

The next `not-started` chunk is **41.2 — Per-op latency metrics** under
[Phase 41](#phase-41--million-knowledge-crud-may-2026). Implement chunks
in order; each one must keep the Full CI Gate green before moving on.

> **Phase 41 status (2026-05-07).** Chunk 41.1 (SQLite PRAGMA tuning) and
> 41.4 (transactional `add_many`) shipped. Measured 1M write 6.37 s
> @ 157 k rows/s, 1M read 1.84 s @ 545 k rows/s — the < 60 s / < 5 s
> target is met. Remaining chunks are still valuable for production
> hardening (metrics, re-embed-on-update, ANN GC, KG cache) but are no
> longer blocking the headline throughput goal.

---

## Phase 41 — Million-Knowledge CRUD (May 2026)

> **Goal.** Optimal create / read / update / delete on the TerranSoul
> brain at **1M+ memories with many chunks per document**, with no
> hacky shortcuts and no functional regressions. Plan is grounded in
> the May 2026 audit below — fixes target real bottlenecks observed in
> the current code, not theoretical ones.

### Audit (state through May 2026)

What is already in place and good:

- SQLite WAL + `synchronous=NORMAL` + `foreign_keys=ON` (`memory/store.rs`).
- V15 schema with indexes on `importance`, `created_at`, `source_hash`,
  `tier`, `session_id`, `decay_score`, `category`, `updated_at`, plus
  composite `idx_memories_eviction(tier, importance, decay_score)`
  (`memory/schema.rs`).
- HNSW via `usearch` behind `native-ann`, persisted to `vectors.usearch`,
  doubling capacity reserve, threaded reserve, brute-force fallback.
- Capacity eviction with a single-statement scored DELETE
  (`memory/eviction.rs`), 1M cap default, 0.95 target ratio, audit log.
- Self-healing embedding queue with exponential backoff capped at 1h
  (`memory/embedding_queue.rs`).
- Pre-update version snapshot in `memory_versions` (`memory/versioning.rs`).
- LLM-checked contradiction detection on cosine ≥ 0.85 near-duplicates
  (`memory/conflicts.rs`).
- Markdown / plaintext semantic chunker with SHA-256 dedup
  (`memory/chunking.rs`), late-chunking pooling helper, contextual
  retrieval prefix at ingest (`memory/contextualize.rs`).
- Criterion bench at 10k smoke / 1M full (`benches/million_memory.rs`).
- Concurrency-limited ingestion via `ingest_semaphore` (`commands/ingest.rs`).

Concrete bottlenecks confirmed in code that **must be fixed for 1M+ to
be optimal**:

1. **Per-row INSERT loops on ingest.** `add_memory_inner` and the
   ingest loop in `commands/ingest.rs` insert one chunk at a time, each
   under its own `state.memory_store.lock()` and its own implicit
   transaction. At 1k chunks per document this is 1k WAL fsync rounds.
2. **PRAGMAs left at SQLite defaults.** No `cache_size`,
   `mmap_size`, `temp_store=MEMORY`, `page_size`, `journal_size_limit`,
   `wal_autocheckpoint`, or `busy_timeout` are set today. Million-row
   write paths leave large gains on the table.
3. **`prepare_cached` is inconsistent** across `add`, `add_to_tier`,
   `set_embedding`, `update` — each call re-parses SQL.
4. **`get_all()` on the hot path.** `relevant_for`, `hybrid_search`,
   and `hybrid_search_rrf` call `get_all()` / `get_with_embeddings()`
   on every query — at 1M × 768-d f32 that loads ≈3 GB into memory per
   call before any ranking happens.
5. **No re-embed on content update.** `MemoryStore::update` snapshots
   the prior version but does not invalidate `embedding` or remove the
   row from the HNSW index when content changes — semantic retrieval
   silently drifts off the new content.
6. **HNSW save-every-50-ops** (`SAVE_INTERVAL = 50` in `ann_index.rs`)
   means a 1M bulk insert performs ~20k full index serialisations.
7. **Single global ANN.** Dimension drift from changing the embedder
   model permanently degrades to brute-force until process restart;
   there is no per-(model_id, dim) ANN.
8. **No bulk `add_many` / `update_many` / `delete_many`**, no streaming
   / cursor reads, no per-op metrics, no quantization, no tombstone
   compaction.
9. **FTS5 + KG indexes** need a verification pass at 1M rows
   (`brain_kg_neighbors` traversal cost, partial indexes for
   `embedding IS NOT NULL` and `valid_to IS NULL`).

State-of-the-art landmarks consulted (through May 2026): HNSW
(Malkov 2018), DiskANN / SPANN (Microsoft 2019, 2021), Matryoshka
embeddings (Kusupati 2022), int8 / binary quantization (Cohere 2024),
late chunking (Günther 2024), Contextual Retrieval (Anthropic 2024),
RRF fusion (Cormack 2009 — already shipped). usearch ≥ 2.x supports
memory-mapped indexes and `i8` / `b1` quantization that we have not
opted into.

The 15 chunks below are deliberately ordered: foundations first,
then bulk CRUD, then embedding/ANN correctness at scale, then
indexes / KG / sharding. Each chunk ships behind the Full CI Gate
(`npx vitest run && npx vue-tsc --noEmit && cargo clippy -D warnings && cargo test`)
and extends `benches/million_memory.rs` where relevant.

### A. Foundations and measurement

| ID | Title | Status |
| -- | ----- | ------ |
| 41.2 | Per-op latency metrics for memory CRUD + retrieval | not-started |
| 41.3 | Extend million-memory bench to write/update/delete mix | not-started |

**41.2 — Per-op latency metrics.**
Add a lightweight `MemoryMetrics` (rolling p50/p95/p99 histograms via
the `hdrhistogram` crate or a simple bucketed counter) for `add`,
`update`, `delete`, `set_embedding`, `hybrid_search`, `hybrid_search_rrf`,
ANN add/remove. Surface a `metrics` block in the `brain_health` /
`/health` JSON and a Tauri command `get_memory_metrics`. Tests assert
counters move on real ops.
Success: every CRUD/search call updates the right histogram; metrics
JSON visible in MCP `brain_health`.

**41.3 — Bench writes, updates, deletes, mix.**
`benches/million_memory.rs` currently measures HNSW retrieval and
capacity eviction only. Add three new sections at scales 10k / 100k /
1M (gated `bench-million`): `bulk_insert`, `bulk_update_reembed`,
`bulk_delete`, plus a `mixed_crud_workload` that interleaves at
80/10/10 ratio. Persist results in the existing JSON report under
new keys; document them in `docs/benchmarking.md`.
Success: report file gains the three new sections; smoke run < 90 s.

### B. Bulk CRUD

| ID | Title | Status |
| -- | ----- | ------ |
| 41.5 | Cursor-based reads — kill `get_all()` from hot search paths | not-started |

> Chunk 41.4 (transactional `add_many` bulk insert API) shipped
> 2026-05-07 — see `rules/completion-log.md`. The Phase 41 1M
> write/read targets pass on top of 41.1 + 41.4 alone.

**41.5 — Cursor reads on hot path.**
Replace `get_all()` / `get_with_embeddings()` calls in `relevant_for`,
`hybrid_search`, `hybrid_search_rrf`, and `find_duplicate` with
SQL-level candidate selection: an FTS5 / index-driven candidate set
(top N by tier + recency + tag/keyword match) followed by ANN candidate
union, then in-Rust scoring on the union only. Keep an
`iter_with_embeddings()` streaming iterator using
`rusqlite::Statement::query_map` for paths that legitimately need a
full scan (rebuild, decay). Add tests asserting hybrid-search results
are stable vs the old code path on a 5k corpus, and a bench-bound
assertion that peak Rust heap is bounded vs corpus size.
Success: `hybrid_search_rrf` p95 at 1M memories < 100 ms; resident
heap during search < 100 MiB.

### C. Embedding correctness and lifecycle

| ID | Title | Status |
| -- | ----- | ------ |
| 41.6 | Re-embed on content update + ANN tombstone | not-started |
| 41.7 | Embedding worker concurrency, rate limiting, pause-on-429 | not-started |
| 41.8 | Multi-model embedding columns + per-(model, dim) ANN | not-started |

**41.6 — Re-embed on content update.**
In `MemoryStore::update`, after the `memory_versions` snapshot, if
`content` changed: clear `embedding` to NULL, remove the row from the
ANN index, and `enqueue` it in `pending_embeddings`. The embedding
worker will repopulate. Tests assert that updating content drops the
old vector from ANN and that semantic search reflects the new content
within one worker tick.
Success: round-trip `update → wait → search` returns the new content
for the same id; old embedding never leaks into `hybrid_search_rrf`.

**41.7 — Worker concurrency + rate limiting.**
Tune `embedding_queue` worker: configurable `BATCH_SIZE` (8/32/128) per
provider, `tokio::Semaphore` cap on concurrent provider calls, treat
HTTP 429 / Ollama "model busy" as a soft pause that scales backoff
without incrementing failures, and a hard fail counter that surfaces
in `brain_health`. Add a graceful shutdown path. Tests cover backoff
math + pause-vs-fail distinction with a fake brain.
Success: ingesting 100k chunks against rate-limited brain finishes
without permanent failures; `brain_health` shows the pause clearly.

**41.8 — Multi-model embeddings (V16 schema).**
Add columns `embedding_model_id TEXT`, `embedding_dim INTEGER`, and a
`memory_embeddings` side table keyed by `(memory_id, model_id)` so a
single memory can carry multiple embeddings (e.g. 768-d nomic + 1024-d
bge-m3). Replace the single `AnnIndex` cell with an `AnnRegistry`
keyed by `(model_id, dim)`, each persisted as
`vectors_<model_id>.usearch`. Provide a Tauri command + numbered seed
migration to backfill the active model id into existing rows.
Success: switching the embedder model no longer triggers a full
brute-force fallback; both embeddings co-exist; per-model recall is
measurable in the bench.

### D. ANN / HNSW at scale

| ID | Title | Status |
| -- | ----- | ------ |
| 41.9 | usearch quantization (i8) with optional binary fallback | not-started |
| 41.10 | Memory-mapped HNSW + debounced async flush | not-started |
| 41.11 | ANN compaction / tombstone GC job | not-started |

**41.9 — Quantization.**
Add `EmbeddingQuantization` setting (`f32` default, `i8`, `b1`)
plumbed into `IndexOptions.quantization`. Bench: build + query
recall@10 vs f32 baseline at 100k and 1M. Persist the quantization
choice next to the index so reloads are correct. Tests include a
recall regression budget.
Success: i8 mode shows < 1% recall drop and ≈ 4× memory reduction at
1M; b1 available behind a setting with documented recall trade-off.

**41.10 — Memory-mapped HNSW + async flush.**
Use the usearch memory-mapped load path when present, so the hot graph
data stays out of process RAM. Replace `SAVE_INTERVAL = 50` with a
debounced flush: dirty counter triggers a `tokio::spawn` task that
flushes after ≥ N ops or T seconds, whichever first; the path is
debounced so concurrent flushes coalesce. Tests assert no double-flush
race and that crash mid-flush is recoverable via the rebuild fallback.
Success: 1M-row bulk insert flushes ≤ 50 times; resident set size
during query workloads visibly drops.

**41.11 — Compaction job.**
Add a `compact_ann()` operation that rebuilds the HNSW from
`memories WHERE embedding IS NOT NULL AND tier='long'`, atomically
swaps the file, and runs only when fragmentation
(`removed_count / size` from usearch stats) crosses a threshold.
Wire to the maintenance scheduler with a daily cadence and expose a
manual Tauri command. Tests cover the swap atomicity.
Success: after a churn workload (50% updates + 20% deletes on 100k),
post-compaction recall ≥ recall on a freshly built index of equivalent
content; compaction is idempotent.

### E. SQL indexes, FTS5, knowledge graph

| ID | Title | Status |
| -- | ----- | ------ |
| 41.12 | Targeted indexes + `PRAGMA optimize` schedule | not-started |
| 41.13 | Bounded KG traversal + cache for `brain_kg_neighbors` | not-started |

**41.12 — Targeted indexes.**
Add partial indexes:
`CREATE INDEX IF NOT EXISTS idx_memories_long_embedded ON memories(id) WHERE tier='long' AND embedding IS NOT NULL;`
`CREATE INDEX IF NOT EXISTS idx_memories_active ON memories(id) WHERE valid_to IS NULL;`
plus `idx_memories_session_recent(session_id, created_at DESC)` and
`idx_pending_due(next_retry_at)`. Run `PRAGMA optimize` on app open
and a periodic `ANALYZE` after the first 10k mutations. Bench
relevant queries before/after.
Success: explain-query-plan confirms the new indexes are used;
audit / decay / `brain_health` queries drop in latency on 1M corpus.

**41.13 — Bounded KG traversal + cache.**
`brain_kg_neighbors` and `graph_rag` walks must be bounded by hop
count and per-hop fan-out, with an LRU cache (size from settings)
keyed by `(seed_id, depth, rel_filter)`. Invalidate cache entries
on `add_edge` / `delete` for any edge endpoint in the cached set
(broadcast via a small event channel). Tests cover cache hit / miss
and invalidation, plus a benchmark at 5M edges.
Success: KG neighbors p95 at 5M edges < 50 ms; cache hit ratio > 60%
on a typical chat session.

### F. Sharding, snapshots, durability

| ID | Title | Status |
| -- | ----- | ------ |
| 41.14 | Optional time-bucketed shards for long-tier memories | not-started |
| 41.15 | Online snapshot + atomic SQLite + ANN backup | not-started |

**41.14 — Time-bucketed shards.**
Behind a feature flag, shard the long-tier table by `created_at`
quarter into attached SQLite databases (`memory.long_2026q2.db`, etc.)
joined via SQLite's `ATTACH DATABASE`. Reads stay transparent (a view
union); writes route on `created_at`. ANN registry already keys by
model — extend it to also key by shard for compaction granularity.
Tests cover query parity vs unsharded and shard rotation at the
quarter boundary.
Success: a 5M-row corpus splits across ≥ 4 shards; per-shard
backup / vacuum is independently runnable; query latency unchanged.

**41.15 — Online snapshot.**
Add `snapshot(dest_dir)` that performs `VACUUM INTO` for the SQLite
file and a usearch `save` of every ANN index, then writes a
`snapshot.json` manifest listing files + checksums. Concurrent CRUD
must continue during snapshot. Restore command verifies checksums
and atomically swaps `data_dir`. Tests cover crash mid-snapshot.
Success: snapshot of a 1M-memory store completes in < 60 s without
blocking new writes; restore round-trips bit-exact.

---

## Phase 42 — Database strategy: offline mobile + future hive (May 2026)

> **Question.** Will SQLite become the bottleneck for TerranSoul as it
> scales to (a) 1M+ memories per device, (b) iOS/Android offline mode,
> and (c) a future "hive" with cross-device knowledge sharing and
> distributed jobs? **Verdict.** No, *as the local engine* — but it is
> not a clustered server. The optimal shape is **two-layer storage**:
> a tuned embedded local engine on every device (Phase 41) **plus** a
> separate distributed "hive layer" reached only when a network is
> available. SQLite stays authoritative offline; the hive layer is
> additive, never required.

### Audit (state through May 2026)

What is already wired:

- `StorageBackend` trait (`src-tauri/src/memory/backend.rs`) abstracts
  SQLite (default), PostgreSQL (`feature = postgres`, `sqlx`),
  SQL Server (`feature = mssql`, `tiberius 0.12`), Cassandra
  (`feature = cassandra`, `scylla 0.14`).
- `StorageConfig` enum selects backend at startup; default is
  `Sqlite { data_dir: None }`.
- All non-SQLite backends are real implementations (real drivers,
  real migrations, real CRUD). They are **not** featured-on by
  default — release builds are SQLite-only.
- HNSW (`usearch`), FTS5, RRF fusion, knowledge graph (`memory_edges`),
  versioning, contextual retrieval, eviction, and contradiction
  detection live in modules that operate **on the SQLite connection**
  (`hybrid_search`, `hybrid_search_rrf`, edges, FTS triggers, ANN
  bridge). The trait surface only exposes generic CRUD + a default
  `hybrid_search_rrf` that delegates to `hybrid_search`. So a
  Postgres / MSSQL / Cassandra deployment today **loses** RRF, FTS5
  triggers, KG queries, ANN, and contextual retrieval — they degrade
  to plain LIKE search until each is ported.
- CRDT primitives (`src-tauri/src/sync/`): `lww_register`, `or_set`,
  `append_log`, plus an `HLC` Lamport-style hybrid-logical clock and
  a `SyncOp` envelope. These are real and tested.
- `link/` ships a single peer-to-peer `LinkManager` over QUIC primary
  + WebSocket fallback, with `LinkPeer` / `LinkMessage` / status
  tracking. **One** peer at a time — there is no mesh, no peer
  discovery beyond mDNS (`lan-share-mdns` feature), no relay.
- Mobile: `#[cfg_attr(mobile, tauri::mobile_entry_point)]` is wired
  in `src-tauri/src/lib.rs`, plus `commands/lan.rs` for pairing UI.
  iOS/Android do compile in principle. SQLite via `rusqlite` with
  `bundled-full` works on both (default `bundled-full` is on per
  Cargo.toml).
- `usearch` ships native code that historically had spotty mobile
  builds; `native-ann` is desktop-default but should be feature-gated
  on mobile or replaced with a pure-Rust fallback already in
  `ann_index.rs`.
- "Hive" / federation: nothing yet. No replication topology, no
  authority model, no per-memory ACL, no signed CRDT ops, no relay
  server, no job-distribution scheduler. Existing CRDT containers
  cover *individual data structures* (a register, a set, an append
  log) but the memory store itself is **not yet a CRDT** — its rows
  carry `updated_at` + `origin_device` columns but no merge function
  is wired.

### Will SQLite be the bottleneck?

Three honest answers, by axis:

1. **Single-device CRUD at 1M+ rows.** No, after Phase 41 lands.
   SQLite + WAL + tuned PRAGMAs + HNSW comfortably serves the
   read/write rate of a personal companion (think 10s of writes/s,
   100s of reads/s). The companion will be CPU/embedding-bound long
   before it is SQLite-bound. The 2026 ecosystem (DuckDB, libSQL,
   sqlite-vec) confirms SQLite-class engines at billion-row scale.
2. **Offline mobile.** No — SQLite is the *correct* engine on iOS and
   Android. WAL works (with the iOS file-locking caveat), and
   `bundled-full` avoids platform sqlite quirks. The risks are not
   SQLite; they are (a) the `usearch` C++ build on mobile, and (b)
   embedding-model availability on-device.
3. **Hive / multi-device knowledge sharing + job distribution.**
   **Yes** — SQLite alone is the wrong shape here. SQLite is a
   single-writer embedded engine; multi-leader replication and global
   ordering are out of scope by design. The fix is *not* to replace
   SQLite; it is to add a second layer above it.

### Recommended database posture (final)

- **Local layer (every device, including mobile):** Tuned SQLite
  remains the source of truth. Phase 41 makes it optimal at 1M+.
  A pure-Rust ANN fallback ships on mobile when `usearch` is unsafe.
- **Sync layer (between a single user's own devices):** Promote the
  memory store to a proper CRDT. Each row carries `(hlc, origin_device)`
  already; add an LWW merge with vector-clock conflict markers and
  ship the diff over the existing QUIC/WS link. No server required.
- **Hive layer (opt-in, multi-user federation + jobs):** Add a thin
  hosted *or self-hosted* "TerranSoul Hive" service that speaks
  CRDT-op exchange + signed knowledge bundles, stores its replica
  copy in **Postgres + pgvector** (the existing `postgres` backend
  finishes RRF/FTS/KG parity), and fronts a job queue. The local app
  never depends on it; it federates only when configured. Cassandra
  / MSSQL stay as deployment options for self-hosters.
- **Vector index choice (long term):** Keep `usearch` HNSW on the
  local layer. On the hive layer use pgvector's HNSW; reject Qdrant /
  Milvus / Pinecone for the local app per the existing decision in
  [docs/brain-advanced-design.md](docs/brain-advanced-design.md) row 18 — the offline-first
  promise rules out a separate vector service. State of the art
  through May 2026: pgvector ≥ 0.7 (HNSW + binary quantization) is
  on par with dedicated vector DBs for ≤ ~50M-row corpora.

The 12 chunks below sequence the work. They land **after** Phase 41.

### A. Mobile-safe local engine

| ID | Title | Status |
| -- | ----- | ------ |
| 42.1 | Mobile feature gates: pure-Rust ANN fallback for iOS/Android | not-started |
| 42.2 | iOS/Android SQLite + WAL hardening + integration tests | not-started |

**42.1 — Mobile ANN fallback.**
Make `native-ann` desktop-only. On mobile (`cfg(target_os = "ios")` /
`"android"`) compile the existing pure-Rust linear path in
`memory/ann_index.rs` and ship a faster fallback (e.g. `instant-distance`
or a hand-rolled IVF + i8 quantized scan) bounded to 200k vectors —
above that, hybrid search degrades to FTS5 + keyword. Tests run on
both `--no-default-features --features desktop` and a synthesized
`mobile` profile.
Success: `cargo build --target aarch64-apple-ios` and
`cargo build --target aarch64-linux-android` succeed without
`usearch`; recall regression budget documented.

**42.2 — Mobile SQLite hardening.**
Verify `rusqlite` `bundled-full` ships on iOS/Android. Apply the
Phase 41 PRAGMAs only on platforms where they are safe (iOS WAL has
a known background-throttling caveat — set `journal_mode=WAL2` or
fall back to `TRUNCATE` when WAL is unsafe). Add a smoke
integration test that opens / migrates / writes / reads on each
mobile target via Tauri's mobile harness.
Success: green CI on mobile targets; data files round-trip between
desktop and mobile.

### B. Memory CRDT (own-device sync)

| ID | Title | Status |
| -- | ----- | ------ |
| 42.3 | Memory rows as LWW CRDT with vector-clock conflicts | not-started |
| 42.4 | KG edges as 2P-Set CRDT with tombstone GC | not-started |
| 42.5 | Op-log replication over `LinkManager` (QUIC + WS) | not-started |

**42.3 — Memory LWW CRDT.**
Wrap memory writes in `SyncOp` envelopes (already defined in
`sync/mod.rs`). On apply, last-write-wins by `(hlc, origin_device)`
with the loser archived into `memory_versions`. Add a
`memory_conflicts` row when two non-comparable HLCs both touched
`content` so the user can resolve. Tests cover concurrent updates
on two simulated devices converging.
Success: simulated two-device divergence converges to the same
state regardless of merge order; no data loss; conflicts surface in
BrainView.

**42.4 — KG edges 2P-Set CRDT.**
Edges become an Observed-Remove-Set keyed by
`(src_id, dst_id, rel_type)` with `created_at` and `tombstone_at`.
The existing `valid_to` column is the natural tombstone. Compaction
removes long-tombstoned rows during the maintenance scheduler.
Success: edge add / remove / re-add converges across devices;
storage growth is bounded by GC.

**42.5 — Op-log replication.**
Build an `append_log` per CRDT keyed by site id, stream missing ops
to the linked peer over `LinkManager.send`, replay on receive. Use
the existing `mdns-sd` discovery (`lan-share-mdns` feature) to find
a paired device. Multi-peer fanout is *not* in scope here — pairwise
sync is sufficient for own-device.
Success: a desktop and a mobile device on the same LAN converge a
1k-op corpus in < 5 s; resume after disconnect picks up cleanly.

### C. Distributed backend parity (hive prerequisite)

| ID | Title | Status |
| -- | ----- | ------ |
| 42.6 | Postgres backend: RRF, FTS, KG, contextual retrieval parity | not-started |
| 42.7 | Postgres backend: pgvector HNSW + ANN parity + bench | not-started |
| 42.8 | Backend test matrix in CI (SQLite + Postgres) | not-started |

**42.6 — Postgres parity.**
In `memory/postgres.rs`, port: FTS via `tsvector` GIN index +
trigger, RRF in a single SQL CTE (Postgres handles `RANK()` /
window functions natively), KG via `memory_edges` table with
`recursive` CTE traversal, contextual retrieval prefix on insert.
Update `StorageBackend` to make `hybrid_search_rrf` non-default for
backends that override.
Success: integration tests asserting parity with SQLite on a fixed
corpus; hybrid search results identical (modulo ordering ties) on
both backends.

**42.7 — pgvector ANN.**
Add `vector(768)` column with HNSW index. Implement `vector_search`
and `find_duplicate` server-side. Add a benchmark mirroring
`benches/million_memory.rs` against a real Postgres in CI (Docker
service container). Quantization parity with SQLite path.
Success: 1M-row pgvector recall@10 within 1% of usearch baseline;
p95 query < 50 ms on commodity hardware.

**42.8 — Backend matrix in CI.**
Run the memory test suite against SQLite and Postgres on every PR
that touches `memory/`. MSSQL and Cassandra stay opt-in / nightly
because their drivers are heavier; their suites must still pass on a
weekly schedule. Document the gap in
[docs/brain-advanced-design.md](docs/brain-advanced-design.md).
Success: `cargo test --features postgres` green in CI; matrix
documented.

### D. Hive layer (opt-in federation + jobs)

| ID | Title | Status |
| -- | ----- | ------ |
| 42.9 | Hive protocol spec + signed knowledge bundle format | not-started |
| 42.10 | Hive relay reference server (Tonic gRPC, Postgres-backed) | not-started |
| 42.11 | Job queue + capability gates for distributed work | not-started |
| 42.12 | Privacy / consent / per-memory ACL for hive sharing | not-started |

**42.9 — Hive protocol spec.**
Spec a CRDT-op exchange protocol with three message types: `BUNDLE`
(signed batch of memories + edges), `OP` (single CRDT op), `JOB`
(work item + capabilities required). Bundles are Ed25519-signed by
the originating device (we already have `identity/` for device
keys). Spec lives in `docs/hive-protocol.md` and is reviewed before
any code lands.
Success: protocol spec merged with worked examples; security review
documented.

**42.10 — Hive relay server.**
Stand up a reference server in `crates/hive-relay/` (new workspace
member): Tonic gRPC, Postgres + pgvector storage, accepts signed
bundles, routes ops, holds the job queue. Distributed in the repo
under MIT — anyone can self-host. The desktop app gets a
`hive_url` setting; without it, hive is invisible.
Success: a reference docker-compose spin-up that two TerranSoul
clients can join; bundle round-trip works; relay is stateless beyond
its Postgres.

**42.11 — Job distribution.**
Reuse the existing capability gates / orchestrator
(`src-tauri/src/orchestrator/`). Jobs declare required capabilities
(brain mode, GPU, embedding model). Workers pull from the relay
queue; results return as `BUNDLE`s. Local-only "self-jobs" continue
to bypass the relay.
Success: a synthetic embed-1k-chunks job dispatched from one device
completes on another and the bundle reconciles into the originator's
store.

**42.12 — Privacy + per-memory ACL.**
Every memory carries a `share_scope` enum: `private` (default,
never leaves device), `paired` (own-device sync only), `hive`
(uploadable). A consent UI + policy engine guards every outbound
bundle. Settings expose a default scope per `cognitive_kind`. Tests
cover redaction (a `private` memory must never appear in a hive
bundle, even by accident). Update README +
[docs/brain-advanced-design.md](docs/brain-advanced-design.md).
Success: integration test asserts `private` rows never appear in
outbound bundles; consent UX captured in `qa-screenshots/`.

---
