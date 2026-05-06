# TerranSoul

> **🚧 This project is under active construction since 10/04/2026.**
> If you are interested, please discuss via <https://discord.gg/RzXcvsabKD> to become a contributor.

> **💡 Why TerranSoul?**
> Any dev/tech person needs and is building their own tech personal assistant, why don't build one benefiting all? Tools like OpenClaw, Claude Cowork, and other AI agents can already perform like J.A.R.V.I.S. — but J.A.R.V.I.S. was never just an AI agent. It connected multiple devices, divided tasks across machines, hosted its own infrastructure, maintained the right RAG pipelines, and had persistent memory. Today's AI is powerful but fragmented: agents don't host infrastructure, don't manage retrieval or memory end-to-end, and can't split work across your PCs. So why not bring everything together under one roof? I'm just kicking this off — if you're interested, come drive it even further with your imagination.

**Your 3D AI companion for everyday life — cross-device, open-source, and built for context and harness engineering, with seamless integration with AI systems like OpenClaw, N8N, Gemma 4, Codex and so on...**

## What this AI is

- **For technical users:** A full 3D AI companion focused on complete-context engineering (using your project, memory, and device context together) and harness engineering (building, testing, and orchestrating AI workflows end-to-end).
- **For non-technical users:** A 3D AI companion that helps with daily work on desktop and mobile, like drafting messages, organizing tasks, and managing reminders.

Try to teach your AI with our software by configuring it, giving it tasks, and experimenting with its capabilities yourself.  
If you have any personal enquiries, please contact the owner of this repo via: **darren.bui@terransoul.com**

[![TerranSoul CI](https://github.com/Terranes/TerranSoul/actions/workflows/terransoul-ci.yml/badge.svg)](https://github.com/Terranes/TerranSoul/actions/workflows/terransoul-ci.yml)
[![CodeQL](https://github.com/Terranes/TerranSoul/actions/workflows/codeql.yml/badge.svg)](https://github.com/Terranes/TerranSoul/actions/workflows/codeql.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)
[![Vue 3](https://img.shields.io/badge/Vue-3.5-4FC08D?logo=vuedotjs&logoColor=white)](https://vuejs.org/)
[![ESLint](https://img.shields.io/badge/ESLint-passing-4B32C3?logo=eslint&logoColor=white)](https://eslint.org/)
[![Vitest](https://img.shields.io/badge/Tests-1475_passing-6E9F18?logo=vitest&logoColor=white)](https://vitest.dev/)

---

## Design Philosophy — Build Your AI Like an RPG Character

Most AI tools give you a settings page full of toggles and dropdowns. TerranSoul does it differently: **you level up your AI the same way you level up a character in a game.**

Every capability your AI can learn — voice, memory, vision, music — is a **quest** you complete. Quests teach you how each feature works and reward you with a smarter companion. Unlock the right combination of skills and you trigger **combos** — powerful synergies like "Offline Sage" (local LLM + memory) or "Omniscient Companion" (vision + memory + voice input).

### Your AI Has a Brain — And You Build It

TerranSoul's architecture mirrors the human brain. Each region maps to a real AI subsystem you progressively unlock:

| Human Brain                | AI System                          | RPG Stat             |
| -------------------------- | ---------------------------------- | -------------------- |
| Prefrontal Cortex          | Reasoning Engine (local/free/paid LLM + Agents) | 🧠 Intelligence      |
| Hippocampus                | Long-term Memory + Memory Hooks    | 📖 Wisdom            |
| Working Memory Network     | Short-term Memory                  | 🎯 Focus             |
| Neocortex                  | Retrieval System (RAG / Knowledge Wiki) | 📚 Knowledge         |
| Basal Ganglia / Cerebellum | Control & Execution Layer (Tauri IPC + RemoteHost/gRPC-Web) | ⚡ Dexterity         |
| Sense of Self / Mirror Neurons | Persona & Self-Learning Animation | 🎭 Charisma       |

> 📖 **Deep dive:** every cell in this table — the LLM providers behind Intelligence, the three-tier store and embedding model behind Wisdom, sandboxed plugin memory hooks, the hybrid 6-signal RAG and Knowledge Wiki layer behind Knowledge, the typed entity-relationship graph, decay/GC, cognitive episodic/semantic/procedural/judgment axes, multi-source ingestion, sleep-time consolidation, RemoteHost/gRPC-Web brain transport, and the April 2026 research survey — is documented in **[docs/brain-advanced-design.md](docs/brain-advanced-design.md)**. Any contribution that touches the brain (LLM, memory, RAG, ingestion, embeddings, cognitive-kind, memory plugin hooks, brain-gating quests) must consult that doc first. The Charisma row — persona traits, the master-mirror self-learning loop, ARKit-blendshape → VRM expression mapping, MediaPipe FaceLandmarker / PoseLandmarker, and the per-session camera consent contract — is documented in **[docs/persona-design.md](docs/persona-design.md)**, which any persona/animation contribution must consult first.

> 🖥️ **Offline / Local LLM models (current):** TerranSoul's hardware-adaptive model recommender selects from **Gemma 4** and **Qwen** families for fully offline, private operation via Ollama — chosen based on your available RAM. This list is kept up to date as better locally-runnable models emerge.

As you unlock skills, your AI's stats grow. A freshly installed TerranSoul starts at level 1 by connecting a free-tier or local brain provider. By the time you've completed the Ultimate tier, you have a fully autonomous assistant with voice, vision, memory, multi-device sync, and community agents — all configured through gameplay, not menus.

### The Skill Tree — Constellation Map

Skills are laid out on a **constellation map** — a full-screen dark star-field with circular category clusters arranged radially, each containing skill nodes in concentric rings:

![Skill Tree — Constellation Map](recording/skill-tree.png)

Each **category cluster** (Brain, Voice, Avatar, Social, Utility) is a radial wheel of nodes. Foundation skills sit in the inner ring, Advanced in the middle ring, and Ultimate on the outer ring. Glowing connection lines trace the prerequisite chains between nodes. Clicking a cluster zooms in; clicking a node opens its quest detail.

```
         ┌─ Voice (🗣️ jade) ──────┐     ┌── Avatar (✨ gold) ──┐
         │  🗣️ Gift of Speech      │     │  ✨ Summon Avatar     │
         │  🎤 Voice Command       │     │  🐾 Desktop Familiar  │
         │  🐉 Dragon's Ear        │     └───────────────────────┘
         │  🔤 Power Words         │
         │  🎭 Voice Splitter      │            ┌── Social (🔗 sapphire) ──┐
         └─────────────────────────┘            │  🔗 Soul Link            │
                                                │  🤖 Agent Summoning      │
    ┌── Brain (🧠 crimson) ────────────┐        └──────────────────────────┘
    │  🧠 Awaken the Mind              │
    │  ⚡ Superior Intellect            │   ┌── Utility (📀 amethyst) ──────┐
    │  🏰 Inner Sanctum                │   │  🎵 Ambient Aura              │
    │  📖 Long-Term Memory             │   │  📀 Jukebox  🎬 Watch Party    │
    │  📸 All-Seeing Eye               │   │  👁️ Sixth Sense  🌍 Babel Tongue│
    │  ⚠️ Evolve Beyond                │   │  🏗️ System Integration         │
    └──────────────────────────────────┘   └────────────────────────────────┘
```

### Quests — Learn by Doing

Each skill node is a **quest** with objectives, rewards, and a story-style description. For example:

> **🧠 Awaken the Mind** — *Connect to a free cloud AI*
>
> Your companion awakens! Connect to a free LLM API and watch your AI come alive with real-time conversation, emotion-tagged responses, and avatar reactions.
>
> **Rewards:** Real-time AI chat · Emotion-tagged responses · Sentiment-based avatar reactions

When you send "Where can I start?" or "What should I do?", your AI responds naturally and suggests the next available quest — no rigid menus, just a conversation with your companion about what to unlock next.

<!-- TODO: Add screenshot of quest overlay with Accept/Tell me more/Maybe later tiles -->
<!-- ![Quest Overlay](recording/quest-overlay.png) -->

### Combos — Skill Synergies

Unlock the right combination of skills and you trigger **combos** — bonus capabilities that emerge from synergy:

| Combo                    | Skills                               | What You Get                               |
| ------------------------ | ------------------------------------ | ------------------------------------------ |
| 🎧 DJ Companion          | Voice + Custom Music                 | AI curates music based on mood             |
| 💬 Full Conversation     | Voice Input + Voice Output           | Hands-free voice chat                      |
| 🧠 True Recall           | Paid Brain + Memory                  | Context-aware responses from full history  |
| 🏔️ Offline Sage          | Local Brain + Memory                 | Full AI offline with persistent memory     |
| 👂 Perfect Hearing       | Whisper ASR + Hotwords               | Boosted speech recognition accuracy        |
| 👥 Social Memory         | Speaker ID + Memory                  | Remembers who said what                    |
| 🌐 Universal Translator  | Translation + Voice Input            | Real-time voice translation                |
| 👁️ Omniscient Companion  | Vision + Memory + Voice              | Sees, hears, and remembers everything      |
| 🐝 Hive Mind             | Agents + Device Link                 | Multi-device agent orchestration           |
| 🐾 Living Desktop Pet    | Pet Mode + Voice + Presence          | Reactive floating desktop companion        |
| ⚡ Instant Companion     | Keyboard Shortcuts + Pet Mode        | Global hotkey summons your AI              |
| 🏠 Always There          | Auto-Start + Pet Mode + Presence     | AI greets you every time you boot up       |

<!-- TODO: Add screenshot of combo unlock animation -->
<!-- ![Combo Unlock](recording/combo-unlock.png) -->

### Brain Evolution Paths

There are multiple paths to evolve your AI's brain — each with different tradeoffs:

```
🧠 Free Brain (OpenRouter/Gemini/NVIDIA/Pollinations/Groq)
├── ⚡ Superior Intellect (Paid API — OpenAI/Anthropic)
│   ├── 🤖 Agent Summoning (community AI agents)
│   ├── 🌍 Babel Tongue (real-time translation)
│   └── 📸 All-Seeing Eye (screen vision)
└── 🏰 Inner Sanctum (Local LLM via llmfit)
    └── Full offline operation — no internet needed
```

Each path is a quest chain. Browser/Vercel asks you to choose a provider from chat or pet mode first; the provider page opens before any key/token is pasted into TerranSoul. From there, you choose: use a free-tier provider, pay for power (Superior Intellect), or invest time in local setup for privacy and offline capability (Inner Sanctum).

---

## Vision

TerranSoul is an open-source **3D virtual assistant + AI package manager** that runs across:

| Platform | Target |
|----------|--------|
| Desktop | Windows · macOS · Linux |
| Mobile | iOS · Android |
| Browser | Vercel-only static web test environment + live pet-mode demo + known-host LAN bridge |

The iOS target now has a Tauri 2 platform overlay (`src-tauri/tauri.ios.conf.json`), shared Vue safe-area layout, and Stronghold-backed secure storage for pairing credentials. Generate the Xcode project on macOS with `npm run tauri:ios:init`; non-macOS hosts can still run `npm run tauri:ios:check` to validate the scaffold.

TerranSoul includes a **TerranSoul Link** layer that securely connects all your devices so you can:

- 💬 Chat with TerranSoul anywhere
- 🔄 Sync conversations and settings across devices
- 🖥️ Control other devices remotely (send commands to run on your PC from your phone)
- 🤖 Orchestrate AI agents and plugins (OpenClaw Bridge, Claude Cowork, etc.)

---

## What's Implemented

TerranSoul has completed **18 phases of development** (Phases 0–14 + partial 15–18). Here's what's working today:

### � Skill Tree / Quest System (RPG Brain Configuration)
- **Constellation map** — full-screen radial cluster layout with pan/zoom and minimap
- Floating **crystal orb** progress indicator opens the constellation
- **3 tiers:** Foundation → Advanced → Ultimate
- **Skill nodes** across Brain, Voice, Avatar, Social and Utility categories
- **Combo abilities** triggered by unlocking specific skill pairs/triples (DJ Companion, Hive Mind, Offline Sage, etc.)
- Quest nodes with prerequisites, rewards, objectives, and story descriptions
- Brain-based quest detection — your AI suggests quests conversationally, not via rigid menus
- Hot-seat overlay with Accept / Tell me more / Maybe later choice tiles
- Daily AI-prioritized quest suggestions
- Pin/dismiss/manual-complete quests
- Quest confirmation dialog + reward panel with choices
- Persistent tracker (Tauri file + localStorage fallback, merged on load)

### 🎭 3D Character System
- **VRM 1.0 & 0.x** model support via Three.js + `@pixiv/three-vrm`
- 2 bundled default models (Shinra, Komori) + persistent custom VRM import
- Natural relaxed pose (not T-pose), spring bone warmup, frustum culling disabled
- **AvatarStateMachine** — 5 body states (idle, thinking, talking, happy, sad) with expression-driven animation
- **Exponential damping** for smooth bone/expression transitions
- **5-channel FFT lip sync** (Aa, Ih, Ou, Ee, Oh) via Web Worker audio analysis
- **Gesture blending** (MANN-inspired procedural animation)
- **On-demand rendering** — throttles to ~15 FPS when idle, 60 FPS when active
- Placeholder fallback character if VRM loading fails
- Error overlay with retry button

### 🧠 Brain System (LLM Integration — The "Prefrontal Cortex")

> Architectural reference: **[docs/brain-advanced-design.md](docs/brain-advanced-design.md)** covers every component below in depth, including the April 2026 modern-RAG research survey and the Graphify/Karpathy-inspired Knowledge Wiki layer documented in **[docs/llm-wiki-pattern-application.md](docs/llm-wiki-pattern-application.md)**.

**Providers & modes** (`src-tauri/src/brain/`)
- **4 brain modes:** Free API (OpenRouter, Gemini, NVIDIA NIM, Pollinations, Groq, Cerebras, and other free-tier providers), Paid API (OpenAI / Anthropic / Gemini / OpenAI-compatible), Local Ollama/LM Studio, Stub fallback
- Implementations: `OllamaAgent`, `OpenAiClient`, `FreeProvider`, `ProviderRotator`, `StubAgent`
- Cloud embedding API (`cloud_embeddings.rs`) — unified `embed_for_mode` dispatcher routes to OpenAI-compatible `/v1/embeddings` for paid/free cloud modes, so vector RAG works without local Ollama
- External CLI agents (Chunk 1.5): multi-agent **roster** + Temporal-style **durable workflow engine** (`src-tauri/src/agents/`, `src-tauri/src/workflows/`)
- Hardware-adaptive **model recommender** (Gemma 4, Phi-4, Kimi K2.6 cloud) based on detected RAM
- Authorize-first provider setup — browser, static-web chat, Marketplace, and the Tauri setup wizard launch provider pages first, with manual key/token entry kept as a secondary direct-call option and selectable free-provider models persisted in `BrainMode::FreeApi.model`
- Streaming responses (SSE → `llm-chunk` Tauri event, parsed by `StreamTagParser` state machine)
- Animation channel: `llm-animation` events for `<anim>` JSON blocks emitted by the LLM
- MCP app activity channel: backend MCP tool calls emit `mcp-activity` snapshots with active provider/model, phase, tool title, and speakable status text so MCP mode visibly and audibly narrates what the configured brain is doing.
- Provider health monitoring + automatic failover, migration detection when APIs deprecate
- Chat-based LLM switching ("switch to groq", "use pollinations")
- Browser-only Vercel onboarding — the landing page shows provider and LAN buttons; chat and pet mode open the provider chooser when no backend brain is connected. Current recommendations put OpenRouter first for free model breadth, with Gemini, NVIDIA NIM, ChatGPT/OpenAI, and Pollinations available through provider-page authorization plus an optional manual key/token step. Choices are remembered in the `brain` Pinia store + localStorage, with a visible Reconfigure LLM button and no Tauri installer or backend account required.
- Browser LAN bridge — the static page can save and probe a user-entered TerranSoul host (`ts.browser.lan.host`) and then open the existing `remote-conversation` chat surface through `RemoteHost`. It does **not** auto-detect every LAN TerranSoul: backendless HTTPS browsers cannot listen for mDNS/UDP advertisements, inspect router/ARP tables, or safely scan private subnets. A Vercel HTTPS page also cannot call plaintext `http://192.168.x.x` hosts; direct browser LAN calls require same-machine loopback, local development over HTTP, or a browser-trusted HTTPS host that allows the origin. Native mobile/desktop pairing remains the reliable LAN path.
- Browser-native RAG for the static web demo — direct provider chat can inject local query-scoped `[RETRIEVED CONTEXT]` packs (containing backward-compatible `[LONG-TERM MEMORY]` records plus an explicit "not the whole database" contract) without a TerranSoul backend by using the browser memory adapter documented in [docs/brain-advanced-design.md](docs/brain-advanced-design.md#browser-mode-surface)
- Persona-based fallback when no LLM is configured
- **RemoteHost transport adapter** (`src/transport/`) — shared frontend contract for local in-process Tauri IPC and remote gRPC-Web hosts. Browser/WebView clients use `@bufbuild/connect` + `@bufbuild/connect-web` descriptors for `Brain.Health`, `Brain.Search`, `Brain.StreamSearch`, `PhoneControl.SendChatMessage`, and `PhoneControl.StreamChatMessage`, while desktop keeps using the same interface over local `invoke()` calls. The phone tool layer (`remote-tools.ts`) exposes `describe_copilot_session`, `describe_workflow_progress`, and `continue_workflow` on top of that same contract, and the iOS notification watcher polls the same adapter for long-running workflow/Copilot updates. Browser mode can opt into this same store from a saved known LAN host, but discovery still requires native pairing or a rendezvous/signaling service.
- **LLM-powered intent classifier** (`src-tauri/src/brain/intent_classifier.rs` + `classify_intent` Tauri command) — every chat turn is classified by the configured brain (Free → Paid → Local) into a typed `IntentDecision` (`chat`, `learn_with_docs{topic}`, `teach_ingest{topic}`, `gated_setup{upgrade_gemini|provide_context}`, `unknown`). Replaces three brittle English-only regex detectors so paraphrases, typos and multilingual phrasings (`học luật Việt Nam từ tài liệu của tôi`) all route correctly. 3 s hard timeout + 30 s in-memory LRU cache; on `unknown` the frontend automatically triggers the install-all overlay so a local Ollama brain is set up — guaranteeing every future turn has a working classifier offline. **Every "LLM decides" surface (intent classifier, unknown→install fallback, don't-know gate, post-reply quest suggestions, chat-based LLM-switching commands, yes/no quick-reply buttons, model-capacity auto-upgrade prompt) is user-toggleable** from the Brain panel's "🧭 AI decision-making" section. New code touching AI routing must follow **[`rules/llm-decision-rules.md`](rules/llm-decision-rules.md)** — no regex / `.includes` / keyword arrays driving AI behaviour. See **[docs/brain-advanced-design.md § Intent Classification](docs/brain-advanced-design.md#intent-classification)**.
- 60s streaming timeout + 30s fallback timeout to prevent stuck states

**Three-tier memory + RAG** (`src-tauri/src/memory/`)
- **Three tiers** mirroring human cognition: **Short-term** (in-memory `Vec<Message>`, last ~20 turns) → **Working** (SQLite, session-scoped) → **Long-term** (SQLite, vector-indexed, decay/GC managed)
- **Cognitive memory axes** — every memory is also classified `episodic` / `semantic` / `procedural` / `judgment` via the pure-function classifier `memory::cognitive_kind::classify` (mirrored 1:1 in TS at `src/utils/cognitive-kind.ts`); SQLite stores an optional `cognitive_kind` column for seeded/known rows and falls back to the classifier when it is `NULL`.
- **Hybrid 6-signal RAG search** — `vector_similarity` (40%) + `keyword_match` (20%) + `recency_bias` (15%) + `importance` (10%) + `decay_score` (10%) + `tier_priority` (5%)
- **Retrieved context-pack contract** (`memory/context_pack.rs` + browser mirror in `conversation.ts`) — all live chat surfaces wrap retrieved records in `[RETRIEVED CONTEXT]` before the legacy `[LONG-TERM MEMORY]` block, telling the LLM these are query results from a database-backed RAG store, not an exhaustive transcript or complete "memory."
- **Embeddings:** Ollama `nomic-embed-text` (768-dim) by default, stored as SQLite BLOB; chat-model fallback with process-lifetime "unsupported" cache + 60s `/api/tags` probe cache
- **Reciprocal Rank Fusion (RRF)** — `memory/fusion.rs` ships the Cormack RRF utility (`k=60`) and `MemoryStore::hybrid_search_rrf` wires it into a real retrieval path that fuses independent vector / keyword / freshness rankings; exposed as the `hybrid_search_memories_rrf` Tauri command (April 2026 research absorption — `docs/brain-advanced-design.md` §19.2 row 2)
- **HyDE — Hypothetical Document Embeddings** (`memory/hyde.rs` + `OllamaAgent::hyde_complete` + `hyde_search_memories` Tauri command) — LLM writes a plausible 1-3 sentence answer, we embed *that* for retrieval; falls back gracefully to raw-query embedding then to keyword + freshness when the brain is unreachable. Improves recall on cold or abstract queries (Gao et al., 2022 — `docs/brain-advanced-design.md` §19.2 row 4)
- **Cross-encoder reranker (LLM-as-judge style)** (`memory/reranker.rs` + `OllamaAgent::rerank_score` + `rerank_search_memories` Tauri command) — two-stage retrieval: RRF-fused recall (default `candidates_k=20`, clamped `limit..=50`) → active brain scores each `(query, document)` pair on a 0–10 scale → reorder. RRF/HyDE MCP search enables rerank by default when a local brain is available and prunes scores below the normalised `rerank_threshold` default `0.55` before `[LONG-TERM MEMORY]` prompt injection. Reuses the active brain (no extra model download); interface matches a future BGE/mxbai backend so swapping is one line (`docs/brain-advanced-design.md` §19.2 row 10)
- **Matryoshka two-stage vector search** (`memory/matryoshka.rs` + `matryoshka_search_memories` Tauri command) — truncate the query embedding to 256 dims for a fast first pass, then re-rank the top survivors at full 768-dim cosine similarity. Pure utility (`truncate_and_normalize` + `two_stage_search`) — no schema change, no migration, no index rebuild. Cuts the brute-force fallback path ~3× per candidate with negligible recall hit; helps cold-start before the ANN index is hot (Kusupati et al., NeurIPS 2022 — `docs/brain-advanced-design.md` §19.2 row 11)
- **NotebookLM-style source guides** (`commands/ingest.rs`) — every imported document now gets one deterministic `MemoryType::Summary` source-guide row with source label, compact synopsis, headings, key topics, and starter questions. It is embedded beside the original chunks, so broad overview questions can retrieve a ~450-token guide instead of injecting several raw chunks; exact questions still retrieve the original source chunks. No LLM call is required at ingest time.
- **Session reflection slash command** (`memory/reflection.rs` + `reflect_on_session`) — typing `/reflect` in chat runs the existing segmented fact extraction path, summarizes the current short-term conversation, stores a Working-tier `session_reflection` summary, and writes `derived_from` KG edges to the source turns so provenance/audit views can answer what the reflection came from.
- **Memory-audit provenance view** (`memory/audit.rs` + `get_memory_provenance`) — the Memory tab's Audit view now loads one joined payload per entry: current memory row, `memory_versions` snapshots, and incident `memory_edges` joined to neighboring memory summaries, so users can inspect where a memory came from and what it affects without client-side ID stitching.
- **Knowledge Wiki operations** (`memory/wiki.rs` + `commands/wiki.rs` + `WikiPanel.vue` + MCP `brain_wiki_*` tools) — Graphify/Karpathy-inspired graph/wiki actions are available from chat, BrainView, and authenticated MCP instead of a CLI: `/digest` deduplicates pasted text or starts URL/file ingest, `/ponder` audits conflicts/orphans/stale rows/embedding gaps, `/spotlight` lists most-connected memories, `/serendipity` finds high-confidence cross-community edges, and `/revisit` surfaces append-and-review candidates. MCP uses the same Rust operations under neutral `brain_wiki_*` names; audit/spotlight/serendipity/revisit are read tools, while digest is write-gated.
- **Million-memory benchmark gate** (`benches/million_memory.rs`) — default `cargo bench --bench million_memory` smoke tier builds 10k synthetic 768-dim vectors and runs 1,000 HNSW queries; the 2026-05-07 Windows/i9-12900K run measured p50 0.57 ms, p95 0.74 ms, p99 0.86 ms and wrote `src-tauri/target/bench-results/million_memory.json`. The full `--features bench-million` tier asserts 1M HNSW p99 <= 100 ms, skips the linear backend at 1M, and times capacity pruning from `cap * 1.05` to `cap * 0.95`. Run instructions, JSON report schema, env knobs (`TS_BENCH_SCALES`, `TS_BENCH_FORCE_LARGE`, `TS_BENCH_OUTPUT_DIR`), and a step-by-step recipe for adding new benchmarks live in [docs/benchmarking.md](docs/benchmarking.md).
- **Auto-edge extraction on ingest** (`commands/ingest.rs` + `memory/edges.rs`) — after document chunks and source guides are stored/embedded, the ingest pipeline best-effort runs the existing LLM edge proposer (`format_memories_for_extraction` → `parse_llm_edges` → `add_edges_batch`) when `AppSettings.auto_extract_edges` is enabled and a local active brain model is configured. Failures never fail the primary ingest; the maintenance pass can retry later.
- **Self-RAG reflection-token controller** (`orchestrator/self_rag.rs`) — pure decision logic for the Self-RAG iterative-refinement protocol (Asai et al., 2023). Parses `<Retrieve>` / `<Relevant>` / `<Supported>` / `<Useful>` reflection tokens out of LLM responses and runs a 3-iteration state machine that decides whether to retrieve again, accept the answer, or refuse on max-iter / unsupported. Ships with `SELF_RAG_SYSTEM_PROMPT` for prompt injection. Orchestrator-loop integration (re-prompting the LLM with augmented context) is the follow-up Chunk 16.4b — `docs/brain-advanced-design.md` §19.2 row 5
- **Late chunking ingest integration** (`memory/late_chunking.rs` + `commands/ingest.rs` + `OllamaAgent::embed_tokens`) — opt-in via `AppSettings.late_chunking`. When a local Ollama embedder returns per-token vectors for the whole document, ingestion aligns stored chunks back to token spans, mean-pools each chunk with `pool_chunks(...)`, L2-renormalises the vectors, and writes them through the existing SQLite embedding column. If the model only returns standard pooled embeddings, ingestion falls back to the existing per-chunk embedding path. Implements the Jina AI 2024 late-chunking pattern without schema changes — `docs/brain-advanced-design.md` §19.2 row 9
- **CRAG retrieval evaluator** (`memory/crag.rs`) — pure classifier for the Corrective RAG protocol (Yan et al., 2024). `build_evaluator_prompts(query, document)` mirrors the reranker shape; `parse_verdict()` extracts `CORRECT` / `AMBIGUOUS` / `INCORRECT` from LLM replies with whole-word token matching (so `"incorrectly"` doesn't false-match `INCORRECT`); `aggregate()` collapses per-document verdicts into a corpus-level `RetrievalQuality` for orchestrator branching. Query-rewrite + web-search fallback is the follow-up Chunk 16.5b — `docs/brain-advanced-design.md` §19.2 row 6
- **Temporal knowledge graph (V6)** — `memory_edges` gains nullable `valid_from` (inclusive) and `valid_to` (exclusive) Unix-ms columns. `MemoryEdge::is_valid_at(t)` is the pure interval predicate; `MemoryStore::get_edges_for_at(memory_id, direction, valid_at)` is the point-in-time query (legacy callers passing `valid_at = None` see identical behaviour to V5); `MemoryStore::close_edge(id, t)` records supersession. New `close_memory_edge` Tauri command + `valid_from` / `valid_to` parameters on `add_memory_edge`. Implements the Zep / Graphiti pattern (2024) — `docs/brain-advanced-design.md` §19.2 row 13
- **Knowledge graph (V6):** typed directional `memory_edges` table with FK cascade + 17-type relationship taxonomy + temporal validity intervals, `extract_edges_via_brain` LLM extractor, `multi_hop_search_memories` traversal, Cytoscape.js visualization, and the Three.js `BrainGraphViewport` 3-D KG view where node color comes from `cognitive_kind` and edge color comes from `rel_type`
- **Decay, GC, and capacity eviction:** exponential decay (`decay_score *= 0.95^(hours/168)`), access-count tracking, periodic garbage collection, a user-configurable in-memory brain memory/RAG cache cap (`AppSettings.max_memory_mb`, default 10 MB), a persistent storage cap (`AppSettings.max_memory_gb`, default 10 GB), and long-tier count eviction via `memory/eviction.rs`. `enforce_capacity` prunes lowest-utility long-tier rows to `cap * 0.95` while preserving `protected = 1` and `importance >= 4` entries; the shared `brain::maintenance_runtime` runs the same path from GUI and MCP tray/coding-agent modes.
- **Multi-source knowledge management:** source-hash change detection, TTL expiry, access-count decay, **LLM-powered conflict resolution**
- **Obsidian vault export/sync:** vault metadata (`obsidian_path`, `last_exported`) stays on each memory row; bidirectional sync records the actual exported file mtime after writes so a just-exported file is not immediately re-imported on Windows timestamp boundaries.
- **Cross-device memory sync:** `memory::crdt_sync` computes LWW deltas keyed by `source_hash` or `(content_prefix, created_at)`; Soul Link dispatches `memory_sync` / `memory_sync_request` messages through `link::handlers`, applies inbound deltas, replies with pre-apply local deltas, and records Unix-ms `sync_log` watermarks so paired devices do not replay already-synced memories.
- **Mobile/remote memory + chat/workflow surface:** `RemoteHost.searchMemories()` and `RemoteHost.streamSearchMemories()` expose the same RRF/HYBRID/HyDE search modes to browser-native gRPC-Web clients, and `RemoteHost.streamChatMessage()` lets iOS `ChatView` consume desktop-hosted chat streams. `[PERSONA]`, `[LONG-TERM MEMORY]`, and `[HANDOFF]` prompt injection stay server-side; the phone receives clean text chunks. Phone prompts such as “what's Copilot doing?” and “continue the next chunk” route through named RemoteHost tools for Copilot-session narration and workflow progress/continue actions. Paired iOS shells also use `mobile-notifications.ts` + `tauri-plugin-notification` for local long-running workflow, ingest-task, and Copilot threshold notifications over the LAN connection.
- **Sandboxed plugin memory hooks:** `plugins::PluginHost` exposes `memory_hooks` contributions with `pre_store` / `post_store` stages. Plugins can lazily activate via `on_memory_tag`, run inside `WasmRunner`, and return JSON patches that transform content / tags / importance / type before `add_memory` writes to SQLite; post-store hooks are notification/indexing-only.
- **Pluggable storage backends** via `StorageBackend` trait: SQLite (default), PostgreSQL (`sqlx`), SQL Server (`tiberius`), CassandraDB (`scylla`); the optional distributed schemas mirror the canonical memory lifecycle columns (`valid_to`, `obsidian_path`, `last_exported`, `updated_at`, `origin_device`) used by Obsidian export and CRDT sync.
- **LLM-powered memory ops:** `extract_facts`, `summarize`, `semantic_search_entries`, `embed_text`
- **Canonical SQLite schema at V15** with `PRAGMA foreign_keys=ON`; fresh databases are created directly from `memory/schema.rs` with `memories`, `memory_edges`, `memory_versions`, `memory_conflicts`, `pending_embeddings`, `paired_devices`, and `sync_log` tables, plus the `protected` flag used by capacity eviction.

**Frontend brain hub** (`src/views/BrainView.vue`, `src/components/BrainAvatar.vue`)
- Top-level **Brain** tab unifies brain config, hardware probe, RAG capability gauges, cognitive-kind breakdown, RPG stats, a mini memory graph, and a Three.js/d3-force-3d memory KG viewport with cognitive-kind and relation legends
- **Active Selection panel** — surfaces the typed `BrainSelection` snapshot (provider · embedding · memory · search · storage · agents · effective RAG quality %) so the user sees, at a glance, exactly *which* component is answering, ranking, embedding, and storing for them. Backed by the `get_brain_selection` Tauri command.
- **Brain component selection & routing** — every routing decision (provider mode, embedding model, memory tier, search method, RAG injection top-k & threshold, agent dispatch, cognitive-kind classification, storage backend, fallback chains) is documented in **[docs/brain-advanced-design.md § 20](docs/brain-advanced-design.md#brain-component-selection--routing--how-the-llm-knows-what-to-use)**
- **Daily-conversation write-back loop** — every chat turn lands instantly in short-term memory; the **`memory::auto_learn`** policy (default: every 10 turns, 3-turn cooldown) decides when to fire `extract_memories_from_session` automatically. Tunable per-user via `get_auto_learn_policy` / `set_auto_learn_policy` and previewed live via the pure `evaluate_auto_learn` decision query. Full loop documented in **[docs/brain-advanced-design.md § 21](docs/brain-advanced-design.md#how-daily-conversation-updates-the-brain--write-back--learning-loop)**
- **Native code intelligence (clean-room GitNexus parity path)** — GitNexus is **PolyForm-Noncommercial-1.0.0**, so TerranSoul must not bundle, vendor, auto-install, or default-spawn its package, binary, Docker images, prompts, generated skills, source, or UI assets. Public GitNexus docs and DeepWiki pages are credited product/architecture research only. The first-class path is TerranSoul-native Rust/Vue code intelligence: `coding/symbol_index.rs` + `coding/processes.rs` + SQLite + MCP tools (`code_query`, `code_context`, `code_impact`, `code_rename`) under neutral names. The old sidecar bridge has been removed entirely; TerranSoul ships no GitNexus command bridge, catalog entry, UI panel, or sidecar RAG fusion path.
- **Native code-RAG and precomputed relational intelligence** — TerranSoul's direction is to precompute repository structure before the LLM asks: symbols, imports, calls, confidence-scored relations, functional clusters, execution processes, hybrid BM25/vector/RRF code search, diff impact, and graph-backed rename. This gives agents complete context in one native tool call instead of depending on an external noncommercial sidecar.
- **Code-graph workbench roadmap** — Phase 37 adds a dense Brain/Coding workbench inspired by public GitNexus UI behaviour but implemented natively in Vue/Pinia with TerranSoul design tokens: graph canvas, file tree, grounded code-reference panel, chat/tool-call visibility, clickable file/node citations, process diagrams, repo switcher, index status, and blast-radius highlights.
- Pinia stores: `brain.ts`, `conversation.ts`, `memory.ts`, `agent-roster.ts`, `skill-tree.ts`

### 🗣️ Voice System (The "Charisma" Stats)
- **ASR:** Web Speech API (browser-native), Whisper, Groq speech-to-text
- **TTS:** Web Speech (browser SpeechSynthesis — free, offline-capable, no third-party endpoint), OpenAI TTS (optional, requires API key)
- **Hotword detection** for wake-word activation
- **Speaker diarization** support
- LipSync ↔ TTS audio pipeline for real-time mouth animation
- **Audio-prosody persona hints** — when ASR is configured, the Master-Echo
  persona suggester analyses the user's typed turns (which mirror their spoken
  patterns) for tone (concise / elaborate / energetic / inquisitive / emphatic
  / playful), pacing (fast / measured / slow), and quirks (filler words, emoji
  use), and folds those hints into the persona-extraction prompt.
  Camera-free; raw audio is never read; hints are never persisted.

### 💾 Memory System (The "Hippocampus")

> Architectural reference: **[docs/brain-advanced-design.md](docs/brain-advanced-design.md)** — full schema, RAG pipeline, decay model, knowledge graph, and April 2026 research survey.

**Core modules** (`src-tauri/src/memory/`)
- `store.rs` — `MemoryStore` (default SQLite + WAL, schema **V15**), `MemoryEntry`, `MemoryTier` (short / working / long), `MemoryType`, `NewMemory`, `MemoryUpdate`, `MemoryStats`, `MemoryCleanupReport`, `cosine_similarity`, `bytes_to_embedding` / `embedding_to_bytes`, `hybrid_search` (6-signal weighted-sum scoring), `hybrid_search_rrf` (RRF-fused vector + keyword + freshness retrievers), `apply_decay`, `promote`, `get_all_within_storage_bytes` for the default 10 MB in-memory cache cap, and `enforce_size_limit` for the default 10 GB persistent storage cap
- `backend.rs` — `StorageBackend` trait + `StorageConfig` + `StorageError` (the seam every backend implements); feature-gated PostgreSQL, SQL Server, and Cassandra backends keep the same Obsidian/CRDT lifecycle fields as the canonical SQLite memory entry shape.
- `schema.rs` — canonical SQLite schema initializer at V15 (`memories` with `protected` and embedding retry metadata support, `memory_edges` with temporal `valid_from` / `valid_to` columns and `edge_source` external-KG provenance, `memory_versions` edit history, `memory_conflicts`, `pending_embeddings`, `paired_devices`, `updated_at` / `origin_device`, and `sync_log`, `PRAGMA foreign_keys=ON`)
- `edges.rs` — `MemoryEdge`, `NewMemoryEdge` (with V7 `edge_source: Option<String>`), `EdgeDirection`, `EdgeSource`, `EdgeStats`, `COMMON_RELATION_TYPES` (17-type taxonomy), `normalise_rel_type`, `parse_llm_edges`, `format_memories_for_extraction`, `delete_edges_by_edge_source`
- `audit.rs` — joined memory provenance payloads for the Audit tab: current `MemoryEntry`, `memory_versions` edit history, and incident `memory_edges` with compact neighboring-memory summaries
- ~~`gitnexus_mirror.rs`~~ — **Removed.** Sidecar bridge code was deleted per clean-room licensing pivot (PolyForm-NC incompatible). Native code intelligence now lives in `src-tauri/src/coding/` — see [docs/native-code-intelligence-spec.md](docs/native-code-intelligence-spec.md).
- `cognitive_kind.rs` — pure-function `classify(memory_type, tags, content) → CognitiveKind` (`Episodic` / `Semantic` / `Procedural` / `Judgment`); mirrored 1:1 in TS at `src/utils/cognitive-kind.ts`
- `brain_memory.rs` — LLM-powered ops: `extract_facts`, `summarize`, `semantic_search_entries`, `extract_edges_via_brain`
- `reflection.rs` — `/reflect` persistence helper: saves extracted facts, a `session_reflection` summary row, source-turn context rows, and `derived_from` provenance edges in one reportable operation
- `fusion.rs` — `reciprocal_rank_fuse(rankings, k)` (Cormack RRF, `k=60`); consumed by `MemoryStore::hybrid_search_rrf`
- `context_pack.rs` — shared prompt formatter for database-first retrieved context packs. Keeps `[LONG-TERM MEMORY]` for compatibility, but wraps it in `[RETRIEVED CONTEXT]` with a clear contract that the snippets are scoped retrieval results, not the full memory store.
- `hyde.rs` — pure HyDE prompt builder + reply cleaner (Gao et al., 2022); consumed by `OllamaAgent::hyde_complete` and the `hyde_search_memories` Tauri command
- `reranker.rs` — pure cross-encoder rerank prompt builder + score parser + reorder/pruning logic; consumed by `OllamaAgent::rerank_score`, the two-stage `rerank_search_memories` Tauri command, MCP RRF/HyDE search, and chat prompt memory assembly (recall via `hybrid_search_rrf`, precision via LLM-as-judge, default pruning threshold `0.55`)
- `auto_learn.rs` — `AutoLearnPolicy` + pure `evaluate(policy, total_turns, last_autolearn_turn) → AutoLearnDecision`; the cadence policy that turns daily conversation into long-term memory (default: every 10 turns, 3-turn cooldown). See **[docs/brain-advanced-design.md § 21](docs/brain-advanced-design.md#how-daily-conversation-updates-the-brain--write-back--learning-loop)**.
- `crdt_sync.rs` — cross-device LWW delta engine: `compute_sync_deltas(since_timestamp, local_device_id)`, `apply_sync_deltas(deltas, local_device_id)`, `log_sync(peer, direction, entry_count)`, and `last_sync_time(peer)` use Unix-ms watermarks compatible with memory `updated_at`.
- `tag_vocabulary.rs` — curated `CURATED_PREFIXES` (`personal`, `domain`, `project`, `tool`, `code`, `external`, `session`, `quest`), `validate()` / `validate_csv()`, `LEGACY_ALLOW_LIST`, `category_decay_multiplier()` per-prefix decay rates
- `auto_tag.rs` — LLM auto-tagger: `auto_tag_content()` dispatches to Ollama/FreeApi/PaidApi; `parse_tag_response()` validates + caps at 4 curated-prefix tags; `merge_tags()` deduplicates with user tags. Opt-in via `AppSettings.auto_tag`
- `commands/ingest.rs` — document ingestion command: URL/file/crawl fetch, semantic chunking, source-hash staleness detection, deterministic NotebookLM-style source-guide summaries, optional contextual retrieval prefixes, optional late-chunking embeddings, best-effort embedding backfill, and best-effort auto-edge extraction into `memory_edges` when `auto_extract_edges` is enabled.
- `obsidian_export.rs` — one-way Obsidian vault export: `export_to_vault(vault_dir, entries)` writes `<id>-<slug>.md` per long-tier memory with YAML frontmatter; idempotent (mtime-based skip); `slugify`, `format_iso`, `render_markdown` pure helpers
- `temporal.rs` — natural-language time-range parser: `parse_time_range(question, now_ms)` resolves "last N days", "since April", "between X and Y", "today", "yesterday" into `TimeRange { start_ms, end_ms }`; pure std calendar math (Howard Hinnant algorithm), no external crate
- `contextualize.rs` — Contextual Retrieval (Anthropic 2024): `generate_doc_summary()` + `contextualise_chunk(doc_summary, chunk, brain_mode)` + `prepend_context()`; opt-in via `AppSettings.contextual_retrieval`; adds document-level context to each chunk before embedding, reducing failed retrievals by ~49 %
- `versioning.rs` — non-destructive edit history: `save_version(conn, memory_id)` snapshots the current state into `memory_versions` (V8 schema) before each update; `get_history()` returns all previous versions; FK cascade on delete
- `chunking.rs` — semantic chunking pipeline: `split_markdown()` (heading/paragraph/sentence-aware via `text-splitter` crate), `split_text()` (Unicode sentence boundaries), `dedup_chunks()` (SHA-256 hash dedup), `Chunk` struct (index, text, hash, heading metadata); replaces naive word-count splitter
- `conflicts.rs` — contradiction resolution (V9 schema): `build_contradiction_prompt` + `parse_contradiction_reply` for LLM-based contradiction check; `MemoryConflict` CRUD (`add_conflict`, `list_conflicts`, `resolve_conflict`, `dismiss_conflict`, `count_open_conflicts`); losers soft-closed via `valid_to` (never deleted)
- `edge_conflict_scan.rs` — scheduled edge conflict detection: `collect_scan_candidates()` (lock-safe candidate collection), `record_contradiction()` (insert contradicts edge + open conflict), scans positive-relation edges for hidden contradictions via LLM-as-judge
- `ann_index.rs` — vector nearest-neighbor adapter: default builds use a pure-Rust linear cosine index for loader-stable CI/headless MCP runs; the `native-ann` cargo feature enables the persisted `usearch` 2.x HNSW index (`vectors.usearch`) for large local stores where O(log n) lookup matters.
- `eviction.rs` — capacity-based long-tier pruning: `enforce_capacity(conn, cap, 0.95, data_dir)` drops lowest-utility unprotected rows, preserves `protected = 1` and `importance >= 4`, and writes `eviction_log.jsonl` audit records.
- `src-tauri/benches/million_memory.rs` — Criterion benchmark harness for Phase 38 million-memory confidence: 10k smoke by default, gated 1M HNSW threshold run with `--features bench-million`, JSON trend report under `src-tauri/target/bench-results/`.
- Pluggable backends behind cargo features: `postgres.rs` (`sqlx`), `mssql.rs` (`tiberius`), `cassandra.rs` (`scylla`)
- `../plugins/host.rs` — sandboxed plugin extension host for memory hook contributions; `pre_store` hooks can rewrite incoming memory payloads, `post_store` hooks observe persisted memories, and `on_memory_tag` activates processors lazily from curated or plugin-defined tags. Default builds expose the hook API but return a clear disabled message for WASM execution; rebuild with `--features wasm-sandbox` to enable Wasmtime.

**Tauri command surface** (`src-tauri/src/commands/memory.rs`)
- `add_memory` (runs sandboxed `pre_store` plugin hooks before SQLite write and `post_store` hooks after persistence), `update_memory`, `delete_memory`, `get_memories`, `search_memories` (SQL `LIKE`), `semantic_search_memories` (cosine), `hybrid_search_memories` (6-signal weighted-sum), `hybrid_search_memories_rrf` (RRF-fused vector + keyword + freshness), `hyde_search_memories` (HyDE — embed an LLM-written hypothetical answer), `rerank_search_memories` (two-stage RRF-recall + LLM-as-judge cross-encoder rerank), `multi_hop_search_memories` (graph traversal), `temporal_query` (natural-language time-range filter: "last week", "since April", "between X and Y"), `get_relevant_memories`, `get_short_term_memory`, `extract_memories_from_session`, `summarize_session`, `reflect_on_session` (`/reflect` session summary + facts + provenance edges), `backfill_embeddings`, `apply_memory_decay`, `gc_memories`, `promote_memory`, `get_memories_by_tier`, `get_schema_info`, `get_memory_stats`, `add_memory_edge` (V6: optional `valid_from` / `valid_to`), `close_memory_edge` (V6: record supersession), `delete_memory_edge`, `list_memory_edges`, `get_edges_for_memory`, `get_edge_stats`, `list_relation_types`, `extract_edges_via_brain`, `scan_edge_conflicts` (LLM-as-judge scan over positive-relation edges for hidden contradictions), `get_auto_learn_policy`, `set_auto_learn_policy`, `evaluate_auto_learn`, `export_to_obsidian` (one-way vault export with YAML frontmatter), `get_memory_history` (V8: version history for a memory entry), `get_memory_provenance` (joined Audit-tab provenance tree), `adjust_memory_importance` (access-pattern-driven ±1 nudge with version audit trail)

**Storage & RAG**
- **Three tiers** mirroring human cognition: short-term (in-memory `Vec<Message>`, last ~20 turns) → working (SQLite, session-scoped) → long-term (SQLite, vector-indexed, decay/GC managed)
- **Knowledge graph (V8):** typed directional `memory_edges` table with FK cascade + temporal `valid_from` / `valid_to` validity intervals + `edge_source` provenance for native or imported graph edges, 17-type relationship taxonomy, LLM edge extractor, `/reflect` `derived_from` session-provenance edges, multi-hop traversal; `memory_versions` table for non-destructive edit history
- **Embeddings:** Ollama `nomic-embed-text` (768-dim) stored as SQLite BLOB; chat-model fallback with process-lifetime "unsupported" cache + 60s `/api/tags` probe cache
- **Hybrid 6-signal RAG search** — `vector_similarity` (40%) + `keyword_match` (20%) + `recency_bias` (15%) + `importance` (10%) + `decay_score` (10%) + `tier_priority` (5%)
- **Decay, GC, and capacity eviction:** exponential decay (`decay_score *= 0.95^(hours/168)`), access-count tracking, periodic garbage collection, protected/high-importance long-tier capacity pruning, and the shared `brain::maintenance_runtime` scheduler in both GUI and MCP tray/coding-agent modes
- **Multi-source knowledge management:** source-hash change detection, TTL expiry, access-count decay, **LLM-powered conflict resolution**

**Frontend**
- `src/views/MemoryView.vue` — list / grid / graph / audit views, tier chips, tag-prefix category filter chips with counts, search + semantic + hybrid search, and joined provenance inspection for `memory_versions` + `memory_edges`
- `src/components/MemoryGraph.vue` — Cytoscape.js semantic graph visualization with typed edges
- `src/stores/memory.ts` — Pinia store (CRUD + search + streaming results)
- Browser/Vercel mode keeps the million-user demo backend-free: memory records persist to browser storage, use browser-side embeddings + vector/keyword/RRF retrieval, inject the same retrieved-context contract into direct provider chat, and export/import a Drive-friendly sync payload; Rust-backed SQLite/vector search/CRDT remains available through desktop/mobile or a paired RemoteHost, as detailed in [docs/brain-advanced-design.md](docs/brain-advanced-design.md#browser-mode-surface).

### 🎭 Persona System (The "Sense of Self & Mirror Neurons")

> Architectural reference: **[docs/persona-design.md](docs/persona-design.md)** — full traits schema, the master-mirror self-learning loop, ARKit-blendshape → VRM 1.0 expression mapping, MediaPipe FaceLandmarker / PoseLandmarker pipeline, the per-session camera consent contract, the persona quest chain, and the April 2026 research survey (Hunyuan-Motion, MoMask, MotionBERT, MMPose, MimicMotion, MagicAnimate, MotionGPT, Audio2Face-3D / FaceFormer-class visemes).

**Core modules**
- `src-tauri/src/commands/persona.rs` — Tauri persistence: `get_persona`, `save_persona`, `set_persona_block` / `get_persona_block`, `list_/save_/delete_learned_expression`, `list_/save_/delete_learned_motion`, `extract_persona_from_brain` (Master-Echo brain-extraction loop, shipped 2026-04-24), `check_persona_drift` (drift detection comparing active persona vs `personal:*` memories, shipped 2026-04-26), `export_persona_pack` / `preview_persona_pack` / `import_persona_pack` (persona pack export / import, shipped 2026-04-24). Atomic JSON-on-disk under `<app_data_dir>/persona/{persona.json, expressions/, motions/}`. Path-traversal-safe id validation. **No camera commands** — webcam frames never cross the IPC boundary; only user-confirmed JSON landmark artifacts ever reach Rust.
- `src-tauri/src/persona/extract.rs` — pure prompt + parser for Master-Echo: `build_persona_prompt`, `assemble_snippets` (last 30 turns + up to 20 long-tier `personal:*` memories), `parse_persona_reply` (tolerant of markdown fences, leading prose, non-string list entries; bio capped at 500 chars; lists deduped + capped at 6). Same testable-seam shape as `memory/hyde.rs` and `memory/reranker.rs`.
- `src-tauri/src/persona/drift.rs` — pure prompt + parser for persona drift detection: `build_drift_prompt` (persona JSON + personal memories → comparison prompt), `parse_drift_reply` (tolerant of fences/prose), `DriftReport` (detected, summary, suggested_changes), `DriftSuggestion` (field/current/proposed). 14 unit tests.
- `src-tauri/src/persona/motion_smooth.rs` — feature-gated native recorded-motion cleanup (`motion-research`): zero-phase Gaussian smoothing over `LearnedMotion` bone channels with endpoint pinning and displacement stats. Chunk 27.5 chose this as the first offline polish product path; HunyuanVideo / MimicMotion / MagicAnimate-style video diffusion remains optional sidecar research documented in [docs/offline-motion-polish-research.md](docs/offline-motion-polish-research.md), not a bundled dependency.
- `src-tauri/src/persona/motion_tokens.rs` — feature-gated MotionGPT-style token codec (`motion-research`): deterministic quantization between VRM bone Euler frames and compact motion-token grids. Optional Chunk 14.16g evaluated MotionGPT / T2M-GPT ONNX inference and deferred bundled model integration until a license-clean, checksummed, VRM-native sidecar artifact exists; see [docs/motion-model-inference-evaluation.md](docs/motion-model-inference-evaluation.md).
- `src-tauri/src/persona/pack.rs` — pure pack codec: `PersonaPack` envelope (versioned, opaque per-asset), `build_pack`, `pack_to_string`, `parse_pack` (1 MiB hard cap, future-version rejection), `validate_asset` (mirrors `validate_id` for path-traversal safety), `ImportReport` (per-entry skip messages capped at 32 + truncation marker). 18 unit tests.
- `src/stores/persona-types.ts` — `PersonaTraits` (name, role, bio, tone, quirks, avoid, active, version), `LearnedExpression`, `LearnedMotion`, `DriftReport`, `DriftSuggestion`, `defaultPersona()`, forward-compatible `migratePersonaTraits()`.
- `src/stores/persona.ts` — Pinia store for the active persona + learned libraries + ephemeral per-session camera consent state (never persisted) + `suggestPersonaFromBrain()` action that wraps the Master-Echo Tauri command + `exportPack()` / `previewImportPack()` / `importPack()` actions for the persona pack. Tauri-persisted with localStorage fallback.
- `src/utils/persona-prompt.ts` — pure `buildPersonaBlock(traits, learnedMotions)` that renders the `[PERSONA]` block injected into every chat's system prompt next to `[LONG-TERM MEMORY]` (browser path) or via `set_persona_block` (server path).

**Persona ↔ Brain integration**
- The rendered `[PERSONA]` block is spliced into the system prompt by both streaming pipelines (`stream_ollama` + `stream_openai_api` in `src-tauri/src/commands/streaming.rs`) alongside `[LONG-TERM MEMORY]`. Empty traits → no injection.
- The "Master's Echo" quest asks the brain to read your conversations + personal memories and propose a persona that mirrors who you are — the camera-free Master-Echo brain-extraction loop documented in [persona-design.md § 3](docs/persona-design.md#3-the-master-mirror-self-learning-loop) and [§ 9.3](docs/persona-design.md#93-llm-assisted-persona-authoring--shipped-2026-04-24). The Persona panel surfaces a "✨ Suggest from my chats" button + review-before-apply card with Apply / Load-into-editor / Discard actions; nothing is auto-saved.
- **Persona drift detection** (Chunk 14.8) — piggybacks on the auto-learn loop. After every 25 accumulated auto-extracted facts, the brain compares the active `PersonaTraits` against the latest `personal:*` memory cluster and surfaces a `DriftReport` with a summary + suggested trait changes so the UI can prompt "Echo noticed you've shifted toward …; update persona?"

**Persona quest chain (main + side)**
- **Main chain (camera-free):** `soul-mirror` → `my-persona` → `master-echo` — every step works without ever turning on the camera.
- **Side chain (camera-driven):** `expressions-pack` ("Mask of a Thousand Faces") + `motion-capture` ("Mirror Dance"). Privacy contract: **per-session/per-chat consent only**, never always-on. See [persona-design.md § 5](docs/persona-design.md#5-privacy--consent--the-per-session-camera-leash).
- `src/renderer/face-mirror.ts` — pure `mapBlendshapesToVRM()` ARKit→VRM mapper (52 blendshapes → 12+2 channels) + `FaceMirror` class wrapping lazy-loaded `@mediapipe/tasks-vision` FaceLandmarker with EMA smoothing. 16 unit tests.
- `src/composables/useCameraCapture.ts` — per-session camera consent composable (getUserMedia + FaceMirror lifecycle, 5-min idle auto-stop, component-unmount cleanup).
- `src/renderer/pose-mirror.ts` — pure `retargetPoseToVRM()` mapper (33 MediaPipe landmarks → 11 VRM humanoid bones via atan2 joint angles + clamping) + `PoseMirror` class wrapping lazy-loaded PoseLandmarker with EMA bone smoothing. 11 unit tests.
- `src-tauri/src/persona/retarget.rs` — feature-gated Rust geometric retargeter (`motion-research`) for offline saved clips: 33 BlazePose landmarks → 17 VRM bones with two-bone IK, joint limits, partial-visibility degradation, and `retarget_sequence()` batch processing. Chunk 27.4 evaluated MoMask-style ML reconstruction against this baseline and chose **no bundled model dependency yet**; any future ML lift must be an optional saved-landmark sidecar documented in [docs/momask-full-body-retarget-research.md](docs/momask-full-body-retarget-research.md).
- `src/renderer/vrma-baker.ts` — pure `bakeMotionToClip()` converts LearnedMotion JSON frames into `THREE.AnimationClip` with quaternion keyframe tracks. `bakeAllMotions()` batch-bakes. 12 unit tests.
- `src/renderer/learned-motion-player.ts` — `LearnedMotionPlayer` class wraps bakeMotionToClip + VrmaManager.playClip for on-demand playback of learned motions. `applyLearnedExpression()` / `clearExpressionPreview()` helpers for timed expression preview on VRM. 10 unit tests.
- `src/renderer/phoneme-viseme.ts` — text-driven phoneme-to-viseme mapper: English grapheme tokenizer (15 digraphs + single-char fallback) → 5-channel viseme timeline builder → `VisemeScheduler` with frame-accurate interpolation. Replaces FFT band-energy lip-sync when text + duration are available. 22 unit tests.
- Neural audio-to-face posture — Chunk 27.6 keeps the `phoneme-viseme.ts` scheduler + `lip-sync.ts` FFT/Web Worker fallback as the default speaking path. Audio2Face-3D is documented as a future optional local NVIDIA sidecar only after CUDA/TensorRT, license, model-checksum, and VRM-frame adapter gates pass; FaceFormer/EmoTalk remain research references. See [docs/neural-audio-to-face-evaluation.md](docs/neural-audio-to-face-evaluation.md).
- `src/components/PersonaTeacher.vue` — "Teach an Expression / Motion" panel: Expression/Motion tab toggle, consent dialog → live camera preview (CAMERA LIVE badge) → capture pose or record motion (30 fps, max 10s) → name + trigger → save. 5 component tests.

**Frontend**
- `src/components/PersonaPanel.vue` — full add / update / delete / review management UI mounted in the Brain hub (`BrainView.vue`); edits all traits, lists every learned-expression / learned-motion artifact with one-click delete, live-previews the rendered `[PERSONA]` system-prompt block, and includes the "✨ Suggest from my chats" Master-Echo button.
- `src/components/PersonaPackPanel.vue` — sibling card for persona pack export / import (Chunk 14.7): export with optional note + clipboard copy + `.json` download (Blob + `<a download>` — no Tauri `dialog` plugin needed), import with **🔍 Preview** dry-run + **⤴ Apply** + per-entry skip report.
- `src/components/PersonaListEditor.vue` — small chip-style list editor used by the persona panel for `tone` / `quirks` / `avoid` arrays.

### 🔗 TerranSoul Link
- Device identity + pairing with QR codes
- Cross-device conversation sync
- Cross-device long-term memory sync over Soul Link `memory_sync` messages, with LWW conflict resolution and automatic sync after connect/reconnect
- Settings synchronization

### 📦 AI Package Manager
- Install / update / remove / start / stop agents
- Package registry with marketplace UI (browse works out-of-the-box via the in-process catalog registry)
- Local Ollama models surface as marketplace agents — install & activate from the same UI
- Optional WASM sandbox for agent isolation (`--features wasm-sandbox`); default builds keep plugin metadata and return a clear disabled message for WASM execution.
- OpenClaw now ships as the built-in `openclaw-bridge` plugin, with `/openclaw read`, `/openclaw fetch`, and `/openclaw chat` routed through plugin capability consent. See [`tutorials/openclaw-plugin-tutorial.md`](./tutorials/openclaw-plugin-tutorial.md).

### 🤖 AI Coding Integrations (Brain Gateway)

> Architectural reference: **[docs/AI-coding-integrations.md](docs/AI-coding-integrations.md)** — full protocol details, security model, auto-setup writers, and the VS Code Copilot incremental-indexing pact.

- **MCP server** (Chunk 15.1) — HTTP/JSON-RPC 2.0 on `127.0.0.1:7421` via axum Streamable HTTP transport, plus dev/app ports `7422` / `7423`. Bearer-token auth (`mcp-token.txt`). 8 brain tools (`brain_search`, `brain_get_entry`, `brain_list_recent`, `brain_kg_neighbors`, `brain_summarize`, `brain_suggest_context`, `brain_ingest_url`, `brain_health`) run with the explicit MCP transport capability profile (`brain_read`, `brain_write`, and `code_read`) so local coding agents can persist durable self-improve knowledge through the brain. Native code-intelligence tools (`code_query`, `code_context`, `code_impact`, `code_rename`) are still gated by `code_read`. MCP app mode auto-configures a usable brain and emits live `mcp-activity` events for the on-screen/audible model status panel. `npm run mcp` builds Vite assets and runs the MCP full-UI tray runtime in Rust release mode, using a built-in status page instead of the dev server. The MCP tray/coding-agent runtime seeds from the committed `mcp-data/shared/` dataset and, on first run, immediately backfills seed embeddings; provider embeddings are preferred, and the deterministic offline embedder hashes tokens into 256-dimensional vectors when no provider embedding is available so SQLite + vector search + RRF are warm before the first agent query. Tokens, SQLite DBs, indexes, logs, locks, sessions, and worktrees stay ignored; MCP self-improve runtime logs under `mcp-data/` keep only the current file plus `.001`, capped at 1 MiB per file.
  For migration and schema bootstrap, all app modes (release/dev/MCP) create the canonical SQLite schema first and then run pending seed migrations. Shared seed resolution is deterministic: `TERRANSOUL_MCP_SHARED_DIR` (if set) -> `<data_dir>/shared` -> `<cwd>/mcp-data/shared` -> compiled-in migration/config fallback.
- **LAN brain sharing** — opt-in MCP LAN mode lets one TerranSoul host advertise its brain on UDP `7424` and choose either `token required` access or `public read-only` access for that LAN session. Public mode exposes only the read-only MCP brain surface without a bearer token; token mode keeps the existing authenticated flow. See the illustrated [LAN MCP sharing tutorial](tutorials/lan-mcp-sharing-tutorial.md) for the Alice Vietnamese law notes walkthrough and safety checklist.
- **BrainGateway trait** (Chunk 15.3) — single typed op surface shared by MCP and future gRPC transports. `AppStateGateway` adapter, capability gating (`GatewayCaps`), typed errors, delta-stable fingerprint for VS Code Copilot cache.
- **MCP ingest sink** — Tauri app/tray MCP servers attach an `AppHandle`-backed ingest sink, and stdio MCP attaches an `AppState`-backed silent sink, so `brain_ingest_url` starts the real background ingest pipeline and returns a task id instead of stopping at capability checks.
- **gRPC/gRPC-Web server** — tonic server on the LAN gRPC port accepts native HTTP/2 and browser-native gRPC-Web (`tonic_web::GrpcWebLayer`) for `terransoul.brain.v1.Brain` plus the paired phone-control surface, including streamed chat and workflow/Copilot remote tools. The Vue `RemoteHost` adapter uses `@bufbuild/connect-web`, so the same components can run locally or from an iOS WebView.
- **AppState Arc newtype** — `AppState(Arc<AppStateInner>)` with `Deref + Clone`. Enables cheap sharing with background servers without changing any of the 150+ existing Tauri commands.
- Tauri commands: `mcp_server_start`, `mcp_server_stop`, `mcp_server_status`, `mcp_regenerate_token`, `get_mcp_activity`
- Control Panel (15.4) and auto-setup writers (15.6) — shipped; see the completion log for details.

### 🧠 MCP Quick Setup

For coding agents, use the MCP tray/coding-agent runtime so it cannot collide with the desktop app or dev server:

1. Copilot cloud sessions auto-start the MCP coding-agent brain via `.github/workflows/copilot-setup-steps.yml` and `scripts/copilot-start-mcp.mjs`; local agents can start it manually with `npm run mcp`.
2. If MCP/app startup fails because `pkg-config` cannot find Linux Tauri libraries, install the missing system packages and retry before reporting a blocker. On Ubuntu the expected set is `libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libsoup-3.0-dev libjavascriptcoregtk-4.1-dev pkg-config`.
3. Copy the generated bearer token from `.vscode/.mcp-token`. The runner also keeps its data-root copy at `mcp-data/mcp-token.txt`.
4. Set VS Code's MCP env var for the `terransoul-brain-mcp` profile, then restart VS Code or launch it from that shell:

  ```powershell
  $env:TERRANSOUL_MCP_TOKEN_MCP = Get-Content .vscode/.mcp-token
  ```

5. Verify the server is reachable with `GET http://127.0.0.1:7423/health`, then call the `brain_health` MCP tool on `terransoul-brain-mcp`.
6. Mandatory for every coding-agent session: use `brain_search` / `brain_suggest_context` for the current chunk before broad manual repo exploration, and write durable self-improve lessons into `mcp-data/shared/migrations/` so the next `npm run mcp` run inherits them.
7. The Self-Improve panel's Finished / Working on / Backlog lanes are backed by `get_self_improve_workboard`, a persisted backend snapshot sourced from `rules/milestones.md`, `rules/completion-log.md`, and the self-improve run log. In MCP mode, `self_improve_runs.jsonl`, `self_improve_gates.jsonl`, and `self_improve_mcp.jsonl` live in `mcp-data/` and rotate to one `.001` archive at 1 MiB.

The checked-in `.vscode/mcp.json` already points `terransoul-brain-mcp` at `http://127.0.0.1:7423/mcp` and reads `TERRANSOUL_MCP_TOKEN_MCP` for bearer auth. Release and dev app profiles use ports `7421` and `7422` with their own token env vars.
The default `mcp-data/shared/` seed includes high-priority rule memories for milestone hygiene, backlog promotion, instruction sync, docs sync, CREDITS, no-mock production code, LLM decision routing, and validation so agents retrieve these before editing.

### 🖥️ Window Modes
- Standard desktop window
- Transparent always-on-top overlay
- **Pet mode** — compact desktop companion

### 🎵 Audio & Ambience
- Procedural JRPG-style BGM (Crystal Theme, Starlit Village, Eternity)
- Volume control with persistence
- Always-visible play/stop button + expandable track selector

### 🎨 UI Polish
- Animated splash screen (kawaii cat loading)
- Chat export (copy/download)
- Typing indicator
- Mobile-responsive layout
- iOS safe-area-aware mobile tab bar and viewport-fit shell
- Keyboard detector for virtual keyboard handling
- Background selection + custom import

---

## Platform Strategy (One Codebase)

Built on **Tauri 2.0** as a unified shell across desktop + mobile:

| Layer | Technology |
|-------|-----------|
| Backend | Rust (shared) |
| UI Shell | WebView (shared) |
| Frontend | Vue 3 + TypeScript (shared) |
| 3D Rendering | Three.js with WebGL2 |

**Platform notes:**

- **Desktop:** transparent always-on-top overlay window + system tray
- **Browser:** Vite serves a product landing page first. Because there is no native Tauri shell, the real VRM renderer is mounted as a small forced pet-mode preview in the bottom-right, and switching to app modes opens a compact responsive in-page window instead of a native window. Pinia stores use browser-safe fallbacks: direct free/paid cloud chat, in-memory/localStorage settings, disabled native-only commands, and an explicit known-host `RemoteHost` bridge for desktop-local LLM or memory capabilities. Automatic LAN discovery is intentionally not claimed in browser mode; it needs native help or a backend rendezvous service.
- **iOS:** full-screen Tauri WebView with `tauri.ios.conf.json`, shared Vue frontend, safe-area navigation, Stronghold-secured pairing credential storage, and gRPC-Web `RemoteHost` transport for paired desktop control. `ChatView` selects `remote-conversation.ts` on iOS so chat streams from the paired desktop brain instead of the phone-local store. Local notifications use `tauri-plugin-notification` while paired to report long-running workflow, ingest, and Copilot activity.
- **Android:** planned follow-up target using the same shared frontend and Rust core
- **Mobile backlog:** background sync later

---

## Core Products (What Users See)

### A) Chat + 3D + Voice Assistant

A single screen showing:

- 🎭 3D character viewport (VRM model with lip sync + expressions)
- 💬 Chat message list with streaming responses
- ⌨️ Text input bar (+ voice input via ASR)
- 🤖 Agent selector ("auto" or choose agent)
- 🎵 Ambient BGM player
- 🎮 Skill tree / quest progression system
- ⚙️ Settings panel (model, background, camera)

### B) RPG Brain Configuration (Quest-Driven Setup)

Instead of traditional setup wizards, TerranSoul guides you through configuration via quests:

- **"Awaken the Mind" quest** → connects your first LLM brain (free, zero config)
- **"Gift of Speech" quest** → enables TTS so your character speaks aloud
- **"Voice Command" quest** → enables ASR so you can talk to your companion
- **"Superior Intellect" quest** → upgrades to a paid API for better responses
- **"Inner Sanctum" quest** → sets up a local LLM for offline + private operation

Each quest teaches you what the feature does, walks you through setup, and rewards you with stat boosts and potential combo unlocks.

### C) Settings / Management

- **Agents:** install · update · remove · start · stop
- **Characters:** import VRM · select built-ins
- **Memory:** view graph · extract · summarize
- **Link devices:** pair + list devices + permissions + remote control

---

## High-Level Architecture (Per Device)

TerranSoul App (on each device) is a **Tauri 2.0** application:

```
┌─────────────────────────────────────────────────────┐
│  Frontend (WebView — Vue 3 + TypeScript)            │
│  ├── 3D Character Viewport (Three.js + VRM)         │
│  │   ├── AvatarStateMachine (expression-driven)     │
│  │   ├── 5-Channel LipSync (FFT via Web Worker)     │
│  │   └── Gesture Blender (MANN-inspired)            │
│  ├── Chat UI (messages, streaming, typing indicator)│
│  ├── Skill Tree / Quest Board (License Board UI)    │
│  ├── Setup Wizards (Brain, Voice)                   │
│  ├── Memory Graph (Cytoscape.js)                    │
│  ├── Pet Overlay / Window Modes / Browser Landing   │
│  └── BGM Player (procedural Web Audio)              │
├─────────────────────────────────────────────────────┤
│  Pinia Stores (state management)                    │
│  ├── brain, conversation, streaming                 │
│  ├── character, identity, memory                    │
│  ├── skill-tree, voice, settings                    │
│  ├── link, sync, messaging, routing                 │
│  └── package, sandbox, provider-health, window      │
├─────────────────────────────────────────────────────┤
│  Rust Core Engine                                   │
│  ├── Brain (Ollama/OpenAI/Anthropic/Free API)       │
│  ├── AI Package Manager                             │
│  ├── Agent Orchestrator + Routing                   │
│  ├── Memory (long-term + short-term)                │
│  ├── TTS (Web Speech / OpenAI TTS)                  │
│  ├── TerranSoul Link (cross-device sync + pairing)  │
│  ├── Messaging (pub/sub IPC)                        │
│  └── Sandbox (WASM agent isolation)                 │
├─────────────────────────────────────────────────────┤
│  AI Agents (separate processes or services)         │
│  ├── Local LLM runtimes (Ollama)                    │
│  ├── Remote API providers (OpenAI, Anthropic, Groq) │
│  └── Community integrations                         │
└─────────────────────────────────────────────────────┘
```

When the same Vue bundle runs in a normal browser, it does not boot the
full-screen desktop shell by default. `App.vue` detects that Tauri IPC is
unavailable, prepares the browser-safe provider prompt, and renders the
landing page with the live TerranSoul model anchored as a small pet preview.
Desktop and chat modes remain available for testing through a small responsive
in-page app window; native-only actions gracefully no-op or show browser
fallbacks. Browser chat asks for OpenRouter, Gemini, NVIDIA, ChatGPT/OpenAI, or
Pollinations with a user-owned key/token and selected model, then talks directly to browser-safe
free/paid cloud providers;
desktop-local Ollama, LM Studio, and Rust-backed memory require an
explicit RemoteHost pairing or known-host LAN path. The Vercel page cannot
auto-discover all LAN hosts without a backend, browser extension, or native
helper; it can only try a host the user supplies and only when browser TLS,
mixed-content, CORS, and private-network rules allow the request. The landing surface has focused regression
coverage for content anchors, live pet companion wiring, and browser-window
launch events.

---

## Development Status

**Completed phases:** 0–14 foundation/partial, Phase 15 partially shipped (15.1 MCP server, 15.3 BrainGateway), Phases 16–18 partially shipped (RAG, memory intelligence, categorisation)
**Test suite:** 1164 frontend (Vitest) + 1075 backend (cargo) + 4 E2E (Playwright) — all passing
**Current focus:** Phase 15 AI Coding Integrations (gRPC, Control Panel), Phase 16 Modern RAG, Phase 17 Brain Intelligence
**See:** `rules/milestones.md` for active chunks and `rules/backlog.md` for deferred work

See [rules/milestones.md](rules/milestones.md) for upcoming work and [rules/completion-log.md](rules/completion-log.md) for the detailed record of all completed work.

---

## 3D Character System

| Property | Choice |
|----------|--------|
| Primary avatar format | **VRM 1.0 & 0.x** |
| Rendering | Three.js WebGL2 + `@pixiv/three-vrm` |
| Animation | AvatarStateMachine (expression-driven, exponential damping) |
| Lip Sync | 5-channel FFT viseme analysis via Web Worker |
| Gestures | MANN-inspired procedural gesture blending |

**Key features:**

- Standard humanoid skeleton with spring bone physics (hair/clothes)
- Facial expressions via BlendShapes (Aa, Ih, Ou, Ee, Oh for speech + emotion blends)
- Natural relaxed pose on load (not T-pose), spring bone warmup for physics settling
- Camera auto-framing per model height
- On-demand rendering: ~15 FPS when idle, 60 FPS on animation
- Placeholder geometric character fallback on load failure
- Persistent camera state (azimuth + distance)

---

## Chat System

**Conversation model:**

```ts
interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  agent_name?: string;
  timestamp: number;
}
```

**Features:**

- Message list with streaming token display
- Typing/thinking indicator
- Agent badge per assistant message
- "Auto agent" routing via conversation router
- Emotion detection from responses (happy, sad, thinking, etc.)
- Chat export (copy to clipboard / download)
- Streaming timeouts (60s streaming, 30s fallback) to prevent stuck states
- Persona-based fallback when no LLM brain is configured

---

## AI Package Manager

**Goal:** Install/manage AI agents as packages across devices.

**Core commands:**

```bash
terransoul install <agent-name>
terransoul update <agent-name>
terransoul remove <agent-name>
terransoul list
terransoul start <agent-name>
terransoul stop <agent-name>
```

---

## Tech Stack

| Component | Technology |
|-----------|-----------|
| App Shell | [Tauri 2.0](https://tauri.app/) |
| Frontend | [Vue 3](https://vuejs.org/) + TypeScript |
| State Management | [Pinia](https://pinia.vuejs.org/) |
| 3D Engine | [Three.js](https://threejs.org/) + [@pixiv/three-vrm](https://github.com/pixiv/three-vrm) |
| Build Tool | [Vite](https://vitejs.dev/) |
| Backend | Rust |
| Package Manager | npm (frontend) · Cargo (backend) |

---

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) ≥ 20
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (Windows only)
- VBScript feature enabled (Windows only)

> **Quick setup with Copilot:** Open GitHub Copilot Chat and type `/setup-prerequisites` to automatically check and install all prerequisites.

### Development

```bash
# Install frontend dependencies
npm install

# Run in development mode (Tauri + Vite dev server)
npm run tauri dev
```

### Build

```bash
# Build the frontend
npm run build

# Build the Tauri app
npm run tauri build
```

---

## Project Structure

```
TerranSoul/
├── src/                    # Vue 3 frontend
│   ├── components/         # UI components (Chat, Quest, Model, Splash, etc.)
│   ├── views/              # Page-level views (Chat, Brain/Voice setup, Memory, etc.)
│   ├── stores/             # Pinia stores (brain, character, conversation, memory, skill-tree, etc.)
│   ├── composables/        # Reusable composables (ASR, TTS, BGM, lip-sync, hotwords, etc.)
│   ├── renderer/           # Three.js rendering (scene, VRM, animator, lip-sync, gestures)
│   ├── config/             # Configuration (default models, gender voices)
│   ├── utils/              # Utilities (API client, emotion parser, VAD, markdown)
│   ├── workers/            # Web Workers (audio analysis)
│   └── types/              # TypeScript type definitions
├── src-tauri/              # Rust backend (Tauri)
│   └── src/
│       ├── brain/          # LLM integration (Ollama, OpenAI, Anthropic, free APIs)
│       ├── memory/         # Long-term + short-term memory
│       ├── messaging/      # Pub/sub IPC messaging
│       ├── routing/        # Agent routing
│       ├── sandbox/        # WASM agent sandbox
│       ├── commands/       # Tauri IPC commands (60+)
│       ├── lib.rs
│       └── main.rs
├── scripts/                # Build scripts (BGM generation, etc.)
├── rules/                  # Architecture, coding standards, milestones, completion log
├── instructions/           # User-facing docs (extending, importing models)
├── e2e/                    # Playwright end-to-end tests
├── public/                 # Static assets (models, backgrounds, audio)
├── .github/workflows/      # CI/CD pipelines
├── package.json
├── vite.config.ts
├── vitest.config.ts
└── tsconfig.json
```

---

## Contributing

This project is in its **earliest stages**. We welcome contributors of all skill levels!

1. Join the discussion on [Discord](https://discord.gg/RzXcvsabKD)
2. Fork the repository
3. Create a feature branch (`git checkout -b feature/amazing-feature`)
4. Commit your changes (`git commit -m 'Add amazing feature'`)
5. Push to the branch (`git push origin feature/amazing-feature`)
6. Open a Pull Request

---

## License

Licensed under the [MIT License](./LICENSE).
