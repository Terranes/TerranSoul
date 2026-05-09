-- Migration 013: Record Chunk 41.7 — Embedding worker concurrency + rate limiting
-- Applied: 2026-05-07

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'Chunk 41.7 refactored the embedding_queue background worker for production-grade concurrency: provider-adaptive batch sizes (ollama=8, free=32, paid=128), tokio::Semaphore(4) concurrency cap, soft-pause vs hard-fail distinction for rate limits (429/busy/quota detected heuristically via is_rate_limit_error), exponential backoff (30s*2^n, max 600s) on consecutive rate-limit batches, lock-free WorkerMetrics (Arc<Atomic*>) exposed via brain_health as embed_worker field, and graceful shutdown via tokio::sync::watch<bool> channel. The worker uses all-None embed_batch_for_mode results as a rate-limit heuristic since HTTP status codes are not surfaced from the embedding layer.',
  'embedding,worker,concurrency,rate-limit,semaphore,backoff,brain_health,shutdown',
  8, 'fact', strftime('%s', 'now'), 'long', 1.0, 'architecture', 'semantic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT src.id, dst.id, 'related_to', 1.0, 'migration', strftime('%s', 'now'), 'migration'
FROM memories src
JOIN memories dst ON dst.content LIKE 'PHASE 41.6 LESSON (2026-05-07): Re-embed on content update shipped.%'
WHERE src.content LIKE 'Chunk 41.7 refactored the embedding_queue background worker for production-grade concurrency:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT src.id, dst.id, 'related_to', 1.0, 'migration', strftime('%s', 'now'), 'migration'
FROM memories src
JOIN memories dst ON dst.content LIKE 'PHASE 38.2 LESSON (2026-04-30):%'
WHERE src.content LIKE 'Chunk 41.7 refactored the embedding_queue background worker for production-grade concurrency:%';
