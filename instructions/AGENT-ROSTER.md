# Agent Roster — Multi-Agent Workspaces with Durable Workflows

> Available since Chunk 1.5 (April 2026). Desktop only.

TerranSoul ships with a single built-in companion by default. With the
**agent roster** you can create additional agents that serve different
jobs — a coding assistant pointed at one repo, a research note-taker
pointed at another, a compliance reviewer with strict rules, and so on.
Each agent is an independent persona with its own:

| Attribute | Notes |
|---|---|
| VRM character | Pick any model in the catalogue. Two agents may share one. |
| Brain backend | TerranSoul's native brain **or** an external CLI worker. |
| Working folder | Only meaningful for `ExternalCli` backends. |
| Last-active timestamp | Drives the sort order of the picker. |

This document explains how to use the feature, how the RAM cap keeps
your laptop responsive, and how progress is preserved across restarts.

---

## 1. Create and switch agents

1. Open **Marketplace → Agents** (the new panel added in Chunk 1.5).
2. Click **New agent**, pick a display name and a VRM character, and
   choose a backend:
   * **Native** — uses your current Free / Paid / Local Ollama brain.
     This is the same behaviour as the pre-Chunk-1.5 single agent.
   * **External CLI** — spawns `codex`, `claude`, `gemini`, or a custom
     allow-listed binary in a folder you pick.
3. To switch, click the agent's row — the on-screen VRM swaps
   immediately via `useCharacterStore.selectModel`.

Under the hood every agent is a JSON file at
`<app-data-dir>/agents/<id>.json`. The currently-active agent is tracked
in a sibling `current_agent.json`. Deleting an agent also clears the
pointer if it happened to be the current one — **no dangling ids**.

---

## 2. External CLI agents

External-CLI agents let TerranSoul drive other AI workspaces — e.g.
**[OpenAI Codex CLI](https://github.com/openai/codex-cli)**,
**[Anthropic Claude Code](https://docs.anthropic.com/claude/docs/claude-code)**,
or **[Google Gemini CLI](https://github.com/google/gemini-cli)** — from
the same chat experience.

### How the sandbox works

| Guarantee | Where it's enforced |
|---|---|
| No shell interpretation | `Command::new(binary)` with pre-split args — never `sh -c`. |
| Only allow-listed kinds by default | `CliKind::{Codex, Claude, Gemini, Custom}`; `Custom` requires manual opt-in. |
| Binary name is validated | Alphanumerics + `-`/`_`/`.` only; no path separators, no shell metacharacters. |
| Limited env | `PATH`/`HOME`/`USER`/`LANG`/`LC_ALL`/`TERM` only; API keys from the main process are **not** leaked. |
| Pinned CWD | Process is bound to the agent's `working_folder`. The folder must exist. |
| stdin is null | `Stdio::null()` prevents the child from blocking on interactive prompts. |
| Prompt size capped | 32 KB; rejects nul bytes. |

Pick the working folder once via the folder picker — TerranSoul stores
the absolute path on the agent profile and passes it to the child via
`current_dir`. No global FS permissions are widened.

### Running a job

When you chat with an `ExternalCli` agent, the message becomes the
prompt passed as the CLI's last argument. Stdout and stderr stream back
into the chat pane line-by-line.

```ts
// Frontend
store.startCliWorkflow(agent.id, "Explain the authentication module")
```

```rust
// Backend
invoke('roster_start_cli_workflow', { request: { agent_id, prompt } })
   -> StartCliWorkflowResult { workflow_id }
```

---

## 3. Durable workflows (the Temporal-style log)

External CLI runs can take minutes to hours. TerranSoul records every
lifecycle event to an append-only SQLite log at
`<app-data-dir>/workflows.sqlite` — inspired by
[Temporal.io's history model](https://docs.temporal.io/workflows) but
**without** the server stack (no JVM, no Postgres, no Cassandra).

Event kinds:

| Kind | Meaning |
|---|---|
| `Started` | First event. Carries workflow name + input JSON. |
| `ActivityScheduled` | A sub-task was launched (e.g. CLI spawn). |
| `ActivityCompleted` / `ActivityFailed` | Sub-task terminated. |
| `Heartbeat` | Liveness marker with optional `message`. Written on every CLI stdout/stderr line. |
| `Completed` / `Failed` / `Cancelled` | Terminal — no further events permitted. |

### Replay after a restart

If TerranSoul is quit (or crashes) while a workflow is mid-run:

1. On next launch the engine loads every non-terminal workflow from
   SQLite and reports them as `Resuming` via `roster_list_pending_workflows`.
2. The agent roster panel shows a banner: *"N workflows were running
   when you quit — resume or cancel?"*
3. Choosing **Resume** reattaches a fresh live handle; choosing **Cancel**
   appends a `Cancelled` event that terminates the workflow.

Unlike Temporal, TerranSoul does **not** replay every event to
reconstruct code-side state — the CLI subprocess was killed with the
app, so there is no in-memory state to reconstruct. What survives is
the event history (for audit) and the ability to re-run the same
prompt in the same folder if the user wants to.

---

## 4. RAM-aware concurrency cap

Running eight Ollama models plus four `codex` CLIs on a laptop would
deadlock the OS. The backend computes a live cap every time the picker
opens:

```text
cap = clamp( floor( (free_mb - 1500 reserve) / mean_per_agent_mb ), 1, 8 )
```

Per-agent footprint estimates:

| Backend | Estimate |
|---|---|
| Native Free / Paid API | 200 MB |
| Native Local Ollama | 200 MB + model size (from `model_recommender.rs`) |
| External CLI worker | 600 MB |

Behaviour when the cap is reached:

* Starting another CLI workflow returns
  *"RAM cap reached — N workflows already running (cap: C). Cancel one
  before starting a new workflow."*
* The roster panel disables the **Start** button on other agents with a
  tooltip explaining how much free RAM is missing.
* Freeing RAM (cancelling a workflow, closing other apps) does **not**
  require a restart — `roster_get_ram_cap` is re-queried every time
  the picker opens.

The cap is always at least **1** (so the default agent can always run)
and at most **8** (above which OS scheduling thrashes).

---

## 5. VRM behaviour

* Only the **active** agent's VRM is loaded into the 3D viewport.
* Inactive agents keep their `vrm_model_id` as metadata — zero GPU cost.
* Two agents may share the same VRM. The GLTF loader cache deduplicates
  identical asset URLs so the file is read from disk once.

---

## 6. Tauri command reference

All commands are namespaced with the `roster_` prefix so they never
collide with the legacy single-agent list.

| Command | Purpose |
|---|---|
| `roster_list` | List all agent profiles. Lazily creates a default on first run. |
| `roster_create` | Create a new agent — validates ID, enforces `MAX_AGENTS=32`. |
| `roster_delete` | Idempotent delete; clears `current_agent.json` if needed. |
| `roster_switch` | Flip the active pointer; touches `last_active_at`. |
| `roster_get_current` | Read the active-agent id, self-healing if the target was deleted. |
| `roster_set_working_folder` | Set / change the bound folder for an external-CLI agent. |
| `roster_get_ram_cap` | Live RAM cap + per-agent footprints. |
| `roster_start_cli_workflow` | Spawn a CLI child + start a durable workflow. |
| `roster_query_workflow` | Status + full event history for one workflow. |
| `roster_cancel_workflow` | Append a `Cancelled` event. Idempotent. |
| `roster_list_workflows` | Every workflow ever recorded. |
| `roster_list_pending_workflows` | Non-terminal only — for startup "resume?" banners. |

All CLI output is broadcast as a `agent-cli-output` event:

```ts
{
  workflow_id: string,
  event: CliEvent  // { type: 'started' | 'line' | 'exited' | 'spawn_error', ... }
}
```

---

## 7. Related files

| Area | Path |
|---|---|
| Roster model + persistence | `src-tauri/src/agents/roster.rs` |
| CLI sandbox | `src-tauri/src/agents/cli_worker.rs` |
| Durable workflow engine | `src-tauri/src/workflows/engine.rs` |
| RAM cap calculator | `src-tauri/src/brain/ram_budget.rs` |
| Tauri commands | `src-tauri/src/commands/agents_roster.rs` |
| Frontend store | `src/stores/agent-roster.ts` |

---

## 8. FAQ

**Q. Can I run two local-Ollama models simultaneously?**
Only if your free RAM minus the 1.5 GB OS reserve is at least the sum
of their sizes. On an 8 GB laptop this is usually a no; on 32 GB+ it's
fine.

**Q. What happens to my CLI workflows if I force-quit the app?**
Events up to the last line written to SQLite survive. On relaunch the
workflow appears as `Resuming`; the child process itself was killed so
its state is lost. You can retry by starting a fresh workflow with the
same prompt.

**Q. Can I add a custom CLI that isn't Codex / Claude / Gemini?**
Yes — pick `Custom` and supply the binary name (e.g. `my-tool`). The
name is validated to contain only alphanumerics + `-`/`_`/`.`. The
binary must be resolvable via `$PATH`; absolute paths are refused.

**Q. Does the agent roster sync across devices?**
Not today. The per-user JSON files are local-only. A future chunk may
extend the existing CRDT sync engine to cover `/agents/`.
