-- 2026-05-07: Phase 41 measured result (chunks 41.1 + 41.4 shipped).

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PHASE 41 RESULT (2026-05-07): chunks 41.1 (SQLite PRAGMA tuning) and 41.4 (transactional add_many) shipped together. Measured on commodity dev hardware via cargo bench --bench million_memory: 10k smoke write 0.04s @ 244,657 rows/s, read 0.01s @ 939,956 rows/s. 1M full (TS_BENCH_SCALES=1000000 TS_BENCH_CRUD_ONLY=1) write 6.37s @ 157,031 rows/s, read 1.84s @ 544,704 rows/s. The Phase 41 headline target of 1M write under 60s and 1M read under 5s is met by ~10x and ~3x respectively, on top of 41.1 + 41.4 alone. Remaining Phase 41 chunks remain valuable for production hardening but are no longer blocking the headline throughput goal. Implementation: add_many uses prepare_cached + single transaction, returns Vec<i64> of assigned ids, takes &mut self. PRAGMAs applied at MemoryStore::new: journal_mode=WAL, synchronous=NORMAL, foreign_keys=ON, cache_size=-65536, mmap_size=268435456, temp_store=MEMORY, busy_timeout=5000, wal_autocheckpoint=1000, journal_size_limit=67108864. FTS5 verification deferred -- schema.rs does not declare an FTS5 virtual table today.',
  'phase-41,result,bench,million,sqlite,add_many,pragma,measured,non-negotiable',
  10, 'fact', 1746576000000, 'long', 1.0, 'memory', 'episodic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT result.id, plan.id, 'fulfills', 1.0, 'auto', 1746576000000, 'auto'
FROM memories result
CROSS JOIN memories plan
WHERE result.content LIKE 'PHASE 41 RESULT (2026-05-07)%'
  AND plan.content LIKE 'PHASE 41 PLAN (2026-05-06)%';
