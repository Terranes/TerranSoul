# TerranSoul — Milestones

> **To continue development, tell the AI agent:**
>
> ```
> Continue
> ```
>
> The agent will read this file, find the next chunk with status `not-started`,
> implement it, update the status to `done`, update `Next Chunk`, and log details
> in `rules/completion-log.md`.

> **ENFORCEMENT RULE — Completed chunks must be archived.**
>
> When a chunk is marked `done`:
> 1. Log full details (date, goal, architecture, files created/modified, test counts) in `rules/completion-log.md`.
> 2. **Remove the done row from this file.** `milestones.md` contains only `not-started` chunks.
> 3. If an entire phase has no remaining rows, replace the table with: `✅ Phase N complete — see completion-log.md`.
> 4. Update the `Next Chunk` section to point to the next `not-started` chunk.
>
> This rule is mandatory for every AI agent session. Never leave done rows in milestones.md.

---

## Completed Work

All chunks listed here are fully implemented. See `rules/completion-log.md` for full details.

### Foundation (Phase 0)
- ✅ Chunk 001 — Project Scaffold — see `completion-log.md`
- ✅ CI Restructure — Consolidated 5 CI jobs → 3; push-only trigger; paths filter — see `completion-log.md`
- ✅ Chunk 002 — Chat UI Polish & Vitest Component Tests — see `completion-log.md`
- ✅ Chunk 003 — Three.js Scene Polish + WebGPU Detection — see `completion-log.md`
- ✅ Chunk 004 — VRM Model Loading & Fallback — see `completion-log.md`
- ✅ Chunk 005 — Character State Machine Tests — see `completion-log.md`
- ✅ Chunk 006 — Rust Chat Commands — Unit Tests — see `completion-log.md`
- ✅ Chunk 007 — Agent Orchestrator Hardening — see `completion-log.md`
- ✅ Chunk 008 — Tauri IPC Bridge Integration Tests — see `completion-log.md`
- ✅ Chunk 009 — Playwright E2E Test Infrastructure — see `completion-log.md`
- ✅ Chunk 010 — Character Reactions — Full Integration — see `completion-log.md`
- ✅ Chunk 011 — VRM Import + Character Selection UI — see `completion-log.md`

### Phase 1 — Chat-First, 3D Character, Text Only
✅ Phase 1 complete — see `completion-log.md`

### Phase 2 — TerranSoul Link (Cross-Device)
- ✅ Chunk 020 — Device Identity & Pairing — see `completion-log.md`
- ✅ Chunk 021 — Link Transport Layer — see `completion-log.md`
- ✅ Chunk 022 — CRDT Sync Engine — see `completion-log.md`
- ✅ Chunk 023 — Remote Command Routing — see `completion-log.md`

### Phase 3 — AI Package Manager & Agent Marketplace
- ✅ Chunk 030 — Package Manifest Format — see `completion-log.md`
- ✅ Chunk 031 — Install / Update / Remove Commands — see `completion-log.md`
- ✅ Chunk 032 — Agent Registry — see `completion-log.md`
- ✅ Chunk 033 — Agent Sandboxing — see `completion-log.md`
- ✅ Chunk 034 — Agent Marketplace UI — see `completion-log.md`
- ✅ Chunk 035 — Agent-to-Agent Messaging — see `completion-log.md`

### Phase 4 — Brain & Memory (Local LLM + Persistent Memory)
- ✅ Chunk 040 — Brain (Local LLM via Ollama) — see `completion-log.md`
- ✅ Chunk 041 — Long/Short-term Memory + Brain-powered Recall — see `completion-log.md`

### Phase 5 — Desktop Experience (Overlay & Streaming)
- ✅ Chunk 050 — Window Mode System — see `completion-log.md`
- ✅ Chunk 051 — Selective Click-Through — see `completion-log.md`
- ✅ Chunk 052 — Multi-Monitor Pet Mode — see `completion-log.md`
- ✅ Chunk 053 — Streaming LLM Responses — see `completion-log.md`
- ✅ Chunk 054 — Emotion Tags in LLM Responses — see `completion-log.md`

### Phase 5.5 — Three-Tier Brain (Free API / Paid API / Local LLM)
- ✅ Chunk 055 — Free LLM API Provider Registry & OpenAI-Compatible Client — see `completion-log.md`
- ✅ Chunk 056+057 — Streaming BrainMode Routing, Auto-Selection & Wizard Redesign — see `completion-log.md`
- ✅ Chunk 058 — Emotion Expansion & UI Fixes — see `completion-log.md`
- ✅ Chunk 059 — Provider Health Check & Rate-Limit Rotation — see `completion-log.md`

### Phase 6 — Voice (User-Defined ASR/TTS)
- ✅ Chunk 060 — Voice Abstraction Layer + Open-LLM-VTuber Integration — see `completion-log.md`
- ✅ Chunk 061 — Web Audio Lip Sync — see `completion-log.md`
- ✅ Chunk 062 — Voice Activity Detection — see `completion-log.md`
- ✅ Chunk 063 — Remove Open-LLM-VTuber + Rewrite Voice in Rust — see `completion-log.md`
- ✅ Chunk 064 — Desktop Pet Overlay with Floating Chat — see `completion-log.md`

### Phase 6.5 — UI Polish, UX Refinement & Art
- ✅ Chunk 065 — Design System & Global CSS Variables — see `completion-log.md`
- ✅ Chunk 066 — New Background Art — see `completion-log.md`
- ✅ Chunk 067 — Enhanced Chat UX — see `completion-log.md`
- ✅ Chunk 068 — Navigation Polish & Micro-interactions — see `completion-log.md`

### Phase 7 — VRM Model Security (Anti-Exploit & Asset Protection)
📦 Chunks 100–105 moved to `rules/backlog.md` — do not start until explicitly requested.

### Phase 8 — Brain-Driven Animation (AI4Animation for VRM)
- ✅ Chunk 080 — Pose Preset Library — see `completion-log.md`
- ✅ Chunk 081 — Pose Blending Engine — see `completion-log.md`
- ✅ Chunk 082 — LLM Pose Prompt Engineering — see `completion-log.md`
- ✅ Chunk 083 — Gesture Tag System — see `completion-log.md`
- ✅ Chunk 084 — Autoregressive Pose Feedback — see `completion-log.md`

### Phase 9.1 — UI/UX (Open-LLM-VTuber Patterns)
- ✅ Chunk 085 — UI/UX Overhaul — see `completion-log.md`

### Phase 9 — Learned Features (High Priority)
- ✅ Chunk 094 — Model Position Saving — see `completion-log.md`
- ✅ Chunk 095 — Procedural Gesture Blending (MANN-inspired) — see `completion-log.md`
- ✅ Chunk 096 — Speaker Diarization — see `completion-log.md`
- ✅ Chunk 097 — Hotword-Boosted ASR — see `completion-log.md`
- ✅ Chunk 098 — Presence / Greeting System — see `completion-log.md`
- ✅ Chunk 106 — Streaming TTS — see `completion-log.md`
- ✅ Chunk 107 — Multi-ASR Provider Abstraction — see `completion-log.md`
- ✅ Chunk 108 — Settings Persistence + Env Overrides — see `completion-log.md`
- ✅ Chunk 109 — Idle Action Sequences — see `completion-log.md`
- ✅ Chunk 110 — Background Music — see `completion-log.md`

### Phase 9 — Learned Features (Lower Priority)
- ✅ Chunk 115 — Live2D Support — see `completion-log.md`
- ✅ Chunk 116 — Screen Recording / Vision — see `completion-log.md`
- 📦 Chunk 117 — Docker Containerization — demoted to `rules/backlog.md` (Tauri desktop apps don't use Docker)
- ✅ Chunk 118 — Chat Log Export — see `completion-log.md`
- ✅ Chunk 119 — Language Translation Layer — see `completion-log.md`

### Phase 10 — Avatar Animation Architecture (VRM Expression-Driven)
- ✅ Chunk 120 — AvatarState Model + Animation State Machine — see `completion-log.md`
- ✅ Chunk 121 — Exponential Damping Render Loop — see `completion-log.md`
- ✅ Chunk 122 — 5-Channel VRM Viseme Lip Sync — see `completion-log.md`
- ✅ Chunk 123 — Audio Analysis Web Worker — see `completion-log.md`
- ✅ Chunk 124 — Decouple IPC from Animation — Coarse State Bridge — see `completion-log.md`
- ✅ Chunk 125 — LipSync ↔ TTS Audio Pipeline — see `completion-log.md`
- ✅ Chunk 126 — On-demand Rendering + Idle Optimization — see `completion-log.md`

### Phase 11 — RPG Brain Configuration
- ✅ Chunk 128 — Constellation Skill Tree (Full-Screen Layout) — see `completion-log.md`
- ✅ Chunk 129 — Constellation Cluster Interaction & Detail Panel — see `completion-log.md`
- ✅ Chunk 130 — Brain RPG Stat Sheet — see `completion-log.md`
- ✅ Chunk 131 — Combo Notification Toast — see `completion-log.md`
- ✅ Chunk 132 — Quest Reward Ceremony — see `completion-log.md`
- ✅ Chunk 133 — Brain Evolution Path (neural pathway) — see `completion-log.md`
- ✅ Chunk 134 — Stat-Based AI Scaling — see `completion-log.md`

### Post-Phase Polish
- ✅ 3D Model Loading Robustness (error overlay, URL encoding, placeholder fallback) — see `completion-log.md`
- ✅ Streaming Timeout Fix (stuck "Thinking" state, 60s/30s race) — see `completion-log.md`
- ✅ Music Bar Redesign (always-visible play/stop button) — see `completion-log.md`
- ✅ Splash Screen (animated kawaii cat loading screen) — see `completion-log.md`
- ✅ BGM Track Replacement (JRPG-style: Crystal Theme, Starlit Village, Eternity) — see `completion-log.md`

---

## Active Chunks

### Phase 12 — Brain Advanced Design (Documentation & QA)

| # | Chunk | Status | Owner | Notes |
|---|---|---|---|---|
| 1.1 | Brain Advanced Design — Validation, Docs Rewrite, QA Walkthrough | in-progress | agent + user (screenshots) | Source tracking + cross-framework comparison table done; user-captured screenshots remain |
| 1.5 | Multi-Agent Roster + External CLI Workers (Codex/Claude) with Temporal-style Durable Workflows | done | agent | Implemented in PR series — see `rules/completion-log.md` |

#### Chunk 1.1 — Brain Advanced Design — Validation, Docs Rewrite, QA Walkthrough

**Goal.** Validate `docs/brain-advanced-design.md` against best-in-class
open-source references (Obsidian, SiYuan, RAGFlow), confirm the Phase-1
implementation in `src-tauri/src/memory/` + `src-tauri/src/commands/ingest.rs`
matches the design, and rewrite the user-facing docs around a single
end-to-end scenario (Vietnamese law portal `http://thuvienphapluat.vn/` +
internal-firm-rules PDF) so a fresh user can reproduce it step-by-step.

**Done in this PR (agent).**
- [x] Read `docs/brain-advanced-design.md` end-to-end and audited current code.
- [x] Wrote design-validation summary (Obsidian / SiYuan / RAGFlow comparison
      table) inside the rewritten walkthrough.
- [x] Confirmed baseline: `cargo test --all-targets` → 561/561 pass on
      `copilot/validate-advanced-design-and-implement`.
- [x] **Replaced** `instructions/BRAIN-COMPLEX-EXAMPLE.md` with a focused
      walkthrough of the thuvienphapluat.vn + PDF scenario.
- [x] **Replaced** `instructions/BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md` with a
      concise quick-reference.
- [x] **Source tracking pipeline** — Extended `NewMemory` + `MemoryEntry`
      with `source_url`, `source_hash`, `expires_at`; full hash-based
      dedup + staleness in `run_ingest_task`. **594 Rust / 948 Vitest pass.**
- [x] **Cross-framework comparison table** — Added a single consolidated
      table to `docs/brain-advanced-design.md` §13 contrasting TerranSoul
      against LangChain, Odyssey, RAGFlow, SiYuan and GitNexus across 14
      dimensions (purpose, distribution, storage, retrieval, graph, etc.).

**Remaining (user environment / follow-up).**
- [ ] Capture **scenario-specific** screenshots on a real Tauri build with
      Vietnamese content loaded.

---

#### Chunk 1.5 — Multi-Agent Roster + External CLI Workers (Codex/Claude) with Temporal-style Durable Workflows

**Goal.** TerranSoul today ships a single in-process companion built on its
own LLM stack. This chunk turns it into an **agent roster** where the user can
create, name, switch between, and delete multiple agents that may share or
have distinct VRM characters, and may be backed by either:

1. **TerranSoul's native brain** (current behaviour — Free / Paid / Local Ollama).
2. **An external CLI worker** that is given a working folder and runs
   `codex`, `claude`, `gemini`, or any other CLI agent on it.

Long-running CLI work is tracked through a **Temporal.io-style durable
workflow runtime** so progress survives app restarts, shows a live status,
and can be retried / resumed from the last successful step.

**Status.** `not-started` — planning only. Do not begin implementation
until the user explicitly says so.

**Scope (proposed).**

- **Agent roster model.** New `src-tauri/src/agents/` module:
  - `AgentProfile { id, display_name, vrm_model_id, brain_backend,
    created_at, last_active_at, working_folder?, cli_kind?,
    permissions, ram_budget_mb }`.
  - `BrainBackend = Native(BrainMode) | ExternalCli { kind: "codex" | "claude"
    | "gemini" | "custom", binary, extra_args }`.
  - Persisted as JSON under `<app_data_dir>/agents/<id>.json` so the roster
    survives reinstalls (consistent with the per-user pattern from Chunk 1.3).
- **Frontend.** New Pinia `useAgentRosterStore`; new `AgentRosterPanel.vue`
  in MarketplaceView; "switch agent" dropdown in the chat header. The
  active agent's `vrm_model_id` drives `useCharacterStore.selectModel(...)`,
  so switching agents also swaps the VRM (different agents can pick the
  same VRM — they are independent records).
- **External CLI worker — sandbox.**
  - `src-tauri/src/agents/cli_worker.rs` spawns the CLI as a child process,
    bound to the agent's `working_folder` (Tauri file dialog + persisted
    on the profile), captures stdout/stderr line-by-line, and emits
    `agent-cli-output` events.
  - Allow-list of binaries (`codex`, `claude`, `gemini`, plus user-added
    "custom" entries that resolve via PATH). Refuse arbitrary shell
    interpolation; arguments are passed as a `Vec<String>`.
  - The folder picked is the **only** filesystem capability granted —
    we don't widen the asset-protocol scope app-wide (matches the user-
    models design where bytes flow through commands rather than direct FS).
- **Durable workflow runtime ("inner Temporal").**
  - New `src-tauri/src/workflows/` module with a tiny embeddable workflow
    engine. We **do not** add a real Temporal server (heavy infra, JVM,
    Postgres, Cassandra). Instead we re-use `tokio` + `rusqlite` for an
    append-only event log identical in spirit to Temporal's history:
    - Event types: `WorkflowStarted`, `ActivityScheduled`,
      `ActivityCompleted`, `ActivityFailed`, `Heartbeat`,
      `WorkflowCompleted`, `WorkflowFailed`, `WorkflowResumed`.
    - On startup, every workflow whose latest event is not terminal is
      replayed so its in-memory `WorkflowHandle` reattaches to the
      already-running CLI process (or relaunches it via signal-based
      resume if the process died).
    - `WorkflowEngine::start(name, input)` returns a `WorkflowId`;
      `query_status(id)` returns the live step + heartbeat.
  - **No new heavy dependency.** Use `rusqlite` (already a dep), plus
    `tokio::sync::mpsc` for fan-out — keeps the binary single-process.
  - Reference inspiration only — Temporal workflow patterns:
    [docs.temporal.io/workflows](https://docs.temporal.io/workflows). We
    borrow the *append-only history + replay* pattern, **not** the server
    or SDK stack.
- **Tauri commands** (proposed):
  - `list_agents` / `create_agent` / `delete_agent` / `switch_agent`
  - `set_agent_working_folder(id, path)`
  - `start_cli_workflow(agent_id, prompt) -> workflow_id`
  - `query_workflow_status(workflow_id) -> WorkflowStatus`
  - `cancel_workflow(workflow_id)`
  - `list_recent_workflows(agent_id, limit)`
- **RAM-aware concurrency cap.** Re-use
  `src-tauri/src/brain/system_info.rs` (already exposes total/free RAM)
  to compute a max simultaneously-active agent count:
  - **Estimate per-agent footprint** by backend:
    - Native Free / Paid API → 200 MB (chat + RAG)
    - Native Local Ollama → 200 MB + the loaded model size from
      `model_recommender.rs`
    - External CLI worker → 600 MB headroom (codex/claude CLI typical)
  - **Cap = floor( (free_ram_mb − 1500 reserve) / per_agent_mb )** but
    never less than 1 and never more than 8.
  - When the user tries to activate an agent that would exceed the cap,
    the UI shows a dialog: "You have N MB free RAM; activating this agent
    would exceed safe limits. Suspend an active agent first." Active
    counts only agents currently streaming or with a running workflow.
  - Cap is recomputed each time the agent picker opens so adding more
    RAM (or closing other apps) is reflected without restart.
- **VRM behaviour.**
  - The active agent's `vrm_model_id` selects the avatar; the other agents
    keep their VRMs as metadata only (no GL load, no GPU cost).
  - Two agents may legitimately reference the same VRM — the model file
    is loaded once and shared via the existing GLTF loader cache.
- **Tests.**
  - Workflow engine: unit tests for replay-after-crash (kill + restart
    in-process, expect event log to recover state).
  - RAM cap: `compute_max_concurrent_agents(free_mb, agents)` is a pure
    function with table-driven tests.
  - Agent roster: serde round-trip, default agent backfill on first run.
- **Docs.**
  - `instructions/AGENT-ROSTER.md` — how to add an agent, point it at a
    folder, choose between native brain vs Codex / Claude CLI.
  - Update `docs/brain-advanced-design.md` §10 to mention the
    `ExternalCli` backend alongside the existing three brain modes.

**Acceptance criteria.**
- Multiple agents can be created, switched, and deleted; each has its own
  VRM (or shares one); switching the agent switches the on-screen avatar.
- An agent of kind `ExternalCli` can be pointed at a folder; running
  "explain this codebase" shells out to `codex` / `claude` and streams
  output into chat; killing the app mid-run and reopening shows the
  workflow as `Resuming` and continues without losing event history.
- The agent picker disables (with a clear hint) any agent whose
  activation would push the active set above the RAM-derived cap.
- Existing single-agent chat experience is preserved as the default
  agent — no UX regression for users who never open the roster.

---

> **Milestones backlog drained for completed phases.** All planned phase
> chunks are complete except the in-progress / planning entries above.
> Chunks 1.2, 1.3 and 1.4 were completed in this PR series — see
> `rules/completion-log.md`.
