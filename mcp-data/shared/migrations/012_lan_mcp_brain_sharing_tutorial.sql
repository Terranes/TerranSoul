-- ============================================================================
-- Migration 012 — LAN MCP brain sharing tutorial and rules
-- ============================================================================
-- Date: 2026-05-05
-- Trigger: Document the full local TerranSoul host MCP server flow where other
-- LAN TerranSouls connect to retrieve query-scoped brain data.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES
(
  'LESSON: LAN MCP brain sharing is an opt-in local-network retrieval flow. The host must enable LAN brain sharing before starting or restarting the MCP server, then name the shared brain and share the bearer token out-of-band. Discovery uses UDP 7424 for metadata only; authenticated retrieval uses MCP HTTP brain_search against the host port. Peers retrieve ranked snippets, not the host memory database.',
  'lesson,mcp,lan,brain-sharing,discovery,token,remote-search,tutorial',
  9, 'procedure', 1777939200000, 'long', 1.0, 'mcp', 'procedural'
),
(
  'RULE: Treat LAN MCP bearer-token access as read access to the shared TerranSoul knowledge surface. Never broadcast the token, never enable LAN sharing on public Wi-Fi, and stop sharing when the session ends. User-facing docs should describe this with the docs/lan-mcp-sharing-tutorial.md Alice Vietnamese law notes scenario and avoid legal-advice claims.',
  'rule,mcp,lan,security,bearer-token,docs,legal-disclaimer',
  9, 'procedure', 1777939200000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1777939200000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: LAN MCP brain sharing is an opt-in local-network retrieval flow%'
  AND (
       d.content LIKE 'MCP server exposes brain on three ports:%'
    OR d.content LIKE 'RULE: MCP full UI mode is required for interactive brain configuration%'
    OR d.content LIKE 'MCP shared data policy:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1777939200000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: Treat LAN MCP bearer-token access as read access%'
  AND d.content LIKE 'LESSON: LAN MCP brain sharing is an opt-in local-network retrieval flow%';