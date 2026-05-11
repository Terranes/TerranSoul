-- 025_local_e2e_latency_budget.sql
-- Persist the local E2E latency rule for existing MCP databases.

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind)
SELECT
  'RULE: Local E2E response latency budget (2026-05-10): Playwright tests outside GitHub Actions must fail any assistant/LLM response latency above 2 seconds with an investigation-focused failure message. Keep diagnostic wait timeouts long enough to collect evidence, but do not resolve latency regressions by increasing Playwright timeouts or relaxing assertions; investigate model warmup, VRAM contention, RAG retrieval, embedding backfill, provider selection, streaming first chunk, and UI state propagation.',
  'rule,e2e,latency,playwright,local-only,ollama,rag,streaming,timeout-discipline',
  10, 'procedure', 1778371200000, 'long', 1.0, 72, 'testing', 'procedural'
WHERE NOT EXISTS (
  SELECT 1
  FROM memories
  WHERE content LIKE 'RULE: Local E2E response latency budget (2026-05-10):%'
);
