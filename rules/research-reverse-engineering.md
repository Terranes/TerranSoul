# Reverse Engineering Research — External Projects

> **Purpose:** Accelerate TerranSoul development by learning proven patterns from
> reference projects. This document records architecture, overlay systems,
> voice approaches, free LLM API strategies, and actionable takeaways chunked
> for incremental implementation.

---

## Table of Contents

1. [aituber-kit](#1-aituber-kit)
2. [Open-LLM-VTuber](#2-open-llm-vtuber)
3. [VibeVoice](#3-vibevoice)
4. [Overlay Comparison — What We're Doing Wrong](#4-overlay-comparison)
5. [Voice System Recommendation](#5-voice-system-recommendation)
6. [Actionable Chunks for TerranSoul](#6-actionable-chunks)
7. [AI4Animation-js — Brain-Driven Neural Animation](#7-ai4animation-js)
8. [Free LLM APIs — Three-Tier Brain Provider Strategy](#8-free-llm-apis)
9. [Integration Analysis — What We Can Learn & Implement](#9-integration-analysis)
10. [Cursor + Claude Code — Coding Workflow Lessons](#10-cursor--claude-code)
11. [CocoIndex — Incremental Code Indexing Architecture](#11-cocoindex)
12. [Claudia — Persistent Brain + Judgment Rules + Slash-Skill Workflows](#12-claudia)

---

## 1. aituber-kit

**Repo:** https://github.com/tegnike/aituber-kit/

### Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | Next.js 15.5, React 18, TypeScript |
| 3D Rendering | Three.js 0.167 + @pixiv/three-vrm 3.4 |
| State | Zustand 4.5 (with localStorage persistence) |
| Desktop | Electron 39 (transparent window) |
| Deploy | Cloudflare Workers (web mode) |

### Voice / TTS System (11+ providers)

1. Koeiromap (Japanese)
2. Voicevox (local)
3. Google Cloud TTS
4. StyleBertVITS2 (local neural)
5. AIVIS Speech (local Japanese)
6. AIVIS Cloud API
7. GSVI
8. ElevenLabs
9. Cartesia (real-time)
10. OpenAI TTS
11. Azure OpenAI TTS

**STT:** Web Speech API (browser-native) + OpenAI Whisper API.

### LLM Integration (Vercel AI SDK)

Supports: OpenAI, Anthropic (Claude with extended thinking), Google Gemini,
Azure, Groq, Cohere, Mistral, Perplexity, xAI (Grok), Fireworks, DeepSeek,
OpenRouter, Dify, LM Studio, Ollama.

Streaming via SSE JSON events (`text-delta`, `reasoning-delta`, `tool-input-start`).
Also supports plain-text and JSONL (Ollama) formats.

### Character Animation Pipeline

```
LLM Response
  → Extract emotion tags [happy] and motion tags [motion:wave]
  → Apply VRM Expression (neutral/happy/sad/angry/relaxed/surprised)
  → Apply Pose via PoseManager
  → Synthesize voice (TTS provider)
  → Play audio + real-time lip sync (Web Audio AnalyserNode → mouth morph)
  → AutoBlink + AutoLookAt running continuously
```

**Lip Sync:** Uses Web Audio API's `AnalyserNode` for frequency analysis.
Maps volume amplitude to VRM mouth shape (`aa` morph target). No external
lip sync library needed.

**Idle Animations:** Loads `idle_loop.vrma` (VRM Animation format). AI-generated
idle phrases via LLM when character is idle too long.

### Electron Desktop Overlay

```javascript
BrowserWindow({
  transparent: true,
  fullscreen: true,  // fills primary display
  webPreferences: { webSecurity: false, contextIsolation: true }
})
```

- Full-screen transparent window over desktop
- Keyboard shortcuts (Cmd/Ctrl+Shift+1-5) for preset switching
- Webcam input support for video background
- Green screen / transparent background modes

### Key Lessons for TerranSoul

1. **Streaming is critical** — Text chunks arrive via SSE, TTS starts on first chunks
   (character speaks while still receiving text). Apply to our existing Ollama brain.
2. **Web Audio API is sufficient for lip sync** — No external lib needed.
3. **Emotion tags in LLM responses** — Parse `[happy]`, `[motion:wave]` from text.
   Works with any brain model the user has configured.
4. **Voice abstraction layer** — aituber-kit supports 11 TTS providers via factory pattern.
   We should build a similar abstraction so users can pick their preferred provider.
5. **Zustand persistence** — Same pattern as our Pinia + localStorage.

---

## 2. Open-LLM-VTuber

**Repo:** https://github.com/Open-LLM-VTuber/Open-LLM-VTuber

### Architecture Overview

**Backend:** Python (FastAPI + WebSocket + uvicorn)
**Frontend:** Electron + React 18 + TypeScript (separate repo: Open-LLM-VTuber-Web)
**Communication:** WebSocket between Python server and Electron frontend

```
┌──────────────────────────────────────────────┐
│  Electron Desktop App (Open-LLM-VTuber-Web)  │
│  ┌────────────────────────────────────────┐   │
│  │  React UI + character canvas           │   │
│  │  (chatbox, character, settings)        │   │
│  └────────────────────────────────────────┘   │
│            ↕ WebSocket                        │
├──────────────────────────────────────────────┤
│  Python Server (FastAPI + uvicorn)            │
│  ┌──────────┐ ┌─────────┐ ┌──────────────┐   │
│  │  LLM     │ │  TTS    │ │  ASR (STT)   │   │
│  │  Agent   │ │  Engine │ │  Engine      │   │
│  └──────────┘ └─────────┘ └──────────────┘   │
│  ┌──────────┐ ┌─────────────────────────┐     │
│  │  VAD     │ │  Service Context        │     │
│  │  (Voice  │ │  (manages all engines)  │     │
│  │  Activity│ └─────────────────────────┘     │
│  │  Detect) │                                 │
│  └──────────┘                                 │
└──────────────────────────────────────────────┘
```

### ⭐ Desktop Overlay — "Pet Mode" (CRITICAL)

**This is what we need to learn.** Open-LLM-VTuber has TWO modes:

#### Mode 1: Window Mode (normal app)
```typescript
// window-manager.ts
window.setAlwaysOnTop(false);
window.setIgnoreMouseEvents(false);
window.setSkipTaskbar(false);
window.setResizable(true);
window.setFocusable(true);
window.setBackgroundColor('#ffffff');
```

#### Mode 2: Pet Mode (desktop overlay) ← THIS IS THE KEY
```typescript
// window-manager.ts — setWindowModePet()
window.setBackgroundColor('#00000000');  // Fully transparent
window.setAlwaysOnTop(true, 'screen-saver');  // Above everything

// continueSetWindowModePet() — spans ALL monitors
const displays = screen.getAllDisplays();
const minX = Math.min(...displays.map(d => d.bounds.x));
const minY = Math.min(...displays.map(d => d.bounds.y));
const maxX = Math.max(...displays.map(d => d.bounds.x + d.bounds.width));
const maxY = Math.max(...displays.map(d => d.bounds.y + d.bounds.height));
window.setBounds({ x: minX, y: minY, width: maxX-minX, height: maxY-minY });

// Critical: Click-through with selective interactivity
window.setResizable(false);
window.setSkipTaskbar(true);
window.setFocusable(false);

// macOS: simple ignore
window.setIgnoreMouseEvents(true);
// Windows/Linux: forward mouse events for hover detection
window.setIgnoreMouseEvents(true, { forward: true });
```

#### The Mouse Event Magic

The key innovation is **component-level hover tracking**:

```typescript
// Renderer sends: 'I'm hovering over the chatbox'
ipcMain.on('update-component-hover', (_event, componentId, isHovering) => {
  windowManager.updateComponentHover(componentId, isHovering);
});

// WindowManager tracks which components are hovered
updateComponentHover(componentId: string, isHovering: boolean): void {
  if (isHovering) {
    this.hoveringComponents.add(componentId);
  } else {
    this.hoveringComponents.delete(componentId);
  }
  // If ANY component is hovered → accept clicks
  // If NONE are hovered → pass clicks through to desktop
  const shouldIgnore = this.hoveringComponents.size === 0;
  window.setIgnoreMouseEvents(shouldIgnore, { forward: true });
}
```

**Result:** Character and chatbox float on desktop. Click on them = interact.
Click anywhere else = passes through to whatever app is underneath.

#### Force-Ignore Toggle

Also has a toggle to force click-through even over components (useful when
you want the character visible but non-interactive):

```typescript
ipcMain.on('toggle-force-ignore-mouse', () => {
  this.forceIgnoreMouse = !this.forceIgnoreMouse;
  // When force-ignore: always pass through
  // When normal: use component hover tracking
});
```

### Mode Switching Animation

Uses opacity fade for smooth transition:
1. Set window opacity to 0
2. Notify renderer to prepare for mode change
3. Renderer adapts layout (hides/shows window chrome)
4. Renderer signals `renderer-ready-for-mode-change`
5. After 500ms delay, apply actual window changes
6. Renderer signals `mode-change-rendered`
7. Restore window opacity to 1

### Backend Modules

| Module | Purpose |
|--------|---------|
| `agent/` | LLM integrations (Ollama, OpenAI, Anthropic, etc.) |
| `asr/` | Speech recognition (sherpa-onnx, FunASR, Faster-Whisper, etc.) |
| `tts/` | Speech synthesis (sherpa-onnx, MeloTTS, GPTSoVITS, Edge TTS, etc.) |
| `vad/` | Voice Activity Detection |
| `conversations/` | Chat history persistence |
| `websocket_handler.py` | Main WebSocket handler |
| `service_context.py` | Engine initialization & management |

### WebSocket Protocol

Frontend connects to `/client-ws`. Server handles:
- Text messages → LLM → TTS → audio response
- Audio data → ASR → text → LLM → TTS → audio response
- Configuration changes
- Character/emotion updates

### Key Lessons for TerranSoul

1. **Two-mode window is essential** — Window mode for setup/settings, Pet mode
   for daily use. We should NOT permanently lock to pet mode.
2. **Component-level hover tracking** — The key to overlay interactivity.
   Tauri can replicate this with `set_ignore_cursor_events()`.
3. **Multi-monitor support** — Window spans all displays in pet mode.
4. **Opacity-based mode transitions** — Smooth switching between modes.
5. **Separation of concerns** — They run LLM/TTS/ASR as backend services,
   frontend just renders. We already do this with Rust backend + Ollama.
   Users configure which services to use — not hardcoded.
6. **WebSocket for streaming** — Real-time audio/text streaming between processes.

---

## 3. VibeVoice

**Repo:** https://github.com/microsoft/VibeVoice

### Overview

Microsoft's open-source (MIT) voice AI framework. Built on Qwen2.5 language models
with continuous speech tokenizers. Uses next-token diffusion for synthesis.

### Models

| Model | Size | Purpose | Latency |
|-------|------|---------|---------|
| VibeVoice-ASR | 7B | Speech recognition + diarization + timestamps | Batch |
| VibeVoice-TTS | 1.5B | Long-form multi-speaker synthesis | Batch |
| VibeVoice-Realtime | 0.5B | Real-time streaming TTS | ~200-300ms |

### ASR Capabilities (VibeVoice-ASR-7B)

- **60-minute** audio in single pass (64K token context)
- **Speaker diarization** built-in (who is speaking)
- **Timestamps** for each utterance
- **50+ languages** with auto-detection
- **Code-switching** (mixed languages)
- **Hotword injection** — inject domain-specific terms for accuracy
- **LoRA fine-tuning** supported

### TTS Capabilities (VibeVoice-Realtime-0.5B)

- **~200-300ms** first audible latency (streaming)
- **Diffusion-based** synthesis (high quality, ICLR 2026)
- **11 English voice styles** + 9 language variants (DE, FR, IT, JP, KR, NL, PL, PT, ES)
- **Streaming text input** — can generate as text arrives
- **Embedded voice control** (no user voice cloning — safe)

### Integration Methods

1. **Python SDK** — Direct PyTorch models via `from_pretrained()`
2. **vLLM Plugin** — Production-scale inference with batching
3. **FastAPI Service** — REST + WebSocket endpoints
4. **HuggingFace Transformers** — Native support (v5.3.0+)

### VibeVoice vs Cloud Providers (Comparison for Users)

| Feature | Cloud providers (aituber-kit style) | VibeVoice (local) |
|---------|-------------------------------------|-------------------|
| ASR | Web Speech API / Whisper API | Local 7B model, 60min, diarization |
| TTS | 11 cloud providers | Local 0.5B model, ~300ms, MIT license |
| Offline | Mostly cloud-dependent | Fully offline capable |
| Languages | Per-provider, complex config | 50+ languages, auto-detect |
| Cost | API fees add up | Free, runs on consumer GPU |
| Customization | Limited to provider options | LoRA fine-tuning, hotwords |
| Privacy | Audio sent to cloud | Everything local |

> **Note:** TerranSoul does not prescribe a voice provider. Users choose
> based on their hardware, privacy needs, and preferences — same as the
> brain system where users pick their own LLM model.

### Key Lesson

VibeVoice demonstrates that **high-quality local voice AI is now feasible**:
- 0.5B model runs alongside Ollama on 8GB+ VRAM
- ~300ms latency is fast enough for real-time conversation
- MIT licensed, no vendor lock-in
- Streaming text input pairs perfectly with LLM streaming output

TerranSoul should offer VibeVoice as **one option** in the voice provider
selection UI — alongside sherpa-onnx, Edge TTS, OpenAI, etc. Users choose.

---

## 4. Overlay Comparison — What We're Doing Wrong {#4-overlay-comparison}

### Current TerranSoul Approach

```json
// tauri.conf.json
{
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "skipTaskbar": true
}
```

**Problems:**
1. ❌ **Always in pet mode** — No normal window mode for settings/setup
2. ❌ **No click-through** — Tauri window blocks all mouse events
3. ❌ **No multi-monitor** — Fixed 420×700 window, can't span displays
4. ❌ **No mode switching** — Can't toggle between window and overlay
5. ❌ **Chat section blocks clicks** — 45% of window is opaque chat panel

### Open-LLM-VTuber's Approach (What Works)

1. ✅ **Dual-mode system** — Window mode (normal app) + Pet mode (overlay)
2. ✅ **Selective click-through** — Character area passes clicks, chatbox catches them
3. ✅ **Multi-monitor** — Pet mode spans all displays
4. ✅ **Smooth transitions** — Opacity fade between modes
5. ✅ **System tray menu** — Mode switching, config, quit

### What TerranSoul Needs

Tauri 2.0 supports the same capabilities as Electron for overlays:

```rust
// Tauri equivalent of Electron's approach
use tauri::Manager;

// Pet mode
window.set_always_on_top(true)?;
window.set_decorations(false)?;
window.set_skip_taskbar(true)?;
window.set_ignore_cursor_events(true)?;  // Click-through!
window.set_resizable(false)?;

// Window mode
window.set_always_on_top(false)?;
window.set_decorations(true)?;
window.set_skip_taskbar(false)?;
window.set_ignore_cursor_events(false)?;
window.set_resizable(true)?;
```

**For selective click-through** (the key innovation), Tauri 2.0 has
`set_ignore_cursor_events(ignore: bool)` which can be toggled dynamically
from the frontend via IPC, just like Electron's approach.

The frontend would:
1. Track mouse position over components
2. When over character/empty area → `invoke('set_ignore_cursor_events', true)`
3. When over chatbox/buttons → `invoke('set_ignore_cursor_events', false)`

---

## 5. Voice System — Abstraction Layer (User Chooses Provider) {#5-voice-system-recommendation}

> **Design principle:** TerranSoul already lets users choose their own brain
> (LLM model via the Ollama brain setup wizard). The same philosophy applies
> to voice — we build the **abstraction layer**, users pick their preferred
> ASR/TTS engine. We do NOT hardcode a single provider.

### Voice Provider Options (for users to choose from)

**ASR (Speech-to-Text) options:**

| Provider | Type | VRAM | Strengths |
|----------|------|------|-----------|
| VibeVoice-ASR-7B | Local (Python sidecar) | ~14GB | 60min, diarization, 50+ langs |
| sherpa-onnx | Local (ONNX) | CPU-only | Lightweight, works on low-end |
| OpenAI Whisper API | Cloud | None | High accuracy, simple API |
| Web Speech API | Browser-native | None | Zero setup, limited accuracy |

**TTS (Text-to-Speech) options:**

| Provider | Type | VRAM | Strengths |
|----------|------|------|-----------|
| VibeVoice-Realtime-0.5B | Local (Python sidecar) | ~2GB | ~300ms, MIT, streaming |
| sherpa-onnx | Local (ONNX) | CPU-only | Lightweight, many voices |
| Edge TTS | Cloud (free) | None | High quality, many languages |
| OpenAI TTS | Cloud (paid) | None | Best quality, simple API |
| ElevenLabs | Cloud (paid) | None | Voice cloning, expressive |

### Integration Architecture

TerranSoul provides:
1. **Rust traits** — `AsrEngine` and `TtsEngine` with async methods
2. **Config-driven selection** — Users pick provider in VoiceSetupView.vue
   (same UX pattern as BrainSetupView.vue)
3. **Sidecar support** — For Python-based engines, Tauri spawns a FastAPI
   sidecar. Users configure which sidecar to run.
4. **Direct HTTP/WebSocket** — For cloud APIs, frontend calls directly.
5. **Graceful fallback** — Text-only mode if no voice provider configured.

This matches TerranSoul's philosophy: **we provide the platform, users
bring their preferred AI services.**

---

## 6. Actionable Chunks for TerranSoul {#6-actionable-chunks}

> **These chunks are registered in `rules/milestones.md` (Phase 5 & 6).**
> Agents will implement them via the standard `Continue` workflow.
>
> **Design principle:** TerranSoul already has a brain system where users
> choose their own LLM model. All new features follow the same philosophy —
> we build abstraction layers, users bring their preferred services.

### Phase 5 — Desktop Experience (Chunks 050–054)

#### Chunk 050 — Window Mode System (High Priority)

**Goal:** Implement dual-mode window (normal + pet mode overlay)

- Add `WindowMode` enum to Pinia store: `'window' | 'pet'`
- Create Tauri commands: `set_window_mode`, `get_window_mode`
- Window mode: `decorations: true, alwaysOnTop: false, skipTaskbar: false`
- Pet mode: `decorations: false, alwaysOnTop: true, skipTaskbar: true,
  ignore_cursor_events: true, full-screen transparent`
- Add system tray toggle between modes
- Opacity-fade transition (match Open-LLM-VTuber pattern)
- Default to window mode on first launch, pet mode after setup

#### Chunk 051 — Selective Click-Through (High Priority)

**Goal:** In pet mode, clicks pass through empty areas but interact with
character and chatbox

- Frontend tracks mouse position (addEventListener 'mousemove')
- Detect hover over interactive elements (chatbox, buttons, character)
- On hover enter → `invoke('set_cursor_passthrough', false)`
- On hover leave → `invoke('set_cursor_passthrough', true)`
- Tauri command calls `window.set_ignore_cursor_events()`
- Test on Windows + macOS (different behaviors, match Open-LLM-VTuber)

#### Chunk 052 — Multi-Monitor Pet Mode (Medium Priority)

**Goal:** Pet mode window spans all connected displays

- Tauri command queries available monitors via `app.available_monitors()`
- Calculate bounding rect of all monitors
- Set window bounds to combined rect
- Character position stored relative to combined screen space
- Allow dragging character between monitors

#### Chunk 053 — Streaming LLM Responses (High Priority)

**Goal:** Stream text from the user's active brain instead of waiting for full response

- Modify OllamaAgent to use streaming API (`/api/chat` with `stream: true`)
- Emit Tauri events for each text chunk
- Frontend subscribes and appends text progressively
- Character starts "talking" animation on first chunk (not after full response)
- Works with whatever brain model the user has configured

#### Chunk 054 — Emotion Tags in LLM Responses (Medium Priority)

**Goal:** User's brain model tags emotions in responses

- Add system prompt instructions for emotion tagging: `[happy] text here`
- Parse emotion tags before displaying text (strip from visible message)
- Map to VRM expressions: happy, sad, angry, relaxed, surprised, neutral
- Optional motion tags: `[motion:wave]`, `[motion:nod]`
- Integrate with character-animator state machine

### Phase 6 — Voice (Chunks 060–063)

#### Chunk 060 — Voice Abstraction Layer (High Priority)

**Goal:** Let users choose their own voice providers (same philosophy as brain)

- Rust traits `AsrEngine` (async `transcribe(audio) → text`) and
  `TtsEngine` (async `synthesize(text) → audio`)
- Config-driven provider selection (same pattern as `AgentProvider` trait)
- Stub implementations for testing
- Tauri commands: `list_voice_providers`, `set_asr_provider`, `set_tts_provider`
- VoiceSetupView.vue for users to pick ASR/TTS provider and configure
  endpoints/API keys (mirrors BrainSetupView.vue UX)

#### Chunk 061 — Web Audio Lip Sync (Medium Priority)

**Goal:** Animate VRM mouth based on audio output (provider-agnostic)

- Create `LipSync` class using Web Audio API `AnalyserNode`
- Connect TTS audio output to analyser
- Extract volume from `getFloatTimeDomainData()`
- Map volume → VRM mouth morph target (`aa`, `oh`)
- Run in requestAnimationFrame loop alongside character animator
- Works with ANY TTS audio output regardless of provider

#### Chunk 062 — Voice Activity Detection (Medium Priority)

**Goal:** Detect when user is speaking, interrupt AI response

- Use `@ricky0123/vad-web` (ONNX, same as Open-LLM-VTuber-Web)
- Detect speech start → pause AI audio, send to user's configured ASR
- Detect speech end → submit transcription to brain
- Handle "AI won't hear itself" — mute TTS during mic capture

#### Chunk 063 — Voice Sidecar Support (Lower Priority)

**Goal:** Support Python-based voice engines as sidecars

- For engines like VibeVoice, sherpa-onnx, etc. that need Python runtime
- Tauri spawns a FastAPI sidecar on startup, health-checks on `/health`
- STT: Browser MediaRecorder → POST audio to `/api/asr`
- TTS: POST text to `/api/tts` or stream via `/ws/tts`
- Users configure which sidecar to run (not hardcoded)
- Graceful fallback to text-only if sidecar unavailable

---

> **Next session:** Pick the highest-priority chunk not already being handled
> by another agent. Chunks 050 and 051 should come first (they fix the overlay
> problem). Skip animation-related chunks as another agent handles those.

---

## 7. AI4Animation-js — Brain-Driven Neural Animation {#7-ai4animation-js}

> **Reverse-engineered from:** https://github.com/sneha-belkhale/AI4Animation-js
> **Original research:** Sebastian Starke et al., "Mode-Adaptive Neural Networks
> for Quadruped Motion Control", SIGGRAPH 2018.
> **Python remake (2026):** https://github.com/facebookresearch/ai4animationpy

### What It Is

AI4Animation-js is a Three.js port of the SIGGRAPH 2018 MANN (Mode-Adaptive
Neural Networks) paper. Instead of using pre-baked animation clips, a neural
network generates bone positions and velocities **every frame** based on:

1. **Trajectory input** — Where the character should go (position, direction,
   velocity, speed, style weights for 6 locomotion styles).
2. **Previous pose** — Current bone positions, forward vectors, up vectors,
   and velocities for all 27 bones.
3. **Neural network prediction** — Outputs next bone positions/velocities plus
   root motion (translation + rotation).

The result: **unlimited smooth transitions** between motion states with no
blend trees or crossfades. The character simply moves however the neural
network decides is natural.

### Architecture (from code analysis)

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│ User Input   │───▶│ Trajectory   │───▶│ MANN Neural  │
│ (WASD keys)  │    │ Prediction   │    │ Network      │
│              │    │ (12 points)  │    │              │
└──────────────┘    └──────────────┘    │ Input: 480   │
                                        │ Hidden: 512  │
┌──────────────┐                        │ Output: 363  │
│ Previous     │───────────────────────▶│              │
│ Bone State   │                        │ Gating Net:  │
│ (27 bones ×  │                        │ 19→32→8 blend│
│ 12 dims)     │                        │ weights      │
└──────────────┘                        └──────┬───────┘
                                               │
                                               ▼
                                        ┌──────────────┐
                                        │ Output Parse │
                                        │ • Trajectory │
                                        │ • Bone pos   │
                                        │ • Bone vel   │
                                        │ • Root motion│
                                        └──────┬───────┘
                                               │
                                               ▼
                                        ┌──────────────┐
                                        │ Skeleton     │
                                        │ Retarget     │
                                        │ (Wolf.js)    │
                                        └──────────────┘
```

#### Key Files

| File | Role | Size |
|------|------|------|
| `MANNNeuralNet.js` | MANN: gating network + expert blending + prediction | 4.4 KB |
| `MainScene.js` | Main loop: packs NN inputs, calls predict, reads outputs, updates character | 15 KB |
| `Wolf.js` | Skeleton retargeting: NN bone positions → FBX bone quaternions via `updatePose()` | 9.6 KB |
| `Trajectory.js` | 12-point trajectory (position, direction, velocity, styles per point) | 1.6 KB |
| `Parameters.js` | Loads `.bin` weight files into numjs matrices | 611 B |
| `Eigen.js` | Linear algebra: Layer, ELU, SoftMax, Normalise, Blend | 1.2 KB |
| `AxisUtils.js` | `setZForward()`: recursively aligns bone +Z to face children | 2.7 KB |
| `Utils.js` | Coordinate transforms: relative position/direction to/from a root matrix | 1.7 KB |

#### MANN Neural Network Details

The MANN is a **mixture-of-experts** architecture:

1. **Gating Network** (small): 19 control neurons → 32 hidden (ELU) → 32 hidden
   (ELU) → 8 expert weights (softmax). Selects blend weights for 8 expert
   sub-networks based on the character's current motion style.

2. **Expert Networks** (8 sets of weights): Each expert has 3 layers
   (480→512→512→363). Weights are **blended** by gating output before forward
   pass, producing a single effective network per frame.

3. **Forward Pass**:
   ```
   X_normalized = (X - Xmean) / Xstd
   control_neurons = X_normalized[ControlNeurons]  // 19 specific indices
   blend_weights = softmax(gating_network(control_neurons))  // 8 weights
   W0, b0, W1, b1, W2, b2 = Σ(weight_i × expert_i_params)
   Y_normalized = ELU(ELU(X_normalized · W0 + b0) · W1 + b1) · W2 + b2
   Y = Y_normalized × Ystd + Ymean
   ```

4. **Input (480 dims)**: 12 trajectory points × 13 dims (pos.xz, dir.xz,
   vel.xz, speed, 6 styles) + 27 bones × 12 dims (pos.xyz, forward.xyz,
   up.xyz, velocity.xyz).

5. **Output (363 dims)**: 6 future trajectory updates × 6 dims + 27 bones ×
   12 dims (new pos/forward/up/vel) + 3 root motion (translation.xz, rotation).

#### Skeleton Retargeting (Wolf.js → VRM adaptation needed)

The key insight from Wolf.js is how NN bone positions are converted to skeleton
quaternions:

```javascript
// For each bone, compute direction to average child position
averagedDir = average(children.map(c => BONES[c.posRef].position))
averagedDir.sub(parentBonePos)
localDir = averagedDir.normalize().transformDirection(inverse(parent.matrixWorld))
setQuaternionFromDirection(localDir, bone.originalUp, bone.quaternion)
```

This is a **direction-based IK** approach: each bone rotates to point toward
where its children should be (as predicted by the NN). Combined with rest-pose
length preservation, this produces smooth, natural skeletal animation.

For VRM humanoid characters, this would need:
- Map NN bone indices → VRM humanoid bone names (hips, spine, chest, head,
  upperArm, lowerArm, hand, upperLeg, lowerLeg, foot, etc.)
- Replace Wolf's 27-bone topology with VRM's ~55 humanoid bones
- Maintain same direction-based quaternion computation

### How This Applies to TerranSoul

**The critical insight:** Instead of the original MANN approach (which requires
massive mocap datasets and offline TensorFlow training), we can use TerranSoul's
**existing LLM brain** to generate animation parameters.

#### Approach: LLM → Animation Parameter Generation

The brain already understands emotion tags (`[happy]`, `[sad]`) and motion tags
(`[motion:wave]`). We can extend this to generate **continuous animation
parameters** per response:

1. **Emotion → Pose Blend Weights**: The LLM output emotion tag maps to a set
   of blend weights for predefined pose clusters (like MANN's 8 experts but
   for VRM humanoid poses: confident stance, shy stance, excited bounce, etc.)

2. **Motion Tag → Procedural Trajectory**: `[motion:nod]` generates a
   trajectory for the head bone, `[motion:wave]` for the arm chain. The brain
   can describe custom motions: `[motion:lean-forward]`, `[motion:look-away]`.

3. **Conversational Context → Dynamic Pose**: Beyond single-word tags, the brain
   generates structured pose data:
   ```
   [pose: { head_tilt: 0.15, body_lean: -0.05, gesture: "open_palms" }]
   ```
   This creates **context-appropriate animation** — leaning in during questions,
   crossing arms during disagreement, etc.

#### Implementation Strategy (What's Feasible for TerranSoul)

Rather than training a full MANN network (which requires massive mocap data and
GPU training), TerranSoul adapts the **core concepts**:

**What we take from AI4Animation:**
- Expert blending architecture (multiple pose presets blended by weights)
- Per-frame bone position → quaternion retargeting via direction-based IK
- Autoregressive feedback (previous pose feeds into next prediction)

**What we replace:**
- MANN neural network → LLM-driven emotion/motion parameter generation
- Mocap training data → Hand-authored VRM pose presets + procedural generation
- Trajectory planning → Stationary VRM (no locomotion needed — it's a desktop
  companion, not a game character)

**The result:** AI-driven character animation that reacts naturally to
conversation context, without needing mocap data or neural network training.
The brain IS the animation controller.

### TerranSoul Adaptation: Phase 8 Chunks

See `rules/milestones.md` Phase 8 for implementation chunks 080–084.

---

## 8. Free LLM APIs — Three-Tier Brain Provider Strategy {#8-free-llm-apis}

> **Source:** https://github.com/mnfst/awesome-free-llm-apis
> **Design principle:** TerranSoul should work out of the box with zero setup.
> Free cloud LLM APIs make this possible — no Ollama install, no GPU required.

### Problem

The current brain system requires users to:
1. Install Ollama
2. Download a multi-GB model
3. Have enough RAM/VRAM to run it

This creates a huge barrier to first-use. Many users (especially on low-end
machines, Chromebooks, or during UAT) cannot or don't want to set up local
inference.

### Solution: Three-Tier Brain Provider System

```
┌─────────────────────────────────────────────────────┐
│ Tier 1: FREE Cloud APIs (default, zero setup)       │
│ ─────────────────────────────────────────────        │
│ • No API key needed (or free key from provider)     │
│ • Auto-rotate between providers when rate-limited   │
│ • Curated from awesome-free-llm-apis               │
│ • Works immediately on any hardware                 │
├─────────────────────────────────────────────────────┤
│ Tier 2: PAID Cloud APIs (user provides API key)     │
│ ─────────────────────────────────────────────        │
│ • OpenAI, Anthropic, Google, Mistral, etc.          │
│ • Higher rate limits, better models                 │
│ • User enters their own API key                     │
├─────────────────────────────────────────────────────┤
│ Tier 3: LOCAL LLM (Ollama, existing system)         │
│ ─────────────────────────────────────────────        │
│ • Full privacy, no internet needed                  │
│ • Requires Ollama + model download                  │
│ • Best for power users with good hardware           │
└─────────────────────────────────────────────────────┘
```

### Auto-Detection Logic

```
App starts → try get_system_info()
  ├─ FAILS (no Tauri backend, UAT, web preview)
  │   → Default to Tier 1 (Free API)
  │
  └─ SUCCEEDS
      ├─ Low-end (< 8GB RAM, no GPU)
      │   → Recommend Tier 1 (Free API)
      │   → Show Tier 2 as upgrade option
      │
      ├─ Mid-range (8–16GB RAM)
      │   → Show all three tiers
      │   → Default to Tier 1 for instant start
      │
      └─ High-end (16GB+ RAM, GPU)
          → Show all three tiers
          → Highlight Tier 3 (Local) as recommended
```

### Free Provider Catalogue (from awesome-free-llm-apis)

All endpoints are **OpenAI SDK-compatible** unless noted. This means we need
ONE generic client that works for all of them.

#### Provider APIs (trained by the company itself)

| Provider | Models | Rate Limits | Notes |
|----------|--------|-------------|-------|
| Google Gemini | Gemini 2.5 Pro/Flash | 5-15 RPM, 100-1K RPD | Not available in EU/UK/CH |
| Mistral AI | Large 3, Small 3.1 | 1 req/s, 1B tok/mo | EU-based, generous limits |
| Cohere | Command A, Command R+ | 20 RPM, 1K/mo | Good for conversation |

#### Inference Providers (host open-weight models)

| Provider | Models | Rate Limits | Priority |
|----------|--------|-------------|----------|
| Groq | Llama 3.3 70B, Kimi K2 | 30 RPM, 1K RPD | **High** — fast, reliable |
| Cerebras | Llama 3.3 70B, Qwen3 235B | 30 RPM, 14.4K RPD | **High** — generous limits |
| GitHub Models | GPT-4o, Llama 3.3 70B | 10-15 RPM, 50-150 RPD | Medium |
| OpenRouter | DeepSeek R1, Llama 3.3 70B | 20 RPM, 50 RPD | Medium (1K RPD w/ $10) |
| Cloudflare Workers AI | Llama 3.3 70B, Qwen QwQ 32B | 10K neurons/day | Medium |
| NVIDIA NIM | Llama 3.3 70B, Mistral Large | 40 RPM | Medium |
| SiliconFlow | Qwen3-8B, DeepSeek-R1 | 1K RPM, 50K TPM | High — very generous |
| Ollama Cloud | DeepSeek-V3.2, Qwen3.5 | Light usage | Uses Ollama API, not OpenAI |

### Token Rotation Strategy

```
1. On app start, load provider list (curated, not fetched live)
2. For each request:
   a. Pick the first healthy provider with quota remaining
   b. Send request
   c. Parse rate-limit headers from response:
      - x-ratelimit-remaining-requests
      - x-ratelimit-remaining-tokens
      - x-ratelimit-reset
   d. If 429 (rate limited) → mark provider exhausted, try next
   e. If success → record usage, return response
3. If ALL free providers exhausted:
   → Show user notification: "Free quota used up"
   → Suggest upgrading to Paid API or Local LLM
```

### OpenAI-Compatible Chat API (Unified Client)

Since nearly all providers use the OpenAI chat completions format, we need
ONE client that handles:

```
POST {base_url}/v1/chat/completions  (or /chat/completions)
{
  "model": "llama-3.3-70b",
  "messages": [{ "role": "system", "content": "..." }, ...],
  "stream": true
}

Response (streaming SSE):
data: {"choices":[{"delta":{"content":"Hello"}}]}
data: {"choices":[{"delta":{"content":" world"}}]}
data: [DONE]
```

This replaces the Ollama-specific NDJSON streaming with standard SSE streaming.
The Ollama path remains as a separate code path for Tier 3 (local).

### Implementation Chunks

See `rules/milestones.md` Phase 5.5 for chunks 055–057.

---

## 9. Integration Analysis — What We Can Learn & Implement {#9-integration-analysis}

> **Date:** 2026-04-14
> **Source repos:** All four above, re-analyzed for implementable patterns.
> **Backlog chunks:** See `rules/backlog.md` Phase 9. The original Phase 9
> research range has since been reconciled: high-priority and medium-priority
> items are archived, the shipped lower-priority utility items are backfilled in
> `rules/completion-log.md`, and closed/demoted items remain out of active
> milestones unless the user explicitly re-promotes them.

### Summary Matrix

| Feature | Source Repo | Effort | Impact | Priority |
|---------|-------------|--------|--------|----------|
| Streaming TTS | VibeVoice | Medium | Very High | **High** |
| Multi-ASR provider abstraction | Open-LLM-VTuber | Low | High | **High** |
| Settings persistence + env overrides | aituber-kit | Low | High | **High** |
| Idle action sequences | aituber-kit | Medium | High | **High** |
| Model position saving | aituber-kit | Low | Medium | **Medium** |
| Procedural gesture blending (MANN) | AI4Animation-js | High | Medium | **Medium** |
| Speaker diarization | VibeVoice | Medium | Medium | **Medium** |
| Hotword-boosted ASR | VibeVoice | Low | Low | **Medium** |
| Presence / greeting system | aituber-kit | Medium | Medium | **Medium** |
| Screen recording / vision | Open-LLM-VTuber | High | Low | **Low** |
| Docker containerization | Open-LLM-VTuber | High | Medium (CI) | **Low** |
| Chat log export | Open-LLM-VTuber | Low | Low | **Low** |
| Language translation layer | VibeVoice + LLM | Medium | Low | **Low** |

### Key Patterns to Adopt

**From aituber-kit:**
- **Idle action queue** — Timer-based idle phrases, auto-greetings, face detection.
  The character should feel alive when the user is away.
- **Settings hydration** — Pre-validate schema before loading persisted state.
  Prevents corruption on rapid restart or auto-update.
- **Model position persistence** — Camera orbit + zoom saved per model.

**From Open-LLM-VTuber:**
- **Agent factory pattern** — Plugin-style provider registration for LLM/TTS/ASR.
  We already have this for LLM brains; extend to voice providers.
- **Docker for CI** — Containerized testing environment for cross-platform validation.

**From VibeVoice:**
- **Streaming TTS** — Voice starts 200ms after first LLM token. Currently our
  Edge TTS waits for the full response. This is the single biggest UX improvement.
- **Hotword injection** — Let users define custom vocabulary for better ASR accuracy.

**From AI4Animation-js:**
- **MANN mixture-of-experts** — 8 expert networks blended by a gating network.
  Our pose blending system already does something similar (weighted presets).
  The next step is to make blending weights learned/adaptive instead of static.
- **Direction-based bone IK** — Each bone rotates to point toward its children's
  predicted position. More natural than static Euler offsets.

### What We Already Have (No Action Needed)

These features from the reference repos are already implemented in TerranSoul:

- ✅ Emotion tags from LLM (`[happy]`, `[sad]`, etc.) — aituber-kit pattern
- ✅ Motion tags (`[motion:wave]`, `[motion:nod]`) — aituber-kit pattern
- ✅ Desktop pet mode with click-through — Open-LLM-VTuber pattern
- ✅ Dual-mode window (normal + pet) — Open-LLM-VTuber pattern
- ✅ Free cloud LLM APIs with rotation — inspired by awesome-free-llm-apis
- ✅ Voice abstraction (Edge TTS + Whisper API) — aituber-kit/VibeVoice inspired
- ✅ VRM expression system (6 emotions) — aituber-kit pattern
- ✅ Pose blending (additive Euler offsets) — AI4Animation-inspired
- ✅ Gesture system (nod, wave, shrug, etc.) — original implementation
- ✅ NotebookLM-style source guides for imported documents — public NotebookLM source-grounding pattern, implemented in Chunk 30.1 as deterministic source-summary rows that reduce broad document Q&A prompt tokens before raw chunks are needed

---

## 10. Cursor + Claude Code — Coding Workflow Lessons {#10-cursor--claude-code}

> **Date:** 2026-05-02
> **Sources:** Cursor public workflow concepts (rules, memories, codebase
> context, checkpoints/review) and Claude Code documentation (project memory,
> path-scoped rules, subagents, hooks, background/forked work, worktree
> isolation).

### Patterns worth absorbing

| Pattern | Cursor / Claude Code behavior | TerranSoul adaptation |
|---|---|---|
| Project rules + memory | Rules and memory files are loaded with the coding session so the agent does not rediscover conventions every turn. | `load_workflow_context` already injects `rules/`, `instructions/`, and `docs/`; keep the files concise and deterministic. |
| Context surfacing | Agents curate relevant codebase context before editing instead of dumping everything. | Chunk 28.14 adds `applyTo` frontmatter plus target-path filtering so scoped rules/docs load only for matching files. |
| Reviewable checkpoints | Agent edits are grouped into a reviewable checkpoint that can be accepted or reverted. | Chunk 28.11 snapshots touched files before apply and restores them on review/test failure. |
| Specialized subagents | Claude Code uses read-only Explore, reviewer, debugger, and test-runner style workers with restricted tools. | TerranSoul has `reviewer`, `test_runner`, and `dag_runner`; 28.11 wires reviewer/tester into the engine, while 28.12 wires the DAG. |
| Hooks / guarded commands | Claude Code hooks can validate tool use before and after actions. | TerranSoul should keep enforcement in Rust: clean-tree preflight, path validation, reviewer verdict, and test gate before staging. |
| Worktree isolation | Claude Code can isolate subagent edits in temporary worktrees. | Chunk 28.13 now runs dirty-checkout coder/tester paths in a detached temporary git worktree and saves a patch artifact for review. |

### Implemented in Chunk 28.11

- ✅ Coder output is constrained to complete `<file path="...">` blocks.
- ✅ Preview diff is reviewed before any file is written.
- ✅ Touched files are snapshotted and restored if apply or tests fail.
- ✅ Dirty working trees are refused before autonomous apply.
- ✅ Tests run through `coding::test_runner`; changed files are staged only after a green gate.

### Implemented in Chunk 28.12

- ✅ `coding::dag_runner` has an async executor for bounded parallel execution within topological layers.
- ✅ The self-improve engine runs a real Planner → Coder → Reviewer → Apply → Tester → Stage DAG.
- ✅ DAG nodes declare capabilities (`llm_call`, `review`, `file_write`, `test_run`, `git_write`) and validate them before execution.
- ✅ Downstream nodes are skipped when a predecessor fails, so rejected reviews or failed tests cannot reach staging.

### Implemented in Chunk 28.13

- ✅ `coding::worktree` creates detached temporary git worktrees and cleans them up with `git worktree remove --force`.
- ✅ Dirty active checkouts run autonomous apply/test/stage against the temporary worktree instead of mixing generated edits with user changes.
- ✅ Successful isolated runs capture the staged binary diff under `target/terransoul-self-improve/patches` for review.

### Implemented in Chunk 28.14

- ✅ `workflow::load_workflow_context_for_paths` filters markdown context files by `applyTo` frontmatter and caller-supplied target paths.
- ✅ `CodingTask.target_paths` lets reusable coding tasks request only relevant scoped rules/docs while preserving legacy loading when no target paths are supplied.
- ✅ Self-improve coder prompts derive likely repo path hints from the approved plan so file-specific context can be narrowed before implementation.

### Follow-up for Chunk 28.12+

All follow-ups from the Cursor/Claude Code comparison are implemented as of Chunk 28.14.

---

## 11. claw-code / Claude Code / OpenClaw — Self-improve UX & sessions {#11-claw-clawcode-openclaw}

> **Date:** 2026-05-04
> **Sources:** [ultraworkers/claw-code](https://github.com/ultraworkers/claw-code)
> README + USAGE, [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
> + overview, OpenClaw (the OG public Claude Code reverse-engineering project that
> claw-code derives from). All three centre on a single CLI agent harness with
> rich session management, slash commands, and resumable conversations.

### Patterns worth absorbing

| Pattern | claw-code / Claude Code / OpenClaw | TerranSoul adaptation |
|---|---|---|
| Named, resumable sessions | `claude --name "auth-refactor"` plus `claude -r "auth-refactor"` resume by name or interactive picker; `--continue` resumes the most recent session in the working directory. claw-code stores per-project session state under `.claude/sessions/`. | Reuse the existing `coding::handoff_store` (per-`session_id` JSON snapshots) for metadata. Add per-session chat-history JSONL alongside it so each session has both a structured handoff seed *and* the verbatim turn-by-turn transcript. Surface a sidebar with "resume" / "rename" / "delete" actions in the self-improve panel. |
| Persistent chat history | Claude Code retains the full transcript per session and lets you scroll back, search, and resume mid-conversation. | New `coding::session_chat` module storing append-only JSONL of `{role, content, ts_ms, kind}` turns. Bounded reads (last N turns) for cheap UI loading. |
| Slash commands inside the input | `/clear`, `/rename`, `/resume`, `/help`, `/status`, `/list` — all dispatched from the input bar, not buttons. | The self-improve chat input parses leading `/` commands locally before sending; unknown slash commands surface a lightweight error toast. Mirrors the Claude Code interactive shell. |
| New session without losing the old | `--fork-session` clones the current session under a new id so experiments do not corrupt the working session. | A "fork" action in the sessions sidebar copies the chat JSONL + handoff JSON under a new id. |
| Compact session summaries | claw-code's `claw doctor` and Claude Code's `/status` show a short health line per session: chunk id, last action, byte size. | `coding_session_list_handoffs` already returns `HandoffSummary{chunk_id, last_action, modified_at, bytes}`; extend the list to include chat message count and last-user-message preview. |
| Project-purge / cleanup | `claude project purge` wipes transcripts, debug logs, prompt history. | `coding_session_clear_handoff` already exists for metadata; mirror it with `coding_session_clear_chat` and a `coding_session_purge` that does both. |

### Mapping to TerranSoul code

- Backend: extend `src-tauri/src/coding/` with a new `session_chat.rs` module
  (pure storage + sanitiser, mirrored on the existing `handoff_store.rs`
  shape). Add new Tauri commands in a new `src-tauri/src/commands/coding_sessions.rs`
  registered through `commands/mod.rs` and `lib.rs`.
- Frontend: extend `src/stores/self-improve.ts` with a `sessions` slice
  (list + active session + chat scrollback) and add a
  `SelfImproveSessionsPanel.vue` component embedded in the existing
  `SelfImprovePanel.vue`. Slash-command parsing lives in a small pure helper
  under `src/utils/` so it is unit-testable without mounting the component.
- Tests: pure-function unit tests for the storage module + the slash-command
  parser, plus a Vitest component test for the session sidebar.

### Implementation slice for Chunk 30.2

The minimum vertical slice covers session list, chat history, resume, fork,
rename, clear, and a small slash-command palette (`/clear`, `/rename`,
`/fork`, `/resume`, `/help`). Autonomous loop integration (auto-appending
runs to the active session's chat) is intentionally out of scope for this
chunk — it depends on the in-progress autonomy loop and lands in a follow-up.

### Implemented in Chunk 30.2

- ✅ `coding::session_chat` JSONL transcript store (append/load/clear/summary/fork) sitting next to existing handoff snapshots.
- ✅ Tauri commands `coding_session_{list,append_message,load_chat,clear_chat,rename,fork,purge}` registered in `lib.rs`.
- ✅ `SelfImproveSessionsPanel.vue` sidebar + transcript scrollback embedded in the existing `SelfImprovePanel.vue`.
- ✅ Pure `parseSlashCommand` parser (`/clear`, `/rename`, `/fork`, `/resume`, `/list`, `/help`, plus chat fall-through and unknown-command discriminator) with full unit coverage.
- ✅ Self-improve store gains a session slice with null-safe `Array.isArray` guards so mocked `invoke` returns no longer crash the panel.

### Implemented follow-up in Chunk 30.6

- ✅ Auto-append autonomous-loop `self-improve-progress` events to the active session's transcript as `system` / `run` messages.
- ✅ Create a timestamped `self-improve-*` transcript session when progress arrives without a selected session.
- ✅ Include transcript-only sessions in the session picker so run history remains resumable before a handoff snapshot exists.

---

## 11. CocoIndex — Incremental Code Indexing Architecture {#11-cocoindex}

> **Date:** 2026-05-04
> **Source:** [cocoindex-io/cocoindex](https://github.com/cocoindex-io/cocoindex)
> v1.0.2, Apache 2.0 license. Python + Rust framework. 7.8k stars.
> **Product:** CocoIndex-code — MCP server for AI coding agents.

### What CocoIndex is

A **declarative incremental data pipeline framework** ("React for data
engineering"). You declare `Target = F(Source)` and the engine keeps the
target in sync with the source forever — recomputing only the Δ.

The *CocoIndex-code* product applies this framework to code: AST-aware
incremental semantic code index that keeps live call graphs, symbols,
vectors, and chunks fresh on every commit.

### Tech Stack

| Layer | Technology |
|-------|------------|
| Core engine | Rust (parallel chunking, zero-copy transforms) |
| User-facing API | Python (`pip install cocoindex`) |
| State store | PostgreSQL (persistent pipeline state) |
| Vector targets | pgvector, LanceDB, Qdrant, Pinecone, Turbopuffer |
| Graph targets | Neo4j, Kuzu, SurrealDB |
| Embeddings | sentence-transformers, OpenAI, any provider |
| MCP | CocoIndex-code server (Claude Code, Cursor) |

### Key Architecture Patterns

#### 1. Δ-only incremental processing

The core insight: **don't re-index what hasn't changed.**

```
hash(input_content) + hash(transformation_code) → cache key
```

- Each source file gets a content hash on ingest
- When re-indexed, only files with changed hashes re-parse/re-embed
- Reports 80–90% cache hits on typical re-index runs
- Sub-second freshness for the target (agent always sees latest)

#### 2. Hash-based memoization (`@coco.fn(memo=True)`)

Functions are memoized by hashing both their input AND their code:

- If you change `file.rs` → only that file's chunks re-embed
- If you change the *embedding function itself* → all outputs invalidate
- No stale data from code changes (a trap for naive caching)

#### 3. Per-row provenance / data lineage

Every target row (vector, graph node, chunk) traces back to its exact
source byte range. Enables:

- Debuggable RAG — "which source produced this retrieval?"
- Auditable pipelines — regulators can verify data flow
- Precise invalidation — when source changes, only provenance-linked
  targets need update

#### 4. AST-aware chunking for code

Instead of naive `RecursiveTextSplitter` (which cuts functions in half):

- Parse source with language-aware AST (tree-sitter equivalent)
- Chunk at semantic boundaries (function/class/module level)
- Preserve structural context in each chunk
- Better retrieval precision for code search

#### 5. Declarative pipeline specification

```python
@coco.fn(memo=True)
async def index_file(file, table):
    for chunk in RecursiveSplitter().split(await file.read_text()):
        table.declare_row(text=chunk.text, embedding=embed(chunk.text))
```

The user declares *what* the target should look like, not *how* to
keep it in sync. The engine handles invalidation, ordering, retries.

### What TerranSoul can adopt (post-31.5)

| CocoIndex pattern | TerranSoul adaptation | Priority |
|---|---|---|
| Content-hash per file | Store SHA-256 of each file in `code_repos` table. On re-index, skip files whose hash hasn't changed. | High — easy win, massive speedup on large repos |
| Skip unchanged embeddings | If a symbol's source text hasn't changed, keep its existing vector. Only re-embed modified functions. | Medium — requires symbol-level content hashing |
| Code-change invalidation | When `symbol_index.rs` logic changes (schema version bump), force full re-index. | Low — manual version field is sufficient for now |
| Per-row provenance | Track which source file+line produced each `code_symbols` row (already have `file`+`line`). Extend to embeddings when added. | Already partially done |
| AST-aware chunking | Already using tree-sitter for symbol extraction. When adding embeddings (Chunk 31.6), chunk at symbol boundaries not arbitrary byte offsets. | High — use tree-sitter node spans |

### What we do NOT adopt

- **PostgreSQL dependency** — CocoIndex requires Postgres for pipeline state.
  TerranSoul is single-binary + SQLite. We implement the *principle*
  (content-hash skip) in our own SQLite schema.
- **Python runtime** — CocoIndex's API is Python. We stay pure Rust.
- **External vector DB** — CocoIndex targets pgvector/Qdrant/etc. We keep
  HNSW ANN in-process via `usearch` crate.
- **Declarative DSL** — The `Target = F(Source)` model is elegant but
  over-engineered for a single indexing pipeline. We use imperative Rust
  with explicit hash-check guards.
- **Full lineage graph** — Production-grade lineage (for regulators/auditors)
  is out of scope for a desktop companion. We keep simple file→symbol provenance.

### Comparison: GitNexus vs CocoIndex vs TerranSoul approach

| Dimension | GitNexus | CocoIndex-code | TerranSoul (current + planned) |
|---|---|---|---|
| **Re-indexing** | Full re-index (implied) | Δ-only, hash-memoized | Full today → content-hash skip post-31.5 |
| **Language** | TypeScript (tree-sitter bindings) | Python + Rust core | Rust (tree-sitter native) |
| **Storage** | LadybugDB (embedded graph) | PostgreSQL + pluggable targets | SQLite (embedded, zero-config) |
| **MCP surface** | 16 tools + resources + prompts | Semantic search + call graph + blast radius | 13 tools today → native code tools in 31.6 |
| **Privacy** | All local, `.gitnexus/` | Configurable (local or cloud targets) | All local, `mcp-data/` |
| **Install** | `gitnexus setup` | `pip install cocoindex` | Built into the Tauri app binary |
| **Dependencies** | Node.js runtime | Python + Postgres | Zero (single binary) |
| **Embedding model** | transformers.js (in-process) | Any (sentence-transformers, OpenAI) | Ollama nomic-embed-text + cloud fallback |
| **Clustering** | Leiden community detection | Not documented | Planned (Chunk 31.5, petgraph) |
| **Best for** | Pure code intelligence MCP | General incremental ETL + code | Companion memory + code intelligence |

### Actionable takeaway

The biggest gap between TerranSoul's current `symbol_index.rs` and both
reference projects is **incremental re-indexing**. Currently we `DELETE FROM
code_symbols WHERE repo_id = ?` + re-insert everything. For a 500-file repo
this takes seconds; for a 10,000-file monorepo it's minutes.

**Minimal implementation (post-31.5):**

1. Add `content_hash TEXT` column to `code_symbols` (or a companion `code_file_hashes` table)
2. On `index_repo`, compute SHA-256 of each source file before parsing
3. Compare against stored hash — skip if unchanged
4. Only parse + extract + insert symbols/edges for changed files
5. Delete orphan symbols from files that no longer exist

This alone gives 80–90% of CocoIndex's performance benefit with zero new
dependencies. The rest (embedding memoization, code-change invalidation)
can layer on later.

---

## 12. Claudia

**Repo:** https://github.com/kbanc85/claudia
**License:** PolyForm Noncommercial 1.0.0 — **patterns and ideas only, no
code copy.** All adoption work must be a clean-room reimplementation.

### Why this section exists

The full reverse-engineering analysis already lives in
[`mcp-data/shared/claudia-research.md`](../mcp-data/shared/claudia-research.md)
(212 lines): architecture, scheduled jobs, slash-skill catalogue, hybrid-search
weight comparison, two-tier agent team layout, judgment-rules pattern, and
ten concrete adoption proposals mapped onto existing TerranSoul modules. That
document is the durable single source of truth — it is also seeded into the
brain via `mcp-data/shared/memory-seed.sql` so any agent session can retrieve
it through `brain_search` without re-reading the upstream repo.

### Promotable chunks

The ten adoption proposals are tracked as `Phase 33B` in
[`rules/backlog.md`](backlog.md#phase-33b--claudia-adoption-catalogue-reverse-engineered-from-kbanc85claudia).
Promote individual rows to `rules/milestones.md` when they're ready to schedule;
do not inline-duplicate the analysis here.

### License-boundary reminder

Claudia ships under PolyForm-NC 1.0.0, which forbids commercial use. TerranSoul
must therefore (a) never copy Claudia source files into the repo, (b) credit
the project in `CREDITS.md` (already done), and (c) implement adopted patterns
from scratch using TerranSoul's own modules. This is the same rule the brain
seed enforces via the `LICENSE BOUNDARY (claudia adoption)` memory entry.
