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
| [abhigyanpatwari/GitNexus](https://github.com/abhigyanpatwari/GitNexus) and [DeepWiki pages for GitNexus](https://deepwiki.com/abhigyanpatwari/GitNexus) | PolyForm Noncommercial 1.0.0 for GitNexus; DeepWiki public generated documentation, verified against upstream public docs | `docs/gitnexus-capability-matrix.md`, native code-intelligence roadmap, MCP tool surface design, graph-workbench UI/UX planning | Thank you for making the public design of precomputed relational code intelligence, process-grouped queries, impact/context/rename tool shapes, multi-repo MCP resources, generated skills, graph explorer, code-reference panel, chat citations, visible tool-call cards, and blast-radius highlights visible enough to learn from. No source, prompts, assets, package, binary, Docker image, or UI identity copied or bundled. TerranSoul uses this only as clean-room product/architecture research for neutral native Rust/Vue implementations. |
| [cocoindex-io/cocoindex](https://github.com/cocoindex-io/cocoindex) | Apache 2.0 | `docs/gitnexus-capability-matrix.md`, incremental Δ-only re-indexing design, hash-based memoization, AST-aware code chunking patterns | Thank you for demonstrating that incremental indexing (only re-parse/re-embed changed files, hash(input)+hash(code) memoization, per-row lineage) is the right architecture for keeping code indexes fresh at low cost. These principles will inform TerranSoul's post-31.5 content-hash optimization. No source copied — studied public README + docs + examples only. |
| [ultraworkers/claw-code](https://github.com/ultraworkers/claw-code) | See upstream | `SelfImproveSessionsPanel.vue`, `coding::session_chat`, resume/fork/rename/delete session UX | Thank you for the open session-management patterns that helped turn TerranSoul's coding workflow into a resumable experience. |
| [sonthanh/brain-os-plugin](https://github.com/sonthanh/brain-os-plugin) | MIT | 2026-05-05 reverse-engineering analysis recorded in `mcp-data/shared/migrations/002_brain_os_plugin_research.sql`; informed our deep-analysis-before-action rule and the upcoming coding-workflow gate-telemetry chunk (skill outcome logs §11, skill trace JSONL §12, description hygiene §13). | Thank you for publishing a clear skill-spec, outcome-log convention, and per-tool-call trace schema. We did not borrow vault structure, GitHub-issues-as-tasks, or Claude-Code hooks (TerranSoul has equivalent or stronger native infrastructure). The outcome-log and trace patterns will inform our own workflow telemetry design without copying source. |
| [safishamsi/graphify](https://github.com/safishamsi/graphify) | MIT | 2026-05-05 reverse-engineering analysis recorded in `mcp-data/shared/migrations/003_graphify_research.sql`; informed knowledge-graph improvement ideas around community detection, surprise scoring, confidence-tagged edges, token-budgeted graph traversal, and graph diffs. | Thank you for publishing a clear multi-modal knowledge-graph pipeline for coding agents. TerranSoul studied DeepWiki + the public README only, copied no source, and recorded possible future improvements as neutral design ideas for TerranSoul's own SQLite + `memory_edges` brain. |
| OpenClaw public project | Public project materials; verify upstream terms before any code reuse | Self-improve UX research alongside claw-code and Claude Code | Thank you for the public exploration work around Claude Code-style agent UX that informed our session and slash-command design. |
| [Obsidian](https://obsidian.md/) | Proprietary freeware (EULA) by Erica Xu & Shida Li / Dynalist Inc. | Vault export/sync architecture, YAML frontmatter schema, wikilink relationship model, local-first philosophy | Thank you for demonstrating that personal knowledge graphs work best as local-first, portable Markdown files with YAML frontmatter and graph-view wikilinks. Obsidian's vault structure directly inspired TerranSoul's memory export (`src-tauri/src/memory/obsidian_export.rs`), bidirectional sync (`obsidian_sync.rs`), the `obsidian_path`/`last_exported` schema columns, and the `[[wikilink]]` edge generation from memory relationships. See `docs/brain-advanced-design.md` § Layer 2 for full architecture. |
| Jonathan Edwards — ["Stop Calling It Memory — The Problem"](https://limitededitionjonathan.substack.com/p/stop-calling-it-memory-the-problem) | Public Substack article; original text not copied | `src-tauri/src/memory/context_pack.rs`, `src/stores/conversation.ts`, `docs/brain-advanced-design.md`, README RAG wording, `mcp-data/shared/memory-philosophy.md` (durable lesson + 7 non-negotiable rules for future agent sessions) | Thank you for the critique that agent "memory" should be treated as queryable external context/database retrieval rather than prompt-stuffed files or an exhaustive human-memory metaphor. TerranSoul adapted that idea by adding an explicit retrieved-context contract around RAG injections while keeping neutral project terminology, and persisted the architectural lesson in `mcp-data/shared/memory-philosophy.md` so the SQLite + vector search + KG-edges direction is preserved across every fresh clone and self-improve session. |
| [kbanc85/claudia](https://github.com/kbanc85/claudia) by Kamil Banc | PolyForm Noncommercial 1.0.0 — patterns/ideas only, no source/prompt/asset copy | `mcp-data/shared/claudia-research.md` (architecture inventory + 10 adoption proposals mapped onto existing TerranSoul modules — judgment-rules persistence, session reflection, morning-brief quest, memory-audit provenance view, 3-D brain viewport, agent-roster capability tags, per-workspace data root, optional stdio MCP transport, PARA opt-in for `obsidian_export.rs`) | Thank you for a thoughtfully argued reference companion that demonstrates the "SQLite is the source of truth; the markdown vault is a read-only projection" pattern in practice, plus a clean catalogue of proactive skills (`/morning-brief`, `/meditate`, `/memory-audit`, `/what-am-i-missing`, `/brain`) and scheduled background jobs (decay, consolidation, vault sync, pattern detection). Studied the public README + ARCHITECTURE.md only; no source code, prompts, scheduler scripts, asset names, or branded identity copied. PolyForm-NC license forbids redistribution, so TerranSoul implements equivalent capabilities natively in Rust under neutral names. |
| [Lum1104/Understand-Anything](https://github.com/Lum1104/Understand-Anything) by Yuxiang Lin | MIT — patterns/ideas only, no source/prompt/asset copy | `rules/backlog.md` Phase 36B, multi-agent/code-graph roadmap, coding workflow graph UX | Thank you for demonstrating a multi-agent code-understanding pipeline that turns files, functions, classes, dependencies, domain flows, guided tours, fuzzy/semantic search, and diff impact into an explorable knowledge graph. TerranSoul only studied the public README, plugin layout, agent catalogue, and license; adoption ideas are mapped onto TerranSoul's existing symbol index, MCP, and workflow surfaces under neutral names. |
| [swarmclawai/swarmclaw](https://github.com/swarmclawai/swarmclaw) | See upstream | [docs/multi-agent-orchestration-analysis-2026.md](docs/multi-agent-orchestration-analysis-2026.md), `mcp-data/shared/migrations/007_multi_agent_orchestration_research.sql`, coding workflow orchestration roadmap | Thank you for making a self-hosted multi-agent workflow dashboard visible enough to study at the system level. TerranSoul studied public architecture/tool/task/memory/UI patterns and adopted only neutral design lessons: durable lineage, session tool bundles, bounded swarm joins, task recovery, and dense operations UI. No source, prompts, branding, or UI identity copied. |
| [DeepWiki pages for swarmclawai/swarmclaw](https://deepwiki.com/swarmclawai/swarmclaw) | DeepWiki public generated documentation; verify against upstream | [docs/multi-agent-orchestration-analysis-2026.md](docs/multi-agent-orchestration-analysis-2026.md), MCP seed migration 007 | Thank you for the navigable system summary used as the first-pass map before cross-checking upstream snippets and TerranSoul's own source. |
| Multi-agent framework documentation survey: [LangGraph](https://docs.langchain.com/langgraph-platform/), [CrewAI](https://docs.crewai.com/), [AutoGen](https://microsoft.github.io/autogen/stable/), [Semantic Kernel](https://learn.microsoft.com/en-us/semantic-kernel/), [OpenAI Agents SDK](https://openai.github.io/openai-agents-python/), [Google ADK](https://adk.dev/), [LlamaIndex Workflows](https://docs.llamaindex.ai/en/stable/module_guides/workflow/), [Pydantic AI](https://ai.pydantic.dev/), [Haystack Agents](https://docs.haystack.deepset.ai/docs/agents), [Agno](https://docs.agno.com/), and [Mastra](https://mastra.ai/docs) | Public/vendor documentation; follow each upstream license/terms | [docs/multi-agent-orchestration-analysis-2026.md](docs/multi-agent-orchestration-analysis-2026.md), `docs/coding-workflow-design.md`, MCP seed migration 007 | Thank you for documenting durable graph execution, role-based crews, event-driven runtimes, plugin/function middleware, typed agent APIs, MCP/A2A integration, tracing/evals, human approval, suspend/resume, and workflow-studio UX. TerranSoul used these as comparison points for a local-first Rust/Tauri adoption plan only. |

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
| [usearch](https://github.com/unum-cloud/usearch) | Apache-2.0 | Optional HNSW ANN index (`native-ann`) | Vector similarity acceleration in `memory/ann_index.rs` for large local stores; default builds use a pure-Rust linear fallback. |
| [text-splitter](https://github.com/benbrandt/text-splitter) | MIT | Semantic chunking | `memory/chunking.rs` |
| [Wasmtime](https://wasmtime.dev/) | Apache-2.0 (with WASM exception) | Optional WASM agent runtime (`wasm-sandbox`) | Sandboxed plugins; default builds keep a clear disabled stub for loader-stable CI/headless MCP. |
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
| Microsoft Learn — [Enabling Visual Styles](https://learn.microsoft.com/en-us/windows/win32/controls/cookbook-overview) and [Application manifests](https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests) | Microsoft documentation / terms | `src-tauri/build.rs` Windows manifest root-cause fix | Thank you for documenting Common Controls v6 activation manifests, `Microsoft.Windows.Common-Controls` dependency metadata, compatibility GUIDs, UTF-8 active code page, DPI awareness, longPathAware, and UAC manifest fields used to fix Windows `TaskDialogIndirect` loader failures. |
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
