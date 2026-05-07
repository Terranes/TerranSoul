-- Migration 014: Record Chunk 41.8 — Multi-model embedding columns + AnnRegistry (V16 schema)
-- Applied: 2026-05-07

INSERT OR REPLACE INTO memory_nodes (id, kind, content, tags, importance, created_at)
VALUES
  ('phase41_8_multi_model_embeddings', 'architecture',
   'Chunk 41.8 added V16 schema: memory_embeddings side table keyed by (memory_id, model_id) with dim, embedding BLOB, created_at. memories table gains embedding_model_id TEXT and embedding_dim INTEGER columns. AnnRegistry in ann_index.rs manages per-model HNSW indices persisted as vectors_<model_id>.usearch. Store APIs: set_embedding_for_model(), get_embedding_for_model(), embedding_models_for(), vector_search_model() with brute-force fallback, backfill_embedding_model(). Tauri command backfill_embedding_model_id tags existing embeddings. Schema upgrade path via ensure_multi_model_embeddings(). Switching embedding models no longer invalidates all vectors.',
   'embedding,multi-model,schema-v16,ann-registry,side-table,backfill', 8,
   strftime('%s', 'now'));

INSERT OR IGNORE INTO memory_edges (source_id, target_id, relation)
VALUES
  ('phase41_8_multi_model_embeddings', 'phase41_7_worker_concurrency', 'follows'),
  ('phase41_8_multi_model_embeddings', 'phase38_3_native_ann', 'extends');
