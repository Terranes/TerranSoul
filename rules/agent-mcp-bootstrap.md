# Agent Bootstrap Rule — `npm run mcp`

> **Audience:** every AI coding agent that works in this repo —
> GitHub Copilot, Codex CLI, Claude Code, Clawcode, Cursor, Continue.dev,
> Aider, and any future MCP-aware assistant.
>
> **Status:** mandatory. Treat this file the same way you treat
> `.github/copilot-instructions.md`.

## 1. What `npm run mcp` is

A **headless MCP HTTP server** that exposes TerranSoul's brain
(memory store, RAG, knowledge graph, gitnexus surface) to AI coding
agents over JSON-RPC on `http://127.0.0.1:7423/mcp`.

Internally it is the Rust binary launched as
`cargo run -- --mcp-http`, which puts the process into **MCP pet
mode**:

- `serverInfo.name` = `terransoul-brain-mcp`
- `serverInfo.buildMode` = `mcp` (not `dev`, not `release`)
- Bound to **127.0.0.1:7423** (loopback only, never LAN)
- Persists state in **`<repo>/mcp-data/`** (gitignored)
- Bearer token at **`<repo>/mcp-data/mcp-token.txt`**

It is intentionally a **separate process** with a **separate data
dir** and a **separate port** so it never collides with:

| Surface | Port | Data dir |
|---|---|---|
| `npm run dev` (Vite) | 1420 | n/a |
| Release build (Tauri app) | 7421 | OS app-data dir |
| Dev build (`cargo tauri dev`) | 7422 | OS app-data dir / `dev` |
| **`npm run mcp` (headless)** | **7423** | **`<repo>/mcp-data/`** |

### Priority — release > dev > mcp

Both the headless (`--mcp-http`) and pet-mode stdio
(`--mcp-stdio` with `TERRANSOUL_MCP_DATA_DIR` set) runners probe
`127.0.0.1:7421` and `127.0.0.1:7422` at startup. If either
answers, the runner **refuses to start** — the user already has a
real TerranSoul app serving MCP, and shadowing it with a stale
repo-local brain would only confuse agents.

Concretely:

- App running on **7421** (release) → `npm run mcp` exits with a
  message; agents must use the `terransoul-brain` MCP entry.
- App running on **7422** (dev) → `npm run mcp` exits with a
  message; agents must use the `terransoul-brain-dev` entry.
- Neither port answers → `npm run mcp` starts on **7423** as
  documented.

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
5. **Survive `git clean`.** `mcp-data/` is gitignored. Wiping it
   only costs the agent its locally-ingested dev knowledge; nothing
   user-owned is lost.
6. **Trigger any user-facing onboarding.** The headless runner has
   **no UI surface** — no Vue app, no skill tree, no charisma
   panel, no persona drift prompts, no notifications, no system
   tray. Quest unlocks, charisma teaching flows, brain-mode
   pickers, voice setup, persona-pack imports, and every other
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

If Ollama is not installed or not reachable, the runner falls back
to keyword-only search and reports `brain_provider: "none"` in
`/status` — it does NOT silently switch providers and does NOT
upload prompts anywhere.

## 5. How agents start it

Every AI coding agent in this repo follows the same procedure:

1. **Check first.** `GET http://127.0.0.1:7423/mcp` — if anything
   answers, the server is already up. Reuse it.
2. **If not running, ask the user once** before launching it (it
   spawns a Rust build the first time). After confirmation, run:
   ```pwsh
   npm run mcp
   ```
   in a background terminal and wait for the
   `[mcp-http] listening on http://127.0.0.1:7423` line, e.g. via
   `node scripts/wait-for-service.mjs http://127.0.0.1:7423/mcp 60`.
3. **Read the token** from `mcp-data/mcp-token.txt` and pass it as
   `Authorization: Bearer <token>` on every request.
4. **Prefer brain tools over file searches** when the question is
   project-knowledge (e.g. "how does the RAG fallback work?",
   "what does Chunk 30.7 do?"): use `brain_search`, `brain_ingest`,
   `brain_health`, `brain_get`, `brain_list_recent`,
   `brain_kg_neighbors`, `brain_summarize`, `brain_suggest_context`
   from the MCP tool list before falling back to manual
   `grep_search`/`file_search`/`read_file`.
5. **Never commit `mcp-data/`.** It is gitignored; agents must not
   force-add it.

## 6. Seed data — pre-populated brain on first run

The committed directory `mcp-data-seed/` contains architecture
knowledge that is automatically applied on the **first** `npm run mcp`
invocation (when `mcp-data/memory.db` does not yet exist):

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
4. If `mcp-data/` already exists (from a previous session), nothing
   is overwritten — incremental knowledge stays intact.

**Updating seed knowledge:**

Edit `mcp-data-seed/memory-seed.sql` with new INSERT statements
following the same column pattern. The seed is compiled into the
binary via `include_str!`, so a `cargo build` picks up changes.

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
      "rag_quality_pct": 80,
      "memory_total": 123
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
| GitHub Copilot (VS Code) | Already wired in `.vscode/mcp.json` as `terransoul-brain-mcp`. Set `$env:TERRANSOUL_MCP_TOKEN_MCP = Get-Content mcp-data/mcp-token.txt`, restart VS Code, then ask Copilot to use `brain_*` tools. |
| Codex CLI | Add an HTTP MCP entry pointing at `http://127.0.0.1:7423/mcp` with the bearer token from `mcp-data/mcp-token.txt`. |
| Claude Code | Same — register an HTTP MCP server at port 7423 with the bearer token. |
| Clawcode | Same — register an HTTP MCP server at port 7423 with the bearer token. |
| Cursor / Continue.dev / Aider | Same — point the agent's MCP config at `http://127.0.0.1:7423/mcp`. |

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

If a code change touches `--mcp-http`, the headless runner, the
`mcp-data/` layout, or the `is_mcp_pet_mode()` flag, the change
**must** also update this file in the same PR — the rule and the
runner are co-versioned.
