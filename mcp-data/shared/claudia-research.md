# Reverse-Engineering Notes — `kbanc85/claudia`

> Reference: <https://github.com/kbanc85/claudia> · Author: Kamil Banc.
> **License:** PolyForm Noncommercial 1.0.0 — **non-commercial use only,
> no source code copying, no redistribution of derivative works**.
> TerranSoul ships as a free product; commercial-license clearance lives
> in `docs/licensing-audit.md`. Adopting *patterns and product ideas*
> from a public README is fine and standard reverse-engineering practice;
> copying source files, prompts verbatim, asset names, identity, or
> scheduler scripts is not.
>
> Captured from the public claudia README on 2026-05-04 so future agent
> sessions don't have to re-fetch it. Crediting + adoption rules are
> persisted in `CREDITS.md` and `mcp-data/shared/memory-seed.sql`.

---

## What claudia is (one paragraph)

Claudia is a Claude-Code-hosted personal companion ("thinking partner")
that tracks **people, commitments, and judgment rules** rather than
just tasks. She runs entirely on top of `claude` CLI plus a local
Python memory daemon. She is built explicitly around the same thesis as
the Substack article: markdown is for instructions (skills,
identity), SQLite + vectors are for facts. The repo's own README
states it bluntly: *"SQLite is the source of truth; the vault is a
read-only projection you can browse and search."*

## Architecture (high-level, derived from public README + ARCHITECTURE.md)

```
You → Claude Code → Reads claudia template (markdown skills/rules) → Becomes claudia
                          │
                          ↓
                  MCP daemon (stdio, per session) ── 33 memory tools
                          │
                          ↓
                  SQLite + vector embeddings + provenance
                          ▲
                          │
              Standalone daemon (24/7, LaunchAgent on macOS)
                          │
                ┌─────────┼─────────┐
                ↓         ↓         ↓
            Scheduler  Ollama  Obsidian vault (PARA, read-only projection)
```

Two layers, two daemons:

- **Template layer** (markdown). 41 skills, rules, identity, workspace
  templates. Pure prompts/config — no facts. Loaded by `claude` at
  startup, identical in spirit to TerranSoul's `.github/copilot-instructions.md`
  + `rules/*.md`.
- **Memory daemon** (Python). One SQLite database, two daemon modes
  sharing it: a per-session **MCP daemon** that exposes ~33 tools to
  Claude Code, and a **standalone daemon** that runs 24/7 (LaunchAgent)
  to execute scheduled jobs even when `claude` is closed.

## Scheduled jobs (the "she gets sharper overnight" loop)

| Job | When | Purpose |
|---|---|---|
| Adaptive decay | 2 AM | Fades old memories; high-importance items decay at half rate |
| Consolidation | 3 AM | Merges duplicates; detects patterns; tracks relationship trends |
| Vault sync | 3:15 AM | One-way export of the SQLite store into the PARA-organised Obsidian vault |
| Pattern detection | every 6 h | Surfaces trends across conversations (over-commitment, cooling contacts, repeated mistakes) |

## Hybrid search ranking

Claudia uses a 4-signal hybrid: **50% vector + 25% importance + 10%
recency + 15% full-text**. Accessing a memory boosts it (the essay
calls this the *rehearsal effect*).

TerranSoul's equivalent (`memory/store.rs`): 6-signal — vector(40%) +
keyword(20%) + recency(15%) + importance(10%) + decay(10%) + tier(5%),
fused via RRF k=60, with optional HyDE and LLM-as-judge reranker.

## Slash-command surface (skills)

Reference list (so we don't accidentally re-propose any of these as
"new ideas"):

- `/morning-brief` — daily roll-up of overdue commitments, meetings, warnings
- `/new-workspace [name]` — scaffold a project workspace from templates
- `/meeting-prep [person]` — one-page briefing
- `/capture-meeting` — turn notes into decisions + action items
- `/what-am-i-missing` — surface risks, overdue items, cooling relationships
- `/research [topic]` — deep research with web sources + memory
- `/inbox-check` — lightweight email triage
- `/brain` — launch a 3-D brain visualiser (interactive node graph)
- `/meditate` — end-of-session reflection that extracts learnings,
  preferences, and judgment calls into persistent memory
- `/deep-context [topic]` — full-context deep analysis
- `/memory-audit` — list everything claudia knows with source chains
- `/weekly-review`, `/growth-check`, `/financial-snapshot`, `/draft-reply`,
  `/follow-up-draft`, `/new-person`, `/pipeline-review`, `/client-health`,
  `/databases`, `/brain-monitor`, `/fix-duplicates`, `/memory-health`,
  `/diagnose`

Plus ~30 **proactive** skills that fire automatically based on context
(commitment detection, pattern recognition, judgment awareness,
cognitive extraction, risk surfacing, etc.).

## Distinct claudia ideas worth adopting (with TerranSoul mapping)

Order is roughly "highest leverage / smallest implementation cost first."
Each item lists the equivalent TerranSoul module (existing or
proposed) so we never duplicate work.

1. **Judgment rules as a first-class persisted artefact.** Claudia keeps
   user-stated decision priorities ("revenue work beats internal
   cleanup") in `context/judgment.yaml` and consults them across briefs,
   triage, delegation, and risk surfacing. → TerranSoul has
   `rules/llm-decision-rules.md` for *project-level* policy but no
   per-user persisted rules store. **Adoption proposal:** add a
   `judgment_rules` table (or reuse `memories` with
   `category='judgment'` + `memory_type='preference'`) and a Pinia
   store + Tauri commands `judgment_add` / `judgment_list` /
   `judgment_apply` consumed by `commands/chat.rs` during system-prompt
   assembly.

2. **`/meditate` end-of-session reflection.** Auto-extracts learnings,
   patterns, and judgment calls before the session is dropped. →
   TerranSoul already has Chunks 30.2 (session memory absorption) and
   30.6 (self-improve session transcript auto-append). **Adoption
   proposal:** wrap them as a user-invocable `/meditate` slash command
   in the chat UI so users can trigger reflection on demand, not only
   at session boundary.

3. **`/morning-brief` proactive surface.** A periodic, opinionated
   roll-up the user *pulls* in one command. → TerranSoul has the
   skill-tree quest system (`src/stores/skill-tree.ts`, ~1500 lines)
   and `commands/quest.rs` for gamified surfacing, plus
   `commands/workflow_plans.rs`. **Adoption proposal:** add a daily
   brief Pinia action that queries
   `hybrid_search_rrf("overdue OR upcoming OR commitment OR
   reminder", since=now-1d)` + `temporal.rs` natural-language windows.

4. **`/memory-audit` with source chains.** Show every fact and trace
   each to its source email/transcript/conversation. → TerranSoul
   already stores `source_url`, `source_hash`, `parent_id`,
   `session_id`, and full versioning (`versioning.rs`). **Adoption
   proposal:** add a frontend memory-audit view in the brain panel
   that joins `memories ⨝ memory_versions ⨝ memory_edges` and renders a
   provenance tree per entry.

5. **3-D brain visualiser.** Claudia renders the entity graph as an
   interactive 3-D scene (`/brain`). → TerranSoul already runs Three.js
   for the VRM, has `graph_rag.rs` community summaries, and
   `code-intelligence` symbol graph clustering (Chunks 31.3-31.5).
   **Adoption proposal:** add a `BrainGraphViewport.vue` view (Three.js
   + `cytoscape` or `d3-force-3d`) that consumes
   `memory_edges` + `memories` and renders the persona's knowledge
   graph in the same canvas family as the VRM. Strictly a *view*; the
   data layer is unchanged. Keep neutral naming — no "claudia",
   "Claudia's brain", or branded labels.

6. **Two-tier agent team.** Claudia delegates to Tier-1 (Haiku)
   document-processing workers and Tier-2 (Sonnet) deep-research scout.
   → TerranSoul has `agents/roster.rs` + `agents/cli_worker.rs` +
   `coding/multi_agent.rs` + `coding/dag_runner.rs`. **Adoption
   proposal:** add capability tags ("fast-extract", "deep-research") to
   the agent roster and let `coding/coding_router.rs` route by tag.

7. **Per-project / per-workspace database isolation.** Each claudia
   workspace gets its own SQLite. → TerranSoul has dev/release data-root
   split (Chunk 20.1) but not per-workspace. **Adoption proposal:**
   surface a `data_root` setting per active workspace in `settings/`,
   with the existing namespacing helper.

8. **Background daemon health endpoint.** Claudia exposes
   `http://localhost:3848/status`. → TerranSoul's MCP server already
   exposes `/health` and `/status` on 7421/7422/7423. ✅ aligned, no
   change needed.

9. **Standalone-vs-stdio MCP split.** Claudia ships *both* a per-session
   stdio daemon and a 24/7 HTTP daemon. → TerranSoul's MCP is HTTP-only
   today. **Adoption proposal:** if/when we ship a `claude` /
   `codex` / `cursor` profile that prefers stdio MCP, add a thin stdio
   adapter that wraps the existing `BrainGateway` trait — no
   refactor needed because `gateway.rs` is transport-agnostic.

10. **PARA-organised export.** Claudia syncs to an Obsidian vault
    organised as Active / Reference / Areas / Archive folders. →
    TerranSoul's `obsidian_export.rs` exports flat. **Adoption
    proposal:** add an opt-in PARA template option to
    `obsidian_export.rs` (folder = derived from `tier` + `category`).

## Anti-patterns (do **not** copy from claudia)

- Don't copy any source file, scheduler script, or skill markdown — the
  PolyForm-NC license forbids redistribution and our project is
  effectively commercial-friendly.
- Don't adopt the "Obsidian vault is the user-facing primary surface"
  pattern. TerranSoul's primary surface is the chat + character +
  brain panel; Obsidian export remains opt-in.
- Don't bolt on a Python memory daemon. TerranSoul's MCP/brain is Rust;
  every new memory feature belongs in `src-tauri/src/memory/` behind
  the `StorageBackend` trait.
- Don't introduce branded names ("claudia", `/meditate` literal label,
  "morning brief" as a product noun). Pick neutral names per
  `rules/coding-standards.md` (e.g. `quest_daily_brief`,
  `session_reflect`).

## Status

This file is **research notes**, not a roadmap. Any adoption above
must be filed as a proper chunk in `rules/milestones.md` with the
usual "context → goal → files → tests" template before
implementation. The point of capturing it here is that the next agent
session can answer "do we already have a `/morning-brief`?" by
querying the brain instead of re-fetching the README.
