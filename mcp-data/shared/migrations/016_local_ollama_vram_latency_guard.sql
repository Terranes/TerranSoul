-- Migration 016: Record LocalOllama real-app latency guard for existing MCP databases
-- Applied: 2026-05-09

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind)
VALUES (
  'LESSON: LocalOllama VRAM eviction by background workers (2026-05-12): In LocalOllama mode the embedding worker (10s tick) and any helper that calls /api/embed (HyDE, late-chunking, batch ingest) loads nomic-embed-text into VRAM, which evicts the chat model (e.g. gemma4:e4b) and adds a 10-20s reload cost on the very next chat reply. The first attempted fix still let the real desktop app take ~10s because AppState.last_chat_at_ms started at 0 and spawn_embedding_queue_worker ran before spawn_local_ollama_warmup; tokio interval workers tick immediately, so the embedding backfill could seize Ollama at app startup before the first user chat. The durable fix is layered: (1) AppState gains last_chat_at_ms: AtomicU64 initialized to now for production AppState and set by run_chat_stream/process_message on every user turn; (2) startup calls spawn_local_ollama_warmup before spawn_embedding_queue_worker; (3) the embedding worker skips ticks while last chat/startup quiet-window was within 5 minutes when provider_category == "ollama"; (4) every Ollama embed body sets keep_alive: 0 so the embed model unloads immediately after each batch; (5) every /api/chat caller sets keep_alive: "30m"; (6) stream_ollama sends think:false so Gemma/Qwen thinking models do not spend seconds in a silent reasoning phase before visible content; (7) LocalOllama non-streaming fallback uses keyword-only memories instead of LLM semantic_search_entries before answering. Verified with Playwright Real-E2E hi-latency including a real Rust run_chat_stream probe: first real backend llm-chunk 537ms with gemma4:e4b; direct Hi chat 595ms; chat-only 681ms; all under 1s first-token target.',
  'lesson,perf,vram,ollama,local-llm,embedding-queue,keep-alive,warmup,streaming,real-app-latency',
  10,
  'procedure',
  1778544000000,
  'long',
  1.0,
  150,
  'brain',
  'procedural'
);