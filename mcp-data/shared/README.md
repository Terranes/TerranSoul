# Shared MCP Seed Dataset

These files seed the MCP tray/coding-agent brain runtime:

| File | Purpose |
|---|---|
| `brain_config.json` | Safe default brain mode for MCP coding agents |
| `app_settings.json` | Safe app settings for MCP coding-agent sessions |
| `memory-seed.sql` | Curated TerranSoul project memories inserted into a fresh `memory.db` |
| `project-index.md` | Single source of truth for navigating the repo without rescanning (modules, stores, composables, docs, rules) |
| `lessons-learned.md` | Durable gotchas/decisions distilled from `rules/completion-log.md` so the same problem is never re-solved |
| `memory-philosophy.md` | Why markdown ≠ memory and why TerranSoul uses a real database; non-negotiable rules for any future "memory" PR (credit: Jonathan Edwards, *Stop Calling It Memory*) |
| `claudia-research.md` | Reverse-engineering notes on `kbanc85/claudia` (PolyForm-NC) with 10 adoption proposals mapped onto existing TerranSoul modules |

`npm run mcp` reads this directory on first run before falling back to compiled
defaults. If `mcp-data/memory.db` already exists, startup never overwrites it.
On first run, the MCP coding-agent runtime now runs a best-effort embedding backfill
after loading `brain_config.json`; provider embeddings are preferred, and the
deterministic offline embedder warms the SQLite + vector search + RRF path when
no provider embedding endpoint is available. Default builds use a pure-Rust
linear vector index; the optional `native-ann` feature enables persisted HNSW.

Update `memory-seed.sql`, `project-index.md`, or `lessons-learned.md` whenever
the repo gains durable MCP/self-improve knowledge that future agents should
inherit. Other contributors and self-improve runs may extend these files.
