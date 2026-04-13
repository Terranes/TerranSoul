# Reverse Engineering Research — External Projects

> **Purpose:** Accelerate TerranSoul development by learning proven patterns from
> three reference projects. This document records architecture, overlay systems,
> voice approaches, and actionable takeaways chunked for incremental implementation.

---

## Table of Contents

1. [aituber-kit](#1-aituber-kit)
2. [Open-LLM-VTuber](#2-open-llm-vtuber)
3. [VibeVoice](#3-vibevoice)
4. [Overlay Comparison — What We're Doing Wrong](#4-overlay-comparison)
5. [Voice System Recommendation](#5-voice-system-recommendation)
6. [Actionable Chunks for TerranSoul](#6-actionable-chunks)

---

## 1. aituber-kit

**Repo:** https://github.com/tegnike/aituber-kit/

### Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | Next.js 15.5, React 18, TypeScript |
| 3D Rendering | Three.js 0.167 + @pixiv/three-vrm 3.4 |
| 2D Rendering | Pixi.js 7.4 (Live2D support) |
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
**Character:** Live2D (via pixi-live2d-display in the WebSDK)
**Communication:** WebSocket between Python server and Electron frontend

```
┌──────────────────────────────────────────────┐
│  Electron Desktop App (Open-LLM-VTuber-Web)  │
│  ┌────────────────────────────────────────┐   │
│  │  React UI + Live2D Canvas              │   │
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
| `live2d_model.py` | Live2D model management |
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
