# TerranSoul LongMemEval-S Retrieval Report

Date: 2026-05-11T20:39:14.353Z
Dataset: D:\Git\TerranSoul\target-copilot-bench\longmemeval\longmemeval_s_cleaned.json
Questions: 180 (0 abstention rows excluded)
Methodology: retrieval-only recall_any@K, matching agentmemory benchmark/longmemeval-bench.ts

| System | R@5 | R@10 | R@20 | NDCG@10 | MRR | Avg latency | Avg retrieved tokens |
|---|---:|---:|---:|---:|---:|---:|---:|
| search | 100.0% | 100.0% | 100.0% | 90.0% | 90.9% | 434.52ms | 58,511 |
| rrf | 100.0% | 100.0% | 100.0% | 89.6% | 90.3% | 550.85ms | 62,679 |

## By Question Type

### search

| Type | Count | R@5 | R@10 | NDCG@10 | MRR |
|---|---:|---:|---:|---:|---:|
| single-session-user | 70 | 100.0% | 100.0% | 94.6% | 92.7% |
| multi-session | 80 | 100.0% | 100.0% | 88.9% | 94.8% |
| single-session-preference | 30 | 100.0% | 100.0% | 82.2% | 76.1% |

### rrf

| Type | Count | R@5 | R@10 | NDCG@10 | MRR |
|---|---:|---:|---:|---:|---:|
| single-session-user | 70 | 100.0% | 100.0% | 94.1% | 92.0% |
| multi-session | 80 | 100.0% | 100.0% | 88.5% | 94.2% |
| single-session-preference | 30 | 100.0% | 100.0% | 81.8% | 75.7% |

## Methodology Notes

- This is not official LongMemEval QA accuracy. It is retrieval-only recall on the LongMemEval-S haystack.
- Each question builds a fresh in-memory TerranSoul `MemoryStore` from that question's haystack sessions, searches with the raw question text, and checks whether any gold answer session appears in the retrieved top-K.
- The optional Ollama judge is a local diagnostic for evidence support and is not comparable to agentmemory's published retrieval-only number.
