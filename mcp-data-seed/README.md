# MCP Data Seed

This directory contains **committed seed data** for the TerranSoul MCP
brain server. When `npm run mcp` starts for the first time (no existing
`mcp-data/memory.db`), the seed files are copied into `mcp-data/` so
that the brain already knows about TerranSoul's architecture, setup
instructions, and recommended configuration.

## Files

| File | Purpose |
|------|---------|
| `brain_config.json` | Default brain mode (free API via Pollinations — no API key needed) |
| `app_settings.json` | Safe app settings for headless MCP mode |
| `memory-seed.sql` | SQL INSERT statements that pre-populate the memory store with TerranSoul knowledge |

## Updating the seed

When significant architecture changes happen, update `memory-seed.sql`
with new facts. The seed is only applied on **first run** — existing
`mcp-data/` directories are never overwritten.

## Security

- No secrets (tokens, keys) are stored here — those are generated at runtime.
- The `device_key.json` is always freshly generated per clone.
- The `mcp-token.txt` is always freshly generated per clone.
