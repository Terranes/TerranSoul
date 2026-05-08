---
applyTo: "**"
---

# MCP Preflight — Local sessions only (skip in GitHub Actions)

> **CI / GitHub Actions skip:** If `GITHUB_ACTIONS=true`, **skip this
> entire file**. MCP startup requires a Rust build (`cargo build`) which
> is too expensive for CI runners. Proceed directly to the work.

Before doing ANY work in a **local** session, you MUST complete the MCP
preflight check. Skipping it in a local session is a violation.

1. Call `brain_health` (the MCP tool) to verify the TerranSoul brain
   server is reachable. The workspace auto-starts it on folder open.
2. If `brain_health` succeeds, call `brain_search` or
   `brain_suggest_context` with the current task topic BEFORE doing
   broad codebase searches (`grep_search`, `file_search`, etc.).
3. If `brain_health` fails or MCP tools are not available, run
   `node scripts/copilot-start-mcp.mjs` in a terminal, then retry.
4. If MCP still cannot start, state the exact blocker to the user.
   Do NOT silently proceed without MCP.

## MCP tool/server error rule

If any MCP call returns an error, do not ignore it or quietly switch to
non-MCP context. Classify it immediately:

- **Bad tool arguments / contract mismatch** — fix the MCP tool schema,
   argument adapter, or gateway behavior so the natural agent call either
   works or returns a clearer repair instruction. Add a regression test.
- **Unhealthy / stale server** — rerun `brain_health`; if needed run
   `node scripts/copilot-start-mcp.mjs`, wait for health, and retry. If the
   server is stale, rebuild/relaunch per `rules/agent-mcp-bootstrap.md`.
- **Missing durable knowledge** — add or refresh the relevant row in
   `mcp-data/shared/memory-seed.sql` and, when needed for existing DBs, add
   a numbered migration under `mcp-data/shared/migrations/`.

The agent must visibly report the MCP error, root cause, fix, and any
remaining blocker. A successful non-MCP fallback is not a complete fix.

## Visible MCP receipt — also mandatory

After the MCP preflight succeeds, the agent MUST tell the user in a short
progress update that MCP was used. The receipt must include:

- `brain_health` status/provider (or exact HTTP health endpoint used).
- The `brain_search` / `brain_suggest_context` query topic used.
- A clear blocker if either call could not run.

Do not bury MCP usage only in hidden tool calls or final summaries. If the
user cannot see a receipt, treat the preflight as incomplete.

After completing work, sync durable lessons into
`mcp-data/shared/memory-seed.sql`.
