# MCP Data

`mcp-data/` is the repo-local data directory for `npm run mcp`, the
TerranSoul MCP tray/coding-agent brain runtime.

## What is tracked

Only `mcp-data/shared/` is committed. It contains safe, reviewable seed
knowledge that every clone can use to bootstrap MCP with TerranSoul project
context:

- `memory-seed.sql` — curated facts inserted into a fresh `memory.db`
- `project-index.md` — full repo map (modules, stores, composables, docs, rules)
- `lessons-learned.md` — durable gotchas / decisions from `rules/completion-log.md`
- `memory-philosophy.md` — *the* lesson on why markdown != memory and why TerranSoul uses SQLite + vector search + KG edges; non-negotiable rules for future PRs (credit: Jonathan Edwards, *Stop Calling It Memory*)
- `claudia-research.md` — reverse-engineering notes on `kbanc85/claudia` (PolyForm-NC) with 10 adoption proposals mapped onto existing TerranSoul modules
- `brain_config.json` / `app_settings.json` — safe MCP coding-agent defaults

Contributors and self-improve runs may update this shared dataset when
architecture, workflows, or agent guidance changes. Treat it as the
**default TerranSoul knowledge base** so the same problem is never solved
twice and the codebase never has to be rescanned from scratch.

## What is ignored

Runtime files stay local and must not be committed:

- `mcp-token.txt` and `.vscode/.mcp-token`
- `memory.db`, SQLite WAL/SHM files, and other `*.sqlite*` databases
- ANN/vector indexes such as `*.usearch`
- logs, locks, temporary files, sessions, and worktrees

MCP self-improve runtime logs are intentionally local: `self_improve_runs.jsonl`,
`self_improve_gates.jsonl`, and `self_improve_mcp.jsonl` keep only the current
file plus `.001`, capped at 1 MiB per file. Durable lessons still belong in
`mcp-data/shared/memory-seed.sql`, not in runtime logs.

This split lets the repository share useful MCP knowledge without leaking
secrets or machine-local agent state.

## Seed bootstrap model

Fresh databases load the single consolidated init snapshot from
`mcp-data/shared/memory-seed.sql`. New durable knowledge is appended to that
file as `INSERT INTO memories ... WHERE NOT EXISTS` blocks plus matching
`INSERT OR IGNORE INTO memory_edges` rows; the runtime loader replays the
snapshot once on first boot and records the version checksum. Per-chunk
numbered migration files are no longer used.
