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
> See `rules/prompting-rules.md` -> "ENFORCEMENT RULE — Clean Up Reverse-Engineering Research on Chunk Completion".

> **Completed work lives in [`rules/completion-log.md`](completion-log.md).**
> Do not re-list done chunks here. Phases 0–28 and all previously tracked
> chunks are recorded there in reverse-chronological order.

---

## Next Chunk

**Chunk 30.8 — Obsidian Credits Audit.**

---

## Phase 30 — Charisma & Teaching

| ID | Status | Title | Goal |
|---|---|---|---|
| 30.8 | not-started | Obsidian Credits Audit | Investigate whether TerranSoul has actually learned from or applied any Obsidian (obsidian.md / Obsidian.md vault format / community plugins) ideas — vault layout, backlinks, daily notes, graph view, MOCs, Dataview, canvas, plugin sandbox, etc. — across the Brain/RAG/Memory/Knowledge-Graph/UI surfaces. **Not** an attribution rubber-stamp: actually answer "did we apply any of this, and if yes, where?" Look at `memory/obsidian_export.rs`, `memory/obsidian_import.rs` (if any), Knowledge Graph design, daily-note/journal patterns, vault watcher, graph visualisation, persona-pack Markdown layout, and any backlink/wikilink syntax in the codebase or docs. Cross-reference with research notes (`rules/research-reverse-engineering.md`) and prior conversation transcripts where Obsidian may have been referenced as inspiration. Produce one of two outcomes in the same PR: (a) if real influence is found, add a respectful entry to `CREDITS.md` (Obsidian by Erica & Shida Li / Dynalist Inc., obsidian.md, proprietary freeware EULA, list affected files/features and what we learned/used) AND a short note in the relevant design doc; or (b) if no real influence is found, add a brief "Considered but not adopted" subsection to `CREDITS.md` (or to a dedicated "Considered alternatives" section) that explains *why* — e.g. licensing, scope mismatch, different storage model — so future contributors don't keep re-asking. Either way, leave a written audit trail. No code/feature changes required unless the audit surfaces a genuinely missing attribution that needs a docstring/doc pointer. |

---

## Phase 31 — Headless MCP Pet Mode

| ID | Status | Title | Goal |
|---|---|---|---|
| 31.1 | not-started | MCP Pet Mode Web UI | The headless `npm run mcp` runner currently has no UI — only `GET /status` and `POST /mcp`. Add a small **read-only-by-default Vue/Vite frontend** served by the same axum process so developers can monitor and adjust the brain's RAG/memory state live during a coding session. Scope: (1) static SPA built with Vite, output bundled into the Rust binary via `rust-embed` or `include_dir!` (no extra runtime, no separate `npm run` step); (2) routes — Dashboard (live `/status` poll: provider, model, RAG quality, memory total, server uptime), Memory Browser (list_recent + search), Ingest panel (paste text or URL → call `brain_ingest`), Health/Logs (recent server-side errors), Settings (brain provider/model picker reusing the existing `BrainSelection` types — local Ollama default, paid/free cloud opt-in). (3) Auth — UI loads bearer token from a query param on first navigation (`/?token=...`), stores it in `sessionStorage`, and adds it to every fetch; `npm run mcp` prints the full URL with embedded token at startup. (4) Hard scope rules from `rules/agent-mcp-bootstrap.md` still apply: no quest prompts, no charisma onboarding, no persona drift UI, no voice setup, no notifications — this UI exposes brain/RAG/memory **only**. (5) New axum routes: `GET /` (index.html), `GET /assets/*` (Vite bundle). (6) Tests: Vitest for the Vue components (mocked fetch), Rust integration test confirming `/` returns 200 with HTML. Update `rules/agent-mcp-bootstrap.md`, `docs/brain-advanced-design.md`, and `README.md` per the brain-doc-sync rule. |
| 31.2 | not-started | GitNexus Competitive Audit & Catch-up | Audit TerranSoul's current GitNexus surface (`src-tauri/src/agent/gitnexus_sidecar.rs`, `commands/gitnexus.rs`, `memory/gitnexus_mirror.rs`) against the public GitNexus capability set (repo introspection, semantic code search, symbol-level retrieval, dependency graph, change impact analysis, PR review hooks, multi-repo workspaces, CI integration, branch/PR-aware memory, MCP exposure). For each capability: (a) document current TerranSoul status (have / partial / missing) in a new `docs/gitnexus-capability-matrix.md`, (b) cite the exact file/function or note the gap, (c) for each gap that is plausibly within scope of a local-first companion (loopback only, no cloud sync of code), add a follow-up chunk row in this file with a one-line goal. Out of scope on purpose: anything that requires hosting code in a vendor cloud, PR-creation against arbitrary remotes (already covered by `coding/github.rs`), or paid features that conflict with TerranSoul's local-first stance. End state: this chunk produces the matrix doc + the new chunk rows; the actual catch-up implementation work happens in those follow-up chunks. Update `docs/brain-advanced-design.md` and `README.md` if the audit reveals brain-surface gaps (graph-aware retrieval, symbol-keyed memory, etc.) per the brain-doc-sync rule. |

---
