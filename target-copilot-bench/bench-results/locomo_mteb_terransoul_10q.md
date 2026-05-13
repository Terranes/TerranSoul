# TerranSoul MTEB LoCoMo Retrieval Report

Date: 2026-05-13T00:52:44.665Z
Dataset: mteb/LoCoMo @ 02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac
Systems: rrf_ctx_rerank
Tasks: adversarial
Top K requested: 100

This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.

## Overall

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| overall | rrf_ctx_rerank | 10 | 0.0% | 40.0% | 60.0% | 90.0% | 90.0% | 24.6% | 13.7% | 15.6% | 3410.01ms | 2,653 |

## By Task

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| adversarial | rrf_ctx_rerank | 10 | 0.0% | 40.0% | 60.0% | 90.0% | 90.0% | 24.6% | 13.7% | 15.6% | 3410.01ms | 2,653 |

## Methodology Notes

- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.
- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.
- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.
- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.
