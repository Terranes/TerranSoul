# TerranSoul MTEB LoCoMo Retrieval Report

Date: 2026-05-12T02:33:01.528Z
Dataset: mteb/LoCoMo @ 02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac
Systems: search, rrf
Tasks: single_hop, multi_hop, temporal_reasoning, open_domain, adversarial
Top K requested: 100

This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.

## Overall

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| overall | search | 250 | 28.5% | 46.2% | 53.6% | 58.5% | 70.8% | 41.4% | 36.2% | 40.5% | 486.27ms | 7,219 |
| overall | rrf | 250 | 28.9% | 47.1% | 54.4% | 58.7% | 71.3% | 42.0% | 36.7% | 41.1% | 407.18ms | 7,298 |

## By Task

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| single_hop | search | 50 | 34.0% | 58.0% | 62.0% | 66.0% | 76.0% | 48.2% | 43.7% | 44.3% | 334.89ms | 7,097 |
| single_hop | rrf | 50 | 36.0% | 58.0% | 62.0% | 66.0% | 76.0% | 48.9% | 44.6% | 45.2% | 328.54ms | 7,371 |
| multi_hop | search | 50 | 4.7% | 16.8% | 32.1% | 41.4% | 60.4% | 20.3% | 12.5% | 24.8% | 590.70ms | 7,372 |
| multi_hop | rrf | 50 | 4.7% | 19.0% | 33.2% | 40.4% | 64.4% | 21.6% | 13.3% | 26.7% | 427.52ms | 7,367 |
| temporal_reasoning | search | 50 | 64.0% | 86.0% | 88.0% | 90.0% | 100.0% | 76.3% | 72.5% | 72.9% | 239.65ms | 7,184 |
| temporal_reasoning | rrf | 50 | 64.0% | 84.0% | 90.0% | 90.0% | 100.0% | 76.9% | 72.7% | 72.9% | 288.36ms | 7,167 |
| open_domain | search | 50 | 9.7% | 17.3% | 23.0% | 28.0% | 40.7% | 17.4% | 13.6% | 20.3% | 890.80ms | 7,207 |
| open_domain | rrf | 50 | 9.7% | 19.3% | 26.0% | 30.0% | 39.2% | 18.8% | 14.4% | 21.0% | 609.53ms | 7,241 |
| adversarial | search | 50 | 30.0% | 53.0% | 63.0% | 67.0% | 77.0% | 44.8% | 39.0% | 40.0% | 375.31ms | 7,235 |
| adversarial | rrf | 50 | 30.0% | 55.0% | 61.0% | 67.0% | 77.0% | 44.0% | 38.6% | 39.8% | 381.98ms | 7,343 |

## Methodology Notes

- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.
- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.
- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.
- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.
