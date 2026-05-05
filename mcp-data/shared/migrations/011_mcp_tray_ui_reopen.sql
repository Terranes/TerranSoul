-- ============================================================================
-- Migration 011 — MCP tray full-UI reopen lifecycle
-- ============================================================================
-- Date: 2026-05-06
-- Trigger: Closing the full MCP UI window could leave tray "Show UI" unable to
-- reopen configuration panels if the main WebView was destroyed.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES
(
  'LESSON: MCP tray mode must keep the full TerranSoul UI reopenable. Tray Show UI must not assume the main WebView still exists; it must show/focus an existing main window or recreate the full UI window when missing. In MCP tray mode, the OS close button should hide only the main window and skip its taskbar entry, while child panel windows may close normally. The tray Exit menu remains the explicit quit path.',
  'lesson,mcp,tray,ui-window,lifecycle,reopen,full-ui,non-headless',
  9, 'procedure', 1778025600000, 'long', 1.0, 'mcp', 'procedural'
),
(
  'RULE: MCP full UI mode is required for interactive brain configuration and memory graph control. Do not regress npm run mcp into headless-only behavior; the tray UI path must expose the normal Vue app shell so users can inspect MCP config, provider state, memory, and graph panels while the HTTP MCP server remains running.',
  'rule,mcp,full-ui,brain-config,memory-graph,ui,non-headless',
  9, 'procedure', 1778025600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: MCP tray mode must keep the full TerranSoul UI reopenable%'
  AND (
       d.content LIKE 'MCP server exposes brain on three ports:%'
    OR d.content LIKE 'MCP EVERY-SESSION RULE:%'
    OR d.content LIKE 'MCP shared data policy:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: MCP full UI mode is required for interactive brain configuration%'
  AND d.content LIKE 'LESSON: MCP tray mode must keep the full TerranSoul UI reopenable%';
