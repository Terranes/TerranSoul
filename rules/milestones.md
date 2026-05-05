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

**Chunk 37.1 — Clean-room code-intelligence architecture spec** (Phase 37). See the Phase 37 table below.

---

## Phase 37 — Native Code Intelligence Parity

> Clean-room implementation plan inspired by public GitNexus docs and DeepWiki
> pages only. GitNexus is PolyForm-Noncommercial-1.0.0, so TerranSoul must not
> copy source/prompts/assets, bundle packages/binaries/images, auto-install it,
> or default-spawn it. The sidecar bridge has been removed entirely; new work
> uses neutral TerranSoul-native Rust/Vue names.

| ID | Status | Title | Goal |
|---|---|---|---|
| 37.1 | not-started | Clean-room code-intelligence architecture spec | Document the native replacement boundary, sidecar removal guardrails, map existing `coding/*` modules to the full parity plan, and add checks that prevent noncommercial dependency creep. |
| 37.2 | not-started | Incremental repo registry + content-hash indexing | Persist indexed repositories and per-file content hashes so unchanged files skip parse/embed/edge recomputation. |
| 37.3 | not-started | Multi-language parser expansion | Extend Tree-sitter extraction beyond Rust/TypeScript with a neutral parser registry and fixture tests for priority languages. |
| 37.4 | not-started | Import, heritage, and type-resolution upgrades | Add re-export resolution, class/trait/interface heritage, receiver inference, and return-aware bindings where language support exists. |
| 37.5 | not-started | Confidence-scored relation provenance | Expand code edges with relation confidence, source span, resolver tier, and provenance for auditable context/impact/rename results. |
| 37.6 | not-started | Process-grouped code search | Improve functional clustering, entry-point scoring, and execution-flow traces so `code_query` can return process-ranked results. |
| 37.7 | not-started | Hybrid semantic code search | Add BM25 + embedding + RRF retrieval over code symbols/processes using TerranSoul's existing embedding providers and fallback embedder. |
| 37.8 | not-started | Native diff impact analysis | Map git diffs to changed symbols and affected processes through native code indexing; surface risk buckets for pre-commit review. |
| 37.9 | not-started | Graph-backed rename review | Expand `code_rename` with graph-confirmed edits, lower-confidence text matches, dry-run review payloads, and focused tests. |
| 37.10 | not-started | MCP resources, prompts, and setup writer | Expose `terransoul://repos`, repo context, clusters, processes, schema/resources, guided impact/map prompts, and a local setup writer for editors. |
| 37.11 | not-started | Native code-graph workbench UI | Build a dense Vue workbench with graph canvas, file tree, code references, chat/tool-call visibility, citation-to-node focus, process diagrams, repo switcher, status bar, and blast-radius highlights. |
| 37.12 | not-started | Generated repo skills + code wiki | Generate reviewable neutral skill/context docs and wiki pages from the native graph using TerranSoul's summarization pipeline. |
| 37.13 | not-started | Multi-repo groups and contracts | Add cross-repo grouping, contract extraction, group status, and cross-service query surfaces after single-repo parity is stable. |

---

## Phase 33B — Claudia Adoption Catalogue

> Patterns/ideas adapted from `kbanc85/claudia` (PolyForm-NC 1.0.0 — no code
> copy). Each chunk is independently promotable.

| ID | Status | Title | Goal |
|---|---|---|---|
| 33B.3 | not-started | `quest_daily_brief` skill-tree quest | Once-per-day quest that runs `hybrid_search_rrf("overdue OR upcoming OR commitment", since=now-1d)` via `memory/temporal.rs` and surfaces results in the existing skill-tree UI. |
| 33B.4 | not-started | Memory-audit provenance view | New brain-panel tab that joins `memories ⨝ memory_versions ⨝ memory_edges` and renders a provenance tree per entry. |
| 33B.5 | not-started | `BrainGraphViewport.vue` 3-D KG visualiser | Three.js + d3-force-3d component consuming `memory_edges` + `memories`; node colour = `cognitive_kind`, edge colour = `rel_type`. |
| 33B.6 | not-started | Agent-roster capability tags + tag-based routing | Extend `agents/roster.rs` with capability tags; `coding/coding_router.rs` selects by tag instead of name. |
| 33B.7 | not-started | Per-workspace `data_root` setting | Allow `app_settings.json` to override the SQLite + HNSW + Obsidian-export root per workspace. |
| 33B.8 | not-started | Stdio MCP transport adapter | Add an alternate transport (alongside HTTP) that speaks JSON-RPC over stdio for editors that only support stdio MCP. Reuses `BrainGateway` trait. |
| 33B.9 | not-started | PARA opt-in template for obsidian export | Optional Project / Area / Resource / Archive folder layout for the one-way Obsidian export, behind a setting. |
| 33B.10 | not-started | Standalone scheduler daemon | Harden the maintenance scheduler into a dedicated `terransoul-scheduler` binary for headless/server environments. |

---

## Phase 36B — Understand-Anything Adoption Catalogue

> Patterns from `Lum1104/Understand-Anything` (MIT). No source/prompts/assets
> copied. Adapts ideas for TerranSoul's local-first code intelligence.

| ID | Status | Title | Goal |
|---|---|---|---|
| 36B.1 | not-started | Committed code-graph snapshot | Reviewable `code-graph.json` export from existing `coding/symbol_index.rs`. |
| 36B.2 | not-started | Persona-adaptive graph explanations | Vary graph explanations for newcomer/maintainer/PM/power-user views via persona state. |
| 36B.3 | not-started | Guided architecture tours | Generate ordered tours from `coding/processes.rs` and dependency edges. |
| 36B.4 | not-started | Diff impact overlay | Changed-file overlay marking impacted symbols, processes, docs, and tests before commit. |

---
