# TerranSoul — Milestones

> **To continue development, tell the AI agent:**
>
> ```
> Continue
> ```
>
> The agent will read this file, find the next chunk with status `not-started`,
> implement it, update the status to `done`, **remove the row from this file**,
> and log details in `rules/completion-log.md`.

> **ENFORCEMENT RULE — Completed chunks must be archived.**
>
> When a chunk is marked `done`:
> 1. Log full details (date, goal, architecture, files created/modified, test counts) in `rules/completion-log.md`.
> 2. **Remove the done row from this file.** `milestones.md` contains only `not-started` and `in-progress` chunks.
> 3. If an entire phase has no remaining rows, drop the phase heading too.
> 4. Update `Next Chunk` (below) to point to the next `not-started` chunk.
>
> This rule is mandatory for every AI agent session. Never leave done rows
> in milestones.md — the full historical record lives in `completion-log.md`.
>
> **Additional:** If the chunk was derived from reverse-engineering research,
> also clean up `rules/research-reverse-engineering.md` and `rules/backlog.md`.
> See `rules/prompting-rules.md` → "ENFORCEMENT RULE — Clean Up Reverse-Engineering Research on Chunk Completion".

> **Completed work lives in [`rules/completion-log.md`](completion-log.md).**
> Do not re-list done chunks here. Phases 0–11 (Foundation through RPG Brain
> Configuration), Chunks 1.2 / 1.3 / 1.4 / 1.5 / 1.6 / 1.7 / 1.8 / 1.9 / 1.10 / 1.11,
> the Phase 9 Learned-Features batch, and all Post-Phase polish are recorded
> there in reverse-chronological order.

---

## Next Chunk

Pick the next `not-started` item from the tables below or from `rules/backlog.md`.

---

## Active Chunks

### Phase 14 — Persona, Self-Learning Animation & Master-Mirror

| # | Chunk | Status | Notes |
|---|---|---|---|
| 14.16 | **LLM-Driven 3D Animation — Research, Implement & Self-Improve** — Full pipeline: LLM generates per-frame VRM bone poses from conversation context, emotion, and motion intent. Integrates with self-improve loop for continuous animation learning. See detailed spec below. (Sub-chunks 14.16a–f shipped 2026-05-01..02 — see `completion-log.md`. Optional GPU-required 14.16g remains.) | in-progress | Only optional 14.16g (MotionGPT / T2M-GPT) remains. |

---

#### Chunk 14.16 — LLM-Driven 3D Animation (Detailed Spec)

**Goal:** The brain generates context-aware 3D animations in real time — no
pre-baked clips needed for novel motions. The self-improve system discovers,
learns, and refines new animation patterns autonomously.

**Sub-chunks (implement sequentially):**

##### 14.16a — Research & Animation Taxonomy (research-only, no code)

Survey the state of the art in LLM-driven 3D character animation and build a
taxonomy of techniques applicable to VRM models:

| Technique | Paper/Source | Applicability |
|---|---|---|
| **MotionGPT** (Jiang et al. 2024) | Text → motion-token sequences decoded to SMPL-X | Retarget SMPL → VRM 11 bones; brain generates motion tokens alongside text |
| **MotionDiffuse** (Zhang et al. 2022) | Text-conditioned diffusion on joint angles | Generate `LearnedMotion` frames directly; long inference (~1s/motion) |
| **MoMask** (Guo et al. 2024) | Masked motion prediction from sparse keypoints | From webcam PoseLandmarker input → full-body VRM retarget |
| **AI4Animation MANN** (Holden et al. 2020) | Mode-Adaptive Neural Networks, expert blending | Already analysed in research-reverse-engineering.md § 7 |
| **T2M-GPT** (Zhang et al. 2023) | VQ-VAE motion tokens + GPT-2 decoder | Lightweight; motion codebook fits in ~50 MB |
| **LLM-as-animator** (novel) | Structured JSON pose output from chat LLM | Zero extra model; works with any Ollama/cloud LLM |

Deliverable: Write `docs/llm-animation-research.md` with pros/cons matrix,
latency estimates, VRM bone mapping strategy, and recommended implementation
order. No code changes.

##### 14.16b — LLM-as-Animator: Structured Pose Generation

The simplest and most immediately useful approach — the existing chat LLM
generates structured animation data alongside its text response.

**New stream tag:** `<pose>{ "head": [0.1, -0.05, 0], "spine": [0.03, 0, 0], ... }</pose>`

> **Sub-chunk 14.16b1 shipped 2026-05-02** — pure-Rust pose-frame parser
> + clamp foundation in `src-tauri/src/persona/pose_frame.rs` (canonical
> 11-bone rig, ±0.5 rad clamp, forgiving JSON parser, case-insensitive
> `<pose>` extractor, 24 tests). The frontend `PoseAnimator` and the
> `StreamTagParser` extension still need wiring — that is the remainder
> of 14.16b.
>
> **Sub-chunk 14.16b2 shipped 2026-05-02** — Rust `StreamTagParser`
> now recognises `<pose>` alongside `<anim>` (multi-tag state machine,
> `StreamFeed` return type, 6 new tests). The streaming pipeline
> emits `llm-pose` Tauri events with validated `LlmPoseFrame` payloads,
> and the system prompt advertises the pose schema. Remaining work is
> the frontend `PoseAnimator` class + `CharacterViewport` wiring
> (sub-chunk 14.16b3).
>
> **Sub-chunk 14.16b3 shipped 2026-05-02** — frontend
> `PoseAnimator` (`src/renderer/pose-animator.ts`, damped-spring
> blender with fade-in/hold/fade-out phases) wired into
> `CharacterViewport.vue` (per-frame `apply` + `playPose` /
> `clearPose` methods) and both `ChatView.vue` + `PetOverlayView.vue`
> (`llm-pose` event listeners). 14 new vitest tests; full LLM-as-
> Animator pipeline now runs end-to-end. **Chunk 14.16b is
> complete — ready to remove from `in-progress`.**

Implementation:
1. **Extend `StreamTagParser`** to recognise `<pose>...</pose>` tags (like existing `<anim>` tags).
2. **New type `LlmPoseFrame`** in `persona-types.ts`:
   ```typescript
   interface LlmPoseFrame {
     bones: Record<string, [number, number, number]>;  // Euler XYZ rad
     duration_s?: number;   // how long to hold (default 2s)
     easing?: 'linear' | 'ease-in-out' | 'spring';
     expression?: Record<string, number>;  // optional face weights
   }
   ```
3. **New `PoseAnimator` class** in `src/renderer/pose-animator.ts`:
   - Receives `LlmPoseFrame` from the stream parser
   - Smoothly interpolates current bone state → target pose via damped spring
   - Layered on top of procedural idle (takes priority, fades back to idle)
   - Respects VRMA lock (yields when a VRMA clip is playing)
4. **Wire into CharacterViewport** — `onPoseTag(frame: LlmPoseFrame)` callback
5. **System prompt injection** — Add pose instruction block to the system prompt
   when the `local-brain` or `paid-brain` skill is active:
   ```
   You can control the character's body by emitting <pose> tags:
   <pose>{"head":[0.1,0,0],"spine":[0.03,0,0]}</pose>
   Bones: head, neck, spine, chest, hips, leftUpperArm, rightUpperArm,
   leftLowerArm, rightLowerArm, leftShoulder, rightShoulder
   Values are Euler angles in radians. Use small values (±0.3 max).
   ```

Tests: Unit tests for PoseAnimator (damping, layering, VRMA yield). Stream
parser tests for `<pose>` tag extraction. Vitest + vue-tsc clean.

##### 14.16c — Motion Library: LLM-Generated Clip Catalogue

> **✅ Shipped 2026-05-02** — sub-chunk 14.16c1 landed the pure-Rust
> motion-clip parser/validator (`src-tauri/src/persona/motion_clip.rs`,
> 18 tests). Sub-chunks 14.16c2 + 14.16c3 followed the same day with
> the `generate_motion_from_text` Tauri command (routes through
> `memory::brain_memory::complete_via_mode` so all four brain modes
> work) and the `PersonaMotionGenerator.vue` UI in `PersonaPanel`
> (text → generate → preview → accept/discard). Chunk 14.16c is
> complete — ready to remove from `in-progress` once 14.16d / e / f
> ship.

Let the brain generate reusable `LearnedMotion` clips from text descriptions.

1. **New Tauri command `generate_motion_from_text`**:
   - Input: `{ description: string, duration_s: number, fps: number }`
   - Calls the active brain with a structured prompt:
     ```
     Generate a VRM animation clip for: "{description}"
     Output JSON: { "frames": [{ "t": 0.0, "bones": { "head": [x,y,z], ... } }, ...] }
     Duration: {duration_s}s at {fps} FPS. Bones: head, neck, spine, chest,
     hips, leftUpperArm, rightUpperArm, leftLowerArm, rightLowerArm,
     leftShoulder, rightShoulder. Values are Euler radians (±0.5 max).
     Output ONLY valid JSON.
     ```
   - Parse response → `LearnedMotion` struct
   - Validate: clamp angles, verify frame count, ensure monotonic timestamps
   - Persist to persona store's `learnedMotions`
2. **UI in Persona panel** — "Generate motion" button, text input, preview, save
3. **Motion preview** — plays generated clip on the character before saving
4. **Motion trigger** — saved motions get a `trigger` key the LLM can emit

Tests: Rust command tests (mock LLM response). Frontend tests for motion
generation flow. End-to-end validation that baked clip plays on VRM.

##### 14.16d — Emotion-Reactive Procedural Blending

> **✅ Shipped 2026-05-02** — `EmotionPoseBias` in
> `src/renderer/emotion-pose-bias.ts` adds tiny additive postural
> offsets driven by `characterStore.state` (±0.18 rad cap, λ=4
> damping). Yields automatically to baked VRMA clips and active
> `PoseAnimator` poses. Wired in `CharacterViewport.vue` per-frame
> loop after `animator.update`. 14 new vitest cases.

Replace the current 6 static idle poses with an emotion-weighted blend system
inspired by AI4Animation's expert blending architecture.

1. **Emotion-weighted pose blending** in `CharacterAnimator`:
   - Current: `IDLE_POSES[randomIndex]` picks one static pose
   - New: `blendedPose = Σ (emotionWeight[i] × EMOTION_POSES[i])` where
     weights come from the AvatarStateMachine emotion layer
   - Emotions: neutral, happy, sad, angry, relaxed, surprised, thinking
   - Each emotion has a characteristic pose (head tilt, spine lean, arm rest)
2. **Conversation energy signal**:
   - Track recent message sentiment and response speed
   - Map to body "energy level" (0–1): low energy = slumped/sleepy, high = upright/alert
   - Blended into spine/chest lean and breathing amplitude
3. **Micro-gestures** — small procedural movements triggered by brain output:
   - Nod on agreement (`<gesture:nod>`)
   - Head shake on disagreement (`<gesture:shake>`)
   - Shrug on uncertainty (`<gesture:shrug>`)
   - These are 0.5s procedural tweens, not full VRMA clips

Tests: Animator unit tests for blend weights, energy mapping, micro-gestures.

##### 14.16e — Self-Improve Animation Learning Loop

> **✅ Shipped 2026-05-02** — motion-feedback log
> (`<persona-root>/motion_feedback.jsonl`) records every accept/reject
> verdict; `motion_feedback::render_prompt_hint` splices the
> trusted-trigger leaderboard into the next `generate_motion_from_text`
> system prompt. New Tauri commands `record_motion_feedback` +
> `get_motion_feedback_stats`; 11 new Rust tests; UI footer in
> `PersonaMotionGenerator.vue` shows "You've taught me N motions".

Integrate animation generation with the self-improve system so the companion
autonomously discovers and learns new animations.

1. **New self-improve task type: `animation_discovery`** in the engine:
   - Engine reads the current `VRMA_ANIMATIONS` catalogue + `learnedMotions`
   - Identifies gaps (e.g., no "waving goodbye" motion, no "thinking hard" pose)
   - Generates a text prompt → calls `generate_motion_from_text` (from 14.16c)
   - Validates the generated clip (plays on headless Three.js, checks bone limits)
   - Saves to `learnedMotions` with auto-generated trigger key
   - Logs the discovery to `self-improve-progress` event feed
2. **Animation quality scoring** (Rust):
   - Score each generated motion on: smoothness (jerk minimisation), range
     adherence (no clipping), expressiveness (variance from idle)
   - Reject motions below quality threshold, retry with refined prompt
   - Log quality scores to self-improve metrics
3. **Iterative refinement**:
   - After N chat sessions, analyse which motions were triggered vs ignored
   - Prune unused motions, regenerate with adjusted prompts
   - Track improvement metrics: motion diversity, trigger frequency, user
     engagement (did user continue chatting after animation played?)
4. **Skill tree integration** — new quest `animation-mastery`:
   - Auto-activates when `local-brain` is active + 5+ learned motions exist
   - Rewards: "Self-taught animations", "Expanding motion vocabulary"
   - Stat boosts: CHA +8, DEX +8, INT +5 (creative expression + precision)

Tests: Engine task type routing tests. Quality scoring tests. Skill tree
activation test.

##### 14.16f — Pack-import provenance markers

> **✅ Shipped 2026-05-02** — `LearnedMotion.provenance` field
> (`'generated' | 'camera'`) now stamped at every save site;
> `ImportReport` carries `motions_generated` + `motions_camera`
> counters; `PersonaPackPanel` renders a "X motions (N generated, M
> camera)" breakdown so packs reveal the origin of each clip. Older
> clips without the field are simply unattributed — no migration. 4 new
> Rust tests in `persona::pack`.

##### 14.16g — MotionGPT / T2M-GPT Inference (Optional, GPU-Required)

For users with capable GPUs, run a dedicated motion generation model locally.

1. **Evaluate T2M-GPT** (smallest: ~50 MB VQ-VAE codebook + 124M GPT-2):
   - Text → motion tokens → VQ-VAE decode → SMPL joint angles
   - Retarget SMPL 23 joints → VRM 11 bones via FK chain mapping
2. **ONNX runtime integration** in Rust (`ort` crate, already used by some projects):
   - Load quantised T2M-GPT model from `~/.terransoul/models/motion/`
   - Inference: ~200ms per 3-second clip on RTX 3060
   - Fallback: if no GPU / model not downloaded, use LLM-as-animator (14.16b)
3. **Model download flow** — reuse the Ollama-style pull progress UI
4. **Quality comparison** — A/B test LLM-generated vs T2M-GPT motions

Feature-flagged behind `AppSettings.motion_model_enabled`. This sub-chunk is
optional and depends on user GPU availability.

Tests: ONNX loading tests (mock model). Retarget bone-mapping tests.
Integration test with VRM playback.

---

### Phase 25 — Self-Improve Autonomous Coding System

> Goal: TerranSoul becomes a self-improving autonomous coding system that
> drives `rules/milestones.md` chunks through a dedicated coding LLM, opens
> PRs against a feature branch (never master), and survives app/computer
> restart. Disabling is the only stop signal. **Foundation done in
> Chunk 25.1 — see `completion-log.md`. Chunks 25.2-25.9 (planner-mode
> autonomous loop, repo binding, autostart, tray, live UI) shipped in
> the 25.2-25.9 batch — see `completion-log.md`.**

| # | Chunk | Status | Notes |
|---|---|---|---|



---

### Phase 15 — AI Coding Integrations (MCP + gRPC brain gateway)

| # | Chunk | Status | Notes |
|---|---|---|---|

| 15.7 | **VS Code Copilot incremental-indexing QA** — e2e test: cold/warm calls, fingerprint cache, file-watcher invalidation. | not-started | Depends on 15.1 + 15.3 + 15.6. |

---

### Phase 16 — Modern RAG

| # | Chunk | Status | Notes |
|---|---|---|---|
| 16.3b | **Late chunking — ingest integration** — wire `memory::late_chunking::pool_chunks` into `run_ingest_task` behind an `AppSettings.late_chunking` flag, calling a long-context Ollama embedder that returns per-token vectors. (16.3a — pure pooling utility — shipped.) | not-started | Needs long-context embedding model in Ollama catalogue that exposes per-token embeddings. |
| 16.6 | **GraphRAG / LightRAG community summaries** — Leiden community detection over `memory_edges`, LLM summaries, dual-level retrieval + RRF. | not-started | Heavy chunk; background workflow job. |

---

### Phase 17 — Brain Phase-5 Intelligence

| # | Chunk | Status | Notes |
|---|---|---|---|
| 17.5 | **Cross-device memory merge via CRDT sync** — wire `MemoryStore` into Soul Link. LWW-Map CRDT keyed on `(content_hash, source_url)`. | not-started | Hardest chunk — may split into 17.5a (schema) + 17.5b (delta sync). |
| 17.7 | **Bidirectional Obsidian sync** — extend one-way export to bidirectional via `notify` file-watcher. LWW conflict resolution. Schema columns now in place (V10, chunk 17.7b shipped). | not-started | Depends on 18.5 (shipped) + 17.7b (shipped). |

---

### Phase 19 — Pre-release schema cleanup

| # | Chunk | Status | Notes |
|---|---|---|---|
| 19.1 | **Collapse migration history → canonical schema; delete migration runner.** Single `create_canonical_schema` block, remove `migrations.rs` + 600 test lines. | not-started | **MUST land last** — after all schema-changing chunks (16.6, 17.5, etc.). |

---

### Phase 20 — Dev/Release Data Isolation (Fresh Dev, Persistent Release)

| # | Chunk | Status | Notes |
|---|---|---|---|

---

### Phase 22 — Plugin System Completion

> **Why this phase exists.** `docs/plugin-development.md` (613 LOC) and
> the Rust host (`src-tauri/src/plugins/{manifest,host}.rs`, ~1,300 LOC,
> 28 tests) ship a *registry* — install / activate / list / uninstall
> work, contributions are stored, settings are persisted. But nothing
> the registry tracks actually *runs* yet:
>
> - `commands` → no Tauri / chat surface invokes plugin commands.
> - `slash_commands` → no chat input router dispatches them.
> - `themes` → no CSS-variable applier consumes them.
> - `memory_hooks` → no `add_memory` pre/post pipeline calls them.
> - `views` → no router renders them.
> - `settings` → no settings UI exposes them.
> - Activation events (`OnStartup`, `OnChatMessage`, `OnMemoryTag`,
>   `OnBrainModeChange`) → no dispatcher fires `check_activation()`.
> - There is no install UI — manifests are only installable via direct
>   `invoke('plugin_install', ...)`.
>
> Either we finish the plugin system or we delete it. The user has
> told us to design proper requirements, not defer — so this phase
> closes the loop. Each chunk below is independently shippable.

| # | Chunk | Status | Notes |
|---|---|---|---|
| 22.5 | **Memory-hook contribution → `add_memory` pre/post pipeline.** When `commands::memory::add_memory` runs, fire each active plugin's `memory_hooks[]` matching the current stage (`PreStore`, `PostStore`, etc.). Hooks are dispatched through the existing `WasmRunner` sandbox (`src-tauri/src/sandbox/wasm_runner.rs`) so untrusted plugins cannot read other memories. PreStore hooks may rewrite tags / content; PostStore hooks are notification-only. Hard-cap each hook at 200 ms. ~300 LOC + integration tests showing a sample WASM tag-rewriter plugin altering a memory in-flight. | not-started | Depends on 22.2 + sandbox/wasm_runner.rs (already exists, used elsewhere). The activation events `OnMemoryTag` are also wired here. |
| 22.7 | **Plugin command execution — Tool / Sidecar / WASM dispatch.** Today `plugin_host.invoke_command` (added in 22.4) only echoes metadata. Wire real execution: `kind: Tool` → call native sidecar via `tauri-plugin-shell` with the plugin's declared args, capture stdout / stderr; `kind: Wasm` → invoke through `WasmRunner`; `kind: Sidecar` → launch + handle bidirectional pipe. Capability-checked at every call site (rejects if user has not granted the capability). ~500 LOC + integration tests for each kind. | not-started | Depends on 22.4 (dispatcher) + 22.2 (capability grant UI). The biggest chunk in this phase; gates the *useful* end of plugin development. |

#### Phase 22 acceptance gate

A user installs a sample WASM plugin (`hello-world.terransoul-plugin.json`)
that contributes one slash-command `/hello`, one theme, and one
memory-hook that prepends `auto:` to every new memory's tags. After
22.1–22.7 land:

- Plugin shows up in `PluginsView.vue` after drag-and-drop install.
- Activating it applies the theme tokens immediately.
- Typing `/hello world` in chat dispatches to the plugin and prints its
  greeting as an assistant turn.
- Adding a new memory shows `auto:` automatically prepended.
- Disabling the plugin removes the theme + slash-command + hook within
  the same render frame.
- Uninstalling deletes the persisted manifest from `<data_dir>/plugins/`.

---

### Phase 23 — Multi-Agent Resilience Wiring

> **Why this phase exists.** The 2026-04-25 entry at the top of
> `rules/completion-log.md` describes "per-agent threads, workflow
> resilience, agent swap context" but a deep code-read shows only the
> *scaffold* shipped:
>
> - `agentMessages` computed in `src/stores/conversation.ts` is
>   exported but no view consumes it (`grep` returns one self-reference).
>   Multi-agent thread filtering is invisible to the user.
> - `ResilientRunner` / `RetryPolicy` / `CircuitBreaker` /
>   `HeartbeatWatchdog` exist in `src-tauri/src/workflows/resilience.rs`
>   with 13 tests, but `grep ResilientRunner` outside the module returns
>   zero hits. Workflow activities still run un-wrapped.
> - `getHandoffContext()` in `src/stores/agent-roster.ts` is exported
>   but never called by any prompt builder. Agent swap therefore loses
>   context in practice even though the data is being recorded.
>
> Library shipped, integration didn't. Each chunk below closes one of
> these three loops.

| # | Chunk | Status | Notes |
|---|---|---|---|

#### Phase 23 acceptance gate

In a single chat session: user starts with Agent A, exchanges 5 turns,
swaps to Agent B, exchanges 5 more turns, swaps back to Agent A.

- Agent B's first reply demonstrably acknowledges the handoff context
  from A (the `[HANDOFF FROM A]` block was injected into its prompt).
- The chip row above the chat lets the user filter to "Agent A only"
  → only A's 5+5 turns visible.
- Disabling the user's network mid-stream during one of A's activities
  triggers a `CircuitBreaker::Open` after the configured failure
  threshold, recovers half-open after the recovery timeout, and the
  workflow resumes on the next chat turn instead of permanently failing.


---

### Phase 24 — Mobile Companion (iOS + LAN gRPC Remote Control)

> **Why this phase exists.** TerranSoul today is desktop-only and
> binds every server to `127.0.0.1`. The user wants to **drive the
> desktop companion from an iOS phone over the home Wi-Fi LAN** —
> e.g. ask the phone "what's the progress of the Copilot session
> running in VS Code on my desktop?" or "continue the next step of
> chunk X" and have the phone-side LLM call into the desktop brain
> + workflow + Copilot-state surface via gRPC.
>
> This phase splits cleanly along three independent axes:
>
> 1. **LAN networking on the desktop** — bind config, address
>    discovery, mTLS pairing, gRPC remote-control surface
>    (24.1 – 24.5). Each ships independently of iOS work.
> 2. **iOS app shell** — Tauri iOS target reusing the Vue frontend
>    with a mobile layout + Keychain-backed pair store
>    (24.6 – 24.8).
> 3. **Phone-driven LLM control loop** — phone-side chat that
>    surfaces gRPC-backed tools (Copilot-session probe, workflow
>    progress, "continue next chunk" trigger) so the LLM can
>    *act* on the desktop, not just *report* (24.9 – 24.11).
>
> Pairs with the long-deferred Chunk 15.2 (gRPC server) — that
> chunk delivers the tonic transport; this phase delivers the
> phone-control RPC surface on top of it.

| # | Chunk | Status | Notes |
|---|---|---|---|
| 24.6 | **Tauri iOS target + shared frontend.** Add `[targets.ios]` to `tauri.conf.json`; mobile layout via existing `viewport` breakpoints; iOS Keychain for pairing token storage via `tauri-plugin-stronghold` or `tauri-plugin-keyring`. Cargo `cargo tauri ios init` + first build pipeline. ~300 LOC + iOS-CI smoke job. | not-started | Heavy chunk — splits into 24.6a (iOS init + smoke build) and 24.6b (mobile layout pass) at implementation time. |
| 24.7 | **iOS pairing UX (`MobilePairingView.vue`).** Scan QR via `tauri-plugin-barcode-scanner`; on confirm, store cert + token in Keychain; trust list in `MobileSettingsView`. Re-pair flow when fingerprint mismatches. ~250 LOC + tests. | not-started | Depends on 24.2b (server side) + 24.6 (iOS shell). |
| 24.8 | **gRPC-Web client + transport adapter (`src/transport/grpc_web.ts`).** Use `@bufbuild/connect` for browser-native streaming over the Tauri WebView. Replace direct `invoke()` calls with a `RemoteHost` abstraction so the same Vue components work locally (in-process Tauri) or remotely (gRPC-Web). ~400 LOC + tests. | not-started | This is the abstraction that lets one frontend codebase serve both desktop and mobile. |
| 24.9 | **Mobile chat view streaming through `RemoteHost`.** Refit `ChatView.vue` to render under mobile breakpoints AND switch its backing store from in-process `conversation.ts` to `remote-conversation.ts` when running on iOS. Same `[PERSONA]` / `[LONG-TERM MEMORY]` / `[HANDOFF]` injection — performed server-side; phone receives the pre-built prompt's stream. ~300 LOC + tests. | not-started | Depends on 24.4 + 24.8. |
| 24.10 | **"Continue next step" remote command + workflow progress narration.** Phone-side LLM tools (`describe_workflow_progress`, `continue_workflow`, `describe_copilot_session`) backed by RPC calls from 24.4 + 24.5. Tool surface mirrors the MCP brain tools so the same LLM prompts work locally and remotely. The phone LLM is the host's brain — the phone is "just" a microphone + screen; capability gates enforce that the phone cannot escalate beyond what its `paired_devices.capabilities` allow. ~250 LOC + tests. | not-started | The chunk that delivers the user's headline use case ("ask what's the progress of Copilot in VS Code, continue next step"). |
| 24.11 | **Local push notification on long-running task completion.** When a workflow run, an ingest job, or a Copilot session crosses a threshold (configurable, default 30 s), fire a local notification via `tauri-plugin-notification` on the iOS shell when the phone is paired and connected. No APNS dependency — push lives over the LAN connection while paired. ~150 LOC + tests. | not-started | Depends on 24.6 + 24.4. Optional polish. |

#### Phase 24 acceptance gate

User on the same Wi-Fi as the desktop:

1. Opens the iOS TerranSoul app, scans the desktop's QR pairing
   code → device shows up in the desktop's paired-devices list
   within 5 s, and the iOS app's home screen reads "Connected to
   `<desktop-name>`".
2. Asks the phone (via voice or chat) "What's Copilot doing on my
   desktop right now?" → the phone's LLM calls
   `describe_copilot_session` → renders e.g. "Copilot Chat is
   active in `D:/Git/TerranSoul`, last assistant reply 30s ago,
   currently streaming chunk 16.4b".
3. Says "continue the next chunk" → phone LLM calls
   `continue_workflow(active_run_id)` → desktop resumes work →
   phone receives a streaming narration of progress.
4. User leaves the LAN → phone shows "Disconnected"; on returning,
   pairing is re-used silently from Keychain.
5. Revoking the phone from the desktop's trust list immediately
   terminates streams and forces a re-pair on the iOS side.

---

### Phase 27 — Agentic RAG, Context Engineering & Persona Roadmap Gaps

> **Why this phase exists.** `docs/brain-advanced-design.md` §19.2 rows
> 14 and 15 (Agentic RAG + Context Engineering) are tracked here; chunks
> 27.1 and 27.2 have shipped and are archived in `completion-log.md`.
> The remaining rows are roadmap gaps mentioned in `docs/persona-design.md`
> §6.3, §7.2, and §14.2 that were not yet represented in this milestones file.

| # | Chunk | Status | Notes |
|---|---|---|---|

| 27.4 | **MoMask-style full-body reconstruction research spike.** Evaluate whether a permissively licensed, locally runnable sparse-keypoint → full-body-pose model can improve the existing BlazePose retarget path without requiring cloud inference. | not-started | Derived from `docs/persona-design.md` §7.2 / §14.2 row 5. Research + thin integration plan first; do not vendor model weights. |
| 27.5 | **Offline recorded-motion polish pass.** Design an explicit-user-trigger background workflow for smoothing / enhancing saved teach-session clips, informed by Hunyuan-Motion, MimicMotion, and MagicAnimate research references. | not-started | Derived from `docs/persona-design.md` §7.2 / §14.2 rows 4, 6, 7. Must remain optional, GPU-aware, and license-clean. |
| 27.6 | **Neural audio-to-face upgrade evaluation.** Compare the shipped phoneme-aware viseme path against Audio2Face, EMOTalk, and FaceFormer-class approaches and define an optional backend if a local, license-clean model is viable. | not-started | Derived from `docs/persona-design.md` §14.2 rows 9 and 11. Do not add NVIDIA ACE or cloud-only dependencies without an explicit opt-in design. |

---

### Phase 28 — Self-Improve Loop Maturation Follow-ups

> **Why this phase exists.** `docs/coding-workflow-design.md` §5 documents
> the self-improve roadmap. Chunks 28.1–28.10 plus 25.10 are archived as
> shipped, but the live engine is still planner-only and some shipped
> primitives are not wired into the autonomous execution path.

| # | Chunk | Status | Notes |
|---|---|---|---|
| 28.11 | **Apply/review/test execution gate for self-improve.** Wire `coding::apply_file`, `coding::reviewer`, and `coding::test_runner` into the autonomous engine after planner approval so generated changes are applied only when review and tests pass. | not-started | Derived from `docs/coding-workflow-design.md` §5 items 1, 2, and 5 plus `coding::engine` planner-only comments. Must keep safe stop/cancel semantics. |
| 28.12 | **Multi-agent coding DAG orchestration wiring.** Connect `coding::dag_runner` to self-improve coding tasks so planner/coder/reviewer/tester nodes can run in dependency order with bounded parallelism. | not-started | Derived from `docs/coding-workflow-design.md` capability matrix + §5 item 4. The pure DAG runner exists; this chunk wires it to real coding tasks. |

