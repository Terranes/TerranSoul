# Code Intelligence Capability Matrix

> **Sources compared:**
> 1. [abhigyanpatwari/GitNexus](https://github.com/abhigyanpatwari/GitNexus)
>    v1.6.3 (May 2026), README + ARCHITECTURE.md.
>    **License:** PolyForm Noncommercial 1.0.0.
> 2. [cocoindex-io/cocoindex](https://github.com/cocoindex-io/cocoindex)
>    v1.0.2 (May 2026), README + docs + examples/code_embedding.
>    **License:** Apache 2.0.
>
> **Audit date:** 2026-05-04.
> **License note:** We do **not** copy any GitNexus source (PolyForm-NC
> incompatible with MIT). CocoIndex is Apache 2.0 — compatible, but we
> still design our own implementation informed by their public patterns.
> We only study public capability surfaces and design license-clean,
> local-first equivalents where they fit TerranSoul's scope.

## Honest summary

**TerranSoul cannot compete with GitNexus on code intelligence today.**

GitNexus is a *code-knowledge-graph* engine: tree-sitter → typed
relationships (CALLS, IMPORTS, EXTENDS, IMPLEMENTS) → cluster
detection → execution-flow tracing → 16 MCP tools tuned for AI
coding agents.

**CocoIndex is a different beast — an incremental data pipeline
framework**, not a code-intelligence tool per se. Its flagship
"CocoIndex-code" MCP server uses the framework to deliver AST-aware
incremental code indexing with semantic search, call graphs, and
blast-radius analysis. The key insight: **Δ-only processing** — when
one file changes, only that file is re-parsed and only affected
embeddings/edges are recomputed. Hash-based memoization and per-row
lineage make re-indexing near-instant (80–90% cache hits).

TerranSoul's brain is a *companion-memory* engine: SQLite memory
store → hybrid search (BM25 + vector + recency + RRF) → 8 MCP tools
tuned for retrieving facts the companion has been told.

These are **different products**. The two overlap only at the MCP
boundary (both serve tools to Copilot/Codex/Claude Code). Catching
up on the code-intelligence side is a real engineering effort —
not a refactor.

The chunks queued in `rules/milestones.md` (31.x series) are the
honest, scoped path to closing the gap on the dimensions that
matter for *TerranSoul as a coding companion* without trying to
clone either project.

## Capability matrix

| Capability | GitNexus | CocoIndex / CocoIndex-code | TerranSoul today | Gap | Tracked |
|---|---|---|---|---|---|
| **Index pipeline** | tree-sitter native bindings, 14 languages, multi-phase pipeline (structure / parse / resolve / cluster / process / search) | AST-aware chunking (Python, TS, Rust, Go), incremental Δ-only processing, hash-based memoization (`@coco.fn(memo=True)`) | tree-sitter Rust + TypeScript parsing, single-pass symbol + edge extraction, `code_symbols` + `code_edges` SQLite tables, `code_index_repo` Tauri command | Partial (2/14 languages, no incremental/Δ-only yet) | Chunk 31.3 (**shipped**) |
| **Incremental re-indexing** | Not documented (full re-index implied on `detect_changes`) | Core differentiator: Δ-only — only changed files re-parse/re-embed. 80–90% cache hits on re-index. Hash of input + hash of code determines staleness. Per-row provenance traces target back to source byte | Full DELETE + re-INSERT on every `index_repo` call — no incremental support | Total — TerranSoul re-indexes from scratch every time | Future (post-31.5, adopt content-hash approach) |
| **Storage** | LadybugDB embedded graph DB (formerly KuzuDB), Cypher queries, vector index | Pluggable targets: pgvector, LanceDB, Neo4j, Qdrant, Pinecone, Turbopuffer. Persistent state in Postgres | SQLite memory store, FTS5 keyword + HNSW vector ANN, no graph schema for code | Total — different DB shape | Chunk 31.4 (proposed) |
| **Symbol-level retrieval** | Functions/classes/methods/interfaces with file+line, heritage, exports, imports | Semantic code search by meaning (not grep), AST-chunked symbols with embeddings | Functions/methods/structs/enums/classes/interfaces/traits with file+line+kind+parent, `query_symbols_by_name` + `query_symbols_in_file` | Partial (no heritage/re-exports yet, no semantic embedding of symbols) | Chunk 31.3 (**shipped**) |
| **Call graph (CALLS)** | Cross-file resolved call edges with confidence scores | Call graph + blast-radius analysis via the code MCP server | Cross-file resolved call edges with `exact`/`inferred` confidence, `code_call_graph(symbol)` Tauri command returning incoming + outgoing | Partial (name-based resolution, no full type inference) | Chunk 31.3 + 31.4 (**shipped**) |
| **Import graph (IMPORTS)** | Cross-file import resolution + named bindings + re-exports | Part of the AST analysis (details in code_embedding example) | Import resolution via symbol-name matching with file-proximity heuristic, `exact`/`inferred` confidence | Partial (no re-exports, no path-level Rust module resolution yet) | Chunk 31.3 + 31.4 (**shipped**) |
| **Heritage graph (EXTENDS / IMPLEMENTS)** | Per-language heritage extraction | Not documented separately | None | Total | Future (post-31.4) |
| **Type resolution / receiver inference** | Constructor inference, `self`/`this` mapping, return-aware loop inference | Not documented separately | None | Total | Future (post-31.4) |
| **Functional clustering** | Leiden community detection on the symbol graph | Global repo view for duplicates and architecture (via MCP server) | Label-propagation clustering + `code_clusters` table | Partial — lighter algorithm than Leiden, sufficient for companion | **Shipped (31.5)** |
| **Execution-flow / process tracing** | Entry-point scoring → call-chain traces → `process_id` indexing | Not documented as a distinct feature | BFS call-chain traces from scored entry points → `code_processes` table | Partial — scoring heuristics are simpler, no ML inference | **Shipped (31.5)** |
| **Hybrid search** | BM25 + semantic (HF transformers.js) + RRF, process-grouped | Semantic search by meaning + vector embeddings (sentence-transformers / OpenAI) | `code_query` MCP tool: symbol-name + file queries via native index; brain_search for text chunks | Partial — symbol search is name-exact, not semantic embedding yet | **Shipped (31.6)** |
| **Data lineage / provenance** | Not documented | End-to-end lineage — every target row traces back to exact source byte. Debuggable, auditable, regulator-friendly | None | Total — CocoIndex's unique strength | Future (informational — not critical for companion use-case) |
| **`query` MCP tool (process-grouped search)** | Returns processes + step-indexed symbols + definitions | Semantic code search tool in CocoIndex-code MCP server | `code_query` native MCP tool: symbol-name/file search against symbol index, process-grouped when clusters exist | Shipped — native implementation | **Shipped (31.6)** |
| **`context` MCP tool (360° symbol view)** | incoming/outgoing CALLS, IMPORTS, processes, depth grouping | Part of code intelligence MCP surface | `code_context` native MCP tool: call graph + cluster membership + process participation | Shipped — native implementation | **Shipped (31.6)** |
| **`impact` MCP tool (blast-radius)** | Upstream/downstream traversal with confidence + relation filters | Blast-radius analysis in CocoIndex-code | `code_impact` native MCP tool: BFS over incoming call edges with depth grouping | Shipped — native implementation | **Shipped (31.6)** |
| **`detect_changes` MCP tool (git-diff impact)** | Maps changed lines to affected processes via symbol resolution | Incremental — automatically knows what changed via content hashing | Removed (was sidecar-dependent); future: re-implement natively | Deferred | Future |
| **`rename` MCP tool (multi-file)** | Graph-coordinated rename + text fallback, dry-run + edit list | Not documented | None | Total | Chunk 31.7 |
| **`cypher` MCP tool (raw graph)** | Direct Cypher over LadybugDB | N/A — uses SQL/vector targets | None | Total | Skip — depends on DB choice |
| **Multi-repo `group_*` tools** | Cross-repo contract extraction + matching, group queries | Multi-repo summarization example (walk N repos, LLM-summarize) | None | Total | Out of scope (single-machine companion) |
| **PreToolUse / PostToolUse hooks** | Claude Code integration enriches tool calls + detects stale index after commits | Not documented | None | Total | Chunk 31.8 |
| **Repo-specific generated skill files** | `.claude/skills/generated/` per detected community | `.claude/` + `skills/cocoindex/` for AI coding agent integration | None | Total | Optional (ties to clustering) |
| **Wiki generation** | LLM-powered docs from the graph | Multi-codebase summarization example | None — but TerranSoul has Obsidian export and brain summaries | Partial — different shape | Chunk 31.9 |
| **Web UI graph explorer** | React + Sigma.js + Graphology, WebGL graph rendering, in-browser AI chat | N/A — CLI/library/MCP, no web UI | Tauri Vue UI for the *companion*, no code-graph view | Total in coding-UX terms | Chunk 31.1 (already queued) |
| **Embeddings** | HF transformers.js (GPU/CPU/WebGPU) | sentence-transformers / OpenAI / any embedding provider | Ollama `nomic-embed-text` (768-dim) + cloud `/v1/embeddings` | Different vendor, similar coverage | None — TerranSoul's choice is fine |
| **Privacy / locality** | All local, `.gitnexus/` in repo, no network | Configurable — can be fully local (local embedding + LanceDB) or cloud targets | All local, `mcp-data/` in repo, no network | Match | — |
| **MCP transports** | stdio (Cursor/Claude Code/Codex/Windsurf/OpenCode) + HTTP for Web UI | MCP server via CocoIndex-code (Claude Code, Cursor, others) | stdio + HTTP, both | Match (after Chunk 30.7.5) | — |
| **Zero-config setup** | `gitnexus setup` auto-detects editors and writes global MCP configs | `pip install cocoindex` + one Python file | Manual `.vscode/mcp.json` editing today | Partial | Chunk 31.10 |
| **`MCP resources`** | `gitnexus://repos`, `gitnexus://repo/{name}/context`, `/clusters`, `/processes`, `/schema` for instant context | Not documented separately | None — only tools, no MCP `resources` | Total | Chunk 31.6 |
| **MCP prompts** | `detect_impact`, `generate_map` guided workflows | Not documented separately | None | Total | Chunk 31.6 |

## What we keep (TerranSoul-original strengths)

- Companion brain — persona, voice, charisma, skill tree, persistent
  memory across all workspaces — entirely outside GitNexus's scope.
- Multi-provider LLM brain (Ollama / paid cloud / free cloud) with
  silent local-first default.
- Tauri-native desktop integration, VRM avatar, motion / animation
  pipeline.
- Knowledge-graph design (`docs/brain-advanced-design.md`) is a
  *memory* graph, not a *code* graph — they don't conflict; they
  could share a tree-sitter ingest layer in the future.

## What we deliberately do NOT chase

- **Cypher / LadybugDB** — adopting an embedded graph DB is a large
  dependency; a pragmatic v1 can use SQLite tables for nodes/edges
  and only consider a graph DB if perf demands it.
- **Full CocoIndex-style Δ pipeline** — CocoIndex is a general-purpose
  incremental ETL framework (Python + Rust, Postgres state store,
  pluggable targets). TerranSoul is a single-binary Tauri app with
  SQLite. Adopting their full lineage/memoization model would require
  a Postgres dependency and Python runtime. Instead, we adopt the
  *principle* (content-hash per file, skip unchanged files) in our
  own Rust code post-31.5.
- **Multi-repo group sync / contracts** — TerranSoul is a
  per-machine companion; cross-machine code-graph sync is an
  enterprise feature with a different threat model.
- **Cloud SaaS / Kubernetes signed-image flow** — out of scope.
- **Copying any GitNexus source.** PolyForm-NC is not compatible
  with TerranSoul's MIT licensing. We design from the public API
  surface only.
- **Copying CocoIndex source.** While Apache 2.0 is compatible,
  CocoIndex is a Python framework — we study its architecture
  patterns (incremental Δ, hash-memoization, lineage) and implement
  them idiomatically in Rust.

## Next steps (queued)

See `rules/milestones.md` for the chunk rows. The 31.x series is
ordered so each chunk produces a usable increment:

| Chunk | What it adds | Why it's first |
|---|---|---|
| 31.1 | MCP app mode + pet-mode runner | Immediate visibility; no code-graph dependency. |
| 31.2 | 5 code-intelligence MCP tools delegating to GitNexus sidecar | ✅ Shipped — external agents can now reach `code_query`/`code_context`/`code_impact`/`code_detect_changes`/`code_graph_sync` via `tools/list` when `code_read` is granted. |
| 31.3 | tree-sitter ingest + symbol table (Rust + TS first) | Foundation for all code-aware tools. |
| 31.4 | Symbol/edge SQLite schema + migration | Storage layer. |
| 31.5 | Cluster detection + entry-point scoring + processes | Enables `query` process-grouping. |
| 31.6 | New MCP tools: `code_query`, `code_context`, `code_impact`, `code_detect_changes` + MCP `resources` + `prompts` | The Copilot-facing surface that closes the gap. |
| 31.7 | `code_rename` multi-file tool | Highest-value editor action. |
| 31.8 | Pre/post-tool-use hook plumbing for Claude Code | Stale-index detection, search enrichment. |
| 31.9 | Wiki generation from the symbol graph | Reuses brain summarize pipeline. |
| 31.10 | `terransoul mcp setup` auto-config writer | Zero-config DX matching GitNexus. |

## Credit

GitNexus's public design — particularly the *precomputed relational
intelligence* idea (return complete context in one call rather than
expecting the LLM to traverse raw edges), the process-grouped
`query` shape, and the impact/context/rename tool surfaces —
informed Chunks 31.3 – 31.7. See `CREDITS.md` for the formal
attribution entry.

CocoIndex's public design — particularly the *Δ-only incremental
processing* principle (hash input + hash code to skip unchanged
work), the *per-row lineage* model (every target row traces to its
source byte), and the *AST-aware chunking* approach for code
embeddings — will inform the incremental re-indexing optimization
planned for post-31.5. See `CREDITS.md` for the formal attribution
entry.
