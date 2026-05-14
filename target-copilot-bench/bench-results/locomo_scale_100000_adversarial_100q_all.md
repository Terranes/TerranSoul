# TerranSoul LoCoMo-at-Scale Report (BENCH-SCALE-1)

Date: 2026-05-14T02:20:47.187Z
Task: adversarial
Scale: 100,000 chunks
Systems: rrf
Shard mode: all
Ingest time: 1461.2s (100000 embedded)

## Quality + Latency

| System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg lat | p50 | p95 | p99 | Max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rrf | 100 | 29.5% | 49.0% | 59.5% | 61.5% | 76.5% | 43.1% | 38.0% | 38.7% | 1353.44ms | 973.52ms | 3167.85ms | 6868.72ms | 6868.72ms |

## Methodology

- Loads MTEB LoCoMo `<task>-corpus`, `<task>-queries`, `<task>-qrels` parquet files from the cached download.
- Augments with cross-task LoCoMo prose as natural distractors, then deterministic entity-swap paraphrases of gold chunks, then synthetic template prose to reach `--scale`.
- Ingests in batches of 500 through `longmemeval-ipc` with `LONGMEM_EMBED=1` (mxbai-embed-large via Ollama, HNSW ANN).
- Runs each `--systems` mode against the buried corpus, records per-query latency.
- Acceptance (BENCH-SCALE-1): R@10 within 10pp of LCM-8 5k baseline AND p99 <= 200ms.
