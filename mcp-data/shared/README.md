# Shared MCP Seed Dataset

These files seed the headless MCP brain used by coding agents:

| File | Purpose |
|---|---|
| `brain_config.json` | Safe default brain mode for headless MCP |
| `app_settings.json` | Headless-safe app settings |
| `memory-seed.sql` | Curated TerranSoul project memories inserted into a fresh `memory.db` |
| `project-index.md` | Single source of truth for navigating the repo without rescanning (modules, stores, composables, docs, rules) |
| `lessons-learned.md` | Durable gotchas/decisions distilled from `rules/completion-log.md` so the same problem is never re-solved |

`npm run mcp` reads this directory on first run before falling back to compiled
defaults. If `mcp-data/memory.db` already exists, startup never overwrites it.

Update `memory-seed.sql`, `project-index.md`, or `lessons-learned.md` whenever
the repo gains durable MCP/self-improve knowledge that future agents should
inherit. Other contributors and self-improve runs may extend these files.
