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

**Chunk 31.4 — Cross-file resolution + call graph.**

---

## Phase 30 — Charisma & Teaching

| ID | Status | Title | Goal |
|---|---|---|---|
| 30.8 | not-started | Obsidian Credits Audit | Investigate whether TerranSoul has actually learned from or applied any Obsidian (obsidian.md / Obsidian.md vault format / community plugins) ideas — vault layout, backlinks, daily notes, graph view, MOCs, Dataview, canvas, plugin sandbox, etc. — across the Brain/RAG/Memory/Knowledge-Graph/UI surfaces. **Not** an attribution rubber-stamp: actually answer "did we apply any of this, and if yes, where?" Look at `memory/obsidian_export.rs`, `memory/obsidian_import.rs` (if any), Knowledge Graph design, daily-note/journal patterns, vault watcher, graph visualisation, persona-pack Markdown layout, and any backlink/wikilink syntax in the codebase or docs. Cross-reference with research notes (`rules/research-reverse-engineering.md`) and prior conversation transcripts where Obsidian may have been referenced as inspiration. Produce one of two outcomes in the same PR: (a) if real influence is found, add a respectful entry to `CREDITS.md` (Obsidian by Erica & Shida Li / Dynalist Inc., obsidian.md, proprietary freeware EULA, list affected files/features and what we learned/used) AND a short note in the relevant design doc; or (b) if no real influence is found, add a brief "Considered but not adopted" subsection to `CREDITS.md` (or to a dedicated "Considered alternatives" section) that explains *why* — e.g. licensing, scope mismatch, different storage model — so future contributors don't keep re-asking. Either way, leave a written audit trail. No code/feature changes required unless the audit surfaces a genuinely missing attribution that needs a docstring/doc pointer. |

---

## Phase 31 — MCP Mode

> The `npm run mcp` runner exposes TerranSoul's brain to AI coding
> agents (Copilot/Codex/Claude Code/Clawcode) via MCP. These chunks
> close the gap between TerranSoul's existing GitNexus sidecar
> integration (Chunks 2.1, 2.3 — already shipped, see
> `src-tauri/src/agent/gitnexus_sidecar.rs`,
> `commands/gitnexus.rs`, `memory/gitnexus_mirror.rs`) and the
> capability surface external agents see today. See
> [`docs/gitnexus-capability-matrix.md`](../docs/gitnexus-capability-matrix.md).

| ID | Status | Title | Goal |
|---|---|---|---|
| 31.4 | not-started | Cross-file resolution + call graph | Build on Chunk 31.3. Add a second pass that resolves imports → file paths and call sites → callee symbol ids using a per-language scope resolver (Rust: `use` paths + `crate::`/`super::` rules; TypeScript: `import { X }` + `tsconfig.json` paths). Populate the `code_edges` table with typed `CALLS` and `IMPORTS` rows + a confidence score (`exact` / `inferred`). New module `src-tauri/src/coding/resolver.rs`. Add `code_call_graph(symbol)` Tauri command returning incoming + outgoing edges. Tests: assert that `run_http_server` calls `start_server` and is called from `main`. Update the capability matrix (rows: call graph, import graph, type resolution). |
| 31.5 | not-started | Functional clustering + entry-point scoring + processes | Build on Chunk 31.4. Use the `petgraph` crate (MIT) to load the symbol/edge graph, run a Louvain-style community-detection pass for clusters, score entry points by in-degree + name heuristics (`main`, `run_*`, route handlers), and trace execution flows (BFS along `CALLS` edges from each entry point, capped at depth N). Persist clusters and process traces in `code_clusters` / `code_processes` tables. New module `src-tauri/src/coding/processes.rs`. Tests: cluster the indexed `mcp-data/` repo, assert `run_http_server` appears in a process traced from `main`. Update the capability matrix. |
| 31.6 | not-started | Code-aware MCP tools + resources + prompts | Build on Chunks 31.3–31.5. Replace the GitNexus-delegating `code_*` tools from Chunk 31.2 with first-class TerranSoul-native versions backed by the symbol index, falling back to GitNexus when the index is empty. Tools: `code_query` (process-grouped hybrid search), `code_context` (360-degree view: incoming/outgoing edges, processes), `code_impact` (BFS blast-radius with depth grouping). Add MCP `resources` (`terransoul://repos`, `terransoul://repo/{name}/context`, `/clusters`, `/processes`) and two `prompts` (`detect_impact`, `generate_map`). Wire into `router.rs`. Tests: integration tests calling each tool via JSON-RPC. Update the capability matrix. |
| 31.7 | not-started | code_rename multi-file tool | Build on Chunks 31.3–31.6. Add a `code_rename` MCP tool that takes `{symbol_name, new_name, dry_run}` and produces an edit plan: graph-resolved edits (via the symbol index, high confidence) + text-search edits (via the existing `MemoryStore` FTS index over file contents, lower confidence). Apply on confirmation. Tests: rename a TerranSoul Rust function in a test fixture and verify the edit list. |
| 31.8 | not-started | Editor pre/post-tool-use hooks | Add MCP `notification` handlers + a sidecar HTTP route that AI coding editors (Claude Code, Cursor) can hit before and after tool calls. Pre-hook enriches search queries with cluster + process context. Post-hook detects stale index after `git commit` and fires a `code_index_repo` re-run in the background. Tests: integration test simulating Claude Code's PreToolUse + PostToolUse pings. |
| 31.9 | not-started | Wiki generation from the symbol graph | Reuse `brain_summarize` on each cluster from Chunk 31.5 to produce per-module Markdown pages with mermaid call graphs, write to `mcp-data/wiki/`. Tauri command `code_generate_wiki()`. Tests: assert wiki output contains a page per cluster with a non-empty body. |
| 31.10 | not-started | terransoul mcp setup auto-config writer | Add a `--mcp-setup` CLI subcommand that detects `.vscode/`, `~/.cursor/`, `~/.codex/`, `~/.claude/`, `~/.config/opencode/` and writes the correct MCP entry into each editor's config (creating files atomically, preserving any unrelated entries). Reuse `src-tauri/src/ai_integrations/mcp/auto_setup.rs`. Tests: the existing auto_setup tests. |

---
