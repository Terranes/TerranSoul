-- 027_chunk_50_4_memory_search_synthesis.sql
-- Durable lesson for Chunk 50.4: consolidation synthesis, diversified RRF,
-- and progressive compact-first memory search. Mirrors memory-seed.sql so
-- existing MCP databases receive the same knowledge.

UPDATE memories
SET content = 'CHUNK 50.3 (2026-05-11): Learned from rohitg00/agentmemory (DeepWiki analysis, MIT, no source copy) and implemented 3 improvements. (1) Privacy scrubbing: new memory/privacy.rs with strip_secrets() that applies 4 passes before storing memories — <private> tag removal, 20 prefix-based API key patterns (sk-ant-*, sk-*, ghp_*, AKIA*, AIza*, hf_*, glpat-*, xoxb-*, npm_*, etc.), JWT base64 triple-dot detection, and key-value secret name matching (password, api_key, secret, authorization, etc.). Uses LazyLock static vecs and native string ops, no regex crate. (2) Content-hash dedup at insert: MemoryStore::add() and add_many() now auto-compute SHA-256 of (scrubbed) content when source_hash is not caller-provided, then call find_by_source_hash() to return existing entry instead of creating a duplicate. This makes memory creation idempotent by content. (3) Circuit breaker: new brain/circuit_breaker.rs implementing CLOSED→OPEN→HALF_OPEN state machine (default: 3 failures in 60s trips to OPEN, 30s recovery timeout to HALF_OPEN, one probe request). Integrated into ProviderStatus in provider_rotator.rs — select_provider() and select_failover_chain() both check CB state after health gate. New FailoverReason::CircuitBreakerOpen. Public methods: record_request_success/failure(provider_id), circuit_breaker_state(provider_id). Follow-up agentmemory retrieval/consolidation ideas were implemented in Chunk 50.4. Tests: 12 privacy, 10 CB, 98 store, 41 rotator all pass.'
WHERE content LIKE 'CHUNK 50.3 (2026-05-11):%'
  AND content LIKE '%agentmemory patterns NOT adopted%';

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'CHUNK 50.4 (2026-05-11): Implemented the remaining agentmemory-derived memory improvements. (1) Consolidation synthesis: memory/consolidation.rs now groups active unparented persistent memories by graph neighbourhood first and tag fallback second, creates N-to-1 parent Summary rows tagged synthetic:consolidation/parent_summary, sets child parent_id via MemoryStore::set_parent_for_memories(), and writes parent -> child derived_from edges with edge_source consolidation_synthesis. ConsolidationResult exposes synthesized count and synthesized_parent_ids; config adds synthesis_min_children, synthesis_max_clusters, synthesis_max_children. (2) Session diversification: MemoryStore::hybrid_search_rrf and hybrid_search_rrf_with_intent now pass fused candidates through select_diversified_ranked(), capping non-empty session_id clusters at DEFAULT_MAX_RESULTS_PER_SESSION=3 while leaving global NULL-session long-term memories uncapped. Cache mode keys changed to rrf_vec_diverse/rrf_diverse. (3) Progressive disclosure search: new Tauri command progressive_search_memories(query, limit?, expand_ids?) returns compact ranked previews first and expands selected full MemoryEntry rows by ID; frontend types CompactMemoryResult/ProgressiveMemorySearchResponse and Pinia memory.progressiveSearch() expose it with a browser fallback. Follow-up validation taught cognitive_kind::classify to treat the common procedure tag alias as procedural, and stale tests were updated for content-hash dedup and targeted incremental retrieval. Docs updated in README and docs/brain-advanced-design.md. Tests: consolidation 10/10, store 99/99, cargo check, cargo clippy -D warnings, full cargo test --lib 2791/2791, vue-tsc clean, vitest 1806/1806.',
  'chunk-50.4,agentmemory,consolidation,synthesis,parent-id,session-diversification,rrf,progressive-search,compact-results,memory-store,architecture',
  9, 'procedure', 1747008000000, 'long', 1.0, 'memory', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'CHUNK 50.4 (2026-05-11):%'
);

UPDATE memories
SET content = content || ' Follow-up validation taught cognitive_kind::classify to treat the common procedure tag alias as procedural, and stale tests were updated for content-hash dedup and targeted incremental retrieval. Full cargo test --lib passed 2791/2791; vitest passed 1806/1806.'
WHERE content LIKE 'CHUNK 50.4 (2026-05-11):%'
  AND content NOT LIKE '%procedure tag alias%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed-migration-027', 1747008000000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CHUNK 50.4 (2026-05-11):%'
  AND (
       d.content LIKE 'CHUNK 50.3 (2026-05-11):%'
    OR d.content LIKE 'Memory module map:%'
    OR d.content LIKE 'Hybrid 6-signal search weights live in src-tauri/src/memory/store.rs%'
    OR d.content LIKE 'RAG pipeline:%'
  );
