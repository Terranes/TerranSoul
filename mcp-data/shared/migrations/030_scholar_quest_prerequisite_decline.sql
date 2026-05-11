-- Scholar's Quest opens with prerequisite decline instead of an in-quest Verify Brain step.

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category)
SELECT
  'LESSON: Scholar''s Quest should not show a Verify Brain step. Entry into KnowledgeQuestDialog gates on unmet prerequisites; if any prerequisites remain, show a prerequisite decline modal listing the missing quests with Cancel and Start Now. Start Now launches the Learn Docs prerequisite setup flow; otherwise the first quest step is Gather Sources.',
  'lesson,scholar-quest,verify-brain,prerequisite-gate,learn-docs,knowledge-quest',
  9,
  'procedure',
  1778544000000,
  'long',
  1.0,
  58,
  'brain'
WHERE NOT EXISTS (
  SELECT 1 FROM memories
  WHERE content LIKE 'LESSON: Scholar''s Quest should not show a Verify Brain step.%'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778544000000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: Scholar''s Quest should not show a Verify Brain step%'
  AND (
       d.content LIKE 'LESSON: Scholar''s Quest is the document-learning chain target%'
    OR d.content LIKE 'DEFAULT SYSTEM SETTING: Intent classifier document-learning setup%'
  );