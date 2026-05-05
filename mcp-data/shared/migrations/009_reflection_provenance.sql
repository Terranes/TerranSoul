-- ============================================================================
-- Migration 009 — /reflect provenance persistence lesson
-- ============================================================================
-- Date: 2026-05-05
-- Trigger: Chunk 33B.2 added reflect_on_session and memory/reflection.rs.
--
-- Durable lesson: provenance-linked brain write paths need concrete inserted
-- memory IDs before writing memory_edges. The older save_summary/save_facts
-- helpers intentionally do not return IDs, so they are insufficient for a
-- derived_from source-turn graph.
-- ============================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'LESSON: For provenance-linked brain write paths such as /reflect, use a dedicated persistence helper that returns concrete inserted memory IDs before writing memory_edges. Generic save_summary/save_facts helpers are fine for simple writes but do not expose IDs, so they cannot create reliable derived_from source-turn edges. Chunk 33B.2 implements this in memory/reflection.rs and reflect_on_session.',
  'lesson,memory,provenance,reflection,brain-write,chunk-33b.2',
  9, 'procedure', 1746489600000, 'long', 1.0, 'lessons', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: For provenance-linked brain write paths such as /reflect%'
  AND (
       d.content LIKE 'Memory module map:%'
    OR d.content LIKE 'RULE (provenance):%'
    OR d.content LIKE 'MCP SELF-LEARNING RULE:%'
    OR d.content LIKE 'DOCUMENTATION SYNC RULES:%'
  );