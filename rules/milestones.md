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

All active phases complete. Remaining chunks are deferred (user-gated on budget/priority):

- **BENCH-SCALE-2 run** — harness shipped, actual two-arm 1M-doc run pending. Deferred: "Finish the entire chunks left except Phase BENCH-SCALE."
- **BENCH-SCALE-3** — IVF-PQ disk-backed bench (Phase BENCH-SCALE, deferred).
- **TOP1-2** — paid gpt-4o-mini end-to-end harness (requires API budget).
- **INTEGRATE-2/3/4 code follow-ups** — doc-shipped; code follow-ups remain scoped but not user-prioritised.

---

## Phase INTEGRATE — remaining code follow-ups

| Chunk | Status | Scope |
|---|---|---|
| INTEGRATE-2 | code-follow-up pending | **Hermes suggest-hook in ChatView.** Dismissable hint fires when `turn_token_estimate ≥ 4000` AND `intent ∈ {deep_research, long_running_workflow, full_ide_coding}` AND `hermes_hint_enabled = true`. |
| INTEGRATE-3 | code-follow-up pending | **OpenClaw status in companions registry.** Detect upstream OpenClaw CLI, offer guided install, show "active plugin" badge in BrainView. |
| INTEGRATE-4 | code-follow-up pending | **Temporal.io optional bridge spec.** Only if user provides a concrete use case for outsourcing a workflow to a Temporal worker. |

---

## Phase TOP1 — remaining

| Chunk | Status | Scope |
|---|---|---|
| TOP1-2 | not-started | **Paid gpt-4o-mini end-to-end harness.** Requires paid API budget for Mem0-paper parity or explicit local-judge variant decision. Scoped in `benchmark/COMPARISON.md` § "TOP1-2 scope". |

---

## Phase BENCH-SCALE — Combined retrieval-quality + scale bench

Goal: validate that LoCoMo R@10 survives when relevant docs are buried in a 1M-distractor corpus.

| Chunk | Status | Scope |
|---|---|---|
| BENCH-SCALE-2 | harness-shipped, run-pending | **Sharded-HNSW scale bench.** Execute the two-arm 1M comparison per `docs/billion-scale-retrieval-design.md` § Phase 2 (router-routed vs all-shards). Report deltas on R@10 / NDCG@10 / MRR / p50 / p95 / p99 / ingest time / peak RSS. |
| BENCH-SCALE-3 | not-started | **IVF-PQ disk-backed bench.** Phase 3 targets >100M with m=96, nbits=8 PQ. Re-run LoCoMo-at-scale bench at 10M and report the PQ accuracy/latency trade against full-precision HNSW. |

