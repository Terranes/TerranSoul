-- 2026-05-06: durable audit + 15-chunk plan for million-memory CRUD.
-- Inserted into the canonical memories + memory_edges schema so
-- future MCP sessions can retrieve the audit via brain_search.

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MILLION-KNOWLEDGE CRUD AUDIT (2026-05-06): TerranSoul memory store today already has SQLite WAL + foreign_keys, V15 schema with eviction-friendly indexes, usearch HNSW with brute-force fallback, capacity eviction with scored single-statement DELETE, embedding retry queue with exponential backoff, memory_versions snapshots, contradiction detection, semantic chunker, late-chunking pooling helper, contextual retrieval prefix, Criterion 10k smoke / 1M full bench, ingest semaphore. Confirmed bottlenecks for 1M+ optimal CRUD: (1) per-row INSERT loop in commands/ingest.rs and add_memory_inner, (2) PRAGMAs cache_size / mmap_size / temp_store / page_size / journal_size_limit / wal_autocheckpoint left at defaults, (3) inconsistent prepare_cached, (4) get_all() / get_with_embeddings() called on every hybrid_search and relevant_for which loads ~3 GB at 1M x 768d, (5) MemoryStore::update never invalidates embedding nor removes the row from HNSW when content changes, (6) HNSW SAVE_INTERVAL=50 causes ~20k full saves during 1M bulk insert, (7) single global ANN regresses to brute-force on dimension drift, (8) no add_many / update_many / delete_many APIs, (9) FTS5 + KG indexes need verification at 1M rows.',
  'audit,memory-store,million-scale,bottlenecks,phase-41,non-negotiable',
  10, 'fact', 1746489600000, 'long', 1.0, 'memory', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PHASE 41 PLAN (2026-05-06) — Million-Knowledge CRUD: 15 ordered chunks across 6 sub-phases. A. Foundations: 41.1 SQLite write-path tuning + FTS5 verification (cache_size=-65536, mmap_size=256 MiB, temp_store=MEMORY, busy_timeout=5000, journal_size_limit=64 MiB, wal_autocheckpoint=1000, page_size=8192 on fresh DBs); 41.2 per-op latency histograms surfaced via brain_health; 41.3 extend million_memory bench with bulk_insert/bulk_update_reembed/bulk_delete/mixed_crud_workload at 10k/100k/1M. B. Bulk CRUD: 41.4 transactional add_many/update_many/delete_many with prepare_cached and batched FTS/ANN sync; 41.5 cursor reads to remove get_all() from hybrid_search/hybrid_search_rrf/relevant_for/find_duplicate. C. Embeddings: 41.6 re-embed on content update + ANN tombstone + enqueue; 41.7 embedding worker concurrency + rate limiting + pause-on-429 + graceful shutdown; 41.8 V16 schema multi-model embeddings with memory_embeddings side table and AnnRegistry keyed by (model_id, dim). D. ANN: 41.9 usearch i8 quantization (optional b1) with recall budget; 41.10 memory-mapped HNSW + debounced async flush replacing SAVE_INTERVAL=50; 41.11 ANN compaction / tombstone GC tied to maintenance scheduler. E. Indexes/KG: 41.12 partial indexes idx_memories_long_embedded WHERE tier=long AND embedding IS NOT NULL, idx_memories_active WHERE valid_to IS NULL, idx_pending_due, idx_memories_session_recent, plus PRAGMA optimize on open and periodic ANALYZE; 41.13 bounded KG traversal + LRU cache for brain_kg_neighbors with edge-event invalidation. F. Sharding/snapshots: 41.14 optional time-bucketed shards via ATTACH DATABASE; 41.15 online VACUUM INTO + ANN save + manifest snapshot/restore. Each chunk must keep the Full CI Gate green and extend benches/million_memory.rs where relevant. State of the art consulted through May 2026: HNSW (Malkov 2018), DiskANN/SPANN (Microsoft 2019, 2021), Matryoshka embeddings (Kusupati 2022), int8/binary quantization (Cohere 2024), late chunking (Günther 2024), Contextual Retrieval (Anthropic 2024), RRF fusion (Cormack 2009).',
  'plan,phase-41,million-scale,chunks,ann,sqlite,embedding,sharding,non-negotiable',
  10, 'fact', 1746489600000, 'long', 1.0, 'memory', 'procedural'
);

-- Connect the plan to the audit so the knowledge graph reflects the relationship.
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT plan.id, audit.id, 'derived_from', 1.0, 'auto', 1746489600000, 'auto'
FROM memories plan
CROSS JOIN memories audit
WHERE plan.content LIKE 'PHASE 41 PLAN (2026-05-06)%'
  AND audit.content LIKE 'MILLION-KNOWLEDGE CRUD AUDIT (2026-05-06)%';
