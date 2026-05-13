# TerranSoul MTEB LoCoMo Retrieval Report

Date: 2026-05-12T14:24:03.016Z
Dataset: mteb/LoCoMo @ 02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac
Systems: rrf
Tasks: single_hop, multi_hop, temporal_reasoning, open_domain, adversarial
Top K requested: 100

This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.

## Overall

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| overall | rrf | 1089 | 25.8% | 46.6% | 55.8% | 64.8% | 80.5% | 42.0% | 35.7% | 41.7% | 1049.74ms | 7,640 |

## By Task

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| single_hop | rrf | 250 | 32.8% | 52.6% | 62.4% | 71.0% | 82.8% | 46.4% | 41.3% | 42.6% | 955.27ms | 7,700 |
| multi_hop | rrf | 250 | 9.2% | 24.8% | 35.3% | 46.4% | 69.2% | 28.3% | 19.8% | 38.1% | 1179.32ms | 7,649 |
| temporal_reasoning | rrf | 250 | 35.2% | 63.7% | 71.0% | 78.3% | 89.7% | 53.9% | 47.7% | 50.5% | 928.67ms | 7,541 |
| open_domain | rrf | 89 | 12.5% | 25.1% | 32.2% | 41.9% | 62.0% | 23.2% | 18.1% | 25.1% | 1306.27ms | 7,687 |
| adversarial | rrf | 250 | 30.6% | 53.0% | 63.0% | 71.4% | 87.0% | 45.9% | 40.5% | 41.7% | 1044.39ms | 7,652 |

## Methodology Notes

- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.
- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.
- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.
- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.
