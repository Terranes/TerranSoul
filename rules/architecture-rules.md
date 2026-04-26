# Architecture Rules

> All architecture decisions must satisfy the Quality Pillars in `rules/quality-pillars.md`.

## General Principles

1. **Separation of Concerns** — Each module has a single, well-defined responsibility.
   Frontend owns rendering and UI state; Rust core owns business logic, IPC, and data persistence.
2. **Dependency Inversion** — Depend on abstractions, not concrete implementations.
   Agent integrations implement a common `AgentProvider` trait; the orchestrator never knows which concrete agent it talks to.
3. **Single Codebase, All Platforms** — Tauri 2.0 is the unified shell.
   One Rust backend + one Vue 3 frontend runs on Windows, macOS, Linux, iOS, and Android with zero platform-specific UI forks.
4. **Async-First Rust** — All Tauri commands are `async`. Use Tokio for all I/O and concurrency.
   Never block the main thread.
5. **CRDT-based Sync** — TerranSoul Link uses CRDTs for conflict-free merge of conversations,
   settings, and agent state across all paired devices. No central server dependency.
6. **End-to-End Encrypted Link** — All inter-device traffic is end-to-end encrypted using Ed25519
   key pairs established during device pairing. No plaintext transport.
7. **Capability-Based Agent Permissions** — Every agent declares its capabilities in its manifest.
   The orchestrator only routes to an agent for capabilities it has declared. Sensitive capabilities
   require explicit user confirmation.
8. **Offline-First** — Every core feature (chat, 3D character, installed agents) must work without
   a network connection. TerranSoul Link sync is additive, not a prerequisite.
9. **Performance Budgets** — Cap `devicePixelRatio` at 2 in the Three.js renderer.
   Keep VRM model polygon count < 100k tris for Phase 1 real-time performance.
10. **Brain Component Reference** — Any implementation that touches the brain
    (LLM provider, memory, RAG, ingestion, embeddings, cognitive-kind axis,
    skill-tree quests that gate brain capabilities, etc.) **must** consult
    `docs/brain-advanced-design.md` first to identify the brain components
    involved and their dependency order. The doc is the source of truth for
    which pieces (free/paid/local LLM, long-term memory, RAG, Scholar's Quest,
    embedding model, ingestion engine) are required for a given feature, and
    in what order they need to be active. New features must reuse the
    existing components / quest chain rather than reimplementing parts of
    the brain from scratch.
11. **Brain Documentation Sync (mandatory)** — Any change that touches the brain
    surface (LLM providers, memory store, RAG pipeline, ingestion, embeddings,
    cognitive-kind classification, knowledge graph, decay/GC, brain-gating
    quests, brain-related Tauri commands or Pinia stores) **must** update
    **both** of the following in the same PR:
    - `docs/brain-advanced-design.md` — keep the design / schema / pipeline
      / roadmap / April 2026 research-survey sections accurate, including
      the Brain Component Selection / Routing section (§20) that explains
      how the LLM and the orchestrator decide which component to use for
      a given turn.
    - `README.md` — keep the "🧠 Brain System" and "💾 Memory System"
      component listings, the Human-Brain ↔ AI-System ↔ RPG-Stat mapping
      table, and the link to `docs/brain-advanced-design.md` in sync with
      reality.
    A brain PR that updates code without updating these two documents is
    incomplete and must not be merged. Reviewers should reject such PRs.
12. **Persona Component Reference** — Any implementation that touches the
    persona surface (persona traits, learned-expression / learned-motion
    libraries, persona Pinia store, persona-prompt builder, persona Tauri
    commands, persona quest chain, camera capture composables, MediaPipe
    FaceLandmarker / PoseLandmarker integration, ARKit-blendshape → VRM
    expression mapping) **must consult `docs/persona-design.md` first**
    to determine which subsystems are involved and how the per-session
    camera consent contract works. New features must reuse the existing
    persona components and the main-chain → side-chain quest layout
    rather than reimplementing parts of the persona surface.
13. **Persona Documentation Sync (mandatory)** — Any change that touches
    the persona surface (the items listed in rule 12) **must** update
    **both** of the following in the same PR:
    - `docs/persona-design.md` — keep the design / schema / pipeline /
      roadmap / April 2026 research-survey / quest-chain / privacy-contract
      sections accurate.
    - `README.md` — keep the "🎭 Persona System" component listing, the
      Human-Soul ↔ AI-System ↔ RPG-Stat row for persona, and the link to
      `docs/persona-design.md` in sync with reality.
    A persona PR that updates code without updating these two documents
    is incomplete and must not be merged. Reviewers should reject such
    PRs.

---

## Layer Boundaries

```
┌──────────────────────────────────────────────────────────────┐
│  Frontend (WebView — Vue 3 + TypeScript)                     │
│  ┌──────────────────┐  ┌───────────────────────────────────┐ │
│  │  Three.js / VRM  │  │  Vue Components + Pinia Stores    │ │
│  │  Renderer        │  │  (ChatView, CharacterViewport …)  │ │
│  └──────────────────┘  └───────────────────────────────────┘ │
│                  ↕  Tauri IPC (invoke / emit)                │
├──────────────────────────────────────────────────────────────┤
│  Rust Core Engine (src-tauri)                                │
│  ┌───────────────┐ ┌─────────────────┐ ┌──────────────────┐ │
│  │  Commands     │ │  Orchestrator   │ │  TerranSoul Link │ │
│  │  (chat,       │ │  (route tasks   │ │  (QUIC/WS sync,  │ │
│  │  agent, char) │ │  to agents)     │ │  CRDT engine)    │ │
│  └───────────────┘ └─────────────────┘ └──────────────────┘ │
│  ┌───────────────┐ ┌─────────────────┐ ┌──────────────────┐ │
│  │  AI Package   │ │  Plugin Loader  │ │  AI Integrations │ │
│  │  Manager      │ │  (WASM sandbox, │ │  (MCP server,    │ │
│  │               │ │  Phase 3)       │ │  BrainGateway)   │ │
│  └───────────────┘ └─────────────────┘ └──────────────────┘ │
├──────────────────────────────────────────────────────────────┤
│  AI Agents (separate processes / services)                   │
│  OpenClaw │ Claude Cowork │ local LLM │ stub agent           │
└──────────────────────────────────────────────────────────────┘
```

---

## Module Dependency Rules

- `src/types/` — zero dependencies; pure TypeScript interfaces and type aliases
- `src/stores/` — depends on `src/types/` and `@tauri-apps/api/core` only
- `src/renderer/` — depends on `three`, `@pixiv/three-vrm`, and `src/types/` only
- `src/components/` — depends on `src/stores/`, `src/types/`, and `src/renderer/`
- `src/views/` — depends on `src/components/` and `src/stores/`
- Rust `commands/` — depends on internal `orchestrator/` and `agent/` modules only
- Rust `orchestrator/` — depends on `agent/` trait abstraction; never on concrete agent implementations directly
- Rust `agent/` — each provider (stub, OpenClaw, etc.) implements the `AgentProvider` trait; no cross-provider dependencies
- Rust `link/` — depends on `commands/` state types; never on `agent/` internals
- Rust `ai_integrations/gateway.rs` — defines `BrainGateway` trait; `AppStateGateway` adapter holds `AppState` (cheaply-clonable Arc newtype)
- Rust `ai_integrations/mcp/` — MCP server (axum HTTP, JSON-RPC 2.0); depends on `BrainGateway` trait only, never on `AppState` internals directly
- Rust `memory/backend.rs` — defines `StorageBackend` trait; depends only on `memory/store.rs` types
- Rust `memory/postgres.rs`, `memory/mssql.rs`, `memory/cassandra.rs` — feature-gated backends; implement `StorageBackend`
- Rust `memory/cognitive_kind.rs` — pure-function classifier for the
  episodic/semantic/procedural axis; depends only on `MemoryType`. See
  `docs/brain-advanced-design.md` § 3.5 for the design rationale.
- Rust `agent/openclaw_agent.rs` — reference `AgentProvider` implementation
  bridging an external platform (OpenClaw) with capability-gated tool
  dispatch. See `instructions/OPENCLAW-EXAMPLE.md`.
- Rust `registry_server/catalog_registry.rs` — in-process default
  `RegistrySource` so the Marketplace browse tab is populated without
  starting the registry HTTP server. The HTTP server remains optional and is
  swapped in via `start_registry_server`.

---

## Communication Patterns

| Channel | Direction | Protocol |
|---------|-----------|----------|
| UI → Rust | Command | Tauri `invoke()` (async, typed) |
| Rust → UI | Event | Tauri `emit()` (streaming updates) |
| Rust → Agent (local) | IPC | JSON-RPC over stdin/stdout |
| Rust → Agent (remote) | IPC | gRPC (streaming preferred) |
| Device ↔ Device | Sync | QUIC (primary) / WebSocket+TLS (fallback) |

---

## Conversation Data Model

Every message in the conversation follows this canonical structure:

```typescript
interface Message {
  id: string;          // UUID v4
  role: 'user' | 'assistant';
  content: string;
  agentName?: string;  // which agent produced this message
  timestamp: number;   // Unix ms UTC
}
```

Messages are **immutable** once created. Edits produce a new message with a new `id`.

---

## 3D Rendering Rules

- Use `WebGPU` when `navigator.gpu` is available; fall back to `WebGL2`
- One `THREE.WebGLRenderer` (or `WebGPURenderer`) per `CharacterViewport` instance
- `clock.getDelta()` drives VRM update loop — never use wall-clock deltas directly
- All resource disposal (renderer, scene, geometries, textures) must happen in the `dispose()` function returned by `initScene()`
- Avatar format priority: VRM 1.0 → glTF 2.0 → procedural capsule placeholder

---

## Security Rules

- No secrets or API keys in source code or committed configuration
- All Tauri capabilities scoped to minimum required (no `shell:execute` unless explicitly approved)
- VRM / glTF files are loaded from user-selected paths only; never auto-loaded from arbitrary URLs
- Agent manifests are verified (hash/signature) before installation
- TerranSoul Link traffic is end-to-end encrypted; keys never leave their device
- All user-provided input is validated before passing to agents

---

## Observability Rules

- Rust `tracing` crate for structured logging in the backend; `tracing-subscriber` for output
- Frontend uses `console.warn` / `console.error` only for genuine errors — no debug spam
- Tauri commands return `Result<T, String>` — the `Err` variant carries a structured error message
- Character state transitions are logged at `tracing::debug!` level for diagnosability
