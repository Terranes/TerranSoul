# TerranSoul Lessons Learned — Default MCP Knowledge Base

> Durable lessons distilled from `rules/completion-log.md` and prior agent
> sessions. Committed under `mcp-data/shared/` so the MCP brain can recall
> them and self-improve never re-solves the same problem twice.
>
> **Append a new entry whenever you discover a non-obvious trade-off, a
> retry-only bug fix, or an architectural decision worth keeping.** Keep each
> entry small and self-contained (problem → resolution → file pointer).

## Build & environment

- **Tauri requires Linux WebKit/GTK system deps before any `cargo` command on
  Linux**: `libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev
  libjavascriptcoregtk-4.1-dev pkg-config libglib2.0-dev libssl-dev`. The
  Copilot cloud agent installs these in `.github/workflows/copilot-setup-steps.yml`.
- **MCP cold-build cost**: `npm run mcp` first compiles the full Rust crate
  (~3-5 min); subsequent runs are warm thanks to `src-tauri/target`. Wait
  on `GET /health` via `scripts/wait-for-service.mjs` before issuing tool calls.
- **Detached background processes**: in this sandbox, prefer `setsid bash -c
  '...'` with stdin/stdout redirected to keep `npm run mcp` alive across
  short tool calls; plain `&` without redirection gets reaped.

## Frontend conventions

- ESLint enforces `vue/max-attributes-per-line`, self-closing void elements,
  and singleline-element newlines. Existing warnings are accepted as-is;
  do not bulk-rewrite unrelated files.
- `vue-tsc --noEmit` requires `npm ci` first because many `vue` / `@vue/*`
  module declarations resolve through `node_modules`.
- Use `var(--ts-*)` design tokens from `src/style.css`; never hardcode hex
  colors.
- Vue components use `<script setup lang="ts">` with scoped styles; no
  Options API.

## Rust conventions

- `#![deny(unused_must_use)]` is on at the crate root in
  `src-tauri/src/lib.rs`. Every `Result` in library code must be handled.
- `#[cfg(test)] mod foo { ... }` modules **must not be followed by other
  items in the same file** — clippy lint `items_after_test_module` rejects
  it. Keep test modules at the very bottom of `lib.rs`.
- Never `.unwrap()` in library code; use `?` and `thiserror`.
- `AppState(Arc<AppStateInner>)` is a cheaply-clonable Arc newtype with
  auto-`Deref`. Background servers (MCP, gRPC) hold references through it
  without lifetime headaches.

## Memory / brain

- Schema version is `13` (`src-tauri/src/memory/schema.rs`). `memories`
  table columns: `content, tags, importance, memory_type, created_at,
  last_accessed, access_count, embedding, source_url, source_hash,
  expires_at, tier, decay_score, session_id, parent_id, token_count,
  valid_to, obsidian_path, last_exported, category, updated_at,
  origin_device`.
- The MCP seed (`mcp-data/shared/memory-seed.sql`) is applied **only on
  first run** when `memory.db` does not yet exist. Existing runtime DBs
  must be re-ingested via `brain_ingest_*` tools when shared seed content
  changes.
- Hybrid 6-signal search weights live in `memory/store.rs`:
  vector(40%) + keyword(20%) + recency(15%) + importance(10%) + decay(10%)
  + tier(5%). RRF fusion uses k=60.
- HyDE and cross-encoder rerank are optional and configurable per query;
  default for cold/abstract queries is HyDE on, rerank on, threshold 0.

## MCP / agent integration

- Three MCP profiles in `.vscode/mcp.json`: `terransoul-brain` (release,
  7421), `terransoul-brain-dev` (dev, 7422), `terransoul-brain-mcp`
  (headless, 7423). Coding agents should use the headless profile so they
  do not collide with a running app's data.
- Bearer token is regenerated when missing; it is written to both
  `mcp-data/mcp-token.txt` and `.vscode/.mcp-token`. Set
  `TERRANSOUL_MCP_TOKEN_MCP` from the file for the headless profile.
- Verify MCP with `GET /health` (no auth required) before calling `brain_health`.

## Self-improve

- Self-improve runs in temporary git worktrees so the main checkout is
  never disturbed. Always read `rules/milestones.md` for the next chunk
  and `rules/completion-log.md` for the most recent context before
  starting work.
- Self-improve sessions append durable knowledge to
  `mcp-data/shared/memory-seed.sql`, `project-index.md`, and this file
  whenever a learning generalizes beyond the current chunk.
- Per the brain documentation sync rule, any change to brain surface
  (LLM providers, memory store, RAG pipeline, ingestion, embeddings,
  cognitive-kind classification, knowledge graph, decay/GC, brain-gating
  quests, brain Tauri commands or Pinia stores) must update both
  `docs/brain-advanced-design.md` and `README.md` in the same PR.

## CI / GitHub

- The CI gate is `npm ci && npm run lint && npx vue-tsc --noEmit && npx
  vitest run && npm run build && (cd src-tauri && cargo clippy
  --all-targets -- -D warnings && cargo test --all-targets)`.
- `clippy::items_after_test_module` and a few brain-doc-sync checks have
  caused PR failures historically; surface them early.
- GitHub Actions workflows on the agent's first push may show
  `action_required` rather than running automatically — that requires
  human approval and is not a code defect.

## Skill tree & quests

- The skill tree (~1500 lines in `src/stores/skill-tree.ts`) auto-detects
  active skills from store state (e.g., `rag-knowledge` activates when
  brain is configured + memories exist). Combos unlock when multiple
  skills are active together.

## Persona & motion

- Motion clip parser/validator lives in `src-tauri/src/persona/motion_clip.rs`;
  motion tokens use the MotionGPT codec (`motion_tokens.rs`).
- Pose frames stream through the `<pose>` tag in `StreamTagParser` and
  emit an `llm-pose` event consumed by `PoseAnimator` in the frontend.
- ARKit blendshape passthrough is the canonical face rig; expanded set
  documented in completion-log Chunk 27.3.

## Sync & devices

- Device identity is Ed25519 (`src-tauri/src/identity/`). Trusted devices
  are persisted in a registry; LAN gRPC enforces mTLS to paired devices.
- CRDT primitives live in `src-tauri/src/sync/` (LWW register, OR-Set,
  append log). Soul Link wire protocol is documented under chunks 17.5a/b.

## Plugins & sandbox

- Plugins run in a WASM sandbox (`src-tauri/src/sandbox/wasm_runner.rs`)
  with explicit capability gating (`capability.rs`).
- Plugin commands dispatch through `commands/plugins.rs` and require
  capability grants prompted via `usePluginCapabilityGrants`.

## What NOT to do

- Don't reintroduce Cypress — the project standardized on Vitest +
  Playwright (`docs/licensing-audit.md`).
- Don't commit MCP runtime state: `mcp-token.txt`, `memory.db`,
  `*.db-shm`, `*.db-wal`, `tasks.db*`, `workflows.sqlite`, `*.idx`,
  `*.lock`, `sessions/`, `worktrees/`.
- Don't recreate `mcp-data-seed/` — it was renamed to
  `mcp-data/shared/`.
- Don't bulk-rewrite unrelated lint warnings; address only what your
  change touches.
- Don't add `console.log` debugging in shipped code; use the existing
  logger.
