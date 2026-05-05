# TerranSoul Memory Philosophy — "Markdown ≠ Memory"

> **Durable lesson.** Captured from Jonathan Edwards' essay *"Stop Calling It
> Memory: The Problem with Every AI + Obsidian Tutorial"* (LIMITED EDITION
> JONATHAN, Substack, 2026-03-23). Credited in `CREDITS.md`. The entire essay
> text is **not** copied here; we extract the architectural argument and
> map it onto TerranSoul's existing implementation so this lesson survives
> across every fresh clone, MCP session, and self-improve run.
>
> **Read this before you ever propose "let's just store memories as
> markdown files in a vault."** TerranSoul already chose the right
> architecture; this file documents *why* so we don't regress.

---

## The thesis (one paragraph)

A folder of markdown files is **not** a memory system, **not** a database,
and **not** infrastructure. It is a notebook. LLMs *read* markdown
natively, but reading is not querying. The .md files popularised by
`CLAUDE.md`, OpenClaw's `MEMORY.md`, and the wave of "Obsidian =
second brain for AI" tutorials are *instruction-delivery mechanisms*
(prompt templates, config, sticky notes), not data stores. Treating
them as memory creates five predictable failure modes:

1. **No querying** — only "read the file and hope".
2. **No relationships** — wikilinks visualize, they don't traverse.
3. **Scale ceiling** — every session burns tokens dumping a vault into context.
4. **No schema enforcement** — the same fact ends up formatted three different ways across three sessions, unfindable.
5. **No concurrent access** — two agents writing the same .md silently corrupt it.

The right pattern is the boring one: a real database (SQLite, Postgres,
Kuzu, etc.) with a schema, indexes, and a query language. Markdown lives
*on top* as a human-readable projection or as instructions ("how to
behave / where to look"), never as the source of truth.

---

## TerranSoul's alignment (what we already do, and where)

TerranSoul was designed from day one with this principle in mind. The
authoritative store is SQLite; Obsidian is an opt-in **one-way
projection** of that store. Every claim below is grounded in code so
future agents can verify rather than rebuild.

| Requirement from the essay | TerranSoul implementation | File / chunk |
|---|---|---|
| Schema-first structured store | `memories` table v13 (24 columns: content, tags, importance, memory_type, tier, decay_score, category, embedding, source_url, source_hash, parent_id, session_id, valid_to, origin_device, …) | `src-tauri/src/memory/schema.rs` |
| Real query language | SQL via `rusqlite`; `WHERE`, `ORDER BY`, joins, indexes | `src-tauri/src/memory/store.rs` (`hybrid_search`, `hybrid_search_rrf`) |
| Full-text search | SQLite **FTS5** virtual table | `schema.rs` |
| Vector / semantic search | HNSW ANN via `usearch` 2.x | `src-tauri/src/memory/ann_index.rs` |
| Hybrid ranking | 6-signal: vector(40) + keyword(20) + recency(15) + importance(10) + decay(10) + tier(5), fused via RRF k=60 | `src-tauri/src/memory/store.rs`, `fusion.rs` |
| Knowledge graph (real traversals, not visual wikilinks) | Typed/directional `memory_edges` table + `idx_edges_src/dst/type/valid_to` indexes | `schema.rs`, `src-tauri/src/memory/edges.rs`, `graph_rag.rs` |
| Provenance / "show me the receipt" | `source_url`, `source_hash`, `parent_id`, `session_id`, `origin_device`, `memory_versions` (non-destructive history) | `schema.rs`, `versioning.rs` |
| Schema enforcement / no format drift | Canonical V13 schema + `cognitive_kind.rs` taxonomy + `tag_vocabulary.rs` controlled vocabulary + `auto_tag.rs` | `cognitive_kind.rs`, `tag_vocabulary.rs` |
| Concurrent access | SQLite WAL mode (default in `rusqlite`); `Arc<AppStateInner>` shared across MCP/gRPC servers | `lib.rs`, `store.rs` |
| Background consolidation / decay (essay's "the system needs to keep itself sane") | `maintenance_runtime.rs` + `maintenance_scheduler.rs` (decay, GC, summarization); `consolidation.rs`; `conflicts.rs` + `edge_conflict_scan.rs` (LLM-powered contradiction resolution) | brain + memory modules |
| Markdown stays a *projection*, not the source of truth | `obsidian_export.rs` (one-way export with `obsidian_path` + `last_exported` columns) and `obsidian_sync.rs` (opt-in bidirectional) | `src-tauri/src/memory/` |
| Markdown for *instructions*, not data | `.github/copilot-instructions.md`, `AGENTS.md`, `CLAUDE.md`, `.cursorrules`, `rules/*.md` — instruction delivery only | repo root + `rules/` |
| Optional larger-scale backends without changing the contract | `StorageBackend` trait + `postgres.rs`, `mssql.rs`, `cassandra.rs` adapters | `memory/backend.rs` |
| The retrieved context is *always wrapped* as a query result, not as "the brain" | `[RETRIEVED CONTEXT]` pack contract in `context_pack.rs` | `src-tauri/src/memory/context_pack.rs` |

The essay also recommends a graph DB *alongside* relational. TerranSoul's
`memory_edges` + `graph_rag.rs` provide the graph traversal pattern
inside SQLite (which keeps deployment trivial). If TerranSoul ever needs
true Cypher-style traversal at >1M edges, the right move is to add an
embedded graph DB (Kuzu, KuzuDB) **behind** the existing
`StorageBackend` trait — not to throw the SQLite store away.

---

## Concrete rules for future TerranSoul work

These are non-negotiable and apply to every PR, agent session, and
self-improve chunk. Persisted in `mcp-data/shared/memory-seed.sql` so
the MCP brain can recall them.

1. **Markdown is for instructions, not facts.** The only files inside
   the repo allowed to *contain* memory data (as opposed to *describing*
   it) are `mcp-data/shared/memory-seed.sql` and the runtime SQLite DB.
   Everything in `rules/`, `docs/`, `.github/copilot-instructions.md`,
   `AGENTS.md`, `CLAUDE.md`, `.cursorrules`, `mcp-data/shared/*.md` is
   *instruction* — descriptive prose pointing at where the data lives.

2. **Never propose "let's store memories in `.md` / Obsidian as the
   source of truth"**. Obsidian is a one-way **projection** via
   `obsidian_export.rs`. Bidirectional sync (`obsidian_sync.rs`) treats
   the SQLite row as authoritative on conflict.

3. **Never bypass the `StorageBackend` trait** with ad-hoc file I/O
   for "memory". If a new backend is needed, add an adapter alongside
   `postgres.rs` / `cassandra.rs`.

4. **Never bulk-load a whole "memory file" into the prompt context.**
   Always retrieve through `hybrid_search_rrf`, optionally HyDE +
   reranker, and inject the result via `context_pack.rs`'s
   `[RETRIEVED CONTEXT]` block (which explicitly tells the LLM the
   contents are *query results*, not the whole DB).

5. **Always preserve provenance.** Every memory must populate
   `source_url`/`source_hash`/`parent_id` where applicable so we can
   answer "how do you know that?" — the same UX claudia.kbanc85 calls
   "show your sources" and the same property the essay calls "the
   receipt."

6. **Schedule background sanitation.** Decay, dedupe, contradiction
   detection, and pattern surfacing belong in the maintenance scheduler
   (`brain/maintenance_scheduler.rs`), not in user-facing request paths.

7. **Schema changes go through `schema.rs` and `CANONICAL_SCHEMA_VERSION`**,
   not by sneaking new fields into markdown.

---

## Why we credit this article

The essay does not invent the database-backed memory pattern, but it
argues it forcefully and gives us a sharp vocabulary for resisting the
"markdown-as-memory" cargo cult. TerranSoul existed before the article,
but the article gives every future contributor a clear reference for
*why* we don't accept "let's just dump everything in `MEMORY.md`" PRs.
Credit lives in `CREDITS.md`. Original article:
<https://limitededitionjonathan.substack.com/p/stop-calling-it-memory-the-problem>.
