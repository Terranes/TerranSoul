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

### Next Chunk

**Chunk 065** — Next phase TBD

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

> **Goal:** Prevent VRM model files from being publicly exposed, downloaded, or
> extracted — whether from the GitHub repository, the local filesystem after install,
> or the app's runtime network traffic. VRM creators' work must be protected.

### Problem Statement

Currently, all default VRM models (~68 MB total) are committed to `public/models/default/`
in plaintext `.vrm` files. This creates **three exploit vectors**:

1. **GitHub repo exposure** — Anyone can clone/download the repo and get the raw `.vrm` files.
2. **Local source code / installed app extraction** — After `npm run build`, VRM files are
   copied as-is to `dist/models/default/` and then bundled into the Tauri app. Users or
   attackers can browse the app resources folder and copy the files.
3. **Network/DevTools interception** — In dev mode the `.vrm` files are served over HTTP.
   Even in production, the WebView loads them from the local filesystem with predictable paths.

### Step-by-Step Configuration Plan

#### Step 1 — Remove VRM files from Git history

VRM files should **never** live in the Git repository.

```bash
# 1a. Add VRM and large binary patterns to .gitignore
echo "*.vrm" >> .gitignore
echo "public/models/default/*.vrm" >> .gitignore

# 1b. Remove tracked VRM files from the index (keeps local copies)
git rm --cached "public/models/default/*.vrm"

# 1c. (Optional) Purge from Git history entirely — use BFG Repo-Cleaner
#     This is a one-time destructive operation; coordinate with team.
bfg --delete-files '*.vrm' --no-blob-protection
git reflog expire --expire=now --all && git gc --prune=now --aggressive
```

#### Step 2 — Encrypted asset bundle (build-time)

Encrypt VRM files at build time so they are never stored in plaintext on disk.

```
Build pipeline:
  1. Keep VRM source files in a private location outside the repo
     (e.g., private GCS/S3 bucket, or a git-ignored local folder).
  2. A build script (scripts/encrypt-models.ts) reads each .vrm file,
     encrypts it with AES-256-GCM using a per-build key, and writes
     the output to public/models/default/<name>.vrm.enc
  3. The encryption key is embedded in the Rust binary (compiled in,
     not in config files) via a build.rs env var or Tauri resource.
  4. .vrm.enc files are committed or downloaded at CI time — never
     the raw .vrm files.
```

#### Key Distribution Architecture — How Clients Get the Decryption Key

> **Q: "This app is released as an installer with auto-update. How do clients
> know about the CI secret?"**
>
> **A: They don't.** The CI secret is compiled into the binary at build time.
> End-users never see, handle, or configure the key.

The encryption key flows through the build pipeline, not through user configuration:

```
┌─────────────────────────────────────────────────────────────────┐
│  GitHub Actions (CI)                                            │
│                                                                 │
│  Secret: VRM_ENCRYPTION_KEY  (stored in Settings → Secrets)     │
│          ↓                                                      │
│  build.rs reads the env var at compile time:                    │
│    let key = std::env::var("VRM_ENCRYPTION_KEY").unwrap();      │
│    println!("cargo:rustc-env=VRM_KEY={}", key);                 │
│          ↓                                                      │
│  Rust compiler embeds it into the binary:                       │
│    const KEY: &str = env!("VRM_KEY");                           │
│          ↓                                                      │
│  Tauri builds the installer (.msi / .dmg / .AppImage)           │
│  with the compiled binary that has the key baked in.            │
└─────────────────────────────────────────────────────────────────┘
          ↓ installer download / auto-update
┌─────────────────────────────────────────────────────────────────┐
│  User's Machine                                                 │
│                                                                 │
│  1. User installs TerranSoul (or receives auto-update).         │
│  2. The compiled Rust binary already contains the key.          │
│  3. At runtime, load_vrm_secure() reads .vrm.enc from disk,    │
│     decrypts in memory using the compiled-in key, and returns   │
│     raw bytes to the frontend.                                  │
│  4. User never sees, configures, or handles the key.            │
└─────────────────────────────────────────────────────────────────┘
```

**Key points:**

- The CI secret `VRM_ENCRYPTION_KEY` lives **only** in GitHub Actions (Settings → Secrets).
  It is never committed to source code, config files, or environment files.
- `build.rs` reads it at compile time and injects it via `cargo:rustc-env`.
  The `env!()` macro embeds the string into the compiled `.exe` / `.app` binary.
- The installer ships this compiled binary. On auto-update, Tauri's updater
  downloads a new binary that was also built by CI with the key embedded.
- **Security trade-off:** A determined reverse engineer *can* extract the key
  from the binary via disassembly or memory dump. This is an inherent limitation
  of any client-side DRM. We mitigate with defense-in-depth layers (Step 7:
  obfuscation, anti-tamper, zeroize). The goal is to make extraction **non-trivial**,
  not mathematically impossible — matching the approach used by Steam, Unity,
  and VRChat for their bundled assets.

#### Step 3 — Runtime decryption in Rust backend

The Tauri Rust backend decrypts models on demand, never writing plaintext to disk.

```
Runtime flow:
  1. Frontend requests a model via Tauri command: invoke('load_vrm_secure', { modelId })
  2. Rust handler reads the .vrm.enc file from the app's resource directory.
  3. Decrypts in memory using the compiled-in AES-256-GCM key.
  4. Returns the decrypted bytes to the frontend as a base64 data URI or
     an ArrayBuffer via Tauri's binary response.
  5. Frontend passes the ArrayBuffer directly to VRMLoaderPlugin.
  6. Plaintext .vrm bytes NEVER touch the filesystem.
```

#### Step 4 — Tauri Content Security Policy (CSP)

Lock down what the WebView can load and where data can be sent.

```json
// tauri.conf.json → app.security
{
  "csp": "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data: blob:; connect-src 'self' http://localhost:* https://localhost:*; object-src 'none'"
}
```

> **Note:** Avoid `'unsafe-inline'` for `style-src` — move all styles to external
> stylesheets or use nonces/hashes. If inline styles are temporarily needed during
> migration, scope the exception and track removal as a follow-up task.

This blocks the WebView from fetching remote URLs (prevents exfiltration) and
restricts resource loading to `'self'` and localhost (for Ollama/sidecar).

#### Step 5 — Disable DevTools in production builds

Prevent users from using browser DevTools to inspect network requests or memory.

```json
// tauri.conf.json → app.windows[0]
{
  "devtools": false
}
```

In Rust, additionally strip the devtools feature from the production build by
gating it behind `#[cfg(debug_assertions)]`.

#### Step 6 — Tauri resource scope restrictions

Use Tauri's file system scope to restrict which files the WebView can access.

```json
// tauri.conf.json → app.security — use default-deny approach
{
  "assetScope": {
    "allow": ["$RESOURCE/models/**/*.vrm.enc", "$RESOURCE/icons/**"],
    "deny": ["$RESOURCE/**"]
  }
}
```

> **Note:** Use a default-deny approach — deny everything, then explicitly allow
> only the encrypted model files and other required resources. This is more secure
> than relying on specific deny patterns for `.vrm` files which could be bypassed
> if files are placed in unexpected directories.

#### Step 7 — Obfuscation & anti-tamper (defense in depth)

Additional hardening for production builds:

- **Code obfuscation**: Use `vite-plugin-obfuscator` or `terser` mangling to
  make it harder to find decryption routines in the JS bundle.
- **Integrity checks**: At startup, Rust computes SHA-256 of each `.vrm.enc`
  file and compares against compiled-in hashes. Reject tampered files.
- **Memory protection**: After decryption, zero the key buffer. Use Rust's
  `zeroize` crate for sensitive data.
- **Anti-dump** (opt-in, behind feature flag): Detect common memory dump tools
  and refuse to load models if debugging/injection is detected (e.g., check
  `IsDebuggerPresent` on Windows). This is easily bypassed by determined
  attackers and may interfere with legitimate development/troubleshooting —
  only enable in release builds where model protection outweighs debuggability.

#### Step 8 — User-imported models stay user-owned

User-imported VRM files (via ModelPanel.vue) are the user's own files. These are
**not** encrypted — they load directly from the user's chosen path. The encryption
pipeline only applies to bundled default models that we ship with TerranSoul.

### Milestone Chunks

| Chunk | Description | Status |
|-------|-------------|--------|
| 070 | **Remove VRM from Git & .gitignore** — Add `*.vrm` to `.gitignore`. Remove tracked VRM files from index with `git rm --cached`. Update CI to download models from a private source (GitHub Release asset, private S3 bucket, or Git LFS with access control). Document the private model storage location. | `not-started` |
| 071 | **Build-Time Encryption Pipeline** — Create `scripts/encrypt-models.ts`. AES-256-GCM encryption of each `.vrm` to `.vrm.enc`. Key generated per-release, stored as CI secret. Build script downloads raw VRM from private source → encrypts → outputs to `public/models/default/`. Update `npm run build` to call encryption step. Add `.vrm.enc` files to the repo (or download at CI). | `not-started` |
| 072 | **Rust Decryption Command** — New Tauri command `load_vrm_secure(model_id: String) → Vec<u8>`. Reads `.vrm.enc` from Tauri resource dir, decrypts with compiled-in key (injected via `build.rs` env var). Returns raw bytes. Use `aes-gcm` crate. Add `zeroize` crate for key cleanup. Unit tests with a test-encrypted VRM. | `not-started` |
| 073 | **Frontend Secure Loading Path** — Update `CharacterViewport.vue` and `vrm-loader.ts` to call `invoke('load_vrm_secure', { modelId })` for default models. Receive `ArrayBuffer`, create `Blob` URL, pass to `GLTFLoader`. User-imported models continue using direct file path. Update `default-models.ts` to flag built-in vs user models. Vitest tests for both paths. | `not-started` |
| 074 | **CSP, DevTools & Scope Lockdown** — Set strict CSP in `tauri.conf.json`. Disable devtools in production. Configure Tauri resource/asset scope to deny raw `.vrm` access. Add integrity hashes for `.vrm.enc` files verified at startup. | `not-started` |
| 075 | **Obfuscation & Anti-Tamper** — Add `vite-plugin-obfuscator` to production build. Rust SHA-256 integrity check of `.vrm.enc` files at load time. `zeroize` sensitive buffers after use. Optional: platform-specific anti-debug checks behind feature flag. | `not-started` |

---

## Phase 8 — Brain-Driven Animation (AI4Animation for VRM)

> **Goal:** Use the LLM brain as an animation controller. Instead of pre-baked
> keyframe clips, the brain generates pose parameters (blend weights, bone
> offsets, gesture tags) that drive VRM character animation in realtime.
> Inspired by AI4Animation-js (SIGGRAPH 2018 MANN), adapted for stationary
> VRM desktop companion use. See `rules/research-reverse-engineering.md` §7.

| Chunk | Description | Status |
|-------|-------------|--------|
| 080 | **Pose Preset Library** — Define 8–12 VRM humanoid pose presets as JSON bone rotation sets (confident, shy, excited, thoughtful, relaxed, defensive, attentive, playful, bored, empathetic). Each preset stores rotation offsets for ~20 key VRM bones (hips, spine, chest, neck, head, shoulders, upper/lower arms, hands). A `PosePreset` type with `id`, `label`, `boneRotations: Record<VRMHumanBoneName, {x,y,z}>`. Load from `src/renderer/poses/` JSON files. Unit tests verifying all presets have valid bone names and angle ranges. | `not-started` |
| 081 | **Pose Blending Engine** — `PoseBlender` class in `src/renderer/pose-blender.ts`. Takes an array of `{ presetId, weight }` blend instructions and produces a final set of bone rotations by weighted-average (same principle as MANN's expert blending). Smooth interpolation over time (lerp/slerp between current and target blend). Integrates with `CharacterAnimator` — replaces or layers on top of procedural sin-wave animations. Breathing and blink remain procedural; body pose comes from blender. Vitest tests for blend math, edge cases (weights sum to 0, single preset at 1.0, etc.). | `not-started` |
| 082 | **LLM Pose Prompt Engineering** — Extend the streaming system prompt to instruct the brain to output structured pose data alongside emotion tags. Format: `[pose:confident=0.6,attentive=0.3]` — blend weights for named presets. The emotion parser (`utils/emotion-parser.ts` and `commands/emotion.rs`) is extended to also extract `pose` tags. When no pose tag is present, fall back to mapping emotion → default pose (happy→excited+playful, sad→shy+defensive, etc.). Frontend streaming store passes parsed pose weights to character store. Rust + TS parser tests. | `not-started` |
| 083 | **Gesture Tag System** — Extend motion tags to support timed gesture sequences. Brain outputs `[gesture:nod]`, `[gesture:wave]`, `[gesture:shrug]`, `[gesture:lean-in]`, etc. Each gesture is a short animation sequence (0.5–2s) defined as keyframe arrays in `src/renderer/gestures/`. `GesturePlayer` class plays the gesture, then returns to the current blended pose. Gestures layer on top of pose blending (additive). The brain can trigger gestures mid-sentence for natural conversational body language. 10+ built-in gestures. Vitest tests. | `not-started` |
| 084 | **Autoregressive Pose Feedback** — Feed the character's current pose state back into the LLM context window. When starting a new streaming response, include a compact pose descriptor in the system context: `Current character pose: confident=0.6, attentive=0.3. Last gesture: nod (2s ago).` This lets the brain make coherent animation decisions — e.g., not repeating the same gesture, gradually transitioning between poses across a conversation. Measure latency impact of extra context. Tests verifying pose context is correctly serialized and injected. | `not-started` |
