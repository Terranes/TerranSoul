-- TerranSoul MCP Brain Seed Data
-- Applied on first `npm run mcp` when memory.db does not exist yet.
-- Contains architectural knowledge so agents can be productive immediately.
--
-- Schema: see src-tauri/src/memory/schema.rs (version 13)
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
('MCP server exposes brain on three ports: 7421 (release app), 7422 (dev app), 7423 (headless npm run mcp). All wired into .vscode/mcp.json. Bearer-token auth required. Tools: brain_search, brain_get_entry, brain_list_recent, brain_kg_neighbors, brain_summarize, brain_suggest_context, brain_ingest_url, brain_health, code_query, code_context, code_impact, code_rename.', 'mcp,server,tools,setup', 5, 'fact', 1746316800000, 'long', 1.0, 70, 'mcp'),

('MCP shared data policy: mcp-data/shared is committed and reviewable; runtime files such as mcp-token.txt, memory.db, SQLite WAL/SHM files, vector indexes, logs, locks, sessions, and worktrees are ignored. Contributors and self-improve runs may update mcp-data/shared/memory-seed.sql with durable project knowledge.', 'mcp,data,gitignore,shared-seed', 5, 'procedure', 1746316800000, 'long', 1.0, 60, 'mcp'),

('To start MCP headless server: run npm run mcp from the repo root. It binds 127.0.0.1:7423 when available, uses mcp-data/ for state, auto-configures brain to Pollinations free API if Ollama is unavailable, and writes the bearer token to mcp-data/mcp-token.txt plus .vscode/.mcp-token.', 'mcp,setup,quickstart', 5, 'procedure', 1746316800000, 'long', 1.0, 50, 'mcp'),

-- Setup & Development
('CI gate command: npx vitest run && npx vue-tsc --noEmit && cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test. Run after every chunk completion. On Linux, install Tauri WebKit/GTK system libraries before Rust checks.', 'ci,testing,workflow', 5, 'procedure', 1746316800000, 'long', 1.0, 40, 'development'),

('MCP/app dependency bootstrap rule: if npm run mcp, npm run dev, cargo tauri dev, or validation fails because pkg-config cannot find Linux system libraries such as glib-2.0 or gio-2.0, install the missing Tauri/MCP packages with the platform package manager and retry before declaring the task blocked. Ubuntu minimum set: libglib2.0-dev, libgtk-3-dev, libwebkit2gtk-4.1-dev, libappindicator3-dev, librsvg2-dev, patchelf, libsoup-3.0-dev, libjavascriptcoregtk-4.1-dev, pkg-config.', 'mcp,setup,dependencies,tauri,linux,agent-rule', 5, 'procedure', 1746487655000, 'long', 1.0, 95, 'development'),

('Dev server: npm run dev starts Vite on :1420. Tauri dev: cargo tauri dev. Full build: cargo tauri build. The app window opens a webview to the Vite dev server.', 'development,setup,commands', 4, 'procedure', 1746316800000, 'long', 1.0, 35, 'development'),

('Coding standards: snake_case for Rust, camelCase for TypeScript. Never .unwrap() in library code — use ? + thiserror. Vue components use <script setup lang="ts"> with scoped styles. CSS uses var(--ts-*) design tokens. Tests required for all new functionality.', 'coding-standards,conventions', 5, 'fact', 1746316800000, 'long', 1.0, 45, 'development'),

-- Code Intelligence (Chunks 31.3-31.8)
('Code intelligence pipeline: (1) tree-sitter symbol-table ingest (Rust + TypeScript grammars), (2) cross-file resolution + call graph with confidence scores, (3) label-propagation functional clustering via petgraph, (4) entry-point scoring + BFS process tracing, (5) native MCP tools (code_query, code_context, code_impact, code_rename), (6) editor pre/post-tool-use hooks with auto re-indexing.', 'code-intelligence,symbol-index,mcp', 5, 'fact', 1746316800000, 'long', 1.0, 65, 'code-intelligence'),

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
('Memory module map: schema.rs (canonical V13 SQLite schema), store.rs (default SQLite memory store with hybrid_search + hybrid_search_rrf + ANN bridge), ann_index.rs (HNSW via usearch), backend.rs (StorageBackend trait/factory), cassandra.rs / mssql.rs / postgres.rs (optional backends), chunking.rs + late_chunking.rs (semantic chunking), code_rag.rs, cognitive_kind.rs, conflicts.rs + edge_conflict_scan.rs (LLM contradiction resolution), consolidation.rs, context_pack.rs ([RETRIEVED CONTEXT] assembly), contextualize.rs (Anthropic Contextual Retrieval), crag.rs, crdt_sync.rs, edges.rs (typed/directional KG edges), fusion.rs (RRF k=60), gitnexus_mirror.rs, graph_rag.rs, hyde.rs (HyDE), matryoshka.rs, obsidian_export.rs + obsidian_sync.rs, query_intent.rs, reflection.rs (/reflect session reflection + derived_from source-turn provenance), replay.rs, reranker.rs (LLM-as-judge), tag_vocabulary.rs, temporal.rs, versioning.rs.', 'memory,module-map,architecture', 5, 'fact', 1746316800000, 'long', 1.0, 165, 'memory'),

('Hybrid 6-signal search weights live in src-tauri/src/memory/store.rs: vector(40%) + keyword(20%) + recency(15%) + importance(10%) + decay(10%) + tier(5%). RRF fusion uses k=60. HyDE and cross-encoder rerank are optional per-query; default for RRF/HyDE MCP search is rerank on with rerank_threshold 0.55.', 'memory,search,signals,rag,rerank', 5, 'fact', 1746316800000, 'long', 1.0, 70, 'memory'),

('SQLite schema is at version 13 (CANONICAL_SCHEMA_VERSION in src-tauri/src/memory/schema.rs). memories columns: content, tags, importance, memory_type, created_at, last_accessed, access_count, embedding, source_url, source_hash, expires_at, tier, decay_score, session_id, parent_id, token_count, valid_to, obsidian_path, last_exported, category, updated_at, origin_device. Edges in memory_edges (typed, directional). Versions in memory_versions. FTS5 virtual table for keyword search.', 'memory,schema,sqlite', 5, 'fact', 1746316800000, 'long', 1.0, 90, 'memory'),

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
('Design docs (docs/): AI-coding-integrations.md (MCP/gRPC for VS Code Copilot/Cursor/Codex/Claude Code), brain-advanced-design.md (brain architecture + schema + RAG pipeline + roadmap, kept in sync with code), charisma-teaching-tutorial.md, coding-workflow-design.md (self-improve), gitnexus-capability-matrix.md, licensing-audit.md, llm-animation-research.md, momask-full-body-retarget-research.md, motion-model-inference-evaluation.md (MotionGPT/T2M-GPT eval), multi-agent-workflows-tutorial.md, neural-audio-to-face-evaluation.md, offline-motion-polish-research.md, persona-design.md + persona-pack-schema.md, plugin-development.md, teachable-capabilities.md.', 'docs,inventory,design', 4, 'fact', 1746316800000, 'long', 1.0, 110, 'docs'),

-- ====================================================================
-- Rules (one-line summaries)
-- ====================================================================
('Rules files (rules/): agent-mcp-bootstrap.md (how agents connect to npm run mcp), architecture-rules.md (incl. brain doc-sync rule), backlog.md, coding-standards.md (incl. Multi-Agent Instruction Sync, CREDITS rule), coding-workflow-reliability.md, completion-log.md (permanent done-chunk record, 10k-line cap then archived), llm-decision-rules.md, local-first-brain.md, milestones.md (active queue: only not-started/in-progress), prompting-rules.md (incl. enforcement rules), quality-pillars.md, reality-filter.md (no pretend code), research-reverse-engineering.md, ui-ux-standards.md.', 'rules,inventory,governance', 5, 'fact', 1746316800000, 'long', 1.0, 110, 'rules'),

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
('STORAGE INVARIANT (mcp-data seed): mcp-data/shared/memory-seed.sql is REAL SQL inserted into the canonical SQLite memories + memory_edges schema (V13, schema.rs) — not a markdown vault. The seed actively exercises the full stack: (a) **schema** = memory_type/tier/decay_score/importance/category populated for every row, (b) **FTS5** = content is indexed automatically by schema.rs CREATE VIRTUAL TABLE, (c) **knowledge graph** = the `-- KNOWLEDGE GRAPH EDGES` section below populates ~40 typed memory_edges (part_of / cites / supports / derived_from / related_to) so brain_kg_neighbors works on day one, (d) **RRF fusion + HyDE + reranker** = all run at query time on whatever signals are populated. **HNSW vectors** are populated lazily: when a brain provider is configured, call the `backfill_embeddings` Tauri command (commands/memory.rs:593) — it walks store.unembedded_ids() and calls embed_for_mode for each. Until then, the 5 non-vector signals (keyword/recency/importance/decay/tier) still fully power RRF, brain_search, and brain_suggest_context.', 'storage,invariant,seed,architecture,kg,embeddings,backfill', 5, 'fact', 1746316800000, 'long', 1.0, 220, 'storage'),

('EMBEDDING BACKFILL PROCEDURE: After first MCP startup with the shared seed, vector signals are populated by the post-seed `backfill_mcp_seed_embeddings` hook when an embedding source is configured, or by the explicit `backfill_embeddings` Tauri command (commands/memory.rs:593) which iterates store.unembedded_ids() and calls embed_for_mode(content, brain_mode, active_brain) per row, then store.set_embedding(id, &emb). The shared maintenance scheduler now starts in both GUI and headless MCP modes for decay, GC, tier promotion, and edge extraction. For headless `npm run mcp` with no embed provider configured, embedding backfill is a no-op by design — RRF + HyDE + reranker still work on keyword/recency/importance/decay/tier signals, plus KG traversal and FTS5 keyword search work fully without vectors.', 'procedure,embeddings,backfill,mcp,headless,maintenance', 5, 'procedure', 1746316800000, 'long', 1.0, 150, 'procedures'),

('LESSON: GitNexus is PolyForm Noncommercial. TerranSoul must not bundle, vendor, auto-install, or default-spawn GitNexus packages, binaries, Docker images, prompts, skills, or UI assets. Treat GitNexus and its DeepWiki pages as credited public product and architecture research only, then implement neutral TerranSoul-native Rust/Vue code-intelligence features. Sidecar bridge code is removed entirely; the only supported MCP/code path is native TerranSoul code intelligence.', 'lesson,gitnexus,license,clean-room,code-intelligence,mcp,native,noncommercial', 10, 'procedure', 1746489600000, 'long', 1.0, 90, 'lessons'),

('CODE INTELLIGENCE ROADMAP: Native TerranSoul parity targets learned from public GitNexus docs are repo registry, incremental content-hash indexing, broader Tree-sitter language coverage, import/re-export resolution, heritage and receiver/type inference, confidence-scored code relations, functional clusters, execution processes, hybrid BM25 plus semantic plus RRF code search, diff impact, graph-backed rename, MCP resources/prompts, generated agent skills, and code-wiki generation. Implement these under neutral names using existing coding/symbol_index.rs, coding/processes.rs, memory RAG, and MCP tool surfaces.', 'roadmap,gitnexus,code-intelligence,native,mcp,symbol-index,processes,search', 9, 'fact', 1746489600000, 'long', 1.0, 95, 'code-intelligence'),

('CODE WORKBENCH UX LESSON: GitNexus public Web UI shows a useful AI-development pattern to reimplement natively: graph canvas as the primary structural map, file tree as physical navigation, code references panel as grounded evidence, right-side chat with visible tool-call cards, clickable file/node citations that focus graph and code, process diagrams, repo switcher, status bar, and blast-radius highlights for change risk. TerranSoul should adapt this as a dense Brain/Coding workbench using Vue, Pinia, Cytoscape or Three.js, and existing design tokens, not copy React components or visual identity.', 'lesson,gitnexus,ui-ux,code-workbench,graph,chat,citations,blast-radius', 9, 'fact', 1746489600000, 'long', 1.0, 95, 'frontend'),

('LESSON: LAN MCP brain sharing is an opt-in local-network retrieval flow. The host must enable LAN brain sharing before starting or restarting the MCP server, then name the shared brain and share the bearer token out-of-band. Discovery uses UDP 7424 for metadata only; authenticated retrieval uses MCP HTTP brain_search against the host port. Peers retrieve ranked snippets, not the host memory database.', 'lesson,mcp,lan,brain-sharing,discovery,token,remote-search,tutorial', 9, 'procedure', 1777939200000, 'long', 1.0, 75, 'mcp'),

('RULE: Treat LAN MCP bearer-token access as read access to the shared TerranSoul knowledge surface. Never broadcast the token, never enable LAN sharing on public Wi-Fi, and stop sharing when the session ends. User-facing docs should describe this with the docs/lan-mcp-sharing-tutorial.md Alice Vietnamese law notes scenario and avoid legal-advice claims.', 'rule,mcp,lan,security,bearer-token,docs,legal-disclaimer', 9, 'procedure', 1777939200000, 'long', 1.0, 70, 'mcp'),

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

('VERDICT: Reject direct GitNexus import/bundling. TerranSoul must keep clean-room native code-intelligence UX inspired by public behavior only; no GitNexus binaries, Docker images, prompts, skills, or UI assets may be bundled, auto-installed, or default-spawned due PolyForm Noncommercial constraints.', 'verdict,gitnexus,clean-room,license,ui-ux,native,noncommercial', 10, 'decision', 1778025600000, 'long', 1.0, 100, 'code-intelligence');

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
    OR d.content LIKE 'SQLite schema is at version 13%'
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
    OR d.content LIKE 'SQLite schema is at version 13%'
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
  'STACK COVERAGE: the mcp-data seed exercises the full TerranSoul retrieval stack — SQLite schema V13 (every row), FTS5 (auto-indexed on insert), KG edges (memory_edges populated by content-LIKE subqueries below), RRF fusion (5 non-vector signals always live; vector signal lights up after Phase 33 chunk 33.1 backfill), HyDE expansion (works on any populated row at query time), and LLM-as-judge reranker (default-on for RRF/HyDE when a local brain is available, pruning below threshold 0.55). See rules/milestones.md Phase 33 for outstanding optimisation chunks.',
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
