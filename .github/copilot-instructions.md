# TerranSoul — Copilot Instructions

> This file is auto-loaded by GitHub Copilot on every request.
> Last updated: 2026-04-22

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
| DB | SQLite via rusqlite (bundled) |
| LLM | Ollama (local), OpenAI-compatible APIs (cloud), Pollinations (free) |

## Architecture

```
Frontend (WebView — Vue 3 + TS)
  ├── Three.js/VRM Renderer (CharacterViewport)
  ├── Vue Components + Pinia Stores
  │   ├── brain.ts — LLM provider management
  │   ├── conversation.ts — chat history + streaming
  │   ├── memory.ts — persistent memory CRUD
  │   ├── skill-tree.ts — gamified quest system (1500+ lines)
  │   ├── voice.ts — TTS/ASR configuration
  │   └── settings.ts — app preferences
  └── Design system: CSS custom properties (--ts-* tokens in style.css)
      ↕ Tauri IPC (invoke / emit)
Rust Core Engine (src-tauri/src/)
  ├── commands/ — 60+ Tauri commands (chat, streaming, memory, brain, voice, window, etc.)
  ├── brain/ — LLM providers: OllamaAgent, OpenAiClient, FreeProvider, ProviderRotator
  │   ├── model_recommender.rs — RAM-based model catalogue (Gemma 4, Phi-4, Kimi K2.6 cloud)
  │   └── ollama_agent.rs — semantic_relevant_ids() for RAG ranking
  ├── memory/ — SQLite store + brain_memory.rs (LLM-powered extract/summarize/search)
  ├── identity/ — Ed25519 device identity for P2P linking
  ├── link/ — CRDT sync engine (QUIC/WebSocket)
  └── orchestrator/ — Agent routing with capability gates
```

## Brain Modes

1. **Free API** — Pollinations AI, auto-configured, no key needed
2. **Paid API** — OpenAI/Anthropic/Groq with user-supplied API key
3. **Local Ollama** — Private, offline-capable, hardware-adaptive model selection

## RAG Pipeline (Current)

Every chat message triggers:
1. `get_all()` — load all memories from SQLite
2. `semantic_search_entries()` — LLM ranks relevance (sends ALL entries in one prompt)
3. Top 5 injected as `[LONG-TERM MEMORY]` block in system prompt
4. Keyword fallback when Ollama is unreachable

**Known limitation**: Brute-force LLM ranking — sends all memories in one prompt. Scales to ~500 entries. For 1000+ entries, needs vector embedding + ANN search upgrade.

## Skill Tree Quest System

Gamified feature discovery with 30+ skills across 5 categories (brain, voice, avatar, social, utility). Auto-detection: skills activate based on actual store state (e.g., `rag-knowledge` activates when brain is configured + memories exist). Combos unlock when multiple skills are active together.

## Key Patterns

- **Tauri commands**: All `async fn`, return `Result<T, String>`, use `#[tauri::command]`
- **State**: `AppState` holds `Mutex<MemoryStore>`, `Mutex<BrainStore>`, etc.
- **Streaming**: SSE via Tauri events (`llm-chunk`), parsed by `StreamTagParser` state machine
- **Error handling**: `?` operator, `thiserror` for typed errors, never `.unwrap()` in library code
- **Testing**: Vitest for frontend (941+ tests), `cargo test` for Rust (514+ tests)
- **CSS**: Use `var(--ts-*)` design tokens from `src/style.css`, never hardcode hex colors

## Coding Standards

- No pretend/placeholder/TODO code — everything must compile and function
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
