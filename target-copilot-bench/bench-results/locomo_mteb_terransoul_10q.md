# TerranSoul MTEB LoCoMo Retrieval Report

Date: 2026-05-11T21:36:01.143Z
Dataset: mteb/LoCoMo @ 02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac
Systems: search, rrf
Tasks: single_hop
Top K requested: 100

This is retrieval-only MTEB qrel scoring over the LoCoMo-derived text-retrieval task. It is not end-to-end LoCoMo QA accuracy.

## Overall

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| overall | search | 10 | 10.0% | 40.0% | 50.0% | 50.0% | 50.0% | 29.2% | 22.5% | 22.5% | 148.08ms | 6,835 |
| overall | rrf | 10 | 10.0% | 40.0% | 50.0% | 50.0% | 50.0% | 29.2% | 22.5% | 22.5% | 203.39ms | 7,310 |

## By Task

| Task | System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 | Avg latency | Avg tokens |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| single_hop | search | 10 | 10.0% | 40.0% | 50.0% | 50.0% | 50.0% | 29.2% | 22.5% | 22.5% | 148.08ms | 6,835 |
| single_hop | rrf | 10 | 10.0% | 40.0% | 50.0% | 50.0% | 50.0% | 29.2% | 22.5% | 22.5% | 203.39ms | 7,310 |

## Methodology Notes

- Each task loads the pinned MTEB `*-corpus`, `*-queries`, and `*-qrels` parquet files.
- Corpus rows are inserted into a fresh in-memory TerranSoul `MemoryStore` through the existing Rust JSONL IPC shim.
- `search` uses TerranSoul FTS5 lexical ranking and gated graph boost paths. `rrf` uses `hybrid_search_rrf(query, None, top_k)`.
- Metrics are computed from qrels: recall@K is relevant-doc coverage, hit@K is any-relevant-hit, NDCG@10 uses qrel scores, MAP@10 is truncated average precision, and MRR@100 is first relevant rank.
