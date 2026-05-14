# TerranSoul LoCoMo-at-Scale Report (BENCH-SCALE-1)

Date: 2026-05-14T01:53:59.394Z
Task: adversarial
Scale: 100,000 chunks
Systems: rrf
Shard mode: routed
Ingest time: 1462.5s (100000 embedded)

## Quality + Latency

| System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg lat | p50 | p95 | p99 | Max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rrf | 100 | 30.5% | 48.0% | 58.5% | 61.5% | 76.5% | 43.2% | 38.4% | 39.3% | 1450.62ms | 959.99ms | 3075.42ms | 17406.46ms | 17406.46ms |

## Methodology

- Loads MTEB LoCoMo `<task>-corpus`, `<task>-queries`, `<task>-qrels` parquet files from the cached download.
- Augments with cross-task LoCoMo prose as natural distractors, then deterministic entity-swap paraphrases of gold chunks, then synthetic template prose to reach `--scale`.
- Ingests in batches of 500 through `longmemeval-ipc` with `LONGMEM_EMBED=1` (mxbai-embed-large via Ollama, HNSW ANN).
- Runs each `--systems` mode against the buried corpus, records per-query latency.
- Acceptance (BENCH-SCALE-1): R@10 within 10pp of LCM-8 5k baseline AND p99 <= 200ms.
