# TerranSoul MTEB LoCoMo Retrieval Report

Date: 2026-05-12T21:59:07.241Z
Dataset: mteb/LoCoMo @ 02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac
Systems: rrf_rerank, rrf_hyde, rrf_hyde_rerank
Tasks: single_hop, multi_hop, temporal_reasoning, open_domain, adversarial
Top K requested: 100

This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.

## Overall

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| overall | rrf_rerank | 489 | 24.6% | 51.6% | 62.7% | 68.5% | 69.1% | 45.0% | 37.4% | 43.1% | 3368.35ms | 2,380 |
| overall | rrf_hyde | 489 | 24.0% | 45.9% | 56.8% | 66.1% | 78.5% | 41.1% | 34.5% | 39.9% | 1543.09ms | 7,803 |
| overall | rrf_hyde_rerank | 489 | 24.6% | 53.0% | 63.7% | 67.7% | 68.3% | 45.1% | 37.3% | 42.4% | 3992.32ms | 2,431 |

## By Task

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| single_hop | rrf_rerank | 100 | 29.5% | 62.0% | 73.5% | 73.5% | 74.5% | 51.4% | 44.2% | 44.4% | 3241.89ms | 2,404 |
| single_hop | rrf_hyde | 100 | 30.5% | 51.5% | 64.0% | 74.0% | 82.0% | 46.2% | 40.7% | 41.8% | 1396.27ms | 7,890 |
| single_hop | rrf_hyde_rerank | 100 | 24.5% | 65.0% | 73.0% | 76.0% | 76.0% | 49.0% | 41.0% | 41.8% | 3853.94ms | 2,478 |
| multi_hop | rrf_rerank | 100 | 15.6% | 36.6% | 47.6% | 56.7% | 57.2% | 38.1% | 28.2% | 46.8% | 3314.43ms | 2,364 |
| multi_hop | rrf_hyde | 100 | 8.6% | 26.7% | 38.3% | 49.9% | 69.9% | 28.9% | 19.5% | 38.7% | 1495.87ms | 7,787 |
| multi_hop | rrf_hyde_rerank | 100 | 13.9% | 36.2% | 48.3% | 53.8% | 55.0% | 37.2% | 27.4% | 43.2% | 3961.39ms | 2,423 |
| temporal_reasoning | rrf_rerank | 100 | 51.5% | 80.5% | 86.5% | 90.0% | 90.0% | 69.8% | 64.3% | 65.0% | 3169.92ms | 2,366 |
| temporal_reasoning | rrf_hyde | 100 | 39.5% | 73.5% | 83.5% | 88.5% | 96.0% | 61.4% | 54.1% | 55.1% | 1328.36ms | 7,600 |
| temporal_reasoning | rrf_hyde_rerank | 100 | 52.5% | 84.0% | 87.0% | 89.5% | 89.5% | 70.9% | 65.2% | 66.4% | 3763.30ms | 2,365 |
| open_domain | rrf_rerank | 89 | 16.0% | 30.0% | 38.6% | 46.4% | 46.6% | 29.2% | 23.2% | 31.3% | 3762.12ms | 2,361 |
| open_domain | rrf_hyde | 89 | 8.8% | 24.5% | 31.0% | 42.0% | 56.9% | 19.9% | 14.8% | 19.7% | 1952.67ms | 7,809 |
| open_domain | rrf_hyde_rerank | 89 | 12.1% | 30.4% | 39.3% | 43.9% | 44.7% | 26.9% | 20.4% | 27.4% | 4398.36ms | 2,415 |
| adversarial | rrf_rerank | 100 | 9.5% | 46.5% | 64.5% | 73.5% | 74.5% | 34.9% | 25.5% | 26.7% | 3396.72ms | 2,402 |
| adversarial | rrf_hyde | 100 | 31.0% | 51.0% | 64.5% | 73.5% | 85.5% | 46.7% | 41.2% | 42.1% | 1587.32ms | 7,930 |
| adversarial | rrf_hyde_rerank | 100 | 18.5% | 47.0% | 68.5% | 72.5% | 73.5% | 39.7% | 30.8% | 31.7% | 4029.29ms | 2,474 |

## Methodology Notes

- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.
- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.
- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.
- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.
