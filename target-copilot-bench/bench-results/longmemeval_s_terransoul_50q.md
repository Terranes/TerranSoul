# TerranSoul LongMemEval-S Retrieval Report

Date: 2026-05-11T20:35:29.860Z
Dataset: D:\Git\TerranSoul\target-copilot-bench\longmemeval\longmemeval_s_cleaned.json
Questions: 50 (0 abstention rows excluded)
Methodology: retrieval-only recall_any@K, matching agentmemory benchmark/longmemeval-bench.ts

| System | R@5 | R@10 | R@20 | NDCG@10 | MRR | Avg latency | Avg retrieved tokens |
|---|---:|---:|---:|---:|---:|---:|---:|
| search | 100.0% | 100.0% | 100.0% | 95.6% | 94.2% | 232.08ms | 49,712 |
| rrf | 100.0% | 100.0% | 100.0% | 95.6% | 94.2% | 377.68ms | 59,239 |

## By Question Type

### search

| Type | Count | R@5 | R@10 | NDCG@10 | MRR |
|---|---:|---:|---:|---:|---:|
| single-session-user | 50 | 100.0% | 100.0% | 95.6% | 94.2% |

### rrf

| Type | Count | R@5 | R@10 | NDCG@10 | MRR |
|---|---:|---:|---:|---:|---:|
| single-session-user | 50 | 100.0% | 100.0% | 95.6% | 94.2% |

## Methodology Notes

- This is not official LongMemEval QA accuracy. It is retrieval-only recall on the LongMemEval-S haystack.
- Each question builds a fresh in-memory TerranSoul `MemoryStore` from that question's haystack sessions, searches with the raw question text, and checks whether any gold answer session appears in the retrieved top-K.
- The optional Ollama judge is a local diagnostic for evidence support and is not comparable to agentmemory's published retrieval-only number.
