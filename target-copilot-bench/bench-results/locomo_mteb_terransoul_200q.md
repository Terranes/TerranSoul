# TerranSoul MTEB LoCoMo Retrieval Report

Date: 2026-05-12T08:38:56.335Z
Dataset: mteb/LoCoMo @ 02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac
Systems: rrf
Tasks: single_hop, multi_hop, open_domain, adversarial
Top K requested: 100

This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.

## Overall

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| overall | rrf | 200 | 23.3% | 48.2% | 56.3% | 68.8% | 85.9% | 41.6% | 34.3% | 41.7% | 1220.20ms | 7,423 |

## By Task

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| single_hop | rrf | 50 | 40.0% | 66.0% | 74.0% | 82.0% | 96.0% | 56.1% | 50.4% | 51.2% | 942.07ms | 7,425 |
| multi_hop | rrf | 50 | 12.8% | 36.1% | 48.2% | 64.4% | 85.6% | 36.2% | 25.6% | 43.2% | 1131.05ms | 7,455 |
| open_domain | rrf | 50 | 12.2% | 33.8% | 39.8% | 52.0% | 67.8% | 29.2% | 22.4% | 31.7% | 1698.06ms | 7,402 |
| adversarial | rrf | 50 | 28.0% | 57.0% | 63.0% | 77.0% | 94.0% | 44.9% | 38.9% | 40.7% | 1109.61ms | 7,411 |

## Methodology Notes

- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.
- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.
- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.
- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.
