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
- **Install missing MCP/app dependencies before declaring a blocker**: if
  `npm run mcp`, `npm run dev`, `cargo tauri dev`, or validation fails because
  `pkg-config` cannot find Linux Tauri libraries (`glib-2.0.pc`, `gio-2.0.pc`,
  WebKit/GTK, etc.), install the missing platform packages and retry before
  reporting MCP/app startup as blocked. On Ubuntu cloud agents, the minimum set
  is `libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev
  libappindicator3-dev librsvg2-dev patchelf libsoup-3.0-dev
  libjavascriptcoregtk-4.1-dev pkg-config`.
- **MCP cold-build cost**: `npm run mcp` first compiles the full Rust crate
  (~3-5 min); subsequent runs are warm thanks to `src-tauri/target`. Wait
  on `GET /health` via `scripts/wait-for-service.mjs` before issuing tool calls.
- **Detached background processes**: in this sandbox, prefer `setsid bash -c
  '...'` with stdin/stdout redirected to keep `npm run mcp` alive across
  short tool calls; plain `&` without redirection gets reaped.
- **Copilot MCP autostart**: `.github/workflows/copilot-setup-steps.yml` runs
  `node scripts/copilot-start-mcp.mjs` after `npm ci`. If a session starts
  without MCP on 7421/7422/7423, fix setup/autostart rather than silently
  proceeding without `brain_health` + relevant `brain_search`.

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

- Schema version is `20` (`src-tauri/src/memory/schema.rs`). `memories`
  table columns: `content, tags, importance, memory_type, created_at,
  last_accessed, access_count, embedding, embedding_model_id,
  embedding_dim, source_url, source_hash, expires_at, tier, decay_score,
  session_id, parent_id, token_count, valid_to, obsidian_path,
  last_exported, category, cognitive_kind, updated_at, origin_device,
  hlc_counter, protected, share_scope, confidence`.
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
  7421), `terransoul-brain-dev` (dev, 7422), and `terransoul-brain-mcp`
  (MCP tray/coding-agent runtime, 7423). Coding agents should use the MCP
  profile unless a running release/dev app is already serving MCP.
- `brain_failover_status` is diagnostics-only and requires provider-rotator
  AppState wiring. In headless/stdio MCP runs without that state attachment,
  it may return `failover status requires app state`; this does not block
  core memory/RAG tools.
- Bearer token is regenerated when missing; it is written to both
  `mcp-data/mcp-token.txt` and `.vscode/.mcp-token`. Set
  `TERRANSOUL_MCP_TOKEN_MCP` from the file for the MCP profile.
- Verify MCP with `GET /health` (no auth required) before calling `brain_health`.

## Seed knowledge audit (2026-05-10)

Durable lesson synced into `mcp-data/shared/memory-seed.sql`.

- **LESSON: Seed knowledge audit (2026-05-10) corrected stale canonical facts: schema is V20 (not V13/V15/V19 in older notes) and brain_failover_status in headless MCP may return "failover status requires app state" when provider-rotator AppState is absent; this is diagnostics-only and does not block core memory/RAG tools.**

## Intent routing no-heuristics rule (2026-05-10)

Durable lesson synced into `mcp-data/shared/memory-seed.sql`.

- **RULE: Document/setup intent routing must remain classifier + RAG driven.** On malformed/unknown classifier JSON, return `Unknown` and continue normal chat/install flow. Do not force `learn_with_docs`, `teach_ingest`, or `gated_setup` via regex/contains/includes/keyword arrays.

## Local E2E latency budget (2026-05-10)

Durable lesson synced into `mcp-data/shared/memory-seed.sql`.

- **RULE: Local E2E response latency budget (2026-05-10): Playwright tests outside GitHub Actions must fail any assistant/LLM response latency above 2 seconds with an investigation-focused failure message.** Keep diagnostic wait timeouts long enough to collect evidence, but do not resolve latency regressions by increasing Playwright timeouts or relaxing assertions; investigate model warmup, VRAM contention, RAG retrieval, embedding backfill, provider selection, streaming first chunk, and UI state propagation.

## Self-improve

## Tutorial screenshot and mobile keyboard lessons (2026-05-10)

Durable lesson synced into `mcp-data/shared/memory-seed.sql`.

- **LESSON: Tutorial screenshot refresh must be captured and verified step-by-step, not by blind batch scripts.** For each referenced tutorial image, open the exact target view, dismiss overlays, confirm mode state, capture, and visually verify before moving on. If a screenshot reveals a UI defect, fix the UI/state first and recapture immediately.
- **LESSON: Mobile black-strip and missing-input bug root cause was false keyboard detection in src/composables/useKeyboardDetector.ts.** Plain viewport resizes must not trigger keyboard offset unless an editable element is focused and `visualViewport` is actually reduced versus the layout viewport.

## Agent-session lesson ingestion gap (2026-05-10)

Durable lesson synced into `mcp-data/shared/memory-seed.sql`.

- **GAP: TerranSoul self-improve has no ingestion path for lessons learned by an EXTERNAL coding agent (Copilot/Claude Code/Codex) operating in the main checkout.** Until `brain_ingest_lesson` is fully wired, durable agent-session lessons need a new INSERT row appended to `mcp-data/shared/memory-seed.sql`, a matching `lessons-learned.md` entry, and retrievability verification before completion.

- Self-improve runs in temporary git worktrees so the main checkout is
  never disturbed. Always read `rules/milestones.md` for the next chunk
  and `rules/completion-log.md` for the most recent context before
  starting work.
- **Milestone hygiene is mandatory**: `rules/milestones.md` contains only
  `not-started` and `in-progress` chunks. When a chunk is complete, add full
  details to `rules/completion-log.md`, remove the done row from
  `milestones.md`, drop empty phase headings, and update `Next Chunk`.
- **Do not start from backlog**: `rules/backlog.md` is a holding area only.
  If no milestone chunks remain, stop and ask the user which backlog item to
  promote before editing either file.
- Self-improve sessions append durable knowledge to
  `mcp-data/shared/memory-seed.sql`, `project-index.md`, and this file
  whenever a learning generalizes beyond the current chunk.
- MCP-mode self-improve runtime logs stay under `mcp-data/` and are bounded:
  `self_improve_runs.jsonl`, `self_improve_gates.jsonl`, and
  `self_improve_mcp.jsonl` keep only the current file plus `.001`, capped at
  1 MiB per file. Durable lessons must still be mirrored into shared seed SQL.
- MCP startup on `7423` must enforce target freshness: when
  `target-mcp/release/terransoul(.exe)` is older than `src-tauri` source/config,
  do not reuse the running process. Required flow is terminate, rebuild,
  relaunch, then `/health` check; if termination fails, report a blocker.
- Shared seed bootstrap must resolve in this order across release/dev/MCP:
  `TERRANSOUL_MCP_SHARED_DIR` -> `<data_dir>/shared` -> `<cwd>/mcp-data/shared`
  -> compiled SQL fallback. This prevents dev/release drift when runtime data
  directories do not contain a `shared/` folder but the repo dataset exists.
- Per the brain documentation sync rule, any change to brain surface
  (LLM providers, memory store, RAG pipeline, ingestion, embeddings,
  cognitive-kind classification, knowledge graph, decay/GC, brain-gating
  quests, brain Tauri commands or Pinia stores) must update both
  `docs/brain-advanced-design.md` and `README.md` in the same PR.
- Rule coverage belongs in MCP too: when an agent misses or skips a rule from
  `rules/`, add a concise high-importance row to
  `mcp-data/shared/memory-seed.sql` plus a short note in `project-index.md` or
  this file so future `brain_search` retrieves it.
- **DeepWiki-first reverse engineering**: when studying any GitHub project,
  check `https://deepwiki.org/<owner>/<repo>` first when reachable, then
  cross-check against the upstream repository and license. If DeepWiki is
  blocked, record the blocker. Credit any learned source in `CREDITS.md` and
  sync durable lessons into `mcp-data/shared/**` so MCP self-improve can recall
  them.
- **MCP self-learning is reviewable source, not chat memory**: when a user adds
  a durable rule or an agent learns a reusable convention, update
  `mcp-data/shared/memory-seed.sql`, `lessons-learned.md`, or
  `project-index.md` in the same PR. Runtime `memory.db` may be refreshed by
  MCP tools, but tracked shared files are the durable default dataset.
- **MCP use must be visible**: after `brain_health` and the relevant
  `brain_search` / `brain_suggest_context`, agents must show a short receipt
  naming the health/provider result and query topic. Hidden tool calls alone do
  not complete preflight.
- **Markdown is not MCP memory**: rules/docs/lessons Markdown can describe
  knowledge for humans, but durable MCP knowledge must also be synced into
  `mcp-data/shared/memory-seed.sql` and connected with `memory_edges`.
  Markdown-only rule or architecture knowledge is incomplete because future
  agents must retrieve it through SQLite/FTS/RRF/KG, not by loading `.md`
  files as memory.
- **Self-improve does NOT yet capture lessons from external coding-agent
  sessions** (Copilot/Claude Code/Codex working in the main checkout).
  `coding/conversation_learning.rs` only routes user-authored chat messages to
  `rules/milestones.md`; `coding/engine.rs` runs in an isolated worktree
  against the configured Coding LLM. Until `brain_ingest_lesson` MCP tool
  ships and `agent_session_lessons.rs` lands, the agent **must append a new
  INSERT row to `mcp-data/shared/memory-seed.sql` plus update this
  document** when it discovers a procedural rule. Verify with `brain_search`
  before declaring done.
- **Custom CSS token references must match a definition in EVERY theme
  block** of `src/style.css`. The 2026-05-10 chat-bar dropdown contrast bug
  was caused by `var(--ts-text, #e2e8f0)` and `var(--ts-bg-base, #0f172a)` in
  `.reasoning-effort-select` — `--ts-text` was never defined per theme so the
  dark fallback bled through on light themes. Use `--ts-text-primary` and
  `--ts-bg-surface` plus `color-scheme: inherit` so the native popup adopts
  the active theme automatically.
- **Chat input is a multi-line auto-grow textarea, never `<input
  type="text">`.** Long messages clip horizontally and become unreadable on
  small screens. The pattern is documented in `src/components/ChatInput.vue`:
  `rows="1"`, `resize:none`, `max-height: calc(1.4em * MAX_ROWS + padding)`,
  `overflow-y:auto`, and `autoResize()` on `@input` that resets height to
  `auto` then sets `min(scrollHeight, maxHeight)`. Submit on Enter without
  Shift; Shift+Enter inserts a newline.

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
- **Don't propose "store memories as `.md` / Obsidian as the source of
  truth"**. See `mcp-data/shared/memory-philosophy.md` — markdown is
  for instructions; SQLite + vector search + `memory_edges` is the source of
  truth; `obsidian_export.rs` is a one-way projection. This is a
  non-negotiable architectural rule absorbed from Jonathan Edwards'
  "Stop Calling It Memory" essay.
- **Don't copy source/prompts/skill markdown/scheduler scripts/asset
  names from `kbanc85/claudia`** — PolyForm Noncommercial 1.0.0
  forbids redistribution. Adopt patterns/product ideas from the public
  README only (see `mcp-data/shared/claudia-research.md`). Use neutral
  TerranSoul names; never ship literal `/meditate`, "morning brief",
  or any branded label.


## OpenAgentd audit (2026-05-10) — coding workflow patterns

Full audit in `docs/openagentd-audit.md`; durable rows synced into
`mcp-data/shared/memory-seed.sql`. Apache-2.0
attribution recorded in `CREDITS.md`. Phase 47 in `rules/milestones.md`
groups the implementable chunks.

- **LESSON: Long tool results must spill to disk, not into the conversation. OpenAgentd ToolResultOffloadHook writes any result above ~40 KB / ~10 K tokens to {workspace}/{agent_name}/.tool_results/{tool_call_id}.txt and replaces the in-context value with a head/tail preview plus the file path.** TerranSoul should write large results to `<worktree>/.terransoul/tool_results/<id>.txt`, replace the in-message value with a head/tail preview plus a path the agent can `read`. Never break
  execution on offload write failure — log a warning and pass the original
  through.
- **Shell output > ~128 KB must spill** to `.shell_output/<id>.txt` and
  reply with last 200 lines + reference. Spawn child processes in a new
  process group and kill the **group** on cancel/timeout, otherwise children
  like `node` under `npm run dev` survive the parent.
- **Cancellation must reach in-flight tool calls**, not only DAG
  boundaries. `coding::engine`'s `Arc<AtomicBool>` is checked between nodes
  only; add a `tokio::sync::watch::Sender<bool>` per run and wrap each tool
  task in `tokio::select!{ tool, _ = watch.changed() }`.
- **Heal orphaned tool calls before the next user turn.** When the app dies
  between persisting an `assistant{tool_calls}` row and the matching tool
  reply rows, the next OpenAI Responses turn 400s with "No tool output
  found for function call". Insert a synthetic
  `tool: "Tool execution was interrupted before a result could be recorded."`
  for every orphaned id, anchored to `last_assistant.created_at + 1µs * (i+1)`
  so input order stays `assistant → tool → … → user`. Also drop tool calls
  with truncated `arguments` JSON on deserialize.
- **Agent loop hooks must be chain-of-responsibility, not branches.** Add a
  runtime `AgentHook` trait in `coding/runtime_hooks.rs` with
  `before_model`, `after_model`, `wrap_tool_call`, `on_chunk`. `wrap_tool_call`
  is the chain — tool offload, OTel, audit, summary injection are composable
  units. Wrap every hook call in a `_safe_invoke_hooks` equivalent
  (`tracing::warn!` with hook name on error) so a buggy hook never crashes
  a turn.
- **A request snapshot is frozen — hooks that mutate state must rebuild it.**
  If a hook adds to `state.messages` (RAG block, inbox message, summary), it
  must return a new request object; otherwise the LLM sees the pre-hook
  snapshot. Make this a doctest on the `AgentHook` trait so it cannot be
  forgotten.
- **Rolling-window summarization needs cross-resume token seeding.**
  `last_prompt_tokens` resets to 0 on every `agent.run()` call. Seed it from
  persisted usage in `coding::session_chat` at load time, otherwise turn
  N+1 of a multi-request session never compresses.
- **Three-tier wiki (`USER.md` always-injected + BM25-searchable topics +
  per-day append-only notes) plus a scheduled "dream" agent** is the right
  long-term shape for turning ephemeral chat notes into durable knowledge.
  Deferred until the runtime hook framework lands so the dream agent stays
  composable.
- **Sandbox needs a secrets denylist** (`**/.env`, `**/secrets/**`,
  `**/*.pem`, `**/id_rsa*`) **plus a tokenised shell pre-flight** that
  resolves path-like tokens against the workspace and rejects denied roots.
  Best-effort only — `$VAR`, `$(...)`, backticks, base64 are not evaluated;
  OS-level user permissions remain the last line of defence.
- **Tool schemas must be sanitised per provider.** Inline `$ref` and drop
  `$defs` for Gemini/Vertex AI; recursively strip `discriminator`, `const`,
  `exclusiveMinimum/Maximum`, `additionalProperties`, `$schema`, `$id`,
  `contentEncoding`, `contentMediaType` for Gemini specifically. Otherwise
  free-mode MCP tool calls silently 400 with `INVALID_ARGUMENT`.
- **Tool-call streaming is three-phase** (`tool_call` → `tool_start` →
  `tool_end`) and the same `tool_call_id` must flow through all three.
  Verify `StreamTagParser` preserves the contract for parallel calls; some
  providers mis-index parallel calls and need a `ToolIdResolver`-style
  stream-scoped id→index map.
- **Live config drift detection avoids restart-on-edit.** Stamp the mtimes
  of every loaded skill/persona file and re-parse + swap in place at
  end-of-turn. Parse failures keep the previous version + re-stamp to avoid
  looping.
- **Multi-agent default: lead-as-translator + verify-before-claim.** Members
  describe needs in plain language; the lead translates to registry names.
  Members must read the result of every tool call before claiming success
  and follow a write with a cheap `ls` / `read` before reporting completion.
  The lead must sanity-check member "done" claims with a cheap read when
  feasible. Codify in `coding::multi_agent` prompts and
  `rules/prompting-rules.md`.

## Live chat RAG parity (2026-05-10)

Durable lesson synced into `mcp-data/shared/memory-seed.sql`.

- **RAG pipeline: contentful live chat uses fast-path skip for short/empty turns, thresholded hybrid 6-signal eligibility, then RRF + query-intent ordering for top-5 prompt injection.** Keep the relevance threshold as the
  eligibility gate; do not compare RRF scores directly to that user setting.
  Live desktop chat and paired-mobile chat should share the same helper so
  prompt memories stay behaviorally aligned. Local Ollama streaming should
  preserve the VRAM guard by passing no query embedding on the hot path, while
  free/paid modes may include cloud embeddings.