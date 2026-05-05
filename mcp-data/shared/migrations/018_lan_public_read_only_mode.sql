-- ============================================================================
-- Migration 018 -- LAN public read-only auth mode
-- ============================================================================
-- Date: 2026-05-05
-- Trigger: add an explicit LAN toggle between token-required access and a
-- restricted public read-only MCP surface.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES
(
  'RULE: LAN MCP sharing must expose an explicit auth mode choice: `token_required` or `public_read_only`. Public mode may skip the bearer token only for the read-only brain MCP surface (initialize, ping, tools/list, and read-only brain tools); write tools, code-intelligence tools, /status, and hook endpoints remain authenticated.',
  'rule,mcp,lan,auth-mode,public-read-only,token-required,security',
  9, 'procedure', 1778112000000, 'long', 1.0, 'mcp', 'procedural'
),
(
  'LESSON: LAN discovery should advertise whether a TerranSoul host requires a token, but it must never broadcast the token itself. UI flows should hide the token field for public-read-only peers and still treat public mode as read access to the shared knowledge surface.',
  'lesson,mcp,lan,discovery,auth-mode,public-read-only,token-ui',
  8, 'procedure', 1778112000000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778112000000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: LAN MCP sharing must expose an explicit auth mode choice%'
  AND (
       d.content LIKE 'LESSON: LAN MCP brain sharing is an opt-in local-network retrieval flow%'
    OR d.content LIKE 'RULE: Treat LAN MCP bearer-token access as read access%'
    OR d.content LIKE 'MCP server exposes brain on three ports:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778112000000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: LAN discovery should advertise whether a TerranSoul host requires a token%'
  AND d.content LIKE 'RULE: LAN MCP sharing must expose an explicit auth mode choice%';