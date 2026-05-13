# MTEB LoCoMo Retrieval Adapter

TerranSoul's LoCoMo adapter runs the MTEB `mteb/LoCoMo` text-retrieval dataset through the real Rust `MemoryStore` path. It is a retrieval-only qrel benchmark over the MTEB corpus/query/qrels parquet configs, not end-to-end LoCoMo QA accuracy.

## Source

- Dataset: `mteb/LoCoMo`
- Revision: `02e2c3dea15d9fdfd1cd7a0f65f5f8ae2ed4c1ac`
- Configs: `single_hop`, `multi_hop`, `temporal_reasoning`, `open_domain`, `adversarial`
- Files used per config: `*-corpus`, `*-queries`, `*-qrels`

The Hugging Face datasets-server row endpoint returned HTTP 501 for at least `single_hop-queries`, so the adapter reads the raw parquet files directly with `hyparquet`.

## Commands

```pwsh
# Download/cache the pinned parquet files.
npm run brain:locomo:prepare

# Fast smoke pass over 10 single-hop queries.
npm run brain:locomo:sample

# Broader slice: 50 queries per task, both MemoryStore modes.
npm run brain:locomo:run -- --limit=50 --systems=search,rrf

# Full retrieval-only run.
npm run brain:locomo:run -- --systems=search,rrf
```

Dataset files are cached under `target-copilot-bench/locomo-mteb/`, and reports are written to `target-copilot-bench/bench-results/`.

## Systems

| System | MemoryStore path |
|---|---|
| `search` | FTS5 lexical search plus TerranSoul lexical rerank and gated graph boost when edges exist |
| `rrf` | `hybrid_search_rrf(query, None, top_k)` over the fresh in-memory store |

## Metrics

The adapter computes qrel-based IR metrics per query and averages them by task and overall:

- `R@K`: relevant-document coverage at K.
- `Hit@K`: whether any relevant document appears in top K.
- `NDCG@10`: graded ranking quality using qrel scores.
- `MAP@10`: truncated average precision.
- `MRR@100`: reciprocal rank of the first relevant hit.

## Current Verified Slice

The first broad slice used `npm run brain:locomo:run -- --limit=50 --systems=search,rrf`, covering 250 queries total.

| System | Queries | R@1 | R@5 | R@10 | R@20 | R@100 | NDCG@10 | MAP@10 | MRR@100 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `search` | 250 | 28.9% | 46.6% | 51.3% | 57.5% | 65.9% | 40.9% | 36.3% | 40.5% |
| `rrf` | 250 | 29.4% | 46.8% | 51.6% | 57.3% | 65.9% | 41.5% | 36.9% | 41.4% |

Strongest task: `temporal_reasoning` at R@10 90.0% / NDCG@10 78.4% for both modes. Weakest tasks: `multi_hop` and `open_domain`, which need query decomposition or stronger semantic retrieval before TerranSoul can claim a leading LoCoMo retrieval score.

## Notes

LoCoMo/LMEB published QA scores from systems such as Mem0, Letta/MemGPT, and MemPalace are not directly comparable to this MTEB retrieval-only table. Keep them in comparison docs as context only, and prefer this adapter when making apples-to-apples TerranSoul retrieval claims.
