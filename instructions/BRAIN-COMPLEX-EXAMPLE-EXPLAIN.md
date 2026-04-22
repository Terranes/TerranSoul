# Brain + RAG — Quick Technical Reference

> **TerranSoul v0.1**
> Last updated: 2026-04-22
>
> **This file is the short, code-anchored cheat sheet.**
> For the full architecture (tier model, 6-signal hybrid score, decay maths,
> knowledge-graph vision, Obsidian export, scaling roadmap, framework
> comparison) see [`docs/brain-advanced-design.md`](../docs/brain-advanced-design.md).
> For the end-user walkthrough see [`BRAIN-COMPLEX-EXAMPLE.md`](BRAIN-COMPLEX-EXAMPLE.md).

---

## Table of Contents

1. [System map (one diagram)](#system-map-one-diagram)
2. [Code map — where things live](#code-map--where-things-live)
3. [Tauri commands at a glance](#tauri-commands-at-a-glance)
4. [Schema cheat sheet (current = V4)](#schema-cheat-sheet-current--v4)
5. [Hybrid search formula](#hybrid-search-formula)
6. [Decay & GC formula](#decay--gc-formula)
7. [Ingest pipeline (URL / file / crawl)](#ingest-pipeline-url--file--crawl)
8. [Multi-source knowledge management](#multi-source-knowledge-management)
9. [Debugging recipes (sqlite3)](#debugging-recipes-sqlite3)
10. [Test suites that cover this code](#test-suites-that-cover-this-code)
11. [Design validation — comparison with open-source systems](#design-validation--comparison-with-open-source-systems)
12. [Code paths — end-to-end flow](#code-paths--end-to-end-flow)
13. [FAQ](#faq)
14. [Distributed Storage Backends](#distributed-storage-backends)

---

## System map (one diagram)

```
Vue 3 UI (WebView)                         Rust Core (Tauri)
─────────────────                          ─────────────────────────────────
ChatView ─────────┐                        commands/chat.rs
MemoryView ───────┤  invoke / events       commands/memory.rs
BrainSetupView ───┤ ─────────────────────► commands/brain.rs
TasksPanel ───────┤                        commands/ingest.rs
QuestPanel ───────┘                        commands/quest.rs
                                                   │
                                                   ▼
                                           brain/  ─── OllamaAgent
                                                       OpenAiClient
                                                       FreeProvider (Pollinations)
                                                       ProviderRotator
                                                       model_recommender (RAM-based)
                                                   │
                                                   ▼
                                           memory/ ─── MemoryStore  (SQLite WAL)
                                                       brain_memory  (LLM ops)
                                                       migrations    (V1…V4)
                                                   │
                                                   ▼
                                           %APPDATA%/com.terransoul.app/
                                              memory.db, memory.db.bak,
                                              brain.json, settings.json
```

---

## Code map — where things live

| Concern | File | Notes |
|---|---|---|
| Schema migrations | `src-tauri/src/memory/migrations.rs` | append-only `MIGRATIONS` list, V1→V4 today; `migrate_to_latest`, `downgrade_to`, `migration_status` |
| Memory CRUD + search | `src-tauri/src/memory/store.rs` | `MemoryStore::add`, `add_to_tier`, `search`, `vector_search`, `hybrid_search`, `find_duplicate`, `apply_decay`, `gc_decayed`, `evict_short_term`, `stats` |
| LLM-driven memory ops | `src-tauri/src/memory/brain_memory.rs` | extract / summarize / semantic-rank (legacy LLM ranker) |
| Brain providers | `src-tauri/src/brain/*.rs` | `OllamaAgent` (chat + embed), `OpenAiClient`, `FreeProvider`, `ProviderRotator`, `model_recommender`, `system_info` |
| URL/PDF ingest | `src-tauri/src/commands/ingest.rs` | `ingest_document`, `crawl_website_with_progress`, `extract_pdf_text`, `extract_text_from_html`, `chunk_text`, checkpoint/resume |
| Memory commands | `src-tauri/src/commands/memory.rs` | `add_memory`, `get_memories`, `search_memories`, `update_memory`, `delete_memory`, `semantic_search_memories`, `hybrid_search_memories`, `backfill_embeddings`, `get_schema_info` |
| Frontend stores | `src/stores/{brain,memory,conversation,skill-tree}.ts` | Pinia, mirror Rust state |
| Quest auto-detection | `src/stores/skill-tree.ts` | predicates over store state — `brain-online`, `rag-knowledge`, `sage-library`, … |

---

## Tauri commands at a glance

| Command | Args | Returns | Purpose |
|---|---|---|---|
| `set_brain_mode` | `mode`, `config?` | `()` | choose `free_api` / `paid_api` / `local_ollama` |
| `add_memory` | `content, tags, importance, memoryType` | `MemoryEntry` | insert + best-effort embed |
| `get_memories` | — | `Vec<MemoryEntry>` | full list (no embeddings on the wire) |
| `search_memories` | `query` | `Vec<MemoryEntry>` | SQL `LIKE` keyword search |
| `semantic_search_memories` | `query, limit` | `Vec<MemoryEntry>` | pure vector cosine |
| `hybrid_search_memories` | `query, limit` | `Vec<MemoryEntry>` | 6-signal score (used for RAG injection) |
| `update_memory` | `id, fields…` | `MemoryEntry` | partial update |
| `delete_memory` | `id` | `()` | hard delete |
| `backfill_embeddings` | — | `i64` | embed every NULL-embedding row |
| `get_schema_info` | — | `SchemaInfo` | version, totals, column descriptions |
| `apply_memory_decay` | — | `usize` | run `apply_decay` once |
| `gc_memories` | — | `usize` | delete decayed + unimportant |
| `ingest_document` | `source, tags?, importance?` | `IngestStartResult` | URL / `crawl:URL` / file path |
| `cancel_ingest_task` | `taskId` | `()` | cancel running ingest |
| `resume_ingest_task` | `taskId` | `()` | resume from checkpoint |
| `get_all_tasks` | — | `Vec<TaskSnapshot>` | for the Tasks panel |

---

## Schema cheat sheet (current = V4)

```sql
CREATE TABLE memories (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    content       TEXT    NOT NULL,
    tags          TEXT    NOT NULL DEFAULT '',
    importance    INTEGER NOT NULL DEFAULT 3,    -- 1..5
    memory_type   TEXT    NOT NULL DEFAULT 'fact', -- fact|preference|context|summary
    created_at    INTEGER NOT NULL,              -- Unix ms
    last_accessed INTEGER,
    access_count  INTEGER NOT NULL DEFAULT 0,
    embedding     BLOB,                          -- 768 × f32 LE  (V2)
    source_url    TEXT,                          -- (V3)
    source_hash   TEXT,                          -- (V3) SHA-256 of source content
    expires_at    INTEGER,                       -- (V3) optional TTL ms
    tier          TEXT    NOT NULL DEFAULT 'long',  -- short|working|long  (V4)
    decay_score   REAL    NOT NULL DEFAULT 1.0,     -- 0.01..1.0          (V4)
    session_id    TEXT,                              -- (V4)
    parent_id     INTEGER REFERENCES memories(id),   -- (V4) summary parent
    token_count   INTEGER NOT NULL DEFAULT 0         -- (V4) ~chars/4
);
-- indices: importance DESC, created_at DESC, source_hash, tier, session_id, decay_score DESC
```

> **Roadmap.** V5 (`category` column), V6 (`memory_edges` table), V7
> (`obsidian_path`, `last_exported`) are documented in
> `docs/brain-advanced-design.md` §8.

---

## Hybrid search formula

`MemoryStore::hybrid_search(query, query_embedding, limit)` —
`src-tauri/src/memory/store.rs:478`.

```
score = 0.40 · cosine(emb_query, emb_entry)        # vector
      + 0.20 · keyword_hits / |query_words|        # BM25-lite
      + 0.15 · exp(-age_hours / 24)                # recency, 24h half-life
      + 0.10 · importance / 5                      # 1..5 → 0.2..1.0
      + 0.10 · decay_score                         # 0.01..1.0
      + 0.05 · tier_boost                          # working=1.0, long=0.5, short=0.3
```

If `query_embedding` is `None` (e.g. Ollama unreachable), the vector term is
zero and the system degrades gracefully to the other 5 signals (~60% RAG
quality, see design §10).

Side effect: every entry returned has `last_accessed` set to `now` and
`access_count += 1`. That's what feeds the §15 GC heuristic.

---

## Decay & GC formula

`MemoryStore::apply_decay()` — `store.rs:581`.

```
decay_score *= 0.95 ^ (hours_since_last_access / 168)
floor       = 0.01
```

Half-life ≈ 2 weeks of non-access. Only `tier='long'` rows are touched.

`MemoryStore::gc_decayed(threshold)` — default threshold `0.05`:

```sql
DELETE FROM memories
 WHERE tier = 'long'
   AND decay_score < ?1
   AND importance  <= 2;
```

So: **important memories survive forever**; trivia gets pruned after a few
months of zero RAG hits.

---

## Ingest pipeline (URL / file / crawl)

`commands/ingest.rs::run_ingest_task`:

```
source string
   │
   ├─ "crawl:URL"        → crawl_website_with_progress (BFS, depth ≤ 2,
   │                        max_pages cap, validate_url blocks private CIDR)
   ├─ "http(s)://…"      → fetch_url → extract_text_from_html
   └─ filesystem path    → read_local_file → (if .pdf) extract_pdf_text
                                      ↓
                              chunk_text(text, target=800, overlap=100)
                                      ↓
                       for each chunk:
                           MemoryStore::add(NewMemory{ content, tags+chunk-i/N,
                                                       importance, type: Fact })
                           OllamaAgent::embed_text → set_embedding
                                      ↓
                         emit `task-progress` events every chunk
                                      ↓
                  IngestCheckpoint persisted on cancel / 30-min timeout
                  → resume_ingest_task continues from next_chunk_index
```

Safety:

- `validate_url` rejects loopback, private CIDRs, `file://`, non-http(s) schemes.
- `extract_text_from_html` strips `<script>`, `<style>`, `<noscript>`, decodes
  entities, collapses whitespace.
- `chunk_text` produces overlapping windows so the embedding of an idea is
  rarely cut in half.

---

## Multi-source knowledge management

The V3 columns enable the four mechanisms documented in `brain-advanced-design.md`
§12:

1. **Source-hash change detection** — recompute SHA-256 on every re-crawl,
   compare to stored `source_hash`; if different → drop & re-embed.
2. **TTL expiry** — `expires_at` lets time-bounded content (court calendars,
   temporary policies) self-delete:
   `DELETE FROM memories WHERE expires_at IS NOT NULL AND expires_at < now;`
3. **Access-count decay** — `access_count = 0 ∧ created_at < now − 90d` is the
   GC short-list of "stored but never useful" memories.
4. **LLM conflict resolution** — when a new chunk's embedding lands in the
   "similar but not duplicate" band (cosine `0.90 .. 0.97`), the active LLM is
   asked which version supersedes the other; the loser is demoted (importance
   ↓, decay ↓), the winner inserted, optionally tagged `rel:supersedes:<id>`
   (will become a typed edge once V6 ships).

The walkthrough in `BRAIN-COMPLEX-EXAMPLE.md` §8 demonstrates mechanism (1)
and (4) end-to-end on a Vietnamese-statute amendment.

---

## Debugging recipes (sqlite3)

DB file: `%APPDATA%/com.terransoul.app/memory.db` (Windows) /
`~/Library/Application Support/com.terransoul.app/memory.db` (macOS) /
`~/.local/share/com.terransoul.app/memory.db` (Linux).

```sql
-- Health
PRAGMA integrity_check;     -- expect "ok"
PRAGMA journal_mode;        -- expect "wal"

-- Embedding coverage
SELECT COUNT(*) total,
       COUNT(embedding) embedded,
       COUNT(*) - COUNT(embedding) unembedded
  FROM memories;

-- RAG hit-list (most retrieved)
SELECT id, substr(content,1,60) snippet, access_count, last_accessed
  FROM memories ORDER BY access_count DESC LIMIT 10;

-- Never-retrieved candidates (GC short-list)
SELECT id, substr(content,1,60), created_at
  FROM memories
 WHERE access_count = 0
   AND created_at < strftime('%s','now') * 1000 - 7776000000  -- 90d
 ORDER BY created_at ASC LIMIT 20;

-- Embedding size sanity (768 × 4 = 3072 bytes)
SELECT id, length(embedding) bytes, length(embedding)/4 dims
  FROM memories WHERE embedding IS NOT NULL LIMIT 5;

-- Per-tier distribution
SELECT tier, memory_type, COUNT(*),
       AVG(importance), AVG(decay_score)
  FROM memories GROUP BY tier, memory_type;

-- Migration history
SELECT version, description,
       datetime(applied_at/1000,'unixepoch','localtime') AS applied
  FROM schema_version ORDER BY version;

-- Find by source URL (re-sync helper)
SELECT id, source_hash, length(content) FROM memories
 WHERE source_url = 'http://thuvienphapluat.vn/.../BLDS-2015';
```

GUI: open `memory.db` in [DB Browser for SQLite](https://sqlitebrowser.org/)
or the VS Code `qwtel.sqlite-viewer` extension.

---

## Test suites that cover this code

```
src-tauri/src/memory/migrations.rs   # 8 tests — V0→V4 round-trip, idempotency
src-tauri/src/memory/store.rs        # ~60 tests — CRUD, hybrid_search, vector_search,
                                     #             find_duplicate, apply_decay,
                                     #             gc_decayed, evict_short_term, stats,
                                     #             find_by_source_hash, delete_by_source_url,
                                     #             delete_expired
src-tauri/src/memory/brain_memory.rs # extract / summarize / rank
src-tauri/src/commands/ingest.rs     # chunk_text, validate_url, extract_text_from_html,
                                     #             read_local_file, truncate_url,
                                     #             sha256_hash (dedup)
```

Verified: `cargo test --all-targets` → 570 pass, `npx vitest run` → 941 pass.

---

## Design validation — comparison with open-source systems

| Capability | TerranSoul | Obsidian | SiYuan | RAGFlow |
|---|---|---|---|---|
| **Storage** | SQLite (WAL) — single file | Plain `.md` files | SQLite + `.sy` JSON | PostgreSQL + ES + MinIO |
| **Offline-first** | ✅ | ✅ | ✅ | ❌ Server stack |
| **Vector search** | ✅ Pure-Rust cosine, 768-dim | Plugins only | Built-in (3.x) | ✅ Native ANN |
| **Hybrid (BM25 + vector + recency)** | ✅ 6-signal | ❌ | Vector + keyword | BM25 + vector + re-rank |
| **Tiered memory (short/working/long)** | ✅ V4 schema | ❌ Flat vault | ❌ Flat DB | ❌ |
| **Decay + GC** | ✅ Exponential, weekly half-life | ❌ | ❌ | ❌ |
| **Knowledge graph** | ✅ Cytoscape.js (entity-graph roadmap) | ✅ Backlinks | ✅ Backlinks | ✅ GraphRAG |
| **Multi-source ingest** | ✅ Checkpoint + resume | Manual / plugins | Built-in import | ✅ Connectors |
| **Source-hash staleness** | ✅ V3 schema | ❌ | ❌ | ✅ |
| **LLM conflict resolution** | ✅ Designed (§12.4) | ❌ | ❌ | Re-rank only |
| **Cross-device sync** | ✅ CRDT QUIC/WS | ✅ Obsidian Sync | ✅ WebDAV | N/A |
| **Bundled binary** | ✅ One Tauri exe | ✅ Electron | ✅ Electron/Tauri | ❌ Docker compose |

---

## Code paths — end-to-end flow

| User action | Frontend | Tauri command | Rust function |
|---|---|---|---|
| App loads, no brain | `App.vue` shows quest badge | — | — |
| Pick brain provider | `useBrainStore().setProvider` | `set_brain_mode` | `BrainStore::save` |
| Chat (no memory) | `useConversationStore().send` | `chat_stream_start` | `OllamaAgent::call_streaming` |
| Paste URL → Learn | MemoryView modal | `ingest_document` (crawl:) | `run_ingest_task` → `crawl_website_with_progress` → `chunk_text` → `MemoryStore::add` → `embed_text` |
| Drop PDF → Learn | same | `ingest_document` | `read_local_file` → `extract_pdf_text` → same path |
| Auto-RAG on chat | conversation store | `chat_stream_start` | `hybrid_search(query, emb, 5)` → `[LONG-TERM MEMORY]` block |
| Dedup during ingest | — | internal | `find_duplicate(emb, 0.97)` |
| Re-sync source | MemoryView | `ingest_document` | hash differs → conflict LLM → update + add |
| Decay sweep | `App.vue` mount | `apply_memory_decay` | `apply_decay` |
| GC sweep | scheduled | `gc_memories` | `gc_decayed(0.05)` |
| Memory graph | `MemoryGraph.vue` | `get_memories` | `get_all` |
| Toggle pet mode | `.mode-toggle-btn` | `toggle_window_mode` | window decorations off, always-on-top |

---

## Three streams contract — `send_message_stream`

Every chat turn the backend emits three event streams that together form
the contract the frontend depends on. Both the live Tauri runtime (`Wry`)
and the test `MockRuntime` go through the same emitter, so the contract
is verified identically in production and in CI.

| # | Tauri event | Payload | Source |
|---|---|---|---|
| 1 | `llm-chunk` (`done:false`) | `{ text, done }` clean text deltas | `commands/streaming.rs` — `StreamTagParser` strips `<anim>` |
| 2 | `llm-animation` | `AnimationCommand { emotion?, motion? }` | `<anim>{...}</anim>` JSON blocks parsed out of the stream |
| 3 | `llm-chunk` (`done:true`) | `{ text:"", done:true }` | end-of-stream sentinel after either SSE `[DONE]` or NDJSON `done:true` |

Frontend consumers:
* `src/views/ChatView.vue` and `src/views/PetOverlayView.vue` call
  `listen('llm-chunk', …)` and `listen('llm-animation', …)`.
* Both forward into `useStreamingStore.handleChunk` /
  `handleAnimation` (`src/stores/streaming.ts`).
* `useConversationStore` mirrors `streamingText` for live UI and
  finalizes the assistant `Message` on the `done:true` sentinel
  (`src/stores/conversation.ts:432-509`).

### Where each stream is verified

| Layer | Test | OS | Notes |
|---|---|---|---|
| Rust streams (no front-end) | `commands::streaming::tests::headless_linux::*` | **Linux** | Drives `run_chat_stream` against an in-process axum SSE mock LLM via Tauri's `MockRuntime`. Asserts on all 3 streams plus persistence. Gated `#[cfg(target_os = "linux")]` per project policy (Linux verifies streams; Windows verifies UI). |
| Pinia streaming store | `src/stores/streaming.test.ts` | cross-OS | Vitest covers `handleChunk` accumulation, `done:true` clearing, and `handleAnimation` emotion/motion routing. |
| End-to-end UI | `e2e/animation-flow.spec.ts` | **Windows** (and Linux CI) | Playwright drives the real browser against the live free LLM API; explicitly samples `conversation.streamingText` mid-stream (stream 1), asserts the final `Message` carries sentiment/motion (stream 2) and that `isStreaming` clears (stream 3). |

---

## FAQ

### What if Ollama is unreachable?

`OllamaAgent::embed_text` returns `None`. `hybrid_search` drops the vector
term and ranks on the other 5 signals — ~60% RAG quality. Chat can still go
through Free or Paid provider via `ProviderRotator`.

### How big can the store get?

| Memories | Embedding bytes | Working RAM | Search time |
|---|---|---|---|
| 1 k      | 3 MB     | ~50 MB  | <1 ms |
| 10 k     | 30 MB    | ~100 MB | ~2 ms |
| 100 k    | 300 MB   | ~500 MB | ~5 ms |
| 1 M      | 3 GB     | ~4 GB   | ~50 ms (linear scan) |

Beyond ~1 M, swap linear cosine for HNSW (`usearch` crate) — design §16 Phase 4.

---

## Distributed Storage Backends

TerranSoul supports four storage backends behind a unified `StorageBackend` trait
(`src-tauri/src/memory/backend.rs`). Backend selection is compile-time via Cargo
feature flags.

| Backend | Feature flag | Crate | Use case |
|---|---|---|---|
| **SQLite** | *(default)* | `rusqlite` | Local/offline, single device, zero-config |
| **PostgreSQL** | `postgres` | `sqlx` | Multi-device sync, server deployment |
| **SQL Server** | `mssql` | `tiberius` | Enterprise, Azure integration |
| **CassandraDB** | `cassandra` | `scylla` | High-write throughput, eventual consistency |

### Build with a distributed backend

```bash
# PostgreSQL
cargo build --features postgres

# SQL Server
cargo build --features mssql

# CassandraDB
cargo build --features cassandra

# Multiple backends
cargo build --features "postgres,cassandra"
```

### Configuration (`StorageConfig`)

```json
// SQLite (default — no config needed)
{ "backend": "sqlite", "data_dir": null }

// PostgreSQL
{ "backend": "postgres",
  "connection_string": "postgresql://user:pass@host:5432/terransoul",
  "max_connections": 10, "ssl": true }

// SQL Server
{ "backend": "sql_server",
  "connection_string": "Server=tcp:host,1433;Database=terransoul;...",
  "max_connections": 10 }

// CassandraDB
{ "backend": "cassandra",
  "contact_points": ["host1:9042", "host2:9042"],
  "keyspace": "terransoul", "replication_factor": 3 }
```

### Architecture notes

- The `StorageBackend` trait requires `Send` only (not `Sync`), because the backend
  is always held behind `Mutex<Box<dyn StorageBackend>>` in `AppState`.
- Distributed backends use `tokio::task::block_in_place` to bridge async drivers
  into the synchronous trait interface.
- All backends share the same V4 schema (17 columns, same indexes).
- Vector search is in-process (cosine similarity) for all backends. PostgreSQL
  has a future upgrade path to pgvector for server-side ANN.
- Cassandra limitations: no `LIKE` queries, no aggregation — falls back to
  application-side filtering. Best suited for write-heavy ingestion workloads.

### Why SQLite over Postgres / Qdrant?

Single-binary desktop app. SQLite = zero-config, one-file backup, WAL crash
safety, sub-5 ms search for any realistic personal corpus, no daemon.

### Search command differences?

| Command | Method | When to use |
|---|---|---|
| `search_memories` | SQL `LIKE` | exact keyword, instant, no brain needed |
| `semantic_search_memories` | Pure cosine | "find anything that means X" |
| `hybrid_search_memories` | 6-signal score | RAG injection (chat path) |

---

## Where to go next

- **Architecture deep dive** — [`docs/brain-advanced-design.md`](../docs/brain-advanced-design.md)
- **Visual walkthrough with screenshots** — [`BRAIN-COMPLEX-EXAMPLE.md`](BRAIN-COMPLEX-EXAMPLE.md)
- **Project rules** — [`rules/architecture-rules.md`](../rules/architecture-rules.md), [`rules/coding-standards.md`](../rules/coding-standards.md)
src-tauri/src/memory/migrations.rs   # 8 tests — V0→V4 round-trip, idempotency
src-tauri/src/memory/store.rs        # ~60 tests — CRUD, hybrid_search, vector_search,
                                     #             find_duplicate, apply_decay,
                                     #             gc_decayed, evict_short_term, stats
src-tauri/src/memory/brain_memory.rs # extract / summarize / rank
src-tauri/src/commands/ingest.rs     # chunk_text, validate_url, extract_text_from_html,
                                     #             read_local_file, truncate_url
```

Verified baseline on `copilot/validate-advanced-design-and-implement`:

```
$ cd src-tauri && cargo test --all-targets --quiet
test result: ok. 561 passed; 0 failed; 0 ignored
```

Run `npm run test` from the repo root for the 893 Vitest cases that exercise
the Pinia stores and the memory / chat flows.

---

## FAQ

### What if Ollama is unreachable?

`OllamaAgent::embed_text` returns `None`. `hybrid_search` then sees
`query_embedding=None`, drops the vector term, and ranks on the other 5
signals — the design (§17) calls this "60% RAG quality". Chat itself can
still go through the Free or Paid provider if either is configured as
fallback (`ProviderRotator`).

### How big can the store realistically get?

| Memories | Embedding bytes | Working RAM | Search time |
|---|---|---|---|
| 1 k      | 3 MB     | ~50 MB  | <1 ms |
| 10 k     | 30 MB    | ~100 MB | ~2 ms |
| 100 k    | 300 MB   | ~500 MB | ~5 ms |
| 1 M      | 3 GB     | ~4 GB   | ~50 ms (linear scan) |

Beyond ~1 M, swap the linear cosine scan for an HNSW index (`usearch` crate)
— this is design §16 Phase 4.

### Why SQLite over Postgres / Qdrant / pgvector?

Because TerranSoul is a **single-binary desktop app**. Embedded SQLite gives
us zero-config install, one-file backup, WAL crash safety, sub-5 ms search
for any realistic personal corpus, and no daemon to start. A server-class DB
would invert all of those trade-offs. See `brain-advanced-design.md` §9.

### How do I export everything?

```typescript
// CSV (quick & dirty)
import { invoke } from '@tauri-apps/api/core';
const all = await invoke('get_memories');
// → write to file via Tauri fs plugin

// Obsidian vault (planned, design §7 Layer 2)
await invoke('export_obsidian_vault', { destDir });
```

### What's the difference between the three search commands?

| Command | Method | When to use |
|---|---|---|
| `search_memories` | SQL `LIKE` | exact keyword lookup, instant, no brain needed |
| `semantic_search_memories` | Pure cosine | "find anything that means X" |
| `hybrid_search_memories` | 6-signal score | what the chat path uses for RAG injection |

---

## Where to go next

- **Architecture deep dive** — [`docs/brain-advanced-design.md`](../docs/brain-advanced-design.md)
- **End-to-end walkthrough with screenshots** — [`BRAIN-COMPLEX-EXAMPLE.md`](BRAIN-COMPLEX-EXAMPLE.md)
- **Project rules / standards** — [`rules/architecture-rules.md`](../rules/architecture-rules.md), [`rules/coding-standards.md`](../rules/coding-standards.md)
