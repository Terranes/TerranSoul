-- Migration 014: Record Chunk 41.8 — Multi-model embedding columns + AnnRegistry (V16 schema)
-- Applied: 2026-05-07

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'Chunk 41.8 added V16 schema: memory_embeddings side table keyed by (memory_id, model_id) with dim, embedding BLOB, created_at. memories table gains embedding_model_id TEXT and embedding_dim INTEGER columns. AnnRegistry in ann_index.rs manages per-model HNSW indices persisted as vectors_<model_id>.usearch. Store APIs: set_embedding_for_model(), get_embedding_for_model(), embedding_models_for(), vector_search_model() with brute-force fallback, backfill_embedding_model(). Tauri command backfill_embedding_model_id tags existing embeddings. Schema upgrade path via ensure_multi_model_embeddings(). Switching embedding models no longer invalidates all vectors.',
  'embedding,multi-model,schema-v16,ann-registry,side-table,backfill',
  8, 'fact', strftime('%s', 'now'), 'long', 1.0, 'architecture', 'semantic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT src.id, dst.id, 'related_to', 1.0, 'migration', strftime('%s', 'now'), 'migration'
FROM memories src
JOIN memories dst ON dst.content LIKE 'Chunk 41.7 refactored the embedding_queue background worker for production-grade concurrency:%'
WHERE src.content LIKE 'Chunk 41.8 added V16 schema: memory_embeddings side table keyed by (memory_id, model_id)%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT src.id, dst.id, 'related_to', 1.0, 'migration', strftime('%s', 'now'), 'migration'
FROM memories src
JOIN memories dst ON dst.content LIKE 'PHASE 38.3 LESSON (2026-04-30):%'
WHERE src.content LIKE 'Chunk 41.8 added V16 schema: memory_embeddings side table keyed by (memory_id, model_id)%';
