-- Refresh stale seed facts after schema V15, feature-gated code parsers,
-- incremental code indexing, and MCP token-usage benchmark docs.
-- Created: 2026-05-06

UPDATE memories
SET content = 'Code intelligence pipeline: (1) tree-sitter symbol-table ingest (Rust + TypeScript always on; Python/Go/Java/C/C++ behind parser-* features), (2) content-hash incremental re-indexing via code_file_hashes, (3) cross-file resolution + call graph with confidence scores, (4) label-propagation functional clustering via petgraph, (5) entry-point scoring + BFS process tracing, (6) native MCP tools (code_query, code_context, code_impact, code_rename), (7) editor pre/post-tool-use hooks with auto re-indexing.',
    token_count = 85
WHERE content LIKE 'Code intelligence pipeline:%';

UPDATE memories
SET content = 'MCP server exposes brain on three ports: 7421 (release app), 7422 (dev app), 7423 (headless npm run mcp). All wired into .vscode/mcp.json. Bearer-token auth required. Tools: brain_search, brain_get_entry, brain_list_recent, brain_kg_neighbors, query-backed brain_summarize, brain_suggest_context, brain_ingest_url, brain_health, brain_failover_status, code_query, code_context, code_impact, code_rename.',
    token_count = 80
WHERE content LIKE 'MCP server exposes brain on three ports:%';

UPDATE memories
SET content = 'Memory module map: schema.rs (canonical V15 SQLite schema), store.rs (default SQLite memory store with hybrid_search + hybrid_search_rrf + ANN bridge), ann_index.rs (HNSW via usearch), eviction.rs (capacity pruning with protected/high-importance preservation), backend.rs (StorageBackend trait/factory), cassandra.rs / mssql.rs / postgres.rs (optional backends), chunking.rs + late_chunking.rs (semantic chunking), code_rag.rs, cognitive_kind.rs, conflicts.rs + edge_conflict_scan.rs (LLM contradiction resolution), consolidation.rs, context_pack.rs ([RETRIEVED CONTEXT] assembly), contextualize.rs (Anthropic Contextual Retrieval), crag.rs, crdt_sync.rs, edges.rs (typed/directional KG edges), fusion.rs (RRF k=60), gitnexus_mirror.rs, graph_rag.rs, hyde.rs (HyDE), matryoshka.rs, obsidian_export.rs + obsidian_sync.rs, query_intent.rs, reflection.rs (/reflect session reflection + derived_from source-turn provenance), replay.rs, reranker.rs (LLM-as-judge), tag_vocabulary.rs, temporal.rs, versioning.rs.',
    token_count = 175
WHERE content LIKE 'Memory module map:%';

UPDATE memories
SET content = 'SQLite schema is at version 15 (CANONICAL_SCHEMA_VERSION in src-tauri/src/memory/schema.rs). memories columns include content, tags, importance, memory_type, created_at, last_accessed, access_count, embedding, source_url, source_hash, expires_at, tier, decay_score, session_id, parent_id, token_count, valid_to, obsidian_path, last_exported, category, cognitive_kind, updated_at, origin_device, and protected. Edges in memory_edges (typed, directional). Versions in memory_versions. FTS5 virtual table for keyword search. pending_embeddings(memory_id PK, attempts, last_error, next_retry_at) backs the self-healing embedding retry queue.',
    token_count = 110
WHERE content LIKE 'SQLite schema is at version 13%'
   OR content LIKE 'SQLite schema is at version 15%';

UPDATE memories
SET content = REPLACE(REPLACE(content, 'schema (V13, schema.rs)', 'schema (V15, schema.rs)'), 'populates ~40 typed memory_edges', 'populates typed memory_edges')
WHERE content LIKE 'STORAGE INVARIANT (mcp-data seed):%';

UPDATE memories
SET content = 'STACK COVERAGE: the mcp-data seed exercises the full TerranSoul retrieval stack — SQLite schema V15 (every row), FTS5 (auto-indexed on insert), KG edges (memory_edges populated by content-LIKE subqueries below), RRF fusion (5 non-vector signals always live; vector signal lights up after embedding backfill), HyDE expansion (works on any populated row at query time), and LLM-as-judge reranker (default-on for RRF/HyDE when a local brain is available, pruning below threshold 0.55).'
WHERE content LIKE 'STACK COVERAGE: the mcp-data seed exercises%';

UPDATE memories
SET content = REPLACE(content, 'Honest range: 11x (worst MCP-only query) to ~100x (best).', 'Honest range in tracked rows: 30x (broad MCP query) to ~100x (best).')
WHERE content LIKE 'MCP TOKEN BENCHMARK (2026-05-06):%';

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'STALE KNOWLEDGE AUDIT (2026-05-06): schema labels must say V15, not V13; BrainSelection storage labels now use "V15 — canonical memory schema". Code-intelligence docs must describe Rust/TypeScript always-on parsers plus feature-gated Python/Go/Java/C/C++ and content-hash incremental indexing via code_file_hashes. TerranSoul docs must not claim fixed third-party numbers such as 71.5x token reduction, 13 languages, image OCR, or a single mixed folder ingest command unless those features ship in code. Old seed migration 001 remains append-only history; migration 002_refresh_seed_facts.sql supersedes stale seeded facts for existing MCP databases.',
  'audit,stale-knowledge,schema-v15,code-intelligence,docs,mcp,terransoul',
  9, 'fact', 1746489600000, 'long', 1.0, 'mcp', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP ERROR FIX RULE (2026-05-06): if any TerranSoul MCP call returns an error, agents must not silently fall back to grep or continue with stale context. Classify the error as (1) bad tool arguments/contract mismatch, (2) unhealthy or stale MCP server/binary, or (3) missing/stale durable knowledge. Then fix the MCP tool schema/adapter/gateway and add a regression test, restart/rebuild MCP via node scripts/copilot-start-mcp.mjs when health/staleness is the cause, or update mcp-data/shared/memory-seed.sql plus a numbered migration for knowledge drift. Always report the original error, root cause, fix, and any remaining blocker. The brain_summarize query error was fixed by adding query-backed summarization to the MCP tool contract.',
  'mcp,error-handling,server-health,tool-contract,regression-test,seed-migration,non-negotiable',
  10, 'procedure', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);
