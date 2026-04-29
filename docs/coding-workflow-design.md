# Coding Workflow — Design & Comparative Study

> **Audience.** Contributors and integrators who want to understand
> *what* a "coding workflow" is in TerranSoul, *how* it is implemented
> today, and *why* it is shaped the way it is when compared to other
> agentic-coding systems on the market.
>
> **Status.** Living document. Last reviewed **2026-04-29**.
>
> **Related rules.** [`rules/coding-workflow-reliability.md`](../rules/coding-workflow-reliability.md),
> [`rules/prompting-rules.md`](../rules/prompting-rules.md),
> [`rules/architecture-rules.md`](../rules/architecture-rules.md).

---

## 1. What "coding workflow" means in TerranSoul

A **coding workflow** is the deterministic, reusable pipeline that turns
a *coding task* — described in plain English, optionally accompanied by
documents — into a *structured reply* from an LLM, persists every step
to disk, and surfaces the result to either:

- the **self-improve engine** (autonomous loop reading
  [`rules/milestones.md`](../rules/milestones.md), planning the next
  chunk, and recording outcomes), or
- a **caller** (chat path, future agent, MCP client) via the
  `run_coding_task` Tauri command.

The workflow is intentionally **provider-agnostic** (Local Ollama,
Anthropic, OpenAI, DeepSeek, any OpenAI-compatible endpoint),
**output-shape-typed** (`NumberedPlan`, `StrictJson`, `BareFileContents`,
`Prose`), and **durable** (atomic JSON writes, append-only JSONL run log,
crash-safe resume).

It is **one entry point** for every "ask an LLM to do something coding-shaped"
operation in the app: [`coding::workflow::run_coding_task`](../src-tauri/src/coding/workflow.rs).

---

## 2. Design at a glance

```
┌──────────────────────────────────────────────────────────────────┐
│                           UI (Vue 3)                             │
│  BrainView · CodingWorkflowConfigPanel · Self-Improve panel      │
└─────────────────────────────┬────────────────────────────────────┘
                              │ Tauri IPC (invoke / emit)
┌─────────────────────────────▼────────────────────────────────────┐
│  commands/coding.rs   ←─ public Tauri command surface            │
│  • get/set/reset coding_workflow_config                          │
│  • preview_coding_workflow_context                               │
│  • list_local_coding_models                                      │
│  • start/stop_self_improve · get_self_improve_metrics            │
└─────────────────────────────┬────────────────────────────────────┘
                              │
┌─────────────────────────────▼────────────────────────────────────┐
│                    coding/  (Rust subsystem)                     │
│                                                                  │
│  workflow.rs      run_coding_task() — single entry point         │
│    │   load_workflow_context() — rules/instructions/docs         │
│    │   build CodingPrompt → OpenAiMessage[]                      │
│    │   client.chat() → raw_reply                                 │
│    │   extract_tag()   → payload + well_formed                   │
│    ▼                                                             │
│  prompting.rs     XML-tag prompt builder (10 Anthropic rules)    │
│  client.rs        OpenAI-compatible HTTP client wrapper          │
│  engine.rs        Self-improve autonomous loop                   │
│  metrics.rs       Append-only JSONL run log + summary            │
│  milestones.rs    Parse rules/milestones.md → ChunkRow[]         │
│  repo.rs          Git repo detection, branch state               │
│  git_ops.rs       pull_main(), branch helpers                    │
│  github.rs        Optional PR creation                           │
│  mod.rs           Configs + atomic_write_json helper             │
└──────────────────────────────────────────────────────────────────┘
```

### 2.1 The four core types

| Type | File | Role |
|---|---|---|
| [`CodingTask`](../src-tauri/src/coding/workflow.rs) | `workflow.rs` | Caller-supplied description + output contract + optional repo root |
| [`CodingTaskResult`](../src-tauri/src/coding/workflow.rs) | `workflow.rs` | `raw_reply`, extracted `payload`, `well_formed`, `context_doc_count` |
| [`CodingLlmConfig`](../src-tauri/src/coding/mod.rs) | `mod.rs` | Provider, model, base URL, API key (persisted as JSON) |
| [`CodingWorkflowConfig`](../src-tauri/src/coding/mod.rs) | `mod.rs` | Caps + include/exclude paths for context loading (user-tunable) |

### 2.2 The output-shape contract

Every coding workflow declares its [`OutputShape`](../src-tauri/src/coding/prompting.rs)
*before* calling the model. The prompt builder embeds the contract in a
`<output_contract>` tag, prefills `<analysis>` to skip preamble, and the
extractor pulls the typed payload out of the matching closing tag:

| Shape | Tag | Use case |
|---|---|---|
| `NumberedPlan { max_steps }` | `<plan>` | Planner cycles, refactor blueprints |
| `StrictJson { schema_description }` | `<json>` | Tool calls, structured updates |
| `BareFileContents` | `<file>` | "Write me this file verbatim" |
| `Prose` | _(none)_ | Free-form (smoke tests, ad-hoc) |

When the model fails to close the tag, `well_formed = false` is recorded
on the result so callers can either retry, fall back, or surface the
raw reply for debugging — never silently ship malformed output.

### 2.3 The Anthropic-style XML prompt

[`CodingPrompt::build()`](../src-tauri/src/coding/prompting.rs)
applies ten prompt-engineering rules uniformly:

1. **XML tags for structure** — never markdown headings.
2. **Job-description role** — skills + tools + priorities.
3. **Think before answering** — `<analysis>` precedes the output tag.
4. **Few-shot examples include reasoning**, not just I/O pairs.
5. **Negative constraints** — `<dont>` lines say what not to do.
6. **Assistant prefill** — pre-writes `<analysis>` to skip "I'd be happy to…".
7. **Exact output shape** — declared in `<output_contract>`.
8. **Indexed `<document>` blocks** for every supplied snippet.
9. **`<on_error>` lines** for graceful degradation.
10. **Versioned schema** — `PROMPT_SCHEMA_VERSION = "v1"` is shipped in every prompt.

The single source of truth for these rules is [`rules/prompting-rules.md`](../rules/prompting-rules.md).

### 2.4 Reliability contracts

Every persisted artefact in the workflow obeys
[`rules/coding-workflow-reliability.md`](../rules/coding-workflow-reliability.md):

- **Atomic config writes** via `coding::atomic_write_json` (temp file →
  `flush` → `sync_all` → `rename` → cleanup-on-failure). Guarantees no
  half-written `coding_llm_config.json`, `self_improve.json`, or
  `coding_workflow_config.json`, even on `SIGKILL`.
- **Append-only JSONL run log** (`self_improve_runs.jsonl`) for the
  metrics layer — single `write_all` per row; partial writes lose at
  most the in-flight row, never corrupt history.
- **Cancellation flag** (`AtomicBool`) — flipping `enabled=false` in the
  UI is the *only* way to stop the self-improve loop, by design.
- **Resume on launch** — `enabled=true` causes the engine to auto-spawn
  on next app start; the loop reads `milestones.md` fresh each cycle.
- **Bounded prompt size** — `MAX_CONTEXT_CHARS` and `MAX_FILE_CHARS`
  caps in `CodingWorkflowConfig` prevent runaway token usage on huge
  repos.

### 2.5 The self-improve loop lifecycle

```
1. User toggles self-improve on (warning dialog confirms).
2. Engine spawns Tokio task (start() in engine.rs).
3. Each cycle:
     a. Detect repo → RepoState (root, branch, clean?)
     b. Read rules/milestones.md → ChunkRow[]
     c. Pick first not-started chunk → planner_prompt(repo, chunk, cfg)
     d. metrics.record_start(chunk_id, …)
     e. client.chat(messages) → raw_reply
     f. extract <plan> tag → plan_text
     g. metrics.record_outcome(success | failure, plan_chars, error?)
     h. emit ProgressEvent { phase, message, progress, level }
4. On failure: backoff, surface error in metrics, continue to next cycle.
5. On all chunks complete: idle (IDLE_SLEEP_SECS) until milestones.md changes.
6. On disable or app quit: cancel flag set, task joins gracefully.
```

The planner currently produces **plans only** — diff application is
gated behind a future chunk so the autonomous path is provably safe.

### 2.6 Multi-provider abstraction

All providers speak **OpenAI-compatible chat completions** (`/v1/chat/completions`).
This is the single pivot that lets us add Claude, OpenAI, DeepSeek,
Ollama, vLLM, LM Studio, Groq, Together, Fireworks, Mistral, etc. by
adding a row to `coding_llm_recommendations()` — no new client code.

`OpenAiClient::completions_url()` is tolerant of `/v1`-suffixed base
URLs (a common foot-gun) and `client_from()` skips bearer auth when
`api_key` is empty (so Local Ollama works with no configuration).

The recommendation catalogue is intentionally small and opinionated:
**Local Ollama is the single top pick** — free, private, offline.
Cloud providers are listed but never auto-selected.

---

## 3. Comparative study

> Last verified **2026-04-29** against public docs and recent posts.

### 3.1 Comparator landscape (what we benchmarked against)

| System | Category | What it is |
|---|---|---|
| **[Temporal.io](https://docs.temporal.io/workflows)** | Durable workflow engine | General-purpose orchestration; deterministic replay; *not* AI-specific |
| **[GitHub Copilot CLI / coding agent](https://github.blog/ai-and-ml/github-copilot/)** | Agentic IDE/CLI assistant | `/fleet` parallel agents, model picker, GitHub-native PR flow |
| **[Claude Code](https://code.claude.com/docs/en/overview)** | Terminal coding agent | Anthropic-first; long-running sessions; tool-use; subagents |
| **Aider** | Terminal pair-programmer | Diff-based edits; git-aware; multi-provider |
| **Cline** (open source) | VS Code extension | Plan-then-act with file/terminal tools; multi-provider |
| **[Roo Code](https://docs.roocode.com/)** | VS Code extension + Cloud | Customisable Modes (Architect/Code), Orchestrator, "trade tokens for quality" — **sunset May 15, 2026** |
| **OpenHands** (formerly OpenDevin) | Autonomous dev agent | Sandboxed execution, planner+executor, multi-provider |
| **GitHub Copilot Workspace** | Cloud-hosted agent | Spec → plan → diff → PR, GitHub-integrated |
| **TerranSoul (this project)** | Embedded desktop companion | Brain + persona + RPG quests; coding workflow is one capability of many |

### 3.2 Architectural philosophy

| Property | Temporal | Copilot CLI / agent | Claude Code | Aider / Cline | Roo Code | OpenHands | **TerranSoul** |
|---|---|---|---|---|---|---|---|
| **Primary purpose** | Generic durable workflows | Coding assistant | Coding agent | Pair-programming | Coding suite | Autonomous SWE | Companion app — coding is *one* capability |
| **Single entry point** | Workflow Definition | Many (chat, fleet, agent) | Many tools | Single CLI | Per-Mode | Multi-agent | **`run_coding_task`** |
| **Runtime substrate** | Temporal cluster (server + workers) | GitHub cloud + CLI | Local terminal | Local CLI | VS Code + cloud | Sandboxed Docker | Embedded Tauri/Rust process |
| **Provider model** | N/A (not LLM-specific) | Multi-model picker | Anthropic-first | Multi-provider | Multi-provider | Multi-provider | **OpenAI-compat only** (covers all majors via one client) |
| **Local-first** | ❌ requires server | ❌ cloud-bound | ⚠️ requires Anthropic API | ✅ | ⚠️ | ✅ | ✅ **Ollama is top-pick by default** |
| **Privacy default** | N/A | Sends to GitHub | Sends to Anthropic | Sends to chosen provider | Sends to chosen provider | Configurable | **No data leaves the machine when Ollama is used** |
| **Persistence model** | Event-sourced log | Cloud session | Conversation file | Git commits | Cloud project | Local state | **Atomic JSON configs + append-only JSONL runs** |
| **Resume after crash** | ✅ deterministic replay | ✅ cloud session | ⚠️ session file | ⚠️ partial | ✅ cloud | ⚠️ | ✅ **`enabled=true` → auto-resume on next launch** |
| **Cancellation contract** | `WorkflowCancelled` signal | UI / Ctrl-C | UI | Ctrl-C | UI | UI | **AtomicBool flag, only `enabled=false` stops it** |
| **Output typing** | User-defined | Free text + tool calls | Free text + tools | Diffs | Free text | Free text + tools | **Typed `OutputShape` enum (Plan/JSON/File/Prose)** |
| **Prompt engineering** | N/A | Internal | Anthropic best practices | Prompt-light | Mode-based | Internal | **Anthropic 10 rules, codified in `prompting.rs`** |
| **Multi-agent** | Activities + child workflows | `/fleet` parallel agents | Subagents | ❌ | Orchestrator → Modes | Planner+Executor | **Future** (orchestrator already in core, not yet wired to coding) |
| **Open source** | ✅ Apache-2.0 | ❌ proprietary | ❌ proprietary | ✅ Apache-2.0 | ⚠️ partial (sunsetting) | ✅ MIT | ✅ MIT |
| **Cost model** | Self-host or Temporal Cloud | Subscription + tokens | Tokens | Tokens | Tokens (premium) | Tokens | **$0 with local Ollama** |

### 3.3 Durability — Temporal vs. TerranSoul

Temporal is the gold standard for **durable execution**: workflows are
expressed as deterministic code that replays from an event history, so
a worker crash never loses state. We don't aim for that level of
generality — we aim for *enough* durability that an autonomous coding
loop survives a Windows reboot without corrupting its config.

| Property | Temporal | TerranSoul |
|---|---|---|
| Deterministic replay | ✅ | ❌ (LLMs are non-deterministic — replay is meaningless) |
| Crash-safe state | ✅ via event log | ✅ via atomic-write configs + append-only JSONL |
| Resume on restart | ✅ | ✅ (engine respawns when `enabled=true`) |
| Long-running (years) | ✅ | ✅ (the toggle is the only stop signal) |
| Backpressure / queues | ✅ | ⚠️ (single-threaded planner today; queue is roadmap) |
| Distributed workers | ✅ | ❌ (single desktop process, by design) |

**Why we don't use Temporal.** It would force every TerranSoul user to
run a Temporal cluster. Our durability surface is small enough
(three JSON files + one JSONL log) that hand-rolled atomic writes are
the right tradeoff.

### 3.4 Prompt engineering — Claude Code vs. TerranSoul

Both apply Anthropic's prompt-engineering principles, but Claude Code
embeds them inside a closed Anthropic-tuned harness, while we
**externalise** them into versioned, testable Rust code:

```rust
// src-tauri/src/coding/prompting.rs
pub const PROMPT_SCHEMA_VERSION: &str = "v1";

pub struct CodingPrompt {
    pub role: String,                       // rule 2
    pub task: String,
    pub negative_constraints: Vec<String>,  // rule 5  → <dont>
    pub documents: Vec<DocSnippet>,         // rule 8  → <document index="N">
    pub output: OutputShape,                // rule 7  → <output_contract>
    pub example: Option<String>,            // rule 4
    pub assistant_prefill: Option<String>,  // rule 6
    pub error_handling: Vec<String>,        // rule 9  → <on_error>
}
```

This buys us three things Claude Code can't:

1. **Provider-agnostic.** The same prompt works against Claude, GPT-5,
   Gemma, DeepSeek-Coder — the schema is in our code, not the model's
   training.
2. **Unit-testable.** `prompting.rs` ships with tests asserting tag
   structure, schema version, and document indexing.
3. **Versioned.** Bumping `PROMPT_SCHEMA_VERSION` is a code change
   reviewed in PR; we never ship a "silent prompt rev".

### 3.5 Multi-agent — Roo Code / Copilot `/fleet` vs. TerranSoul

Roo Code's "Orchestrator → Modes" and Copilot's `/fleet` both run
multiple agents in parallel and coordinate them. Today TerranSoul runs
**one** agent (the self-improve planner) but the underlying
infrastructure already supports multi-agent:

- `crate::orchestrator::*` exists for chat-side agent routing with
  capability gates.
- The brain gateway (`ai_integrations/gateway.rs`) exposes 8 brain
  operations any future sub-agent can call.
- `coding::workflow::run_coding_task` is reentrant — N parallel
  Tokio tasks calling it will not corrupt each other (no shared state
  beyond the read-only configs).

The roadmap (see [`rules/backlog.md`](../rules/backlog.md)) wires the
orchestrator to the coding workflow so future cycles can dispatch a
**Planner → Coder → Reviewer** chain similar to Roo's Modes, but
running entirely on the user's machine.

### 3.6 What we deliberately did **not** copy

| Pattern | Where it appears | Why we skipped it |
|---|---|---|
| Cloud-bound execution | Copilot, Roo Cloud, Claude Code | Breaks "your AI lives on your machine" promise |
| Sandbox containers | OpenHands | Tauri desktop app — host OS *is* the sandbox; Docker is overkill |
| Diff-based-only output | Aider | We need plans, JSON, *and* files — `OutputShape` covers all |
| Per-task subscription | Most cloud agents | $0 with Ollama is non-negotiable for the user base |
| Custom workflow language | Temporal DSL, GitHub Agentic Workflows | Plain Rust async functions are enough at our scale |
| "Auto-approve everything" | Roo Auto-Approve | Self-improve is gated behind a confirm dialog + explicit toggle |

---

## 4. Best practices applied (mapped to current code)

| Best practice | Where it lives in TerranSoul |
|---|---|
| **Single entry point per capability** | `workflow::run_coding_task` |
| **Typed output contracts** | `OutputShape` enum + `<output_contract>` tag |
| **XML-structured prompts** | `CodingPrompt::build()` |
| **Versioned prompt schema** | `PROMPT_SCHEMA_VERSION` constant |
| **Atomic config writes** | `coding::atomic_write_json` |
| **Append-only run log** | `MetricsLog` (JSONL) |
| **Bounded context** | `CodingWorkflowConfig.max_context_chars / max_file_chars` |
| **Provider abstraction** | OpenAI-compat client + recommendation catalogue |
| **Local-first default** | Local Ollama is the single `is_top_pick: true` |
| **Cancellation as truth** | `AtomicBool` cancel flag; only `enabled=false` stops it |
| **Auto-resume after crash** | Engine respawns at app launch when `enabled=true` |
| **Observability** | Per-run start/outcome rows; aggregate `MetricsSummary` |
| **Recommendation hygiene** | Test asserts no recommendation has `/v1` suffix in `base_url` |
| **No-key auth path** | `client_from()` skips bearer when `api_key` is empty |
| **Live verification** | `tests/ollama_self_improve_smoke.rs` (gated by `OLLAMA_REAL_TEST=1`) |

---

## 5. Roadmap & future implementations

The current implementation is **planning-only** by design — the
autonomous loop produces plans; humans (or a future gated chunk) apply
them. The roadmap, in priority order:

1. **Diff application gate.** Take a `<plan>` payload, run a `Coder`
   sub-task with `OutputShape::BareFileContents`, write through
   `coding::workflow::apply_file` (does not yet exist) using the same
   atomic-write contract as configs.
2. **Reviewer sub-agent.** Feed the produced diff back into a third
   workflow with `OutputShape::StrictJson { schema_description: "{ ok: bool, issues: string[] }" }`.
3. **Orchestrator wiring.** Connect `crate::orchestrator` so chat
   messages can dispatch coding workflows just like the brain queries
   today.
4. **Multi-agent fan-out.** Mirror Copilot `/fleet` — parallelise
   independent chunks, serialise dependent ones via a small DAG runner.
5. **Sandboxed test runs.** Before committing, run `cargo test` and
   `vitest` in a child process and gate the diff on a green run.
6. **GitHub PR flow.** Already partially implemented in
   `coding::github`; finish the OAuth device flow and per-chunk PR
   creation.
7. **Cost / token telemetry.** Extend `RunRecord` with token counts so
   the metrics panel can show cost-per-chunk by provider.
8. **Persistent task queue.** Move from "read milestones.md every
   cycle" to a real SQLite-backed queue so external tools (MCP, CLI)
   can enqueue tasks.

Each item lands as one chunk in [`rules/milestones.md`](../rules/milestones.md)
under Phase 25, with the same enforcement contract as every other
chunk: full CI gate, completion-log entry, doc updates, and (if it
touches the brain) `docs/brain-advanced-design.md` sync.

---

## 6. Quick-start: invoking the coding workflow

```rust
use terransoul_lib::coding::{
    workflow::{run_coding_task, CodingTask, TaskOutputKind},
    CodingLlmConfig, CodingLlmProvider,
};

let cfg = CodingLlmConfig {
    provider: CodingLlmProvider::Custom,
    base_url: "http://127.0.0.1:11434".into(), // Local Ollama
    model: "gemma3:4b".into(),
    api_key: String::new(),                    // no key needed
};

let task = CodingTask {
    id: "demo.1".into(),
    description: "Summarise this repo's coding workflow in 3 bullets.".into(),
    repo_root: Some(std::env::current_dir()?),
    include_rules: true,
    include_instructions: true,
    include_docs: true,
    output_kind: TaskOutputKind::NumberedPlan { max_steps: 3 },
    extra_documents: vec![],
};

let result = run_coding_task(&cfg, &task, None).await?;
assert!(result.well_formed);
println!("{}", result.payload);
```

Same call works against Claude, GPT-5, DeepSeek, Groq — change three
fields in `CodingLlmConfig`. **That is the whole point of the design.**
