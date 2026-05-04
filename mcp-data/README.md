# MCP Data

`mcp-data/` is the repo-local data directory for `npm run mcp`, the
headless TerranSoul brain server used by coding agents.

## What is tracked

Only `mcp-data/shared/` is committed. It contains safe, reviewable seed
knowledge that every clone can use to bootstrap MCP with TerranSoul project
context:

- `memory-seed.sql` — curated facts inserted into a fresh `memory.db`
- `project-index.md` — full repo map (modules, stores, composables, docs, rules)
- `lessons-learned.md` — durable gotchas / decisions from `rules/completion-log.md`
- `memory-philosophy.md` — *the* lesson on why markdown ≠ memory and why TerranSoul uses SQLite + HNSW + KG edges; non-negotiable rules for future PRs (credit: Jonathan Edwards, *Stop Calling It Memory*)
- `claudia-research.md` — reverse-engineering notes on `kbanc85/claudia` (PolyForm-NC) with 10 adoption proposals mapped onto existing TerranSoul modules
- `brain_config.json` / `app_settings.json` — safe headless defaults

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

This split lets the repository share useful MCP knowledge without leaking
secrets or machine-local agent state.
