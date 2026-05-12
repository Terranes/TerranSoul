# TerranSoul — 3D AI Companion with Persistent Memory

> **🚧 Active development since 10/04/2026.**
> Interested? Join us at <https://discord.gg/RzXcvsabKD>

[![CI](https://github.com/Terranes/TerranSoul/actions/workflows/terransoul-ci.yml/badge.svg)](https://github.com/Terranes/TerranSoul/actions/workflows/terransoul-ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Vue 3](https://img.shields.io/badge/Vue-3.5-4FC08D?logo=vuedotjs&logoColor=white)](https://vuejs.org/)
[![Tauri 2](https://img.shields.io/badge/Tauri-2.x-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)

TerranSoul is a local 3D AI assistant and built on harness/context engeering for daily live reliability. It combines hybrid-memory RAG, knowledge graphs, and self-running MCP infrastructure to give AI coding agents (Copilot, Claude Code, Cursor, Codex) persistent project-wide context, semantic retrieval, and self-improving workflows — fully offline-capable with pluggable local or cloud LLMs.

If you want a personal AI that **remembers everything**, **runs on your hardware**, **everything offline** and **makes your other AI tools smarter by sharing its brain** — this is it.

[Tutorials](tutorials/) · [Architecture](docs/brain-advanced-design.md) · [Design](docs/DESIGN.md) · [Persona](docs/persona-design.md) · [Hive Protocol](docs/hive-protocol.md) · [Contributing](CONTRIBUTING.md)

---

## Why TerranSoul?

Almost every dev, technical user, and even non-technical person now stitches together a **personal AI stack** out of pieces — a chat UI here, a RAG tool there, a voice assistant on the phone, a coding agent in the IDE, a workflow runner in the cloud, and a notes app pretending to be memory. Each one is great in isolation. None of them share a brain. None of them follow you across devices.

**TerranSoul is the project that puts all of these into one open, local-first, MIT-licensed companion** — a 3D VRM avatar with voice, a persistent memory + RAG brain that you own, an MCP server that lets your coding agents share that brain, cross-device sync over CRDT, a skill tree that makes the system discoverable for non-technical users, and a plugin/agent harness for power users.

The pitch is simple: every dev/tech person is already building a personal assistant out of duct tape and every non tech person need one but it is very complicated to setup by their own. **Why not build one that benefits everyone?** That's what TerranSoul is.

If that resonates and wanna become contributor, contact me via:

- 💬 Discord: <https://discord.gg/RzXcvsabKD>
- ✉️ Email: [darren.bui@terransoul.com](mailto:darren.bui@terransoul.com)

Contributions from devs, designers, VRM artists, prompt engineers, and non-technical testers are all welcome.

---

## Install

Download from [GitHub Releases](https://github.com/Terranes/TerranSoul/releases):

| Platform | Format |
|----------|--------|
| Windows | `.msi` / `.exe` installer |
| macOS | `.dmg` (Apple Silicon + Intel) |
| Linux | `.deb` / `.rpm` / `.AppImage` |

First launch walks you through brain setup automatically (free cloud, paid API, or local Ollama).

---

## Quick Start

### Prerequisites

| Requirement | Min Version | Check |
|---|---|---|
| Node.js | ≥ 20 | `node -v` |
| Rust | stable | `rustc --version` |
| Tauri CLI | latest | `cargo tauri --version` |
| WebView2 | any (Windows only) | Auto-detected |

**Let your AI agent handle it** — every supported agent has a built-in setup command:

| Agent | Command |
|---|---|
| VS Code Copilot | `/setup-prerequisites` |
| Cursor | `@setup-prerequisites` |
| Claude Code | `/setup-prerequisites` |
| Codex CLI | "Run setup-prerequisites" |
| Roo Code | `/setup-prerequisites` |
| Windsurf | `@setup-prerequisites` |
| TerranSoul (in-app) | `/setup-prerequisites` |

The agent checks each requirement, installs what's missing, and re-verifies.

### Build & Run

```bash
git clone https://github.com/Terranes/TerranSoul.git
cd TerranSoul
npm install          # Install frontend dependencies
cargo tauri dev      # Full Tauri app with hot-reload
```

New user? The First Launch Wizard auto-configures everything. Just open the app.

---

## The Key Differentiator: Self-Running MCP Brain

TerranSoul runs a **headless MCP server** (`npm run mcp`) that exposes its brain — persistent memory, semantic search, knowledge graph, and code intelligence — to any AI coding agent over HTTP or stdio.

```bash
npm run mcp                              # Starts on 127.0.0.1:7423
curl http://127.0.0.1:7423/health        # Verify
```

For CI/research environments that need isolation without opening the Tauri
tray runtime, use the display-free MCP container:

```bash
npm run mcp:container          # docker compose up -d --build
npm run mcp:container:config   # validate compose config
npm run mcp:container:logs     # follow container logs
npm run mcp:container:stop     # stop the container stack
```

The desktop app remains a native Tauri install; containers are for MCP/headless
services and research infrastructure only. The service binds to `0.0.0.0` inside
the container, but Compose publishes it to host loopback (`127.0.0.1:7423`).

**Why this matters:** Your coding agents (Copilot, Claude Code, Cursor, Codex) gain:

- **Project memory** — decisions, architecture facts, lessons learned persist across sessions
- **Semantic search** — diversified RRF + HyDE + cross-encoder reranking over 1M+ memories, with compact-first result previews for low-token agent workflows
- **Code intelligence** — symbol index, impact analysis, cross-repo contracts
- **Self-improvement** — agents write learnings back to the brain for future sessions
- **10–50× context reduction** — retrieval returns focused facts, not raw file dumps

The MCP auto-starts when VS Code opens the workspace. No manual setup needed.

> Full setup: [MCP for Coding Agents tutorial](tutorials/mcp-coding-agents-tutorial.md) · [Agent bootstrap](rules/agent-mcp-bootstrap.md)

---

## Why Hybrid RAG (Vector + Knowledge Graph + Temporal Memory)

Most "RAG" systems are actually **vector-only RAG**: chunk → embed → cosine-similarity → top-k. That pattern is fast and easy, but it loses on three failure modes that matter for a long-running personal companion:

1. **Multi-hop / relational questions** — "Who approved that change, and who do they report to?" Vector search ranks by surface similarity, not by following an explicit relationship chain. A knowledge graph with typed edges (`works_for`, `depends_on`, `approved_by`, `child_of`, …) traverses the chain deterministically and returns the supporting subgraph with provenance.
2. **Temporal correctness** — "Why did we change the auth flow last week?" Pure vector stores happily return a stale paragraph from two months ago next to yesterday's decision. A temporal memory layer (per-memory `decay_score`, append-only versioning, recency multipliers) ensures newer truths override older ones rather than blending into a contradictory soup.
3. **"What's actually relevant to *me*"** — Vector similarity has no notion of which files, projects, or people the user actually interacts with. A user-history / observation layer biases retrieval toward your active workspace and recent decisions.

TerranSoul's brain is a deliberate **Hybrid RAG**: vector for breadth, graph for depth, temporal layer for currency, and a corpus-aware lexical/RRF fuser on top so none of the three signals can starve the others.

```
       ┌──────────────────┐        ┌──────────────────┐        ┌──────────────────┐
       │ Vector (HNSW)    │        │ Knowledge Graph  │        │ Temporal Memory  │
       │ semantic recall  │        │ memory_edges     │        │ decay + versions │
       │ "meaning-similar"│        │ "who/what/why"   │        │ "what's current" │
       └────────┬─────────┘        └────────┬─────────┘        └────────┬─────────┘
                └──────────────┬────────────┴─────────────┬─────────────┘
                               ▼                          ▼
                  ┌─────────────────────────────┐   ┌──────────────────────┐
                  │  Reciprocal Rank Fusion     │   │  Lexical / FTS5      │
                  │  k=60 across all retrievers │   │  acronym + rare-term │
                  └──────────────┬──────────────┘   └─────────┬────────────┘
                                 ▼                            ▼
                          ┌──────────────────────────────────────┐
                          │ Session-diversified candidate pool   │
                          │ → HyDE (cold queries) → cross-encoder│
                          │ rerank → relevance threshold         │
                          └──────────────────┬───────────────────┘
                                             ▼
                               [LONG-TERM MEMORY] block → LLM
```

**What that gives you concretely:**

- *Vector* finds the paragraph that "means" your question (Vector RAG strength: fast, scalable, fuzzy).
- *Graph* (the `memory_edges` table, populated from extracted entities + tags) lets the retriever follow `manager → approved_by → deployment` chains that vectors fragment. This is the [GraphRAG](docs/brain-advanced-design.md#6-knowledge-graph-vision) pattern.
- *Temporal layer* — decay scores, source-hash invalidation, LLM-powered contradiction resolution, and non-destructive version history — keeps stale facts from drowning out new ones, the way agent-memory frameworks like Graphify/Cognee describe.
- *Lexical / FTS5 with corpus-aware acronym + rare-term weighting* rescues exact-match queries (file paths, IDs, version numbers) that pure semantic similarity routinely misses.
- *RRF + diversification + HyDE + cross-encoder rerank* turn the four candidate streams into one tight, deduplicated, scored top-k. Measured: **LongMemEval-S R@10 99.6 %, NDCG@10 91.3 %, MRR 92.6 %** — beats published agentmemory and MemPalace numbers on the comparable retrieval slice.

**When this stops being academic:** the moment you start using TerranSoul daily, your brain accumulates conversations, documents, code, persona drift, and decisions. After a few weeks, a vector-only store starts surfacing "plausible-but-wrong" old notes. The hybrid design is what keeps the companion *reliable enough to trust* over months.

> Deep dive: [docs/brain-advanced-design.md](docs/brain-advanced-design.md) · The GraphRAG-vs-Vector RAG comparison and the agentmemory + Graphify hybrid pattern that informed this design are credited in [CREDITS.md](CREDITS.md).

---

## Harness + Context Engineering

TerranSoul's 3D assistant is not only a chat UI + avatar layer. It is built on a coding harness and context-engineering stack that keeps agent work reliable over long sessions.

Core harness capabilities:
- Deterministic coding workflow runner with typed output contracts
- Milestone-driven DAG execution with explicit test/review gates
- MCP-native project memory and code-intelligence tools for retrieval-first operation
- Session replay, handoff seeding, and transcript repair for resilient resume

Core context-engineering capabilities:
- Policy-driven context loading from rules, instructions, and docs
- Priority-based budget fitting before every coding task prompt
- Rolling summarization hooks that mark stale messages out of active context
- Cross-resume prompt-token seeding so compression behavior survives restarts

This is what lets TerranSoul support long-running, multi-agent coding workflows while still staying inspectable and controllable by humans.

> Design references: [Coding Workflow Design](docs/coding-workflow-design.md) · [Harness/Context Audit (2026-05)](docs/harness-context-engineering-audit-2026-05.md) · [Prompting Rules](rules/prompting-rules.md)

---

## Highlights

- **3D VRM Avatar** — lip sync, expressions, motion capture, spring-bone physics. Pet mode floats on your desktop.
- **Multi-Provider Brain** — Free cloud (Pollinations/OpenRouter/Gemini), paid (OpenAI/Anthropic/Groq), or local Ollama. Switch anytime.
- **Persistent Memory + RAG** — thresholded hybrid eligibility, corpus-aware acronym/rare-term lexical ranking with low-signal caps, session-diversified RRF + query-intent prompt ordering for live chat, gated knowledge-graph neighbor boosts, cognitive-kind tags including `procedure`/`procedural` aliases, HyDE, cross-encoder reranker, N-to-1 consolidation summaries with parent/child links, knowledge graph with typed edges, progressive compact-first search responses, RAG-contextual intent classification for setup/quest routing from user-customizable seeded system defaults, a deterministic shortcut for explicit onboarding phrases like "Learn from my documents", and a fast chat path that skips retrieval for greetings so LocalLLM replies stay under 1s when warm. 1M+ entries benchmarked; LongMemEval-S retrieval-only verified at R@5 **99.2 %**, R@10 **99.6 %**, R@20 **100.0 %**, NDCG@10 **91.3 %**, MRR **92.6 %**.
- **Knowledge Wiki** — `/digest`, `/spotlight`, `/serendipity`, `/revisit` commands for graph curation.
- **Voice** — ASR (Web Speech, Groq Whisper, OpenAI Whisper) + TTS (Web Speech, OpenAI), editable model/persona voice profiles, and full lip-sync pipeline.
- **Skill Tree** — 40+ skills across 5 categories. RPG-style quest progression, auto-detection, combo unlocks.
- **Device Sync** — CRDT-based peer-to-peer replication over QUIC/WebSocket. QR pairing.
- **Hive Federation** — opt-in relay for shared knowledge + distributed AI jobs. Ed25519-signed bundles, per-memory privacy ACLs.
- **AI Package Manager** — browse, install, manage agents from a built-in marketplace.
- **Plugin System** — manifest-driven extensibility with capability-gated permissions. Optional WASM sandbox.
- **Multi-Agent Workflows** — 6 agent roles, YAML plans, DAG execution, recurring schedules.
- **Cross-Platform** — Windows, macOS, Linux, iOS (Tauri 2), browser (static deploy + LAN bridge).
- **MCP Brain Server** — self-running knowledge layer for external AI agents (see above).
- **Code Intelligence** — native symbol indexer, `code_query`, `code_impact`, `code_rename`, cross-repo contracts.

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Shell | Tauri 2.x (Windows/macOS/Linux/iOS/Android) |
| Backend | Rust (stable), async Tokio |
| Frontend | Vue 3.5 + TypeScript 5.x, Pinia |
| 3D | Three.js + @pixiv/three-vrm |
| Bundler | Vite 6.x |
| DB | SQLite (local, 1M+ benchmarked), Postgres (hive relay) |
| LLM | Ollama (local), OpenAI/Anthropic/Groq (cloud), Pollinations (free) |
| Vector | per-shard usearch HNSW (desktop, `vectors/<tier>__<kind>.usearch`), pure-Rust fallback (mobile) |
| Sync | CRDT (LWW + 2P-Set) over QUIC/WebSocket |

---

At billion-scale fan-out, TerranSoul also uses a persisted coarse shard router
(`vectors/shard_router.json`) that stores centroid vectors + shard mapping,
allowing lazy reload of top-p shard routing across restarts before rebuilding
from live embeddings.

Router maintenance is now policy-driven: rebuild attempts are cooldown-gated,
triggered by stale/missing routers or mutation-volume thresholds, and forced
on scheduled maintenance (`AnnCompact`) to avoid repeated on-query rebuild
bursts under heavy traffic. MCP `brain_health` also exposes router metadata
(`router_health`) including build age, centroid count, and staleness.

Phase 3 disk-backed ANN now includes an executable migration hook:
`MemoryStore::run_disk_ann_migration_job` writes per-shard IVF-PQ sidecar
metadata (`vectors/<shard>.ivfpq.json`) from deterministic planner candidates,
`AnnCompact` triggers sidecar writes on schedule, and MCP `brain_health`
surfaces `disk_ann_health` (eligible candidates, sidecars present, and gaps).

---

## Brain Modes

| Mode | Privacy | Setup | Use Case |
|------|---------|-------|----------|
| **Free API** | Cloud (free-tier) | Zero config | Quick start, no cost |
| **Paid API** | Cloud (your key) | API key | Best quality (GPT-4o, Claude, etc.) |
| **Local Ollama** | Fully offline | ~2 GB download | Maximum privacy, no internet |

Local chat keeps small turns fast: short greetings and acknowledgements skip intent-classifier, embedding, and RAG retrieval work, avoiding `nomic-embed-text`/chat-model VRAM swaps on consumer GPUs. Contentful setup requests still use the backend classifier in every brain mode, including Local Ollama; `classify_intent` retrieves app knowledge from the memory/RAG store and also preserves a small deterministic shortcut for explicit onboarding phrases like **"Learn from my documents"** so Scholar's Quest still opens when a local model returns `unknown` for the exact tutorial wording. Local Ollama also pre-warms the chat model on startup, pauses background embedding ticks during the startup/active-chat quiet window, unloads embedding models immediately after batch work, and disables raw silent thinking on the hot stream so visible tokens begin quickly. Contentful questions use thresholded memory eligibility, corpus-aware exact lexical ranking over content/tags with broad-term caps, gated `memory_edges` neighbor boosts, and session-diversified RRF + query-intent prompt ordering, with uncapped global memories and a default cap for noisy per-session clusters; Local Ollama keeps this keyword/freshness-only on the hot path when embedding would swap models.

Scholar's Quest starts after its prerequisites are active rather than being auto-completed by setup. Sage's Library (`rag-knowledge`) is the final prerequisite; Learn Docs re-checks the live brain and memory state instead of trusting saved quest completion, then shows that setup check as a collapsed thinking block on the chat prompt. If a user opens the quest early, the dialog shows the missing prerequisite quests and offers Cancel or Start Now for setup instead of showing a Verify Brain step. Once prerequisites are active, the quest opens the document picker. The web-crawl toggle in that picker is saved in app settings with configurable depth and page limits (defaults: off, depth 2, 20 pages), and the ingest backend clamps crawl requests to depth 1..=5 and pages 1..=100 while preserving legacy `crawl:<url>` imports.

Local Ollama hardware recommendations favor responsive interactive models by default; larger catalogue models remain selectable for users who prefer slower, heavier reasoning runs.

---

## MCP Tools (for AI Coding Agents)

When connected, agents get 21 tools:

| Category | Tools |
|----------|-------|
| **Brain** | `brain_health`, `brain_search`, `brain_suggest_context`, `brain_get_entry`, `brain_list_recent`, `brain_kg_neighbors`, `brain_summarize`, `brain_ingest_url`, `brain_failover_status` |
| **Code** | `code_query`, `code_context`, `code_impact`, `code_rename`, `code_generate_skills`, `code_list_groups`, `code_create_group`, `code_add_repo_to_group`, `code_group_status`, `code_extract_contracts`, `code_list_group_contracts`, `code_cross_repo_query` |

Ports: `7421` (release app), `7422` (dev), `7423` (headless `npm run mcp`).

---

## Tutorials

| Tutorial | Covers |
|----------|--------|
| [Quick Start](tutorials/quick-start-tutorial.md) | Install, first chat, pet mode, VRM import |
| [Brain + RAG (Free)](tutorials/brain-rag-setup-tutorial.md) | Cloud brain, document ingestion, RAG Q&A |
| [Brain + RAG (Local)](tutorials/brain-rag-local-lm-tutorial.md) | Ollama setup, offline privacy |
| [Voice Setup](tutorials/voice-setup-tutorial.md) | ASR, TTS, hotwords, lip sync |
| [Skill Tree & Quests](tutorials/skill-tree-quests-tutorial.md) | 40+ skills, combos, auto-detection |
| [Advanced Memory & RAG](tutorials/advanced-memory-rag-tutorial.md) | HyDE, reranker, cognitive axes, decay, conflicts |
| [Knowledge Wiki](tutorials/knowledge-wiki-tutorial.md) | /digest, /spotlight, /serendipity, /revisit |
| [Folder → Knowledge Graph](tutorials/folder-to-knowledge-graph-tutorial.md) | Code indexing, Obsidian export, wiki |
| [Teaching Animations](tutorials/teaching-animations-expressions-persona-tutorial.md) | Webcam expressions, motion capture |
| [Charisma Teaching](tutorials/charisma-teaching-tutorial.md) | Self-learning animation promotion system |
| [Device Sync & Hive](tutorials/device-sync-hive-tutorial.md) | Pairing, CRDT sync, relay, privacy ACLs |
| [LAN Brain Sharing](tutorials/lan-mcp-sharing-tutorial.md) | Share brain on local network |
| [MCP for Coding Agents](tutorials/mcp-coding-agents-tutorial.md) | VS Code Copilot, headless runner, code tools |
| [Multi-Agent Workflows](tutorials/multi-agent-workflows-tutorial.md) | 6 roles, YAML plans, DAG execution |
| [Packages & Plugins](tutorials/packages-plugins-tutorial.md) | Marketplace, manifests, WASM sandbox |
| [Browser & Mobile](tutorials/browser-mobile-tutorial.md) | Web deploy, phone pairing, gRPC remote |
| [Self-Improve → PR](tutorials/self-improve-to-pr-tutorial.md) | Coding workflow, GitHub PR generation |
| [OpenClaw Plugin](tutorials/openclaw-plugin-tutorial.md) | Plugin example (legal document analysis) |

---

## Security

- All local data encrypted at rest via SQLite + OS keychain
- MCP uses bearer-token authentication (auto-generated per session)
- Hive bundles are Ed25519 signed; relay verifies before accepting
- Per-memory privacy ACL: `private` (never leaves device) → `paired` → `hive`
- WASM sandbox isolates untrusted plugins
- See [SECURITY.md](SECURITY.md)

---

## Development

```bash
npm run dev                    # Kill all + start full Tauri app (Vite + Rust)
npm run dev:vite               # Vite-only dev server (:1420)
npx vitest run                 # Frontend tests (1738 passing)
cargo test                     # Backend tests (2383 passing)
cargo clippy -- -D warnings    # Lint
npm run mcp                    # Start headless MCP brain (:7423)
npm run mcp:container          # Start isolated MCP HTTP container (:7423)
```

**CI Gate:** `npx vitest run && npx vue-tsc --noEmit && cargo clippy -- -D warnings && cargo test`

See [rules/milestones.md](rules/milestones.md) for active work and [rules/completion-log.md](rules/completion-log.md) for history.

---

## Architecture

```
Frontend (Vue 3 + Three.js/VRM + Pinia)
    ↕ Tauri IPC
Rust Core (150+ commands)
  ├── brain/         — LLM providers, model recommender, embeddings
  ├── memory/        — SQLite store, lexical/vector RAG, gated KG boosts, wiki, eviction
  ├── hive/          — Federation protocol, signing, jobs, privacy
  ├── ai_integrations/mcp/ — MCP server (HTTP + stdio)
  ├── coding/        — Self-improve, DAG runner, task queue
  ├── persona/       — Traits, voice profile design, drift detection, pack export
  ├── identity/      — Ed25519 device keys
  ├── link/          — CRDT sync over QUIC/WebSocket
  └── orchestrator/  — Agent routing, capability gates
```

> Deep dives: [brain-advanced-design.md](docs/brain-advanced-design.md) · [persona-design.md](docs/persona-design.md) · [hive-protocol.md](docs/hive-protocol.md) · [AI-coding-integrations.md](docs/AI-coding-integrations.md)

---

## Contact

**Darren Bui** — [darren.bui@terransoul.com](mailto:darren.bui@terransoul.com)

Interested in becoming a contributor? Join the Discord at <https://discord.gg/RzXcvsabKD> or email Darren directly. Devs, designers, VRM artists, prompt engineers, testers, and non-technical users are all welcome.

Built for the community. MIT License.
