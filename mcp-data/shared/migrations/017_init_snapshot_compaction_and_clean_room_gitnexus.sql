-- ============================================================================
-- Migration 017 -- init snapshot compaction + clean-room GitNexus verdict
-- ============================================================================
-- Date: 2026-05-06
-- Trigger: requested consolidation of historical seed migrations into one
-- fresh-db init baseline and request to import GitNexus directly.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES
(
  'RULE: Fresh MCP databases should bootstrap from a single init snapshot (mcp-data/shared/memory-seed.sql) and then apply only future numbered deltas. Keep numbered migrations append-only for compatibility/history, but avoid replaying all historical scripts on first boot.',
  'rule,mcp,seed,migrations,init-snapshot,bootstrap,performance',
  9, 'procedure', 1778025600000, 'long', 1.0, 'mcp', 'procedural'
),
(
  'VERDICT: Reject direct GitNexus import/bundling. TerranSoul must keep clean-room native code-intelligence UX inspired by public behavior only; no GitNexus binaries, Docker images, prompts, skills, or UI assets may be bundled, auto-installed, or default-spawned due PolyForm Noncommercial constraints.',
  'verdict,gitnexus,clean-room,license,ui-ux,native,noncommercial',
  10, 'decision', 1778025600000, 'long', 1.0, 'code-intelligence', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: Fresh MCP databases should bootstrap from a single init snapshot%'
  AND (
       d.content LIKE 'MCP shared data policy:%'
    OR d.content LIKE 'RULE: Migration/schema bootstrap must resolve shared seed sources in deterministic order%'
    OR d.content LIKE 'LESSON: MCP-mode self-improve runtime logs live under mcp-data/%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'VERDICT: Reject direct GitNexus import/bundling.%'
  AND (
       d.content LIKE 'LESSON: GitNexus is PolyForm Noncommercial.%'
    OR d.content LIKE 'CODE WORKBENCH UX LESSON:%'
  );
