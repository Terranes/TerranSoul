-- Chunk 117: MCP containerization is scoped to CI/research/headless services.

UPDATE memories
SET content = 'Brain modes: (1) Free API — Pollinations/OpenRouter free-tier with no API key needed, (2) Paid API — OpenAI/Anthropic/Groq with user-supplied key, (3) Local Ollama — private, offline-capable, hardware-adaptive model selection. MCP headless mode seeds a local Ollama config when available and does not silently fall back to a free API if Ollama is missing.',
    token_count = 65
WHERE content LIKE 'Brain modes: (1) Free API%Default for MCP headless mode is free API via Pollinations.%';

UPDATE memories
SET content = 'To start the local MCP tray/coding-agent server: run npm run mcp from the repo root. It binds 127.0.0.1:7423 when available, uses mcp-data/ for state, configures Local Ollama when available, leaves setup explicit when no local brain is available, and writes the bearer token to mcp-data/mcp-token.txt plus .vscode/.mcp-token.',
    token_count = 60
WHERE content LIKE 'To start MCP headless server:%auto-configures brain to Pollinations free API if Ollama is unavailable%';

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category)
SELECT
  'LESSON: MCP containerization is for CI, research, and isolated headless services only. Use the explicit npm run mcp:container aliases and the display-free terransoul --mcp-http entry point; the container must set TERRANSOUL_MCP_BIND=0.0.0.0 internally while Compose publishes host loopback only. Keep npm run mcp as the local tray/coding-agent workflow and never make the Tauri desktop app depend on Docker.',
  'lesson,mcp,container,docker,ci,research,headless,desktop-native,bind',
  9,
  'procedure',
  1778544000000,
  'long',
  1.0,
  110,
  'mcp'
WHERE NOT EXISTS (
  SELECT 1 FROM memories
  WHERE content LIKE 'LESSON: MCP containerization is for CI, research, and isolated headless services only.%'
);