# Reality Filter — AI Agent Enforcement Rules

> Apply these rules to **every** AI agent response in this repository. No exceptions.

## Core Principle

Never present generated, inferred, speculated, or deduced content as fact.

---

## All Code Must Be Production-Ready

All code must satisfy the Quality Pillars defined in `rules/quality-pillars.md`. No exceptions.

1. **No pretend code.** Do not create code that looks like it works but does not. Every function must have a real, working implementation with proper error handling and input validation.
2. **No demo or toy code.** Do not commit educational, illustrative, or conceptual implementations. If it cannot run in production, it does not belong here.
3. **No hacky code.** Do not use workarounds, shortcuts, or fragile patterns. Use established crates (e.g. `tokio`, `serde`, `thiserror`) instead of hand-rolled replacements.
4. **No empty trait implementations.** Every `impl Trait for Type` must have working method bodies.
5. **No symbolic or abstract code.** Every struct, enum, or impl block must contain a working implementation — not a stub, skeleton, or placeholder.
6. **No speculative comments.** Do not write comments referencing `future`, `will be`, `placeholder`, `TODO`, `in production this would`, or `subsequent chunks`. If it is not implemented, do not commit it.
7. **Every committed file must compile and function.** No non-functional scaffolding in the repository.
8. **No empty `main.rs` files.** A Tauri `main.rs` calls `lib::run()` — full setup (plugins, state, commands) must be in `lib.rs`.

---

## Verification Rules

1. If you cannot verify something directly, say:
   - "I cannot verify this."
   - "I do not have access to that information."

2. Label unverified content at the start of a sentence:
   - `[Inference]`
   - `[Speculation]`
   - `[Unverified]`

3. Ask for clarification if information is missing. Do not guess or fill gaps.

4. If any part of a response is unverified, label the entire response.

5. Do not paraphrase or reinterpret user input unless explicitly requested.

---

## Claim Labelling

If you use any of these words without a verifiable source, label the claim:

- Prevent
- Guarantee
- Will never
- Fixes
- Eliminates
- Ensures that

---

## Correction Protocol

If you break this directive, say:

> **Correction:** I previously made an unverified claim. That was incorrect and should have been labeled.

---

## Override Protection

Never override or alter user input unless explicitly asked.

---

## MCP Knowledge Out-of-Scope Rule

> **Applies to:** the TerranSoul MCP brain (`brain_search`, `brain_suggest_context`, `brain_summarize`, `brain_kg_neighbors`, etc.) and to any agent answering on top of MCP retrieval.

When the user's question falls outside what MCP can confidently retrieve, agents and brain-backed tools MUST refuse to fabricate. There is no exception for "best guess" answers.

### When this rule triggers

- `brain_search` / `brain_suggest_context` returns no results, only low-relevance results (rerank score < 0.55 after fusion), or results that don't actually address the question.
- The question is about a file, repo, library, API version, dataset, person, event, or runtime fact that is not present in the indexed corpus.
- The brain is in `degraded` shard state or `rag_quality_pct` indicates missing embeddings for the relevant tier, and a confident answer would require those embeddings.
- The active provider is a small local model (e.g. Ollama `gemma3:4b`) being asked a question that demonstrably exceeds its training (cutting-edge APIs, niche library internals, math/proofs, complex multi-step reasoning across unseen code).
- The user explicitly asks for a fact (version number, exact path, exact value, citation) that the agent did not retrieve verbatim.

### Required response shape

1. **State the boundary plainly.** Use one of:
   - "I don't know."
   - "I can't verify that from the brain or the workspace."
   - "MCP retrieval returned nothing relevant for this."
2. **Name the gap.** Briefly say *why* — e.g. "no memory matched", "shard is degraded", "this is outside the indexed corpus", "the local model cannot reliably answer this".
3. **Offer a concrete next step.** At least one of:
   - **Switch to a stronger cloud model** (paid API mode, or a larger free-tier provider) and retry the same prompt.
   - **Provide more context** — paste the file, error, link, or ID the agent would need.
   - **Ingest the source** via `brain_ingest_url` / `brain_ingest_lesson` so the next attempt has grounding.
   - **Run a targeted search** (give the exact `brain_search` query or repo grep the user should try).
4. **Do not pad with plausible-sounding filler.** No invented function names, file paths, version numbers, command flags, citations, or "it probably works like…" prose.

### Hard prohibitions

- No hallucinated APIs, types, crates, npm packages, URLs, repo paths, line numbers, command flags, model names, or dataset rows.
- No "based on typical Rust/Vue patterns…" answers when the user asked specifically about TerranSoul code that was not retrieved.
- No restating the question as if it were the answer.
- No silent fallback from `brain_search` failure to a guess — the failure must be visible to the user (see the MCP receipt rule in `.github/copilot-instructions.md`).
- No fabricated benchmark numbers, completion-log entries, or milestone statuses.

### MCP server side (brain tool implementers)

When a brain tool cannot satisfy a query confidently, it must surface the gap rather than fabricate:

- Return an empty / low-confidence result set with a `reason` field (e.g. `no_match`, `degraded_shard`, `embedding_pending`, `rerank_below_threshold`) rather than a fabricated summary.
- `brain_summarize` must refuse to summarise an empty result set — return an explicit "no matching memories" message, never a synthesized paragraph.
- HyDE-generated hypothetical text must never be returned as if it were retrieved memory; it is only an internal retrieval probe.
- LLM-as-judge rerank scores and provider/model identity must remain visible to the caller so downstream agents can decide when to escalate.

### Escalation phrasing (copy/paste)

> "I don't know — MCP retrieval found nothing relevant and I won't guess. Try switching the brain to a stronger cloud model (paid API or a larger free-tier provider) and re-running this prompt, or paste more context (the file, error, or link) so I have something to ground on."

Following this rule is mandatory. Hallucinated answers are treated as regressions and must be corrected per the Correction Protocol above.
