# TerranSoul MTEB LoCoMo Retrieval Report

Date: 2026-05-13T00:31:47.362Z
Dataset: mteb/LoCoMo @ 02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac
Systems: rrf_hyde_rerank
Tasks: single_hop, multi_hop, temporal_reasoning, open_domain, adversarial
Top K requested: 100

This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.

## Overall

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| overall | rrf_hyde_rerank | 1976 | 27.6% | 58.2% | 68.5% | 73.7% | 74.7% | 49.1% | 41.5% | 45.8% | 4394.10ms | 2,412 |

## By Task

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| single_hop | rrf_hyde_rerank | 840 | 34.9% | 68.1% | 77.1% | 80.7% | 81.3% | 56.3% | 49.3% | 50.5% | 4111.14ms | 2,426 |
| multi_hop | rrf_hyde_rerank | 280 | 12.4% | 34.3% | 45.0% | 52.2% | 54.4% | 36.9% | 26.6% | 47.4% | 4771.77ms | 2,437 |
| temporal_reasoning | rrf_hyde_rerank | 321 | 40.3% | 70.5% | 77.1% | 80.3% | 81.2% | 60.0% | 53.7% | 55.9% | 4406.58ms | 2,333 |
| open_domain | rrf_hyde_rerank | 89 | 9.9% | 28.2% | 38.1% | 43.3% | 44.4% | 25.4% | 18.8% | 25.1% | 4859.24ms | 2,412 |
| adversarial | rrf_hyde_rerank | 446 | 17.9% | 51.7% | 67.0% | 75.4% | 76.7% | 40.2% | 31.8% | 32.6% | 4588.10ms | 2,427 |

## Methodology Notes

- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.
- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.
- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.
- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.
