-- ============================================================================
-- Migration 015 — visible MCP preflight receipt rule
-- ============================================================================
-- Date: 2026-05-06
-- Trigger: MCP tools were being used invisibly or inconsistently, so the user
-- could not verify that agents actually used TerranSoul MCP.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES
(
  'RULE: TerranSoul MCP preflight must be visible to the user, not only hidden in tool calls. After brain_health plus a relevant brain_search or brain_suggest_context succeeds, the agent must send a short MCP receipt naming the health/provider result and the search/context query topic. If MCP is blocked, the receipt must name the blocker. If the user cannot see the receipt, treat preflight as incomplete.',
  'rule,mcp,preflight,visible-receipt,user-visible,enforcement,non-negotiable',
  10, 'procedure', 1778025600000, 'long', 1.0, 'mcp', 'procedural'
),
(
  'LESSON: Text-only MCP rules were not enough because users cannot see hidden tool calls. The durable fix is to require a visible MCP receipt in .github/instructions/mcp-preflight.instructions.md, .github/copilot-instructions.md, AGENTS.md, CLAUDE.md, .cursorrules, rules/agent-mcp-bootstrap.md, and MCP seed memory.',
  'lesson,mcp,preflight,visible-receipt,instruction-sync,user-trust',
  9, 'procedure', 1778025600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: TerranSoul MCP preflight must be visible to the user%'
  AND (
       d.content LIKE 'MCP EVERY-SESSION RULE:%'
    OR d.content LIKE 'MCP PREFLIGHT ENFORCEMENT:%'
    OR d.content LIKE 'MCP PREFLIGHT INSTRUCTIONS FILE:%'
    OR d.content LIKE 'RULES ENFORCEMENT BUNDLE:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: Text-only MCP rules were not enough%'
  AND d.content LIKE 'RULE: TerranSoul MCP preflight must be visible to the user%';
