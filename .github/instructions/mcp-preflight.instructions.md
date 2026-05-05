---
applyTo: "**"
---

# MCP Preflight — Mandatory for every agent request

Before doing ANY work in this repository, you MUST complete the MCP
preflight check. This is not optional. Skipping it is a violation.

1. Call `brain_health` (the MCP tool) to verify the TerranSoul brain
   server is reachable. The workspace auto-starts it on folder open.
2. If `brain_health` succeeds, call `brain_search` or
   `brain_suggest_context` with the current task topic BEFORE doing
   broad codebase searches (`grep_search`, `file_search`, etc.).
3. If `brain_health` fails or MCP tools are not available, run
   `node scripts/copilot-start-mcp.mjs` in a terminal, then retry.
4. If MCP still cannot start, state the exact blocker to the user.
   Do NOT silently proceed without MCP.

After completing work, sync durable lessons into
`mcp-data/shared/memory-seed.sql`.
