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
| Prefrontal Cortex          | Reasoning Engine (LLM + Agents)    | 🧠 Intelligence      |
| Hippocampus                | Long-term Memory                   | 📖 Wisdom            |
| Working Memory Network     | Short-term Memory                  | 🎯 Focus             |
| Neocortex                  | Retrieval System (RAG / Knowledge) | 📚 Knowledge         |
| Basal Ganglia / Cerebellum | Control & Execution Layer          | ⚡ Dexterity         |
| Sense of Self / Mirror Neurons | Persona & Self-Learning Animation | 🎭 Charisma       |

> 📖 **Deep dive:** every cell in this table — the LLM providers behind Intelligence, the three-tier store and embedding model behind Wisdom, the hybrid 6-signal RAG behind Knowledge, the typed entity-relationship graph, decay/GC, cognitive episodic/semantic/procedural axes, multi-source ingestion, sleep-time consolidation, and the April 2026 research survey — is documented in **[docs/brain-advanced-design.md](docs/brain-advanced-design.md)**. Any contribution that touches the brain (LLM, memory, RAG, ingestion, embeddings, cognitive-kind, brain-gating quests) must consult that doc first. The Charisma row — persona traits, the master-mirror self-learning loop, ARKit-blendshape → VRM expression mapping, MediaPipe FaceLandmarker / PoseLandmarker, and the per-session camera consent contract — is documented in **[docs/persona-design.md](docs/persona-design.md)**, which any persona/animation contribution must consult first.

> 🖥️ **Offline / Local LLM models (current):** TerranSoul's hardware-adaptive model recommender selects from **Gemma 4** and **Qwen** families for fully offline, private operation via Ollama — chosen based on your available RAM. The full catalogue is maintained in [docs/brain-advanced-design.md §26](docs/brain-advanced-design.md#recommended-local-llm-catalogue) and ships with every release build, so the app always has the latest recommendations.

As you unlock skills, your AI's stats grow. A freshly installed TerranSoul starts at level 1 with just a free cloud brain. By the time you've completed the Ultimate tier, you have a fully autonomous assistant with voice, vision, memory, multi-device sync, and community agents — all configured through gameplay, not menus.

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
🧠 Free Brain (Pollinations/Groq)
├── ⚡ Superior Intellect (Paid API — OpenAI/Anthropic)
│   ├── 🤖 Agent Summoning (community AI agents)
│   ├── 🌍 Babel Tongue (real-time translation)
│   └── 📸 All-Seeing Eye (screen vision)
└── 🏰 Inner Sanctum (Local LLM via llmfit)
    └── Full offline operation — no internet needed
```

Each path is a quest chain. The free brain auto-configures on first launch (zero setup). From there, you choose: pay for power (Superior Intellect), or invest time in local setup for privacy and offline capability (Inner Sanctum).

---

## Vision

TerranSoul is an open-source **3D virtual assistant + AI package manager** that runs across:

| Platform | Target |
|----------|--------|
| Desktop | Windows · macOS · Linux |
| Mobile | iOS · Android |

TerranSoul includes a **TerranSoul Link** layer that securely connects all your devices so you can:

- 💬 Chat with TerranSoul anywhere
- 🔄 Sync conversations and settings across devices
- 🖥️ Control other devices remotely (send commands to run on your PC from your phone)
- 🤖 Orchestrate multiple AI agents (OpenClaw, Claude Cowork, etc.)

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

> Architectural reference: **[docs/brain-advanced-design.md](docs/brain-advanced-design.md)** covers every component below in depth, including the April 2026 modern-RAG research survey.

**Providers & modes** (`src-tauri/src/brain/`)
- **4 brain modes:** Free API (Pollinations, Groq), Paid API (OpenAI / Anthropic / Groq / OpenAI-compatible), Local Ollama, Stub fallback
- Implementations: `OllamaAgent`, `OpenAiClient`, `FreeProvider`, `ProviderRotator`, `StubAgent`
- Cloud embedding API (`cloud_embeddings.rs`) — unified `embed_for_mode` dispatcher routes to OpenAI-compatible `/v1/embeddings` for paid/free cloud modes, so vector RAG works without local Ollama
- External CLI agents (Chunk 1.5): multi-agent **roster** + Temporal-style **durable workflow engine** (`src-tauri/src/agents/`, `src-tauri/src/workflows/`)
- Hardware-adaptive **model recommender** (Gemma 4, Phi-4, Kimi K2.6 cloud) based on detected RAM — catalogue defined in [docs/brain-advanced-design.md §26](docs/brain-advanced-design.md#recommended-local-llm-catalogue) (single source of truth), parsed at runtime from bundled docs, with hardcoded fallback
- Zero-setup first launch — free brain auto-configures with no API keys
- Streaming responses (SSE → `llm-chunk` Tauri event, parsed by `StreamTagParser` state machine)
- Animation channel: `llm-animation` events for `<anim>` JSON blocks emitted by the LLM
- Provider health monitoring + automatic failover, migration detection when APIs deprecate
- Chat-based LLM switching ("switch to groq", "use pollinations")
- Persona-based fallback when no LLM is configured
- **LLM-powered intent classifier** (`src-tauri/src/brain/intent_classifier.rs` + `classify_intent` Tauri command) — every chat turn is classified by the configured brain (Free → Paid → Local) into a typed `IntentDecision` (`chat`, `learn_with_docs{topic}`, `teach_ingest{topic}`, `gated_setup{upgrade_gemini|provide_context}`, `unknown`). Replaces three brittle English-only regex detectors so paraphrases, typos and multilingual phrasings (`học luật Việt Nam từ tài liệu của tôi`) all route correctly. 3 s hard timeout + 30 s in-memory LRU cache; on `unknown` the frontend automatically triggers the install-all overlay so a local Ollama brain is set up — guaranteeing every future turn has a working classifier offline. **Every "LLM decides" surface (intent classifier, unknown→install fallback, don't-know gate, post-reply quest suggestions, chat-based LLM-switching commands, yes/no quick-reply buttons, model-capacity auto-upgrade prompt) is user-toggleable** from the Brain panel's "🧭 AI decision-making" section. New code touching AI routing must follow **[`rules/llm-decision-rules.md`](rules/llm-decision-rules.md)** — no regex / `.includes` / keyword arrays driving AI behaviour. See **[docs/brain-advanced-design.md § Intent Classification](docs/brain-advanced-design.md#intent-classification)**.
- 60s streaming timeout + 30s fallback timeout to prevent stuck states

**Three-tier memory + RAG** (`src-tauri/src/memory/`)
- **Three tiers** mirroring human cognition: **Short-term** (in-memory `Vec<Message>`, last ~20 turns) → **Working** (SQLite, session-scoped) → **Long-term** (SQLite, vector-indexed, decay/GC managed)
- **Cognitive memory axes** — every memory is also classified `episodic` / `semantic` / `procedural` via the pure-function classifier `memory::cognitive_kind::classify` (mirrored 1:1 in TS at `src/utils/cognitive-kind.ts`)
- **Hybrid 6-signal RAG search** — `vector_similarity` (40%) + `keyword_match` (20%) + `recency_bias` (15%) + `importance` (10%) + `decay_score` (10%) + `tier_priority` (5%)
- **Embeddings:** Ollama `nomic-embed-text` (768-dim) by default, stored as SQLite BLOB; chat-model fallback with process-lifetime "unsupported" cache + 60s `/api/tags` probe cache
- **Reciprocal Rank Fusion (RRF)** — `memory/fusion.rs` ships the Cormack RRF utility (`k=60`) and `MemoryStore::hybrid_search_rrf` wires it into a real retrieval path that fuses independent vector / keyword / freshness rankings; exposed as the `hybrid_search_memories_rrf` Tauri command (April 2026 research absorption — `docs/brain-advanced-design.md` §19.2 row 2)
- **HyDE — Hypothetical Document Embeddings** (`memory/hyde.rs` + `OllamaAgent::hyde_complete` + `hyde_search_memories` Tauri command) — LLM writes a plausible 1-3 sentence answer, we embed *that* for retrieval; falls back gracefully to raw-query embedding then to keyword + freshness when the brain is unreachable. Improves recall on cold or abstract queries (Gao et al., 2022 — `docs/brain-advanced-design.md` §19.2 row 4)
- **Cross-encoder reranker (LLM-as-judge style)** (`memory/reranker.rs` + `OllamaAgent::rerank_score` + `rerank_search_memories` Tauri command) — two-stage retrieval: RRF-fused recall (default `candidates_k=20`, clamped `limit..=50`) → active brain scores each `(query, document)` pair on a 0–10 scale → reorder. Unscored candidates are kept below scored ones (no silent recall loss). Reuses the active brain (no extra model download); interface matches a future BGE/mxbai backend so swapping is one line (`docs/brain-advanced-design.md` §19.2 row 10)
- **Matryoshka two-stage vector search** (`memory/matryoshka.rs` + `matryoshka_search_memories` Tauri command) — truncate the query embedding to 256 dims for a fast first pass, then re-rank the top survivors at full 768-dim cosine similarity. Pure utility (`truncate_and_normalize` + `two_stage_search`) — no schema change, no migration, no index rebuild. Cuts the brute-force fallback path ~3× per candidate with negligible recall hit; helps cold-start before the ANN index is hot (Kusupati et al., NeurIPS 2022 — `docs/brain-advanced-design.md` §19.2 row 11)
- **Self-RAG reflection-token controller** (`orchestrator/self_rag.rs`) — pure decision logic for the Self-RAG iterative-refinement protocol (Asai et al., 2023). Parses `<Retrieve>` / `<Relevant>` / `<Supported>` / `<Useful>` reflection tokens out of LLM responses and runs a 3-iteration state machine that decides whether to retrieve again, accept the answer, or refuse on max-iter / unsupported. Ships with `SELF_RAG_SYSTEM_PROMPT` for prompt injection. Orchestrator-loop integration (re-prompting the LLM with augmented context) is the follow-up Chunk 16.4b — `docs/brain-advanced-design.md` §19.2 row 5
- **Context budgeter** (`brain/context_budget.rs`) — pure module that owns the *token budget split* across persona / history / retrieval / tool-schema sections of every prompt. Three preset configs (`for_free_mode` / `for_paid_mode` / `for_local_mode`), `fit(inputs, config)` performs in-priority-order truncation (drop lowest-score retrieval first, oldest history second, tools wholesale last; persona is never truncated), and `BudgetStats` provides an audit trail. Replaces ad-hoc concatenation in chat with a single deterministic, testable function — 14 unit tests (Chunk 27.2; `commands::chat` wiring is the follow-up). `docs/brain-advanced-design.md` §19.2 row 15
- **Topic segmenter** (`brain/segmenter.rs`) — pure module that splits a conversation transcript into topic-coherent ranges via local-maxima cosine-distance peak detection on per-turn embeddings (`mean + k·stddev` threshold, default `k = 1.0`), then merges segments shorter than `min_segment_size` (default 2) into their previous neighbour. Returns `Vec<TopicSegment { start, end, summary }>` with auto-derived 80-char summaries. Foundation for per-topic LLM fact extraction in place of the current single-blob extractor — 18 unit tests (Chunk 26.2; the `extract_facts` wiring is Chunk 26.2b). `docs/brain-advanced-design.md` §21.7
- **Background-maintenance scheduler decision logic** (`brain/maintenance_scheduler.rs`) — pure module that decides which of the four brain-maintenance jobs (`Decay`, `GarbageCollect`, `PromoteTier`, `EdgeExtract`) are due to fire on a given tick, given persisted last-run timestamps and per-job cool-downs (default 23h). `jobs_due(state, config, now_ms)` returns jobs in canonical cheap→expensive order so pure-SQL jobs always run before LLM jobs. `jitter_ms(seed, max)` gives deterministic-per-device spread to avoid thundering-herd. 17 unit tests (Chunk 26.1). `docs/brain-advanced-design.md` §21.7
- **Background-maintenance runtime** (`brain/maintenance_runtime.rs`) — closes the loop on Chunk 26.1 with the `tokio::time::interval` tick wrapper, atomic JSON persistence of `MaintenanceState` to `<data_dir>/maintenance_state.json` (temp-file + rename, crash-safe), and direct dispatch into the underlying primitives (`apply_decay`, `gc_decayed`, `auto_promote_to_long`, edge extraction). Spawned from `lib.rs::setup` so the loop starts at app launch. 6 unit tests covering disk round-trip, corrupt-state fallback, and atomic-write cleanup (Chunk 26.1b). `docs/brain-advanced-design.md` §21.7
- **Replay-from-history backfill** (`memory/replay.rs` + `replay_extract_history` command) — walks every persisted `MemoryType::Summary`, feeds each through the segmented extractor (`extract_facts_segmented_any_mode`) used by live chats, and saves the new facts. `dry_run: true` previews the count without persisting. `since_timestamp_ms` and `max_summaries` filters scope the replay. Emits `brain-replay-progress` events for the BrainView "Replay history →" button. Backfills memories that were created before Chunk 26.2 auto-learn existed. 11 unit tests on the pure selection/progress module (Chunk 26.4). `docs/brain-advanced-design.md` §21.7
- **Late chunking pooling utility** (`memory/late_chunking.rs`) — pure mean-pool + L2-renormalise utility for the Jina AI 2024 late-chunking technique. `mean_pool_token_embeddings(token_embeddings, span)` averages a span of per-token embeddings emitted by a long-context embedder, then renormalises so the result is directly comparable via cosine similarity to other unit-norm embeddings in the store; `pool_chunks(...)` applies it across a chunk list in one pass; `spans_from_token_counts(...)` builds the contiguous span vector. Refuses dimensionality drift, empty spans, and zero-norm means rather than silently degrading. Ingest-pipeline integration that flips `embed_text` from per-chunk to per-token mode is the follow-up Chunk 16.3b (gated on a long-context Ollama embedder) — `docs/brain-advanced-design.md` §19.2 row 9
- **CRAG retrieval evaluator** (`memory/crag.rs`) — pure classifier for the Corrective RAG protocol (Yan et al., 2024). `build_evaluator_prompts(query, document)` mirrors the reranker shape; `parse_verdict()` extracts `CORRECT` / `AMBIGUOUS` / `INCORRECT` from LLM replies with whole-word token matching (so `"incorrectly"` doesn't false-match `INCORRECT`); `aggregate()` collapses per-document verdicts into a corpus-level `RetrievalQuality` for orchestrator branching. Query-rewrite + web-search fallback is the follow-up Chunk 16.5b — `docs/brain-advanced-design.md` §19.2 row 6
- **Temporal knowledge graph (V6)** — `memory_edges` gains nullable `valid_from` (inclusive) and `valid_to` (exclusive) Unix-ms columns. `MemoryEdge::is_valid_at(t)` is the pure interval predicate; `MemoryStore::get_edges_for_at(memory_id, direction, valid_at)` is the point-in-time query (legacy callers passing `valid_at = None` see identical behaviour to V5); `MemoryStore::close_edge(id, t)` records supersession. New `close_memory_edge` Tauri command + `valid_from` / `valid_to` parameters on `add_memory_edge`. Implements the Zep / Graphiti pattern (2024) — `docs/brain-advanced-design.md` §19.2 row 13
- **Knowledge graph (V6):** typed directional `memory_edges` table with FK cascade + 17-type relationship taxonomy + temporal validity intervals, `extract_edges_via_brain` LLM extractor (auto-fires after every successful `extract_memories_from_session` when `auto_extract_edges` is enabled — Chunk 26.3), `multi_hop_search_memories` traversal, Cytoscape.js visualization
- **Decay & GC:** exponential decay (`decay_score *= 0.95^(hours/168)`), access-count tracking, periodic garbage collection
- **Multi-source knowledge management:** source-hash change detection, TTL expiry, access-count decay, **LLM-powered conflict resolution**
- **Pluggable storage backends** via `StorageBackend` trait: SQLite (default), PostgreSQL (`sqlx`), SQL Server (`tiberius`), CassandraDB (`scylla`)
- **LLM-powered memory ops:** `extract_facts`, `summarize`, `semantic_search_entries`, `embed_text`
- **SQLite schema at V6** with `PRAGMA foreign_keys=ON` and auto-migrating `memories` + `memory_edges` tables (V6 adds temporal `valid_from` / `valid_to` columns to edges)

**Frontend brain hub** (`src/views/BrainView.vue`, `src/components/BrainAvatar.vue`)
- Top-level **Brain** tab unifies brain config, hardware probe, RAG capability gauges, cognitive-kind breakdown, RPG stats, and a mini memory graph
- **Active Selection panel** — surfaces the typed `BrainSelection` snapshot (provider · embedding · memory · search · storage · agents · effective RAG quality %) so the user sees, at a glance, exactly *which* component is answering, ranking, embedding, and storing for them. Backed by the `get_brain_selection` Tauri command.
- **Brain component selection & routing** — every routing decision (provider mode, embedding model, memory tier, search method, RAG injection top-k & threshold, agent dispatch, cognitive-kind classification, storage backend, fallback chains) is documented in **[docs/brain-advanced-design.md § 20](docs/brain-advanced-design.md#brain-component-selection--routing--how-the-llm-knows-what-to-use)**
- **Daily-conversation write-back loop** — every chat turn lands instantly in short-term memory; the **`memory::auto_learn`** policy (default: every 10 turns, 3-turn cooldown) decides when to fire `extract_memories_from_session` automatically. Tunable per-user via `get_auto_learn_policy` / `set_auto_learn_policy` and previewed live via the pure `evaluate_auto_learn` decision query. Full loop documented in **[docs/brain-advanced-design.md § 21](docs/brain-advanced-design.md#how-daily-conversation-updates-the-brain--write-back--learning-loop)**
- **Code-intelligence bridge — GitNexus sidecar (Tier 1)** — out-of-process MCP/JSON-RPC bridge to the upstream `gitnexus` server (`abhigyanpatwari/GitNexus`, **PolyForm-Noncommercial-1.0.0** — never bundled). Exposes four read-only tools (`gitnexus_query`, `gitnexus_context`, `gitnexus_impact`, `gitnexus_detect_changes`) as Tauri commands gated by the new `code_intelligence` capability. Implementation: `src-tauri/src/agent/gitnexus_sidecar.rs` (transport-agnostic bridge with `tokio::process` stdio transport + in-memory mock for tests) and `src-tauri/src/commands/gitnexus.rs`. Documented in **[docs/brain-advanced-design.md § 22](docs/brain-advanced-design.md)**.
- **Code-RAG fusion in `rerank_search_memories` (Tier 2)** — when the GitNexus sidecar is configured + the `code_intelligence` capability is granted, the recall stage of `rerank_search_memories` also dispatches the user query to GitNexus, normalises the JSON response into pseudo-`MemoryEntry` records (negative ids, tier=`Working`, type=`Context`, tag `code:gitnexus`), and RRF-fuses them with the SQLite candidate set via the existing `memory::fusion::reciprocal_rank_fuse` (k=60). Failures degrade silently to DB-only recall — code intelligence augments, never gates. Implementation: `src-tauri/src/memory/code_rag.rs` (pure shape-tolerant normaliser supporting 5 known GitNexus response shapes + 5 field aliases). Documented in **[docs/brain-advanced-design.md § 23](docs/brain-advanced-design.md)**.
- **Code-knowledge-graph mirror (Tier 3, V7)** — opt-in `gitnexus_sync` Tauri command pulls the structured KG from the GitNexus `graph` MCP tool, maps the upstream `CONTAINS` / `CALLS` / `IMPORTS` / `EXTENDS` / `HANDLES_ROUTE` relations into the existing 17-relation taxonomy (`contains`, `depends_on`, `derived_from`, `governs`, …), and persists every edge with `edge_source = 'gitnexus:<scope>'` so the rest of the brain (`hybrid_search_with_graph`, BrainView graph panel, multi-hop RAG) can reason over code structure alongside free-text memories. Idempotent — re-running the same scope reuses memories via the existing `source_hash` dedup index. `gitnexus_unmirror` rolls back exactly one mirror without touching native or LLM-extracted edges. V7 SQLite migration adds the `edge_source` TEXT column + `idx_edges_edge_source` index. Implementation: `src-tauri/src/memory/gitnexus_mirror.rs`. Strictly opt-in: never runs at startup. Documented in **[docs/brain-advanced-design.md § 8 (V7)](docs/brain-advanced-design.md)**.
- **BrainView "Code knowledge" panel (Tier 4)** — `src/components/CodeKnowledgePanel.vue` surfaces the GitNexus pipeline in the Brain hub: a sync form for `repo:owner/name@sha` scopes, a list of every mirrored repo (edge count + last-sync time, ordered most-recent-first) with a per-row "Unmirror" button, and a blast-radius pre-flight that runs `gitnexus_impact` on a symbol you're about to change. Powered by the new `gitnexus_list_mirrors` Tauri command (which aggregates `memory_edges` by `edge_source` LIKE `gitnexus:%`). Pure frontend wiring on top of Chunks 2.1 + 2.3.
- Pinia stores: `brain.ts`, `conversation.ts`, `memory.ts`, `agent-roster.ts`, `skill-tree.ts`

### 🗣️ Voice System (The "Charisma" Stats)
- **ASR:** Web Speech API (browser-native), Whisper, Groq speech-to-text
- **TTS:** Web Speech (browser SpeechSynthesis — free, offline-capable, no third-party endpoint), OpenAI TTS (optional, requires API key)
- **Hotword detection** for wake-word activation
- **Speaker diarization** support
- LipSync ↔ TTS audio pipeline for real-time mouth animation
- **Translator mode plugin** — say “become a translator to help me translate between English and Vietnamese” to activate the built-in `terransoul-translator` reference plugin. Translator mode uses a free cloud LLM with an API key (for example Groq/Cerebras) or a local LLM (Local Ollama / LM Studio); the no-key Pollinations fallback is not used for translator mode. TerranSoul directly translates each spoken ASR transcript or typed turn to the other person’s language, alternating direction until “stop translator mode”. The implementation doubles as the documented example for new chat-mode plugins in [docs/plugin-development.md](docs/plugin-development.md#translator-mode-reference-plugin).
- **Audio-prosody persona hints** — when ASR is configured, the Master-Echo
  persona suggester analyses the user's typed turns (which mirror their spoken
  patterns) for tone (concise / elaborate / energetic / inquisitive / emphatic
  / playful), pacing (fast / measured / slow), and quirks (filler words, emoji
  use), and folds those hints into the persona-extraction prompt.
  Camera-free; raw audio is never read; hints are never persisted.

### 💾 Memory System (The "Hippocampus")

> Architectural reference: **[docs/brain-advanced-design.md](docs/brain-advanced-design.md)** — full schema, RAG pipeline, decay model, knowledge graph, and April 2026 research survey.

**Core modules** (`src-tauri/src/memory/`)
- `store.rs` — `MemoryStore` (default SQLite + WAL, schema **V8**), `MemoryEntry`, `MemoryTier` (short / working / long), `MemoryType`, `NewMemory`, `MemoryUpdate`, `MemoryStats`, `cosine_similarity`, `bytes_to_embedding` / `embedding_to_bytes`, `hybrid_search` (6-signal weighted-sum scoring), `hybrid_search_rrf` (RRF-fused vector + keyword + freshness retrievers), `apply_decay`, `promote`
- `backend.rs` — `StorageBackend` trait + `StorageConfig` + `StorageError` (the seam every backend implements)
- `migrations.rs` — auto-applied schema migrations through V8 (`memories` + `memory_edges` with temporal `valid_from` / `valid_to` columns + V7 `edge_source` external-KG provenance + V8 `memory_versions` edit history table, `PRAGMA foreign_keys=ON`)
- `edges.rs` — `MemoryEdge`, `NewMemoryEdge` (with V7 `edge_source: Option<String>`), `EdgeDirection`, `EdgeSource`, `EdgeStats`, `COMMON_RELATION_TYPES` (17-type taxonomy), `normalise_rel_type`, `parse_llm_edges`, `format_memories_for_extraction`, `delete_edges_by_edge_source`
- `gitnexus_mirror.rs` — Phase 13 Tier 3 mapper: GitNexus `CONTAINS`/`CALLS`/`IMPORTS`/`EXTENDS`/`HANDLES_ROUTE` → 17-relation taxonomy; `mirror_kg(store, scope, payload)` / `unmirror(store, scope)` pure functions; idempotent across re-syncs (uses `source_hash` for node dedup)
- `cognitive_kind.rs` — pure-function `classify(memory_type, tags, content) → CognitiveKind` (`Episodic` / `Semantic` / `Procedural`); mirrored 1:1 in TS at `src/utils/cognitive-kind.ts`
- `brain_memory.rs` — LLM-powered ops: `extract_facts`, `summarize`, `semantic_search_entries`, `extract_edges_via_brain`
- `fusion.rs` — `reciprocal_rank_fuse(rankings, k)` (Cormack RRF, `k=60`); consumed by `MemoryStore::hybrid_search_rrf`
- `hyde.rs` — pure HyDE prompt builder + reply cleaner (Gao et al., 2022); consumed by `OllamaAgent::hyde_complete` and the `hyde_search_memories` Tauri command
- `reranker.rs` — pure cross-encoder rerank prompt builder + score parser + reorder logic; consumed by `OllamaAgent::rerank_score` and the two-stage `rerank_search_memories` Tauri command (recall via `hybrid_search_rrf`, precision via LLM-as-judge)
- `auto_learn.rs` — `AutoLearnPolicy` + pure `evaluate(policy, total_turns, last_autolearn_turn) → AutoLearnDecision`; the cadence policy that turns daily conversation into long-term memory (default: every 10 turns, 3-turn cooldown). See **[docs/brain-advanced-design.md § 21](docs/brain-advanced-design.md#how-daily-conversation-updates-the-brain--write-back--learning-loop)**.
- `tag_vocabulary.rs` — curated `CURATED_PREFIXES` (`personal`, `domain`, `project`, `tool`, `code`, `external`, `session`, `quest`), `validate()` / `validate_csv()`, `LEGACY_ALLOW_LIST`, `category_decay_multiplier()` per-prefix decay rates
- `auto_tag.rs` — LLM auto-tagger: `auto_tag_content()` dispatches to Ollama/FreeApi/PaidApi; `parse_tag_response()` validates + caps at 4 curated-prefix tags; `merge_tags()` deduplicates with user tags. Opt-in via `AppSettings.auto_tag`
- `obsidian_export.rs` — one-way Obsidian vault export: `export_to_vault(vault_dir, entries)` writes `<id>-<slug>.md` per long-tier memory with YAML frontmatter; idempotent (mtime-based skip); `slugify`, `format_iso`, `render_markdown` pure helpers
- `temporal.rs` — natural-language time-range parser: `parse_time_range(question, now_ms)` resolves "last N days", "since April", "between X and Y", "today", "yesterday" into `TimeRange { start_ms, end_ms }`; pure std calendar math (Howard Hinnant algorithm), no external crate
- `contextualize.rs` — Contextual Retrieval (Anthropic 2024): `generate_doc_summary()` + `contextualise_chunk(doc_summary, chunk, brain_mode)` + `prepend_context()`; opt-in via `AppSettings.contextual_retrieval`; adds document-level context to each chunk before embedding, reducing failed retrievals by ~49 %
- `versioning.rs` — non-destructive edit history: `save_version(conn, memory_id)` snapshots the current state into `memory_versions` (V8 schema) before each update; `get_history()` returns all previous versions; FK cascade on delete
- `chunking.rs` — semantic chunking pipeline: `split_markdown()` (heading/paragraph/sentence-aware via `text-splitter` crate), `split_text()` (Unicode sentence boundaries), `dedup_chunks()` (SHA-256 hash dedup), `Chunk` struct (index, text, hash, heading metadata); replaces naive word-count splitter
- `conflicts.rs` — contradiction resolution (V9 schema): `build_contradiction_prompt` + `parse_contradiction_reply` for LLM-based contradiction check; `MemoryConflict` CRUD (`add_conflict`, `list_conflicts`, `resolve_conflict`, `dismiss_conflict`, `count_open_conflicts`); losers soft-closed via `valid_to` (never deleted)
- `edge_conflict_scan.rs` — scheduled edge conflict detection: `collect_scan_candidates()` (lock-safe candidate collection), `record_contradiction()` (insert contradicts edge + open conflict), scans positive-relation edges for hidden contradictions via LLM-as-judge
- `ann_index.rs` — HNSW approximate nearest neighbor index via `usearch` 2.x: `AnnIndex` (lazy OnceCell init, auto-rebuild from DB, periodic save to `vectors.usearch`), accelerates `vector_search` + `find_duplicate` from O(n) to O(log n)
- Pluggable backends behind cargo features: `postgres.rs` (`sqlx`), `mssql.rs` (`tiberius`), `cassandra.rs` (`scylla`)

**Tauri command surface** (`src-tauri/src/commands/memory.rs`)
- `add_memory`, `update_memory`, `delete_memory`, `get_memories`, `search_memories` (SQL `LIKE`), `semantic_search_memories` (cosine), `hybrid_search_memories` (6-signal weighted-sum), `hybrid_search_memories_rrf` (RRF-fused vector + keyword + freshness), `hyde_search_memories` (HyDE — embed an LLM-written hypothetical answer), `rerank_search_memories` (two-stage RRF-recall + LLM-as-judge cross-encoder rerank), `multi_hop_search_memories` (graph traversal), `temporal_query` (natural-language time-range filter: "last week", "since April", "between X and Y"), `get_relevant_memories`, `get_short_term_memory`, `extract_memories_from_session`, `summarize_session`, `backfill_embeddings`, `apply_memory_decay`, `gc_memories`, `promote_memory`, `get_memories_by_tier`, `get_schema_info`, `get_memory_stats`, `add_memory_edge` (V6: optional `valid_from` / `valid_to`), `close_memory_edge` (V6: record supersession), `delete_memory_edge`, `list_memory_edges`, `get_edges_for_memory`, `get_edge_stats`, `list_relation_types`, `extract_edges_via_brain`, `scan_edge_conflicts` (LLM-as-judge scan over positive-relation edges for hidden contradictions), `gitnexus_sync` (V7: opt-in code-KG mirror), `gitnexus_unmirror` (V7: per-scope rollback), `gitnexus_list_mirrors` (V7: BrainView panel), `get_auto_learn_policy`, `set_auto_learn_policy`, `evaluate_auto_learn`, `export_to_obsidian` (one-way vault export with YAML frontmatter), `get_memory_history` (V8: version history for a memory entry), `adjust_memory_importance` (access-pattern-driven ±1 nudge with version audit trail)

**Storage & RAG**
- **Three tiers** mirroring human cognition: short-term (in-memory `Vec<Message>`, last ~20 turns) → working (SQLite, session-scoped) → long-term (SQLite, vector-indexed, decay/GC managed)
- **Knowledge graph (V8):** typed directional `memory_edges` table with FK cascade + temporal `valid_from` / `valid_to` validity intervals + `edge_source` external-KG provenance (e.g. `gitnexus:<scope>` for the Phase 13 Tier 3 code-KG mirror), 17-type relationship taxonomy, LLM edge extractor, multi-hop traversal; `memory_versions` table for non-destructive edit history
- **Embeddings:** Ollama `nomic-embed-text` (768-dim) stored as SQLite BLOB; chat-model fallback with process-lifetime "unsupported" cache + 60s `/api/tags` probe cache
- **Hybrid 6-signal RAG search** — `vector_similarity` (40%) + `keyword_match` (20%) + `recency_bias` (15%) + `importance` (10%) + `decay_score` (10%) + `tier_priority` (5%)
- **Decay & GC:** exponential decay (`decay_score *= 0.95^(hours/168)`), access-count tracking, periodic garbage collection
- **Multi-source knowledge management:** source-hash change detection, TTL expiry, access-count decay, **LLM-powered conflict resolution**

**Frontend**
- `src/views/MemoryView.vue` — list / grid / graph view, tier chips, tag-prefix category filter chips with counts, search + semantic + hybrid search
- `src/components/MemoryGraph.vue` — Cytoscape.js semantic graph visualization with typed edges
- `src/stores/memory.ts` — Pinia store (CRUD + search + streaming results)

### 🎭 Persona System (The "Sense of Self & Mirror Neurons")

> Architectural reference: **[docs/persona-design.md](docs/persona-design.md)** — full traits schema, the master-mirror self-learning loop, ARKit-blendshape → VRM 1.0 expression mapping, MediaPipe FaceLandmarker / PoseLandmarker pipeline, the per-session camera consent contract, the persona quest chain, and the April 2026 research survey (Hunyuan Motion, MoCha, OmniHuman, ID-Patch, MotionAura).

**Core modules**
- `src-tauri/src/commands/persona.rs` — Tauri persistence: `get_persona`, `save_persona`, `set_persona_block` / `get_persona_block`, `list_/save_/delete_learned_expression`, `list_/save_/delete_learned_motion`, `extract_persona_from_brain` (Master-Echo brain-extraction loop, shipped 2026-04-24), `check_persona_drift` (drift detection comparing active persona vs `personal:*` memories, shipped 2026-04-26), `export_persona_pack` / `preview_persona_pack` / `import_persona_pack` (persona pack export / import, shipped 2026-04-24). Atomic JSON-on-disk under `<app_data_dir>/persona/{persona.json, expressions/, motions/}`. Path-traversal-safe id validation. **No camera commands** — webcam frames never cross the IPC boundary; only user-confirmed JSON landmark artifacts ever reach Rust.
- `src-tauri/src/persona/extract.rs` — pure prompt + parser for Master-Echo: `build_persona_prompt`, `assemble_snippets` (last 30 turns + up to 20 long-tier `personal:*` memories), `parse_persona_reply` (tolerant of markdown fences, leading prose, non-string list entries; bio capped at 500 chars; lists deduped + capped at 6). Same testable-seam shape as `memory/hyde.rs` and `memory/reranker.rs`.
- `src-tauri/src/persona/drift.rs` — pure prompt + parser for persona drift detection: `build_drift_prompt` (persona JSON + personal memories → comparison prompt), `parse_drift_reply` (tolerant of fences/prose), `DriftReport` (detected, summary, suggested_changes), `DriftSuggestion` (field/current/proposed). 14 unit tests.
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
- `src/renderer/vrma-baker.ts` — pure `bakeMotionToClip()` converts LearnedMotion JSON frames into `THREE.AnimationClip` with quaternion keyframe tracks. `bakeAllMotions()` batch-bakes. 12 unit tests.
- `src/renderer/learned-motion-player.ts` — `LearnedMotionPlayer` class wraps bakeMotionToClip + VrmaManager.playClip for on-demand playback of learned motions. `applyLearnedExpression()` / `clearExpressionPreview()` helpers for timed expression preview on VRM. 10 unit tests.
- `src/renderer/phoneme-viseme.ts` — text-driven phoneme-to-viseme mapper: English grapheme tokenizer (15 digraphs + single-char fallback) → 5-channel viseme timeline builder → `VisemeScheduler` with frame-accurate interpolation. Replaces FFT band-energy lip-sync when text + duration are available. 22 unit tests.
- `src/components/PersonaTeacher.vue` — "Teach an Expression / Motion" panel: Expression/Motion tab toggle, consent dialog → live camera preview (CAMERA LIVE badge) → capture pose or record motion (30 fps, max 10s) → name + trigger → save. 5 component tests.

**Frontend**
- `src/components/PersonaPanel.vue` — full add / update / delete / review management UI mounted in the Brain hub (`BrainView.vue`); edits all traits, lists every learned-expression / learned-motion artifact with one-click delete, live-previews the rendered `[PERSONA]` system-prompt block, and includes the "✨ Suggest from my chats" Master-Echo button.
- `src/components/PersonaPackPanel.vue` — sibling card for persona pack export / import (Chunk 14.7): export with optional note + clipboard copy + `.json` download (Blob + `<a download>` — no Tauri `dialog` plugin needed), import with **🔍 Preview** dry-run + **⤴ Apply** + per-entry skip report.
- `src/components/PersonaListEditor.vue` — small chip-style list editor used by the persona panel for `tone` / `quirks` / `avoid` arrays.

### 🔗 TerranSoul Link
- Device identity + pairing with QR codes
- Cross-device conversation sync
- Settings synchronization

### 📦 AI Package Manager
- Install / update / remove / start / stop agents
- Package registry with marketplace UI (browse works out-of-the-box via the in-process catalog registry)
- Local Ollama models surface as marketplace agents — install & activate from the same UI
- WASM sandbox for agent isolation
- See [`instructions/OPENCLAW-EXAMPLE.md`](./instructions/OPENCLAW-EXAMPLE.md) for an end-to-end walkthrough using the OpenClaw bridge agent

### 🤖 AI Coding Integrations (Brain Gateway)

> Architectural reference: **[docs/AI-coding-integrations.md](docs/AI-coding-integrations.md)** — full protocol details, security model, auto-setup writers, and the VS Code Copilot incremental-indexing pact.

- **MCP server** (Chunk 15.1) — HTTP/JSON-RPC 2.0 on `127.0.0.1:7421` via axum Streamable HTTP transport. Bearer-token auth (`mcp-token.txt`). 8 brain tools (`brain_search`, `brain_get_entry`, `brain_list_recent`, `brain_kg_neighbors`, `brain_summarize`, `brain_suggest_context`, `brain_ingest_url`, `brain_health`).
- **BrainGateway trait** (Chunk 15.3) — single typed op surface shared by MCP and future gRPC transports. `AppStateGateway` adapter, capability gating (`GatewayCaps`), typed errors, delta-stable fingerprint for VS Code Copilot cache.
- **AppState Arc newtype** — `AppState(Arc<AppStateInner>)` with `Deref + Clone`. Enables cheap sharing with background servers without changing any of the 150+ existing Tauri commands.
- Tauri commands: `mcp_server_start`, `mcp_server_stop`, `mcp_server_status`, `mcp_regenerate_token`
- gRPC server (Chunk 15.2), Control Panel (15.4), auto-setup writers (15.6) — planned

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
- **Mobile:** full-screen app (or compact mode), push notifications later, background sync later

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
│  ├── Pet Overlay / Window Modes                     │
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
