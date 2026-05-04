-- TerranSoul MCP Brain Seed Data
-- Applied on first `npm run mcp` when memory.db does not exist yet.
-- Contains architectural knowledge so agents can be productive immediately.
--
-- Schema: see src-tauri/src/memory/schema.rs (version 13)
-- Fields: content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category)
VALUES
-- Architecture overview
('TerranSoul is a Vue 3 + Tauri 2 desktop AI companion app with a Rust backend. It features a 3D VRM anime character, multi-provider LLM chat, persistent memory with semantic-search RAG, voice I/O, CRDT-based device sync, and a gamified skill tree quest system.', 'architecture,overview,terransoul', 5, 'fact', 1746316800000, 'long', 1.0, 50, 'architecture'),

('Tech stack: Shell=Tauri 2.x, Backend=Rust (stable, MSRV 1.80+) with Tokio async, Frontend=Vue 3.5+TypeScript 5.x with Pinia, 3D=Three.js 0.175+ with @pixiv/three-vrm 3.x, Bundler=Vite 6.x, DB=SQLite (default) with StorageBackend trait, LLM=Ollama (local) + OpenAI-compatible APIs + Pollinations (free).', 'architecture,tech-stack,terransoul', 5, 'fact', 1746316800000, 'long', 1.0, 60, 'architecture'),

('Frontend source is in src/ — Vue 3.5 + TypeScript 5.x, Pinia stores. Backend source is in src-tauri/src/ — Rust async (Tokio), 150+ Tauri commands. Tests: npx vitest run, npm run build, npm run lint, cargo test, and cargo clippy --all-targets -- -D warnings.', 'architecture,project-structure,testing', 5, 'fact', 1746316800000, 'long', 1.0, 55, 'architecture'),

-- Brain system
('Brain modes: (1) Free API — Pollinations/OpenRouter free-tier with no API key needed, (2) Paid API — OpenAI/Anthropic/Groq with user-supplied key, (3) Local Ollama — private, offline-capable, hardware-adaptive model selection. Default for MCP headless mode is free API via Pollinations.', 'brain,llm,providers,terransoul', 5, 'fact', 1746316800000, 'long', 1.0, 55, 'brain'),

('RAG pipeline: every chat triggers hybrid 6-signal search, RRF fusion (k=60), optional HyDE, optional cross-encoder rerank, and relevance threshold filtering. Live prompts wrap retrieved records in a [RETRIEVED CONTEXT] pack containing backward-compatible [LONG-TERM MEMORY] snippets plus a contract that the snippets are query results, not the whole database.', 'brain,rag,memory,search,context-pack', 5, 'fact', 1746316800000, 'long', 1.0, 70, 'brain'),

('Vector support: Ollama nomic-embed-text (768-dim) locally, or cloud embedding API (/v1/embeddings) for paid/free modes. HNSW ANN index via usearch crate for O(log n) scaling to 1M+ entries.', 'brain,embeddings,vector,ann', 4, 'fact', 1746316800000, 'long', 1.0, 40, 'brain'),

-- MCP Server
('MCP server exposes brain on three ports: 7421 (release app), 7422 (dev app), 7423 (headless npm run mcp). All wired into .vscode/mcp.json. Bearer-token auth required. Tools: brain_search, brain_get_entry, brain_list_recent, brain_kg_neighbors, brain_summarize, brain_suggest_context, brain_ingest_url, brain_health, code_query, code_context, code_impact, code_rename.', 'mcp,server,tools,setup', 5, 'fact', 1746316800000, 'long', 1.0, 70, 'mcp'),

('MCP shared data policy: mcp-data/shared is committed and reviewable; runtime files such as mcp-token.txt, memory.db, SQLite WAL/SHM files, vector indexes, logs, locks, sessions, and worktrees are ignored. Contributors and self-improve runs may update mcp-data/shared/memory-seed.sql with durable project knowledge.', 'mcp,data,gitignore,shared-seed', 5, 'procedure', 1746316800000, 'long', 1.0, 60, 'mcp'),

('To start MCP headless server: run npm run mcp from the repo root. It binds 127.0.0.1:7423 when available, uses mcp-data/ for state, auto-configures brain to Pollinations free API if Ollama is unavailable, and writes the bearer token to mcp-data/mcp-token.txt plus .vscode/.mcp-token.', 'mcp,setup,quickstart', 5, 'procedure', 1746316800000, 'long', 1.0, 50, 'mcp'),

-- Setup & Development
('CI gate command: npx vitest run && npx vue-tsc --noEmit && cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test. Run after every chunk completion. On Linux, install Tauri WebKit/GTK system libraries before Rust checks.', 'ci,testing,workflow', 5, 'procedure', 1746316800000, 'long', 1.0, 40, 'development'),

('Dev server: npm run dev starts Vite on :1420. Tauri dev: cargo tauri dev. Full build: cargo tauri build. The app window opens a webview to the Vite dev server.', 'development,setup,commands', 4, 'procedure', 1746316800000, 'long', 1.0, 35, 'development'),

('Coding standards: snake_case for Rust, camelCase for TypeScript. Never .unwrap() in library code — use ? + thiserror. Vue components use <script setup lang="ts"> with scoped styles. CSS uses var(--ts-*) design tokens. Tests required for all new functionality.', 'coding-standards,conventions', 5, 'fact', 1746316800000, 'long', 1.0, 45, 'development'),

-- Code Intelligence (Chunks 31.3-31.8)
('Code intelligence pipeline: (1) tree-sitter symbol-table ingest (Rust + TypeScript grammars), (2) cross-file resolution + call graph with confidence scores, (3) label-propagation functional clustering via petgraph, (4) entry-point scoring + BFS process tracing, (5) native MCP tools (code_query, code_context, code_impact, code_rename), (6) editor pre/post-tool-use hooks with auto re-indexing.', 'code-intelligence,symbol-index,mcp', 5, 'fact', 1746316800000, 'long', 1.0, 65, 'code-intelligence'),

('To index a repo for code intelligence: use the code_index_repo Tauri command with the repo path. Then code_resolve_edges for cross-file resolution, then code_compute_processes for clustering. Results are runtime state and should stay ignored under mcp-data/.', 'code-intelligence,indexing,setup', 4, 'procedure', 1746316800000, 'long', 1.0, 45, 'code-intelligence'),

-- Self-Improve System
('Self-improve system: TerranSoul can improve its own codebase via temporary git worktrees. Flow: detect target repo → create worktree → make changes → run CI gate → create PR. Controlled by coding workflow config. Uses coding LLM configured separately from chat LLM.', 'self-improve,coding,workflow', 4, 'fact', 1746316800000, 'long', 1.0, 50, 'self-improve'),

('Self-improve with MCP mode: agents should start npm run mcp, call brain_health/brain_search/brain_suggest_context for project context, and update mcp-data/shared when a durable repo convention or architecture fact should help future sessions. Runtime self-improve artifacts stay ignored.', 'self-improve,mcp,workflow,shared-seed', 5, 'procedure', 1746316800000, 'long', 1.0, 60, 'self-improve'),

-- Memory System
('Memory store uses SQLite with FTS5 for keyword search and HNSW (usearch) for vector ANN. Memories have: content, tags, importance (1-5), memory_type (fact/preference/episode/procedure), tier (short/long/archival), decay_score, category, optional embedding. Knowledge graph via memory_edges table (typed, directional).', 'memory,schema,storage', 5, 'fact', 1746316800000, 'long', 1.0, 55, 'memory'),

-- Recommended First Steps for New Contributors
('Recommended first steps after cloning: (1) npm ci, (2) npm run mcp to start the brain server, (3) Read rules/milestones.md for current work queue, (4) Read rules/completion-log.md for recent history, (5) Run the CI gate to verify everything builds. The MCP server pre-loads shared TerranSoul knowledge from mcp-data/shared/.', 'onboarding,setup,quickstart', 5, 'procedure', 1746316800000, 'long', 1.0, 60, 'onboarding'),

('Key directories: src/ (Vue frontend), src-tauri/src/ (Rust backend), rules/ (project rules + milestones), docs/ (design docs), scripts/ (dev utilities), public/ (static assets — models, animations, audio), mcp-data/shared/ (committed seed knowledge for MCP brain), mcp-data/ runtime files (ignored).', 'project-structure,directories', 4, 'fact', 1746316800000, 'long', 1.0, 50, 'architecture');
