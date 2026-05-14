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

All active phases complete. Three full bench runs were launched 2026-05-14 19:50 local
(see in-progress entries below). When they finish, archive the chunks per the enforcement
rule above.

---

## In-Progress Runs (launched 2026-05-14)

| Chunk | Terminal | Log | Status |
|---|---|---|---|
| BENCH-SCALE-2 routed @ 1M | `c1e09715` | `target-copilot-bench/scale-1m-routed-20260514-195000.log` | Router-routed shard policy, 100 adversarial queries vs 1M-doc corpus. |
| BENCH-SCALE-2 all-shards @ 1M | `52321122` | `target-copilot-bench/scale-1m-allshards-20260514-195015.log` | All-shards baseline arm; same corpus, no router. |

Live monitor: `Get-Content target-copilot-bench\scale-1m-*.log -Wait -Tail 5`

---

## Phase BENCH-SCALE — Combined retrieval-quality + scale bench

Goal: validate that LoCoMo R@10 survives when relevant docs are buried in a 1M-distractor corpus.

| Chunk | Status | Scope |
|---|---|---|
| BENCH-SCALE-2 | run-in-progress (1M, both arms) | **Sharded-HNSW scale bench.** Two-arm 100k done previously (R@10 58.5% vs 59.5%). 1M routed + all-shards launched 2026-05-14 19:50. Each arm: 1M-doc corpus, 100 adversarial queries, mxbai-embed-large via Ollama. Expected ~6 h ingest per arm (sequential due to single Ollama instance). |
| BENCH-SCALE-3 | code-done, run-deferred | **IVF-PQ disk-backed bench.** Phase 3 code complete (codebook training + IVF-PQ build + ADC search path + `build_ivf_pq_indexes` Tauri command). Remaining: write a 10M-doc bench runner (none exists yet — `locomo-at-scale.mjs` uses HNSW via `longmemeval-ipc`, not IVF-PQ) and run it (~40h+ wall clock). |

