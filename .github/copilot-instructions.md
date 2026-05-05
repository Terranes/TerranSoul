# TerranSoul — Copilot Instructions

> This file is auto-loaded by GitHub Copilot on every request.
> Last updated: 2026-05-05

## MANDATORY — TerranSoul MCP Preflight (every session, no exceptions)

**This rule is the highest priority instruction in this file. Execute it
before any other work — before reading milestones, before exploring code,
before planning, before implementing. Skipping this is a violation.**

1. **Check MCP health** — call `brain_health` (the MCP tool). If tools are
   not loaded yet, check `GET http://127.0.0.1:7423/health` (or `:7421`/`:7422`).
2. **If MCP is healthy** — call `brain_search` or `brain_suggest_context`
   with the current task/chunk topic before doing broad repo searches.
3. **If MCP is not healthy** — start it: run
   `node scripts/copilot-start-mcp.mjs` in a terminal, wait for health,
   then proceed to step 2.
4. **If MCP cannot start** (build failure, missing deps) — record the exact
   blocker in your progress report. Do NOT silently skip this step.
5. **Show the MCP receipt** — immediately after MCP preflight succeeds, send a
   user-visible progress update naming the health result/provider and the
   `brain_search` / `brain_suggest_context` topic used. Hidden tool calls are
   not enough; if the user cannot see the receipt, the preflight is incomplete.
6. **After completing work** — sync durable lessons into
   `mcp-data/shared/memory-seed.sql`.

> The workspace auto-starts MCP on folder open via a VS Code task
> (`TerranSoul MCP: Auto-Start`). If you still see no MCP, the build may
> have failed — check the task output.

---

## What is TerranSoul?

A **Vue 3 + Tauri 2** desktop AI companion app with a Rust backend. It features a 3D VRM anime character, multi-provider LLM chat, persistent memory with semantic-search RAG, voice I/O, CRDT-based device sync, and a gamified skill tree quest system.

## Tech Stack

| Layer | Technology |
|---|---|
| Shell | Tauri 2.x (Windows/macOS/Linux/iOS/Android) |
| Backend | Rust (stable, MSRV 1.80+), async-first with Tokio |
| Frontend | Vue 3.5 + TypeScript 5.x, Pinia state management |
| 3D | Three.js 0.175+ with @pixiv/three-vrm 3.x |
| Bundler | Vite 6.x |
| DB | SQLite (default), PostgreSQL, SQL Server, CassandraDB — via `StorageBackend` trait |
| LLM | Ollama (local), OpenAI-compatible APIs (cloud), Pollinations (free) |

## Architecture

```
Frontend (WebView — Vue 3 + TS)
  ├── Three.js/VRM Renderer (CharacterViewport)
  ├── Vue Components + Pinia Stores
  │   ├── brain.ts — LLM provider management
  │   ├── conversation.ts — chat history + streaming
  │   ├── memory.ts — persistent memory CRUD
  │   ├── persona.ts — persona traits + learned expressions/motions
  │   ├── skill-tree.ts — gamified quest system (1500+ lines)
  │   ├── voice.ts — TTS/ASR configuration
  │   └── settings.ts — app preferences
  └── Design system: CSS custom properties (--ts-* tokens in style.css)
      ↕ Tauri IPC (invoke / emit)
Rust Core Engine (src-tauri/src/)
  ├── commands/ — 150+ Tauri commands (chat, streaming, memory, brain, voice, window, mcp, etc.)
  ├── brain/ — LLM providers: OllamaAgent, OpenAiClient, FreeProvider, ProviderRotator
  │   ├── model_recommender.rs — RAM-based model catalogue (Gemma 4, Phi-4, Kimi K2.6 cloud)
  │   ├── ollama_agent.rs — embed_text(), hyde_complete(), rerank_score()
  │   └── cloud_embeddings.rs — unified embed_for_mode() for paid/free cloud modes
  ├── memory/ — StorageBackend trait + SQLite/Postgres/MSSQL/Cassandra backends
  │   ├── store.rs — SQLite MemoryStore (hybrid_search, hybrid_search_rrf, ANN index)
  │   ├── ann_index.rs — HNSW approximate nearest neighbor via usearch 2.x
  │   ├── fusion.rs — Reciprocal Rank Fusion (RRF, k=60)
  │   ├── hyde.rs — HyDE (Hypothetical Document Embeddings)
  │   ├── reranker.rs — LLM-as-judge cross-encoder reranker
  │   ├── chunking.rs — semantic chunking (text-splitter crate)
  │   ├── contextualize.rs — Contextual Retrieval (Anthropic 2024)
  │   ├── conflicts.rs — LLM-powered contradiction resolution
  │   ├── temporal.rs — natural-language time-range queries
  │   ├── versioning.rs — non-destructive edit history (V8)
  │   ├── obsidian_export.rs — one-way Obsidian vault export
  │   └── brain_memory.rs — LLM-powered extract/summarize/search
  ├── ai_integrations/ — expose brain to external AI coding assistants
  │   ├── gateway.rs — BrainGateway trait + AppStateGateway adapter (8 ops)
  │   └── mcp/ — MCP server (HTTP on 127.0.0.1:7421, bearer-token auth)
  ├── persona/ — persona traits, drift detection, pack export/import
  ├── identity/ — Ed25519 device identity for P2P linking
  ├── link/ — CRDT sync engine (QUIC/WebSocket)
  └── orchestrator/ — Agent routing with capability gates
```

## Brain Modes

1. **Free API** — OpenRouter/Gemini/NVIDIA/Pollinations free-tier providers with user-owned keys or tokens
2. **Paid API** — OpenAI/Anthropic/Groq with user-supplied API key
3. **Local Ollama** — Private, offline-capable, hardware-adaptive model selection

> ⚠️ **Brain Documentation Sync (mandatory rule)** — Any change that touches
> the brain surface (LLM providers, memory store, RAG pipeline, ingestion,
> embeddings, cognitive-kind classification, knowledge graph, decay/GC,
> brain-gating quests, brain Tauri commands or Pinia stores) **must update
> both `docs/brain-advanced-design.md` and `README.md` in the same PR** —
> the design doc for architecture/schema/pipeline/roadmap, and the README
> for the "🧠 Brain System" + "💾 Memory System" component listings, the
> Human-Brain ↔ AI-System ↔ RPG-Stat table, and the link to the design
> doc. See `rules/architecture-rules.md` rule 10.

## RAG Pipeline (Current)

Every chat message triggers:
1. **Hybrid 6-signal search** — `vector_similarity` (40%) + `keyword_match` (20%) + `recency_bias` (15%) + `importance` (10%) + `decay_score` (10%) + `tier_priority` (5%)
2. **RRF fusion** — vector + keyword + freshness retrievers fused via Reciprocal Rank Fusion (k=60)
3. **Optional HyDE** — LLM writes a hypothetical answer, embeds *that* for retrieval (cold/abstract queries)
4. **Optional cross-encoder rerank** — LLM-as-judge scores each (query, doc) pair 0–10
5. **Relevance threshold** — only entries above a configurable score threshold are injected
6. Top-k injected as `[LONG-TERM MEMORY]` block in system prompt
7. Keyword fallback when Ollama is unreachable

**Vector support:** Ollama `nomic-embed-text` (768-dim) locally, or cloud embedding API (`/v1/embeddings`) for paid/free modes. HNSW ANN index via `usearch` for O(log n) scaling to 1M+ entries.

## Skill Tree Quest System

Gamified feature discovery with 30+ skills across 5 categories (brain, voice, avatar, social, utility). Auto-detection: skills activate based on actual store state (e.g., `rag-knowledge` activates when brain is configured + memories exist). Combos unlock when multiple skills are active together.

## Key Patterns

- **Tauri commands**: All `async fn`, return `Result<T, String>`, use `#[tauri::command]`
- **State**: `AppState(Arc<AppStateInner>)` — cheaply clonable Arc newtype; all fields accessed via auto-`Deref`. Enables background servers (MCP, gRPC) to hold references without lifetime issues.
- **Streaming**: SSE via Tauri events (`llm-chunk`), parsed by `StreamTagParser` state machine
- **Error handling**: `?` operator, `thiserror` for typed errors, never `.unwrap()` in library code
- **Testing**: Vitest for frontend (1164 tests), `cargo test` for Rust (1075+ tests)
- **CSS**: Use `var(--ts-*)` design tokens from `src/style.css`, never hardcode hex colors

## Coding Standards

- No pretend/placeholder/TODO code — everything must compile and function
- Do not name code, commands, files, UI labels, docs, milestones, or persisted paths after third-party creators, channels, projects, products, mascots, or branded demos unless required by an imported dependency/public API. Use neutral descriptive names and keep attribution/license notes only in dedicated research/licensing docs.
- Always credit external authors, creators, channels/videos, open-source projects, papers, docs, datasets, tutorials, and reverse-engineered references in the top-level `CREDITS.md` whenever their work informs code, docs, roadmap, design/product insights, comparison matrices, prompt shapes, feature catalogues, or rejected decisions. No-code influence still counts: if TerranSoul learns from it, compares against it, or uses it to generate implementation insight, add or update a respectful `CREDITS.md` entry in the same PR (name, URL, license/terms when known, affected files/features, and what we learned/used). Keep rule text in `rules/coding-standards.md`; make `CREDITS.md` an appreciative public thanks page. Removing a referenced source removes its entry.
- When reverse-engineering any GitHub project, check `https://deepwiki.org/<owner>/<repo>` first when reachable, then cross-check with the upstream repository and credit the source in `CREDITS.md`. If DeepWiki is unavailable, record that blocker and proceed with direct upstream research.
- When a session learns a durable rule, convention, architecture fact, or reverse-engineering lesson, sync it into TerranSoul MCP self-improve knowledge in the same PR by updating `mcp-data/shared/memory-seed.sql` and, when MCP is running, using the brain tools to verify it is retrievable.
- Do not use Markdown files as TerranSoul MCP memory. Markdown may describe rules/docs or serve as a human-readable projection, but any durable MCP knowledge contained in Markdown must be synced into the SQLite-backed default dataset (`mcp-data/shared/memory-seed.sql`) and connected through `memory_edges` so the knowledge graph/database remains the source of truth.
- `snake_case` for Rust, `camelCase` for TypeScript
- `#[derive(Debug, Serialize, Deserialize, Clone)]` on all public Rust types
- Vue components use `<script setup lang="ts">` with scoped styles
- Tests required for all new functionality
- **Use existing libraries — don't reinvent the wheel** (see below)

## Use Existing Libraries First

Before writing any non-trivial functionality from scratch, **search for a well-maintained open-source crate or npm package** that already solves the problem. Only write custom code when no suitable library exists, or when the library would introduce unacceptable bloat or licensing issues.

### Decision checklist

1. **Search first** — check crates.io / npm / GitHub for existing solutions before coding.
2. **Prefer battle-tested** — choose libraries with active maintenance, >100 stars, and recent releases over writing your own.
3. **Prefer permissive licenses** — MIT, Apache-2.0, BSD, ISC, MPL-2.0 are fine. Avoid GPL/AGPL in library dependencies (Tauri app is not GPL).
4. **Prefer zero/low-dependency** — between two equal libraries, pick the one with fewer transitive dependencies.
5. **Wrap, don't fork** — if a library needs slight customization, write a thin wrapper. Don't copy-paste its source.

### Rust crate preferences

| Need | Use | Don't reinvent |
|---|---|---|
| HTTP client | `reqwest` | Custom TCP/TLS |
| JSON | `serde_json` | Manual parsing |
| Async runtime | `tokio` | Custom thread pools |
| CLI argument parsing | `clap` | Manual arg matching |
| Error handling | `thiserror` / `anyhow` | String-based errors |
| Logging | `tracing` | `println!` debugging |
| UUID generation | `uuid` | Custom ID schemes |
| Date/time | `chrono` or `time` | Manual epoch math |
| HTML parsing | `scraper` | Regex on HTML |
| URL parsing | `url` | Manual string splits |
| Embeddings / vectors | `usearch` or HNSW crate | Custom brute-force ANN |
| SQLite | `rusqlite` (bundled) | Raw FFI |
| Regex | `regex` | Hand-rolled matchers |
| Base64 / hex | `base64` / `hex` | Manual encode/decode |
| Crypto | `ring` / `ed25519-dalek` | Custom crypto primitives |

### Frontend (npm) preferences

| Need | Use | Don't reinvent |
|---|---|---|
| State management | `pinia` | Custom reactive stores |
| 3D rendering | `three` + `@pixiv/three-vrm` | WebGL from scratch |
| Markdown rendering | `marked` or `markdown-it` | Regex-based markdown |
| Date formatting | `Intl.DateTimeFormat` (built-in) | Custom date formatters |
| Drag & drop | `@vueuse/core` `useDraggable` | Manual pointer events |
| Keyboard shortcuts | `@vueuse/core` `useMagicKeys` | Manual keydown listeners |
| Clipboard | `@vueuse/core` `useClipboard` | Manual `navigator.clipboard` |
| Debounce/throttle | `@vueuse/core` | Hand-rolled timers |
| Chart/graph viz | `cytoscape` / `d3` | Canvas drawing from scratch |
| E2E testing | `@playwright/test` | Custom browser automation |
| Unit testing | `vitest` + `@vue/test-utils` | Custom test harness |

### When to write custom code

- The feature is truly domain-specific (e.g., TerranSoul's quest skill tree logic, VRM emotion pipeline, stream tag parser).
- No library exists for the exact need after a genuine search.
- The library adds >5 MB to bundle size for a trivial feature.
- The library is unmaintained (no commits in 2+ years, unpatched CVEs).
- Licensing conflict (GPL in a non-GPL project).

### Anti-patterns (don't do this)

- Writing a custom HTTP client when `reqwest` exists.
- Writing a custom JSON parser when `serde_json` exists.
- Writing a custom vector similarity search when `usearch` or cosine-distance crates exist.
- Writing a custom markdown renderer when `marked` exists.
- Copy-pasting Stack Overflow implementations of well-known algorithms instead of using a crate.
- Building a custom task queue when `tokio::spawn` + channels solve it.

## File References

- Architecture: `rules/architecture-rules.md`
- Standards: `rules/coding-standards.md`
- History: `rules/completion-log.md` (130+ completed chunks)
- Backlog: `rules/backlog.md`
- Milestones: `rules/milestones.md`

## Session Resumption & Progress Tracking

When starting a new session or resuming after a context-limit break:

1. **Read `rules/milestones.md`** — find the next `not-started` chunk.
2. **Read `rules/completion-log.md`** (top 50 lines) — see what was recently done.
3. **Check for in-progress work** — if a chunk is `in-progress`, resume it.
4. **Track progress** — use the todo list tool to track multi-step chunks.
5. **After completing a chunk** — archive it per the milestones enforcement rule:
   log in `completion-log.md`, remove from `milestones.md`, update docs if brain-related.

When the "Continue" prompt is received with no other context, follow steps 1–3 above.

### Long-Running Session Guidelines

- VS Code auto-summarizes conversation history when the context window fills up.
- The `chat.agent.maxRequests` is set to 100 in workspace settings.
- After each chunk, run the CI gate: `npx vitest run && cargo test && cargo clippy`.
- If a service is needed (Ollama, dev server), use `scripts/wait-for-service.mjs`.
- The `scripts/copilot-loop.mjs` script generates resume prompts with context.

### Mobile LAN / gRPC Emulator Testing

- For any change that affects mobile pairing, `RemoteHost`, gRPC, gRPC-Web, LAN discovery, TLS/cert trust, or phone-control chat/workflow RPCs, attempt a real free mobile runtime before closing the task: Android Emulator/AVD via the official Android SDK on Windows/macOS/Linux, and iOS Simulator via Xcode on macOS.
- Start the desktop host services locally, then verify the emulator/simulator can reach the host over LAN-style addressing and exercise the relevant gRPC/gRPC-Web path from the mobile frontend. Prefer real emulator/simulator runs over mocked unit tests for final confidence.
- If a required runtime is unavailable on the current OS (for example, iOS Simulator on Windows/Linux) or the SDK/emulator image is not installed, explicitly say what was attempted, include the blocking tool/output, and run the closest available fallback instead of claiming mobile LAN validation passed.

### MCP Server

TerranSoul exposes its brain via MCP on three ports — `7421` (release
app), `7422` (dev app), and `7423` (headless `npm run mcp` "pet mode"
runner used by AI coding agents). All three are wired into
`.vscode/mcp.json` as `terransoul-brain`, `terransoul-brain-dev`, and
`terransoul-brain-mcp`. Use the brain tools (`brain_search`,
`brain_ingest`, `brain_health`, etc.) to access the companion's memory
during coding sessions.

For dev/coding work, prefer the headless runner: Copilot setup steps run
`node scripts/copilot-start-mcp.mjs` automatically; manual sessions can run
`npm run mcp`. The headless brain starts on `127.0.0.1:7423` with state in
`<repo>/mcp-data/` (no collision with `npm run dev` or a running app, no
end-user data touched). It writes the current bearer token to `.vscode/.mcp-token`;
set `TERRANSOUL_MCP_TOKEN_MCP` from that file for VS Code's
`terransoul-brain-mcp` profile, verify `GET http://127.0.0.1:7423/health`,
then call `brain_health`. **Mandatory every-session rule:** every coding
agent must check/reuse/start TerranSoul MCP at session start, call
`brain_health` plus a relevant `brain_search` / `brain_suggest_context`
before broad repo exploration when MCP is available, and feed durable
self-improve lessons back into `mcp-data/shared/` or the rules/docs rather
than leaving them only in chat. If MCP is blocked, record the blocker in
the progress/final report. Full procedure, scope limits, and per-agent setup
live in [rules/agent-mcp-bootstrap.md](rules/agent-mcp-bootstrap.md) — read
it before invoking `npm run mcp`.
