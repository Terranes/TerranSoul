# TerranSoul MTEB LoCoMo Retrieval Report

Date: 2026-05-12T02:21:28.044Z
Dataset: mteb/LoCoMo @ 02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac
Systems: search, rrf
Tasks: single_hop, multi_hop, open_domain
Top K requested: 100

This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.

## Overall

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| overall | search | 389 | 17.9% | 33.8% | 39.7% | 46.2% | 59.9% | 30.7% | 25.7% | 32.1% | 522.76ms | 7,183 |
| overall | rrf | 389 | 18.5% | 35.4% | 41.7% | 47.3% | 60.7% | 32.0% | 26.7% | 33.3% | 365.66ms | 7,206 |

## By Task

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| single_hop | search | 150 | 32.3% | 54.7% | 57.7% | 64.3% | 73.7% | 45.3% | 41.2% | 42.3% | 474.12ms | 7,197 |
| single_hop | rrf | 150 | 33.0% | 56.0% | 61.0% | 65.7% | 75.0% | 46.9% | 42.2% | 43.2% | 301.45ms | 7,281 |
| multi_hop | search | 150 | 6.9% | 20.4% | 29.6% | 37.8% | 54.2% | 22.5% | 16.0% | 27.8% | 461.61ms | 7,188 |
| multi_hop | rrf | 150 | 7.8% | 22.6% | 31.0% | 38.4% | 56.4% | 24.0% | 17.3% | 29.9% | 346.32ms | 7,134 |
| open_domain | search | 89 | 12.2% | 21.1% | 26.4% | 29.8% | 46.3% | 19.9% | 15.9% | 22.0% | 707.81ms | 7,150 |
| open_domain | rrf | 89 | 12.2% | 22.3% | 27.0% | 31.5% | 43.8% | 20.4% | 16.2% | 22.5% | 506.44ms | 7,201 |

## Methodology Notes

- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.
- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.
- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.
- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.
