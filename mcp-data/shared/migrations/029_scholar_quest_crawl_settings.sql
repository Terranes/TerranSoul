-- Scholar's Quest: prerequisite-only Learn Docs setup and persisted crawl bounds.

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category)
SELECT
  'LESSON: Scholar''s Quest is the document-learning chain target, not an installable prerequisite. The Learn Docs flow should only auto-install unmet prerequisites through Sage''s Library (`rag-knowledge`), then start Scholar''s Quest. Scholar crawl settings are persisted in AppSettings (`scholar_crawl_enabled`, `scholar_crawl_max_depth`, `scholar_crawl_max_pages`) and the ingest backend preserves custom crawl bounds in canonical `crawl:depth=<n>,pages=<n>:<url>` task sources for resume safety.',
  'lesson,scholar-quest,learn-docs,rag-knowledge,crawl,app-settings,ingest',
  9,
  'procedure',
  1778544000000,
  'long',
  1.0,
  80,
  'brain'
WHERE NOT EXISTS (
  SELECT 1 FROM memories
  WHERE content LIKE 'LESSON: Scholar''s Quest is the document-learning chain target, not an installable prerequisite.%'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778544000000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: Scholar''s Quest is the document-learning chain target%'
  AND (
       d.content LIKE 'DEFAULT SYSTEM SETTING: Intent classifier document-learning setup%'
    OR d.content LIKE 'Batched embedding pipeline%'
    OR d.content LIKE 'commands/ files%ingest.rs%'
  );
