# MCP Data

`mcp-data/` is the repo-local data directory for `npm run mcp`, the
headless TerranSoul brain server used by coding agents.

## What is tracked

Only `mcp-data/shared/` is committed. It contains safe, reviewable seed
knowledge that every clone can use to bootstrap MCP with TerranSoul project
context. Contributors and self-improve runs may update this shared dataset when
architecture, workflows, or agent guidance changes.

## What is ignored

Runtime files stay local and must not be committed:

- `mcp-token.txt` and `.vscode/.mcp-token`
- `memory.db`, SQLite WAL/SHM files, and other `*.sqlite*` databases
- ANN/vector indexes such as `*.usearch`
- logs, locks, temporary files, sessions, and worktrees

This split lets the repository share useful MCP knowledge without leaking
secrets or machine-local agent state.
