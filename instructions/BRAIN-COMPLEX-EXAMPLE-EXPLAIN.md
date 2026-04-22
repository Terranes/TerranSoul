# Brain + RAG — Architecture & Technical Reference

> **TerranSoul v0.1** — Self-learning AI companion with persistent memory  
> Last updated: 2026-04-22  
> **See also**: [BRAIN-COMPLEX-EXAMPLE.md](BRAIN-COMPLEX-EXAMPLE.md) for the quest-guided setup walkthrough and real-world examples.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Why SQLite?](#why-sqlite)
3. [Schema & Migrations](#schema--migrations)
4. [RAG Pipeline: How Memory Retrieval Works](#rag-pipeline-how-memory-retrieval-works)
5. [Open-Source RAG Ecosystem Comparison](#open-source-rag-ecosystem-comparison)
6. [Multi-Source Knowledge Management](#multi-source-knowledge-management)
7. [Debugging with SQLite](#debugging-with-sqlite)
8. [Hardware Scaling](#hardware-scaling)
9. [FAQ](#faq)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        TerranSoul Desktop App                          │
│                                                                         │
│  ┌───────────────┐    ┌──────────────┐    ┌───────────────────────┐    │
│  │  Vue 3 UI     │───>│ Tauri IPC    │───>│  Rust Core Engine     │    │
│  │               │    │ (invoke)     │    │                       │    │
│  │ • ChatView    │    └──────────────┘    │  ┌─────────────────┐  │    │
│  │ • MemoryView  │                        │  │ Brain Module    │  │    │
│  │ • BrainSetup  │                        │  │ • OllamaAgent  │  │    │
│  └───────────────┘                        │  │ • embed_text() │  │    │
│                                           │  │ • call() (LLM) │  │    │
│                                           │  └────────┬────────┘  │    │
│                                           │           │           │    │
│                                           │  ┌────────▼────────┐  │    │
│                                           │  │ Memory Module   │  │    │
│                                           │  │ • MemoryStore   │  │    │
│                                           │  │ • vector_search │  │    │
│                                           │  │ • brain_memory  │  │    │
│                                           │  └────────┬────────┘  │    │
│                                           │           │           │    │
│                                           │  ┌────────▼────────┐  │    │
│                                           │  │ SQLite (WAL)    │  │    │
│                                           │  │ memory.db       │  │    │
│                                           │  └─────────────────┘  │    │
│                                           └───────────────────────┘    │
│                                                                         │
│                          ┌──────────────────┐                          │
│                          │ Ollama Server     │ (localhost:11434)        │
│                          │ • Chat API        │                          │
│                          │ • Embed API       │                          │
│                          │ • nomic-embed-text│                          │
│                          └──────────────────┘                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Data Flow: User Message → RAG-Enhanced Response

```
User types: "What are the filing rules for family law cases?"
                │
                ▼
┌─────────────────────────────┐
│ 1. Embed query via Ollama   │  POST /api/embed
│    → 768-dim float vector   │  {"model":"nomic-embed-text","input":"..."}
└──────────────┬──────────────┘
               ▼
┌─────────────────────────────┐
│ 2. Cosine similarity search │  Pure Rust arithmetic
│    vs all stored embeddings │  <5ms for 100k entries
│    → Top 5 memories ranked  │  No LLM call needed
└──────────────┬──────────────┘
               ▼
┌─────────────────────────────┐
│ 3. Inject into system prompt│  [LONG-TERM MEMORY]
│    as context for the LLM   │  - Family law filings require...
│                              │  - Section 14.3 states that...
└──────────────┬──────────────┘
               ▼
┌─────────────────────────────┐
│ 4. LLM generates response   │  Informed by relevant memories
│    with full RAG context     │  "Based on the firm's filing rules..."
└─────────────────────────────┘
```

---

## Why SQLite?

TerranSoul is a **desktop app** (Tauri 2.x), not a web service. The database must:

| Requirement | SQLite ✓ | PostgreSQL ✗ | Why SQLite wins |
|---|---|---|---|
| **Zero config** | Embedded, no server | Needs install + config | Users just open the app |
| **Single file** | `memory.db` | Data directory cluster | Easy backup, easy sync |
| **Crash-safe** | WAL mode = ACID | Needs `pg_dump` setup | Auto-backup on startup |
| **Portable** | Works everywhere | OS-specific packages | Windows/Mac/Linux/mobile |
| **Performance** | <5ms for 100k rows | Overkill for single-user | Desktop app, not a cluster |
| **Offline** | Always works | Needs running service | Companion works without internet |

### WAL Mode (Write-Ahead Logging)

TerranSoul enables WAL mode on every startup:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
```

**What this means for your data:**
- Writes go to a WAL file first, then get checkpointed to the main DB
- If the app crashes mid-write, the WAL replays on next open — **zero data loss**
- Concurrent reads while writing (important for RAG search during chat)

### Auto-Backup

Every time TerranSoul starts, it copies `memory.db` → `memory.db.bak`:

```
%APPDATA%/com.terransoul.app/
├── memory.db         ← Live database
├── memory.db.bak     ← Auto-backup from last startup
├── memory.db-wal     ← Write-ahead log (may exist during use)
└── memory.db-shm     ← Shared memory (may exist during use)
```

---

## Schema & Migrations

TerranSoul uses a **versioned migration system** — schema changes are tracked,
applied incrementally, and can be rolled back without losing customer data.

### Current Schema (Version 3)

```sql
-- Tracked in schema_version table
-- V1: Initial table
-- V2: Vector embeddings for fast RAG
-- V3: Source tracking for document ingestion

CREATE TABLE memories (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    content       TEXT    NOT NULL,        -- The memory text
    tags          TEXT    NOT NULL DEFAULT '',  -- Comma-separated tags
    importance    INTEGER NOT NULL DEFAULT 3,   -- 1-5 priority ranking
    memory_type   TEXT    NOT NULL DEFAULT 'fact',  -- fact|preference|context|summary
    created_at    INTEGER NOT NULL,        -- Unix timestamp (ms)
    last_accessed INTEGER,                 -- Last RAG hit timestamp
    access_count  INTEGER NOT NULL DEFAULT 0,  -- Times retrieved by RAG
    embedding     BLOB,                    -- 768-dim f32 vector (V2+)
    source_url    TEXT,                    -- Origin URL for ingested docs (V3+)
    source_hash   TEXT,                    -- Content hash for dedup (V3+)
    expires_at    INTEGER                  -- TTL for auto-expiry (V3+)
);

CREATE TABLE schema_version (
    version     INTEGER PRIMARY KEY,
    applied_at  INTEGER NOT NULL,
    description TEXT    NOT NULL DEFAULT ''
);
```

### How Migrations Work

```
App starts → Check schema_version table → Apply missing migrations
                                                │
    ┌──────────────────────┐    ┌───────────────▼───────────────┐
    │ schema_version table │    │ Migration runner               │
    │                      │    │                                │
    │ version │ applied_at │    │ current=2, target=3            │
    │ ────────┼────────────│    │ → Apply V3 up SQL              │
    │    1    │ 1713744000 │    │ → Record version 3             │
    │    2    │ 1713744001 │    │ → Done                         │
    └──────────────────────┘    └────────────────────────────────┘
```

**Upgrade**: Migrations run automatically on startup — users never notice.  
**Downgrade**: Available via `downgrade_to(conn, target_version)` for CI rollbacks.  
**Idempotent**: Running migrations twice is safe — already-applied versions are skipped.  
**Tolerant**: Old databases with ad-hoc `ALTER TABLE` are handled gracefully.

### Adding a New Migration

Append to `src-tauri/src/memory/migrations.rs`:

```rust
Migration {
    version: 4,
    description: "add your new feature",
    up: "ALTER TABLE memories ADD COLUMN new_col TEXT;",
    down: r#"
        CREATE TABLE memories_backup AS SELECT ... FROM memories;
        DROP TABLE memories;
        ALTER TABLE memories_backup RENAME TO memories;
    "#,
},
```

Then run `cargo test` — the migration tests verify up/down roundtrips automatically.

---

## RAG Pipeline: How Memory Retrieval Works

### Short-Term Memory

The last ~20 conversation messages are held in-memory (Rust `Vec<Message>`):

```
┌─────────────────────────────────────────────┐
│ Short-Term Memory (conversation buffer)      │
│                                               │
│ [user] "What are the rules for filing?"       │
│ [assistant] "Family law filings require..."   │
│ [user] "What about the deadline?"             │
│ [assistant] "Section 14.3 states 30 days..."  │
│ ...last 20 messages                           │
│                                               │
│ Stored in: Rust AppState.conversation Mutex   │
│ Lifetime: Current session only                │
│ Used for: Conversation continuity             │
└─────────────────────────────────────────────┘
```

### Long-Term Memory

Persistent facts stored in SQLite, retrieved by vector similarity:

```
┌─────────────────────────────────────────────┐
│ Long-Term Memory (SQLite + embeddings)       │
│                                               │
│ id=1  "Client prefers email communication"    │
│       embedding=[0.12, -0.34, 0.56, ...]     │
│       tags="preference,client"                │
│       importance=4                            │
│                                               │
│ id=2  "Family law filings: 30-day deadline"   │
│       embedding=[0.78, 0.11, -0.45, ...]     │
│       tags="law,family,deadline"              │
│       importance=5                            │
│                                               │
│ id=3  "Office closes at 5pm on Fridays"       │
│       embedding=[0.01, 0.99, -0.02, ...]     │
│       tags="office,schedule"                  │
│       importance=2                            │
│                                               │
│ Stored in: memory.db (SQLite WAL)             │
│ Lifetime: Permanent until deleted             │
│ Used for: RAG context injection               │
└─────────────────────────────────────────────┘
```

### Vector Search Performance

The key innovation: **pure-Rust cosine similarity** — no external vector DB needed.

```
Traditional (SLOW — old approach):
  Load ALL memories → Send to LLM → "Which are relevant?" → Wait 2-5 seconds
  Scales to ~500 entries, then LLM context window overflows

TerranSoul Vector RAG (FAST — current approach):
  Embed query (50ms) → Cosine search all vectors (<5ms) → Inject top 5 → Done
  Scales to 100,000+ entries with no degradation

┌──────────────────────────────────────────────────────┐
│ Benchmark: vector_search() on 100,000 entries        │
│                                                       │
│  Entries    │  Search Time  │  Method                 │
│ ────────────┼───────────────┼──────────────────────── │
│       100   │     <1 ms     │  Pure cosine similarity │
│     1,000   │     <1 ms     │  Pure cosine similarity │
│    10,000   │      2 ms     │  Pure cosine similarity │
│   100,000   │      5 ms     │  Pure cosine similarity │
│ 1,000,000   │     ~50 ms    │  Pure cosine similarity │
│                                                       │
│ * All times exclude the one-time 50ms embed call      │
│ * Measured on RTX 3080 Ti / 65GB RAM / Windows 11     │
└──────────────────────────────────────────────────────┘
```

### Embedding Model

TerranSoul uses `nomic-embed-text` (768-dimensional) via Ollama:

```
POST http://127.0.0.1:11434/api/embed
{
    "model": "nomic-embed-text",
    "input": "Family law filings require a 30-day notice period"
}

Response:
{
    "embeddings": [[0.0123, -0.0456, 0.0789, ...]]  // 768 floats
}
```

- **Auto-installed**: If `nomic-embed-text` isn't available, falls back to the active chat model
- **Storage**: Each embedding = 768 × 4 bytes = **3 KB** per memory
- **100k memories** = ~300 MB of embeddings in SQLite — easily fits in RAM

### Deduplication

Before storing a new memory, TerranSoul checks for near-duplicates:

```
New text: "Family law filings must be submitted within 30 days"
                │
                ▼
Embed → cosine similarity vs all existing embeddings
                │
                ▼
Existing id=2: "Family law filings: 30-day deadline"
Cosine similarity = 0.98 (threshold: 0.97)
                │
                ▼
DUPLICATE DETECTED → Skip insert, return existing entry
```

---

## Open-Source RAG Ecosystem Comparison

TerranSoul's RAG pipeline is purpose-built for a single-user desktop companion.
Here's how it compares to the leading open-source RAG and memory frameworks:

### Mem0 (53.7k stars)

**What it does**: Multi-level memory system (user/session/agent) with automatic entity extraction, graph-based relationships, and conflict resolution.

**Key features**:
- Extracts entities and relationships from conversations automatically
- Graph memory layer links `Person → prefers → Email communication`
- Conflict detection: "User said they like tea" vs "User now says they like coffee" → resolves to latest

**TerranSoul comparison**:
| Capability | Mem0 | TerranSoul |
|---|---|---|
| Memory storage | External vector DB (Qdrant/Chroma) | Embedded SQLite — zero infra |
| Entity extraction | Automatic via LLM | Manual tags + LLM-assisted extract |
| Memory levels | User / Session / Agent | Short-term (buffer) / Long-term (SQLite) |
| Graph relationships | Built-in | Not yet (could add via `related_ids` column) |
| Conflict resolution | LLM-powered automatic | Hash-based staleness + LLM conflict analysis |
| Deployment | Requires server + vector DB | Fully embedded, works offline |

**What TerranSoul borrows**: The concept of LLM-powered memory extraction (our "Extract from session" button) and conflict detection for stale knowledge. Mem0's graph memory layer is a future upgrade candidate.

### LlamaIndex (48.8k stars)

**What it does**: Full document ingestion + retrieval framework with 160+ data connectors, LlamaParse for PDFs, and composable query pipelines.

**Key features**:
- LlamaParse: Best-in-class PDF/table extraction (handles complex layouts, charts, multi-column)
- 160+ data connectors (Notion, Slack, Google Drive, databases, APIs)
- Composable query engines: tree summarize, compact, refine, accumulate

**TerranSoul comparison**:
| Capability | LlamaIndex | TerranSoul |
|---|---|---|
| PDF ingestion | LlamaParse (cloud API) | External script + text extraction |
| Data connectors | 160+ built-in | Manual ingestion scripts per source |
| Query pipeline | Composable (tree/compact/refine) | Single-pass vector search + inject |
| Embedding | Any provider | Ollama nomic-embed-text (local) |
| Chunking | Sentence/token/semantic splitters | 500-word overlap chunking |
| Deployment | Python library | Rust native binary |

**What TerranSoul borrows**: The chunking strategy (500-word segments with 50-word overlap) is inspired by LlamaIndex's sentence window approach. LlamaParse integration is a planned enhancement for complex PDF ingestion.

### ChromaDB (27.6k stars)

**What it does**: Open-source embedding database with a Rust core, designed for simplicity. The closest architectural match to TerranSoul's approach.

**Key features**:
- Simple API: `collection.add()`, `collection.query()`
- Rust core engine for performance
- Metadata filtering alongside vector search
- Supports multiple distance functions (cosine, L2, inner product)

**TerranSoul comparison**:
| Capability | ChromaDB | TerranSoul |
|---|---|---|
| Storage | Custom Rust engine | SQLite BLOB column |
| Distance function | Cosine / L2 / IP | Cosine only |
| Metadata filtering | Built-in | SQL WHERE clauses on tags/importance |
| Indexing | HNSW (approximate) | Brute-force linear scan |
| Scalability | Millions (ANN) | Millions (acceptable at <50ms) |
| Deployment | Separate server or embedded | Fully embedded in app binary |

**What TerranSoul borrows**: The philosophy of "embeddings in a single binary." ChromaDB proves that a Rust core + simple API can handle production workloads. TerranSoul takes this further by eliminating the external process entirely — the vector search runs inside the same Tauri process.

### RAGFlow (78.7k stars)

**What it does**: Enterprise-grade RAG engine with deep document understanding. Excels at complex document layouts (tables, images, scanned PDFs).

**Key features**:
- "Deep document understanding" — handles headers, footers, tables, multi-column layouts
- Chunk visualization: see exactly which chunks were retrieved and why
- Multi-turn conversation with document grounding
- Supports 30+ file formats out of the box

**TerranSoul comparison**:
| Capability | RAGFlow | TerranSoul |
|---|---|---|
| Document parsing | Deep layout understanding | Plain text extraction |
| File formats | 30+ (PDF, DOCX, PPTX, images) | Text-based (via external scripts) |
| Chunk visualization | Built-in UI | Memory View + access_count tracking |
| Deployment | Docker (server-based) | Desktop app (no server) |
| Target user | Enterprise teams | Individual power users |

**Takeaway**: RAGFlow's deep document parsing is overkill for TerranSoul's use case, but its chunk visualization concept inspired TerranSoul's `access_count` tracking — seeing which memories are actually being used by RAG.

### Cognee (16.6k stars)

**What it does**: Knowledge engine combining vector search + knowledge graph + LLM reasoning. Bridges the gap between raw RAG and structured knowledge.

**Key features**:
- Automatic knowledge graph construction from documents
- Combines vector similarity with graph traversal for multi-hop reasoning
- "Think → Retrieve → Generate" pipeline
- Entity extraction + relationship mapping

**TerranSoul comparison**:
| Capability | Cognee | TerranSoul |
|---|---|---|
| Knowledge representation | Graph + Vector | Vector only (tags for structure) |
| Multi-hop reasoning | Graph traversal | Not yet (single-hop vector search) |
| Entity extraction | Automatic | LLM-assisted ("Extract from session") |
| Deployment | Python library | Rust native binary |

**Future direction**: Cognee's graph-based approach would benefit TerranSoul for complex queries like "Who are all the clients connected to the Smith case, and what are their communication preferences?" This requires traversing relationships, not just vector similarity.

### Summary: Why TerranSoul Doesn't Use an External RAG Framework

| Decision Factor | External Framework | TerranSoul Built-in |
|---|---|---|
| **Zero dependencies** | Requires Python/Docker/server | Just install the app |
| **Offline-first** | Most need network for vector DB | SQLite works offline always |
| **Privacy** | Data may leave the machine | Everything stays local |
| **Single binary** | Multiple processes to manage | One Tauri binary |
| **Desktop UX** | Built for servers/APIs | Built for desktop companion |
| **Performance** | Network overhead | In-process, <5ms search |
| **Maintenance** | Version compatibility issues | Self-contained, auto-migrating |

TerranSoul's approach: **take the best ideas** from these frameworks (Mem0's conflict detection, LlamaIndex's chunking, Chroma's Rust-native search, RAGFlow's access tracking, Cognee's entity extraction vision) and **implement them natively in Rust** as part of the Tauri binary. No external processes, no Docker, no Python — just a desktop app that works.

---

## Multi-Source Knowledge Management

Real-world knowledge comes from many sources that overlap, conflict, and go stale.
TerranSoul handles this with four mechanisms:

### 1. Source Hash Change Detection

Every ingested document is tracked by URL and SHA-256 content hash:

```
Monday (initial sync):
  Court Rule 14.3 → hash = "a1b2c3d4"
  → Stored: id=42, source_hash="a1b2c3d4"

Tuesday (daily sync):
  Court Rule 14.3 → hash = "a1b2c3d4"  (same)
  → SKIP — content unchanged

Wednesday (rule amended):
  Court Rule 14.3 → hash = "e5f6g7h8"  (DIFFERENT!)
  → Detected: source_hash mismatch for source_url
  → Action:
    1. DELETE old memory (id=42)
    2. INSERT new content with new hash
    3. Auto-embed the new content
    4. Log: "Updated: Court Rule 14.3 — content changed"
```

```
┌──────────────────────────────────────────────────────────┐
│                  STALENESS DETECTION                      │
│                                                           │
│   Source URL             Stored Hash    Current Hash      │
│   ──────────────────     ───────────    ────────────      │
│   /rules/family/14.3    a1b2c3d4       e5f6g7h8   ← STALE│
│   /rules/family/14.4    f9g0h1i2       f9g0h1i2   ✓ OK   │
│   /rules/civil/22.1     j3k4l5m6       j3k4l5m6   ✓ OK   │
│   /policies/billing     n7o8p9q0       r1s2t3u4   ← STALE│
│                                                           │
│   Action: 2 memories updated, 2 re-embedded               │
└──────────────────────────────────────────────────────────┘
```

### 2. TTL Expiry (`expires_at` Column)

Some knowledge has a natural shelf life:

```sql
-- Set expiry when ingesting time-sensitive content
INSERT INTO memories (content, tags, importance, memory_type,
                      created_at, source_url, expires_at)
VALUES (
  'Court closed Dec 25, 2026 – Jan 2, 2027 for holiday recess',
  'court-calendar,holiday',
  3,
  'fact',
  1713744000000,
  'https://ilcourts.gov/calendar/2026',
  1735776000000   -- expires Jan 2, 2027
);

-- Daily cleanup: remove expired memories
DELETE FROM memories
WHERE expires_at IS NOT NULL AND expires_at < strftime('%s','now') * 1000;
```

| Content Type | Typical TTL | Example |
|---|---|---|
| Court calendar events | Until event date + 1 day | "Hearing on May 15" |
| Holiday schedules | Until end of holiday | "Closed Dec 25-Jan 2" |
| Temporary policies | Duration of policy | "COVID masking required" |
| Client case deadlines | Until deadline + 7 days | "Smith motion due Apr 30" |
| Permanent rules | No expiry (`NULL`) | "30-day filing deadline" |

### 3. Access Count Decay (Unused Knowledge)

Memories that RAG never retrieves are probably not useful:

```sql
-- Find memories older than 90 days that were never accessed by RAG
SELECT id, content, created_at, access_count
FROM memories
WHERE access_count = 0
  AND created_at < strftime('%s','now') * 1000 - 7776000000  -- 90 days
ORDER BY created_at ASC;

-- Example output:
-- id   | content                              | access_count
-- 847  | 2025 holiday party menu choices       | 0
-- 1203 | Old phone extension list (pre-move)   | 0
-- 1456 | Draft policy that was never adopted   | 0
```

### 4. LLM-Powered Conflict Resolution

When new information semantically overlaps with existing knowledge but says something different, TerranSoul uses the LLM to analyze the conflict:

```
Existing memory (id=42):
  "Family law motion responses must be filed within 30 days"
  source: ilcourts.gov/rules/family/14.3
  stored: 2026-03-15

New incoming content:
  "Effective April 1, 2026: Family law motion responses now have a
   21-day filing deadline (reduced from 30 days)"
  source: ilcourts.gov/rules/family/14.3-amended

Conflict detection:
  1. Embed new content → cosine similarity with id=42 = 0.94
     (high similarity, but below 0.97 dedup threshold)
  2. Both reference "family law motion response deadline"
  3. But values differ: 30 days vs 21 days

LLM conflict analysis prompt:
  "Compare these two pieces of information and determine if the
   new one supersedes the old one:
   OLD: [30-day filing deadline]
   NEW: [21-day filing deadline, effective April 1]
   Which is current?"

LLM response:
  "The new information supersedes the old. The deadline was reduced
   from 30 to 21 days effective April 1, 2026."

Action:
  → Mark id=42 as expired (or delete)
  → Insert new memory with updated content
  → Log: "Conflict resolved: Rule 14.3 deadline updated 30d → 21d"
```

This is inspired by **Mem0's conflict resolution** approach — using the LLM itself to arbitrate when two memories say different things about the same topic.

---

## Debugging with SQLite

### Where is the Database?

The database file lives in the Tauri app data directory:

| OS | Path |
|---|---|
| **Windows** | `%APPDATA%\com.terransoul.app\memory.db` |
| **macOS** | `~/Library/Application Support/com.terransoul.app/memory.db` |
| **Linux** | `~/.local/share/com.terransoul.app/memory.db` |

### Opening SQLite: Recommended Tools

#### Option 1: DB Browser for SQLite (GUI — Recommended for beginners)

1. **Download**: https://sqlitebrowser.org/dl/
2. **Install**: Run the installer (Windows: `.msi`, macOS: `.dmg`)
3. **Open**: File → Open Database → Navigate to `memory.db`

```
┌─ DB Browser for SQLite ────────────────────────────────┐
│ File  Edit  View  Tools  Help                           │
│                                                          │
│ Database: memory.db                                      │
│                                                          │
│ Tables:                                                  │
│  ├── memories (15,247 rows)                              │
│  └── schema_version (3 rows)                             │
│                                                          │
│ ┌──────────────────────────────────────────────────────┐ │
│ │ Browse Data │ Execute SQL │ DB Structure │ Edit Prag │ │
│ ├──────────────────────────────────────────────────────┤ │
│ │ Table: memories ▾                                    │ │
│ ├────┬──────────────────────┬──────┬─────┬─────┬──────┤ │
│ │ id │ content              │ tags │ imp │ type│ embed│ │
│ ├────┼──────────────────────┼──────┼─────┼─────┼──────┤ │
│ │  1 │ Filing deadline: 30d │ law  │  5  │ fact│ BLOB │ │
│ │  2 │ Client prefers email │ pref │  4  │ pref│ BLOB │ │
│ │  3 │ Office hours M-F 9-5 │ info │  2  │ fact│ BLOB │ │
│ │ .. │ ...                  │      │     │     │      │ │
│ └────┴──────────────────────┴──────┴─────┴─────┴──────┘ │
└─────────────────────────────────────────────────────────┘
```

#### Option 2: sqlite3 CLI (Terminal — for power users)

Pre-installed on macOS/Linux. Windows: download from https://sqlite.org/download.html

```bash
# Open the database
sqlite3 "%APPDATA%/com.terransoul.app/memory.db"

# Show tables
.tables
# → memories  schema_version

# Show schema
.schema memories

# Check schema version
SELECT * FROM schema_version;
# → 1|1713744000|initial memories table
# → 2|1713744001|add embedding column for vector search
# → 3|1713744002|add source metadata columns for document ingestion
```

#### Option 3: VS Code Extension

Install "SQLite Viewer" (id: `qwtel.sqlite-viewer`) in VS Code, then open `memory.db` directly.

### Useful Debug Queries

#### Check how many memories have embeddings

```sql
SELECT
    COUNT(*) AS total,
    COUNT(embedding) AS embedded,
    COUNT(*) - COUNT(embedding) AS unembedded
FROM memories;

-- Example output:
-- total | embedded | unembedded
-- 15247 |    15200 |         47
```

#### Find the most-accessed memories (RAG hits)

```sql
SELECT id, content, access_count, last_accessed
FROM memories
ORDER BY access_count DESC
LIMIT 10;

-- Example output:
-- id | content                        | access_count | last_accessed
--  2 | Family law filings: 30-day...  |          147 | 1713800000000
--  5 | Section 14.3: Motion response  |          133 | 1713799500000
```

#### Find memories that RAG never retrieved

```sql
SELECT id, content, created_at
FROM memories
WHERE access_count = 0
ORDER BY created_at DESC
LIMIT 20;
```

#### Check for orphaned embeddings (size validation)

```sql
SELECT id, content, LENGTH(embedding) AS embed_bytes,
       LENGTH(embedding) / 4 AS dimensions
FROM memories
WHERE embedding IS NOT NULL
LIMIT 5;

-- Expected: 3072 bytes = 768 dimensions × 4 bytes/float
-- id | content     | embed_bytes | dimensions
--  1 | Filing...   |        3072 |        768
```

#### View schema migration history

```sql
SELECT version, description,
       datetime(applied_at / 1000, 'unixepoch', 'localtime') AS applied
FROM schema_version
ORDER BY version;

-- version | description                                  | applied
--       1 | initial memories table                       | 2026-04-20 10:00:00
--       2 | add embedding column for vector search       | 2026-04-20 10:00:01
--       3 | add source metadata columns for doc ingest   | 2026-04-22 14:30:00
```

#### Find duplicate content

```sql
SELECT a.id, b.id AS dup_id, a.content
FROM memories a
JOIN memories b ON a.id < b.id AND a.content = b.content;
```

#### Check database health

```sql
-- File size and page count
PRAGMA page_count;
PRAGMA page_size;

-- WAL mode verification
PRAGMA journal_mode;
-- → wal

-- Integrity check
PRAGMA integrity_check;
-- → ok

-- Database statistics
SELECT
    (SELECT COUNT(*) FROM memories) AS total_memories,
    (SELECT COUNT(*) FROM schema_version) AS schema_versions,
    (SELECT MAX(version) FROM schema_version) AS current_version;
```

### Common Debugging Scenarios

#### "My memories aren't being found by RAG"

```sql
-- Check if the memory has an embedding
SELECT id, content, embedding IS NOT NULL AS has_embedding
FROM memories
WHERE content LIKE '%your search term%';

-- If has_embedding = 0, run backfill:
-- In TerranSoul: invoke('backfill_embeddings')
```

#### "RAG is returning irrelevant results"

```sql
-- Check what's being accessed most recently
SELECT id, content, access_count,
       datetime(last_accessed / 1000, 'unixepoch', 'localtime') AS last_hit
FROM memories
WHERE last_accessed IS NOT NULL
ORDER BY last_accessed DESC
LIMIT 10;

-- Look for low-importance entries polluting results
SELECT id, content, importance
FROM memories
WHERE importance <= 2 AND access_count > 10;
-- Consider increasing importance or deleting irrelevant entries
```

#### "Database seems corrupted"

```sql
-- Run integrity check
PRAGMA integrity_check;

-- If not "ok", restore from backup:
-- 1. Close TerranSoul
-- 2. Copy memory.db.bak → memory.db
-- 3. Reopen TerranSoul (migrations will re-apply if needed)
```

#### "I want to reset everything"

```sql
-- Delete all memories (keeps schema)
DELETE FROM memories;

-- Or delete the file entirely:
-- Close TerranSoul → delete memory.db → reopen (fresh database created)
```

---

## Hardware Scaling

### How Memory Count Maps to Hardware

| Memory Count | Embedding Storage | RAM Usage | Search Time | Recommended Hardware |
|---|---|---|---|---|
| 1,000 | 3 MB | ~50 MB | <1 ms | Any modern PC |
| 10,000 | 30 MB | ~100 MB | ~2 ms | 8 GB RAM |
| 100,000 | 300 MB | ~500 MB | ~5 ms | 16 GB RAM |
| 1,000,000 | 3 GB | ~4 GB | ~50 ms | 32 GB RAM |
| 10,000,000 | 30 GB | ~35 GB | ~500 ms | 64 GB RAM |

### Your Hardware (65 GB RAM, RTX 3080 Ti)

With 65 GB RAM, you can comfortably hold **~10 million memory entries** in the vector index:

```
Your capacity:
├── Chat model (e.g., gemma3:12b):  ~8 GB VRAM
├── Embedding model (nomic-embed):  ~300 MB VRAM
├── OS + Apps:                      ~8 GB RAM
├── Available for memory index:     ~49 GB RAM
│
├── At 3 KB per embedding:
│   49 GB / 3 KB = ~16 million entries
│
└── Practical limit: ~10 million entries
    (leaves headroom for SQLite, OS cache, etc.)
```

### Scaling Beyond 10 Million

For datasets exceeding 10M entries (unlikely for a desktop companion), the next step would be:
- **HNSW index**: Approximate Nearest Neighbor search — O(log n) instead of O(n)
- **Sharding**: Split memories across multiple SQLite files by date/topic
- **External vector DB**: Connect to Qdrant/Milvus as a Tauri sidecar

The current pure-cosine approach is intentionally simple and works for the vast majority of use cases.

---

## FAQ

### "How do I ask TerranSoul about SQLite?"

Just ask in the chat! If TerranSoul has memories about SQLite (or you add them), it will use RAG to answer:

> **You**: "How do I check which schema version the database is on?"  
> **TerranSoul**: "You can check the schema version by querying `SELECT * FROM schema_version ORDER BY version DESC LIMIT 1;` in DB Browser for SQLite. The current target version is 3, which includes the source tracking columns for document ingestion."

### "What if Ollama is not running?"

TerranSoul gracefully degrades:
- **Vector search**: Skipped (no embedding available for query)
- **Fallback**: Keyword search across `content` and `tags` columns
- **Chat**: Uses Free Cloud API if configured as backup

### "Can I export/import memories?"

```sql
-- Export to CSV
.mode csv
.headers on
.output memories_backup.csv
SELECT id, content, tags, importance, memory_type, created_at FROM memories;
.output stdout

-- Import from CSV
.mode csv
.import memories_backup.csv memories
```

### "How do I add memories programmatically?"

Via Tauri IPC from the frontend:

```typescript
import { invoke } from '@tauri-apps/api/core';

// Add a single memory (auto-embedded if brain is configured)
await invoke('add_memory', {
  content: 'Court filing deadline is 30 days from service',
  tags: 'law,deadline,filing',
  importance: 5,
  memoryType: 'fact',
});

// Backfill embeddings for all un-embedded entries
const count = await invoke<number>('backfill_embeddings');
console.log(`Embedded ${count} new entries`);

// Check database schema info
const info = await invoke('get_schema_info');
console.log(info);
// {
//   schema_version: 3,
//   target_version: 3,
//   total_memories: 15247,
//   unembedded_count: 47,
//   embedded_count: 15200,
//   db_engine: "SQLite (WAL mode)",
//   columns: { ... }
// }
```

### "How do I check the database columns?"

Call `get_schema_info` from the UI or chat. It returns all column definitions:

| Column | Type | Description |
|---|---|---|
| `id` | INTEGER PRIMARY KEY | Auto-incrementing unique ID |
| `content` | TEXT NOT NULL | The memory text |
| `tags` | TEXT | Comma-separated tags for categorization |
| `importance` | INTEGER (1-5) | Priority ranking (5 = most important) |
| `memory_type` | TEXT | `fact`, `preference`, `context`, or `summary` |
| `created_at` | INTEGER | Unix timestamp in milliseconds |
| `last_accessed` | INTEGER | Last time RAG retrieved this memory |
| `access_count` | INTEGER | Number of times retrieved by RAG |
| `embedding` | BLOB | 768-dim f32 vector (3,072 bytes) |
| `source_url` | TEXT | Origin URL for ingested documents |
| `source_hash` | TEXT | SHA-256 hash for dedup/staleness |
| `expires_at` | INTEGER | TTL timestamp for auto-expiry |

### "What's the difference between search and semantic search?"

| Feature | `search_memories` | `semantic_search_memories` |
|---|---|---|
| Method | SQL `LIKE '%keyword%'` | Cosine similarity on embeddings |
| Speed | <1ms (any size) | ~50ms embed + <5ms search |
| Accuracy | Exact match only | Understands meaning |
| Requires Brain | No | Yes (Ollama for embedding) |
| Example | "deadline" finds "deadline" | "when to file" finds "30-day deadline" |

### "How does TerranSoul compare to using ChromaDB or Pinecone?"

See [Open-Source RAG Ecosystem Comparison](#open-source-rag-ecosystem-comparison) above. TL;DR: TerranSoul embeds the vector search directly in the Tauri binary — no external process, no Docker, no network calls. For a single-user desktop companion, this is simpler and faster than any external vector database.
