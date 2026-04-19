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

✅ Chunk 110 — Background Music — see `rules/completion-log.md`

`useBgmPlayer` composable: Web Audio API procedural ambient tracks (3 presets: Calm Ambience,
Night Breeze, Cosmic Drift). Fade-in/fade-out transitions. Toggle, volume slider, track selector
in CharacterViewport settings dropdown. BGM state persisted via `AppSettings` (bgm_enabled,
bgm_volume, bgm_track_id). Schema version bumped to 2. 10 Vitest tests.

### Phase 9 — Medium Priority (Chunks 094–098)

✅ Chunk 094 — Model Position Saving — see `rules/completion-log.md`

Per-model camera positions (`ModelCameraPosition` struct, `HashMap` in `AppSettings`).
`get_model_camera_positions` + `save_model_camera_position` Tauri commands. `useModelCameraStore` composable.
11 Vitest tests + 6 Rust tests.

✅ Chunk 095 — Procedural Gesture Blending (MANN-inspired) — see `rules/completion-log.md`

`GestureBlender` with layered sine-harmonic noise, per-state `BlendConfig`, cosine-interpolated cross-fades.
Integrated into `CharacterAnimator.applyStateBonePose()`. 13 Vitest tests.

✅ Chunk 096 — Speaker Diarization — see `rules/completion-log.md`

`DiarizationEngine` trait, `StubDiarization`, `diarize_audio` Tauri command.
`useDiarization` composable. 12 Vitest tests + 8 Rust tests.

✅ Chunk 097 — Hotword-Boosted ASR — see `rules/completion-log.md`

`Hotword` struct on `VoiceConfig`. CRUD commands (`get/add/remove/clear_hotwords`).
`useHotwords` composable. 13 Vitest tests + 10 Rust tests.

✅ Chunk 098 — Presence / Greeting System — see `rules/completion-log.md`

`usePresenceDetector`: activity tracking via DOM events + `visibilitychange`, away duration
classification (short/medium/long/extended), duration-specific greeting pools. 15 Vitest tests.

### Phase 9 — Lower Priority (Chunks 115–119)

✅ Chunk 115 — Live2D Support — see `rules/completion-log.md`

Renderer abstraction layer (`CharacterRenderer` interface, `Live2DStubRenderer`, `createRenderer()` factory).
Ready for Cubism SDK integration when needed. 14 Vitest tests.

✅ Chunk 116 — Screen Recording / Vision — see `rules/completion-log.md`

`ScreenFrame` + `VisionAnalysis` types. `capture_screen` + `analyze_screen` Tauri commands (stub).
`useVisionCapture` composable (capture → analyze pipeline). 13 Vitest tests + 6 Rust tests.

📦 Chunk 117 — Docker Containerization — demoted back to `rules/backlog.md`.

Re-analysis: TerranSoul is a Tauri desktop app — Docker containers are not useful here. If container
orchestration is ever needed (e.g. for running LLM inference servers), use .NET Aspire instead.

✅ Chunk 118 — Chat Log Export — see `rules/completion-log.md`

`useChatExport` composable (formatExport, toJson, downloadJson, filters by date/role/sentiment).
`export_chat_log` Tauri command. JSON export includes metadata (sentiment summary, date range).
14 Vitest tests + 3 Rust tests.

✅ Chunk 119 — Language Translation Layer — see `rules/completion-log.md`

`TranslationResult` type. `list_languages`, `translate_text`, `detect_language` Tauri commands (stub).
10 supported languages. `useTranslation` composable. 11 Vitest tests + 9 Rust tests.

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

---

## Phase 10 — Avatar Animation Architecture (VRM Expression-Driven)

✅ Phase 10 complete — see completion-log.md

---

## Phase 11 — RPG Brain Configuration (Configure Your AI Like an RPG Character)

> **Goal:** Reframe the entire AI configuration experience as an RPG progression system.
> Instead of dry settings panels, users "build their brain" by unlocking capabilities
> through quests, combos, and tier upgrades — the same way a gamer builds a character.
>
> **Design Principle:** Your AI assistant is a living character. Its capabilities are
> skills you unlock, not checkboxes you toggle. Every configuration choice is a quest
> that teaches you something and rewards you with a smarter companion.

### The Human Brain → AI System Mapping

TerranSoul's architecture mirrors how a real brain works:

| Human Brain                | AI System                          | RPG Equivalent              |
| -------------------------- | ---------------------------------- | --------------------------- |
| Prefrontal Cortex          | Reasoning Engine (LLM + Agents)    | Intelligence stat           |
| Hippocampus                | Long-term Memory                   | Wisdom stat                 |
| Working Memory Network     | Short-term Memory                  | Focus / Concentration       |
| Neocortex                  | Retrieval System (RAG / Knowledge) | Knowledge / Lore stat       |
| Basal Ganglia / Cerebellum | Control & Execution Layer          | Dexterity / Reflexes stat   |

Each "brain region" maps to a real AI subsystem that the user progressively unlocks:

- **Prefrontal Cortex → Reasoning:** Start with a free LLM (`free-brain`), evolve to paid API (`paid-brain`) or local LLM (`local-brain`). Each upgrade is a quest that boosts the "Intelligence" stat.
- **Hippocampus → Memory:** Unlock `memory` to give your AI persistent recall. Combine with `diarization` for the **Social Memory** combo — your AI remembers who said what.
- **Working Memory → Context:** Short-term memory (last 20 messages) is auto-injected. Upgrade to RAG with `paid-brain` + `memory` for the **True Recall** combo.
- **Neocortex → Knowledge:** Install community agents (`agents`) to expand your AI's knowledge domains. Combine with `device-link` for the **Hive Mind** combo.
- **Basal Ganglia → Execution:** `pet-mode`, `windows-shortcuts`, `windows-startup` — the motor cortex of your AI. Enable combos like **Instant Companion** and **Always There**.

### Visual Design: The FF16 Constellation Map

The skill tree renders as an **FF16 Abilities-style constellation map** — a full-screen dark
star-field with circular category clusters arranged radially. Each cluster is a wheel of nodes
like the Eikon ability circles in Final Fantasy XVI:

![FF16 Abilities Reference](recording/ff16-abilities-reference.png)

**Layout:**
- 5 category clusters (Brain, Voice, Avatar, Social, Utility) positioned on a deep-blue background
- Each cluster is a **radial wheel**: center emblem + concentric rings of nodes
  - Inner ring: Foundation skills
  - Middle ring: Advanced skills
  - Outer ring: Ultimate skills
- Glowing connection lines between prerequisite nodes
- Clusters connected by faint constellation lines across the map
- Pannable + zoomable (touch + mouse wheel)

**Cluster Color Themes:**
- 🧠 Brain — crimson/red diamond border (like Ifrit in FF16)
- 🗣️ Voice — jade/green diamond border (like Garuda)
- ✨ Avatar — gold diamond border (like Titan)
- 🔗 Social — sapphire/blue diamond border (like Shiva)
- 📀 Utility — amethyst/purple diamond border (like Odin)

**Node States (same as before):**
- **Locked:** Dim, desaturated, 0.35 opacity
- **Available:** Gold breathing border + cost label below
- **Active:** Full glow + completed checkmark

**Interaction Flow:**
1. Click floating orb → full-screen constellation view
2. Click a cluster → zoom into that category's radial wheel
3. Click a node → quest detail overlay (objectives, rewards, prereqs)
4. Breadcrumb navigation: "All Clusters > Brain > Awaken the Mind"
5. Minimap in corner shows all clusters with colored status dots

### Stat Boosts & Combo System

When the user unlocks specific skill combinations, they trigger **combos** that
provide bonus capabilities — like equipping a matching armor set in an RPG:

| Combo Name             | Required Skills                     | Bonus Effect                          |
| ---------------------- | ----------------------------------- | ------------------------------------- |
| 🎧 DJ Companion        | `tts` + `bgm-custom`               | AI curates music based on mood        |
| 💬 Full Conversation   | `asr` + `tts`                       | Hands-free voice chat                 |
| 🎬 Film Critic         | `bgm-video` + `paid-brain`         | AI comments on videos you watch       |
| 🧠 True Recall         | `paid-brain` + `memory`            | Context-aware responses from history  |
| 🏔️ Offline Sage        | `local-brain` + `memory`           | Full AI offline with memory           |
| 👂 Perfect Hearing     | `whisper-asr` + `hotwords`         | Boosted recognition accuracy          |
| 👥 Social Memory       | `diarization` + `memory`           | Remembers who said what               |
| 🌐 Universal Translator| `translation` + `asr`              | Real-time voice translation           |
| 👁️ Omniscient Companion| `vision` + `memory` + `asr`        | Sees, hears, and remembers everything |
| 🐝 Hive Mind           | `agents` + `device-link`           | Multi-device agent orchestration      |
| 🐾 Living Desktop Pet  | `pet-mode` + `asr` + `presence`    | Reactive floating companion           |
| ⚡ Instant Companion   | `windows-shortcuts` + `pet-mode`   | Global hotkey summons your AI         |
| 🏠 Always There        | `windows-startup` + `pet-mode` + `presence` | AI greets you every boot     |

### Game-Style Stat Progression

Each unlocked skill boosts one or more character stats displayed in the UI:

| Stat          | Icon | Boosted By                                                     |
| ------------- | ---- | -------------------------------------------------------------- |
| Intelligence  | 🧠   | `free-brain`, `paid-brain`, `local-brain`, `agents`            |
| Wisdom        | 📖   | `memory`, `device-link`                                        |
| Charisma      | 🗣️   | `tts`, `asr`, `translation`, `diarization`                     |
| Perception    | 👁️   | `vision`, `presence`, `hotwords`, `whisper-asr`                |
| Dexterity     | ⚡   | `pet-mode`, `windows-shortcuts`, `windows-startup`, `bgm`     |
| Endurance     | 🛡️   | `local-brain` (offline), `device-link` (redundancy)            |

### Implementation Chunks

| Chunk | Description | Status |
|-------|-------------|--------|
| 128 | **FF16 Constellation Skill Tree — Full-screen layout redesign.** Replace the 360px CSS grid panel in QuestBubble.vue with a full-screen (or large overlay) **FF16 Abilities-style constellation map**. Each category (Brain, Voice, Avatar, Social, Utility) becomes a **circular cluster** of nodes arranged radially — like the Eikon wheels in FF16. Clusters are positioned on a dark deep-blue background with subtle particle/star-field effects. Each cluster has: (a) a central **category emblem** with the category icon + Ability Points cost, (b) skill nodes arranged in concentric rings around the center (foundation = inner ring, advanced = middle ring, ultimate = outer ring), (c) glowing connection lines between prerequisite nodes, (d) a colored diamond border matching the category (red/crimson for Brain, green/jade for Voice, gold for Avatar, blue/sapphire for Social, purple/amethyst for Utility). Nodes use the same locked/available/active states but rendered as circular gems with cost labels below. The orb floats over the viewport and clicking it opens the full-screen constellation view. Must be pannable + zoomable (touch + mouse). **Reference:** FF16 Abilities screen (circular Eikon clusters with radial node layouts). | `not-started` |
| 129 | **Constellation cluster interaction & detail panel.** Clicking a cluster zooms into it (smooth camera transition). Inside, each node is clickable to show the quest detail overlay (objectives, rewards, prerequisites) — reuse existing `.ff-detail` content. Add a breadcrumb: "All Clusters > Brain > Awaken the Mind". Back button zooms out to full constellation. Clicking an available node's "Begin Quest" starts the quest. Add a minimap in the corner showing all clusters with dots for active/available/locked status. | `not-started` |
| 130 | Brain config UI as RPG stat sheet — show Intelligence/Wisdom/Charisma/Perception/Dexterity/Endurance stats with animated bars. Each stat computed from unlocked skills. Visual feedback on stat increase when a new skill is unlocked. | `not-started` |
| 131 | Combo detection & notification system — when the user unlocks the second skill of a combo pair, show an RPG-style "Combo Unlocked!" animation with the combo name, icon, and bonus description. Persist combo state in skill-tree store. | `not-started` |
| 132 | Quest reward ceremony — after completing a quest, show a "Level Up" style reward screen with stat changes (before → after), new combos unlocked, and next recommended quest. Include particle effects and sound. | `not-started` |
| 133 | Brain evolution path visualization — show the brain upgrade tree (Free → Paid/Local) as a glowing neural pathway in the skill tree. Animate signals flowing along completed paths. Dim locked paths. | `not-started` |
| 134 | Stat-based AI behavior scaling — use the computed stats to actually influence AI behavior: higher Intelligence = longer context window, higher Wisdom = better memory recall, higher Charisma = more expressive TTS, higher Perception = faster hotword detection. | `not-started` |

> **Next Chunk:** 128 — FF16 Constellation Skill Tree layout redesign

