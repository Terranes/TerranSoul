-- Migration 013: Record Chunk 41.7 — Embedding worker concurrency + rate limiting
-- Applied: 2026-05-07

INSERT OR REPLACE INTO memory_nodes (id, kind, content, tags, importance, created_at)
VALUES
  ('phase41_7_worker_concurrency', 'architecture',
   'Chunk 41.7 refactored the embedding_queue background worker for production-grade concurrency: provider-adaptive batch sizes (ollama=8, free=32, paid=128), tokio::Semaphore(4) concurrency cap, soft-pause vs hard-fail distinction for rate limits (429/busy/quota detected heuristically via is_rate_limit_error), exponential backoff (30s*2^n, max 600s) on consecutive rate-limit batches, lock-free WorkerMetrics (Arc<Atomic*>) exposed via brain_health as embed_worker field, and graceful shutdown via tokio::sync::watch<bool> channel. The worker uses all-None embed_batch_for_mode results as a rate-limit heuristic since HTTP status codes are not surfaced from the embedding layer.',
   'embedding,worker,concurrency,rate-limit,semaphore,backoff,brain_health,shutdown', 8,
   strftime('%s', 'now'));

INSERT OR IGNORE INTO memory_edges (source_id, target_id, relation)
VALUES
  ('phase41_7_worker_concurrency', 'phase41_6_reembed_on_update', 'follows'),
  ('phase41_7_worker_concurrency', 'phase38_2_pending_embeddings', 'extends');
