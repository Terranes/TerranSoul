-- 023_seed_knowledge_current_state.sql
-- Refresh canonical seed knowledge so runtime MCP databases reflect
-- current schema/app-state behavior.

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind)
SELECT
  'LESSON: Seed knowledge audit (2026-05-10) corrected stale canonical facts: schema is V20 (not V13/V15/V19 in older notes) and brain_failover_status in headless MCP may return "failover status requires app state" when provider-rotator AppState is absent; this is diagnostics-only and does not block core memory/RAG tools.',
  'lesson,mcp,seed,audit,schema,failover-status,headless',
  9, 'lesson', 1778371200000, 'long', 1.0, 70, 'mcp', 'procedural'
WHERE NOT EXISTS (
  SELECT 1
  FROM memories
  WHERE content LIKE 'LESSON: Seed knowledge audit (2026-05-10) corrected stale canonical facts:%'
);

UPDATE memories
SET
  content = 'MCP server exposes brain on three ports: 7421 (release app), 7422 (dev app), 7423 (headless npm run mcp). All wired into .vscode/mcp.json. Bearer-token auth required. Tools include brain_search, brain_get_entry, brain_list_recent, brain_kg_neighbors, query-backed brain_summarize, brain_suggest_context, brain_ingest_url, brain_health, and brain_failover_status. In headless/stdio runs where MCP is not attached to full AppState provider-rotator state, brain_failover_status may return "failover status requires app state" as a diagnostics-only limitation; core memory/RAG tools still work.',
  token_count = 95,
  cognitive_kind = 'semantic'
WHERE content LIKE 'MCP server exposes brain on three ports:%'
  AND tags = 'mcp,server,tools,setup';

UPDATE memories
SET
  content = 'Memory module map: schema.rs (canonical V20 SQLite schema), store.rs (default SQLite memory store with hybrid_search + hybrid_search_rrf + ANN bridge), ann_index.rs (HNSW via usearch), eviction.rs (capacity pruning with protected/high-importance preservation), backend.rs (StorageBackend trait/factory), cassandra.rs / mssql.rs / postgres.rs (optional backends), chunking.rs + late_chunking.rs (semantic chunking), code_rag.rs, cognitive_kind.rs, conflicts.rs + edge_conflict_scan.rs (LLM contradiction resolution), consolidation.rs, context_pack.rs ([RETRIEVED CONTEXT] assembly), contextualize.rs (Anthropic Contextual Retrieval), crag.rs, crdt_sync.rs, edges.rs (typed/directional KG edges), fusion.rs (RRF k=60), gitnexus_mirror.rs, graph_rag.rs, hyde.rs (HyDE), matryoshka.rs, obsidian_export.rs + obsidian_sync.rs, query_intent.rs, reflection.rs (/reflect session reflection + derived_from source-turn provenance), replay.rs, reranker.rs (LLM-as-judge), tag_vocabulary.rs, temporal.rs, versioning.rs.',
  cognitive_kind = 'semantic'
WHERE content LIKE 'Memory module map:%'
  AND tags = 'memory,module-map,architecture';

UPDATE memories
SET
  content = 'SQLite schema is at version 20 (CANONICAL_SCHEMA_VERSION in src-tauri/src/memory/schema.rs). memories columns include content, tags, importance, memory_type, created_at, last_accessed, access_count, embedding, embedding_model_id, embedding_dim, source_url, source_hash, expires_at, tier, decay_score, session_id, parent_id, token_count, valid_to, obsidian_path, last_exported, category, cognitive_kind, updated_at, origin_device, hlc_counter, protected, share_scope, and confidence. Edges in memory_edges are typed/directional and include origin_device plus hlc_counter. Versions in memory_versions. FTS5 virtual table for keyword search. pending_embeddings(memory_id PK, attempts, last_error, next_retry_at) backs the self-healing embedding retry queue.',
  token_count = 125,
  cognitive_kind = 'semantic'
WHERE content LIKE 'SQLite schema is at version %'
  AND tags = 'memory,schema,sqlite';

UPDATE memories
SET
  content = 'STACK COVERAGE: the mcp-data seed exercises the full TerranSoul retrieval stack — SQLite schema V20 (every row), FTS5 (auto-indexed on insert), KG edges (memory_edges populated by content-LIKE subqueries below), RRF fusion (5 non-vector signals always live; vector signal lights up after embedding backfill), HyDE expansion (works on any populated row at query time), and LLM-as-judge reranker (default-on for RRF/HyDE when a local brain is available, pruning below threshold 0.55).',
  cognitive_kind = 'semantic'
WHERE content LIKE 'STACK COVERAGE: the mcp-data seed exercises the full TerranSoul retrieval stack%';
