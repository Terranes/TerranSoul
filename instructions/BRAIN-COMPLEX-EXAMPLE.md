# Brain + Knowledge Quest + RAG — Walkthrough

> **TerranSoul v0.1** · Last updated: 2026-04-24
>
> Technical reference: [`BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md`](BRAIN-COMPLEX-EXAMPLE-EXPLAIN.md) ·
> Brain component map: [`docs/brain-advanced-design.md`](../docs/brain-advanced-design.md)

The "Alice learns Vietnamese law" demo — a single explicit instruction
walks through dependency installation, document ingestion, and
RAG-augmented chat, all driven by the existing skill-tree quest engine.

---

## Why this flow

Earlier versions either auto-fired Scholar's Quest on a plain question,
or asked the user to type a literal command after a *don't-know* answer.
The current behavior is:

- **Asking a law question ≠ teaching the AI.** The LLM answers directly.
- **One explicit instruction starts everything.** Alice types
  *"Learn Vietnamese laws using my provided documents"* and TerranSoul
  walks the entire dependency chain for her.
- **No installer is reimplemented from scratch.** The auto-install path
  simply auto-triggers and accepts each missing quest in the Scholar's
  Quest prerequisite chain (`free-brain → memory → rag-knowledge →
  scholar-quest`) using the existing skill-tree engine.

---

## Step 1 — Alice asks for a doc-grounded study session

Alice types into chat:

> **Learn Vietnamese laws using my provided documents**

`detectLearnWithDocsIntent()` matches the phrase, extracts the topic
(*"Vietnamese laws"*), and short-circuits the LLM call.

---

## Step 2 — TerranSoul lists missing components

TerranSoul walks the Scholar's Quest prerequisite chain via
`getMissingPrereqQuests()`, lists every quest in that chain that isn't
already `active`, and pushes a System message with **three inline
buttons**:

| Button | Action |
|---|---|
| ⚡ **Install all** | Sub-prompt for Auto vs Manual install |
| 📋 **Install one by one** | Per-quest button list (manual) |
| ❌ **Cancel** | Dismiss the prompt |

> Each button shows its full label — they're inline tiles that wrap onto
> a new row only when they don't all fit on one line.

If every prerequisite quest is already active the missing-components
prompt is skipped and Step 4 happens immediately.

---

## Step 3 — Install all → Auto install

Picking **Install all** opens a second three-button prompt:

| Button | Action |
|---|---|
| ⚡ **Auto install** | Trigger and accept every missing quest without further prompts |
| 🛠️ **Manual install** | Same per-quest button list as *one by one* |
| ↩️ **Back** | Re-show the original three choices |

**Auto install** calls, for each missing quest in dependency order:

```ts
skillTree.triggerQuestEvent(questId);
await skillTree.handleQuestChoice(questId, 'accept');
```

No new installer code, no provider-specific logic — the skill-tree
engine is the single source of truth. If a quest can't be auto-accepted
(because it requires user input that the engine routes elsewhere),
TerranSoul keeps going through the rest of the list and surfaces a
**Open Quests tab** follow-up button for whatever's still inactive.

---

## Step 4 — Pick the documents to import

When every prerequisite quest is active, TerranSoul pushes the existing
**📚 Scholar's Quest** invitation:

| Button | Action |
|---|---|
| ⚔️ **Start Knowledge Quest** | Opens `KnowledgeQuestDialog` at the *Gather Sources* step |
| 💤 **No thanks** | Dismiss |

Clicking **Start Knowledge Quest** opens the existing dialog directly at
**Step 2 — Gather Sources**. Alice adds a URL and a file:

- `localhost:1420/demo/vietnamese-civil-code.html` (Articles 351–468)
- `article-429-commentary.txt`

Then clicks **⚡ Start Learning**. The ingestion pipeline runs: URL fetch
→ HTML extraction → chunking (800/100) → SHA256 dedup → Ollama
embedding → SQLite storage.

---

## Step 5 — RAG-augmented answers

After ingestion completes Alice can ask any question grounded in the
sources she just imported, e.g.:

> *What is the statute of limitations for contract disputes under
> Vietnamese law?*

The chat answer is now drawn from the ingested civil-code chunks via
hybrid search.

---

## Implementation map

| Concern | File |
|---|---|
| Intent detection | `src/stores/conversation.ts` — `detectLearnWithDocsIntent()` |
| Missing quests | `src/stores/conversation.ts` — `getMissingPrereqQuests()` |
| Choice routing | `src/stores/conversation.ts` — `handleLearnDocsChoice()` |
| Skill-tree engine | `src/stores/skill-tree.ts` — `triggerQuestEvent()` / `handleQuestChoice()` |
| Document import UI | `src/components/KnowledgeQuestDialog.vue` |
| Inline buttons | `src/components/QuestChoiceOverlay.vue` |
| Brain component map | `docs/brain-advanced-design.md` |

The brain components used by this flow (Free/Local LLM, Long-Term
Memory, RAG, Scholar's Quest, embedding model, ingestion engine) and
their dependency order are documented in
[`docs/brain-advanced-design.md`](../docs/brain-advanced-design.md) —
consult that doc first for any new feature that touches the brain.
