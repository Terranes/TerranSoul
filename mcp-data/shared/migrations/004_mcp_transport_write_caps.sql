-- ============================================================================
-- Migration 004 — MCP transport write capability fix
-- ============================================================================
-- Date: 2026-05-05
-- Trigger: MCP tool call returned "permission denied: capability `brain_write`"
--          when the coding agent tried to ingest durable research knowledge.
--
-- Root cause: GatewayCaps::default() correctly remains read-only, but the
-- HTTP and stdio MCP transports were also using read-only caps. Explicit MCP
-- transports are already trusted/authenticated surfaces, so they need the
-- READ_WRITE profile while the gateway default stays safe for embedders/tests.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP WRITE CAPABILITY FIX (2026-05-05): MCP transports must use the explicit transport capability profile, not GatewayCaps::default(). Root cause of brain_ingest_url permission denied was src-tauri/src/ai_integrations/mcp/mod.rs constructing brain_write=false and stdio using GatewayCaps::default(). Fix: mcp::transport_caps() returns GatewayCaps::READ_WRITE for HTTP bearer-token MCP and trusted stdio MCP. GatewayCaps::default remains read-only for tests/future embedders. This allows coding agents to persist durable self-improve/research knowledge through brain_ingest_url while preserving fail-closed defaults outside MCP.',
  'mcp,brain_write,capabilities,permission-fix,self-improve,transport-caps',
  9, 'procedure', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP WRITE CAPABILITY FIX:%'
  AND (
       d.content LIKE 'MCP EVERY-SESSION RULE:%'
    OR d.content LIKE 'SELF-IMPROVE WRITE-BACK CONTRACT:%'
    OR d.content LIKE 'MCP SELF-LEARNING RULE:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP WRITE CAPABILITY FIX:%'
  AND d.content LIKE 'MCP server exposes brain on three ports:%';
