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

The pitch is simple: every dev/tech person is already building a personal assistant, aka J.A.R.V.I.S or 3D companion or RAG/knowledge graph, out of duct tape and every non tech person need one but it is very complicated to setup by their own. **Why not build one that benefits everyone?** That's what TerranSoul is.

If that resonates and wanna become contributor, contact me via:

- 💬 Discord: <https://discord.gg/RzXcvsabKD>
- ✉️ Email: [darren.bui@terransoul.com](mailto:darren.bui@terransoul.com)

Contributions from devs, designers, VRM artists, prompt engineers, and non-technical testers are all welcome.

---

## TerranSoul in five scenes — how graph, RAG and memory show up in a regular day

[TerranSoul in five scenes](https://terransyn.github.io/TerranSoul/) is the public, non-technical primer for the memory stack: raw model answers, personal context, vector retrieval, knowledge graphs, and production hybrid memory.

**TerranSoul is built for production hybrid memory.** Concretely:

- **Personal-context construction** (Scene 2) is shipped via persona traits, observation history, decay, and the cognitive-kind axis (`episodic` / `semantic` / `procedural` / `judgment`).
- **Vector recall** (Scene 3) is shipped via per-shard HNSW (`usearch`) over `mxbai-embed-large` / `nomic-embed-text` with semantic chunking and Anthropic-style Contextual Retrieval.
- **Typed knowledge graph + multi-hop** (Scene 4) is shipped via the `memory_edges` table, entity resolution, and `multi_hop_search_memories`.
- **Production discipline** (Scene 5) is shipped via RRF fusion, pre-computed shard router, query-class HyDE, cross-encoder rerank, search/KG caches, and a public bench harness (LongMemEval-S, LoCoMo MTEB, agentmemory token-efficiency).

The "Why Hybrid RAG" section below explains the technical architecture. The five-scenes primer explains the user-facing shape of the same system.

> Read the Pages version: <https://terransyn.github.io/TerranSoul/>. The source mapping remains in [docs/ai-memory-five-scenes-terransoul.md](docs/ai-memory-five-scenes-terransoul.md), and the original cognee inspiration is credited in [CREDITS.md](CREDITS.md); no prose or imagery from it is reproduced.

---

## Why TerranSoul is different

Most "AI companion" apps are a chat box plus an embedding store. TerranSoul is built on the assumption that **a personal assistant must keep working over years, across devices, across teammates, and across failures** — which means treating it as distributed infrastructure from day one, not a chat UI with a database glued on.

The differentiators below name what's **shipped** vs what's a **design target** (with a milestone chunk that closes the gap). No aspirational claims dressed up as facts.

| Pillar | What it means | Status |
|---|---|---|
| **Hybrid-RAG retrieval over 6 signals** | Vector (HNSW ANN) + keyword (FTS5) + recency + importance + decay + tier, fused with **RRF**, sharpened by per-query-class **HyDE**, cut by an **LLM-as-judge cross-encoder rerank**, and expanded across **knowledge-graph edges** + **semantic chunking** + **LLM-resolved contradictions**. Local embeddings via `mxbai-embed-large` (1024-d) or `nomic-embed-text` (768-d) on Ollama; cloud embeddings for paid/free brain modes. | **Shipped** ([docs/brain-advanced-design.md](docs/brain-advanced-design.md)) |
| **Agent fleet, not a monolith** | Each agent runs as a separate OS process registered with the orchestrator; adding the 100th agent requires no core change. Heavy agents land on "Primary" devices, lightweight on "Secondary". Inspired by the actor-style isolation pattern — failures stay local to one process. | **Shipped** ([rules/quality-pillars.md § Scalability](rules/quality-pillars.md)) |
| **Scale-to-infinity memory** | Sharded HNSW (15 logical shards, 3 tiers × 5 cognitive_kinds) with a coarse centroid router persisted to disk; IVF-PQ disk-backed shards for >100M; benchmarked at 100k LoCoMo-at-scale with R@10 64.0 % retrieval-only. Per-shard knowledge graph for graph hops at scale. | **Shipped at 100k, harness ready for 1M** — `BENCH-SCALE-2` run pending |
| **Per-repo knowledge sources** | A `memory_sources` registry (schema v22) lets the Memory panel pin separate brains alongside `🧠 TerranSoul`: each `📦 owner/repo` source gets its own SQLite + HNSW under `mcp-data/repos/<id>/`. Per-repo ingest runs `gix` shallow clone → `ignore`-aware walker → secret scanner → text chunker → tree-sitter `parent_symbol` annotation → per-repo `repo_chunks` table → embed pass (`cloud_embeddings::embed_for_mode`) → per-repo `vectors.usearch`, with per-file SHA-256 incremental sync and phase-by-phase `task-progress` events, behind a desktop-default `repo-rag` Cargo feature. Source-scoped retrieval (`repo_search` / `repo_list_files` / `repo_read_file` Tauri commands; `RepoStore::hybrid_search` 3-signal RRF fusion of vector + keyword + recency) lets the Memory panel query a single repo in isolation, and the same three operations are exposed as `BrainGateway` trait methods + identically-named MCP tools so external AI coding agents reach indexed repositories through MCP. Cross-source `All`-mode fan-out (`cross_source_search` gateway trait method + Tauri command + MCP tool) ranks chunks from the main brain *and* every indexed repo together, RRF-merged at `k=60` via a `usize`-arena keyed `reciprocal_rank_fuse`, returning `MultiSourceHit` rows tagged with `source_id` (`"self"` or the repo id), `source_label`, `tier`, and optional `file_path` / `parent_symbol`. The desktop chat surface wires the fan-out through a frontend prompt assembler that groups retrieved memories per source (`── 🧠 TerranSoul ──` / `── 📦 owner/repo ──` headers in the `[LONG-TERM MEMORY]` block), recognises `@source-id` chat-composer mentions as one-turn pulls (the Continue `@codebase` precedent — never mutates the active source), and renders a per-turn citation footer of contributing rows under each assistant message. Aider-style compression-mode surfaces (`repo_map(source_id, budget_tokens)` and `repo_signatures(source_id, file_path)`) expose a budget-bounded ⋮/│ repo overview and a single-file signature-only preview as Tauri commands + identically-named MCP tools, letting external coding agents pull a compass of the entire repo into one tool call. The new `repo-scholar-quest` skill activates when the brain is configured and at least one `📦` source is registered. The unified knowledge graph extends the same provenance to its visualization layer — when the user picks the `All sources` view in the Memory panel, `cross_source_graph_nodes(per_source_limit)` projects each connected repo's most recent AST-annotated chunks into the 2D `MemoryGraph` and 3D `MemoryGalaxy` as additional nodes (negative `graphId`s avoid colliding with personal `memories.id`), rendered in the `--ts-warning` hue with a `📦 owner/repo · file::symbol` provenance line in the right-side inspector so the graph reads as a code-aware browser across every connected source. | **Per-repo ingest + single-source retrieval + MCP surface + cross-source backend + chat wiring + Aider-style repo map / signatures + repo-scholar quest + private-repo OAuth device flow + cross-source graph projection + deep-scan ingest visibility complete** (`BRAIN-REPO-RAG-1a`, `1b-i`, `1b-ii-a`, `1b-ii-b`, `1c-a`, `1c-b-i`, `1c-b-ii-a`, `1c-b-ii-b`, `1d`, `1e`, `2a`, `2b`). Private-repo auth uses GitHub's RFC 8628 device flow; the token persists at `<data_dir>/oauth/github.json` with FS-permission hardening (Unix 0o600 / non-Unix readonly) and is injected as `https://x-access-token:<token>@github.com/...` at clone time. Deep-scan visibility (`2b`): every size/binary/unchanged/secret skip now emits an explicit `IngestPhase::Skip` event with a typed `skip_reason`, a per-run `IngestPhase::Summary` event fires before `Done`, and `TaskProgressBar.vue` surfaces a collapsible per-task debug log with sticky skip/index counter chips so users can audit every file the walker touched. |
| **Knowledge sharing across TerranSoul instances** | Hive relay (`crates/hive-relay/`) lets a user's partner / company / second PC subscribe to specific shards under signed bundles, with per-memory privacy ACLs. CRDT sync over QUIC/WebSocket handles peer-to-peer pairing without a central server. | **Shipped peer-to-peer**, **design target for org/team relays** (see `SCALE-INF-1`) |
| **Resilience under failure** | CRDT merges on reconnect (partial sync never corrupts state); persist-before-acknowledge for every state-changing IPC; atomic `write-temp-then-rename` for every JSON config; agent-crash detect + retry. | **Shipped** ([rules/coding-workflow-reliability.md](rules/coding-workflow-reliability.md), [rules/quality-pillars.md § Resilience](rules/quality-pillars.md)) |
| **Availability target: five nines (99.999 %)** | Personal assistants must not "go down". Treat the local TerranSoul as a single-tenant service with a budget of ≈ 5 min of unplanned downtime per year, measured via in-process uptime telemetry + crash-loop detection. | **Design target** — uptime SLO + telemetry + chaos test in `RESILIENCE-1` |
| **CAP-aware sync: P mandatory, A or C per purpose** | A device on a plane has a partition. We choose **A** (eventual consistency via CRDT) for memories tagged `personal`/`scratch` so writes never block, and **C** (consensus before commit) for memories tagged `legal`/`financial`/`shared-team` so two devices never disagree on a binding fact. | **Design target** — explicit per-memory CAP profile selector in `CAP-1` |
| **Durable workflows** | The coding-workflow runner uses an append-only SQLite event log + deterministic replay (inspired by Temporal's history pattern, **not** a Temporal client). Long-running coding sessions survive process restart, OS reboot, and partial network loss. | **Shipped** ([docs/coding-workflow-design.md](docs/coding-workflow-design.md), [instructions/AGENT-ROSTER.md](instructions/AGENT-ROSTER.md)) |
| **Companion ecosystem, not a walled garden** | TerranSoul detects-and-links — never silently installs — companion AI apps (Hermes Desktop GUI, Hermes Agent CLI, OpenClaw bridge) for workloads heavier than a single TerranSoul session should answer alone. Install only after an explicit click + OS UAC consent. | **Shipped** (see [Companion AI Ecosystem](#companion-ai-ecosystem) below) |

> **The "and" rule.** Most projects optimise for one or two of these and let the rest rot. TerranSoul refuses to ship a feature unless it composes cleanly with the others — that's why the brain, the avatar, the agent fleet, the sync layer, and the workflow runner all share the same memory store, the same persona, and the same consent model.

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

TerranSoul runs a **local MCP tray runtime** (`npm run mcp`) that exposes its brain — persistent memory, semantic search, knowledge graph, and code intelligence — to any AI coding agent. Agents reuse an existing release app (`127.0.0.1:7421`), MCP tray (`127.0.0.1:7423`), or dev app (`127.0.0.1:7422`) in that order, so Copilot, Claude Code, Cursor, and Codex can share one running brain instead of spawning one process per session.

```bash
npm run mcp                              # Starts on 127.0.0.1:7423
curl http://127.0.0.1:7423/health        # Verify
node scripts/mcp-tray-proxy.mjs --probe  # Show which running MCP server agents will use
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
- **10–30× context reduction** — retrieval returns focused facts, not raw file dumps (BENCH-AM-4: **91.4 %** token savings vs full-context paste at R@10 63.6 %; see [benchmark/terransoul/agentmemory-quality/](benchmark/terransoul/agentmemory-quality/README.md))

The MCP tray auto-starts when VS Code opens the workspace if no release/tray/dev server is already available. The VS Code profile uses a stdio proxy that reads token files directly and forwards requests to the existing server, so no bearer-token environment variable or VS Code restart is required.

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
- **Persistent Memory + RAG** — thresholded hybrid eligibility, corpus-aware acronym/rare-term lexical ranking with low-signal caps, session-diversified RRF + query-intent prompt ordering for live chat, gated knowledge-graph neighbor boosts (opt-in `enable_kg_boost` setting; cloud streaming chat exercises all 5 design-doc stages: embed → class-gated HyDE → RRF → KG cascade → cross-encoder rerank), cognitive-kind tags including `procedure`/`procedural` aliases, HyDE, cross-encoder reranker, N-to-1 consolidation summaries with parent/child links, knowledge graph with typed edges, progressive compact-first search responses, RAG-contextual intent classification for setup/quest routing from user-customizable seeded system defaults, a deterministic shortcut for explicit onboarding phrases like "Learn from my documents", and a fast chat path that skips retrieval for greetings so LocalLLM replies stay under 1s when warm. 1M+ entries latency-benchmarked; 100k+ entries quality-benchmarked on LoCoMo-at-scale (BENCH-SCALE-1b); LongMemEval-S retrieval-only verified at R@5 **99.2 %**, R@10 **99.6 %**, R@20 **100.0 %**, NDCG@10 **91.3 %**, MRR **92.6 %**.
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

**Default local embedding model:** `mxbai-embed-large` (1024-dim, ~660 MB; promoted in BENCH-LCM-5). `nomic-embed-text` (768-dim, ~270 MB) is available as the lightweight fallback.

Local chat keeps small turns fast: short greetings and acknowledgements skip intent-classifier, embedding, and RAG retrieval work, avoiding `nomic-embed-text`/chat-model VRAM swaps on consumer GPUs. Contentful setup requests still use the backend classifier in every brain mode, including Local Ollama; `classify_intent` retrieves app knowledge from the memory/RAG store and also preserves a small deterministic shortcut for explicit onboarding phrases like **"Learn from my documents"** so Scholar's Quest still opens when a local model returns `unknown` for the exact tutorial wording. Local Ollama also pre-warms the chat model on startup, pauses background embedding ticks during the startup/active-chat quiet window, unloads embedding models immediately after batch work, and disables raw silent thinking on the hot stream so visible tokens begin quickly. Contentful questions use thresholded memory eligibility, corpus-aware exact lexical ranking over content/tags with broad-term caps, gated `memory_edges` neighbor boosts, and session-diversified RRF + query-intent prompt ordering, with uncapped global memories and a default cap for noisy per-session clusters; Local Ollama keeps this keyword/freshness-only on the hot path when embedding would swap models.

Scholar's Quest starts after its prerequisites are active rather than being auto-completed by setup. Sage's Library (`rag-knowledge`) is the final prerequisite; Learn Docs re-checks the live brain and memory state instead of trusting saved quest completion, then shows that setup check as a collapsed thinking block on the chat prompt. If a user opens the quest early, the dialog shows the missing prerequisite quests and offers Cancel or Start Now for setup instead of showing a Verify Brain step. Once prerequisites are active, the quest opens the document picker. The web-crawl toggle in that picker is saved in app settings with configurable depth and page limits (defaults: off, depth 2, 20 pages), and the ingest backend clamps crawl requests to depth 1..=5 and pages 1..=100 while preserving legacy `crawl:<url>` imports.

Local Ollama hardware recommendations favor responsive interactive models by default; larger catalogue models remain selectable for users who prefer slower, heavier reasoning runs.

---

## MCP Tools (for AI Coding Agents)

When connected, agents get 35 tools:

| Category | Tools |
|----------|-------|
| **Brain (18)** | `brain_health`, `brain_search`, `brain_suggest_context`, `brain_get_entry`, `brain_list_recent`, `brain_kg_neighbors`, `brain_summarize`, `brain_ingest_url`, `brain_ingest_lesson`, `brain_append`, `brain_failover_status`, `brain_wiki_audit`, `brain_wiki_spotlight`, `brain_wiki_serendipity`, `brain_wiki_revisit`, `brain_wiki_digest_text`, `brain_review_gaps`, `brain_session_checklist` |
| **Code (17)** | `code_query`, `code_context`, `code_impact`, `code_rename`, `code_generate_skills`, `code_list_groups`, `code_create_group`, `code_add_repo_to_group`, `code_group_status`, `code_group_drift`, `code_extract_contracts`, `code_extract_negatives`, `code_list_group_contracts`, `code_cross_repo_query`, `code_branch_diff`, `code_branch_sync`, `code_index_commit` |

Ports: `7421` (release app), `7422` (dev), `7423` (headless `npm run mcp`).

---

## Companion AI Ecosystem

TerranSoul is a personal assistant, not a walled garden. For tasks that are heavier than a single TerranSoul session should answer alone — long deep-research, full-IDE coding sessions, durable multi-day workflows — TerranSoul **suggests the right companion AI app** and helps you install it the safe way. We do not silently install third-party software in the background.

**Install policy.** Every companion app is *detect-and-link* by default. When you open the Integrations panel or accept a companion quest, TerranSoul checks whether the tool is already installed. If not, it surfaces an **Install** button that runs the official per-OS installer (`winget`, `dnf`, `brew`, `apt`, or an official `.dmg` / `.exe` / `.AppImage`) — and only after you click and your OS confirms the elevation/UAC prompt. No background install. No bundled redistribution. No silent auto-update.

| Companion | Role | Integration today | Install path |
|---|---|---|---|
| **[Hermes Desktop](https://github.com/fathah/hermes-desktop)** *(MIT, Electron — `fathah/hermes-desktop`)* | Native desktop GUI for Hermes Agent: chat, sessions, profiles, memory, skills, tools, scheduling, 16 messaging gateways, 14 toolsets. | Suggest-and-link. TerranSoul recommends Hermes Desktop in-chat when your turn looks like deep research or a long multi-day workflow; clicking the suggestion opens the official install page and (on Windows) the `winget install NousResearch.HermesDesktop` command. See [docs/integrations/hermes-setup.md](docs/integrations/hermes-setup.md). | `winget install NousResearch.HermesDesktop` *(pending winget-pkgs PR)* · `.dmg` / `.exe` / `.AppImage` / `.deb` / `.rpm` from the [Releases page](https://github.com/fathah/hermes-desktop/releases) |
| **[Hermes Agent](https://github.com/NousResearch/hermes-agent)** *(MIT, Python CLI — NousResearch)* | The underlying self-improving agent: MCP support, AGENTS.md, FTS5 session search, subagent delegation, learning loop, scheduled tasks. | **Already wired.** TerranSoul writes a marker-managed MCP block into Hermes's `cli-config.yaml` via `setup_hermes_mcp` / `setup_hermes_mcp_stdio` so your TerranSoul brain shows up as a first-class MCP server to the Hermes CLI. See [docs/hermes-vs-openclaw-analysis.md](docs/hermes-vs-openclaw-analysis.md). | Install Hermes Desktop (above) and let it guide the Hermes Agent install, **or** follow the upstream CLI install at the [Hermes Agent repo](https://github.com/NousResearch/hermes-agent). |
| **[OpenClaw](https://github.com/openclaw/openclaw)** *(MIT, TypeScript)* | Open Claude-Code-style coding-agent UX. | **Already wired** as the built-in `openclaw-bridge` plugin ([`src-tauri/src/agent/openclaw_agent.rs`](src-tauri/src/agent/openclaw_agent.rs), [tutorials/openclaw-plugin-tutorial.md](tutorials/openclaw-plugin-tutorial.md)). TerranSoul owns memory/persona/consent; OpenClaw owns tool execution. | Install the OpenClaw CLI per upstream README, then `/openclaw …` slash commands route to it through the plugin. |
| **[Temporal.io](https://docs.temporal.io/workflows)** | Durable workflow engine. | **Design reference, not an integration.** TerranSoul's coding-harness runner is *inspired by* Temporal's deterministic-history pattern (see [`docs/coding-workflow-design.md`](docs/coding-workflow-design.md), [`instructions/AGENT-ROSTER.md`](instructions/AGENT-ROSTER.md)) — but TerranSoul does not run Temporal workers or talk to a Temporal cluster today. Outsourcing a TerranSoul workflow to a Temporal worker is on the backlog (see [`rules/milestones.md`](rules/milestones.md) Phase INTEGRATE, chunk INTEGRATE-4 follow-up). | n/a — TerranSoul does not require Temporal to run. |

**When TerranSoul suggests Hermes.** During chat, if your current turn looks heavier than TerranSoul should handle alone (large token budget, deep-research intent class, multi-day cron-style workflow), the chat surfaces a one-line dismissable hint linking to Hermes Desktop. The trigger is gated on **all three** of: estimated turn tokens ≥ `TS_HERMES_HINT_TOKENS` (default 4000), intent class ∈ {`deep_research`, `long_running_workflow`, `full_ide_coding`}, and `app_settings.hermes_hint_enabled` (default `true`, toggleable in Settings → Integrations). The hint never auto-launches anything — clicking it opens the Hermes Desktop setup quest. *(Hint-gate code lives behind Phase INTEGRATE in [`rules/milestones.md`](rules/milestones.md); the README contract is the bar the implementation must meet.)*

**Not listed = not integrated.** TerranSoul will not list a companion until we can verify the upstream URL, license, and at least one concrete TerranSoul workflow that benefits from delegating to it. If you want to see another companion here, open an issue with the upstream link and the workflow you'd delegate.

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
| [Hermes Setup](docs/integrations/hermes-setup.md) | Install Hermes Desktop + wire your TerranSoul MCP brain into the Hermes Agent CLI |

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
npx vitest run                 # Frontend tests (1872+ passing)
cargo test                     # Backend tests (2871+ passing)
cargo clippy -- -D warnings    # Lint
npm run mcp                    # Start MCP brain tray (:7423)
npm run mcp:container          # Start isolated MCP container (:7423)
```

**CI Gate:** `npx vitest run && npx vue-tsc --noEmit && cargo clippy -- -D warnings && cargo test`

See [rules/milestones.md](rules/milestones.md) for active work and [rules/completion-log.md](rules/completion-log.md) for history.

---

## Architecture

```
Frontend (Vue 3 + Three.js/VRM + Pinia)
    ↕ Tauri IPC
Rust Core (354 commands)
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
