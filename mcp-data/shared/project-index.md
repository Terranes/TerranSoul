# TerranSoul Project Index — Default MCP Knowledge Base

> This file is the **single source of truth** for navigating TerranSoul without
> rescanning the codebase. It is committed under `mcp-data/shared/` and seeded
> into the MCP brain on first run, so any agent (Copilot, Codex, Cursor, Claude
> Code, self-improve) can answer "where does feature X live?" by querying the
> brain or reading this file directly. **Update this file in the same PR
> whenever a major module, store, doc, or rule is added, removed, or
> significantly restructured** so future sessions never duplicate work.

## Top-level layout

| Path | Purpose |
|---|---|
| `src/` | Vue 3.5 + TypeScript 5 frontend (Pinia stores, components, composables, views) |
| `src-tauri/` | Rust backend (Tauri 2.x desktop shell, MCP server, brain, memory, voice, sync) |
| `src-tauri/src/` | All Rust source modules (see "Rust modules") |
| `public/` | Static assets — VRM models, animations, audio clips, images |
| `rules/` | Project rules, milestones, completion log, agent bootstrap (see "Rules") |
| `docs/` | Long-form design documents (see "Design docs") |
| `scripts/` | Dev utilities (`wait-for-service.mjs`, `copilot-loop.mjs`, etc.) |
| `mcp-data/` | MCP brain runtime root (token, sqlite db, indexes — most ignored) |
| `mcp-data/shared/` | **Tracked** seed knowledge for the MCP brain (this file lives here) |
| `.github/workflows/` | CI: `terransoul-ci.yml`, `codeql.yml`, `copilot-setup-steps.yml` |
| `.vscode/` | Workspace settings + `mcp.json` for the three MCP profiles (release/dev/headless) |

## Durable agent rules to retrieve via MCP

- Reverse-engineering GitHub projects is DeepWiki-first: check
  `https://deepwiki.org/<owner>/<repo>` when reachable, cross-check upstream,
  credit the source in `CREDITS.md`, and sync durable lessons into
  `mcp-data/shared/**`.
- MCP self-improve knowledge lives in tracked shared files. When a session
  learns a durable project rule, update `memory-seed.sql`,
  `lessons-learned.md`, or this index in the same PR so future agents retrieve
  it without rescanning chat history.
- Markdown is not MCP memory: if durable knowledge is written in any `.md`,
  mirror it into `memory-seed.sql` and wire relationships with `memory_edges`
  so SQLite + the knowledge graph stays authoritative for MCP retrieval.

## Rust modules (`src-tauri/src/`)

### `brain/` — LLM provider management & RAG glue
| File | Purpose |
|---|---|
| `mod.rs` | Module root; provider trait, shared types |
| `brain_config.rs` | Persisted brain config (provider, model, endpoints, keys) |
| `brain_store.rs` | Brain state container + lifecycle |
| `cloud_embeddings.rs` | Unified `embed_for_mode()` for paid/free cloud embedding APIs |
| `context_budget.rs` | Token budgeting for prompt assembly |
| `doc_catalogue.rs` | Brain-aware doc catalogue used during context assembly |
| `docker_ollama.rs` | Auto-setup Ollama via Docker when present |
| `free_api.rs` | Pollinations / OpenRouter free-tier provider |
| `intent_classifier.rs` | Lightweight intent classifier for query routing |
| `lm_studio.rs` | LM Studio provider adapter |
| `maintenance_runtime.rs`, `maintenance_scheduler.rs` | Shared GUI/headless MCP background maintenance (decay, GC, tier promotion, edge extraction) |
| `mcp_auto_config.rs` | Auto-configures the brain when running headless MCP |
| `model_recommender.rs` | RAM-based model catalogue (Gemma 4, Phi-4, Kimi K2.6, etc.) |
| `ollama_agent.rs` | Local Ollama provider; `embed_text`, `hyde_complete`, `rerank_score` |
| `ollama_lifecycle.rs` | Start/stop/health for local Ollama |
| `openai_client.rs` | OpenAI-compatible cloud provider |
| `provider_rotator.rs` | Round-robin/failover across providers |
| `ram_budget.rs` | RAM probing for safe model selection |
| `segmenter.rs` | Text segmentation for ingestion |
| `selection.rs` | Model selection policy |
| `system_info.rs` | Host hardware probe |

### `memory/` — Storage backends, RAG, knowledge graph
| File | Purpose |
|---|---|
| `mod.rs` | Module root; `StorageBackend` trait |
| `schema.rs` | Canonical SQLite schema (V15) — `memories`, `memory_edges`, FTS5, `pending_embeddings`, protected eviction, etc. |
| `store.rs` | Default SQLite memory store; `hybrid_search`, `hybrid_search_rrf`, ANN bridge |
| `ann_index.rs` | HNSW ANN via `usearch` 2.x |
| `auto_learn.rs`, `auto_tag.rs` | Automatic ingestion + tagging |
| `backend.rs` | `StorageBackend` enum + factory |
| `brain_maintenance.rs`, `brain_memory.rs` | LLM-powered extract/summarize/search |
| `cassandra.rs`, `mssql.rs`, `postgres.rs` | Optional non-default backends |
| `chunking.rs`, `late_chunking.rs` | Semantic chunking (text-splitter crate) |
| `code_rag.rs` | Code-aware retrieval |
| `cognitive_kind.rs` | Memory taxonomy classification |
| `conflicts.rs`, `edge_conflict_scan.rs` | LLM-powered contradiction resolution |
| `consolidation.rs` | Long-term consolidation pass |
| `context_pack.rs` | `[RETRIEVED CONTEXT]` pack assembly |
| `contextualize.rs` | Anthropic-style Contextual Retrieval |
| `crag.rs`, `crdt_sync.rs` | Corrective RAG; CRDT integration hooks |
| `edges.rs` | Knowledge graph edges (typed, directional) |
| `fusion.rs` | Reciprocal Rank Fusion (RRF, k=60) |
| `gitnexus_mirror.rs` | GitNexus repo mirror surface |
| `graph_rag.rs` | GraphRAG community summaries |
| `hyde.rs` | Hypothetical Document Embeddings |
| `matryoshka.rs` | Matryoshka embedding tier support |
| `obsidian_export.rs`, `obsidian_sync.rs` | One-way + bidirectional Obsidian vault integration |
| `query_intent.rs` | Query-intent classifier wiring into hybrid search |
| `replay.rs` | Replayable session memory |
| `reranker.rs` | LLM-as-judge cross-encoder reranker with default 0.55 threshold pruning for RRF/HyDE |
| `tag_vocabulary.rs` | Controlled tag vocabulary |
| `temporal.rs` | Natural-language time-range queries |
| `versioning.rs` | Non-destructive memory edit history (V8) |

### `ai_integrations/` — External AI assistant surface
| File | Purpose |
|---|---|
| `mod.rs` | Module root |
| `gateway.rs` | `BrainGateway` trait + `AppStateGateway` adapter (8 ops) |
| `mcp/` | MCP server (HTTP on 7421/7422/7423, bearer auth, tools, prompts, resources) |
| `grpc/` | gRPC `brain.v1` transport for desktop ↔ mobile bridge |

### `persona/` — Personality, motion, charisma
| File | Purpose |
|---|---|
| `mod.rs`, `pack.rs` | Persona pack import/export, schema |
| `extract.rs`, `drift.rs` | Trait extraction + drift detection |
| `charisma.rs` | Charisma teaching system |
| `motion_clip.rs`, `motion_feedback.rs`, `motion_reconstruction.rs`, `motion_smooth.rs`, `motion_tokens.rs` | Motion clip parsing, polish, MotionGPT token codec |
| `pose_frame.rs`, `prosody.rs`, `retarget.rs` | Pose frames, prosody features, MoMask-style retargeting |

### `voice/` — TTS / ASR / diarization
| File | Purpose |
|---|---|
| `mod.rs`, `config_store.rs` | Voice config persistence |
| `stub_asr.rs`, `stub_diarization.rs`, `stub_tts.rs` | Default offline stubs |
| `whisper_api.rs` | Whisper-compatible ASR endpoint |

### `commands/` — Tauri command surface (~150)
| File | Purpose |
|---|---|
| `agent.rs`, `agents_roster.rs` | Agent commands & roster |
| `auto_setup.rs` | First-launch auto-setup |
| `brain.rs`, `chat.rs`, `streaming.rs` | Chat + streaming pipeline |
| `character.rs`, `emotion.rs`, `vision.rs` | Character + emotion + vision |
| `charisma.rs`, `persona.rs` | Persona/charisma surface |
| `coding.rs`, `coding_sessions.rs` | Self-improve coding workflow, persisted workboard, and session transcripts |
| `consolidation.rs`, `crag.rs`, `gitnexus.rs`, `ingest.rs`, `memory.rs` | Memory operations |
| `docker.rs`, `lan.rs`, `link.rs`, `routing.rs` | Networking, container, sync |
| `github_auth.rs` | GitHub OAuth device flow for self-improve PRs |
| `grpc.rs`, `mcp.rs` | gRPC + MCP lifecycle commands |
| `identity.rs`, `messaging.rs` | Device identity + cross-device messaging |
| `package.rs`, `plugins.rs`, `registry.rs`, `sandbox.rs` | Plugins + WASM sandbox |
| `quest.rs` | Skill-tree quests |
| `settings.rs`, `window.rs`, `workflow_plans.rs` | Settings, windows, workflow plans |
| `teachable_capabilities.rs` | Teachable capabilities registry |
| `translation.rs` | Worldwide translator |
| `user_models.rs` | Custom VRM / user-supplied models |
| `voice.rs` | Voice commands |
| `vscode.rs` | VS Code workspace integration |
| `ipc_contract_tests.rs` | IPC contract regression tests |

### `coding/` — Self-improve engine
| File | Purpose |
|---|---|
| `mod.rs` | Module root |
| `apply_file.rs`, `git_ops.rs`, `worktree.rs` | Patch application + temp git worktrees |
| `autostart.rs`, `client.rs`, `engine.rs`, `workflow.rs` | Self-improve engine + autostart |
| `context_budget.rs`, `context_engineering.rs`, `prompting.rs` | Prompt assembly |
| `conversation_learning.rs`, `session_chat.rs`, `task_queue.rs` | Session learning + queues |
| `cost.rs`, `metrics.rs`, `gate_telemetry.rs`, `rolling_log.rs` | Cost, metrics, gate telemetry, and bounded runtime JSONL rotation |
| `dag_runner.rs`, `multi_agent.rs`, `resolver.rs`, `reviewer.rs`, `processes.rs` | DAG orchestration + reviewers |
| `github.rs`, `handoff.rs`, `handoff_store.rs`, `milestones.rs`, `promotion_plan.rs` | GitHub PR flow + handoff |
| `repo.rs`, `rename.rs`, `symbol_index.rs`, `wiki.rs`, `test_runner.rs` | Repo helpers, symbol index, test runner |

### Other Rust modules
| Path | Purpose |
|---|---|
| `agent/` | Stub + GitNexus sidecar + OpenClaw bridge agents |
| `agents/` | CLI worker + agent roster |
| `container/` | Container/runtime helpers |
| `identity/` | Ed25519 device identity, key store, QR pairing, trusted device registry |
| `link/` | Cross-device sync transport (QUIC + WebSocket) |
| `messaging/` | Cross-device messaging |
| `network/` | LAN discovery + binding |
| `orchestrator/` | Agent orchestrator, agentic RAG, coding router, self-RAG |
| `package_manager/` | App package manager surface |
| `plugins/` | Plugin host + manifest |
| `registry_server/` | Local plugin/agent registry server |
| `routing/` | Command router (multi-device) |
| `sandbox/` | WASM sandbox (capability gating, host API) |
| `settings/` | App settings |
| `sync/` | CRDT primitives (LWW register, OR-Set, append log) |
| `tasks/` | Long-running task manager |
| `teachable_capabilities/` | Teachable capability registry |
| `vscode_workspace/` | VS Code workspace launcher + registry + resolver |
| `workflows/` | Workflow engine + resilience (retry, circuit breaker, watchdog) |
| `lib.rs`, `main.rs` | Tauri entry points; `--mcp-http` headless mode |

## Frontend Pinia stores (`src/stores/`)

| Store | Purpose |
|---|---|
| `agent-roster` | Agent inventory + roles |
| `ai-decision-policy` | LLM decision-routing policy |
| `ai-integrations` | Configures MCP/gRPC for external assistants |
| `audio` | Audio device + playback state |
| `background` | Background scene preset selection |
| `brain` | Active brain config (provider/model/keys), provider switching |
| `browser-lan` | Browser-mode LAN discovery |
| `character` | Active VRM + emotion state |
| `charisma` | Charisma teaching scores |
| `chat-store-router` | Routes chat to local vs. remote conversation store |
| `coding-workflow` | Self-improve workflow config + state |
| `conversation` | Local chat history + streaming |
| `identity` | Device identity for pairing |
| `link` | Soul-link sync session |
| `mcp-activity` | Live MCP tool-use activity for the UI badge |
| `memory` | Memory CRUD + search |
| `messaging` | Cross-device messages |
| `mobile-notifications`, `mobile-pairing` | Mobile companion |
| `package` | App package metadata |
| `persona`, `persona-types` | Persona traits + learned expressions/motions |
| `plugins` | Installed plugins + capability grants |
| `provider-health` | Provider health probes |
| `remote-conversation` | Remote (paired-device) conversation |
| `routing` | Command router state |
| `sandbox` | WASM sandbox state |
| `self-improve` | Self-improve session list + transcripts |
| `settings` | App preferences |
| `skill-tree` | Gamified quest skill tree (~1500 lines) |
| `streaming` | Streaming flag/state |
| `sync` | CRDT sync state |
| `tasks` | Long-running task tracking |
| `teachable-capabilities` | Capability registry |
| `voice` | TTS/ASR config |
| `window` | Window/Pet mode + visibility |
| `workflow-plans` | Multi-agent workflow plans |

## Frontend composables (`src/composables/`)

| Composable | Purpose |
|---|---|
| `useActivePluginTheme` | Active plugin theme tokens |
| `useAsrManager` | ASR lifecycle (browser + native) |
| `useBgmPlayer` | Background music playback |
| `useCameraCapture` | Camera capture for vision |
| `useChatExpansion` | Chat input expansion logic |
| `useChatExport` | Export chat transcripts |
| `useDiarization` | Voice diarization |
| `useHotwords` | Hotword detection |
| `useKeyboardDetector` | Detect on-screen keyboard |
| `useLipSyncBridge` | Lip-sync between TTS and VRM |
| `useModelCameraStore` | Per-model camera framing |
| `usePluginCapabilityGrants` | Plugin capability prompts |
| `usePluginSlashDispatch` | Slash command dispatch into plugins |
| `usePresenceDetector` | Presence detection |
| `useTheme` | Global theme |
| `useTranslation` | Worldwide translator |
| `useTtsPlayback` | TTS playback |
| `useVrmThumbnail` | Off-screen VRM thumbnail rendering |

## Design docs (`docs/`)

| Doc | What it covers |
|---|---|
| `AI-coding-integrations.md` | How MCP/gRPC integrates with VS Code Copilot, Cursor, Codex, Claude Code |
| `brain-advanced-design.md` | Brain architecture, schema, RAG pipeline, roadmap (kept in sync with code) |
| `tutorials/charisma-teaching-tutorial.md` | Charisma teaching workflow |
| `coding-workflow-design.md` | Self-improve coding workflow |
| `gitnexus-capability-matrix.md` | GitNexus tool capability matrix |
| `licensing-audit.md` | Dependency license audit |
| `llm-animation-research.md` | LLM-driven animation research |
| `momask-full-body-retarget-research.md` | MoMask retarget research |
| `motion-model-inference-evaluation.md` | MotionGPT/T2M-GPT eval |
| `tutorials/multi-agent-workflows-tutorial.md` | Multi-agent workflows |
| `neural-audio-to-face-evaluation.md` | Audio-to-face eval |
| `offline-motion-polish-research.md` | Offline motion polish research |
| `persona-design.md`, `persona-pack-schema.md` | Persona system + pack schema |
| `plugin-development.md` | Plugin authoring |
| `teachable-capabilities.md` | Teachable capabilities |

## Rules (`rules/`)

| File | Purpose |
|---|---|
| `agent-mcp-bootstrap.md` | How any agent connects to `npm run mcp` (token/profile/health) |
| `architecture-rules.md` | Architectural constraints + brain doc-sync rule |
| `backlog.md` | Future-work backlog |
| `coding-standards.md` | Coding standards (incl. Multi-Agent Instruction Sync, CREDITS rule) |
| `coding-workflow-reliability.md` | Self-improve reliability rules |
| `completion-log.md` | Permanent record of all done chunks (10k-line cap, then archived) |
| `llm-decision-rules.md` | LLM decision policy |
| `local-first-brain.md` | Local-first brain principles |
| `milestones.md` | Active work queue (only `not-started` / `in-progress` rows) |
| `prompting-rules.md` | Prompt-shape rules + enforcement rules |
| `quality-pillars.md` | Quality pillars |
| `reality-filter.md` | "No pretend code" guard |
| `research-reverse-engineering.md` | Reverse-engineering research log |
| `ui-ux-standards.md` | UI/UX standards (design tokens, accessibility) |

### Non-negotiable rules agents must retrieve from MCP before editing

- `milestones.md` is the active queue only: keep only `not-started` /
  `in-progress` rows, archive completed chunks to `completion-log.md`,
  remove done rows, drop empty phase headings, and update `Next Chunk`.
- `prompting-rules.md` forbids starting from `backlog.md`: if no milestone
  chunks remain, stop and ask the user which backlog items to promote.
- `prompting-rules.md` requires coding/self-improve prompts to load
  `rules/*.md`, `instructions/*.md`, and `docs/*.md` through the shared
  `CodingPrompt` / `run_coding_task` path.
- `coding-standards.md` makes `.github/copilot-instructions.md` canonical and
  requires `AGENTS.md`, `CLAUDE.md`, and `.cursorrules` quick references to
  stay aligned when instructions change.
- `architecture-rules.md` requires brain changes to update both
  `docs/brain-advanced-design.md` and `README.md`; persona changes update both
  `docs/persona-design.md` and `README.md`.
- `coding-standards.md` requires `CREDITS.md` updates for external influences
  (code, docs, roadmap, prompt shapes, feature catalogues, or rejected
  decisions), and neutral TerranSoul names for runtime identifiers.
- `reality-filter.md` / `coding-standards.md` prohibit pretend/TODO/mock
  production code. Implement fully with tests or track it as a chunk and hide
  it from user-reachable paths.
- `llm-decision-rules.md` requires AI-behaviour decisions to route through the
  configured brain classifier/command with a user toggle; no regex/includes
  keyword routing except documented parsing/fallback exceptions.
- Always validate changed areas with existing lint/build/tests and record
  blockers honestly; do not claim unrun checks passed.

## Validation commands (CI gate)

```
npm ci
npm run lint            # eslint, 0 errors required
npx vue-tsc --noEmit    # TypeScript typecheck
npx vitest run          # frontend unit tests (~1600+ tests)
npm run build           # vite + vue-tsc production build
( cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test --all-targets )
```

On Linux, install Tauri system deps before Rust checks:
`libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev pkg-config libglib2.0-dev libssl-dev`.

## MCP server contract

- Ports: `7421` (release app), `7422` (dev app), `7423` (headless `npm run mcp`).
- Bearer-token auth (token written to `mcp-data/mcp-token.txt` and `.vscode/.mcp-token`).
- Endpoints: `GET /health`, `GET /status`, `POST /mcp` (JSON-RPC 2.0).
- Tools: `brain_health`, `brain_search`, `brain_get_entry`, `brain_list_recent`, `brain_kg_neighbors`, `brain_summarize`, `brain_suggest_context`, `brain_ingest_url`, `code_query`, `code_context`, `code_impact`, `code_rename`, GitNexus tools.
- Seed: on first run with no `memory.db`, applies `mcp-data/shared/memory-seed.sql`.
- Configs: `mcp-data/shared/brain_config.json` and `mcp-data/shared/app_settings.json` are copied to the runtime root if missing.
