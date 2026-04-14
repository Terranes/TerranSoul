# TerranSoul — Backlog

> **Never-started work lives here.** Only move chunks from this file to
> `milestones.md` when the user explicitly says so. This file is the holding
> area for planned but unscheduled work.

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

## Phase 9 — Learned Features (From Reference Projects)

> **Source repos:** Open-LLM-VTuber, AI4Animation-js, VibeVoice, aituber-kit
> **Analysis:** See `rules/research-reverse-engineering.md` §9.
> **Principle:** Integrate proven patterns; don't reinvent.

### High Priority

| Chunk | Description | Status |
|-------|-------------|--------|
| 090 | **Streaming TTS (VibeVoice Realtime or streaming Edge TTS)** — Replace batched TTS with streaming output. Voice starts ~200ms after LLM generates first token. Massive UX win for natural dialogue. Requires WebSocket or event stream plumbing between Rust TTS engine and frontend audio playback. | `not-started` |
| 091 | **Multi-ASR Provider Abstraction** — Abstract ASR into a plugin-style factory (like Open-LLM-VTuber's agent pattern). Currently only Whisper API. Add runtime provider swap: Whisper → Groq → Azure → browser Web Speech API. Config-driven selection in VoiceSetupView. | `not-started` |
| 092 | **Settings Persistence + Env Overrides** — Persist camera orbit position, zoom, model selection, TTS/ASR provider between sessions (aituber-kit pattern). Use Tauri `tauri-plugin-store`. Support `.env` override for dev/CI. Pre-validate schema before loading to prevent corruption. | `not-started` |
| 093 | **Idle Action Sequences** — When character is idle too long: time-based greetings, auto-speak via LLM, face detection triggers (aituber-kit pattern). Scheduled action queue with interruption handling. Makes character feel alive when user is away. | `not-started` |

### Medium Priority

| Chunk | Description | Status |
|-------|-------------|--------|
| 094 | **Model Position Saving** — Persist camera orbit position, zoom, rotation per model. Resume user's preferred viewing angle on app restart. Store in Tauri settings alongside model selection. | `not-started` |
| 095 | **Procedural Gesture Blending (MANN-inspired)** — Learn from AI4Animation MANN approach: instead of hardcoded JSON keyframes, use lightweight ML or procedural blending to generate smooth transitions between emotion states. Train on existing gesture data. Replace stiff cross-fades with natural motion. | `not-started` |
| 096 | **Speaker Diarization** — Detect multiple speakers in room (VibeVoice-ASR-7B pattern). Tag "who said what" in conversation log. Useful for group scenarios or streaming. | `not-started` |
| 097 | **Hotword-Boosted ASR** — Let users define domain-specific keywords (character names, game terms) that ASR should recognize better. VibeVoice supports hotword injection. | `not-started` |
| 098 | **Presence / Greeting System** — Auto-greeting when user appears (timer-based or face detection), auto-goodbye when away. Track "away duration" for different responses (aituber-kit pattern). | `not-started` |

### Lower Priority

| Chunk | Description | Status |
|-------|-------------|--------|
| 099 | **Live2D Support** — Add Live2D rendering alongside VRM using `@cubism/cubism4-runtime-js` (aituber-kit pattern). Useful for users who prefer 2D or have only 2D models. Renderer abstraction layer. | `not-started` |
| 100 | **Screen Recording / Vision** — Extend beyond static context: real-time screen activity analysis (Open-LLM-VTuber pattern). Use Tauri window capture API. Character can comment on what user is doing. | `not-started` |
| 101 | **Docker Containerization** — Run TerranSoul in isolated containers for CI/testing and server deployment (Open-LLM-VTuber pattern). CPU/GPU variants. | `not-started` |
| 102 | **Chat Log Export** — JSON export with timestamps, sentiment tags, emotion metadata. Build on existing conversation persistence. | `not-started` |
| 103 | **Language Translation Layer** — Accept input in one language, TTS output in another. Use LLM for translation. Store original + translated text. | `not-started` |
