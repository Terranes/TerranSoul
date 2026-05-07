# TerranSoul — 3D AI Companion with Persistent Memory

> **🚧 Active development since 10/04/2026.**
> Interested? Join us at <https://discord.gg/RzXcvsabKD>

[![CI](https://github.com/Terranes/TerranSoul/actions/workflows/terransoul-ci.yml/badge.svg)](https://github.com/Terranes/TerranSoul/actions/workflows/terransoul-ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Vue 3](https://img.shields.io/badge/Vue-3.5-4FC08D?logo=vuedotjs&logoColor=white)](https://vuejs.org/)
[![Tauri 2](https://img.shields.io/badge/Tauri-2.x-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)

TerranSoul is a desktop/mobile AI companion with a 3D VRM avatar, persistent memory, semantic-search RAG, and a self-running MCP brain server that gives AI coding agents (Copilot, Claude Code, Cursor, Codex) project-wide knowledge, retrieval, and self-improvement — all local-first and offline-capable.

If you want a personal AI that **remembers everything**, **runs on your hardware**, and **makes your other AI tools smarter by sharing its brain** — this is it.

[Tutorials](tutorials/) · [Architecture](docs/brain-advanced-design.md) · [Design](docs/DESIGN.md) · [Persona](docs/persona-design.md) · [Hive Protocol](docs/hive-protocol.md) · [Contributing](CONTRIBUTING.md)

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

**One-command setup** — checks everything and installs what's missing:

```bash
node scripts/setup-prerequisites.mjs --auto
```

Or use your AI coding agent:
- **VS Code Copilot:** `/setup-prerequisites`
- **Cursor:** `@setup-prerequisites`
- **Claude Code / Codex:** "Run setup-prerequisites"

### Build & Run

```bash
git clone https://github.com/Terranes/TerranSoul.git
cd TerranSoul
npm run setup        # Check prerequisites (--auto to install)
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

**Why this matters:** Your coding agents (Copilot, Claude Code, Cursor, Codex) gain:

- **Project memory** — decisions, architecture facts, lessons learned persist across sessions
- **Semantic search** — RRF + HyDE + cross-encoder reranking over 1M+ memories
- **Code intelligence** — symbol index, impact analysis, cross-repo contracts
- **Self-improvement** — agents write learnings back to the brain for future sessions
- **10–50× context reduction** — retrieval returns focused facts, not raw file dumps

The MCP auto-starts when VS Code opens the workspace. No manual setup needed.

> Full setup: [MCP for Coding Agents tutorial](tutorials/mcp-coding-agents-tutorial.md) · [Agent bootstrap](rules/agent-mcp-bootstrap.md)

---

## Highlights

- **3D VRM Avatar** — lip sync, expressions, motion capture, spring-bone physics. Pet mode floats on your desktop.
- **Multi-Provider Brain** — Free cloud (Pollinations/OpenRouter/Gemini), paid (OpenAI/Anthropic/Groq), or local Ollama. Switch anytime.
- **Persistent Memory + RAG** — hybrid 6-signal search, RRF fusion, HyDE, cross-encoder reranker, knowledge graph with typed edges. 1M+ entries benchmarked.
- **Knowledge Wiki** — `/digest`, `/spotlight`, `/serendipity`, `/revisit` commands for graph curation.
- **Voice** — ASR (Web Speech, Groq Whisper, OpenAI Whisper) + TTS (Web Speech, OpenAI). Full lip-sync pipeline.
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
| Vector | usearch HNSW (desktop), pure-Rust fallback (mobile) |
| Sync | CRDT (LWW + 2P-Set) over QUIC/WebSocket |

---

## Brain Modes

| Mode | Privacy | Setup | Use Case |
|------|---------|-------|----------|
| **Free API** | Cloud (free-tier) | Zero config | Quick start, no cost |
| **Paid API** | Cloud (your key) | API key | Best quality (GPT-4o, Claude, etc.) |
| **Local Ollama** | Fully offline | ~2 GB download | Maximum privacy, no internet |

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
npm run dev                    # Vite dev server (:1420)
cargo tauri dev                # Full app with hot-reload
npx vitest run                 # Frontend tests (1738 passing)
cargo test                     # Backend tests (2383 passing)
cargo clippy -- -D warnings    # Lint
npm run mcp                    # Start headless MCP brain (:7423)
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
  ├── memory/        — SQLite store, RAG pipeline, KG, wiki, eviction
  ├── hive/          — Federation protocol, signing, jobs, privacy
  ├── ai_integrations/mcp/ — MCP server (HTTP + stdio)
  ├── coding/        — Self-improve, DAG runner, task queue
  ├── persona/       — Traits, drift detection, pack export
  ├── identity/      — Ed25519 device keys
  ├── link/          — CRDT sync over QUIC/WebSocket
  └── orchestrator/  — Agent routing, capability gates
```

> Deep dives: [brain-advanced-design.md](docs/brain-advanced-design.md) · [persona-design.md](docs/persona-design.md) · [hive-protocol.md](docs/hive-protocol.md) · [AI-coding-integrations.md](docs/AI-coding-integrations.md)

---

## Contact

**Darren Bui** — darren.bui@terransoul.com

Built for the community. MIT License.
