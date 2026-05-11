-- Learn Docs live prerequisite precheck and collapsed chat thinking status.

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category)
SELECT
  'LESSON: Learn Docs must precheck Scholar''s Quest prerequisites from live app state before deciding whether Sage''s Library or Scholar''s Quest should start. Saved quest completion can be stale if the user removes a brain mode, Local Ollama setup, or memories. Attach the precheck to the resulting chat prompt as collapsed thinkingContent so users can see why the hotseat shows Install Sage''s Library + Cancel or the Scholar''s Quest start prompt.',
  'lesson,learn-docs,scholar-quest,rag-knowledge,sage-library,precheck,thinking-content,chat-ui',
  9,
  'procedure',
  1778630400000,
  'long',
  1.0,
  76,
  'brain'
WHERE NOT EXISTS (
  SELECT 1 FROM memories
  WHERE content LIKE 'LESSON: Learn Docs must precheck Scholar''s Quest prerequisites from live app state%'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778630400000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: Learn Docs must precheck Scholar''s Quest prerequisites from live app state%'
  AND (
       d.content LIKE 'LESSON: Scholar''s Quest should not show a Verify Brain step%'
    OR d.content LIKE 'LESSON: Scholar''s Quest is the document-learning chain target%'
  );