# TerranSoul MTEB LoCoMo Retrieval Report

Date: 2026-05-12T06:04:40.668Z
Dataset: mteb/LoCoMo @ 02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac
Systems: rrf_emb, search_emb
Tasks: single_hop, adversarial
Top K requested: 100

This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.

## Overall

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| overall | rrf_emb | 100 | 35.0% | 62.5% | 68.5% | 72.5% | 83.0% | 51.1% | 45.5% | 46.1% | 433.65ms | 7,643 |
| overall | search_emb | 100 | 36.0% | 58.5% | 69.5% | 71.5% | 79.5% | 51.8% | 46.2% | 46.6% | 392.68ms | 7,559 |

## By Task

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| single_hop | rrf_emb | 50 | 36.0% | 64.0% | 66.0% | 70.0% | 82.0% | 51.2% | 46.4% | 47.0% | 367.81ms | 7,654 |
| single_hop | search_emb | 50 | 38.0% | 60.0% | 68.0% | 68.0% | 82.0% | 52.7% | 47.8% | 48.1% | 333.16ms | 7,577 |
| adversarial | rrf_emb | 50 | 34.0% | 61.0% | 71.0% | 75.0% | 84.0% | 51.0% | 44.6% | 45.2% | 499.48ms | 7,633 |
| adversarial | search_emb | 50 | 34.0% | 57.0% | 71.0% | 75.0% | 77.0% | 50.9% | 44.6% | 45.2% | 452.21ms | 7,542 |

## Methodology Notes

- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.
- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.
- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.
- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.
