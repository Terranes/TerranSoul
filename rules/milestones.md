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

### Next Chunk

**Chunk 023** — Remote Command Routing (Phase 2)

---

## Phase 2 — TerranSoul Link (Cross-Device)

> **Goal:** TerranSoul on all devices behaves like "one assistant."
> Pair devices, sync conversations, route commands remotely.

| Chunk | Description | Status |
|-------|-------------|--------|
| 023 | **Remote Command Routing** — Allow a secondary device (phone) to send a command to the primary device (PC). Implement command envelope: `{command_id, origin_device, target_device, payload}`. Implement permission check on target device (user must approve first remote command). Return result to originating device. | `not-started` |

---

## Phase 3 — AI Package Manager & Agent Marketplace

> **Goal:** Install, update, and remove AI agents as packages across devices.
> Community agent registry with one-command install.

| Chunk | Description | Status |
|-------|-------------|--------|
| 030 | **Package Manifest Format** — Define agent manifest schema (name, version, description, system_requirements, install_method, capabilities, ipc_protocol_version). Implement manifest parser in Rust (`serde_json`). Write unit tests for valid/invalid manifests. | `not-started` |
| 031 | **Install / Update / Remove Commands** — Implement `terransoul install <agent>`, `update <agent>`, `remove <agent>`, `list` CLI commands via Tauri shell plugin. Download, verify hash/signature, and install agent binary. Write integration tests with a local mock registry. | `not-started` |
| 032 | **Agent Registry** — Stand up a minimal registry server (Rust `axum`) that serves manifest JSON and binary downloads. Implement `terransoul search <query>` command. Host official agents (stub, OpenClaw bridge). | `not-started` |
| 033 | **Agent Sandboxing** — Run community agents inside a WASM sandbox (via `wasmtime`). Expose a capability-gated host API: file access, clipboard, network — each requires explicit user consent recorded in settings. Write security tests verifying capability enforcement. | `not-started` |
