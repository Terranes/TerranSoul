# Deep-Analysis-Before-Action Rule

> **Status:** Mandatory. Non-negotiable. Applies to every agent session,
> every chunk, every PR, every code change, every reverse-engineering
> task, and every "should we adopt X?" decision.

## The rule

**Never decide, propose, or apply a change to TerranSoul on a whim.**
Before any non-trivial action — adopting an idea from another project,
adding a dependency, refactoring a subsystem, accepting a research
finding, or rejecting one — the agent MUST complete a deep analysis
that consults *both* of the following sources:

1. **TerranSoul MCP brain** (`brain_search`, `brain_suggest_context`,
   `brain_get_entry`, `brain_kg_neighbors`) — the durable memory of
   what we already know, decided, tried, rejected, and learned.
2. **TerranSoul source code + rules + docs** (`rules/**`, `docs/**`,
   `src/**`, `src-tauri/src/**`, `mcp-data/shared/migrations/**`) —
   the current state of the system, not assumptions about it.

External research (DeepWiki, GitHub, papers, videos) is **input** to the
analysis, never a substitute for it. A claim like "X project has feature
Y, so we should add Y" is invalid until the analysis shows that (a) Y is
actually missing or weaker in TerranSoul, (b) adopting Y is consistent
with our architecture, licensing, and roadmap, and (c) MCP has no prior
decision rejecting Y.

## What "deep analysis" requires (no partial scans)

For every decision the agent must produce, in writing, the following:

1. **MCP-prior-art check** — at least one `brain_search` query against
   each of: the topic, the proposed change, and obvious synonyms.
   If MCP returns relevant prior decisions, they are binding context.
   If MCP is empty/blocked, record the blocker explicitly and continue
   only with caller acknowledgement — do not assume "no result" means
   "no prior decision".
2. **Source-of-truth check** — read the actual files in `src/`,
   `src-tauri/src/`, `rules/`, `docs/`, and `mcp-data/shared/migrations/`
   that touch the affected subsystem. *Reading the README is not
   sufficient.* `grep_search` / `file_search` must be issued against
   the relevant module trees, not a single guess path.
3. **Gap analysis** — explicitly state what TerranSoul already does
   for this concern, what is missing, and whether the missing piece is
   intentional (rejected before, out of scope, deferred) or genuine.
4. **Verdict** — one of `adopt`, `partial-adopt`, `defer`, `reject`,
   with a one-paragraph reason grounded in steps 1–3. "It looks cool"
   is not a reason. "Their X is structurally inferior to our Y" is.
5. **Self-improve write-back** — sync the verdict, the analysis, and
   any durable lessons into `mcp-data/shared/migrations/<NNN>_<topic>.sql`
   so future sessions retrieve the conclusion via `brain_search` and do
   not re-scan the upstream repo or re-read the same files.

## No partial scans

A "partial scan" is any of the following, and each is a violation:

- Reading only the top-level README of an upstream repo and concluding
  about its architecture.
- Searching only one keyword in MCP and declaring "no prior art".
- Reading one Rust file and inferring the shape of a whole subsystem.
- Citing a feature from another project without checking whether
  TerranSoul already implements it under a different name.
- Skipping the source-code check because "the rule probably says X".

If any required step in §"What deep analysis requires" cannot be
completed (MCP down, network down, file inaccessible, time-boxed
session), the agent must **stop and report the blocker** rather than
proceed with reduced certainty.

## Self-improving the brain (write-back contract)

Every deep analysis must end with a SQL migration appended under
`mcp-data/shared/migrations/`. Each migration:

- Has a numbered filename (`002_*.sql`, `003_*.sql`, …) — append-only,
  never edit a shipped migration.
- Uses `INSERT OR IGNORE INTO memories (...)` so re-running is safe.
- Records `RESEARCH:`, `LESSON:`, `VERDICT:`, or `RULE:` rows with
  high importance (≥7), `cognitive_kind = 'procedural'` for rules,
  `'episodic'` for one-time research.
- Adds `memory_edges` connecting the new rows to existing rules and
  modules they relate to (`supports`, `related_to`, `contradicts`,
  `superseded_by`, `part_of`).
- Is registered in `compiled_migrations()` inside
  `src-tauri/src/memory/seed_migrations.rs` so release builds pick it
  up without the on-disk migrations folder.

This is the "self-improve" loop: the cost of a deep analysis is paid
once; future agents retrieve the verdict from `brain_search` for free.
Re-scanning the same upstream repo or the same TerranSoul subsystem
across sessions is a process bug, not a feature.

## Why this rule exists

Without it, agents drift toward two failure modes:

1. **Cargo-cult adoption** — "Project X has feature Y" → add Y, even
   though TerranSoul already has Y in a different shape, or rejected
   Y three months ago.
2. **Ungrounded rejection** — "I don't think we need that" → never
   actually checked whether we have it or whether the user asked for
   it. The current source/rules disagree, but the agent doesn't read
   them.

Both failure modes destroy trust and waste session budget. The rule
binds every decision to MCP + source code, and forces the durable
write-back so each session compounds the brain instead of restarting it.

## Cross-references

- Mandatory MCP preflight: [`rules/agent-mcp-bootstrap.md`](agent-mcp-bootstrap.md)
- Reverse-engineering protocol: [`rules/research-reverse-engineering.md`](research-reverse-engineering.md)
- Reality / no-pretend rule: [`rules/reality-filter.md`](reality-filter.md)
- Coding standards: [`rules/coding-standards.md`](coding-standards.md)
- Seed migration system: [`src-tauri/src/memory/seed_migrations.rs`](../src-tauri/src/memory/seed_migrations.rs)
