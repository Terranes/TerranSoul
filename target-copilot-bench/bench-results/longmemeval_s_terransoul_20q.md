# TerranSoul LongMemEval-S Retrieval Report

Date: 2026-05-11T19:45:07.228Z
Dataset: D:\Git\TerranSoul\target-copilot-bench\longmemeval\longmemeval_s_cleaned.json
Questions: 20 (0 abstention rows excluded)
Methodology: retrieval-only recall_any@K, matching agentmemory benchmark/longmemeval-bench.ts

| System | R@5 | R@10 | R@20 | NDCG@10 | MRR | Avg latency | Avg retrieved tokens |
|---|---:|---:|---:|---:|---:|---:|---:|
| search | 100.0% | 100.0% | 100.0% | 95.0% | 93.3% | 303.15ms | 44,681 |
| rrf | 100.0% | 100.0% | 100.0% | 95.0% | 93.3% | 100.70ms | 57,180 |
| emb | 5.0% | 5.0% | 5.0% | 5.0% | 5.0% | 13.92ms | 28,203 |
| rrf_emb | 35.0% | 70.0% | 100.0% | 36.1% | 28.2% | 318.63ms | 40,658 |

## By Question Type

### search

| Type | Count | R@5 | R@10 | NDCG@10 | MRR |
|---|---:|---:|---:|---:|---:|
| single-session-user | 20 | 100.0% | 100.0% | 95.0% | 93.3% |

### rrf

| Type | Count | R@5 | R@10 | NDCG@10 | MRR |
|---|---:|---:|---:|---:|---:|
| single-session-user | 20 | 100.0% | 100.0% | 95.0% | 93.3% |

### emb

| Type | Count | R@5 | R@10 | NDCG@10 | MRR |
|---|---:|---:|---:|---:|---:|
| single-session-user | 20 | 5.0% | 5.0% | 5.0% | 5.0% |

### rrf_emb

| Type | Count | R@5 | R@10 | NDCG@10 | MRR |
|---|---:|---:|---:|---:|---:|
| single-session-user | 20 | 35.0% | 70.0% | 36.1% | 28.2% |

## Methodology Notes

- This is not official LongMemEval QA accuracy. It is retrieval-only recall on the LongMemEval-S haystack.
- Each question builds a fresh in-memory TerranSoul `MemoryStore` from that question's haystack sessions, searches with the raw question text, and checks whether any gold answer session appears in the retrieved top-K.
- The optional Ollama judge is a local diagnostic for evidence support and is not comparable to agentmemory's published retrieval-only number.
