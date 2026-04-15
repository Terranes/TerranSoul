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

## Completed Phases

✅ Chunk 001 — Project Scaffold — see `rules/completion-log.md`

37 files created. Tauri 2.0 + Vue 3 + TypeScript + Three.js + @pixiv/three-vrm + Pinia.
Rust backend: chat/agent/character commands, stub agent, orchestrator.
`npm run build` and `cargo check` both pass.

✅ CI Restructure — see `rules/completion-log.md`

Consolidated 5 separate CI jobs (frontend-build, rust-build, tauri-build, vitest, playwright-e2e)
into 3 jobs (build-and-test, vitest, playwright-e2e). Removed `pull_request` trigger to eliminate
double-firing on copilot branches. Added `paths` filter so CI only runs when source files change.
Modeled after [devstress/My3DLearning eip-ci.yml](https://github.com/devstress/My3DLearning/blob/main/.github/workflows/eip-ci.yml).

✅ Chunk 002 — Chat UI Polish & Vitest Component Tests — see `rules/completion-log.md`

Polished visual styles for all 4 chat components. Added Vitest + @vue/test-utils + jsdom.
26 component tests across 4 test files. `npm run test` passes. CI `vitest` job added.

✅ Chunk 003 — Three.js Scene Polish + WebGPU Detection — see `rules/completion-log.md`

WebGPU renderer with WebGL fallback. ResizeObserver for canvas resize. Debug overlay (Ctrl+D).
WebGPU chunk is code-split via dynamic import.

✅ Chunk 004 — VRM Model Loading & Fallback — see `rules/completion-log.md`

Hardened vrm-loader.ts with error handling, progress callback, VRM 0.0/1.0 metadata extraction.
Safe loader returns null on error (capsule fallback). 12 VRM loader tests.

✅ Chunk 005 — Character State Machine Tests — see `rules/completion-log.md`

7 Rust tests for stub_agent (name, hello, hi, sad, happy, neutral). 9 Vitest tests for
character-animator (state transitions, animations, error handling). Total: 16 new tests.

✅ Chunk 006 — Rust Chat Commands — Unit Tests — see `rules/completion-log.md`

8 Rust tests for chat commands. Refactored to extract testable `process_message` and
`fetch_conversation` functions. Added empty input validation.

✅ Chunk 007 — Agent Orchestrator Hardening — see `rules/completion-log.md`

`AgentProvider` trait with `respond`, `health_check`, `id`, `name`. Orchestrator uses trait-based
dispatch with agent registry. 8 orchestrator tests with MockAgent.

✅ Chunk 010 — Character Reactions — Full Integration — see `rules/completion-log.md`

Sentiment-driven character reactions. BlendShape mouth animation for VRM talking. Head bone
animations for thinking/sad. Scale pulse for placeholder. 6 new Vitest tests.

✅ Chunk 011 — VRM Import + Character Selection UI — see `rules/completion-log.md`

ModelPanel.vue for VRM import and character selection. CharacterViewport watches vrmPath
to auto-load models. Character metadata displayed in viewport overlay. 8 ModelPanel tests.

✅ Chunk 008 — Tauri IPC Bridge Integration Tests — see `rules/completion-log.md`

12 store integration tests with mocked invoke(). Conversation store: round-trip, error,
isThinking, getConversation, sentiment, ordering, custom agent. Character store: loadVrm,
resetCharacter, error handling.

✅ Chunk 009 — Playwright E2E Test Infrastructure — see `rules/completion-log.md`

6 E2E tests with Playwright + Chromium. App loads, chat input, send message, 3D canvas,
state badge, model panel toggle. CI `playwright-e2e` job added.

---

## Phase 1 — Chat-First, 3D Character, Text Only

> **Goal:** Deliver a working desktop application showing a chat UI + 3D character viewport
> with text-only messaging routed through an agent stub.
> Desktop first (Windows), then macOS/Linux, then mobile.

✅ Phase 1 complete — see completion-log.md

✅ Chunk 020 — Device Identity & Pairing — see `rules/completion-log.md`

Ed25519 key pair per-device, file-backed key storage, QR SVG pairing code, trusted device list.
5 Tauri commands, 16 Rust tests, 9 Vitest tests.

✅ Chunk 021 — Link Transport Layer — see `rules/completion-log.md`

QUIC primary + WebSocket fallback behind `LinkTransport` trait. Link manager with auto-reconnect
and transport fallback. 4 Tauri commands, 31 Rust tests, 11 Vitest tests.

✅ Chunk 022 — CRDT Sync Engine — see `rules/completion-log.md`

Append-only log (conversation), LWW register (character selection), OR-Set (agent status).
HLC timestamps with site tiebreaker. 37 Rust tests, 8 Vitest tests.

✅ Chunk 023 — Remote Command Routing — see `rules/completion-log.md`

Command envelope, permission management (Allow/Deny/Ask), router with pending approval queue.
5 Tauri commands, 31 Rust tests, 10 Vitest tests.

## Phase 9 — Learned Features (From Reference Projects) — High Priority

> **Source repos:** Open-LLM-VTuber, AI4Animation-js, VibeVoice, aituber-kit

✅ Chunk 106 — Streaming TTS — see `rules/completion-log.md`

`synthesize_tts` Tauri command, `useTtsPlayback` composable (sentence queuing, sequential playback),
wired into ChatView.vue. Voice starts ~200ms after first sentence. 13 Vitest tests + 4 Rust tests.

✅ Chunk 107 — Multi-ASR Provider Abstraction — see `rules/completion-log.md`

`groq-whisper` provider added. `transcribe_audio` Tauri command (float32 PCM → Whisper/Groq/stub).
`useAsrManager` composable (web-speech + Tauri IPC paths). Mic button in ChatView. Groq mode in VoiceSetupView.
13 Vitest tests + 8 Rust tests.

✅ Chunk 108 — Settings Persistence + Env Overrides — see `rules/completion-log.md`

`AppSettings` struct (JSON + schema validation + `TERRANSOUL_MODEL_ID` env override). `get_app_settings` /
`save_app_settings` Tauri commands. `useSettingsStore` Pinia store. Model selection + camera azimuth/distance
persisted. CharacterViewport restores camera on mount. ChatView loads persisted model on mount.
9 Vitest tests + 11 Rust unit tests.

✅ Chunk 109 — Idle Action Sequences — see `rules/completion-log.md`

`useIdleManager` composable: 45s idle timeout, shuffled greeting pool (5 variants, round-robin),
repeat every 90s. Blocked when character is thinking/streaming. Wired into ChatView.vue.
10 Vitest tests.

| Chunk | Description | Status |
|-------|-------------|--------|
| 110 | **Background Music** | `not-started` |

### Next Chunk

→ **Chunk 110** — Background Music

---

---

## Phase 9.1 — UI/UX (Open-LLM-VTuber Patterns)

✅ Chunk 085 — UI/UX Overhaul — see `rules/completion-log.md`

Full-screen character layout, floating subtitle overlay, collapsible input footer,
animated AI state pill, slide-over chat drawer. Learned from Open-LLM-VTuber-Web.

---

## Phase 6.5 — UI Polish, UX Refinement & Art

> **Goal:** Elevate TerranSoul's visual identity and user experience with a unified design
> system, richer background art, markdown-rendered chat, and polished navigation micro-interactions.

| Chunk | Description | Status |
|-------|-------------|--------|
| (Chunks 065–068 complete — Design system, backgrounds, chat UX, navigation polish.) | | |

---

## Phase 5.5 — Three-Tier Brain (Free API / Paid API / Local LLM)

> **Goal:** Make TerranSoul work out of the box with zero setup by defaulting
> to free cloud LLM APIs. Users can optionally upgrade to paid APIs or local
> Ollama. Free providers are sourced from awesome-free-llm-apis and auto-rotated
> when rate-limited. See `rules/research-reverse-engineering.md` §8.

✅ Phase 5.5 complete — see completion-log.md

---

## Phase 4 — Brain & Memory (Local LLM + Persistent Memory)

✅ Chunk 040 — Brain (Local LLM via Ollama) — see `rules/completion-log.md`

Hardware analysis, tiered model recommendations (Gemma 3, Phi-4 Mini, TinyLlama).
OllamaAgent with full conversation history. 7 Tauri commands. BrainSetupView.vue wizard.
38 Rust tests, 11 Vitest tests.

✅ Chunk 041 — Long/Short-term Memory + Brain-powered recall — see `rules/completion-log.md`

SQLite-backed MemoryStore (rusqlite). Brain reuses active Ollama model for:
- Automatic memory extraction from sessions
- Session summarization into memory entries
- Semantic memory search (LLM-ranked, keyword fallback)
Short-term memory = last 20 messages injected into each Ollama call.
9 Tauri commands. MemoryView.vue + MemoryGraph.vue (cytoscape.js).
14 Rust tests + 10 Vitest tests.


---

## Phase 2 — TerranSoul Link (Cross-Device)

> **Goal:** TerranSoul on all devices behaves like "one assistant."
> Pair devices, sync conversations, route commands remotely.

✅ Phase 2 complete — see completion-log.md

---

## Phase 3 — AI Package Manager & Agent Marketplace

> **Goal:** Install, update, and remove AI agents as packages across devices.
> Community agent registry with one-command install.

✅ Chunk 030 — Package Manifest Format — see `rules/completion-log.md`

AgentManifest schema, parser, validation, 3 Tauri commands, Pinia store.
28 Rust tests, 10 Vitest tests.

✅ Chunk 031 — Install / Update / Remove Commands — see `rules/completion-log.md`

RegistrySource trait, MockRegistry, PackageInstaller, SHA-256 verification.
4 new Tauri commands, 24 new Rust tests, 8 new Vitest tests.

✅ Chunk 032 — Agent Registry — see `rules/completion-log.md`

axum 0.8 in-process registry server, HttpRegistry, 3 official agents (stub-agent, openclaw-bridge, claude-cowork).
4 Tauri commands, 8 Rust tests, 8 Vitest tests.

✅ Chunk 033 — Agent Sandboxing — see `rules/completion-log.md`

wasmtime 36.0.7 (Cranelift), CapabilityStore (file-backed JSON consent), HostContext + capability-gated host API, WasmRunner.
5 Tauri commands, 12 Rust tests, 12 Vitest tests.

✅ Phase 3 complete — see completion-log.md

---

## Phase 5 — Desktop Experience (Overlay & Streaming)

> **Goal:** Transform the desktop window into a proper overlay companion.
> Dual-mode window (normal + pet mode), selective click-through, multi-monitor,
> streaming LLM responses, emotion-driven character reactions.
> Patterns learned from Open-LLM-VTuber and aituber-kit — see `rules/research-reverse-engineering.md`.

✅ Phase 5 complete — see completion-log.md

---

## Phase 6 — Voice (User-Defined ASR/TTS)

> **Goal:** Add voice input/output. Users choose their own voice provider — same
> philosophy as the brain system where users pick their own LLM model.
> TerranSoul provides the abstraction layer; users bring their preferred engine.
> Reference implementations studied: VibeVoice, sherpa-onnx, Edge TTS, OpenAI Whisper — see `rules/research-reverse-engineering.md`.

| Chunk | Description | Status |
|-------|-------------|--------|
| (Phase 6 complete — chunks 060–064 done. Open-LLM-VTuber removed; Edge TTS + Whisper API in pure Rust; desktop pet overlay.) | | |

---

## Phase 7 — VRM Model Security (Anti-Exploit & Asset Protection)

📦 Moved back to `rules/backlog.md` — chunks 100–105 (renumbered). Do not start until user says so.

---

## Phase 8 — Brain-Driven Animation (AI4Animation for VRM)

> **Goal:** Use the LLM brain as an animation controller. Instead of pre-baked
> keyframe clips, the brain generates pose parameters (blend weights, bone
> offsets, gesture tags) that drive VRM character animation in realtime.
> Inspired by AI4Animation-js (SIGGRAPH 2018 MANN), adapted for stationary
> VRM desktop companion use. See `rules/research-reverse-engineering.md` §7.

✅ Phase 8 complete (chunks 080–084) — see completion-log.md
