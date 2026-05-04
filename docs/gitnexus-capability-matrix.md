# GitNexus Capability Matrix

> **Source compared:** [abhigyanpatwari/GitNexus](https://github.com/abhigyanpatwari/GitNexus)
> v1.6.3 (May 2026), README + ARCHITECTURE.md.
> **Audit date:** 2026-05-04.
> **License note:** GitNexus is licensed **PolyForm Noncommercial 1.0.0**.
> We do **not** copy any GitNexus source. We only study its public
> capability surface and design a license-clean, local-first equivalent
> where it fits TerranSoul's scope.

## Honest summary

**TerranSoul cannot compete with GitNexus on code intelligence today.**

GitNexus is a *code-knowledge-graph* engine: tree-sitter → typed
relationships (CALLS, IMPORTS, EXTENDS, IMPLEMENTS) → cluster
detection → execution-flow tracing → 16 MCP tools tuned for AI
coding agents.

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
clone GitNexus.

## Capability matrix

| Capability | GitNexus | TerranSoul today | Gap | Tracked |
|---|---|---|---|---|
| **Index pipeline** | tree-sitter native bindings, 14 languages, multi-phase pipeline (structure / parse / resolve / cluster / process / search) | None — no code parser, no AST, no symbol table | Total | Chunk 31.3 (proposed) |
| **Storage** | LadybugDB embedded graph DB (formerly KuzuDB), Cypher queries, vector index | SQLite memory store, FTS5 keyword + HNSW vector ANN, no graph schema for code | Total — different DB shape | Chunk 31.4 (proposed) |
| **Symbol-level retrieval** | Functions/classes/methods/interfaces with file+line, heritage, exports, imports | Free-text chunks only | Total | Chunk 31.3 |
| **Call graph (CALLS)** | Cross-file resolved call edges with confidence scores | None | Total | Chunk 31.3 |
| **Import graph (IMPORTS)** | Cross-file import resolution + named bindings + re-exports | None | Total | Chunk 31.3 |
| **Heritage graph (EXTENDS / IMPLEMENTS)** | Per-language heritage extraction | None | Total | Chunk 31.3 |
| **Type resolution / receiver inference** | Constructor inference, `self`/`this` mapping, return-aware loop inference | None | Total | Chunk 31.3 |
| **Functional clustering** | Leiden community detection on the symbol graph | None | Total | Chunk 31.5 |
| **Execution-flow / process tracing** | Entry-point scoring → call-chain traces → `process_id` indexing | None | Total | Chunk 31.5 |
| **Hybrid search** | BM25 + semantic (HF transformers.js) + RRF, process-grouped | BM25 + ANN + RRF over text chunks (no symbol awareness) | Partial — engine exists, not aimed at code | Chunk 31.6 |
| **`query` MCP tool (process-grouped search)** | Returns processes + step-indexed symbols + definitions | `brain_search` returns text chunks | Partial | Chunk 31.6 |
| **`context` MCP tool (360° symbol view)** | incoming/outgoing CALLS, IMPORTS, processes, depth grouping | None | Total | Chunk 31.6 |
| **`impact` MCP tool (blast-radius)** | Upstream/downstream traversal with confidence + relation filters | None | Total | Chunk 31.6 |
| **`detect_changes` MCP tool (git-diff impact)** | Maps changed lines to affected processes via symbol resolution | None — we have git mirror but no symbol map | Total | Chunk 31.6 |
| **`rename` MCP tool (multi-file)** | Graph-coordinated rename + text fallback, dry-run + edit list | None | Total | Chunk 31.7 |
| **`cypher` MCP tool (raw graph)** | Direct Cypher over LadybugDB | None | Total | Skip — depends on DB choice |
| **Multi-repo `group_*` tools** | Cross-repo contract extraction + matching, group queries | None | Total | Out of scope (single-machine companion) |
| **PreToolUse / PostToolUse hooks** | Claude Code integration enriches tool calls + detects stale index after commits | None | Total | Chunk 31.8 |
| **Repo-specific generated skill files** | `.claude/skills/generated/` per detected community | None | Total | Optional (ties to clustering) |
| **Wiki generation** | LLM-powered docs from the graph | None — but TerranSoul has Obsidian export and brain summaries | Partial — different shape | Chunk 31.9 |
| **Web UI graph explorer** | React + Sigma.js + Graphology, WebGL graph rendering, in-browser AI chat | Tauri Vue UI for the *companion*, no code-graph view | Total in coding-UX terms | Chunk 31.1 (already queued) |
| **Embeddings** | HF transformers.js (GPU/CPU/WebGPU) | Ollama `nomic-embed-text` (768-dim) + cloud `/v1/embeddings` | Different vendor, similar coverage | None — TerranSoul's choice is fine |
| **Privacy / locality** | All local, `.gitnexus/` in repo, no network | All local, `mcp-data/` in repo, no network | Match | — |
| **MCP transports** | stdio (Cursor/Claude Code/Codex/Windsurf/OpenCode) + HTTP for Web UI | stdio + HTTP, both | Match (after Chunk 30.7.5) | — |
| **Zero-config setup** | `gitnexus setup` auto-detects editors and writes global MCP configs | Manual `.vscode/mcp.json` editing today | Partial | Chunk 31.10 |
| **`MCP resources`** | `gitnexus://repos`, `gitnexus://repo/{name}/context`, `/clusters`, `/processes`, `/schema` for instant context | None — only tools, no MCP `resources` | Total | Chunk 31.6 |
| **MCP prompts** | `detect_impact`, `generate_map` guided workflows | None | Total | Chunk 31.6 |

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
- **Multi-repo group sync / contracts** — TerranSoul is a
  per-machine companion; cross-machine code-graph sync is an
  enterprise feature with a different threat model.
- **Cloud SaaS / Kubernetes signed-image flow** — out of scope.
- **Copying any GitNexus source.** PolyForm-NC is not compatible
  with TerranSoul's MIT licensing. We design from the public API
  surface only.

## Next steps (queued)

See `rules/milestones.md` for the chunk rows. The 31.x series is
ordered so each chunk produces a usable increment:

| Chunk | What it adds | Why it's first |
|---|---|---|
| 31.1 | Pet-mode Web UI (already queued) | Immediate visibility; no code-graph dependency. |
| 31.2 | This audit doc + matrix | Done in this PR. |
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
