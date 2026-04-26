# TerranSoul — Milestones

> **To continue development, tell the AI agent:**
>
> ```
> Continue
> ```
>
> The agent will read this file, find the next chunk with status `not-started`,
> implement it, update the status to `done`, **remove the row from this file**,
> and log details in `rules/completion-log.md`.

> **ENFORCEMENT RULE — Completed chunks must be archived.**
>
> When a chunk is marked `done`:
> 1. Log full details (date, goal, architecture, files created/modified, test counts) in `rules/completion-log.md`.
> 2. **Remove the done row from this file.** `milestones.md` contains only `not-started` and `in-progress` chunks.
> 3. If an entire phase has no remaining rows, drop the phase heading too.
> 4. Update `Next Chunk` (below) to point to the next `not-started` chunk.
>
> This rule is mandatory for every AI agent session. Never leave done rows
> in milestones.md — the full historical record lives in `completion-log.md`.
>
> **Additional:** If the chunk was derived from reverse-engineering research,
> also clean up `rules/research-reverse-engineering.md` and `rules/backlog.md`.
> See `rules/prompting-rules.md` → "ENFORCEMENT RULE — Clean Up Reverse-Engineering Research on Chunk Completion".

> **Completed work lives in [`rules/completion-log.md`](completion-log.md).**
> Do not re-list done chunks here. Phases 0–11 (Foundation through RPG Brain
> Configuration), Chunks 1.2 / 1.3 / 1.4 / 1.5 / 1.6 / 1.7 / 1.8 / 1.9 / 1.10 / 1.11,
> the Phase 9 Learned-Features batch, and all Post-Phase polish are recorded
> there in reverse-chronological order.

---

## Next Chunk

Pick the next `not-started` item from the tables below or from `rules/backlog.md`.

---

## Active Chunks

### Phase 12 — Brain Advanced Design (Documentation & QA)

| # | Chunk | Status | Notes |
|---|---|---|---|
| 1.1 | Brain Advanced Design — QA screenshots | in-progress | All agent work done; waiting on user to capture scenario-specific screenshots on a real Tauri build with Vietnamese content loaded. |

---

### Phase 14 — Persona, Self-Learning Animation & Master-Mirror

| # | Chunk | Status | Notes |
|---|---|---|---|
| 14.13 | **Hunyuan-Motion / MimicMotion offline polish** — research chunk, feature-flagged. ML pass smooths recorded motion clips. | not-started | Deferred until 3+ user requests. |
| 14.14 | **MoMask full-body retarget from sparse keypoints** — research chunk. SMPL-X reconstruction from 33 PoseLandmarker keypoints. | not-started | Research, off by default. |
| 14.15 | **MotionGPT motion token generation** — research chunk. Brain generates motion tokens → `LearnedMotionPlayer` playback. | not-started | Furthest-out research chunk. |

---

### Phase 15 — AI Coding Integrations (MCP + gRPC brain gateway)

| # | Chunk | Status | Notes |
|---|---|---|---|
| 15.2 | **gRPC server** — `tonic` + mTLS on `127.0.0.1:7422`, `brain.v1.proto`, streaming search. | not-started | ~700 LOC + tests. |
| 15.4 | **Control Panel** — `AICodingIntegrationsView.vue` + `ai-integrations.ts` store. Server status, clients, auto-setup buttons, LAN toggle. | not-started | ~500 LOC + tests. |
| 15.5 | **Voice / chat intents** — `ai_integrations` capability + intents in routing.rs. "ai-bridge" skill activation gate. | not-started | ~300 LOC + tests. |
| 15.7 | **VS Code Copilot incremental-indexing QA** — e2e test: cold/warm calls, fingerprint cache, file-watcher invalidation. | not-started | Depends on 15.1 + 15.3 + 15.6. |
| 15.8 | **Doc finalisation** — replace all "Planned" sections in `docs/AI-coding-integrations.md` with as-built reality. | not-started | Final QA gate for Phase 15. |
| 15.9 | **MCP stdio transport shim** — `terransoul --mcp-stdio` runs the same `BrainGateway` adapter over stdin/stdout JSON-RPC instead of HTTP, so editors that prefer the canonical MCP stdio transport (Claude Desktop default, Codex CLI default, VS Code MCP extension) can connect without a TCP listener. Single binary entry point — no separate companion exe — guarded by a CLI flag so a normal launch still spawns the GUI. Reuses bearer-token auth (token still needed because the editor and TerranSoul are different processes; the editor reads the token from `mcp-token.txt` and passes it as a JSON-RPC arg on each call). Auto-setup writers in 15.6 flip to write `command: terransoul --mcp-stdio` instead of an HTTP endpoint when the user picks "stdio" in the Control Panel (15.4). | not-started | Deep-analysis verdict: stdio is the *canonical* MCP transport per spec, not optional polish. Loopback HTTP is enough for power-users today but every cited client documents stdio as the primary path, so the doc-promise debt grows the longer this slips. ~250 LOC + integration tests. Depends on 15.1 (HTTP gateway), 15.3 (BrainGateway adapter), and benefits 15.4 (Control Panel transport picker) + 15.6 (auto-setup writers). |
| 15.10 | **VS Code workspace surfacing — open / focus / ancestor-reuse for the project folder.** Pure-Rust `vscode_workspace` module + `vscode_open_project(target_path)` Tauri command. When the user (or a chat intent, or a Copilot autonomous-loop step) asks TerranSoul to "open this project in VS Code", the resolver picks the right window per the rules: exact match → focus; closest ancestor of `target_path` already open → focus that window; otherwise → launch new window. **No duplicate windows for a folder that's already inside an open VS Code workspace.** | not-started | See design notes below. Foundation for the Copilot autonomous loop (Phase 10) so the next-chunk prompt always lands in the right editor window. ~450 LOC + 25 unit tests + 4 integration tests gated by `TERRANSOUL_VSCODE_INTEGRATION=1`. Depends on nothing already shipped; can land independently of 15.1 / 15.4. Surfaces in the Control Panel (15.4) as a "📂 Open project in VS Code" button and in voice intents (15.5) as the phrase set defined below. |

#### Design notes — Chunk 15.10

**Problem.** Today the Copilot autonomous loop (Phase 10) and the
auto-setup writers (Chunk 15.6, shipped) leave the user responsible
for finding *which* VS Code window to paste the next-chunk prompt
into. If the user has multiple VS Code windows open — e.g. one at
`D:\Git\` (monorepo root) and one at `D:\Git\TerranSoul\` — TerranSoul
cannot answer "let me code on TerranSoul" without guessing. Worse,
naive `code D:\Git\TerranSoul` always opens a new window even when
`D:\Git\` is already open and would contain the project. We end up
with duplicate windows and a confused Copilot session that doesn't
share context with the existing window.

**The user's contract** (verbatim spec, mapped to test cases):

| Spec phrase | Resolver branch | Acceptance test |
|---|---|---|
| *"if the folder related with the current project isn't opened, please open vscode with the project folder"* | `WindowChoice::None` → spawn `code <target>` | `pick_window` returns None when registry is empty; integration test asserts a fresh process is launched. |
| *"If there is an opened one same with folder … please use the existing one instead"* | `WindowChoice::Exact { pid }` → focus | Registry has `(pid=123, root=/D/Git/TerranSoul)`; target=`/D/Git/TerranSoul` → returns `Exact { pid: 123 }`. |
| *"… or its parents containing that folder"* | `WindowChoice::Ancestor { pid, depth }` → focus | Registry has `(pid=99, root=/D/Git)`; target=`/D/Git/TerranSoul/src` → returns `Ancestor { pid: 99, depth: 1 }` (one component above target). |
| *"If there are multiple vscode windows, priority to the most children near the current folder one"* | `Ancestor` candidates ranked by deepest root (longest common prefix) | Registry has both `(pid=99, root=/D/Git)` and `(pid=42, root=/D/Git/TerranSoul)`; target=`/D/Git/TerranSoul/src` → returns `Ancestor { pid: 42, depth: 1 }` because `/D/Git/TerranSoul` (3 components) beats `/D/Git` (2 components). |
| *"if not, creating new"* | All ancestor candidates fail PID-liveness check → fall through to `WindowChoice::None` | Registry has `(pid=42, root=/D/Git/TerranSoul)` but `sysinfo` says PID 42 is dead → `pick_window` returns None. |

**Why a self-launched registry, not VS Code introspection.** A full
day of investigation (recorded in this audit, 2026-04-25) confirmed
there is **no reliable cross-platform way** for an outside process to
enumerate currently-open VS Code windows with their folder paths:

- `code --status` prints the editor *title*, not the workspace path,
  and the title format is unstable across extensions and locales.
- VS Code child processes do not carry `--folder-uri` on their command
  lines (the arg is consumed before the renderer launches, so
  `Win32_Process.CommandLine` / `/proc/*/cmdline` show nothing useful).
- `<user-data>/User/workspaceStorage/<hash>/workspace.json` records
  every workspace VS Code has *ever* opened, not which are currently
  open — useless for our question.
- `<user-data>/Backups/<hash>/` exists for windows with hot-exit
  state but is not a reliable signal of "currently open" either.
- VS Code's IPC singleton pipe is not part of the documented
  extension surface and changes between releases.

So the only honest answer is: TerranSoul tracks **the windows it
launched itself**, validates them via PID liveness on every query,
and falls through to "launch new" when nothing matches. This trades
one limitation (we miss windows the user opened from the Start menu)
for full reliability. Manually-opened windows can be picked up by a
future best-effort `WorkspaceStorageScanner` follow-up — see the
"Out of scope" section below.

**Architecture.**

```
src-tauri/src/vscode_workspace/
├── mod.rs              # public API: open_project, pick_window
├── registry.rs         # SelfLaunchedRegistry — JSON-on-disk, PID-validated
├── resolver.rs         # pick_window(target, &[VsCodeWindow]) -> WindowChoice
├── launcher.rs         # spawn `code <path>` detached, cross-platform
└── path_norm.rs        # canonicalise + case-fold (Windows) for prefix match

src-tauri/src/commands/vscode.rs
├── vscode_open_project(target: String) -> Result<OpenOutcome, String>
├── vscode_list_known_windows() -> Vec<VsCodeWindow>     # for Control Panel
└── vscode_forget_window(pid: u32) -> Result<(), String>  # manual purge
```

**Data shapes.**

```rust
pub struct VsCodeWindow {
    pub pid: u32,
    pub root: PathBuf,            // canonicalised
    pub launched_at_ms: i64,
    pub launched_by: LaunchSource, // SelfLaunched | DiscoveredViaScanner
}

pub enum WindowChoice {
    Exact { pid: u32 },
    Ancestor { pid: u32, depth: usize }, // depth = target_components - root_components
    None,
}

pub enum OpenOutcome {
    Focused { pid: u32, kind: ChoiceKind }, // existing window brought forward
    Launched { pid: u32 },                   // fresh window spawned
}
```

**`pick_window` algorithm** (pure, fully unit-testable):

1. Canonicalise `target` (resolve `..`, symlinks, case-fold on Windows).
2. For each window in the registry:
   - If `window.root == target` → emit `Exact { pid }` candidate.
   - Else if `target.starts_with(&window.root)` → emit `Ancestor { pid, depth }` where `depth = target.components().count() - window.root.components().count()`.
3. PID-liveness filter — drop any candidate whose `pid` is no longer
   alive (per `sysinfo::System::process(pid)`).
4. If any `Exact` survives → return it (there should be at most one
   per the registry's `(root → pid)` invariant; if duplicates exist,
   pick the lowest depth then the most-recently-launched).
5. Else if any `Ancestor` survives → return the one with **smallest
   `depth`** (= deepest root = most-children-near-target per the spec).
6. Else → `WindowChoice::None`.

**`open_project` flow:**

```
1. Normalise target_path (canonicalise, reject if missing).
2. Load registry, run pick_window.
3. Match WindowChoice:
   ├── Exact { pid }     → spawn `code <window.root>`        → OpenOutcome::Focused
   ├── Ancestor { pid }  → spawn `code <window.root>`        → OpenOutcome::Focused
   └── None              → spawn `code <target>` detached, capture PID,
                           append (pid, target) to registry  → OpenOutcome::Launched
4. On Launched: poll the new PID for up to 3s to confirm it stayed alive
   (catches "code not on PATH" / immediate-exit), else surface error to UI
   with the recovery hint "Run Cmd+Shift+P → 'Shell Command: Install code in PATH'".
```

Note step 3 calls `code <window.root>` for Exact / Ancestor reuse,
not `code <target>` — `code <subpath>` would create a *new* window
even when an ancestor is open, defeating the whole point. The
existing window already contains the target subfolder so the user
can navigate to it inside VS Code; we deliberately do **not** try to
do `code -g <target>/<some-file>` because we don't always have a
file to land on, and the VS Code Explorer auto-scrolls to the active
file when the user opens one anyway.

**Path-prefix matching gotchas.**

- Windows: `Path::starts_with` is case-sensitive but the filesystem
  isn't — `path_norm.rs::canonical_eq` lowercases on `cfg!(windows)`
  before comparing. UNC paths (`\\server\share\...`) round-trip
  through the same canonicaliser.
- macOS: HFS+ / APFS are usually case-insensitive but `Path` treats
  them as case-sensitive; same lowercase fold applied.
- Linux: case-sensitive both ways, no special handling.
- Symlinks: `std::fs::canonicalize` resolves them on all platforms;
  registry stores the canonical path, never the symlink.

**Registry persistence.** `<data_dir>/vscode-windows.json` (atomic
write, follows the dev/release split design from Chunk 20.1 once
that lands). Format:

```json
{
  "version": 1,
  "windows": [
    {
      "pid": 47588,
      "root": "D:\\Git\\TerranSoul",
      "launched_at_ms": 1714050000000,
      "launched_by": "SelfLaunched"
    }
  ]
}
```

PID-liveness check happens on every read; dead entries are filtered
out and rewritten back. Stale entries can never linger across an OS
reboot because PIDs reset.

**Frontend surface (folded into the Control Panel chunk 15.4).**

- Big primary button "📂 Open this project in VS Code" — pre-fills
  `target` with the current project root inferred from `Cargo.toml`
  + `package.json` discovery; user can override via folder picker.
- Status pill below the button: "VS Code: 2 windows open at
  `D:\Git\` and `D:\Git\TerranSoul\` — clicking will focus the
  TerranSoul window" so the user sees *why* a particular window will
  be picked before they click.
- Sub-button "Forget this window" per registry row, in case the
  registry got out of sync (the user closed VS Code via Task
  Manager, etc.).

**Voice / chat intents (folded into chunk 15.5).**

| Phrase examples | Intent |
|---|---|
| "open this project in VS Code", "let me code on TerranSoul", "show me the code" | `vscode.open_project` (uses inferred project root) |
| "open `<path>` in VS Code" | `vscode.open_project { target: <path> }` |
| "which VS Code windows do you know about?" | `vscode.list_known` |

**Out of scope for 15.10** (captured here so they don't get lost):

1. **Multi-root `.code-workspace` files.** A VS Code window opened on
   `myproject.code-workspace` has *N* folder roots, any of which
   could be an ancestor of `target`. v1 treats workspace files as
   opaque (registers the `.code-workspace` path, not its inner roots).
   Follow-up chunk: parse the workspace file and emit one registry
   row per inner root.
2. **Discovering manually-opened VS Code windows.** v1 only knows
   about windows TerranSoul launched. Follow-up:
   `WorkspaceStorageScanner` reads `<user-data>/User/workspaceStorage/`
   + `Backups/` heuristically and matches against running PIDs.
   Document the technique in the chunk so later contributors don't
   re-investigate.
3. **VS Code Insiders / VSCodium / Cursor.** v1 hard-codes the `code`
   binary. The launcher is parametrised over the binary name so
   adding `code-insiders` / `cursor` is a one-constant change in a
   later chunk; the user picks the preferred editor in the Control
   Panel.
4. **Remote / WSL workspaces.** A VS Code window opened with a
   `vscode-remote://wsl+Ubuntu/home/user/proj` URI cannot be matched
   against a Windows-side `D:\` path. Detect remote URIs in the
   registry and skip them in `pick_window` (never reuse, always
   launch new on the local side).
5. **Navigating to a sub-path inside a focused ancestor.** v1 just
   focuses the ancestor window. If the user wants the Explorer
   highlighted on the target subfolder, they navigate themselves.
   Follow-up chunk could add `code -g <target>/.gitkeep` style
   tricks but the use case is weak.

**Acceptance.**

- Two consecutive `vscode_open_project("/D/Git/TerranSoul")` calls:
  first launches a new window (registry was empty); second focuses
  the same window (registry exact-match hits).
- Open VS Code at `D:\Git\` manually via the OS, then run
  `vscode_open_project("/D/Git/TerranSoul/src")` from TerranSoul:
  v1 launches a *new* window (manually-opened windows are not in the
  registry — documented limitation, see Out-of-scope #2).
- Run `vscode_open_project("/D/Git/TerranSoul")`, then
  `vscode_open_project("/D/Git/TerranSoul/src/components")`: the
  second call focuses the existing TerranSoul window (Ancestor match,
  depth = 2). Registry still has exactly one entry.
- Kill the VS Code window via Task Manager; immediately call
  `vscode_open_project("/D/Git/TerranSoul")`: registry's PID-liveness
  filter drops the dead entry; new window launches; registry rewritten.
- Three windows open at `D:\`, `D:\Git\`, `D:\Git\TerranSoul\`. Call
  `vscode_open_project("/D/Git/TerranSoul/src")`: focuses the
  `D:\Git\TerranSoul\` window (deepest ancestor wins).
- `vscode_open_project("/Z/does-not-exist")` returns a clear "target
  path does not exist" error without touching the registry.

---

### Phase 16 — Modern RAG

| # | Chunk | Status | Notes |
|---|---|---|---|
| 16.3 | **Late chunking** — long-context embed → mean-pool per-chunk windows. `memory::late_chunking` module. | not-started | Needs long-context embedding model in Ollama catalogue. |
| 16.4 | **Self-RAG iterative refinement** — orchestrator loop with `<Retrieve>`/`<Relevant>`/`<Supported>`/`<Useful>` reflection tokens, max 3 iterations. | not-started | Reuses `StreamTagParser`. |
| 16.5 | **Corrective RAG (CRAG)** — LLM evaluator classifies recall as Correct/Ambiguous/Incorrect; rewrite or web-search fallback. | not-started | Web-search only with crawl capability. |
| 16.6 | **GraphRAG / LightRAG community summaries** — Leiden community detection over `memory_edges`, LLM summaries, dual-level retrieval + RRF. | not-started | Heavy chunk; background workflow job. |
| 16.8 | **Matryoshka embeddings** — two-stage search: fast 256-dim pass → re-rank at 768-dim. | not-started | Pairs with ANN index (16.10, shipped). |

---

### Phase 17 — Brain Phase-5 Intelligence

| # | Chunk | Status | Notes |
|---|---|---|---|
| 17.5 | **Cross-device memory merge via CRDT sync** — wire `MemoryStore` into Soul Link. LWW-Map CRDT keyed on `(content_hash, source_url)`. | not-started | Hardest chunk — may split into 17.5a (schema) + 17.5b (delta sync). |
| 17.7 | **Bidirectional Obsidian sync** — extend one-way export to bidirectional via `notify` file-watcher. LWW conflict resolution. | not-started | Depends on 18.5 (shipped). |

---

### Phase 19 — Pre-release schema cleanup

| # | Chunk | Status | Notes |
|---|---|---|---|
| 19.1 | **Collapse migration history → canonical schema; delete migration runner.** Single `create_canonical_schema` block, remove `migrations.rs` + 600 test lines. | not-started | **MUST land last** — after all schema-changing chunks (16.6, 17.5, etc.). |

---

### Phase 20 — Dev/Release Data Isolation (Fresh Dev, Persistent Release)

| # | Chunk | Status | Notes |
|---|---|---|---|
| 20.1 | **Dev/release data-root split.** Single resolver behind `cfg!(debug_assertions)`: dev → ephemeral subdir wiped on launch; release → existing stable path. Covers `memory.db`, `workflows.sqlite`, agent roster JSON, settings, learned assets, (future) per-agent chat history. Backing services (Ollama container + named volumes) namespaced `-dev` vs prod. | not-started | See design notes below. Already-precedented by MCP port split (`7422` debug / `7421` release). |

#### Design notes — Chunk 20.1

**Problem.** Persistent state now spans (a) the agent roster (`<data>/agents/<id>.json` + `current_agent.json`, `src-tauri/src/agents/roster.rs:3`), (b) the durable workflow event log `workflows.sqlite` whose `Resuming` event explicitly re-attaches non-terminal runs across restarts (`src-tauri/src/workflows/engine.rs`), (c) the RAG memory store `memory.db` + `.bak` (`src-tauri/src/memory/store.rs:17`), and (d) settings/brain/voice config + learned-asset bundles + user VRM models. In dev, this state mutates across debugging sessions and contaminates scenarios; in release, the same persistence is required — losing a long-running workflow on app restart is unacceptable.

**Targets to namespace** (all currently rooted at `app.path().app_data_dir()`, resolved at `src-tauri/src/lib.rs:558-561`):
- `memory.db` + `memory.db.bak`
- `workflows.sqlite` event log
- `agents/<id>.json`, `current_agent.json`
- `app_settings.json`, `active_brain.txt`, brain config, voice config
- `user_models/*.vrm`, learned-asset bundles
- (Future) per-agent chat-history persistence — chat is currently an in-memory `Vec<Message>` (`src-tauri/src/lib.rs:148`); when persistence lands it must follow the same rules.

**Approach.**
1. **One resolver, one switch.** Add a `terransoul_data_root(app: &AppHandle) -> PathBuf` helper. Every call site that currently calls `app.path().app_data_dir()` for app data switches to it. In `cfg!(debug_assertions)` it appends a `dev/` segment; release returns the current stable path unchanged (no migration needed for existing users).
2. **Wipe before open.** On app start in debug, recursively remove the `dev/` subtree *before* any module opens its files. Single chokepoint avoids the "module X already opened the SQLite file" hazard.
3. **Backing-service split.** Ollama (managed via `src-tauri/src/commands/docker.rs`) namespaces container name + named volume by mode: `terransoul-ollama-dev` (removed-and-recreated each launch) vs `terransoul-ollama` (reused). This is where the "Docker in dev" suggestion fits naturally — Docker already lives at the service boundary, so we get fresh-dev for free without trying to wrap the Tauri GUI in a container (impractical on Windows).
4. **Env override stays.** Existing `TERRANSOUL_*` setting overrides keep their precedence so CI can force a fresh data root without rebuilding.
5. **Precedent.** Same shape as the MCP port split (`src-tauri/src/ai_integrations/mcp/mod.rs:46-50`): debug=`7422`, release=`7421`. Extend the pattern; don't invent a new one.

**Why not full Docker for the app itself.** Tauri is a desktop GUI app; running the Vue/Tauri shell inside a Linux container on Windows means X server / WSLg gymnastics for marginal benefit. The dirty state we want to wipe is files, not the runtime — a path-namespace + rmdir gives the same isolation with none of the GUI-in-container pain. Reserve Docker isolation for the service tier (Ollama, future gRPC sidecars).

**Out of scope for 20.1.** Adding chat-history persistence itself (separate chunk when ready — this chunk only ensures it inherits the rules); cloud-sync across machines (covered by 17.5); migrating existing user data (release path is unchanged).

**Acceptance.**
- Two consecutive `cargo tauri dev` launches: agent roster, workflow log, memory store all empty on second launch. No residue under `dev/` from prior session. Release data dir untouched.
- Installed release build: workflow started in run A appears as `Resuming` in run B; agents + memory survive restart. Existing users' data still loads from the unchanged release path.
- Ollama dev container/volume can be wiped without affecting release container/volume and vice-versa.

---

### Phase 21 — Doc & Completion-Log Hygiene (QA Audit 2026-04-25)

> Surfaced by a full QA pass over `docs/`, `rules/completion-log.md`,
> `rules/milestones.md`, and `rules/backlog.md`. Every row here is a
> small completable doc / log fix — no new code, no design decisions.
> The two heavier audit findings (plugin-system completion + multi-agent
> resilience wiring) escalated to dedicated phases below (Phase 22 and
> Phase 23) because they are real not-shipped feature work, not paperwork.
> The deferred MCP stdio transport escalated to Chunk 15.9 in Phase 15
> because deep-analysis decided it is not optional polish.

| # | Chunk | Status | Notes |
|---|---|---|---|
| 21.1 | **Restore missing `## Chunk 14.7 — Persona Pack Export / Import` H2 heading in `rules/completion-log.md`.** TOC links to `#chunk-147--persona-pack-export--import` (line 55) but the section body at line ≈1273 starts directly at `**Date:** 2026-04-24` with no H2 heading — anchor link is broken. Insert the heading on a new line before line 1273. | not-started | Pure log hygiene. |
| 21.2 | **Backfill Chunk 14.1 (Persona MVP) entry in `rules/completion-log.md`.** Per `docs/persona-design.md` § 15.1, the Persona MVP is `PersonaTraits` store + `persona-prompt.ts` injection + `PersonaPanel.vue` + Soul Mirror quest activation. All four artifacts exist (`src/stores/persona.ts`, `src/utils/persona-prompt.ts`, `src/components/PersonaPanel.vue`, Soul Mirror node in `src/stores/skill-tree.ts`) and tests pass, but no chunk-numbered entry exists. Reconstruct the entry from `git log --all -- src/stores/persona.ts src/utils/persona-prompt.ts` and file it with the same shape as the other 14.x entries. | not-started | Foundation chunk for Phase 14 — predates 14.2. |
| 21.3 | **Number the "Multi-Agent Resilience" entry at the top of `rules/completion-log.md`.** The 2026-04-25 entry at line 183 (per-agent threads + `workflows/resilience.rs` + agent-swap context) ships with no chunk #. Per the deep-analysis verdict in Phase 23, this entry actually only delivers the *scaffold* (library code + per-agent stamping), so renumber it as **Chunk 23.0 — Multi-agent resilience scaffold** and amend the entry's text to make clear the wiring chunks 23.1–23.3 are still pending. | not-started | Names a real chunk so Phase 23 has a clean predecessor. |
| 21.4 | **Backfill TaskControls (Stop / Stop-and-Send) chunk in `rules/completion-log.md`.** New component `src/components/TaskControls.vue` + `src/components/TaskControls.test.ts`, wired into `src/views/ChatView.vue` at lines 228 / 369 with `conversationStore.stopGeneration()` / `stopAndSend()` (defined in `src/stores/conversation.ts`). File a chunk entry — appropriate name **Chunk 23.0b — Stop & Stop-and-Send Controls** since it shipped in the same multi-agent-resilience PR per repo timestamps. | not-started | Pairs with 21.3. |
| 21.5 | **Fix MCP tool names in `docs/brain-advanced-design.md` § 24.2.** The doc lists `brain_health / brain_search / brain_ingest / brain_ask / brain_summarize / brain_extract / brain_list_memories / brain_stats`. Real names per `src-tauri/src/ai_integrations/mcp/tools.rs` are `brain_search / brain_get_entry / brain_list_recent / brain_kg_neighbors / brain_summarize / brain_suggest_context / brain_ingest_url / brain_health`. Replace the table to match the code (and match `docs/AI-coding-integrations.md § Surface`, which is already correct). | not-started | brain-doc-sync rule (architecture-rules.md rule 11). |
| 21.6 | **Refresh `docs/AI-coding-integrations.md` to reality.** (a) Roadmap table row 15.6 still says "not-started"; flip to ✅ shipped 2026-04-25 with the per-client config-path summary. (b) Top-line status banner says "MCP server (Chunk 15.1) are complete. gRPC (15.2) and the Control Panel (15.4–15.8) are in progress" — add 15.6 to the complete list and add 15.9 to the in-progress list (per the new Phase 15 row). (c) Add a note linking the stdio transport sentence in the table at line ≈22 to chunk 15.9. | not-started | Smaller than 15.8 (full doc rewrite). 15.8 is the final pass; this is the half-time correction. |
| 21.7 | **Renumber `docs/persona-design.md` § 15 from "Phase 13.A/B + 140-155" to "Phase 14.A/B + 14.1-14.15".** Current § 15.1 / § 15.2 still use legacy "Phase 13.A" / "Phase 13.B" headings and chunk numbers 140–155 (the pre-audit numbering). The audit at `completion-log.md` line 1114 has long since renumbered to Phase 14 / 14.1–14.15, and "Phase 13" in the repo today is GitNexus Code-Intelligence (chunks 2.1–2.4). Renumber tables, update cross-references in § 10 and § 14.3, and add a one-line note at the top of § 15 pointing to `rules/completion-log.md` for the as-shipped status of each row. | not-started | persona-doc-sync rule (architecture-rules.md rule 13). |

> **How to handle these.** Each row is a small doc / log edit; pick
> one, do it, log it in `completion-log.md`, remove the row from this
> file. Rows 21.5 / 21.6 / 21.7 are pure doc edits and can ship as a
> single bundled chunk. Rows 21.1–21.4 are log-only edits and can
> ship as a second bundle.

---

### Phase 22 — Plugin System Completion

> **Why this phase exists.** `docs/plugin-development.md` (613 LOC) and
> the Rust host (`src-tauri/src/plugins/{manifest,host}.rs`, ~1,300 LOC,
> 28 tests) ship a *registry* — install / activate / list / uninstall
> work, contributions are stored, settings are persisted. But nothing
> the registry tracks actually *runs* yet:
>
> - `commands` → no Tauri / chat surface invokes plugin commands.
> - `slash_commands` → no chat input router dispatches them.
> - `themes` → no CSS-variable applier consumes them.
> - `memory_hooks` → no `add_memory` pre/post pipeline calls them.
> - `views` → no router renders them.
> - `settings` → no settings UI exposes them.
> - Activation events (`OnStartup`, `OnChatMessage`, `OnMemoryTag`,
>   `OnBrainModeChange`) → no dispatcher fires `check_activation()`.
> - There is no install UI — manifests are only installable via direct
>   `invoke('plugin_install', ...)`.
>
> Either we finish the plugin system or we delete it. The user has
> told us to design proper requirements, not defer — so this phase
> closes the loop. Each chunk below is independently shippable.

| # | Chunk | Status | Notes |
|---|---|---|---|
| 22.1 | **Backfill `## Chunk 22.1 — Plugin host registry (engine + manifest + Pinia store)` entry in `rules/completion-log.md`.** Documents the as-shipped engine: `src-tauri/src/plugins/{mod,manifest,host}.rs`, `src-tauri/src/commands/plugins.rs` (12 Tauri commands), `src/stores/plugins.ts` + tests, `docs/plugin-development.md` (613 LOC). Marks engine ✅ but explicitly notes 22.2–22.7 are required before plugins are *useful*. Counts: 28 Rust tests, 152-LOC store with full vitest coverage. | not-started | Engine is real; only the log entry is missing. Same shape as 21.1–21.4 but big enough to be its own chunk. |
| 22.2 | **PluginsView.vue — install / activate / disable / uninstall UI surface.** New `src/views/PluginsView.vue` with: list of installed plugins (status pill + last-active timestamp), drag-and-drop / file-picker install for `.terransoul-plugin.json` manifests, per-plugin Activate / Disable / Uninstall buttons that call the existing Tauri commands (`plugin_install`, `plugin_activate`, `plugin_deactivate`, `plugin_uninstall`), and a permissions panel showing each plugin's declared `capabilities` with explicit user-grant toggles before activation. Routed under the existing Brain or Settings tab — no new top-level navigation. ~400 LOC + ~12 vitest tests. | not-started | Depends on 22.1 (just for log naming). Real prerequisite is the existing engine, which works. Activation event `OnStartup` is wired here too — fires `plugin_host.check_activation(&ActivationEvent::OnStartup)` once after `load_installed()`. |
| 22.3 | **Theme contribution → CSS variable applier.** Active plugins' `contributes.themes[].tokens` (already a `Record<string, string>`) flow into a new composable `useActiveTheme()` that writes them to `document.documentElement.style.setProperty(...)`. Theme picker UI added to PluginsView (22.2) so the user can pick which contributed theme is active; persisted to a single `active_theme_id` setting. Hot-swap on activate / deactivate without reload. ~150 LOC + tests for token application + idempotent reset on deactivate. | not-started | Depends on 22.2. The CSS variable layer (`--ts-*`) already exists per Chunk 065 (Design System) — this just wires plugin tokens into it. |
| 22.4 | **Slash-command contribution → ChatView dispatcher.** When the user types `/<name>` in `ChatView.vue` and `<name>` matches an active plugin's `slash_commands[].name`, dispatch to that plugin's `command_id` via `plugin_host.invoke_command(command_id, args)`. New `invoke_command` method on `PluginHost` that resolves `command_id` → `CommandEntry` and (for now) returns the contributed command's metadata as a chat message; full execution path lands in 22.7. Slash-command autocomplete dropdown in the input field. ~250 LOC + tests for fuzzy match + dispatch + unknown-command graceful error. | not-started | Depends on 22.2. `ContributedSlashCommand` already has `name` + `command_id` fields, no schema change. |
| 22.5 | **Memory-hook contribution → `add_memory` pre/post pipeline.** When `commands::memory::add_memory` runs, fire each active plugin's `memory_hooks[]` matching the current stage (`PreStore`, `PostStore`, etc.). Hooks are dispatched through the existing `WasmRunner` sandbox (`src-tauri/src/sandbox/wasm_runner.rs`) so untrusted plugins cannot read other memories. PreStore hooks may rewrite tags / content; PostStore hooks are notification-only. Hard-cap each hook at 200 ms. ~300 LOC + integration tests showing a sample WASM tag-rewriter plugin altering a memory in-flight. | not-started | Depends on 22.2 + sandbox/wasm_runner.rs (already exists, used elsewhere). The activation events `OnMemoryTag` are also wired here. |
| 22.6 | **Plugin settings UI — read / write contributed settings.** Each active plugin's `contributes.settings[]` rendered as a section in the new PluginsView (22.2) using the existing form-control primitives. Read via `plugin_get_setting`, write via `plugin_set_setting`. Schema validation (per `value_type`) is already implemented backend-side — the UI just renders the appropriate control (boolean → toggle, string → input, number → number input, enum → select). ~200 LOC + tests. | not-started | Depends on 22.2. Pure UI work — backend support is complete. |
| 22.7 | **Plugin command execution — Tool / Sidecar / WASM dispatch.** Today `plugin_host.invoke_command` (added in 22.4) only echoes metadata. Wire real execution: `kind: Tool` → call native sidecar via `tauri-plugin-shell` with the plugin's declared args, capture stdout / stderr; `kind: Wasm` → invoke through `WasmRunner`; `kind: Sidecar` → launch + handle bidirectional pipe. Capability-checked at every call site (rejects if user has not granted the capability). ~500 LOC + integration tests for each kind. | not-started | Depends on 22.4 (dispatcher) + 22.2 (capability grant UI). The biggest chunk in this phase; gates the *useful* end of plugin development. |

#### Phase 22 acceptance gate

A user installs a sample WASM plugin (`hello-world.terransoul-plugin.json`)
that contributes one slash-command `/hello`, one theme, and one
memory-hook that prepends `auto:` to every new memory's tags. After
22.1–22.7 land:

- Plugin shows up in `PluginsView.vue` after drag-and-drop install.
- Activating it applies the theme tokens immediately.
- Typing `/hello world` in chat dispatches to the plugin and prints its
  greeting as an assistant turn.
- Adding a new memory shows `auto:` automatically prepended.
- Disabling the plugin removes the theme + slash-command + hook within
  the same render frame.
- Uninstalling deletes the persisted manifest from `<data_dir>/plugins/`.

---

### Phase 23 — Multi-Agent Resilience Wiring

> **Why this phase exists.** The 2026-04-25 entry at the top of
> `rules/completion-log.md` describes "per-agent threads, workflow
> resilience, agent swap context" but a deep code-read shows only the
> *scaffold* shipped:
>
> - `agentMessages` computed in `src/stores/conversation.ts` is
>   exported but no view consumes it (`grep` returns one self-reference).
>   Multi-agent thread filtering is invisible to the user.
> - `ResilientRunner` / `RetryPolicy` / `CircuitBreaker` /
>   `HeartbeatWatchdog` exist in `src-tauri/src/workflows/resilience.rs`
>   with 13 tests, but `grep ResilientRunner` outside the module returns
>   zero hits. Workflow activities still run un-wrapped.
> - `getHandoffContext()` in `src/stores/agent-roster.ts` is exported
>   but never called by any prompt builder. Agent swap therefore loses
>   context in practice even though the data is being recorded.
>
> Library shipped, integration didn't. Each chunk below closes one of
> these three loops.

| # | Chunk | Status | Notes |
|---|---|---|---|
| 23.1 | **Wire `ResilientRunner` into workflow activities (`src-tauri/src/workflows/engine.rs`).** Every `Activity::run()` invocation in the engine routes through a `ResilientRunner` configured per workflow type (defaults: `RetryPolicy::default` 3-attempt exponential, `TimeoutPolicy` 60 s overall + 30 s per activity, `CircuitBreaker` 5-failure / 60 s recovery, `HeartbeatWatchdog` 30 s stale threshold). Re-exec / `Resuming` events on app restart inherit the same policies. New `WorkflowResilienceConfig` Tauri command surface so power users can override per-workflow-type. ~250 LOC + 8 integration tests showing retry-on-transient + circuit-open-after-N-fails + workflow-resumes-after-restart-with-half-open. | not-started | The library and tests already exist (resilience.rs, 13 unit tests). This chunk is "just" the integration into engine.rs, but engine.rs is core durable-workflow code so it is genuinely a careful chunk. |
| 23.2 | **Inject handoff context into system prompts on agent switch.** When `setAgent(newAgentId)` is called, look up the previous agent's recorded `handoffContexts[prevAgentId]` and emit it as a `[HANDOFF FROM <prev-agent-name>]` block in the *next* assistant turn's system prompt — same precedence as `[PERSONA]` / `[LONG-TERM MEMORY]`, just below them. New helper `buildHandoffBlock(ctx)` in `src/utils/handoff-prompt.ts` (pure, like `persona-prompt.ts`). Cleared after one turn so the new agent gets briefed once and then operates on its own thread. ~120 LOC + 10 vitest tests covering empty context, multi-line context, character-budget cap, and one-shot-clear. | not-started | The data flow already exists — the agent-roster store records `handoffContexts` on switch (line 224). This wires the consumer side. |
| 23.3 | **Surface per-agent threads in the chat UI.** Today messages are stamped with `agentId` (via `stampAgent`) and `agentMessages` filters them, but no view reads `agentMessages`. Add a per-agent thread filter chip row above the chat scroll (existing visual style — same as MemoryView's tag chips), backed by `agentMessages`. When toggled, the message list shows only that agent's turns. Default chip is "All agents" (= existing flat list). Persists across app restarts via `localStorage`. ~180 LOC + 8 vitest tests. | not-started | Pure frontend chunk; backend is unchanged. Closes the visibility gap noted in the audit. |

#### Phase 23 acceptance gate

In a single chat session: user starts with Agent A, exchanges 5 turns,
swaps to Agent B, exchanges 5 more turns, swaps back to Agent A.

- Agent B's first reply demonstrably acknowledges the handoff context
  from A (the `[HANDOFF FROM A]` block was injected into its prompt).
- The chip row above the chat lets the user filter to "Agent A only"
  → only A's 5+5 turns visible.
- Disabling the user's network mid-stream during one of A's activities
  triggers a `CircuitBreaker::Open` after the configured failure
  threshold, recovers half-open after the recovery timeout, and the
  workflow resumes on the next chat turn instead of permanently failing.
