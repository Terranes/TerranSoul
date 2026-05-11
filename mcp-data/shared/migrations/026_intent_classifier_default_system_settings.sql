-- 026_intent_classifier_default_system_settings.sql
-- Move intent-classifier examples/defaults out of Rust prompt constants and
-- into user-customizable system default setting memories with KG edges.

UPDATE memories
SET
  content = 'DEFAULT SYSTEM SETTING: Intent classifier document-learning setup (2026-05-10): When a user says "Learn from my documents" or asks to learn/study from their own documents, files, notes, PDFs, or sources, classify the turn as learn_with_docs. Phrase examples include "Learn from my documents", "Learn my documents", "Learn documents", "Please look at my provided documents and learn it", "I want you to read my files", "Can you study these documents for me?", "Read and learn from my notes", and "học luật Việt Nam từ tài liệu của tôi". If they do not specify a topic or setup mode, use topic "the material in your documents" and assume the recommended setup flow. The recommended flow is the Scholar''s Quest prerequisite chain: free-brain (Awaken the Mind) -> memory (Long-Term Memory) -> rag-knowledge (Sage''s Library, 6-signal hybrid RAG) -> scholar-quest (document ingestion/attachment). If classifier JSON is malformed or unknown, return Unknown and continue normal chat/install flows; never force docs/setup routing via regex, contains, includes, or keyword arrays. This is the behavior described by tutorials/brain-rag-setup-tutorial.md.',
  tags = 'system,default-system-setting,system:default-system-setting,rule,intent-classifier,learn-with-docs,documents,rag,quest,scholar-quest,recommended-setup,tutorial,no-heuristic-fallback,user-customizable',
  importance = 10,
  memory_type = 'preference',
  token_count = 170,
  category = 'system.default_system_setting',
  cognitive_kind = 'procedural',
  protected = 1
WHERE content LIKE 'RULE: Intent classifier document-learning setup (2026-05-10):%';

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind, protected)
SELECT
  'DEFAULT SYSTEM SETTING: Intent classifier document-learning setup (2026-05-10): When a user says "Learn from my documents" or asks to learn/study from their own documents, files, notes, PDFs, or sources, classify the turn as learn_with_docs. Phrase examples include "Learn from my documents", "Learn my documents", "Learn documents", "Please look at my provided documents and learn it", "I want you to read my files", "Can you study these documents for me?", "Read and learn from my notes", and "học luật Việt Nam từ tài liệu của tôi". If they do not specify a topic or setup mode, use topic "the material in your documents" and assume the recommended setup flow. The recommended flow is the Scholar''s Quest prerequisite chain: free-brain (Awaken the Mind) -> memory (Long-Term Memory) -> rag-knowledge (Sage''s Library, 6-signal hybrid RAG) -> scholar-quest (document ingestion/attachment). If classifier JSON is malformed or unknown, return Unknown and continue normal chat/install flows; never force docs/setup routing via regex, contains, includes, or keyword arrays. This is the behavior described by tutorials/brain-rag-setup-tutorial.md.',
  'system,default-system-setting,system:default-system-setting,rule,intent-classifier,learn-with-docs,documents,rag,quest,scholar-quest,recommended-setup,tutorial,no-heuristic-fallback,user-customizable',
  10, 'preference', 1778371200000, 'long', 1.0, 170, 'system.default_system_setting', 'procedural', 1
WHERE NOT EXISTS (
  SELECT 1 FROM memories
  WHERE content LIKE 'DEFAULT SYSTEM SETTING: Intent classifier document-learning setup%'
     OR content LIKE 'RULE: Intent classifier document-learning setup (2026-05-10):%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind, protected)
SELECT
  'DEFAULT SYSTEM SETTING: Intent classifier teach-ingest setup (2026-05-10): When the user is pasting content or a source for the brain to remember, classify the turn as teach_ingest. Phrase examples include "remember the following law", "ingest this URL", "memorize this fact", "here is the source", and similar paraphrases in any language. Return a short topic phrase that names the source/content when one is present; otherwise use a concise generic topic. This setting is user-customizable through system default setting memories tagged intent-classifier.',
  'system,default-system-setting,system:default-system-setting,rule,intent-classifier,teach-ingest,documents,rag,source-ingest,user-customizable',
  10, 'preference', 1778371200000, 'long', 1.0, 92, 'system.default_system_setting', 'procedural', 1
WHERE NOT EXISTS (
  SELECT 1 FROM memories
  WHERE content LIKE 'DEFAULT SYSTEM SETTING: Intent classifier teach-ingest setup%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind, protected)
SELECT
  'DEFAULT SYSTEM SETTING: Intent classifier gated-setup setup (2026-05-10): When the user explicitly asks to upgrade to Gemini, configure Gemini, or use Gemini, classify as gated_setup with setup "upgrade_gemini". When the user asks to connect, add, provide, or supply context to the brain/app, classify as gated_setup with setup "provide_context". Keep ordinary questions as chat. This setting is user-customizable through system default setting memories tagged intent-classifier.',
  'system,default-system-setting,system:default-system-setting,rule,intent-classifier,gated-setup,upgrade-gemini,provide-context,user-customizable',
  10, 'preference', 1778371200000, 'long', 1.0, 84, 'system.default_system_setting', 'procedural', 1
WHERE NOT EXISTS (
  SELECT 1 FROM memories
  WHERE content LIKE 'DEFAULT SYSTEM SETTING: Intent classifier gated-setup setup%'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed-migration-026', 1778371200000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'DEFAULT SYSTEM SETTING: Intent classifier %'
  AND (
       d.content LIKE 'RAG pipeline:%'
    OR d.content LIKE 'Brain module map:%'
    OR d.content LIKE 'MCP shared data policy:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed-migration-026', 1778371200000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'DEFAULT SYSTEM SETTING: Intent classifier document-learning setup%'
  AND (
       d.content LIKE 'DEFAULT SYSTEM SETTING: Intent classifier teach-ingest setup%'
    OR d.content LIKE 'DEFAULT SYSTEM SETTING: Intent classifier gated-setup setup%'
    OR d.content LIKE 'RULE: Local E2E response latency budget%'
  );