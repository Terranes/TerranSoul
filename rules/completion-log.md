# TerranSoul — Completion Log

> This file is the permanent record of all completed chunks.
> `rules/milestones.md` contains only chunks that are `not-started` or `in-progress`.
> When a chunk is done, its full details are recorded here and the row is removed from milestones.md.

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

## Chunk 128 — FF16 Constellation Skill Tree (Full-Screen Layout)

**Date:** 2026-04-20
**Phase:** Phase 11 — RPG Brain Configuration

**Goal:** Replace the 360px CSS grid panel inside `QuestBubble.vue` with a full-screen FF16 Abilities-style constellation map. Each of the five categories (Brain, Voice, Avatar, Social, Utility) becomes a circular cluster of nodes laid out radially with concentric rings, glowing connection lines, a colored diamond border, and a star-field background. Pannable + zoomable.

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

**Goal:** Make the FF16 constellation interactive — click a cluster to zoom into it, click a node to open a quest detail overlay (objectives, rewards, prerequisites), provide breadcrumb navigation, a back button, and a corner minimap with status dots.

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
