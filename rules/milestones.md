567# TerranSoul — Milestones

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
> Do not re-list done chunks here. Chunks are recorded there in reverse-chronological order.

---

## Next Chunk

No `not-started` chunks remain. The only open row is **BENCH-SCALE-3**
(code-done, run-deferred): IVF-PQ bench code is complete; only the
10M-doc bench runner + the ~40h wall-clock run are deferred.

The full BRAIN-REPO-RAG phase shipped 2026-05-16, closed by chunk
`1e` (OAuth device flow for private repos: GitHub RFC 8628 device
authorization grant, `<data_dir>/oauth/github.json` token persistence
with FS-permission hardening, `x-access-token:<token>@github.com` URL
injection at clone time, 4 Tauri commands +
`src/components/RepoOAuthDialog.vue` + 5 Rust unit tests + 4 vitest
cases). The follow-up chunk `2a` (per-source knowledge-graph
visualization: `RepoStore::recent_chunks` projection,
`cross_source_graph_nodes` Tauri command emitting a collision-free
negative-id node space, `MemoryGraph` + `MemoryGalaxy` warning-hue
repo nodes with `📦 source · file::symbol` inspector lines, MemoryView
`All`-view fan-out, 1 new Rust unit test) also shipped 2026-05-16. The
follow-up chunk `2b` (deep-scan ingest visibility: every
size/binary/unchanged/secret skip now emits an explicit `IngestPhase::Skip`
event with a typed `skip_reason`; a per-run `IngestPhase::Summary` event
fires before `Done`; `TaskProgressEvent` gained an optional `log_line`
field; `TauriIngestSink` formats each event into a log line; the frontend
`useTaskStore` keeps a per-task 500-line ring buffer surfaced through a
collapsible debug log in `TaskProgressBar.vue` with sticky skip/index
counter chips; 1 new Rust unit test +
3 new vitest cases) also shipped 2026-05-16. The 1d Aider-style repo map +
`repo_signatures` + `repo-scholar-quest` skill + final docs sync
shipped 2026-05-16; MCP tool count is now 24 brain + 17 code = 41
total. The 1c-b-ii-b frontend cross-source wiring (prompt assembler
grouped by `source_id`, citation footer with per-source badges,
`@source-id` mention syntax, 14 new vitest cases) shipped 2026-05-16;
the 1c-b-ii-a backend (cross-source `All` fan-out:
`cross_source_search` gateway trait method + `AppStateGateway` impl
that runs `MemoryStore::hybrid_search_rrf` for `'self'` + per-repo
`RepoStore::hybrid_search` for every `memory_sources` repo and
RRF-merges via `reciprocal_rank_fuse` k=60, `MultiSourceHit` wire type
tagging each hit with `source_id` / `source_label` / `tier` /
`file_path`, identically-named Tauri command + MCP tool +
4 gateway tests + integration test slot bump 38→39) shipped
2026-05-16; the 1a registry + Memory-panel source picker shipped
2026-05-15; the 1b-i foundation (gix shallow clone, ignore walker,
secret scanner, text chunker, per-repo SQLite, three Tauri commands)
shipped 2026-05-16; the 1b-ii-a static-analysis slice (AST
`parent_symbol` annotation via tree-sitter for Rust/TS, per-file
SHA-256 incremental sync, `IngestSink`/`IngestPhase` progress events,
20/20 tests) and the 1b-ii-b embedding slice (`embed_repo_with_fn` +
per-repo HNSW at `<data_dir>/repos/<source_id>/vectors.usearch` +
`TauriIngestSink` emitting `task-progress` events + 3 new integration
tests, 23/23 total) shipped 2026-05-16; the 1c-a source-scoped
retrieval backend (`RepoStore::hybrid_search` 3-signal RRF fusion +
`repo_search` / `repo_list_files` / `repo_read_file` Tauri commands +
5 new tests, 28/28 total) and the 1c-b-i `BrainGateway` MCP surface
(`repo_search` / `repo_list_files` / `repo_read_file` trait methods +
identically-named MCP tools + path-traversal hardening + 5 new tests,
33/33 total) shipped 2026-05-16 (see `completion-log.md`). Design
research lives in
[`docs/repo-rag-systems-research-2026-05-16.md`](../docs/repo-rag-systems-research-2026-05-16.md).

---

## Phase UI-2026-05 — Responsive panel unification + audio panel redesign

Goal: ship a consistent panel template across all `*Panel.vue` components,
a redesigned audio panel that surfaces TTS providers + per-provider test
buttons, and a dedicated default TTS provider. Filed 2026-05-15 from a
user UX review (screenshot of persona editor); see entries 1100/515 in
`mcp-data/shared/memory-seed.sql` for context on the voice-design
decision history.

_All chunks complete (see `completion-log.md`)._

---

## Phase BENCH-SCALE — Combined retrieval-quality + scale bench

Goal: validate that LoCoMo R@10 survives when relevant docs are buried in a 1M-distractor corpus.

| Chunk | Status | Scope |
|---|---|---|
| BENCH-SCALE-3 | code-done, run-deferred | **IVF-PQ disk-backed bench.** Phase 3 code complete (codebook training + IVF-PQ build + ADC search path + `build_ivf_pq_indexes` Tauri command). Remaining: write a 10M-doc bench runner (none exists yet — `locomo-at-scale.mjs` uses HNSW via `longmemeval-ipc`, not IVF-PQ) and run it (~40h+ wall clock). |

