-- Migration 015: Record LocalLLM fast chat path for existing MCP databases
-- Applied: 2026-05-09

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind)
VALUES (
  'LESSON: LocalLLM fast chat path (2026-05-09): short content-light turns such as "Hi", "Hello", "OK", or "who are you" must not call the intent classifier, embedding model, or hybrid RAG retrieval. On consumer GPUs, gemma4:e4b can fill VRAM and loading nomic-embed-text evicts the chat model, causing 5-15s model swaps. The durable fix is pure-code fast-path guards in src/stores/conversation.ts, src-tauri/src/brain/intent_classifier.rs, and src-tauri/src/commands/streaming.rs; contentful questions still use classifier + full RAG.',
  'lesson,perf,rag,streaming,vram,ollama,local-llm,fast-path',
  10,
  'procedure',
  1778284800000,
  'long',
  1.0,
  115,
  'brain',
  'procedural'
);
