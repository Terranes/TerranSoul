-- 2026-05-07: Phase 41.2 + 41.3 durable result sync.

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PHASE 41.2/41.3 LESSON (2026-05-07): added per-op MemoryMetrics histograms/timers for memory CRUD and retrieval, exposed via get_memory_metrics and MCP health metrics. The million_memory CRUD bench now includes update/mixed/delete sections with hard timeout guards (TS_BENCH_TIMEOUT_SECS, default 300) so runs cannot appear infinite. At 1M scale, write/read remain the headline SLO and stay comfortably under 60s/5s; mixed workload can be much slower due to post-write index pressure, and benchmark delete is skipped at 1M because secondary-index maintenance dominates runtime and is not representative of the write/read objective.',
  'phase-41,chunk-41.2,chunk-41.3,metrics,benchmark,timeout,million-memory,sqlite',
  10, 'fact', 1746579600000, 'long', 1.0, 'memory', 'episodic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT newer.id, older.id, 'related_to', 1.0, 'auto', 1746579600000, 'auto'
FROM memories newer
CROSS JOIN memories older
WHERE newer.content LIKE 'PHASE 41.2/41.3 LESSON (2026-05-07)%'
  AND older.content LIKE 'PHASE 41 RESULT (2026-05-07)%';
