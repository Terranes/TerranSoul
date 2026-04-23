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

## Completed Work

All chunks listed here are fully implemented. See `rules/completion-log.md` for full details.

### Foundation (Phase 0)
- ✅ Chunk 001 — Project Scaffold — see `completion-log.md`
- ✅ CI Restructure — Consolidated 5 CI jobs → 3; push-only trigger; paths filter — see `completion-log.md`
- ✅ Chunk 002 — Chat UI Polish & Vitest Component Tests — see `completion-log.md`
- ✅ Chunk 003 — Three.js Scene Polish + WebGPU Detection — see `completion-log.md`
- ✅ Chunk 004 — VRM Model Loading & Fallback — see `completion-log.md`
- ✅ Chunk 005 — Character State Machine Tests — see `completion-log.md`
- ✅ Chunk 006 — Rust Chat Commands — Unit Tests — see `completion-log.md`
- ✅ Chunk 007 — Agent Orchestrator Hardening — see `completion-log.md`
- ✅ Chunk 008 — Tauri IPC Bridge Integration Tests — see `completion-log.md`
- ✅ Chunk 009 — Playwright E2E Test Infrastructure — see `completion-log.md`
- ✅ Chunk 010 — Character Reactions — Full Integration — see `completion-log.md`
- ✅ Chunk 011 — VRM Import + Character Selection UI — see `completion-log.md`

### Phase 1 — Chat-First, 3D Character, Text Only
✅ Phase 1 complete — see `completion-log.md`

### Phase 2 — TerranSoul Link (Cross-Device)
- ✅ Chunk 020 — Device Identity & Pairing — see `completion-log.md`
- ✅ Chunk 021 — Link Transport Layer — see `completion-log.md`
- ✅ Chunk 022 — CRDT Sync Engine — see `completion-log.md`
- ✅ Chunk 023 — Remote Command Routing — see `completion-log.md`

### Phase 3 — AI Package Manager & Agent Marketplace
- ✅ Chunk 030 — Package Manifest Format — see `completion-log.md`
- ✅ Chunk 031 — Install / Update / Remove Commands — see `completion-log.md`
- ✅ Chunk 032 — Agent Registry — see `completion-log.md`
- ✅ Chunk 033 — Agent Sandboxing — see `completion-log.md`
- ✅ Chunk 034 — Agent Marketplace UI — see `completion-log.md`
- ✅ Chunk 035 — Agent-to-Agent Messaging — see `completion-log.md`

### Phase 4 — Brain & Memory (Local LLM + Persistent Memory)
- ✅ Chunk 040 — Brain (Local LLM via Ollama) — see `completion-log.md`
- ✅ Chunk 041 — Long/Short-term Memory + Brain-powered Recall — see `completion-log.md`

### Phase 5 — Desktop Experience (Overlay & Streaming)
- ✅ Chunk 050 — Window Mode System — see `completion-log.md`
- ✅ Chunk 051 — Selective Click-Through — see `completion-log.md`
- ✅ Chunk 052 — Multi-Monitor Pet Mode — see `completion-log.md`
- ✅ Chunk 053 — Streaming LLM Responses — see `completion-log.md`
- ✅ Chunk 054 — Emotion Tags in LLM Responses — see `completion-log.md`

### Phase 5.5 — Three-Tier Brain (Free API / Paid API / Local LLM)
- ✅ Chunk 055 — Free LLM API Provider Registry & OpenAI-Compatible Client — see `completion-log.md`
- ✅ Chunk 056+057 — Streaming BrainMode Routing, Auto-Selection & Wizard Redesign — see `completion-log.md`
- ✅ Chunk 058 — Emotion Expansion & UI Fixes — see `completion-log.md`
- ✅ Chunk 059 — Provider Health Check & Rate-Limit Rotation — see `completion-log.md`

### Phase 6 — Voice (User-Defined ASR/TTS)
- ✅ Chunk 060 — Voice Abstraction Layer + Open-LLM-VTuber Integration — see `completion-log.md`
- ✅ Chunk 061 — Web Audio Lip Sync — see `completion-log.md`
- ✅ Chunk 062 — Voice Activity Detection — see `completion-log.md`
- ✅ Chunk 063 — Remove Open-LLM-VTuber + Rewrite Voice in Rust — see `completion-log.md`
- ✅ Chunk 064 — Desktop Pet Overlay with Floating Chat — see `completion-log.md`

### Phase 6.5 — UI Polish, UX Refinement & Art
- ✅ Chunk 065 — Design System & Global CSS Variables — see `completion-log.md`
- ✅ Chunk 066 — New Background Art — see `completion-log.md`
- ✅ Chunk 067 — Enhanced Chat UX — see `completion-log.md`
- ✅ Chunk 068 — Navigation Polish & Micro-interactions — see `completion-log.md`

### Phase 7 — VRM Model Security (Anti-Exploit & Asset Protection)
📦 Chunks 100–105 moved to `rules/backlog.md` — do not start until explicitly requested.

### Phase 8 — Brain-Driven Animation (AI4Animation for VRM)
- ✅ Chunk 080 — Pose Preset Library — see `completion-log.md`
- ✅ Chunk 081 — Pose Blending Engine — see `completion-log.md`
- ✅ Chunk 082 — LLM Pose Prompt Engineering — see `completion-log.md`
- ✅ Chunk 083 — Gesture Tag System — see `completion-log.md`
- ✅ Chunk 084 — Autoregressive Pose Feedback — see `completion-log.md`

### Phase 9.1 — UI/UX (Open-LLM-VTuber Patterns)
- ✅ Chunk 085 — UI/UX Overhaul — see `completion-log.md`

### Phase 9 — Learned Features (High Priority)
- ✅ Chunk 094 — Model Position Saving — see `completion-log.md`
- ✅ Chunk 095 — Procedural Gesture Blending (MANN-inspired) — see `completion-log.md`
- ✅ Chunk 096 — Speaker Diarization — see `completion-log.md`
- ✅ Chunk 097 — Hotword-Boosted ASR — see `completion-log.md`
- ✅ Chunk 098 — Presence / Greeting System — see `completion-log.md`
- ✅ Chunk 106 — Streaming TTS — see `completion-log.md`
- ✅ Chunk 107 — Multi-ASR Provider Abstraction — see `completion-log.md`
- ✅ Chunk 108 — Settings Persistence + Env Overrides — see `completion-log.md`
- ✅ Chunk 109 — Idle Action Sequences — see `completion-log.md`
- ✅ Chunk 110 — Background Music — see `completion-log.md`

### Phase 9 — Learned Features (Lower Priority)
- ✅ Chunk 115 — Live2D Support — see `completion-log.md`
- ✅ Chunk 116 — Screen Recording / Vision — see `completion-log.md`
- 📦 Chunk 117 — Docker Containerization — demoted to `rules/backlog.md` (Tauri desktop apps don't use Docker)
- ✅ Chunk 118 — Chat Log Export — see `completion-log.md`
- ✅ Chunk 119 — Language Translation Layer — see `completion-log.md`

### Phase 10 — Avatar Animation Architecture (VRM Expression-Driven)
- ✅ Chunk 120 — AvatarState Model + Animation State Machine — see `completion-log.md`
- ✅ Chunk 121 — Exponential Damping Render Loop — see `completion-log.md`
- ✅ Chunk 122 — 5-Channel VRM Viseme Lip Sync — see `completion-log.md`
- ✅ Chunk 123 — Audio Analysis Web Worker — see `completion-log.md`
- ✅ Chunk 124 — Decouple IPC from Animation — Coarse State Bridge — see `completion-log.md`
- ✅ Chunk 125 — LipSync ↔ TTS Audio Pipeline — see `completion-log.md`
- ✅ Chunk 126 — On-demand Rendering + Idle Optimization — see `completion-log.md`

### Phase 11 — RPG Brain Configuration
- ✅ Chunk 128 — Constellation Skill Tree (Full-Screen Layout) — see `completion-log.md`
- ✅ Chunk 129 — Constellation Cluster Interaction & Detail Panel — see `completion-log.md`
- ✅ Chunk 130 — Brain RPG Stat Sheet — see `completion-log.md`
- ✅ Chunk 131 — Combo Notification Toast — see `completion-log.md`
- ✅ Chunk 132 — Quest Reward Ceremony — see `completion-log.md`
- ✅ Chunk 133 — Brain Evolution Path (neural pathway) — see `completion-log.md`
- ✅ Chunk 134 — Stat-Based AI Scaling — see `completion-log.md`

### Post-Phase Polish
- ✅ 3D Model Loading Robustness (error overlay, URL encoding, placeholder fallback) — see `completion-log.md`
- ✅ Streaming Timeout Fix (stuck "Thinking" state, 60s/30s race) — see `completion-log.md`
- ✅ Music Bar Redesign (always-visible play/stop button) — see `completion-log.md`
- ✅ Splash Screen (animated kawaii cat loading screen) — see `completion-log.md`
- ✅ BGM Track Replacement (JRPG-style: Crystal Theme, Starlit Village, Eternity) — see `completion-log.md`

---

## Active Chunks

### Phase 12 — Brain Advanced Design (Documentation & QA)

| # | Chunk | Status | Owner | Notes |
|---|---|---|---|---|
| 1.1 | Brain Advanced Design — Validation, Docs Rewrite, QA Walkthrough | in-progress | agent + user (screenshots) | Source tracking pipeline complete; screenshots remain |
| 1.2 | Mac & Linux Support — Build, Distribution & Platform Polish | not-started | unassigned | Planning only — do not start without explicit kickoff |
| 1.3 | Per-User VRM Model Persistence + Remove GENSHIN Default | in-progress | agent | Imported VRMs copied into per-user OS folder; GENSHIN bundled model removed |
| 1.4 | Podman + Docker Desktop Dual Container Runtime | not-started | unassigned | Planning only — do not start without explicit kickoff |

#### Chunk 1.1 — Brain Advanced Design — Validation, Docs Rewrite, QA Walkthrough

**Goal.** Validate `docs/brain-advanced-design.md` against best-in-class
open-source references (Obsidian, SiYuan, RAGFlow), confirm the Phase-1
implementation in `src-tauri/src/memory/` + `src-tauri/src/commands/ingest.rs`
matches the design, and rewrite the user-facing docs around a single
end-to-end scenario (Vietnamese law portal `http://thuvienphapluat.vn/` +
internal-firm-rules PDF) so a fresh user can reproduce it step-by-step.

**Done in this PR (agent).**
- [x] Read `docs/brain-advanced-design.md` end-to-end and audited current code.
- [x] Wrote design-validation summary (Obsidian / SiYuan / RAGFlow comparison
      table) inside the rewritten walkthrough.
- [x] Confirmed baseline: `cargo test --all-targets` → 561/561 pass on
      `copilot/validate-advanced-design-and-implement`.
- [x] **Replaced** `instructions/BRAIN-COMPLEX-EXAMPLE.md` with a focused
      walkthrough of the thuvienphapluat.vn + PDF scenario (brain setup quest
      → URL crawl → PDF ingest → hybrid RAG → cross-source dedup → amendment
      conflict resolution → Obsidian export). Includes QA validation log,
      reproduction recipe, and code-path map.
- [x] **Replaced** `instructions/BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md` with a
      concise quick-reference (system map, code map, Tauri command table,
      schema cheat-sheet, hybrid-search formula, decay/GC formula, ingest
      pipeline, multi-source mechanics, sqlite3 debug recipes) that points
      to `docs/brain-advanced-design.md` as the canonical deep dive.
- [x] Embedded the existing real screenshots from
      `instructions/screenshots/01-fresh-launch.png` … `11-skill-tree.png`
      throughout the walkthrough.
- [x] **Source tracking pipeline** — Extended `NewMemory` + `MemoryEntry`
      with `source_url`, `source_hash`, `expires_at`; updated `add()`,
      `add_to_tier()`, all SELECTs, row mappers; added `find_by_source_hash()`,
      `find_by_source_url()`, `delete_by_source_url()`, `delete_expired()`;
      wired SHA-256 hashing + dedup/staleness into `run_ingest_task`;
      added `sha2` + `hex` crates; 9 new tests. **570 Rust / 941 Vitest pass.**

**Remaining (user environment / follow-up).**
- [ ] Capture **scenario-specific** screenshots on a real Tauri build with
      Vietnamese content loaded (statute crawl progress, PDF ingest task,
      Memory list with `vn-law` tag, sourced RAG answer, conflict toast,
      Obsidian export folder). Replace the generic placeholders embedded
      from `instructions/screenshots/` once real ones exist.
- [ ] Optional: short MP4 walkthrough via
      `playwright codegen --record-video` wrapping
      `scripts/brain-flow-screenshots.mjs`; convert with
      `ffmpeg -i in.webm out.mp4`; commit under `recording/`.

**Files touched.**
- `instructions/BRAIN-COMPLEX-EXAMPLE.md` (rewritten)
- `instructions/BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md` (rewritten)
- `src-tauri/Cargo.toml` (added sha2, hex)
- `src-tauri/src/memory/store.rs` (NewMemory, MemoryEntry, add, queries, 4 new methods, 9 tests)
- `src-tauri/src/memory/brain_memory.rs` (Default spreads)
- `src-tauri/src/commands/ingest.rs` (SHA-256 hashing, dedup, staleness, 2 tests)
- `src-tauri/src/commands/memory.rs` (Default spread)
- `rules/milestones.md` (this entry)
- `rules/completion-log.md` (logged source tracking pipeline)

**Validation.**
- `cd src-tauri && cargo test --all-targets` → 570/570 pass.
- `npx vitest run` → 941/941 pass.

---

#### Chunk 1.2 — Mac & Linux Support — Build, Distribution & Platform Polish

**Goal.** Bring macOS and Linux support to first-class parity with Windows for
TerranSoul. Today the codebase compiles on all three platforms via Tauri 2.x,
but many tested code paths, packaging artifacts, dependency installation
documentation, and CI coverage are Windows-biased. This chunk closes that gap.

**Status.** `not-started` — planning only. Do not begin implementation until
the user explicitly says so.

**Scope (proposed).**
- **Build & toolchain audit.** Verify `cargo build` and `npm run tauri build`
  succeed on macOS (Intel + Apple Silicon) and Linux (Ubuntu/Fedora/Arch).
  Document required system packages (e.g. `libwebkit2gtk-4.1-dev`,
  `libsoup-3.0-dev`, `libjavascriptcoregtk-4.1-dev` on Debian/Ubuntu;
  matching `webkit2gtk4.1` etc. on Fedora/Arch).
- **CI matrix.** Extend `.github/workflows/terransoul-ci.yml` (currently
  Linux-only for `cargo`/`npm` jobs) to also exercise macOS and Windows
  runners for build + tests on each push to `main`.
- **Bundle targets.** Configure `tauri.conf.json` `bundle.targets` per OS:
  `dmg`/`app` on macOS, `deb`/`appimage`/`rpm` on Linux. Validate icons
  (`.icns` on macOS, `.png` on Linux) render correctly.
- **Window mode platform parity.** `src-tauri/src/commands/window.rs` —
  audit `set_cursor_passthrough`, transparent-overlay, always-on-top, and
  pet-mode multi-monitor logic on macOS and Linux. Several transparency /
  click-through code paths only have Windows fallbacks today.
- **Tray icon parity.** Verify `TrayIconBuilder` behaviour on GNOME
  (StatusNotifier), KDE, and macOS menubar.
- **File dialogs / paths.** Replace any Windows-only path assumptions with
  `tauri-plugin-dialog` and `app_data_dir()` (cross-platform).
- **Voice / TTS / ASR.** Confirm `msedge-tts` works on macOS/Linux (it should,
  as it speaks to Microsoft Edge cloud TTS, but smoke-test). Web Speech API
  ASR availability per browser engine.
- **Local LLM bootstrap.** macOS and Linux users may prefer Ollama directly
  over Docker — coordinate with Chunk 1.4 (Podman/Docker dual support).
- **Docs.** New `instructions/PLATFORM-MAC.md` and `instructions/PLATFORM-LINUX.md`
  with prerequisites, install commands, known issues, and screenshots from
  each OS.

**Acceptance criteria.**
- CI green for `ubuntu-latest`, `macos-latest`, `windows-latest`.
- Bundles produced for `.dmg`, `.deb`, `.appimage`, `.msi` in CI release
  workflow.
- Manual smoke test on at least one macOS + one Linux machine: launch,
  chat, switch model, pet mode, BGM toggle.
- All cross-platform path APIs (`app_data_dir`, file pickers) used
  uniformly — no `if cfg!(target_os = "windows")` branches without a
  matching `else` branch for mac/linux.

---

#### Chunk 1.3 — Per-User VRM Model Persistence + Remove GENSHIN Default

**Goal.** (1) Remove the bundled GENSHIN VRM (and its companion thumbnail
file `2250278607152806301.vrm` + `GENSHIN.png`) from the default model set
so the repository ships only the two truly-original characters. (2) Ensure
every VRM model the user imports is copied into a per-user OS-specific
data folder (Tauri's `app_data_dir`) so models survive a fresh install,
re-build, or app upgrade — instead of being remembered only by an absolute
path that breaks the moment the source file moves.

**Status.** `in-progress` — implementation in this PR.

**Scope.**
- Drop `genshin` entry from `src/config/default-models.ts`; delete the
  underlying `public/models/default/2250278607152806301.vrm` and
  `public/models/default/GENSHIN.png` assets; update tests.
- Extend `AppSettings` (Rust) with `user_models: Vec<UserModel>`; bump
  `CURRENT_SCHEMA_VERSION` to 3 with `serde(default)` so existing on-disk
  v2 settings hydrate cleanly.
- New backend commands:
  - `import_user_model(source_path) -> UserModel` — reads bytes from the
    source path, generates a UUID id, writes to
    `<app_data_dir>/user_models/<id>.vrm`, appends to settings, persists.
  - `list_user_models() -> Vec<UserModel>`
  - `delete_user_model(id) -> ()` — removes file + settings entry.
  - `read_user_model_bytes(id) -> Vec<u8>` — for the frontend to wrap in
    a `Blob` URL (no asset-protocol scope to widen, works identically on
    every OS).
- Frontend `useCharacterStore`:
  - Loads user models on init via `list_user_models`.
  - Exposes a unified `models` view (defaults + user) so `selectModel(id)`
    works for either kind.
  - For user models, fetches bytes via `read_user_model_bytes`, wraps in
    `URL.createObjectURL(new Blob([...]))`, sets `vrmPath` to that URL.
  - Persists `selected_model_id` for user models the same way as defaults.
- `ModelPanel.vue`:
  - Calls the import command (with `window.prompt` fallback for browser dev).
  - Renders the user-model list with a delete (×) button per row.

**Acceptance criteria.**
- GENSHIN no longer appears in the model list, anywhere in code or assets.
- After uninstalling and re-installing the app to the same user account
  (or a fresh `npm run build` cycle), previously imported models still
  appear in the picker and load successfully.
- All existing settings on disk (schema v2) still load (forward-compat).
- `cargo test --all-targets` and `npm run test` remain green.

---

#### Chunk 1.4 — Podman + Docker Desktop Dual Container Runtime

**Goal.** Allow TerranSoul's local-LLM setup quest ("Inner Sanctum") to
work on machines that — for company compliance reasons — cannot install
Docker Desktop but do have **Podman** installed. The runtime detection
must be transparent for users who already have Docker Desktop, and only
prompt for a choice when both (or neither) are present, or when no
container runtime is found at all.

**Status.** `not-started` — planning only. Do not begin implementation
until the user explicitly says so.

**Scope (proposed).**
- **Detection.** Extend `src-tauri/src/commands/docker.rs` (or refactor
  into `src-tauri/src/container/`) with a new abstraction
  `ContainerRuntime { Docker, Podman, None }` and a
  `detect_container_runtime()` Tauri command. Detection order:
  1. `docker --version` succeeds **and** `docker info` reports a running
     daemon → `Docker`.
  2. Else `podman --version` succeeds → `Podman`.
  3. Else → `None`.
- **User override.** New setting `preferred_container_runtime` in
  `AppSettings` (default `Auto`). When set to `Docker` or `Podman`
  explicitly, detection skips probing and uses the chosen one (failing
  with a clear error if missing). UI added to the local-LLM quest.
- **Quest UX — "Inner Sanctum".**
  - If a runtime is detected → continue silently as today.
  - If both are available → show a one-time picker ("We found Docker
    Desktop and Podman. Which do you want TerranSoul to use?") and
    persist the choice.
  - If neither is available → show install instructions for both
    (with platform-specific links: Docker Desktop, Podman Desktop,
    `brew install podman`, `apt install podman`, `dnf install podman`).
- **Command translation.** Almost every Docker CLI invocation we use
  (`docker pull`, `docker run -d`, `docker ps`, `docker exec`,
  `docker stop`) has a Podman-compatible equivalent. Wrap the CLI
  binary name in a `runtime_binary()` helper. For commands that differ
  (e.g. `docker desktop start` has no Podman analogue — Podman uses
  `podman machine start` on macOS/Windows), branch in a single place.
- **Compose files.** If we ever add `docker-compose.yml`, also generate
  a Podman-compatible variant or use `podman-compose` /
  `podman play kube`.
- **Tests.** Backend unit tests with the binary path injected (no real
  Docker/Podman needed): assert the right CLI string is produced for
  each runtime + each operation.
- **Docs.** Update `instructions/DOCKER-CLEANUP.md` (or rename to
  `CONTAINER-RUNTIME.md`) explaining both runtimes and the picker.

**Acceptance criteria.**
- A clean Linux/macOS box with only Podman installed completes the
  Ollama setup quest without ever invoking `docker`.
- A box with Docker Desktop installed continues to behave exactly as
  today (no regression).
- A box with both installed shows the one-time picker; the choice
  persists across sessions.
- A box with neither shows install instructions for both options.
- No hard-coded `"docker"` strings remain outside the
  `runtime_binary()` helper / detection module.

---

> **Milestones backlog drained for completed phases.** All planned phase
> chunks are complete except the in-progress / planning entries above.
> Chunks 1.2 and 1.4 are explicitly marked `not-started` and must not be
> implemented without explicit user kickoff.
> See `rules/backlog.md` for deferred / speculative future work.
> To add new chunks, describe the feature and a new numbered entry will be created here.
