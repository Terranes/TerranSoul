INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP HEALTH RESPONSE EXPLANATION (2026-05-06): brain_health plus GET /health and GET /status keep backward-compatible rag_quality_pct and memory_total fields, but now also return rag_quality, memory, and descriptions objects. rag_quality_pct means embedded_long_memory_count / long_memory_count * 100, not an overall intelligence score. A 12% value means only about 12% of long-term memories currently have vector embeddings; keyword/RRF and graph lookup still work, but semantic vector recall is partial until pending embedding backfill completes. Use the nested raw counts and description strings when displaying health JSON to humans.',
  'mcp,health,rag-quality,embedding-coverage,json,docs,non-negotiable',
  10, 'fact', 1746489600000, 'long', 1.0, 'mcp', 'semantic'
);