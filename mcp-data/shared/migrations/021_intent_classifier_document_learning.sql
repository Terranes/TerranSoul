-- Add durable intent-classifier knowledge for document-learning setup.
-- Created: 2026-05-10

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind)
VALUES (
  'RULE: Intent classifier document-learning setup (2026-05-10): When a user says "Learn from my documents" or asks to learn/study from their own documents, files, notes, PDFs, or sources, classify the turn as learn_with_docs. If they do not specify a topic or setup mode, use topic "the material in your documents" and assume the recommended setup flow. The recommended flow is the Scholar''s Quest prerequisite chain: free-brain (Awaken the Mind) -> memory (Long-Term Memory) -> rag-knowledge (Sage''s Library, 6-signal hybrid RAG) -> scholar-quest (document ingestion/attachment). This is the behavior described by tutorials/brain-rag-setup-tutorial.md.',
  'rule,intent-classifier,learn-with-docs,documents,rag,quest,scholar-quest,recommended-setup,tutorial',
  10, 'procedure', 1778371200000, 'long', 1.0, 105, 'brain', 'procedural'
);