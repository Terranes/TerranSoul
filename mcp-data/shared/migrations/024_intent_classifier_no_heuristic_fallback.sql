-- 024_intent_classifier_no_heuristic_fallback.sql
-- Keep runtime MCP databases aligned with the intent-routing principle:
-- docs/setup intent decisions must come from classifier + RAG context,
-- never string heuristics on malformed/unknown classifier output.

UPDATE memories
SET
  content = 'RULE: Intent classifier document-learning setup (2026-05-10): When a user says "Learn from my documents" or asks to learn/study from their own documents, files, notes, PDFs, or sources, classify the turn as learn_with_docs. If they do not specify a topic or setup mode, use topic "the material in your documents" and assume the recommended setup flow. The recommended flow is the Scholar''s Quest prerequisite chain: free-brain (Awaken the Mind) -> memory (Long-Term Memory) -> rag-knowledge (Sage''s Library, 6-signal hybrid RAG) -> scholar-quest (document ingestion/attachment). If classifier JSON is malformed or unknown, return Unknown and continue normal chat/install flows; never force docs/setup routing via regex, contains, includes, or keyword arrays. This is the behavior described by tutorials/brain-rag-setup-tutorial.md.',
  tags = 'rule,intent-classifier,learn-with-docs,documents,rag,quest,scholar-quest,recommended-setup,tutorial,no-heuristic-fallback',
  token_count = 133,
  cognitive_kind = 'procedural'
WHERE content LIKE 'RULE: Intent classifier document-learning setup (2026-05-10):%';
