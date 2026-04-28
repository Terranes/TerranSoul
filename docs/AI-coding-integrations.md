# TerranSoul — AI Coding Integrations

> **Status — Phase 15 mostly shipped (April 2026).** Shipped: `BrainGateway`
> trait (15.3), MCP HTTP server (15.1), auto-setup writers (15.6), MCP stdio
> transport (15.9), VS Code workspace surfacing (15.10), voice/chat intents
> (15.5). Remaining: gRPC server (15.2), Control Panel Vue view (15.4),
> incremental-indexing QA (15.7), final doc pass (15.8). Sections marked
> **Planned** are not yet wired up and will be filled in (with screenshots,
> exact command names, and CLI samples) as each chunk lands.

## Why this exists

TerranSoul has its own brain — local Ollama, RAG pipeline, knowledge graph,
typed memory store, and a growing roster of cognitive skills. A natural next
step is to let *other* AI coding assistants (GitHub Copilot, Anthropic Claude,
OpenAI Codex / ChatGPT desktop, Cursor, Continue, Aider, etc.) **query that
brain over a documented, secure protocol** instead of re-scanning the whole
codebase every session.

Two transports are exposed:

| Transport | Audience | Default port | Auth | When to choose |
|---|---|---|---|---|
| **MCP (stdio + HTTP/SSE)** | Claude Desktop, Cursor, Continue, OpenAI Codex desktop, any Model Context Protocol client | `127.0.0.1:7421` (HTTP/SSE) or stdio | Bearer token written to `mcp-token.txt` in the TerranSoul config dir | Easiest plug-and-play. Most editor agents already speak MCP. |
| **gRPC (HTTP/2 + TLS / mTLS)** | Long-running services, custom IDE plugins, sidecar daemons, anything that needs streaming + strict auth | `127.0.0.1:7422` | Self-signed mTLS by default; bearer-token-over-TLS optionally | **Recommended for production / always-on connections** — strict typed schema, mTLS, bidi streams, and lower per-call overhead than JSON-RPC over SSE. |

Both servers expose the **same logical surface** (see [§ Surface](#surface))
so a client can pick whichever transport fits.

---

## Goals

1. **No re-scan tax.** A connected editor (e.g. VS Code Copilot Chat) should be
   able to ask TerranSoul "what does `MemoryStore` do?" and get back the
   already-indexed knowledge-graph + RAG result instantly, instead of pulling
   the whole repo into its own context window every turn.
2. **One control panel.** A single Brain-tab sub-view (`AICodingIntegrationsView`)
   shows server status, ports, tokens, connected clients, recent calls, and
   start/stop controls.
3. **Voice / chat operability.** The user can say (or type) things like
   *"start the MCP server"*, *"stop gRPC"*, *"set up Copilot to talk to you"* —
   handled through the existing intent-routing surface (`commands::routing`).
4. **Auto-setup, not manual fiddling.** TerranSoul writes the right config
   files for **Codex / ChatGPT desktop**, **GitHub Copilot (VS Code)** and
   **Claude Desktop** so the user just clicks "Connect" once.
5. **Security by default.** Loopback-only bind, per-server bearer token, and
   for gRPC self-generated TLS / mTLS certificates rotated on demand. Public
   network exposure is opt-in and gated behind an explicit toggle + warning.

## Non-goals

- Acting as a *general* MCP / gRPC gateway for arbitrary tools. The server
  exposes TerranSoul's brain only.
- Forwarding LLM completions on behalf of clients. Clients use their own LLM.
- Any cloud round-trip — every call is loopback unless the user opts in.

---

## Surface

The following operations are exposed by both MCP and gRPC. Names use the MCP
"tool" naming convention; gRPC uses `PascalCase` RPCs on the same names.

| Operation | Description | Maps to |
|---|---|---|
| `brain.search` | Hybrid + RRF + (optional) HyDE search over memories. Returns top-k entries with scores and source URLs. | `commands::memory::hybrid_search_memories_rrf`, `hyde_search_memories` |
| `brain.get_entry` | Full memory entry by id, including tags, kind, edges, source. | `commands::memory::get_entry` |
| `brain.list_recent` | Last N memories with filters (kind, tag, since). | `commands::memory::list_recent` |
| `brain.kg_neighbors` | Knowledge-graph traversal — given an entry id, return typed/directional edges + neighbours up to depth `d`. | `memory::edges` |
| `brain.summarize` | LLM-summarise an arbitrary block of text (or a list of memory ids) using TerranSoul's active brain. | `brain::ollama_agent::summarize` |
| `brain.suggest_context` | Given the user's current file path + cursor + selection, return the curated "what TerranSoul thinks is relevant" pack (top memories + KG snippet + source links). The flagship call for editor agents. | new — composes the three above |
| `brain.ingest_url` | Best-effort fetch + extract + chunk + embed of a URL into the user's brain. Off by default; requires `allow_writes=true`. | `commands::ingest::run_ingest_task` |
| `brain.health` | Server health, version, active brain provider, model, RAG quality %. | `commands::brain::get_brain_selection` |

### Capability gating

Tools are gated through the existing orchestrator capability surface
(`brain.read`, `brain.write`, `code.read`). The **default profile** is
read-only; write tools (`brain.ingest_url`) require the user to flip a
per-client switch in the Control Panel.

---

## Protocol details

### MCP server (✅ shipped 2026-04-25, Chunk 15.1)

- Implementation: `src-tauri/src/ai_integrations/mcp/`.
- Library: thin JSON-RPC 2.0 router on `axum` (Streamable HTTP transport).
  The `rmcp` crate was evaluated but the hand-rolled axum approach is simpler
  for the request/response ops surface and avoids pulling in an extra SDK.
- Bind: `127.0.0.1:7421` (configurable). Loopback only by default.
- Auth: bearer token, generated on first start (SHA-256 of UUID v4), persisted
  to `${app_data}/mcp-token.txt` with mode `0600`.
- Transports:
  - **HTTP** (POST `/mcp` — JSON-RPC 2.0) on `127.0.0.1:7421`. Bearer-token
    auth on every request.
  - **stdio** (newline-delimited JSON-RPC over stdin/stdout) — invoked as
    `terransoul --mcp-stdio`, the canonical MCP transport per spec
    (Claude Desktop / VS Code MCP / Codex CLI defaults). No bearer token
    on stdio: the editor spawns TerranSoul as a child process so the
    pipe is already trusted.
- Tauri commands: `mcp_server_start`, `mcp_server_stop`, `mcp_server_status`,
  `mcp_regenerate_token`.
- **AppState refactor**: `AppState` is now a newtype around `Arc<AppStateInner>`
  with `Deref + Clone`, enabling cheap cloning for the MCP server (and future
  gRPC server) without changing any of the 150+ existing Tauri commands.

### gRPC server (Planned — Chunk 15.2)

- Implementation: `src-tauri/src/ai_integrations/grpc/`.
- Crates: `tonic` + `prost` + `tonic-build`. **Recommended over MCP** for
  always-on / IDE-plugin scenarios because:
  - Strict typed schema (`.proto`) prevents drift between TerranSoul releases
    and client editors.
  - Built-in TLS / mTLS via `rustls` (already in the dep tree) — every call
    is encrypted, even on loopback.
  - Streaming RPCs are first-class — `brain.search` can stream RRF results as
    they arrive.
  - Lower per-call overhead than JSON-over-HTTP/SSE.
- Bind: `127.0.0.1:7422` (configurable).
- Auth: mTLS (server + client cert pair generated by TerranSoul on first
  start, exported as a `.p12` bundle the user installs into their client).
  Bearer-token-over-TLS supported as a fallback for clients that can't do
  mTLS.
- Schema: `proto/terransoul/brain.v1.proto`, versioned (`v1`, `v2`, ...).

### Shared surface (✅ shipped 2026-04-24, Chunk 15.3)

Both transports route to a common [`BrainGateway`] trait so the surface
definition lives in one place:

```text
ai_integrations/
├── gateway.rs   # BrainGateway trait + AppStateGateway adapter +
│                # IngestSink trait + 8 typed request/response structs
├── mod.rs       # re-exports the public surface
├── mcp/         # MCP adapter (rmcp / axum-based) — Chunk 15.1
└── grpc/        # tonic adapter — Chunk 15.2
```

As-built specifics (`src-tauri/src/ai_integrations/gateway.rs`):

- **Trait** — `pub trait BrainGateway: Send + Sync` with one async method
  per op, every method takes `&GatewayCaps` so authorisation is checked
  inline.
- **Capabilities** — `GatewayCaps { brain_read, brain_write, code_read }`;
  `Default` is read-only (`brain_read = true`, others off). Convenience
  constants `GatewayCaps::NONE` and `GatewayCaps::READ_WRITE` for tests.
- **Errors** — typed `GatewayError` (`PermissionDenied / NotConfigured /
  InvalidArgument / NotFound / Storage / Internal`); transports map this
  cleanly to MCP `is_error` codes and gRPC `tonic::Status`.
- **Adapter** — `AppStateGateway::new(state: AppState)` for read-only
  deployments (`ingest_url` returns `NotConfigured`), or
  `AppStateGateway::with_ingest(state, Arc<dyn IngestSink>)` for full
  read+write. `AppState` is a cheaply clonable `Arc` newtype (Chunk 15.1
  refactor). The `IngestSink` trait keeps the gateway free of any
  Tauri `AppHandle` dependency, so it remains unit-testable without a
  real Tauri runtime — production constructs an `AppHandleIngestSink`
  in the transport layer (15.1 / 15.2) that delegates to the existing
  `commands::ingest::ingest_document` flow.
- **Search modes** — `SearchMode::{Hybrid, Rrf, Hyde}` selects between
  the three retrieval pipelines documented in
  `docs/brain-advanced-design.md` § 19.2.
- **`suggest_context`** — composes `search` (HyDE when a brain is
  configured, RRF otherwise) → `kg_neighbors` (one hop around the top
  hit) → `summarize` (LLM over resolved hits). Returns a
  `SuggestContextPack` whose `fingerprint` field is a SHA-256 hex over
  the resolved hit ids + the active brain identifier. Identical inputs
  yield identical fingerprints — the contract VS Code Copilot caches
  against in Chunk 15.7.
- **Tests** — 17 unit tests covering: capability gates (read-fail,
  write-fail, write-routes-when-permitted, ingest-without-sink),
  empty-query rejection, positional-score ordering, missing-id 404,
  `list_recent` filters (kind + tag + since), KG truncation reporting,
  `summarize` graceful degradation when no brain is configured,
  `suggest_context` delta-stable fingerprint, fingerprint sensitivity
  to brain change, and health-snapshot heuristics.

[`BrainGateway`]: ../src-tauri/src/ai_integrations/gateway.rs

---

## Control Panel (Planned — Chunk 15.4)

A new sub-view under the existing **Brain** tab:
`src/views/AICodingIntegrationsView.vue`.

Sections:

1. **Server status** — start/stop toggles for MCP and gRPC, current bind
   address, uptime, version, active-token redacted preview, "regenerate
   token" / "regenerate cert" buttons.
2. **Connected clients** — live list (name, transport, last call, allowed
   capabilities). Each row has a kill-switch.
3. **Auto-setup** — three big buttons: *Set up Copilot*, *Set up Claude
   Desktop*, *Set up Codex / ChatGPT desktop*. Each writes the right config
   file (see [§ Auto-setup](#auto-setup)) and shows a one-line "Done — restart
   the editor" confirmation.
4. **Recent calls log** — rolling 200-call window with op name, client, ms,
   result size. Pure local — never sent anywhere.
5. **Network exposure** — single "Allow LAN" toggle with a clear red warning;
   off by default.

Pinia store: `src/stores/ai-integrations.ts` (mirrors the existing pattern of
`brain.ts`, `voice.ts`, etc.).

---

## Voice / chat operability (✅ shipped 2026-04-29, Chunk 15.5)

The matcher lives at `src-tauri/src/routing/ai_integrations.rs` and is
exposed to the frontend via the `match_ai_integration_intent(text)`
Tauri command. It is a **pure phrase matcher** — no LLM call — chosen
because these intents are short, exact, and high-stakes (they spawn
processes and rewrite editor configs). Returns `Option<AiIntegrationIntent>`;
the frontend dispatches the matched intent (via the existing Tauri
commands) on `Some`, and falls through to normal chat on `None`.

| Phrase examples | Intent variant |
|---|---|
| "start the MCP server", "turn MCP on" | `McpStart` |
| "stop the MCP server", "turn MCP off" | `McpStop` |
| "is MCP running?", "MCP status" | `McpStatus` |
| "open this project in VS Code", "let me code on TerranSoul", "show me the code" | `VscodeOpenProject { target: None }` |
| "open `<path>` in VS Code" | `VscodeOpenProject { target: Some(path) }` |
| "which VS Code windows do you know about?" | `VscodeListKnown` |
| "set up Copilot", "let VS Code talk to you" | `AutosetupCopilot { transport }` |
| "set up Claude Desktop" | `AutosetupClaude { transport }` |
| "set up Codex" / "set up ChatGPT desktop" | `AutosetupCodex { transport }` |

Transport defaults to **`stdio`** (canonical since 15.9). It bumps to
**`http`** when the utterance explicitly contains "via http", "over
http", or "http transport".

The matcher is case-insensitive, punctuation-tolerant, and whitespace-
collapsing. `looks_like_path()` rejects gibberish (e.g. "open the door
in vs code") by requiring `/`, `\`, `~/`, or a Windows drive letter.

After dispatch, TerranSoul reports back through the chat surface using
the existing assistant-message pipeline.

---

## Auto-setup (✅ shipped 2026-04-25, Chunk 15.6 + 15.9)

TerranSoul *writes* the integration config for the user. Each writer is a pure
function of `(transport, bind, token, cert_path)` so it can be unit-tested.

Both **HTTP** and **stdio** transport variants are shipped. Tauri commands
`setup_vscode_mcp` / `setup_claude_mcp` / `setup_codex_mcp` write the HTTP
form; the `_stdio` siblings (`setup_vscode_mcp_stdio`, etc.) write
`command: <terransoul.exe> --mcp-stdio` instead — picked by the user from
the Control Panel transport picker (Chunk 15.4).

### GitHub Copilot (VS Code)

- Path: per-workspace `.vscode/mcp.json` (or user-level
  `${user.config}/Code/User/mcp.json`).
- Entry name: `terransoul-brain`.
- Transport: prefer **stdio** with the bundled `terransoul-mcp` shim
  (avoids a long-lived HTTP listener). Fall back to HTTP/SSE if the user
  opts out of the shim.
- After writing, show the exact line the user should paste into Copilot Chat
  to verify (`@workspace use terransoul-brain to find ...`).

### Claude Desktop

- Path: `~/Library/Application Support/Claude/claude_desktop_config.json`
  (macOS), `%AppData%\Claude\claude_desktop_config.json` (Windows),
  `~/.config/Claude/claude_desktop_config.json` (Linux).
- Adds an `mcpServers.terransoul-brain` entry pointing at the stdio shim.

### Codex / ChatGPT desktop

- Path: `~/.codex/config.toml` (CLI) or the ChatGPT desktop MCP servers
  panel — both supported. CLI form is the source of truth (deterministic,
  diff-able).
- Entry name: `terransoul-brain`.

All writers must:

- Read existing config (preserve other servers).
- Idempotently update the `terransoul-brain` entry (never duplicate).
- Atomically write via temp-file + rename.
- Be undoable from the Control Panel (one-click "Remove from <client>").

---

## VS Code Copilot — incremental indexing pact (Planned — Chunk 15.7)

The single most important property for the Copilot use case: **don't make
Copilot re-scan the codebase every chat turn**.

The pact:

1. TerranSoul ingests the workspace once (Brain ingestion already exists).
2. The MCP `brain.suggest_context` tool returns a *delta-stable* context pack:
   the same input → same output until a file actually changes.
3. TerranSoul publishes a `brain.fingerprint` value (hash of the indexed
   set + active brain config). Copilot caches against this fingerprint;
   if the fingerprint is unchanged, the cache is reused.
4. File-watcher (`notify` crate) updates the fingerprint only on real
   content changes — not on cursor moves or focus changes.
5. QA scenarios live under `e2e/ai-integrations/copilot.spec.ts`:
   - First call ingests + answers (cold).
   - Second identical call hits the cache (warm) and returns in &lt; 50 ms.
   - Editing one file invalidates only that file's slice; the rest of the
     cache survives.

---

## Security model (summary)

| Risk | Mitigation |
|---|---|
| Other process on the same machine reads the bearer token | Token file is `0600`, lives in the user-scoped config dir, not the repo |
| Plain-text traffic on loopback | gRPC uses TLS even on `127.0.0.1`; MCP HTTP/SSE accepts only requests carrying the bearer token |
| Client impersonation | gRPC mTLS — client must present a cert signed by TerranSoul's local CA |
| LAN exposure by accident | "Allow LAN" toggle is off by default; turning it on shows a red banner and requires re-typing the word `EXPOSE` |
| Ingest abuse via `brain.ingest_url` | Disabled by default per client; rate-limited; URL fetcher honours `robots.txt` and a deny-list |
| Token leakage via logs | All structured logs redact token values (`token=***`); never log full URLs that contain a query-string token |

---

## Roadmap

See **Phase 15** in [`rules/milestones.md`](../rules/milestones.md).

| Chunk | Status | Title |
|---|---|---|
| 15.1 | ✅ shipped 2026-04-25 | MCP server (HTTP/JSON-RPC on 7421) + bearer-token auth + AppState Arc newtype |
| 15.2 | not-started | gRPC server (`tonic`) with mTLS — recommended transport |
| 15.3 | ✅ shipped 2026-04-24 | `BrainGateway` trait + shared op surface |
| 15.4 | not-started | Control Panel sub-view under Brain tab |
| 15.5 | ✅ shipped 2026-04-29 | Voice / chat intents to start / stop / set up |
| 15.6 | ✅ shipped 2026-04-25 | Auto-setup writers for Copilot, Claude Desktop, Codex (HTTP transport) |
| 15.7 | not-started | VS Code Copilot incremental-indexing QA + e2e |
| 15.8 | not-started | Final pass over this doc — replace "Planned" with screenshots, exact CLI commands, and the verified version matrix |
| 15.9 | ✅ shipped 2026-04-29 | MCP stdio transport shim (`terransoul --mcp-stdio` + `setup_*_mcp_stdio` Tauri commands) |
| 15.10 | ✅ shipped 2026-04-29 | VS Code workspace surfacing (`vscode_open_project`, `vscode_list_known_windows`, `vscode_forget_window` Tauri commands; self-launched window registry; PID-liveness pruning) |

When a chunk lands, the agent that finishes it must:

1. Replace its **Planned** section in this doc with the as-built reality
   (paths, ports, command names, screenshots).
2. Cross off the row above.
3. Move the chunk row out of `rules/milestones.md` and into
   `rules/completion-log.md`.
4. Update the brain-system table in `README.md` if the change is brain-facing
   (per the architecture-rules brain-doc-sync rule).
