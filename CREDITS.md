# Credits & Thanks

TerranSoul exists because many authors, researchers, maintainers, designers,
and creators chose to share their work in public. Some of the projects below
are direct dependencies; others are papers, videos, reference apps, public API
docs, or community patterns that helped us think more clearly about companion
UX, memory, voice, animation, and self-improving coding workflows.

We are grateful for the craft, experiments, documentation, and hard work behind
those sources.

This page is meant to say thank you. We credit sources even when we only used
them for insight, comparison, roadmap decisions, or reverse-engineering notes,
because that learning is still part of the work. Runtime code and UI keep
neutral TerranSoul names; the people and projects that helped us belong here.

For full commercial-license review see
[`docs/licensing-audit.md`](docs/licensing-audit.md).

---

## Community reference projects and creators

These sources shaped design and product decisions. We did not vendor their
source code unless a dependency table below says otherwise.

| Source | License / Terms | Used For | Thanks / Notes |
|---|---|---|---|
| [tegnike/aituber-kit](https://github.com/tegnike/aituber-kit/) | See upstream | `rules/research-reverse-engineering.md`, voice/provider roadmap, emotion and motion tag patterns | Thank you for a practical AI companion reference with provider breadth, VRM animation flow, idle behavior, and settings patterns that informed TerranSoul's own neutral capability design. |
| [Open-LLM-VTuber](https://github.com/Open-LLM-VTuber/Open-LLM-VTuber) and Open-LLM-VTuber-Web | See upstream | Pet mode, two-mode window behavior, VAD/ASR/TTS separation, provider abstractions, overlay interactivity | Thank you to the Open-LLM-VTuber authors and community for making a rich local companion architecture visible enough to learn from without copying identity, branding, or assets. |
| [Microsoft VibeVoice](https://github.com/microsoft/VibeVoice) | MIT | Voice roadmap, streaming TTS/ASR research, hotword and diarization evaluation | Thank you for showing a local-first path for high-quality speech systems and for documenting model/runtime tradeoffs clearly. |
| [sneha-belkhale/AI4Animation-js](https://github.com/sneha-belkhale/AI4Animation-js) | See upstream | Animation research, MANN-style expert blending, direction-based bone IK | Thank you for a readable Three.js reference that made neural animation ideas easier to translate into TerranSoul's VRM-safe motion roadmap. |
| Sebastian Starke et al. — Mode-Adaptive Neural Networks for Quadruped Motion Control | ACM research paper | Animation architecture research | The original MANN paper informed our understanding of gating networks, expert blending, and autoregressive motion state. |
| [mnfst/awesome-free-llm-apis](https://github.com/mnfst/awesome-free-llm-apis) | See upstream | Free cloud brain-provider research | Thank you for collecting free-tier LLM API options that helped shape TerranSoul's Free API mode and provider-rotation thinking. |
| [Just Rayen](https://www.youtube.com/@JustRayen) (`@JustRayen`) | YouTube / creator-owned content; follow YouTube terms and creator rights | 2026-05-04 analysis of what Just Rayen does in his companion project/channel | Thank you for the public project examples that made those companion behaviors visible. This credit is for analyzing what Just Rayen does in his project and using that analysis to choose neutral, configurable capability patterns for TerranSoul. Users can then teach, measure, and promote proven versions through TerranSoul's own source-promotion workflow like @JustRayen did for his AI companion. No video, transcript, branding, or protected assets are copied. |
| Cursor public workflow concepts | Public product/docs behavior | `docs/coding-workflow-design.md`, coding rules, memories, checkpoints, context curation | Thank you for visible workflow ideas around project rules, agent memory, reviewable checkpoints, and codebase context. |
| [Anthropic Claude Code documentation](https://code.claude.com/docs/) | Vendor documentation | Self-improve sessions, slash commands, subagents, path-scoped rules, hooks, worktree isolation | Thank you for clear public docs that helped us design a safer local coding loop. |
| [ultraworkers/claw-code](https://github.com/ultraworkers/claw-code) | See upstream | `SelfImproveSessionsPanel.vue`, `coding::session_chat`, resume/fork/rename/delete session UX | Thank you for the open session-management patterns that helped turn TerranSoul's coding workflow into a resumable experience. |
| OpenClaw public project | Public project materials; verify upstream terms before any code reuse | Self-improve UX research alongside claw-code and Claude Code | Thank you for the public exploration work around Claude Code-style agent UX that informed our session and slash-command design. |

## Frontend dependencies (npm)

| Project | License | Used For | Notes |
|---|---|---|---|
| [Vue.js](https://vuejs.org/) | MIT | Entire frontend | Reactivity, SFCs, `<script setup>` |
| [Pinia](https://pinia.vuejs.org/) | MIT | All `src/stores/*.ts` | State management |
| [Three.js](https://threejs.org/) | MIT | `src/renderer/*` | 3D rendering |
| [@pixiv/three-vrm](https://github.com/pixiv/three-vrm), [@pixiv/three-vrm-animation](https://github.com/pixiv/three-vrm) | MIT | VRM avatar pipeline | VRM 1.0 + VRMA loading and rendering |
| [Tauri](https://tauri.app/) (`@tauri-apps/api`, `@tauri-apps/cli`, `@tauri-apps/plugin-shell`) | MIT / Apache-2.0 | Desktop shell + IPC | Window, command, event, shell open |
| [Vite](https://vitejs.dev/) + `@vitejs/plugin-vue` | MIT | Bundler / dev server | |
| [PrimeVue](https://primevue.org/) | MIT | UI components | Vue 3 component library |
| [Tailwind CSS v4](https://tailwindcss.com/) | MIT | Utility CSS | |
| [Cytoscape.js](https://js.cytoscape.org/) | MIT | Brain knowledge-graph viz | |
| [PDFKit](https://pdfkit.org/) | MIT | Persona / memory PDF export | |
| [better-sqlite3](https://github.com/WiseLibs/better-sqlite3) | MIT | Embedded SQLite (frontend tooling) | |
| [@ricky0123/vad-web](https://github.com/ricky0123/vad) | MIT | Voice-activity detection | ONNX VAD running in WebAssembly |
| [Vitest](https://vitest.dev/), [@vue/test-utils](https://test-utils.vuejs.org/), [Playwright](https://playwright.dev/), [jsdom](https://github.com/jsdom/jsdom) | MIT | Tests | |
| ESLint, TypeScript-ESLint, TypeScript, `globals` | MIT / Apache-2.0 | Linting / typecheck | |

## Backend dependencies (Rust)

| Project | License | Used For | Notes |
|---|---|---|---|
| [Tauri](https://tauri.app/) (`tauri*`, `tauri-plugin-shell`) | MIT / Apache-2.0 | Desktop shell + IPC | |
| [Tokio](https://tokio.rs/) (`tokio`, `tokio-tungstenite`, `tokio-util`, `futures-util`, `async-trait`) | MIT | Async runtime | |
| [Axum](https://github.com/tokio-rs/axum) | MIT | Local HTTP server (MCP, registry) | |
| [reqwest](https://github.com/seanmonstar/reqwest) | MIT / Apache-2.0 | HTTP client | LLM / GitHub / OpenAI calls |
| [rustls](https://github.com/rustls/rustls), `rustls-pemfile`, [Quinn](https://github.com/quinn-rs/quinn), [rcgen](https://github.com/rustls/rcgen) | MIT / Apache-2.0 / ISC | TLS, QUIC, certs | Mobile pairing + sync |
| [Serde](https://serde.rs/) (`serde`, `serde_json`, `serde_yaml`) | MIT / Apache-2.0 | (De)serialization | |
| [thiserror](https://github.com/dtolnay/thiserror), [anyhow](https://github.com/dtolnay/anyhow) | MIT / Apache-2.0 | Error handling | |
| [rusqlite](https://github.com/rusqlite/rusqlite), [sqlx](https://github.com/launchbadge/sqlx), [tiberius](https://github.com/prisma/tiberius), [scylla](https://github.com/scylladb/scylla-rust-driver), [postgres](https://github.com/sfackler/rust-postgres) | MIT / Apache-2.0 | `StorageBackend` implementations | SQLite default + Postgres / MSSQL / Cassandra |
| [usearch](https://github.com/unum-cloud/usearch) | Apache-2.0 | HNSW ANN index | Vector similarity in `memory/ann_index.rs` |
| [text-splitter](https://github.com/benbrandt/text-splitter) | MIT | Semantic chunking | `memory/chunking.rs` |
| [Wasmtime](https://wasmtime.dev/) | Apache-2.0 (with WASM exception) | WASM agent runtime | Sandboxed plugins |
| [ed25519-dalek](https://github.com/dalek-cryptography/ed25519-dalek), [ring](https://github.com/briansmith/ring), [sha2](https://github.com/RustCrypto/hashes), `base64`, `hex`, `rand`, `rand_core` | BSD-3 / MIT / Apache-2.0 | Crypto primitives + manifest signing | Device identity, plugin signing |
| `scraper`, `url`, `uuid`, `qrcode`, `sysinfo`, `tempfile`, `chrono` | MIT / Apache-2.0 / ISC | Utilities | |
| [tracing](https://github.com/tokio-rs/tracing), [clap](https://github.com/clap-rs/clap) | MIT / Apache-2.0 | Logging, CLI parsing | |

## Models, formats, and protocols

| Reference | License / Terms | Used For | Notes |
|---|---|---|---|
| [VRM 1.0](https://vrm.dev/en/vrm/vrm_about/) | Open spec (Vroid / VRM Consortium) | Avatar format | We support VRM exclusively; Live2D is rejected (`docs/persona-design.md`, `docs/licensing-audit.md`) |
| [VRMA](https://github.com/vrm-c/vrm-specification) | Open spec | Bundled body animations | |
| [Ollama](https://ollama.com/) HTTP API | MIT (client compatibility) | Local LLM provider | We are an HTTP client; no Ollama code is vendored |
| [OpenAI Chat Completions / Embeddings API](https://platform.openai.com/docs/) | Vendor API | Paid cloud brain mode | We follow the public REST contract |
| [Anthropic Messages API](https://docs.anthropic.com/) | Vendor API | Paid cloud brain mode | We follow the public REST contract |
| [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) | Open spec (Anthropic) | `src-tauri/src/ai_integrations/mcp/*` | TerranSoul exposes its brain over MCP at `127.0.0.1:7421` |
| [GitHub Device Flow OAuth](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow) | GitHub API | Self-improve authorization | `commands/github_auth.rs` + `SelfImprovePanel.vue` |
| [`nomic-embed-text`](https://huggingface.co/nomic-ai/nomic-embed-text-v1) | Apache-2.0 | Local embeddings (default) | 768-dim, served via Ollama |

## Research patterns referenced

These entries are *technique* attributions: we did not vendor source code,
but the public papers / posts informed how we built specific subsystems.

| Reference | License / Source | Used For | Notes |
|---|---|---|---|
| [Reciprocal Rank Fusion (Cormack, Clarke, Buettcher 2009)](https://plg.uwaterloo.ca/~gvcormac/cormacksigir09-rrf.pdf) | Academic paper | `memory/fusion.rs` | RRF (k=60) blending vector + keyword + freshness retrievers |
| [HyDE — Hypothetical Document Embeddings (Gao et al. 2022)](https://arxiv.org/abs/2212.10496) | Academic paper | `memory/hyde.rs` | LLM writes a hypothetical answer that is embedded for retrieval |
| [Anthropic — Contextual Retrieval (2024)](https://www.anthropic.com/news/contextual-retrieval) | Public engineering post | `memory/contextualize.rs` | Per-chunk LLM-generated context prepended before embedding |
| [HNSW (Malkov & Yashunin 2016)](https://arxiv.org/abs/1603.09320) | Academic paper | `memory/ann_index.rs` (via `usearch`) | Approximate nearest neighbor index for 1M+ entries |
| [Reverse-engineering audit of `msedge-tts`](docs/licensing-audit.md) | Internal review | TTS provider catalogue | Removed; replaced with browser SpeechSynthesis + user-supplied OpenAI TTS |

## Removed / rejected references

Items that were considered or briefly used and are intentionally **not**
shipped. See `docs/licensing-audit.md` for full rationale.

| Reference | Reason |
|---|---|
| Live2D | Avatar-format licensing/runtime fit; VRM-only avatar pipeline |
| `msedge-tts` (Microsoft Edge "Read Aloud" endpoint) | Endpoint is not a public API; commercial use directed to paid Azure Cognitive Services |
| `@vercel/analytics`, `@vercel/speed-insights` | Local-first privacy posture; free tier is non-commercial |
| MotionGPT / T2M-GPT (bundled) | Code is MIT but depends on SMPL/SMPL-X/PyTorch3D and datasets with separate licenses |
| VideoPose3D | Upstream license is CC BY-NC |
| NVIDIA Audio2Emotion | License gated; restricted to the Audio2Face project, not standalone use |
| Stable Video Diffusion | Stability AI community license has commercial thresholds |
| HunyuanVideo / Hunyuan-Motion (bundled) | Tencent community/model license terms; GPU-heavy and video-output-oriented |

