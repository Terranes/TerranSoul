-- TerranSoul MCP Brain Seed Data
-- Applied on first `npm run mcp` when memory.db does not exist yet.
-- Contains architectural knowledge so agents can be productive immediately.
--
-- Schema: see src-tauri/src/memory/schema.rs (version 19)
-- Fields: content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category, cognitive_kind

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category)
VALUES
-- Architecture overview
('TerranSoul is a Vue 3 + Tauri 2 desktop AI companion app with a Rust backend. It features a 3D VRM anime character, multi-provider LLM chat, persistent memory with semantic-search RAG, voice I/O, CRDT-based device sync, and a gamified skill tree quest system.', 'architecture,overview,terransoul', 5, 'fact', 1746316800000, 'long', 1.0, 50, 'architecture'),

('Tech stack: Shell=Tauri 2.x, Backend=Rust (stable, MSRV 1.80+) with Tokio async, Frontend=Vue 3.5+TypeScript 5.x with Pinia, 3D=Three.js 0.175+ with @pixiv/three-vrm 3.x, Bundler=Vite 6.x, DB=SQLite (default) with StorageBackend trait, LLM=Ollama (local) + OpenAI-compatible APIs + Pollinations (free).', 'architecture,tech-stack,terransoul', 5, 'fact', 1746316800000, 'long', 1.0, 60, 'architecture'),

('Frontend source is in src/ — Vue 3.5 + TypeScript 5.x, Pinia stores. Backend source is in src-tauri/src/ — Rust async (Tokio), 150+ Tauri commands. Tests: npx vitest run, npm run build, npm run lint, cargo test, and cargo clippy --all-targets -- -D warnings.', 'architecture,project-structure,testing', 5, 'fact', 1746316800000, 'long', 1.0, 55, 'architecture'),

-- Brain system
('Brain modes: (1) Free API — Pollinations/OpenRouter free-tier with no API key needed, (2) Paid API — OpenAI/Anthropic/Groq with user-supplied key, (3) Local Ollama — private, offline-capable, hardware-adaptive model selection. Default for MCP headless mode is free API via Pollinations.', 'brain,llm,providers,terransoul', 5, 'fact', 1746316800000, 'long', 1.0, 55, 'brain'),

('RAG pipeline: every chat triggers hybrid 6-signal search, RRF fusion (k=60), optional HyDE, default-on cross-encoder rerank for RRF/HyDE when a local brain is available, and relevance threshold filtering. Rerank scores are normalised from 0-10 and pruned below the default 0.55 threshold before prompt injection. Live prompts wrap retrieved records in a [RETRIEVED CONTEXT] pack containing backward-compatible [LONG-TERM MEMORY] snippets plus a contract that the snippets are query results, not the whole database.', 'brain,rag,memory,search,context-pack', 5, 'fact', 1746316800000, 'long', 1.0, 95, 'brain'),

('Vector support: Ollama nomic-embed-text (768-dim) locally, or cloud embedding API (/v1/embeddings) for paid/free modes. Default builds use a pure-Rust linear cosine AnnIndex for loader-stable CI/headless MCP; the native-ann feature enables persisted usearch HNSW vectors.usearch for large local stores.', 'brain,embeddings,vector,ann,native-ann', 4, 'fact', 1746316800000, 'long', 1.0, 55, 'brain'),

-- MCP Server
('MCP server exposes brain on three ports: 7421 (release app), 7422 (dev app), 7423 (headless npm run mcp). All wired into .vscode/mcp.json. Bearer-token auth required. Tools: brain_search, brain_get_entry, brain_list_recent, brain_kg_neighbors, query-backed brain_summarize, brain_suggest_context, brain_ingest_url, brain_health, brain_failover_status, code_query, code_context, code_impact, code_rename.', 'mcp,server,tools,setup', 5, 'fact', 1746316800000, 'long', 1.0, 80, 'mcp'),

('MCP shared data policy: mcp-data/shared is committed and reviewable; runtime files such as mcp-token.txt, memory.db, SQLite WAL/SHM files, vector indexes, logs, locks, sessions, and worktrees are ignored. Contributors and self-improve runs may update mcp-data/shared/memory-seed.sql with durable project knowledge.', 'mcp,data,gitignore,shared-seed', 5, 'procedure', 1746316800000, 'long', 1.0, 60, 'mcp'),

('To start MCP headless server: run npm run mcp from the repo root. It binds 127.0.0.1:7423 when available, uses mcp-data/ for state, auto-configures brain to Pollinations free API if Ollama is unavailable, and writes the bearer token to mcp-data/mcp-token.txt plus .vscode/.mcp-token.', 'mcp,setup,quickstart', 5, 'procedure', 1746316800000, 'long', 1.0, 50, 'mcp'),

-- Setup & Development
('CI gate command: npx vitest run && npx vue-tsc --noEmit && cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test. Run after every chunk completion. On Linux, install Tauri WebKit/GTK system libraries before Rust checks.', 'ci,testing,workflow', 5, 'procedure', 1746316800000, 'long', 1.0, 40, 'development'),

('MCP/app dependency bootstrap rule: if npm run mcp, npm run dev, cargo tauri dev, or validation fails because pkg-config cannot find Linux system libraries such as glib-2.0 or gio-2.0, install the missing Tauri/MCP packages with the platform package manager and retry before declaring the task blocked. Ubuntu minimum set: libglib2.0-dev, libgtk-3-dev, libwebkit2gtk-4.1-dev, libappindicator3-dev, librsvg2-dev, patchelf, libsoup-3.0-dev, libjavascriptcoregtk-4.1-dev, pkg-config.', 'mcp,setup,dependencies,tauri,linux,agent-rule', 5, 'procedure', 1746487655000, 'long', 1.0, 95, 'development'),

('Dev server: npm run dev starts Vite on :1420. Tauri dev: cargo tauri dev. Full build: cargo tauri build. The app window opens a webview to the Vite dev server.', 'development,setup,commands', 4, 'procedure', 1746316800000, 'long', 1.0, 35, 'development'),

('Coding standards: snake_case for Rust, camelCase for TypeScript. Never .unwrap() in library code — use ? + thiserror. Vue components use <script setup lang="ts"> with scoped styles. CSS uses var(--ts-*) design tokens. Tests required for all new functionality.', 'coding-standards,conventions', 5, 'fact', 1746316800000, 'long', 1.0, 45, 'development'),

-- Code Intelligence (Chunks 31.3-31.8)
('Code intelligence pipeline: (1) tree-sitter symbol-table ingest (Rust + TypeScript always on; Python/Go/Java/C/C++ behind parser-* features), (2) content-hash incremental re-indexing via code_file_hashes, (3) cross-file resolution + call graph with confidence scores, (4) label-propagation functional clustering via petgraph, (5) entry-point scoring + BFS process tracing, (6) native MCP tools (code_query, code_context, code_impact, code_rename), (7) editor pre/post-tool-use hooks with auto re-indexing.', 'code-intelligence,symbol-index,mcp', 5, 'fact', 1746316800000, 'long', 1.0, 85, 'code-intelligence'),

('To index a repo for code intelligence: use the code_index_repo Tauri command with the repo path. Then code_resolve_edges for cross-file resolution, then code_compute_processes for clustering. Results are runtime state and should stay ignored under mcp-data/.', 'code-intelligence,indexing,setup', 4, 'procedure', 1746316800000, 'long', 1.0, 45, 'code-intelligence'),

-- Self-Improve System
('Self-improve system: TerranSoul can improve its own codebase via temporary git worktrees. Flow: detect target repo → create worktree → make changes → run CI gate → create PR. Controlled by coding workflow config. Uses coding LLM configured separately from chat LLM.', 'self-improve,coding,workflow', 4, 'fact', 1746316800000, 'long', 1.0, 50, 'self-improve'),

('Self-improve with MCP mode: agents should start npm run mcp, call brain_health/brain_search/brain_suggest_context for project context, and update mcp-data/shared when a durable repo convention or architecture fact should help future sessions. Runtime self-improve artifacts stay ignored.', 'self-improve,mcp,workflow,shared-seed', 5, 'procedure', 1746316800000, 'long', 1.0, 60, 'self-improve'),

('DEEPWIKI REVERSE-ENGINEERING RULE: when studying any GitHub project, first check https://deepwiki.org/<owner>/<repo> when reachable, then cross-check upstream README/docs/code/license. If DeepWiki is unavailable, record the blocker and proceed with direct upstream research. Credit any learned source in CREDITS.md.', 'rules,deepwiki,reverse-engineering,credits,non-negotiable', 10, 'procedure', 1746416915000, 'long', 1.0, 75, 'self-improve'),

('MCP SELF-LEARNING RULE: durable rules, conventions, architecture facts, and reverse-engineering lessons learned in an agent session must be synced into mcp-data/shared/memory-seed.sql in the same PR. The Obsidian vault at mcp-data/wiki/ is auto-generated by the maintenance scheduler and must not be edited by hand. Runtime memory.db is not the durable source of truth.', 'rules,mcp,self-improve,self-learning,shared-data,non-negotiable', 10, 'procedure', 1746416915000, 'long', 1.0, 80, 'self-improve'),

-- Memory System
('Memory store uses SQLite with FTS5 for keyword search and a vector nearest-neighbor adapter. Default builds use pure-Rust linear cosine scan; native-ann enables usearch HNSW. Memories have: content, tags, importance (1-5), memory_type (fact/preference/episode/procedure), tier, decay_score, category, optional embedding. Knowledge graph via memory_edges table.', 'memory,schema,storage,ann,native-ann', 5, 'fact', 1746316800000, 'long', 1.0, 65, 'memory'),

-- Recommended First Steps for New Contributors
('Recommended first steps after cloning: (1) npm ci, (2) npm run mcp to start the brain server, (3) Read rules/milestones.md for current work queue, (4) Read rules/completion-log.md for recent history, (5) Run the CI gate to verify everything builds. The MCP server pre-loads shared TerranSoul knowledge from mcp-data/shared/.', 'onboarding,setup,quickstart', 5, 'procedure', 1746316800000, 'long', 1.0, 60, 'onboarding'),

('Key directories: src/ (Vue frontend), src-tauri/src/ (Rust backend), rules/ (project rules + milestones), docs/ (design docs), scripts/ (dev utilities), public/ (static assets — models, animations, audio), mcp-data/shared/ (committed seed knowledge for MCP brain), mcp-data/ runtime files (ignored).', 'project-structure,directories', 4, 'fact', 1746316800000, 'long', 1.0, 50, 'architecture'),

-- ====================================================================
-- Pointers to the rest of the shared dataset (read these for full detail)
-- ====================================================================
('MCP shared seed dataset lives in mcp-data/shared/memory-seed.sql (the single SQL source of truth). The Obsidian vault at mcp-data/wiki/ is auto-generated by the maintenance scheduler as a human-readable projection. Read the seed SQL or query the brain for project navigation before scanning code.', 'project-index,navigation,onboarding,shared-doc', 5, 'procedure', 1746316800000, 'long', 1.0, 60, 'onboarding'),

('Durable gotchas, decisions, and lessons learned from past agent sessions are stored as LESSON: rows in mcp-data/shared/memory-seed.sql and retrievable via brain_search. The Obsidian vault at mcp-data/wiki/ provides a browsable view. Append new LESSON: rows to memory-seed.sql when a non-obvious trade-off or architectural decision is worth keeping.', 'lessons-learned,gotchas,self-improve,shared-doc', 5, 'procedure', 1746316800000, 'long', 1.0, 60, 'self-improve'),

-- ====================================================================
-- Brain submodule map (src-tauri/src/brain/)
-- ====================================================================
('Brain module map: brain_config.rs (persisted provider/model/keys), brain_store.rs (state container), cloud_embeddings.rs (paid/free embed_for_mode), context_budget.rs (token budgeting), doc_catalogue.rs (brain-aware doc catalogue), docker_ollama.rs (auto-setup Ollama via Docker), free_api.rs (Pollinations/OpenRouter free tier), intent_classifier.rs, lm_studio.rs, maintenance_runtime.rs + maintenance_scheduler.rs (decay/GC/summarization), mcp_auto_config.rs (headless brain auto-config), model_recommender.rs (RAM-based catalogue), ollama_agent.rs (embed_text + hyde_complete + rerank_score), ollama_lifecycle.rs, openai_client.rs, provider_rotator.rs, ram_budget.rs, segmenter.rs, selection.rs, system_info.rs.', 'brain,module-map,architecture', 5, 'fact', 1746316800000, 'long', 1.0, 120, 'brain'),

-- ====================================================================
-- Memory submodule map (src-tauri/src/memory/)
-- ====================================================================
('Memory module map: schema.rs (canonical V15 SQLite schema), store.rs (default SQLite memory store with hybrid_search + hybrid_search_rrf + ANN bridge), ann_index.rs (HNSW via usearch), eviction.rs (capacity pruning with protected/high-importance preservation), backend.rs (StorageBackend trait/factory), cassandra.rs / mssql.rs / postgres.rs (optional backends), chunking.rs + late_chunking.rs (semantic chunking), code_rag.rs, cognitive_kind.rs, conflicts.rs + edge_conflict_scan.rs (LLM contradiction resolution), consolidation.rs, context_pack.rs ([RETRIEVED CONTEXT] assembly), contextualize.rs (Anthropic Contextual Retrieval), crag.rs, crdt_sync.rs, edges.rs (typed/directional KG edges), fusion.rs (RRF k=60), gitnexus_mirror.rs, graph_rag.rs, hyde.rs (HyDE), matryoshka.rs, obsidian_export.rs + obsidian_sync.rs, query_intent.rs, reflection.rs (/reflect session reflection + derived_from source-turn provenance), replay.rs, reranker.rs (LLM-as-judge), tag_vocabulary.rs, temporal.rs, versioning.rs.', 'memory,module-map,architecture', 5, 'fact', 1746316800000, 'long', 1.0, 175, 'memory'),

('Hybrid 6-signal search weights live in src-tauri/src/memory/store.rs: vector(40%) + keyword(20%) + recency(15%) + importance(10%) + decay(10%) + tier(5%). RRF fusion uses k=60. HyDE and cross-encoder rerank are optional per-query; default for RRF/HyDE MCP search is rerank on with rerank_threshold 0.55.', 'memory,search,signals,rag,rerank', 5, 'fact', 1746316800000, 'long', 1.0, 70, 'memory'),

('SQLite schema is at version 15 (CANONICAL_SCHEMA_VERSION in src-tauri/src/memory/schema.rs). memories columns include content, tags, importance, memory_type, created_at, last_accessed, access_count, embedding, source_url, source_hash, expires_at, tier, decay_score, session_id, parent_id, token_count, valid_to, obsidian_path, last_exported, category, cognitive_kind, updated_at, origin_device, and protected. Edges in memory_edges (typed, directional). Versions in memory_versions. FTS5 virtual table for keyword search. pending_embeddings(memory_id PK, attempts, last_error, next_retry_at) backs the self-healing embedding retry queue.', 'memory,schema,sqlite', 5, 'fact', 1746316800000, 'long', 1.0, 110, 'memory'),

('Self-healing embedding retry queue (Chunk 38.2): src-tauri/src/memory/embedding_queue.rs spawns a background worker that backfills NULL-embedding memories on boot and drains pending_embeddings every 10s in batches of 32 via embed_batch_for_mode. Per-row exponential backoff (10s, 20s, 40s, ... cap 1h) on failure. Tauri command embedding_queue_status returns pending/failing/next_retry_at. BrainView.vue polls every 5s. rag_quality_pct self-heals to ~99% as long as the embedding provider is reachable.', 'memory,embedding-queue,self-heal,rag-quality', 5, 'fact', 1746316800000, 'long', 1.0, 110, 'memory'),

('Batched embedding pipeline (Chunk 38.1): OllamaAgent::embed_text_batch and brain::embed_batch_for_mode POST array input to /api/embed (or cloud /v1/embeddings) for ~10x faster ingest. AppStateInner.ingest_semaphore (Semaphore::new(4)) caps concurrent ingest tasks. commands/ingest.rs replaced its per-chunk for-loop with a single batch call and enqueues failed embeds into pending_embeddings (Chunk 38.2 picks them up).', 'brain,embedding,batch,ingest,perf', 5, 'fact', 1746316800000, 'long', 1.0, 100, 'brain'),

-- ====================================================================
-- ai_integrations submodule map
-- ====================================================================
('ai_integrations exposes the brain to external AI assistants. gateway.rs defines the BrainGateway trait + AppStateGateway adapter (8 ops: search, get, list_recent, kg_neighbors, summarize, suggest_context, ingest_url, health). mcp/ holds the MCP HTTP server (bearer-token auth, tools/prompts/resources). grpc/ holds the brain.v1 transport for desktop-mobile bridge.', 'ai-integrations,mcp,grpc,gateway', 5, 'fact', 1746316800000, 'long', 1.0, 80, 'ai-integrations'),

-- ====================================================================
-- Persona / motion / charisma
-- ====================================================================
('Persona module map: pack.rs (persona pack import/export schema), extract.rs + drift.rs (trait extraction + drift detection), charisma.rs (charisma teaching system), motion_clip.rs (motion clip parser/validator), motion_tokens.rs (MotionGPT motion token codec), motion_reconstruction.rs (MoMask-style full-body reconstruction), motion_smooth.rs + motion_feedback.rs (offline polish + self-improve loop), pose_frame.rs (LLM-as-Animator pose-frame parser), prosody.rs, retarget.rs.', 'persona,motion,module-map', 5, 'fact', 1746316800000, 'long', 1.0, 90, 'persona'),

('Pose pipeline: <pose> tag in StreamTagParser emits llm-pose event consumed by frontend PoseAnimator (Chunks 14.16b1/b2/b3). Emotion-reactive procedural pose bias (14.16d). generate_motion_from_text Tauri command + Persona-panel UI (14.16c2/c3). ARKit blendshape passthrough is the canonical face rig (Chunk 27.3).', 'persona,pose,animation,arkit', 4, 'fact', 1746316800000, 'long', 1.0, 65, 'persona'),

-- ====================================================================
-- Voice
-- ====================================================================
('Voice module map: config_store.rs (persisted voice config), stub_asr.rs / stub_diarization.rs / stub_tts.rs (default offline stubs), whisper_api.rs (Whisper-compatible ASR endpoint). Defaults: Web Speech TTS, stub ASR/diarization. VoiceConfig serde-stable across hotword field rollouts.', 'voice,module-map,asr,tts', 4, 'fact', 1746316800000, 'long', 1.0, 60, 'voice'),

-- ====================================================================
-- Self-improve / coding workflow
-- ====================================================================
('coding/ module map: engine.rs + workflow.rs + autostart.rs + client.rs (self-improve engine), apply_file.rs + git_ops.rs + worktree.rs (patch application + temporary git worktrees), context_budget.rs + context_engineering.rs + prompting.rs (prompt assembly), conversation_learning.rs + session_chat.rs + task_queue.rs (session learning), cost.rs + metrics.rs, dag_runner.rs + multi_agent.rs + resolver.rs + reviewer.rs + processes.rs (DAG orchestration), github.rs (GitHub PR flow), handoff.rs + handoff_store.rs + milestones.rs + promotion_plan.rs (handoff + promotion), repo.rs + rename.rs + symbol_index.rs + wiki.rs + test_runner.rs.', 'self-improve,coding,module-map', 5, 'fact', 1746316800000, 'long', 1.0, 130, 'self-improve'),

('Self-improve flow: detect target repo -> create temporary git worktree (chunk 28.13) -> path-scoped workflow context loading (28.14) -> coding intent router (28.2) -> multi-agent DAG runner (28.3 + 28.12) -> apply/review/test execution gate (28.11) -> GitHub PR flow with OAuth device authorization (28.5). Session transcripts auto-append to mcp-data via Chunk 30.6. Isolated patch auto-merge added in 32.4. Chunk completion + retry in 32.3. Chunk 34.1 adds a persisted backend workboard sourced from milestones, completion-log, and run metrics.', 'self-improve,workflow,history,workboard', 5, 'fact', 1746316800000, 'long', 1.0, 105, 'self-improve'),

-- ====================================================================
-- Identity, link, sync, network, messaging
-- ====================================================================
('Device identity uses Ed25519. Files: identity/device.rs (DeviceIdentity), key_store.rs, qr.rs (pairing QR), trusted_devices.rs (registry). LAN gRPC enforces mTLS to paired devices (Chunks 24.2b/24.3). Phone-control RPC surface (24.4). gRPC-Web client + transport adapter for browser (24.8).', 'identity,sync,grpc,mtls', 4, 'fact', 1746316800000, 'long', 1.0, 70, 'sync'),

('Sync primitives in src-tauri/src/sync/: lww_register.rs, or_set.rs, append_log.rs. Soul Link wire protocol (Chunks 17.5a + 17.5b). link/ module: manager.rs, quic.rs, ws.rs (QUIC + WebSocket transports), handlers.rs.', 'sync,link,crdt,transport', 4, 'fact', 1746316800000, 'long', 1.0, 55, 'sync'),

-- ====================================================================
-- Plugins, sandbox, agents, orchestrator, workflows, tasks
-- ====================================================================
('Plugins run in a WASM sandbox: src-tauri/src/sandbox/wasm_runner.rs + capability.rs (capability gating) + host_api.rs. Plugin host: plugins/host.rs + manifest.rs. Capability grants prompted via composables/usePluginCapabilityGrants. Plugin command dispatch in commands/plugins.rs (Chunk 22.7).', 'plugins,sandbox,wasm,capabilities', 4, 'fact', 1746316800000, 'long', 1.0, 65, 'plugins'),

('Orchestrator submodules: agent_orchestrator.rs (agent routing with capability gates), agentic_rag.rs, coding_router.rs, self_rag.rs (Self-RAG orchestrator loop, Chunk 16.4b). Workflows engine: workflows/engine.rs + resilience.rs (retry, circuit breaker, watchdog). Tasks: tasks/manager.rs (long-running task tracking).', 'orchestrator,workflows,tasks', 4, 'fact', 1746316800000, 'long', 1.0, 60, 'orchestrator'),

-- ====================================================================
-- Frontend Pinia stores (high-level inventory)
-- ====================================================================
('Frontend Pinia stores in src/stores/: brain (active provider config), conversation (local chat history + streaming), memory (memory CRUD + search), persona (traits + learned expressions), skill-tree (~1500 lines, gamified quest system with auto-detection), voice (TTS/ASR), settings, character (active VRM + emotion), audio, background, charisma, ai-decision-policy, ai-integrations, agent-roster, browser-lan, chat-store-router, coding-workflow, identity, link, mcp-activity (live MCP tool-use UI badge), messaging, mobile-notifications, mobile-pairing, package, plugins, provider-health, remote-conversation, routing, sandbox, self-improve, streaming, sync, tasks, teachable-capabilities, window (Window vs Pet mode), workflow-plans.', 'frontend,stores,pinia,inventory', 4, 'fact', 1746316800000, 'long', 1.0, 130, 'frontend'),

('Frontend composables in src/composables/: useAsrManager + useTtsPlayback + useDiarization + useHotwords + useLipSyncBridge (voice pipeline), useChatExpansion + useChatExport (chat UI), useCameraCapture + usePresenceDetector (vision), useTheme + useActivePluginTheme, useTranslation (worldwide translator), usePluginCapabilityGrants + usePluginSlashDispatch (plugin UX), useVrmThumbnail (offscreen VRM rendering), useModelCameraStore (per-model framing), useBgmPlayer, useKeyboardDetector.', 'frontend,composables,inventory', 4, 'fact', 1746316800000, 'long', 1.0, 90, 'frontend'),

-- ====================================================================
-- Tauri command surface inventory
-- ====================================================================
('commands/ files (~150 Tauri commands): agent.rs + agents_roster.rs (agents), auto_setup.rs (first-launch), brain.rs + chat.rs + streaming.rs (chat pipeline), character.rs + emotion.rs + vision.rs, charisma.rs + persona.rs, coding.rs + coding_sessions.rs (self-improve), consolidation.rs + crag.rs + gitnexus.rs + ingest.rs + memory.rs (memory ops), docker.rs + lan.rs + link.rs + routing.rs (network), github_auth.rs (GitHub OAuth device flow), grpc.rs + mcp.rs (transport lifecycle), identity.rs + messaging.rs, package.rs + plugins.rs + registry.rs + sandbox.rs (plugins), quest.rs (skill tree), settings.rs + window.rs + workflow_plans.rs, teachable_capabilities.rs, translation.rs, user_models.rs, voice.rs, vscode.rs, ipc_contract_tests.rs.', 'commands,inventory,architecture', 4, 'fact', 1746316800000, 'long', 1.0, 130, 'commands'),

-- ====================================================================
-- Design docs (one-line summaries)
-- ====================================================================
('Design docs (docs/) and tutorials (tutorials/): docs/ contains AI-coding-integrations.md, brain-advanced-design.md, coding-workflow-design.md, DESIGN.md, gitnexus-capability-matrix.md, hive-protocol.md, licensing-audit.md, llm-animation-research.md, momask-full-body-retarget-research.md, motion-model-inference-evaluation.md, neural-audio-to-face-evaluation.md, offline-motion-polish-research.md, persona-design.md, persona-pack-schema.md, plugin-development.md, teachable-capabilities.md. tutorials/ has 18 files: quick-start, brain-rag-setup, brain-rag-local-lm, voice-setup, skill-tree-quests, advanced-memory-rag, knowledge-wiki, folder-to-knowledge-graph, teaching-animations-expressions-persona, charisma-teaching, device-sync-hive, lan-mcp-sharing, mcp-coding-agents, multi-agent-workflows, packages-plugins, browser-mobile, self-improve-to-pr, openclaw-plugin (all *-tutorial.md).', 'docs,tutorials,inventory,design', 4, 'fact', 1746316800000, 'long', 1.0, 110, 'docs'),

-- ====================================================================
-- Rules (one-line summaries)
-- ====================================================================
('Rules files (rules/): agent-mcp-bootstrap.md (how agents connect to npm run mcp), architecture-rules.md (incl. brain doc-sync rule), backlog.md, coding-standards.md (incl. Multi-Agent Instruction Sync, CREDITS rule), coding-workflow-reliability.md, completion-log.md (permanent done-chunk record, 10k-line cap then archived), llm-decision-rules.md, local-first-brain.md, milestones.md (active queue: only not-started/in-progress), prompting-rules.md (incl. enforcement rules), quality-pillars.md, reality-filter.md (no pretend code), research-reverse-engineering.md, tutorial-template.md (required structure for tutorials/ Markdown files: H1, intro block-quote, requirements, numbered steps quoting exact UI/code paths, worked example, troubleshooting, where-to-next), ui-ux-standards.md.', 'rules,inventory,governance', 5, 'fact', 1746316800000, 'long', 1.0, 110, 'rules'),

-- ====================================================================
-- Lessons learned (durable; never re-solve)
-- ====================================================================
('LESSON: Tauri requires Linux WebKit/GTK system deps before any cargo command on Linux: libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev pkg-config libglib2.0-dev libssl-dev. The Copilot cloud agent installs these in .github/workflows/copilot-setup-steps.yml.', 'lesson,build,linux,tauri', 5, 'procedure', 1746316800000, 'long', 1.0, 50, 'lessons'),

('LESSON: First `npm run mcp` build is slow (full Rust crate, ~3-5 min). Subsequent runs are warm via src-tauri/target. Always wait on GET /health via scripts/wait-for-service.mjs before issuing tool calls. In sandboxed environments, use setsid to detach so npm run mcp survives short tool calls.', 'lesson,mcp,build,workflow', 5, 'procedure', 1746316800000, 'long', 1.0, 55, 'lessons'),

('LESSON: clippy lint items_after_test_module rejects items declared after a `#[cfg(test)] mod ...` block in the same file. Always place test modules at the very bottom of lib.rs / module roots.', 'lesson,clippy,rust,testing', 5, 'procedure', 1746316800000, 'long', 1.0, 40, 'lessons'),

('LESSON: The MCP seed (mcp-data/shared/memory-seed.sql) is applied ONLY on first run when memory.db does not yet exist. Existing runtime DBs must be re-ingested via brain_ingest_* tools when shared seed content changes — there is no automatic re-seed.', 'lesson,mcp,seed,memory', 5, 'procedure', 1746316800000, 'long', 1.0, 50, 'lessons'),

('LESSON: Cypress was removed; do not reintroduce. Frontend tests use Vitest + Playwright only (docs/licensing-audit.md).', 'lesson,testing,dependencies', 4, 'preference', 1746316800000, 'long', 1.0, 30, 'lessons'),

('LESSON: GitHub Actions on the agent first push may show conclusion=action_required with zero jobs/logs. This means the workflow needs human approval, not that the code failed. Code-side validation (lint/build/test/clippy) is the source of truth.', 'lesson,ci,github-actions', 4, 'fact', 1746316800000, 'long', 1.0, 45, 'lessons'),

('LESSON: Per the Brain Documentation Sync rule (rules/architecture-rules.md rule 10), any change touching the brain surface (LLM providers, memory store, RAG pipeline, ingestion, embeddings, cognitive-kind classification, knowledge graph, decay/GC, brain-gating quests, brain Tauri commands or Pinia stores) MUST update both docs/brain-advanced-design.md and README.md in the same PR.', 'lesson,brain,docs,governance', 5, 'procedure', 1746316800000, 'long', 1.0, 70, 'lessons'),

('LESSON: AppState is Arc<AppStateInner> with auto-Deref, so background servers (MCP, gRPC) can hold cheap clones without lifetime issues. Use it for any server task that must outlive the originating Tauri command.', 'lesson,rust,architecture,state', 4, 'fact', 1746316800000, 'long', 1.0, 50, 'lessons'),

('LESSON: For provenance-linked brain write paths such as /reflect, use a dedicated persistence helper that returns concrete inserted memory IDs before writing memory_edges. Generic save_summary/save_facts helpers are fine for simple writes but do not expose IDs, so they cannot create reliable derived_from source-turn edges.', 'lesson,memory,provenance,reflection,brain-write', 5, 'procedure', 1746487655000, 'long', 1.0, 70, 'lessons'),

('LESSON: When making frontend changes, do NOT bulk-rewrite unrelated lint warnings — fix only what your change touches. ESLint enforces vue/max-attributes-per-line, self-closing void elements, and singleline-element newlines but tolerates pre-existing warnings.', 'lesson,frontend,lint,scope', 4, 'preference', 1746316800000, 'long', 1.0, 50, 'lessons'),

('LESSON: Never .unwrap() in library code. Use ? + thiserror. The crate roots use #![deny(unused_must_use)] so every Result must be handled.', 'lesson,rust,error-handling', 5, 'preference', 1746316800000, 'long', 1.0, 35, 'lessons'),

('LESSON: Use var(--ts-*) design tokens from src/style.css; never hardcode hex colors. Vue components use <script setup lang="ts"> with scoped styles only.', 'lesson,frontend,css,vue', 4, 'preference', 1746316800000, 'long', 1.0, 40, 'lessons'),

('LESSON: Do not commit MCP runtime state. .gitignore covers mcp-token.txt, memory.db, *.db-shm, *.db-wal, tasks.db*, workflows.sqlite, *.idx, *.lock, sessions/, worktrees/. Only mcp-data/shared/** and mcp-data/README.md are tracked.', 'lesson,mcp,gitignore,data-policy', 5, 'preference', 1746316800000, 'long', 1.0, 60, 'lessons'),

('LESSON: Self-improve runs in temporary git worktrees so the main checkout is never disturbed. Always read rules/milestones.md (next not-started chunk) and the top of rules/completion-log.md (recent context) before starting.', 'lesson,self-improve,git,workflow', 5, 'procedure', 1746316800000, 'long', 1.0, 55, 'lessons'),

-- ====================================================================
-- Standing user requirement (May 2026) — verbatim, never forget
-- ====================================================================
('STANDING REQUIREMENT (user, 2026-05-04): "Make sure mcp-data is refined and attached to git repo. Please analysis and choose wisely gitignore stuffs for mcp-data to expose shared data in GitHub repo for mcp usage of TerranSoul. Other people changes and self-improve will update these dataset. Make sure that the entire TerranSoul is saved in mcp-data as default dataset so we never forgot anything and repeat same problems twice and continue implement new features without duplication or rescan entire code base." Implementation: only mcp-data/shared/** + mcp-data/README.md are tracked; runtime tokens/DBs/indexes/logs/locks/sessions/worktrees are ignored. Shared dataset = memory-seed.sql + project-index.md + lessons-learned.md + brain_config.json + app_settings.json. Update them in any PR that adds durable knowledge. Every agent session must read project-index.md and lessons-learned.md (or query the brain) before scanning code or attempting features.', 'requirement,user,mcp-data,policy,governance', 5, 'preference', 1746316800000, 'long', 1.0, 200, 'requirements'),

('STANDING REQUIREMENT (user, 2026-05-04): every agent session must actively USE the MCP brain (start `npm run mcp`, verify GET /health, set TERRANSOUL_MCP_TOKEN_MCP from mcp-data/mcp-token.txt, then query brain_search / brain_suggest_context for project context BEFORE rescanning the codebase or implementing features). Self-improve writes new durable knowledge back to mcp-data/shared/ in the same PR.', 'requirement,user,mcp,session-protocol', 5, 'preference', 1746316800000, 'long', 1.0, 90, 'requirements'),

-- ====================================================================
-- Memory philosophy (markdown != memory) — see mcp-data/shared/memory-philosophy.md
-- ====================================================================
('CORE LESSON (Stop Calling It Memory, Jonathan Edwards 2026-03): Markdown files are NOT a memory system, NOT a database, NOT infrastructure — they are a notebook. .md files (CLAUDE.md, MEMORY.md, Obsidian vaults) are instruction-delivery mechanisms (prompts/config/sticky notes), not data stores. Treating them as memory creates 5 predictable failure modes: no querying, no relationships, scale ceiling (token cost grows linearly with vault), no schema enforcement (same fact gets formatted three ways), no concurrent access (silent corruption). Real memory needs a database with schema + indexes + query language. TerranSoul already implements this: SQLite + FTS5 + HNSW (usearch) + memory_edges KG + RRF k=60 fusion + HyDE + LLM-as-judge reranker + maintenance scheduler. Read mcp-data/shared/memory-philosophy.md for the full mapping table and the 7 non-negotiable rules for future PRs.', 'lesson,memory,architecture,philosophy,markdown,database', 5, 'fact', 1746316800000, 'long', 1.0, 200, 'memory'),

('RULE (memory philosophy): Markdown is for INSTRUCTIONS, not facts. The only files in the repo allowed to CONTAIN memory data (vs. describe it) are mcp-data/shared/memory-seed.sql and the runtime SQLite memory.db. Everything in rules/, docs/, .github/copilot-instructions.md, AGENTS.md, CLAUDE.md, .cursorrules, and mcp-data/shared/*.md is INSTRUCTION — prose pointing at where the data lives. Never propose "store memories as .md / Obsidian as the source of truth" — Obsidian is a one-way projection via obsidian_export.rs.', 'rule,memory,architecture,governance', 5, 'preference', 1746316800000, 'long', 1.0, 100, 'memory'),

('RULE (memory access pattern): Never bulk-load a whole "memory file" into the prompt context. Always retrieve via memory/store.rs hybrid_search_rrf, optionally HyDE + reranker, and inject through memory/context_pack.rs which wraps results in a [RETRIEVED CONTEXT] block that explicitly tells the LLM the contents are query results, not the whole DB. Never bypass the StorageBackend trait with ad-hoc file I/O for memory; new backends go alongside postgres.rs / cassandra.rs / mssql.rs.', 'rule,memory,rag,context-pack', 5, 'preference', 1746316800000, 'long', 1.0, 90, 'memory'),

('RULE (provenance): Every memory must populate source_url / source_hash / parent_id where applicable so we can answer "how do you know that?" — same pattern Claudia calls "show your sources" and the Substack article calls "the receipt." TerranSoul also has memory_versions for non-destructive history (versioning.rs), session_id, origin_device, and obsidian_path/last_exported for export tracking.', 'rule,memory,provenance,versioning', 5, 'preference', 1746316800000, 'long', 1.0, 80, 'memory'),

-- ====================================================================
-- Claudia (kbanc85/claudia) reverse-engineering — see mcp-data/shared/claudia-research.md
-- ====================================================================
('REFERENCE PROJECT — kbanc85/claudia (PolyForm Noncommercial 1.0.0): A Claude-Code-hosted personal companion that tracks people, commitments, and judgment rules. Architecture: markdown TEMPLATE LAYER (41 skills, identity, rules — instructions only) + Python MEMORY DAEMON (one SQLite DB shared by a per-session MCP daemon and a 24/7 standalone scheduler daemon). Hybrid search 50% vector + 25% importance + 10% recency + 15% FTS, with rehearsal-effect access boost. Scheduled jobs: adaptive decay (2 AM), consolidation/dedupe (3 AM), vault sync (3:15 AM, one-way to PARA-organised Obsidian), pattern detection (every 6 h). Slash skills include /morning-brief, /meditate, /memory-audit, /brain (3-D visualiser), /what-am-i-missing, /capture-meeting, /research, /follow-up-draft. README explicitly: "SQLite is the source of truth; the vault is a read-only projection." Read mcp-data/shared/claudia-research.md for the full inventory and the 10 adoption proposals mapped to existing TerranSoul modules.', 'reference,claudia,reverse-engineering,memory,architecture', 5, 'fact', 1746316800000, 'long', 1.0, 230, 'references'),

('CLAUDIA LICENSE (CRITICAL): kbanc85/claudia is licensed under PolyForm Noncommercial 1.0.0. Do NOT copy any source files, prompts, skill markdown, scheduler scripts, or asset names. Adopt PATTERNS and PRODUCT IDEAS from the public README only — that is standard reverse engineering and is allowed. TerranSoul ships as a free product, so commercial copy of PolyForm-NC code is forbidden. Credited in CREDITS.md.', 'rule,license,claudia,polyform-noncommercial', 5, 'preference', 1746316800000, 'long', 1.0, 80, 'licensing'),

('CLAUDIA ADOPTION PROPOSALS (from research file, in order of leverage): (1) judgment_rules persisted artefact (extend memories with category=judgment + memory_type=preference and inject into chat system prompt), (2) /meditate slash command exposing existing Chunks 30.2 + 30.6 session reflection, (3) /morning-brief daily quest (uses temporal.rs + hybrid_search_rrf), (4) /memory-audit view joining memories + memory_versions + memory_edges with provenance tree, (5) BrainGraphViewport.vue 3-D visualiser using Three.js (already loaded for VRM), (6) capability tags on agent roster routed by coding/coding_router.rs, (7) per-workspace data_root setting, (8) /health + /status — already aligned, (9) optional stdio MCP transport adapter on top of BrainGateway, (10) PARA-organised opt-in template for obsidian_export.rs. Each must be filed as a chunk in rules/milestones.md before implementation.', 'reference,claudia,adoption,roadmap', 4, 'fact', 1746316800000, 'long', 1.0, 180, 'references'),

('ANTI-PATTERN (claudia): Do NOT bolt on a Python memory daemon — TerranSoul memory is Rust in src-tauri/src/memory/ behind StorageBackend. Do NOT make Obsidian the user-facing primary surface — TerranSoul primary surface is chat + character + brain panel; Obsidian export stays opt-in. Do NOT introduce branded names ("claudia", literal /meditate label, "morning brief" as product noun) — pick neutral names per rules/coding-standards.md.', 'rule,claudia,anti-pattern,naming', 4, 'preference', 1746316800000, 'long', 1.0, 80, 'references'),

-- ====================================================================
-- Storage invariant — the seed itself eats the SQLite + KG-edges + RRF
-- + HyDE + reranker dog food. Documents how each layer is exercised so
-- agents don't suspect the seed is a "markdown vault in disguise."
-- ====================================================================
('STORAGE INVARIANT (mcp-data seed): mcp-data/shared/memory-seed.sql is REAL SQL inserted into the canonical SQLite memories + memory_edges schema (V15, schema.rs) — not a markdown vault. The seed actively exercises the full stack: (a) **schema** = memory_type/tier/decay_score/importance/category populated for every row, (b) **FTS5** = content is indexed automatically by schema.rs CREATE VIRTUAL TABLE, (c) **knowledge graph** = the `-- KNOWLEDGE GRAPH EDGES` section below populates typed memory_edges (part_of / cites / supports / derived_from / related_to) so brain_kg_neighbors works on day one, (d) **RRF fusion + HyDE + reranker** = all run at query time on whatever signals are populated. **HNSW vectors** are populated lazily: when a brain provider is configured, call the `backfill_embeddings` Tauri command (commands/memory.rs:593) — it walks store.unembedded_ids() and calls embed_for_mode for each. Until then, the 5 non-vector signals (keyword/recency/importance/decay/tier) still fully power RRF, brain_search, and brain_suggest_context.', 'storage,invariant,seed,architecture,kg,embeddings,backfill', 5, 'fact', 1746316800000, 'long', 1.0, 220, 'storage'),

('EMBEDDING BACKFILL PROCEDURE: After first MCP startup with the shared seed, vector signals are populated by the post-seed `backfill_mcp_seed_embeddings` hook when an embedding source is configured, or by the explicit `backfill_embeddings` Tauri command (commands/memory.rs:593) which iterates store.unembedded_ids() and calls embed_for_mode(content, brain_mode, active_brain) per row, then store.set_embedding(id, &emb). The shared maintenance scheduler now starts in both GUI and headless MCP modes for decay, GC, tier promotion, and edge extraction. For headless `npm run mcp` with no embed provider configured, embedding backfill is a no-op by design — RRF + HyDE + reranker still work on keyword/recency/importance/decay/tier signals, plus KG traversal and FTS5 keyword search work fully without vectors.', 'procedure,embeddings,backfill,mcp,headless,maintenance', 5, 'procedure', 1746316800000, 'long', 1.0, 150, 'procedures'),

('LESSON: GitNexus is PolyForm Noncommercial. TerranSoul must not bundle, vendor, auto-install, or default-spawn GitNexus packages, binaries, Docker images, prompts, skills, or UI assets. Treat GitNexus and its DeepWiki pages as credited public product and architecture research only, then implement neutral TerranSoul-native Rust/Vue code-intelligence features. Sidecar bridge code is removed entirely; the only supported MCP/code path is native TerranSoul code intelligence.', 'lesson,gitnexus,license,clean-room,code-intelligence,mcp,native,noncommercial', 10, 'procedure', 1746489600000, 'long', 1.0, 90, 'lessons'),

('CODE INTELLIGENCE ROADMAP: Native TerranSoul parity targets learned from public GitNexus docs are repo registry, incremental content-hash indexing, broader Tree-sitter language coverage, import/re-export resolution, heritage and receiver/type inference, confidence-scored code relations, functional clusters, execution processes, hybrid BM25 plus semantic plus RRF code search, diff impact, graph-backed rename, MCP resources/prompts, generated agent skills, and code-wiki generation. Implement these under neutral names using existing coding/symbol_index.rs, coding/processes.rs, memory RAG, and MCP tool surfaces.', 'roadmap,gitnexus,code-intelligence,native,mcp,symbol-index,processes,search', 9, 'fact', 1746489600000, 'long', 1.0, 95, 'code-intelligence'),

('CODE WORKBENCH UX LESSON: GitNexus public Web UI shows a useful AI-development pattern to reimplement natively: graph canvas as the primary structural map, file tree as physical navigation, code references panel as grounded evidence, right-side chat with visible tool-call cards, clickable file/node citations that focus graph and code, process diagrams, repo switcher, status bar, and blast-radius highlights for change risk. TerranSoul should adapt this as a dense Brain/Coding workbench using Vue, Pinia, Cytoscape or Three.js, and existing design tokens, not copy React components or visual identity.', 'lesson,gitnexus,ui-ux,code-workbench,graph,chat,citations,blast-radius', 9, 'fact', 1746489600000, 'long', 1.0, 95, 'frontend'),

('LESSON: LAN MCP brain sharing is an opt-in local-network retrieval flow. The host must enable LAN brain sharing before starting or restarting the MCP server, then name the shared brain and share the bearer token out-of-band. Discovery uses UDP 7424 for metadata only; authenticated retrieval uses MCP HTTP brain_search against the host port. Peers retrieve ranked snippets, not the host memory database.', 'lesson,mcp,lan,brain-sharing,discovery,token,remote-search,tutorial', 9, 'procedure', 1777939200000, 'long', 1.0, 75, 'mcp'),

('RULE: Treat LAN MCP bearer-token access as read access to the shared TerranSoul knowledge surface. Never broadcast the token, never enable LAN sharing on public Wi-Fi, and stop sharing when the session ends. User-facing docs should describe this with the tutorials/lan-mcp-sharing-tutorial.md Alice Vietnamese law notes scenario and avoid legal-advice claims.', 'rule,mcp,lan,security,bearer-token,docs,legal-disclaimer', 9, 'procedure', 1777939200000, 'long', 1.0, 70, 'mcp'),

('RULE: LAN MCP sharing must expose an explicit auth mode choice: `token_required` or `public_read_only`. Public mode may skip the bearer token only for the read-only brain MCP surface (initialize, ping, tools/list, and read-only brain tools); write tools, code-intelligence tools, /status, and hook endpoints remain authenticated.', 'rule,mcp,lan,auth-mode,public-read-only,token-required,security', 9, 'procedure', 1778112000000, 'long', 1.0, 95, 'mcp'),

('LESSON: LAN discovery should advertise whether a TerranSoul host requires a token, but it must never broadcast the token itself. UI flows should hide the token field for public-read-only peers and still treat public mode as read access to the shared knowledge surface.', 'lesson,mcp,lan,discovery,auth-mode,public-read-only,token-ui', 8, 'procedure', 1778112000000, 'long', 1.0, 85, 'mcp'),

('LESSON: MCP-mode self-improve runtime logs live under mcp-data/ and are bounded runtime state, not durable project memory. self_improve_runs.jsonl, self_improve_gates.jsonl, and self_improve_mcp.jsonl each keep only the current file plus a .001 archive, with a 1 MiB cap per file. The UI reads both current and archive, while durable lessons still belong in mcp-data/shared/memory-seed.sql and numbered migrations.', 'lesson,mcp,self-improve,logs,rolling-log,mcp-data,jsonl,runtime-state', 9, 'procedure', 1778025600000, 'long', 1.0, 85, 'mcp'),

('RULE: New self-improve runtime logs must use coding::rolling_log or an equivalent current-plus-.001 rotation policy before writing under mcp-data/. Do not create unbounded MCP runtime logs, do not commit runtime logs, and do not treat runtime JSONL as the durable MCP knowledge source.', 'rule,mcp,self-improve,logs,rotation,runtime-state,shared-seed', 9, 'procedure', 1778025600000, 'long', 1.0, 70, 'mcp'),

('LESSON: For MCP 7423 startup, a healthy process is not sufficient if target-mcp is stale. If target-mcp/release/terransoul(.exe) is older than src-tauri sources/config, startup must terminate managed MCP, rebuild target-mcp, relaunch, and re-check /health. If termination fails, report a blocker instead of silently reusing stale binaries.', 'lesson,mcp,target-mcp,stale-binary,rebuild,relaunch,startup', 9, 'procedure', 1778025600000, 'long', 1.0, 85, 'mcp'),

('RULE: Agents must not reuse MCP port 7423 when target-mcp is out of date. Required sequence is terminate -> rebuild -> relaunch -> health-check; skipping any step is a process violation because it hides stale runtime behavior.', 'rule,mcp,target-mcp,stale-binary,terminate,rebuild,relaunch,health-check', 9, 'procedure', 1778025600000, 'long', 1.0, 70, 'mcp'),

('RULE: TerranSoul MCP preflight must be visible to the user, not only hidden in tool calls. After brain_health plus a relevant brain_search or brain_suggest_context succeeds, the agent must send a short MCP receipt naming the health/provider result and the search/context query topic. If MCP is blocked, the receipt must name the blocker. If the user cannot see the receipt, treat preflight as incomplete.', 'rule,mcp,preflight,visible-receipt,user-visible,enforcement,non-negotiable', 10, 'procedure', 1778025600000, 'long', 1.0, 90, 'mcp'),

('LESSON: Text-only MCP rules were not enough because users cannot see hidden tool calls. The durable fix is to require a visible MCP receipt in .github/instructions/mcp-preflight.instructions.md, .github/copilot-instructions.md, AGENTS.md, CLAUDE.md, .cursorrules, rules/agent-mcp-bootstrap.md, and MCP seed memory.', 'lesson,mcp,preflight,visible-receipt,instruction-sync,user-trust', 9, 'procedure', 1778025600000, 'long', 1.0, 75, 'mcp'),

('RULE: Migration/schema bootstrap must resolve shared seed sources in deterministic order across release, dev, and MCP modes: TERRANSOUL_MCP_SHARED_DIR (override) -> <data_dir>/shared -> <cwd>/mcp-data/shared -> compiled fallback. This guarantees predictable boot while letting local dev/release runs consume checked-in mcp-data/shared updates without manual copy steps.', 'rule,mcp,seed,migrations,schema,dev,release,shared-data,resolution-order', 9, 'procedure', 1778025600000, 'long', 1.0, 95, 'mcp'),

('LESSON: Relying only on <data_dir>/shared for seed migrations caused dev/release drift from repository mcp-data/shared when no runtime shared folder existed. The durable fix is explicit source resolution plus startup logging of the selected source, with compiled SQL as the final fallback.', 'lesson,mcp,seed,migrations,schema,drift,dev,release,fallback', 9, 'procedure', 1778025600000, 'long', 1.0, 80, 'mcp'),

('RULE: Fresh MCP databases should bootstrap from a single init snapshot (mcp-data/shared/memory-seed.sql) and then apply only future numbered deltas. Keep numbered migrations append-only for compatibility/history, but avoid replaying all historical scripts on first boot.', 'rule,mcp,seed,migrations,init-snapshot,bootstrap,performance', 9, 'procedure', 1778025600000, 'long', 1.0, 100, 'mcp'),

('VERDICT: Reject direct GitNexus import/bundling. TerranSoul must keep clean-room native code-intelligence UX inspired by public behavior only; no GitNexus binaries, Docker images, prompts, skills, or UI assets may be bundled, auto-installed, or default-spawned due PolyForm Noncommercial constraints.', 'verdict,gitnexus,clean-room,license,ui-ux,native,noncommercial', 10, 'decision', 1778025600000, 'long', 1.0, 100, 'code-intelligence'),

('LESSON: LocalLLM fast chat path (2026-05-09): short content-light turns such as "Hi", "Hello", "OK", or "who are you" must not call the intent classifier, embedding model, or hybrid RAG retrieval. On consumer GPUs, gemma4:e4b can fill VRAM and loading nomic-embed-text evicts the chat model, causing 5-15s model swaps. The durable fix is pure-code fast-path guards in src/stores/conversation.ts, src-tauri/src/brain/intent_classifier.rs, and src-tauri/src/commands/streaming.rs; contentful questions still use classifier + full RAG.', 'lesson,perf,rag,streaming,vram,ollama,local-llm,fast-path', 10, 'procedure', 1778284800000, 'long', 1.0, 115, 'brain'),

('LESSON: LocalOllama VRAM eviction by background workers (2026-05-12): In LocalOllama mode the embedding worker (10s tick) and any helper that calls /api/embed (HyDE, late-chunking, batch ingest) loads nomic-embed-text into VRAM, which evicts the chat model (e.g. gemma4:e4b) and adds a 10-20s reload cost on the very next chat reply. The first attempted fix still let the real desktop app take ~10s because AppState.last_chat_at_ms started at 0 and spawn_embedding_queue_worker ran before spawn_local_ollama_warmup; tokio interval workers tick immediately, so the embedding backfill could seize Ollama at app startup before the first user chat. The durable fix is layered: (1) AppState gains last_chat_at_ms: AtomicU64 initialized to now for production AppState and set by run_chat_stream/process_message on every user turn; (2) startup calls spawn_local_ollama_warmup before spawn_embedding_queue_worker; (3) the embedding worker skips ticks while last chat/startup quiet-window was within 5 minutes when provider_category == "ollama"; (4) every Ollama embed body sets keep_alive: 0 so the embed model unloads immediately after each batch; (5) every /api/chat caller sets keep_alive: "30m"; (6) stream_ollama sends think:false so Gemma/Qwen thinking models do not spend seconds in a silent reasoning phase before visible content; (7) LocalOllama non-streaming fallback uses keyword-only memories instead of LLM semantic_search_entries before answering. Verified with Playwright Real-E2E hi-latency including a real Rust run_chat_stream probe: first real backend llm-chunk 537ms with gemma4:e4b; direct Hi chat 595ms; chat-only 681ms; all under 1s first-token target.', 'lesson,perf,vram,ollama,local-llm,embedding-queue,keep-alive,warmup,streaming,real-app-latency', 10, 'procedure', 1778544000000, 'long', 1.0, 150, 'brain');

-- ====================================================================
-- KNOWLEDGE GRAPH EDGES
--
-- Wire the seeded memories into a typed graph so the KG layer
-- (memory_edges, brain_kg_neighbors, graph_rag.rs) is exercised
-- immediately on first run, instead of being dead weight until LLM
-- extraction populates it.
--
-- We match by unique substring of `content` rather than hard-coded
-- AUTOINCREMENT IDs so the seed is robust to row reordering. Each
-- edge uses INSERT OR IGNORE against the UNIQUE(src_id, dst_id,
-- rel_type) constraint, so re-running the seed (e.g. against a fresh
-- DB on a different machine) is idempotent.
--
-- rel_type vocabulary follows POSITIVE_REL_TYPES from
-- edge_conflict_scan.rs: supports, implies, related_to, derived_from,
-- cites, part_of. `normalise_rel_type` (edges.rs:199) lowercases and
-- strips, so the literals below already match the stored form.
-- ====================================================================

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1777939200000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: LAN MCP brain sharing is an opt-in local-network retrieval flow%'
  AND (
       d.content LIKE 'MCP server exposes brain on three ports:%'
    OR d.content LIKE 'MCP shared data policy:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1777939200000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: Treat LAN MCP bearer-token access as read access%'
  AND d.content LIKE 'LESSON: LAN MCP brain sharing is an opt-in local-network retrieval flow%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778112000000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: LAN MCP sharing must expose an explicit auth mode choice%'
  AND (
       d.content LIKE 'LESSON: LAN MCP brain sharing is an opt-in local-network retrieval flow%'
    OR d.content LIKE 'RULE: Treat LAN MCP bearer-token access as read access%'
    OR d.content LIKE 'MCP server exposes brain on three ports:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778112000000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: LAN discovery should advertise whether a TerranSoul host requires a token%'
  AND d.content LIKE 'RULE: LAN MCP sharing must expose an explicit auth mode choice%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: MCP-mode self-improve runtime logs live under mcp-data/%'
  AND (
       d.content LIKE 'MCP shared data policy:%'
    OR d.content LIKE 'Self-improve with MCP mode:%'
    OR d.content LIKE 'LESSON: Do not commit MCP runtime state.%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: New self-improve runtime logs must use coding::rolling_log%'
  AND d.content LIKE 'LESSON: MCP-mode self-improve runtime logs live under mcp-data/%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: For MCP 7423 startup, a healthy process is not sufficient if target-mcp is stale%'
  AND (
       d.content LIKE 'MCP AUTO-START TASK:%'
    OR d.content LIKE 'MCP PREFLIGHT ENFORCEMENT:%'
    OR d.content LIKE 'LESSON: MCP-mode self-improve runtime logs live under mcp-data/%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: Agents must not reuse MCP port 7423 when target-mcp is out of date%'
  AND d.content LIKE 'LESSON: For MCP 7423 startup, a healthy process is not sufficient if target-mcp is stale%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: TerranSoul MCP preflight must be visible to the user%'
  AND (
       d.content LIKE 'MCP EVERY-SESSION RULE:%'
    OR d.content LIKE 'MCP PREFLIGHT ENFORCEMENT:%'
    OR d.content LIKE 'MCP PREFLIGHT INSTRUCTIONS FILE:%'
    OR d.content LIKE 'RULES ENFORCEMENT BUNDLE:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: Text-only MCP rules were not enough%'
  AND d.content LIKE 'RULE: TerranSoul MCP preflight must be visible to the user%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: Migration/schema bootstrap must resolve shared seed sources in deterministic order%'
  AND (
       d.content LIKE 'MCP shared data policy:%'
    OR d.content LIKE 'MCP server exposes brain on three ports:%'
    OR d.content LIKE 'LESSON: MCP-mode self-improve runtime logs live under mcp-data/%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: Relying only on <data_dir>/shared for seed migrations caused dev/release drift%'
  AND d.content LIKE 'RULE: Migration/schema bootstrap must resolve shared seed sources in deterministic order%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE: Fresh MCP databases should bootstrap from a single init snapshot%'
  AND (
       d.content LIKE 'MCP shared data policy:%'
    OR d.content LIKE 'RULE: Migration/schema bootstrap must resolve shared seed sources in deterministic order%'
    OR d.content LIKE 'LESSON: MCP-mode self-improve runtime logs live under mcp-data/%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1778025600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'VERDICT: Reject direct GitNexus import/bundling.%'
  AND (
       d.content LIKE 'LESSON: GitNexus is PolyForm Noncommercial.%'
    OR d.content LIKE 'CODE WORKBENCH UX LESSON:%'
  );

-- Hub edges: every inventory fact is part_of the project-index pointer
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE d.content LIKE '%MCP shared seed dataset lives in mcp-data/shared/memory-seed.sql%'
  AND (
       s.content LIKE 'Brain module map:%'
    OR s.content LIKE 'Memory module map:%'
    OR s.content LIKE 'ai_integrations exposes the brain%'
    OR s.content LIKE 'Persona module map:%'
    OR s.content LIKE 'Voice module map:%'
    OR s.content LIKE 'coding/ module map:%'
    OR s.content LIKE 'commands/ files (~150 Tauri commands):%'
    OR s.content LIKE 'Frontend Pinia stores in src/stores/:%'
    OR s.content LIKE 'Frontend composables in src/composables/:%'
    OR s.content LIKE 'Design docs (docs/):%'
    OR s.content LIKE 'Rules files (rules/):%'
    OR s.content LIKE 'Pose pipeline:%'
    OR s.content LIKE 'Orchestrator submodules:%'
    OR s.content LIKE 'Plugins run in a WASM sandbox:%'
    OR s.content LIKE 'Sync primitives in src-tauri/src/sync/:%'
    OR s.content LIKE 'Device identity uses Ed25519.%'
    OR s.content LIKE 'Self-improve flow:%'
  );

-- Hub edges: every LESSON is part_of the lessons-learned pointer
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE d.content LIKE '%Durable gotchas, decisions, and lessons learned%'
  AND s.content LIKE 'LESSON:%';

-- Memory-philosophy cluster: rules derive_from the core lesson; the
-- core lesson cites the concrete TerranSoul implementation it endorses
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'derived_from', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE d.content LIKE 'CORE LESSON (Stop Calling It Memory%'
  AND (
       s.content LIKE 'RULE (memory philosophy):%'
    OR s.content LIKE 'RULE (memory access pattern):%'
    OR s.content LIKE 'RULE (provenance):%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'cites', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CORE LESSON (Stop Calling It Memory%'
  AND (
       d.content LIKE 'Hybrid 6-signal search weights live in src-tauri/src/memory/store.rs%'
    OR d.content LIKE 'SQLite schema is at version 15%'
    OR d.content LIKE 'Memory module map:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULE (memory philosophy):%'
  AND d.content LIKE 'MCP shared data policy: mcp-data/shared is committed%';

-- Claudia cluster: adoption + license + anti-pattern are part_of the
-- reference; the reference cites the memory-philosophy core lesson
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE d.content LIKE 'REFERENCE PROJECT — kbanc85/claudia%'
  AND (
       s.content LIKE 'CLAUDIA LICENSE (CRITICAL):%'
    OR s.content LIKE 'CLAUDIA ADOPTION PROPOSALS%'
    OR s.content LIKE 'ANTI-PATTERN (claudia):%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'derived_from', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CLAUDIA ADOPTION PROPOSALS%'
  AND d.content LIKE 'REFERENCE PROJECT — kbanc85/claudia%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'cites', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'REFERENCE PROJECT — kbanc85/claudia%'
  AND d.content LIKE 'CORE LESSON (Stop Calling It Memory%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 0.8, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CLAUDIA ADOPTION PROPOSALS%'
  AND (
       d.content LIKE 'Memory module map:%'
    OR d.content LIKE 'coding/ module map:%'
    OR d.content LIKE 'commands/ files (~150 Tauri commands):%'
    OR d.content LIKE 'Frontend Pinia stores in src/stores/:%'
  );

-- Standing requirements cluster: explicit support + relation links
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'STANDING REQUIREMENT (user, 2026-05-04): "Make sure mcp-data%'
  AND d.content LIKE 'MCP shared data policy: mcp-data/shared is committed%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'STANDING REQUIREMENT (user, 2026-05-04): every agent session%'
  AND d.content LIKE 'To start MCP headless server:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'STANDING REQUIREMENT (user, 2026-05-04): "Make sure mcp-data%'
  AND (
       d.content LIKE '%MCP shared seed dataset lives in mcp-data/shared/memory-seed.sql%'
    OR d.content LIKE '%Durable gotchas, decisions, and lessons learned%'
  );

-- Storage invariant + backfill procedure connect to the architectural facts
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'STORAGE INVARIANT (mcp-data seed):%'
  AND (
       d.content LIKE 'CORE LESSON (Stop Calling It Memory%'
    OR d.content LIKE 'SQLite schema is at version 15%'
    OR d.content LIKE 'Hybrid 6-signal search weights live in src-tauri/src/memory/store.rs%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'EMBEDDING BACKFILL PROCEDURE:%'
  AND d.content LIKE 'STORAGE INVARIANT (mcp-data seed):%';

-- ============================================================================
-- STACK COVERAGE ANCHOR (single hub node any agent can land on first)
-- ============================================================================
-- Documents which memory/retrieval layer each seeded fact-cluster exercises.
-- Future Phase 33 work (see rules/milestones.md) must keep this in sync.
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'STACK COVERAGE: the mcp-data seed exercises the full TerranSoul retrieval stack — SQLite schema V15 (every row), FTS5 (auto-indexed on insert), KG edges (memory_edges populated by content-LIKE subqueries below), RRF fusion (5 non-vector signals always live; vector signal lights up after embedding backfill), HyDE expansion (works on any populated row at query time), and LLM-as-judge reranker (default-on for RRF/HyDE when a local brain is available, pruning below threshold 0.55).',
  'mcp-data,architecture,retrieval-stack,phase-33,non-negotiable',
  10, 'fact', 1746316800000, 'long', 1.0, 'general', 'principle'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE '%Durable gotchas, decisions, and lessons learned%'
  AND d.content LIKE 'STACK COVERAGE: the mcp-data seed exercises%';

-- Wire the anchor to every other seed hub so RRF + KG traversal both land here first.
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'STACK COVERAGE: the mcp-data seed exercises%'
  AND (
       d.content LIKE 'STORAGE INVARIANT (mcp-data seed):%'
    OR d.content LIKE 'EMBEDDING BACKFILL PROCEDURE:%'
    OR d.content LIKE 'CORE LESSON (Stop Calling It Memory%'
    OR d.content LIKE '%lessons-learned.md captures durable%'
    OR d.content LIKE '%project-index.md is the single source of truth%'
    OR d.content LIKE 'Hybrid 6-signal search weights live in src-tauri/src/memory/store.rs%'
  );

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP EVERY-SESSION RULE: every coding agent run must use TerranSoul MCP as the project-memory layer when available — check/reuse/start the release, dev, or headless MCP server; call brain_health plus a relevant brain_search or brain_suggest_context before broad repo exploration; and preserve durable self-improve lessons in mcp-data/shared or rules/docs. If MCP is blocked, report the blocker instead of silently skipping it.',
  'mcp,agent-rule,self-improve,project-memory,non-negotiable',
  10, 'fact', 1746316800000, 'long', 1.0, 'general', 'principle'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP EVERY-SESSION RULE:%'
  AND (
       d.content LIKE 'STACK COVERAGE: the mcp-data seed exercises%'
    OR d.content LIKE 'STORAGE INVARIANT (mcp-data seed):%'
    OR d.content LIKE 'EMBEDDING BACKFILL PROCEDURE:%'
  );

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP AUTOSTART + OFFLINE EMBEDDER: Copilot setup runs scripts/copilot-start-mcp.mjs after dependency install to reuse or start headless MCP on port 7423. Fresh headless MCP seed/query embedding falls back to a deterministic in-process token-hash embedder when provider embeddings are unavailable, keeping HNSW + RRF vector retrieval active with zero network.',
  'mcp,autostart,offline-embedder,hnsw,rrf,copilot-setup,self-improve',
  10, 'fact', 1746316800000, 'long', 1.0, 'general', 'principle'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP AUTOSTART + OFFLINE EMBEDDER:%'
  AND (
       d.content LIKE 'MCP EVERY-SESSION RULE:%'
    OR d.content LIKE 'STACK COVERAGE: the mcp-data seed exercises%'
    OR d.content LIKE 'EMBEDDING BACKFILL PROCEDURE:%'
  );

-- ============================================================================
-- Rules enforcement coverage — high-priority operational rules from rules/*.md
-- ============================================================================
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'RULES ENFORCEMENT BUNDLE: every agent must query MCP for project rules before broad exploration, then obey rules/milestones.md, rules/prompting-rules.md, rules/coding-standards.md, rules/architecture-rules.md, rules/reality-filter.md, and rules/quality-pillars.md. If a rule is missing from retrieval, update mcp-data/shared in the same PR so future agents cannot skip it.',
  'rules,enforcement,mcp,self-improve,non-negotiable',
  10, 'fact', 1746316800000, 'long', 1.0, 'rules', 'principle'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MILESTONE HYGIENE RULE: rules/milestones.md is the active queue only. It may contain only not-started or in-progress chunks. When a chunk is complete, log full details in rules/completion-log.md, remove the done row from milestones.md, drop empty phase headings, and update Next Chunk to the next not-started item. Never leave done rows in milestones.md.',
  'rules,milestones,completion-log,chunk-hygiene,non-negotiable',
  10, 'procedure', 1746316800000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'BACKLOG PROMOTION RULE: rules/backlog.md is a holding area only. Agents must never start backlog chunks directly. If milestones.md has no not-started chunks, stop and ask the user which backlog items to promote; only after user confirmation move selected items into milestones.md and continue.',
  'rules,backlog,milestones,user-confirmation,non-negotiable',
  10, 'procedure', 1746316800000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PROMPT CONTEXT RULE: every coding workflow/self-improve prompt must auto-load rules/*.md, instructions/*.md, and docs/*.md into indexed XML document blocks through the shared CodingPrompt/run_coding_task path. Do not create one-off prompt builders that bypass these rule documents.',
  'rules,prompting,self-improve,codingprompt,documents',
  9, 'procedure', 1746316800000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MULTI-AGENT INSTRUCTION SYNC RULE: .github/copilot-instructions.md is canonical. When changing project-wide instructions, keep AGENTS.md, CLAUDE.md, and .cursorrules quick references aligned in the same commit. rules/agent-mcp-bootstrap.md must stay agent-agnostic even when Copilot setup owns cloud autostart.',
  'rules,instruction-sync,copilot,agents,claude,cursor',
  9, 'procedure', 1746316800000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'DOCUMENTATION SYNC RULES: any brain-surface change must update both docs/brain-advanced-design.md and README.md; any persona-surface change must update both docs/persona-design.md and README.md. Brain/persona PRs missing those docs are incomplete.',
  'rules,docs-sync,brain,persona,readme,non-negotiable',
  10, 'procedure', 1746316800000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'CREDITS AND LICENSING RULE: any external author, open-source project, paper, docs page, dataset, tutorial, or reverse-engineered reference that informs code, docs, roadmap, prompts, feature catalogues, or rejected decisions must be credited in top-level CREDITS.md in the same PR. Use neutral TerranSoul names for runtime identifiers.',
  'rules,credits,licensing,third-party,naming,non-negotiable',
  9, 'procedure', 1746316800000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'NO PRETEND CODE RULE: production code must be real, compilable, and functional. No TODO/placeholder/future/subsequent-chunk comments, no empty trait impls, no mock/canned production responses, no symbolic scaffolding. Either implement fully with tests or track it as a milestone/backlog item and remove it from user-reachable paths.',
  'rules,reality-filter,no-mocks,production-ready,non-negotiable',
  10, 'procedure', 1746316800000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'LLM DECISION ROUTING RULE: decisions about what the AI should do — agent/tool choice, UX overlay, RAG injection, model switching, setup gates — must route through the configured brain classifier/command with a user toggle in src/stores/ai-decision-policy.ts. Hand-rolled regex/includes/keyword arrays for AI behaviour are banned except documented parsing/fallback exceptions.',
  'rules,llm-decision,ai-routing,brain,classifier,non-negotiable',
  10, 'procedure', 1746316800000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'VALIDATION AND REALITY RULE: after each chunk, run relevant existing lint/build/tests and report blockers honestly. Do not claim verification that did not run; label unverified/inferred claims. Code changes must also run security review via CodeQL checker when available, and dependency additions must check GH advisory data first for supported ecosystems.',
  'rules,validation,reality-filter,codeql,security,non-negotiable',
  10, 'procedure', 1746316800000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP MARKDOWN BOUNDARY RULE: Markdown files are not TerranSoul MCP memory. If rules/docs/lessons Markdown contains durable project knowledge for future agents, sync the same knowledge into mcp-data/shared/memory-seed.sql and connect it with memory_edges so SQLite plus the knowledge graph remains the authoritative MCP memory source.',
  'rules,mcp,markdown,memory,knowledge-graph,non-negotiable',
  10, 'procedure', 1746416974000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'RULES ENFORCEMENT BUNDLE:%'
  AND (
       d.content LIKE 'MCP EVERY-SESSION RULE:%'
    OR d.content LIKE 'MILESTONE HYGIENE RULE:%'
    OR d.content LIKE 'BACKLOG PROMOTION RULE:%'
    OR d.content LIKE 'PROMPT CONTEXT RULE:%'
    OR d.content LIKE 'MULTI-AGENT INSTRUCTION SYNC RULE:%'
    OR d.content LIKE 'DOCUMENTATION SYNC RULES:%'
    OR d.content LIKE 'CREDITS AND LICENSING RULE:%'
    OR d.content LIKE 'NO PRETEND CODE RULE:%'
    OR d.content LIKE 'LLM DECISION ROUTING RULE:%'
    OR d.content LIKE 'VALIDATION AND REALITY RULE:%'
    OR d.content LIKE 'MCP MARKDOWN BOUNDARY RULE:%'
    OR d.content LIKE 'Rules files (rules/):%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746316800000, 'seed'
FROM memories s, memories d
WHERE d.content LIKE 'RULES ENFORCEMENT BUNDLE:%'
  AND (
       s.content LIKE 'MILESTONE HYGIENE RULE:%'
    OR s.content LIKE 'BACKLOG PROMOTION RULE:%'
    OR s.content LIKE 'PROMPT CONTEXT RULE:%'
    OR s.content LIKE 'MULTI-AGENT INSTRUCTION SYNC RULE:%'
    OR s.content LIKE 'DOCUMENTATION SYNC RULES:%'
    OR s.content LIKE 'CREDITS AND LICENSING RULE:%'
    OR s.content LIKE 'NO PRETEND CODE RULE:%'
    OR s.content LIKE 'LLM DECISION ROUTING RULE:%'
    OR s.content LIKE 'VALIDATION AND REALITY RULE:%'
    OR s.content LIKE 'MCP MARKDOWN BOUNDARY RULE:%'
    OR s.content LIKE 'MCP PREFLIGHT ENFORCEMENT:%'
  );

-- ============================================================================
-- MCP Preflight Enforcement (added 2026-05-05)
-- ============================================================================
-- These entries document the THREE mechanical enforcement layers that prevent
-- agents from silently skipping MCP. Without these, agents read the rule and
-- then violate it anyway because rules-in-markdown have no teeth.
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP PREFLIGHT ENFORCEMENT: three layers enforce MCP usage for every agent session. (1) VS Code task "TerranSoul MCP: Auto-Start" in .vscode/tasks.json with runOptions.runOn=folderOpen auto-launches node scripts/copilot-start-mcp.mjs when the workspace opens — no agent action required. (2) .github/instructions/mcp-preflight.instructions.md with applyTo="**" is loaded by VS Code Copilot on EVERY request as mandatory context — it instructs the agent to call brain_health before any work. (3) The MCP mandate is the FIRST section in .github/copilot-instructions.md, AGENTS.md, and CLAUDE.md (before architecture, before tech stack) so it is never truncated by context-window limits. All three layers work together: auto-start ensures MCP is running, the .instructions.md ensures the agent knows to use it, and the top-of-file placement ensures the rule is never lost to summarization.',
  'mcp,preflight,enforcement,auto-start,instructions,non-negotiable',
  10, 'procedure', 1746489600000, 'long', 1.0, 'rules', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP AUTO-START TASK: .vscode/tasks.json contains a task labeled "TerranSoul MCP: Auto-Start" with type=shell, command=node scripts/copilot-start-mcp.mjs, isBackground=true, and runOptions.runOn=folderOpen. This means VS Code launches it automatically whenever the TerranSoul workspace folder is opened. The script checks ports 7421/7422/7423, reuses any running MCP, or starts npm run mcp detached if none is healthy. No manual agent intervention needed.',
  'mcp,auto-start,vscode-task,setup,enforcement',
  9, 'fact', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP PREFLIGHT INSTRUCTIONS FILE: .github/instructions/mcp-preflight.instructions.md with YAML frontmatter applyTo="**" is auto-loaded by VS Code Copilot on every agent request regardless of which file the user is editing. It tells the agent to call brain_health, then brain_search/brain_suggest_context before any codebase exploration. If MCP is not healthy, start it. If MCP cannot start, report the blocker. This is VS Code''s native per-request instruction injection — the agent cannot skip it because VS Code prepends it to every prompt.',
  'mcp,preflight,instructions-file,vscode,enforcement,copilot',
  9, 'fact', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP PREFLIGHT ENFORCEMENT:%'
  AND (
       d.content LIKE 'MCP EVERY-SESSION RULE:%'
    OR d.content LIKE 'MCP AUTOSTART + OFFLINE EMBEDDER:%'
    OR d.content LIKE 'RULES ENFORCEMENT BUNDLE:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'part_of', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE d.content LIKE 'MCP PREFLIGHT ENFORCEMENT:%'
  AND (
       s.content LIKE 'MCP AUTO-START TASK:%'
    OR s.content LIKE 'MCP PREFLIGHT INSTRUCTIONS FILE:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP AUTO-START TASK:%'
  AND d.content LIKE 'To start MCP headless server:%';

-- ============================================================================
-- MCP write capability lesson (added 2026-05-05)
-- ============================================================================
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP WRITE CAPABILITY FIX (2026-05-05): MCP transports must use the explicit transport capability profile, not GatewayCaps::default(). Root cause of brain_ingest_url permission denied was src-tauri/src/ai_integrations/mcp/mod.rs constructing brain_write=false and stdio using GatewayCaps::default(). Fix: mcp::transport_caps() returns GatewayCaps::READ_WRITE for HTTP bearer-token MCP and trusted stdio MCP. GatewayCaps::default remains read-only for tests/future embedders. This allows coding agents to persist durable self-improve/research knowledge through brain_ingest_url while preserving fail-closed defaults outside MCP.',
  'mcp,brain_write,capabilities,permission-fix,self-improve,transport-caps',
  9, 'procedure', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP WRITE CAPABILITY FIX:%'
  AND (
       d.content LIKE 'MCP EVERY-SESSION RULE:%'
    OR d.content LIKE 'SELF-IMPROVE WRITE-BACK CONTRACT:%'
    OR d.content LIKE 'MCP SELF-LEARNING RULE:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP WRITE CAPABILITY FIX:%'
  AND d.content LIKE 'MCP server exposes brain on three ports:%';

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP INGEST SINK FIX (2026-05-05): brain_ingest_url requires both brain_write caps and a production IngestSink. After fixing caps, MCP returned "not configured: ingest sink not attached" because src-tauri/src/ai_integrations/mcp/mod.rs still created AppStateGateway::new(). Fix: when start_server_with_activity receives a Tauri AppHandle, construct AppHandleIngestSink and AppStateGateway::with_ingest(); the sink calls commands::ingest::ingest_document and returns IngestUrlResponse task_id/source/source_type. HTTP MCP app/tray mode can now start real background ingest tasks. Stdio remains a trusted transport for reads/write caps, but URL ingestion requires an AppHandle-backed process unless a direct non-UI ingest sink is added later.',
  'mcp,ingest_url,ingest-sink,apphandle,brain_write,self-improve',
  9, 'procedure', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP INGEST SINK FIX:%'
  AND (
       d.content LIKE 'MCP WRITE CAPABILITY FIX:%'
    OR d.content LIKE 'MCP server exposes brain on three ports:%'
    OR d.content LIKE 'ai_integrations exposes the brain%'
  );

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP STDIO INGEST SINK FIX (2026-05-05): editor MCP clients may connect through terransoul --mcp-stdio rather than HTTP 7423. Stdio already uses READ_WRITE caps, but brain_ingest_url still failed with "not configured: ingest sink not attached" because stdio::run_with_state constructed AppStateGateway::new(state). Fix: attach StdioIngestSink via AppStateGateway::with_ingest(); the sink calls commands::ingest::ingest_document_silent(), which starts the real background ingest pipeline against AppState without requiring a Tauri AppHandle or emitting WebView progress events. Both HTTP MCP tray and stdio MCP now support brain_ingest_url writes.',
  'mcp,stdio,ingest_url,ingest-sink,brain_write,self-improve',
  9, 'procedure', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP STDIO INGEST SINK FIX:%'
  AND (
       d.content LIKE 'MCP INGEST SINK FIX:%'
    OR d.content LIKE 'MCP WRITE CAPABILITY FIX:%'
    OR d.content LIKE 'MCP server exposes brain on three ports:%'
    OR d.content LIKE 'ai_integrations exposes the brain%'
  );

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MULTI-AGENT ORCHESTRATION RESEARCH SOURCE CHECK (2026-05-05): for public GitHub multi-agent projects, first check DeepWiki when reachable, then cross-check upstream repo/docs/license and TerranSoul source before proposing adoption. The 2026-05 survey covered a self-hosted multi-agent dashboard plus LangGraph, CrewAI, AutoGen, Semantic Kernel, OpenAI Agents SDK, Google ADK, LlamaIndex Workflows, Pydantic AI, Haystack Agents, Agno, and Mastra. Credit all sources in CREDITS.md and keep TerranSoul names neutral.',
  'research,multi-agent,reverse-engineering,deepwiki,credits,self-improve',
  9, 'procedure', 1746489600000, 'long', 1.0, 'research', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MULTI-AGENT ORCHESTRATION VERDICT (2026-05-05): partial-adopt. TerranSoul should absorb durable lineage, per-session tool bundles, bounded swarm joins, task queue recovery, quality gates, trace/eval hooks, and dense operations UI patterns. Do not import a Next.js/Node dashboard stack, hosted workflow service, or another memory store. Keep orchestration local-first in Rust/Tauri and reuse TerranSoul memory/RAG, provider policy, MCP caps, and LAN sharing.',
  'multi-agent,orchestration,verdict,partial-adopt,local-first,rust-tauri',
  10, 'fact', 1746489600000, 'long', 1.0, 'architecture', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'TERRANSOUL MULTI-AGENT BASELINE (2026-05-05): current equivalents include coding/multi_agent.rs workflow plans and recurrence, coding/dag_runner.rs dependency layers and parallel execution, coding/task_queue.rs persistent SQLite retry queue, workflows/engine.rs durable SQLite event log, workflows/resilience.rs retry/timeout/circuit-breaker/watchdog, coding/engine.rs self-improve gates, coding/gate_telemetry.rs gate metrics, workflow-plans.ts plan UI store, agent-roster.ts handoff profiles, MCP brain exposure, and LAN MCP brain sharing. Next work is integration, not a rewrite.',
  'multi-agent,terransoul-baseline,coding-workflow,dag-runner,task-queue,workflow-engine',
  9, 'fact', 1746489600000, 'long', 1.0, 'architecture', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MULTI-AGENT BACKEND ADOPTION PLAN (2026-05-05): add durable run lineage with parent/child ids, agent role, model, status, tool-bundle hash, cancellation flag, timestamps, and verdict; emit typed sub-agent events; build session tool bundles from capability policy, provider policy, MCP transport caps, plan role, and approval state; layer a bounded swarm pool over dag_runner with all/any/quorum/best joins; extend task_queue with dead-letter, stalled recovery, quality gate rows, and budget deferral; add trace ids to every LLM/tool/approval/retry/gate event.',
  'multi-agent,backend-plan,lineage,tool-bundles,swarm-runtime,task-queue,tracing',
  10, 'procedure', 1746489600000, 'long', 1.0, 'self-improve', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MULTI-AGENT UI UX ADOPTION PLAN (2026-05-05): build a compact operations workbench, not a landing page. First screen should answer what is running, which agent owns it, what tools it can touch, what is blocked, and what changed. Use active-run rail, agent lanes, backlog/queued/running/blocked/review/failed/done task cards, parent-child run graph, transcript bubbles for delegation/tool/approval/validation/verdict events, repair controls, and LAN peer brain status panels with source host and token state.',
  'multi-agent,ui-ux,operations-workbench,run-graph,agent-lanes,lan-sharing',
  9, 'procedure', 1746489600000, 'long', 1.0, 'ui-ux', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'WINDOWS COMMON CONTROLS DLL LESSON (2026-05-05): Rust lib test harnesses on Windows can fail before Rust code runs with STATUS_ENTRYPOINT_NOT_FOUND when native dependencies import comctl32.dll!TaskDialogIndirect without a Common Controls v6 activation manifest. Root fix: embed one canonical Windows app manifest that declares Microsoft.Windows.Common-Controls version 6.0.0.0, compatibility, UTF-8 code page, DPI, longPathAware, and asInvoker privileges; disable Tauri duplicate default manifest via WindowsAttributes::new_without_app_manifest(). Validate with cargo test, not only cargo check.',
  'windows,manifest,comctl32,TaskDialogIndirect,dll-loader,tauri,testing',
  10, 'procedure', 1746489600000, 'long', 1.0, 'development', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'ANN AND WASM FEATURE-GATING LESSON (2026-05-05): native-heavy dependencies should be opt-in when not required for normal MCP/test flows. TerranSoul default builds now use a pure-Rust linear cosine AnnIndex fallback and a WASM runner stub that returns a clear disabled message. The native-ann feature enables persisted usearch HNSW vectors.usearch for large stores; the wasm-sandbox feature enables Wasmtime plugin execution. README and docs/brain-advanced-design.md must describe default vs feature-enabled behavior together.',
  'memory,ann,usearch,wasmtime,feature-gating,rag,brain-docs,self-improve',
  10, 'procedure', 1746489600000, 'long', 1.0, 'brain', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MULTI-AGENT ORCHESTRATION VERDICT:%'
  AND d.content LIKE 'TERRANSOUL MULTI-AGENT BASELINE:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'informs', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MULTI-AGENT BACKEND ADOPTION PLAN:%'
  AND d.content LIKE 'MULTI-AGENT ORCHESTRATION VERDICT:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'informs', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MULTI-AGENT UI UX ADOPTION PLAN:%'
  AND d.content LIKE 'MULTI-AGENT ORCHESTRATION VERDICT:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MULTI-AGENT ORCHESTRATION RESEARCH SOURCE CHECK:%'
  AND (
       d.content LIKE 'DEEPWIKI REVERSE-ENGINEERING RULE:%'
    OR d.content LIKE 'MCP SELF-LEARNING RULE:%'
    OR d.content LIKE 'DEEP ANALYSIS RULE:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 0.9, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'ANN AND WASM FEATURE-GATING LESSON:%'
  AND (
       d.content LIKE 'RAG pipeline:%'
    OR d.content LIKE 'Vector support:%'
    OR d.content LIKE 'Memory store uses SQLite%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 0.9, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'WINDOWS COMMON CONTROLS DLL LESSON:%'
  AND (
       d.content LIKE 'CI gate command:%'
    OR d.content LIKE 'MCP/app dependency bootstrap rule:%'
    OR d.content LIKE 'Self-improve with MCP mode:%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'LESSON: GitNexus is PolyForm Noncommercial%'
  AND (
       d.content LIKE 'Code intelligence pipeline:%'
    OR d.content LIKE 'MCP SELF-LEARNING RULE:%'
    OR d.content LIKE 'DEEPWIKI REVERSE-ENGINEERING RULE:%'
    OR d.content LIKE 'LESSON: Per the Brain Documentation Sync rule%'
  );

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CODE INTELLIGENCE ROADMAP:%'
  AND d.content LIKE 'Code intelligence pipeline:%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CODE WORKBENCH UX LESSON:%'
  AND (
       d.content LIKE 'Frontend Pinia stores in src/stores/:%'
    OR d.content LIKE 'Design docs (docs/):%'
  );

-- ═══════════════════════════════════════════════════════════════════════════
-- UI/UX Design System & Reference Rules (added 2026-05-05)
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, token_count, category)
VALUES
('UI/UX DESIGN RULE: Before implementing any new UI screen, component, or layout, agents MUST consult styles.refero.design for real-product design references. Search for the screen type being built, extract spacing/hierarchy/layout patterns, then map findings to TerranSoul --ts-* tokens. Document which reference informed the design.', 'ui-ux,design-rule,refero,design-reference,non-negotiable', 5, 'procedure', 1746489600000, 'long', 1.0, 65, 'ui-ux'),

('DESIGN SYSTEM: TerranSoul canonical style spec lives at docs/DESIGN.md (Refero DESIGN.md format). Color palette: Cosmic Violet #7c6fff (brand), Sky Blue #60a5fa (secondary), Void Navy #0f172a (bg-base), Deep Slate #1e293b (bg-surface). Typography: Inter (UI) + JetBrains Mono (code), 0.9rem base. Spacing: 4px base unit, 8px element gap, 12px card padding, 16px section gap. Radius: sm=6px, md=10px, lg=14px.', 'design-system,tokens,colors,typography,spacing,terransoul', 5, 'fact', 1746489600000, 'long', 1.0, 90, 'ui-ux'),

('DESIGN TOOLS AUDIT (05/2026): Primary: styles.refero.design (130k+ screens, MCP integration, DESIGN.md export). Supporting: Open Props v1.7 (500+ CSS tokens), Tailwind CSS v4 (utility-first, already integrated), Radix Colors (accessible P3 scales), W3C Design Tokens DTCG format, Style Dictionary (token transforms), Figma Variables+Dev Mode, Storybook 8.x, shadcn/ui (patterns), UnoCSS, Every Layout (intrinsic CSS), Inclusive Components (a11y).', 'design-tools,audit,refero,open-props,tailwind,tooling', 4, 'fact', 1746489600000, 'long', 1.0, 80, 'ui-ux'),

('DESIGN TOKEN HIERARCHY: Design Reference (styles.refero.design) → docs/DESIGN.md (canonical spec) → src/style.css :root { --ts-* } (runtime tokens) → Vue Components (consume via var(--ts-*)). When adding new tokens update both src/style.css and docs/DESIGN.md in the same PR.', 'design-system,tokens,hierarchy,workflow', 4, 'procedure', 1746489600000, 'long', 1.0, 55, 'ui-ux');

-- Edge: design rule supports coding standards
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'UI/UX DESIGN RULE:%'
  AND d.content LIKE 'Coding standards: snake_case for Rust%';

-- Edge: design system supports design rule
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'DESIGN SYSTEM: TerranSoul canonical style%'
  AND d.content LIKE 'UI/UX DESIGN RULE:%';

-- Edge: design tools audit supports design rule
INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'DESIGN TOOLS AUDIT (05/2026):%'
  AND d.content LIKE 'UI/UX DESIGN RULE:%';

-- ── Chunk 37.13 (2026-05-06) ─────────────────────────────────────────────
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MULTI-REPO CODE INTEL LESSON (chunk 37.13, 2026-05-06): TerranSoul groups multiple indexed repos via three new tables on code_index.sqlite — code_repo_groups (label UNIQUE), code_repo_group_members (group_id, repo_id, role), and code_contracts (signature_hash over name|kind|parent for change detection). Contracts are top-level function/struct/enum/trait/class/interface/type_alias/constant symbols only (parent IS NULL). Cross-repo queries restrict symbol search to a group and flag whether each match is part of that repo''s contract surface. Use repo_groups.rs API; do not query the tables directly from MCP/Tauri layers. Extracting contracts is transactional (atomic per-repo replace) so a partial failure cannot leave a half-replaced contract set on disk.',
  'code-intelligence,multi-repo,groups,contracts,signature-hash,native,chunk-37.13',
  9, 'procedure', 1746489600000, 'long', 1.0, 'coding', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP TOOL COUNT (post chunk 37.13, 2026-05-06): TerranSoul MCP exposes 9 brain tools always-on, plus 12 code-intelligence tools when caps.code_read is granted, totalling 21 tools. The 12 code tools are: code_query, code_context, code_impact, code_rename, code_generate_skills, code_list_groups, code_create_group, code_add_repo_to_group, code_group_status, code_extract_contracts, code_list_group_contracts, code_cross_repo_query. Update tools_list_returns_21_tools and definitions_has_21_tools_with_code_read assertions if the count changes.',
  'mcp,tool-count,code-intelligence,brain-tools,assertions,testing',
  8, 'procedure', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);
-- ─ Chunk 33B.4 (2026-05-06) ─
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MEMORY AUDIT TAB LESSON (chunk 33B.4, 2026-05-06): MemoryView.vue has an Audit tab backed by memory::audit::get_memory_provenance and the get_memory_provenance Tauri command. The backend returns one joined payload: current MemoryEntry, memory_versions history, incident memory_edges, direction labels, and compact neighboring-memory summaries. The Pinia memory store exposes getMemoryProvenance(memoryId), so audit/provenance UI should prefer that authoritative joined query over client-side stitching of get_memory_history plus get_edges_for_memory.',
  'memory,audit,provenance,versioning,edges,frontend,chunk-33B.4',
  8, 'procedure', 1746489600000, 'long', 1.0, 'memory', 'procedural'
);

-- ─ Chunk 33B.5 (2026-05-06) ─
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'BRAIN 3D KG VISUALISER LESSON (chunk 33B.5, 2026-05-06): BrainGraphViewport.vue renders memories as an interactive 3D force-directed graph using Three.js InstancedMesh + d3-force-3d (npm package, no @types — use src/types/d3-force-3d.d.ts ambient declarations). Node colour comes from the shared TS/Rust cognitive-kind classifier, including episodic/semantic/procedural/judgment parity; do not duplicate classifier logic in the component. Edge colour hashes rel_type into an 8-colour design-token palette and the viewport displays both cognitive-kind and relation legends. The graph rebuilds when memory or edge semantic inputs change, uses orbit controls and InstancedMesh raycasting, and is toggled by the 3-D checkbox in MemoryView.vue Graph tab toolbar. Pre-warm the simulation 80 ticks for stable initial layout.',
  'brain,3d,knowledge-graph,three-js,d3-force-3d,visualiser,frontend,chunk-33B.5',
  8, 'procedure', 1746489600000, 'long', 1.0, 'brain', 'procedural'
);
-- Chunk 33B.6 (2026-05-06)
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'AGENT CAPABILITY ROUTING LESSON (chunk 33B.6, 2026-05-06): AgentProfile now has a serde-default capabilities: Vec<String> field. AgentProvider trait gained fn capabilities() with default empty. AgentOrchestrator exposes agents_with_capabilities(required) (AND-match on all tags) and dispatch_by_capability(required, message) which falls back to default agent. CodingCapability::required_tags() bridges intent-detection output to capability tags. CreateAgentRequest accepts capabilities from the frontend. Tag vocabulary is open/extensible convention strings.',
  'agents,capability,routing,orchestrator,coding-router,tauri,chunk-33B.6',
  8, 'procedure', 1746489600000, 'long', 1.0, 'coding', 'procedural'
);

-- ====================================================================
-- Phase 38 (Million-memory MCP) — chunks 38.3, 38.4, 38.5 + UI/UX redesign
-- Synced: 2026-05-07
-- ====================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'CHUNK 38.3 (2026-05-07) — Native-ANN default for desktop builds: native-ann is now part of the default desktop feature set in src-tauri/Cargo.toml ([features] default = ["desktop"], desktop = ["native-ann"], headless-mcp = []). Production desktop builds get persisted usearch HNSW vectors.usearch by default; the headless-mcp build remains pure-Rust linear cosine via AnnIndex fallback. Bench feature bench-million = ["native-ann"] gates the standalone million_memory benchmark binary.',
  'chunk-38.3,native-ann,usearch,feature-flags,desktop,terransoul,architecture',
  9, 'fact', 1746576000000, 'long', 1.0, 'memory', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'CHUNK 38.4 (2026-05-07) — Capacity-based self-eviction: src-tauri/src/memory/eviction.rs implements enforce_capacity(conn, cap, target_ratio, data_dir). Single-pass SQL drops lowest-priority entries when memory count exceeds cap, targeting DEFAULT_TARGET_RATIO=0.95. Eviction order: protected=0 first, then by ascending (importance + decay_score) within each tier, oldest first as tie-break. Writes JSONL audit log under <data_dir>/eviction_log.jsonl. Configurable via SettingsConfig.max_long_term_entries (u64). 5 unit tests cover cap=0 no-op, under-cap no-op, eviction order, protected entries skipped, and audit-log shape.',
  'chunk-38.4,eviction,capacity,self-eviction,audit-log,settings,terransoul',
  9, 'fact', 1746576000000, 'long', 1.0, 'memory', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'CHUNK 38.5 (2026-05-07) — Million-memory benchmark: src-tauri/benches/million_memory.rs is a Criterion harness with deterministic xoshiro 768-dim synthetic vectors. Default smoke: cargo bench --bench million_memory builds 10k vectors, runs 1,000 HNSW top-10 queries, writes src-tauri/target/bench-results/million_memory.json, and times enforce_capacity. Measured Windows/i9-12900K smoke: p50 0.57ms, p95 0.74ms, p99 0.86ms, max 1.03ms; capacity pruned 10,500 -> 9,500 in 0.26s while preserving protected/high-importance rows. Full local/nightly tier: --features bench-million gates 1M HNSW assertions p50<=30ms, p95<=60ms, p99<=100ms, linear backend skipped at 1M, capacity 1,050,000 -> 950,000 <=30s.',
  'chunk-38.5,benchmark,million-memory,hnsw,criterion,enforce-capacity,performance,terransoul',
  10, 'fact', 1746576000000, 'long', 1.0, 'memory', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'SCHEMA UPDATE (2026-05-07): SQLite memory store CANONICAL_SCHEMA_VERSION is now 15 (was 14). v15 adds the protected column on memories (prevents eviction) and the pending_embeddings table for the self-healing retry queue. Migration is forward-only and idempotent.',
  'schema,sqlite,migration,version-15,protected,pending-embeddings',
  10, 'fact', 1746576000000, 'long', 1.0, 'memory', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'UI/UX PREMIUM GLASSMORPHISM REDESIGN (2026-05-06): src/style.css gained --ts-glass-bg, --ts-glass-border, --ts-glass-blur, --ts-glass-highlight, --ts-shadow-glow, --ts-shadow-inset, --ts-transition-spring tokens (also wired into corporate and corporate-dark themes). Sidebar (App.css), chat input (ChatInput.vue), chat bubbles + welcome state (ChatMessageList.vue), ChatView panels (bottom-panel, input-footer, chatbox-header/footer, brain-status-pill, subtitle-overlay, brain-card) all now use frosted glass surfaces with backdrop-filter blur+saturate, gradient user bubbles, spring-animated hover states, and progressive disclosure (timestamps/ratings reveal on hover). 1713 vitest tests pass; vite build clean.',
  'ui-ux,redesign,glassmorphism,design-tokens,style-css,chat,sidebar,terransoul',
  8, 'fact', 1746489600000, 'long', 1.0, 'frontend', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'RULE (user expectation, 2026-05-06): when a TerranSoul chunk is marked completed, MCP MUST already know its content. Sync durable lessons to mcp-data/shared/memory-seed.sql in the same PR and verify retrievability via brain_search. Retrieval latency for any RAG/knowledge query must remain sub-second (current measured: 5-48ms over MCP HTTP).',
  'rule,mcp,sync,completion-log,retrieval-latency,non-negotiable,terransoul',
  10, 'rule', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746576000000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CHUNK 38.3%' AND d.content LIKE 'CHUNK 38.4%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'related_to', 1.0, 'seed', 1746576000000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CHUNK 38.4%' AND d.content LIKE 'SCHEMA UPDATE (2026-05-07)%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746576000000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CHUNK 38.5%' AND d.content LIKE 'CHUNK 38.3%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'supports', 1.0, 'seed', 1746576000000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'CHUNK 38.5%' AND d.content LIKE 'CHUNK 38.4%';

-- ====================================================================
-- Brain Wiki / graph operations (Graphify + LLM Wiki application)
-- Synced: 2026-05-06
-- ====================================================================

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'BRAIN WIKI APPLICATION LESSON (2026-05-06): TerranSoul studied safishamsi/graphify (MIT; public README, ARCHITECTURE.md, docs/how-it-works.md, tests) plus Karpathy LLM Wiki and append-and-review notes. Apply the pattern as a chat/UI-first Knowledge Wiki: raw sources remain immutable memories with source_hash; graph structure stays in memory_edges with confidence/provenance; synthesized wiki pages are protected summary memories; audit/spotlight/serendipity/revisit expose lint, god-node, cross-community, and append-review behavior without a CLI.',
  'brain-wiki,graphify,karpathy,llm-wiki,knowledge-graph,rag,chat-ui',
  10, 'procedure', 1746489600000, 'long', 1.0, 'brain', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'BRAIN WIKI APPLICATION LESSON (2026-05-06):%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'BRAIN WIKI BACKEND SURFACE (2026-05-06): src-tauri/src/memory/wiki.rs is the shared Rust operation layer for graph/wiki brain actions. It adds fingerprint/ensure_source_dedup, confidence_label over existing EdgeSource+confidence, audit_report, god_nodes, surprising_connections, append_and_review_queue, and gravity_score. src-tauri/src/commands/wiki.rs exposes Tauri commands brain_wiki_audit, brain_wiki_digest_text, brain_wiki_spotlight, brain_wiki_serendipity, and brain_wiki_revisit. Unit tests mirror graphify expectations for cache/dedup, incremental reingest, confidence rubric, graph validation, god nodes, surprises, and append-review order.',
  'brain-wiki,rust,tauri,source-dedup,confidence-rubric,god-nodes,serendipity,revisit,tests',
  9, 'procedure', 1746489600000, 'long', 1.0, 'brain', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'BRAIN WIKI BACKEND SURFACE (2026-05-06):%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'BRAIN WIKI CHAT/UI SURFACE (2026-05-06): src/utils/slash-commands.ts now has a separate parseBrainWikiSlashCommand for /digest, /ponder, /spotlight, /serendipity, /revisit plus planned /weave, /trace, /why. ChatView dispatches supported commands before plugin slash dispatch; /digest routes URLs/files/crawl: sources to ingest_document and pasted text to brain_wiki_digest_text. src/components/WikiPanel.vue is wired into BrainView and shows Audit, Spotlight, Serendipity, and Revisit tabs backed by the same Tauri commands. Full Vitest after this work: 133 files, 1738 tests passing; vue-tsc clean.',
  'brain-wiki,chat,slash-commands,wikipanel,brainview,frontend,vitest,vue-tsc',
  9, 'procedure', 1746489600000, 'long', 1.0, 'frontend', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'BRAIN WIKI CHAT/UI SURFACE (2026-05-06):%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'FUTURE MCP GRAPHIFY-LIKE SURFACE RULE (2026-05-06): TerranSoul MCP should eventually expose graph/wiki brain operations like graphify-style query/analyse/path capabilities, but MCP must call the same TerranSoul-native Rust functions used by chat and UI. Humans interact through TerranSoul chatbox and BrainView configuration/panels, not a graphify-like CLI. Future tool names should stay neutral (brain_wiki_audit, brain_wiki_spotlight, brain_wiki_serendipity, brain_wiki_revisit, brain_wiki_digest_text, later trace/why/weave) and remain capability-gated by existing MCP caps.',
  'mcp,brain-wiki,graphify-like,chat-first,ui-first,capability-gating,rule',
  10, 'procedure', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'FUTURE MCP GRAPHIFY-LIKE SURFACE RULE (2026-05-06):%'
);

INSERT INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
SELECT
  'MCP BRAIN WIKI SURFACE IMPLEMENTED (2026-05-06): src-tauri/src/ai_integrations/mcp/tools.rs now advertises and dispatches brain_wiki_audit, brain_wiki_spotlight, brain_wiki_serendipity, brain_wiki_revisit, and brain_wiki_digest_text through the same memory::wiki Rust functions used by ChatView and WikiPanel. Audit/spotlight/serendipity/revisit require brain_read and are allowed in LAN public read-only mode; digest_text requires brain_write because it persists a source-hash-deduplicated memory row. Integration tests cover tool listing, audit, digest dedup, and public read-only routing.',
  'mcp,brain-wiki,implemented,tools,capability-gating,public-read-only,tests',
  10, 'procedure', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
WHERE NOT EXISTS (
  SELECT 1 FROM memories WHERE content LIKE 'MCP BRAIN WIKI SURFACE IMPLEMENTED (2026-05-06):%'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'implements', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'BRAIN WIKI BACKEND SURFACE (2026-05-06):%'
  AND d.content LIKE 'BRAIN WIKI APPLICATION LESSON (2026-05-06):%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'exposes', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'BRAIN WIKI CHAT/UI SURFACE (2026-05-06):%'
  AND d.content LIKE 'BRAIN WIKI BACKEND SURFACE (2026-05-06):%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'extends', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'FUTURE MCP GRAPHIFY-LIKE SURFACE RULE (2026-05-06):%'
  AND d.content LIKE 'BRAIN WIKI BACKEND SURFACE (2026-05-06):%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'implements', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP BRAIN WIKI SURFACE IMPLEMENTED (2026-05-06):%'
  AND d.content LIKE 'FUTURE MCP GRAPHIFY-LIKE SURFACE RULE (2026-05-06):%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'reuses', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP BRAIN WIKI SURFACE IMPLEMENTED (2026-05-06):%'
  AND d.content LIKE 'BRAIN WIKI BACKEND SURFACE (2026-05-06):%';


-- ====================================================================
-- Benchmarking guide (docs/benchmarking.md)
-- Synced: 2026-05-07
-- ====================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'BENCHMARK DOC (2026-05-07): docs/benchmarking.md is the operator guide for TerranSoul Criterion benches. It documents the Million-memory benchmark (src-tauri/benches/million_memory.rs): smoke command "cargo bench --bench million_memory --target-dir ../target-copilot-bench" (10k vectors, no feature), full command adds "--features bench-million" (1M vectors), CRUD-only command sets TS_BENCH_CRUD_ONLY=1 with TS_BENCH_TIMEOUT_SECS and optional TS_BENCH_SCALES=1000000, env knobs TS_BENCH_SCALES, TS_BENCH_CRUD_ONLY, TS_BENCH_TIMEOUT_SECS, TS_BENCH_FORCE_LARGE, TS_BENCH_OUTPUT_DIR, JSON report at src-tauri/target/bench-results/million_memory.json with machine/hnsw/linear_backend/capacity/crud sections, hard thresholds HNSW p50<=30ms, p95<=60ms, p99<=100ms, capacity 1.05x->0.95x <=30s, and CRUD 1M write<=60s/read<=5s. It records the 2026-05-07 CRUD reference: write 7.27s, read 2.13s, update 2.58s, mixed 10k ops in 160.03s, delete skipped at 1M benchmark scale. The doc also gives the canonical recipe for adding a new bench: add [[bench]] with harness=false and required-features in src-tauri/Cargo.toml, gate heavy tier behind a bench-* feature flag, use deterministic xoshiro seeds, write JSON to target/bench-results/<name>.json, honour env knobs, assert thresholds in main, never kill the running MCP terminal, always use --target-dir ../target-copilot-bench on Windows, and sync results to README.md + docs/brain-advanced-design.md + this seed file.',
  'docs,benchmarking,benchmark,criterion,million-memory,how-to,terransoul',
  9, 'fact', 1746489600000, 'long', 1.0, 'docs', 'semantic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'documents', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'BENCHMARK DOC (2026-05-07):%'
  AND d.content LIKE 'CHUNK 38.5%';

-- ====================================================================
-- Folder-to-knowledge-graph tutorial + token-usage benchmark
-- Synced: 2026-05-06
-- ====================================================================

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'TUTORIAL FOLDER->KG (2026-05-06): tutorials/folder-to-knowledge-graph-tutorial.md is the canonical English tutorial for turning any folder into a TerranSoul knowledge graph + Obsidian vault + auto-wiki + NL Q&A. Pipeline: code path uses code_index_repo -> code_resolve_edges -> code_compute_processes -> code_generate_wiki Tauri commands; doc/PDF path uses brain_ingest_url MCP / ingest_document Tauri (read_local_file in src-tauri/src/commands/ingest.rs accepts md/markdown/txt/csv/json/xml/html/htm/log/rst/adoc + pdf via extract_pdf_text). Languages actually parsed: 7 in default desktop build (Rust + TS/TSX always on; Python/Go/Java/C/C++ behind cargo features parser-python, parser-go, parser-java, parser-c — see src-tauri/src/coding/parser_registry.rs). Image OCR is NOT supported. There is NO single chat command yet to ingest a mixed code+docs folder in one shot — call code_index_repo for code and loop brain_ingest_url for docs. Do NOT claim 13 languages or 71.5x token reduction in TerranSoul docs; those are third-party claims. Wiki: code_generate_wiki writes <data_dir>/wiki/index.md + per-cluster pages with mermaid call graphs; brain wiki tools brain_wiki_audit/spotlight/serendipity/revisit/digest_text serve memory-side curation. Obsidian: obsidian_export.rs (one-way, <vault>/TerranSoul/<id>-<slug>.md with YAML frontmatter) + obsidian_sync.rs (bidirectional LWW based on file mtime vs last_exported). Backlinks come from memory_edges rendered with id-prefixed filenames.',
  'tutorial,folder-ingest,knowledge-graph,obsidian,wiki,languages,pdf,coverage,terransoul',
  9, 'fact', 1746489600000, 'long', 1.0, 'docs', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP TOKEN BENCHMARK (2026-05-06): docs/mcp-token-usage-benchmark.md is the honest per-session measurement of TerranSoul MCP context savings. Methodology: for each brain_search / brain_get_entry call, record bytes returned vs bytes of the closest source-of-truth file the agent would otherwise grep. Aggregate from the tracked subset that wrote the folder->KG tutorial: ~12.5KB returned over 5 brain_search + 1 brain_get_entry calls vs ~360KB of underlying seed.sql + brain-advanced-design.md = ~29x aggregate reduction for in-scope queries. Per-query range: 30x (broad search) to ~100x (targeted lookup / get_entry). Caveats: (a) TerranSoul code is NOT auto-ingested into the brain, so code-discovery questions still fell back to grep_search / search_subagent and consumed ~80KB in one sub-agent call, dropping session-wide saving to ~3-5x; (b) bytes/4 token estimate ±25%; (c) single-machine single-session result. Honest range in tracked rows: 30x (broad MCP query) to ~100x (best). Reject any claim of a fixed externally-published number like 71.5x in TerranSoul docs — measure per session.',
  'benchmark,mcp,token-usage,context,savings,methodology,terransoul',
  9, 'fact', 1746489600000, 'long', 1.0, 'docs', 'semantic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'documents', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'TUTORIAL FOLDER->KG (2026-05-06):%'
  AND d.content LIKE 'BENCHMARK DOC (2026-05-07):%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT s.id, d.id, 'measures', 1.0, 'seed', 1746489600000, 'seed'
FROM memories s, memories d
WHERE s.content LIKE 'MCP TOKEN BENCHMARK (2026-05-06):%'
  AND d.content LIKE 'TUTORIAL FOLDER->KG (2026-05-06):%';

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP ERROR FIX RULE (2026-05-06): if any TerranSoul MCP call returns an error, agents must not silently fall back to grep or continue with stale context. Classify the error as (1) bad tool arguments/contract mismatch, (2) unhealthy or stale MCP server/binary, or (3) missing/stale durable knowledge. Then fix the MCP tool schema/adapter/gateway and add a regression test, restart/rebuild MCP via node scripts/copilot-start-mcp.mjs when health/staleness is the cause, or update mcp-data/shared/memory-seed.sql plus a numbered migration for knowledge drift. Always report the original error, root cause, fix, and any remaining blocker. The brain_summarize query error was fixed by adding query-backed summarization to the MCP tool contract.',
  'mcp,error-handling,server-health,tool-contract,regression-test,seed-migration,non-negotiable',
  10, 'procedure', 1746489600000, 'long', 1.0, 'mcp', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP HEALTH RESPONSE EXPLANATION (2026-05-06): brain_health plus GET /health and GET /status keep backward-compatible rag_quality_pct and memory_total fields, but now also return rag_quality, memory, and descriptions objects. rag_quality_pct means embedded_long_memory_count / long_memory_count * 100, not an overall intelligence score. A 12% value means only about 12% of long-term memories currently have vector embeddings; keyword/RRF and graph lookup still work, but semantic vector recall is partial until pending embedding backfill completes. Use the nested raw counts and description strings when displaying health JSON to humans.',
  'mcp,health,rag-quality,embedding-coverage,json,docs,non-negotiable',
  10, 'fact', 1746489600000, 'long', 1.0, 'mcp', 'semantic'
);
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MILLION-KNOWLEDGE CRUD AUDIT STATE (updated 2026-05-07): TerranSoul memory store has SQLite WAL + tuned Phase 41.1 PRAGMAs, V15 schema with eviction-friendly indexes, usearch HNSW with brute-force fallback, capacity eviction, embedding retry queue, memory_versions, contradiction detection, semantic chunker, late chunking, contextual retrieval, Criterion 10k/1M bench, ingest semaphore, Phase 41.4 transactional add_many, Phase 41.2 per-op metrics, and Phase 41.3 bounded CRUD benchmark sections. Resolved from the original audit: PRAGMA defaults, missing add_many, missing per-op metrics, and unbounded benchmark stalls. Remaining 1M+ optimization work: route high-level ingest fully through bulk APIs, remove get_all/get_with_embeddings from hot search paths, re-embed and tombstone ANN entries on content update, reduce HNSW save-every-50-op churn, support per-model/dim ANN registries, add cursor reads/quantization/tombstone compaction, and verify FTS/KG indexes at 1M rows.',
  'audit,memory-store,million-scale,bottlenecks,phase-41,non-negotiable',
  10, 'fact', 1746489600000, 'long', 1.0, 'memory', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PHASE 41 CURRENT PLAN (updated 2026-05-07) — Million-Knowledge CRUD: completed chunks are 41.1 SQLite write-path tuning + FTS5 verification, 41.4 transactional add_many/update_many/delete_many, 41.2 per-op latency histograms surfaced through get_memory_metrics and MCP health, and 41.3 extended million_memory CRUD benchmarking with TS_BENCH_TIMEOUT_SECS guards, bounded mixed workload, and explicit 1M delete skip. Headline 1M write/read SLO is green: latest CRUD-only run wrote 1M in 7.27s and read 1M in 2.13s. Remaining chunks are production-hardening work, not blockers for the headline target: 41.5 cursor reads to remove get_all/get_with_embeddings from hybrid_search/hybrid_search_rrf/relevant_for/find_duplicate; 41.6 re-embed on content update + ANN tombstone + enqueue; 41.7 embedding worker concurrency + rate limiting + pause-on-429 + graceful shutdown; 41.8 V16 multi-model embeddings with memory_embeddings and AnnRegistry keyed by (model_id, dim); 41.9 usearch i8 quantization with recall budget; 41.10 memory-mapped HNSW + debounced async flush replacing SAVE_INTERVAL=50; 41.11 ANN compaction/tombstone GC; 41.12 partial indexes + PRAGMA optimize/ANALYZE; 41.13 bounded KG traversal + LRU cache; 41.14 optional time-bucketed shards; 41.15 online VACUUM INTO + ANN save + manifest snapshot/restore. Each remaining chunk must keep the Full CI Gate green and extend benches/million_memory.rs where relevant.',
  'plan,phase-41,million-scale,chunks,ann,sqlite,embedding,sharding,non-negotiable',
  10, 'fact', 1746489600000, 'long', 1.0, 'memory', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT plan.id, audit.id, 'derived_from', 1.0, 'seed', 1746489600000, 'seed'
FROM memories plan
CROSS JOIN memories audit
WHERE plan.content LIKE 'PHASE 41 CURRENT PLAN (updated 2026-05-07)%'
  AND audit.content LIKE 'MILLION-KNOWLEDGE CRUD AUDIT STATE (updated 2026-05-07)%';
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'DB STRATEGY VERDICT (2026-05-06): SQLite is NOT a bottleneck for TerranSoul as the local engine, even at 1M+ memories and on offline mobile. After Phase 41 tuning, the companion is CPU/embedding-bound long before SQLite-bound. SQLite IS the wrong shape for "hive" multi-user federation and distributed jobs. Final posture is two-layer storage: (1) Local layer on every device (desktop + iOS + Android) keeps tuned SQLite + WAL as authoritative source of truth; pure-Rust ANN fallback ships on mobile because usearch C++ build is fragile there; (2) Sync layer between a single user own devices promotes memories + KG edges to CRDTs (LWW for memory rows, 2P-Set/OR-Set for edges) replicated as op-logs over the existing QUIC/WS LinkManager — no server required; (3) Hive layer is opt-in: a reference Tonic gRPC relay backed by Postgres + pgvector accepts Ed25519-signed knowledge bundles, runs a job queue, and federates only when configured. The local app never depends on the hive. Reject standalone vector services (Qdrant/Milvus/Pinecone) for the local app per existing decision (brain-advanced-design.md row 18). Keep usearch HNSW locally; pgvector HNSW on the hive layer. Existing alt backends (postgres.rs/mssql.rs/cassandra.rs) currently lack RRF / FTS5 / KG / contextual-retrieval parity; bringing Postgres to parity is a hive prerequisite. Memory store rows already carry updated_at + origin_device columns but no merge function — wire that.',
  'verdict,database,sqlite,postgres,hive,mobile,crdt,federation,phase-42,non-negotiable',
  10, 'fact', 1746489600000, 'long', 1.0, 'memory', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PHASE 42 PLAN (2026-05-06) — DB strategy for offline mobile + future hive: 12 ordered chunks across 4 sub-phases, lands AFTER Phase 41. A. Mobile-safe local engine: 42.1 mobile feature gates + pure-Rust ANN fallback for iOS/Android (usearch desktop-only); 42.2 mobile SQLite/WAL hardening + integration tests on aarch64-apple-ios and aarch64-linux-android. B. Memory CRDT (own-device sync): 42.3 memory rows as LWW CRDT with vector-clock conflict markers (HLC + origin_device, loser archived to memory_versions, conflicts surfaced in BrainView); 42.4 KG edges as 2P-Set/OR-Set CRDT with valid_to as tombstone and scheduled compaction; 42.5 op-log replication over LinkManager (QUIC primary + WS fallback) using existing mdns-sd discovery, pairwise sync only. C. Distributed backend parity (hive prerequisite): 42.6 Postgres backend gains tsvector GIN FTS, RRF in single SQL CTE, recursive-CTE KG traversal, contextual retrieval; make hybrid_search_rrf non-default; 42.7 pgvector HNSW vector(768) parity + benchmark mirroring million_memory.rs in Docker CI; 42.8 backend test matrix runs SQLite + Postgres on every memory PR, MSSQL/Cassandra weekly. D. Hive layer (opt-in federation + jobs): 42.9 hive protocol spec in docs/hive-protocol.md with BUNDLE/OP/JOB messages and Ed25519-signed bundles using identity/ device keys; 42.10 reference relay server in crates/hive-relay/ (new workspace member) — Tonic gRPC + Postgres + pgvector, MIT, self-hostable, docker-compose ready; 42.11 job queue + capability gates reusing src-tauri/src/orchestrator/, workers pull jobs and return BUNDLEs; 42.12 privacy/consent/per-memory ACL with share_scope enum (private/paired/hive), default per cognitive_kind, redaction tests proving private rows never appear in outbound bundles. Each chunk keeps Full CI Gate green; brain-advanced-design.md + README updated for any brain-surface change per existing rule.',
  'plan,phase-42,database,mobile,crdt,hive,federation,postgres,pgvector,job-distribution,non-negotiable',
  10, 'fact', 1746489600000, 'long', 1.0, 'memory', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT plan.id, verdict.id, 'derived_from', 1.0, 'seed', 1746489600000, 'seed'
FROM memories plan
CROSS JOIN memories verdict
WHERE plan.content LIKE 'PHASE 42 PLAN (2026-05-06)%'
  AND verdict.content LIKE 'DB STRATEGY VERDICT (2026-05-06)%';
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PHASE 41 RESULT (2026-05-07): chunks 41.1 (SQLite PRAGMA tuning), 41.4 (transactional add_many), 41.2 (per-op metrics), and 41.3 (extended CRUD bench + timeout guards) shipped. Measured on commodity dev hardware via cargo bench --bench million_memory: 10k smoke write 0.04s @ 244,657 rows/s, read 0.01s @ 939,956 rows/s. 1M CRUD-only runs (TS_BENCH_SCALES=1000000 TS_BENCH_CRUD_ONLY=1) keep the headline target green; latest bounded run wrote 1M in 7.27s @ 137,521 rows/s and read 1M in 2.13s @ 469,036 rows/s, with update 2.58s @ 387,042 rows/s and mixed 10k ops in 160.03s. Million-scale benchmark delete is skipped because secondary-index maintenance dominates wall-clock and is not part of the write/read SLO. Remaining Phase 41 chunks (41.5 cursor reads, 41.6 re-embed on update, 41.7 worker concurrency, 41.8 multi-model embeddings, 41.9-41.11 ANN scaling, 41.12-41.13 indexes/KG, 41.14-41.15 sharding/snapshot) remain valuable for production hardening but are no longer blocking the headline throughput goal.',
  'phase-41,result,bench,million,sqlite,add_many,pragma,measured,non-negotiable',
  10, 'fact', 1746576000000, 'long', 1.0, 'memory', 'episodic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT result.id, plan.id, 'fulfills', 1.0, 'seed', 1746576000000, 'seed'
FROM memories result
CROSS JOIN memories plan
WHERE result.content LIKE 'PHASE 41 RESULT (2026-05-07)%'
  AND plan.content LIKE 'PHASE 41 CURRENT PLAN (updated 2026-05-07)%';

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PHASE 41.2/41.3 LESSON (2026-05-07): added per-op MemoryMetrics histograms/timers for memory CRUD and retrieval, exposed via get_memory_metrics and MCP health metrics. The million_memory CRUD bench now includes update/mixed/delete sections with hard timeout guards (TS_BENCH_TIMEOUT_SECS, default 300) so runs cannot appear infinite. At 1M scale, write/read remain the headline SLO and stay comfortably under 60s/5s; mixed workload can be much slower due to post-write index pressure, and benchmark delete is skipped at 1M because secondary-index maintenance dominates runtime and is not representative of the write/read objective.',
  'phase-41,chunk-41.2,chunk-41.3,metrics,benchmark,timeout,million-memory,sqlite',
  10, 'fact', 1746579600000, 'long', 1.0, 'memory', 'episodic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT newer.id, older.id, 'related_to', 1.0, 'auto', 1746579600000, 'auto'
FROM memories newer
CROSS JOIN memories older
WHERE newer.content LIKE 'PHASE 41.2/41.3 LESSON (2026-05-07)%'
  AND older.content LIKE 'PHASE 41 RESULT (2026-05-07)%';

-- ====================================================================
-- Phase 42 Section D completion (Hive federation) — 2026-05-07
-- ====================================================================
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PHASE 42 COMPLETE (2026-05-07): all 12 chunks shipped across 4 sub-phases. A. Mobile-safe local engine (42.1-42.2): feature-gated pure-Rust ANN fallback, mobile SQLite/WAL. B. Memory CRDT (42.3-42.5): LWW rows with HLC+origin_device, 2P-Set edges, op-log replication over LinkManager. C. Distributed backend parity (42.6-42.8): Postgres RRF+FTS+KG+contextual parity, pgvector HNSW, CI test matrix. D. Hive layer (42.9-42.12): protocol spec in docs/hive-protocol.md (BUNDLE/OP/JOB, Ed25519-signed envelopes, MessagePack+LZ4), reference relay server in crates/hive-relay/ (Tonic gRPC + Postgres, docker-compose), job queue with capability matching in src-tauri/src/hive/jobs.rs, privacy policy engine in src-tauri/src/hive/privacy.rs (share_scope private/paired/hive, filter_bundle, default_scope_for_kind), schema v19 adds share_scope column. 2383 Rust + 1738 TS tests green, clippy clean.',
  'phase-42,complete,hive,federation,crdt,mobile,postgres,privacy,share_scope,non-negotiable',
  10, 'fact', 1746662400000, 'long', 1.0, 'memory', 'episodic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT result.id, plan.id, 'fulfills', 1.0, 'seed', 1746662400000, 'seed'
FROM memories result
CROSS JOIN memories plan
WHERE result.content LIKE 'PHASE 42 COMPLETE (2026-05-07)%'
  AND plan.content LIKE 'PHASE 42 PLAN (2026-05-06)%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT result.id, verdict.id, 'fulfills', 1.0, 'seed', 1746662400000, 'seed'
FROM memories result
CROSS JOIN memories verdict
WHERE result.content LIKE 'PHASE 42 COMPLETE (2026-05-07)%'
  AND verdict.content LIKE 'DB STRATEGY VERDICT (2026-05-06)%';

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'HIVE MODULE MAP (2026-05-07): src-tauri/src/hive/ contains 4 sub-modules. protocol.rs: wire types (HiveEnvelope, MsgType, ShareScope, Bundle, MemoryDelta, EdgeDelta, Op, OpTarget, OpDelta, JobSpec, JobStatus, Capability). signing.rs: Ed25519 sign/verify for envelopes using identity/ device keys, sign_input format is version(1)||msg_type(1)||sender_id(var)||timestamp(8 LE)||hlc_counter(8 LE)||payload(var). jobs.rs: JobHandler trait, JobDispatcher (register_handler, can_execute_locally, execute_local, dispatch), capabilities_match AND-logic. privacy.rs: filter_bundle(bundle, target), default_scope_for_kind(), apply_default_scopes(), scope_satisfies() — private never leaves device, paired only to own devices, hive to relay.',
  'hive,module-map,protocol,signing,jobs,privacy,architecture,terransoul',
  9, 'fact', 1746662400000, 'long', 1.0, 'architecture', 'semantic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT newer.id, completion.id, 'part_of', 1.0, 'seed', 1746662400000, 'seed'
FROM memories newer
CROSS JOIN memories completion
WHERE newer.content LIKE 'HIVE MODULE MAP (2026-05-07)%'
  AND completion.content LIKE 'PHASE 42 COMPLETE (2026-05-07)%';

-- ====================================================================
-- README rewrite + MCP-as-differentiator positioning (2026-05-07)
-- ====================================================================
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'README REWRITE (2026-05-07): README.md was rewritten from 587 lines to ~153 lines following OpenClaw-style concise landing page pattern. Structure: title + badges + elevator pitch, Install table, Quick Start, Key Differentiator (MCP Brain), Highlights (bullet list), Tech Stack, Brain Modes, MCP Tools table, Tutorials table, Security, Development commands, Architecture diagram, Contact. The key differentiator section prominently positions TerranSoul MCP self-running brain as the feature that makes other AI coding agents smarter — giving them persistent memory, semantic search, code intelligence, and self-improvement across sessions.',
  'readme,documentation,mcp,differentiator,openclaw-style,concise,landing-page',
  9, 'fact', 1746662400000, 'long', 1.0, 'documentation', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'KEY DIFFERENTIATOR POSITIONING (2026-05-07): TerranSoul unique value vs other AI companions is the self-running MCP brain server (npm run mcp on 127.0.0.1:7423) that exposes persistent memory, semantic search (RRF+HyDE+reranker over 1M+ entries), knowledge graph, and code intelligence to ANY external AI coding agent (VS Code Copilot, Claude Code, Cursor, Codex). This gives agents: project memory across sessions, 10-50x context reduction via focused retrieval, self-improvement (agents write learnings back), and code intelligence (symbol index, impact analysis). Auto-starts when VS Code opens the workspace.',
  'mcp,differentiator,positioning,marketing,readme,coding-agents,self-improve',
  9, 'fact', 1746662400000, 'long', 1.0, 'documentation', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'TUTORIALS INDEX (2026-05-07): 18 tutorials in tutorials/ covering: quick-start, brain-rag-setup, brain-rag-local-lm, voice-setup, skill-tree-quests, advanced-memory-rag, knowledge-wiki, folder-to-knowledge-graph, teaching-animations-expressions-persona, charisma-teaching, device-sync-hive, lan-mcp-sharing, mcp-coding-agents, multi-agent-workflows, packages-plugins, browser-mobile, self-improve-to-pr, openclaw-plugin. Each follows rules/tutorial-template.md structure. README links all 18 in a Tutorials table.',
  'tutorials,documentation,index,readme,18-tutorials',
  8, 'fact', 1746662400000, 'long', 1.0, 'documentation', 'semantic'
);

-- ====================================================================
-- jcode reverse-engineering research + Phase 43 plan (2026-05-07)
-- ====================================================================
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'JCODE RESEARCH RECORD (2026-05-07): reverse-engineered 1jehuang/jcode (MIT, github.com/1jehuang/jcode) via DeepWiki and four upstream architecture docs (AMBIENT_MODE.md, MEMORY_ARCHITECTURE.md, SAFETY_SYSTEM.md, SWARM_ARCHITECTURE.md, SERVER_ARCHITECTURE.md). Full analysis in docs/jcode-research.md. License is MIT so we may study patterns freely; we still implement everything natively under neutral TerranSoul names. jcode is a Rust TUI agent harness with 4 design pillars: (1) persistent server-process owning provider auth/sessions/swarm/memory/MCP-pool, lightweight clients attach via Unix socket, sessions named adjective+animal, hot-reload via exec(), cross-harness resume from Claude Code/Codex/OpenCode. (2) Ambient mode: agentic background loop with garden/scout/work tools, mandatory end_cycle, adaptive rate-limit-aware scheduler, persistent scheduled queue, single-instance guard, crash safety. (3) Graph-first memory: cascade BFS retrieval through tag/cluster/semantic edges after embedding hits with depth-decayed edge-weighted scoring, per-category confidence half-lives, reinforcement provenance table, negative memories with trigger patterns, gap detection, post-retrieval maintenance. (4) Swarm coordination: coordinator/worktree-manager/agent roles, lifecycle states (spawned/ready/running/blocked/completed/failed/stopped/crashed), DM/channel/broadcast messaging, file-touch notifications, no locks. Skip list: 1800x mermaid renderer, custom terminal handterm, TTFI/RAM micro-benchmarks, multi-account /account UX, native Firefox bridge.',
  'jcode,research,reverse-engineering,phase-43,mit,coding-workflow,ambient,cascade,graph-rag,swarm',
  10, 'fact', 1746662400000, 'long', 1.0, 'research', 'episodic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'JCODE ADOPTION PROPOSALS (2026-05-07): twelve Phase 43 chunks sequenced foundations-first. 43.1 memorable session names + idle timeout for headless MCP (adjective+animal registry in mcp-data/sessions.json, --resume <name>; touches commands/mcp.rs, coding/coding_sessions.rs). 43.2 V20 schema migration adding memories.confidence REAL DEFAULT 1.0, memory_reinforcements(memory_id, session_id, message_index, ts), memory_trigger_patterns(memory_id, pattern, kind), memory_gaps(query_embedding, context_snippet, session_id, ts), safety_decisions(action, decision, decided_at, decided_via). One numbered seed migration under mcp-data/shared/migrations/. 43.3 per-category confidence decay half-lives Correction 365d / Preference 90d / Procedure 60d / Fact 30d / Inferred 7d in memory/maintenance_runtime.rs. 43.4 reinforcement provenance hooks at memory/reranker.rs and commands/streaming.rs. 43.5 cascade retrieval through memory_edges in memory/store.rs|graph_rag.rs (BFS depth<=2, weight*0.7^depth, edge-type priors Supersedes 0.9 / HasTag 0.8 / RelatesTo confidence / InCluster 0.6 / Contradicts|DerivedFrom 0.3, behind cascade=true flag, default-on for brain_suggest_context). 43.6 post-retrieval maintenance background task in new memory/post_retrieval.rs (strengthen co-relevant edges, +0.05 confidence verified / -0.02 rejected, log gap when verified empty). 43.7 negative memories cognitive_kind extension + memory/negative.rs prepends [NEGATIVE — DO NOT DO THIS] markers when triggers match (regex|substring|file_glob|language). 43.8 gap detection threshold top_score<0.3 && embedding_norm>0.7 + review_gaps MCP tool. 43.9 embedding-indexed instruction slices replaces bulk-XML rules-doc injection (chunk rules/instructions/docs by heading, top-K=10 + per-file TOC pointer line, --bulk-rules escape hatch, then update rules/prompting-rules.md). 43.10 Tier1/Tier2 safety classifier in coding/safety.rs with persistent decision history and 14-consecutive-approvals auto-promotion proposer. 43.11 background-maintenance agent skeleton coding/ambient.rs + coding/ambient_scheduler.rs (default disabled, garden-only until 20 cycles of feedback exist, single-instance PID guard in mcp-data/, x-ratelimit-* header parsing, 20% user headroom, exponential 429 backoff, AmbientControlPanel.vue). 43.12 cross-harness session import coding/session_import.rs reads ~/.claude|.codex|.opencode|.cursor|.config/github-copilot/cli/ transcripts and feeds memory/brain_memory.rs::extract_memories with imported_from tag. Out of scope for Phase 43: swarm same-repo multi-agent (extends mem id 110), MCP runner exec() hot-reload, structural agent-grep refactor of code_query (slot in later code-intel phase).',
  'jcode,adoption,phase-43,milestones,memory-schema,cascade,ambient,safety,instruction-slicing,session-import',
  10, 'procedure', 1746662400000, 'long', 1.0, 'planning', 'procedural'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'JCODE AMBIENT MODE LESSON (2026-05-07): a fixed-cadence maintenance scheduler (what TerranSoul has today in memory/maintenance_scheduler.rs) is strictly weaker than an *agentic* ambient loop. The right shape: tool surface garden/extract_from_session/verify_fact/scout_recent_sessions/request_permission/schedule_next/end_cycle (end_cycle is mandatory final tool that yields control back), adaptive interval that reads provider x-ratelimit-* headers and reserves 20% headroom for the user with exponential 429 backoff, persistent scheduled queue rows on disk surviving restarts, single-instance guard via PID file, subscription-OAuth providers prioritised over pay-per-token, atomic temp-file rename + last-processed checkpoints + interrupted-transcript markers for crash safety, every proactive-work action goes through request_permission and lands on a coding/ worktree branch, user feedback (rejections) becomes memories so future cycles avoid the pattern. Default enabled=false on first release with proactive_work=false (garden only) until decision-history feedback loop has at least 20 cycles of data. This is the single most behaviour-changing lift of Phase 43.',
  'jcode,ambient,lesson,scheduler,rate-limit,crash-safety,phase-43,maintenance',
  9, 'fact', 1746662400000, 'long', 1.0, 'architecture', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'JCODE CASCADE RETRIEVAL LESSON (2026-05-07): TerranSoul already populates memory_edges (typed/directional, V19) but hybrid_search_rrf does not traverse them at query time. Adding BFS to depth<=2 from the top-K seeds with edge-weighted depth-decayed scoring (weight * 0.7^depth, edge-type priors per memory_edges rel_type) is the single largest retrieval-quality upgrade we can ship without changing storage. Must ship behind a cascade=true query flag with an A/B recall benchmark extending benches/million_memory.rs; promote to default for brain_suggest_context only after recall@10 holds within +/-1% versus flat RRF on the existing test corpus. Final pool merges seed scores + cascade scores, dedupes by memory_id, returns top-K. We are not changing the RRF fusion itself, only enriching the candidate pool before scoring.',
  'jcode,cascade-retrieval,memory_edges,graph-rag,phase-43,recall-benchmark,brain_suggest_context',
  9, 'fact', 1746662400000, 'long', 1.0, 'architecture', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'JCODE INSTRUCTION SLICING LESSON (2026-05-07): the existing PROMPT CONTEXT RULE that bulk-loads every rules/*.md, instructions/*.md, docs/*.md as XML blocks into every coding-workflow prompt is a major context tax (~5-10k tokens per turn) and silently drops items that do not fit. The jcode pattern is to chunk those files by markdown heading, index each chunk as a memory (category=rule, cognitive_kind=instruction), and at prompt-build time embed the task description and pull only top-K matching slices plus a one-line table-of-contents pointer per file so the agent can request specific files explicitly. We must guarantee a hit on rules whose topic matches the task. Ship behind a --bulk-rules escape hatch in coding/prompting.rs for one release cycle. Update rules/prompting-rules.md once the new path is the default. This is chunk 43.9 and is strictly stronger than today.',
  'jcode,instruction-slicing,prompt-context,phase-43,prompting-rules,context-economy',
  9, 'fact', 1746662400000, 'long', 1.0, 'architecture', 'semantic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT child.id, parent.id, 'part_of', 1.0, 'seed', 1746662400000, 'seed'
FROM memories child
CROSS JOIN memories parent
WHERE parent.content LIKE 'JCODE RESEARCH RECORD (2026-05-07)%'
  AND child.content LIKE 'JCODE ADOPTION PROPOSALS (2026-05-07)%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT child.id, parent.id, 'part_of', 1.0, 'seed', 1746662400000, 'seed'
FROM memories child
CROSS JOIN memories parent
WHERE parent.content LIKE 'JCODE RESEARCH RECORD (2026-05-07)%'
  AND child.content LIKE 'JCODE AMBIENT MODE LESSON (2026-05-07)%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT child.id, parent.id, 'part_of', 1.0, 'seed', 1746662400000, 'seed'
FROM memories child
CROSS JOIN memories parent
WHERE parent.content LIKE 'JCODE RESEARCH RECORD (2026-05-07)%'
  AND child.content LIKE 'JCODE CASCADE RETRIEVAL LESSON (2026-05-07)%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT child.id, parent.id, 'part_of', 1.0, 'seed', 1746662400000, 'seed'
FROM memories child
CROSS JOIN memories parent
WHERE parent.content LIKE 'JCODE RESEARCH RECORD (2026-05-07)%'
  AND child.content LIKE 'JCODE INSTRUCTION SLICING LESSON (2026-05-07)%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT lesson.id, module.id, 'related_to', 0.9, 'seed', 1746662400000, 'seed'
FROM memories lesson
CROSS JOIN memories module
WHERE lesson.content LIKE 'JCODE AMBIENT MODE LESSON (2026-05-07)%'
  AND module.content LIKE 'HIVE MODULE MAP (2026-05-07)%';

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PHASE 43 COMPARISON CAPSTONE (2026-05-07): add a deferred final chunk (43.13) to run only after all other milestones are complete and green. Scope is a neutral TerranSoul-vs-jcode-plus-other-coding-tools comparison focused on measurable workflow outcomes (session continuity, memory quality, context efficiency, safety approvals, multi-agent orchestration, self-improve throughput), producing docs/coding-workflow-comparison-2026.md and a follow-up promotion proposal for only materially impactful gaps.',
  'phase-43,comparison,capstone,jcode,coding-tools,workflow-benchmark,milestones',
  8, 'fact', 1746662400000, 'long', 1.0, 'planning', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT capstone.id, plan.id, 'part_of', 1.0, 'seed', 1746662400000, 'seed'
FROM memories capstone
CROSS JOIN memories plan
WHERE capstone.content LIKE 'PHASE 43 COMPARISON CAPSTONE (2026-05-07)%'
  AND plan.content LIKE 'JCODE ADOPTION PROPOSALS (2026-05-07)%';

-- Phase 43 completion record (2026-05-07)
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PHASE 43 COMPLETION (2026-05-07): all 13 chunks complete, CI green. New modules: session_names, negative, gap_detection, instruction_slices, safety (Tier1/Tier2 classifier + decision history + auto-promotion), ambient (agent skeleton + PID guard + garden/gate/end_cycle), ambient_scheduler (rate-limit-aware + 429 exponential backoff + 20% user headroom), session_import (5 harnesses: Claude/Codex/OpenCode/Cursor/CopilotCli, JSON/JSONL parsing, secret redaction). Rust test count: 2496 (from ~2340 at phase start). Comparison doc: docs/coding-workflow-comparison-2026.md. Key schema: V20 with confidence, reinforcements, trigger_patterns, gaps, safety_decisions tables. safety_decisions columns: (id, action, decision, decided_at, decided_via) — NOT (tier, reason, ts). Follow-up proposal in comparison doc covers RAG latency, setup wizard, ambient validation, replay mode, embedding model registry.',
  'phase-43,completion,summary,safety,ambient,session-import,instruction-slices,negative-memories,gap-detection',
  10, 'fact', 1746662400000, 'long', 1.0, 'summary', 'episodic'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT completion.id, plan.id, 'completes', 1.0, 'seed', 1746662400000, 'seed'
FROM memories completion
CROSS JOIN memories plan
WHERE completion.content LIKE 'PHASE 43 COMPLETION (2026-05-07)%'
  AND plan.content LIKE 'JCODE ADOPTION PROPOSALS (2026-05-07)%';

-- Phase 45 plan + design lessons (2026-05-07): project-knowledge architecture
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'PROJECT KNOWLEDGE ARCHITECTURE (2026-05-07): TerranSoul code knowledge uses a 3-tier layer model — base snapshot at .codegraph/snapshot.json (deterministic, committable, no merge driver needed because every row keys on (repo_label, file, content_hash, line) and outputs are sorted), branch overlay in SQLite table code_branch_overlays(repo_id, base_ref, branch_ref, file, hash) with overlay_id columns on code_symbols/code_edges (queries union overlay-over-base), and in-memory working-tree overlay for dirty files. Branch switches call code_branch_sync(prev, new) which re-indexes only git diff --name-only prev..new files. Designed in docs/project-knowledge-architecture.md.',
  'design,phase-45,project-knowledge,branch-overlay,snapshot,code-intelligence,architecture',
  9, 'fact', 1746662400000, 'long', 1.0, 'architecture', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'GRAPHIFY ISSUE #52 LESSONS (2026-05-07): public failure-mode catalogue for large-repo knowledge graphs. (1) tree-sitter version mismatch crashed whole pipeline → TerranSoul gates each parser behind a parser-* Cargo feature, single file parse error never aborts others. (2) Python-only cross-file resolution crashed Swift/ObjC → resolver per-language with provenance=skipped:parser_error fallback. (3) PDFs in .imageset/.xcassets misclassified as papers → vendor/asset detector excludes by path. (4) 7414 micro-clusters on 22k nodes → min_cluster_size (default 8) + two-phase clustering (directory partition first). (5) HTML 5000-node hard error → workbench renders top-N degree nodes per cluster with drill-in, no hard cap. (6) god nodes dominated by Pods/node_modules → vendor-tier files excluded from god-node ranking by default. (7) no iOS preset → .codeignore auto-detected from build manifests. (8) no progress feedback → gate_telemetry emits code_index_progress every 100 files. Source: https://github.com/safishamsi/graphify/issues/52',
  'lesson,graphify,scale,clustering,vendor-detection,phase-45,reverse-engineering,credits',
  9, 'fact', 1746662400000, 'long', 1.0, 'lesson', 'semantic'
);

INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'BRANCH SYNC PROTOCOL (2026-05-07): TerranSoul code-knowledge git lifecycle uses three POSIX hook scripts (post-checkout, post-merge, post-commit) installed by code_install_hooks Tauri command. Hooks POST to local MCP (127.0.0.1:7421/7422/7423) and EXIT 0 on any error so git is never blocked. post-checkout/post-merge call code_branch_sync(prev, new); post-commit calls code_index_commit(sha) and atomically promotes overlay to base when HEAD == base_ref. No git merge driver is needed because the snapshot is deterministic. .codegraph/ is committed (snapshot.json + snapshot.meta.json); code_index.sqlite stays local-only and is a derived cache. Multi-repo orgs use existing code_repo_groups + code_contracts (chunk 37.13) plus new code_group_drift / code_branch_diff MCP tools.',
  'design,phase-45,git-hooks,branch-sync,mcp,code-intelligence,multi-repo,non-blocking',
  9, 'fact', 1746662400000, 'long', 1.0, 'architecture', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT plan.id, lesson.id, 'informed_by', 1.0, 'seed', 1746662400000, 'seed'
FROM memories plan
CROSS JOIN memories lesson
WHERE plan.content LIKE 'PROJECT KNOWLEDGE ARCHITECTURE (2026-05-07)%'
  AND lesson.content LIKE 'GRAPHIFY ISSUE #52 LESSONS (2026-05-07)%';

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT protocol.id, plan.id, 'part_of', 1.0, 'seed', 1746662400000, 'seed'
FROM memories protocol
CROSS JOIN memories plan
WHERE protocol.content LIKE 'BRANCH SYNC PROTOCOL (2026-05-07)%'
  AND plan.content LIKE 'PROJECT KNOWLEDGE ARCHITECTURE (2026-05-07)%';

-- MCP Compliance Gate (2026-05-07): active enforcement of project governance rules
INSERT OR IGNORE INTO memories (content, tags, importance, memory_type, created_at, tier, decay_score, category, cognitive_kind)
VALUES (
  'MCP COMPLIANCE GATE (2026-05-07): TerranSoul MCP now actively enforces project rules via compliance_gate.rs. Instead of passive text, the MCP server tracks per-session compliance state and (1) injects reminder annotations into tool responses when preflight is incomplete, (2) exposes a brain_session_checklist tool showing completed/outstanding steps, (3) accepts compliance/signal notifications from agents to mark steps done. Preflight steps: brain_health called, brain_search/brain_suggest_context called, MCP receipt shown. Post-chunk steps: completion-log updated, milestones cleaned, seed synced. Reminders appear in the first 5 tool calls and every 10th call thereafter if preflight is not done. Does NOT block tool calls (MCP protocol-compliant) but makes violations visible in the response text. Located at src-tauri/src/ai_integrations/mcp/compliance_gate.rs, wired into router.rs tool dispatch.',
  'mcp,compliance-gate,enforcement,rules,session-tracking,preflight,milestone-hygiene,non-negotiable',
  10, 'fact', 1746662400000, 'long', 1.0, 'architecture', 'procedural'
);

INSERT OR IGNORE INTO memory_edges (src_id, dst_id, rel_type, confidence, source, created_at, edge_source)
SELECT gate.id, rule.id, 'implements', 1.0, 'seed', 1746662400000, 'seed'
FROM memories gate
CROSS JOIN memories rule
WHERE gate.content LIKE 'MCP COMPLIANCE GATE (2026-05-07)%'
  AND rule.content LIKE 'MILESTONE HYGIENE RULE%';
