# TerranSoul — Completion Log

> This file is the permanent record of all completed chunks.
> `rules/milestones.md` contains only chunks that are `not-started` or `in-progress`.
> When a chunk is done, its full details are recorded here and the row is removed from milestones.md.

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
