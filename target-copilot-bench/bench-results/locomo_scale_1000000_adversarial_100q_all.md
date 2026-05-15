# TerranSoul LoCoMo-at-Scale Report (BENCH-SCALE-1)

Date: 2026-05-14T16:39:39.986Z
Task: adversarial
Scale: 1,000,000 chunks
Systems: rrf_rerank
Shard mode: all
Ingest time: 24048.3s (1000000 embedded)

## Quality + Latency

| System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg lat | p50 | p95 | p99 | Max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rrf_rerank | 100 | 14.5% | 46.0% | 59.5% | 69.5% | 70.5% | 34.9% | 27.0% | 28.4% | 5140.99ms | 4127.58ms | 7401.79ms | 67075.15ms | 67075.15ms |

## Methodology

- Loads MTEB LoCoMo `<task>-corpus`, `<task>-queries`, `<task>-qrels` parquet files from the cached download.
- Augments with cross-task LoCoMo prose as natural distractors, then deterministic entity-swap paraphrases of gold chunks, then synthetic template prose to reach `--scale`.
- Ingests in batches of 500 through `longmemeval-ipc` with `LONGMEM_EMBED=1` (mxbai-embed-large via Ollama, HNSW ANN).
- Runs each `--systems` mode against the buried corpus, records per-query latency.
- Acceptance (BENCH-SCALE-1): R@10 within 10pp of LCM-8 5k baseline AND p99 <= 200ms.
