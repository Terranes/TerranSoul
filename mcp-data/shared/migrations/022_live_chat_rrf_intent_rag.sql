-- Refresh durable RAG knowledge for the live daily-chatbox retrieval path.
-- Created: 2026-05-10

UPDATE memories
SET
  content = 'RAG pipeline: contentful live chat uses fast-path skip for short/empty turns, thresholded hybrid 6-signal eligibility, then RRF + query-intent ordering for top-5 prompt injection. Free/paid modes can include query embeddings; Local Ollama hot stream stays keyword/freshness-only when embedding would swap models. HyDE, matryoshka, and LLM-as-judge rerank are available through MemoryView/MCP/non-streaming helper surfaces rather than automatic streamed chat by default. Live prompts wrap retrieved records in a [RETRIEVED CONTEXT] pack containing backward-compatible [LONG-TERM MEMORY] snippets plus a contract that the snippets are query results, not the whole database.',
  token_count = 110,
  cognitive_kind = 'semantic'
WHERE content LIKE 'RAG pipeline:%'
  AND tags = 'brain,rag,memory,search,context-pack';

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind)
SELECT
  'RAG pipeline: contentful live chat uses fast-path skip for short/empty turns, thresholded hybrid 6-signal eligibility, then RRF + query-intent ordering for top-5 prompt injection. Free/paid modes can include query embeddings; Local Ollama hot stream stays keyword/freshness-only when embedding would swap models. HyDE, matryoshka, and LLM-as-judge rerank are available through MemoryView/MCP/non-streaming helper surfaces rather than automatic streamed chat by default. Live prompts wrap retrieved records in a [RETRIEVED CONTEXT] pack containing backward-compatible [LONG-TERM MEMORY] snippets plus a contract that the snippets are query results, not the whole database.',
  'brain,rag,memory,search,context-pack',
  5, 'fact', 1778371200000, 'long', 1.0, 110, 'brain', 'semantic'
WHERE NOT EXISTS (
  SELECT 1
  FROM memories
  WHERE content LIKE 'RAG pipeline: contentful live chat uses fast-path skip%'
);
