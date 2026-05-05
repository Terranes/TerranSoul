-- ============================================================================
-- Migration 013 — MCP self-improve rolling runtime logs
-- ============================================================================
-- Date: 2026-05-06
-- Trigger: Keep MCP-mode self-improve runtime logs in mcp-data with bounded
-- current-plus-.001 rotation instead of unbounded append-only files.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES
(
  'LESSON: MCP-mode self-improve runtime logs live under mcp-data/ and are bounded runtime state, not durable project memory. self_improve_runs.jsonl, self_improve_gates.jsonl, and self_improve_mcp.jsonl each keep only the current file plus a .001 archive, with a 1 MiB cap per file. The UI reads both current and archive, while durable lessons still belong in mcp-data/shared/memory-seed.sql and numbered migrations.',
  'lesson,mcp,self-improve,logs,rolling-log,mcp-data,jsonl,runtime-state',
  9, 'procedure', 1778025600000, 'long', 1.0, 'mcp', 'procedural'
),
(
  'RULE: New self-improve runtime logs must use coding::rolling_log or an equivalent current-plus-.001 rotation policy before writing under mcp-data/. Do not create unbounded MCP runtime logs, do not commit runtime logs, and do not treat runtime JSONL as the durable MCP knowledge source.',
  'rule,mcp,self-improve,logs,rotation,runtime-state,shared-seed',
  9, 'procedure', 1778025600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: MCP-mode self-improve runtime logs live under mcp-data/%'
  AND (
       d.content LIKE 'MCP shared data policy:%'
    OR d.content LIKE 'Self-improve with MCP mode:%'
    OR d.content LIKE 'LESSON: Do not commit MCP runtime state.%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: New self-improve runtime logs must use coding::rolling_log%'
  AND d.content LIKE 'LESSON: MCP-mode self-improve runtime logs live under mcp-data/%';