-- ============================================================================
-- Migration 006 — MCP stdio ingest sink wiring
-- ============================================================================
-- Date: 2026-05-05
-- Trigger: VS Code MCP tooling was connected through --mcp-stdio, so
--          brain_ingest_url still returned "not configured: ingest sink not
--          attached" even after the HTTP tray path was fixed.
--
-- Root cause: stdio::run_with_state used AppStateGateway::new(state). Stdio
-- has no Tauri AppHandle, so it needs a direct AppState-backed sink that starts
-- the same ingest pipeline without UI progress events.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP STDIO INGEST SINK FIX (2026-05-05): editor MCP clients may connect through terransoul --mcp-stdio rather than HTTP 7423. Stdio already uses READ_WRITE caps, but brain_ingest_url still failed with "not configured: ingest sink not attached" because stdio::run_with_state constructed AppStateGateway::new(state). Fix: attach StdioIngestSink via AppStateGateway::with_ingest(); the sink calls commands::ingest::ingest_document_silent(), which starts the real background ingest pipeline against AppState without requiring a Tauri AppHandle or emitting WebView progress events. Both HTTP MCP tray and stdio MCP now support brain_ingest_url writes.',
  'mcp,stdio,ingest_url,ingest-sink,brain_write,self-improve',
  9, 'procedure', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP STDIO INGEST SINK FIX:%'
  AND (
       d.content LIKE 'MCP INGEST SINK FIX:%'
    OR d.content LIKE 'MCP WRITE CAPABILITY FIX:%'
    OR d.content LIKE 'MCP server exposes brain on three ports:%'
    OR d.content LIKE 'ai_integrations exposes the brain%'
  );
