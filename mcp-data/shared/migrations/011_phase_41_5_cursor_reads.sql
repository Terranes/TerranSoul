-- 2026-05-07: record Phase 41.5 completion — cursor-based reads on hot paths.

UPDATE memories
SET content = 'PHASE 41.5 LESSON (2026-05-07): Cursor-based reads shipped. Removed get_all()/get_with_embeddings() from hybrid_search, hybrid_search_rrf, hybrid_search_rrf_with_intent, and relevant_for. Architecture: CANDIDATE_POOL=200 per signal; three retrieval signals (ANN top-200 via usearch, keyword INSTR top-200, freshness/tier top-200) union-deduplicated via HashSet; fetch only those IDs with full row data; score in-Rust on bounded set (max ~600 rows). New methods: keyword_candidate_ids, freshness_candidate_ids, get_entries_by_ids, get_entries_by_ids_with_embeddings, search_candidate_ids, iter_with_embeddings. Peak heap during search is now O(CANDIDATE_POOL) not O(corpus). 14 new tests cover cursor stability, pool bounds, and streaming iterator correctness. Pre-existing MCP tool-count test failures (brain_wiki tools) also fixed in same chunk. Full CI green: 2315 Rust tests, 1738 vitest, clippy clean.',
    created_at = 1746666000000
WHERE content LIKE 'PHASE 41.2/41.3 LESSON%';

-- Also update the plan to reflect 41.5 done
UPDATE memories
SET content = 'PHASE 41 CURRENT PLAN (updated 2026-05-07) — Million-Knowledge CRUD: completed chunks are 41.1 SQLite write-path tuning + FTS5 verification, 41.4 transactional add_many/update_many/delete_many, 41.2 per-op latency histograms surfaced through get_memory_metrics and MCP health, 41.3 extended million_memory CRUD benchmarking with timeout guards, and 41.5 cursor-based reads replacing get_all() on hot search paths with bounded CANDIDATE_POOL=200-per-signal candidate selection. Headline 1M write/read SLO is green: latest CRUD-only run wrote 1M in 7.27s and read 1M in 2.13s. Remaining chunks: 41.6 re-embed on content update + ANN tombstone; 41.7 embedding worker concurrency + rate limiting; 41.8 multi-model embeddings; 41.9 usearch i8 quantization; 41.10 memory-mapped HNSW + debounced flush; 41.11 ANN compaction/tombstone GC; 41.12 partial indexes + PRAGMA optimize; 41.13 bounded KG traversal + LRU cache; 41.14 time-bucketed shards; 41.15 online VACUUM INTO + snapshot/restore.',
    created_at = 1746666000000
WHERE content LIKE 'PHASE 41 CURRENT PLAN (updated 2026-05-07)%';
