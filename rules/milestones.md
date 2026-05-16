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

**BENCH-SCALE-3** remains code-done / run-in-flight. Pick the next chunk from
`rules/backlog.md` (the contradiction subsystem now has UI + ranker
penalty + scan command; remaining open candidates include per-conflict
importance metadata, auto-scan on idle/settings-toggle, or a brain-side
"explain this contradiction" command).

> ⛔ Do NOT promote anything from `rules/research-reverse-engineering.md` —
> all reverse-engineering research is already shipped and recorded in
> `rules/completion-log.md`. That file is historical reference only.

> CLAIM-VERIFY-1/2/3 (claim verification / contradiction-aware
> indexing / MemoryView UI), MEM-SCENARIO-1, and the AppBreadcrumb →
> ts-cockpit-crumb unification shipped 2026-05-17 — see
> `rules/completion-log.md`. Brain-orb hero was already adopted in
> `src/views/BrainView.vue` so no work was required.

---

## Phase BENCH-SCALE — Combined retrieval-quality + scale bench

Goal: validate that LoCoMo R@10 survives when relevant docs are buried in a 1M-distractor corpus.

| Chunk | Status | Scope |
|---|---|---|
| BENCH-SCALE-3 | runner-built, run-in-flight | **IVF-PQ disk-backed bench.** Phase 3 code complete (codebook training + IVF-PQ build + ADC search path + `build_ivf_pq_indexes` Tauri command). Runner shipped: `scripts/locomo-ivfpq.mjs` (cargo-run `longmemeval-ipc --features bench-million`, deterministic mulberry32 corpus, on-disk MemoryStore, progress writer, **`--resume` + SIGINT/SIGTERM safety net** added 2026-05-16). Live 10M ingest run in flight (~23%+; ETA ~83h remaining). IPC `op: count` exists for any future bench needing partial-resume. |

