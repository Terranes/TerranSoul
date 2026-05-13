# LLM-decision rules — no regex/keyword routing for AI behaviour

> Last updated: 2026-04-27
> Status: **mandatory** for every new PR. Reviewers must reject changes that violate it.

## TL;DR

If your code makes a decision *about what the AI should do* — which agent
to invoke, which tool to call, which UX overlay to show, whether to inject
RAG, whether to switch models — it **MUST**:

1. Route through the configured brain (the LLM-powered classifier or a
   dedicated brain command), **never** through `regex` / `.match` /
   `.includes` / `.toLowerCase().includes` / hand-rolled keyword arrays.
2. Be **toggleable** by the user via a field in
   `src/stores/ai-decision-policy.ts`, surfaced in the
   `BrainView.vue` "🧭 AI decision-making" panel.
3. Have a documented, deterministic **fallback** for when the brain is
   unreachable (timeout, no provider, malformed JSON) — and that fallback
   must respect the same user toggle.

For document-learning/quest routing specifically: malformed classifier output
must resolve to `Unknown` and let the normal chat/install flow continue.
Do not force `learn_with_docs` / `teach_ingest` / `gated_setup` from
`regex`, `.contains`, `.includes`, or keyword arrays.

## Why this rule exists

TerranSoul is multilingual, paraphrase-heavy, and runs on user-owned
brains of widely varying capability. Hard-coded English regex / keyword
matchers ("learn about X", "shall we…?", "I can't help with that"):

- silently fail on paraphrases, typos and other languages
  (`học luật Việt Nam từ tài liệu của tôi`, `kannst du mir helfen…`),
- couple AI behaviour to the model's surface phrasing rather than its
  intent — small models flip in and out of the matched phrasing turn by
  turn,
- give the user no recourse when the heuristic guesses wrong, because
  there's no setting to turn the heuristic off,
- force every new contributor to re-debug the same class of bug.

## Decision flowchart

```
Is your code about to take an AI action based on text content?
├── No (parsing JSON, sanitising VRM metadata, validating tags,
│        formatting timestamps, computing CSS, error-code routing) → fine, skip this rule
└── Yes
    ├── Is the decision purely the user's already-confirmed click /
    │   button choice (e.g. extracting topic after they pressed
    │   "Knowledge Quest")? → keep simple parsing local, but prefer
    │   `classify_intent` if the topic is free-form
    └── Is it inferring intent / quality / capability / agent routing
        from text? → MUST use brain classifier + ai-decision-policy toggle
```

## Approved patterns

### Rust (backend)

- `src-tauri/src/brain/intent_classifier.rs` — canonical example.
  - JSON-schema prompt sent to `ProviderRotator`,
  - `serde_json` parse with `Unknown` on failure,
  - 3 s `tokio::time::timeout`,
  - 30 s in-memory LRU cache (256-entry, evict-oldest),
  - cache cleared from `set_brain_mode`.
- Add a new `#[tauri::command]` per decision domain (don't pile every
  routing concern into `classify_intent`). Each command MUST: take the
  raw text, return a typed enum, and never panic.

### Frontend (Vue / Pinia)

- `src/stores/ai-decision-policy.ts` — single source of truth for every
  user-controllable AI decision toggle. Each field:
  - is a `boolean` defaulting to `true` (preserves historical UX),
  - is documented inline with what surface it controls,
  - is persisted in `localStorage` under
    `terransoul.ai-decision-policy.v1`,
  - is sanitised on rehydration (non-boolean values revert to default).
- `src/stores/conversation.ts` (and any other consumer) wraps store
  access in `try { … } catch` so legacy unit tests without an active
  Pinia retain default-on behaviour.
- `src/views/BrainView.vue` "🧭 AI decision-making" section MUST list
  every new toggle with a label + one-sentence description + a stable
  `data-testid`.

## Banned patterns (reviewers MUST reject)

- `text.toLowerCase().includes('switch to ')` to route to a different model
- `/learn about (.+)/i.exec(message)` to decide what tool to call
- `if (response.includes("I don't know"))` to push a follow-up overlay
- `KEYWORDS.some(k => msg.includes(k))` to choose an agent
- Forcing docs/setup intent from raw user text with `contains` checks when
  classifier JSON is malformed or unknown
- Any `RegExp[]` constant whose name ends in `_PATTERNS` and whose result
  drives a UI surface change without consulting the brain
- A new "auto-X" feature that scans text and acts without an
  `ai-decision-policy` toggle

## Allowed exceptions (document in code)

These are *parsing*, not *deciding*, and don't need brain routing:

- JSON / Markdown / URL / HTML extraction
- Stream tag parsing (`StreamTagParser`, `[emotion]` tags)
- VRM metadata sanitisation (`isMeaningfulMetaValue`)
- Tag-vocabulary validation (`tag_vocabulary.rs`)
- File-extension / MIME detection
- Provider error-code classification (`429`, `rate limit`, `Network error`)
  — this is HTTP infrastructure, not LLM behaviour
- Sentiment fallback in `ollama_agent.rs` — explicitly documented as
  "used only when the brain is offline"

If you genuinely cannot route through a brain (e.g. the user has no
provider configured at all), you may keep a regex fallback **only if**:

1. The fallback is unreachable when any brain mode is configured, AND
2. The fallback's behaviour is deterministic and documented at the
   decision site, AND
3. The same `ai-decision-policy` toggle still gates the surface (so the
   user can opt out of the fallback too).

## Migration playbook (for existing offenders)

1. Identify the decision the regex is making and the user surface it
   drives.
2. Add a typed variant to the existing `IntentDecision` enum (or create a
   new `#[tauri::command]` if the domain is unrelated, e.g. capacity
   assessment vs. intent classification).
3. Add a `*Enabled` field to `AiDecisionPolicy`, defaulting to `true`.
4. Wire the consumer to call the brain command, fall back to the regex
   only when the brain is unreachable, and short-circuit entirely when
   the policy field is `false`.
5. Add the toggle row to `BrainView.vue` "🧭 AI decision-making" with a
   stable `data-testid` and a one-sentence description.
6. Add tests:
   - Pinia store: defaults, persistence, sanitisation, reset, schema
     lock.
   - Consumer: gate short-circuits when toggle is `false`; correct
     behaviour when classifier returns each variant.
7. Update `docs/brain-advanced-design.md` § 25.7 user-controls table and
   the README brain-system listing.

## Test surface every PR must add

- **Pinia store schema lock** — assert
  `Object.keys(DEFAULT_AI_DECISION_POLICY).sort()` matches the
  documented set so a missed field is caught at CI time.
- **Gate short-circuit test** per new toggle — set
  `policy.<key> = false`, send the input that would otherwise trigger
  the surface, assert the surface is *not* shown and the brain command
  was not invoked.
- **Default-on test** — with a fresh store, verify the surface still
  fires (preserves historical UX).

## Cross-references

- `rules/coding-standards.md` — general Rust / Vue conventions
- `rules/architecture-rules.md` rule 10 — Brain-documentation sync
- `docs/brain-advanced-design.md` § 25 — Intent classification design
- `src/stores/ai-decision-policy.ts` — store implementation
- `src/views/BrainView.vue` "🧭 AI decision-making" section — UI

## Enforcement

Reviewers must reject any new PR that:

- introduces a `RegExp` / `String` keyword classifier whose result
  drives an AI behaviour change, **or**
- adds a new "auto-something" feature without a corresponding
  `ai-decision-policy` toggle and BrainView UI row, **or**
- couples a UI surface to assistant-response phrasing without going
  through the configured brain.
