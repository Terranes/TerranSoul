# TerranSoul LoCoMo-at-Scale Report (BENCH-SCALE-1)

Date: 2026-05-14T16:41:21.481Z
Task: adversarial
Scale: 1,000,000 chunks
Systems: rrf_rerank
Shard mode: routed
Ingest time: 24050.4s (1000000 embedded)

## Quality + Latency

| System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg lat | p50 | p95 | p99 | Max |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rrf_rerank | 100 | 11.5% | 44.0% | 60.5% | 70.5% | 70.5% | 33.3% | 24.6% | 25.9% | 6288.35ms | 4167.44ms | 7957.76ms | 178116.78ms | 178116.78ms |

## Methodology

- Loads MTEB LoCoMo `<task>-corpus`, `<task>-queries`, `<task>-qrels` parquet files from the cached download.
- Augments with cross-task LoCoMo prose as natural distractors, then deterministic entity-swap paraphrases of gold chunks, then synthetic template prose to reach `--scale`.
- Ingests in batches of 500 through `longmemeval-ipc` with `LONGMEM_EMBED=1` (mxbai-embed-large via Ollama, HNSW ANN).
- Runs each `--systems` mode against the buried corpus, records per-query latency.
- Acceptance (BENCH-SCALE-1): R@10 within 10pp of LCM-8 5k baseline AND p99 <= 200ms.
