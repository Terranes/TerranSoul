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

---

## Phase 1 — Chat-First, 3D Character, Text Only

> **Goal:** Deliver a working desktop application showing a chat UI + 3D character viewport
> with text-only messaging routed through an agent stub.
> Desktop first (Windows), then macOS/Linux, then mobile.

### Next Chunk

**Chunk 002** — Chat UI Polish & Vitest Component Tests

---

### Chunks

| Chunk | Description | Status |
|-------|-------------|--------|
| 002 | **Chat UI Polish & Vitest Component Tests** — Refine `ChatMessageList`, `ChatInput`, `TypingIndicator`, `AgentBadge` visual styles. Add Vitest + @vue/test-utils. Write component tests for all 4 chat components (render, props, emit, disabled state). Add `npm run test` script. Target: ≥ 12 component tests passing. CI `vitest` job will automatically pick these up. | `not-started` |
| 003 | **Three.js Scene Polish + WebGPU Detection** — Enhance `scene.ts`: attempt `WebGPURenderer` via `navigator.gpu` detection; fall back to `WebGLRenderer`. Add resize observer so canvas adapts to window resizes. Add `renderer.info` debug overlay toggled by `Ctrl+D`. Verify 60fps on desktop with the capsule placeholder. | `not-started` |
| 004 | **VRM Model Loading & Fallback** — Harden `vrm-loader.ts`: handle corrupt/missing VRM files gracefully (error boundary → capsule fallback). Add loading progress callback. Expose loaded VRM metadata (title, author, license) to the character store. Write Vitest unit tests for the loader error paths. | `not-started` |
| 005 | **Character State Machine Tests** — Add `#[tokio::test]` Rust unit tests for `stub_agent.rs` (all 4 keyword branches + neutral). Add Vitest tests for `character-animator.ts` state transitions (idle→thinking→talking→idle, happy, sad). Target: ≥ 8 tests. | `not-started` |
| 006 | **Rust Chat Commands — Unit Tests** — Add `#[tokio::test]` tests for `commands/chat.rs`: `send_message` with stub agent (success, empty input error), `get_conversation` ordering. Mock `AppState` via trait injection. Target: ≥ 6 Rust tests. | `not-started` |
| 007 | **Agent Orchestrator Hardening** — Add `AgentProvider` trait to `src-tauri/src/agent/mod.rs`. Implement `AgentOrchestrator::dispatch()` using the trait (not a direct `StubAgent` reference). Add health-check ping method to `AgentProvider`. Write unit tests for orchestrator routing. Target: ≥ 4 Rust tests. | `not-started` |
| 008 | **Tauri IPC Bridge Integration Tests** — Wire up the frontend conversation store to use real `invoke()` calls against the Rust backend. Use `@tauri-apps/api/mocks` to mock the IPC layer in Vitest. Write integration tests that simulate a full send → response round-trip. Target: ≥ 4 integration tests. | `not-started` |
| 009 | **Playwright E2E Test Infrastructure** — Install `@playwright/test`. Create `playwright.config.ts` (baseURL: Vite dev server, projects: chromium). Write first E2E tests: app loads, chat input visible, send a message and receive stub agent response, 3D viewport canvas renders. CI `playwright-e2e` job will automatically detect and run these. Target: ≥ 4 E2E tests passing in CI. | `not-started` |
| 010 | **Character Reactions — Full Integration** — Connect `character-animator.ts` update loop to the Three.js clock delta. Implement per-state visual animations: idle (subtle sway), thinking (head bob), talking (mouth-open BlendShape if VRM loaded, else scale pulse), happy (bounce), sad (droop). Verify animations play correctly at 60fps. | `not-started` |
| 011 | **VRM Import + Character Selection UI** — Add `CharactersView.vue` (import VRM file via Tauri file dialog, list imported characters, set active character). Wire `load_vrm` Tauri command to persist the VRM path in app state. Show character name and thumbnail in the viewport overlay. | `not-started` |

---

## Phase 2 — TerranSoul Link (Cross-Device)

> **Goal:** TerranSoul on all devices behaves like "one assistant."
> Pair devices, sync conversations, route commands remotely.

| Chunk | Description | Status |
|-------|-------------|--------|
| 020 | **Device Identity & Pairing** — Generate Ed25519 key pair per device on first launch (stored in OS keychain via Tauri `keyring` plugin). Implement QR-code-based pairing handshake (display QR on one device, scan on another). Persist trusted device list. | `not-started` |
| 021 | **Link Transport Layer** — Implement QUIC transport using `quinn` crate (primary). Implement WebSocket+TLS fallback using `tokio-tungstenite`. Abstract behind a `LinkTransport` trait. Write unit tests for connection establishment and reconnect logic. | `not-started` |
| 022 | **CRDT Sync Engine** — Implement CRDT-based sync for: conversation log (append-only log), character selection (last-write-wins register), agent status map (OR-Set). Use `crdts` crate or implement minimal LWW/OR-Set. Write unit tests for merge correctness (concurrent edits on 2 devices). | `not-started` |
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
