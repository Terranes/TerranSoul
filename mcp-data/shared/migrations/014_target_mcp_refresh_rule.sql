-- ============================================================================
-- Migration 014 — target-mcp stale rebuild/relaunch rule
-- ============================================================================
-- Date: 2026-05-06
-- Trigger: stale target-mcp binaries were being reused while MCP on 7423 stayed
-- healthy, so updates were not terminated, rebuilt, and relaunched.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES
(
  'LESSON: For MCP 7423 startup, a healthy process is not sufficient if target-mcp is stale. If target-mcp/release/terransoul(.exe) is older than src-tauri sources/config, startup must terminate managed MCP, rebuild target-mcp, relaunch, and re-check /health. If termination fails, report a blocker instead of silently reusing stale binaries.',
  'lesson,mcp,target-mcp,stale-binary,rebuild,relaunch,startup',
  9, 'procedure', 1778025600000, 'long', 1.0, 'mcp', 'procedural'
),
(
  'RULE: Agents must not reuse MCP port 7423 when target-mcp is out of date. Required sequence is terminate -> rebuild -> relaunch -> health-check; skipping any step is a process violation because it hides stale runtime behavior.',
  'rule,mcp,target-mcp,stale-binary,terminate,rebuild,relaunch,health-check',
  9, 'procedure', 1778025600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: For MCP 7423 startup, a healthy process is not sufficient if target-mcp is stale%'
  AND (
       d.content LIKE 'MCP AUTO-START TASK:%'
    OR d.content LIKE 'MCP PREFLIGHT ENFORCEMENT:%'
    OR d.content LIKE 'LESSON: MCP-mode self-improve runtime logs live under mcp-data/%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: Agents must not reuse MCP port 7423 when target-mcp is out of date%'
  AND d.content LIKE 'LESSON: For MCP 7423 startup, a healthy process is not sufficient if target-mcp is stale%';
