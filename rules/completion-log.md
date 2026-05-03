# TerranSoul — Completion Log

> This file is the permanent record of all completed chunks.
> `rules/milestones.md` contains only chunks that are `not-started` or `in-progress`.
> When a chunk is done, its full details are recorded here and the row is removed from milestones.md.
>
> **📏 File size cap — 10,000 lines max. Always contains the latest history.**
> When the next append would push this file over 10,000 lines, the **oldest**
> entries are moved out into a dated archive file named
> `completion-log-{YYYY-MM-DD}.md` (the date is the archive date — the day
> the rotation is performed). This file (`completion-log.md`) is never
> renamed — its filename is stable, so external links keep working, and it
> always contains the newest history. Full procedure in
> [`rules/prompting-rules.md` → "ENFORCEMENT RULE — Completion-Log File Size Cap"](prompting-rules.md).

---

## Table of Contents

Entries are in **reverse chronological order** (newest first).

| Entry | Date |
|-------|------|
| [Chunk 28.14 — Path-scoped workflow context loading](#chunk-2814--path-scoped-workflow-context-loading) | 2026-05-03 |
| [Chunk 28.13 — Temporary-worktree coding execution](#chunk-2813--temporary-worktree-coding-execution) | 2026-05-03 |
| [Chunk 27.4b — Motion reconstruction backend seam](#chunk-274b--motion-reconstruction-backend-seam) | 2026-05-03 |
| [Chunk 27.5c — Learned-motion polish preview UI](#chunk-275c--learned-motion-polish-preview-ui) | 2026-05-03 |
| [Chunk 27.5b — Native learned-motion polish preview command](#chunk-275b--native-learned-motion-polish-preview-command) | 2026-05-02 |
| [Milestones audit — Doc chunk coverage reconciliation](#milestones-audit--doc-chunk-coverage-reconciliation) | 2026-05-02 |
| [Chunk 22.8 — OpenClaw bridge plugin alignment](#chunk-228--openclaw-bridge-plugin-alignment) | 2026-05-02 |
| [Chunk 19.1 — Canonical memory schema collapse](#chunk-191--canonical-memory-schema-collapse) | 2026-05-02 |
| [Chunk 14.16g — MotionGPT / T2M-GPT inference evaluation](#chunk-1416g--motiongpt--t2m-gpt-inference-evaluation) | 2026-05-02 |
| [Chunk 27.6 — Neural audio-to-face upgrade evaluation](#chunk-276--neural-audio-to-face-upgrade-evaluation) | 2026-05-02 |
| [Chunk 27.5 — Offline recorded-motion polish pass](#chunk-275--offline-recorded-motion-polish-pass) | 2026-05-02 |
| [Chunk 27.4 — MoMask-style full-body reconstruction research spike](#chunk-274--momask-style-full-body-reconstruction-research-spike) | 2026-05-02 |
| [Chunk 24.11 — Local push notification on long-running task completion](#chunk-2411--local-push-notification-on-long-running-task-completion) | 2026-05-02 |
| [Chunk 24.10 — Remote command tools + workflow progress narration](#chunk-2410--remote-command-tools--workflow-progress-narration) | 2026-05-02 |
| [Chunk 24.9 — Mobile chat view streaming through RemoteHost](#chunk-249--mobile-chat-view-streaming-through-remotehost) | 2026-05-02 |
| [Chunk 24.8 — gRPC-Web client + transport adapter](#chunk-248--grpc-web-client--transport-adapter) | 2026-05-02 |
| [Chunk 24.7 — iOS pairing UX](#chunk-247--ios-pairing-ux) | 2026-05-02 |
| [Chunk 24.6 — Tauri iOS target + shared frontend](#chunk-246--tauri-ios-target--shared-frontend) | 2026-05-02 |
| [Chunk 22.7 — Plugin command execution dispatch](#chunk-227--plugin-command-execution-dispatch) | 2026-05-02 |
| [Chunk 22.5 — Memory-hook contribution pipeline](#chunk-225--memory-hook-contribution-pipeline) | 2026-05-02 |
| [Chunk 16.3b — Late chunking ingest integration](#chunk-163b--late-chunking-ingest-integration) | 2026-05-02 |
| [Chunk 17.5b — Cross-device memory sync Soul Link wire protocol](#chunk-175b--cross-device-memory-sync-soul-link-wire-protocol) | 2026-05-02 |
| [Chunk 28.12 — Multi-agent coding DAG orchestration wiring](#chunk-2812--multi-agent-coding-dag-orchestration-wiring) | 2026-05-02 |
| [Chunk 28.11 — Apply/review/test execution gate](#chunk-2811--applyreviewtest-execution-gate) | 2026-05-02 |
| [Chunk 17.5a — CRDT sync schema + LWW core](#chunk-175a--crdt-sync-schema--lww-core) | 2026-05-02 |
| [Chunk 16.6 — GraphRAG community summaries](#chunk-166--graphrag-community-summaries) | 2026-05-02 |
| [Chunk 15.7 — VS Code Copilot incremental-indexing QA](#chunk-157--vs-code-copilot-incremental-indexing-qa) | 2026-05-02 |
| [Chunk 17.7 — Bidirectional Obsidian sync](#chunk-177--bidirectional-obsidian-sync) | 2026-05-02 |
| [Chunk 24.4 — Phone-control RPC surface](#chunk-244--phone-control-rpc-surface) | 2026-05-02 |
| [Chunk 24.3 — LAN gRPC activation + paired-device mTLS enforcement](#chunk-243--lan-grpc-activation--paired-device-mtls-enforcement) | 2026-05-02 |
| [Chunk 24.2b — mTLS pairing flow + persistent device registry](#chunk-242b--mtls-pairing-flow--persistent-device-registry) | 2026-05-02 |
| [Chunk 24.5b — VS Code / Copilot session probe FS wrapper](#chunk-245b--vs-code--copilot-session-probe-fs-wrapper) | 2026-05-02 |
| [Chunk 24.1b — LAN bind config + OS probe wrapper](#chunk-241b--lan-bind-config--os-probe-wrapper) | 2026-05-02 |
| [Chunk 20.1 — Dev/release data-root split (Docker namespacing)](#chunk-201--devrelease-data-root-split-docker-namespacing) | 2026-05-02 |
| [Chunk 16.5b — CRAG query-rewrite + web-search fallback](#chunk-165b--crag-query-rewrite--web-search-fallback) | 2026-05-02 |
| [Chunk 16.4b — Self-RAG orchestrator loop](#chunk-164b--self-rag-orchestrator-loop) | 2026-05-02 |
| [Chunk 27.8 — Persona pack schema spec document](#chunk-278--persona-pack-schema-spec-document) | 2026-05-02 |
| [Chunk 27.7 — Persona example-dialogue field](#chunk-277--persona-example-dialogue-field) | 2026-05-02 |
| [Chunk 15.8 — AI Coding Integrations doc finalisation](#chunk-158--ai-coding-integrations-doc-finalisation) | 2026-05-02 |
| [Chunk 27.3 — Blendshape passthrough — expanded ARKit rig](#chunk-273--blendshape-passthrough--expanded-arkit-rig) | 2026-05-02 |
| [Chunk 15.4 — AI Coding Integrations Control Panel](#chunk-154--ai-coding-integrations-control-panel) | 2026-05-02 |
| [Chunk 15.2 — gRPC `brain.v1` transport foundation](#chunk-152--grpc-brainv1-transport-foundation) | 2026-05-01 |
| [Chunk 14.16f — Pack-import provenance markers](#chunk-1416f--pack-import-provenance-markers) | 2026-05-02 |
| [Chunk 14.16e — Self-improve motion-feedback loop](#chunk-1416e--self-improve-motion-feedback-loop) | 2026-05-02 |
| [Chunk 14.16d — Emotion-reactive procedural pose bias](#chunk-1416d--emotion-reactive-procedural-pose-bias) | 2026-05-02 |
| [Chunk 14.16c2 + c3 — `generate_motion_from_text` command + Persona-panel UI](#chunk-1416c2--c3--generate_motion_from_text-command--persona-panel-ui) | 2026-05-02 |
| [Chunk 14.16c1 — Motion clip parser/validator foundation](#chunk-1416c1--motion-clip-parservalidator-foundation) | 2026-05-02 |
| [Chunk 14.16b3 — Frontend `PoseAnimator` + `llm-pose` wiring](#chunk-1416b3--frontend-poseanimator--llm-pose-wiring) | 2026-05-02 |
| [Chunk 14.16b2 — `<pose>` tag in StreamTagParser + `llm-pose` event](#chunk-1416b2--pose-tag-in-streamtagparser--llm-pose-event) | 2026-05-02 |
| [Chunk 14.16b1 — Pose-frame parser foundation (LLM-as-Animator)](#chunk-1416b1--pose-frame-parser-foundation-llm-as-animator) | 2026-05-02 |
| [Chunk 16.6c — Wire query-intent classifier into hybrid_search_rrf](#chunk-166c--wire-query-intent-classifier-into-hybrid_search_rrf) | 2026-05-01 |
| [Chunk 14.16a — LLM-driven 3D animation research & taxonomy](#chunk-1416a--llm-driven-3d-animation-research--taxonomy) | 2026-05-01 |
| [Chunk 17.8 — V11 schema: category column + index](#chunk-178--v11-schema-category-column--index) | 2026-04-30 |
| [Chunk 17.7b — V10 schema: Obsidian sync metadata columns](#chunk-177b--v10-schema-obsidian-sync-metadata-columns) | 2026-04-30 |
| [Chunk 16.9b — Embedding-model fallback chain](#chunk-169b--embedding-model-fallback-chain) | 2026-04-30 |
| [Chunk 16.6b — Query-intent classifier for retrieval ranking](#chunk-166b--query-intent-classifier-for-retrieval-ranking) | 2026-04-30 |
| [Chunk 25.12 — Brain data migration & maintenance scheduler](#chunk-2512--brain-data-migration--maintenance-scheduler) | 2026-04-30 |
| [Chunk 25.11 — MCP server self-host & dynamic tool registry](#chunk-2511--mcp-server-self-host--dynamic-tool-registry) | 2026-04-30 |
| [Chunk 28.5 — GitHub PR flow (OAuth device + per-chunk PRs)](#chunk-285--github-pr-flow-oauth-device--per-chunk-prs) | 2026-04-30 |
| [Chunk 27.2 — Context engineering budget-aware assembly](#chunk-272--context-engineering-budget-aware-assembly) | 2026-04-30 |
| [Chunk 28.3 — Multi-agent DAG runner](#chunk-283--multi-agent-dag-runner) | 2026-04-30 |
| [Chunk 28.2 — Coding intent router](#chunk-282--coding-intent-router) | 2026-04-30 |
| [Chunk 14.15 — MotionGPT motion token codec](#chunk-1415--motiongpt-motion-token-codec) | 2026-04-30 |
| [Chunk 14.14 — Full-body retarget from BlazePose landmarks](#chunk-1414--full-body-retarget-from-blazepose-landmarks) | 2026-04-30 |
| [Chunk 14.13 — Offline motion-clip smoothing](#chunk-1413--offline-motion-clip-smoothing) | 2026-04-30 |
| [Chunks 21.1–21.4 — Doc & Completion-Log Hygiene bundle](#chunks-2114--doc--completion-log-hygiene-bundle) | 2026-04-30 |
| [Chunk 28.10 — Context budget manager for long coding sessions](#chunk-2810--context-budget-manager-for-long-coding-sessions) | 2026-04-30 |
| [Chunk 1.1 (Phase 12) — Brain Advanced Design QA screenshots](#chunk-11-phase-12--brain-advanced-design-qa-screenshots) | 2026-04-30 |
| [Chunk 28.1 — Reviewer sub-agent](#chunk-281--reviewer-sub-agent) | 2026-04-30 |
| [Chunk 27.1 — Agentic RAG retrieve-as-tool](#chunk-271--agentic-rag-retrieve-as-tool) | 2026-04-30 |
| [Chunk 25.10 — apply_file (LLM output writer)](#chunk-2510--apply_file-llm-output-writer) | 2026-04-30 |
| [Chunk 28.6 — Persistent SQLite task queue](#chunk-286--persistent-sqlite-task-queue) | 2026-04-30 |
| [Chunk 26.1 — Daily background maintenance scheduler (settings + idle guard)](#chunk-261--daily-background-maintenance-scheduler) | 2026-04-30 |
| [Chunk 28.7 — Real token usage capture from OpenAI-compat providers](#chunk-287--real-token-usage-capture) | 2026-04-30 |
| [Chunks 26.2 / 26.3 / 26.4 — Milestones bookkeeping reconciliation](#chunks-262263264--milestones-bookkeeping-reconciliation) | 2026-04-30 |
| [Chunk 28.9 — Coding workflow handoff persistence + Tauri wiring](#chunk-289--coding-workflow-handoff-persistence--tauri-wiring) | 2026-04-30 |
| [Chunk 28.8 — Coding workflow session handoff codec](#chunk-288--coding-workflow-session-handoff-codec) | 2026-04-30 |
| [Chunk 23.2b — Handoff system-prompt block consumer wiring](#chunk-232b--handoff-system-prompt-block-consumer-wiring) | 2026-04-29 |
| [Chunk 24.5a — VS Code / Copilot log parser (Phase 24)](#chunk-245a--vs-code--copilot-log-parser) | 2026-04-29 |
| [Chunk 24.2a — Pairing payload codec (Phase 24)](#chunk-242a--pairing-payload-codec) | 2026-04-29 |
| [Chunk 24.1a — Pure LAN address classifier (Phase 24 foundation)](#chunk-241a--pure-lan-address-classifier) | 2026-04-29 |
| [Chunk 23.2a — Handoff system-prompt block builder](#chunk-232a--handoff-system-prompt-block-builder) | 2026-04-29 |
| [Chunk 21.5/6/7 — Doc reality bundle (MCP tool names + persona renumber)](#chunk-21567--doc-reality-bundle) | 2026-04-29 |
| [Chunk 16.3a — Late chunking pooling utility](#chunk-163a--late-chunking-pooling-utility) | 2026-04-29 |
| [Chunk 16.5a — CRAG retrieval evaluator](#chunk-165a--crag-retrieval-evaluator) | 2026-04-29 |
| [Chunk 16.4a — Self-RAG reflection-token controller](#chunk-164a--self-rag-reflection-token-controller) | 2026-04-29 |
| [Chunk 16.8 — Matryoshka embeddings (two-stage vector search)](#chunk-168--matryoshka-embeddings-two-stage-vector-search) | 2026-04-29 |
| [Chunk 15.5 — Voice / chat intents (AI integrations)](#chunk-155--voice--chat-intents-ai-integrations) | 2026-04-29 |
| [Chunk 15.10 — VS Code workspace surfacing](#chunk-1510--vs-code-workspace-surfacing) | 2026-04-29 |
| [Chunk 15.9 — MCP stdio transport shim](#chunk-159--mcp-stdio-transport-shim) | 2026-04-29 |
| [Chunk 23.0 — Multi-agent resilience scaffold](#chunk-230--multi-agent-resilience-scaffold-per-agent-threads-workflow-resilience-agent-swap-context) | 2026-04-25 |
| [Chunk 23.0b — Stop & Stop-and-Send Controls](#chunk-230b--stop--stop-and-send-controls-taskcontrols) | 2026-04-25 |
| [Chunk 16.7 — Sleep-time consolidation](#chunk-167--sleep-time-consolidation) | 2026-04-25 |
| [Chunk 15.6 — Auto-setup writers for Copilot, Claude Desktop, Codex](#chunk-156--auto-setup-writers-for-copilot-claude-desktop-codex) | 2026-04-25 |
| [Chunks 10.1 / 10.2 / 10.3 — Copilot Autonomous Mode + Auto-Resume + Health Gate](#chunks-101--102--103--copilot-autonomous-mode--auto-resume--health-gate) | 2026-04-25 |
| [Chunk 15.1 — MCP server](#chunk-151--mcp-server) | 2026-04-25 |
| [Chunk 14.12 — Phoneme-aware viseme model](#chunk-1412--phoneme-aware-viseme-model) | 2026-04-25 |
| [Chunks 14.9 / 14.10 / 14.11 — Learned asset persistence + player + bundle](#chunks-149--1410--1411--learned-asset-persistence--player--bundle) | 2026-04-25 |
| [Chunk 14.5 — VRMA baking](#chunk-145--vrma-baking) | 2026-04-25 |
| [Chunk 14.4 — Motion-capture camera quest](#chunk-144--motion-capture-camera-quest) | 2026-04-25 |
| [Chunk 14.3 — Expressions-pack camera quest](#chunk-143--expressions-pack-camera-quest) | 2026-04-25 |
| [Chunk 16.10 — ANN index (usearch)](#chunk-1610--ann-index-usearch) | 2026-04-25 |
| [Chunk 17.6 — Edge conflict detection](#chunk-176--edge-conflict-detection) | 2026-04-26 |
| [Chunk 16.9 — Cloud embedding API for free / paid modes](#chunk-169--cloud-embedding-api-for-free--paid-modes) | 2026-04-26 |
| [Chunk 17.2 — Contradiction resolution (LLM picks winner)](#chunk-172--contradiction-resolution-llm-picks-winner) | 2026-04-26 |
| [Chunk 16.11 — Semantic chunking pipeline](#chunk-1611--semantic-chunking-pipeline) | 2026-04-26 |
| [Chunk 17.4 — Memory importance auto-adjustment](#chunk-174--memory-importance-auto-adjustment) | 2026-04-26 |
| [Chunk 16.12 — Memory versioning (V8 schema)](#chunk-1612--memory-versioning-v8-schema) | 2026-04-25 |
| [Chunk 16.2 — Contextual Retrieval (Anthropic 2024)](#chunk-162--contextual-retrieval-anthropic-2024) | 2026-04-25 |
| [Chunk 17.3 — Temporal reasoning queries](#chunk-173--temporal-reasoning-queries) | 2026-04-25 |
| [Chunk 18.5 — Obsidian vault export (one-way)](#chunk-185--obsidian-vault-export-one-way) | 2026-04-25 |
| [Chunk 18.3 — Category filters in Memory View](#chunk-183--category-filters-in-memory-view) | 2026-04-24 |
| [Chunk 18.1 — Auto-categorise via LLM on insert](#chunk-181--auto-categorise-via-llm-on-insert) | 2026-04-24 |
| [CI Fix — Embed cache test race condition](#ci-fix--embed-cache-test-race-condition) | 2026-04-24 |
| [Chunk 18.2 — Category-aware decay rates](#chunk-182--category-aware-decay-rates) | 2026-04-24 |
| [Chunk 18.4 — Tag-prefix convention vocabulary + audit (Phase 18 first chunk)](#chunk-184--tag-prefix-convention-vocabulary--audit) | 2026-04-24 |
| [Chunk 17.1 — Auto-promotion based on access patterns (Phase 17 first chunk)](#chunk-171--auto-promotion-based-on-access-patterns) | 2026-04-24 |
| [Chunk 16.1 — Relevance threshold for `[LONG-TERM MEMORY]` injection (Phase 16 first chunk)](#chunk-161--relevance-threshold-for-long-term-memory-injection) | 2026-04-24 |
| [Chunk 15.3 — `BrainGateway` trait + shared op surface (Phase 15 foundation)](#chunk-153--braingateway-trait--shared-op-surface) | 2026-04-24 |
| [Milestones audit — Phase 14.8–14.15 + Phase 16 + Phase 17 + Phase 18 added](#milestones-audit) | 2026-04-24 |
| [Commercial-Licence Audit & Cleanup (msedge-tts + @vercel/* removed)](#commercial-licence-audit--cleanup) | 2026-04-24 |
| [Chunk 14.6 — Audio-Prosody Persona Hints (Camera-Free)](#chunk-146--audio-prosody-persona-hints-camera-free) | 2026-04-24 |
| [Chunk 14.7 — Persona Pack Export / Import](#chunk-147--persona-pack-export--import) | 2026-04-24 |
| [Chunk 14.2 — Master-Echo Brain-Extraction Loop (Persona Suggestion)](#chunk-142--master-echo-brain-extraction-loop-persona-suggestion) | 2026-04-24 |
| [Chunk 14.1 — Persona MVP (PersonaTraits store + prompt injection + UI)](#chunk-141--persona-mvp-personatraits-store--prompt-injection--ui) | 2026-04-24 |
| [Chunk 2.4 — BrainView "Code knowledge" panel (Phase 13 Tier 4)](#chunk-24--brainview-code-knowledge-panel-phase-13-tier-4) | 2026-04-24 |
| [Chunk 2.3 — Knowledge-Graph Mirror (V7 `edge_source` column, Phase 13 Tier 3)](#chunk-23--knowledge-graph-mirror-v7-edge_source-column-phase-13-tier-3) | 2026-04-24 |
| [Repo Tooling — File-Size Quality Check (Rust 1000 / Vue 800 lines)](#repo-tooling--file-size-quality-check) | 2026-04-24 |
| [Chunk 2.2 — Code-RAG Fusion in `rerank_search_memories` (Phase 13 Tier 2)](#chunk-22--code-rag-fusion-in-rerank_search_memories-phase-13-tier-2) | 2026-04-24 |
| [Chunk 2.1 — GitNexus Sidecar Agent (Phase 13 Tier 1)](#chunk-21--gitnexus-sidecar-agent-phase-13-tier-1) | 2026-04-24 |
| [Chunk 1.11 — Temporal KG Edges (V6 schema)](#chunk-111--temporal-kg-edges-v6-schema) | 2026-04-24 |
| [Chunk 1.10 — Cross-encoder Reranker (LLM-as-judge)](#chunk-110--cross-encoder-reranker-llm-as-judge) | 2026-04-24 |
| [Chunk 1.9 — HyDE (Hypothetical Document Embeddings)](#chunk-19--hyde-hypothetical-document-embeddings) | 2026-04-24 |
| [Chunk 1.8 — RRF Wired into Hybrid Search](#chunk-18--rrf-wired-into-hybrid-search) | 2026-04-24 |
| [Chunk 1.7 (Distribution) — Real Downloadable Agent Distribution](#chunk-17-distribution--real-downloadable-agent-distribution) | 2026-04-23 |
| [Chunk 1.7 — Cognitive Memory Axes + Marketplace Catalog Default + Local Models as Agents + OpenClaw Bridge](#chunk-17--cognitive-memory-axes--marketplace-catalog-default--local-models-as-agents--openclaw-bridge) | 2026-04-23 |
| [Chunk 1.6 — Entity-Relationship Graph (V5 schema, typed/directional edges, multi-hop RAG)](#chunk-16--entity-relationship-graph-v5-schema-typeddirectional-edges-multi-hop-rag) | 2026-04-23 |
| [Chunk 1.5 — Multi-Agent Roster + External CLI Workers + Temporal-style Durable Workflows](#chunk-15--multi-agent-roster--external-cli-workers--temporal-style-durable-workflows) | 2026-04-23 |
| [Chunk 1.4 — Podman + Docker Desktop Dual Container Runtime](#chunk-14--podman--docker-desktop-dual-container-runtime) | 2026-04-23 |
| [Chunk 1.2 — Mac & Linux CI Matrix + Platform Docs](#chunk-12--mac--linux-ci-matrix--platform-docs) | 2026-04-23 |
| [Chunk 1.3 — Per-User VRM Model Persistence + Remove GENSHIN Default](#chunk-13--per-user-vrm-model-persistence--remove-genshin-default) | 2026-04-23 |
| [Chunk 1.1 — Brain Advanced Design: Source Tracking Pipeline](#chunk-11--brain-advanced-design-source-tracking-pipeline) | 2026-04-22 |
| [Chunks 130–134 — Phase 11 Finale: RPG Brain Configuration](#chunks-130134--phase-11-finale-rpg-brain-configuration) | 2026-04-20 |
| [Chunk 128 — Constellation Skill Tree](#chunk-128--constellation-skill-tree-full-screen-layout) | 2026-04-20 |
| [Chunk 129 — Constellation Cluster Interaction & Detail Panel](#chunk-129--constellation-cluster-interaction--detail-panel) | 2026-04-20 |
| [Post-Phase — 3D Model Loading Robustness](#post-phase--3d-model-loading-robustness) | 2026-04-18 |
| [Post-Phase — Streaming Timeout Fix](#post-phase--streaming-timeout-fix-stuck-thinking) | 2026-04-18 |
| [Post-Phase — Music Bar Redesign](#post-phase--music-bar-redesign-always-visible-playstop) | 2026-04-18 |
| [Post-Phase — Splash Screen](#post-phase--splash-screen) | 2026-04-18 |
| [Post-Phase — BGM Track Replacement](#post-phase--bgm-track-replacement-jrpg-style) | 2026-04-18 |
| [Chunk 126 — On-demand Rendering + Idle Optimization](#chunk-126--on-demand-rendering--idle-optimization) | 2026-04-18 |
| [Chunk 125 — LipSync ↔ TTS Audio Pipeline](#chunk-125--lipsync--tts-audio-pipeline) | 2026-04-18 |
| [Chunk 124 — Decouple IPC from Animation](#chunk-124--decouple-ipc-from-animation--coarse-state-bridge) | 2026-04-18 |
| [Chunk 123 — Audio Analysis Web Worker](#chunk-123--audio-analysis-web-worker) | 2026-04-17 |
| [Chunk 122 — 5-Channel VRM Viseme Lip Sync](#chunk-122--5-channel-vrm-viseme-lip-sync) | 2026-04-17 |
| [Chunk 121 — Exponential Damping Render Loop](#chunk-121--exponential-damping-render-loop) | 2026-04-17 |
| [Chunk 120 — AvatarState Model + Animation State Machine](#chunk-120--avatarstate-model--animation-state-machine) | 2026-04-17 |
| [Chunk 110 — Background Music](#chunk-110--background-music) | 2026-04-15 |
| [Chunk 109 — Idle Action Sequences](#chunk-109--idle-action-sequences) | 2026-04-15 |
| [Chunk 108 — Settings Persistence + Env Overrides](#chunk-108--settings-persistence--env-overrides) | 2026-04-15 |
| [Chunk 107 — Multi-ASR Provider Abstraction](#chunk-107--multi-asr-provider-abstraction) | 2026-04-15 |
| [Chunk 106 — Streaming TTS](#chunk-106--streaming-tts) | 2026-04-15 |
| [Chunk 085 — UI/UX Overhaul](#chunk-085--uiux-overhaul-open-llm-vtuber-layout-patterns) | 2026-04-14 |
| [Phase 8 Summary (Chunks 080–084)](#phase-8-summary) | 2026-04-14 |
| [Chunk 084 — Autoregressive Pose Feedback](#chunk-084--autoregressive-pose-feedback-done) | 2026-04-14 |
| [Chunk 083 — Gesture Tag System](#chunk-083--gesture-tag-system-done) | 2026-04-14 |
| [Chunk 082 — LLM Pose Prompt Engineering](#chunk-082--llm-pose-prompt-engineering-done) | 2026-04-14 |
| [Chunk 081 — Pose Blending Engine](#chunk-081--pose-blending-engine-done) | 2026-04-14 |
| [Chunk 080 — Pose Preset Library](#chunk-080--pose-preset-library-done) | 2026-04-14 |
| [Chunk 068 — Navigation Polish](#chunk-068--navigation-polish--micro-interactions-done) | 2026-04-14 |
| [Chunk 067 — Enhanced Chat UX](#chunk-067--enhanced-chat-ux-done) | 2026-04-14 |
| [Chunk 066 — New Background Art](#chunk-066--new-background-art-done) | 2026-04-14 |
| [Chunk 065 — Design System & Global CSS Variables](#chunk-065--design-system--global-css-variables-done) | 2026-04-14 |
| [Chunk 064 — Desktop Pet Overlay](#chunk-064--desktop-pet-overlay-with-floating-chat-done) | 2026-04-13 |
| [Chunk 063 — Rewrite Voice in Rust](#chunk-063--remove-open-llm-vtuber--rewrite-voice-in-rust-done) | 2026-04-13 |
| [Chunk 062 — Voice Activity Detection](#chunk-062--voice-activity-detection) | 2026-04-13 |
| [Chunk 061 — Web Audio Lip Sync](#chunk-061--web-audio-lip-sync) | 2026-04-13 |
| [Chunk 060 — Voice Abstraction Layer](#chunk-060--voice-abstraction-layer--open-llm-vtuber-integration) | 2026-04-13 |
| [Chunk 059 — Provider Health Check & Rate-Limit Rotation](#chunk-059--provider-health-check--rate-limit-rotation) | 2026-04-13 |
| [Chunk 058 — Emotion Expansion & UI Fixes](#chunk-058--emotion-expansion--ui-fixes) | 2026-04-13 |
| [Chunk 056+057 — Streaming BrainMode Routing](#chunk-056057--streaming-brainmode-routing-auto-selection--wizard-redesign) | 2026-04-13 |
| [Chunk 055 — Free LLM API Provider Registry](#chunk-055--free-llm-api-provider-registry--openai-compatible-client) | 2026-04-13 |
| [Chunk 054 — Emotion Tags in LLM Responses](#chunk-054--emotion-tags-in-llm-responses) | 2026-04-13 |
| [Chunk 053 — Streaming LLM Responses](#chunk-053--streaming-llm-responses) | 2026-04-13 |
| [Chunk 052 — Multi-Monitor Pet Mode](#chunk-052--multi-monitor-pet-mode) | 2026-04-13 |
| [Chunk 051 — Selective Click-Through](#chunk-051--selective-click-through) | 2026-04-13 |
| [Chunk 050 — Window Mode System](#chunk-050--window-mode-system) | 2026-04-13 |
| [Chunk 035 — Agent-to-Agent Messaging](#chunk-035--agent-to-agent-messaging) | 2026-04-13 |
| [Chunk 034 — Agent Marketplace UI](#chunk-034--agent-marketplace-ui) | 2026-04-13 |
| [Chunk 033 — Agent Sandboxing](#chunk-033--agent-sandboxing) | 2026-04-13 |
| [Chunk 032 — Agent Registry](#chunk-032--agent-registry) | 2026-04-13 |
| [Chunk 041 — Long/Short-term Memory](#chunk-041--longshort-term-memory--brain-powered-recall) | 2026-04-12 |
| [Chunk 040 — Brain (Local LLM via Ollama)](#chunk-040--brain-local-llm-via-ollama) | 2026-04-12 |
| [Chunk 031 — Install / Update / Remove Commands](#chunk-031--install--update--remove-commands) | 2026-04-11 |
| [Chunk 030 — Package Manifest Format](#chunk-030--package-manifest-format) | 2026-04-11 |
| [Chunk 023 — Remote Command Routing](#chunk-023--remote-command-routing) | 2026-04-10 |
| [Chunk 022 — CRDT Sync Engine](#chunk-022--crdt-sync-engine) | 2026-04-10 |
| [Chunk 021 — Link Transport Layer](#chunk-021--link-transport-layer) | 2026-04-10 |
| [Chunk 020 — Device Identity & Pairing](#chunk-020--device-identity--pairing) | 2026-04-10 |
| [Chunk 009 — Playwright E2E Test Infrastructure](#chunk-009--playwright-e2e-test-infrastructure) | 2026-04-10 |
| [Chunk 008 — Tauri IPC Bridge Integration Tests](#chunk-008--tauri-ipc-bridge-integration-tests) | 2026-04-10 |
| [Chunk 011 — VRM Import + Character Selection UI](#chunk-011--vrm-import--character-selection-ui) | 2026-04-10 |
| [Chunk 010 — Character Reactions — Full Integration](#chunk-010--character-reactions--full-integration) | 2026-04-10 |
| [Chunk 007 — Agent Orchestrator Hardening](#chunk-007--agent-orchestrator-hardening) | 2026-04-10 |
| [Chunk 006 — Rust Chat Commands — Unit Tests](#chunk-006--rust-chat-commands--unit-tests) | 2026-04-10 |
| [Chunk 005 — Character State Machine Tests](#chunk-005--character-state-machine-tests) | 2026-04-10 |
| [Chunk 004 — VRM Model Loading & Fallback](#chunk-004--vrm-model-loading--fallback) | 2026-04-10 |
| [Chunk 003 — Three.js Scene Polish + WebGPU Detection](#chunk-003--threejs-scene-polish--webgpu-detection) | 2026-04-10 |
| [Chunk 002 — Chat UI Polish & Vitest Component Tests](#chunk-002--chat-ui-polish--vitest-component-tests) | 2026-04-10 |
| [CI Restructure](#ci-restructure--consolidate-jobs--eliminate-double-firing) | 2026-04-10 |
| [Chunk 001 — Project Scaffold](#chunk-001--project-scaffold) | 2026-04-10 |

---

## Chunk 28.14 — Path-scoped workflow context loading

**Status:** Complete
**Date:** 2026-05-03

### Summary

Added path-scoped workflow context loading for coding prompts. Markdown rules/docs can now declare `applyTo` frontmatter, reusable coding tasks can pass target paths, and the self-improve coder prompt derives likely repo path hints from the approved plan so large repos avoid injecting unrelated scoped guidance into every implementation prompt.

### What changed

- Added direct `glob` dependency for path pattern matching using the already-locked `glob 0.3.3` package.
- Added `CodingTask.target_paths` so callers can request context relevant to expected touched files.
- Added `workflow::load_workflow_context_for_paths`, preserving legacy loading when no target paths are supplied while filtering `applyTo`-scoped markdown files when paths are present.
- Supported YAML-style frontmatter patterns such as `applyTo: src-tauri/**`, list syntax, and inline arrays.
- Updated self-improve coder prompt context loading to derive likely repo path hints from the approved plan before loading scoped docs/rules.
- Updated `docs/coding-workflow-design.md` and `rules/research-reverse-engineering.md` to mark the final Cursor/Claude Code follow-up shipped.
- Removed the completed 28.14 row from `rules/milestones.md`; no active chunks remain.

### Validation

- `cargo test coding::workflow` - 17 passed.
- `cargo test extract_plan_path_hints_finds_repo_paths` - 1 passed.
- `cargo clippy -- -D warnings` - passed.

### Notes

- Files without `applyTo` remain global, so existing project-wide instructions still load as before.
- Empty `target_paths` preserves the previous preview/planner behavior and loads all configured context files.

---

## Chunk 28.13 — Temporary-worktree coding execution

**Status:** Complete
**Date:** 2026-05-03

### Summary

Added temporary git-worktree isolation for dirty-checkout self-improve execution. When the user's active checkout is dirty, the autonomous DAG now runs apply/test/stage in a detached temporary worktree, captures the staged binary diff as a review patch, and cleans up the worktree so generated edits never mix with the user's uncommitted changes.

### What changed

- Added `src-tauri/src/coding/worktree.rs`, a small git-worktree utility that creates detached temporary worktrees from `HEAD`, captures cached binary diffs, and removes/prunes the worktree on cleanup/drop.
- Wired `coding::engine::execute_chunk_dag` through `prepare_execution_workspace`, keeping clean-checkout behavior unchanged while routing dirty-checkout execution through an isolated worktree root.
- Saved successful isolated patches under `target/terransoul-self-improve/patches/{chunk}-isolated.patch` for review instead of applying them into the dirty checkout.
- Updated self-improve progress messaging so isolated runs report the patch artifact path.
- Updated `docs/coding-workflow-design.md`, `rules/research-reverse-engineering.md`, and `rules/coding-workflow-reliability.md` to mark Chunk 28.13 shipped and keep the remaining follow-up focused on 28.14.
- Removed the completed 28.13 row from `rules/milestones.md`; the next active chunk is 28.14.

### Validation

- `cargo test coding::worktree` - 2 passed.
- `cargo test coding::engine::tests` - 15 passed.
- `cargo clippy -- -D warnings` - passed.

### Notes

- Clean worktrees still use the existing review/apply/test/stage path and stage green changes in the active checkout.
- Dirty worktrees are isolated by default: generated edits and test side effects stay in the temporary worktree, while the active checkout receives only an ignored patch artifact under `target/`.

---

## Chunk 27.4b — Motion reconstruction backend seam

**Status:** Complete
**Date:** 2026-05-03

### Summary

Introduced the first saved-landmark motion reconstruction seam without changing live camera behavior. The new feature-gated backend boundary exposes a bundled `geometric` backend that wraps the existing Rust full-body retargeter, plus deterministic static saved-landmark fixtures for future MotionBERT/MMPose sidecar comparisons.

### What changed

- Added `src-tauri/src/persona/motion_reconstruction.rs` behind the existing `motion-research` feature.
- Added `MotionReconstructionBackend`, `MotionReconstructionBackendId`, `MotionReconstructionConfig`, `SavedLandmarkFrame`, and result/frame metadata types.
- Implemented `GeometricMotionReconstructionBackend`, which routes saved landmark frames through `retarget_pose` and reports per-frame pose confidence from 17-bone completeness.
- Added backend metadata with `accepts_live_camera: false`, `requires_sidecar: false`, and bundled `geometric` availability.
- Added static synthetic saved-landmark fixtures for T-pose and raised-arm clips so future adapters can test against stable non-camera inputs.
- Updated `docs/momask-full-body-retarget-research.md` and `docs/persona-design.md` to mark the 27.4b seam shipped.
- Removed the completed 27.4b row from `rules/milestones.md`; the next active chunk is 28.13.

### Validation

- `cargo test --features motion-research motion_reconstruction` - 4 passed.
- `cargo test --features motion-research persona::retarget` - 12 passed.
- `cargo clippy --features motion-research -- -D warnings` - passed.
- `cargo clippy -- -D warnings` - passed.

### Notes

- The default build path remains unchanged because both the existing retargeter and the new reconstruction seam are feature-gated behind `motion-research`.
- The fixture clip is synthetic and contains no raw camera frames; it exists only to exercise the saved-landmark interface and preserve privacy invariants.

---

## Chunk 27.5c — Learned-motion polish preview UI

**Status:** Complete
**Date:** 2026-05-03

### Summary

Shipped the Persona-panel UI for non-destructive learned-motion polish previews. Users can choose an existing learned motion, generate a smoothed candidate with light/medium/heavy presets, toggle playback between the original and polished candidate, inspect displacement stats, then explicitly save the candidate as a new clip or discard it.

### What changed

- Added `src/components/PersonaMotionPolishPanel.vue`, a focused panel over the existing `persona.polishLearnedMotion` and `saveLearnedMotion` store APIs.
- Wired the panel into `src/components/PersonaPanel.vue` directly under the learned-motion library.
- Preserved the non-destructive contract by marking accepted candidates with `polish.acceptedByUser = true` and saving through the existing new-clip path rather than replacing the source motion.
- Added `src/components/PersonaMotionPolishPanel.test.ts` for preview generation, original/polished playback toggles, and Save as new clip behavior.
- Updated `docs/offline-motion-polish-research.md` and `docs/persona-design.md` so the design docs now reflect the shipped UI slice.
- Removed the completed 27.5c row from `rules/milestones.md`; the next active chunk is 27.4b.

### Validation

- `npm run lint` — 0 errors, 254 warnings.
- `npx vue-tsc --noEmit` — passed.
- `npm run build` — passed, with existing Vite chunking advisories.
- `npm run test` — 111 files / 1521 tests passed.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo test --all-targets` — 1981 library tests plus 4 smoke tests passed.
- `npm run test:e2e` — 5 Playwright specs passed.
- `npm run tauri:ios:check` — iOS config valid; Xcode project generation skipped on Windows as expected.
- VS Code diagnostics check on touched code files — no errors found.

### Notes

- The Playwright mobile flow expectation was refreshed for the current seven-tab mobile shell after the Link tab was added.
- Before 27.5c, CI cleanup also removed stale Rust warning/test blockers: unused Obsidian-sync test locals, plugin view max-line pressure via `usePluginCapabilityGrants`, and registry-server catalog tests expecting the former OpenClaw package entry.

---

## Chunk 27.5b — Native learned-motion polish preview command

**Status:** Complete
**Date:** 2026-05-02

### Summary

Exposed the native zero-phase Gaussian learned-motion smoother as a non-destructive backend preview command and added the frontend Pinia wrapper/types needed for the upcoming preview UI. The command returns a polished candidate motion plus displacement stats and never writes or overwrites saved clips; acceptance still flows through `save_learned_motion`.

### What changed

- Added `polish_learned_motion(id, config)` in `src-tauri/src/commands/persona.rs`, including light/medium/heavy presets, sigma/radius clamping warnings, applied-config reporting, candidate provenance metadata, and per-bone displacement stats.
- Registered the command in `src-tauri/src/lib.rs` and moved `persona::motion_smooth` into the default build so the production command compiles without `motion-research`.
- Fixed `src-tauri/src/persona/motion_smooth.rs` reflection padding for kernels wider than short clips, preventing unsigned underflow during heavy smoothing.
- Added `MotionPolish*` TypeScript types and optional polish metadata in `src/stores/persona-types.ts`.
- Added `persona.polishLearnedMotion(id, config)` in `src/stores/persona.ts`, mapping backend snake_case fields to frontend camelCase without mutating `learnedMotions`.
- Added focused frontend and backend tests for preview mapping, non-destructive candidate generation, clamped configs, non-motion rejection, and the short-clip reflection regression.
- Updated `docs/offline-motion-polish-research.md` and `docs/persona-design.md` to mark the backend command shipped while leaving visual controls as Chunk 27.5c.

### Validation

- `npx vitest run src/stores/persona.test.ts` — 28 passed.
- `cargo test motion_polish` — 3 passed.
- `cargo test convolve_handles_kernel_wider_than_channel` — 1 passed.

### Notes

- Cargo still reports three unrelated existing `unused variable: db_path` warnings in `src-tauri/src/memory/obsidian_sync.rs` tests.
- Next active chunk is 27.5c, the learned-motion polish preview UI.

---

## Milestones audit — Doc chunk coverage reconciliation

**Status:** Complete
**Date:** 2026-05-02

### Summary

Reconciled doc/rule chunk references against `rules/milestones.md` so the active tracker stays concise and only contains not-started or in-progress work. Completed or closed items below are intentionally not re-added to milestones.

### Backfilled and closed coverage

- **Chunk 28.4 — Sandboxed test runs.** `docs/coding-workflow-design.md` correctly describes this as shipped. Backfilled the missing completion-log coverage for `src-tauri/src/coding/test_runner.rs`: isolated `tokio::process` test suites, stripped environment, per-suite timeout, retry-once flaky detection, stdout/stderr tail capture, and `TestRunResult { suites, all_green, total_duration_ms, flaky_suites }` for the coding workflow gate.
- **Chunk 094 — Model Position Saving.** Verified current implementation via `src/composables/useModelCameraStore.ts`, `src/composables/useModelCameraStore.test.ts`, and the `get_model_camera_positions` / `save_model_camera_position` command surface.
- **Chunk 095 — Procedural Gesture Blending.** Verified current implementation via `src/renderer/gesture-blender.ts`, `src/renderer/gesture-blender.test.ts`, and `CharacterAnimator` integration.
- **Chunk 096 — Speaker Diarization.** Verified current implementation via the `DiarizationEngine` / `StubDiarization` Rust path, `diarize_audio`, and `src/composables/useDiarization.ts`.
- **Chunk 097 — Hotword-Boosted ASR.** Verified current implementation via hotword commands (`get_hotwords`, `add_hotword`, `remove_hotword`, `clear_hotwords`) and `src/composables/useHotwords.ts`.
- **Chunk 098 — Presence / Greeting System.** Verified current implementation via `src/composables/usePresenceDetector.ts` and its test coverage.
- **Chunk 115 — Live2D Support.** Closed as no-action rather than re-promoted: the current docs make VRM the only supported avatar format and reject Live2D for licensing/runtime fit.
- **Chunk 116 — Screen Recording / Vision.** Verified current implementation via `src-tauri/src/commands/vision.rs`, `capture_screen`, `analyze_screen`, `ScreenFrame`, `VisionAnalysis`, and `src/composables/useVisionCapture.ts`.
- **Chunk 117 — Docker Containerization.** Left demoted from active milestones; desktop Tauri runtime work should not depend on Docker unless explicitly promoted for CI/research.
- **Chunk 118 — Chat Log Export.** Verified current implementation via `export_chat_log` and `src/composables/useChatExport.ts`.
- **Chunk 119 — Language Translation Layer.** Verified current implementation via `translate_text`, `detect_language`, `list_languages`, and `src/composables/useTranslation.ts`.
- **Chunk 15.7 — VS Code Copilot incremental-indexing QA.** Corrected the stale `docs/AI-coding-integrations.md` roadmap row to match the existing shipped completion-log entry.

### Validation

- Documentation-only reconciliation. No runtime code changed for this audit entry.
- Active milestones remain limited to current open work: 27.5c, 27.4b, 28.13, and 28.14.

---

## Chunk 22.8 — OpenClaw bridge plugin alignment

**Status:** Complete
**Date:** 2026-05-02

### Summary

Aligned OpenClaw with the PluginHost model used by Translator Mode and updated
the docs so OpenClaw is presented as a capability-gated tool plugin, not an
Agent Marketplace conversation provider.

### What changed

- Verified `src-tauri/src/plugins/host.rs` registers `openclaw-bridge` as a
  built-in plugin with `openclaw-bridge.dispatch`, `read`, `fetch`, `chat`,
  `status`, and `/openclaw` contributions.
- Verified OpenClaw is no longer in the in-process Agent Marketplace catalog;
  catalog tests now assert the agent catalog through `stub-agent`,
  `claude-cowork`, and `gitnexus-sidecar`.
- Rewrote `instructions/OPENCLAW-EXAMPLE.md` as the canonical plugin example,
  including the TerranSoul/OpenClaw responsibility split, capability grants,
  JSON-RPC runtime seam, best-use guidance, and future chunk plan.
- Added OpenClaw as a second built-in plugin example in
  `docs/plugin-development.md` next to Translator Mode.
- Updated `README.md`, `instructions/EXTENDING.md`, and
  `rules/architecture-rules.md` so OpenClaw is described as a PluginHost tool
  bridge while `agent/openclaw_agent.rs` is legacy parser/provider support.

### Validation

- `npx vitest run src/stores/plugins.test.ts src/views/PluginsView.test.ts src/views/MarketplaceView.test.ts` — 43 passed.
- `cargo test openclaw` — 19 passed.
- `cargo test catalog_registry` — 7 passed.
- `npx vue-tsc --noEmit` — passed.
- `get_errors` on changed OpenClaw docs/code surfaces — no diagnostics.

### Notes

- Cargo emitted three existing `unused variable: db_path` warnings in
  `src-tauri/src/memory/obsidian_sync.rs`; they are unrelated to this chunk.
- No active row was added to `rules/milestones.md`, keeping the tracker concise
  and active-only.

---

## Chunk 19.1 — Canonical memory schema collapse

**Date:** 2026-05-02

**Summary:** Collapsed the SQLite memory migration history into one canonical V13 schema initializer. Fresh databases now create the final `memories`, `memory_edges`, `memory_versions`, `memory_conflicts`, `paired_devices`, and `sync_log` tables directly through `memory::schema::create_canonical_schema`. The old append-only `migrations.rs` runner and downgrade/round-trip migration tests were removed.

**Files changed:**
- `src-tauri/src/memory/schema.rs` — added canonical V13 schema SQL, schema-version recording, final-shape validation, and focused schema tests.
- `src-tauri/src/memory/migrations.rs` — deleted the versioned migration runner and historical migration tests.
- `src-tauri/src/memory/store.rs` — switched startup/in-memory setup to `create_canonical_schema` and updated schema-version assertions.
- `src-tauri/src/memory/mod.rs`, `src-tauri/src/commands/memory.rs`, `src-tauri/src/memory/versioning.rs`, `src-tauri/src/memory/edges.rs` — updated module exports, status reporting, and tests away from `migrations`.
- `docs/brain-advanced-design.md` and `README.md` — synced the brain/memory docs with canonical V13 schema creation instead of auto-applied migrations.
- `rules/milestones.md` — removed the final required 19.1 row and marked no tracked chunks remaining.

**Validation:**
- `cargo test memory::schema; cargo test schema_version_returns_latest; cargo test memory::versioning; cargo test canonical_schema_has_memory_edges_with_cascade` — passed.
- `cargo test brain::selection` — 9 passed.
- `cargo test memory:: -- --test-threads=1` — 435 passed.
- `cargo test -- --test-threads=1` — 1960 unit tests + 4 integration tests + 1 doc test passed.
- `cargo clippy -- -D warnings` — passed.
- `get_errors` on changed Rust/Markdown files — no diagnostics reported.

**Notes:**
- Test compilation still reports existing unused-variable warnings in `src-tauri/src/memory/obsidian_sync.rs` test code during `cargo test`; those warnings predate this chunk and did not fail the targeted test runs or clippy.

---

## Chunk 14.16g — MotionGPT / T2M-GPT inference evaluation

**Date:** 2026-05-02

**Summary:** Completed the optional GPU-gated MotionGPT / T2M-GPT inference evaluation without adding ONNX, CUDA, Python, SMPL, dataset, or model-weight dependencies. The local machine has a capable RTX 3080 Ti, but the upstream model boundary is not clean enough to ship as an in-app dependency. TerranSoul keeps the existing LLM motion generator and feature-gated deterministic `motion_tokens` codec as the product path; neural text-to-motion remains a future local sidecar only.

**Files changed:**
- `docs/motion-model-inference-evaluation.md` — added the detailed 14.16g evaluation, local GPU probe, candidate matrix, sidecar contract, acceptance gates, and rejected behaviors.
- `docs/persona-design.md` — added §7.6, updated §14.2 row 8, expanded sources, and clarified the roadmap row for MotionGPT / T2M-GPT.
- `docs/llm-animation-research.md` — corrected the MotionGPT / T2M-GPT runtime/license posture and added the 14.16g decision.
- `README.md` — documented `motion_tokens.rs` and the no-bundled-motion-model decision.
- `rules/milestones.md` — archived the completed Phase 14 optional work and advanced the pointer to final required chunk 19.1.

**Research outcome:**
- Local GPU probe found `nvidia-smi` available with NVIDIA GeForce RTX 3080 Ti / 12 GB VRAM; `nvcc` is not installed.
- MotionGPT code is MIT, but the public project depends on SMPL, SMPL-X, PyTorch3D, and datasets with separate licenses.
- T2M-GPT code is Apache-2.0, but the public project is a Python/PyTorch research stack using HumanML3D/KIT-style assets, pretrained download scripts, evaluator assets, and optional SMPL rendering.
- Adding Rust `ort` now would add native runtime risk without a verified, checksummed, VRM-native ONNX model artifact to run.
- A future sidecar must declare license, checksum, input/output schema, skeleton contract, and fallback behavior before it becomes user-facing.

**Validation:**
- `get_errors` on changed Markdown files — no diagnostics reported.
- No Rust/TypeScript runtime code changed for this evaluation-only optional chunk.

**Follow-ups (not in this chunk):**
- Future model integration should start from a concrete model manifest and artifact, not from a runtime dependency.
- Final required milestone is 19.1, the migration-history collapse, and it should remain last.

---

## Chunk 27.6 — Neural audio-to-face upgrade evaluation

**Date:** 2026-05-02

**Summary:** Completed the neural audio-to-face research and backend-boundary spike without adding runtime dependencies or model weights. The chunk compared the shipped `phoneme-viseme` + `lip-sync` stack against NVIDIA Audio2Face-3D / ACE, FaceFormer, and EmoTalk-class approaches. The decision is to keep TerranSoul's current model-free viseme scheduler as the default, with Audio2Face-3D only as a future optional local NVIDIA sidecar after hardware, license, checksum, and adapter gates pass.

**Files changed:**
- `docs/neural-audio-to-face-evaluation.md` — added the detailed evaluation, candidate matrix, optional backend contract, UX/safety requirements, acceptance gates, and rejected behaviors.
- `docs/persona-design.md` — added §7.5, updated §14.2 rows 9 and 11, expanded sources, and updated roadmap row 14.12.
- `docs/llm-animation-research.md` — added the 27.6 audio-to-face decision and sources.
- `README.md` — synced the Persona System overview with the default viseme path and optional Audio2Face-3D sidecar posture.
- `rules/milestones.md` — removed completed row 27.6 and noted that no non-last required chunk remains; optional 14.16g is GPU-gated and 19.1 remains the final required cleanup.

**Research outcome:**
- The shipped path is stronger than the old roadmap wording: `useLipSyncBridge` already prefers text-driven `VisemeScheduler` timelines when TTS text/duration are available, and falls back to Web Audio FFT/RMS analysis when needed.
- NVIDIA Audio2Face-3D is the only plausible future neural backend: the SDK is MIT, current model cards use the NVIDIA Open Model License, and outputs are facial pose/motion arrays. It still requires CUDA/TensorRT, NVIDIA hardware, model installation, and a model-specific adapter to TerranSoul's five-viseme / optional expanded-blendshape contracts.
- NVIDIA Audio2Emotion is not a default dependency because its license is gated and restricts use to the Audio2Face project; it must not be used as standalone emotion recognition.
- FaceFormer is MIT but research-mesh-oriented, tied to VOCASET/BIWI/FLAME-style vertex outputs and older Python/PyTorch runtime assumptions.
- EmoTalk is useful conceptually for emotional disentanglement, but no clear public maintained repository was found during this spike.

**Validation:**
- `get_errors` on changed Markdown files — no diagnostics reported.
- No code tests were required for this research-only chunk; no Rust/TypeScript runtime code changed.

**Follow-ups (not in this chunk):**
- Optional 14.16g can be evaluated only when a suitable GPU/model runtime is available.
- Final required chunk 19.1 should only start after confirming no schema-changing work remains.

---

## Chunk 27.5 — Offline recorded-motion polish pass

**Date:** 2026-05-02

**Summary:** Completed the offline recorded-motion polish research and workflow-design spike without adding runtime dependencies or model weights. The chunk evaluated HunyuanVideo / Hunyuan-Motion-class models, MimicMotion, MagicAnimate, Stable Video Diffusion, and the shipped TerranSoul Gaussian smoother. The recommendation is to expose the existing `motion_smooth` path first as a non-destructive saved-motion polish workflow, while keeping video diffusion systems as optional sidecar research only.

**Files changed:**
- `docs/offline-motion-polish-research.md` — added the detailed research deliverable, candidate matrix, non-destructive workflow, backend boundary, evaluation gate, and rejected behaviors.
- `docs/persona-design.md` — updated §7.4, §14.2, sources, and the roadmap row for Hunyuan / MimicMotion / MagicAnimate posture.
- `docs/llm-animation-research.md` — updated the Hunyuan row and added the 27.5 decision note plus sources.
- `README.md` — synced the Persona System overview with the existing `motion_smooth` baseline and the no-bundled-video-diffusion decision.
- `rules/milestones.md` — removed completed row 27.5 and advanced `Next Chunk` to 27.6.

**Research outcome:**
- `src-tauri/src/persona/motion_smooth.rs` already provides a license-clean, in-repo zero-phase Gaussian smoother over `LearnedMotion` frames with endpoint pinning and displacement stats. This should be the first product polish path.
- HunyuanVideo / Hunyuan-Motion-class models are useful research references but are not suitable as bundled polish dependencies because the open model stack is GPU-heavy, video-output-oriented, and governed by Tencent community/model license terms.
- MimicMotion has Apache-2.0 code but depends on Stable Video Diffusion weights, has 8-16 GB+ VRAM requirements, and outputs rendered video rather than reusable VRM bone frames.
- MagicAnimate has BSD-3-Clause code but depends on Stable Diffusion 1.5, a VAE, MagicAnimate checkpoints, CUDA, and ffmpeg; it also outputs rendered image animation rather than `LearnedMotion` frames.
- Stable Video Diffusion has gated access and a Stability AI community license with commercial thresholds, so it is not a default TerranSoul dependency.

**Validation:**
- `get_errors` on changed Markdown files — no diagnostics reported.
- No code tests were required for this research-only chunk; no Rust/TypeScript runtime code changed.

**Follow-ups (not in this chunk):**
- Chunk 27.6: evaluate neural audio-to-face upgrades against the shipped phoneme-aware viseme mapper.
- Future polish implementation: expose `motion_smooth::smooth_clip` through a non-destructive preview command/UI before considering any ML sidecar.

---

## Chunk 27.4 — MoMask-style full-body reconstruction research spike

**Date:** 2026-05-02

**Summary:** Completed the MoMask-style full-body retarget research spike without adding runtime dependencies or vendoring model weights. The spike evaluated MoMask, MotionBERT-Lite, MMPose / RTMPose3D, VideoPose3D, and the shipped TerranSoul geometric retarget baseline. The recommendation is to keep `src-tauri/src/persona/retarget.rs` as the default offline full-body baseline, avoid bundling MoMask for now, and make any future ML reconstruction an optional saved-landmark sidecar that never processes live camera frames.

**Files changed:**
- `docs/momask-full-body-retarget-research.md` — added the detailed research deliverable, candidate matrix, MoMask fit analysis, sidecar interface sketch, privacy constraints, and future acceptance gate.
- `docs/persona-design.md` — updated §7.2 / §7.3, §14.2, sources, and the roadmap row to reflect the 27.4 decision.
- `docs/llm-animation-research.md` — corrected the MoMask row, added MotionBERT-Lite, and linked the 27.4 decision.
- `README.md` — synced the Persona System overview with the current research posture and the feature-gated Rust retarget baseline.
- `rules/milestones.md` — removed completed row 27.4 and advanced `Next Chunk` to 27.5.

**Research outcome:**
- MoMask code is MIT and has a CPU WebUI path, but its app-facing outputs are 22-joint arrays / BVH and its temporal editing path expects HumanML3D 263D source features. Its README also flags separate licenses for SMPL, SMPL-X, PyTorch3D, and datasets. Treat it as a later offline inpainting/synthesis candidate, not a default BlazePose-to-VRM dependency.
- MotionBERT-Lite is the best first ML-lift candidate if a prototype is built: Apache-2.0 code, H36M 17-keypoint sequence input, 3D pose/mesh tasks, and a published small checkpoint. It still needs a BlazePose-to-H36M remap and sidecar/runtime audit.
- MMPose / RTMPose3D is a useful Apache-2.0 research harness but too heavy for the default Tauri app bundle.
- VideoPose3D is rejected for bundled use because the upstream license is CC BY-NC.

**Validation:**
- `get_errors` on changed Markdown files — no diagnostics reported.
- No code tests were required for this research-only chunk; no Rust/TypeScript runtime code changed.

**Follow-ups (not in this chunk):**
- Chunk 27.5: design the optional offline recorded-motion polish workflow for Hunyuan-Motion / MimicMotion / MagicAnimate-style references.
- If ML reconstruction is later implemented, add a model-agnostic `MotionReconstructionBackend` boundary and validate it against `persona::retarget` geometric fixtures before surfacing it in the UI.

---

## Chunk 24.11 — Local push notification on long-running task completion

**Date:** 2026-05-02

**Summary:** Added paired-mobile local notifications for long-running desktop work without APNS or a cloud push relay. The new `mobile-notifications.ts` watcher starts only for the iOS/remote runtime, polls the paired desktop through `RemoteHost.listWorkflowRuns(true)` and `RemoteHost.getCopilotSessionStatus()`, observes local `task-progress` events when available, and sends native notifications via `tauri-plugin-notification` once the configured threshold is met. Workflow and task notifications fire only after a previously observed run reaches a terminal state; Copilot sessions notify once when active work crosses the threshold.

**Files changed:**
- `src/stores/mobile-notifications.ts` — added the notification tracker, RemoteHost polling watcher, task-progress listener, threshold/poll clamping, native notification permission flow, and test adapters.
- `src/stores/mobile-notifications.test.ts` — covered workflow completion, short-run suppression, ingest-task completion, Copilot threshold notifications, and store polling.
- `src/App.vue` — started/stopped the mobile notification watcher from the main app lifecycle.
- `src/stores/settings.ts` — mirrored the new mobile notification settings in frontend defaults and types.
- `src-tauri/src/settings/mod.rs`, `src-tauri/src/commands/settings.rs`, and `src-tauri/src/settings/config_store.rs` — added persisted `mobile_notifications_enabled`, `mobile_notification_threshold_ms`, and `mobile_notification_poll_ms` defaults plus serde/default tests.
- `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`, and `src-tauri/capabilities/mobile.json` — registered `tauri-plugin-notification` in JS/Rust and granted the notification capability to desktop/mobile shells.
- `README.md` and `docs/brain-advanced-design.md` — documented the paired-mobile notification watcher and settings.
- `rules/milestones.md` — removed completed row 24.11 and advanced `Next Chunk` to 27.4.

**Validation:**
- `npx vitest run src/stores/mobile-notifications.test.ts` — 1 file / 5 tests passed.
- `npx vue-tsc --noEmit` — passed with no output.
- `cd src-tauri; cargo check && cargo clippy -- -D warnings` — passed.
- `cd src-tauri; cargo test settings::` — 36 settings/config-store tests passed; cargo printed 3 pre-existing unused-variable warnings in `memory/obsidian_sync.rs` test code.
- Full CI gate (`npx vitest run && npx vue-tsc --noEmit && cd src-tauri && cargo clippy -- -D warnings && cargo test`) — frontend tests, typecheck, and clippy passed; the final parallel `cargo test` stage failed on the unrelated `memory::obsidian_sync::tests::sync_creates_and_imports_roundtrip` assertion. That same test passed in isolation, and `cargo test -- --test-threads=1` passed 1970 lib tests, 4 smoke tests, and 1 doctest, confirming a parallel-sensitive existing test issue rather than a 24.11 regression.
- Mobile runtime probe on Windows: Android Emulator/AVD validation blocked because `adb`, `emulator`, `avdmanager`, and `sdkmanager` were not installed and `ANDROID_HOME` / `ANDROID_SDK_ROOT` were unset. iOS Simulator validation blocked because `xcrun` is unavailable on Windows; `npm run tauri:ios:check` passed iOS config validation and skipped Xcode-only checks on `win32`.

**Follow-ups (not in this chunk):**
- Run real Android Emulator and iOS Simulator/device LAN notification validation from hosts with the required SDKs installed.
- Expose the mobile notification threshold/toggle in a dedicated settings panel if users need a visible control beyond persisted `AppSettings`.

---

## Chunk 24.10 — Remote command tools + workflow progress narration

**Date:** 2026-05-02

**Summary:** Added the phone-side RemoteHost tool layer for the user's headline workflow: asking the phone what Copilot is doing on the desktop, asking for current workflow progress, and saying "continue the next chunk" from mobile chat. The new `remote-tools.ts` layer exposes `describe_copilot_session`, `describe_workflow_progress`, and `continue_workflow` as capability-gated tools over existing PhoneControl RPCs, and `remote-conversation.ts` now detects those prompts before falling back to streamed chat. New pairings now receive default phone capabilities, and mobile Stronghold credentials persist those capabilities so the chat store can enforce the phone's allowed actions.

**Files changed:**
- `src/transport/remote-tools.ts` — added remote tool definitions, intent detection, capability checks, workflow selection, and narration formatting.
- `src/transport/index.ts` — exported the remote tool surface from the transport barrel.
- `src/stores/remote-conversation.ts` — routed Copilot/workflow/continue prompts through the remote tool dispatcher before normal chat streaming and read saved pairing capabilities.
- `src/utils/secure-pairing-store.ts` — extended stored pairing credentials with optional capability metadata.
- `src/stores/mobile-pairing.ts` — saved paired-device capabilities into the Stronghold credential bundle during pairing confirmation.
- `src-tauri/src/network/pairing.rs` — added `DEFAULT_PHONE_CAPABILITIES` and assigned chat/read/workflow permissions to newly confirmed phone pairings.
- `src/transport/remote-tools.test.ts` and `src/stores/remote-conversation.test.ts` — added focused coverage for tool definitions, narration, capability denial, intent detection, and remote chat tool routing.
- `README.md` and `docs/brain-advanced-design.md` — documented the phone-side tool layer, supported tool names, pairing capabilities, and remote chat routing.
- `rules/milestones.md` — removed completed row 24.10 and advanced `Next Chunk` to 24.11.

**Validation:**
- `npx vitest run src/transport/remote-tools.test.ts src/stores/remote-conversation.test.ts` — 2 files / 11 tests passed.
- `npx vue-tsc --noEmit` — passed through the VS Code task with no reported output.
- `cargo test network::pairing` — 10 focused pairing tests passed; cargo printed 3 pre-existing unused-variable warnings in `memory/obsidian_sync.rs` test code.
- `cargo test phone_control` — 4 focused phone-control tests passed; cargo printed the same pre-existing unused-variable warnings.
- `cargo clippy -- -D warnings` — passed.
- `get_errors` on the 24.10 touched files — no diagnostics reported.
- Mobile runtime probe on Windows: Android Emulator/AVD validation blocked because `adb`, `emulator`, `avdmanager`, and `sdkmanager` were not installed and `ANDROID_HOME` / `ANDROID_SDK_ROOT` were unset. iOS Simulator validation blocked because `xcrun` is unavailable on Windows; `npm run tauri:ios:check` passed iOS config validation and skipped Xcode-only checks on `win32`.

**Follow-ups (not in this chunk):**
- Chunk 24.11: add local notification delivery for long-running task completion while a phone is paired and connected.
- Add server-side enforcement for the same phone capability names if the desktop RPC layer later exposes higher-risk workflow actions.
- Run real Android Emulator and iOS Simulator/device LAN gRPC-Web validation from hosts with the required SDKs installed.

---

## Chunk 24.9 — Mobile chat view streaming through RemoteHost

**Date:** 2026-05-02

**Summary:** Refit the mobile chat path so iOS uses a `remote-conversation.ts` store backed by `RemoteHost.streamChatMessage()` while desktop keeps the existing in-process conversation store. The phone-control proto now has `StreamChatMessage(ChatRequest) returns (stream ChatChunk)`, and the Rust service assembles the full desktop prompt server-side with `SYSTEM_PROMPT_FOR_STREAMING`, hybrid long-term memory injection, persona context, and one-shot handoff context before streaming clean text chunks to mobile. `ChatView.vue` now binds to a shared local/remote store surface, preserving agent filtering, queue/stop controls, subtitles, and mobile breakpoints.

**Files changed:**
- `src-tauri/proto/terransoul/phone_control.v1.proto` — added `ChatChunk` and the server-streaming `StreamChatMessage` RPC.
- `src-tauri/src/ai_integrations/grpc/phone_control.rs` — implemented desktop-side phone chat streaming, prompt assembly, memory/persona/handoff injection, clean text chunking, conversation persistence, and unary fallback parity.
- `src-tauri/src/commands/streaming.rs` — exposed `StreamTagParser` and `strip_anim_blocks` within the crate for the phone-control stream.
- `src/transport/phone_control_pb.ts`, `src/transport/remote-host.ts`, and `src/transport/grpc_web.ts` — added protobuf-es descriptors, `RemoteChatChunk`, and local/gRPC-Web `streamChatMessage()` implementations.
- `src/stores/remote-conversation.ts` — added the iOS remote chat Pinia store with streaming, unary fallback, queue/stop controls, agent filtering, and test adapters.
- `src/stores/chat-store-router.ts` and `src/utils/runtime-target.ts` — added runtime selection so iOS, or explicit test/query overrides, choose the remote store.
- `src/views/ChatView.vue` — switched to the store router, suppressed local brain setup chrome for remote chat, and skipped local Tauri streaming listeners on iOS remote mode.
- `src/utils/runtime-target.test.ts`, `src/stores/chat-store-router.test.ts`, `src/stores/remote-conversation.test.ts`, and `src/transport/grpc_web.test.ts` — added focused coverage for runtime detection, store routing, remote streaming, fallback behavior, and gRPC-Web chunk mapping.
- `README.md` and `docs/brain-advanced-design.md` — documented remote mobile chat streaming, server-side prompt injection, and the updated RemoteHost surface.
- `rules/milestones.md` — removed completed row 24.9 and advanced `Next Chunk` to 24.10.

**Validation:**
- `npx vitest run src/utils/runtime-target.test.ts src/stores/chat-store-router.test.ts src/stores/remote-conversation.test.ts src/transport/grpc_web.test.ts` — 4 files / 10 tests passed.
- `npx vue-tsc --noEmit` — passed.
- `cargo check` — passed.
- `cargo test phone_control` — 4 focused phone-control tests passed; cargo printed 3 pre-existing unused-variable warnings in `memory/obsidian_sync.rs` test code.
- `cargo clippy -- -D warnings` — passed.
- `Full CI Gate` task — passed through frontend tests, Vue type-check, Rust clippy, Rust unit tests (`1968 passed`), Ollama smoke tests (`4 passed`), and doctest (`1 passed`).
- Mobile runtime probe on Windows: Android Emulator/AVD validation blocked because `adb`, `emulator`, `sdkmanager`, and `avdmanager` were not installed, `ANDROID_HOME` / `ANDROID_SDK_ROOT` were unset, and `%LOCALAPPDATA%\Android\Sdk` did not exist. iOS Simulator validation blocked because `xcrun` / `xcodebuild` are unavailable on Windows; `npm run tauri:ios:check` passed config validation and skipped Xcode-only checks by design.

**Follow-ups (not in this chunk):**
- Chunk 24.10: add phone-side workflow/Copilot progress narration and the remote “continue next step” command surface.
- Run real Android Emulator and iOS Simulator/device LAN gRPC-Web validation from hosts with the required SDKs installed.

---

## Chunk 24.8 — gRPC-Web client + transport adapter

**Date:** 2026-05-02

**Summary:** Added the shared `RemoteHost` transport seam so Vue components can call the desktop brain either through local Tauri IPC or through browser-native gRPC-Web from an iOS WebView. The frontend now has Connect/protobuf descriptors for the Brain and PhoneControl RPC surfaces, a gRPC-Web adapter with unary and server-streaming memory search, and local IPC mapping for the same DTOs. The Rust gRPC server now enables `tonic_web::GrpcWebLayer` with HTTP/1 support, and pairing payloads advertise the LAN gRPC/phone-control port (`7422`) instead of the MCP port.

**Files changed:**
- `package.json` and `package-lock.json` — added `@bufbuild/connect`, `@bufbuild/connect-web`, and `@bufbuild/protobuf` for WebView gRPC-Web clients.
- `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock` — added `tonic-web`.
- `src-tauri/src/ai_integrations/grpc/mod.rs` — enabled gRPC-Web translation via `GrpcWebLayer` and HTTP/1 while preserving the existing Brain and PhoneControl services.
- `src-tauri/src/commands/lan.rs` — switched pairing URI generation to the gRPC server handle/port with a `7422` fallback.
- `src/transport/brain_pb.ts` and `src/transport/phone_control_pb.ts` — added protobuf-es descriptors for the Brain health/search/streaming surface and the Phase 24 phone-control RPCs.
- `src/transport/remote-host.ts`, `src/transport/grpc_web.ts`, and `src/transport/index.ts` — added the local-vs-remote host abstraction, local Tauri IPC adapter, gRPC-Web adapter, endpoint helpers, DTO mapping, and server-streaming search support.
- `src/stores/mobile-pairing.ts` — routed default trust-list loading through `RemoteHost` so the pairing store can use the same seam as upcoming mobile chat.
- `src/transport/grpc_web.test.ts` — added focused adapter tests for phone-control DTO mapping, streaming brain search, and endpoint helper validation.
- `README.md` and `docs/brain-advanced-design.md` — documented the RemoteHost/gRPC-Web brain transport and mobile memory-search surface.
- `rules/milestones.md` — removed completed row 24.8 and advanced `Next Chunk` to 24.9.

**Validation:**
- `npx vitest run src/transport/grpc_web.test.ts src/stores/mobile-pairing.test.ts src/utils/mobile-pairing.test.ts src/views/MobilePairingView.test.ts` — 4 files / 10 tests passed.
- `npx vue-tsc --noEmit` — passed.
- `cargo check` — passed after switching from the removed `tonic_web::enable` helper to `GrpcWebLayer`.
- `cargo clippy -- -D warnings` — passed.

**Follow-ups (not in this chunk):**
- Chunk 24.9: refit mobile chat to stream through `RemoteHost`, using the gRPC-Web adapter when running against a paired desktop.
- Generate protobuf descriptors in a build step once the frontend proto toolchain is standardised; the hand-written descriptors intentionally cover only the Phase 24 surfaces needed now.

---

## Chunk 24.7 — iOS pairing UX

**Date:** 2026-05-02

**Summary:** Added the iOS/mobile pairing UX for scanning `terransoul://pair` QR payloads, reviewing the desktop endpoint/fingerprint, confirming pairing through an adapter-ready workflow, and storing certificate bundles plus desktop trust metadata in the Stronghold-backed secure pairing store. The flow now detects saved desktop fingerprint mismatches and requires an explicit re-pair/trust action before overwriting credentials. Because this session ran on Windows, the QR camera path is scaffolded and unit-tested through adapters; physical iOS camera validation remains a macOS/device follow-up.

**Files changed:**
- `package.json` and `package-lock.json` — added `@tauri-apps/plugin-barcode-scanner`.
- `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/src/lib.rs` — added `tauri-plugin-barcode-scanner` and register it only under Tauri's `mobile` cfg, matching the crate's mobile-only Rust API.
- `src-tauri/Info.ios.plist` and `src-tauri/capabilities/mobile.json` — added the iOS camera usage string plus mobile barcode scanner permissions.
- `scripts/tauri-ios-check.mjs` — extended the iOS scaffold check to verify scanner registration, `NSCameraUsageDescription`, and mobile capability permissions.
- `src/utils/secure-pairing-store.ts` — extended the stored credential bundle with optional desktop host/port, desktop fingerprint, and pairing token metadata while preserving existing records.
- `src/utils/mobile-pairing.ts` and `src/utils/mobile-pairing.test.ts` — added the TypeScript mirror of the Rust pairing URI codec, scanner payload normalization, endpoint/fingerprint helpers, and focused tests.
- `src/stores/mobile-pairing.ts` and `src/stores/mobile-pairing.test.ts` — added the Pinia workflow for scan/manual URI review, secure-store unlock/save/remove, IPC-backed confirmation, trust-list loading, and fingerprint mismatch re-pair handling with injectable adapters for 24.8's remote transport.
- `src/views/MobilePairingView.vue`, `src/views/MobileSettingsView.vue`, and `src/views/MobilePairingView.test.ts` — added the mobile pairing screen, trust-list/settings panel, and component coverage for scan/review/confirm.
- `src/App.vue` — added the Link tab and panel-only route for the mobile pairing UX.
- `rules/milestones.md` — removed completed row 24.7 and advanced `Next Chunk` to 24.8.

**Validation:**
- `npx vitest run src/utils/mobile-pairing.test.ts src/stores/mobile-pairing.test.ts src/views/MobilePairingView.test.ts` — 3 files / 7 tests passed.
- `npm run tauri:ios:check` — passed; validates iOS config and scanner scaffold on Windows, then skips Xcode-only checks by design.
- `npx vue-tsc --noEmit` — passed.
- `Cargo Check + Clippy` task — passed after gating scanner registration to mobile targets.
- `Full CI Gate` task — passed: frontend tests (104 files, 1492 tests), Vue type-check, Rust clippy, Rust unit tests including Ollama smoke tests (4 passed), and doctest (1 passed). Cargo test still prints 3 pre-existing unused-variable warnings in `memory/obsidian_sync.rs`.

**Follow-ups (not in this chunk):**
- Chunk 24.8: replace the current local IPC confirmation seam with the `RemoteHost` / gRPC-Web transport adapter so the same pairing workflow can talk to a LAN desktop from the iOS WebView.
- Run real QR camera validation and signed iOS simulator/device checks from macOS with Xcode.

---

## Chunk 24.6 — Tauri iOS target + shared frontend

**Date:** 2026-05-02

**Summary:** Added the Tauri 2 iOS app-shell scaffold while keeping one shared Vue frontend. The repo now has an iOS config overlay, safe-area-aware mobile navigation, Stronghold-backed secure pairing credential storage, guarded macOS/Xcode init checks, and a macOS CI smoke job. Because this session ran on Windows, `tauri ios init` / Xcode project generation is intentionally guarded and documented instead of claimed as locally built.

**Files changed:**
- `src-tauri/tauri.ios.conf.json` — added the iOS platform-specific config overlay with minimum iOS version, opaque full-screen main window, disabled input accessory view, and disabled link previews.
- `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock` — added `tauri-plugin-stronghold` and the recommended dev profile override for `scrypt`.
- `package.json` and `package-lock.json` — added `@tauri-apps/plugin-stronghold` plus `tauri:ios:check` / `tauri:ios:init` scripts.
- `src-tauri/capabilities/default.json` and `src-tauri/src/lib.rs` — granted `stronghold:default` and registered the Stronghold plugin with an app-local Argon2 salt file.
- `src/utils/secure-pairing-store.ts` and `src/utils/secure-pairing-store.test.ts` — added a typed Stronghold wrapper for pairing certificate bundles with caller-supplied vault passwords and unit coverage using a fake loader.
- `index.html`, `src/style.css`, and `src/App.vue` — added `viewport-fit=cover` plus reusable safe-area tokens so mobile content and bottom navigation clear iOS notches/home indicators.
- `.github/workflows/terransoul-ci.yml` — added a macOS `ios-smoke` job that runs the guarded iOS scaffold checker.
- `scripts/tauri-ios-check.mjs` — added non-mutating config/tooling validation plus a macOS-only `--init` path for `npx tauri ios init`.
- `README.md` and `instructions/PLATFORM-SUPPORT.md` — documented the iOS scaffold, Stronghold storage, safe shared frontend, CI reality, and macOS signing/build requirements.
- `rules/milestones.md` — removed completed row 24.6 and advanced `Next Chunk` to 24.7.

**Validation:**
- `npm run tauri:ios:check` — passed; validated config on Windows and skipped Xcode-only checks by design.
- `npx vitest run src/utils/secure-pairing-store.test.ts` — 5 tests passed.
- `npx vue-tsc --noEmit` — passed after aligning the local Stronghold interface with the plugin's `remove` return type.
- `Cargo Check + Clippy` task — passed (`cargo check` + `cargo clippy -- -D warnings`).
- `Full CI Gate` task — passed: frontend tests (101 files, 1485 tests), Vue type-check, Rust clippy, Rust unit tests (1968 passed), Ollama smoke tests (4 passed), and doctest (1 passed). Cargo test still prints 3 pre-existing unused-variable warnings in `memory/obsidian_sync.rs`.

**Follow-ups (not in this chunk):**
- Chunk 24.7: build the iOS pairing UX, QR scan flow, and mobile trust-list settings on top of this shell.
- Run `npm run tauri:ios:init` and the first signed simulator/device build from a macOS machine with Xcode and `APPLE_DEVELOPMENT_TEAM` configured.
- Android remains a later mobile target using the same shared frontend/Rust core.

---

## Chunk 22.7 — Plugin command execution dispatch

**Date:** 2026-05-02

**Summary:** Replaced the plugin command echo stub with real command execution. Plugin commands now lazily activate on `OnCommand`, dispatch through built-in handlers, WASM `handle_command`, native binary argv, or sidecar stdin/stdout contracts, and enforce persisted sandbox capabilities before sensitive execution.

**Files changed:**
- `src-tauri/src/plugins/host.rs` — added `invoke_command_with_store` / `invoke_slash_command_with_store`, lazy `OnCommand` activation, capability snapshot checks, `ProcessSpawn` enforcement for binaries/sidecars, WASM command execution, stdout/stderr/exit-code capture, and runtime dispatch tests.
- `src-tauri/src/sandbox/wasm_runner.rs` — added `run_command_json` for the `handle_command(ptr, len) -> packed_ptr_len` ABI plus a compact WASM fixture test.
- `src-tauri/src/commands/plugins.rs` — routed Tauri command invocation through `AppState.capability_store` so frontend calls use the user's persisted grants.
- `docs/plugin-development.md` — documented command runtime contracts, argv/stdin payloads, WASM command ABI, lazy command activation, and capability mapping.
- `rules/milestones.md` — removed completed Phase 22 row 22.7 and the now-empty Phase 22 section.

**Validation:**
- `cargo test test_run_command_json_returns_output` — WASM command ABI test passed.
- `cargo test plugins::host::tests::invoke_` — 11 plugin command/slash dispatch tests passed, including WASM, binary, sidecar, capability denial, and lazy activation cases.
- `Cargo Check + Clippy` task — passed (`cargo check` + `cargo clippy -- -D warnings`).
- `Full CI Gate` task — first run hit a transient existing `memory::obsidian_sync::tests::sync_creates_and_imports_roundtrip` assertion; rerunning that single test passed. Second full run passed: frontend tests/typecheck, clippy, Rust unit tests (1968 passed), Ollama smoke tests (4 passed), and doctest (1 passed).

**Follow-ups (not in this chunk):**
- Surface binary/sidecar command stderr and exit codes in the plugin UI if users need richer troubleshooting affordances.
- Add sample plugin packages that exercise `/hello`, a theme, and a memory hook for Phase 22 end-to-end acceptance QA.

---

## Chunk 22.5 — Memory-hook contribution pipeline

**Date:** 2026-05-02

**Summary:** Wired plugin `memory_hooks` contributions into the real memory write path. `add_memory` now lazily activates plugins for matching `OnMemoryTag` events, runs sandboxed `pre_store` hooks before SQLite persistence, applies JSON patches for content/tags/importance/type, and runs `post_store` hooks after persistence as notification-only processors.

**Files changed:**
- `src-tauri/src/plugins/host.rs` — added the active memory-hook registry, `MemoryHookPayload` / `MemoryHookPatch` / `MemoryHookRunResult`, lazy `activate_for_memory_tags`, 200 ms sandbox execution through `WasmRunner`, optional SHA-256 verification, Windows-safe local WASM path handling, and hook registry tests.
- `src-tauri/src/sandbox/wasm_runner.rs` — added the `memory_hook(input_ptr, input_len) -> packed_ptr_len` ABI runner and a WASM-encoder fixture test proving JSON patch output is read back correctly.
- `src-tauri/src/commands/memory.rs` — split `add_memory_inner` for tests, fired `pre_store` / `post_store` hooks around the existing embedding/store/auto-tag flow, and added an integration-style sample WASM tag-rewriter test.
- `src-tauri/src/plugins/mod.rs`, `src-tauri/Cargo.toml`, and `src-tauri/Cargo.lock` — exported the new host types and added `wasm-encoder` as a direct dev dependency for compact WASM fixtures.
- `README.md` and `docs/brain-advanced-design.md` — documented sandboxed plugin memory hooks in the brain/memory architecture docs.
- `rules/milestones.md` — removed completed row 22.5.

**Validation:**
- `cargo test memory_hook` — 3 targeted hook/ABI tests passed.
- `cargo test memory_tag_activation_matches_prefix_tags` — 1 lazy activation test passed.
- `cargo test add_memory_applies_prestore` — sample WASM tag-rewriter integration test passed.
- `Cargo Check + Clippy` task — passed (`cargo check` + `cargo clippy -- -D warnings`).
- `Full CI Gate` task — completed successfully: frontend test/typecheck stage, clippy, Rust unit tests (1962 passed), Ollama smoke tests (4 passed), and doctest (1 passed).

**Follow-ups (not in this chunk):**
- Chunk 22.7 still needs real Tool / Sidecar / WASM command execution dispatch.
- Add retrieval/consolidation hook stages only when the host has concrete call sites for them.

---

## Chunk 16.3b — Late chunking ingest integration

**Date:** 2026-05-02

**Summary:** Wired late chunking into document ingestion behind `AppSettings.late_chunking`. When a local Ollama embedder returns per-token whole-document vectors, ingestion aligns semantic chunks back to text spans, converts them to token spans, pools chunk embeddings with `memory::late_chunking::pool_chunks`, and stores those vectors through the existing SQLite embedding column. If the embedder only returns the standard pooled `/api/embed` shape, ingestion gracefully falls back to the existing per-chunk embedding path.

**Files changed:**
- `src-tauri/src/memory/late_chunking.rs` — added `CharSpan` and `token_spans_for_char_spans` for chunk-to-token alignment.
- `src-tauri/src/brain/ollama_agent.rs` — added `OllamaAgent::embed_tokens` plus token-vector response parsing for offsets, token text, and batched shapes.
- `src-tauri/src/commands/ingest.rs` — added `AppSettings.late_chunking` gating, whole-document token embedding, pooled vector storage, and created-entry embedding bookkeeping.
- `src-tauri/src/settings/mod.rs`, `src-tauri/src/settings/config_store.rs`, `src-tauri/src/commands/settings.rs`, `src/stores/settings.ts`, and `src/views/BrainView.test.ts` — added the persisted `late_chunking` setting with default-off serde/TS support.
- `docs/brain-advanced-design.md` and `README.md` — documented the shipped late-chunking ingest path.
- `rules/milestones.md` — removed completed row 16.3b.

**Validation:**
- `cargo test memory::late_chunking -- --nocapture` — 18 tests passed.
- `cargo test brain::ollama_agent -- --nocapture` — 24 tests passed.
- `cargo test commands::ingest -- --nocapture` — 12 tests passed.
- `cargo test settings:: -- --nocapture` — 34 settings/command tests passed.
- `Full CI Gate` — passed after fixing one Clippy style warning: frontend tests (1480 passed), `npx vue-tsc --noEmit`, `cargo clippy -- -D warnings`, Rust unit tests (1958 passed), Ollama smoke tests (4 passed), and doctest (1 passed).

**Follow-ups (not in this chunk):**
- Add a Brain hub toggle for `late_chunking` if users need an in-app control instead of saving the setting programmatically.
- Add live-model QA once an Ollama model exposes per-token embeddings through `/api/embed` on the user's machine.

---

## Chunk 17.5b — Cross-device memory sync Soul Link wire protocol

**Date:** 2026-05-02

**Summary:** Finished the Soul Link wire protocol for CRDT memory sync. Inbound `memory_sync` messages now apply LWW deltas, reply with local deltas captured before inbound application, and record sync watermarks. `memory_sync_request` returns local deltas for a requested timestamp, `sync_memories_with_peer` triggers outbound sync, and the receive loop starts sync on connect/reconnect.

**Files changed:**
- `src-tauri/src/link/handlers.rs` — added message dispatch, memory sync/request handlers, reconnect-triggered sync, peer-address parsing, and protocol tests.
- `src-tauri/src/commands/link.rs` — starts the receive loop on connect and exposes `sync_memories_with_peer`.
- `src-tauri/src/link/mod.rs` — exports the handlers module.
- `src-tauri/src/lib.rs` — registers the sync command.
- `src-tauri/src/memory/crdt_sync.rs` — stores `sync_log` watermarks in Unix-ms units so they compare correctly with memory `updated_at` values.
- `docs/brain-advanced-design.md` and `README.md` — documented the shipped cross-device memory sync protocol.
- `rules/milestones.md` — removed completed row 17.5b.

**Validation:**
- `cargo test link::handlers -- --nocapture` — 7 handler/protocol tests passed.
- `cargo test memory::crdt_sync -- --nocapture` — 9 CRDT sync tests passed.
- `Full CI Gate` — passed: frontend tests/typecheck, `cargo clippy -- -D warnings`, Rust unit tests (1946 passed), Ollama smoke tests (4 passed), and doctest (1 passed).

**Follow-ups (not in this chunk):**
- Add a frontend action in `src/stores/link.ts` for manual memory sync if the Link panel needs an explicit button.
- Move the receive loop off the outer `AppState.link_manager` mutex if future transports need concurrent send/receive from multiple commands.

---

## Chunk 16.1 — Relevance threshold for `[LONG-TERM MEMORY]` injection

**Date.** 2026-04-24
**Phase.** 16 (Modern RAG) — first chunk; cheapest impact-rich win.
**Goal.** Stop diluting the brain's context window with weakly-matching memories. Until now `commands::streaming` always injected the top-5 hybrid-search results regardless of how poorly they matched the user's query.

**Architecture.**
- New `MemoryStore::hybrid_search_with_threshold(query, query_embedding, limit, min_score)` returns the same shape as `hybrid_search` but filters out entries whose final hybrid score is below `min_score` *before* truncating to `limit`.
- Internal helper `MemoryStore::hybrid_search_scored` factors out the scoring loop so the legacy `hybrid_search` and the new threshold variant share a single source of truth.
- Crucial side-effect tweak: filtered (below-threshold) rows are **not** counted as accesses. The legacy method touched every returned row's `access_count` + `last_accessed`; the new method only touches *survivors*. This keeps the decay signal honest — irrelevant rows continue ageing out of relevance instead of being kept artificially fresh by retrieval misses.
- New `AppSettings.relevance_threshold: f64` field with `#[serde(default = "default_relevance_threshold")]` (default `0.30`) for back-compat with persisted settings files. Constant `crate::settings::DEFAULT_RELEVANCE_THRESHOLD = 0.30` is the single source of truth.
- Both `commands::streaming` call sites (cloud OpenAI-compatible path + local Ollama path) now read the threshold from `AppSettings` and pass it into the new method. `lock` errors degrade to the documented default — no panics.

**Files modified.**
- `src-tauri/src/memory/store.rs` — added `hybrid_search_with_threshold` + `hybrid_search_scored` helper + 5 new unit tests.
- `src-tauri/src/settings/mod.rs` — added `relevance_threshold` field, `DEFAULT_RELEVANCE_THRESHOLD` constant, `default_relevance_threshold` serde fallback.
- `src-tauri/src/settings/config_store.rs` — propagated the new field through every `AppSettings { … }` literal in tests.
- `src-tauri/src/commands/settings.rs` — propagated the new field through every `AppSettings { … }` literal in tests.
- `src-tauri/src/commands/streaming.rs` — both RAG retrieval blocks (cloud + local) call `hybrid_search_with_threshold` with the user-tunable threshold.
- `docs/brain-advanced-design.md` § 16 Phase 4 — flipped the row from `○` to `✓` with module + setting pointers.
- `rules/milestones.md` — Phase 16 row 16.1 removed (per the "completed chunks belong in completion-log only" rule).

**Tests.** 5 new unit tests in `memory::store::tests`, plus 909 existing tests still passing — total **914 passing**:
1. `hybrid_search_with_threshold_zero_matches_legacy_top_k` — back-compat invariant: `min_score = 0.0` reproduces the legacy `hybrid_search` top-k exactly (same ids, same order). Critical because every existing call site that hasn't been migrated yet must keep working.
2. `hybrid_search_with_threshold_filters_below_score` — high threshold drops weakly-matching rows.
3. `hybrid_search_with_threshold_keeps_strong_matches` — low threshold + strong keyword + freshness combo retains the matching row.
4. `hybrid_search_with_threshold_does_not_increment_access_for_filtered` — decay-signal-honesty invariant: filtered rows' `access_count` is **not** bumped.
5. `hybrid_search_with_threshold_respects_limit` — `limit` cap still applies even when many rows survive the threshold.

**Validation.** `cargo test --lib` (914 pass, 0 fail) + `cargo clippy --lib --tests -- -D warnings` (clean).

**Follow-ups (not in this chunk).**
- Frontend: surface the threshold in the Brain hub "Active Selection" preview panel so users can preview what *would* be injected at the current threshold (deferred to a small frontend chunk; the Rust surface already supports it).
- 16.2 (Contextual Retrieval) — next chunk in Phase 16; orthogonal to this one.

---

## Chunk 28.12 — Multi-agent coding DAG orchestration wiring

**Date:** 2026-05-02

**Summary:** Wired the self-improve coding loop through the existing DAG orchestration layer. `coding::dag_runner` now includes an async executor with bounded parallelism per topological layer, and `coding::engine` runs each chunk as a Planner → Coder → Reviewer → Apply → Tester → Stage graph with capability validation and skip-on-failure behavior.

**Files changed:**
- `src-tauri/src/coding/dag_runner.rs` — added `execute_dag_async` and async tests for success and predecessor-failure skipping.
- `src-tauri/src/coding/engine.rs` — replaced the linear plan/apply path with `execute_chunk_dag`, explicit DAG nodes, capability config, shared node state, and failure summaries.
- `src-tauri/src/coding/mod.rs` — updated stale module overview now that the autonomous loop is live.
- `docs/coding-workflow-design.md` — documented Chunk 28.12 and the graph-backed coding gate.
- `rules/research-reverse-engineering.md` — marked the DAG wiring lesson as implemented and kept remaining follow-ups focused on worktree isolation and path-scoped context.
- `rules/milestones.md` — removed completed row 28.12.

**Validation:**
- `cargo test coding:: -- --nocapture` — 252 coding-module tests passed.
- `Full CI Gate` — passed after fixing the order-independent GraphRAG community test: frontend tests/typecheck, `cargo clippy -- -D warnings`, Rust unit tests (1942 passed), Ollama smoke tests (4 passed), and doctests (1 passed).

**Follow-ups (not in this chunk):**
- Add optional temporary-worktree execution for dirty or high-risk runs.
- Add path-scoped context loading so large repos can load only rules relevant to touched files.

---

## Chunk 28.11 — Apply/review/test execution gate

**Date:** 2026-05-02

**Summary:** Upgraded the autonomous coding self-improve loop from plan-only output to a conservative execution gate. The engine now asks the coding LLM for complete typed file blocks, previews synthetic diffs through the reviewer, snapshots touched files, applies accepted changes, runs configured test suites, restores on failure, and stages paths only after validation passes.

**Files changed:**
- `src-tauri/src/coding/engine.rs` — added coder prompt contract, preview diff review, file snapshots, restore-on-failure, test summaries, and post-pass staging.
- `docs/coding-workflow-design.md` — documented Cursor + Claude Code workflow lessons and marked the checkpointed execution gate as shipped.
- `rules/research-reverse-engineering.md` — added Cursor/Claude Code reverse-engineering notes and follow-up workflow patterns.
- `rules/milestones.md` — removed completed row 28.11; 28.12 is now the next Phase 28 chunk.

**Validation:**
- `cargo test coding::engine` — 10 passed.
- `cargo check` — passed.
- `cargo clippy -- -D warnings` — passed.
- Follow-up local CI sweep fixed stale frontend contracts and task wiring, then passed `npx vue-tsc --noEmit`, `npx vitest run`, `cargo test`, and `cd src-tauri && cargo check && cargo clippy -- -D warnings`.

**Follow-ups (not in this chunk):**
- Chunk 28.12: wire multi-agent coding DAG orchestration into the execution gate.
- Add temporary worktree execution and path-scoped context loading so future generated patches can be isolated even more tightly.

---

## Chunk 17.5a — CRDT sync schema + LWW core

**Date:** 2026-05-02

**Summary:** Implemented the LWW-Map CRDT foundation for cross-device memory sync. V13 schema migration adds `updated_at` and `origin_device` columns to `memories` + `sync_log` audit table. New `crdt_sync` module provides `compute_sync_deltas()` and `apply_sync_deltas()` with LWW conflict resolution (highest `updated_at` wins, lexicographic `origin_device` tiebreaker). Custom `SyncKey` type matches entries by `content_hash` (primary) or `(content_prefix, created_at)` for legacy entries. Two new Tauri commands: `get_memory_deltas`, `apply_memory_deltas`.

**Files changed:**
- `src-tauri/src/memory/crdt_sync.rs` (new, ~380 LOC)
- `src-tauri/src/memory/migrations.rs` (V13 migration + sentinel bump)
- `src-tauri/src/memory/store.rs` (`updated_at` + `origin_device` on MemoryEntry, SELECT updates)
- `src-tauri/src/memory/mod.rs` (module declaration)
- `src-tauri/src/commands/link.rs` (2 new commands)
- `src-tauri/src/lib.rs` (command registration)
- Various files: added new fields to struct literals

**Tests:** 8 unit tests (compute_deltas, insert/update/skip/tiebreaker/soft_close/roundtrip/sync_log)

---

## Chunk 16.6 — GraphRAG community summaries

**Date:** 2026-05-02

**Summary:** Implemented GraphRAG with Leiden-style community detection over `memory_edges`, community persistence (`memory_communities` table), and dual-level retrieval (entity keyword search + community summary search) fused via RRF. Custom Louvain/modularity-greedy algorithm (~130 LOC) avoids external graph dependency. Two new Tauri commands: `graph_rag_detect_communities` (runs detection + stores), `graph_rag_search` (dual-level retrieval). LLM community summarization is a separate step (communities stored without summaries initially, summaries populated via brain when available).

**Files changed:**
- `src-tauri/src/memory/graph_rag.rs` (new, ~320 LOC)
- `src-tauri/src/memory/mod.rs` (module declaration)
- `src-tauri/src/commands/memory.rs` (2 new commands)
- `src-tauri/src/lib.rs` (command registration)

**Tests:** 5 unit tests (detect_communities_finds_two_clusters, detect_and_store_communities_persists, graph_rag_search_returns_relevant_hits, detect_communities_handles_empty_graph, community_ranking_keyword_only)

---

## Chunk 15.7 — VS Code Copilot incremental-indexing QA

**Date:** 2026-05-02

**Summary:** Added 5 integration tests validating the `brain_suggest_context` fingerprint caching contract for VS Code Copilot. Tests cover: cold call (valid fingerprint + hits), warm call (cache-hit stability), invalidation after new memory addition, invalidation after memory deletion, and query-sensitivity (different queries → different fingerprints).

**Files changed:**
- `src-tauri/src/ai_integrations/gateway.rs` (5 new tests in `tests` module)

**Tests:** `incremental_indexing_cold_call_returns_valid_fingerprint`, `incremental_indexing_warm_call_cache_hit`, `incremental_indexing_invalidation_after_new_memory`, `incremental_indexing_invalidation_after_delete`, `incremental_indexing_fingerprint_is_query_sensitive`

---

## Chunk 17.7 — Bidirectional Obsidian sync

**Date:** 2026-05-02

**Summary:** Extended the one-way Obsidian export to bidirectional sync using a `notify` file-watcher. Added `obsidian_path` and `last_exported` columns to `MemoryEntry`. Implemented `parse_obsidian_markdown()` for roundtrip frontmatter parsing, `sync_bidirectional()` with LWW conflict resolution, and `ObsidianWatcher` background task with 1-second debounce. Three new Tauri commands: `obsidian_sync` (manual one-shot), `obsidian_sync_start` (background watcher), `obsidian_sync_stop`.

**Files changed:**
- `src-tauri/src/memory/obsidian_sync.rs` (new, ~420 LOC)
- `src-tauri/src/memory/store.rs` (schema fields + `set_obsidian_sync` + SELECT updates)
- `src-tauri/src/memory/mod.rs` (module declaration)
- `src-tauri/src/commands/memory.rs` (3 new commands)
- `src-tauri/src/lib.rs` (AppState field + command registration)
- `src-tauri/Cargo.toml` (notify v7)
- Various files: added `obsidian_path`/`last_exported` to struct literals

**Tests:** 5 Rust unit tests (roundtrip, import, parse variants)

---

## Chunk 24.4 — Phone-control RPC surface

**Date:** 2026-05-02
**Status:** ✅ Complete

**What was done:**
- Created `proto/terransoul/phone_control.v1.proto` — 8 RPCs: `GetSystemStatus`, `ListVsCodeWorkspaces`, `GetCopilotSessionStatus`, `ListWorkflowRuns`, `GetWorkflowProgress`, `ContinueWorkflow`, `SendChatMessage`, `ListPairedDevices`.
- Updated `build.rs` to compile both `brain.v1.proto` and `phone_control.v1.proto`.
- Created `src/ai_integrations/grpc/phone_control.rs` (~280 LOC) — full tonic service implementation:
  - `GetSystemStatus` — sysinfo for CPU/RAM, reads brain mode from AppState.
  - `ListVsCodeWorkspaces` — discovers recent VS Code workspaces from `storage.json`.
  - `GetCopilotSessionStatus` — delegates to `vscode_probe::probe_copilot_session()`.
  - `ListWorkflowRuns` / `GetWorkflowProgress` — delegates to workflow engine.
  - `ContinueWorkflow` — sends heartbeat to workflow engine.
  - `SendChatMessage` — non-streaming one-shot completion via `OpenAiClient`.
  - `ListPairedDevices` — reads from paired_devices SQLite table.
- Modified `grpc/mod.rs` `serve_with_shutdown()` to accept optional `AppState` and register `PhoneControlServer` alongside `BrainServer`.
- Updated `commands/grpc.rs` to pass `AppState` clone into the gRPC server spawn.
- All 1904 Rust tests pass, clippy clean, 1480 frontend tests pass.

**Files changed:**
- `src-tauri/proto/terransoul/phone_control.v1.proto` (new)
- `src-tauri/build.rs`
- `src-tauri/src/ai_integrations/grpc/mod.rs`
- `src-tauri/src/ai_integrations/grpc/phone_control.rs` (new)
- `src-tauri/src/commands/grpc.rs`

---

## Chunk 24.3 — LAN gRPC activation + paired-device mTLS enforcement

**Date:** 2026-05-02
**Status:** ✅ Complete
**Phase:** 24 (Mobile Companion)

**Goal.** Wire the shipped brain.v1 gRPC transport into LAN mode with mTLS enforcement when `lan_enabled`.

**Deliverables:**
- `src-tauri/src/commands/grpc.rs` — `GrpcServerHandle`, Tauri commands `grpc_server_start`, `grpc_server_stop`, `grpc_server_status`.
- When `lan_enabled`: binds `0.0.0.0:7422`, issues server cert from pairing CA, requires mTLS client verification (paired devices only).
- When loopback: binds `127.0.0.1:7422`, plaintext (safe — existing `PlaintextNonLoopback` guard enforces this).
- `PairingManager.issue_server_cert()` + internal `issue_server_cert()` function in `network/pairing.rs`.
- `AppStateInner.grpc_server: TokioMutex<Option<GrpcServerHandle>>`.
- 1 unit test in `commands/grpc.rs`.

**Tests:** 1905 Rust tests pass, clippy clean.

---

## Chunk 24.2b — mTLS pairing flow + persistent device registry

**Date:** 2026-05-02
**Status:** ✅ Complete
**Phase:** 24 (Mobile Companion)

**Goal.** Self-signed CA for mTLS device pairing, per-device client cert issuance, persistent device registry in SQLite, Tauri commands for the pairing flow.

**Deliverables:**
- V12 SQLite migration: `paired_devices` table (`device_id`, `display_name`, `cert_fingerprint`, `capabilities` JSON, `paired_at`, `last_seen_at`).
- `src-tauri/src/network/pairing.rs` (~310 LOC) — `PairingManager` (CA load/generate, start_pairing, confirm_pairing), `PairedDevice` struct, SQLite CRUD (`insert_paired_device`, `list_paired_devices`, `revoke_device`, `touch_device`, `find_device_by_fingerprint`).
- CA persisted as PEM files (`pairing_ca_cert.pem`, `pairing_ca_key.pem`) in data dir.
- 5-minute pairing window enforced server-side, constant-time token comparison.
- `AppStateInner.pairing_manager: Mutex<Option<PairingManager>>` — lazily initialized on first `start_pairing` call.
- Tauri commands: `start_pairing`, `confirm_pairing`, `revoke_device`, `list_paired_devices` in `commands/lan.rs`.
- 10 unit tests in `pairing.rs`.

**Tests:** 1904 Rust tests pass, clippy clean.

---

## Chunk 24.5b — VS Code / Copilot session probe FS wrapper

**Date:** 2026-05-02
**Status:** ✅ Complete
**Phase:** 24 (Mobile Companion)

**Goal.** Wrap the pure Copilot log parser (24.5a) with real filesystem I/O so the phone companion can query "what's Copilot doing on your desktop?".

**Deliverables:**
- `src-tauri/src/network/vscode_probe.rs` — `vscode_user_data_dir()` (per-OS path resolution), `find_latest_copilot_log(user_data)` (walks `logs/<date>/window<N>/exthost/GitHub.copilot-chat/Copilot-Chat.log`, picks most-recently modified), `probe_copilot_session()` (async read + summarise).
- Tauri command `get_copilot_session_status` → `Option<CopilotLogSummary>` in `commands/lan.rs`.
- 3 unit tests + 1 tokio integration test.

**Tests:** 1894 Rust tests pass, clippy clean.

---

## Chunk 24.1b — LAN bind config + OS probe wrapper

**Date:** 2026-05-02
**Status:** ✅ Complete
**Phase:** 24 (Mobile Companion)

**Goal.** Enable LAN-mode brain exposure with explicit user opt-in. Provide OS network-interface discovery for the pairing UI.

**Deliverables:**
- `src-tauri/src/network/lan_probe.rs` — `discover_lan_addresses()` / `discover_lan_addresses_with(options)` / `enumerate_os_addresses()` using `local-ip-address` crate v0.6. Feeds addresses through 24.1a's `classify_addresses()` filter.
- `src-tauri/src/commands/lan.rs` — Tauri command `list_lan_addresses` returning `Vec<LanAddress>` for the pairing UI.
- `AppSettings.lan_enabled: bool` (default `false`, `#[serde(default)]`) — when true, MCP server binds to `0.0.0.0` instead of `127.0.0.1`.
- `ai_integrations/mcp/mod.rs` — `start_server()` now takes `lan_enabled` param to select bind address.
- 3 unit tests in `lan_probe.rs`, 1 in `commands/lan.rs`.

**Tests:** 1891 Rust tests pass, 1480 Vitest pass, clippy clean.

---

## Chunk 20.1 — Dev/release data-root split (Docker namespacing)

**Date:** 2026-05-02
**Status:** ✅ Complete
**Phase:** 20 (Data Lifecycle)

**Goal.** Namespace Docker/Podman Ollama containers and volumes by build mode so dev and release don't interfere with each other.

**Deliverables:**
- `src-tauri/src/brain/docker_ollama.rs` — `CONTAINER_NAME` and `VOLUME_NAME` consts using `cfg!(debug_assertions)`: dev → `"ollama-dev"` / `"ollama_data_dev"`, release → `"ollama"` / `"ollama_data"`.
- `volume_mount` now uses `format!("{VOLUME_NAME}:/root/.ollama")`.
- Follows the same `cfg!` pattern already established by MCP port split.

**Tests:** All passing, clippy clean.

---

## Chunk 16.5b — CRAG query-rewrite + web-search fallback

**Date:** 2026-05-02
**Status:** ✅ Complete

**Summary:** Wired the CRAG evaluator (16.5a) into a full orchestrator command `crag_retrieve` that implements the Corrective RAG pipeline:
1. Retrieves memories via hybrid search
2. Evaluates each memory with LLM-based CRAG classifier (`CORRECT`/`AMBIGUOUS`/`INCORRECT`)
3. On `Ambiguous` → rewrites the query via LLM prompt + retries retrieval
4. On `Incorrect` → falls back to DuckDuckGo HTML scraping (gated by `web_search_enabled` setting)
5. Returns quality-assessed memories with metadata (quality, rewrite status, web fallback used)

Also added query rewriter prompts + parser + web-search URL builder to `memory::crag` module, with 5 new tests.

**Files changed:**
- `src-tauri/src/commands/crag.rs` (NEW, ~290 LOC) — `crag_retrieve` Tauri command + `run_crag_retrieve` testable entry point, `evaluate_document`, `rewrite_query`, `try_web_fallback`, `extract_search_snippets`, `filter_by_verdicts` + 3 tests
- `src-tauri/src/memory/crag.rs` — added `build_rewriter_prompts`, `parse_rewritten_query`, `build_web_search_url` + 5 tests
- `src-tauri/src/settings/mod.rs` — added `web_search_enabled: bool` field (default false, capability gate)
- `src-tauri/src/settings/config_store.rs` — added `web_search_enabled` to test fixtures
- `src-tauri/src/commands/settings.rs` — added `web_search_enabled` to test fixtures
- `src-tauri/src/commands/mod.rs` — registered `pub mod crag`
- `src-tauri/src/lib.rs` — imported + registered `crag_retrieve` command

**Testing:** 22 CRAG tests pass, cargo clippy clean, all cargo tests pass, 1480 Vitest tests pass.

---

## Chunk 16.4b — Self-RAG orchestrator loop

**Date:** 2026-05-02
**Status:** ✅ Complete

**Summary:** Wired `SelfRagController` (shipped in 16.4a) into a new Tauri streaming command `send_message_stream_self_rag` that implements the full Self-RAG iterative refinement loop over Ollama. The command:
1. Embeds the user query and retrieves memories via `hybrid_search_with_threshold`
2. Builds a system prompt augmented with `[LONG-TERM MEMORY]` + `SELF_RAG_SYSTEM_PROMPT` (asks LLM to emit reflection tokens)
3. Streams the LLM response in real-time via `llm-chunk` events (with `StreamTagParser` for anim/pose blocks)
4. Evaluates the complete response via `SelfRagController::next_step()`
5. On `Decision::Retrieve` → emits a "refining…" indicator and loops (re-embed + re-retrieve + re-prompt)
6. On `Decision::Accept` → stores the cleaned answer and emits done
7. On `Decision::Reject` → emits a graceful refusal message

**Files changed:**
- `src-tauri/src/commands/streaming.rs` — added `send_message_stream_self_rag` command + `run_self_rag_stream` testable entry point (~200 lines)
- `src-tauri/src/lib.rs` — registered `send_message_stream_self_rag` in import + handler list

**Testing:** Cargo clippy clean (0 warnings), all cargo tests pass, 1480 Vitest tests pass.

---

## Chunk 27.8 — Persona pack schema spec document

**Date:** 2026-05-02
**Status:** ✅ Complete

**Summary:** Created a stable `.terransoul-persona` v1 schema specification document and added 6 schema-conformance unit tests to `persona::pack`.

**Files changed:**
- `docs/persona-pack-schema.md` (NEW) — ~230-line spec covering envelope structure, traits/expressions/motions, packVersion semantics, additive-only-within-version contract, forward/backward compat rules, import merge semantics, examples
- `src-tauri/src/persona/pack.rs` — added 6 `schema_spec_*` tests validating minimal/full packs, unknown-keys round-trip, id-charset validation, optional provenance

**Testing:** All 27 pack.rs tests pass, cargo clippy clean.

---

## Chunk 27.7 — Persona example-dialogue field

**Date:** 2026-05-02
**Status:** ✅ Complete

### Summary

Extended the persona schema with an optional `exampleDialogue: string[]` field
for character-card-style example exchanges. The field round-trips cleanly
through the existing Rust backend (which is schema-opaque — `serde_json::Value`)
and pack export/import without any Rust changes.

### Files Modified

- `src/stores/persona-types.ts` — added `exampleDialogue` to `PersonaTraits`, `defaultPersona()`, `migratePersonaTraits()`
- `src/utils/persona-prompt.ts` — renders "Example dialogue:" section with up to 4 deduplicated entries
- `src/utils/persona-prompt.test.ts` — 4 new tests (render, cap at 4, dedup, skip-when-empty)
- `src/components/PersonaPanel.vue` — new `PersonaListEditor` for example dialogue; `cloneTraits` + `loadSuggestionIntoDraft` updated
- `src/stores/persona.test.ts` — 1 new round-trip test

### Tests

- 47 persona-related tests passing (20 prompt builder + 27 store)
- `vue-tsc --noEmit` clean
- No Rust changes needed (backend is schema-opaque)

---

## Chunk 15.8 — AI Coding Integrations doc finalisation

**Date:** 2026-05-02
**Status:** ✅ Complete

### Summary

Replaced all "Planned" sections in `docs/AI-coding-integrations.md` with
as-built reality. Updated gRPC section (Chunk 15.2 shipped), Control Panel
section (Chunk 15.4 shipped), and the roadmap table. Only Chunk 15.7
(incremental-indexing QA) remains not-started.

### Files Modified

- `docs/AI-coding-integrations.md` — status header, gRPC section, Control Panel section, roadmap table all updated

---

## Chunk 27.3 — Blendshape passthrough — expanded ARKit rig

**Date:** 2026-05-02
**Status:** ✅ Complete (discovered already shipped — archiving)

### Summary

Opt-in per-ARKit-blendshape passthrough for advanced VRM rigs beyond the
6-preset baseline. Gated by `AppSettings.expanded_blendshapes`.

### Files

- `src/renderer/expanded-blendshapes.ts` (111 LOC) — `applyExpandedBlendshapes`, `clearExpandedBlendshapes`, `ARKIT_BLENDSHAPE_NAMES` (52 shapes)
- `src/renderer/expanded-blendshapes.test.ts` (111 LOC) — 10 tests (rig-aware writes, baseline overlap skip, clamping, null safety)
- `src/renderer/face-mirror.ts` — baseline map: 52 ARKit → 6+5+2 VRM channels

### Tests

Vitest: 10 new tests all passing.

---

## Chunk 15.4 — AI Coding Integrations Control Panel

**Date:** 2026-05-02
**Status:** ✅ Complete (discovered already shipped — archiving)

### Summary

`AICodingIntegrationsView.vue` + `ai-integrations.ts` Pinia store. Server
status card, start/stop/regenerate, client config cards (VS Code, Claude,
Codex) with set-up/remove buttons, transport preference toggle, VS Code
workspace windows list.

### Files

- `src/views/AICodingIntegrationsView.vue` (468 LOC) — full control panel UI
- `src/stores/ai-integrations.ts` (204 LOC) — Pinia store wrapping Tauri MCP/auto-setup commands
- `src/stores/ai-integrations.test.ts` (156 LOC) — unit tests

### Tests

Vitest: 156 lines of store tests all passing.

---

## Chunk 15.2 — gRPC `brain.v1` transport foundation

**Date:** 2026-05-01
**Status:** ✅ Complete

### Summary

Lands the typed gRPC transport foundation for AI coding integrations. The new
`brain.v1` protobuf schema exposes the existing `BrainGateway` surface over
tonic, including unary search and streaming search, while preserving the same
capability-gated business logic used by MCP.

### Implementation

- **`src-tauri/proto/terransoul/brain.v1.proto`** — versioned protobuf schema
  with `Health`, `Search`, `StreamSearch`, `GetEntry`, `ListRecent`,
  `KgNeighbors`, `Summarize`, `SuggestContext`, and `IngestUrl` RPCs.
- **`src-tauri/build.rs`** — compiles the proto with `tonic-prost-build` and a
  vendored `protoc`, avoiding host protobuf compiler drift in CI.
- **`src-tauri/src/ai_integrations/grpc/mod.rs`** — tonic service adapter over
  `Arc<dyn BrainGateway>`, conversion helpers, gRPC status mapping, mTLS-capable
  `tls_config_from_pem`, and `serve_with_shutdown`. Plaintext serving is
  fail-closed for non-loopback addresses so future LAN mode cannot expose the
  brain without TLS.
- **`src-tauri/src/ai_integrations/mod.rs`** — exports the new `grpc` module.
- **`docs/AI-coding-integrations.md`** — records 15.2 as shipped and documents
  the as-built transport foundation.
- **`rules/milestones.md`** — archives 15.2 from Phase 15 and narrows Phase 24.3
  to LAN/runtime activation over the shipped transport.

### Validation

- `cargo test -q ai_integrations::grpc` — 4 passed.

---

## Chunk 14.16f — Pack-import provenance markers

**Date:** 2026-05-02
**Status:** ✅ Complete

### Summary

Closes the LLM-as-Animator family (Chunk 14.16) by tagging every saved
learned-motion clip with its origin so persona pack import previews can
report the breakdown. Generator output (Chunk 14.16c) is now stamped
`provenance: "generated"` and camera-mirror captures (PersonaTeacher) are
stamped `provenance: "camera"`. The pack import path counts both
buckets and the panel renders e.g. *"5 motions (3 generated, 2 camera),
1 skipped."* Older clips without the field stay readable and are simply
left unattributed in the report — no migration, no breaking change.

### Implementation

- **`src-tauri/src/persona/pack.rs`** — extended `ImportReport` with
  `motions_generated: u32` + `motions_camera: u32` (both `#[serde(default)]`
  for forward/backward compat). Added pure helper
  `note_motion_provenance(report, motion_value)` that peeks at the
  motion JSON's `provenance` string and bumps the matching counter;
  unknown / missing labels stay unattributed. Four new unit tests cover
  generated, camera, missing, and unknown-label cases.
- **`src-tauri/src/commands/persona.rs`** — both `import_persona_pack`
  and `preview_persona_pack` now invoke `note_motion_provenance` after
  each motion's `validate_asset` accepts so the preview and the actual
  import agree on the breakdown.
- **`src/stores/persona-types.ts`** — added optional
  `provenance?: 'generated' | 'camera'` to `LearnedMotion` with a
  documented forward-compat note.
- **`src/components/PersonaMotionGenerator.vue`** — `acceptAndSave`
  now spreads the candidate and sets `provenance: 'generated'` before
  calling `store.saveLearnedMotion`.
- **`src/components/PersonaTeacher.vue`** — `saveMotion` now sets
  `provenance: 'camera'` on the recorded clip.
- **`src/components/PersonaPackPanel.vue`** + **`src/stores/persona.ts`**
  — extended both `ImportReport` mirrors with the two new counters and
  added a `motionProvenanceLabel` computed that renders only the
  non-zero buckets ("3 generated, 2 camera"). Empty for legacy packs.

### Tests

- **Rust:** 1619 lib tests pass (4 new in `persona::pack::tests`).
- **Frontend:** 1457 vitest pass (no new specs — provenance is a
  passive serialised field exercised by existing pack round-trips).
- **Clippy:** `cargo clippy --lib -- -D warnings` clean.

### Repair note

Session entry showed a pre-existing build break: `src/commands/coding.rs`
was still in `commands/mod.rs` after the prior cleanup removed the
top-level `coding` module + `AppState::coding_llm_config` field. Dropping
the stray `pub mod coding;` from `commands/mod.rs` (a one-line repair)
unblocked the lib build with no behavioural change.

---

## Chunk 14.16e — Self-improve motion-feedback loop

**Date:** 2026-05-02
**Status:** ✅ Complete
**Brain-surface change:** No (persona/animation pipeline; the
feedback hint is appended to the motion-clip system prompt only,
leaves the long-term memory + RAG retrieval pipelines untouched).

### What shipped

Closes chunk 14.16e by capturing user accept/reject signals on
LLM-generated motion clips and feeding them back into the next
generation so the brain learns the user's preferred movement
vocabulary over time.

**Backend:**

- New module
  [src-tauri/src/persona/motion_feedback.rs](src-tauri/src/persona/motion_feedback.rs)
  (~360 LOC): `MotionFeedbackEntry`, `FeedbackVerdict::{Accepted,
  Rejected}`, `MotionFeedbackStats`, `TrustedTrigger`, plus pure
  helpers `append_entry`, `load_entries`, `aggregate_stats`,
  `render_prompt_hint`. Storage is a single newline-delimited JSON
  file under `<app_data_dir>/persona/motion_feedback.jsonl`,
  append-only and crash-safe at line granularity.
- New helpers in `motion_clip.rs`: `build_motion_prompt_with_hint`
  (the no-hint version delegates to it). The hint is a single
  sentence listing the user's top trusted triggers —
  `render_prompt_hint` returns empty when there's nothing to nudge
  with, making the path zero-cost on first launch.
- Two new Tauri commands in
  [src-tauri/src/commands/persona.rs](src-tauri/src/commands/persona.rs):
  - `record_motion_feedback(payload)` — appends one accept/reject
    event with server-stamped `at`. Validates description length
    and trigger non-empty.
  - `get_motion_feedback_stats()` — returns the aggregate
    `MotionFeedbackStats` for the persona-panel UI.
- `generate_motion_from_text` now reads the feedback log up-front
  and pipes the prompt hint into the brain request, closing the
  loop with no extra IPC round-trips for the frontend.
- 11 new unit tests covering append/load round-trip, missing-file
  empty-vec, corrupt-line skipping, total/accepted/rejected counts,
  trusted-trigger ordering (accept count desc → alpha), 3-rejection
  / no-accept threshold for discouraged descriptions, mixed-history
  protection (a single accept removes from discouraged), prompt
  hint empty/non-empty, parent-dir creation, and the 50-entry cap.
- `lib.rs` registers both commands in the invoke handler.

**Frontend:**

- New persona-store actions `recordMotionFeedback` (best-effort,
  swallows errors so a save never fails because of feedback) and
  `fetchMotionFeedbackStats`.
- [PersonaMotionGenerator.vue](src/components/PersonaMotionGenerator.vue)
  now records `accepted` after a successful save and `rejected` on
  Discard, then refreshes the stats so the new "You've taught me N
  accepted motion(s) across M tries; favourites: …" footer updates
  in place.

### Verification

- `cargo test --lib persona::motion_feedback` → 11 passed.
- `cargo test --lib` → **1860 passed** (was 1849, +11).
- `cargo clippy --lib -- -D warnings` → clean.
- `npx vitest run` → **1457 passed**, no regressions.

### What's next in 14.16

- 14.16f — Persona pack export already round-trips learned-motion
  artifacts; remaining polish is a provenance marker ("generated
  vs camera-mirrored") in the import-preview report so the
  receiving user can tell which motions came from the brain.

---

## Chunk 14.16d — Emotion-reactive procedural pose bias

**Date:** 2026-05-02
**Status:** ✅ Complete
**Brain-surface change:** No (frontend renderer only — reads
`characterStore.state` which is already populated by the existing
sentiment pipeline; nothing in the brain/RAG layer changes).

### What shipped

A self-contained `EmotionPoseBias` layer in
[src/renderer/emotion-pose-bias.ts](src/renderer/emotion-pose-bias.ts)
(~200 LOC) that adds a tiny additive postural bias on top of
whatever the `CharacterAnimator` + VRMA mixer produced in the same
frame — so a `happy` mood gets a small chest lift + relaxed shoulders,
`sad` gets a soft head/chest droop, `angry` tightens shoulders
inward, `relaxed` adds a lazy chest droop + slight head tilt, and
`surprised` opens up with a small backward lean + raised chin. Hard
cap of 0.18 rad per component (well under `pose_frame::CLAMP_RADIANS`'s
0.5) so the avatar stays expressive without ever looking puppeted.

- Pure mapping table `EMOTION_BIAS_TABLE` keyed by
  `BiasEmotion` (`'neutral' | 'happy' | 'sad' | 'angry' | 'relaxed' |
  'surprised'`) over the canonical 11-bone rig used by every
  LLM-as-Animator stage. Symmetric left/right shoulder-Z values
  enforced by a unit test.
- Stateful `EmotionPoseBias` class with damped-spring blender
  (λ=4 → ~0.25 s ramp), `setEmotion(emotion, intensity)`, `yield()`,
  and `apply(vrm, dt, suppress?)`. The `suppress` flag yields
  unconditionally so a baked VRMA clip or an active `PoseAnimator`
  pose always wins.
- Wired into
  [src/components/CharacterViewport.vue](src/components/CharacterViewport.vue):
  the `characterStore.state` watcher pushes the mapped emotion into
  the bias, and the per-frame loop calls
  `emotionBias.apply(vrm, delta, vrmaManager.isPlaying ||
  poseAnimator.isActive)` *after* `animator.update` and the LLM pose
  blender, so all four animation layers compose cleanly.

### Verification

- 14 new vitest cases in
  [src/renderer/__tests__/emotion-pose-bias.test.ts](src/renderer/__tests__/emotion-pose-bias.test.ts)
  covering: cap invariant, neutral = zeros, happy/sad sign
  expectations, left/right shoulder symmetry, intensity scaling,
  intensity clamp to [0, 1], non-finite input safety, weight ramp
  on activation, fade on `yield()` / `suppress=true` /
  `setEmotion('neutral')` / `intensity = 0`, null-VRM safety.
- `npx vitest run` → **1457 passed** (was 1443, +14).
- `npx vue-tsc --noEmit` → only the three pre-existing
  unrelated errors (`FirstLaunchWizard.vue` unused imports +
  `PluginsView.test.ts` enum literal type).

### What's next in 14.16

- 14.16e — Self-improve learning loop: capture user accept/reject
  signals on generated motions and feed them back into the prosody
  / persona analyser so the brain learns the user's preferred
  movement vocabulary over time.
- 14.16f — Persona pack already round-trips `learned-motion`
  artifacts; this chunk is a docs/UX polish pass to surface the
  generator's library entries with provenance markers ("generated
  vs camera-mirrored") in the import preview.

---

## Chunk 14.16c2 + c3 — `generate_motion_from_text` command + Persona-panel UI

**Date:** 2026-05-02
**Status:** ✅ Complete
**Brain-surface change:** No (persona/animation pipeline; reuses
`memory::brain_memory::complete_via_mode` for one-shot brain
routing).

### What shipped

Closes chunk 14.16c (Motion Library: LLM-Generated Clip Catalogue)
by wiring the 14.16c1 pure-Rust foundation into a Tauri command
and a Persona-panel UI so the user can type "wave hello" and the
active brain produces a multi-frame VRM motion clip the avatar can
perform.

**Backend (14.16c2):**

- New Tauri command `generate_motion_from_text` in
  [src-tauri/src/commands/persona.rs](src-tauri/src/commands/persona.rs)
  taking `description: String, duration_s: Option<f32>, fps:
  Option<u32>` and returning a `GeneratedMotionEnvelope`
  (`motion_json`, `trigger`, parser-cleanup diagnostic counts).
- Routes through the existing
  [`memory::brain_memory::complete_via_mode`](src-tauri/src/memory/brain_memory.rs)
  helper (now `pub`) so all four brain modes (Free / Paid / Local
  Ollama / LM Studio) work without bespoke per-mode wiring.
- Falls back to the legacy `active_brain` Ollama model when no
  `brain_mode` is configured, so users on the old single-mode
  setup still get the feature.
- Calls `motion_clip::build_motion_prompt` for the
  prompt + `parse_motion_payload` for the reply — both shipped
  in 14.16c1 with full unit-test coverage.
- Never auto-saves: returns the candidate as JSON the frontend
  previews, then commits via the existing `save_learned_motion`
  command on Accept (same human-in-the-loop contract as
  `extract_persona_from_brain`).
- Registered in `lib.rs` invoke handler list.

**Frontend (14.16c3):**

- New action `generateMotionFromText(description, opts)` in
  [src/stores/persona.ts](src/stores/persona.ts) that invokes the
  command and parses the envelope into a `LearnedMotion` plus
  diagnostics struct.
- New convenience action `saveLearnedMotion(motion)` so callers
  don't have to duplicate the `invoke('save_learned_motion', ...)`
  + library-merge plumbing that `PersonaTeacher.vue` already does.
- New component
  [src/components/PersonaMotionGenerator.vue](src/components/PersonaMotionGenerator.vue)
  embedded in `PersonaPanel.vue` next to the learned-motions
  library: text input, duration / FPS controls, Generate button,
  preview card with name + trigger + frame count + cleanup
  diagnostics, and Play / Accept / Discard actions.
- Reuses the existing `LearnedMotionPlayer` runtime via
  `store.requestMotionPreview(candidate)` — the same cross-view
  bridge `PersonaPanel`'s "▶ Play" button on saved motions uses,
  so no new player code was needed.

### Verification

- `cargo check --lib` → clean.
- `cargo clippy --lib -- -D warnings` → clean.
- `cargo test --lib` → **1849 passed, 0 failed**.
- `npx vitest run` → **1443 passed, 0 failed**.
- `npx vue-tsc --noEmit` → only the three pre-existing errors
  unrelated to this chunk (`FirstLaunchWizard.vue` unused
  imports + `PluginsView.test.ts` enum literal type).

### What's next in 14.16

- 14.16d — Emotion-reactive procedural blending: the
  `PoseAnimator` already supports per-frame additive offsets;
  next we drive it from the live VRM expression weights so a
  "happy" sentiment automatically biases the chest / shoulders
  upward, etc.
- 14.16e — Self-improve learning loop: when a user repeatedly
  rejects generated motions for the same trigger, capture the
  before/after preferences as training data for the persona
  prosody analyser.
- 14.16f — Library shareable via persona pack (the existing
  pack already includes `learned-motion` artifacts, so this is
  a docs/UX polish chunk rather than new code).

---

## Chunk 14.16c1 — Motion clip parser/validator foundation

**Date:** 2026-05-02
**Status:** ✅ Complete
**Brain-surface change:** No (persona/animation pipeline; touches the
brain prompt template only).

### What shipped

Pure-Rust foundation for chunk 14.16c (Motion Library:
LLM-Generated Clip Catalogue). Parses + validates the JSON the
brain returns when asked for a multi-frame VRM animation clip,
producing a [`GeneratedMotion`] struct that serialises to the
exact shape the frontend `LearnedMotion` Pinia store expects —
so the (next) `generate_motion_from_text` Tauri command can hand
the output straight to `save_learned_motion` without further
validation.

- New module: [src-tauri/src/persona/motion_clip.rs](src-tauri/src/persona/motion_clip.rs) (~440 LOC).
- Public API:
  - Constants: `MIN_FRAMES = 2`, `MAX_FRAMES = 240`,
    `DEFAULT_FPS = 24`, `MAX_FPS = 60`, `DEFAULT_DURATION_S = 3.0`,
    `MAX_DURATION_S = 30.0`.
  - `MotionRequest { description, duration_s, fps }` with a
    `sanitised()` method that clamps every field.
  - `build_motion_prompt(&MotionRequest) -> (system, user)` —
    pure prompt builder, unit-testable without the network.
  - `parse_motion_payload(payload, id, name, trigger, fps,
    duration_s, learned_at)` — forgiving parser; strips markdown
    code fences, drops unknown bones, clamps Eulers, repairs
    non-monotonic timestamps, renormalises duration when the LLM
    overshoots.
  - `slugify_trigger(description) -> String` — prefixes
    `learned-` so generated triggers can never collide with the
    canonical motion list.
  - `GeneratedMotion`, `GeneratedFrame`,
    `MotionParseDiagnostics`, `MotionParseError`
    (`thiserror::Error`).
- Re-uses [`crate::persona::pose_frame`] for canonical bones and
  the ±0.5 rad clamp constant — single source of truth across the
  pose + clip pipelines.
- Module wired in [src-tauri/src/persona/mod.rs](src-tauri/src/persona/mod.rs).
- 18 unit tests cover prompt-content invariants, minimal 2-frame
  parse, drop-unknown-bones, ±range clamp, non-finite handling,
  empty-frame skip, monotonic-repair, min/max frame rejection,
  invalid-JSON / missing-frames hard failures, markdown-fence
  strip, duration overshoot renormalisation, slug edge cases,
  request sanitiser, fps-zero default, and a serde round-trip
  asserting the JSON shape matches the frontend `LearnedMotion`
  field names exactly.

### Verification

- `cargo test --lib persona::motion_clip` → 18 passed.
- `cargo test --lib` → **1849 passed, 0 failed** (was 1831).
- `cargo clippy --lib -- -D warnings` → clean.

### What's next in 14.16c

14.16c2 — wire the foundation into a `generate_motion_from_text`
Tauri command: dispatch to the active brain mode (Ollama /
OpenAI / FreeProvider), parse the reply, persist via
`save_learned_motion`, and emit an `llm-motion-generated` event so
the frontend can preview + accept/reject the clip.

---

## Chunk 14.16b3 — Frontend `PoseAnimator` + `llm-pose` wiring

**Date:** 2026-05-02
**Status:** ✅ Complete
**Brain-surface change:** No (rendering pipeline only).

### What shipped

Closes out chunk 14.16b: the LLM can now drive the VRM body with
`<pose>` tags during streaming. The Rust pipeline (14.16b1 + 14.16b2)
emits validated `LlmPoseFrame` payloads on the `llm-pose` Tauri
event; this chunk adds the frontend blender that consumes them and
layers them on top of the procedural idle animation.

- New module: [src/renderer/pose-animator.ts](src/renderer/pose-animator.ts) (~270 LOC).
  - `PoseAnimator` class: damped-spring blender (λ=6) with explicit
    fade-in (0.3s) / hold (`duration_s`, 0.05–10s) / fade-out (0.5s)
    phases. Supports `linear`, `ease-in-out`, and `spring` easing.
  - Additive bone offsets layered on top of `CharacterAnimator` (the
    blender mutates `node.rotation.x/y/z` after the procedural
    animator has written its values).
  - VRMA-yield: `setVrmaPlaying(true)` triggers a fade-out so the
    baked clip drives bones unmodified; new frames received while
    VRMA is playing are silently dropped.
  - Defence-in-depth sanitisation: clamps any non-canonical bone,
    out-of-range Euler, or non-finite value (Rust already does this,
    but the frontend never trusts wire data either).
  - Optional VRM expression weights are applied through
    `expressionManager.setValue` and clamped to `[0, 1]`.
- Wired into [src/components/CharacterViewport.vue](src/components/CharacterViewport.vue):
  - Constructed alongside `CharacterAnimator` + `VrmaManager`.
  - `vrmaManager.onPlaybackChange` now also calls
    `poseAnimator.setVrmaPlaying`.
  - `poseAnimator.apply(vrmaManager.vrm, delta)` runs each frame
    after `animator.update`.
  - New `playPose(frame)` and `clearPose()` methods exposed via
    `defineExpose`.
- Wired into both chat surfaces:
  - [src/views/ChatView.vue](src/views/ChatView.vue) — new `llm-pose`
    Tauri-event listener forwards frames to
    `viewportRef.value?.playPose`. Listener cleanup added.
  - [src/views/PetOverlayView.vue](src/views/PetOverlayView.vue) —
    same wiring + cleanup.
- New unit tests: [src/renderer/pose-animator.test.ts](src/renderer/pose-animator.test.ts)
  (14 tests) cover initial state, fade-in target convergence,
  drop-unknown-bones, ±0.5 rad clamp, non-finite → 0, no-recognised
  rejection, full lifecycle fade back to idle, VRMA yield, frame
  drop while VRMA active, expression apply + clamp, `reset()`,
  null-VRM safety, and replace-active-pose damping.

### Verification

- `npx vitest run src/renderer/pose-animator.test.ts` → 14 passed.
- `npx vitest run` → **1443 passed** (was 1429).
- `npx vue-tsc --noEmit` → no new errors (3 pre-existing unrelated).

### Chunk 14.16b status

14.16b is fully shipped (b1 parser foundation → b2 stream-tag
integration → b3 frontend blender). The next sub-chunk in the
14.16x family is **14.16c — Motion Library: LLM-Generated Clip
Catalogue** (a new `generate_motion_from_text` Tauri command that
bakes multi-frame clips into `LearnedMotion` entries).

---

## Chunk 14.16b2 — `<pose>` tag in StreamTagParser + `llm-pose` event

**Date:** 2026-05-02
**Status:** ✅ Complete
**Brain-surface change:** No (animation pipeline; surface-level system-prompt addition only).

### What shipped

Extends the Rust streaming pipeline so the LLM can emit
`<pose>{ ... }</pose>` blocks alongside the existing `<anim>` tags.
The pose JSON is parsed + clamped through the foundation shipped in
14.16b1, and validated frames are emitted as a new `llm-pose`
Tauri event for the frontend `PoseAnimator` (next sub-chunk) to
consume.

- [src-tauri/src/commands/streaming.rs](src-tauri/src/commands/streaming.rs):
  - `StreamTagParser` refactored from single-tag to multi-tag
    state machine. New `BlockKind::{Anim, Pose}` enum drives the
    inner state; `find_earliest_open` picks whichever tag opens
    first; partial-prefix hold-back now considers all known
    openers so a streamed `<pos` chunk doesn't leak into chat
    text.
  - New `StreamFeed { text, anim_commands, pose_frames }` return
    type — replaces the old `(String, Vec<AnimationCommand>)`
    tuple. All four call sites (OpenAI streaming, Ollama
    streaming, plus their flush paths) updated to emit `llm-pose`
    when the parser yields a frame.
  - `strip_anim_blocks` extended to also strip `<pose>` blocks
    from the text persisted to conversation history.
- [src-tauri/src/commands/chat.rs](src-tauri/src/commands/chat.rs):
  System prompt updated with the `<pose>` schema + bone list +
  clamp note ("±0.3 recommended; renderer hard-clamps ±0.5 rad").
- 6 new unit tests cover pose-only, pose split across chunks,
  out-of-range clamp, invalid JSON drop, mixed `<anim>` +
  `<pose>` in one chunk, and partial-open-tag hold-back. All 8
  existing parser tests migrated to the new `StreamFeed` API.

### Verification

- `cargo test --lib commands::streaming` → 29 passed.
- `cargo test --lib` → **1831 passed, 0 failed** (was 1825).
- `cargo clippy --lib -- -D warnings` → clean.

### What's next in 14.16b

14.16b3 — frontend `PoseAnimator` class (damped-spring blender,
VRMA-yield logic) + `llm-pose` event listener + `CharacterViewport`
wiring. The Rust contract is now frozen, so the frontend chunk can
proceed independently.

---

## Chunk 14.16b1 — Pose-frame parser foundation (LLM-as-Animator)

**Date:** 2026-05-02
**Status:** ✅ Complete
**Brain-surface change:** No (persona/animation pipeline; not retrieval).

### What shipped

Pure-Rust pose-frame parser + clamp module that the upcoming
frontend `PoseAnimator` (chunk 14.16b) and the offline
`generate_motion_from_text` Tauri command (chunk 14.16c) will both
consume. Locks the canonical 11-bone VRM contract (head, neck,
spine, chest, hips, left/right Upper/Lower Arm, left/right
Shoulder), enforces the ±0.5 rad safety clamp from
`docs/llm-animation-research.md`, and provides a forgiving JSON
parser that drops unknown bones, clamps out-of-range Eulers, and
replaces non-finite values with 0 — so a noisy LLM still produces a
renderable pose.

- New module: [src-tauri/src/persona/pose_frame.rs](src-tauri/src/persona/pose_frame.rs) (~480 LOC).
- Public API:
  - `CANONICAL_BONES: &[&str]` — 11-entry rig contract.
  - `CLAMP_RADIANS = 0.5`, `DEFAULT_DURATION_SECS = 2.0`,
    `MAX_DURATION_SECS = 10.0`.
  - `PoseEasing` (Linear / EaseInOut / Spring; default Spring;
    serde kebab-case).
  - `LlmPoseFrame { bones, duration_s, easing, expression }` —
    deterministic `BTreeMap` ordering for golden-vector tests.
  - `PoseParseResult { frame, dropped_bones, clamped_components }`.
  - `PoseParseError` (InvalidJson / MissingBones / NoRecognisedBones,
    `thiserror::Error`).
  - `parse_pose_payload(&str)` — forgiving parser; only structural
    JSON failures are hard errors.
  - `extract_pose_payloads(&str)` — case-insensitive `<pose>...</pose>`
    extractor for streamed responses.
- Module wired in [src-tauri/src/persona/mod.rs](src-tauri/src/persona/mod.rs).
- 24 unit tests cover canonical rig invariants, full + bare frame
  shapes, unknown-bone drop, ±range clamp, non-finite handling,
  duration floor/ceiling, expression-weight clamp, easing default +
  fallback, every `extract_pose_payloads` edge case (single, multi,
  case-insensitive, missing, unclosed), serde round-trip, and an
  end-to-end extract → parse pipeline against a simulated streamed
  response.

### Verification

- `cargo test --lib persona::pose_frame` → 24 passed.
- `cargo test --lib` → **1825 passed, 0 failed** (was 1801).
- `cargo clippy --lib -- -D warnings` → clean.

### Why this lands as a separate sub-chunk

14.16b is a multi-day frontend-heavy chunk (PoseAnimator,
CharacterViewport wiring, StreamTagParser pose forwarding,
damped-spring blender). Shipping the pure-Rust parser now de-risks
the contract: every later chunk in the 14.16x family reuses the
same canonical bone list, clamp constants, and parse helpers — and
failures are caught at the Rust boundary before any pose ever
reaches the renderer.

---

## Chunk 16.6c — Wire query-intent classifier into hybrid_search_rrf

**Date:** 2026-05-01 · **Phase:** 16 (Modern RAG) · **Tests:** 1801 (+5 new), clippy clean

### What shipped

New method `MemoryStore::hybrid_search_rrf_with_intent` that wires the
16.6b query-intent classifier into the RRF fusion pipeline. Per
`docs/brain-advanced-design.md` §3.5.6.

### How it works

1. Run `query_intent::classify_query(query)` to get an
   `IntentClassification { intent, confidence, kind_boosts }`.
2. Run the standard 3-signal RRF fusion (vector + keyword + freshness).
3. **If** intent is anything other than `Unknown`, multiply each fused
   score by `kind_boosts.for_kind(doc.cognitive_kind)` where the doc's
   cognitive kind is computed by `cognitive_kind::classify`. Re-sort.
4. **If** intent is `Unknown`, skip rerank entirely — method is
   identical to plain `hybrid_search_rrf`.

### Files

- `src-tauri/src/memory/store.rs` — added
  `hybrid_search_rrf_with_intent` (~120 LOC).
- 5 new tests:
  - `..._unknown_matches_plain_rrf` — unknown intent doesn't perturb ordering.
  - `..._zero_limit_returns_empty`
  - `..._empty_store_returns_empty`
  - `..._boosts_procedural_kind` — a how-to query promotes a procedural
    memory above a generic factoid that shares the same keywords.
  - `..._deterministic` — same query twice returns same id list.

### Backward compatibility

Additive only. Existing `hybrid_search_rrf` unchanged. New method is
opt-in — callers can migrate at their own pace. Safe drop-in for any
caller that wants kind-aware ranking with zero risk on `Unknown`-intent
queries.

### Quality gates

- `cargo test --lib` — 1801 passed (was 1796, +5 new)
- `cargo clippy --lib -- -D warnings` — clean

---

## Chunk 14.16a — LLM-driven 3D animation research & taxonomy

**Date:** 2026-05-01 · **Phase:** 14 (Persona, Self-Learning Animation & Master-Mirror) · **Tests:** n/a (research-only deliverable)

### What shipped

Research deliverable for Chunk 14.16a per `rules/milestones.md` Phase 14
spec. Surveys eight state-of-the-art techniques for LLM-driven 3D
character animation, classifies them into four families, picks the
v1 implementation path, and locks the canonical 11-bone VRM contract.

### Files

- `docs/llm-animation-research.md` (NEW, ~270 lines):
  - Comparison matrix (8 techniques: MotionGPT, MotionDiffuse, MoMask,
    AI4Animation MANN, T2M-GPT, LLM-as-animator, Hunyuan-Motion,
    PriorMDM) with latency estimates, model sizes, license notes.
  - VRM bone-mapping strategy (matches Master-Mirror's 11-bone
    contract).
  - Latency budget per sub-chunk.
  - Recommended implementation order: 14.16b → 14.16c → 14.16d →
    14.16e → 14.16f.
  - Risks & mitigations (non-anatomical poses, tag collisions, VRMA
    precedence, GPU-only models, SMPL-X licensing).
  - Out-of-scope items (locomotion, fingers, multi-character, VR/AR).
  - 9 references (papers + cross-doc links).

### Key decisions locked

1. **Canonical surface = 11 upper-body VRM bones** (head, neck, spine,
   chest, hips, leftUpperArm, rightUpperArm, leftLowerArm,
   rightLowerArm, leftShoulder, rightShoulder). Same contract as
   Chunk 14.7 Master-Mirror — no separate skeleton format.
2. **v1 path = LLM-as-animator (14.16b)** — zero install weight, zero
   added latency, reuses the chat brain.
3. **Diffusion / token-based techniques are capability-gated** and
   ship in later sub-chunks; they never block 14.16b.
4. **Pose values clamped to ±0.5 rad** in `PoseAnimator` to prevent
   non-anatomical output regardless of LLM behaviour.

### Quality gates

- Research-only deliverable. No code, no tests.
- Doc passes spell-check; all internal links resolve.

---

## Chunk 17.8 — V11 schema: category column + index

**Date:** 2026-04-30 · **Phase:** 17 (Brain Phase-5 Intelligence) · **Tests:** 1796 (+2 new), clippy clean

### What shipped

Migration **V11** — adds a dedicated `category TEXT` nullable column to
`memories` plus a B-tree index `idx_memories_category`. Per
`docs/brain-advanced-design.md` §3 "Proposed Category Taxonomy".

No backfill in this migration — entries without a category continue to
rely on the existing `category:` tag-prefix convention. Backfill runs
lazily (or via the brain maintenance scheduler) in a follow-up.

### Files

- `src-tauri/src/memory/migrations.rs` — appended Migration { version: 11, ... }
- Sentinel test `target_version_is_v11` updated.
- New tests:
  - `v11_adds_category_column_and_index` — verifies column accepts values + EXPLAIN QUERY PLAN confirms `idx_memories_category` is used.
  - `v11_round_trip_drops_column_and_index` — V11 → V10 downgrade drops index + column without losing rows.

### Quality gates

- `cargo test --lib memory::migrations` — 16 passed (+2 new for V11)
- `cargo test --lib` — 1796 passed
- `cargo clippy --lib -- -D warnings` — clean

---

## Chunk 17.7b — V10 schema: Obsidian sync metadata columns

**Date:** 2026-04-30 · **Phase:** 17 (Brain Phase-5 Intelligence) · **Tests:** 1796 (+2 new), clippy clean

### What shipped

Migration **V10** — adds two nullable columns to `memories` for
bidirectional Obsidian sync tracking, per `docs/brain-advanced-design.md`
§8 (On-Disk Schema & Storage Layout):

- `obsidian_path TEXT` — relative path inside the user's Obsidian vault
  (e.g. `daily/2026-04-30.md`). NULL = never exported.
- `last_exported INTEGER` — Unix-ms timestamp of the most recent
  successful write. NULL = never exported. The bidirectional sync
  watcher in chunk 17.7 will compare this against the .md file's mtime
  for LWW conflict resolution.

No new index — the export pass scans a small (<1 % of memories) subset.

### Files

- `src-tauri/src/memory/migrations.rs` — appended Migration { version: 10, ... }
- New tests:
  - `v10_adds_obsidian_sync_columns` — both columns accept NULL + non-NULL values.
  - `v10_round_trip_drops_columns_and_preserves_rows` — V10 → V9 downgrade drops columns; re-upgrade restores them as NULL.
- Side fix: converted flaky `intent_classifier::cache_short_circuits_classification` to a sync-only test. The async version was racing against `set_brain_mode` and other commands that call `intent_classifier::clear_cache()` without acquiring the test lock. The cache contract is synchronous, so the test no longer needs `.await`.

### Quality gates

- `cargo test --lib memory::migrations` — 16 passed (+2 new for V10)
- `cargo test --lib` — 1796 passed
- `cargo clippy --lib -- -D warnings` — clean

---

## Chunk 16.9b — Embedding-model fallback chain

**Date:** 2026-04-30 · **Phase:** 16 (Modern RAG) · **Tests:** 1792 (+4 new), clippy clean

### What shipped

Per `docs/brain-advanced-design.md` §4 resilience notes — when
`nomic-embed-text` is unavailable, the embed-model resolver now walks a
fallback chain of dedicated open-source embedders before falling through
to the chat-model hint.

### Resolution chain

1. **`nomic-embed-text`** (preferred, 768d) — unchanged.
2. **`mxbai-embed-large`** (1024d, strong general-purpose).
3. **`snowflake-arctic-embed`** (1024d / 768d depending on tag).
4. **`bge-m3`** (1024d, multilingual).
5. **`all-minilm`** (384d, tiny last-resort).
6. **`model_hint`** (active chat model — almost always rejects embed
   requests; `embed_text` then marks it unsupported and the caller falls
   back to keyword-only retrieval).

### Files

- `src-tauri/src/brain/ollama_agent.rs`:
  - Added `EMBED_MODEL_FALLBACKS: &[&str]` constant.
  - Refactored `resolve_embed_model` to delegate to new `pick_embed_model`
    helper that walks the chain and skips models in the unsupported set.
  - +4 tests (`fallback_chain_falls_through_to_hint_when_nothing_installed`,
    `fallback_chain_skips_known_unsupported_preferred`,
    `fallback_chain_constants_are_well_formed`,
    `fallback_chain_skips_unsupported_fallbacks`).
- Side fix: `intent_classifier::cache_short_circuits_classification` was
  flaky under parallel test execution (race between `cache_put` and the
  `await` boundary). Now holds the sync `cache_test_lock()` across the
  `.await` with `#[allow(clippy::await_holding_lock)]` — safe because
  `#[tokio::test]` uses current-thread runtime.

### Quality gates

- `cargo test --lib` — 1792 passed (was 1788, +4 new)
- `cargo clippy --lib -- -D warnings` — clean

---

## Chunk 16.6b — Query-intent classifier for retrieval ranking

**Date:** 2026-04-30 · **Phase:** 16 (Modern RAG) · **Tests:** 1788 (+19 new), clippy clean

### What shipped

Pure-logic query-intent classifier per `docs/brain-advanced-design.md` §3.5.6.
Classifies user queries as `Procedural`, `Episodic`, `Factual`, `Semantic`, or
`Unknown` and emits per-`CognitiveKind` boost multipliers callers can apply
during RRF fusion.

### Files

- `src-tauri/src/memory/query_intent.rs` (NEW, ~360 LOC, 19 tests)
  - Types: `QueryIntent`, `IntentClassification`, `KindBoosts`
  - Function: `classify_query(query: &str) -> IntentClassification`
- `src-tauri/src/memory/mod.rs` — registered `pub mod query_intent;`

### Heuristic

Heuristic-based with no LLM call. Matches lower-cased query against:
- **Procedural prefixes** ("how to", "how do i", "walk me through")
- **Procedural verbs** ("install", "configure", "build", "deploy", "step-by-step")
- **Episodic prefixes** ("when did", "what did i say")
- **Episodic time anchors** ("yesterday", "last week", " ago", " on monday")
- **Factual prefixes** ("what is", "who is", "define")
- **Semantic prefixes** ("why", "explain", "describe", "compare")

Each heuristic contributes to a score; highest score wins. Confidence is
normalised to 0.0–1.0. Empty/no-signal queries return `Unknown` with neutral
boosts (1.0 across all kinds), letting the caller fall back to default RRF.

### Boost shape per intent

| Intent | Procedural | Episodic | Semantic |
|---|---|---|---|
| Procedural | 1.5 | 0.8 | 1.0 |
| Episodic | 0.8 | 1.6 | 0.9 |
| Factual | 0.9 | 0.9 | 1.3 |
| Semantic | 1.0 | 0.95 | 1.2 |
| Unknown | 1.0 | 1.0 | 1.0 |

### Integration notes

This is a **pure utility chunk** — no callers wired yet. Future RAG/RRF
integration (16.6 GraphRAG or 16.4b Self-RAG) can call `classify_query`
on the user's question and multiply each candidate doc's RRF score by
`boosts.for_kind(doc.cognitive_kind)` before final ranking.

### Quality gates

- `cargo test --lib memory::query_intent` — 19 passed
- `cargo test --lib` — 1788 passed (was 1769)
- `cargo clippy --lib -- -D warnings` — clean

---

## Chunk 25.12 — Brain data migration & maintenance scheduler

**Date:** 2026-04-30 · **Phase:** 25 (Self-Improve Autonomous Coding) · **Tests:** 1769 (+16 new), clippy clean

**What.** Pure-logic background maintenance scheduler for the brain data layer. Provides the scheduling/reporting decisions the autonomous loop needs to:

1. **Schema migrations** — `check_migration_needed(current, target)` reports if migrations are pending.
2. **ANN health checks** — `check_ann_health(stored_dim, expected_dim, index_count, db_count)` detects dimension mismatches and out-of-sync indices.
3. **GC eligibility** — `gc_eligible_count(...)` counts entries past the retention cutoff.
4. **Pass orchestration** — `execute_maintenance_pass(...)` runs the full check sequence and returns a `MaintenanceReport`.

**Key types:** `BrainMaintenanceConfig`, `MaintenanceReport`, `MaintenanceStatus`, `MaintenanceState`, `MigrationCheckResult`, `AnnHealthCheck`, `should_run()`.

**Side effect:** Discovered that `memory::replay` (Chunk 26.4) was implemented but never registered in `memory/mod.rs`, breaking the full library build. Registered it as part of this chunk so the autonomous maintenance task can run.

**File:** `src-tauri/src/memory/brain_maintenance.rs` (~370 LOC, 16 unit tests). Plus `memory/mod.rs` registration of `replay`.

---

## Chunk 25.11 — MCP server self-host & dynamic tool registry

**Date:** 2026-04-30 · **Phase:** 25 (Self-Improve Autonomous Coding) · **Tests:** 1753 (+11 new), clippy clean

**What.** Enables the self-improve loop to auto-spawn a local MCP server and extend its own tools at runtime. Key components:

1. **`DynamicToolRegistry`** — thread-safe (Arc<RwLock>) registry that allows registering/unregistering tools at runtime. Tools can have static responses or (in future) trait-object handlers.

2. **Self-improve tools** — `register_self_improve_tools()` adds 3 standard tools: `self_improve_status`, `self_improve_history`, `self_improve_metrics`. External AI assistants can query coding workflow progress via MCP.

3. **`McpAutoSpawnConfig`** — configuration for auto-start behaviour (enabled flag, port override, tool registration toggle).

**File:** `src-tauri/src/ai_integrations/mcp/self_host.rs` (~310 LOC, 11 unit tests).

---

## Chunk 28.5 — GitHub PR flow (OAuth device + per-chunk PRs)

**Date:** 2026-04-30 · **Phase:** 28 (Self-Improve Loop Maturation) · **Tests:** 1742 (+9 new github tests), clippy clean

**What.** Completed the GitHub PR automation module with two major additions:

1. **OAuth Device Authorization Grant (RFC 8628)** — `request_device_code()` and `poll_for_token()` implement the full device flow so users can authenticate without manually creating a PAT. Types: `DeviceCodeResponse`, `DevicePollResult`, `OAuthDeviceConfig`.

2. **Per-chunk PR generation** — `build_chunk_pr_title(record)` and `build_chunk_pr_body(record, metrics)` generate structured PR titles and markdown bodies from `RunRecord` + `MetricsSummary`. Includes chunk ID, duration, token usage, cost, and session metrics table. `chunk_branch_name()` sanitizes chunk IDs into valid git branch names.

**File:** `src-tauri/src/coding/github.rs` (extended from ~450 to ~790 LOC, 14 total tests).

---

## Chunk 27.2 — Context engineering budget-aware assembly

**Date:** 2026-04-30 · **Phase:** 27 (Agentic RAG & Context Engineering) · **Tests:** 1733 (+14 new), clippy clean

**What.** Bridges `coding::context_budget` with `coding::prompting` to provide budget-aware prompt assembly. Converts task/document/plan inputs into priority-ranked `ContextSection`s, runs `fit_to_budget`, and rewrites prompt documents with the surviving subset. Includes label-based priority inference heuristics.

**Key functions:** `budget_aware_assembly(prompt, extra_docs, config) -> AssemblyResult`, `auto_budget_assembly(prompt, config) -> AssemblyResult`, `infer_priority(label) -> SectionPriority`.

**Key types:** `PrioritisedDoc`, `AssemblyResult`.

**File:** `src-tauri/src/coding/context_engineering.rs` (~280 LOC, 14 unit tests).

---

## Chunk 28.3 — Multi-agent DAG runner

**Date:** 2026-04-30 · **Phase:** 28 (Self-Improve Loop Maturation) · **Tests:** 1719 (+21 new), clippy clean

**What.** DAG-based workflow runner that executes nodes respecting dependency edges. Independent nodes can run in parallel (layer-based scheduling); dependent nodes wait for predecessors. Includes cycle detection (Kahn's algorithm), capability validation, skip-on-failure propagation, and configurable max parallelism.

**Key types:** `WorkflowGraph { nodes, edges }`, `DagNode`, `DagEdge`, `NodeStatus` (Pending/Running/Success/Failed/Skipped), `NodeResult`, `DagRunResult`, `DagRunnerConfig`, `DagValidationError`.

**Key functions:** `validate_graph()`, `compute_layers()`, `execute_dag_sync()`.

**File:** `src-tauri/src/coding/dag_runner.rs` (~380 LOC, 21 unit tests — 9 validation, 3 layer computation, 7 execution, 2 serde).

---

## Chunk 28.2 — Coding intent router

**Date:** 2026-04-30 · **Phase:** 28 (Self-Improve Loop Maturation) · **Tests:** 1698 (+18 new), clippy clean

**What.** Detects coding intent from chat messages and routes them to the coding-workflow orchestrator. Analyses message text for implementation signals (keywords like "implement", "fix", "refactor", "test"), file extensions, chunk references, and capability requirements.

**Key types:** `IntentConfidence` (None/Low/Medium/High), `CodingIntent`, `CodingCapability` (FileWrite/TestRun/GitOps/LlmCall/ShellExec), `SuggestedShape` (Plan/BareFileContents/StrictJson), `RoutingDecision` (AutoRoute/Confirm/PassThrough), `RouterConfig`.

**Key functions:** `detect_intent(message) -> CodingIntent`, `route(message, config) -> RoutingDecision`.

**File:** `src-tauri/src/orchestrator/coding_router.rs` (~320 LOC, 18 unit tests).

---

## Chunk 14.15 — MotionGPT motion token codec

**Date:** 2026-04-30 · **Phase:** 14 (Persona, Self-Learning Animation) · **Tests:** 1701 (+13 new), clippy clean (feature-gated)

**Why.** MotionGPT-style discrete motion tokens let the LLM *generate*
motion sequences by emitting quantized bone-angle tokens in plain text.
This enables the brain to author new animations from natural language
descriptions — the final piece of the self-learning animation research
trilogy (14.13 smoothing → 14.14 retarget → 14.15 generation).

**What landed.**

- `src-tauri/src/persona/motion_tokens.rs` (~370 LOC):
  - `TokenCodecConfig` — n_bins (default 32), per-bone joint-limit ranges
  - `MotionTokenCodec` — `encode_frame()`, `decode_frame()`, `encode_clip()`, `decode_clip()`
  - `to_text()` / `from_text()` — LLM-friendly `<motion_tokens>` format with pipe-separated bones
  - `build_vocabulary_prompt()` — generates system prompt describing the token vocabulary
  - Uniform quantization: maps joint angles to discrete bins via per-bone [min, max] ranges
- Feature-gated behind `motion-research` (off by default)
- 13 unit tests covering encode/decode roundtrips, extremes, custom bins, text serialization, parse validation

**Anti-drift.** Phase 14 rows 14.13/14.14/14.15 + heading archived from `rules/milestones.md`.

---

## Chunk 14.14 — Full-body retarget from BlazePose landmarks

**Date:** 2026-04-30 · **Phase:** 14 (Persona, Self-Learning Animation) · **Tests:** 1701 (+12 new), clippy clean (feature-gated)

**Why.** MediaPipe BlazePose gives 33 keypoints from a webcam feed, but
VRM avatars use a named bone hierarchy. This module bridges the gap with
geometric IK (two-bone for limbs, look-at for spine/head), anatomical
joint limits, and partial-visibility graceful degradation — enabling
real-time motion capture without ML model inference.

**What landed.**

- `src-tauri/src/persona/retarget.rs` (~380 LOC):
  - `Landmark` (x/y/z + visibility), `EulerTriple`, `VrmBonePose` (17 named bones)
  - `RetargetConfig` — depth_scale, visibility_threshold, enable_joint_limits
  - `retarget_pose(landmarks, config) -> Option<VrmBonePose>` — single-frame retarget
  - `retarget_sequence()` — batch processing for recorded clips
  - `two_bone_ik()` — geometric IK for arm/leg chains
  - `direction_to_euler()` — direction vector → pitch/yaw
  - Anatomical joint limits (shoulder ±180°, elbow 0–150°, knee 0–140°, etc.)
- Feature-gated behind `motion-research` (off by default)
- 12 unit tests covering T-pose, partial visibility, sequence batch, joint limits, serde roundtrips

---

## Chunk 14.13 — Offline motion-clip smoothing

**Date:** 2026-04-30 · **Phase:** 14 (Persona, Self-Learning Animation) · **Tests:** 1701 (+12 new), clippy clean (feature-gated)

**Why.** Raw motion-capture clips from webcam tracking are jittery.
A zero-phase Gaussian filter smooths the clip offline without introducing
phase lag — essential before baking captured motion into VRMA animations.
This is the foundation layer for the motion-processing research pipeline.

**What landed.**

- `src-tauri/src/persona/motion_smooth.rs` (~280 LOC):
  - `MotionFrame` (timestamp + 6 channels), `MotionClip`, `SmoothConfig`, `SmoothResult`
  - `smooth_clip(clip, config) -> SmoothResult` — main entry point
  - Two-pass (forward + backward) Gaussian convolution with reflection padding
  - Configurable sigma (default 1.5), kernel radius, pin-endpoints option
  - Preserves frame count and timing; returns max/mean smoothing delta stats
- Feature-gated behind `motion-research` (off by default)
- 12 unit tests covering identity (sigma=0), constant signal, impulse, short clips, pin endpoints, serde

---

## Chunks 21.1–21.4 — Doc & Completion-Log Hygiene bundle

**Date:** 2026-04-30 · **Phase:** 21 (Doc QA Audit)

**What landed (all log-only edits, no code changes):**

- **21.1** — Restored missing `## Chunk 14.7 — Persona Pack Export / Import` H2 heading. TOC anchor link was broken.
- **21.2** — Backfilled `Chunk 14.1 — Persona MVP` entry (PersonaTraits store + prompt injection + UI + Soul Mirror quest).
- **21.3** — Renumbered "Multi-Agent Resilience" entry → `Chunk 23.0 — Multi-agent resilience scaffold`. Clarified it's scaffold only; wiring chunks 23.1/23.3 still pending.
- **21.4** — Backfilled `Chunk 23.0b — Stop & Stop-and-Send Controls` (TaskControls component).
- **Bonus** — Removed a duplicate entry (MCP stdio content under wrong heading) that was corrupting the log.
- **Bonus** — Added machine-readable `<!-- BEGIN MODEL_CATALOGUE -->` / `<!-- BEGIN TOP_PICKS -->` tables to `docs/brain-advanced-design.md` fixing pre-existing `doc_catalogue` test failure.

Phase 21 heading removed from milestones (all items complete).

---

## Chunk 28.10 — Context budget manager for long coding sessions

**Date:** 2026-04-30 · **Phase:** 28 (Self-Improve Loop Maturation) · **Tests:** 1680 (+16 new), clippy clean

**Why.** Long autonomous coding workflows (10+ cycles) overflow the LLM's
context window. Models silently truncate, lose the thread of the task, and
start repeating prior work. The handoff codec (28.8/28.9) provides session-to-session
continuity but doesn't manage within-session budget. This module adds
priority-based pruning so workflows stay within budget across arbitrary
session lengths.

**What landed.**

- `src-tauri/src/coding/context_budget.rs` (~380 LOC):
  - `SectionPriority` enum (8 levels: Background → System, never-prune for System)
  - `ContextSection` — labeled chunk of context with priority + content + summarizable flag
  - `BudgetConfig` — max_tokens (24K default), response_reserve (4K), summary settings
  - `fit_to_budget(sections, config) -> BudgetResult` — priority-based pruning:
    lowest priority first, largest sections first within same priority, System never pruned
  - `BudgetResult` — kept/pruned partition + total_tokens + optional pruned summary
  - `SessionChain` — multi-session continuity: tracks workflow_id, session numbers,
    rolling summaries (capped at 10), cumulative tokens/steps. Builds handoff context sections.
  - `estimate_tokens()` — conservative 4 chars/token approximation
- Integration point: sits between `load_workflow_context()` and `CodingPrompt::build()`
- 16 unit tests covering: all-fit, priority pruning, size-within-priority, System immunity,
  response reserve, chain advance/cap/context, serde roundtrips, defaults, ordering

**Design.** Extends the 28.8/28.9 handoff system with *within-session* budget
management. The `SessionChain` type enables 10+ session workflows by keeping only
the last 10 session summaries (bounded memory). When a session ends, it calls
`chain.advance(summary, steps, tokens)` which appends to the chain and prunes
oldest entries. Next session prepends the chain as a Handoff-priority context section.

---

## Chunk 1.1 (Phase 12) — Brain Advanced Design QA screenshots

**Date:** 2026-04-30 · **Phase:** 12 (Brain Advanced Design — Documentation & QA)

**Why.** The brain walkthrough docs (`BRAIN-COMPLEX-EXAMPLE.md` and
`BRAIN-COMPLEX-EXAMPLE-LOCAL-LM.md`) need real screenshots showing the
full flow with Vietnamese legal content loaded — from fresh launch
through brain setup, memory ingestion, RAG-augmented chat in multiple
languages, to skill tree completion.

**What landed.**

- Verified 18 screenshots in `instructions/screenshots/` (Free API flow)
  and 18 in `instructions/screenshots/local-lm/` (LM Studio flow).
  All are real Playwright-captured images (200–270 KB each).
- Confirmed Vietnamese content screenshot (`11-vietnamese-answer.png`)
  shows authentic Vietnamese text with Article 429 Civil Code citations.
- All `![alt](path)` references in both walkthrough docs resolve to
  existing files — zero broken image links.
- Added missing `BRAIN-COMPLEX-EXAMPLE-LOCAL-LM.md` link to
  `docs/brain-advanced-design.md` "Related Documents" section.
- Playwright capture scripts (`scripts/brain-flow-screenshots.mjs`,
  `scripts/capture-brain-example-screenshots.mjs`,
  `scripts/capture-brain-local-lm-screenshots.mjs`) all present for
  re-capture if UI changes.

**Anti-drift.** Row 1.1 and Phase 12 heading archived from `rules/milestones.md`.

---

## Chunk 28.1 — Reviewer sub-agent

**Date:** 2026-04-30 · **Phase:** 28 (Self-Improve Loop Maturation) · **Tests:** 1664 / 1664 (+18 new), clippy lib clean

**Why.** The self-improve loop applies LLM-generated code directly to
the repo. Without a reviewer gate, bugs, security holes, and style
violations pass unchecked. Chunk 28.1 adds a second LLM pass that
produces a structured `{ ok, issues[] }` verdict — only clean diffs
proceed to `apply_file`.

**What landed.**

- New module [`src-tauri/src/coding/reviewer.rs`] (~340 LOC + 18 tests).
- **Types:** `Severity` (error/warning/info), `ReviewIssue` (severity +
  file + line + msg), `ReviewResult` (ok + issues), `ReviewVerdict`
  (Accept | Reject { reason, blocking_issues }), `ReviewerConfig`
  (reject_on_warnings, max_issues_in_reason).
- **Prompt builders:**
  - `build_review_task(task_id, diff, context_docs)` — returns a
    `CodingTask` with `OutputShape::StrictJson` review schema.
  - `build_review_prompt(diff, extra_docs)` — returns a raw
    `CodingPrompt` for direct LLM invocation.
- **Parser:** `parse_review_result(payload)` — lenient JSON parse
  accepting both raw JSON and `<json>`-wrapped payloads.
- **Decision logic:** `decide(result, config)` — safety-net rejects
  even if model says `ok=true` but has error-severity issues;
  optionally rejects on warnings via config flag.
- Schema constant `REVIEW_SCHEMA_DESCRIPTION` for prompt injection.

**Test coverage (18 tests):** parse_valid_accept,
parse_valid_reject_with_errors, parse_with_json_tag_wrapper,
parse_invalid_json_returns_none, parse_mixed_severities,
decide_accept_when_ok_no_issues, decide_accept_with_info_only,
decide_accept_with_warnings_default_config, decide_reject_when_ok_false,
decide_reject_on_error_even_if_ok_true, decide_reject_on_warnings_when_configured,
decide_reject_ok_false_no_error_issues, build_review_task_shape,
build_review_prompt_shape, config_default_values, max_issues_in_reason_caps_output,
severity_serde_roundtrip, review_result_serde_roundtrip.

**Anti-drift.** Row 28.1 archived from `rules/milestones.md`.
`docs/coding-workflow-design.md` §5 item 2 ("Reviewer sub-agent") is now
delivered — the module lives in `coding/reviewer.rs` (not
`coding_workflow/reviewer.rs` as originally speculated in the milestone
note; follows the flat module pattern established by the rest of the
coding module).

---

## Chunk 27.1 — Agentic RAG retrieve-as-tool

**Date:** 2026-04-30 · **Phase:** 27 (Agentic RAG & context engineering) · **Tests:** 1646 / 1646 (+12 new), clippy lib clean

**Why.** The current chat pipeline does a *single* static retrieval at
the start of each turn. Agentic RAG embeds `retrieve_memory` as an
explicit LLM tool so the model can plan → retrieve → reflect →
re-retrieve in a bounded loop (capped at 5 iterations).

**What landed.**

- New module [`src-tauri/src/orchestrator/agentic_rag.rs`] (~270 LOC + 12 tests).
- **Tool protocol** — the LLM is instructed via `AGENTIC_RAG_TOOL_DESCRIPTION`
  to emit `<tool_call name="retrieve_memory"><query>…</query></tool_call>`.
  `parse_tool_call` extracts it; `strip_tool_call` removes it from
  partial answers; `format_tool_result` wraps retrieval results as
  `<tool_result>…</tool_result>`.
- **Public helpers:**
  - `build_system_prompt(base)` — appends tool description to the
    existing companion system prompt.
  - `parse_tool_call(reply) -> Option<ToolCall>` — extracts name +
    query from the model's XML tool-call block.
  - `strip_tool_call(reply) -> String` — returns the clean answer text.
  - `format_tool_result(results) -> String` — wraps memory entries.
- **Types:** `ToolCall`, `LoopTurn`, `AgenticRagResult`, `AgenticRagConfig`.
- `AgenticRagConfig::default()` — `max_iterations: 5`, `top_k: 5`.

**What's deferred:**
- The actual async loop driver that calls the LLM + memory store
  alternately. That lands in 28.2 (orchestrator → coding wiring)
  once the orchestrator surface is stable.
- Integration with Self-RAG (16.4a) reflection tokens.

**Test coverage (12 tests):** parse_tool_call_valid,
parse_tool_call_single_quotes, parse_tool_call_none_when_missing,
parse_tool_call_none_when_empty_query, parse_tool_call_none_when_unclosed,
strip_tool_call_removes_block, strip_tool_call_noop_when_absent,
format_tool_result_with_entries, format_tool_result_empty,
build_system_prompt_appends_tool, config_default_values,
agentic_result_fields.

**Anti-drift.** Row 27.1 archived from `rules/milestones.md`.

---

## Chunk 25.10 — apply_file (LLM output writer)

**Date:** 2026-04-30 · **Phase:** 25 (Self-improve core) · **Tests:** 1634 / 1634 (+13 new), clippy lib clean

**Why.** The coding workflow can ask an LLM to produce new file contents
but until now had no safe way to *write* them into the working tree.
This chunk provides the "last mile" writer that the reviewer sub-agent
(28.1) and the GitHub PR flow (28.5) both depend on.

**What landed.**

- New module [`src-tauri/src/coding/apply_file.rs`] (~340 LOC + 13 tests).
- **Parser** — `parse_file_blocks(reply)` extracts zero or more
  `<file path="…">content</file>` blocks from raw LLM output.
  Handles both double-quoted and single-quoted path attributes. Skips
  malformed or path-less blocks gracefully.
- **Security validator** — `validate_path(repo_root, rel_path)`:
  - Rejects absolute paths.
  - Rejects `..` traversal.
  - Rejects writes into `.git/`.
  - Creates parent dirs and canonicalizes to confirm the resolved path
    is still under the repo root.
- **Atomic writer** — `atomic_write(path, content)` via `*.apply_tmp` +
  rename so a crash can't leave a torn file.
- **Git staging** — `git_add(repo_root, file)` stages written files.
  Handles Windows `\\?\` canonical path differences via fallback.
- **Public entry points:**
  - `apply_blocks(repo_root, &[FileBlock], git_stage) -> ApplySummary`
  - `apply_from_reply(repo_root, reply, git_stage) -> ApplySummary`
- **Types:** `FileBlock`, `ApplyResult`, `ApplyRejection`, `ApplySummary`.

**Test coverage (13 tests).** parse_single_file_block,
parse_multiple_file_blocks, parse_skips_malformed_blocks,
validate_rejects_absolute_path, validate_rejects_traversal,
validate_rejects_dot_git, validate_allows_normal_path,
validate_creates_parent_dirs, atomic_write_creates_file,
apply_blocks_writes_and_stages, apply_blocks_rejects_traversal_but_applies_others,
apply_from_reply_end_to_end, apply_overwrites_existing_file.

**Anti-drift.** Row 25.10 archived from `rules/milestones.md`.

---

## Chunk 28.6 — Persistent SQLite task queue

**Date:** 2026-04-30 · **Phase:** 28 (Self-improve loop maturation) · **Tests:** 1621 / 1621 (+19), clippy lib clean

**Why.** Today the self-improve loop reads the next chunk by re-parsing
`rules/milestones.md` every cycle. That works for a single interactive
session but breaks down for the Phase 24 phone surface and the MCP /
CLI surfaces, which need to enqueue tasks asynchronously and pick
them up later. This chunk introduces the durable, FIFO+priority+retry
queue that backs all three.

**What landed.**

- New module [`src-tauri/src/coding/task_queue.rs`](../src-tauri/src/coding/task_queue.rs)
  (~470 LOC including 19 tests). Re-exported from `coding::mod`.
- Schema: single `coding_tasks` table at `<data_dir>/coding_tasks.sqlite`
  with `status`, `priority` (DESC), `enqueued_at` (ASC tie-break),
  `attempts` / `max_attempts` for retry, `started_at` / `finished_at`,
  `result` / `error`, `enqueued_by` for cross-surface attribution
  (`local`, `mcp`, `phone`). Index on `(status, priority DESC, enqueued_at ASC)`
  for O(log n) claim. WAL + `synchronous = NORMAL` for crash-safe
  multi-process access.
- Public surface: `TaskQueue::open`, `open_in_memory`, `enqueue`,
  `claim_next`, `complete`, `fail`, `cancel`, `get`, `list`,
  `counts_by_status`, `purge_finished_before`. All have
  `_with_now(...)` testing variants that take an explicit timestamp.
- **Atomic claim**: `claim_next` uses a single `UPDATE … RETURNING`
  with a sub-select so two concurrent callers can never both pick
  the same row. Verified by `concurrent_claim_does_not_double_pick`
  (5 tasks across two `TaskQueue` handles on the same disk DB).
- **Retry semantics**: `fail` reads `attempts`/`max_attempts` and
  routes back to `pending` (clearing `started_at`) or to `failed`
  terminal. Verified by `fail_retries_until_max_attempts`.
- All `enum TaskStatus` variants (`Pending`, `InProgress`, `Done`,
  `Failed`, `Cancelled`) round-trip through their string forms.

**Test coverage (19 tests).**

`open_in_memory_creates_schema`, `open_is_idempotent`, `enqueue_returns_unique_ids`,
`fifo_within_same_priority`, `higher_priority_wins`,
`claim_next_is_none_when_no_pending`, `complete_marks_done_and_records_result`,
`complete_rejects_non_in_progress`, `fail_retries_until_max_attempts`,
`fail_rejects_non_in_progress`, `fail_unknown_id_returns_not_found`,
`cancel_pending_or_in_progress_only`, `list_filters_by_status`,
`counts_by_status_aggregates`, `purge_finished_before_keeps_active`,
`purge_respects_cutoff`, `enqueue_persists_to_disk_path`,
`task_status_round_trips_string`, `concurrent_claim_does_not_double_pick`.

**What's deferred to a follow-up sub-chunk** (intentionally — keeps
this PR small and easy to review):

- A `tokio::spawn`-ed worker that loops `claim_next` → `run_coding_task`
  → `complete` / `fail`. The queue primitives are designed for this
  but wiring it touches the orchestrator, which is in flux for 27.1.
- `coding_task_*` Tauri commands (enqueue / list / cancel) for the
  Brain panel UI.
- MCP tool surface (`brain_enqueue_coding_task`).

**Anti-drift.** Row 28.6 archived from `rules/milestones.md`.

---

## Chunk 26.1 — Daily background maintenance scheduler

**Date:** 2026-04-30 · **Phase:** 26 (Brain background-maintenance & auto-learn completion) · **Tests:** 1602 / 1602 (+5 new), clippy lib clean

**Status pre-chunk.** The pure decision module (`brain::maintenance_scheduler`)
and the runtime wrapper (`brain::maintenance_runtime::spawn`) had landed
in earlier sub-chunks (26.1a / 26.1b). The runtime was already wired
from `lib.rs` and persisted per-job timestamps to
`<data_dir>/maintenance_state.json` with hourly ticks. **What was still
missing:** the milestones row required `AppSettings.background_maintenance_enabled`
+ `AppSettings.maintenance_interval_hours` (1–168) + an idle guard so
the scheduler can be turned off, retuned live, or skipped during active
chat. This chunk closes that gap.

**What landed.**

- `AppSettings` gained three forward-compatible fields with `serde(default)`:
  - `background_maintenance_enabled: bool` (default `true`).
  - `maintenance_interval_hours: u32` (default `24`, clamped `1..=168`).
  - `maintenance_idle_minimum_minutes: u32` (default `0` = disabled).
- New `AppSettings::maintenance_cooldown_ms()` helper translates the
  hour count to ms and applies the documented clamp so a corrupt config
  (`= 0` or `= u32::MAX`) cannot disable maintenance entirely or push
  it out forever.
- `brain::maintenance_runtime::spawn` now reads the live `AppSettings`
  on every tick (and the `ActivityTracker` for the idle guard), so
  edits take effect on the next interval without restart:
  - If `background_maintenance_enabled == false`, skip dispatch.
  - If `maintenance_idle_minimum_minutes > 0` and the user has been
    active within that window, skip dispatch.
  - Otherwise, derive the live `MaintenanceConfig` from
    `maintenance_cooldown_ms()` and call the new
    `MaintenanceRuntime::jobs_due_with(&live_config, now_ms)` so all
    four jobs share the same user-controlled cool-down.
- 5 new unit tests in `settings::tests`:
  - `default_background_maintenance_is_on`
  - `serde_fills_maintenance_defaults_when_missing` (forward-compat)
  - `maintenance_cooldown_clamps_below_minimum` (`0h` → `1h`)
  - `maintenance_cooldown_clamps_above_maximum` (`1000h` → `168h`)
  - `maintenance_cooldown_default_is_24h`

**Files touched.**

- `src-tauri/src/settings/mod.rs` — three new fields + `maintenance_cooldown_ms` + 5 tests + `Default` body update.
- `src-tauri/src/settings/config_store.rs` — three sites updated.
- `src-tauri/src/commands/settings.rs` — two sites updated.
- `src-tauri/src/brain/maintenance_runtime.rs` — added `jobs_due_with`; rewrote tick loop body to honour live settings + idle guard.

**Brain doc sync.** `docs/brain-advanced-design.md` §21.7 item 1 (next-day
update note) — runtime is wired and now user-controllable. README's
"💾 Memory System" surface unchanged.

**Anti-drift.** Row 26.1 archived from `rules/milestones.md`.

---

## Chunk 28.7 — Real token usage capture

**Date:** 2026-04-30 · **Phase:** 28 (Self-improve loop maturation) · **Tests:** existing 1597 still green, clippy clean

### Problem

Token telemetry plumbing (`RunRecord.prompt_tokens` / `completion_tokens` / `cost_usd`, `MetricsSummary` rolling 7d totals + per-provider breakdown, `TokenUsage`, `cost.rs` price catalogue) had landed earlier under Chunk 28.7's umbrella, but `coding::engine::plan_one_chunk` was still passing `TokenUsage::default()` for every recorded outcome. The dashboard reported zero spend even when real cloud APIs were billing the user.

### Solution

- Added `ChatCompletionUsage { prompt_tokens, completion_tokens }` to `brain::openai_client`, deserialised from the standard OpenAI `usage` block (`#[serde(default)]` so providers that omit it still parse).
- New `OpenAiClient::chat_with_usage` returns `Result<(String, Option<ChatCompletionUsage>), String>`. The existing `chat` is now a thin wrapper that discards the usage half — wire-format unchanged for every other caller.
- `coding::engine::plan_one_chunk` switched to `chat_with_usage`; the returned usage is folded into `crate::coding::metrics::TokenUsage` and passed to `record_outcome`. Failed runs still record `TokenUsage::default()` (the call never returned usage to capture).

### Files changed

- `src-tauri/src/brain/openai_client.rs` (+34 LOC)
- `src-tauri/src/coding/engine.rs` (+13 LOC)

### Tests

No new tests — the change is covered by the existing 28.7 metrics tests in `coding::metrics` (totalling, rolling 7d, per-provider) plus the OpenAI client `reachability_succeeds_against_stub_chat_completions_server` test which exercises `chat` (forwarding to `chat_with_usage`). Full `cargo test --lib` returns **1597 / 1597 pass**. Clippy `-D warnings` clean.

### Brain doc-sync (rule 10)

Not brain-touching — coding-LLM telemetry only. README and `docs/brain-advanced-design.md` unchanged.

---

## Chunks 26.2 / 26.3 / 26.4 — Milestones bookkeeping reconciliation

**Date:** 2026-04-30 · **Phase:** 26 (Daily conversation → brain write-back) · **Tests:** existing coverage already green

### Problem

Three Phase-26 chunks were fully implemented in the codebase but still listed as `not-started` in `rules/milestones.md`, violating the "single source of truth" enforcement rule. Reconciled the milestones board with reality.

### Discovered to be already-shipped

- **Chunk 26.2 — Conversation-aware (segmented) extraction.** `crate::brain::segmenter` (pure topic-shift segmenter) plus `brain_memory::extract_facts_segmented_any_mode` + tests; wired into `commands::memory::extract_memories_from_session` so any configured brain mode runs segment-by-segment extraction with single-pass fallback when embeddings are unavailable.
- **Chunk 26.3 — Auto-edge extraction after `extract_facts`.** `AppSettings.auto_extract_edges` (default true, forward-compat tested), and `extract_memories_from_session` calls a private `run_edge_extraction` helper after a successful save when an Ollama active brain is configured. Best-effort: failures don't propagate.
- **Chunk 26.4 — Replay-from-history rebuild command.** `memory::replay` module ships `ReplayConfig`, `ReplayProgress`, `select_summaries`, `synthetic_history_from_summary`, `next_progress` plus 11 unit tests. `commands::memory::replay_extract_history` is the Tauri command, registered in `lib.rs` `invoke_handler!` and surfaces a `brain-replay-progress` event stream.

### Action

Removed the four "not-started" rows from `rules/milestones.md`. No code changes.

### Brain doc-sync (rule 10)

Brain-touching: yes (segmentation + auto-edges + replay). The design doc `docs/brain-advanced-design.md` §21.7 already enumerates these as the four daily-update roadmap items so no new schema/architecture text is needed; this entry just records that they are now also reflected in the milestones board.

---
## Chunk 28.9 — Coding workflow handoff persistence + Tauri wiring

**Date:** 2026-04-30 · **Phase:** 28 (Self-improve loop maturation) · **Tests:** +11 Rust unit tests (1597 lib total, clippy clean)

### Problem

Chunk 28.8 shipped the pure codec (`build_handoff_block` / `parse_handoff_reply`) but had no I/O surface, so the long-coding-session OOM problem was only half-solved. The codec sat unused.

### Solution

Added a thin I/O layer plus four Tauri commands and integrated both halves into `run_coding_task`, closing the long-session loop end-to-end.

**New module `src-tauri/src/coding/handoff_store.rs` (~320 LOC):**

- `save_handoff(data_dir, &state)` — atomic write via `*.tmp` + `fs::rename`; creates `<data_dir>/coding_workflow/sessions/<id>.json` on demand.
- `load_handoff(data_dir, session_id)` — returns `Ok(None)` for missing files (UI distinguishes never-saved from corrupt).
- `clear_handoff(data_dir, session_id)` — idempotent delete.
- `list_handoffs(data_dir)` — returns lean `HandoffSummary` records, newest first by mtime; corrupt files are silently skipped so one bad record can't brick the sessions panel.
- `sanitize_session_id(raw)` — strips path separators / traversal / leading dots, caps at 64 bytes; `""` becomes `"default"`.

**`run_coding_task` integration (`coding/workflow.rs`):**

- `CodingTask` gained an optional `prior_handoff: Option<HandoffState>` field. When present, `build_handoff_block` is rendered as the first `<document>` (label `resuming_session`) so the model re-grounds before reading any other context.
- The task description is suffixed with `emit_handoff_seed_instruction()` so the model knows to emit a fresh `<next_session_seed>` payload.
- After the LLM call, `parse_handoff_reply` is invoked; the parsed state lands in the new `CodingTaskResult.next_handoff` field (`skip_serializing_if = "Option::is_none"` keeps the wire format backward-compatible).

**Tauri command surface (`commands/coding.rs`):**

- `run_coding_task` gained an optional `handoffSessionId` parameter. When supplied: load prior state from disk, inject into task, call workflow, parse + atomic-save next state. Load/save errors are logged to `stderr` and never block the user.
- `coding_session_save_handoff(handoff)` — explicit bookmark.
- `coding_session_load_handoff(sessionId)` — `Option<HandoffState>` for the resume picker.
- `coding_session_list_handoffs()` — summaries for the sessions panel.
- `coding_session_clear_handoff(sessionId)` — idempotent delete.

All four registered in `lib.rs` `invoke_handler!`.

### Files changed

- `src-tauri/src/coding/handoff_store.rs` (NEW)
- `src-tauri/src/coding/workflow.rs` (CodingTask + CodingTaskResult + run_coding_task)
- `src-tauri/src/coding/mod.rs` (module registration + re-exports)
- `src-tauri/src/commands/coding.rs` (extended run_coding_task + 4 new commands)
- `src-tauri/src/lib.rs` (imports + invoke_handler registrations)
- `src-tauri/tests/ollama_self_improve_smoke.rs` (added `prior_handoff: None` to CodingTask construction)

### Tests

11 new unit tests in `handoff_store::tests` covering: roundtrip, missing-returns-None, overwrite, idempotent clear, list newest-first, list skips corrupt, list on missing dir, sanitiser strips traversal, sanitiser length cap, sanitised filename round-trip, monotonic timestamp helper. Full `cargo test --lib` returns **1597 / 1597 pass**. Clippy `-D warnings` clean.

### Brain doc-sync (rule 10)

Not brain-touching — coding-workflow infrastructure only. README and `docs/brain-advanced-design.md` unchanged.

---
## Chunk 28.8 — Coding workflow session handoff codec

**Date:** 2026-04-30 · **Phase:** 28 (Self-improve loop maturation) · **Tests:** +14 Rust unit tests

Pure-utility chunk that delivers the codec half of the long-coding-session
context-passing design. Long autonomous coding runs (planner → coder →
reviewer chained over hours, or a single chunk re-invoked many times)
routinely exhaust the LLM's context window — the VS Code Copilot host
auto-summarises history when budget fills, local Ollama drops
sliding-window context past ~32 K tokens, cloud providers truncate
silently. When that happens the model loses the thread.

This chunk lets every `run_coding_task` invocation stamp a compact
JSON "next-session seed" describing what it just did, what is still
pending, and which files are open; the next invocation prepends a
`[RESUMING SESSION]` block to its prompt that re-grounds the model in
O(few-hundred-tokens). Same shape as Chunk 23.2a (agent-swap handoff
block builder) so the two systems compose cleanly.

**Files added.**
- `src-tauri/src/coding/handoff.rs` (~360 LOC) — pure module:
  - `HandoffState` struct (`session_id`, `chunk_id`, `last_action`,
    `pending_steps[]`, `open_artefacts[]`, `summary`, `created_at`).
  - `build_handoff_block(state) -> String` — renders a
    `[RESUMING SESSION] … [/RESUMING SESSION]` block, hard-capped at
    `MAX_BLOCK_BYTES = 4 KiB`, with priority-ordered truncation
    (open_artefacts → pending_steps → summary → last_action) and
    `…(+N more)` footers when lists are clipped.
  - `parse_handoff_reply(reply) -> Option<HandoffState>` — extracts
    `<next_session_seed>…</next_session_seed>` from the model's reply,
    tolerates fenced code blocks (```` ```json ````), returns `None`
    on malformed JSON or missing required fields.
  - `emit_handoff_seed_instruction() -> String` — the system-prompt
    fragment that asks the model to emit the seed before ending.
  - `truncate_with_ellipsis(s, max)` private helper, UTF-8-safe.

**Files modified.**
- `src-tauri/src/coding/mod.rs` — added `pub mod handoff;`.
- `rules/milestones.md` — chunk 28.8 row removed (this entry is the
  archive).

**Tests.** 14 unit tests in `coding::handoff::tests`:
1. `build_block_renders_all_fields`
2. `build_block_handles_empty_lists`
3. `build_block_is_deterministic`
4. `build_block_respects_hard_cap_clipping_artefacts_first` — pile of
   1000 artefact paths shrinks to ≤ 4 KiB with `…(+N more)` footer,
   pending_steps left intact.
5. `build_block_clips_pending_when_artefacts_alone_insufficient`
6. `build_block_clips_summary_as_last_resort`
7. `truncate_with_ellipsis_respects_utf8`
8. `parse_reply_extracts_seed`
9. `parse_reply_handles_fenced_json`
10. `parse_reply_returns_none_when_tags_absent`
11. `parse_reply_returns_none_on_malformed_json`
12. `parse_reply_returns_none_on_missing_required_field`
13. `round_trip_through_render_and_parse`
14. `emit_instruction_mentions_tags_and_schema`

**Validation.** `cargo test --lib coding::handoff` → 14/14 pass.
`cargo clippy --lib -- -D warnings` clean.

**Follow-up.** Chunk 28.9 (28.8b) wires this into `run_coding_task`:
read `<data>/coding_workflow/sessions/<session_id>.json` if present,
prepend `[RESUMING SESSION]` block to the prompt, on success parse
`<next_session_seed>` from the reply and atomic-write the updated
state. Adds Tauri commands `coding_session_save_handoff`,
`coding_session_load_handoff`, `coding_session_list_handoffs`,
`coding_session_clear_handoff`.

---

## Chunk 23.2b — Handoff system-prompt block consumer wiring

**Date:** 2026-04-29 · **Phase:** 23 (Multi-agent resilience) · **Tests:** +6 frontend, +2 Rust

Wired the pure builder shipped in 23.2a (`buildHandoffBlock`) end-to-end. On agent
switch the roster store now records both `handoffContexts[newId]` and
`handoffPrevAgentName[newId]`. The conversation store's pre-send pipeline calls
a new `consumeHandoff(agentId)` (read-and-clear, returns `{ prevAgentName, context }`)
and renders the block via `buildHandoffBlock`, then either:

- **Tauri path:** `await invoke('set_handoff_block', { block })` before
  `streaming.sendStreaming(content)`. The Rust streaming splice (both Ollama
  and OpenAI paths in `commands/streaming.rs`) reads `state.handoff_block`,
  appends to the assembled system prompt, and `clear()`s the slot — same
  one-shot semantics as the milestone called for.
- **Browser path:** appended inline to the system-prompt string passed to
  `streamChatCompletion` in `conversation.ts`.

New Rust surface in `commands/persona.rs`: `set_handoff_block` (8 KiB cap) +
`get_handoff_block` (debug peek) registered in `lib.rs` `invoke_handler!`.
`AppStateInner.handoff_block: Mutex<String>` initialised in both production
and `for_test` constructors.

**Files touched (8):**

- `src-tauri/src/lib.rs` — `handoff_block` field + 2 ctor inits + 2 command registrations.
- `src-tauri/src/commands/persona.rs` — `set_handoff_block` / `get_handoff_block` + 2 unit tests.
- `src-tauri/src/commands/streaming.rs` — read-and-clear splice in OpenAI + Ollama paths.
- `src/stores/agent-roster.ts` — `handoffPrevAgentName` ref + `consumeHandoff()` + `display_name` capture.
- `src/stores/agent-roster.test.ts` — +6 vitest covering record / one-shot / clear / no-msgs / peek.
- `src/stores/conversation.ts` — imports + Tauri-path `invoke('set_handoff_block', ...)` + browser-path inline append.

**No brain-doc-sync triggered** (UI/agent-roster surface, not brain).
**Test count:** Rust 1359 → 1361 · frontend 1319 → 1325. `cargo clippy --all-targets -- -D warnings` clean. `vue-tsc --noEmit` clean.

---

## Chunk 24.5a — VS Code / Copilot log parser

**Date.** 2026-04-29

**Goal.** Ship the pure parser half of the Phase 24 mobile-companion
"what's Copilot doing right now" feature. This is the data layer
that backs the user's headline use case — phone asks "what's the
progress of using Copilot in VS Code?" and the desktop returns a
structured summary the phone-side LLM can narrate.

**Why split into a/b.** The data lives in two places (an
append-only log file + a SQLite `state.vscdb`). The
classification + summarisation rules — which substrings count as
"user turn" vs "assistant turn" vs "tool call", how to pick the
most-recent of each, how to truncate previews UTF-8-safely — are
pure logic, separable from the FS / SQLite I/O. Shipping the
parser first locks the contract `CopilotLogSummary` that 24.5b's
`get_copilot_session_status` Tauri command will return; 24.5b is
then a ~150 LOC FS wrapper. Same a/b pattern used for 16.3, 16.4,
16.5, 23.2, 24.1, 24.2 across this multi-prompt session.

**What shipped.**

- `src-tauri/src/network/vscode_log.rs` (~370 LOC, 22 tests):
  - `pub struct LogEvent { timestamp, level, kind, body }`.
  - `pub enum EventKind { WorkspaceFolder, SessionId, UserTurn, AssistantTurn, ToolInvocation, ModelSelected, Other }`.
  - `pub struct CopilotLogSummary { workspace_folder, session_id, model, last_user_turn_ts, last_user_preview, last_assistant_turn_ts, last_assistant_preview, tool_invocation_count, event_count }` deriving `Default` for the empty-log case.
  - `pub fn parse_events(log: &str) -> Vec<LogEvent>` — line-by-line; silently skips malformed continuation lines.
  - `pub fn summarise_log(log: &str) -> CopilotLogSummary` — the call the Tauri layer will hit; reverse-iterates events to pick the most-recent of each field.
  - `pub const ASSISTANT_PREVIEW_MAX_CHARS = 240`, `pub const USER_PREVIEW_MAX_CHARS = 160`.

**Critical design choices.**

- **Substring matching, case-insensitive.** Copilot Chat's log
  phrasing has shifted between extension versions; matching
  `"user message" / "user turn" / "user prompt"` (and similar
  for assistant / tool / model / workspace) means a Copilot
  bump tweaking phrasing degrades the affected line to
  `EventKind::Other` rather than hard-failing the parser. Worst
  case: `tool_invocation_count` undercounts; the summary still
  renders.
- **Tail-first summarisation.** `summarise_log` walks events in
  reverse and the *first* match for each field wins — exactly
  what the phone narrator wants ("the **most recent** assistant
  turn was 30 s ago"). Tool invocations are the one field that's
  whole-file: count, not pick.
- **UTF-8-safe truncation.** Both previews use `chars().take(N)`
  + appended ellipsis, so the truncation never splits a
  multi-byte char (Greek letters / emoji / CJK characters from
  pasted user prompts are common in test logs).
- **`CopilotLogSummary` derives `Default`.** Empty-log case
  returns `CopilotLogSummary::default()` directly — no awkward
  `Option<Summary>` branches in the FS wrapper.
- **Skips garbage lines.** Multi-line stack-trace continuations
  and free-form banners that don't match `[ts] [level] body` are
  silently dropped. `parse_events` therefore always returns
  well-formed events.

**Tests.** 22 unit tests, all passing — every classification
path, every malformed-input case, the realistic session excerpt
(`User → Assistant → Tool call → Tool call → Assistant`), the
multi-workspace tie-break (newest wins), the UTF-8 truncation
edge-case with Greek letters, and the empty-log identity.

- Whole `cargo test --lib`: **1359 passed** (was 1337 before this chunk → +22).
- `cargo clippy --all-targets -- -D warnings`: clean.

**Files touched.**

- `src-tauri/src/network/vscode_log.rs` (new).
- `src-tauri/src/network/mod.rs` — `pub mod vscode_log;`
  registered alphabetically after `pair_token`.
- `rules/milestones.md` — Phase 24 row 24.5 transformed into
  24.5b (FS wrapper + Tauri command).

**Docs.** No brain-doc-sync impact — Phase 24 is a transport /
mobile-shell phase, and this chunk reads VS Code's own log
format. When 24.4 (phone-control RPC surface) lands and
`GetCopilotSessionStatus { workspace }` exposes
`CopilotLogSummary` over gRPC to the paired phone, that is when
`docs/AI-coding-integrations.md` will get its Phase-24 entry.
The parser itself is generic enough that the doc update can
defer to the chunk that actually exposes the surface.

---

## Chunk 24.2a — Pairing payload codec

**Date.** 2026-04-29

**Goal.** Ship the pure codec half of the iOS-companion pairing
handshake: a stable `terransoul://pair?...` URI scheme that
encodes everything the phone needs (LAN host, gRPC port, 32-byte
pairing token, TLS-cert SHA-256 fingerprint, expiry timestamp)
plus the small set of crypto primitives the consumer chunk
(24.2b) will compose against — token generation, constant-time
comparison, expiry check.

**Why split into a/b.** The pairing surface is the gate that lets
a phone speak to the brain. Splitting the codec (24.2a) from the
mTLS-issuance + SQLite-persistence flow (24.2b) means the URI
format, token byte-length, fingerprint byte-length, expiry
semantics, and timing-attack surface are all hand-auditable and
unit-testable without a database, without `rcgen` CA generation,
and without Tauri command plumbing. When 24.2b lands, it's a
straight composition: `gen_token` → `PairPayload::from_bytes` →
`encode_uri` → render QR; on confirm, `decode_uri` →
`is_expired` → `constant_time_eq` against the stored token.

**What shipped.**

- `src-tauri/src/network/pair_token.rs` (~440 LOC, 23 tests):
  - `pub struct PairPayload { host, port, token_b64, fingerprint_b64, expires_at_unix_ms }` with serde.
  - `pub enum PairError` covering `BadScheme`, `BadHost`, `MissingField`, `InvalidField`, `UriTooLong`, `BadByteLength`, `Malformed`.
  - `pub fn encode_uri(&PairPayload) -> Result<String, PairError>` — emits `terransoul://pair?host=...&port=...&token=...&fp=...&exp=...`.
  - `pub fn decode_uri(&str) -> Result<PairPayload, PairError>` — strict scheme/host/field validation; tolerates unknown extension keys.
  - `pub fn gen_token() -> [u8; 32]` — only impure function, uses `rand_core::OsRng::fill_bytes`.
  - `pub fn constant_time_eq(&[u8], &[u8]) -> bool` — hand-rolled XOR-OR with `std::hint::black_box` to discourage short-circuit optimisation; no `subtle` crate dep.
  - `pub fn is_expired(&PairPayload, now_unix_ms: u64) -> bool`.
  - `pub const DEFAULT_EXPIRY_MS = 300_000` (5 min), `pub const TOKEN_BYTE_LEN = 32`, `pub const MAX_URI_LEN = 480`, `pub const PAIR_URI_SCHEME = "terransoul"`, `pub const PAIR_URI_HOST = "pair"`.
  - `PairPayload::from_bytes(host, port, token, fingerprint, expires_at)` constructor that enforces 32-byte token + 32-byte fingerprint up-front.
  - `PairPayload::token_bytes()` / `fingerprint_bytes()` decoders that re-validate length on the way out (defence in depth).

**Critical design choices.**

- **base64url-no-pad for token + fingerprint.** All bytes that go
  in the URI are URL-safe by construction, so the encode path is
  almost a no-op (only `&`, `=`, `#`, `?`, ` `, `+`, `%` are escaped
  for the `host` field, which can contain IPv6 colons or DNS dots).
- **Length validation on every byte-decode.** Both
  `token_bytes()` and `fingerprint_bytes()` re-check
  `bytes.len() == 32` after base64-decode, even though
  `from_bytes` already enforced it. Defence in depth — the codec
  is the trust boundary.
- **`is_expired` is `>=` strict.** Exactly at the boundary is
  considered expired (tests pin this) — the pairing window is
  half-open `[issued, expires_at)`.
- **`MAX_URI_LEN = 480`.** QR codes degrade past ~512 chars at
  reasonable error-correction levels; capping at 480 leaves
  ~30 chars of headroom for an optional `display_name=` field
  in 24.2b without breaking the QR scan.
- **Tolerates unknown extension keys.** `decode_uri` ignores
  query parameters it doesn't recognise — forward-compatible for
  future extensions (e.g. `display_name`, `cap=` capability bits)
  without breaking older iOS clients.
- **Fixed scheme + host strings as constants.** `PAIR_URI_SCHEME`
  / `PAIR_URI_HOST` are public so the iOS client can build its
  validator against the same constants the Rust desktop emits.

**Tests.** 23 unit tests, all passing:

- Constructor: `from_bytes_rejects_short_token`, `from_bytes_rejects_short_fingerprint`.
- Round-trip: `round_trip_encode_decode`, `encoded_uri_uses_terransoul_pair_scheme`, `token_bytes_round_trip`, `fingerprint_bytes_round_trip`, `host_with_special_chars_round_trips`.
- Decode rejection: `decode_rejects_bad_scheme`, `decode_rejects_bad_host`, `decode_reports_each_missing_field` (5 cases inline), `decode_rejects_unparseable_port`, `decode_rejects_unparseable_exp`, `decode_rejects_short_token_byte_length`, `decode_rejects_uri_too_long`.
- Encode rejection: `encode_rejects_uri_too_long`.
- Forward-compat: `decode_tolerates_unknown_extension_keys`.
- Crypto primitives: `gen_token_is_well_formed_and_nondeterministic`, `constant_time_eq_handles_length_mismatch`, `constant_time_eq_matches_eq_on_content`.
- Time: `is_expired_strict_ge_boundary`, `default_expiry_is_five_minutes`.
- Encoding helper: `url_encode_component_passes_safe_chars`, `url_encode_component_escapes_query_breakers`.
- Whole `cargo test --lib`: **1337 passed** (was 1314 before this chunk → +23).
- `cargo clippy --all-targets -- -D warnings`: clean.

**Files touched.**

- `src-tauri/src/network/pair_token.rs` (new).
- `src-tauri/src/network/mod.rs` — `pub mod pair_token;` registered alphabetically after `lan_addresses`.
- `rules/milestones.md` — Phase 24 row 24.2a removed after archival.

**Docs.** No brain-doc-sync impact yet — Phase 24 is a transport
phase, not a brain-surface phase. When 24.4 (phone-control RPC
surface) lands and the gRPC handlers expose brain operations to
authenticated phones, that's when `docs/brain-advanced-design.md`
§24 needs an "MCP-over-LAN-paired-device" section and
`docs/AI-coding-integrations.md` needs a Phase 24 entry. The
codec itself is generic enough that those updates can defer to
the chunk that actually exposes the surface.

---

## Chunk 24.1a — Pure LAN address classifier

**Date.** 2026-04-29

**Goal.** Open Phase 24 (Mobile Companion — iOS + LAN gRPC remote
control) with the foundation chunk: a pure, security-critical
classifier that decides which of the OS-reported network interface
addresses are legitimate pairing endpoints for the upcoming iOS
companion app. Every later chunk in Phase 24 (LAN bind, mTLS pairing,
gRPC remote control, iOS shell) sits on this filter.

**Why split into a/b.** The "expose the brain to the LAN" surface is
the highest-blast-radius security boundary in the project. Splitting
the OS probe (24.1b) from the classifier (24.1a) makes the
filtering rules — RFC 1918 / RFC 6598 / loopback / link-local /
documentation / benchmarking / multicast — hand-auditable and
deterministically unit-testable without mocking
`local-ip-address` / `if-addrs`. When 24.1b lands it is "just" a
30-line OS call followed by `classify_addresses(...)`. Same a/b
pattern applied to 16.3, 16.4, 16.5, 23.2 earlier this session.

**What shipped.**

- `src-tauri/src/network/mod.rs` (new module entry).
- `src-tauri/src/network/lan_addresses.rs` (~340 LOC, 14 tests):
  - `pub enum LanAddressKind { Private, Public }`.
  - `pub struct LanAddress { addr: IpAddr, kind: LanAddressKind }`.
  - `pub struct ClassifyOptions { allow_ipv6, allow_public }`
    deriving `Default` (both false → conservative posture).
  - `pub fn classify_addresses(&[IpAddr], ClassifyOptions) -> Vec<LanAddress>`.
  - `pub fn private_lan_addresses(&[IpAddr]) -> Vec<LanAddress>` —
    convenience for the common pairing-UI case.
- `src-tauri/src/lib.rs` — `pub mod network;` registered alphabetically between `messaging` and `orchestrator`.

**Filtering rules** (always rejected, regardless of options):

- `is_unspecified()` (`0.0.0.0`, `::`)
- `is_loopback()` (`127/8`, `::1`)
- `is_multicast()` (`224/4`, `ff00::/8`)
- IPv4 link-local (`169.254/16`)
- IPv4 documentation ranges (`192.0.2/24`, `198.51.100/24`, `203.0.113/24`)
- IPv4 benchmarking (`198.18/15`)
- IPv4 broadcast (`255.255.255.255`)

**Classification rules:**

- IPv4 Private = RFC 1918 (`10/8`, `172.16/12`, `192.168/16`) ∪ RFC 6598 (`100.64/10`, carrier-grade NAT).
- IPv6 Private = unique-local (`fc00::/7`).
- Everything else = Public.

**Tests.** 14 unit tests, all passing:

- `rejects_loopback_unspecified_multicast_broadcast`,
  `rejects_link_local_v4`,
  `rejects_documentation_and_benchmarking_ranges`,
  `classifies_rfc1918_as_private`,
  `rfc1918_boundary_addresses` (ensures `172.15.x.x` / `172.32.x.x` are *not* classified private),
  `classifies_rfc6598_carrier_grade_nat_as_private`,
  `drops_public_by_default`,
  `allow_public_keeps_routable_addresses`,
  `drops_ipv6_by_default`,
  `ipv6_unique_local_classified_as_private_when_allowed`,
  `ipv6_global_classified_as_public_and_dropped_by_default`,
  `preserves_input_order`,
  `private_lan_addresses_helper_matches_default_options`,
  `empty_input_yields_empty_output`.
- Whole `cargo test --lib`: **1314 passed** (was 1300 before this chunk → +14).
- `cargo clippy --all-targets -- -D warnings`: clean.

**Critical design choices.**

- **Conservative defaults.** `ClassifyOptions::default()` is
  IPv6-off, public-off. Surfacing a routable IPv4 to the pairing UI
  is almost always a misconfiguration; the caller has to opt in.
- **No syscalls.** The classifier takes a slice of `IpAddr` and
  returns a `Vec<LanAddress>`. Every rule is unit-testable on
  fixture input without an actual network interface.
- **Order preservation.** Output mirrors input order so the UI can
  display "the interface listed first by the OS first" — usually the
  primary Wi-Fi adapter on Windows/macOS.
- **RFC 6598 included.** Mobile / tethered LANs commonly hand out
  `100.64/10`; treating those as private is correct for the
  iOS-companion use case (phone hotspots, tethered Mac).

**Files touched.**

- `src-tauri/src/network/mod.rs` (new).
- `src-tauri/src/network/lan_addresses.rs` (new).
- `src-tauri/src/lib.rs` — `pub mod network;` declaration.
- `rules/milestones.md` — Phase 24 added (12 chunks: 24.1a–24.11);
  24.1a row removed after archival.

**Docs.** Phase 24 milestones row introduced in `rules/milestones.md`
defines the full mobile-companion roadmap and the user's headline
acceptance gate ("ask the phone what's the progress of Copilot in VS
Code → continue next step"). No brain-doc-sync impact yet — Phase 24
is a transport / mobile-shell phase; the brain surface (RAG, memory,
embeddings) is unchanged. When 24.4 (phone-control RPC surface)
lands and exposes brain-search to the phone, that's when
`docs/AI-coding-integrations.md` needs the parallel update.

---

## Chunk 23.2a — Handoff system-prompt block builder

**Date.** 2026-04-29

**Goal.** Ship the pure `buildHandoffBlock(input)` helper so Phase
23's "agent swap loses context" gap can be closed by a thin
conversation-store integration (23.2b) rather than a parallel
rewrite. Pure, dependency-free, fully unit-testable in isolation —
identical shape to `src/utils/persona-prompt.ts::buildPersonaBlock`.

**Why split into a/b.** The block-builder is pure data-in,
data-out and has no Pinia / no Tauri / no streaming surface to
mock. Shipping it now (a) freezes the contract that 23.2b will
compose against — exact block format, truncation semantics,
guard cases — and (b) makes 23.2b a ~60 LOC integration patch
instead of a self-contained component. Same a/b pattern applied
to 16.4 (Self-RAG), 16.5 (CRAG), and 16.3 (Late chunking) earlier
this session.

**What shipped.**

- `src/utils/handoff-prompt.ts` (~120 LOC):
  - `export interface HandoffBlockInput { prevAgentName, context, nextAgentName? }`.
  - `export function buildHandoffBlock(input): string` — emits
    `\n\n[HANDOFF FROM <prev>]\n<body>\n[/HANDOFF]` (same
    precedence-shape as `[PERSONA]` and `[LONG-TERM MEMORY]`).
  - `export const HANDOFF_MAX_CHARS = 2000`,
    `export const HANDOFF_MAX_LINES = 40`.
- `src/utils/handoff-prompt.test.ts` — 14 vitest tests covering:
  null/undefined input, blank agent name, blank context,
  basic single-line render, multi-line ordering, empty-line
  drop + trailing-whitespace trim, CRLF→LF normalisation,
  control-char stripping in name, line-cap takes the **tail**,
  hard char-cap with `…(truncated)\n` marker preserving tail,
  exact-cap no-truncate, non-string context guard,
  snapshot-style stable format, and `nextAgentName` accepted
  but never rendered.

**Critical design choices.**

- **Tail-keeping truncation.** Both line-cap and char-cap keep
  the *most recent* turns and drop the older head — recency
  bias, since a handoff briefing wants the freshest context.
  Char-cap prefixes a `…(truncated)\n` marker so the LLM can
  see the truncation happened.
- **Guard-rail returns `''` rather than throwing.** Empty
  agent name, empty context, non-string context, and
  null/undefined input all silently render to `''`. The
  consumer can safely concatenate the result into the system
  prompt without a try/catch — same convention as
  `buildPersonaBlock`.
- **Control-character sanitisation.** Both the agent name and
  the context body strip ASCII control chars (\\x00–\\x1F /
  \\x7F, except \\n / \\t in the body) before rendering. The
  recorded handoff context comes from a Pinia ref that could
  contain arbitrary message content — paranoia is cheap here.
- **`nextAgentName` accepted but unused.** The interface
  preserves it for future symmetry (e.g. a `[HANDOFF FROM A
  TO B]` variant) without forcing 23.2b to evolve the type.

**Tests.** 14 unit tests, all passing.

- Whole `npx vitest run`: **1319 passed** across 87 files (was
  1305 before this chunk → +14).
- Frontend test gate clean.

**Files touched.**

- `src/utils/handoff-prompt.ts` (new).
- `src/utils/handoff-prompt.test.ts` (new).
- `rules/milestones.md` — row 23.2 transformed into 23.2b
  ("consumer wiring"), keeping only the deferred half visible.

**Docs.** No brain-doc-sync impact — this chunk is in the
agent-roster / conversation-store surface, not the brain. The
Phase 23 acceptance gate text in `milestones.md` stays valid;
once 23.2b lands, Agent B's first reply demonstrably acknowledges
the `[HANDOFF FROM A]` block.

---

## Chunk 21.5/6/7 — Doc reality bundle

**Date.** 2026-04-29

**Goal.** Three Phase 21 doc-hygiene rows shipped as one bundle:
21.5 (MCP tool name correction), 21.6 (AI-coding-integrations status
re-check), 21.7 (persona-design.md renumber from legacy Phase 13 to
canonical Phase 14).

**What shipped.**

- **21.5 — MCP tool name table fix.** `docs/brain-advanced-design.md`
  §24.2 listed eight invented tool names (`brain_ask`, `brain_extract`,
  `brain_list_memories`, `brain_stats`, plus a wrong `brain_ingest`).
  Replaced with the real eight from `src-tauri/src/ai_integrations/mcp/tools.rs`:
  `brain_search`, `brain_get_entry`, `brain_list_recent`,
  `brain_kg_neighbors`, `brain_summarize`, `brain_suggest_context`,
  `brain_ingest_url`, `brain_health`. Added a "Source of truth" note
  pointing at `tools.rs` so the next drift is catchable.
- **21.6 — AI-coding-integrations.md re-check.** Verified the doc was
  already updated by the 15.5 / 15.9 / 15.10 chunks earlier this
  session: status banner says "Phase 15 mostly shipped"; table rows
  15.6 / 15.9 already marked ✅; stdio transport already linked to
  15.9. The audit row's complaint was stale — no edit required.
  Closing the row.
- **21.7 — persona-design.md §15 renumber.** Flipped legacy
  "Phase 13.A/B + chunks 140–155" to canonical "Phase 14.A/B + chunks
  14.1–14.15". Reflects the as-shipped reality from the Phase 14
  completion-log entries (14.1, 14.3, 14.4, 14.5, 14.6, 14.7, 14.9–14.12
  all ✅). Updated §15.1 main-chain table, §15.2 side-chain table,
  the §10 cross-reference at line 1081, and the §16 "Sources" link to
  `rules/milestones.md`. Added a banner at the top of §15 pointing
  readers to `completion-log.md` for as-shipped status, since the
  shipped chunks no longer have rows in `milestones.md`.

**Why bundle.** All three are non-code doc edits, all three are
mandated by the architecture-rules.md doc-sync rules (brain-doc-sync
for 21.5, ai-integrations-doc-sync for 21.6, persona-doc-sync for
21.7). Bundling means one commit, one log entry, one milestone-row
removal — the per-row prose was disproportionate to the actual edit
size.

**Tests.** None — pure doc edits. Existing 1300-test gate stays green
(no code touched).

**Files touched.**

- `docs/brain-advanced-design.md` — §24.2 tool table replaced.
- `docs/persona-design.md` — §10 cross-ref + §15 fully renumbered + §16 source link.
- `rules/milestones.md` — rows 21.5 / 21.6 / 21.7 removed; trailing prose updated.

**Docs.** Self-contained — these *are* the doc updates.

---

## Chunk 16.3a — Late chunking pooling utility

**Date.** 2026-04-29

**Goal.** Ship the pure pooling half of the Jina AI 2024 late-chunking
technique as a fully-tested utility module — `mean_pool_token_embeddings`,
`pool_chunks`, and `spans_from_token_counts` — so the follow-up
ingest-pipeline integration (16.3b) becomes a thin glue layer rather
than a parallel rewrite.

**Why split into a/b.** Late chunking has two genuinely separable
halves: (1) the **pooling math** — given per-token embeddings + chunk
spans, mean-pool and L2-renormalise — which is pure, deterministic,
and unit-testable without any LLM, network, or DB; and (2) the
**ingest integration** — calling a long-context embedder that
returns per-token vectors and threading the result into
`run_ingest_task`, which depends on an Ollama model that exposes
per-token embeddings (none currently in the catalogue) and on the
`AppSettings.late_chunking` flag plumbing. Shipping (1) now locks
the contract the integration will compose against; shipping (2) is
deferred until a long-context embedder is pullable. Same a/b pattern
used for 16.4 (Self-RAG) and 16.5 (CRAG) earlier this session.

**What shipped.**

- `src-tauri/src/memory/late_chunking.rs` (~380 LOC, 15 tests):
  - `pub struct TokenSpan { start, end }` with `new`, `is_empty`, `len`.
  - `pub fn mean_pool_token_embeddings(token_embeddings: &[Vec<f32>], span: TokenSpan) -> Option<Vec<f32>>` — averages tokens in `span`, L2-renormalises so the result is directly comparable via cosine similarity. Returns `None` on: empty span, out-of-range span, dimensionality drift mid-document, zero-norm result, zero-dim tokens.
  - `pub fn pool_chunks(token_embeddings: &[Vec<f32>], spans: &[TokenSpan]) -> Vec<Option<Vec<f32>>>` — vectorised application aligned 1:1 with input spans so callers can zip with chunk metadata.
  - `pub fn spans_from_token_counts(&[usize]) -> Vec<TokenSpan>` — convenience builder for contiguous gap-free span lists.
- `src-tauri/src/memory/mod.rs` — `pub mod late_chunking;` registered alphabetically between `hyde` and `matryoshka`.

**Critical design choices.**

- **L2-renormalise, not just mean.** Raw mean of unit-norm token
  vectors has magnitude that grows roughly with √(span.len), which
  would bias cosine scores. Renormalising makes the pooled chunk
  embedding numerically identical-shape to anything else in the
  store.
- **Refuse rather than degrade.** Dimensionality drift mid-document,
  empty spans, and zero-norm means all return `None`. The ingest
  pipeline can decide whether to skip the chunk or fail the job —
  the utility doesn't silently pad/truncate.
- **f64 accumulator, f32 output.** Pooling 8k-token windows of
  768-dim vectors in f32 accumulates noticeable error; using f64
  internally costs nothing measurable and keeps the result clean.
- **Reuses the existing `Chunk` shape from `memory::chunking`.** No
  parallel type — when 16.3b lands, it will pass the same
  `Vec<Chunk>` around plus a parallel `Vec<TokenSpan>`.

**Tests.** 15 unit tests, all passing:

- `token_span_basics`, `pool_single_token_returns_normalized_input`,
  `pool_averages_then_normalises`,
  `pool_orthogonal_tokens_yields_45_degree_vector`,
  `pool_rejects_empty_span`, `pool_rejects_out_of_range_span`,
  `pool_rejects_dimension_mismatch`, `pool_rejects_zero_norm_mean`,
  `pool_rejects_zero_dim_tokens`, `pool_chunks_aligns_with_spans`,
  `spans_from_counts_round_trip`,
  `spans_from_counts_handles_empty_counts`,
  `spans_from_counts_empty_input_empty_output`,
  `pooled_output_is_unit_norm`, `pool_partial_span`.
- Whole `cargo test --lib`: **1300 passed**, 0 failed (was 1285 before this chunk → +15).
- `cargo clippy --all-targets -- -D warnings`: clean.

**Files touched.**

- `src-tauri/src/memory/late_chunking.rs` (new).
- `src-tauri/src/memory/mod.rs` (module registration).
- `docs/brain-advanced-design.md` — Phase 6 ASCII diagram flips
  `○ Late chunking` to `◐ Late chunking — pooling utility shipped (16.3a); 16.3b wires long-context embedder`; §19.2 row 9 status flips from 🔵 to 🟡 with file refs to `mean_pool_token_embeddings`, `pool_chunks`, `spans_from_token_counts`.
- `README.md` — Brain System list gains Late chunking pooling utility paragraph.
- `rules/milestones.md` — row 16.3 transformed into 16.3b
  ("ingest-integration"), keeping only the deferred half visible.

**Docs.** Brain Documentation Sync rule honoured —
`docs/brain-advanced-design.md` and `README.md` updated in the same
commit as the code.

---

## Chunk 16.5a — CRAG retrieval evaluator

**Date.** 2026-04-29

**Goal.** Ship the *evaluator half* of Corrective RAG (Yan et al.,
2024): a pure classifier that, given a `(query, document)` pair,
decides whether the document is `CORRECT` / `AMBIGUOUS` /
`INCORRECT` for that query — plus a corpus-level aggregator that
collapses per-document verdicts into a single retrieval-quality
classification the orchestrator can branch on.

**Why split into a/b** (mirrors 16.4 split). The original Chunk 16.5
spec was "evaluator + rewrite + web-search fallback". The evaluator
is the load-bearing piece — without it, the rewriter and web search
are firing blindly. By landing the evaluator standalone:

- **16.5a (this chunk)** — pure prompt builder + reply parser +
  aggregator. 100 % synchronous, 100 % testable without an LLM,
  tokio runtime, or DB. Independently useful: callers can use
  `RetrievalQuality::Incorrect` today as a confidence check before
  injecting low-quality memories into the system prompt.
- **16.5b (next-chunk row)** — wire the evaluator into a Tauri
  command, add an LLM-driven query rewriter (mirrors HyDE), and
  hook the web-search fallback (gated on the `code.read` / web-fetch
  capability surface). The web-search piece depends on the crawl
  pipeline.

**What shipped (16.5a).**

- `src-tauri/src/memory/crag.rs` (NEW, ~280 LOC):
  - `pub enum DocumentVerdict { Correct, Ambiguous, Incorrect }` —
    per-document classification.
  - `pub enum RetrievalQuality { Correct, Ambiguous, Incorrect }` —
    corpus-level aggregate. Used by orchestrator branching.
  - `pub fn build_evaluator_prompts(query, document) -> (String, String)`
    — mirrors `memory::reranker::build_rerank_prompts` shape (system
    + user) so the LLM-call pipeline is identical.
  - `pub fn parse_verdict(reply) -> Option<DocumentVerdict>` —
    case-insensitive, robust to chat-noise prefixes (`"Verdict:
    CORRECT"`), uses **whole-word** token matching to distinguish
    `CORRECT` from `INCORRECT` and reject substring-of-word
    matches like `"incorrectly"`.
  - `pub fn aggregate(&[DocumentVerdict]) -> RetrievalQuality` —
    canonical CRAG decision rule: any `Correct` → `Correct`; else
    any `Ambiguous` → `Ambiguous`; else `Incorrect` (including the
    empty-corpus case).
- `src-tauri/src/memory/mod.rs`: registered `pub mod crag;`.

**Decision rule (canonical CRAG aggregation).**

| Verdicts | Aggregate |
|---|---|
| at least one `Correct` | `Correct` (use as-is) |
| no `Correct`, ≥ 1 `Ambiguous` | `Ambiguous` (rewrite + retry) |
| all `Incorrect`, or empty | `Incorrect` (drop, seek alternatives) |

**Token-boundary check.** Critical edge case: `INCORRECT` contains
`CORRECT` as a substring. The parser's `find_token` helper rejects
matches that aren't bounded by string-end or non-alphanumeric
characters, so `"incorrectly phrased"` doesn't false-match
`INCORRECT`, and `"INCORRECT"` is correctly classified rather than
landing as `CORRECT`.

**Tests.** 1285 Rust tests pass (was 1271); `cargo clippy --all-targets
-- -D warnings` clean. 14 new unit tests covering: prompt-format
sanity, clean verdicts in all three states, case-insensitivity, chat-
noise tolerance ("Verdict: CORRECT"), earliest-keyword-wins on
multi-keyword replies, substring-of-word rejection (`"incorrectly"`,
`"correctness"`), empty/unrelated-reply handling, punctuation-
bounded tokens (`"(CORRECT)"`, `"CORRECT."`), CORRECT-vs-INCORRECT
disambiguation, and every aggregation branch (empty, any-correct,
ambiguous-only, all-incorrect, all-ambiguous).

**Files touched.**
- NEW: `src-tauri/src/memory/crag.rs` (~280 LOC, 14 tests).
- MODIFIED: `src-tauri/src/memory/mod.rs`.

**Docs.** §16 Phase 6 of `docs/brain-advanced-design.md` and §19.2
row 6 ("Corrective RAG / CRAG") to be flipped from 🔵 to 🟡
(evaluator only, rewriter + web-search pending) in the doc-tick pass.

---

## Chunk 16.4a — Self-RAG reflection-token controller

**Date.** 2026-04-29

**Goal.** Ship the *pure decision logic* half of Self-RAG (Asai et
al., 2023): a parser for the four reflection tokens
(`<Retrieve>` / `<Relevant>` / `<Supported>` / `<Useful>`), and a
state-machine controller that decides — given each LLM response —
whether to retrieve again, accept the answer, or refuse. Capped at
3 iterations per the milestone spec.

**Why split into a/b.** The original Chunk 16.4 was scoped as
"orchestrator loop with reflection tokens". The honest cleavage is:

- **16.4a (this chunk)** — pure controller, 100 % synchronous, 100 %
  testable without an LLM, tokio runtime, or DB. Independently
  useful: any future integration site can plug it in.
- **16.4b (next-chunk row)** — wire the controller into a Tauri
  streaming command that calls `OllamaAgent::respond_contextual` +
  `hybrid_search_rrf` in a loop. Depends on a streaming-pipeline
  decision (does each iteration re-emit `llm-chunk` events? does the
  frontend see intermediate failed attempts?) that's worth its own
  design pass.

**What shipped (16.4a).**

- `src-tauri/src/orchestrator/self_rag.rs` (NEW, ~440 LOC):
  - `pub enum RetrieveToken { Yes, No, Continue }`
  - `pub enum RelevantToken { Relevant, Irrelevant }`
  - `pub enum SupportedToken { Fully, Partially, No }`
  - `pub struct Reflection { retrieve, relevant, supported, useful }`
    with `is_complete()` for caller telemetry.
  - `pub fn parse_reflection(response) -> Reflection` — case-
    insensitive on tag names AND values; first-occurrence-wins on
    duplicates; out-of-range `<Useful>` (must be 1..=5) silently
    rejected; missing tokens yield `None` so a chatty model can't
    crash the controller.
  - `pub fn strip_reflection_tokens(response) -> String` — for
    user-visible rendering; collapses runs of blank lines created
    by stripping; tolerates malformed (no-close-tag) inputs.
  - `pub struct SelfRagController` with
    `new()` / `with_max_iterations(max)` (clamped 1..=10) /
    `iteration() -> u8` / `next_step(response) -> Decision`.
  - `pub enum Decision { Retrieve, Accept { answer }, Reject { reason } }`
    where `RejectReason::{ MaxIterationsExceeded, Unsupported }`.
  - `pub const SELF_RAG_SYSTEM_PROMPT: &str` — addendum that
    instructs an LLM to emit reflection tokens in our exact format.
  - `pub const DEFAULT_MAX_ITERATIONS: u8 = 3` and
    `pub const MIN_ACCEPTABLE_USEFULNESS: u8 = 3`.
- `src-tauri/src/orchestrator/mod.rs`: registered `pub mod self_rag;`.

**Decision rules** (table — implemented in `next_step`):

| Iteration cap reached? | `<Supported>` | `<Useful>` | `<Retrieve>` | Verdict |
|---|---|---|---|---|
| Yes | `NO` | * | * | Reject (Unsupported) |
| Yes | `FULLY` / `PARTIALLY` | * | * | Accept |
| Yes | missing | * | * | Reject (MaxIterationsExceeded) |
| No | * | * | `YES` / `CONTINUE` | Retrieve |
| No | `FULLY` | * | `NO` | Accept |
| No | `PARTIALLY` | ≥ 3 | `NO` | Accept |
| No | `PARTIALLY` | < 3 | `NO` | Retrieve |
| No | `NO` | * | `NO` | Retrieve |
| No | * | * | missing | Accept iff `FULLY`, else Retrieve |

**Why a fresh parser instead of reusing `StreamTagParser`.** The
existing parser at `src-tauri/src/commands/streaming.rs:45` is
hard-coded to `<anim>{"…"}</anim>` JSON-payload blocks streamed
mid-text, with state for partial-prefix holdback across chunk
boundaries. Self-RAG reflection tokens are emitted *at the end* of a
complete response, not streamed mid-generation, and contain plain
enum values rather than JSON. A simpler whole-string parser fits
better and avoids contaminating either surface with the other's
concerns. Documented in the module-level comment.

**Tests.** 1271 Rust tests pass (was 1251); `cargo clippy --all-targets
-- -D warnings` clean. 20 new unit tests covering: complete and
partial reflections, case-insensitivity, missing tokens, garbage
values, range checks on `<Useful>`, stripping (with malformed-input
tolerance), every branch of the decision table (accept/retrieve/reject
at and below cap), iteration counter advancement, max-iterations
clamping, and a sanity check that the system-prompt addendum
mentions all four tag families.

**Files touched.**
- NEW: `src-tauri/src/orchestrator/self_rag.rs` (~440 LOC, 20 tests).
- MODIFIED: `src-tauri/src/orchestrator/mod.rs`.

**Docs.** §16 Phase 6 of `docs/brain-advanced-design.md` and §19.2
row 5 ("Self-RAG") to be flipped from 🔵 to 🟡 (controller only,
loop pending) in the doc-tick pass.

---

## Chunk 16.8 — Matryoshka embeddings (two-stage vector search)

**Date.** 2026-04-29

**Goal.** Implement Matryoshka Representation Learning (Kusupati et
al., NeurIPS 2022) on the brute-force vector-search path. Truncate
the query embedding to 256 dims for a fast first-pass scan, then
re-rank only the top survivors with the full 768-dim embedding.
Cuts brute-force per-candidate cost ~3× with negligible recall hit
on the top-K — meaningful on cold-start (when the ANN index isn't
populated), on dimension-mismatch fallbacks (after model swap), and
on smaller corpora where the ANN overhead doesn't pay off.

**Why now.** The ANN index (Chunk 16.10) is optional and rebuilds
lazily; until it's hot, every query falls through to the O(n)
brute-force scan. Matryoshka makes that fallback path much cheaper
without touching the schema or the embedding model.

**What shipped.**

- `src-tauri/src/memory/matryoshka.rs` (NEW, ~330 LOC):
  - `pub fn truncate_and_normalize(emb, target_dim) -> Option<Vec<f32>>` —
    pure utility. Slices the first `target_dim` components and L2-
    renormalises so cosine similarity stays meaningful. Rejects
    `target_dim == 0`, `target_dim > emb.len()`, empty input, and
    zero-norm degenerate cases.
  - `pub fn two_stage_search(query, candidates, fast_dim, fast_top_k,
    final_top_k) -> Vec<ScoredId>` — pure function over a slice of
    `(id, full_embedding)` pairs. Stage 1: dot-product against
    truncated+renormalised query. Stage 2: full-dim cosine re-rank
    of survivors. Mismatched-dim candidates skipped up-front.
  - `pub const DEFAULT_FAST_DIM: usize = 256` — picked for
    `nomic-embed-text` per the model card.
  - 12 unit tests covering truncation invariants, unit-norm output,
    full-dim winner bubbling up despite a misleading truncated
    score, dim-mismatch filtering, empty-input handling, fallback
    when query truncation fails (`fast_dim > emb.len()`), agreement
    with `memory::store::cosine_similarity` for re-rank scoring,
    and degenerate `fast_top_k <= final_top_k`.
- `src-tauri/src/commands/memory.rs`: new `matryoshka_search_memories`
  Tauri command. Embeds the query via the same path as the other
  search commands (`embed()` helper), pulls all entries with
  embeddings, and runs `two_stage_search`. Returns `Vec<MemoryEntry>`
  in full-dim cosine order — drop-in compatible with
  `hybrid_search_memories_rrf` / `hyde_search_memories` callers.
- `src-tauri/src/memory/mod.rs`: registered `pub mod matryoshka;`.
- `src-tauri/src/lib.rs`: registered the new command.

**Why this module is pure.** Storage stays at full-dim — we never
persist truncated vectors. Truncation happens at query time only;
the cost is one slice + one L2 renormalise. No schema change, no
migration, no index rebuild. Feature can be turned on/off per-query.

**Not done.** The hybrid 6-signal pipeline still uses full-dim
vectors throughout. Wiring Matryoshka into `hybrid_search_rrf` /
`hybrid_search_with_threshold` is a future optimisation chunk —
the current change adds a new entry point so callers can opt in
explicitly via `matryoshka_search_memories`. ANN-index integration
(per-leaf truncated codes) is also future work.

**Tests.** 1251 Rust tests pass (was 1239); `cargo clippy --all-targets
-- -D warnings` clean.

**Files touched.**
- NEW: `src-tauri/src/memory/matryoshka.rs` (~330 LOC, 12 tests).
- MODIFIED: `src-tauri/src/memory/mod.rs`,
  `src-tauri/src/commands/memory.rs`,
  `src-tauri/src/lib.rs`.

**Docs.** §16 Phase 6 of `docs/brain-advanced-design.md` and §19.2
row 11 ("Matryoshka Representation Learning") to be flipped from 🔵
to ✅ in the doc-tick pass.

---

## Chunk 15.5 — Voice / chat intents (AI integrations)

**Date.** 2026-04-29

**Goal.** Recognise short, deterministic voice/chat phrases that drive
the AI-integrations control plane (start/stop/status the MCP server,
open VS Code, run the auto-setup writers for Copilot / Claude Desktop /
Codex) without involving the LLM intent classifier. The phrases are
high-stakes (they spawn processes and rewrite editor configs), so a
phrase-based matcher is faster, free, deterministic, and trivially
auditable — falling through to normal chat on anything it doesn't
recognise.

**What shipped.**

- `src-tauri/src/routing/ai_integrations.rs` (NEW, ~370 LOC):
  - `pub enum AiIntegrationIntent` with variants `McpStart`, `McpStop`,
    `McpStatus`, `VscodeOpenProject { target: Option<String> }`,
    `VscodeListKnown`, `AutosetupCopilot { transport }`,
    `AutosetupClaude { transport }`, `AutosetupCodex { transport }`.
  - `pub enum McpTransport { Http, Stdio }` — defaults to **stdio**
    (canonical since Chunk 15.9), bumps to HTTP when the utterance
    explicitly says "via http" / "over http" / "http transport".
  - `pub fn match_intent(text: &str) -> Option<AiIntegrationIntent>` —
    pure phrase matcher. Case-insensitive, punctuation-tolerant,
    whitespace-collapsing. Specific patterns tested before generic ones.
  - VS Code path extraction: handles "open `<path>` in vs code" /
    "open `<path>` in vscode". `looks_like_path()` rejects gibberish
    like "the door" by requiring `/`, `\`, `~/` or a Windows drive
    letter.
  - 19 unit tests covering MCP control, VS Code surfacing, autosetup
    writers, transport detection, punctuation tolerance, JSON tagging,
    and negative fall-through cases.
- `src-tauri/src/routing/mod.rs`: re-exports `match_intent`,
  `AiIntegrationIntent`, `McpTransport`.
- `src-tauri/src/commands/routing.rs`: new `match_ai_integration_intent`
  Tauri command — pure phrase matcher, no state needed, returns
  `Result<Option<AiIntegrationIntent>, String>`. Frontend pattern: call
  on every chat turn *before* the LLM; on `Some(intent)` route to the
  matching Tauri command (`mcp_server_start`, `setup_vscode_mcp_stdio`,
  `vscode_open_project`, etc.); on `None` proceed with normal chat.
- `src-tauri/src/lib.rs`: registered the new command in the
  `invoke_handler` block.

**Tests.** 1239 Rust tests pass (was 1220); `cargo clippy --all-targets
-- -D warnings` clean.

**Why phrase-based, not LLM.** The existing `intent_classifier` (chat
vs. teach vs. learn) is a separate concern; piggy-backing the AI-
integrations control intents onto it would (a) add latency to every
chat turn for high-stakes branches, (b) introduce a non-zero false-
positive rate on commands that spawn processes, and (c) make these
phrases harder to audit. The phrase matcher is O(n) over a small
constant table and runs in ~microseconds.

**ai-bridge skill gate.** The skill-tree quest activation lives in
the frontend (`src/stores/skill-tree.ts`). The Rust matcher is
gate-agnostic — it always returns matches; the frontend decides whether
to dispatch the matched intent based on whether the relevant
integration is configured. Skill activation happens organically when
the user runs an autosetup command for the first time.

**Files touched.**
- NEW: `src-tauri/src/routing/ai_integrations.rs` (~370 LOC).
- MODIFIED: `src-tauri/src/routing/mod.rs`,
  `src-tauri/src/commands/routing.rs`, `src-tauri/src/lib.rs`.

---

## Chunk 15.10 — VS Code workspace surfacing

**Date.** 2026-04-29

**Goal.** Resolve "open this project in VS Code" intelligently: focus an
existing window if one (or any ancestor of `target_path`) is already open,
else launch a new `code <target>` window. Foundation for the Copilot
autonomous loop so next-chunk prompts always land in the right editor
window — and the Control Panel's "📂 Open project in VS Code" button
(Chunk 15.4).

**Architecture.**

```
src-tauri/src/vscode_workspace/
├── mod.rs       — public API: open_project, list_known_windows, forget_window
├── path_norm.rs — canonicalise + case-fold (Windows/macOS) for prefix match
├── registry.rs  — SelfLaunchedRegistry, JSON-on-disk, PID-validated
├── resolver.rs  — pure pick_window(target, &[VsCodeWindow]) -> WindowChoice
└── launcher.rs  — cross-platform `code <path>` spawn, detached child

src-tauri/src/commands/vscode.rs
├── vscode_open_project(target_path) -> OpenOutcome
├── vscode_list_known_windows() -> Vec<VsCodeWindow>
└── vscode_forget_window(pid)
```

**Resolver algorithm** (per the milestones spec, fully unit-testable
with injected `pid_alive`):

1. For each registered window, classify against canonicalised target:
   `Exact` if equal, `Ancestor { depth }` if `target_inside_root`.
2. Drop dead PIDs via `sysinfo::System::refresh_processes_specifics`.
3. `Exact` always beats `Ancestor`; ties broken by most-recent launch.
4. Among `Ancestor` candidates, pick the deepest root (most components,
   "most-children-near-target"); ties broken by most-recent launch.
5. Otherwise return `WindowChoice::None` → caller spawns fresh window.

`open_project` re-launches `code <window.root>` for reuse (not
`code <subpath>`, which would create a new window) — the existing
window already contains the subpath so the user navigates inside VS Code.

**Cross-platform path matching.** `path_norm::canonicalise` resolves
`..` and symlinks via `std::fs::canonicalize`, then strips the
`\\?\` verbatim prefix on Windows. Comparison helpers
(`paths_equal`, `target_inside_root`, `depth_below`) lowercase paths
on Windows and macOS to match filesystem case-insensitivity, while
preserving case-sensitive semantics on Linux.

**Registry persistence.** `<data_dir>/vscode-windows.json`, atomic
write via temp-file + rename. Format version 1; corrupt files or
version mismatches yield an empty registry (worst case: TerranSoul
forgets some windows and launches fresh ones — harmless).
Liveness-pruned on every read; PIDs reset on OS reboot so stale
entries never linger.

**Launcher.** `Command::new("code")` (Linux/macOS) or `code.cmd`
(Windows, resolved via `PATHEXT`). Child stdio is `null`-redirected;
the spawned process is `mem::forget`-ed so we never wait on it,
giving fire-and-forget detached semantics without needing
`pre_exec` / `setsid`. `NotFound` errors translate to a friendly
"Run Cmd+Shift+P → 'Shell Command: Install code in PATH'" message.

**Out of scope (documented in milestones design notes).**

- Multi-root `.code-workspace` files — registry stores them as opaque.
- Discovery of manually-opened VS Code windows — v1 only knows about
  windows it launched itself. Folded into a future
  `WorkspaceStorageScanner` follow-up.
- Insiders / VSCodium / Cursor — `CODE_BIN` is a single constant for v1.
- Remote / WSL workspaces (`vscode-remote://...`) — never reused.
- Highlighting a sub-path inside a focused ancestor.

**Files created.**

- `src-tauri/src/vscode_workspace/mod.rs` (~210 LOC)
- `src-tauri/src/vscode_workspace/path_norm.rs` (~130 LOC)
- `src-tauri/src/vscode_workspace/registry.rs` (~250 LOC)
- `src-tauri/src/vscode_workspace/resolver.rs` (~250 LOC)
- `src-tauri/src/vscode_workspace/launcher.rs` (~130 LOC)
- `src-tauri/src/commands/vscode.rs` (~60 LOC)

**Files modified.**

- `src-tauri/src/lib.rs` — declared `pub mod vscode_workspace`,
  imported and registered the 3 new Tauri commands.
- `src-tauri/src/commands/mod.rs` — declared `pub mod vscode`.

**Test counts.** 37 new unit tests across the 6 module files:
- `path_norm` — 7 tests (equality, case folding, ancestor matching,
  depth, specificity; Windows-only tests gated with `#[cfg(windows)]`).
- `registry` — 10 tests (load/save round-trip, append, forget,
  prune, corrupt-file recovery, version-mismatch recovery, atomic
  parent-dir creation, real-PID liveness checks).
- `resolver` — 11 tests (empty, exact-wins, ancestor depth,
  deepest-ancestor-wins, three-window chain, dead-pid filter,
  dead-exact-falls-through, unrelated target, duplicate-exact
  most-recent, tie-breaking, exact-beats-ancestor).
- `launcher` — 2 tests (Unix-only PATH-hijack failure path; cross-platform
  error-message readability).
- `mod` — 5 tests (missing-target rejection, empty `list_known_windows`,
  `forget_window` no-op + remove, `now_ms` sanity).

All 1183 existing Rust tests still green; clippy `-D warnings` clean.

**Acceptance scenarios** (from the milestones spec; first three
verified by unit tests, real `code` exec covered by future
`TERRANSOUL_VSCODE_INTEGRATION=1` integration tests):

- Empty registry → `vscode_open_project` launches new window.
- Same target called twice → second call focuses the registered window.
- Three windows at `D:\`, `D:\Git\`, `D:\Git\TerranSoul\`; target
  `D:\Git\TerranSoul\src` → focuses `D:\Git\TerranSoul\` (deepest
  ancestor wins).
- Killed window's PID is no longer alive → resolver falls through to
  launch-new and rewrites the registry.
- Missing target path → clear `TargetMissing` error without touching
  the registry.

**Notes.**

- Concurrent `vscode_open_project` calls are serialised by the caller —
  Tauri's `spawn_blocking` keeps each invocation on its own thread,
  but the registry file is rewritten atomically so the worst case is
  the last writer wins (one extra entry that gets pruned on next
  load if its PID is dead).
- Voice / chat intents (`vscode.open_project`, `vscode.list_known`)
  and the Control Panel's status pill are folded into Chunks 15.5 and
  15.4 respectively; this chunk ships only the Tauri-level surface.

---

## Chunk 15.9 — MCP stdio transport shim

**Date.** 2026-04-29

**Goal.** Ship the canonical MCP transport (newline-delimited JSON-RPC over
stdin/stdout) alongside the existing loopback HTTP transport, so editors that
prefer stdio (Claude Desktop, the VS Code MCP extension, Codex CLI defaults)
can connect to TerranSoul's brain without a TCP listener. Single binary
entry point — no separate companion exe — guarded by a CLI flag so a normal
launch still spawns the GUI.

**Architecture.**

- New module `src-tauri/src/ai_integrations/mcp/stdio.rs` —
  `run_loop<R, W>` reads NDJSON requests from any `AsyncRead`, dispatches via
  the shared `router::dispatch_method`, writes NDJSON responses to any
  `AsyncWrite`. Exits cleanly on EOF; parse errors emit a JSON-RPC `-32700`
  reply but keep the loop alive. Notifications (no `id`) produce no output
  per JSON-RPC 2.0.
- `router.rs` refactored to expose `pub(crate) dispatch_method(gw, caps,
  method, params, id) -> JsonRpcResponse` so the HTTP and stdio handlers
  share one source of truth for the `initialize` / `tools/list` /
  `tools/call` / `ping` surface. The HTTP handler keeps bearer-token auth;
  stdio runs in a trusted parent–child relationship and skips auth (matches
  canonical MCP behaviour — Claude Desktop, the VS Code MCP extension, etc.
  never pass tokens to stdio servers).
- `lib.rs` gains `pub fn run_stdio() -> std::io::Result<()>` plus a
  private `resolve_data_dir_for_cli()` that mirrors the GUI's
  `app_data_dir / dev` split using the `dirs` crate (no Tauri `AppHandle`
  required) but **never** wipes the dev directory.
- `main.rs` checks `std::env::args` for `--mcp-stdio` *before* calling
  `terransoul_lib::run()`. When present, runs the stdio shim and exits;
  otherwise launches the GUI as normal.
- `auto_setup.rs` gains stdio entry builders + writers
  (`write_vscode_stdio_config`, `write_claude_stdio_config`,
  `write_codex_stdio_config`) sharing a private `upsert_entry` helper
  with the existing HTTP writers. Switching transport overwrites the
  previous entry cleanly — no stale `url` / `headers` fields leak through.
- `commands/auto_setup.rs` adds three new Tauri commands
  (`setup_vscode_mcp_stdio`, `setup_claude_mcp_stdio`,
  `setup_codex_mcp_stdio`) tracked under separate `mcp_*_stdio` quest keys.

**Test counts.** 9 stdio loop tests + 5 stdio auto-setup tests = 14 new.
All 1115+ existing Rust tests green; clippy clean.

**Notes / out-of-scope.**

- Windows release builds use `windows_subsystem = "windows"`; editors that
  spawn TerranSoul with explicit `STARTUPINFO::hStd*` redirection (every
  cited MCP client) inherit working pipes, so this does not break stdio.
- No bearer-token validation on stdio — canonical MCP behaviour.
- Frontend Control Panel transport picker writing the stdio config via the
  new commands lives in Chunk 15.4.

---

## Chunk 23.0 — Multi-agent resilience scaffold (per-agent threads, workflow resilience, agent swap context)

**Date.** 2026-04-25

**Scope.** Three interconnected improvements for multi-agent reliability, resilience, and atomicity. This is the **scaffold only** — library code + per-agent stamping. The wiring chunks (23.1 ResilientRunner integration, 23.3 per-agent thread UI) are still pending in milestones.

### 1. Per-agent conversation threads
- Added `agent_id: Option<String>` to Rust `Message` struct (`commands/chat.rs`) and all 6 construction sites (chat.rs ×2, streaming.rs ×3, ipc_contract_tests.rs ×1)
- Added `agentId?: string` to TypeScript `Message` interface (`types/index.ts`)
- Conversation store gains: `activeAgentId()`, `agentMessages` computed (filters by active agent), `agentSwitchHistory` ref, `setAgent()`, `stampAgent()` helper
- All user/assistant messages stamped with the active agent ID via `stampAgent()`

### 2. Workflow resilience (Temporal.io patterns via `backon` crate)
- **New file:** `src-tauri/src/workflows/resilience.rs` (~480 LOC, 13 tests)
- **`RetryPolicy`** — configurable max_attempts, exponential backoff (min/max interval, multiplier), non-retryable error filter. Uses `backon` crate for battle-tested retry-with-backoff.
- **`TimeoutPolicy`** — workflow-level + activity-level + heartbeat timeouts via `tokio::time::timeout`
- **`CircuitBreaker`** — 3-state FSM (Closed → Open → HalfOpen → Closed). Failure threshold, recovery timeout, probe-on-half-open, metrics snapshot.
- **`HeartbeatWatchdog`** — tracks last-seen timestamps per workflow, detects stale workflows exceeding configurable threshold.
- **`ResilientRunner`** — combined runner: circuit breaker → retry → timeout (outermost → innermost). Single entry point for resilient activity execution.
- Added `backon = "1"` to Cargo.toml

### 3. Agent swap with context summary
- `switchAgent()` now accepts optional `conversationMessages` parameter
- On agent switch, builds a plain-text handoff context from the outgoing agent's recent messages (up to 20)
- `handoffContexts` map stores per-agent context summaries
- `getHandoffContext(agentId)` retrieves the summary for injection into system prompts
- Backward-compatible: existing callers without the second argument still work

**Tests.** 1112+ Rust (13 new resilience tests), 1164 Vitest, clippy clean.

---

## Chunk 23.0b — Stop & Stop-and-Send Controls (TaskControls)

**Date:** 2026-04-25 (backfilled 2026-04-30)

**What shipped.** New component `src/components/TaskControls.vue` + test file
`src/components/TaskControls.test.ts`, wired into `src/views/ChatView.vue`.
Provides Stop (cancels current generation) and Stop-and-Send (stops generation
and immediately sends the partial response as context for user's next message)
buttons. Backed by `conversationStore.stopGeneration()` / `stopAndSend()`
methods in `src/stores/conversation.ts`. Shipped in the same multi-agent
resilience PR as Chunk 23.0.

---

## Chunk 16.7 — Sleep-time consolidation

**Date.** 2026-04-25

**Goal.** Idle-triggered background workflow that consolidates memory:
compress short→working, link related memories by embedding similarity,
promote high-access working→long, apply decay + GC, adjust importance.

**Architecture.**
- 5-step pipeline: compress → link → promote → decay+GC → importance
- `ActivityTracker` (AtomicI64) for idle detection
- `ConsolidationConfig` + `ConsolidationResult` DTOs
- `cosine_similarity()` helper for embedding-based linking
- Each step is non-fatal — failures collected as warnings

**Files created.**
- `src-tauri/src/memory/consolidation.rs` (~340 LOC, 9 tests)
- `src-tauri/src/commands/consolidation.rs` (~45 LOC, 3 commands)

**Files modified.**
- `src-tauri/src/memory/mod.rs` — added `pub mod consolidation;`
- `src-tauri/src/commands/mod.rs` — added `pub mod consolidation;`
- `src-tauri/src/lib.rs` — added `activity_tracker` to `AppStateInner`, registered 3 Tauri commands

**Tauri commands.**
- `run_sleep_consolidation(session_ids, config?)` — trigger full consolidation
- `touch_activity()` — reset idle timer
- `get_idle_status(threshold_ms?)` — query idle state

**Test count.** 9 tests (all pass). CI: 1090+ Rust tests.

**Depends on.** 17.1 (auto-promote), 17.4 (importance adjustment), edges (V5 schema).

---

## Chunk 15.6 — Auto-setup writers for Copilot, Claude Desktop, Codex

**Date.** 2026-04-25

**Goal.** One-click setup of external AI coding assistant MCP integrations. TerranSoul writes the correct config file for each client, preserving existing entries.

**Architecture.** Pure-function config writers in `src-tauri/src/ai_integrations/mcp/auto_setup.rs` + 7 Tauri commands in `src-tauri/src/commands/auto_setup.rs`.

**Supported clients:**

| Client | Config path | Key |
|---|---|---|
| VS Code / Copilot | `<workspace>/.vscode/mcp.json` | `servers.terransoul-brain` |
| Claude Desktop | `%APPDATA%/Claude/claude_desktop_config.json` (Win) / `~/.config/Claude/` (Linux/macOS) | `mcpServers.terransoul-brain` |
| Codex CLI | `~/.codex/config.json` | `mcpServers.terransoul-brain` |

**Key features:**
- **Preserve existing config** — reads file, merges, writes back. Never overwrites other servers.
- **Idempotent** — calling setup twice updates (never duplicates) the entry.
- **Atomic writes** — temp file + rename pattern.
- **JSONC support** — strips `//` and `/* */` comments before parsing (VS Code uses JSONC).
- **Undoable** — `remove_*_mcp` commands delete only the `terransoul-brain` entry.
- **Status listing** — `list_mcp_clients` checks which clients are configured.

**Files created:**
- `src-tauri/src/ai_integrations/mcp/auto_setup.rs` (~350 LOC, 14 unit tests)
- `src-tauri/src/commands/auto_setup.rs` (~90 LOC, 7 Tauri commands)

**Files modified:**
- `src-tauri/src/ai_integrations/mcp/mod.rs` — added `pub mod auto_setup`
- `src-tauri/src/commands/mod.rs` — added `pub mod auto_setup`
- `src-tauri/src/lib.rs` — registered 7 new commands in `invoke_handler`, added `use commands::auto_setup::*`
- `src-tauri/Cargo.toml` — added `dirs = "6"` dependency

**Tauri commands (7):**
`setup_vscode_mcp`, `setup_claude_mcp`, `setup_codex_mcp`, `remove_vscode_mcp`, `remove_claude_mcp`, `remove_codex_mcp`, `list_mcp_clients`

**Tests.** 14 unit tests: entry structure (3), write new (1), preserve existing (1), idempotent (1), remove (1), remove nonexistent (1), JSONC strip (2), atomic write parent dirs (1), client status detect (2), Claude write (1).

**CI.** 1164 Vitest ✅ | 1089 Rust tests ✅ (14 new) | clippy clean

**Note.** The stdio transport shim originally planned in this chunk was deferred — HTTP transport on `127.0.0.1:7421` is sufficient for all three clients today. Stdio can be added later if needed.

---

## Chunks 10.1 / 10.2 / 10.3 — Copilot Autonomous Mode + Auto-Resume + Health Gate

**Date.** 2026-04-25

**Goal.** Set up VS Code workspace for long-running autonomous Copilot agent sessions with auto-approve permissions, auto-resume tooling, service health gates, and MCP server integration.

**Architecture.** Three chunks shipped together as a cohesive developer-experience package:

- **10.1 (Autonomous Mode)** — `.vscode/settings.json` with `chat.permissions.default: "autopilot"`, 100-request budget, terminal auto-approve for safe build/test commands, file edit auto-approve for all workspace paths, conversation auto-summarization enabled.
- **10.2 (Auto-Retrigger)** — `scripts/copilot-loop.mjs` parses `rules/milestones.md` and `rules/completion-log.md` to generate context-rich "Continue" prompts. Copies to clipboard for paste into new sessions. Tracks session progress in `.vscode/copilot-session.log` (git-ignored). Modes: `--status`, `--next`, `--log`.
- **10.3 (Health Gate)** — `scripts/wait-for-service.mjs` polls any HTTP endpoint with configurable timeout. Supports dev server (`:1420`), Ollama (`:11434`), and MCP server (`:7421`). Wired into `.vscode/tasks.json` as pre-tasks.

**Additional deliverables:**
- `.vscode/tasks.json` — 9 tasks: Dev Server, Wait for Dev Server / Ollama / MCP Server, Run All Tests, Cargo Check + Clippy, Vue TypeScript Check, Full CI Gate, Copilot: Continue Session.
- `.vscode/mcp.json` — MCP server config for VS Code Copilot (HTTP transport, `${env:TERRANSOUL_MCP_TOKEN}` auth).
- `.github/copilot-instructions.md` — Added "Session Resumption & Progress Tracking" section with long-running session guidelines and MCP server reference.
- Phase 10 chunks promoted from `rules/backlog.md` to `rules/milestones.md` then archived here.

**Files created:**
- `.vscode/settings.json` (~115 lines)
- `.vscode/tasks.json` (~85 lines)
- `.vscode/mcp.json` (~25 lines)
- `scripts/wait-for-service.mjs` (~60 lines)
- `scripts/copilot-loop.mjs` (~190 lines)

**Files modified:**
- `.github/copilot-instructions.md` — Session Resumption section added
- `rules/milestones.md` — Phase 10 promoted from backlog, then all 3 chunks archived
- `rules/backlog.md` — Phase 10 chunks replaced with "Promoted" note

**Tests.** No new unit tests — these are config/script files. Manual verification: `copilot-loop.mjs --status` parses milestones correctly, `wait-for-service.mjs` times out on unreachable endpoint with exit code 1.

**CI.** 1164 Vitest ✅ | 1075 Rust tests ✅ (1 known flaky: `hybrid_search_rrf_keyword_ranking`)

---

## Chunk 15.1 — MCP server

**Date.** 2026-04-25

**Goal.** Expose TerranSoul's brain to AI coding assistants (GitHub Copilot, Claude Desktop, Cursor, Codex) via an MCP-compatible HTTP server on `127.0.0.1:7421` with bearer-token auth and all 8 gateway ops.

**Architecture.**
- **Transport:** Streamable HTTP (POST `/mcp`) on axum — milestones-endorsed fallback since `rmcp`'s SSE transport API wasn't needed for request/response ops.
- **Protocol:** JSON-RPC 2.0 per MCP 2024-11-05 spec. Handles `initialize`, `tools/list`, `tools/call`, `ping`, and notifications.
- **Auth:** Bearer token from `<data_dir>/mcp-token.txt` (SHA-256 of UUID v4, `0600` permissions on Unix).
- **Ops:** 8 tools (`brain_search`, `brain_get_entry`, `brain_list_recent`, `brain_kg_neighbors`, `brain_summarize`, `brain_suggest_context`, `brain_ingest_url`, `brain_health`) routed to `BrainGateway` trait via `AppStateGateway`.
- **AppState refactor:** Wrapped `AppState` as a newtype around `Arc<AppStateInner>` with `Deref` + `Clone`. Zero signature changes to existing 150+ Tauri commands — all auto-deref through the newtype. Enables cheap cloning for MCP server (and future gRPC server).

**Files created.**
- `src-tauri/src/ai_integrations/mcp/mod.rs` — module entry, `McpServerHandle`, `start_server()` async function.
- `src-tauri/src/ai_integrations/mcp/auth.rs` — token file CRUD (`load_or_create`, `regenerate`), SHA-256 generation.
- `src-tauri/src/ai_integrations/mcp/router.rs` — axum JSON-RPC 2.0 handler, bearer auth validation, MCP protocol dispatch.
- `src-tauri/src/ai_integrations/mcp/tools.rs` — 8 tool definitions (JSON Schema) + `dispatch()` function routing to gateway.
- `src-tauri/src/ai_integrations/mcp/integration_tests.rs` — 11 integration tests (full HTTP round-trips).
- `src-tauri/src/commands/mcp.rs` — 4 Tauri commands (`mcp_server_start`, `mcp_server_stop`, `mcp_server_status`, `mcp_regenerate_token`).

**Files modified.**
- `src-tauri/src/lib.rs` — `AppState` newtype wrapper (`AppState(Arc<AppStateInner>)` + `Deref` + `Clone`), added `mcp_server: TokioMutex<Option<McpServerHandle>>` field, wired 4 MCP commands to invoke_handler.
- `src-tauri/src/ai_integrations/gateway.rs` — `AppStateGateway` now takes `AppState` (cheaply clonable) instead of `Arc<AppState>`.
- `src-tauri/src/ai_integrations/mod.rs` — added `pub mod mcp;`.
- `src-tauri/src/commands/mod.rs` — added `pub mod mcp;`.

**Tests.** 22 new Rust tests (4 auth, 6 router, 3 tools, 11 integration). Baseline: 1053 → 1075 total. Clippy clean. vue-tsc clean.

---

## Chunk 14.12 — Phoneme-aware viseme model

**Date.** 2026-04-25

**Summary.** Replaced the FFT band-energy lip-sync fallback with a deterministic text-driven phoneme-to-viseme mapper. English graphemes (including 15 digraphs/trigraphs) are tokenized into the existing 5-channel viseme space (`aa`, `ih`, `ou`, `ee`, `oh`), then distributed proportionally across the audio duration to produce a frame-accurate timeline. The `VisemeScheduler` class samples interpolated weights per animation frame. Integrated into `useLipSyncBridge` — phoneme-driven visemes take priority when text + duration are available; FFT analysis remains as automatic fallback for external audio sources.

**Architecture.**
- `phoneme-viseme.ts`: `tokenizeToVisemes()` — grapheme tokenizer with digraph-first matching (th, sh, ch, oo, ee, etc.). `buildVisemeTimeline()` — proportional keyframe builder. `VisemeScheduler` — frame-accurate sampler with lerp between keyframes.
- `useLipSyncBridge.ts`: dual-mode tick loop — `phonemeScheduler.sample()` preferred, `lipSync.getVisemeValues()` fallback. Auto-schedule on `onAudioStart` using `tts.currentSentence` + `audio.duration`.

**Files created.**
- `src/renderer/phoneme-viseme.ts` — tokenizer + timeline builder + scheduler (~230 LOC)
- `src/renderer/phoneme-viseme.test.ts` — 22 unit tests (tokenizer, timeline, scheduler)

**Files modified.**
- `src/composables/useLipSyncBridge.ts` — added `VisemeScheduler` integration, `schedulePhonemes()` API, dual-mode tick

**Test count after.** 1164 Vitest (22 new); 1053 Rust (unchanged).

---

## Chunks 14.9 / 14.10 / 14.11 — Learned asset persistence + player + bundle

**Date.** 2026-04-25

**Summary.** Shipped the learned-asset persistence + playback trifecta. Chunk 14.9 (expression presets) and 14.11 (persona side-chain bundle) were already fully implemented in prior chunks — the backend CRUD commands, frontend store wiring, and persona pack export/import all existed. Chunk 14.10's new deliverable is `LearnedMotionPlayer` + expression preview helper, wired into CharacterViewport with a cross-view Pinia bridge so PersonaPanel's "Play" / "Preview" buttons work from BrainView.

**Architecture.**
- `learned-motion-player.ts`: `LearnedMotionPlayer` class wraps `bakeMotionToClip()` (14.5) + `VrmaManager.playClip()`. `applyLearnedExpression()` + `clearExpressionPreview()` static helpers set/reset VRM expression manager weights for timed previews.
- `VrmaManager.vrm` getter: exposes the bound VRM model for expression preview access.
- Persona store bridge: `previewExpressionRequest` / `previewMotionRequest` refs + `requestExpressionPreview()` / `requestMotionPreview()` actions. PersonaPanel writes, CharacterViewport watches and consumes.
- PersonaPanel: "▶ Preview" buttons for expressions, "▶ Play" buttons for motions.

**Files created.**
- `src/renderer/learned-motion-player.ts` — player + expression preview helpers (~80 LOC)
- `src/renderer/learned-motion-player.test.ts` — 10 unit tests

**Files modified.**
- `src/renderer/vrma-manager.ts` — added `vrm` getter
- `src/stores/persona.ts` — added preview request refs + actions
- `src/components/CharacterViewport.vue` — wired `LearnedMotionPlayer`, persona preview watchers
- `src/components/PersonaPanel.vue` — added Preview/Play buttons for expressions and motions

**Test count after.** 1142 Vitest (10 new); 1053 Rust (unchanged).

---

## Chunk 14.5 — VRMA baking

**Date.** 2026-04-25

**Summary.** Shipped `vrma-baker.ts` — bakes recorded `LearnedMotion` JSON frame timelines into `THREE.AnimationClip` objects with quaternion keyframe tracks, so the avatar can replay learned motions through the existing `VrmaManager` instead of streaming landmarks per-frame. Added `playClip()` to `VrmaManager` for playing pre-built clips without loading from file.

**Architecture.**
- `vrma-baker.ts`: Pure `bakeMotionToClip()` converts per-bone Euler triples to `QuaternionKeyframeTrack[]` → `AnimationClip`. `bakeAllMotions()` batch-bakes to a trigger-keyed Map.
- `VrmaManager.playClip()`: Accepts a pre-built `AnimationClip`, reuses the same fadeOut/fadeIn/action pipeline as `play()`. Refactored `play()` to delegate to `playClip()` after loading.

**Files created.**
- `src/renderer/vrma-baker.ts` — pure baker (~100 LOC)
- `src/renderer/vrma-baker.test.ts` — 12 unit tests (empty frames, quaternion validity, batch bake, etc.)

**Files modified.**
- `src/renderer/vrma-manager.ts` — added `playClip()`, refactored `play()` to delegate

**Test count after.** 1132 Vitest (12 new); 1053 Rust (unchanged).

---

## Chunk 14.4 — Motion-capture camera quest

**Date.** 2026-04-25

**Summary.** Shipped the `motion-capture` quest — PoseLandmarker (33 keypoints) → VRM humanoid bone retargeting via inverse trig, with real-time EMA smoothing and fixed-FPS recording (30 fps, max 10s). Reuses the same per-session camera consent from 14.3. PersonaTeacher.vue gained a mode toggle (Expression / Motion tabs), record/stop/save flow, and saved motions list.

**Architecture.**
- `pose-mirror.ts`: Pure `retargetPoseToVRM()` function (unit-testable seam) maps 33 MediaPipe landmarks → 11 VRM humanoid bones via atan2-based joint angle extraction with per-bone clamping. `PoseMirror` class wraps lazy-loaded PoseLandmarker. `smoothBonePose()` applies EMA with graceful decay when landmarks are lost.
- `PersonaTeacher.vue`: Expression/Motion tab toggle, motion recording at 30 fps with auto-stop at 10s, save via `save_learned_motion` Tauri command, saved motions list with duration display.

**Files created.**
- `src/renderer/pose-mirror.ts` — pure retargeter + PoseMirror class (~260 LOC)
- `src/renderer/pose-mirror.test.ts` — 11 unit tests on the pure retargeter

**Files modified.**
- `src/components/PersonaTeacher.vue` — added Motion tab, recording flow, saved motions list
- `src/components/PersonaTeacher.test.ts` — updated for new tab layout

**Test count after.** 1120 Vitest (11 new); 1053 Rust (unchanged).

**Activation gate.** `motion-capture` quest auto-activates when `persona.learnedMotions.length > 0` — already wired in skill-tree.ts.

---

## Chunk 14.3 — Expressions-pack camera quest

**Date.** 2026-04-25

**Summary.** Shipped the `expressions-pack` camera quest — per-session webcam capture with MediaPipe FaceLandmarker (52 ARKit blendshapes) mapped to TerranSoul's 12+2 VRM expression channels. Includes a pure ARKit→VRM mapper (`face-mirror.ts`), per-session consent composable (`useCameraCapture.ts`), "Teach an Expression" panel (`PersonaTeacher.vue`), idle-timeout auto-stop (5 min), and camera live badge. The `@mediapipe/tasks-vision` dependency is lazy-imported to avoid bundle bloat until the quest is used.

**Architecture.**
- `face-mirror.ts`: Pure `mapBlendshapesToVRM()` function (unit-testable seam) maps 52 ARKit blendshape coefficients → happy/sad/angry/relaxed/surprised/neutral + 5 visemes + blink + lookAt, following the `docs/persona-design.md` § 6.1 mapping table. `FaceMirror` class wraps MediaPipe FaceLandmarker with lazy WASM init and EMA smoothing.
- `useCameraCapture.ts`: Per-session getUserMedia + FaceMirror lifecycle. Camera consent is in-memory only (no on-disk flag). Auto-stops on unmount, idle timeout, or explicit stop.
- `PersonaTeacher.vue`: 4-step UI flow — consent dialog → live camera preview with CAMERA LIVE badge → capture pose → name + trigger word → save to Tauri backend via `save_learned_expression` command.

**Files created.**
- `src/renderer/face-mirror.ts` — pure mapper + FaceMirror class (~200 LOC)
- `src/renderer/face-mirror.test.ts` — 16 unit tests on the pure mapper
- `src/composables/useCameraCapture.ts` — camera session composable (~130 LOC)
- `src/components/PersonaTeacher.vue` — teach expression panel (~310 LOC)
- `src/components/PersonaTeacher.test.ts` — 5 component tests

**Dependencies added.**
- `@mediapipe/tasks-vision` (Apache-2.0, ~3 MB, lazy-loaded)

**Test count after.** 1109 Vitest (21 new); 1053 Rust (unchanged).

**Activation gate.** `expressions-pack` quest auto-activates when `persona.learnedExpressions.length > 0` — already wired in skill-tree.ts.

---

## Chunk 16.10 — ANN index (usearch)

**Date.** 2026-04-25

**Summary.** Replace brute-force O(n) cosine scan in `vector_search` and `find_duplicate` with an HNSW ANN index via the `usearch` crate (v2.25). Index is lazily initialized on first vector operation, auto-rebuilt from DB embeddings when missing, and periodically persisted to `vectors.usearch` alongside `memory.db`. Falls back to brute-force when the index is unavailable (dimension mismatch, empty DB, corrupt file).

**Files changed.**

| File | What |
|------|------|
| `src-tauri/src/memory/ann_index.rs` | **NEW** — `AnnIndex` wrapper (HNSW via usearch), `detect_dimensions()`, save/load/rebuild, 8 tests. |
| `src-tauri/src/memory/mod.rs` | Added `pub mod ann_index;` |
| `src-tauri/src/memory/store.rs` | Added `ann: OnceCell<AnnIndex>` + `data_dir` fields; `ann_index()` lazy init; `ensure_ann_for_dim()`; `vector_search` ANN fast-path; `find_duplicate` ANN fast-path; `set_embedding` updates index; `delete` removes from index |
| `src-tauri/Cargo.toml` | Added `usearch = "2"` dependency |

**Test counts.** 1053 Rust (+8 new), 1083 Vitest (unchanged).

---

## Chunk 17.6 — Edge conflict detection

**Date.** 2026-04-26

**Summary.** Scheduled LLM-as-judge scan over `memory_edges` with positive relation types (supports, implies, related_to, derived_from, cites, part_of). When the LLM says two connected memories actually contradict, a `"contradicts"` edge is inserted and a `MemoryConflict` row is opened for the user to resolve. Lock-safe three-phase pattern: collect candidates → async LLM calls → write results.

**Files changed.**

| File | What |
|------|------|
| `src-tauri/src/memory/edge_conflict_scan.rs` | **NEW** — `collect_scan_candidates()`, `record_contradiction()`, `has_contradicts_edge()`, `has_open_conflict()`, `ScanCandidates`, `EdgeConflictScanResult`. 6 tests. |
| `src-tauri/src/memory/mod.rs` | Added `pub mod edge_conflict_scan;` |
| `src-tauri/src/commands/memory.rs` | `scan_edge_conflicts` Tauri command — 3-phase lock-safe pattern |
| `src-tauri/src/lib.rs` | Registered `scan_edge_conflicts` in imports + handler list |

**Test counts.** 1045 Rust (6 new), 1083 Vitest (unchanged).

---

## Chunk 16.9 — Cloud embedding API for free / paid modes

**Date.** 2026-04-26

**What.** Extended the embedding pipeline to dispatch to OpenAI-compatible `/v1/embeddings` when the brain mode is `FreeApi` or `PaidApi`, so cloud users get real vector RAG quality without requiring local Ollama. Previously, all embedding calls went through `OllamaAgent::embed_text` which only talks to `127.0.0.1:11434` — when the brain mode was cloud, embeddings were skipped entirely and RAG degraded to keyword-only retrieval.

**Architecture.** New unified `embed_for_mode(text, brain_mode, active_brain)` dispatcher:
- `LocalOllama` → delegates to existing `OllamaAgent::embed_text`
- `PaidApi` → calls provider's `/v1/embeddings` with default model (e.g. `text-embedding-3-small` for OpenAI)
- `FreeApi` → calls free provider's embed endpoint where supported (Mistral, GitHub Models, SiliconFlow, NVIDIA NIM); returns `None` for providers without embed API (Pollinations, Groq, Cerebras)
- `None` → legacy fallback to Ollama

**Files changed.**

| File | What |
|------|------|
| `src-tauri/src/brain/cloud_embeddings.rs` (NEW) | `embed_text_openai` (OpenAI-compat `/v1/embeddings` caller), `embed_for_mode` (unified dispatcher), unsupported-provider cache, default model mappings. 8 unit tests. |
| `src-tauri/src/brain/mod.rs` | `pub mod cloud_embeddings; pub use cloud_embeddings::embed_for_mode;` |
| `src-tauri/src/commands/memory.rs` | `embed()` helper + all 8 embed call sites switched from `OllamaAgent::embed_text` to `embed_for_mode`. |
| `src-tauri/src/commands/streaming.rs` | `stream_openai_api` now calls `embed_for_mode` for vector RAG (was `None` before). |
| `src-tauri/src/commands/ingest.rs` | Ingest pipeline embed loop switched to `embed_for_mode`. |
| `src-tauri/src/commands/brain.rs` | `set_brain_mode` + `reset_embed_cache` now also clear cloud embed cache. |

**Test count.** 1039 Rust (+8), 1083 Vitest (unchanged). Clippy + vue-tsc clean.

---

## Chunk 17.2 — Contradiction resolution (LLM picks winner)

**Date.** 2026-04-26

**What.** When `add_memory` detects a near-duplicate (cosine ≥ 0.85) whose content semantically contradicts the new entry, an LLM "do these contradict?" check runs and opens a `MemoryConflict` row. The user can resolve (pick winner → loser soft-closed via `valid_to`) or dismiss the conflict. Maps to §16 Phase 5 of `brain-advanced-design.md`.

**Schema.** V9 migration adds `valid_to INTEGER` to `memories` and creates `memory_conflicts` table (id, entry_a_id, entry_b_id, status, winner_id, created_at, resolved_at, reason).

**Files changed.**

| File | What |
|------|------|
| `src-tauri/src/memory/conflicts.rs` (NEW) | `ConflictStatus` enum, `MemoryConflict` struct, `ContradictionResult` struct, `build_contradiction_prompt`, `parse_contradiction_reply`, `strip_fences`, `MemoryStore` impl: `add_conflict`, `list_conflicts`, `resolve_conflict`, `dismiss_conflict`, `count_open_conflicts`. 12 unit tests. |
| `src-tauri/src/memory/migrations.rs` | V9 migration (up + down), `TARGET_VERSION` → 9, sentinel test updated. |
| `src-tauri/src/memory/store.rs` | `valid_to: Option<i64>` on `MemoryEntry`, `close_memory(id, valid_to_ms)` method, all SELECT queries updated to include `valid_to`. |
| `src-tauri/src/memory/mod.rs` | `pub mod conflicts;` |
| `src-tauri/src/brain/ollama_agent.rs` | `check_contradiction(content_a, content_b)` method. |
| `src-tauri/src/commands/memory.rs` | `add_memory` wired with contradiction detection (lock-safe), 4 new commands: `list_memory_conflicts`, `resolve_memory_conflict`, `dismiss_memory_conflict`, `count_memory_conflicts`. |
| `src-tauri/src/lib.rs` | Registered 4 new conflict commands. |
| `src-tauri/src/memory/code_rag.rs` | `valid_to: None` in `MemoryEntry` constructors. |
| `src-tauri/src/memory/obsidian_export.rs` | `valid_to: None` in test helper. |
| `src-tauri/src/memory/reranker.rs` | `valid_to: None` in test helper. |
| `src-tauri/src/memory/cassandra.rs` | `valid_to: None` in `row_to_entry` + `add`. |
| `src-tauri/src/memory/mssql.rs` | `valid_to: None` in `row_to_entry`. |
| `src-tauri/src/memory/postgres.rs` | `valid_to: None` in `row_to_entry`. |

**Test count.** 1031 Rust (was 1019), 1083 Vitest (unchanged). Clippy + vue-tsc clean.

---

## Chunk 16.11 — Semantic chunking pipeline

**Date.** 2026-04-26
**Phase.** 16 (Modern RAG). Maps to `docs/brain-advanced-design.md` §16 Phase 4.

**Goal.** Replace the naive word-count splitter in `commands::ingest` with
semantic-boundary-aware chunking via the `text-splitter` crate (MIT, 1.2M+
downloads). Markdown documents are split at heading / paragraph / sentence
boundaries; plain text uses Unicode sentence boundary detection.
Deduplication by SHA-256 hash and Markdown heading metadata propagation.

**Architecture.**
- New `memory::chunking` module with `split_markdown()`, `split_text()`,
  `dedup_chunks()`, `Chunk` struct (index, text, hash, heading).
- `MarkdownSplitter` (from `text-splitter` crate with `markdown` feature)
  for `.md` files and HTML-sourced content; `TextSplitter` for everything
  else.
- Default chunk capacity: 1024 chars (≈256 tokens at ~4 chars/token).
- Heading metadata propagated as `section:<slug>` tags on each chunk.
- Old `chunk_text()` function kept (dead code) for resume-from-checkpoint
  path.

**Files created.**
- `src-tauri/src/memory/chunking.rs` — new module (~165 LOC)

**Files modified.**
- `src-tauri/Cargo.toml` — added `text-splitter = { version = "0.30", features = ["markdown"] }`
- `src-tauri/src/memory/mod.rs` — registered `pub mod chunking`
- `src-tauri/src/commands/ingest.rs` — replaced `chunk_text(&text, 800, 100)` with
  `split_markdown` / `split_text` + `dedup_chunks`; heading metadata propagated as
  `section:*` tags; old `chunk_text` marked `#[allow(dead_code)]` with deprecation note

**Tests.** 8 new Rust unit tests in `memory::chunking::tests`:
short_text_single_chunk, long_text_produces_multiple_chunks,
markdown_heading_extraction, markdown_splits_at_heading_boundaries,
dedup_removes_duplicates, sha256_hex_deterministic,
empty_text_produces_no_chunks, min_chunk_chars_enforced.

**Totals.** 1005 Rust tests, 1083 Vitest, clippy clean.

---

## Chunk 14.8 — Persona drift detection

**Date.** 2026-04-26
**Phase.** 14 (Persona self-learning). Maps to `docs/persona-design.md` § 15.1 row 143.

**Goal.** Periodically compare the user's active `PersonaTraits` against their
accumulated `personal:*` memories. When the auto-learn loop has extracted 25+
new facts since the last drift check, fire a lightweight LLM comparison prompt.
If drift is detected, surface a `DriftReport` with a summary and suggested
changes so the frontend can show "Echo noticed you've shifted toward …".

**Architecture.**
- `persona::drift` module (`drift.rs`) — pure prompt construction + reply
  parsing, 14 unit tests. `DriftReport` struct with `drift_detected`,
  `summary`, and `suggested_changes` (field/current/proposed triples).
- `OllamaAgent::check_persona_drift()` — sends the drift prompt to the LLM.
- `check_persona_drift` Tauri command — reads persona from disk, filters
  `personal:*` long-tier memories, calls brain, returns `DriftReport`.
- Frontend wiring in `conversation.ts` — `factsSinceDriftCheck` counter
  accumulates after each `extract_memories_from_session`; at threshold 25,
  fires `check_persona_drift` and exposes `lastDriftReport` for UI.

**Files created.**
- `src-tauri/src/persona/drift.rs` — ~280 LOC, 14 unit tests

**Files modified.**
- `src-tauri/src/persona/mod.rs` — added `pub mod drift`
- `src-tauri/src/brain/ollama_agent.rs` — added `check_persona_drift` method
- `src-tauri/src/commands/persona.rs` — added `check_persona_drift` Tauri command
- `src-tauri/src/lib.rs` — registered import + handler invocation
- `src/stores/persona-types.ts` — added `DriftReport` + `DriftSuggestion` types
- `src/stores/conversation.ts` — drift state refs + `maybeAutoLearn` integration

**Tests.** 14 new Rust unit tests in `persona::drift::tests`:
build_drift_prompt_includes_persona_and_memories, build_drift_prompt_empty_memories,
build_drift_prompt_respects_char_budget, parse_drift_reply_clean_json,
parse_drift_reply_no_drift, parse_drift_reply_with_fences,
parse_drift_reply_with_leading_prose, parse_drift_reply_missing_optional_fields,
parse_drift_reply_garbage_returns_none, parse_drift_reply_missing_drift_detected_returns_none,
strip_fences_removes_json_fences, strip_fences_removes_plain_fences,
strip_fences_passthrough_no_fences, drift_report_serde_round_trip.

**Totals.** 1019 Rust tests, 1083 Vitest, clippy clean, vue-tsc clean.

---

## Chunk 17.4 — Memory importance auto-adjustment

**Date.** 2026-04-26
**Phase.** 17 (Brain Phase-5 Intelligence). Maps to `docs/brain-advanced-design.md` §16 Phase 5.

**Goal.** Periodic job that nudges memory `importance` based on access
patterns: hot entries (access_count ≥ 10) gain +1 (capped at 5); cold
entries (access_count = 0 for 30+ days) lose −1 (floored at 1). Each
adjustment is audited via the `memory_versions` table (V8 schema from
chunk 16.12). Access counts are reset after boosting to prevent re-boost.

**Architecture.**
- `MemoryStore::adjust_importance_by_access(hot_threshold, cold_days)`
  method on `store.rs`. Pure SQL + version audit trail.
- `adjust_memory_importance` Tauri command (wraps store method with defaults
  hot=10, cold=30). Returns `{ boosted, demoted }`.
- `adjustImportance()` action on `src/stores/memory.ts`.

**Files modified.**
- `src-tauri/src/memory/store.rs` — new `adjust_importance_by_access` method (~80 LOC) + 8 tests
- `src-tauri/src/commands/memory.rs` — new `adjust_memory_importance` Tauri command + `ImportanceAdjustResult` struct
- `src-tauri/src/lib.rs` — registered import + handler invocation
- `src/stores/memory.ts` — new `adjustImportance()` action + exposed in store return

**Tests.** 8 new Rust unit tests in `memory::store::tests`:
adjust_boosts_hot_entries, adjust_caps_at_5, adjust_demotes_cold_entries,
adjust_floors_at_1, adjust_resets_access_count_after_boost,
adjust_leaves_middling_entries_alone, adjust_mixed_hot_and_cold,
adjust_creates_version_audit_trail.

**Totals.** 1005 Rust tests, 1083 Vitest, clippy clean.

---

## Chunk 16.12 — Memory versioning (V8 schema)

**Date.** 2026-04-25
**Phase.** 16 (Modern RAG). Maps to `docs/brain-advanced-design.md` §16 Phase 4.

**Goal.** Track edits to memory entries as immutable version snapshots so
`update_memory` no longer destroys history. New `memory_versions` V8 SQLite
table + `get_memory_history` Tauri command.

**Files created.**
- `src-tauri/src/memory/versioning.rs` — `save_version(conn, memory_id)`,
  `get_history(conn, memory_id)`, `version_count(conn, memory_id)`.
  `MemoryVersion` struct with `id`, `memory_id`, `version_num`, `content`,
  `tags`, `importance`, `memory_type`, `created_at`.

**Files modified.**
- `src-tauri/src/memory/migrations.rs` — V8 migration: `CREATE TABLE memory_versions` (FK cascade, `UNIQUE(memory_id, version_num)`, index on `memory_id`). Sentinel test updated to V8.
- `src-tauri/src/memory/mod.rs` — added `pub mod versioning`.
- `src-tauri/src/memory/store.rs` — `update()` now calls `versioning::save_version()` before applying changes (best-effort; silent fallback on pre-V8 schema).
- `src-tauri/src/commands/memory.rs` — added `get_memory_history` command.
- `src-tauri/src/lib.rs` — registered `get_memory_history` in imports + handler list.
- `src/stores/memory.ts` — added `getMemoryHistory(memoryId)` action.
- `rules/milestones.md` — removed 16.12 row, updated Next Chunk.
- `docs/brain-advanced-design.md` — marked Memory versioning ✓, updated §16 tree.
- `README.md` — added `versioning.rs` module listing, `get_memory_history` command, updated V7→V8 references, test count 989.

**Test counts.** 7 new Rust tests (versioning module) + 12 migration tests pass.

---

## Chunk 16.2 — Contextual Retrieval (Anthropic 2024)

**Date.** 2026-04-25
**Phase.** 16 (Modern RAG). Maps to `docs/brain-advanced-design.md` §19.2 row 3.

**Goal.** At ingest time, LLM prepends a 50–100 token document-level context
to each chunk *before* embedding. Opt-in via `AppSettings.contextual_retrieval`.
Anthropic reports ~49 % reduction in failed retrievals.

**Files created.**
- `src-tauri/src/memory/contextualize.rs` — `generate_doc_summary(text, brain_mode)`,
  `contextualise_chunk(doc_summary, chunk, brain_mode)`, `prepend_context(ctx, chunk)`.
  Brain-mode agnostic (dispatches to Ollama / FreeApi / PaidApi via `call_llm` helper).

**Files modified.**
- `src-tauri/src/memory/mod.rs` — added `pub mod contextualize`.
- `src-tauri/src/settings/mod.rs` — added `contextual_retrieval: bool` to `AppSettings` (default `false`, `#[serde(default)]`).
- `src-tauri/src/commands/ingest.rs` — `run_ingest_task` now reads `contextual_retrieval` from settings; generates a doc summary once; prepends context to each chunk.
- `src-tauri/src/settings/config_store.rs` — added `contextual_retrieval` to 3 test struct literals.
- `src-tauri/src/commands/settings.rs` — added `contextual_retrieval` to 2 test struct literals.
- `src/stores/settings.ts` — added `contextual_retrieval` field + default.
- `src/views/BrainView.test.ts` — added `contextual_retrieval: false` to mock.
- `rules/milestones.md` — removed 16.2 row, updated Next Chunk.
- `docs/brain-advanced-design.md` — flipped §19.2 row 3 from 🔵 to ✅, updated §16 tree.
- `README.md` — added `contextualize.rs` module listing.

**Test counts.** 6 new Rust tests (contextualize module) + all settings tests green.

---

## Chunk 17.3 — Temporal reasoning queries

**Date.** 2026-04-25
**Phase.** 17 (Brain Phase-5 Intelligence). Maps to `docs/brain-advanced-design.md` §16 Phase 5.
**Goal.** Extend `commands::memory` with `temporal_query(question)` that
parses natural-language time expressions and returns memories whose
`created_at` falls within the resolved range.

**Architecture.**
- New `src-tauri/src/memory/temporal.rs` module (~300 LOC):
  - `TimeRange { start_ms, end_ms }` — resolved interval in Unix ms.
  - `parse_time_range(question, now_ms) -> Option<TimeRange>` — parses:
    `last N days/weeks/months/hours`, `last day/week/month/year`,
    `today`, `yesterday`, `since YYYY-MM-DD`, `since <month-name>`,
    `before YYYY-MM-DD`, `between YYYY-MM-DD and YYYY-MM-DD`.
  - Pure-std calendar helpers: `ymd_to_ms`, `ms_to_ymd` (Howard Hinnant
    civil-from-days algorithm), `midnight_utc`, `strip_punct`.
  - No external crate — all date math is pure `std::time`.
- New Tauri command `temporal_query(question)`:
  - Parses time range from question.
  - Filters `get_all()` by `created_at ∈ [start_ms, end_ms)`.
  - Falls back to keyword `search()` when no time expression detected.
  - Returns `TemporalQueryResult { time_range, memories }`.
- 20 unit tests (calendar roundtrips, all parse patterns, edge cases).

**Files created.**
- `src-tauri/src/memory/temporal.rs` — **new** (20 tests)

**Files modified.**
- `src-tauri/src/memory/mod.rs` — added `pub mod temporal`
- `src-tauri/src/commands/memory.rs` — added `temporal_query` command + `TemporalQueryResult`
- `src-tauri/src/lib.rs` — registered `temporal_query` in import + handler

**Test counts.** Backend: 976 cargo tests; Frontend: 1083 Vitest tests.

---

## Chunk 18.5 — Obsidian vault export (one-way)

**Date.** 2026-04-25
**Phase.** 18 (Categorisation & Taxonomy — final chunk). Maps to §16 Phase 2 + Phase 4.
**Goal.** New Tauri command `export_to_obsidian(vault_dir)` that writes one
Markdown file per long-tier memory under `<vault_dir>/TerranSoul/<id>-<slug>.md`
with YAML frontmatter. Idempotent: file mtime drives "should I rewrite?"
decision. Completes Phase 18.

**Architecture.**
- New `src-tauri/src/memory/obsidian_export.rs` module (~280 LOC):
  - `slugify(content)` — filesystem-safe slug (≤60 bytes).
  - `filename_for(entry)` — `<id>-<slug>.md`.
  - `format_iso(ms)` — pure Unix-ms → ISO 8601 UTC (Howard Hinnant).
  - `render_markdown(entry)` — YAML frontmatter (id, created_at,
    importance, memory_type, tier, tags as list, source_url, source_hash)
    + body.
  - `export_to_vault(vault_dir, entries) -> ExportReport` — creates
    `TerranSoul/` dir, writes only long-tier entries, skips unchanged
    files (mtime >= memory's `last_accessed`).
- New Tauri command `export_to_obsidian(vault_dir)`.
- Frontend: `MemoryView.vue` gains "📓 Export to Obsidian" button +
  modal with vault-path input and result feedback.
- `memory.ts` Pinia store: `exportToObsidian(vaultDir)` action.
- 14 Rust unit tests (slugify, filename, ISO, frontmatter, export
  idempotency, tier filtering).

**Files created.**
- `src-tauri/src/memory/obsidian_export.rs` — **new** (14 tests)

**Files modified.**
- `src-tauri/src/memory/mod.rs` — added `pub mod obsidian_export`
- `src-tauri/src/commands/memory.rs` — added `export_to_obsidian` command
- `src-tauri/src/lib.rs` — registered `export_to_obsidian` in import + handler
- `src/stores/memory.ts` — added `exportToObsidian` action
- `src/views/MemoryView.vue` — added export button + modal + handler

**Test counts.** Backend: 976 cargo tests; Frontend: 1083 Vitest tests.

---

## Chunk 18.3 — Category filters in Memory View

**Date.** 2026-04-24
**Phase.** 18 (Categorisation & Taxonomy). Builds on 18.4 (tag vocabulary) and 18.1 (auto-tag).
**Goal.** Add a tag-prefix multi-select chip row to Memory View so users
can filter memories by curated category (`personal`, `domain`, `project`,
`tool`, `code`, `external`, `session`, `quest`).

**Architecture.**
- `MemoryView.vue` gains a `tagPrefixCounts` computed that scans all displayed
  memories and counts occurrences per curated prefix. A `tagPrefixFilter` ref
  toggles prefix filtering that composes with the existing type/tier/search
  filters.
- Chips show count badges; disabled when count = 0; active = purple accent.
- New `MemoryView.test.ts` (10 tests) exercises the tag-prefix counting and
  filtering logic as pure functions.

**Files modified.**
- `src/views/MemoryView.vue` — added tag-prefix filter row + CSS
- `src/views/MemoryView.test.ts` — **new** (10 tests)

**Test counts.** Frontend: 1083 Vitest tests (67 files); Backend: 943 cargo tests.

---

## Chunk 18.1 — Auto-categorise via LLM on insert

**Date.** 2026-04-24
**Phase.** 18 (Categorisation & Taxonomy). Uses 18.4 tag-prefix vocabulary for validation.
**Goal.** When `AppSettings.auto_tag = true` (default off), every
`add_memory` call runs a fast LLM pass that classifies the content into
≤ 4 tags drawn from the curated prefix vocabulary and merges them with
user-supplied tags.

**Architecture.**
- New `src-tauri/src/memory/auto_tag.rs` module (~140 LOC):
  - `system_prompt()` / `user_prompt()` — prompt builders
  - `parse_tag_response()` — parses LLM comma-separated tag response,
    validates each against `validate_csv()`, keeps only `Curated` verdicts,
    caps at 4 tags
  - `merge_tags()` — deduplicates auto-tags against user tags (case-insensitive)
  - `auto_tag_content()` — dispatches to Ollama / FreeApi / PaidApi based on
    active `BrainMode`
- `AppSettings.auto_tag: bool` (default `false`) persisted to disk
- `commands::memory::add_memory` — after insert + embedding, checks
  `auto_tag` setting and brain_mode; if both present, runs auto-tagger and
  updates the entry's tags via `store.update()`
- `OllamaAgent::call()` promoted to `pub(crate)` for internal use
- Frontend: `AppSettings` interface gains `auto_tag?: boolean`; BrainView
  gains an "Auto-Tag" toggle section with checkbox + description

**Files created.**
- `src-tauri/src/memory/auto_tag.rs` (10 unit tests)

**Files modified.**
- `src-tauri/src/memory/mod.rs` — added `pub mod auto_tag`
- `src-tauri/src/brain/ollama_agent.rs` — `call()` → `pub(crate)`
- `src-tauri/src/settings/mod.rs` — added `auto_tag` field to `AppSettings`
- `src-tauri/src/settings/config_store.rs` — updated test initializers
- `src-tauri/src/commands/memory.rs` — auto-tag logic in `add_memory`
- `src-tauri/src/commands/settings.rs` — updated test initializers
- `src/stores/settings.ts` — added `auto_tag` to `AppSettings` interface
- `src/views/BrainView.vue` — auto-tag toggle UI section
- `src/views/BrainView.test.ts` — added `get_app_settings` to mock

**Test counts.** Backend: 943 cargo tests (10 new in auto_tag); Frontend: 1083 Vitest.

---

## CI Fix — Embed cache test race condition

**Date.** 2026-04-24
**Goal.** Fix flaky `clear_embed_caches_forgets_unsupported_models` test
that failed in CI due to parallel test interference on shared global
embed cache statics.

**Root cause.** Five `#[tokio::test]` tests in `ollama_agent::tests` share
process-global `OnceLock<Mutex<…>>` statics for the embed model cache
and unsupported-models set. Running in parallel, one test's
`clear_embed_caches()` call could race against another test's
`mark_unsupported()` + `assert!()` sequence.

**Fix.** Added `EMBED_TEST_LOCK: tokio::sync::Mutex<()>` static — all
five cache tests acquire the lock before running, serialising access to
the shared statics. Also added an initial `clear_embed_caches()` to the
`clear_embed_caches_forgets_unsupported_models` test for a clean baseline.

**Files modified.**
- `src-tauri/src/brain/ollama_agent.rs` — added `EMBED_TEST_LOCK` + guard
  acquisition in 5 tests

---

## Chunk 18.2 — Category-aware decay rates

**Date.** 2026-04-24
**Phase.** 18 (Categorisation & Taxonomy). Composes directly on top of 18.4 (tag-prefix vocabulary).
**Goal.** Stop decaying every long-term memory at the same uniform rate. A `personal:*` fact about the user (precious) should outlive a `tool:*` flag (rots quarterly when product UI changes).

**Architecture.**
- New `memory::tag_vocabulary::category_decay_multiplier(tags_csv: &str) -> f64`. Pure — no I/O. Returns the **lowest** (slowest-decaying) multiplier among all curated prefixes present on the entry; legacy / non-conforming tags collapse to the baseline `1.0`.
- Per-prefix multipliers (calibrated against §16 Phase 2 design intent):
  - `personal` → **0.5** (2× slower — precious)
  - `domain`, `code` → **0.7** (~1.4× slower — reference material)
  - `project`, `external` → **1.0** (baseline)
  - `tool` → **1.5** (1.5× faster — flags / UI change)
  - `session`, `quest` → **2.0** (2× faster — short-lived)
- "Slowest wins" rule: a single `personal:*` tag protects a row even if it also carries `tool:*` — matches the design principle that downgrading a precious memory is the costliest mistake.
- `MemoryStore::apply_decay` SELECT extended with `tags`; computes `0.95 ^ ((hours_since / 168) * multiplier)` instead of the previous prefix-blind formula. Clamp to `>= 0.01` and the `> 0.001` change-threshold are unchanged so the call remains idempotent on already-decayed-flat rows.

**Files modified.**
- `src-tauri/src/memory/tag_vocabulary.rs` — added `category_decay_multiplier` + 4 unit tests.
- `src-tauri/src/memory/store.rs` — `apply_decay` now passes the entry's `tags` through `category_decay_multiplier`; added 2 integration tests.

**Tests.** 6 new tests, all passing alongside 930 existing tests (total **936 passing**):
1. `decay_multiplier_baseline_for_no_curated_tags` — empty / legacy / non-conforming → 1.0.
2. `decay_multiplier_per_prefix` — every curated prefix returns its expected multiplier.
3. `decay_multiplier_picks_slowest_when_multiple_prefixes` — `personal` (0.5) beats `tool` (1.5); `domain` (0.7) beats `project` (1.0); `session` (2.0) loses to `project` (1.0).
4. `decay_multiplier_ignores_legacy_and_non_conforming_when_curated_present` — `fact` + `personal:*` + `randomtag` → 0.5.
5. `apply_decay_personal_decays_slower_than_tool` — store integration: forced `last_accessed = -30 days`, `personal:*` row ends up with strictly higher `decay_score` than `tool:*` row after one `apply_decay()`.
6. `apply_decay_baseline_for_legacy_or_non_conforming_tags` — `fact` (legacy) and `project:*` (curated 1.0) decay identically (within float tolerance).

**Validation.** `cargo test --lib` (936 pass / 0 fail) + `cargo clippy --lib --tests -- -D warnings` (clean).

**Follow-ups (not in this chunk).**
- BrainView: per-user multiplier tuning UI (the chunk description mentions this; deferred — defaults are calibrated and shipping the multiplier engine first lets later UI just edit them).
- 18.3 (Memory View filter chips) — frontend chunk that surfaces the same prefix taxonomy.

---

## Chunk 18.4 — Tag-prefix convention vocabulary + audit

**Date.** 2026-04-24
**Phase.** 18 (Categorisation & Taxonomy) — first chunk; pure-Rust foundation that 18.1 (auto-categorise), 18.2 (category-aware decay), and 18.3 (Memory View filters) all consume.
**Goal.** Make the long-implicit `<prefix>:<value>` tag convention explicit and auditable, without breaking the write path. Existing free-form tags continue to work; non-conforming tags surface as a soft "review tag" warning in BrainView instead of being rejected.

**Architecture.**
- New `src-tauri/src/memory/tag_vocabulary.rs` (~230 LOC + 10 unit tests). Pure — no I/O.
- `CURATED_PREFIXES: &[&str]` lists the 8 sanctioned prefixes (`personal`, `domain`, `project`, `tool`, `code`, `external`, `session`, `quest`) with a docblock describing each one's intent. Adding a new prefix is a small design decision documented in the source.
- `LEGACY_ALLOW_LIST: &[&str]` covers the seed-fixture / pre-convention tags (`user`, `assistant`, `system`, `fact`, `preference`, `todo`, `summary`) — short by design, every entry is debt to be migrated.
- `validate(tag: &str) -> TagValidation` returns one of:
  - `Curated { prefix }` — canonical-cased prefix from `CURATED_PREFIXES` (so callers can pattern-match safely against `&'static str`).
  - `Legacy` — case-insensitive whole-tag match against the allow-list.
  - `NonConforming { reason: NonConformingReason }` — `UnknownPrefix(String)`, `MissingPrefix`, `EmptyValue { prefix }`, or `Empty`.
- `validate_csv(tags_csv: &str) -> Vec<TagValidation>` matches the on-disk shape stored in `MemoryEntry.tags` (comma-separated). Empty entries from a trailing comma are dropped.
- Case-insensitive prefix matching (`Personal:Foo` and `personal:foo` both validate as `Curated { prefix: "personal" }`).
- Values are not interpreted — `personal:🍕` and `external:https://foo.bar:8080/x` both pass cleanly because `split_once(':')` only splits on the first colon.
- New Tauri command `audit_memory_tags` in `commands/memory.rs` — walks every memory, returns only the rows with at least one non-conforming tag, paired with a human-readable reason. Read-only; ingest still accepts everything.
- New types `MemoryTagAudit { memory_id, flagged: Vec<TagAuditFlag> }` and `TagAuditFlag { tag, reason }` for the BrainView surface.

**Files modified / created.**
- `src-tauri/src/memory/tag_vocabulary.rs` (new, 230 LOC + 10 tests).
- `src-tauri/src/memory/mod.rs` — added `pub mod tag_vocabulary;`.
- `src-tauri/src/commands/memory.rs` — added `audit_memory_tags` Tauri command + `MemoryTagAudit` / `TagAuditFlag` serde types.
- `src-tauri/src/lib.rs` — wired into `commands::memory::*` import + invoke handler list.
- `rules/milestones.md` — Phase 18 row 18.4 removed.

**Tests.** 10 new unit tests, all passing alongside 920 existing tests (total **930 passing**):
1. `curated_prefixes_validate` — happy path for several prefixes.
2. `case_insensitive_prefix_match` — `Personal:Foo` and `DOMAIN:law` accepted.
3. `legacy_allow_list_passes` — case-insensitive whole-tag match.
4. `unknown_prefix_is_non_conforming` — `color:blue` flagged with `UnknownPrefix("color")`.
5. `no_separator_and_not_in_allow_list_is_non_conforming` — `randomtag` flagged with `MissingPrefix`.
6. `empty_value_is_non_conforming` — `personal:` and `personal:   ` both flagged with `EmptyValue`.
7. `empty_or_whitespace_tag_is_non_conforming` — `""` and `"   "` flagged with `Empty`.
8. `validate_csv_parses_each_tag_in_order` — 5-tag CSV with one empty entry collapses to 4 results in input order.
9. `is_acceptable_only_curated_or_legacy` — convenience predicate.
10. `value_can_contain_colons_and_unicode` — URL-as-value and emoji-as-value edge cases.

**Validation.** `cargo test --lib` (930 pass / 0 fail) + `cargo clippy --lib --tests -- -D warnings` (clean).

**Follow-ups (not in this chunk).**
- 18.1 (auto-categorise via LLM) — will write tags using `CURATED_PREFIXES` as the LLM's allowed-prefix prompt.
- 18.2 (category-aware decay) — will look up per-prefix multipliers keyed off `Curated { prefix }`.
- 18.3 (Memory View filter chips) — frontend chunk that calls `audit_memory_tags` for the warning badge + filters by prefix.
- BrainView "review tags" warning panel that consumes `audit_memory_tags`.

---

## Chunk 17.1 — Auto-promotion based on access patterns

**Date.** 2026-04-24
**Phase.** 17 (Brain Phase-5 Intelligence) — first chunk; pure-Rust foundation that the rest of Phase 17 composes onto.
**Goal.** Stop forcing the user to manually promote frequently-revisited working-tier memories. When a working-tier entry is accessed often enough recently, it earns long-tier status automatically — and the heuristic is honest enough to be a no-op on stale or never-touched rows.

**Architecture.**
- New `MemoryStore::auto_promote_to_long(min_access_count: i64, window_days: i64) -> SqlResult<Vec<i64>>`. Pure SQL — selects every `tier = 'working'` row where `access_count >= min_access_count` AND `last_accessed IS NOT NULL` AND `last_accessed >= now - window_days * 86_400_000`, then `UPDATE`s their tier to `'long'`. Returns the IDs that were promoted in ascending order so callers (BrainView, future workflow jobs) can audit / display them.
- The `last_accessed IS NOT NULL` guard is load-bearing: a working entry that was inserted but never accessed has `last_accessed = NULL` even if its `access_count` happens to be high (e.g. set by a backfill job). Treating NULL as "not recent" prevents accidental promotion of cold rows.
- Defensive math: `window_days <= 0` collapses to "no recency requirement" (cutoff = 0), and `min_access_count` is floored at 0, so callers can't trip arithmetic underflow.
- Idempotent by construction — a second call only sees `tier = 'working'` rows, so already-promoted entries stay put.
- Stays off the `StorageBackend` trait (mirrors `apply_decay`'s scope) — this is a SQLite-only concern; Postgres / MSSQL / Cassandra backends ignore it. Avoids touching three backend impls for a feature the alternative backends don't need.
- New `commands::memory::auto_promote_memories(min_access_count: Option<i64>, window_days: Option<i64>)` Tauri command with sensible defaults (5, 7). Registered in `lib.rs` invoke-handler list.

**Files modified.**
- `src-tauri/src/memory/store.rs` — new method + 6 new unit tests.
- `src-tauri/src/commands/memory.rs` — new Tauri command.
- `src-tauri/src/lib.rs` — wired into invoke handler + command imports.
- `docs/brain-advanced-design.md` § 16 Phase 5 — flipped row from `○` to `✓` with module + command pointers.
- `rules/milestones.md` — Phase 17 row 17.1 removed (per the "completed chunks belong in completion-log only" rule).

**Tests.** 6 new unit tests in `memory::store::tests`, all passing alongside 914 existing tests (total **920 passing**):
1. `auto_promote_promotes_when_both_thresholds_met` — happy path.
2. `auto_promote_skips_when_access_count_below_threshold` — boundary: 4 vs threshold 5 stays working.
3. `auto_promote_skips_when_outside_recency_window` — 30-day-old access doesn't promote at 7-day window.
4. `auto_promote_ignores_long_and_short_tiers` — only working-tier is considered (idempotency-by-tier).
5. `auto_promote_is_idempotent` — second call after a successful promotion is a no-op.
6. `auto_promote_skips_rows_with_null_last_accessed` — the load-bearing NULL-guard invariant.

**Validation.** `cargo test --lib` (920 pass / 0 fail) + `cargo clippy --lib --tests -- -D warnings` (clean).

**Follow-ups (not in this chunk).**
- Frontend: surface the promoted IDs in BrainView's "Active selection" panel so the user can see what just got promoted (deferred — pure Rust surface is in place).
- Schedule: today the command is invoke-on-demand (frontend or background job's choice). Once the workflow engine grows a periodic-job slot (post-17.5), schedule this daily alongside `apply_memory_decay`.

---

## Chunk 15.3 — `BrainGateway` trait + shared op surface

**Date.** 2026-04-24
**Goal.** Define a single typed op surface (`BrainGateway`) that every transport (MCP, gRPC) routes through, so the eight ops in `docs/AI-coding-integrations.md § Surface` (`brain.search`, `get_entry`, `list_recent`, `kg_neighbors`, `summarize`, `suggest_context`, `ingest_url`, `health`) cannot drift between transports.

**Architecture.**
- `src-tauri/src/ai_integrations/mod.rs` — module root + re-exports.
- `src-tauri/src/ai_integrations/gateway.rs` — typed request/response structs, `GatewayCaps`, `GatewayError` (`thiserror`), `BrainGateway` async trait, `IngestSink` trait, `AppStateGateway` adapter.
- The adapter delegates straight to `MemoryStore` (for `search`, `get_entry`, `list_recent`, `kg_neighbors`), `OllamaAgent::summarize_conversation` / `embed_text` / `hyde_complete` (for `summarize`, HyDE search), and `IngestSink::start_ingest` (for `ingest_url`). **No new business logic** — the gateway is pure composition over existing `commands::memory` / `brain` surfaces.
- `IngestSink` trait keeps the gateway free of any Tauri `AppHandle` dependency, so it remains unit-testable without a real Tauri runtime. Production constructs an `AppHandleIngestSink` in the transport layer (15.1 / 15.2) that wraps the existing `commands::ingest::ingest_document` flow.
- **Capability gates** — every op takes `&GatewayCaps`. Reads require `brain_read`; writes require `brain_write`. `Default` is read-only. Convenience constants `GatewayCaps::NONE` and `GatewayCaps::READ_WRITE` for tests.
- **Delta-stable `suggest_context`** — composes search (HyDE when a brain is configured, RRF otherwise) → KG one-hop around top hit → LLM summary. Returns a `SuggestContextPack { hits, kg, summary, fingerprint }` where `fingerprint` is a SHA-256 hex over the resolved hit ids + the active brain identifier. Identical inputs ⇒ identical fingerprints — the contract VS Code Copilot caches against in Chunk 15.7.
- **Lock discipline** — `std::sync::Mutex` locks on `AppState` are scoped tightly and dropped before any `.await`, matching the convention used by the existing Tauri commands.

**Files created.**
- `src-tauri/src/ai_integrations/mod.rs` (1 module + re-exports, 31 lines).
- `src-tauri/src/ai_integrations/gateway.rs` (1165 lines including 17 unit tests).

**Files modified.**
- `src-tauri/src/lib.rs` — added `pub mod ai_integrations;`.
- `docs/AI-coding-integrations.md` — flipped the Shared Surface section from "Planned" to "shipped 2026-04-24" with as-built specifics (trait shape, capability constants, error variants, IngestSink rationale, delta-stable fingerprint contract, test coverage).

**Tests.** 17 new unit tests in `gateway::tests`, all passing. Coverage:
1. `read_op_requires_brain_read_capability` — `search` rejects `GatewayCaps::NONE`.
2. `write_op_requires_brain_write_capability` — `ingest_url` rejects default caps even when sink attached.
3. `write_op_routes_through_sink_when_permitted` — call reaches `RecordingIngestSink` exactly once with the right args.
4. `write_op_without_sink_reports_not_configured` — `NotConfigured` error, no panic.
5. `search_rejects_empty_query` — `InvalidArgument`.
6. `search_returns_descending_positional_scores` — score ordering invariant.
7. `get_entry_returns_not_found_for_missing_id` — `NotFound` not `Storage`.
8. `list_recent_filters_by_kind_and_tag` — kind + tag filters work; `since` is permissive.
9. `kg_neighbors_reports_truncation_when_depth_above_one` — honest reporting, no silent capping.
10. `summarize_requires_text_or_memory_ids` — `InvalidArgument` when both empty.
11. `summarize_no_brain_returns_none_summary_with_resolution_count` — graceful degradation contract.
12. `suggest_context_is_delta_stable_for_identical_input` — same input ⇒ same fingerprint + same hit order.
13. `suggest_context_fingerprint_changes_when_brain_changes` — flipping `active_brain` invalidates the fingerprint.
14. `health_reports_provider_and_memory_total` — counts + provider id correct.
15. `fingerprint_is_deterministic_and_id_sensitive` — pure-function fingerprint contract.
16. `default_caps_are_read_only` — security default invariant.
17. `parse_memory_type_is_tolerant` — case-insensitive + permissive parser.

**Validation.** `cargo build --lib` succeeds; `cargo test --lib` runs 909 tests (all passing); `cargo clippy --lib --tests -- -D warnings` clean.

**Follow-ups (not in this chunk).**
- 15.1 (MCP transport) wires the adapter behind `127.0.0.1:7421` with bearer-token auth.
- 15.2 (gRPC transport) wires the adapter behind `127.0.0.1:7422` with mTLS.
- 15.4–15.8 build the Control Panel, voice intents, auto-setup writers, and the e2e Copilot harness on top.

---

## Milestones audit

**Date.** 2026-04-24
**Goal.** Surface every chunk that's described in `docs/` but not yet enumerated in `rules/milestones.md`, design coherent phases for each, and make them pickable by future agent sessions.

**Audit findings.** Three docs contained chunks not represented in milestones.md:

1. `docs/persona-design.md` § 15 — eight side-chain rows (143, 147, 149, 151, 152, 153, 154, 155) and one main-chain row (143 drift detection).
2. `docs/brain-advanced-design.md` § 16 Phase 6 + § 19.2 — eight 🔵 modern-RAG techniques (Contextual Retrieval, Late Chunking, GraphRAG/LightRAG, Self-RAG, CRAG, Sleep-time consolidation, Matryoshka, relevance threshold) plus four Phase-4 items (ANN index, cloud embeddings, chunking pipeline, memory versioning).
3. `docs/brain-advanced-design.md` § 16 Phase 5 + Phase 2 leftovers — auto-promotion, contradiction resolution, temporal reasoning, importance auto-adjustment, CRDT memory merge, conflict detection, Obsidian sync (bidirectional), auto-categorise on insert, category-aware decay, category filters, tag-prefix enforcement, Obsidian one-way export.

**Phases added to `rules/milestones.md`.**
- **Phase 14 expansion** — added rows 14.8 (persona drift detection), 14.9 (save/load learned expression presets), 14.10 (save/load learned motion clips + `LearnedMotionPlayer`), 14.11 (side-chain bundle export — persona pack envelope v2), 14.12 (phoneme-aware viseme model), 14.13 (Hunyuan-Motion offline polish, opt-in), 14.14 (MoMask reconstruction), 14.15 (MotionGPT brain capability).
- **Phase 16 — Modern RAG** (12 chunks): 16.1 relevance threshold, 16.2 contextual retrieval, 16.3 late chunking, 16.4 self-RAG, 16.5 CRAG, 16.6 GraphRAG community summaries, 16.7 sleep-time consolidation, 16.8 matryoshka embeddings, 16.9 cloud embedding API, 16.10 ANN index (`usearch`), 16.11 chunking pipeline, 16.12 memory versioning (V8 schema).
- **Phase 17 — Brain Phase-5 Intelligence** (7 chunks): 17.1 auto-promotion, 17.2 contradiction resolution + `MemoryConflict`, 17.3 temporal reasoning, 17.4 importance auto-adjustment, 17.5 CRDT memory merge via Soul Link, 17.6 connected-memory conflict detection, 17.7 bidirectional Obsidian sync.
- **Phase 18 — Categorisation & Taxonomy** (5 chunks): 18.1 auto-categorise on insert, 18.2 category-aware decay rates, 18.3 category filters in MemoryView, 18.4 tag-prefix enforcement lint, 18.5 Obsidian vault export (one-way).

**Files modified.**
- `rules/milestones.md` — `Next Chunk` summary refreshed; eight rows appended to Phase 14 table; three new phase sections (16 / 17 / 18) added.

**Cross-doc invariants preserved.**
- Each new chunk row carries an explicit "Maps to" pointer back to the originating doc section so the brain-doc-sync rule (architecture-rules.md rule 11) and persona-doc-sync rule (architecture-rules.md rule 12) keep working when chunks land.
- No chunk numbering collisions; all rows still match the phase-prefix `<phase>.<n>` convention.

---



**Date:** 2026-04-24
**Reference:** `docs/licensing-audit.md` (new); `rules/coding-standards.md` *"Use Existing Libraries First"*.

**Trigger.** User requirement: *"check to make sure all package or
integrations or libraries meet the commercial usage."*

**Findings.** Every other dependency is permissively licensed
(MIT / Apache-2.0 / BSD / ISC / MPL-2.0). Two integrations failed
the strict commercial-use bar and were removed:

- **`msedge-tts`** (Rust). Crate is MIT, but it calls Microsoft Edge's
  undocumented `speech.platform.bing.com` *"Read Aloud"* WebSocket
  endpoint. Microsoft directs commercial users to paid Azure
  Cognitive Services — TTS; the unofficial endpoint is a ToS-violation
  risk and historically rate-limited.
- **`@vercel/analytics` + `@vercel/speed-insights`** (npm). Libraries
  are MPL-2.0, but they phone home to Vercel servers without a
  user-visible privacy contract; Vercel's free Web Analytics tier is
  restricted to non-commercial projects; runtime telemetry from a
  desktop binary conflicts with TerranSoul's local-first privacy
  posture. `vue-router` was only included to satisfy these libraries'
  unconditional `useRoute()` calls and was removed too.

**Replacements.**

- **TTS:** new `web-speech` provider id (browser `SpeechSynthesis`
  API). The backend's `synthesize_tts` returns `Vec::new()` for
  `web-speech` and the existing `useTtsPlayback` composable already
  falls back to `speechSynthesis.speak()` when the WAV payload is
  empty (≤44 bytes). Free, offline-capable, no telemetry, no
  third-party ToS. Default `tts_provider` flips from `"edge-tts"` →
  `"web-speech"`. Optional cloud upgrade remains available via the
  user-supplied `openai-tts` provider with an explicit API key.
- **Analytics:** none. A privacy-first desktop app should not phone
  home for usage analytics.

**Files touched.**

- Deleted: `src-tauri/src/voice/edge_tts.rs`.
- Removed deps: `msedge-tts` from `src-tauri/Cargo.toml`;
  `@vercel/analytics`, `@vercel/speed-insights`, `vue-router` from
  `package.json`.
- Updated: `src-tauri/src/voice/mod.rs` (catalogue + default config +
  tests), `src-tauri/src/voice/config_store.rs` (test fixture),
  `src-tauri/src/commands/voice.rs` (`synthesize_tts` arm + tests),
  `src-tauri/src/commands/ipc_contract_tests.rs` (test fixture),
  `src/stores/voice.ts` (fallback provider + `autoConfigureVoice`),
  `src/stores/voice.test.ts` (id sweep), `src/views/VoiceSetupView.vue`
  (`activateBrowser`), `src/views/ChatView.vue` (`gift-of-speech`
  quest auto-config), `src/App.vue` (drop Analytics / SpeedInsights
  components + imports), `src/main.ts` (drop createRouter +
  vue-router import), `README.md` (Voice System section).
- Added: `docs/licensing-audit.md` capturing findings + process.

**Validation.** `cargo test --lib` 892 passing (was 901; the deleted
`edge_tts` module accounted for the delta, including its 6 routing
tests). `npm test -- --run` 1073 passing. `npm run lint` 0 errors.
`cargo clippy --lib --no-deps -- -D warnings` clean.

**Privacy / commercial implications.** TerranSoul builds can now be
distributed as part of a paid commercial product without requiring any
extra third-party licence purchase, without violating any upstream ToS,
and without any silent runtime telemetry to a third-party SaaS.

---

## Chunk 14.6 — Audio-Prosody Persona Hints (Camera-Free)

**Date:** 2026-04-24
**Reference:** `docs/persona-design.md` § 9.4 (new); `rules/milestones.md` Phase 14 row 14.6 (removed).

**Goal.** When the user has an ASR provider configured, derive
camera-free *prosody-style* hints (tone / pacing / quirks) from their
typed turns — which mirror their spoken patterns — and fold them into
the Master-Echo persona-extraction prompt so the suggested persona
better matches how the user actually talks.

**What shipped.**

- New module `src-tauri/src/persona/prosody.rs` (≈490 lines, 23 unit
  tests). Pure / I/O-free analyzer over user-role utterances →
  `ProsodyHints { tone, pacing, quirks }`. Signals: avg sentence
  length (concise / elaborate), exclamation density (energetic),
  question density (inquisitive), ALLCAPS ratio gated by ≥50 alpha
  letters (emphatic), filler-word density via whole-word matcher
  (quirk: `um`, `uh`, `like`, `literally`, `you know`, `i mean`,
  `kind of`, `sort of`, `actually`, `basically`, `er`, `hmm`), emoji
  density via Unicode-block check (playful + quirk). Tone capped at
  4, quirks at 3, both matching the persona-schema budget.
  `MIN_UTTERANCES = 3` and `MAX_INPUT_BYTES = 1 MiB` short-circuit
  thin or pathological corpora.
- `render_prosody_block(&hints) -> Option<String>` emits a single
  user-facing line and returns `None` for empty hints so the caller
  skips the section entirely (no dead cue for the LLM to hallucinate
  from).
- New extract overload
  `build_persona_prompt_with_hints(snippets, hints) -> (system, user)`.
  When `hints == None`, the output is **byte-identical** to the
  existing `build_persona_prompt`, so all prior tests stay green
  (verified by a new equivalence test).
- New agent surface
  `OllamaAgent::propose_persona_with_hints(snippets, hints)`. Legacy
  `propose_persona` delegates with `hints = None`.
- Wired into `commands/persona::extract_persona_from_brain`: only
  when `voice_config.asr_provider.is_some()` are user-role utterances
  filtered out of the conversation snapshot, fed through
  `analyze_user_utterances`, rendered, and passed to the agent.

**Privacy contract.**

- Raw audio is never read — by the time a turn reaches the message
  log, the audio is already gone.
- Hints are computed on demand at suggestion time and discarded once
  the LLM reply is parsed; no on-disk artefact is ever produced.
- Hints are deliberately coarse (single-word adjectives + at most
  three quirk strings); they read as friendly tone guidance rather
  than a forensic profile.
- The hint block is inserted between the transcript and the
  OUTPUT FORMAT instructions inside the user message, so positionally
  the LLM treats it as supporting context, not content to echo.

**Files touched.**

- `src-tauri/src/persona/mod.rs` — register `prosody` module.
- `src-tauri/src/persona/prosody.rs` — new (analyzer + 23 tests).
- `src-tauri/src/persona/extract.rs` — `build_persona_prompt_with_hints`
  + 4 new equivalence / integration tests.
- `src-tauri/src/brain/ollama_agent.rs` — `propose_persona_with_hints`
  surface.
- `src-tauri/src/commands/persona.rs` — wiring (only when ASR is
  configured).
- `docs/persona-design.md` — new § 9.4 with full signal table and
  privacy contract.
- `README.md` — Voice System section.
- `rules/milestones.md` — row 14.6 removed; Phase-14 summary updated.

**Validation.** Persona test family grew from 47 → 70 (`cargo test
--lib persona::`). Full lib suite: 892 passing. Clippy clean. No
network or audio I/O introduced.

---

## Chunk 14.7 — Persona Pack Export / Import

**Date:** 2026-04-24
**Reference:** `docs/persona-design.md` § 11.3 + § 12 (both updated this PR); architectural rule "brain documentation sync" (architecture-rules.md § 11).

**Goal.** Ship the camera-free persona pack so a user can back up an entire persona setup (active traits + every learned-expression + every learned-motion artifact) as a single self-describing JSON document — copyable to clipboard, savable as `.json`, ready to drop into Soul Link sync. Receiving side: dry-run preview before commit, atomic apply, per-entry skip report.

**Architecture.**
- New module **`src-tauri/src/persona/pack.rs`** — pure, I/O-free codec:
  - `PersonaPack { packVersion, exportedAt, note?, traits, expressions[], motions[] }` envelope. Per-asset entries kept as opaque `serde_json::Value` so future trait / expression / motion fields round-trip even when this binary doesn't know about them.
  - `build_pack` (constructor; trims+drops empty/whitespace `note`).
  - `pack_to_string` (pretty-printed JSON).
  - `parse_pack` — rejects empty input, oversize input (`PERSONA_PACK_MAX_BYTES = 1 MiB`), malformed JSON, missing required envelope fields, future `pack_version`, non-object `traits`.
  - `validate_asset(value, expected_kind) -> Result<id>` — mirrors the existing `validate_id` rules (alphanumeric + `_-`, length 1..=128) so path-traversal is impossible regardless of caller behaviour.
  - `ImportReport { traits_applied, expressions_accepted, motions_accepted, skipped[] }` + `note_skip` helper that caps the report at 32 entries plus a single truncation marker so a hostile pack cannot OOM the UI through skip messages.
- Three new Tauri commands in **`commands/persona.rs`**:
  - `export_persona_pack(note?)` — reads `persona.json` + `expressions/*.json` + `motions/*.json`, builds a `PersonaPack`, returns the pretty-printed string. Corrupt asset files are skipped silently (existing § 13 contract). `list_assets_as_values` preserves the on-disk `learnedAt` ordering for deterministic round-trips.
  - `preview_persona_pack(json)` — dry-run validator returning the per-entry report **without writing anything**. Powers the "🔍 Preview" button.
  - `import_persona_pack(json)` — replaces traits via the existing `atomic_write` helper; merges asset libraries (matching ids overwrite, others kept). Per-entry failures (wrong `kind`, illegal id, write failure) record a skip and continue, so a single bad asset doesn't lose the rest of the pack.
- Frontend: new Pinia store actions **`exportPack` / `previewImportPack` / `importPack`** in `src/stores/persona.ts`. `importPack` chains a `load()` so all UI bindings reflect the merged state in a single round-trip.
- New component **`src/components/PersonaPackPanel.vue`** (extracted from `PersonaPanel.vue` to keep both files under the 800-line Vue budget):
  - Export: optional one-line note, "⬇ Export" button, "📋 Copy" (uses `navigator.clipboard`), "💾 Save .json" (uses `Blob` + `<a download>` — works inside Tauri's WebView without the `dialog` plugin).
  - Import: collapsible textarea, "🔍 Preview" / "⤴ Apply import" / "Clear" buttons, inline error pane for parse failures, per-entry skip list. Uses `var(--ts-*)` design tokens throughout.
- `PersonaPanel.vue` mounts the new component and exposes `onPackImported` to re-sync its local draft state after a successful apply.

**Files created.**
- `src-tauri/src/persona/pack.rs` (408 lines incl. 18 unit tests).
- `src/components/PersonaPackPanel.vue` (326 lines incl. scoped styles).

**Files modified.**
- `src-tauri/src/persona/mod.rs` — added `pub mod pack`.
- `src-tauri/src/commands/persona.rs` — three new commands + `list_assets_as_values` helper (653 lines, well under 1000-line cap).
- `src-tauri/src/lib.rs` — register the three commands in the import + invoke-handler list.
- `src/stores/persona.ts` — three new actions + `ImportReport` type (364 lines).
- `src/stores/persona.test.ts` — 6 new tests (Tauri-unavailable export, success export, preview-throws-on-parse-error, preview-success, import-reloads-store, import-error).
- `src/components/PersonaPanel.vue` — replaced inline pack UI with `<PersonaPackPanel>` mount + `onPackImported` handler (653 lines, back under budget).
- `docs/persona-design.md` — new § 11.3 documents the envelope shape + size cap + merge semantics; § 12 lists the three new commands with ✅ shipped marker.
- `README.md` — Persona System component listing updated: new pack module, new store actions, new UI component.
- `rules/milestones.md` — chunk 14.7 row removed; Phase-14 footer + Next-Chunk pointer refreshed.
- `.gitignore` — added agent-scratch patterns (`test-output.txt`, `*.log`, `*.tmp`, `.scratch/`, `/tmp-agent/`) following the new prompting rule.
- `rules/prompting-rules.md` — new ENFORCEMENT RULE "Clean Up Temporary Files After Each Session".

**Tests.**
- Rust: 860 → **878** passing (18 new in `persona::pack` covering round-trip, missing/non-object/oversize/garbage envelope, future-version rejection, traits-only pack, all `validate_asset` rejection paths, and the `note_skip` 32+marker cap).
- Frontend Vitest: 1067 → **1073** passing across 67 files (6 new in `persona.test.ts`, plus the new `PersonaPackPanel.vue` covered indirectly via the store action tests).
- `vue-tsc --noEmit` clean.
- `npm run lint` 0 errors (only pre-existing `v-html` warnings).
- `cargo clippy --lib --no-deps -- -D warnings` clean.
- File sizes within budget (PersonaPanel.vue 653/800, PersonaPackPanel.vue 326/800, persona.rs 653/1000, pack.rs 408/1000).

**Privacy contract preserved.** This chunk is entirely camera-free. Persona packs only contain JSON artifacts (traits + landmark presets + retargeted-keypoint clips) — the same data already on disk under `<app_data_dir>/persona/`. No MediaStream is opened by either the export or the import flow; no webcam frames cross the IPC boundary.

---

## Chunk 14.2 — Master-Echo Brain-Extraction Loop (Persona Suggestion)

**Date:** 2026-04-24
**Reference:** `docs/persona-design.md` § 3 + § 9.3 + § 12 (all updated this PR); architectural rule "brain documentation sync".

**Goal.** Close the camera-free leg of the Master-Mirror loop: when a brain is configured, let the user click "✨ Suggest a persona from my chats" and have the active LLM read recent conversation history + their long-tier `personal:*` memories, propose a `PersonaTraits` JSON, and surface it for review-before-apply. Nothing auto-saves; the candidate flows through the existing `save_persona` path only after the user clicks Apply.

**Architecture.**
- New module **`src-tauri/src/persona/extract.rs`** (pure, I/O-free — same testable-seam shape as `memory/hyde.rs` / `memory/reranker.rs`):
  - `PromptSnippet` + `PersonaCandidate` types.
  - `assemble_snippets(history, memories)` — takes the last 30 turns + up to 20 memories, preferring `personal:*`-tagged ones and falling back to plain long-tier rows when none are tagged.
  - `build_persona_prompt(snippets) -> (system, user)` — explicit OUTPUT FORMAT block asking for ONLY a JSON object; honours a 12 KB char budget so the prompt never overflows small local models.
  - `parse_persona_reply(raw) -> Option<PersonaCandidate>` — tolerant of markdown fences, leading prose, brace-balanced extraction (skips `{`/`}` inside string literals), drops non-string list entries, dedupes case-insensitively, caps lists at 6, caps bio at 500 chars, requires non-empty `name`/`role`/`bio`.
- New brain method **`OllamaAgent::propose_persona(snippets)`** — three-line wrapper: build prompt → call → parse.
- New Tauri command **`extract_persona_from_brain`** in `commands/persona.rs`:
  - Snapshots `state.conversation` + `MemoryStore::get_by_tier(MemoryTier::Long)` *without* holding either lock across the await point (consistent with `extract_memories_from_session`).
  - Returns the candidate as a JSON string, `""` when the reply could not be parsed (UI shows soft "try again" message), or an `Err(...)` when no brain is configured (UI disables button + tooltip).
  - **Never** auto-saves.
- Frontend persona store action **`suggestPersonaFromBrain()`** — invokes the command, parses the JSON, defensively coerces list fields, stamps `lastBrainExtractedAt` only on success.
- Frontend UI in **`PersonaPanel.vue`** — "✨ Suggest from my chats" button next to the existing Save / Discard / Reset buttons + a green-bordered review card with three actions: **Apply** (routes through `saveTraits` so atomic-write + `set_persona_block` sync still happen), **Load into editor** (seeds the draft so the user can fine-tune before saving), **Discard**.

**Files created.**
- `src-tauri/src/persona/mod.rs` (10 lines, module doc)
- `src-tauri/src/persona/extract.rs` (463 lines incl. 16 unit tests)

**Files modified.**
- `src-tauri/src/lib.rs` — register `pub mod persona`, import + invoke-handler-register `extract_persona_from_brain`.
- `src-tauri/src/brain/ollama_agent.rs` — added `propose_persona` method.
- `src-tauri/src/commands/persona.rs` — added `extract_persona_from_brain` command.
- `src/stores/persona.ts` — added `suggestPersonaFromBrain` action.
- `src/stores/persona.test.ts` — added 6 new tests covering Tauri-unavailable, empty reply, malformed JSON, missing required fields, success stamps timestamp, and non-string list coercion.
- `src/components/PersonaPanel.vue` — new button + review card + scoped styles.
- `docs/persona-design.md` — § 3 mentions the camera-free third loop; § 9.3 marked "✅ shipped 2026-04-24" with full implementation breadcrumbs; § 12 updated.
- `README.md` — Persona System section: new module + new store action + new "✨ Suggest from my chats" UI flow listed.

**Tests.**
- Rust: 842 → **860** passing (16 new in `persona::extract` covering prompt construction, snippet assembly with personal-tag preference + fallback, char budget, all parser tolerances, and required-field rejection).
- Frontend Vitest: 1061 → **1067** passing across 66 files (6 new in `persona.test.ts`).
- `vue-tsc --noEmit` clean.
- `npm run lint` 0 errors (only pre-existing v-html warnings).
- `cargo clippy --lib --no-deps -- -D warnings` clean.
- File sizes well within budget (PersonaPanel.vue 638/800, extract.rs 463/1000, persona.rs 458/1000).

**Privacy contract preserved.** This loop is *entirely* camera-free. The persona-design § 5 invariants remain intact — no MediaStream is opened, no webcam frames cross any boundary, the per-session `cameraSession` state is untouched.

---

## Chunk 14.1 — Persona MVP (PersonaTraits store + prompt injection + UI)

**Date:** 2026-04-24 (backfilled 2026-04-30)
**Reference:** `docs/persona-design.md` § 15.1.

**Goal.** Foundation layer for the persona system: a `PersonaTraits` data model,
Pinia store for persistence, system-prompt injection utility, UI panel for editing,
and Soul Mirror quest activation in the skill tree.

**What shipped.**

- `src/stores/persona.ts` — Pinia store with `PersonaTraits` (name, role, bio,
  personality[], interests[], speaking_style[]), `load()` / `saveTraits()`,
  localStorage fallback + Tauri backend sync.
- `src/utils/persona-prompt.ts` — `buildPersonaBlock(traits)` → injects
  `[PERSONA]` block into the system prompt with name/role/bio/style directives.
- `src/components/PersonaPanel.vue` — editable form for all persona fields,
  Save / Discard / Reset buttons, design-token styling.
- `src/stores/skill-tree.ts` — "Soul Mirror" node activates when persona
  traits are configured (non-default name + role).
- `src-tauri/src/commands/persona.rs` — `get_persona` / `save_persona`
  Tauri commands with atomic JSON write.

**Tests.** Persona store tests + PersonaPanel component tests in vitest.
Foundation for all subsequent Phase 14 chunks (14.2–14.15).

---

## Chunk 2.4 — BrainView "Code knowledge" panel (Phase 13 Tier 4)

**Date:** 2026-04-24
**Reference:** `docs/brain-advanced-design.md` Phase 13 row in §22; built directly on Chunks 2.1 / 2.3 shipped earlier today.

**Goal.** Surface the GitNexus Tier 1 + Tier 3 plumbing in the Brain
hub so a user can mirror an indexed repo's KG, see what's already
mirrored, roll back a mirror, and run a blast-radius pre-flight on a
symbol — all without touching the CLI or copy-pasting JSON.

**Implementation.**
- `src-tauri/src/memory/edges.rs` — new
  `MemoryStore::list_external_mirrors(like_pattern)` aggregates
  `memory_edges` by `edge_source` (filtered by SQL LIKE) into one row
  per scope: `(edge_source, COUNT(*), MAX(created_at))`. Native edges
  (NULL `edge_source`) are excluded. Three new unit tests (groups
  correctly, empty store, scoped delete-by-edge-source).
- `src-tauri/src/commands/gitnexus.rs` — new
  `gitnexus_list_mirrors() -> Vec<GitNexusMirrorSummary>` Tauri
  command. Strips the `gitnexus:` prefix into a separate `scope`
  field so the frontend can pass it straight back to
  `gitnexus_unmirror`.
- `src-tauri/src/lib.rs` — command registered in `invoke_handler`.
- `src/components/CodeKnowledgePanel.vue` (new, ~430 lines incl.
  scoped CSS) — Vue 3 `<script setup lang="ts">` component:
  * Sync form: text input for the `repo:owner/name@sha` scope +
    "Sync KG" button → calls `gitnexus_sync` and renders an
    inserted/reused/skipped report.
  * Mirror list: rendered from `gitnexus_list_mirrors`, formats
    `last_synced_at` via `Intl.DateTimeFormat` (no extra date lib),
    per-row "Unmirror" button.
  * Blast-radius pre-flight: text input for a symbol + "Probe impact"
    button → calls `gitnexus_impact`; `summariseImpact` extracts a
    one-line dependent count from the three known upstream response
    shapes (`{symbol, dependents}`, `{items}`, `{count}`) and falls
    back to a JSON snippet for unknown shapes (forward-compatible).
  * All design tokens via `var(--ts-*)`; no hard-coded hex outside
    the `…, fallback` arguments.
  * Defensive: `mirrors.value` is always normalised to `[]` so that
    other test files mounting `BrainView` (with a stub `invoke` that
    returns `undefined`) don't crash.
- `src/views/BrainView.vue` — three-line wiring: import +
  `<section class="bv-code-knowledge-section"><CodeKnowledgePanel /></section>`
  inserted between the stats sheet and the persona panel. No other
  BrainView changes.

**Tests.** 9 new Vitest unit tests (`CodeKnowledgePanel.test.ts`):
empty state, disabled-when-empty sync button, ordered mirror render,
sync round-trip with refresh, per-row unmirror, impact summary, error
banner on capability denial, `summariseImpact` shape coverage,
`formatTimestamp` defensive fallback. **Frontend suite: 1052 → 1061
passing across 66 files.** Rust suite: 839 → 842 passing.
`cargo clippy --lib --no-deps -- -D warnings` clean; `npm run lint`
yields only the pre-existing `v-html` warnings (none on the new
files); `npx vue-tsc --noEmit` clean.

**Files changed.** 5 files (`memory/edges.rs`,
`commands/gitnexus.rs`, `lib.rs`,
`components/CodeKnowledgePanel.vue` [new],
`components/CodeKnowledgePanel.test.ts` [new],
`views/BrainView.vue`) + `docs/brain-advanced-design.md` +
`rules/milestones.md` + `rules/completion-log.md`.

---

## Chunk 2.3 — Knowledge-Graph Mirror (V7 `edge_source` column, Phase 13 Tier 3)

**Date:** 2026-04-24
**Reference:** `docs/brain-advanced-design.md` §8 (V7 schema) + Phase 13 Tier 3 row in §22; `rules/milestones.md` Phase 13.

**Goal.** Make GitNexus's structured knowledge graph durable inside
the TerranSoul brain. Prior chunks made the sidecar (2.1) and ephemeral
Code-RAG fusion (2.2) work; Tier 3 is the opt-in path that mirrors the
KG into SQLite so the rest of the brain (multi-hop traversal, the
BrainView graph panel) can reason over code structure alongside
free-text memories.

**Implementation.**
- `src-tauri/src/memory/migrations.rs` — new V7 migration adds a
  nullable `edge_source TEXT` column to `memory_edges` plus
  `idx_edges_edge_source`. Distinct from the existing `source` column
  (which records `user`/`llm`/`auto` provenance inside TerranSoul):
  `edge_source` records which **external KG** the edge came from.
  `NULL` is the default for every native edge. Up + down migrations
  shipped; round-trip test rebuilt for V7.
- `src-tauri/src/memory/edges.rs` — `MemoryEdge` and `NewMemoryEdge`
  gain `edge_source: Option<String>`; every SELECT/INSERT touched.
  New `MemoryStore::delete_edges_by_edge_source` for per-mirror
  rollback. All 23 existing test literals updated.
- `src-tauri/src/memory/gitnexus_mirror.rs` (new, ~440 lines incl.
  tests) — pure mapper:
  * `KgNode` / `KgEdge` / `KgPayload` deserialize-permissive structs
    (the `rel_type` field accepts `type` / `rel_type` / `relation`
    aliases for forward compatibility).
  * `map_relation(label)` — case-insensitive mapping of GitNexus's
    `CONTAINS` / `CALLS` / `IMPORTS` / `EXTENDS` / `HANDLES_ROUTE`
    into the existing 17-relation taxonomy (`contains`,
    `depends_on`, `derived_from`, `governs`); unknown labels flow
    through `normalise_rel_type` so future GitNexus versions don't
    break the mirror.
  * `mirror_kg(store, scope, payload)` — upserts one memory entry per
    KG node (idempotent via `source_hash` dedup), then batch-inserts
    every translated edge with `edge_source = "gitnexus:<scope>"`.
    Self-loops and dangling references are silently skipped and
    counted in the returned `MirrorReport`.
  * `unmirror(store, scope)` — single SQL DELETE by `edge_source`;
    leaves memory nodes intact (they may have accreted user-asserted
    or LLM-extracted edges).
- `src-tauri/src/agent/gitnexus_sidecar.rs` — new
  `GitNexusSidecar::graph(repo_label)` bridge method calling the
  upstream `graph` MCP tool.
- `src-tauri/src/commands/gitnexus.rs` — two new Tauri commands
  `gitnexus_sync(repoLabel, kgPayload?)` and
  `gitnexus_unmirror(repoLabel)`, plus a shape-tolerant
  `extract_kg_payload` that handles three known response shapes
  (top-level, nested under `graph.*`, and the MCP-standard
  `content[].text` envelope). Caller may bypass the sidecar by passing
  a payload directly — useful for tests and for clients that fetched
  the KG out-of-band.
- `src-tauri/src/lib.rs` — both commands registered in
  `invoke_handler`.
- Documentation: `docs/brain-advanced-design.md` §8 V7 schema entry +
  Phase 13 row marked done; README "Brain System" + "Memory System"
  sections updated to mention the new module, the V7 schema, the two
  new Tauri commands, and the `edge_source` column (per the brain-doc
  sync rule in `rules/architecture-rules.md`).

**Strictly opt-in.** No code in this module runs at startup. The
frontend explicitly calls `gitnexus_sync` when the user asks (Chunk
2.4 will add the BrainView panel that surfaces it).

**Tests.** 11 new unit tests in `gitnexus_mirror` (relation mapping,
case-insensitivity, normalised fall-through, scope formatting, full
mirror round-trip, idempotency, scoped unmirror, no-op unmirror, empty
scope rejection, alias parsing) + 4 extractor tests in
`commands::gitnexus` (top-level / nested / MCP-content / unknown
shapes) + 1 bridge test that the new `graph` method emits the right
JSON-RPC tool call. **Full suite: 839 → 853 tests, all passing.**
`cargo clippy --lib --no-deps -- -D warnings` clean.

**Files changed.** 7 files (`migrations.rs`, `edges.rs`,
`gitnexus_mirror.rs` [new], `gitnexus_sidecar.rs`, `commands/gitnexus.rs`,
`commands/memory.rs`, `lib.rs`, `mod.rs`) + `docs/brain-advanced-design.md`
+ `README.md` + `rules/milestones.md` + `rules/completion-log.md`.

---

## Repo Tooling — File-Size Quality Check

**Date:** 2026-04-24
**Reference:** `rules/coding-standards.md` "File Size Budget" section
**Trigger:** User input: "Please implement quality check for rust and Vue so these tools will make sure not a lot of code in just one file."

**Goal.** Prevent files from ballooning past a reviewable size. Rust
files capped at **1000 lines**, Vue SFCs at **800 lines**. Existing
oversized files are pinned in an allowlist and **must not grow** beyond
their pinned size — the long-term goal is for the allowlist to shrink
to zero.

**Implementation.**
- `scripts/check-file-sizes.mjs` — single-purpose Node script (zero
  dependencies, walks `src-tauri/src/**/*.rs` and `src/**/*.vue`,
  counts `\n` bytes for accuracy, supports `--update` to regenerate the
  allowlist, prints top-5 largest files on every run).
- `scripts/file-size-allowlist.json` — JSON map of repo-relative POSIX
  paths to their pinned line counts. Currently 10 entries (4 Rust + 6
  Vue), all pre-existing oversized files.
- `package.json` — new `check:file-sizes` npm script.
- `rules/coding-standards.md` — new "File Size Budget" section
  documenting thresholds, allowlist semantics, and the path to remove
  an entry once a file is split.
- `rules/prompting-rules.md` — `npm run check:file-sizes` added to the
  per-chunk Build Verification block.

**Behaviour.**
- Pass: every non-allowlisted file is ≤ its threshold AND every
  allowlisted file is ≤ its pinned size.
- Fail (exit 1): a non-allowlisted file exceeds its threshold, OR an
  allowlisted file has grown beyond its pinned size.

**Verified.** `node scripts/check-file-sizes.mjs` passes on the current
tree with the 10-entry allowlist; new chunk-2.2 files are all well
under budget (`memory/code_rag.rs` = 415 lines, `commands/memory.rs` =
847 lines after edits).

---

## Chunk 2.2 — Code-RAG Fusion in `rerank_search_memories` (Phase 13 Tier 2)

**Date:** 2026-04-24
**Reference:** `docs/brain-advanced-design.md` §22 (sidecar) + new §23 (fusion); `rules/milestones.md` Phase 13

**Goal.** With the GitNexus sidecar bridge (Chunk 2.1) in place, wire
its `query` tool into the recall stage of `rerank_search_memories` so
that — when **both** the user has granted `code_intelligence` for the
`gitnexus-sidecar` agent **and** a sidecar handle is live — the LLM
sees code-intelligence snippets alongside SQLite memories during the
LLM-as-judge rerank stage. Failures degrade silently to DB-only recall.

**Architecture.**

```
Stage 1   — RRF recall over SQLite (vector ⊕ keyword ⊕ freshness)
Stage 1.5 — NEW: gitnexus.query(prompt) → normalise → pseudo-MemoryEntries
            → reciprocal_rank_fuse([db_ids, code_ids], k=60)
            → truncate to candidates_k
Stage 2   — LLM-as-judge rerank (unchanged)
```

**Files created.**
- `src-tauri/src/memory/code_rag.rs` (415 LOC, 13 unit tests) —
  `gitnexus_response_to_entries(value, base_id_offset) → Vec<MemoryEntry>`,
  `is_code_rag_entry(&entry) → bool`, `CODE_RAG_TAG` constant,
  `MAX_CODE_RAG_ENTRIES = 16` defensive cap. Pure functions; no
  IO, no async, fully unit-tested.

**Files modified.**
- `src-tauri/src/memory/mod.rs` — register `code_rag` module.
- `src-tauri/src/commands/memory.rs` — new private async helper
  `code_rag_fuse(query, db_candidates, candidates_k, &state)` between
  Stages 1 and 2 of `rerank_search_memories`. ~80 LOC. Wraps every
  failure mode in `eprintln!` warnings + DB-only fallback.
- `docs/brain-advanced-design.md` — new §23 (full fusion pipeline,
  pseudo-entry schema, response-shape tolerance, failure-mode table,
  scope guard); §22.5 roadmap row marked ✅.
- `README.md` — new Brain System bullet under Tier 1.

**Pseudo-entry discriminators** (so downstream code can identify and
skip GitNexus-derived entries):
- `id`: strictly **negative** (`-1, -2, …`) — cannot collide with
  SQLite's positive `INTEGER PRIMARY KEY`.
- `tier`: `MemoryTier::Working` (ephemeral).
- `memory_type`: `MemoryType::Context` (transient retrieval context).
- `tags`: `code:gitnexus[,code:<sanitised-path>]`.
- `embedding`: `None` (we never embed code snippets locally).
- `decay_score`: `1.0`.

**Response-shape tolerance.** The normaliser accepts five published
shapes (`{snippets:[]}`, `{answer,sources:[]}`, `{results:[]}`,
top-level array, lone `{answer}`) and five field aliases
(`content`/`text`/`snippet`/`body`/`code` for body; `path`/`file`/
`location`/`uri`/`source` for source link). Defensive cap of 16 entries
prevents runaway responses from flooding the rerank stage.

**Failure modes — all degrade to DB-only recall**, never error:
1. Capability not granted → skip Stage 1.5.
2. Sidecar handle absent → skip Stage 1.5.
3. Sidecar process crashed / pipe closed → warn + DB results.
4. GitNexus returned RPC error → warn + DB results.
5. Unrecognised JSON shape → no merge.
6. Empty snippet list → no merge.

**Tests.**
- 13 new unit tests on the normaliser (empty values, all 5 shapes, ID
  monotonicity, MAX cap, whitespace dropping, comma-in-path tag
  round-trip, `is_code_rag_entry` selectivity, unknown-shape graceful
  empty, ephemeral-entry invariants).
- Backend total: **823 tests passing** (up from 809 after Chunk 2.1).
- Frontend total: 1052 tests passing (no changes required).
- File-size check: ✅ all new/modified files within budget.

**Out of scope (deferred to later tiers).**
- Tier 3 (Chunk 2.3) — KG mirror with V7 `edge_source` column.
- Tier 4 (Chunk 2.4) — BrainView "Code knowledge" panel.

---

## Chunk 2.1 — GitNexus Sidecar Agent (Phase 13 Tier 1)

**Date:** 2026-04-24
**Reference:** `rules/milestones.md` Phase 13 (GitNexus Code-Intelligence Integration), `docs/brain-advanced-design.md` §22 (new)

**Goal.** Ship Tier 1 of the four-tier GitNexus integration: spawn the
upstream `gitnexus` MCP server (`abhigyanpatwari/GitNexus`,
PolyForm-Noncommercial-1.0.0) as an out-of-process sidecar over stdio,
and expose the four core read-only tools (`query`, `context`, `impact`,
`detect_changes`) as Tauri commands behind a `code_intelligence`
capability gate. **Strictly out-of-process** — GitNexus's license
prevents bundling, so the user installs it under their own license terms
via the marketplace (`npx gitnexus mcp` by default).

**Architecture.**
- `agent/gitnexus_sidecar.rs` — async JSON-RPC 2.0 / MCP bridge with a
  pluggable `RpcTransport` trait (production `StdioTransport` wrapping
  `tokio::process::Command`, in-memory `mock::MockTransport` for tests).
  Performs the spec-mandated MCP handshake (`initialize` → response →
  `notifications/initialized`) lazily on first tool call and caches the
  initialization state. ID-tracked request/response loop skips stray
  notifications and stale responses; bounded by `MAX_SKIPPED_LINES = 256`
  to defend against runaway sidecars.
- `commands/gitnexus.rs` — 7 Tauri commands: `configure_gitnexus_sidecar`,
  `get_gitnexus_sidecar_config`, `gitnexus_sidecar_status`,
  `gitnexus_query`, `gitnexus_context`, `gitnexus_impact`,
  `gitnexus_detect_changes`. Each call refreshes capability state from
  `CapabilityStore`, lazily spawns the sidecar (cached in `AppState`),
  and forwards the JSON-RPC `result` to the frontend as `serde_json::Value`.
- `sandbox::Capability::CodeIntelligence` — new variant gating tool
  invocation. The user must approve `code_intelligence` for
  `gitnexus-sidecar` via the existing consent dialog before any tool
  call is forwarded.
- `registry_server::catalog` — added `gitnexus-sidecar` manifest with
  `InstallMethod::Sidecar { path: "npx gitnexus mcp" }`,
  `Network`+`Filesystem` capabilities, and the upstream's
  PolyForm-Noncommercial-1.0.0 license declared in the manifest.
- `package_manager::installer` — extended the "no binary download"
  branch (formerly `is_builtin`) to `skip_binary` covering both
  `BuiltIn` and `Sidecar` install methods, matching the existing
  `verify_manifest_trust` doc comment that already exempted sidecars
  from `sha256` requirements.

**Files created.**
- `src-tauri/src/agent/gitnexus_sidecar.rs` (~570 LOC, 11 unit tests)
- `src-tauri/src/commands/gitnexus.rs` (~230 LOC, 4 unit tests)

**Files modified.**
- `src-tauri/src/agent/mod.rs` — register new sidecar module
- `src-tauri/src/commands/mod.rs` — register new commands module
- `src-tauri/src/commands/sandbox.rs` — accept `"code_intelligence"`
  capability string in `parse_capability`
- `src-tauri/src/sandbox/capability.rs` — add `Capability::CodeIntelligence`
  variant, update `all()` and `display_name()`
- `src-tauri/src/registry_server/catalog.rs` — add `gitnexus-sidecar` entry
- `src-tauri/src/registry_server/server.rs` — bump catalog count to 4
- `src-tauri/src/package_manager/installer.rs` — generalize `is_builtin`
  → `skip_binary` to include `Sidecar`
- `src-tauri/src/lib.rs` — `AppState.gitnexus_config` +
  `AppState.gitnexus_sidecar` fields, register 7 new commands in the
  invoke handler
- `docs/brain-advanced-design.md` — new §22 covering the bridge
- `README.md` — new Code-Intelligence component listing

**Tests.**
- 11 sidecar unit tests (capability denial, handshake, ID matching,
  notification skipping, RPC error propagation, EOF handling,
  malformed-JSON handling, default config sanity)
- 4 Tauri-command-layer unit tests (capability rejection, full round
  trip, argument forwarding, RPC error pass-through)
- Backend total: 809 tests passing (up from 797 pre-chunk)
- Frontend total: 1052 tests passing (no changes required)

**Out of scope (deferred to later tiers).**
- Tier 2 (Chunk 2.2) — Code-RAG fusion in `rerank_search_memories`
- Tier 3 (Chunk 2.3) — Knowledge-graph mirror with `edge_source` column
- Tier 4 (Chunk 2.4) — BrainView "Code knowledge" panel

---

## Chunk 1.11 — Temporal KG Edges (V6 schema)

**Date:** 2026-04-24
**Phase 6 / §19.2 row 13 status:** 🔵 → ✅
**Reference:** `docs/brain-advanced-design.md` §16 Phase 6, §19.2 row 13 (Zep / Graphiti pattern, 2024)

### Goal
Give every `memory_edges` row an optional **temporal validity interval** so the brain can answer point-in-time graph queries ("what was true on date X?") and represent superseded facts non-destructively.

### Architecture
- **V6 migration** adds two nullable Unix-ms columns: `valid_from` (inclusive lower bound, `NULL` ≡ "always") and `valid_to` (exclusive upper bound, `NULL` ≡ "still valid"), plus an `idx_edges_valid_to` index. Right-exclusive convention keeps supersession unambiguous: closing edge A at `t` and inserting B with `valid_from = Some(t)` produces exactly one valid edge per timestamp.
- **`MemoryEdge::is_valid_at(t)`** — pure interval predicate. Uses `is_none_or` (clippy-clean).
- **`MemoryStore::get_edges_for_at(memory, dir, valid_at: Option<i64>)`** — point-in-time query; `valid_at = None` preserves legacy "return all edges" behaviour for full backward compatibility.
- **`MemoryStore::close_edge(id, t)`** — idempotent supersession (returns SQL row count).
- **Tauri surface:** `add_memory_edge` gained optional `valid_from` / `valid_to`; new `close_memory_edge` command exposes supersession to the frontend.
- **`StorageSelection.schema_label`** bumped from `"V5 — memory_edges"` to `"V6 — memory_edges + temporal validity"`.

### Files modified
- `src-tauri/src/memory/migrations.rs` — V6 migration up/down, `TARGET_VERSION = 6`, V6 round-trip + sentinel tests.
- `src-tauri/src/memory/edges.rs` — `MemoryEdge` + `NewMemoryEdge` extended with two `Option<i64>` fields; `add_edge` / `add_edges_batch` / `get_edge` / `get_edge_unique` / `list_edges` / `get_edges_for` / `row_to_edge` updated; new `is_valid_at`, `get_edges_for_at`, `close_edge` + 13 unit tests covering open/closed intervals, boundary inclusivity, point-in-time filtering, supersession pattern, and legacy-API non-regression.
- `src-tauri/src/commands/memory.rs` — `add_memory_edge` gained `valid_from` / `valid_to` parameters; new `close_memory_edge` command.
- `src-tauri/src/lib.rs` — registered `close_memory_edge`.
- `src-tauri/src/brain/selection.rs`, `src-tauri/src/commands/brain.rs` — schema label bumped to V6.
- 23 existing `NewMemoryEdge { … }` literals across the test suite updated with `valid_from: None, valid_to: None` (script-driven additive change; no behavioural diff).
- `docs/brain-advanced-design.md` — §16 ASCII roadmap row, §19.2 row 13 status + payoff text, §19.3 explanatory paragraph, §22 storage line bumped to V6.
- `README.md` — Brain System bullet for V6 temporal KG, Memory System V6 schema labels, Tauri command surface listing.
- `rules/milestones.md` — Chunk 1.11 row removed; Phase 13 (GitNexus integration, Chunks 2.1–2.4) filed as the new active set per the deep-analysis plan delivered in this session.

### Tests
- `cargo test --lib`: **783 passed** (768 baseline + 13 new edge tests + 2 new migration tests). 0 failures.
- Clippy: 1 warning fixed (`map_or` → `is_none_or`).

### Backward compatibility
- All 4 alternate storage backends (Postgres / MSSQL / Cassandra) do not implement the edges API today — V6 is SQLite-only and additive.
- Every legacy caller of `get_edges_for(..)` continues to receive every edge; the temporal filter is opt-in via the new `get_edges_for_at(..)` / `valid_at: Some(t)` path.

---

## Chunk 1.10 — Cross-encoder Reranker (LLM-as-judge)

**Date.** 2026-04-24
**Phase.** 12 (Brain Advanced Design)
**Origin.** `docs/brain-advanced-design.md` §16 Phase 6 / §19.2 row 10.

**Goal.** Add a true two-stage retrieval pipeline:

```text
RRF-fused hybrid recall (top candidates_k = 20)
        │
        ▼
Cross-encoder rerank (top limit = 10)  ──► prompt context
```

Bi-encoders (cosine vector search) embed query and document
independently and compare them with one dot product — fast at retrieval
time but lossy. A cross-encoder feeds `(query, document)` together so
phrase-level interactions are preserved; this is too expensive to run
over the whole corpus, hence the recall → precision split.

**Implementation choice — LLM-as-judge.** Rather than ship a separate
BGE-reranker-v2-m3 / mxbai-rerank model (extra download, extra RAM,
not available in the Free brain mode), we **reuse the active brain**
as the reranker by asking it to score each `(query, document)` pair
on a 0–10 integer scale. This is the well-documented LLM-as-judge
pattern (widely used in 2024 RAG eval pipelines and as a pragmatic
production reranker fallback). Quality is competitive when the chat
model is decent (Llama-3-8B+, Qwen-2.5+, any cloud model), and it
works in *all three* brain modes (Free / Paid / Local Ollama). The
`(query, document) -> Option<u8>` interface is identical to a future
dedicated-reranker backend, so swapping it in later is a one-line
change in the Tauri command.

**Architecture (three layers — same shape as Chunk 1.9 HyDE).**

1. **Pure logic** (`src-tauri/src/memory/reranker.rs`):
   - `build_rerank_prompt(query, doc) -> (system, user)` — includes a
     calibrated 0/3/6/8/10 rubric so even small models produce
     consistent scores; clips the document to 1500 chars to stay
     within small-model context budgets.
   - `parse_rerank_score(reply) -> Option<u8>` — robust to chat
     noise: `"7"`, `"7."`, `"**7**"`, `"Score: 7"`, `"7 out of 10"`
     all parse to `Some(7)`; rejects out-of-range and unparseable.
   - `rerank_candidates(candidates, scores, limit) -> Vec<MemoryEntry>`
     — sorts by score descending, breaks ties by original bi-encoder
     rank, **keeps unscored candidates ranked below scored ones
     rather than dropping them** so a flaky brain never silently
     loses recall.
2. **Brain wrapper** (`OllamaAgent::rerank_score`) — single LLM round-
   trip per pair; returns `Option<u8>` (`None` on failure).
3. **Tauri command** (`commands::memory::rerank_search_memories`) —
   stage 1 calls `hybrid_search_rrf` with `candidates_k` (default 20,
   clamped `limit..=50`) for recall; stage 2 scores each candidate
   sequentially (sequential to stay under provider rate limits) and
   reorders. **Cold-start safety:** if no brain is configured, the
   rerank stage is skipped and the command behaves exactly like
   `hybrid_search_memories_rrf` so callers can adopt it
   unconditionally.

**Files modified.**
- `src-tauri/src/memory/reranker.rs` — **new module** (~260 LOC
  including 14 unit tests covering prompt structure, doc truncation,
  whitespace trimming, score parsing across 6 reply shapes,
  out-of-range rejection, no-digits rejection, zero-limit, empty-
  candidates, score-descending sort, original-rank tie break,
  unscored-kept-below, all-unscored-preserves-order, limit truncation).
- `src-tauri/src/memory/mod.rs` — register `pub mod reranker;`.
- `src-tauri/src/brain/ollama_agent.rs` — `OllamaAgent::rerank_score`.
- `src-tauri/src/commands/memory.rs` — `rerank_search_memories` Tauri
  command with two-stage pipeline + no-brain fallback.
- `src-tauri/src/lib.rs` — command registration.
- `docs/brain-advanced-design.md` — §16 Phase 6 row + §19.2 row 10
  status flipped to ✅; §19.3 expanded.
- `rules/milestones.md` — Chunk 1.10 row removed; next-chunk pointer
  advanced to Chunk 1.11.
- `README.md` — Brain System / Memory System / Tauri command surface
  sections updated.

**Tests.** 768 Rust unit tests pass (754 baseline + 14 new
`memory::reranker::tests::*`).

---

## Chunk 1.9 — HyDE (Hypothetical Document Embeddings)

**Date.** 2026-04-24
**Phase.** 12 (Brain Advanced Design)
**Origin.** `docs/brain-advanced-design.md` §16 Phase 6 / §19.2 row 4
(Gao et al., 2022 — *"Precise Zero-Shot Dense Retrieval without
Relevance Labels"*).

**Goal.** Add a `hyde_search_memories(query, limit)` Tauri command that
asks the active brain to write a *hypothetical answer* to the query,
embeds that hypothetical answer, then runs RRF-fused hybrid search
using the hypothetical embedding instead of the raw query embedding.
Improves recall on cold, abstract or one-word queries — the seminal
HyDE paper reports large gains across BEIR / TREC / Mr. TyDi without
fine-tuning.

**Architecture.** Three layers:

1. **Pure prompt + reply cleaner** (`src-tauri/src/memory/hyde.rs`).
   `build_hyde_prompt(query) -> (system, user)` produces the LLM
   prompt; `clean_hyde_reply(reply) -> Option<String>` strips common
   chat preambles ("Sure, ...", "Answer: ...", "Hypothetical answer: ..."),
   collapses whitespace and rejects too-short outputs (< 4 chars). Both
   are pure, fully unit-tested without the network.
2. **Brain wrapper** (`OllamaAgent::hyde_complete`). Wraps the prompt +
   `call` round-trip + cleaner; returns `Option<String>` (`None` on
   network failure or unusable reply).
3. **Tauri command** (`commands::memory::hyde_search_memories`). Chains
   `hyde_complete → embed_text → hybrid_search_rrf` with a three-stage
   fallback so the command is *always* useful:
   - HyDE expansion fails → embed the raw query.
   - Embedding step also fails → fall back to keyword + freshness
     ranking via `hybrid_search_rrf` with no embedding.
   - No brain configured → keyword + freshness only.

**Why a separate command, not an option flag.** HyDE costs one extra
LLM round-trip per query, which is fine for explicit retrieval calls
but should not silently apply to every chat-time RAG injection.
Exposing it as `hyde_search_memories` lets callers (a Search panel,
an "explain my memories" workflow) opt in explicitly while
`hybrid_search_memories_rrf` stays the cheap default.

**Files modified.**
- `src-tauri/src/memory/hyde.rs` — **new module** (~190 LOC including 10
  unit tests covering preamble stripping, whitespace collapsing,
  too-short rejection, query trimming, idempotence, no-preamble safety).
- `src-tauri/src/memory/mod.rs` — register `pub mod hyde;`.
- `src-tauri/src/brain/ollama_agent.rs` — `OllamaAgent::hyde_complete`.
- `src-tauri/src/commands/memory.rs` — `hyde_search_memories` Tauri
  command.
- `src-tauri/src/lib.rs` — command registration.
- `docs/brain-advanced-design.md` — §16 Phase 6 row + §19.2 row 4 status
  flipped to ✅; §19.3 expanded with HyDE description.
- `rules/milestones.md` — Chunk 1.9 row removed; next-chunk pointer
  advanced to Chunk 1.10.

**Tests.** 754 Rust unit tests pass (744 baseline + 10 new
`memory::hyde::tests::*`).

---

## Chunk 1.8 — RRF Wired into Hybrid Search

**Date.** 2026-04-24
**Phase.** 12 (Brain Advanced Design)
**Origin.** `docs/brain-advanced-design.md` §16 Phase 6 / §19.2 row 2.

**Goal.** Wire the already-shipped Reciprocal Rank Fusion utility
(`src-tauri/src/memory/fusion.rs`) into a real `MemoryStore` retrieval
method so RRF moves from "utility on the shelf" to "production retrieval
path", flipping §19.2 row 2 from 🟡 → ✅.

**Why RRF, not weighted sum.** The legacy `hybrid_search` combines six
signals (vector cosine, keyword hits, recency, importance, decay, tier)
with hand-tuned weights summed into a single score. This is fragile when
the underlying signal scales differ — raw cosine is in `[0, 1]`, keyword
hit ratio is in `[0, 1]`, decay is in `[0, 1]`, but the sum has no
principled interpretation. RRF (Cormack et al., SIGIR 2009) operates
purely on rank position with a single dampening constant (`k = 60`), is
the de-facto standard across LangChain / LlamaIndex / Weaviate, and
removes the need for weight tuning when retrievers disagree on score
magnitude.

**Architecture.**

1. `MemoryStore::hybrid_search_rrf(query, query_embedding, limit)` builds
   three independent rankings:
   - **Vector** — cosine similarity of `query_embedding` against every
     embedded memory, descending; deterministic id tie-break.
   - **Keyword** — count of distinct query tokens (length > 2) appearing
     in `content` or `tags`, case-insensitive, descending; entries with
     zero hits are excluded from this ranking only.
   - **Freshness** — composite of recency (24 h half-life), importance
     (1–5), `decay_score`, and tier weight (Working > Long > Short).
2. The non-empty rankings are passed to
   `crate::memory::fusion::reciprocal_rank_fuse` with the standard
   `DEFAULT_RRF_K = 60`. Missing-from-some-rankings is handled
   gracefully by the fusion utility itself.
3. Top `limit` ids are materialised back into `MemoryEntry` structs (no
   second DB round-trip — entries are indexed by id from the original
   load) and `last_accessed` / `access_count` are updated, matching the
   semantics of every other search method.

**Storage backend trait.** `StorageBackend::hybrid_search_rrf` ships
with a default implementation that delegates to
`StorageBackend::hybrid_search`, so Postgres / MSSQL / Cassandra
backends keep compiling and may opt into RRF natively later (the SQLite
backend overrides it with the real implementation).

**Tauri surface.** New command `hybrid_search_memories_rrf(query, limit)`
registered in `src-tauri/src/lib.rs` next to `hybrid_search_memories`.
The legacy weighted-sum command is preserved for backward compatibility
and for callers that want the original behaviour.

**Files modified.**
- `src-tauri/src/memory/store.rs` — new `hybrid_search_rrf` method
  (~120 LOC) + 6 unit tests covering keyword ranking, zero-limit, empty
  store, freshness-only fallback, vector primacy, determinism.
- `src-tauri/src/memory/backend.rs` — new trait method with default
  delegation.
- `src-tauri/src/commands/memory.rs` — new `hybrid_search_memories_rrf`
  Tauri command.
- `src-tauri/src/lib.rs` — command registration.
- `docs/brain-advanced-design.md` — §16 Phase 6 row updated to ✅, §19.2
  row 2 status text updated, §19.3 expanded with the wire-in details.
- `rules/milestones.md` — Chunk 1.8 row removed.

**Tests.** 744 Rust unit tests pass (738 baseline + 6 new
`hybrid_search_rrf_*` tests).

---

## Chunk 1.7 (Distribution) — Real Downloadable Agent Distribution

**Date:** 2026-04-23

### Summary

Closed the last "no path to ship a third-party downloadable agent" gap
in the agent marketplace.

### What changed

1. **Mandatory `sha256` on every downloadable install method.** The
   installer (`PackageInstaller::install` / `::update`) now refuses to
   install a `Binary { url }` or `Wasm { url }` agent whose manifest
   omits the `sha256` field, returning a new
   `InstallerError::MissingSha256(name)` before any download or disk
   write. Built-in (`InstallMethod::BuiltIn`) and bundled (`Sidecar`)
   agents are exempt — they have no remote bytes to hash.
2. **Optional Ed25519 manifest signatures with a curated publisher
   allow-list.** New module
   `src-tauri/src/package_manager/signing.rs` wraps `ed25519-dalek` to
   verify a detached `signature` field over a deterministic
   `canonical_signing_payload(manifest)` (covers
   `name` + `version` + install-method discriminator + URL +
   `sha256`). When a manifest declares a `publisher`, the publisher
   must appear in the compile-time `PUBLISHER_ALLOW_LIST` and the
   signature must verify against the recorded public key. Cosmetic
   edits (description, homepage, capabilities, license, author) do
   **not** invalidate the signature; swapping the binary URL or hash
   does. New `InstallerError::SignatureVerificationFailed(SigningError)`
   and `SigningError::{UnknownPublisher, InvalidSignatureEncoding,
   InvalidSignatureLength, SignatureMismatch, InvalidPublicKey}`
   surface the failure mode. The allow-list ships **empty** — real
   publisher keys are added by maintainers in code-reviewed PRs only,
   never injectable at runtime.
3. **Hosting model: `307 Temporary Redirect` from the registry to the
   upstream binary host.** `registry_server::server::download_agent`
   no longer returns a fixed empty body for downloadable agents — it
   issues `Redirect::temporary(url)` to the manifest's
   `Binary { url }` / `Wasm { url }`. This keeps the registry stateless
   and bandwidth-free; agent binaries live on GitHub Releases (or
   S3 / R2). `reqwest` already follows redirects, so `HttpRegistry`
   needed no client-side changes.
4. **End-to-end integration test
   (`src-tauri/src/registry_server/distribution_e2e_tests.rs`).**
   Spawns two real `axum` HTTP servers on free ports — an "upstream
   binary host" serving the bytes and a "registry server" serving the
   manifest with the redirect contract — then drives
   `PackageInstaller::install` through `HttpRegistry` and asserts:
   - the happy path writes a non-empty `agent.bin` whose content and
     SHA-256 match the upstream payload;
   - a tampered upstream binary triggers `HashMismatch` with **no disk
     side-effects** (the agent directory is never created);
   - a manifest without `sha256` is rejected with `MissingSha256`
     before any download is attempted.
5. **Manifest schema extended.** `AgentManifest` gains optional
   `publisher: Option<String>` and `signature: Option<String>` fields.
   Validator rejects malformed signatures (must be 128 hex chars / 64
   bytes); new `ManifestError::InvalidSignature` variant.

### Files touched

- `src-tauri/src/package_manager/manifest.rs` — `publisher` + `signature`
  fields, `ManifestError::InvalidSignature` + `validate_signature`.
- `src-tauri/src/package_manager/installer.rs` — `verify_manifest_trust`
  helper, `InstallerError::{MissingSha256, SignatureVerificationFailed}`,
  installer + updater enforcement, new tests for missing-sha and
  unknown-publisher rejection.
- `src-tauri/src/package_manager/signing.rs` — **new**, full
  Ed25519 signing/verification module with 11 unit tests.
- `src-tauri/src/package_manager/mod.rs` — re-exports.
- `src-tauri/src/registry_server/server.rs` — `307 Temporary Redirect`
  contract for downloadable install methods.
- `src-tauri/src/registry_server/catalog.rs` — backfill `publisher`/
  `signature: None` on built-in catalog entries.
- `src-tauri/src/registry_server/distribution_e2e_tests.rs` — **new**,
  three end-to-end integration tests against real `axum` fixtures.
- `src-tauri/src/registry_server/mod.rs` — wires the new test module.
- `rules/milestones.md` — Chunk 1.7 row removed (now done).
- `rules/completion-log.md` — this entry.

### Verification

- `cargo build --tests` (from `src-tauri`) — ✅
- `cargo test --all-targets` — **712 tests pass** (was 561 before
  Chunk 1.7 work; 11 new signing tests + 3 new e2e tests + 2 new
  installer guard tests).
- `cargo clippy --all-targets -- -D warnings` — ✅ (0 warnings)
- `npm run build` — ✅
- `npm run test` — **1016 frontend tests pass** (no frontend code touched
  by this chunk; verifies nothing regressed).

### Notes for future maintainers

- Adding a real publisher: append a `PublisherEntry` to
  `PUBLISHER_ALLOW_LIST` in `src-tauri/src/package_manager/signing.rs`.
  Use a 32-byte raw Ed25519 verifying key; store hex in PR description
  for review.
- Signing a manifest: build the canonical payload with
  `signing::canonical_signing_payload(&manifest)` and sign with
  `ed25519-dalek`'s `SigningKey::sign`. The hex-encoded 64-byte
  signature is the `signature` field.
- The HTTP registry deliberately **does not** stream-proxy binary
  bytes — keep this property when adding new install methods.

---

## Chunk 1.7 — Cognitive Memory Axes + Marketplace Catalog Default + Local Models as Agents + OpenClaw Bridge

**Date:** 2026-04-23

### Summary
Four entwined improvements landed in one PR:

1. **Episodic vs Semantic Memory analysis & implementation.** Added a deep
   analysis section (`docs/brain-advanced-design.md` § 3.5) arguing that we
   need a third *cognitive* axis (episodic / semantic / procedural) on top
   of the existing `MemoryType` and `MemoryTier` axes, but **derived not
   stored** to avoid a schema migration. Shipped a pure-function classifier
   in `src-tauri/src/memory/cognitive_kind.rs` that resolves the kind from
   `(memory_type, tags, content)` with explicit `episodic:*` / `semantic:*`
   / `procedural:*` tag override. 16 unit tests cover the resolution rules.
2. **Marketplace browse fix.** The default `package_registry` was an empty
   `MockRegistry`, so the Marketplace browse tab showed nothing until the
   user manually started the registry HTTP server. Added
   `registry_server::CatalogRegistry` — an in-process `RegistrySource` that
   pre-populates from `catalog::all_entries()` — and wired it as the default.
   `start_registry_server` still swaps in `HttpRegistry` for cross-device
   discovery; `stop_registry_server` restores the catalog registry.
3. **Local LLM models as marketplace agents.** Extended `search_agents` to
   merge local Ollama recommendations as virtual agents (`kind: "local_llm"`,
   capability `local_llm` + `chat`). `MarketplaceView` now renders local-LLM
   cards with **Install & Activate** that calls `pull_ollama_model` +
   `set_active_brain` + `set_brain_mode`. Card surfaces top-pick / cloud /
   RAM badges and warns if Ollama isn't running.
4. **OpenClaw example provider.** New `src-tauri/src/agent/openclaw_agent.rs`
   implementing `AgentProvider` with capability gating, tool-call dispatch
   (`/openclaw read | fetch | chat`), and sentiment passthrough. The match
   arms in `handle_command` are the single integration point for swapping in
   a real JSON-RPC client. Documented end-to-end in
   `instructions/OPENCLAW-EXAMPLE.md`, referenced from the README.

### Files Added
- `src-tauri/src/memory/cognitive_kind.rs` (classifier + 16 tests)
- `src-tauri/src/registry_server/catalog_registry.rs` (default registry + 7 tests)
- `src-tauri/src/agent/openclaw_agent.rs` (provider + 12 tests)
- `instructions/OPENCLAW-EXAMPLE.md` (end-to-end walkthrough)

### Files Modified
- `docs/brain-advanced-design.md` — new § 3.5
- `rules/architecture-rules.md` — module-dependency rules updated
- `instructions/EXTENDING.md` — references to OpenClaw example + cognitive kinds
- `README.md` — Marketplace bullet links to OpenClaw walkthrough
- `src-tauri/src/lib.rs` — default `package_registry` → `CatalogRegistry`
- `src-tauri/src/memory/mod.rs` — re-export `cognitive_kind`
- `src-tauri/src/agent/mod.rs` — register `openclaw_agent`
- `src-tauri/src/registry_server/mod.rs` — re-export `CatalogRegistry`
- `src-tauri/src/commands/registry.rs` — `AgentSearchResult` gains
  `kind`/`model_tag`/`required_ram_mb`/`is_top_pick`/`is_cloud`;
  `search_agents` merges local-LLM recommendations;
  `stop_registry_server` restores catalog registry
- `src/types/index.ts` — `AgentSearchResult` extended (all new fields optional)
- `src/views/MarketplaceView.vue` — local-LLM cards + Install & Activate flow

### Test Counts
- **Rust:** +41 tests → 695 total (was 654). All passing under
  `cargo clippy --all-targets -- -D warnings` and `cargo test --all-targets`.
- **Frontend:** 988 vitest tests, 60 files — all passing.

### Architectural notes
- **No schema migration.** The cognitive axis is computed; the V4 schema is
  unchanged. Migration path to a V6 column documented in § 3.5.7 if profiling
  later requires it.
- **No new Tauri commands.** All UX uses existing commands
  (`search_agents`, `pull_ollama_model`, `set_active_brain`, `set_brain_mode`).
- **OpenClaw bridge is reference-grade.** Capability set is held inside the
  agent so misconfigured orchestrators cannot bypass consent.

---



**Date:** 2026-04-23

**Goal.** Promote the memory layer from a tag-based co-occurrence graph to a
proper knowledge graph with typed, directional edges between memories, plus a
multi-hop RAG path that walks the graph from each direct hit. This was
documented as "Future: Entity-Relationship Graph" in
`docs/brain-advanced-design.md` §6 and is now shipped end-to-end (DB → Rust
core → Tauri commands → Pinia store → Cytoscape UI).

### What shipped

**Schema (V5 migration — `src-tauri/src/memory/migrations.rs`).**
`memory_edges (id, src_id, dst_id, rel_type, confidence, source, created_at)`
with `ON DELETE CASCADE` to both endpoints, `UNIQUE(src_id, dst_id, rel_type)`
for idempotent inserts, and `idx_edges_src` / `idx_edges_dst` /
`idx_edges_type` for traversal speed. `PRAGMA foreign_keys=ON` is now
enforced at every SQLite connection so cascade actually fires (V4 didn't need
it; V5 does).

**Rust core (`src-tauri/src/memory/edges.rs`, ~620 LOC).**
- `MemoryEdge`, `NewMemoryEdge`, `EdgeSource` (`user`/`llm`/`auto`),
  `EdgeDirection` (`in`/`out`/`both`), `EdgeStats` types with serde + clamping
  helpers.
- `MemoryStore::add_edge` / `add_edges_batch` / `list_edges` /
  `get_edges_for(id, direction)` / `delete_edge` / `delete_edges_for_memory` /
  `edge_stats` — all implemented as inherent methods using a new
  `pub(crate) fn conn(&self) -> &Connection` accessor on `MemoryStore`.
- Cycle-safe BFS `traverse_from(start_id, max_hops, rel_filter)` walks edges
  in **both** directions (a knowledge-graph hop is undirected for retrieval),
  excludes the start node from the result, and supports an optional relation
  whitelist.
- `hybrid_search_with_graph(query, query_emb, limit, hops)` runs the existing
  `hybrid_search` for seed pool, then walks `hops` deep from each seed and
  re-ranks with `seed_score / (hop + 1)`, deduping by id (keeping the highest
  score). Falls back to plain hybrid when `hops == 0` or no direct hits.
- `parse_llm_edges(text, known_ids)` is a forgiving JSON-line parser that
  drops self-loops, unknown ids, and clamps confidence to `[0, 1]`.
- 17 curated relation types (`COMMON_RELATION_TYPES`) + `normalise_rel_type`
  (lowercase, spaces → `_`, ASCII alnum + `_` only).

**LLM extraction (`src-tauri/src/brain/ollama_agent.rs` +
`src-tauri/src/memory/brain_memory.rs`).**
- `OllamaAgent::propose_edges(memories_block) -> String` — prompt-engineered
  to reply with one JSON object per line or the literal `NONE`. Reuses the
  existing private `call` so we don't expose `ChatMessage` outside the brain
  module.
- `extract_edges_via_brain(model, store, chunk_size)` — chunks memories
  (default 25, clamped 2..=50), calls `propose_edges`, parses, and inserts via
  `add_edges_batch`. Returns count of new edges actually inserted.

**Tauri commands (`src-tauri/src/commands/memory.rs` +
`src-tauri/src/lib.rs`).**
- `add_memory_edge(srcId, dstId, relType, confidence?, source?)`
- `delete_memory_edge(edgeId)`
- `list_memory_edges()`
- `get_edges_for_memory(memoryId, direction?)`
- `get_edge_stats()`
- `list_relation_types()` — returns the curated vocabulary
- `extract_edges_via_brain(chunkSize?)` — async; releases store lock across
  every LLM call so the UI never freezes
- `multi_hop_search_memories(query, limit?, hops?)` — `hops` hard-capped at 3

**Frontend (`src/types/index.ts`, `src/stores/memory.ts`,
`src/components/MemoryGraph.vue`, `src/views/MemoryView.vue`).**
- New TS types: `MemoryEdge`, `EdgeStats`, `EdgeSource`, `EdgeDirection`.
- `useMemoryStore` extended with `edges`, `edgeStats`, `fetchEdges`, `addEdge`
  (upsert-style), `deleteEdge`, `getEdgesForMemory`, `getEdgeStats`,
  `listRelationTypes`, `extractEdgesViaBrain`, `multiHopSearch`.
- `MemoryGraph.vue` — three rendering modes (`typed` | `tag` | `both`),
  directional target arrows, per-relation-type stable color hashing, edge
  labels with `text-rotation: autorotate`, and edge selection (`select-edge`
  emit). Tag overlays render faded so typed edges remain visually dominant.
- `MemoryView.vue` — toolbar with edge-mode dropdown, "🔗 Extract edges"
  brain action, edge counter, and per-node edge list with delete buttons in
  the node detail panel.

### Tests added

- **Rust (14 new tests in `memory::edges::tests`):** add_edge round-trip,
  self-loop rejection, idempotent insert, rel_type normalisation, batch with
  duplicate + self-loop skip, **cascade delete on memory removal**, directional
  `get_edges_for`, BFS hop limits + cycle handling, rel-type filter, edge
  stats aggregation, LLM JSON parser invalid-line handling, **multi-hop graph
  re-ranking pulling in keyword-disjoint neighbours**, V5 migration
  up/down/up round-trip, format truncation.
- **Frontend (6 new tests in `src/stores/memory.test.ts`):** `fetchEdges`,
  `addEdge` upsert behavior, `deleteEdge` cache update, `extractEdgesViaBrain`
  refresh, `multiHopSearch` parameter forwarding, `getEdgeStats` cache.

### Validation

- `cargo clippy --all-targets -- -D warnings` ✅ (0 warnings)
- `cargo test --all-targets` ✅ **654 passed** (640 baseline + 14 new)
- `npm run build` ✅
- `npm run test` ✅ **982 passed** (976 baseline + 6 new)

### Why this matters

The "Future: Entity-Relationship Graph" section of the brain design doc is
now retired — the V5 schema, multi-hop search, and LLM-powered edge
extraction are all live. This unblocks the queries Cognee was praised for in
§13.4 ("Who are all the clients connected to the Smith case, and what are
their communication preferences?") and gives the UI a true knowledge-graph
visualisation instead of tag overlap.

Documents updated alongside the code:
- `docs/brain-advanced-design.md` — §6 promoted from "Future" to
  "Implemented (V5)"; §8 schema split into Shipped V5 / Proposed V6/V7;
  §11 ops table gained Extract Edges + Multi-Hop sections; §13 Mem0 row +
  cross-framework knowledge-graph row updated; §16 Phase 3 marked shipped;
  §13.4 Cognee paragraph rewritten in present tense.
- `rules/milestones.md` — added Chunk 1.6 row (status `done`).
- `rules/completion-log.md` — this entry.

---

## Chunk 1.5 — Multi-Agent Roster + External CLI Workers + Temporal-style Durable Workflows

**Date:** 2026-04-23

**Goal.** Turn TerranSoul's single in-process companion into a full
**agent roster** where the user can create, name, switch between, and
delete multiple agents that may share or have distinct VRMs and may be
backed by either the native brain or an external CLI worker (Codex /
Claude / Gemini / custom). Long-running CLI work is tracked via a
**Temporal.io-style durable workflow engine** (append-only SQLite log,
replay-after-crash) and limited by a **RAM-aware concurrency cap** so
a laptop doesn't deadlock.

**Scope delivered.**

- **Backend — agent roster**
  - `src-tauri/src/agents/roster.rs` — `AgentProfile` + `BrainBackend`
    (`Native(BrainMode)` / `ExternalCli { kind, binary, extra_args }`).
    Atomic JSON persistence under `<data_dir>/agents/<id>.json` with
    `fs::rename` tmp-file swap; `current_agent.json` sibling that
    **self-heals** when the referenced agent is deleted.
  - `MAX_AGENTS = 32` roster cap; IDs restricted to
    `[A-Za-z0-9_-]{1,64}`; display names ≤ 120 chars; custom binary
    names validated alphanumerics + `-`/`_`/`.` only (no path
    separators, no shell metacharacters).
- **Backend — external CLI sandbox** (`src-tauri/src/agents/cli_worker.rs`)
  - Allow-list of kinds (`Codex`, `Claude`, `Gemini`, `Custom`).
  - `Command::new(binary)` with pre-split `Vec<String>` args — no
    `sh -c`. Sets `Stdio::null()` on stdin, clears env and keeps only
    `PATH` / `HOME` / `USER` / `LANG` / `LC_ALL` / `TERM` so API keys
    in the main process are **not** leaked.
  - Validates working folder exists + is a directory, prompt is
    non-empty and ≤ 32 KB, args contain no NUL bytes.
  - Emits `CliEvent::{Started, Line, Exited, SpawnError}` via
    `tokio::sync::mpsc::UnboundedReceiver` so the workflow engine
    persists each line before ACK.
- **Backend — durable workflow engine** (`src-tauri/src/workflows/engine.rs`)
  - Append-only `workflow_events` table in `<data_dir>/workflows.sqlite`
    (`UNIQUE(workflow_id, seq)`, covering indices on `workflow_id` and
    `kind`). Every append runs in a transaction so a crash mid-write
    never produces a gap in `seq`.
  - 8 event kinds: `Started`, `ActivityScheduled`,
    `ActivityCompleted`, `ActivityFailed`, `Heartbeat`, `Completed`,
    `Failed`, `Cancelled`. Appends after a terminal event are rejected.
  - On startup the engine loads every non-terminal workflow and reports
    it as `Resuming` until a live handle re-attaches — inspired by
    Temporal.io's history pattern but **without** the server stack
    (no JVM, no Postgres, no Cassandra; just `rusqlite` + `tokio`).
- **Backend — RAM cap** (`src-tauri/src/brain/ram_budget.rs`)
  - Pure `compute_max_concurrent_agents(free_mb, agents)`:
    `clamp( floor((free_mb - 1500) / mean_per_agent_mb), 1, 8 )`.
  - Footprint estimates: Native API 200 MB, Local Ollama 200 MB +
    model size, External CLI 600 MB.
  - `free_ram_mb()` reads `sysinfo::System::available_memory()` so the
    number reflects reclaimable cache, not just the raw free figure.
- **Tauri commands** (12 new, all `roster_*`-prefixed)
  - `roster_list`, `roster_create`, `roster_delete`, `roster_switch`,
    `roster_get_current`, `roster_set_working_folder`,
    `roster_get_ram_cap`, `roster_start_cli_workflow`,
    `roster_query_workflow`, `roster_cancel_workflow`,
    `roster_list_workflows`, `roster_list_pending_workflows`.
  - `roster_start_cli_workflow` enforces the RAM cap at activation time
    and rejects with a clear error message when saturated.
  - CLI output is fanned out to the frontend on the `agent-cli-output`
    event channel so the chat UI can stream stdout/stderr live.
- **Frontend.** `src/stores/agent-roster.ts` Pinia store:
  `agents`, `currentAgent`, `ramCap`, `workflows`, `atRamCap`,
  `activeWorkflowCount`, plus `createAgent`, `deleteAgent`,
  `switchAgent`, `setWorkingFolder`, `startCliWorkflow`,
  `cancelWorkflow`. Browser fallback yields a single in-memory default
  agent so the web preview never crashes.
- **Tests.**
  - **Rust — 41 new tests** covering roster serde round-trip,
    shell-metachar refuse-list, max-agents overflow, atomic save
    resilience, self-healing current-agent pointer, echo spawn +
    drain, unknown-binary failure path, workflow replay after
    simulated app restart, terminal-event lock, activity round-trip,
    RAM-cap exhaustive table.
  - **Frontend — 9 new Vitest tests** covering the store's
    browser fallback, Tauri refresh fan-out, `atRamCap` derivation,
    `createAgent` payload shape, error surfacing.
- **Docs.**
  - `instructions/AGENT-ROSTER.md` — user walkthrough, sandbox model
    table, RAM cap formula, workflow replay semantics, FAQ.
  - `docs/brain-advanced-design.md` §10.1 — external CLI backend
    cross-links to the agent-roster guide.

**Validation (final).**

- `cargo clippy --all-targets -- -D warnings` — **clean**.
- `cargo test --all-targets` — **640 / 640 pass** (+41 new).
- `npm run build` — ok (5.8 s).
- `npm run test` — **957 / 957 pass** (+9 new).

**Files added.**

```
src-tauri/src/agents/mod.rs
src-tauri/src/agents/roster.rs
src-tauri/src/agents/cli_worker.rs
src-tauri/src/workflows/mod.rs
src-tauri/src/workflows/engine.rs
src-tauri/src/brain/ram_budget.rs
src-tauri/src/commands/agents_roster.rs
src/stores/agent-roster.ts
src/stores/agent-roster.test.ts
instructions/AGENT-ROSTER.md
```

**Files modified.**

```
src-tauri/src/lib.rs                       (modules + AppState + handler registration)
src-tauri/src/brain/mod.rs                  (added ram_budget submodule)
src-tauri/src/commands/mod.rs               (added agents_roster submodule)
docs/brain-advanced-design.md               (§10.1 ExternalCli cross-link)
rules/milestones.md                         (archived Chunk 1.5)
```

---

## Chunk 1.4 — Podman + Docker Desktop Dual Container Runtime

**Date:** 2026-04-23

**Goal.** Allow the local-LLM setup quest to work on machines that — for
company-compliance reasons — cannot install Docker Desktop but do have
Podman, while preserving today's behaviour for users with Docker.

**Architecture.** New `src-tauri/src/container/` module with:
- `ContainerRuntime { Docker, Podman }` enum with `binary()` / `label()`.
- `RuntimePreference { Auto, Docker, Podman }` (default `Auto`),
  serde-persisted in `AppSettings.preferred_container_runtime`.
- `detect_runtimes()` returns a `RuntimeDetection` struct with both CLI
  presence + daemon health flags, an `auto_pick`, and `both_available`
  for the UI to show a one-time picker.
- `resolve_runtime(preference)` returns either the explicit choice or
  the auto-pick, with a clear install hint when neither is found.

`src-tauri/src/brain/docker_ollama.rs` was refactored: every Docker CLI
invocation now goes through a `bin: &str` parameter via new `_for` /
`_with` variants (`check_ollama_container_for`, `ensure_ollama_container_for`,
`docker_pull_model_for`, `auto_setup_local_llm_with`). The legacy
`auto_setup_local_llm`/`docker_pull_model`/etc. delegate with
`ContainerRuntime::Docker` so existing callers and tests remain valid.

`commands/docker.rs` exposes new Tauri commands:
`detect_container_runtimes`, `get_runtime_preference`,
`set_runtime_preference`, `auto_setup_local_llm_with_runtime`. The
existing `auto_setup_local_llm` reads the persisted preference first and
forwards.

**Files created.**
- `src-tauri/src/container/mod.rs` (235 lines, 7 unit tests)

**Files modified.**
- `src-tauri/src/lib.rs` — register `container` module + new commands
- `src-tauri/src/brain/docker_ollama.rs` — refactor to runtime-parameterised, add 4 new tests
- `src-tauri/src/commands/docker.rs` — 4 new commands, persist preference
- `src-tauri/src/settings/mod.rs` — `preferred_container_runtime` field, default `Auto`
- `src-tauri/src/settings/config_store.rs` — struct literals updated

**Validation.**
- `cargo clippy --all-targets -- -D warnings` ✓ clean
- `cargo test --all-targets` → **594/594** pass (was 583)

---

## Chunk 1.2 — Mac & Linux CI Matrix + Platform Docs

**Date:** 2026-04-23

**Goal.** Catch macOS and Windows-specific Rust regressions automatically
and document the build/install story for non-Windows users.

**What shipped.**
- New `cross-platform-rust` job in `.github/workflows/terransoul-ci.yml`
  that runs `cargo check --all-targets` and `cargo test --lib --no-fail-fast`
  on `macos-latest` and `windows-latest` for every push. Uses
  `actions/cache@v4` keyed on `Cargo.lock`. Matrix uses `fail-fast: false`
  so a macOS failure doesn't cancel the Windows run (and vice-versa).
- New `instructions/PLATFORM-SUPPORT.md` documenting:
  - Per-OS install paths (`.msi`/`.dmg`/`.deb`/`.rpm`/`.AppImage`).
  - Source-build prerequisites with the exact `apt` command.
  - The platform-specific code map (`docker_ollama.rs`,
    `commands/window.rs`, `commands/user_models.rs`).
  - Known gaps tracked under future work (notarisation, repo publishing).

**Out of scope (intentionally deferred).**
- Full Tauri bundle smoke tests on macOS / Windows runners (need signing
  certs, would 4× the CI minutes).
- macOS notarisation pipeline.
- iOS / Android targets.

**Validation.**
- Workflow YAML linted by GitHub on push.
- Existing Linux build-and-test job is unchanged (no regression risk).

---

## Chunk 1.3 — Per-User VRM Model Persistence + Remove GENSHIN Default

**Date:** 2026-04-23

**Goal.** (1) Stop bundling the GENSHIN VRM (and its thumbnail) so the
repository ships only the two truly-original characters. (2) Persist
imported VRMs in the OS-specific user data folder so they survive
re-installs and fresh builds.

**What shipped.**
- Removed `genshin` from `src/config/default-models.ts`; deleted
  `public/models/default/2250278607152806301.vrm` and
  `public/models/default/GENSHIN.png`; updated all touching tests.
- `AppSettings.user_models: Vec<UserModel>` (forward-compatible via
  `#[serde(default)]` — no schema bump required, existing v2 settings
  files load unchanged).
- New `src-tauri/src/commands/user_models.rs` with five Tauri commands:
  `import_user_model`, `list_user_models`, `delete_user_model`,
  `read_user_model_bytes`, `update_user_model`. Files stored under
  `<app_data_dir>/user_models/<uuid>.vrm`. 256 MiB cap; ID restricted
  to `[A-Za-z0-9-]` to prevent path traversal.
- Frontend `useCharacterStore` extended (`userModels`, `allModels`,
  `loadUserModels`, `importUserModel`, `deleteUserModel`). User models
  are loaded as bytes and wrapped in a `Blob` URL — no asset-protocol
  scope change needed.
- `ModelPanel.vue` rewritten with bundled vs imported `<optgroup>`,
  per-card delete (`×`) button, and a persistence hint.
- `ChatView.vue` startup loads user models before restoring
  `selected_model_id`, so a previously selected imported VRM resurrects
  on launch.

**Per-user storage paths.**

| OS | Path |
|---|---|
| Windows | `%APPDATA%\com.terranes.terransoul\user_models\` |
| macOS | `~/Library/Application Support/com.terranes.terransoul/user_models/` |
| Linux | `~/.local/share/com.terranes.terransoul/user_models/` |

**Validation.**
- 8 new Rust tests + 7 new TS tests for the user-model flow.
- `cargo clippy` ✓; `cargo test --all-targets` → 583/583; `npm run test`
  → 948/948; `npm run build` ✓.

---

## Chunk 1.1 — Brain Advanced Design: Source Tracking Pipeline

**Date:** 2026-04-22
**Phase:** Phase 12 — Brain Advanced Design (Documentation & QA)

### Goal

Wire `source_url` and `source_hash` through the full ingest pipeline so the V3 schema columns (added by migration but previously unused) are actually populated. This enables the staleness detection and re-ingest skip/replace flows described in `docs/brain-advanced-design.md` §12.

### Architecture

- **NewMemory** struct extended with `source_url: Option<String>`, `source_hash: Option<String>`, `expires_at: Option<i64>`.
- **MemoryEntry** struct extended with the same 3 fields, read from DB.
- **`add()` / `add_to_tier()`** SQL now writes all 3 source columns.
- **All 7 SELECT queries** + both row mappers updated to read the 3 new columns.
- **New store methods**: `find_by_source_hash()`, `find_by_source_url()`, `delete_by_source_url()`, `delete_expired()`.
- **Ingest pipeline** (`commands/ingest.rs::run_ingest_task`): computes SHA-256 of fetched content, checks for existing hash (skip if unchanged), deletes stale entries by URL on content change, passes `source_url` + `source_hash` to each chunk's `NewMemory`.
- **Dependencies added**: `sha2 0.10`, `hex 0.4` (per coding standards: use existing crates).

### Files Modified

| File | Changes |
|------|---------|
| `src-tauri/Cargo.toml` | Added `sha2`, `hex` dependencies |
| `src-tauri/src/memory/store.rs` | Extended `NewMemory` (+ `Default`), `MemoryEntry`, `MemoryType` (+ `Default`); updated `add()`, `add_to_tier()`, all SELECTs, both row mappers; added 4 new methods + 9 new tests |
| `src-tauri/src/memory/brain_memory.rs` | Added `..Default::default()` to 4 `NewMemory` constructions |
| `src-tauri/src/commands/ingest.rs` | SHA-256 hashing, source dedup check, stale deletion, source fields in `NewMemory`; added 2 hash tests |
| `src-tauri/src/commands/memory.rs` | Added `..Default::default()` to `NewMemory` construction |

### Tests

- **Rust**: 570 passed (+9 new), 0 failed.
- **Frontend (Vitest)**: 941 passed, 0 failed.
- **New tests**: `add_with_source_fields`, `find_by_source_hash_returns_match`, `find_by_source_url_returns_all`, `delete_by_source_url_removes_all`, `reingest_skip_when_hash_unchanged`, `reingest_replaces_when_hash_changed`, `delete_expired_removes_past_entries`, `sha256_hash_deterministic`, `sha256_hash_changes_with_content`.

---

## Chunks 130–134 — Phase 11 Finale: RPG Brain Configuration

**Date:** 2026-04-20
**Phase:** Phase 11 — RPG Brain Configuration (final)

Five tightly-coupled chunks shipped together so the UI/UX stays coherent and free of overlapping floating surfaces.

### Shared foundations
- **`src/utils/stats.ts`** — single source of truth for the six RPG stats (Intelligence, Wisdom, Charisma, Perception, Dexterity, Endurance). Pure `computeStat(stat, activeSkills)` / `computeStats(activeSkills)` / `diffStats(before, after)` functions; per-stat skill-weight map; baseline 5; clamped to `[0, 100]`.
- **`src/utils/stat-modifiers.ts`** — pure stat → behaviour-knob translation (`getMemoryRecallLimit`, `getContextWindowMultiplier`, `getChatHistoryLimit`, `getHotwordSensitivity`, `getTtsExpressiveness`, plus a single-call `computeModifiers`).
- **`QuestTrackerData`** schema extended with `seenComboKeys: string[]` and `lastSeenActivationTimestamp: number` (with migration + merge logic + persistence) and exposed via two new store actions: `markCombosSeen(keys)` and `setLastSeenActivationTimestamp(ts)`.

### Chunk 130 — Brain RPG Stat Sheet
- New **`src/components/BrainStatSheet.vue`** — animated 6-bar panel themed in FF-style (gold "BRAIN STAT SHEET" heading, Lv. badge, per-stat icon + 3-letter abbr + bar with shimmer + numeric value + description). Stats are reactive to `skillTree.getSkillStatus`; when a stat increases, the bar pulses for 1.5s.
- Embedded inside `SkillTreeView.vue` between the progress header and the daily-quests banner — does NOT overlap the floating QuestBubble orb (orb is right edge, sheet is centred max-800).

### Chunk 131 — Combo Notification Toast
- New **`src/components/ComboToast.vue`** — slide-in toast queue with sparkling burst animation. Mounted in `App.vue` (only in non-pet mode). Anchored bottom-left so it never collides with the QuestBubble orb on the right. Watches `skillTree.activeCombos`; new combos that aren't in `tracker.seenComboKeys` are pushed onto the queue, marked seen, and auto-dismiss after 6s. On mobile, anchored above the bottom nav (bottom: 64px).

### Chunk 132 — Quest Reward Ceremony
- New **`src/components/QuestRewardCeremony.vue`** — full-screen modal teleported to `body` with a radial gradient + particle-burst background and a centred "QUEST COMPLETE" card. Card shows: quest icon + name + tagline, a per-stat row with `before → after (+delta)` and animated bar, the rewards list, and any newly-unlocked combos.
- Mounted in `App.vue`. Watches `skillTree.tracker.activationTimestamps`; on first launch establishes a high-water mark so the user isn't blasted with retroactive ceremonies for already-active skills. New activations above the mark are queued and shown one at a time.
- Auto-dismisses after 8s; `Continue ▸` button or backdrop click dismisses immediately. On dismiss, `setLastSeenActivationTimestamp` is called so each ceremony only fires once.

### Chunk 133 — Brain Evolution Path (neural pathway)
- CSS-only enhancement to `SkillConstellation.vue`: brain-cluster edges now render as glowing red neural pathways. Active edges get `stroke-dasharray: 6 6` plus a `stroke-dashoffset` animation (`sc-neural-flow`, 2.4s linear infinite) so signals visibly flow along completed prerequisite paths. Locked brain nodes are desaturated/dimmed; active brain nodes get a coral inner-glow. Other clusters retain their previous cleaner constellation look.

### Chunk 134 — Stat-Based AI Scaling
- `BrainStatSheet.vue` includes a live **"⚙ Active Modifiers"** panel that reads `computeModifiers(stats)` and renders the four scalable behaviours so users can SEE the stats actually changing AI behaviour: memory recall depth, chat history kept, hotword sensitivity, TTS expressiveness.
- `stat-modifiers.ts` is pure & exported, ready for downstream consumption (memory store, ASR detector, TTS adapter) without breaking existing call-sites — defaults are unchanged for a fresh install.

### Files
**Created:**
- `src/utils/stats.ts` + `src/utils/stats.test.ts` (9 tests)
- `src/utils/stat-modifiers.ts` + `src/utils/stat-modifiers.test.ts` (6 tests)
- `src/components/BrainStatSheet.vue` + `src/components/BrainStatSheet.test.ts` (5 tests)
- `src/components/ComboToast.vue` + `src/components/ComboToast.test.ts` (4 tests)
- `src/components/QuestRewardCeremony.vue` + `src/components/QuestRewardCeremony.test.ts` (4 tests)

**Modified:**
- `src/stores/skill-tree.ts` — extended `QuestTrackerData` with `seenComboKeys` + `lastSeenActivationTimestamp`, added `markCombosSeen` / `setLastSeenActivationTimestamp` actions, updated `freshTracker` / `migrateTracker` / `mergeTrackers`.
- `src/stores/skill-tree.test.ts` — extended fixtures with the two new fields.
- `src/views/SkillTreeView.vue` — embedded `<BrainStatSheet />`.
- `src/App.vue` — mounted `<ComboToast />` and `<QuestRewardCeremony />` in normal-mode only.
- `src/components/SkillConstellation.vue` — added neural-pathway CSS for the brain cluster.
- `rules/milestones.md` — drained Phase 11 chunks.

### Verification
- `npm run build` → ✓ built in 5.47s (vue-tsc + vite)
- `npm run test` → **58 files, 925 tests passing** (baseline 53/897 → +5 files, +28 tests, no regressions)
- `npm run test:e2e e2e/desktop-flow.spec.ts` → **passed** (full end-to-end app flow: app load, brain/voice auto-config, send message, get response, subtitle, 3D model, BGM, marketplace nav, LLM switch, quest system)
- `npm run test:e2e e2e/mobile-flow.spec.ts` → **passed**
- A dedicated visual-coexistence Playwright test confirmed bounding boxes for `BrainStatSheet`, `ComboToast`, `QuestBubble` orb, and `SkillConstellation` overlay never overlap horizontally + vertically simultaneously, and the constellation Esc-close path leaves the stat sheet visible.
- `parallel_validation` (Code Review + CodeQL) — **0 issues**.

---

## Chunk 128 — Constellation Skill Tree (Full-Screen Layout)

**Date:** 2026-04-20
**Phase:** Phase 11 — RPG Brain Configuration

**Goal:** Replace the 360px CSS grid panel inside `QuestBubble.vue` with a full-screen Abilities-style constellation map. Each of the five categories (Brain, Voice, Avatar, Social, Utility) becomes a circular cluster of nodes laid out radially with concentric rings, glowing connection lines, a colored diamond border, and a star-field background. Pannable + zoomable.

**Architecture:**
- **`SkillConstellation.vue`** — new full-screen overlay teleported to `body`. World canvas of 1600×1200 with five `ClusterMeta` placements arranged in a pentagon. Each cluster renders:
  - SVG diamond border + concentric dashed rings (`foundation` r=90, `advanced` r=155, `ultimate` r=220).
  - Center emblem button (icon + label + `activeCount/total AP`).
  - Skill nodes positioned by polar coordinates: `angle = 2π * i / count` per ring with a tier-staggered offset.
  - Per-cluster SVG `<line>` edges for in-cluster prerequisite chains; `--active` class brightens edges where both endpoints are unlocked.
  - CSS custom properties (`--cluster-color`, `--cluster-glow`) drive theme: Brain crimson, Voice jade, Avatar gold, Social sapphire, Utility amethyst.
- **Star-field** — three layered animated CSS backgrounds (`sc-stars-1/2/3`) with drift + twinkle keyframes plus a blurred nebula gradient.
- **Pan / zoom** — `transform: translate(...) scale(...)` on `.sc-world`. Anchor-aware mouse-wheel zoom (cursor stays under the same world point), drag-to-pan via `mousedown/move/up`, single-finger pan + two-finger pinch-zoom for touch. Scale clamped to `[0.35, 2.5]`. Reset/zoom-in/zoom-out buttons in the corner.
- **`fitInitial()`** computes the initial fit-to-viewport scale & offset; `ResizeObserver` keeps the viewport size live.
- **QuestBubble.vue** — drastically simplified (1046 → ~290 lines): orb is preserved with its progress ring and percentage, but clicking it now toggles the constellation overlay. The 360px `.ff-panel`, tabs, grid, detail pane, transitions, and ~600 lines of CSS were removed. AI quest sorting (`sortQuestsWithAI`) is preserved for downstream consumers.

**Files created:**
- `src/components/SkillConstellation.vue` (~1100 lines incl. styles)
- `src/components/SkillConstellation.test.ts` (15 tests)

**Files modified:**
- `src/components/QuestBubble.vue` — replaced `.ff-panel` + grid + detail with `<SkillConstellation>`; orb behaviour preserved
- `src/components/QuestBubble.test.ts` — rewritten for the new constellation-based wiring (13 tests)
- `rules/milestones.md` — removed Chunk 128 row, updated `Next Chunk` pointer
- `rules/completion-log.md` — this entry

**Test counts:** 53 test files, 897 Vitest tests passing locally (`npm run test`). `npm run build` passes (`vue-tsc && vite build`).

---

## Chunk 129 — Constellation Cluster Interaction & Detail Panel

**Date:** 2026-04-20
**Phase:** Phase 11 — RPG Brain Configuration

**Goal:** Make the constellation interactive — click a cluster to zoom into it, click a node to open a quest detail overlay (objectives, rewards, prerequisites), provide breadcrumb navigation, a back button, and a corner minimap with status dots.

**Architecture (delivered together with Chunk 128):**
- **Cluster zoom-in** — `zoomToCluster(id)` animates `tx/ty/scale` so the cluster centre is recentred at scale `1.6`; `animating` toggles a 450ms cubic-bezier CSS transition on `.sc-world`. Selecting a node in another cluster auto-focuses that cluster first.
- **Detail overlay** — `.sc-detail` panel reuses the same content blocks as the legacy `.ff-detail`: tagline, description, objectives (with `▸` Go buttons that emit `navigate`), rewards, prerequisites (with `◆/◇` met/unmet markers), Pin/Unpin and Begin Quest actions. The Begin button is suppressed for `locked` nodes. Cluster-coloured border via `.sc-detail--{cluster}` modifiers.
- **Breadcrumb** — top bar shows `✦ All Clusters › {Cluster} › {Quest}` reflecting current focus depth; each crumb segment is independently clickable.
- **Back button** — appears whenever a cluster or node is focused. Pops state in order `detail → cluster → home`. `Esc` mirrors the same behaviour, falling through to `emit('close')` from the home view.
- **Minimap** — fixed 180×135 SVG bottom-left mirroring the world coords, showing cluster outlines (per-cluster stroke colour), per-node dots tinted by status (`locked`/`available`/`active`), inter-cluster constellation lines, and a dashed yellow viewport rectangle that updates from `tx/ty/scale`.
- **`QuestBubble.vue` integration** — `@begin` from `SkillConstellation` flows into the existing `QuestConfirmationDialog`, which on accept calls `skillTree.triggerQuestEvent(...)`, emits `trigger`, and re-runs `sortQuestsWithAI()`. `@navigate` is forwarded so existing tab routing (`brain-setup`, `voice`, etc.) still works. `@close` simply hides the overlay.

**Files modified / created:** Same as Chunk 128 above (the layout and the interactions ship as one component).

**Test counts:** Unchanged — 53 files, 897 Vitest tests. New tests covering 129 specifically include `zooms into a cluster and updates the breadcrumb`, `opens the detail overlay when a node is clicked`, `emits begin when the Begin Quest button is clicked`, `does not show Begin Quest for locked nodes`, `emits navigate when a step Go button is clicked`, `back button steps from detail → cluster → all clusters`, and `pin/unpin actions delegate to the store`.

---

## Post-Phase — 3D Model Loading Robustness

**Date:** 2026-04-18
**Phase:** Post-phase polish

**Goal:** Fix 3D VRM model failing to load silently, show error UI, and add placeholder fallback.

**Architecture:**
- **URL encoding** — `loadVRM()` in `vrm-loader.ts` now applies `encodeURI()` to HTTP paths (preserving blob:/data: URLs) before passing to Three.js `GLTFLoader`, fixing models with spaces in filenames (e.g. "Annabelle the Sorcerer.vrm").
- **Error overlay** — `CharacterViewport.vue` template now renders `characterStore.loadError` in a visible overlay with ⚠️ icon and a "Retry" button when VRM loading fails.
- **Placeholder fallback** — On `loadVRMSafe` returning null, `createPlaceholderCharacter()` is called to add a simple geometric figure to the scene so it's not empty.
- **Retry action** — `retryModelLoad()` re-triggers `selectModel()` on the current selection.

**Files modified:**
- `src/renderer/vrm-loader.ts` — encodeURI for HTTP paths
- `src/components/CharacterViewport.vue` — error overlay, placeholder fallback, retry button, imported `createPlaceholderCharacter`

**Files tested:**
- `src/renderer/vrm-loader.test.ts` — 4 new tests (placeholder creation, URL encoding)
- `src/stores/character.test.ts` — 3 new tests (error state management)
- `src/config/default-models.test.ts` — 5 new tests (path validation, encoding, uniqueness)

**Test counts:** 893 total tests passing (52 test files)

---

## Post-Phase — Streaming Timeout Fix (Stuck Thinking)

**Date:** 2026-04-18
**Phase:** Post-phase polish

**Goal:** Prevent chat from getting permanently stuck in "Thinking" state when streaming or backend calls hang.

**Architecture:**
- **Tauri streaming timeout** — `conversation.ts` wraps `streaming.sendStreaming()` in `Promise.race` with 60s timeout
- **Fallback invoke timeout** — `invoke('send_message')` wrapped in `Promise.race` with 30s timeout
- **Grace period reduced** — 3s → 1.5s for stream completion grace period
- **Finally cleanup** — `finally` block resets `isStreaming` and `streamingText` in addition to `isThinking`

**Files modified:**
- `src/stores/conversation.ts` — timeout wrappers on both streaming paths

**Test counts:** 893 total tests passing (52 test files)

---

## Post-Phase — Music Bar Redesign (Always-Visible Play/Stop)

**Date:** 2026-04-18
**Phase:** Post-phase polish

**Goal:** Make BGM play/stop button always visible without expanding the track selector panel.

**Architecture:**
- Split old single toggle into two buttons: `.music-bar-play` (▶️/⏸ always visible) and `.music-bar-expand` (🎵/◀ for track controls)
- Updated mobile responsive CSS for both buttons

**Files modified:**
- `src/components/CharacterViewport.vue` — music bar template + CSS

**Test counts:** 893 total tests passing (52 test files)

---

## Post-Phase — Splash Screen

**Date:** 2026-04-18
**Phase:** Post-phase polish

**Goal:** Show a cute animated loading screen during app initialization instead of a blank screen.

**Architecture:**
- **`SplashScreen.vue`** — CSS-animated kawaii cat with bouncing, blinking eyes, waving paws, sparkle effects, "TerranSoul..." text
- **`App.vue` integration** — `appLoading` ref starts true, shows splash during init, fades out with transition when ready

**Files created:**
- `src/components/SplashScreen.vue`

**Files modified:**
- `src/App.vue` — appLoading state, SplashScreen import, v-show gating

**Test counts:** 893 total tests passing (52 test files)

---

## Post-Phase — BGM Track Replacement (JRPG-Style)

**Date:** 2026-04-18
**Phase:** Post-phase polish

**Goal:** Replace placeholder BGM tracks with original JRPG-style synthesized compositions. 40-second loops with multi-tap reverb, resonant filters, plucked string models, and soft limiter.

**Tracks:**
- **Crystal Theme** (prelude.wav) — Harp arpeggios in C major pentatonic
- **Starlit Village** (moonflow.wav) — Acoustic town theme with warm pad and plucked melody
- **Eternity** (sanctuary.wav) — Save-point ambience with ethereal pad and bell tones

**Files modified:**
- `scripts/generate-bgm.cjs` — complete rewrite with new synthesis engine
- `src/composables/useBgmPlayer.ts` — updated track display names
- `src/stores/skill-tree.ts` — updated BGM quest description

**Test counts:** 893 total tests passing (52 test files)

---

## Chunk 126 — On-demand Rendering + Idle Optimization

**Date:** 2026-04-18
**Phase:** 10 — Avatar Animation Architecture (VRM Expression-Driven)

**Goal:** Reduce GPU/CPU load when avatar is idle by throttling render rate to ~15 FPS when animation is settled, restoring 60 FPS on any state change.

**Architecture:**
- **`CharacterAnimator.isAnimationSettled(epsilon)`** — checks `AvatarStateMachine.isSettled()`, then iterates all EXPR_COUNT expression channels and all bone channels, comparing current vs target within epsilon (default 0.002).
- **Frame-skip logic in `CharacterViewport.vue`** render loop — tracks `idleAccum` elapsed time. When `isAnimationSettled() && body==='idle' && !needsRender`, accumulates delta and skips render if < 66ms (IDLE_INTERVAL = 1/15). On any active state, resets accumulator and renders every frame.
- **`needsRender` one-shot flag** — cleared after each render frame, used for immediate wake-up on state mutations.

**Files modified:**
- `src/renderer/character-animator.ts` — added `isAnimationSettled()` method
- `src/components/CharacterViewport.vue` — added frame-skip logic with `IDLE_INTERVAL` and `idleAccum`

**Files tested:**
- `src/renderer/character-animator.test.ts` — 5 new tests (settled after convergence, false after state change, false with active visemes, false when not idle, custom epsilon)

**Test counts:**
- 5 new Vitest tests (38 total in character-animator.test.ts)
- 668 total tests passing (46 test files)

---

## Chunk 125 — LipSync ↔ TTS Audio Pipeline

**Date:** 2026-04-18
**Phase:** 10 — Avatar Animation Architecture (VRM Expression-Driven)

**Goal:** Wire TTS audio playback into LipSync engine, feeding 5-channel viseme values into AvatarStateMachine for real-time lip animation.

**Architecture:**
- **`useTtsPlayback` callback hooks** — 3 new lifecycle hooks:
  - `onAudioStart(cb)` — fires with `HTMLAudioElement` before `play()`, enabling `MediaElementAudioSourceNode` creation
  - `onAudioEnd(cb)` — fires on sentence `onended`/`onerror`
  - `onPlaybackStop(cb)` — fires on hard `stop()` call
- **`useLipSyncBridge` composable** — new bridge wiring TTS → LipSync → AvatarState:
  - Single shared `AudioContext` across TTS lifetime
  - `onAudioStart`: creates `MediaElementAudioSourceNode` → `AnalyserNode` → `LipSync.connectAnalyser()`
  - Per-frame `tick()` via rAF: reads `lipSync.getVisemeValues()` → `asm.setViseme()`
  - `onAudioEnd`/`onPlaybackStop`: cleans up source node, zeroes visemes
  - `start()`/`dispose()` lifecycle for mount/unmount
- **ChatView integration** — `lipSyncBridge.start()` in `onMounted`, `lipSyncBridge.dispose()` in `onUnmounted`

**Files created:**
- `src/composables/useLipSyncBridge.ts` — bridge composable
- `src/composables/useLipSyncBridge.test.ts` — 8 tests (callback registration, rAF loop, idempotent start, dispose cleanup, zero visemes on end/stop, null ASM safety, audio start safety)

**Files modified:**
- `src/composables/useTtsPlayback.ts` — added `TtsPlaybackHandle` interface extensions, callback fields, hook invocations
- `src/composables/useTtsPlayback.test.ts` — 4 new tests (onAudioStart, onAudioEnd, onPlaybackStop, optional callbacks)
- `src/views/ChatView.vue` — wired lipSyncBridge start/dispose

**Test counts:**
- 12 new Vitest tests (8 bridge + 4 TTS hooks)
- 668 total tests passing (46 test files)

---

## Chunk 124 — Decouple IPC from Animation — Coarse State Bridge

**Date:** 2026-04-18
**Phase:** 10 — Avatar Animation Architecture (VRM Expression-Driven)

**Goal:** Remove per-frame reactive state updates from streaming/IPC path. Bridge coarse body/emotion transitions through a single `setAvatarState()` function that updates both the Pinia store (for UI pill) and the AvatarStateMachine (for render loop).

**Architecture:**
- **`setAvatarState()` bridge** in `ChatView.vue` — updates `characterStore.setState(name)` (UI) AND `asm.forceBody()`/`asm.setEmotion()` (render loop) in one call
- **`getAsm()` accessor** — reads `CharacterViewport.defineExpose({ avatarStateMachine })` via template ref
- **All 5 `characterStore.setState()` calls** replaced with `setAvatarState()`: thinking (on send), talking (on first chunk), emotion (on stream done + parseTags), idle (on timeout)
- **TTS watcher** — `watch(tts.isSpeaking)`: `true` → `setAvatarState('talking')`, `false` → `setAvatarState('idle')`
- **Emotion from streaming** — reads `streaming.currentEmotion` once when stream completes

**Files modified:**
- `src/components/CharacterViewport.vue` — added `defineExpose({ avatarStateMachine })` getter
- `src/views/ChatView.vue` — added `setAvatarState()`, `getAsm()`, replaced all setState calls, added TTS/emotion watchers

**Test counts:**
- No new tests (wiring-only changes in view components)
- 668 total tests passing (46 test files)

---

## Chunk 123 — Audio Analysis Web Worker

**Date:** 2026-04-17
**Phase:** 10 — Avatar Animation Architecture (VRM Expression-Driven)

**Goal:** Move FFT processing, RMS calculation, and frequency band extraction off the main thread into a Web Worker. LipSync class delegates to worker when available, falls back to main-thread analysis.

**Architecture:**
- **`src/workers/audio-analyzer.worker.ts`** — standalone worker with message protocol:
  - `analyze` message: receives `Float32Array` time-domain + `Uint8Array` frequency data, returns `{ volume, visemes: {aa,ih,ou,ee,oh} }`
  - `configure` message: updates silence threshold and sensitivity
- **Pure computation functions** exported for direct testing: `calculateRMS()`, `computeBandEnergies()`, `analyzeAudio()`
- **Worker integration in `LipSync`**:
  - `enableWorker()` — creates worker via `new URL()` + Vite module worker, sends initial config
  - `disableWorker()` — terminates worker, reverts to main-thread
  - `getVisemeValues()` — when worker ready: sends raw data off-thread (copies for transfer), returns last result immediately (non-blocking); when worker busy, returns cached last result; when no worker, falls back to synchronous main-thread FFT analysis
  - `disconnect()` — also tears down worker
- **Zero-copy transfer**: `Float32Array.buffer` transferred to worker; `Uint8Array` copied (small)
- **Graceful degradation**: if Worker constructor unavailable (SSR, old browser), stays on main thread

**Files created:**
- `src/workers/audio-analyzer.worker.ts` — worker + exported pure functions
- `src/workers/audio-analyzer.worker.test.ts` — 21 tests (RMS, band energies, analyzeAudio, message protocol types)

**Files modified:**
- `src/renderer/lip-sync.ts` — worker fields, `enableWorker()`, `disableWorker()`, worker delegation in `getVisemeValues()`
- `src/renderer/lip-sync.test.ts` — 4 new tests (workerReady default, enableWorker safety, disableWorker safety, disconnect cleanup)

**Test counts:**
- 25 new Vitest tests (21 worker + 4 lip-sync integration)
- 651 total tests passing (45 test files)

---

## Chunk 122 — 5-Channel VRM Viseme Lip Sync

**Date:** 2026-04-17
**Phase:** 10 — Avatar Animation Architecture (VRM Expression-Driven)

**Goal:** Extend `LipSync` class to produce 5 VRM visemes (`aa`, `ih`, `ou`, `ee`, `oh`) via FFT frequency-band analysis instead of just 2-channel `aa`/`oh`. Feed viseme values into `AvatarState.viseme` mutable ref. Keep backward-compatible 2-channel `getMouthValues()`.

**Architecture:**
- **5 frequency bands** mapped to VRM visemes: low (0–12% Nyquist) → `aa` (open jaw), mid-low (12–25%) → `ou` (round), mid (25–45%) → `oh` (half-round), mid-high (45–65%) → `ee` (spread), high (65–100%) → `ih` (narrow).
- **`getVisemeValues(): VisemeValues`** — new method using `getByteFrequencyData()` for FFT band analysis + `getFloatTimeDomainData()` for RMS volume gating.
- **`visemeValuesFromBands()`** — static factory for pre-computed band energies (Web Worker path in Chunk 123).
- **`VisemeValues`** type alias to `VisemeWeights` from `avatar-state.ts` — shared type between LipSync and AvatarState.
- **`frequencyData: Uint8Array`** — allocated alongside `timeDomainData` in `connectAudioElement()` and `connectAnalyser()`.
- **Backward compatible**: `getMouthValues()` still works as 2-channel fallback (RMS-based `aa`/`oh`).
- **`CharacterAnimator`** already reads `AvatarState.viseme` and damps at λ=18 (from Chunk 121).

**Files modified:**
- `src/renderer/lip-sync.ts` — added 5-channel FFT analysis, `getVisemeValues()`, `visemeValuesFromBands()`, `VisemeValues` type, `BAND_EDGES`, `computeBandEnergies()`
- `src/renderer/lip-sync.test.ts` — 9 new tests (getVisemeValues inactive, VisemeValues type, visemeValuesFromBands: clamping, zeroes, per-band mapping, sensitivity, negatives)

**Test counts:**
- 9 new Vitest tests (23 total in lip-sync.test.ts)
- 626 total tests passing (44 test files)

---

## Chunk 121 — Exponential Damping Render Loop

**Date:** 2026-04-17
**Phase:** 10 — Avatar Animation Architecture (VRM Expression-Driven)

**Goal:** Replace linear `smoothStep` interpolation in `CharacterAnimator` with proper exponential damping (`damp`). Replace `Map`-based expression/bone tracking with flat `Float64Array` typed arrays for zero-alloc frame loops. Integrate `AvatarStateMachine` for blink cycle and viseme reading. Apply per-channel damping rates: λ=8 emotions, λ=18 visemes, λ=25 blink, λ=6 bones.

**Architecture:**
- New `damp(current, target, lambda, delta)` function: `current + (target - current) * (1 - exp(-lambda * delta))` — frame-rate independent.
- 12-channel flat `Float64Array` for expressions: 6 emotions + 5 visemes + 1 blink, each with per-channel λ from `EXPR_LAMBDAS`.
- Flat `Float64Array` for bone rotations (7 bones × 3 components = 21 floats), damped at λ=6.
- `AvatarStateMachine` integrated: `setState(CharacterState)` bridges to body+emotion; blink delegated to `AvatarStateMachine.tickBlink()`.
- Public `avatarStateMachine` getter for external code to read/write layered state directly.
- All existing placeholder + VRM animation behavior preserved.

**Files modified:**
- `src/renderer/character-animator.ts` — replaced smoothStep→damp, Maps→Float64Arrays, added AvatarStateMachine, per-channel lambda damping
- `src/renderer/character-animator.test.ts` — 12 new tests (7 damp + 5 AvatarStateMachine integration)

**Test counts:**
- 12 new Vitest tests
- 617 total tests passing (44 test files)

---

## Chunk 120 — AvatarState Model + Animation State Machine

**Date:** 2026-04-17
**Phase:** 10 — Avatar Animation Architecture (VRM Expression-Driven)

**Goal:** Define a layered `AvatarState` type with body/emotion/viseme/blink/lookAt channels and an `AvatarStateMachine` class enforcing valid body transitions while keeping all other layers independent.

**Architecture:**
- `AvatarState` is a plain mutable object — NOT Vue reactive — for zero-overhead frame-loop reads.
- Body layer: `idle | listen | think | talk` with enforced transition graph (idle→listen→think→talk→idle; idle always reachable; talk→think allowed for re-think).
- Emotion layer: `neutral | happy | sad | angry | relaxed | surprised` — overlays any body state, always settable.
- Viseme layer: 5 VRM channels (`aa/ih/ou/ee/oh`, 0–1) — only applied when body=talk; auto-zeroed otherwise.
- Blink layer: self-running randomised cycle (2–6s intervals, 150ms duration); overridable for expressions like surprise.
- LookAt layer: normalised (x,y) gaze offset — independent of all other layers.
- `needsRender` flag set on any channel change for future on-demand rendering (Chunk 126).
- `isSettled()` method for idle detection.

**Files created:**
- `src/renderer/avatar-state.ts` — AvatarState type, AvatarStateMachine class, createAvatarState factory
- `src/renderer/avatar-state.test.ts` — 53 unit tests

**Test counts:**
- 53 new Vitest tests (body transitions, emotion, viseme, blink, lookAt, layer independence, reset, constructor)
- 605 total tests passing (44 test files)

---

## Chunk 110 — Background Music

**Date:** 2026-04-15
**Status:** ✅ Done

### Goal
Add ambient background music to the 3D character viewport. Procedurally generated audio tracks
using the Web Audio API — no external audio files needed. Users can toggle BGM on/off, choose
from 3 ambient presets, and adjust volume. Settings are persisted between sessions.

### Architecture
- **`useBgmPlayer` composable** — procedural ambient audio via `OscillatorNode`, `BiquadFilterNode`,
  and noise buffers. Three preset tracks: Calm Ambience (C major pad), Night Breeze (A minor pad),
  Cosmic Drift (deep F drone + high shimmer). Master gain with `linearRampToValueAtTime` for 1.5s
  fade-in/fade-out transitions.
- **`AppSettings` schema v2** — added `bgm_enabled` (bool), `bgm_volume` (f32, 0.0–1.0),
  `bgm_track_id` (string). Rust `#[serde(default)]` ensures backward compatibility.
- **Settings persistence** — `saveBgmState()` convenience method on `useSettingsStore`.
  BGM state restored from settings on `CharacterViewport` mount.
- **UI controls** — toggle switch, track selector dropdown, volume slider. All in the existing
  settings dropdown in `CharacterViewport.vue`.

### Files Created
- `src/composables/useBgmPlayer.ts` — composable (225 lines)
- `src/composables/useBgmPlayer.test.ts` — 10 Vitest tests (Web Audio mock)

### Files Modified
- `src-tauri/src/settings/mod.rs` — `AppSettings` v2 with BGM fields + 2 new Rust tests
- `src-tauri/src/settings/config_store.rs` — no changes (serde defaults handle migration)
- `src/stores/settings.ts` — `AppSettings` interface + `saveBgmState()` + default schema v2
- `src/stores/settings.test.ts` — updated defaults test + new `saveBgmState` test
- `src/components/CharacterViewport.vue` — BGM toggle/selector/slider UI + restore on mount + cleanup on unmount

### Test Counts
- **Vitest tests added:** 11 (10 BGM + 1 saveBgmState)
- **Rust tests added:** 2 (default_bgm_settings, serde_fills_bgm_defaults_when_missing)
- **Total Vitest:** 417 (34 files, all pass)
- **Build:** `npm run build` ✅ clean

---

## Chunk 109 — Idle Action Sequences

**Date:** 2026-04-15
**Status:** ✅ Done

### Goal
Make the character feel alive when the user is away. After a period of silence the character
initiates conversation with a natural greeting, cycling through variants so it never feels robotic.

### Architecture
- **`useIdleManager` composable** — timeout-based idle detection. Uses `setTimeout` chain (not `setInterval`)
  to avoid drift. Exposes `start`, `stop`, `resetIdle` lifecycle methods and reactive `isIdle`.
- **`IDLE_TIMEOUT_MS = 45_000`** — first greeting fires 45 seconds after last user activity.
- **`IDLE_REPEAT_MS = 90_000`** — repeat gap between subsequent greetings.
- **5 greeting variants** in `IDLE_GREETINGS`, shuffled and cycled in round-robin before repeating.
- **`isBlocked` guard** — callback checked before firing; blocked when `conversationStore.isThinking`
  or `conversationStore.isStreaming` to avoid interrupting an active AI response.
- **ChatView.vue wiring** — `idle.start()` on `onMounted`, `idle.stop()` on `onUnmounted`,
  `idle.resetIdle()` at the top of `handleSend`.

### Files Created
- `src/composables/useIdleManager.ts` — composable (95 lines)
- `src/composables/useIdleManager.test.ts` — 10 Vitest tests (fake timers)

### Files Modified
- `src/views/ChatView.vue` — import + instantiate `useIdleManager`; wire start/stop/reset

### Test Counts
- **Vitest tests added:** 10 (initial state, timeout, greeting content, repeat, reset, stop, block, round-robin)
- **Total Vitest:** 406 (33 files, all pass)
- **Build:** `npm run build` ✅ clean

---

## Chunk 108 — Settings Persistence + Env Overrides

**Date:** 2026-04-15
**Status:** ✅ Done

### Goal
Persist user preferences between sessions so TerranSoul "remembers" the character model and
camera orientation. Support `.env` override for dev/CI without touching user config files.

### Architecture
- **Rust: `settings` module** — `AppSettings` struct (version, selected_model_id, camera_azimuth,
  camera_distance). JSON persistence via `settings/config_store.rs` following voice/brain patterns.
  Schema validation: stale/corrupt files silently replaced with defaults.
- **Rust: `.env` override** — `TERRANSOUL_MODEL_ID` env var overrides `selected_model_id` at load time.
  Non-secrets only; API keys remain user-configured.
- **Rust: Tauri commands** — `get_app_settings`, `save_app_settings` in `commands/settings.rs`.
- **AppState** — `app_settings: Mutex<settings::AppSettings>` field.
- **`useSettingsStore`** — Pinia store with `loadSettings`, `saveSettings`, `saveModelId`,
  `saveCameraState` convenience helpers. Falls back silently when Tauri unavailable.
- **Model persistence** — `characterStore.selectModel()` calls `settingsStore.saveModelId()`.
- **Camera persistence** — `scene.ts` exports `onCameraChange(cb)` callback (fired on OrbitControls
  `end` event with spherical azimuth + radius). `CharacterViewport.vue` registers callback → saves.
- **Camera restore** — `CharacterViewport.vue` restores camera position from settings on mount.
- **App start** — `ChatView.vue` `onMounted` loads settings and selects persisted model if different
  from default.

### Files Created
- `src-tauri/src/settings/mod.rs` — AppSettings struct + env override + schema validation (120 lines)
- `src-tauri/src/settings/config_store.rs` — JSON load/save + 6 tests (115 lines)
- `src-tauri/src/commands/settings.rs` — `get_app_settings` + `save_app_settings` + 3 tests
- `src/stores/settings.ts` — `useSettingsStore` Pinia store
- `src/stores/settings.test.ts` — 9 Vitest tests

### Files Modified
- `src-tauri/src/commands/mod.rs` — added `settings` module
- `src-tauri/src/lib.rs` — settings module, AppState field, commands registered
- `src/stores/character.ts` — `selectModel` persists via `settingsStore.saveModelId`
- `src/components/CharacterViewport.vue` — `onCameraChange` wired, camera restored from settings
- `src/views/ChatView.vue` — load settings + restore persisted model on mount
- `src/renderer/scene.ts` — `onCameraChange(cb)` API added to `SceneContext`

### Test Counts
- **Rust tests added:** 11 (schema validation × 6 in mod.rs, config_store × 5, command tests × 3)
- **Vitest tests added:** 9 (useSettingsStore: defaults, load, save, patch, helpers, error resilience)
- **Total Vitest:** 396 (32 files, all pass)
- **Build:** `npm run build` ✅ clean

---

## Chunk 107 — Multi-ASR Provider Abstraction

**Date:** 2026-04-15
**Status:** ✅ Done

### Goal
Abstract speech recognition into a provider-agnostic factory so users can choose between
browser Web Speech API (zero setup), OpenAI Whisper (best quality), and Groq Whisper (fastest, free tier).

### Architecture
- **Rust: `groq-whisper`** added to `asr_providers()` catalogue in `voice/mod.rs`.
- **Rust: `float32_to_pcm16`** helper in `commands/voice.rs` converts VAD float32 samples to int16 PCM.
- **Rust: `transcribe_audio` command** — accepts `Vec<f32>` samples, converts to PCM-16, routes to
  stub / whisper-api / groq-whisper (OpenAI-compatible endpoint). `web-speech` returns helpful error.
- **`useAsrManager` composable** — provider factory: `web-speech` uses browser `SpeechRecognition`;
  all Rust-backed providers go through VAD → `transcribe_audio` IPC. `isListening`, `error` reactive state.
- **Mic button in ChatView.vue** — shown only when `voice.config.asr_provider` is set. Pulsing red
  animation while listening. `toggleMic()` wired to `asr.startListening/stopListening`.
- **Groq mode in VoiceSetupView.vue** — new tier card ("⚡ Groq (fast)"), dedicated config step
  with Groq API key input, done screen updated.
- **Bug fix:** `useTtsPlayback.ts` `Blob([bytes.buffer])` for correct BlobPart type.

### Files Created
- `src/composables/useAsrManager.ts` — provider factory composable (185 lines)
- `src/composables/useAsrManager.test.ts` — 13 Vitest tests

### Files Modified
- `src-tauri/src/voice/mod.rs` — added `groq-whisper` provider
- `src-tauri/src/commands/voice.rs` — `float32_to_pcm16`, `transcribe_audio` command, 8 Rust tests
- `src-tauri/src/lib.rs` — registered `transcribe_audio`
- `src/views/ChatView.vue` — `useAsrManager` import, `asr` instance, `toggleMic`, mic button CSS
- `src/views/VoiceSetupView.vue` — Groq tier + config step + groq activate function
- `src/composables/useTtsPlayback.ts` — `Blob([bytes.buffer])` fix
- `src/composables/useTtsPlayback.test.ts` — removed unused `afterEach` import

### Test Counts
- **Rust tests added:** 8 (float32_to_pcm16 × 2, transcribe_audio routing × 6)
- **Vitest tests added:** 13 (useAsrManager: routing × 3, transcript × 2, VAD+IPC × 5, stop/idle × 3)
- **Total Vitest:** 387 → 396 after chunk 108

---

## Chunk 106 — Streaming TTS

**Date:** 2026-04-15
**Status:** ✅ Done

### Goal
Replace the stub/batched TTS architecture with a real streaming pipeline. Voice synthesis begins
~200ms after the first LLM sentence completes — a major UX win over waiting for the full response.
Learned from VibeVoice realtime streaming pattern.

### Architecture
- **Rust: `synthesize_tts` Tauri command** — routes to configured TTS provider (edge-tts, stub).
  Takes `text: String`, returns `Vec<u8>` (WAV bytes). Empty text guard returns error.
- **`useTtsPlayback` composable** — sentence-boundary detection (`SENTENCE_END_RE`), synthesis
  queue (Promise chain), sequential HTMLAudioElement playback, stop/flush lifecycle API.
  `MIN_SENTENCE_CHARS = 4` filters stray punctuation. Blob URL cleanup on stop.
- **ChatView.vue wired**: `tts.stop()` on new message send, `tts.feedChunk()` per llm-chunk
  event, `tts.flush()` on stream done. Voice store initialized on mount. `tts.stop()` on unmount.

### Files Created
- `src/composables/useTtsPlayback.ts` — streaming TTS composable (160 lines)
- `src/composables/useTtsPlayback.test.ts` — 13 Vitest tests

### Files Modified
- `src-tauri/src/commands/voice.rs` — added `synthesize_tts` command + 4 Rust tests
- `src-tauri/src/lib.rs` — registered `synthesize_tts` in invoke handler
- `src/views/ChatView.vue` — import `useTtsPlayback` + `useVoiceStore`; wire tts.feedChunk/flush/stop; voice.initialise() on mount; tts.stop() on unmount

### Test Counts
- **Rust tests added:** 4 (synthesize_tts empty text guard, stub WAV bytes, no provider error, unknown provider error)
- **Vitest tests added:** 13 (sentence detection × 6, flush × 3, stop × 2, error handling × 1, isSpeaking × 1)
- **Total Vitest:** 374 (35 files, all pass)
- **Build:** `npx vite build` ✅ clean

---

## Chunk 001 — Project Scaffold

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Bootstrap the full TerranSoul Phase 1 project: Tauri 2.0 shell, Vue 3 + TypeScript frontend,
Rust backend with Tauri commands, Three.js scene, @pixiv/three-vrm VRM loader, Pinia stores,
all core Vue components, and a stub local agent.

### Architecture
- Tauri 2.0 with `tauri-plugin-shell`
- Vue 3 + TypeScript via Vite 6
- Three.js 0.175 + @pixiv/three-vrm 3.4
- Pinia 2.3 for state management
- Rust: `tokio`, `serde`, `serde_json`, `uuid`

### Files Created
**Frontend (src/)**
- `src/types/index.ts` — Message, CharacterState, Agent TypeScript interfaces
- `src/stores/conversation.ts` — Pinia store: messages, isThinking, sendMessage (Tauri IPC)
- `src/stores/character.ts` — Pinia store: CharacterState, vrmPath, setState, loadVrm
- `src/renderer/scene.ts` — Three.js WebGL2 renderer, camera, 3-point lighting, clock
- `src/renderer/vrm-loader.ts` — GLTFLoader + VRMLoaderPlugin; capsule fallback if no VRM
- `src/renderer/character-animator.ts` — State machine: idle/thinking/talking/happy/sad
- `src/components/AgentBadge.vue` — Agent name badge on assistant messages
- `src/components/CharacterViewport.vue` — Canvas + Three.js render loop
- `src/components/ChatInput.vue` — Text input + send button, disabled when isThinking
- `src/components/ChatMessageList.vue` — Scrollable messages, auto-scroll, TypingIndicator
- `src/components/TypingIndicator.vue` — Animated three-dot loader
- `src/views/ChatView.vue` — Main layout (60% viewport / 40% chat), character reaction wiring
- `src/App.vue` — Root component, Pinia provider
- `src/main.ts` — App entry point
- `src/style.css` — Global CSS reset + dark theme base

**Root**
- `index.html`
- `package.json`
- `vite.config.ts`
- `tsconfig.json`
- `tsconfig.node.json`
- `.gitignore`

**Rust backend (src-tauri/)**
- `src-tauri/Cargo.toml`
- `src-tauri/build.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/src/main.rs`
- `src-tauri/src/lib.rs` — AppState (conversation Mutex, vrm_path Mutex), Tauri builder
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/commands/chat.rs` — `send_message`, `get_conversation`
- `src-tauri/src/commands/agent.rs` — `list_agents`, `get_agent_status`
- `src-tauri/src/commands/character.rs` — `load_vrm`
- `src-tauri/src/agent/mod.rs` — `AgentProvider` trait
- `src-tauri/src/agent/stub_agent.rs` — Keyword-based response + Sentiment enum; 500–1000ms simulated delay
- `src-tauri/src/orchestrator/mod.rs`
- `src-tauri/src/orchestrator/agent_orchestrator.rs` — Routes requests to `StubAgent`

### Build Results
- `npm run build` (vue-tsc + vite): ✅ 0 errors, dist/ emitted
- `cargo check`: ✅ compiled cleanly
- Tests: 0 (scaffold chunk; test infrastructure established in Chunk 008)

### Notes
- `@types/three` added because three.js 0.175 ships without bundled `.d.ts`
- `src-tauri/icons/icon.png` created (placeholder) — required by `tauri::generate_context!()`
- WebGPU renderer not yet enabled (Three.js WebGPU API requires `three/addons` import path; deferred to Chunk 003 polish)
- VRM import UI (file picker + selection) deferred to Chunk 010

---

## CI Restructure — Consolidate Jobs & Eliminate Double-Firing

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Reduce GitHub Actions usage from ~10 jobs per push (5 jobs × 2 triggers) to 3 jobs × 1 trigger.
Modeled after [devstress/My3DLearning eip-ci.yml](https://github.com/devstress/My3DLearning/blob/main/.github/workflows/eip-ci.yml).

### Problem
- CI triggered on both `push` and `pull_request` → double-fired on every copilot branch push with an open PR
- 5 separate jobs (`frontend-build`, `rust-build`, `tauri-build`, `vitest`, `playwright-e2e`) ran independently, with `tauri-build` duplicating setup from `frontend-build` and `rust-build`

### Changes
1. **Removed `pull_request` trigger** — push-only avoids double-firing on copilot branches
2. **Added `paths` filter** — CI only runs when source files, configs, or the workflow itself change (not on README/docs-only changes)
3. **Consolidated `frontend-build` + `rust-build` + `tauri-build` into single `build-and-test` job** — one runner installs system deps, Node.js, and Rust once; runs frontend build, cargo check/test/clippy, and `npx tauri build` sequentially
4. **Kept `vitest` as independent parallel job** — fast, no system deps needed
5. **Kept `playwright-e2e` gated on `build-and-test` + `vitest`** — only runs after both pass

### Files Modified
- `.github/workflows/terransoul-ci.yml` — full restructure

### Result
- Jobs per push: 5 → 3 (`build-and-test`, `vitest`, `playwright-e2e`)
- Workflow runs per push: 2 → 1 (no more push+PR duplication)
- Total CI jobs per push: ~10 → 3

---

## Chunk 002 — Chat UI Polish & Vitest Component Tests

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Polish visual styles for all 4 chat components. Add Vitest + @vue/test-utils testing infrastructure.
Write comprehensive component tests for ChatInput, ChatMessageList, TypingIndicator, and AgentBadge.
Add `npm run test` script. Add `vitest` CI job.

### Architecture
- Vitest 4.1 with jsdom environment for Vue component testing
- @vue/test-utils 2.4 for Vue component mounting
- Separate `vitest.config.ts` using `@vitejs/plugin-vue`
- Tests colocated with components (`*.test.ts` alongside `*.vue`)

### Changes

**New files:**
- `vitest.config.ts` — Vitest configuration (jsdom environment, globals)
- `src/components/AgentBadge.test.ts` — 3 tests (render, class, different names)
- `src/components/TypingIndicator.test.ts` — 3 tests (container, dot count, element type)
- `src/components/ChatInput.test.ts` — 9 tests (render, disabled, empty, enabled, emit, clear, disabled submit, whitespace, placeholder)
- `src/components/ChatMessageList.test.ts` — 11 tests (empty, user class, assistant class, content, order, typing on, typing off, badge, no badge for user, default agent, timestamp)

**Modified files:**
- `package.json` — Added `test` and `test:watch` scripts; added vitest, @vue/test-utils, jsdom devDependencies
- `src/components/AgentBadge.vue` — Added dot indicator before badge text, improved spacing
- `src/components/TypingIndicator.vue` — Added background bubble, adjusted dot sizing and color
- `src/components/ChatInput.vue` — Added focus ring glow, active press scale, improved padding and transitions
- `src/components/ChatMessageList.vue` — Added gradient to user bubbles, subtle shadow, adjusted spacing and border-radius
- `.github/workflows/terransoul-ci.yml` — Added `vitest` job (parallel, no system deps needed), added `vitest.config.ts` to paths filter

### Test Results
- 4 test files, 26 tests, all passing
- AgentBadge: 3 tests
- TypingIndicator: 3 tests
- ChatInput: 9 tests
- ChatMessageList: 11 tests

### Notes
- Tests use jsdom environment — no browser needed for CI
- `vitest` CI job runs independently of `build-and-test` (no system deps required)
- Vitest globals enabled for cleaner test syntax

---

## Chunk 003 — Three.js Scene Polish + WebGPU Detection

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Enhance the Three.js scene with WebGPU renderer detection and fallback to WebGL.
Replace window resize listener with ResizeObserver for accurate per-element resize handling.
Add renderer.info debug overlay toggled by Ctrl+D.

### Architecture
- Async `initScene()` — attempts WebGPU first via `navigator.gpu` check and dynamic import
- Dynamic `import('three/webgpu')` — code-split into separate chunk, only loaded if WebGPU available
- ResizeObserver — watches canvas parent element for resize instead of global window event
- Debug overlay — shows renderer type, triangle count, draw calls, and shader programs

### Changes

**Modified files:**
- `src/renderer/scene.ts` — Made `initScene` async; added WebGPU detection via `navigator.gpu` + dynamic import of `three/webgpu`; fallback to WebGLRenderer; replaced `window.addEventListener('resize')` with `ResizeObserver`; added `RendererType`, `RendererInfo` types and `getRendererInfo()` helper; zero-guard on resize dimensions
- `src/components/CharacterViewport.vue` — Updated to `async onMounted` for async `initScene()`; added `Ctrl+D` keyboard handler to toggle debug overlay; added reactive `showDebug`, `rendererType`, `debugInfo` refs; renders debug overlay with renderer type, triangles, draw calls, shader programs; cleans up keydown listener in `onUnmounted`

### Build Results
- `npm run build`: ✅ passes, WebGPU renderer code-split into `three.webgpu-*.js` chunk (537 KB)
- `npm run test`: ✅ 26 tests passing (no regressions)

### Notes
- WebGPU renderer chunk is only downloaded at runtime when `navigator.gpu` exists
- In jsdom tests, WebGPU is not available — WebGL fallback path is always used
- Debug overlay is invisible by default; toggle with Ctrl+D during development

---

## Chunk 004 — VRM Model Loading & Fallback

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Harden vrm-loader.ts with robust error handling for corrupt/missing VRM files.
Add loading progress callback. Extract and expose VRM metadata (title, author, license)
supporting both VRM 0.0 and VRM 1.0 formats. Write Vitest unit tests for loader error paths.

### Architecture
- `loadVRM()` — validates path input, throws on empty/null path, throws if GLTF has no VRM data
- `loadVRMSafe()` — wraps loadVRM in try/catch, returns null on error (caller falls back to capsule)
- `extractVrmMetadata()` — handles VRM 1.0 (name, authors, licenseUrl) and VRM 0.0 (title, author, licenseName)
- `ProgressCallback` type — (loaded, total) callback fired during XHR loading
- `VrmMetadata` interface added to types/index.ts
- Character store extended with `vrmMetadata`, `loadError`, `setMetadata`, `setLoadError`

### Changes

**New files:**
- `src/renderer/vrm-loader.test.ts` — 12 tests (VRM 1.0 extraction, VRM 0.0 extraction, null meta, empty meta, path validation, safe loader error handling)

**Modified files:**
- `src/renderer/vrm-loader.ts` — Added input validation, error boundaries, `loadVRMSafe()`, `extractVrmMetadata()`, `ProgressCallback` type, `VrmLoadResult` interface
- `src/types/index.ts` — Added `VrmMetadata` interface (title, author, license)
- `src/stores/character.ts` — Added `vrmMetadata`, `loadError` refs; `setMetadata()`, `setLoadError()` actions

### Test Results
- 5 test files, 38 tests, all passing
- VRM loader: 12 tests (8 metadata + 4 error path)

### Notes
- VRM 1.0 uses `name`, `authors[]`, `licenseUrl`; VRM 0.0 uses `title`, `author`, `licenseName`
- `loadVRMSafe` logs errors and returns null — callers use capsule placeholder as fallback
- Three.js GLTFLoader not testable in jsdom; tests focus on metadata extraction and validation logic

---

## Chunk 005 — Character State Machine Tests

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Add Rust unit tests for `stub_agent.rs` covering all keyword branches and the neutral fallback.
Add Vitest tests for `character-animator.ts` covering all state transitions and animation behaviors.

### Changes

**Modified files:**
- `src-tauri/src/agent/stub_agent.rs` — Added `#[cfg(test)]` module with 7 tests: name resolution (2), keyword branches (hello, hi, sad, happy, neutral)

**New files:**
- `src/renderer/character-animator.test.ts` — 9 Vitest tests: default idle, setState resets, thinking vs idle, talking animation, happy bounce, sad droop, full transition chain, no-op update, setPlaceholder behavior

### Test Results
- **Rust:** 7 tests passing (stub_agent)
- **Vitest:** 6 test files, 47 tests, all passing (9 new character-animator tests)
- **Total new tests this chunk:** 16

### Notes
- Rust async tests use `#[tokio::test]` with real async `respond()` calls (500ms+ simulated delay)
- Character animator tests use real `THREE.Group` instances in jsdom — basic transforms work without WebGL

---

## Chunk 006 — Rust Chat Commands — Unit Tests

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Add unit tests for `commands/chat.rs`: `send_message` success, empty input validation,
conversation ordering, custom agent ID. Refactor commands to be testable without Tauri runtime.

### Architecture
- Extracted `process_message(&str, Option<&str>, &AppState)` — core logic, testable without `tauri::State`
- Extracted `fetch_conversation(&AppState)` — core logic, testable directly
- `send_message` and `get_conversation` Tauri commands now delegate to these functions
- Added empty/whitespace input validation returning `Err("Message cannot be empty")`

### Changes

**Modified files:**
- `src-tauri/src/commands/chat.rs` — Refactored into `process_message` + `fetch_conversation` helper functions; Tauri commands delegate to helpers; added empty input validation; added 8 tests
- `src/renderer/character-animator.test.ts` — Fixed unused variable warnings from vue-tsc

### Test Results
- **Rust:** 15 tests passing (7 stub_agent + 8 chat commands)
- **Vitest:** 6 test files, 47 tests, all passing
- **New chat command tests:** success, empty input, whitespace, message pairing, conversation ordering, empty conversation, custom agent ID, timestamp ordering

### Notes
- `process_message` and `fetch_conversation` take `&AppState` directly — no Tauri runtime needed
- Empty/whitespace input now returns an error instead of sending to agent

---

## Chunk 007 — Agent Orchestrator Hardening

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Add `AgentProvider` trait for pluggable agent implementations. Refactor orchestrator to use
trait-based dispatch with agent registry. Add health-check method. Write unit tests with MockAgent.

### Architecture
- `AgentProvider` trait — `id()`, `name()`, `respond()`, `health_check()` (async_trait)
- `StubAgent` implements `AgentProvider` — existing behavior preserved
- `AgentOrchestrator` — holds `HashMap<String, Arc<dyn AgentProvider>>`, supports `register()`, `dispatch()`, `health_check()`, `list_agents()`
- `dispatch()` now returns `Result<(String, String), String>` — errors on unknown agent ID
- "auto" and empty agent_id route to default agent ("stub")

### Changes

**Modified files:**
- `src-tauri/Cargo.toml` — Added `async-trait = "0.1"`
- `src-tauri/src/agent/mod.rs` — Added `AgentProvider` trait definition with `async_trait`
- `src-tauri/src/agent/stub_agent.rs` — Implemented `AgentProvider` for `StubAgent`; extracted `classify()` method; added `health_check()` returning true; `Sentiment` now derives `Clone, PartialEq, Eq, Debug`
- `src-tauri/src/orchestrator/agent_orchestrator.rs` — Rewritten with agent registry (`HashMap<String, Arc<dyn AgentProvider>>`); `dispatch()` returns `Result`; added `register()`, `get_agent()`, `health_check()`, `list_agents()`; 8 tests with `MockAgent`
- `src-tauri/src/commands/chat.rs` — Added `use crate::agent::AgentProvider` for trait method resolution

### Test Results
- **Rust:** 23 tests passing (7 stub_agent + 8 chat + 8 orchestrator)
- **Vitest:** 6 test files, 47 tests, all passing
- **Clippy:** ✅ 0 warnings

### Notes
- `async_trait` crate used for trait-based async dispatch
- MockAgent in tests verifies dispatch routing, health checks, and agent registration
- Agent registry enables future hot-plugging of real agents (OpenAI, local models, etc.)

---

## Chunk 010 — Character Reactions — Full Integration

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Connect sentiment from the Rust backend to the frontend character animations. Enhance
the character-animator with BlendShape mouth animation for VRM models, head bone animations,
scale pulse for placeholder talking, and improved droop/tilt for sad state.

### Architecture
- Rust `Message` struct now includes `sentiment: Option<String>` field
- `process_message()` maps `Sentiment` enum to string ("happy", "sad", "neutral")
- Frontend `ChatView.vue` reads sentiment from assistant response
- `sentimentToState()` maps sentiment → CharacterState for animation
- `CharacterAnimator.setBlendShape()` wraps VRM expressionManager for safe BlendShape access
- Enhanced animations: head bone for thinking/sad, aa/oh BlendShapes for talking, scale pulse for placeholder

### Changes

**Modified files:**
- `src-tauri/src/commands/chat.rs` — Added `sentiment` field to `Message` struct, map `Sentiment` enum to string in `process_message()`, 4 new sentiment tests
- `src/types/index.ts` — Added `sentiment?: 'happy' | 'sad' | 'neutral'` to `Message` interface
- `src/renderer/character-animator.ts` — Added `getState()` accessor, BlendShape support via `setBlendShape()`, head bone animations for idle/thinking/sad, mouth open/close for talking (aa/oh), happy BlendShape, scale animations for all placeholder states
- `src/views/ChatView.vue` — Added `sentimentToState()` function, reads sentiment from last response to drive character state
- `src/renderer/character-animator.test.ts` — 6 new tests: getState, talking scale pulse, happy scale, sad tilt, sad scale, idle scale reset

### Test Results
- **Rust:** 27 tests passing (7 stub_agent + 12 chat + 8 orchestrator)
- **Vitest:** 7 test files, 61 tests, all passing (6 new character-animator tests)
- **Build:** ✅ clean

---

## Chunk 011 — VRM Import + Character Selection UI

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Add VRM import panel with character selection and switching. Wire CharacterViewport
to auto-load VRM models when path changes. Display character name and author from VRM metadata.

### Architecture
- `ModelPanel.vue` — Slide-in panel from viewport with import button, character cards, error display
- `CharacterViewport.vue` — Watches `characterStore.vrmPath`, loads VRM on change, shows metadata
- `character.ts` store — Added `resetCharacter()` action for switching back to default
- Toggle button overlaid on viewport (absolute positioned, z-index above canvas)

### Changes

**New files:**
- `src/components/ModelPanel.vue` — Import VRM panel with: import button (Tauri file dialog), default placeholder card, custom VRM card, error banner, instructions reference
- `src/components/ModelPanel.test.ts` — 8 tests (render header, import button, default card, overlay close, close button, format hint, instructions ref, default active)
- `instructions/README.md` — Overview, quick start, format support, model sources
- `instructions/IMPORTING-MODELS.md` — Step-by-step import guide, flow diagram, requirements, troubleshooting
- `instructions/EXTENDING.md` — Developer guide: architecture, extension points, custom animations, agents, UI, scene elements, testing

**Modified files:**
- `src/components/CharacterViewport.vue` — Added VRM metadata overlay (character name + author), computed `characterName`, watcher for `vrmPath` to auto-load VRM, stores `SceneContext` for VRM loading
- `src/stores/character.ts` — Added `resetCharacter()` action
- `src/views/ChatView.vue` — Added ModelPanel component, toggle button, relative positioning on viewport section

### Test Results
- **Vitest:** 7 test files, 61 tests, all passing (8 new ModelPanel tests)
- **Build:** ✅ clean

### Notes
- Model import currently uses `window.prompt()` as fallback when Tauri file dialog is unavailable (browser preview mode)
- In full Tauri desktop mode, this should be replaced with `@tauri-apps/plugin-dialog` for native file picker
- VRM path is persisted in Rust `AppState` via `load_vrm` command
- `instructions/` folder added at project root with 3 documentation files

---

## Chunk 008 — Tauri IPC Bridge Integration Tests

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Write integration tests that mock the Tauri IPC `invoke()` function and test the
conversation and character stores end-to-end. Verify round-trip message flow, error
handling, isThinking lifecycle, sentiment propagation, and conversation history.

### Architecture
- `vi.mock('@tauri-apps/api/core')` replaces `invoke()` with a Vitest mock function
- Each test configures `mockInvoke` with `mockResolvedValueOnce` / `mockRejectedValueOnce`
- Tests use real Pinia stores (via `setActivePinia(createPinia())`)
- No Tauri runtime needed — pure JavaScript-level integration testing

### Changes

**New files:**
- `src/stores/conversation.test.ts` — 8 tests: send message round-trip, custom agent routing, error handling, isThinking lifecycle, getConversation, getConversation error, sentiment preservation, multiple message ordering
- `src/stores/character.test.ts` — 4 tests: loadVrm success, loadVrm error, clear state before load, resetCharacter

### Test Results
- **Vitest:** 9 test files, 73 tests, all passing (12 new store integration tests)
- **Build:** ✅ clean

### Notes
- In Tauri v2, `@tauri-apps/api/mocks` from v1 is not available — using `vi.mock()` directly
- Tests verify the full store lifecycle: user message → invoke → response → store update
- The `isThinking` lifecycle test uses a deferred promise to observe mid-flight state

---

## Chunk 009 — Playwright E2E Test Infrastructure

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Install Playwright with Chromium browser, create E2E tests that run against the Vite
dev server, and add a `playwright-e2e` CI job that runs after `build-and-test`.

### Architecture
- `@playwright/test` 1.59.1 with Chromium headless shell
- `playwright.config.ts` — baseURL `http://localhost:1420`, auto-starts Vite dev server
- Tests run against pure frontend (no Tauri backend) — `invoke()` errors handled gracefully
- CI job: `playwright-e2e` depends on `build-and-test`, installs Chromium with deps, uploads report artifact

### Changes

**New files:**
- `playwright.config.ts` — Chromium project, Vite webServer, GitHub reporter in CI
- `e2e/app.spec.ts` — 6 E2E tests: app loads, chat input, send message, 3D canvas, state badge, model panel toggle

**Modified files:**
- `package.json` — Added `test:e2e` script, `@playwright/test` devDependency
- `.github/workflows/terransoul-ci.yml` — Added `playwright-e2e` job (needs build-and-test, installs Chromium, runs tests, uploads report)

### Test Results
- **Playwright:** 6 tests, all passing (~8.8s)
- **Vitest:** 9 test files, 73 tests, all passing (no regression)
- **Build:** ✅ clean

### Notes
- E2E tests run against Vite dev server only — no Tauri runtime required
- When `invoke()` fails (no backend), the conversation store catches errors and displays "Error: ..." messages — tests verify this graceful degradation
- Playwright report uploaded as CI artifact for debugging failures
- `--with-deps` flag installs Chromium OS dependencies in CI

---

## Chunk 020 — Device Identity & Pairing

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Implement per-device Ed25519 identity (generated on first launch, persisted to app data dir),
QR-code-based pairing handshake (SVG QR encoding device_id + public key), and a trusted device
list (persisted as JSON in app data dir).

### Architecture
- `src-tauri/src/identity/device.rs` — `DeviceIdentity` wraps `ed25519_dalek::SigningKey` with a UUID device_id. `DeviceInfo` (serialisable) exposes device_id, base64 public key, and name.
- `src-tauri/src/identity/key_store.rs` — `load_or_generate_identity(data_dir)`: loads from `device_key.json` if present, otherwise generates and persists.
- `src-tauri/src/identity/qr.rs` — `generate_pairing_qr(info)`: encodes JSON payload `{app, v, device_id, pub_key, name}` as an SVG QR code via the `qrcode` crate.
- `src-tauri/src/identity/trusted_devices.rs` — `TrustedDevice` struct; `add/remove/load/save_trusted_devices` functions operating on `Vec<TrustedDevice>` and `trusted_devices.json`.
- `src-tauri/src/commands/identity.rs` — 5 Tauri commands: `get_device_identity`, `get_pairing_qr`, `list_trusted_devices`, `add_trusted_device_cmd`, `remove_trusted_device_cmd`.
- `AppState` extended with `device_identity: Mutex<Option<DeviceIdentity>>` and `trusted_devices: Mutex<Vec<TrustedDevice>>`.
- Identity is initialised in `setup()` before the window opens.

### New Dependencies
- `ed25519-dalek = { version = "2", features = ["rand_core"] }` — Ed25519 key pair generation
- `rand_core = { version = "0.6", features = ["getrandom"] }` — `OsRng` for key generation
- `qrcode = "0.14"` — SVG QR code rendering
- `base64 = "0.22"` — encoding key bytes for transport/display
- `tempfile = "3"` (dev-only) — temp dirs for key_store and trusted_devices tests

### Files Created
**Rust:**
- `src-tauri/src/identity/mod.rs`
- `src-tauri/src/identity/device.rs` (6 unit tests)
- `src-tauri/src/identity/key_store.rs` (2 unit tests)
- `src-tauri/src/identity/qr.rs` (2 unit tests)
- `src-tauri/src/identity/trusted_devices.rs` (6 unit tests)
- `src-tauri/src/commands/identity.rs`

**Frontend:**
- `src/stores/identity.ts` — Pinia identity store (loadIdentity, loadPairingQr, loadTrustedDevices, addTrustedDevice, removeTrustedDevice, clearError)
- `src/stores/identity.test.ts` — 9 Vitest tests
- `src/views/PairingView.vue` — QR display, identity info, trusted device list with remove buttons

### Files Modified
- `src-tauri/Cargo.toml` — new deps + dev-dep
- `src-tauri/src/commands/mod.rs` — added `identity` module
- `src-tauri/src/lib.rs` — added identity module, extended AppState, setup() initialisation, 5 new commands registered
- `src-tauri/src/commands/chat.rs` — updated `make_state()` test helper to use `AppState::for_test()`
- `src/types/index.ts` — added `DeviceInfo` and `TrustedDevice` interfaces

### Test Results
- **Rust:** 16 new unit tests in the identity module (device: 6, key_store: 2, qr: 2, trusted_devices: 6)
- **Vitest:** 10 test files, 82 tests, all passing (9 new identity store tests)
- **TypeScript:** `vue-tsc --noEmit` passes with 0 errors

### Notes
- Key storage uses a file-based approach (`device_key.json` in app data dir) — a production upgrade path to OS keychain via the `keyring` crate is straightforward by swapping the storage layer.
- QR payload is compact JSON: `{"app":"TerranSoul","v":1,"device_id":"…","pub_key":"…","name":"…"}`
- `AppState::for_test()` is `#[cfg(test)]`-gated to keep test ergonomics clean without polluting production API

---

## Chunk 021 — Link Transport Layer

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Implement the peer-to-peer transport layer for TerranSoul Link cross-device communication.
QUIC as primary transport, WebSocket as fallback. Abstract behind a `LinkTransport` trait.
Link manager with reconnection logic and transport fallback.

### Architecture
- `src-tauri/src/link/mod.rs` — `LinkTransport` async trait, `LinkMessage`, `LinkStatus`, `LinkPeer`, `PeerAddr` types. 6 unit tests for type serialisation.
- `src-tauri/src/link/quic.rs` — `QuicTransport` using `quinn` crate. Self-signed TLS certs via `rcgen`. Length-prefixed JSON frames over bidirectional QUIC streams. Server cert verification skipped (trust via device pairing). 9 unit tests.
- `src-tauri/src/link/ws.rs` — `WsTransport` using `tokio-tungstenite`. JSON text frames. 6 unit tests.
- `src-tauri/src/link/manager.rs` — `LinkManager` wraps a `LinkTransport` with connect/reconnect/send/recv/disconnect. Auto-fallback from QUIC → WebSocket after max reconnect attempts. Configurable `max_reconnect_attempts`. `with_transport()` constructor for testability. 10 unit tests with `MockTransport`.
- `src-tauri/src/commands/link.rs` — 4 Tauri commands: `get_link_status`, `start_link_server`, `connect_to_peer`, `disconnect_link`.
- `AppState` extended with `link_manager: TokioMutex<LinkManager>` and `link_server_port: TokioMutex<Option<u16>>` (tokio Mutex for async commands).

### New Dependencies
- `quinn = "0.11"` — QUIC transport
- `rustls = { version = "0.23", default-features = false, features = ["ring", "std"] }` — TLS for QUIC
- `rcgen = "0.13"` — self-signed certificate generation
- `rustls-pemfile = "2"` — PEM parsing
- `tokio-tungstenite = { version = "0.26", features = ["rustls-tls-webpki-roots"] }` — WebSocket transport
- `futures-util = "0.3"` — stream/sink combinators for WebSocket

### Files Created
**Rust:**
- `src-tauri/src/link/mod.rs` — `LinkTransport` trait + shared types (6 tests)
- `src-tauri/src/link/quic.rs` — QUIC transport (9 tests)
- `src-tauri/src/link/ws.rs` — WebSocket transport (6 tests)
- `src-tauri/src/link/manager.rs` — Link manager with reconnection (10 tests)
- `src-tauri/src/commands/link.rs` — 4 Tauri commands

**Frontend:**
- `src/stores/link.ts` — Pinia link store (fetchStatus, startServer, connectToPeer, disconnect, clearError)
- `src/stores/link.test.ts` — 11 Vitest tests

### Files Modified
- `src-tauri/Cargo.toml` — 6 new dependencies (quinn, rustls, rcgen, rustls-pemfile, tokio-tungstenite, futures-util)
- `src-tauri/src/commands/mod.rs` — added `link` module
- `src-tauri/src/lib.rs` — added link module, extended AppState with TokioMutex fields, 4 new commands registered
- `src/types/index.ts` — added `LinkStatusValue`, `LinkPeer`, `LinkStatusResponse` types

### Test Results
- **Rust:** 31 new unit tests in the link module (mod: 6, quic: 9, ws: 6, manager: 10)
- **Vitest:** 11 test files, 93 tests, all passing (11 new link store tests)
- **TypeScript:** `vue-tsc --noEmit` passes with 0 errors

### Notes
- Self-signed certificates are used for QUIC TLS — trust is established via device pairing (Ed25519 identity from Chunk 020), not PKI
- Messages are framed as length-prefixed JSON (QUIC) or text frames (WebSocket) — both use `LinkMessage` JSON
- Frame size limit: 16 MiB to prevent memory exhaustion
- `LinkManager::with_transport()` enables full unit testing with `MockTransport`
- QUIC → WebSocket fallback is automatic after `max_reconnect_attempts` (default 5)

---

## Chunk 022 — CRDT Sync Engine

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Implement CRDT-based data synchronisation for cross-device sync:
- Append-only log (conversation history)
- Last-Write-Wins register (character selection)
- OR-Set (agent status map)

All CRDTs use HLC (Hybrid Logical Clock) timestamps with site tiebreaker for deterministic ordering.

### Architecture
- `src-tauri/src/sync/mod.rs` — `HLC` (counter + site_ord), `SyncOp` (crdt_id, kind, hlc, site, payload), `CrdtState` trait (apply, snapshot_ops), `SiteId` type. 6 unit tests.
- `src-tauri/src/sync/append_log.rs` — `AppendLog` CRDT: ordered by HLC, idempotent duplicate rejection via binary search insert. 9 unit tests incl. concurrent edit convergence.
- `src-tauri/src/sync/lww_register.rs` — `LwwRegister` CRDT: last write wins, tiebreak by higher site_ord. 11 unit tests incl. concurrent edit convergence.
- `src-tauri/src/sync/or_set.rs` — `OrSet` CRDT: observed-remove semantics, each add creates a unique tag (HLC + site), remove only removes observed tags. Concurrent add + remove → add wins for unseen tags. 11 unit tests incl. add-wins-concurrent test.
- Frontend `src/stores/sync.ts` — Pinia store mirroring CRDT summary (conversationCount, characterSelection, agentCount, lastSyncedAt).
- Frontend `src/stores/sync.test.ts` — 8 Vitest tests.

### Files Created
**Rust:**
- `src-tauri/src/sync/mod.rs` — HLC + SyncOp + CrdtState trait (6 tests)
- `src-tauri/src/sync/append_log.rs` — Append-only log CRDT (9 tests)
- `src-tauri/src/sync/lww_register.rs` — LWW register CRDT (11 tests)
- `src-tauri/src/sync/or_set.rs` — OR-Set CRDT (11 tests)

**Frontend:**
- `src/stores/sync.ts` — Pinia sync store
- `src/stores/sync.test.ts` — 8 Vitest tests

### Files Modified
- `src-tauri/src/lib.rs` — added `sync` module
- `src/types/index.ts` — added `SyncState` interface

### Test Results
- **Rust:** 37 new unit tests in the sync module (mod: 6, append_log: 9, lww_register: 11, or_set: 11)
- **Vitest:** 12 test files, 101 tests, all passing (8 new sync store tests)
- **TypeScript:** `vue-tsc --noEmit` passes with 0 errors

### Notes
- No external CRDT crate used — minimal custom implementation avoids dependency bloat
- HLC ordering: `(counter, site_ord)` — deterministic total order across all devices
- AppendLog: binary search insert + duplicate check makes `apply()` O(log n)
- OR-Set: concurrent add + remove resolves to add-wins for unobserved tags, matching standard OR-Set semantics
- All CRDTs implement `snapshot_ops()` for full state transfer to new peers

---

## Chunk 023 — Remote Command Routing

**Date:** 2026-04-10
**Status:** ✅ Done

### Goal
Allow a secondary device (e.g. phone) to send commands to a primary device (e.g. PC)
via a command envelope protocol. Target device runs permission checks — first remote
command from an unknown device requires explicit user approval. Results are returned
to the originating device.

### Architecture
- `src-tauri/src/routing/command_envelope.rs` — `CommandEnvelope` (command_id, origin_device, target_device, command_type, payload, status), `CommandResult` (success/denied/failed constructors), `CommandStatus` enum (PendingApproval, Executing, Completed, Denied, Failed). 7 unit tests.
- `src-tauri/src/routing/permission.rs` — `PermissionPolicy` (Allow/Deny/Ask), `PermissionStore` (per-device policy map, pending command set, approve/deny with remember/block). 10 unit tests.
- `src-tauri/src/routing/router.rs` — `CommandRouter` handles incoming envelopes: wrong target → deny, allowed device → execute, blocked → deny, unknown → pending. Executes ping, list_agents, send_message stubs. approve/deny pending commands with policy memory. 14 unit tests.
- `src-tauri/src/commands/routing.rs` — 5 Tauri commands: `list_pending_commands`, `approve_remote_command`, `deny_remote_command`, `set_device_permission`, `get_device_permissions`.
- `AppState` extended with `command_router: TokioMutex<CommandRouter>`. Router initialised in `setup()` with device_id from identity.

### Files Created
**Rust:**
- `src-tauri/src/routing/mod.rs` — re-exports
- `src-tauri/src/routing/command_envelope.rs` (7 tests)
- `src-tauri/src/routing/permission.rs` (10 tests)
- `src-tauri/src/routing/router.rs` (14 tests)
- `src-tauri/src/commands/routing.rs` — 5 Tauri commands

**Frontend:**
- `src/stores/routing.ts` — Pinia routing store (fetchPendingCommands, approveCommand, denyCommand, setDevicePermission, getDevicePermissions)
- `src/stores/routing.test.ts` — 10 Vitest tests

### Files Modified
- `src-tauri/src/commands/mod.rs` — added `routing` module
- `src-tauri/src/lib.rs` — added routing module, extended AppState with command_router, setup() initialisation, 5 new commands registered
- `src/types/index.ts` — added `CommandStatusValue`, `PendingCommand`, `CommandResultResponse` types

### Test Results
- **Rust:** 31 new unit tests in the routing module (command_envelope: 7, permission: 10, router: 14)
- **Vitest:** 13 test files, 111 tests, all passing (10 new routing store tests)
- **TypeScript:** `vue-tsc --noEmit` passes with 0 errors

### Notes
- Unknown devices default to "Ask" — first remote command goes to pending queue
- `approve(remember=true)` sets the device to "Allow" for all future commands
- `deny(block=true)` sets the device to "Deny" permanently
- CommandRouter has stub execute() for ping, list_agents, send_message — production will delegate to the real orchestrator
- Phase 2 is now complete (chunks 020–023)

---

## Chunk 030 — Package Manifest Format

**Date:** 2026-04-11
**Status:** ✅ Done

### Goal
Define the agent package manifest schema that every TerranSoul agent must include.
Implement a manifest parser with full validation in Rust, expose Tauri commands for
the frontend to parse and validate manifests, and add TypeScript types and a Pinia store.

### Architecture
- Manifest schema: `AgentManifest` struct with name, version, description, system_requirements,
  install_method, capabilities, ipc_protocol_version, and optional homepage/license/author/sha256
- `SystemRequirements`: min_ram_mb, os targets, arch targets, gpu_required
- `InstallMethod`: tagged enum — Binary (url), Wasm (url), Sidecar (path)
- `Capability`: 7 variants — chat, filesystem, clipboard, network, remote_exec, character,
  conversation_history. Sensitive caps (filesystem, clipboard, network, remote_exec) require consent.
- Validation: name format (lowercase, alphanum+hyphens, 1–64 chars), semver version,
  non-empty description, supported IPC protocol range, SHA-256 format
- 3 Tauri commands: parse_agent_manifest, validate_agent_manifest, get_ipc_protocol_range

### Files Created
**Rust (src-tauri/src/)**
- `package_manager/mod.rs` — Module re-exports
- `package_manager/manifest.rs` — AgentManifest, SystemRequirements, InstallMethod, Capability,
  OsTarget, ArchTarget, ManifestError, parse/validate/serialize functions, 28 unit tests
- `commands/package.rs` — ManifestInfo, parse_agent_manifest, validate_agent_manifest,
  get_ipc_protocol_range Tauri commands

### Files Modified
**Rust (src-tauri/src/)**
- `lib.rs` — Added `package_manager` module, imported and registered 3 new commands
- `commands/mod.rs` — Added `package` module

**Frontend (src/)**
- `types/index.ts` — Added ManifestInfo and InstallType types
- `stores/package.ts` — Pinia store: parseManifest, validateManifest, getIpcProtocolRange, clearManifest, clearError
- `stores/package.test.ts` — 10 Vitest tests

### Test Counts
- **Rust:** 169 total (28 new manifest tests)
- **Vitest:** 14 test files, 126 tests (10 new package store tests)
- **Clippy:** 0 warnings
- **TypeScript:** `vue-tsc --noEmit` passes with 0 errors

---

## Chunk 031 — Install / Update / Remove Commands

**Date:** 2026-04-11
**Status:** ✅ Done

### Goal
Implement agent install, update, remove, and list commands. Registry client trait with mock
implementation for testing. SHA-256 hash verification for downloaded binaries. File-backed
persistence of installed agent manifests and binaries.

### Architecture
- `RegistrySource` trait: async fetch_manifest, download_binary, search. Allows swapping real
  HTTP registry for mock in tests.
- `MockRegistry`: in-memory HashMap-backed registry for testing.
- `PackageInstaller`: manages `agents/` directory. On install: fetch manifest → download binary →
  verify SHA-256 → write manifest.json + agent.bin. On update: check version, re-download if newer.
  On remove: delete agent directory. Reloads installed manifests from disk on construction.
- Pure-Rust SHA-256 implementation (no new crate dependency) for hash verification.
- 4 new Tauri commands: install_agent, update_agent, remove_agent, list_installed_agents.
- AppState gains `package_installer` and `package_registry` TokioMutex fields.
  `AppState::new()` now takes `data_dir: &Path`.

### Files Created
**Rust (src-tauri/src/)**
- `package_manager/registry.rs` — RegistrySource trait, RegistryError, MockRegistry (8 tests)
- `package_manager/installer.rs` — PackageInstaller, InstalledAgent, InstallerError, SHA-256
  digest, filesystem persistence (16 tests)

### Files Modified
**Rust (src-tauri/src/)**
- `package_manager/mod.rs` — Added registry and installer re-exports
- `commands/package.rs` — Added InstalledAgentInfo, install_agent, update_agent, remove_agent,
  list_installed_agents Tauri commands
- `lib.rs` — AppState gains 2 new fields, `new()` takes data_dir, 4 new commands registered

**Frontend (src/)**
- `types/index.ts` — Added InstalledAgentInfo interface
- `stores/package.ts` — Added installAgent, updateAgent, removeAgent, fetchInstalledAgents, installedAgents ref
- `stores/package.test.ts` — Expanded to 18 tests (8 new)

### Test Counts
- **Rust:** 193 total (24 new: 8 registry + 16 installer)
- **Vitest:** 14 test files, 134 tests (18 package store tests, 8 new)
- **Clippy:** 0 warnings
- **TypeScript:** `vue-tsc --noEmit` passes with 0 errors

---

## Chunk 040 — Brain (Local LLM via Ollama)

### Summary
Adds a local LLM "brain" to TerranSoul powered by Ollama. The first time the app
launches (no brain configured), a 5-step onboarding wizard analyses the user's hardware
(RAM, CPU, OS) and recommends the best model tier:

| RAM | Top pick |
|-----|---------|
| < 4 GB | TinyLlama |
| 4–8 GB | Gemma 3 1B |
| 8–16 GB | Gemma 3 4B ⭐ |
| 16–32 GB | Gemma 3 12B |
| 32 GB+ | Gemma 3 27B |

Once configured, all chat messages are routed through the active Ollama model.

### Files Added / Modified
- `src-tauri/src/brain/system_info.rs` — sysinfo-based hardware detection + RAM tier
- `src-tauri/src/brain/model_recommender.rs` — tiered model recommendations
- `src-tauri/src/brain/brain_store.rs` — persist/load active model from disk
- `src-tauri/src/brain/ollama_agent.rs` — OllamaAgent (AgentProvider + respond_contextual + extract/summarize helpers)
- `src-tauri/src/brain/mod.rs`
- `src-tauri/src/commands/brain.rs` — 7 Tauri commands
- `src-tauri/src/commands/chat.rs` — route through OllamaAgent when brain set
- `src-tauri/src/lib.rs` — active_brain + ollama_client + data_dir in AppState
- `src/views/BrainSetupView.vue` — 5-step wizard
- `src/stores/brain.ts` + `src/stores/brain.test.ts`
- `src/types/index.ts` — SystemInfo, ModelRecommendation, OllamaStatus, OllamaModelEntry types
- `src-tauri/Cargo.toml` — sysinfo, reqwest (json+stream), futures-util

### New Tauri Commands
`get_system_info` · `recommend_brain_models` · `check_ollama_status` · `get_ollama_models`
`pull_ollama_model` · `set_active_brain` · `get_active_brain` · `clear_active_brain`

### Test Counts
- **Rust:** 38 new tests in brain module (245 total)
- **Vitest:** 11 new tests in brain.test.ts (153 total)

---

## Chunk 041 — Long/Short-term Memory + Brain-powered Recall

### Summary
Adds a SQLite-backed memory system that the brain model actively manages:

**Short-term memory:** The last 20 conversation messages are passed as context to every
Ollama call, giving the brain a working memory of the current session.

**Long-term memory:** Persistent facts/preferences/context stored in `memory.db`.
The brain reuses the active Ollama model for three memory operations:

1. **Extract** — After a session, Ollama identifies and stores memorable facts
2. **Summarize** — Ollama produces a 1–3 sentence session summary as a memory entry
3. **Semantic search** — Ollama ranks stored memories by relevance (keyword fallback when offline)

Before every assistant reply, the most relevant long-term memories are retrieved (via
semantic or keyword search) and injected into the Ollama system prompt — giving TerranSoul
genuine recall of past conversations.

### Memory Visualization
A **MemoryView** with three tabs:
- **List** — searchable, filterable memory cards with manual add/edit/delete
- **Graph** — cytoscape.js network where nodes = memories, edges = shared tags
- **Session** — the live short-term memory window

### Files Added / Modified
- `src-tauri/src/memory/store.rs` — SQLite CRUD + keyword search (MemoryStore)
- `src-tauri/src/memory/brain_memory.rs` — async LLM helpers (extract_facts, summarize, semantic_search_entries)
- `src-tauri/src/memory/mod.rs`
- `src-tauri/src/commands/memory.rs` — 9 Tauri commands
- `src-tauri/src/commands/chat.rs` — inject memories into every Ollama call
- `src-tauri/src/lib.rs` — memory_store in AppState
- `src/views/MemoryView.vue` — 3-tab memory manager
- `src/components/MemoryGraph.vue` — cytoscape.js knowledge graph
- `src/stores/memory.ts` + `src/stores/memory.test.ts`
- `src/App.vue` — brain-gated routing + Memory nav tab
- `src-tauri/Cargo.toml` — rusqlite (bundled)
- `package.json` — cytoscape + @types/cytoscape

### New Tauri Commands
`add_memory` · `get_memories` · `search_memories` · `update_memory` · `delete_memory`
`get_relevant_memories` · `get_short_term_memory` · `extract_memories_from_session`
`summarize_session` · `semantic_search_memories`

### Test Counts
- **Rust:** 14 new tests (12 memory/store + 4 brain_memory) — 245 total
- **Vitest:** 10 new tests in memory.test.ts — 153 total
- **Clippy:** 0 warnings

---

## Chunk 032 — Agent Registry

### Summary
Stands up a minimal in-process axum HTTP server that serves an official agent catalog. 
`HttpRegistry` implements `RegistrySource` via reqwest, replacing `MockRegistry` in `AppState`.

### Endpoints
- `GET /agents` — list all agent manifests
- `GET /agents/:name` — single manifest (404 if not found)
- `GET /agents/:name/download` — placeholder binary bytes
- `GET /search?q=` — case-insensitive search on name + description

### Official Catalog (3 agents)
| Agent | Capabilities |
|-------|-------------|
| `stub-agent` | chat |
| `openclaw-bridge` | chat, file_read, network |
| `claude-cowork` | chat, file_read, file_write, network |

### Files Added / Modified
- `src-tauri/src/registry_server/catalog.rs` — 3 official agent manifests
- `src-tauri/src/registry_server/server.rs` — axum router + start() → (port, JoinHandle)
- `src-tauri/src/registry_server/http_registry.rs` — HttpRegistry (reqwest-backed RegistrySource)
- `src-tauri/src/registry_server/mod.rs`
- `src-tauri/src/commands/registry.rs` — 4 Tauri commands
- `src-tauri/src/lib.rs` — package_registry → Box<dyn RegistrySource>, registry_server_handle field
- `src/types/index.ts` — AgentSearchResult type
- `src/stores/package.ts` — searchAgents, startRegistryServer, stopRegistryServer, getRegistryServerPort
- `src/stores/package.test.ts` — 8 new tests
- `src-tauri/Cargo.toml` — axum 0.8.4

### New Tauri Commands
`start_registry_server` · `stop_registry_server` · `get_registry_server_port` · `search_agents`

### Test Counts
- **Rust:** 8 new tests (server routes + HttpRegistry) — 265 total
- **Vitest:** 8 new tests in package.test.ts — 174 total

---

## Chunk 033 — Agent Sandboxing

### Summary
Runs community agents inside a wasmtime 36.0.7 (Cranelift) WASM sandbox with a
capability-gated host API. Each capability (FileRead, FileWrite, Clipboard, Network,
ProcessSpawn) requires explicit user consent recorded on disk before the host function
will execute.

### Architecture
- `CapabilityStore` — JSON-backed HashMap of (agent_name, capability) → bool; auto-saves
- `HostContext` — holds agent name + Arc<Mutex<CapabilityStore>>; `check_capability` returns
  Err if not granted
- `WasmRunner` — wasmtime Engine (Cranelift, not Winch); links host functions; calls `run()→i32`
- Security guarantee: host functions return error code before touching OS if capability missing

### Files Added / Modified
- `src-tauri/src/sandbox/capability.rs` — Capability enum + CapabilityStore
- `src-tauri/src/sandbox/host_api.rs` — HostContext + file read/write stubs
- `src-tauri/src/sandbox/wasm_runner.rs` — WasmRunner (Engine + Linker + Module)
- `src-tauri/src/sandbox/mod.rs`
- `src-tauri/src/commands/sandbox.rs` — 5 Tauri commands
- `src-tauri/src/lib.rs` — capability_store: TokioMutex<CapabilityStore>
- `src/types/index.ts` — CapabilityName + ConsentInfo types
- `src/stores/sandbox.ts` + `src/stores/sandbox.test.ts`
- `src-tauri/Cargo.toml` — wasmtime 36.0.7 (default-features=false, cranelift+runtime)

### New Tauri Commands
`grant_agent_capability` · `revoke_agent_capability` · `list_agent_capabilities`
`clear_agent_capabilities` · `run_agent_in_sandbox`

### Test Counts
- **Rust:** 12 new tests (capability grant/revoke/enforce + wasm runner) — 265 total
- **Vitest:** 12 new tests in sandbox.test.ts — 174 total
- **Clippy:** 0 warnings

---

## Chunk 034 — Agent Marketplace UI

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Create a marketplace UI for browsing, searching, installing, updating, and removing agents
from the registry. Includes capability consent dialog before install and sandbox status
badges on installed agents.

### Architecture
- `MarketplaceView.vue` — Full marketplace tab with Browse and Installed sub-tabs
- `CapabilityConsentDialog.vue` — Modal dialog showing required capabilities before install
- Integrates with existing `usePackageStore` (install/update/remove/search) and
  `useSandboxStore` (capability grant/list/clear)
- Sandbox status badges on installed agents (Sandboxed/Unrestricted/Unknown)
- New "🏪 Marketplace" tab in `App.vue` navigation

### Files Created
- `src/views/MarketplaceView.vue` — Marketplace view (browse + installed tabs)
- `src/components/CapabilityConsentDialog.vue` — Pre-install consent dialog
- `src/views/MarketplaceView.test.ts` — 12 Vitest component tests

### Files Modified
- `src/App.vue` — Added marketplace tab and MarketplaceView import

### Test Counts
- **Vitest:** 12 new tests in MarketplaceView.test.ts — 200 total across 19 files

---

## Chunk 035 — Agent-to-Agent Messaging

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Allow installed agents to pass messages to each other via a topic-based pub/sub message bus.
Agents subscribe to topics and the message bus fans out published messages to all subscribers.

### Architecture
- `MessageBus` — In-memory topic-based pub/sub with per-agent inboxes (max 100 msgs)
- `AgentMessage` — Message envelope with id, sender, topic, payload, timestamp
- Sender exclusion — publishers don't receive their own messages
- Inbox size limits — oldest messages trimmed when capacity exceeded
- 5 Tauri commands for frontend integration

### Files Created
**Rust (src-tauri/src/)**
- `messaging/mod.rs` — Module declarations
- `messaging/message_bus.rs` — `MessageBus`, `AgentMessage`, `Subscription` + 15 tests
- `commands/messaging.rs` — 5 Tauri commands

**Frontend (src/)**
- `src/stores/messaging.ts` — Pinia store with publish/subscribe/unsubscribe/getMessages/listSubscriptions
- `src/stores/messaging.test.ts` — 11 Vitest tests

### Files Modified
- `src-tauri/src/lib.rs` — Added messaging module, MessageBus to AppState, registered 5 commands
- `src-tauri/src/commands/mod.rs` — Added messaging module
- `src/types/index.ts` — Added AgentMessageInfo type

### New Tauri Commands
`publish_agent_message` · `subscribe_agent_topic` · `unsubscribe_agent_topic`
`get_agent_messages` · `list_agent_subscriptions`

### Test Counts
- **Rust:** 15 new tests (message bus pub/sub/drain/peek/limits) — 280 total
- **Vitest:** 11 new tests in messaging.test.ts — 200 total across 19 files

---

## Chunk 050 — Window Mode System

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Dual-mode window: normal window mode (decorations, resizable, taskbar) + pet mode overlay
(transparent, always-on-top, skip-taskbar). Default to window mode on first launch.

### Architecture
- `commands/window.rs` — `WindowMode` enum (`Window` | `Pet`), `apply_window_mode()` helper,
  3 Tauri commands: `set_window_mode`, `get_window_mode`, `toggle_window_mode`
- `window_mode` field added to `AppState`
- System tray "Switch to Pet Mode" menu item with event emission
- `tauri.conf.json` updated: `decorations: true`, `alwaysOnTop: false`, `skipTaskbar: false`
- `stores/window.ts` — Pinia store wrapping all window/monitor IPC

### Files Created
- `src-tauri/src/commands/window.rs` — Window mode commands + 4 Rust tests
- `src/stores/window.ts` — Pinia window store
- `src/stores/window.test.ts` — 15 Vitest tests

### Files Modified
- `src-tauri/src/lib.rs` — Added window_mode to AppState, registered 3 commands, tray toggle
- `src-tauri/src/commands/mod.rs` — Added window module
- `src-tauri/tauri.conf.json` — Switched defaults from pet to window mode
- `src/types/index.ts` — Added WindowMode, MonitorInfo types

### New Tauri Commands
`set_window_mode` · `get_window_mode` · `toggle_window_mode`

---

## Chunk 051 — Selective Click-Through

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
In pet mode, clicks pass through empty areas but interact with character and chatbox.

### Architecture
- `set_cursor_passthrough` Tauri command in `commands/window.rs` — calls `window.set_ignore_cursor_events()`
- Frontend `setCursorPassthrough(ignore: boolean)` in window store

### Files Modified
- `src-tauri/src/commands/window.rs` — Added `set_cursor_passthrough` command
- `src/stores/window.ts` — Added `setCursorPassthrough` method
- `src/stores/window.test.ts` — 3 click-through tests

### New Tauri Commands
`set_cursor_passthrough`

---

## Chunk 052 — Multi-Monitor Pet Mode

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Pet mode window spans all connected displays. Character can be dragged between monitors.

### Architecture
- `get_all_monitors` — queries `available_monitors()`, returns MonitorInfo vec
- `set_pet_mode_bounds` — calculates bounding rect spanning all monitors, sets window position/size
- Frontend `loadMonitors()` / `spanAllMonitors()` in window store

### Files Modified
- `src-tauri/src/commands/window.rs` — Added `get_all_monitors`, `set_pet_mode_bounds` commands
- `src/stores/window.ts` — Added monitor methods
- `src/stores/window.test.ts` — 3 monitor tests

### New Tauri Commands
`get_all_monitors` · `set_pet_mode_bounds`

---

## Chunk 053 — Streaming LLM Responses

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Modify OllamaAgent to use streaming API. Emit Tauri events for each text chunk. Character
starts "talking" animation on first chunk (not after full response).

### Architecture
- `send_message_stream` command — streams from Ollama `/api/chat` with `stream: true`,
  emits `llm-chunk` Tauri events with `{ text, done }` payload
- Falls back to stub response (single chunk + done) when no brain is configured
- Adds complete assistant message to conversation after stream finishes
- `stores/streaming.ts` — Pinia store tracking `isStreaming`, `streamText`, `streamRawText`,
  `currentEmotion`, `currentMotion`. `handleChunk()` parses emotion/motion tags from each chunk.
- System prompt updated with emotion/motion tag instructions

### Files Created
- `src-tauri/src/commands/streaming.rs` — Streaming command + 4 Rust tests
- `src/stores/streaming.ts` — Pinia streaming store
- `src/stores/streaming.test.ts` — 11 Vitest tests

### Files Modified
- `src-tauri/src/commands/mod.rs` — Added streaming module
- `src-tauri/src/commands/chat.rs` — Added SYSTEM_PROMPT_FOR_STREAMING constant
- `src-tauri/src/brain/ollama_agent.rs` — Added `infer_sentiment_static()` public method
- `src-tauri/src/lib.rs` — Registered `send_message_stream` command

### New Tauri Commands
`send_message_stream` (emits `llm-chunk` events)

---

## Chunk 054 — Emotion Tags in LLM Responses

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
System prompt instructs brain to tag emotions: `[happy] text`. Parse and strip tags before
display. Map to VRM expressions. Support optional motion tags `[motion:wave]`.

### Architecture
- Rust `commands/emotion.rs` — `EmotionTag` enum (happy/sad/angry/relaxed/surprised/neutral),
  `ParsedChunk` struct, `parse_tags()` and `strip_tags()` functions
- Frontend `utils/emotion-parser.ts` — Same parsing logic in TypeScript for streaming chunks
- Streaming store integrates emotion parser: `currentEmotion` and `currentMotion` refs updated
  on each chunk

### Files Created
- `src-tauri/src/commands/emotion.rs` — Emotion parser + 18 Rust tests
- `src/utils/emotion-parser.ts` — TypeScript emotion parser
- `src/utils/emotion-parser.test.ts` — 20 Vitest tests

### Files Modified
- `src-tauri/src/commands/mod.rs` — Added emotion module
- `src/types/index.ts` — Added EmotionTag, MotionTag, ParsedLlmChunk types

### Test Counts (Phase 5 total)
- **Rust:** 25 new tests (window 4 + streaming 4 + emotion 18) — 305 total
- **Vitest:** 46 new tests (window 15 + streaming 11 + emotion 20) — 246 total across 22 files

---

## Chunk 055 — Free LLM API Provider Registry & OpenAI-Compatible Client

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Curate a free LLM API provider catalogue from awesome-free-llm-apis. Build a generic
OpenAI-compatible chat client that works for all providers (POST `/v1/chat/completions`
with SSE streaming). Create a three-tier `BrainMode` enum (FreeApi / PaidApi / LocalOllama)
with JSON persistence and legacy migration from `active_brain.txt`.

### Architecture
- `brain/free_api.rs` — `FreeProvider` struct with `id`, `display_name`, `base_url`, `model`,
  `rpm_limit`, `rpd_limit`, `requires_api_key`, `notes`. Curated catalogue of 8 providers:
  Groq, Cerebras, SiliconFlow, Mistral, GitHub Models, OpenRouter, NVIDIA NIM, Google Gemini.
- `brain/openai_client.rs` — `OpenAiClient` with `chat()` (non-streaming) and `chat_stream()`
  (SSE streaming with callback). Handles `data: {...}` SSE lines and `data: [DONE]` sentinel.
  Bearer auth when API key provided. Works with any OpenAI-compatible endpoint.
- `brain/brain_config.rs` — `BrainMode` enum with serde tagged JSON (`"mode":"free_api"` /
  `"mode":"paid_api"` / `"mode":"local_ollama"`). `load()` checks new `brain_config.json`
  first, falls back to legacy `active_brain.txt` for migration. `save()` writes JSON.
  `clear()` removes both new and legacy config files.
- `commands/brain.rs` — `list_free_providers`, `get_brain_mode`, `set_brain_mode` Tauri commands.
  `set_brain_mode` also updates legacy `active_brain` field for backwards compatibility.
- `AppState` gains `brain_mode: Mutex<Option<BrainMode>>` field, loaded on startup.
- Frontend `types/index.ts` — `FreeProvider` and `BrainMode` TypeScript types.
- Frontend `stores/brain.ts` — `fetchFreeProviders()`, `loadBrainMode()`, `setBrainMode()`.
  `hasBrain` computed now considers `brainMode` in addition to `activeBrain`.

### Files Created
- `src-tauri/src/brain/free_api.rs` — Free provider catalogue + 8 Rust tests
- `src-tauri/src/brain/openai_client.rs` — OpenAI-compatible client + 11 Rust tests
- `src-tauri/src/brain/brain_config.rs` — BrainMode config + 12 Rust tests

### Files Modified
- `src-tauri/src/brain/mod.rs` — Added free_api, openai_client, brain_config modules
- `src-tauri/src/commands/brain.rs` — Added 3 new Tauri commands + 2 Rust tests
- `src-tauri/src/lib.rs` — Registered new commands, added brain_mode to AppState
- `src/types/index.ts` — Added FreeProvider, BrainMode types
- `src/stores/brain.ts` — Added three-tier brain methods
- `src/stores/brain.test.ts` — Added 9 new Vitest tests

### New Tauri Commands
`list_free_providers` · `get_brain_mode` · `set_brain_mode`

### Test Counts (Phase 5.5 — Chunk 055)
- **Rust:** 33 new tests (free_api 8 + openai_client 11 + brain_config 12 + commands 2) — 361 total
- **Vitest:** 9 new tests — 264 total across 23 files

---

## Chunk 056+057 — Streaming BrainMode Routing, Auto-Selection & Wizard Redesign

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Route `send_message_stream` through BrainMode (free API SSE / paid API SSE / Ollama NDJSON).
Auto-configure free API when Tauri backend is unavailable (zero-setup). Redesign the brain
setup wizard as a three-tier selector (Free Cloud API / Paid Cloud API / Local Ollama).
Write a single consolidated E2E test for free LLM brain (to avoid spamming free providers in CI/CD).

### Architecture
- `streaming.rs` — Refactored into helper functions: `stream_openai_api()` (SSE for free/paid),
  `stream_ollama()` (NDJSON for local), `emit_stub_response()` (no brain fallback),
  `store_assistant_message()` (shared). Routes via `brain_mode` → `active_brain` → stub.
- `brain.ts` — `autoConfigureFreeApi()` sets `brainMode` to free_api/groq with fallback provider
  list. `isFreeApiMode` computed. `initialise()` catches Tauri errors and auto-defaults.
  `FALLBACK_FREE_PROVIDERS` constant for offline use.
- `App.vue` — `onMounted` catches `loadActiveBrain()` failure and calls `autoConfigureFreeApi()`,
  then also tries `loadBrainMode()`. Skips setup when any brain mode is configured.
- `BrainSetupView.vue` — Three-tier wizard: Step 0 (choose tier), Step 1A (free provider list),
  Step 1B (paid API credentials), Step 1C (local hardware analysis), Steps 2-5 (local flow).
  Free API tier is pre-selected and highlighted with "Instant — no setup" badge.
- `ChatView.vue` — Inline brain card now shows "☁️ Use Free Cloud API (no setup)" button above
  the local Ollama section. Ollama warning only shown when local models are available.

### Files Modified
- `src-tauri/src/commands/streaming.rs` — Three-tier routing + 3 new Rust tests
- `src/stores/brain.ts` — autoConfigureFreeApi(), isFreeApiMode, FALLBACK_FREE_PROVIDERS
- `src/stores/brain.test.ts` — 5 new Vitest tests for auto-configure behavior
- `src/App.vue` — Auto-configure free API on Tauri failure
- `src/views/BrainSetupView.vue` — Three-tier wizard redesign
- `src/views/ChatView.vue` — Free API quick-start in inline brain card
- `e2e/app.spec.ts` — 1 consolidated E2E test (intentionally 1 test to avoid spamming free LLM providers in CI/CD)

### Test Counts (Phase 5.5 — Chunks 056+057)
- **Rust:** 3 new tests (streaming routing) — 364 total
- **Vitest:** 5 new tests (auto-configure) — 269 total across 23 files
- **E2E:** 1 new test (free LLM brain) — 28 total (27 existing + 1 new)

---

## Chunk 058 — Emotion Expansion & UI Fixes

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Extend the character emotion system from 5 states to 8 (adding angry, relaxed, surprised).
Fix VRM thumbnail cropping in model panel. Add welcome/empty state to chat. Focus on
different emotions and animations when the brain is installed.

### Architecture
- `types/index.ts` — CharacterState expanded: `'idle' | 'thinking' | 'talking' | 'happy' | 'sad' | 'angry' | 'relaxed' | 'surprised'`. Message sentiment expanded to include all 6 emotion tags.
- `animation-loader.ts` — PersonaAnimationData interface updated with angry/relaxed/surprised fields. States array expanded.
- `witch.json` + `idol.json` — 9 new animation variants (3 states × 3 variants each) with varied durations, loop_sin continuity, and natural bone rotation limits.
- `character-animator.ts` — STATE_EXPRESSIONS for new emotions (angry: 0.7 angry expression, relaxed: 0.6 relaxed + 0.15 happy, surprised: 0.8 surprised). Placeholder animations for all new states.
- `conversation.ts` — Persona fallback detects angry (angry/furious/frustrated), relaxed (relax/calm/peaceful), and surprised (surprise/wow/amazing) keywords.
- `ChatView.vue` — sentimentToState expanded to route all 6 emotions to character states.
- `CharacterViewport.vue` — State badge CSS for angry (red), relaxed (teal), surprised (amber).
- `ModelPanel.vue` — Thumbnail cropping fixed: `object-fit: cover` → `object-fit: contain`, size 40→56px, subtle background.
- `ChatMessageList.vue` — Welcome state shown when messages are empty: icon, title, hint text.

### Files Modified
- `src/types/index.ts` — CharacterState + Message sentiment expansion
- `src/renderer/animation-loader.ts` — PersonaAnimationData + states array
- `src/renderer/animations/witch.json` — 9 new animation variants
- `src/renderer/animations/idol.json` — 9 new animation variants
- `src/renderer/character-animator.ts` — STATE_EXPRESSIONS + placeholder animations
- `src/stores/conversation.ts` — Persona fallback emotion detection
- `src/views/ChatView.vue` — sentimentToState expansion
- `src/components/CharacterViewport.vue` — State badge CSS
- `src/components/ModelPanel.vue` — Thumbnail cropping fix
- `src/components/ChatMessageList.vue` — Welcome state

### Test Counts (Chunk 058)
- **Vitest:** 3 new tests (angry/relaxed/surprised placeholder) — 272 total across 23 files
- **E2E:** 4 new tests (angry/relaxed/surprised emotions + 8-emotion cycle) — 28 total
- **E2E fix:** Model selector option count 4→2

---

## Chunk 059 — Provider Health Check & Rate-Limit Rotation

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Implement automatic provider rotation when free LLM API providers are rate-limited.
Track per-provider usage, parse rate-limit headers, health-check all providers on startup,
and automatically fall back to the next healthy provider on HTTP 429 or quota exhaustion.

### Architecture

**Rust — `ProviderRotator`** (`src-tauri/src/brain/provider_rotator.rs`):
- `ProviderStatus` struct: tracks requests_sent, remaining_requests, remaining_tokens,
  rate_limit_reset, is_rate_limited, is_healthy, latency, last_health_check per provider.
- `ProviderRotator::new()` — pre-loads all providers from `free_provider_catalogue()`.
- `health_check_all()` — async parallel HEAD requests to all providers, records latency,
  sorts by response time (fastest first).
- `record_response_headers()` — parses `x-ratelimit-remaining-requests`,
  `x-ratelimit-remaining-tokens`, `x-ratelimit-reset` from HTTP response headers.
  Auto-marks as rate-limited when remaining reaches zero.
- `record_rate_limit()` — marks a provider as rate-limited (e.g., on HTTP 429).
- `next_healthy_provider()` — returns the fastest healthy, non-rate-limited provider.
  Auto-clears expired rate limits before selecting.
- `all_exhausted()` — returns true when all providers are unavailable.
- `clear_expired_limits()` — resets stale rate-limit flags after reset time passes.

**Rust Integration**:
- `AppState` gains `provider_rotator: Mutex<ProviderRotator>`.
- `streaming.rs` FreeApi path: uses rotator to select the best healthy provider.
  On 429/rate-limit errors, records the limit and emits `providers-exhausted` event
  if all providers are down. Successful requests increment the request count.
- `commands/brain.rs`: Two new Tauri commands — `health_check_providers` (returns
  `ProviderHealthInfo[]` with status of all providers) and `get_next_provider`
  (returns the next healthy provider ID).

**TypeScript**:
- `ProviderHealthInfo` type in `types/index.ts`.
- `useProviderHealthStore` Pinia store: wraps Tauri commands, provides browser-side
  rate-limit tracking (`markRateLimited`), `nextHealthyBrowserProvider()` for rotation
  in browser mode, `allExhausted` computed.
- Conversation store Path 2 (browser mode): on 429 errors, marks provider as
  rate-limited and retries with the next available provider from the catalogue.

**Also fixed: Brain-to-Conversation wiring** (the "I hear you" bug):
- Conversation store now has 3 paths: Tauri streaming IPC, browser-side free API
  streaming via fetch, and persona fallback (only when no brain is configured).
- `free-api-client.ts` — browser-side OpenAI-compatible SSE streaming client.
- ChatView wires up Tauri `llm-chunk` event listener for live streaming display.
- ChatMessageList shows live streaming bubble with cursor blink animation.

### Files Created
- `src-tauri/src/brain/provider_rotator.rs` — ProviderRotator with health check + rotation
- `src/stores/provider-health.ts` — Pinia store for provider health tracking
- `src/stores/provider-health.test.ts` — 12 tests for provider health store
- `src/utils/free-api-client.ts` — browser-side OpenAI SSE streaming client
- `src/utils/free-api-client.test.ts` — 7 tests for the free API client

### Files Modified
- `src-tauri/src/brain/mod.rs` — register provider_rotator module
- `src-tauri/src/lib.rs` — add provider_rotator to AppState + register commands
- `src-tauri/src/commands/brain.rs` — ProviderHealthInfo struct + 2 new commands
- `src-tauri/src/commands/streaming.rs` — use rotator for provider selection + error handling
- `src/types/index.ts` — ProviderHealthInfo interface
- `src/stores/conversation.ts` — three-path brain routing with provider rotation
- `src/stores/conversation.test.ts` — rewritten tests for brain-aware flow
- `src/views/ChatView.vue` — Tauri event listener + streaming display
- `src/components/ChatMessageList.vue` — streaming bubble + cursor blink

### Test Counts (Chunk 059)
- **Rust:** 23 new tests (provider_rotator) — 387 total
- **Vitest:** 24 new tests (12 provider-health, 7 free-api-client, 5 conversation) — 296 total across 25 files
- **Build:** `npm run build` ✓, `cargo test --lib` ✓, `cargo clippy` ✓

---

## Chunk 060 — Voice Abstraction Layer + Open-LLM-VTuber Integration

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Complete the Voice Abstraction Layer (Phase 6) with frontend voice setup wizard and
Open-LLM-VTuber integration. Users can choose their preferred voice provider — same
philosophy as the brain system where users pick their own LLM model.

### Architecture

**Rust — Voice Provider Catalogue** (`src-tauri/src/voice/mod.rs`):
- Added Open-LLM-VTuber as both ASR and TTS provider in the catalogue.
- ASR providers: stub, web-speech, whisper-api, sidecar-asr, open-llm-vtuber (5 total).
- TTS providers: stub, edge-tts, openai-tts, sidecar-tts, open-llm-vtuber (5 total).
- All existing Tauri commands (list_asr_providers, list_tts_providers, get_voice_config,
  set_asr_provider, set_tts_provider, set_voice_api_key, set_voice_endpoint,
  clear_voice_config) already wired and registered.

**TypeScript — Types** (`src/types/index.ts`):
- `VoiceProviderInfo` interface matching Rust struct.
- `VoiceConfig` interface matching Rust VoiceConfig.

**TypeScript — Voice Store** (`src/stores/voice.ts`):
- `useVoiceStore` Pinia store wrapping all voice Tauri commands.
- Fallback provider catalogues for browser-side use when Tauri unavailable.
- Computed: `hasVoice`, `isTextOnly`, `selectedAsrProvider`, `selectedTtsProvider`.
- Actions: `initialise`, `setAsrProvider`, `setTtsProvider`, `setApiKey`,
  `setEndpointUrl`, `clearConfig`.

**TypeScript — Open-LLM-VTuber Client** (`src/utils/ollv-client.ts`):
- `OllvClient` WebSocket client implementing Open-LLM-VTuber's protocol.
- Outgoing messages: text-input, mic-audio-data, mic-audio-end, interrupt-signal.
- Incoming messages: audio (with lip-sync volumes), user-input-transcription,
  full-text, conversation-chain-start/end, interrupt-signal, control.
- `OllvClient.healthCheck()` static method for connection verification.
- Default URL: `ws://localhost:12393/client-ws`.
- All message types fully typed with TypeScript interfaces.

**Vue — VoiceSetupView** (`src/views/VoiceSetupView.vue`):
- Step-by-step wizard mirroring BrainSetupView.vue UX pattern.
- Step 0: Choose voice mode (Open-LLM-VTuber recommended, Browser, Cloud API, Text Only).
- Step 1A: Open-LLM-VTuber config with WebSocket URL + health check.
- Step 1B: Browser voice (Web Speech API).
- Step 1C: Cloud API with API key and ASR/TTS checkboxes.
- Done screen with confirmation.
- Install instructions for Open-LLM-VTuber included.

**App Integration** (`src/App.vue`):
- Added 🎤 Voice tab to navigation.
- VoiceSetupView mounted when voice tab is active.
- Returns to chat tab on completion.

### Open-LLM-VTuber Integration Details
- Studied Open-LLM-VTuber's WebSocket protocol (websocket_handler.py).
- Frontend sends text or audio via WS, server processes through its LLM/TTS/ASR pipeline.
- Server returns audio with lip-sync volumes for mouth animation.
- Supports 18+ TTS engines (Edge, OpenAI, ElevenLabs, CosyVoice, etc.).
- Supports 7+ ASR engines (Faster Whisper, Groq, sherpa-onnx, etc.).
- Each client gets unique context and can connect independently.

### Files Created
- `src/stores/voice.ts` — Pinia store for voice configuration
- `src/stores/voice.test.ts` — 12 tests for voice store
- `src/utils/ollv-client.ts` — Open-LLM-VTuber WebSocket client
- `src/utils/ollv-client.test.ts` — 19 tests for OLLV client
- `src/views/VoiceSetupView.vue` — Voice setup wizard

### Files Modified
- `src-tauri/src/voice/mod.rs` — Added open-llm-vtuber to ASR + TTS catalogues
- `src/types/index.ts` — VoiceProviderInfo + VoiceConfig interfaces
- `src/App.vue` — Added Voice tab + VoiceSetupView integration
- `rules/milestones.md` — Marked chunk 060 done, updated Next Chunk to 061
- `rules/completion-log.md` — This entry

### Test Counts (Chunk 060)
- **Vitest:** 31 new tests (12 voice store, 19 OLLV client) — 329 total across 27 files
- **Build:** `npm run build` ✓

---

## Chunk 061 — Web Audio Lip Sync

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Create a provider-agnostic LipSync class that maps audio volume to VRM mouth morph
targets (aa, oh). Works with any TTS audio output via Web Audio API AnalyserNode.
Integrate with CharacterAnimator so external lip-sync values override the procedural
sine-wave mouth animation.

### Architecture

**LipSync Class** (`src/renderer/lip-sync.ts`):
- `LipSync` class using Web Audio API `AnalyserNode`.
- `connectAudioElement(audio)` — connects to an HTMLAudioElement via
  `createMediaElementSource`, pipes through AnalyserNode to destination.
- `connectAnalyser(analyser)` — connects to an external AnalyserNode.
- `getMouthValues()` — reads `getFloatTimeDomainData()`, calculates RMS volume,
  maps to `{ aa, oh }` morph targets with configurable sensitivity + threshold.
- `mouthValuesFromVolume(volume)` — static method for Open-LLM-VTuber's pre-computed
  volume arrays. Converts a single volume level to mouth values.
- Options: `fftSize`, `smoothingTimeConstant`, `silenceThreshold`, `sensitivity`.
- `disconnect()` — releases AudioContext and source resources.

**CharacterAnimator Integration** (`src/renderer/character-animator.ts`):
- Added `setMouthValues(aa, oh)` method — accepts external lip-sync values.
- Added `clearMouthValues()` — reverts to procedural sine-wave animation.
- When `useExternalLipSync` is true, talking state uses external aa/oh values
  instead of procedural sine wave. Also applies `oh` morph for rounding.
- Backward compatible — when no external lip-sync is provided, falls back to
  the existing sine-wave mouth animation.

### Files Created
- `src/renderer/lip-sync.ts` — LipSync class with Web Audio API integration
- `src/renderer/lip-sync.test.ts` — 14 tests for LipSync

### Files Modified
- `src/renderer/character-animator.ts` — setMouthValues/clearMouthValues, external lip-sync support
- `rules/milestones.md` — Marked chunk 061 done, updated Next Chunk to 062
- `rules/completion-log.md` — This entry

### Test Counts (Chunk 061)
- **Vitest:** 14 new tests (lip-sync) — 343 total across 28 files
- **Build:** `npm run build` ✓

---

## Chunk 062 — Voice Activity Detection

**Date:** 2026-04-13
**Status:** ✅ Done

### Goal
Browser-side voice activity detection using @ricky0123/vad-web (ONNX WebAssembly).
Detect speech start → pause AI audio and capture mic. Detect speech end → audio data
available for ASR. Echo cancellation support via mic management.

### Architecture

**VAD Composable** (`src/utils/vad.ts`):
- `useVad()` Vue composable using @ricky0123/vad-web MicVAD.
- Dynamic import of @ricky0123/vad-web — ONNX model only loaded when voice is used.
- Reactive state: `micOn`, `isSpeaking`, `lastProbability`, `error`.
- Callbacks: `onSpeechStart`, `onSpeechEnd(audio)`, `onMisfire`, `onFrameProcessed(prob)`.
- Configurable: `positiveSpeechThreshold` (0.5), `negativeSpeechThreshold` (0.35),
  `redemptionMs` (300ms).
- `startMic()` — creates MicVAD instance, starts microphone capture.
- `stopMic()` — pauses + destroys VAD, releases mic.
- Auto-cleanup on component unmount via `onUnmounted`.

**Open-LLM-VTuber Integration**:
- Speech audio (Float32Array 16kHz) from `onSpeechEnd` can be sent directly to
  Open-LLM-VTuber via `OllvClient.sendAudioChunk()` + `sendAudioEnd()`.
- The `onSpeechStart` callback can pause TTS playback (echo cancellation).
- Matches Open-LLM-VTuber-Web's VAD context pattern.

### Files Created
- `src/utils/vad.ts` — useVad composable with @ricky0123/vad-web
- `src/utils/vad.test.ts` — 14 tests for VAD composable

### Dependencies Added
- `@ricky0123/vad-web@0.0.30` — ONNX-based voice activity detection (no advisories)

### Files Modified
- `package.json` — Added @ricky0123/vad-web dependency
- `rules/milestones.md` — Marked chunk 062 done, updated Next Chunk to 063
- `rules/completion-log.md` — This entry

### Test Counts (Chunk 062)
- **Vitest:** 14 new tests (VAD) — 357 total across 29 files
- **Build:** `npm run build` ✓

---

## Chunk 063 — Remove Open-LLM-VTuber + Rewrite Voice in Rust (done)

**Date:** 2026-04-13
**Goal:** Remove all Open-LLM-VTuber WebSocket integration and replace with
pure Rust implementations for TTS (Edge TTS) and ASR (OpenAI Whisper API).

### Architecture

- **OLLV Removal:** Deleted `ollv-client.ts` (WebSocket client to Open-LLM-VTuber).
  Removed 'external' provider kind. Voice system now has only 'local' and 'cloud' kinds.
- **Edge TTS (Rust):** `src-tauri/src/voice/edge_tts.rs` — uses `msedge-tts` crate
  (sync WebSocket to Microsoft Edge Read Aloud API, wrapped in `spawn_blocking` for
  Tokio compatibility). Outputs PCM→WAV 24kHz 16-bit mono. Free, no API key.
- **Whisper API (Rust):** `src-tauri/src/voice/whisper_api.rs` — uses `reqwest`
  multipart form POST to OpenAI `/v1/audio/transcriptions`. Requires API key.
- **VoiceSetupView:** Simplified from 4-tier (OLLV/Browser/Cloud/Text) to 3-tier
  (Browser/Cloud/Text). Browser mode now uses Edge TTS for output (was text-only).

### Files Created
- `src-tauri/src/voice/edge_tts.rs` — Edge TTS engine (TtsEngine trait impl)
- `src-tauri/src/voice/whisper_api.rs` — Whisper API engine (AsrEngine trait impl)

### Files Modified
- `src/utils/ollv-client.ts` — **DELETED**
- `src/utils/ollv-client.test.ts` — **DELETED**
- `src/stores/voice.ts` — Removed OLLV from fallback providers, added Edge TTS
- `src/stores/voice.test.ts` — Rewritten without OLLV, new cloud API tests
- `src/types/index.ts` — Removed 'external' kind from VoiceProviderInfo
- `src/views/VoiceSetupView.vue` — Removed OLLV wizard step
- `src/renderer/lip-sync.ts` — Removed OLLV references in comments
- `src/utils/vad.ts` — Removed OLLV pattern reference
- `src-tauri/src/voice/mod.rs` — Removed OLLV from catalogues, added new modules
- `src-tauri/src/commands/voice.rs` — Updated kind validation ('local'/'cloud' only)
- `src-tauri/src/voice/config_store.rs` — Updated test fixture
- `src-tauri/Cargo.toml` — Added msedge-tts, reqwest multipart+rustls-tls features

### Dependencies Added
- `msedge-tts@0.3.0` (Rust) — Microsoft Edge TTS WebSocket client (no advisories)
- `reqwest` features: `multipart`, `rustls-tls` (already a dependency, added features)

### Test Counts (Chunk 063)
- **Vitest:** 338 total across 28 files (was 357; OLLV test file deleted, voice tests rewritten)
- **Rust:** 395 total (was 387; +4 edge_tts tests, +4 whisper_api tests)
- **Build:** `npm run build` ✓ · `cargo clippy` clean

---

## Chunk 064 — Desktop Pet Overlay with Floating Chat (done)

**Date:** 2026-04-13
**Goal:** Implement desktop pet mode — the main feature of Open-LLM-VTuber —
natively in Tauri/Vue without any external dependency. Character floats on
the desktop as a transparent overlay with a floating chat box.

### Architecture

- **PetOverlayView.vue:** Full-screen transparent overlay containing:
  - VRM character in bottom-right corner (CharacterViewport)
  - Floating speech bubble showing latest assistant message
  - Expandable chat panel (left side) with recent messages + input
  - Hover-reveal controls: 💬 toggle chat, ✕ exit pet mode
  - Emotion badge showing character state
  - Cursor passthrough when chat is collapsed (clicks go to desktop)
- **App.vue integration:** New `isPetMode` computed from `windowStore.mode`.
  When `pet`, renders PetOverlayView instead of normal tabbed UI.
  🐾 button in nav bar (Tauri-only) toggles pet mode.
  Body background switches to transparent in pet mode.
- **Existing Rust backend:** Already has `set_window_mode`, `toggle_window_mode`,
  `set_cursor_passthrough`, `set_pet_mode_bounds` commands (from earlier chunks).
  tauri.conf.json already has `transparent: true`.

### Files Created
- `src/views/PetOverlayView.vue` — Desktop pet overlay component
- `src/views/PetOverlayView.test.ts` — 9 tests

### Files Modified
- `src/App.vue` — Added PetOverlayView, 🐾 toggle, pet mode routing
- `rules/milestones.md` — Updated Next Chunk, Phase 6 note
- `rules/completion-log.md` — This entry

### Test Counts (Chunk 064)
- **Vitest:** 347 total across 29 files (+9 PetOverlayView tests)
- **Rust:** 395 total (unchanged)
- **Build:** `npm run build` ✓

---

## Chunk 065 — Design System & Global CSS Variables (done)

**Date:** 2026-04-14
**Status:** ✅ Done

### Goal
Create a unified design system with CSS custom properties to eliminate hardcoded
colors, spacing, and typography values scattered across components. Establish
reusable utility classes for buttons, inputs, cards, badges, and labels.

### Architecture

**Design System** (`src/style.css`):
- `:root` CSS custom properties for: surface palette (7 vars), brand accent (6 vars),
  semantic colors (5 vars), text hierarchy (5 vars), borders (3 vars), radius (5 vars),
  spacing (5 vars), shadows (3 vars), transitions (3 vars), typography (7 vars).
- Global utility classes: `.ts-btn` (with modifiers: `-primary`, `-blue`, `-violet`,
  `-success`, `-ghost`, `-danger`), `.ts-input`, `.ts-card`, `.ts-label`, `.ts-badge`.

**Components Updated**:
- `App.vue` — Uses CSS vars for nav, surfaces, active indicators.
- `ChatView.vue` — Brain card, status bar, buttons use design tokens.
- `ChatInput.vue` — Input field and send button use design tokens.
- `CharacterViewport.vue` — Settings dropdown, badges, debug overlay use tokens.

### Files Modified
- `src/style.css` — Complete design system with CSS custom properties
- `src/App.vue` — Migrated to CSS vars, added active tab indicator + tooltip labels
- `src/views/ChatView.vue` — Migrated to CSS vars
- `src/components/ChatInput.vue` — Migrated to CSS vars
- `src/components/CharacterViewport.vue` — Migrated to CSS vars, responsive dropdown
- `rules/milestones.md` — Updated Next Chunk, added Phase 6.5
- `rules/completion-log.md` — This entry

### Test Counts (Chunk 065)
- **Vitest:** 371 total across 30 files (was 354; +8 markdown tests, +9 background tests)
- **Build:** `npm run build` ✓

---

## Chunk 066 — New Background Art (done)

**Date:** 2026-04-14
**Status:** ✅ Done

### Goal
Expand the background scene library from 3 to 7 with visually rich SVG
backgrounds that add atmosphere and variety to the character viewport.

### Architecture

**New SVG Backgrounds** (`public/backgrounds/`):
1. **Cyberpunk City** — Dark purple cityscape with neon building silhouettes,
   magenta/cyan light strips, window lights, floor glow.
2. **Enchanted Forest** — Night forest with moonlight, tree silhouettes,
   firefly particles, green ground glow.
3. **Deep Ocean** — Underwater scene with caustic light rays, bioluminescent
   particles, seafloor, depth gradient.
4. **Cosmic Nebula** — Space scene with purple/pink/cyan nebula clouds,
   star field, bright star, dust band.

**Background Store** (`src/stores/background.ts`):
- `PRESET_BACKGROUNDS` expanded from 3 to 7 entries.
- All backgrounds follow the same `BackgroundOption` interface with `preset` kind.

### Files Created
- `public/backgrounds/cyberpunk-city.svg`
- `public/backgrounds/enchanted-forest.svg`
- `public/backgrounds/deep-ocean.svg`
- `public/backgrounds/cosmic-nebula.svg`
- `src/stores/background.test.ts` — 9 tests for background store

### Files Modified
- `src/stores/background.ts` — Added 4 new preset backgrounds

### Test Counts (Chunk 066)
- **Vitest:** 371 total across 30 files (+9 background store tests)
- **Build:** `npm run build` ✓

---

## Chunk 067 — Enhanced Chat UX (done)

**Date:** 2026-04-14
**Status:** ✅ Done

### Goal
Improve chat message rendering with lightweight markdown support, enhanced
welcome screen with suggestion chips, and XSS-safe HTML escaping.

### Architecture

**Markdown Renderer** (`ChatMessageList.vue`):
- Lightweight inline markdown: `**bold**`, `*italic*`, `` `code` ``,
  ` ```code blocks``` `. No external dependency.
- `escapeHtml()` sanitizes all content before markdown processing (XSS prevention).
- Uses `v-html` with pre-escaped content for safe rendering.
- `:deep()` scoped styles for markdown elements (`.md-code-block`, `.md-inline-code`).

**Welcome Screen Enhancement**:
- Sparkle icon (✨) with drop shadow glow.
- Radial glow behind welcome text using accent color.
- Suggestion chips: 3 starter prompts that emit `suggest` event.
- ChatView listens to `@suggest` and sends as message.

### Files Modified
- `src/components/ChatMessageList.vue` — Markdown renderer, welcome screen, suggestions
- `src/components/ChatMessageList.test.ts` — +8 tests (bold, italic, code, blocks, XSS, welcome, suggest)
- `src/views/ChatView.vue` — Wired `@suggest` event

### Test Counts (Chunk 067)
- **Vitest:** 371 total across 30 files (+8 markdown/welcome tests)
- **Build:** `npm run build` ✓

---

## Chunk 068 — Navigation Polish & Micro-interactions (done)

**Date:** 2026-04-14
**Status:** ✅ Done

### Goal
Add polish to navigation and UI interactions: active tab indicators, tooltip
labels, thinking badge pulse, responsive dropdown, brand-consistent hover
effects.

### Architecture

**Navigation Improvements** (`App.vue`):
- Active tab indicator: 3px accent-colored bar on the left edge (desktop),
  bottom edge (mobile).
- Hover tooltip: CSS `::before` pseudo-element shows `title` text on hover.
  Hidden on mobile to avoid overlap with bottom bar.
- Hover scale animation on nav buttons (1.06x).

**Viewport Improvements** (`CharacterViewport.vue`):
- Thinking state badge has pulsing box-shadow animation (`badge-pulse`).
- State badge transitions smoothly between states (0.3s color/bg transition).
- Settings toggle hover shows accent glow shadow.
- Background chips have `translateY(-1px)` hover lift effect.
- Settings dropdown: `max-width: min(280px, 90vw)` prevents overflow on tablets.
- Loading spinner uses accent color instead of generic blue.

**Chat Toggle** (`ChatView.vue`):
- Toggle button hover now shows accent glow shadow.
- Active state uses accent color instead of generic blue.

### Files Modified
- `src/App.vue` — Active indicator, tooltip, hover animations
- `src/components/CharacterViewport.vue` — Badge pulse, responsive dropdown, glow effects
- `src/views/ChatView.vue` — Toggle button glow

### Test Counts (Chunk 068)
- **Vitest:** 371 total across 30 files (unchanged)
- **Build:** `npm run build` ✓

---

## Chunk 080 — Pose Preset Library (done)

**Date:** 2026-04-14
**Status:** ✅ Done

### Goal
Define 10 VRM humanoid pose presets as TypeScript data, covering the full
emotional range: confident, shy, excited, thoughtful, relaxed, defensive,
attentive, playful, bored, empathetic.

### Architecture

**Pose Presets** (`src/renderer/pose-presets.ts`):
- `PosePreset` interface: `{ id, label, boneRotations: Partial<Record<string, PoseBoneRotation>> }`
- `PoseBoneRotation`: `{ x, y, z }` Euler angles in radians
- Sparse representation — only bones that deviate from neutral are listed
- 10 presets, each touching 3–8 upper-body bones
- `getAllPosePresets()` and `getPosePreset(id)` accessors
- `EMOTION_TO_POSE` mapping: each CharacterState maps to default pose blend weights
- `VALID_POSE_BONES` set for validation

**Types** (`src/types/index.ts`):
- `PoseBoneRotation` — `{ x, y, z }` Euler rotation
- `PoseBlendInstruction` — `{ presetId: string, weight: number }`

### Files Created/Modified
- `src/renderer/pose-presets.ts` — Pose preset library
- `src/renderer/pose-presets.test.ts` — 15 tests
- `src/types/index.ts` — `PoseBoneRotation`, `PoseBlendInstruction` types added

### Test Counts (Chunk 080)
- **Vitest:** 15 new tests in pose-presets.test.ts

---

## Chunk 081 — Pose Blending Engine (done)

**Date:** 2026-04-14
**Status:** ✅ Done

### Goal
`PoseBlender` class applies weighted-average pose offsets to VRM normalized
bone nodes, with smooth interpolation (exponential decay, BLEND_SPEED = 4).

### Architecture

**PoseBlender** (`src/renderer/pose-blender.ts`):
- `currentWeights: Map<string, number>` — smoothed live weights
- `targetWeights: Map<string, number>` — target weights set by `setTarget()`
- `setTarget(instructions)` — set blend targets, fades others to 0
- `reset()` — immediately clears all weights
- `apply(vrm, delta)` — interpolates weights, computes weighted-average Euler
  offsets per bone, multiplies onto `node.quaternion`
- Integration point: called in `CharacterAnimator.applyVRMAnimation()` AFTER
  `mixer.update(delta)` and BEFORE `vrm.update(delta)`

**CharacterAnimator integration** (`src/renderer/character-animator.ts`):
- `poseBlender` instance field
- `setPoseBlend(instructions)` — explicit LLM-driven blend
- `clearPoseBlend()` — revert to emotion→pose fallback
- `setState()` now triggers default pose blend from `EMOTION_TO_POSE`
- `hasExplicitPose` flag: LLM pose overrides emotion fallback

### Files Created/Modified
- `src/renderer/pose-blender.ts` — PoseBlender class
- `src/renderer/pose-blender.test.ts` — 13 tests
- `src/renderer/character-animator.ts` — PoseBlender integrated

### Test Counts (Chunk 081)
- **Vitest:** 13 new tests in pose-blender.test.ts

---

## Chunk 082 — LLM Pose Prompt Engineering (done)

**Date:** 2026-04-14
**Status:** ✅ Done

### Goal
Extend the emotion-tag parser to also handle `[pose:presetId=weight,...]` tags.
Update system prompt to instruct LLM on all available pose presets and format.
Propagate parsed pose blend instructions through the streaming store.

### Architecture

**Parser extension** (`src/utils/emotion-parser.ts`):
- `parsePoseTag(body)` — parses `confident=0.6,attentive=0.3` into
  `PoseBlendInstruction[]`; clamps weights to [0,1]
- `parseTags()` now returns `poseBlend: PoseBlendInstruction[] | null`
- Uses broader `[^\]]+` regex (vs previous `[\w:]+`) to match `=` and `,`
- First `[pose:...]` tag wins; second is stripped

**Streaming store** (`src/stores/streaming.ts`):
- `currentPoseBlend` reactive ref added
- Reset on `sendStreaming()` / `reset()`
- Updated in `handleChunk()` when `parsed.poseBlend` is set

**System prompt** (`src/utils/free-api-client.ts`):
- Documents all 10 pose presets and the tag format
- Lists all 8 motion/gesture ids in the motion tag description
- `streamChatCompletion()` accepts optional `poseContextSuffix` parameter

### Files Modified
- `src/utils/emotion-parser.ts` — `[pose:...]` parsing
- `src/utils/emotion-parser.test.ts` — +11 pose tag tests
- `src/types/index.ts` — `poseBlend` field in `ParsedLlmChunk`
- `src/stores/streaming.ts` — `currentPoseBlend` ref
- `src/utils/free-api-client.ts` — extended system prompt, optional suffix

### Test Counts (Chunk 082)
- **Vitest:** 11 new tests in emotion-parser.test.ts (pose tag suite)

---

## Chunk 083 — Gesture Tag System (done)

**Date:** 2026-04-14
**Status:** ✅ Done

### Goal
10 built-in gesture sequences (keyframe-based), `GesturePlayer` class with
a queue, integrated into `CharacterAnimator` as an additive layer above pose blending.

### Architecture

**Gesture definitions** (`src/renderer/gestures.ts`):
- `GestureDefinition`: `{ id, duration, keyframes: GestureKeyframe[] }`
- `GestureKeyframe`: `{ time, bones: Partial<Record<string, {x,y,z}>> }`
- 10 built-in gestures: `nod`, `wave`, `shrug`, `lean-in`, `head-tilt`,
  `reach-out`, `bow`, `nod-slow`, `shake-head`, `idle-fidget`
- `getAllGestures()` and `getGesture(id)` accessors

**GesturePlayer** (`src/renderer/gesture-player.ts`):
- `play(gestureId)` — start immediately or queue (max depth 4)
- `stop()` — clear active + queue
- `apply(vrm, delta)` — advance elapsed, interpolate keyframes, apply offsets
- Linear interpolation between adjacent keyframes
- `isPlaying`, `currentId`, `queueLength` getters
- Integration: called in `CharacterAnimator.applyVRMAnimation()` after pose blending

**CharacterAnimator integration** (`src/renderer/character-animator.ts`):
- `gesturePlayer` instance field
- `playGesture(gestureId)` → delegates to `gesturePlayer.play()`
- `stopGesture()` → `gesturePlayer.stop()`
- `isGesturePlaying` getter

### Files Created/Modified
- `src/renderer/gestures.ts` — Gesture library (10 gestures)
- `src/renderer/gesture-player.ts` — GesturePlayer class
- `src/renderer/gesture-player.test.ts` — 18 tests
- `src/renderer/character-animator.ts` — GesturePlayer integrated

### Test Counts (Chunk 083)
- **Vitest:** 18 new tests in gesture-player.test.ts

---

## Chunk 084 — Autoregressive Pose Feedback (done)

**Date:** 2026-04-14
**Status:** ✅ Done

### Goal
Serialize current pose state to a compact descriptor injected into the LLM
system prompt, enabling coherent animation decisions across conversation turns.

### Architecture

**Pose feedback serializer** (`src/utils/pose-feedback.ts`):
- `PoseContextInput`: `{ weights: Map<string, number>, lastGestureId, secondsSinceLastGesture }`
- `serializePoseContext(input)` → compact string e.g.
  `"Current character pose: thoughtful=0.75. Last gesture: nod (3.2s ago)."`
- Filters presets below 0.05 threshold, sorts by weight, limits to 3 presets
- Rounds weights to 2 decimal places for readability
- Output always < 200 chars
- `buildPoseContextSuffix(input)` → wraps output with `\n\n[Character state] ...`
  for system prompt injection

**System prompt integration** (`src/utils/free-api-client.ts`):
- `streamChatCompletion()` accepts `poseContextSuffix = ''` (optional 6th param)
- Appends suffix to system prompt content when provided

### Files Created/Modified
- `src/utils/pose-feedback.ts` — Serializer
- `src/utils/pose-feedback.test.ts` — 14 tests
- `src/utils/free-api-client.ts` — `poseContextSuffix` parameter

### Test Counts (Chunk 084)
- **Vitest:** 14 new tests in pose-feedback.test.ts

---

## Phase 8 Summary

**Date:** 2026-04-14
**Chunks:** 080–084
**Status:** ✅ Complete

- 10 pose presets with emotion→pose fallback mapping
- PoseBlender: smooth weighted-average blend with exponential interpolation
- `[pose:...]` tag parsing in emotion-parser + streaming store propagation
- 10 built-in gesture sequences with queuing in GesturePlayer
- Autoregressive pose context serialization for LLM system prompt injection
- System prompt updated with full pose/gesture/motion documentation
- **438 total Vitest tests across 34 files** (+67 new tests for Phase 8)
- Build: `npm run build` ✓

---

## Chunk 085 — UI/UX Overhaul (Open-LLM-VTuber Layout Patterns)

**Date:** 2026-04-14
**Status:** ✅ Done
**Source:** Learned from Open-LLM-VTuber-Web (React/Electron) — adapted to Vue 3/Tauri.

### Goal
Transform the stacked viewport+chat layout into a modern full-screen character experience
with floating glass overlays. Key patterns adopted from Open-LLM-VTuber:
1. Character canvas fills the entire viewport (not squeezed to 55%).
2. Chat panel is a slide-over drawer from the right (not a fixed bottom panel).
3. Input bar is a collapsible floating footer.
4. AI response text appears as a floating subtitle on the canvas.
5. AI state shown as an animated glassmorphism pill (not a plain text badge).

### Architecture Changes
- **ChatView.vue** — Complete layout restructure:
  - Viewport fills 100% of parent, positioned absolutely as z-index 0.
  - All UI elements (brain setup, subtitle, state pill, input, chat drawer) float on top.
  - New subtitle system: `showSubtitle()` displays truncated AI response text with 8s auto-dismiss.
  - State labels: human-readable labels ("Thinking…", "Happy") instead of raw state strings.
  - Streaming watcher updates subtitle in real-time.
- **CharacterViewport.vue** — Removed `state-badge` element and all its CSS (67 lines removed).
  State indicator now lives in ChatView as the new `ai-state-pill`.
- **New UI components:**
  - `.subtitle-overlay` — Centered floating text with glassmorphism, 65% width, animated entry/exit.
  - `.ai-state-pill` — 8 color variants with animated dot, glassmorphism background.
  - `.input-footer` — Collapsible bar with chevron toggle, slides down when collapsed.
  - `.chat-drawer` — 380px slide-over from right with header, close button, shadow.
  - `.brain-overlay` — Brain setup card now centered on screen instead of inline.
  - `.brain-status-pill` — Compact pill centered at top instead of full-width bar.

### Files Modified
- `src/views/ChatView.vue` — Template, script, and styles completely overhauled.
- `src/components/CharacterViewport.vue` — Removed state-badge element and CSS.

### Test Counts (Chunk 085)
- **Vitest:** 438 tests across 34 files — all pass (no test changes needed)
- Build: `npm run build` ✓
