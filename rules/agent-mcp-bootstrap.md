# Agent Bootstrap Rule — `npm run mcp`

> **Audience:** every AI coding agent that works in this repo —
> GitHub Copilot, Codex CLI, Claude Code, Clawcode, Cursor, Continue.dev,
> Aider, and any future MCP-aware assistant.
>
> **Status:** mandatory. Treat this file the same way you treat
> `.github/copilot-instructions.md`.

## 1. What `npm run mcp` is

A **local MCP tray runtime** that exposes TerranSoul's brain
(memory store, RAG, knowledge graph, gitnexus surface) to AI coding
agents over JSON-RPC on `http://127.0.0.1:7423/mcp` while keeping a
visible system-tray icon and reopenable UI handle.

Internally it is the Rust binary launched as
`target-mcp/release/terransoul.exe --mcp-tray`, which puts the
process into **MCP pet mode with a visible system-tray icon**:

> **Local-run rule:** every local session must use `npm run mcp`
> (which routes through `scripts/copilot-start-mcp.mjs` and passes
> `--mcp-tray`). `mcp-http` is no longer the local workflow. Agents
> must attach to an existing release/tray/dev server when one is open;
> only start the tray when no authenticated TerranSoul MCP server is
> already available.

- `serverInfo.name` = `terransoul-brain-mcp`
- `serverInfo.buildMode` = `mcp` (not `dev`, not `release`)
- Bound to **127.0.0.1:7423** (loopback only, never LAN)
- Persists runtime state in **`<repo>/mcp-data/`**
- Bearer token at **`<repo>/mcp-data/mcp-token.txt`**
- Loads committed shared seed data from **`<repo>/mcp-data/shared/`**

It is intentionally a **separate process** with a **separate data
dir** and a **separate port** so it never collides with:

| Surface | Port | Data dir |
|---|---|---|
| `npm run dev` (Vite) | 1420 | n/a |
| Release build (Tauri app) | 7421 | OS app-data dir |
| Dev build (`cargo tauri dev`) | 7422 | OS app-data dir / `dev` |
| **`npm run mcp` (headless)** | **7423** | **`<repo>/mcp-data/`** |

### Error handling rule

If an MCP tool returns an error, fix the MCP surface instead of treating
the error as permission to bypass MCP. Use this triage order:

1. **Tool contract mismatch.** If the server rejected natural agent input
   (for example `brain_summarize` got a search-style query), update the
   tool definition, wire adapter, and `BrainGateway` behavior so the call
   works or returns a specific repair instruction. Add a regression test.
2. **Server health / stale binary.** Run `brain_health`. If unhealthy,
   start or restart with `node scripts/copilot-start-mcp.mjs`; if the
   managed binary is stale, rebuild and relaunch instead of reusing it.
3. **Knowledge drift.** If the error is caused by missing/stale seed
   knowledge, append a new `INSERT INTO memories ... WHERE NOT EXISTS`
   block to `mcp-data/shared/memory-seed.sql` so the consolidated init
   snapshot picks it up.

Always report the original MCP error, the diagnosed root cause, the fix,
and any remaining blocker. A grep/file-search fallback can be used for
emergency context, but it does not close the MCP error.

### Priority — release > tray > dev

All agent-facing entry points use the same priority order:
release app on `127.0.0.1:7421`, the MCP tray on `127.0.0.1:7423`, then
dev app on `127.0.0.1:7422`. The workspace VS Code MCP entry is
a stdio proxy (`scripts/mcp-tray-proxy.mjs`) that reads the appropriate
token file and forwards every JSON-RPC request to the first available
server. Older `terransoul --mcp-stdio` configs also proxy to the same
existing server when one is authenticated.

Concretely:

- App running on **7421** (release) → agents use it instead of the tray.
- Tray running on **7423** → agents use it if no release app is serving MCP.
- App running on **7422** (dev) → agents use it if no release app or MCP tray is serving MCP.
- No authenticated server → `npm run mcp` starts the tray on **7423**.

## 2. Purpose — what `npm run mcp` is *for*

`npm run mcp` exists for **one purpose only**: improving
**development quality** of the TerranSoul repo itself.

It lets the user (and any AI coding agent in this repo) **monitor,
control, and adjust the RAG and memory surface live** during a
coding session — search the brain, ingest docs, inspect health,
re-index, etc. — without launching the Tauri app.

It is **not** the user's companion brain. It is **not** part of the
end-user runtime. It is a **dev tool**.

## 3. What `npm run mcp` MUST NOT do

The headless MCP runner is a development assistant for this repo
and must stay narrowly scoped. Specifically, agents and code paths
that touch this surface MUST NOT:

1. **Learn from end-user chat content.** The headless runner does
   not consume conversation transcripts, voice turns, persona drift
   signals, charisma ratings, or any other companion-runtime data.
   It only ingests what the developer (or an agent acting on the
   developer's behalf) explicitly tells it to ingest — typically
   repo files, design docs, RFCs, PR diffs, and external research
   pages relevant to the current chunk.
2. **Read or write the user's app-data dir.** The Tauri app's
   memory store under `com.terranes.terransoul/` is off-limits to
   this process by construction (different `data_dir`).
3. **Send memories home.** No telemetry, no remote sync, no CRDT
   peers, no cloud uploads. State stays in `<repo>/mcp-data/`.
4. **Bind anything but loopback.** Even if `lan_enabled` is set in
   the user's app settings, the headless runner ignores it and
   binds `127.0.0.1` only.
5. **Fail to maintain separation between shared data and runtime state.** `mcp-data/shared/`
   is committed reviewable seed knowledge. Runtime files under
   `mcp-data/` (token, SQLite DB, WAL/SHM, vector indexes, logs,
   locks, sessions, worktrees) are ignored. Wiping ignored runtime
   state only costs the agent its locally-ingested dev knowledge;
   nothing user-owned is lost.
6. **Trigger any user-facing onboarding.** The runner shows **only
   a system-tray icon** (so the developer can confirm MCP is alive
   and quit it) — no Vue main-window UI, no skill tree, no
   charisma panel, no persona drift prompts, no toast
   notifications. Quest unlocks, charisma teaching flows,
   brain-mode pickers, voice setup, persona-pack imports, and
   every other
   onboarding/wizard prompt the companion app would show simply
   **cannot fire** in this process. If a code path tries to surface
   one (via emit/notification/dialog), that is a bug — file it as
   a violation of this rule.

## 4. LLM defaults — local-first, opt-in for cloud

The headless runner defaults to **local Ollama** for any LLM call
it makes (embeddings, HyDE, rerank, summarize, conflict
resolution). It does **not** prompt the user to pick a provider on
first run — local-first is the silent default.

Why:

- Dev sessions can run offline.
- No API keys are needed to start a session.
- Repo content (which is often pre-publication code) never leaves
  the machine by default.

If the user wants to swap to a paid or free cloud provider, they
configure it the same way they would for the normal companion brain
— through the standard `brain_*` Tauri commands / Pinia store
flow. The MCP server will pick up the new selection on its next
LLM call. There is **no separate "MCP brain settings" UI**; the
runner reuses the same `BrainSelection` types and `ProviderRotator`
that the app uses.

If Ollama is not installed or not reachable, provider-specific LLM calls
degrade gracefully and the headless runner uses the deterministic
in-process embedder for seed/query vectors. This keeps the SQLite + vector +
RRF signal live with zero network. In `/status`, report both values explicitly:
the selected provider remains `ollama` (user selection), and the active runtime
path indicates deterministic in-process fallback. The runner does NOT silently
switch providers and does NOT upload prompts anywhere.

## 5. How agents start it

### Local-session rule (skip in GitHub Actions / CI)

> **CI skip:** If `GITHUB_ACTIONS=true`, skip MCP entirely for the session.
> MCP startup requires a Rust build (`cargo build --release`) which is too
> expensive for CI runners. Use grep/glob/file searches instead.

Every AI coding agent **local** session in this repo MUST use TerranSoul MCP as its
project memory layer:

1. **At session start**, read this file and check/reuse a running
   TerranSoul MCP server before broad repo searches. Prefer release
   (`7421`), then the MCP tray (`7423`), then dev (`7422`) when
   authenticated; only start a new tray if none of those is reusable.
2. **Before planning or implementing**, call at least one brain tool when
    available (`brain_health`, then `brain_search` /
    `brain_suggest_context` for the current chunk). If MCP is blocked by
    missing tools, missing system packages, or user policy, record the
    blocker in the progress/final report instead of silently skipping it.
   If the blocker is a missing dependency needed to run `npm run mcp`,
   `npm run dev`, `cargo tauri dev`, or app validation, the agent MUST install
   the missing dependency with the platform package manager when permissions
   allow, then retry the command before declaring it blocked. On Ubuntu/Linux
   cloud agents, the minimum Tauri/MCP build set is:
   `libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev
   librsvg2-dev patchelf libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
   pkg-config`.
3. **Show a visible MCP receipt.** Immediately after preflight succeeds, send
   a short user-visible progress update with the health/provider result and the
   `brain_search` / `brain_suggest_context` topic used. Tool calls alone are
   not enough; if the user cannot see the receipt, the MCP preflight is
   incomplete.
4. **During self-improve work**, ingest durable lessons back into the MCP
    seed surface: update `mcp-data/shared/memory-seed.sql` when the
   session discovers knowledge that future agents must retain. The
   Obsidian vault at `mcp-data/wiki/` is auto-generated from the brain
   by the maintenance scheduler — do not edit it by hand. Do not
   commit ignored runtime files (`memory.db*`, token, vector indexes,
   logs, locks, sessions, worktrees).
5. **After completing a chunk**, archive it in
   `rules/completion-log.md`, remove it from `rules/milestones.md`, and
   if the chunk changed MCP/brain behaviour, update the shared seed/docs
   so the next `npm run mcp` session can recover the decision without
   rescanning the repo.

Every AI coding agent in a **local** session follows the same startup procedure:

1. **Check first.** Run `node scripts/mcp-tray-proxy.mjs --probe` or
   call `brain_health`. If release/tray/dev is already serving MCP,
   reuse it.
2. **Copilot cloud agent (local only).** If running locally (not in
   GitHub Actions), `.github/workflows/copilot-setup-steps.yml` or
   `node scripts/copilot-start-mcp.mjs 300` may be used after installing
   dependencies. That script reuses release/tray/dev when authenticated;
   otherwise it starts the tray detached, waits for `/health`, and leaves
   logs/PIDs under `mcp-data/`. **Do not run this step when `GITHUB_ACTIONS=true`.**
3. **If not running in any other agent**, ask the user once before launching it
   (it spawns a Rust build the first time). After confirmation, run:
   ```pwsh
   npm run mcp
   ```
   in a background terminal and wait for the
   `[mcp-app] MCP server listening on http://127.0.0.1:7423` line, e.g. via
   `node scripts/wait-for-service.mjs http://127.0.0.1:7423/health 60`.
4. **Read the token** from the selected runtime's token file and pass it as
   `Authorization: Bearer <token>` on every direct HTTP request. The stdio
   proxy reads token files itself.
5. **Prefer brain tools over file searches** when the question is
   project-knowledge (e.g. "how does the RAG fallback work?",
   "what does Chunk 30.7 do?"): use `brain_search`, `brain_ingest`,
   `brain_health`, `brain_get`, `brain_list_recent`,
   `brain_kg_neighbors`, query-backed `brain_summarize`,
   `brain_suggest_context`, `brain_failover_status`
   from the MCP tool list before falling back to manual
   `grep_search`/`file_search`/`read_file`.
6. **Commit only shared MCP data.** It is valid to update
   `mcp-data/shared/**` when durable project knowledge should help
   future MCP sessions. Never force-add ignored runtime files such as
   `mcp-token.txt`, `memory.db*`, indexes, logs, locks, sessions, or
   worktrees.

### target-mcp freshness rule (when starting a new tray)

Freshness must be determined by **filesystem modification time** (`mtime`) only
(use UTC/epoch comparison; do not use git commit times, content hashes, or
version strings for this rule).

When no authenticated release/tray/dev MCP server is reusable, treat
`target-mcp/release/terransoul(.exe)` as **stale** when any of the
following is true:

- the binary does not exist;
- the binary `mtime` cannot be read reliably;
- any MCP Rust source/config path has `mtime` strictly newer than the binary:
  `src-tauri/src/**`, `src-tauri/Cargo*.toml`, `src-tauri/build.rs`,
  `src-tauri/tauri.conf.json`.

In other words, if `max(mtime of source/config set) > mtime(binary)`, rebuild
before starting a new tray. Do not terminate an already-open authenticated
release/tray/dev server just because source files changed; report that the
running app/tray is being reused and let the user restart it when they want the
new binary loaded.

Required behavior:

1. Reuse authenticated release/tray/dev if available.
2. If none is available, rebuild stale `target-mcp`.
3. Launch MCP tray and wait for `/health`.

If a port is occupied but `/status` cannot authenticate with the known token,
exit with a blocker message instead of killing the process.

## 6. Seed data — pre-populated brain on first run

The committed directory `mcp-data/shared/` contains architecture
knowledge that is automatically applied on the **first** `npm run mcp`
invocation (when `mcp-data/memory.db` does not yet exist). This is the
only Git-tracked part of `mcp-data/`:

| File | Role |
|---|---|
| `brain_config.json` | Default brain config (Pollinations free API, no key) |
| `app_settings.json` | Headless-safe app settings |
| `memory-seed.sql` | INSERT statements with TerranSoul facts |

**How it works:**

1. `run_http_server()` calls `seed_mcp_data(&data_dir)`.
2. If `memory.db` is missing, the function creates it with the
   canonical schema and runs `memory-seed.sql`.
3. Config files are written only when missing.
4. After brain config is applied, a first-run-only best-effort
   `mcp-seed-embedded` pass backfills vectors for seed rows. Provider
   embeddings are preferred; when unavailable, the deterministic offline
   embedder hashes token features into 256-dimensional vectors so vector search
   + RRF still exercise a retrieval signal before the first agent query.
5. If `mcp-data/memory.db` already exists (from a previous session),
   nothing is overwritten — incremental knowledge stays intact.

**Updating seed knowledge:**

Edit `mcp-data/shared/memory-seed.sql` with new INSERT statements
following the same column pattern. `npm run mcp` reads the shared file
at runtime before falling back to compiled defaults, so self-improve
and contributor changes can update the shared dataset in normal PRs.

## 7. Live monitor / adjust loop

Two endpoints exist for live monitoring without speaking JSON-RPC:

- **`POST /mcp`** — full JSON-RPC 2.0 surface (initialize,
  tools/list, tools/call, ping).
- **`GET /status`** — bearer-authenticated snapshot:

  ```json
  {
    "name": "terransoul-brain-mcp",
    "version": "...",
    "buildMode": "mcp",
    "petMode": true,
    "health": {
      "version": "...",
      "brain_provider": "ollama",
      "brain_model": "llama3.1:8b",
         "memory_total": 123,
         "rag_quality": {
            "label": "mostly_ready",
            "description": "80% means 98 of 123 long-term memories currently have vector embeddings...",
            "formula": "embedded_long_memory_count / long_memory_count * 100",
            "embedded_long_memory_count": 98,
            "long_memory_count": 123,
            "pending_embedding_count": 25,
            "failing_embedding_count": 0,
            "next_embedding_retry_at": 1778070000000
         },
         "memory": {
            "total": 123,
            "short_count": 0,
            "working_count": 0,
            "long_count": 123,
            "embedded_total": 98,
            "description": "123 memories total: 0 short, 0 working, 123 long. 98 memories across all tiers have vector embeddings."
         },
         "descriptions": {
            "rag_quality_pct": "RAG means retrieval-augmented generation. This percentage is long-term memory vector coverage: embedded_long_memory_count / long_memory_count * 100...",
            "memory_total": "All memories stored across short, working, and long tiers."
         }
    }
  }
  ```

Quick polling check:

```pwsh
$token = Get-Content mcp-data\mcp-token.txt
Invoke-RestMethod -Uri http://127.0.0.1:7423/status `
  -Headers @{ Authorization = "Bearer $token" }
```

To adjust the RAG/memory surface live (e.g. ingest a doc, search,
re-index), call the standard `brain_*` MCP tools — they are the
same tools the in-app server exposes, so any agent that already
speaks MCP works unchanged.

## 8. Per-agent quick-reference

| Agent | How it reaches `npm run mcp` |
|---|---|
| GitHub Copilot (VS Code) | Already wired in `.vscode/mcp.json` as `terransoul-brain-mcp` using **stdio proxy transport** — VS Code runs `node scripts/mcp-tray-proxy.mjs`, which attaches to release, tray, or dev in priority order. No bearer-token env var or VS Code restart is required. |
| Codex CLI | Use an HTTP MCP entry pointing at the selected server (`7421`, `7423`, or `7422`) with that server's bearer token, or run a stdio command entry through `node scripts/mcp-tray-proxy.mjs`. |
| Claude Code | Same — register HTTP with the selected server/token, or use the stdio proxy command. |
| Clawcode | Same — register HTTP with the selected server/token, or use the stdio proxy command. |
| Cursor / Continue.dev / Aider | Same — use the selected HTTP server/token when supported, otherwise the stdio proxy. |

## 9. Configuration knobs

| Env var | Effect | Default |
|---|---|---|
| `TERRANSOUL_MCP_PORT` | Override the bound port | `7423` |
| `TERRANSOUL_MCP_DATA_DIR` | Override the data dir | `<cwd>/mcp-data` |

Both are optional. The defaults are correct for `npm run mcp`
launched from the repo root.

## 10. Stopping the server

`Ctrl+C` in the terminal running `npm run mcp`. The runner installs
a `tokio::signal::ctrl_c` handler and shuts the server down
gracefully (max 2 s drain).

## 11. Authoring rule (for agents touching this surface)

If a code change touches MCP tray startup, the stdio proxy, the
`mcp-data/` layout, or the `is_mcp_pet_mode()` flag, the change
**must** also update this file in the same PR — the rule and the
runner are co-versioned.
