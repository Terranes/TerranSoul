-- ============================================================================
-- Migration 016 -- shared seed resolution across dev/release/MCP modes
-- ============================================================================
-- Date: 2026-05-06
-- Trigger: dev/release runs could miss repo-shared migrations/config updates
-- because only <data_dir>/shared was checked before compiled fallback.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES
(
  'RULE: Migration/schema bootstrap must resolve shared seed sources in deterministic order across release, dev, and MCP modes: TERRANSOUL_MCP_SHARED_DIR (override) -> <data_dir>/shared -> <cwd>/mcp-data/shared -> compiled fallback. This guarantees predictable boot while letting local dev/release runs consume checked-in mcp-data/shared updates without manual copy steps.',
  'rule,mcp,seed,migrations,schema,dev,release,shared-data,resolution-order',
  9, 'procedure', 1778025600000, 'long', 1.0, 'mcp', 'procedural'
),
(
  'LESSON: Relying only on <data_dir>/shared for seed migrations caused dev/release drift from repository mcp-data/shared when no runtime shared folder existed. The durable fix is explicit source resolution plus startup logging of the selected source, with compiled SQL as the final fallback.',
  'lesson,mcp,seed,migrations,schema,drift,dev,release,fallback',
  9, 'procedure', 1778025600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: Migration/schema bootstrap must resolve shared seed sources in deterministic order%'
  AND (
       d.content LIKE 'MCP shared data policy:%'
    OR d.content LIKE 'MCP server exposes brain on three ports:%'
    OR d.content LIKE 'LESSON: MCP-mode self-improve runtime logs live under mcp-data/%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: Relying only on <data_dir>/shared for seed migrations caused dev/release drift%'
  AND d.content LIKE 'RULE: Migration/schema bootstrap must resolve shared seed sources in deterministic order%';
