# TerranSoul LongMemEval-S Retrieval Report

Date: 2026-05-11T19:44:17.457Z
Dataset: built-in sample
Questions: 2 (0 abstention rows excluded)
Methodology: retrieval-only recall_any@K, matching agentmemory benchmark/longmemeval-bench.ts

| System | R@5 | R@10 | R@20 | NDCG@10 | MRR | Avg latency | Avg retrieved tokens |
|---|---:|---:|---:|---:|---:|---:|---:|
| emb | 100.0% | 100.0% | 100.0% | 100.0% | 100.0% | 14.93ms | 74 |
| rrf_emb | 100.0% | 100.0% | 100.0% | 100.0% | 100.0% | 15.35ms | 74 |

## By Question Type

### emb

| Type | Count | R@5 | R@10 | NDCG@10 | MRR |
|---|---:|---:|---:|---:|---:|
| single-session-user | 1 | 100.0% | 100.0% | 100.0% | 100.0% |
| multi-session | 1 | 100.0% | 100.0% | 100.0% | 100.0% |

### rrf_emb

| Type | Count | R@5 | R@10 | NDCG@10 | MRR |
|---|---:|---:|---:|---:|---:|
| single-session-user | 1 | 100.0% | 100.0% | 100.0% | 100.0% |
| multi-session | 1 | 100.0% | 100.0% | 100.0% | 100.0% |

## Methodology Notes

- This is not official LongMemEval QA accuracy. It is retrieval-only recall on the LongMemEval-S haystack.
- Each question builds a fresh in-memory TerranSoul `MemoryStore` from that question's haystack sessions, searches with the raw question text, and checks whether any gold answer session appears in the retrieved top-K.
- The optional Ollama judge is a local diagnostic for evidence support and is not comparable to agentmemory's published retrieval-only number.
