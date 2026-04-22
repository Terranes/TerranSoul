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
| 1.1 | Brain Advanced Design — Validation, Docs Rewrite, QA Walkthrough | in-progress | agent + user (screenshots) | See details below |

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
      walkthrough of the thuvienphapluat.vn + PDF scenario (brain setup quest
      → URL crawl → PDF ingest → hybrid RAG → cross-source dedup → amendment
      conflict resolution → Obsidian export). Includes QA validation log,
      reproduction recipe, and code-path map.
- [x] **Replaced** `instructions/BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md` with a
      concise quick-reference (system map, code map, Tauri command table,
      schema cheat-sheet, hybrid-search formula, decay/GC formula, ingest
      pipeline, multi-source mechanics, sqlite3 debug recipes) that points
      to `docs/brain-advanced-design.md` as the canonical deep dive.
- [x] Embedded the existing real screenshots from
      `instructions/screenshots/01-fresh-launch.png` … `11-skill-tree.png`
      throughout the walkthrough.

**Remaining (user environment / follow-up).**
- [ ] Capture **scenario-specific** screenshots on a real Tauri build with
      Vietnamese content loaded (statute crawl progress, PDF ingest task,
      Memory list with `vn-law` tag, sourced RAG answer, conflict toast,
      Obsidian export folder). Replace the generic placeholders embedded
      from `instructions/screenshots/` once real ones exist.
- [ ] Optional: short MP4 walkthrough via
      `playwright codegen --record-video` wrapping
      `scripts/brain-flow-screenshots.mjs`; convert with
      `ffmpeg -i in.webm out.mp4`; commit under `recording/`.
- [ ] Phase-1 implementation gap noticed during validation:
      `commands/ingest.rs::run_ingest_task` does not yet populate
      `source_url` / `source_hash` on inserted chunks even though the V3
      schema columns exist. Wire this in (extend `NewMemory` with optional
      source fields, plumb through ingest, add unit tests for re-ingest
      skip + content-change replace) so design §12 staleness detection
      works end-to-end without a manual SQL workaround.

**Files touched.**
- `instructions/BRAIN-COMPLEX-EXAMPLE.md` (rewritten)
- `instructions/BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md` (rewritten)
- `rules/milestones.md` (this entry)

**Validation.**
- `cd src-tauri && cargo test --all-targets` → 561/561 pass.
- No code changes outside docs in this PR; markdown-only diff.

---

> **Milestones backlog drained.** All planned chunks are complete except
> the in-progress entry above.
> See `rules/backlog.md` for deferred / speculative future work.
> To add new chunks, describe the feature and a new numbered entry will be created here.
