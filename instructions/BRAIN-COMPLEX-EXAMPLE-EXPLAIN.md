# Brain + RAG — Technical Reference

> **TerranSoul v0.1** · Last updated: 2026-04-23
>
> Walkthrough with screenshots: [`brain-rag-setup-tutorial.md`](../tutorials/brain-rag-setup-tutorial.md)
> Architecture deep dive: [`docs/brain-advanced-design.md`](../docs/brain-advanced-design.md)

---

## System Map

```
Vue 3 UI (WebView)                         Rust Core (Tauri)
─────────────────                          ──────────────────────────
ChatView ─────────┐                        commands/  (97 commands)
MemoryView ───────┤  invoke / events         chat, streaming, memory,
BrainSetupView ───┤ ────────────────────►    brain, ingest, quest, …
SkillTreeView ────┤                        brain/
QuestPanel ───────┘                          OllamaAgent, OpenAiClient,
                                             FreeProvider, ProviderRotator,
                                             model_recommender
                                           memory/
                                             MemoryStore (SQLite WAL)
                                             StorageBackend trait
                                             brain_memory (LLM ops)
                                             migrations (V1–V4)
                                           ──────────────────────────
                                           %APPDATA%/com.terransoul.app/
                                             memory.db, brain.json,
                                             settings.json
```

---

## Code Map

| Concern | File |
|---|---|
| Schema migrations | `src-tauri/src/memory/migrations.rs` — V1→V4, `migrate_to_latest` |
| Memory CRUD + search | `src-tauri/src/memory/store.rs` — add, search, vector_search, hybrid_search, apply_decay, gc_decayed |
| LLM memory ops | `src-tauri/src/memory/brain_memory.rs` — extract, summarize, semantic-rank |
| Storage backends | `src-tauri/src/memory/backend.rs` — StorageBackend trait (SQLite/Postgres/MSSQL/Cassandra) |
| Brain providers | `src-tauri/src/brain/*.rs` — OllamaAgent, OpenAiClient, FreeProvider, ProviderRotator |
| Ingest pipeline | `src-tauri/src/commands/ingest.rs` — ingest_document, crawl, chunk, embed |
| Memory commands | `src-tauri/src/commands/memory.rs` — CRUD, search, backfill, schema info |
| Streaming | `src-tauri/src/commands/streaming.rs` — StreamTagParser, llm-chunk/llm-animation events |
| Frontend stores | `src/stores/{brain,memory,conversation,skill-tree}.ts` |
| Quest detection | `src/stores/conversation.ts` — maybeShowKnowledgeQuest() |
| KQ dialog | `src/components/KnowledgeQuestDialog.vue` |

---

## Schema (V4)

```sql
CREATE TABLE memories (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    content       TEXT    NOT NULL,
    tags          TEXT    NOT NULL DEFAULT '',
    importance    INTEGER NOT NULL DEFAULT 3,       -- 1..5
    memory_type   TEXT    NOT NULL DEFAULT 'fact',   -- fact|preference|context|summary
    created_at    INTEGER NOT NULL,                  -- Unix ms
    last_accessed INTEGER,
    access_count  INTEGER NOT NULL DEFAULT 0,
    embedding     BLOB,                              -- 768 × f32 LE
    source_url    TEXT,
    source_hash   TEXT,                              -- SHA-256
    expires_at    INTEGER,                           -- optional TTL ms
    tier          TEXT    NOT NULL DEFAULT 'long',   -- short|working|long
    decay_score   REAL    NOT NULL DEFAULT 1.0,      -- 0.01..1.0
    session_id    TEXT,
    parent_id     INTEGER REFERENCES memories(id),
    token_count   INTEGER NOT NULL DEFAULT 0         -- ~chars/4
);
```

---

## Hybrid Search Formula

`MemoryStore::hybrid_search` — `store.rs`

```
score = 0.40 · cosine(query_emb, entry_emb)     # vector similarity
      + 0.20 · keyword_hits / |query_words|      # BM25-lite
      + 0.15 · exp(-age_hours / 24)              # recency (24h half-life)
      + 0.10 · importance / 5                    # 1..5 → 0.2..1.0
      + 0.10 · decay_score                       # 0.01..1.0
      + 0.05 · tier_boost                        # working=1.0, long=0.5, short=0.3
```

If `query_embedding` is `None` (Ollama unreachable), the vector term is zero —
system degrades to 5 remaining signals (~60% RAG quality).

Side effect: returned entries get `last_accessed = now`, `access_count += 1`.

---

## Decay & GC

**Decay** — `apply_decay`:

```
decay_score *= 0.95 ^ (hours_since_last_access / 168)
floor = 0.01
```

Half-life ≈ 2 weeks of non-access. Only `tier='long'` rows.

**GC** — `gc_decayed(threshold)`:

```sql
DELETE FROM memories WHERE tier = 'long' AND decay_score < threshold AND importance <= 2;
```

Important memories (≥ 3) survive forever; trivia gets pruned.

---

## Ingest Pipeline

`commands/ingest.rs` → `run_ingest_task`:

```
source
  ├─ "crawl:URL"    → BFS crawl (depth ≤ 2, validate_url blocks private CIDR)
  ├─ "http(s)://…"  → fetch → extract_text_from_html
  └─ file path      → read_local_file → (if .pdf) extract_pdf_text
                           ↓
                   chunk_text(text, 800, 100)
                           ↓
                   for each chunk:
                     MemoryStore::add(content, tags, importance)
                     OllamaAgent::embed_text → set_embedding
                           ↓
                   emit task-progress events per chunk
                           ↓
                   IngestCheckpoint on cancel / timeout → resume later
```

Safety: `validate_url` rejects loopback/private CIDRs/`file://`.
`extract_text_from_html` strips `<script>`, `<style>`, `<noscript>`.

---

## Streaming Contract

Every chat turn emits three event streams:

| Event | Payload | Source |
|---|---|---|
| `llm-chunk` (done:false) | `{ text, done }` clean deltas | StreamTagParser strips `<anim>` |
| `llm-animation` | `{ emotion?, motion? }` | `<anim>{…}</anim>` JSON parsed from stream |
| `llm-chunk` (done:true) | `{ text:"", done:true }` | end-of-stream sentinel |

Frontend: `ChatView.vue` / `PetOverlayView.vue` → `listen('llm-chunk')` →
`useStreamingStore` → `useConversationStore` finalizes on `done:true`.

---

## Storage Backends

| Backend | Feature | Crate | Use case |
|---|---|---|---|
| SQLite | *(default)* | `rusqlite` | Local, zero-config, single device |
| PostgreSQL | `postgres` | `sqlx` | Multi-device, server |
| SQL Server | `mssql` | `tiberius` | Enterprise, Azure |
| CassandraDB | `cassandra` | `scylla` | High-write, eventual consistency |

```bash
cargo build --features postgres          # single backend
cargo build --features "postgres,mssql"  # multiple
```

All backends share V4 schema. Vector search is in-process cosine for all
(PostgreSQL remains an optional relational backend; SQLite/usearch is the default vector path).

---

## Debug Recipes

DB: `%APPDATA%/com.terransoul.app/memory.db`

```sql
-- Health
PRAGMA integrity_check;
PRAGMA journal_mode;        -- expect "wal"

-- Embedding coverage
SELECT COUNT(*) total, COUNT(embedding) embedded FROM memories;

-- Most retrieved
SELECT id, substr(content,1,60), access_count
  FROM memories ORDER BY access_count DESC LIMIT 10;

-- GC candidates (never accessed, >90 days old)
SELECT id, substr(content,1,60)
  FROM memories
 WHERE access_count = 0
   AND created_at < strftime('%s','now')*1000 - 7776000000
 LIMIT 20;

-- Per-tier distribution
SELECT tier, memory_type, COUNT(*), AVG(importance), AVG(decay_score)
  FROM memories GROUP BY tier, memory_type;

-- Migration history
SELECT version, description,
       datetime(applied_at/1000,'unixepoch','localtime')
  FROM schema_version ORDER BY version;
```

---

## Search Commands

| Command | Method | Use case |
|---|---|---|
| `search_memories` | SQL `LIKE` | Exact keyword, no brain needed |
| `semantic_search_memories` | Pure cosine | "find anything that means X" |
| `hybrid_search_memories` | 6-signal score | RAG injection (chat path) |

---

## Test Coverage

```
cargo test --all-targets    → 570+ pass
npx vitest run              → 941+ pass
```

| Area | Tests |
|---|---|
| `memory/migrations.rs` | V0→V4 round-trip, idempotency |
| `memory/store.rs` | CRUD, hybrid_search, vector_search, find_duplicate, decay, GC |
| `commands/ingest.rs` | chunk_text, validate_url, extract_text_from_html, sha256 dedup |
| `commands/streaming.rs` | StreamTagParser, 3-stream contract (Linux MockRuntime) |
| `stores/*.test.ts` | Pinia stores — brain, conversation, memory, skill-tree |
| `components/*.test.ts` | Vue components — ChatInput, KQ dialog, etc. |
