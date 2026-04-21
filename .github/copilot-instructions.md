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

## File References

- Architecture: `rules/architecture-rules.md`
- Standards: `rules/coding-standards.md`
- History: `rules/completion-log.md` (130+ completed chunks)
- Backlog: `rules/backlog.md`
- Milestones: `rules/milestones.md`
