# TerranSoul MTEB LoCoMo Retrieval Report

Date: 2026-05-12T09:19:28.089Z
Dataset: mteb/LoCoMo @ 02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac
Systems: rrf
Tasks: single_hop, multi_hop, open_domain, adversarial
Top K requested: 100

This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.

## Overall

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| overall | rrf | 1655 | 29.4% | 54.2% | 64.0% | 74.5% | 89.4% | 47.7% | 41.0% | 46.1% | 1102.97ms | 7,300 |

## By Task

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| single_hop | rrf | 840 | 40.1% | 64.6% | 73.5% | 83.1% | 94.7% | 56.4% | 50.7% | 52.6% | 1169.20ms | 7,283 |
| multi_hop | rrf | 280 | 11.8% | 35.0% | 46.2% | 59.1% | 81.3% | 36.8% | 26.8% | 45.5% | 1000.20ms | 7,400 |
| open_domain | rrf | 89 | 15.1% | 36.0% | 42.0% | 53.5% | 71.5% | 31.0% | 24.5% | 32.8% | 1236.24ms | 7,311 |
| adversarial | rrf | 446 | 23.3% | 50.2% | 61.7% | 72.1% | 88.1% | 41.4% | 34.9% | 36.7% | 1016.18ms | 7,268 |

## Methodology Notes

- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.
- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.
- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.
- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.
