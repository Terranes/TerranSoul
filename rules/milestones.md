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
> Do not re-list done chunks here. Chunks are recorded there in reverse-chronological order.

---

## Next Chunk

Next up: **BENCH-LCM-6 — Fix adversarial regression + combined model strategy** (`not-started`).

---

## Phase BENCH-LCM — Beat LoCoMo / LMEB retrieval benchmarks

Goal: add a direct, reproducible TerranSoul run on the MTEB `LoCoMo` text-retrieval dataset so the benchmark table can move beyond mixed published LoCoMo QA numbers and compare TerranSoul against top memory systems on a shared retrieval task.

> **Round 1 baseline (BENCH-LCM-1, 2026-05-12):** 250-query slice shows search R@10 51.3%, NDCG@10 40.9%. Strongest: temporal_reasoning 90% R@10. Weakest: multi_hop 15%, open_domain 24%. Root cause: insufficient morphological stemming (only -s/-ies), weak all-terms bonus (16), no conversational concept expansion.

> **Round 2 result (BENCH-LCM-2, 2026-05-12):** 250-query slice shows rrf R@10 **54.4%** (+2.8pp), search R@10 **53.6%** (+2.3pp). multi_hop nearly doubled: 15→33% R@10. Morphological variants now FTS5-recall-only (not scored), fixing the `configuration_term` regression. Added 11 new QUERY_TERM_EXPANSIONS and 3 new phrase expansions (activities, destress, art).

> **Round 3 result (BENCH-LCM-3, 2026-05-12):** Full 1655-query run, all 4 tasks. Added `rrf_emb` system (3-tier embedding-enhanced RRF: lexical+freshness fusion, cosine re-rank of candidates, embedding rescue for semantically-missed docs). rrf_emb **59.4%** R@10 overall (+3.7pp vs rrf 55.7%). Wins every task: single_hop 68.1% (+2.6pp), multi_hop 33.3% (+2.7pp), open_domain 34.1% (+4.4pp), adversarial 64.3% (+6.2pp). Also added 12 new query expansions (career, degree, education, financial, music, etc.).

> **Round 4 result (BENCH-LCM-4, 2026-05-12):** Store-level embedding integration. Embeddings stored in HNSW ANN index, query embeddings passed to `hybrid_search_rrf()` for native 3-way RRF. rrf+emb **59.9%** R@10 overall (+4.2pp vs plain rrf, +0.5pp vs IPC-level rrf_emb). single_hop 68.6%, multi_hop 35.6%, open_domain 32.6%, adversarial 64.3%.

> **Round 5 result (BENCH-LCM-5, 2026-05-12):** Upgraded from nomic-embed-text (137M, 768d) to mxbai-embed-large (335M, 1024d). Massive gains: overall **63.6%** R@10 (+3.7pp). single_hop 73.5% (+4.9pp), multi_hop 46.2% (+10.6pp), open_domain 42.0% (+9.4pp). Adversarial regressed to 61.7% (-2.6pp) — stronger semantic matching creates false positives on trick questions.

> **Loop rule (per user request).** After each `BENCH-LCM-N` chunk completes, re-run the LoCoMo benchmark, diff against the previous round, and open the next fix chunk. Stop only when TerranSoul holds rank 1 on every measured metric.

| Chunk | Status | Scope |
|---|---|---|
| BENCH-LCM-6 | not-started | Fix adversarial regression (mxbai 61.7% vs nomic 64.3%). Try keyword-emphasis for adversarial queries. Test snowflake-arctic-embed2 as alternative. Explore multi-model ensemble. Target: overall R@10 > 65%, adversarial R@10 > 64%. |

---

## Phase BENCH-AM — Beat agentmemory's published benchmark

Goal: match-or-beat the agentmemory v0.6.0 quality bench (Recall@10 ≥ 58.6 %, NDCG@10 ≥ 84.7 %, MRR ≥ 95.4 %) and stage LongMemEval-S so we can claim a public retrieval-accuracy number. Reference: `https://github.com/rohitg00/agentmemory/blob/main/benchmark/COMPARISON.md`.

> **Round 3 result (BENCH-AM-3, 2026-05-12):** TerranSoul `search` with lexical rerank + gated KG boost → R@10 **64.1 %** (+5.5 pp ahead), NDCG@10 **94.7 %** (+10.0 pp ahead), MRR **95.8 %** (+0.3 pp ahead vs agentmemory BM25-only's 95.5 %). TerranSoul now leads the pinned agentmemory `bench:quality` case set on every measured quality metric. Full numbers in [docs/agentmemory-comparison.md](../docs/agentmemory-comparison.md).

> **Round 4 result (BENCH-AM-4, 2026-05-12):** token-efficiency accounting now ships in the harness JSON/Markdown report plus `npm run brain:tokens`. Full-context paste costs 32,660 tokens/query on the pinned fixture; 200-line MEMORY.md costs 7,960 tokens/query. TerranSoul no-vector RRF uses 2,798 retrieved-memory tokens/query while holding R@10 63.6 %, NDCG@10 94.3 %, MRR 95.8 %, saving **91.4 %** vs full paste and **64.8 %** vs the 200-line baseline.

> **Round 5 adapter (BENCH-AM-5, 2026-05-12):** LongMemEval-S plumbing now ships: `npm run brain:longmem:prepare`, `npm run brain:longmem:run`, `npm run brain:longmem:sample`, and a Rust JSONL IPC shim over the real `MemoryStore`.

> **Round 6 result (BENCH-AM-6/6.1, 2026-05-11):** LongMemEval-S retrieval-only top-1 verified on the 500-question cleaned set. TerranSoul `search` with corpus-aware lexical weighting and light query variants hit R@5 **99.2 %**, R@10 **99.6 %**, R@20 **100.0 %**, NDCG@10 **91.3 %**, MRR **92.6 %**. This beats agentmemory's published **95.2 / 98.6 / 99.4 / 87.9 / 88.2** and MemPalace's ~**96.6 % R@5** on the comparable retrieval table. Full numbers live in [docs/agentmemory-comparison.md](../docs/agentmemory-comparison.md) and `target-copilot-bench/bench-results/longmemeval_s_terransoul.{json,md}`.

> **Round 7 result (BENCH-AM-7, 2026-05-11):** feature-matrix parity sweep complete. The remaining partial rows are documented scope boundaries (Hive/MCP instead of a core-memory lease mesh; MCP/Tauri/Rust/Vue APIs instead of separate SDK packages). The required quality rerun found and fixed a candidate-pool rarity regression by capping broad low-signal terms (`configuration`, `setup`, `test`, `validation`) while preserving LongMem rare-anchor weighting. Final post-fix checks: agentmemory bench `search` **66.4 % R@10 / 96.5 % NDCG / 100.0 % MRR**, no-vector RRF **67.1 % / 98.2 % / 100.0 %**, and LongMemEval-S unchanged at **99.2 % R@5 / 99.6 % R@10 / 100.0 % R@20 / 91.3 % NDCG / 92.6 % MRR**.

> **Loop rule (per user request).** After each `BENCH-AM-N` chunk completes, the next agent session must re-run the quality harness, diff against the previous round, and either promote `BENCH-AM-(N+1)` or open a new fix chunk if a regression appears. Stop only when TerranSoul holds rank 1 on every measured metric and `BENCH-AM-7` is done.

---



 



