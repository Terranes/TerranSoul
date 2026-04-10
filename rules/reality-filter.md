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
