-- ============================================================================
-- Migration 005 — MCP HTTP ingest sink wiring
-- ============================================================================
-- Date: 2026-05-05
-- Trigger: After granting MCP brain_write, brain_ingest_url reached the write
--          path but failed with "not configured: ingest sink not attached".
--
-- Root cause: AppStateGateway supports an optional IngestSink, but MCP HTTP
-- server construction still used AppStateGateway::new() even in Tauri app/tray
-- mode where an AppHandle is available.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP INGEST SINK FIX (2026-05-05): brain_ingest_url requires both brain_write caps and a production IngestSink. After fixing caps, MCP returned "not configured: ingest sink not attached" because src-tauri/src/ai_integrations/mcp/mod.rs still created AppStateGateway::new(). Fix: when start_server_with_activity receives a Tauri AppHandle, construct AppHandleIngestSink and AppStateGateway::with_ingest(); the sink calls commands::ingest::ingest_document and returns IngestUrlResponse task_id/source/source_type. HTTP MCP app/tray mode can now start real background ingest tasks. Stdio remains a trusted transport for reads/write caps, but URL ingestion requires an AppHandle-backed process unless a direct non-UI ingest sink is added later.',
  'mcp,ingest_url,ingest-sink,apphandle,brain_write,self-improve',
  9, 'procedure', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP INGEST SINK FIX:%'
  AND (
       d.content LIKE 'MCP WRITE CAPABILITY FIX:%'
    OR d.content LIKE 'MCP server exposes brain on three ports:%'
    OR d.content LIKE 'ai_integrations exposes the brain%'
  );
