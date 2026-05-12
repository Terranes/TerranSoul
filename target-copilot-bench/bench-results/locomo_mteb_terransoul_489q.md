# TerranSoul MTEB LoCoMo Retrieval Report

Date: 2026-05-12T13:51:01.986Z
Dataset: mteb/LoCoMo @ 02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac
Systems: rrf
Tasks: single_hop, multi_hop, temporal_reasoning, open_domain, adversarial
Top K requested: 100

This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.

## Overall

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| overall | rrf | 489 | 27.7% | 48.2% | 56.9% | 64.6% | 79.9% | 43.2% | 37.1% | 42.8% | 1172.01ms | 7,704 |

## By Task

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| single_hop | rrf | 100 | 36.5% | 54.0% | 65.0% | 71.0% | 79.0% | 49.2% | 44.3% | 45.1% | 1394.93ms | 7,729 |
| multi_hop | rrf | 100 | 9.9% | 24.4% | 35.5% | 47.3% | 71.6% | 27.5% | 18.9% | 37.0% | 980.83ms | 7,666 |
| temporal_reasoning | rrf | 100 | 41.5% | 76.5% | 82.5% | 89.0% | 95.0% | 62.8% | 56.2% | 57.4% | 860.00ms | 7,657 |
| open_domain | rrf | 89 | 12.5% | 25.1% | 32.2% | 41.9% | 62.0% | 23.2% | 18.1% | 25.1% | 1556.37ms | 7,687 |
| adversarial | rrf | 100 | 36.5% | 58.5% | 66.5% | 71.5% | 90.0% | 51.2% | 46.2% | 47.2% | 1110.21ms | 7,777 |

## Methodology Notes

- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.
- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.
- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.
- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.
