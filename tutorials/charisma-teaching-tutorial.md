# Charisma — Teaching TerranSoul Self, Persona & Animation

> **Full conduct & analysis (to May 2026) of TerranSoul's self-learning,
> persona, animation, and facial-expression systems, plus the Charisma
> teaching panel introduced in Chunk 30.4.**
>
> Maps to the Human-Brain ↔ AI-System ↔ RPG-Stat triple:
>
> | Human cognition | AI subsystem | RPG stat |
> |---|---|---|
> | Sense of Self / Mirror Neurons | Persona & Self-Learning Animation | 🎭 Charisma |
>
> When **Self-Improve** is enabled, proven user teachings are promoted
> from runtime data into source-code defaults, so every future install
> ships with what *this* user (and the rest of the community, via
> Persona Packs) has taught the companion.

---

## 1. The state of the union — May 2026

Before Chunk 30.4, TerranSoul already had the **infrastructure** to
learn from the user; it just had no aggregate view of "what's working,
what's mature enough to ship, what should I forget?". This section is
the audit.

### 1.1 Persona — the explicit identity

| Surface | Where | What it stores |
|---|---|---|
| `PersonaTraits` | [src/stores/persona.ts](../src/stores/persona.ts) + [src-tauri/src/commands/persona.rs](../src-tauri/src/commands/persona.rs) | name / role / bio / tone[] / quirks[] / avoid[] / exampleDialogue[] / active / updatedAt |
| Persona block injection | [src/utils/persona-prompt.ts](../src/utils/persona-prompt.ts) | Renders a `[PERSONA]` system-prompt fragment |
| Master-Echo loop | [src-tauri/src/persona/extract.rs](../src-tauri/src/persona/extract.rs) | LLM proposes traits from 30 chat turns + 20 `personal:*` memories |
| Drift detection | [src-tauri/src/persona/drift.rs](../src-tauri/src/persona/drift.rs) | Brain compares active persona vs. accumulated memories, emits a `DriftReport` |
| Pack export/import | [src-tauri/src/persona/pack.rs](../src-tauri/src/persona/pack.rs) | One-file JSON envelope (≤ 1 MiB) shareable across machines |

### 1.2 Mirror Neurons — facial expressions

The mirror-neuron analogue is the **face mirror**: real-time
ARKit-style blendshapes from the user's webcam are mapped to the VRM
character's expression manager.

| Surface | Where | What it does |
|---|---|---|
| Live mirror | [src/renderer/face-mirror.ts](../src/renderer/face-mirror.ts) | Pure mapper from 52 ARKit coefficients to 12+2 VRM channels (happy/sad/angry/relaxed/surprised/neutral + visemes + gaze + blink), with EMA smoothing |
| Capture | [src/components/PersonaTeacher.vue](../src/components/PersonaTeacher.vue) | Camera consent → MediaPipe FaceLandmarker → "Capture" button → snapshot weights → name + trigger → save |
| Storage | `LearnedExpression` in [src/stores/persona-types.ts](../src/stores/persona-types.ts) | id, name, trigger, weights record, optional lookAt + blink |
| Playback | [src/renderer/learned-motion-player.ts](../src/renderer/learned-motion-player.ts) | `applyLearnedExpression(vrm, expr)` writes weights directly to the expression manager |

### 1.3 Mirror Neurons — body motion

Same camera flow, different MediaPipe model: **PoseLandmarker** →
33 landmarks → 11 VRM humanoid bones expressed as Euler XYZ in
radians, recorded at 30 fps for ≤ 1 s clips.

| Surface | Where | What it does |
|---|---|---|
| Pose mapper | [src/renderer/pose-mirror.ts](../src/renderer/pose-mirror.ts) | 33 landmarks → 11 VRM bone Euler angles |
| Recording | `PersonaTeacher.vue` § motion flow | Streams `mirror-bones` to parent, accumulates frames, names + saves |
| Storage | `LearnedMotion` | frames[] with `{ t, bones }` map, fps, duration_s, optional `provenance: 'camera' \| 'generated'` |
| Bake | [src/renderer/vrma-baker.ts](../src/renderer/vrma-baker.ts) | Convert frame array → THREE.QuaternionKeyframeTracks → AnimationClip |
| Polish | [src-tauri/src/persona/motion_smooth.rs](../src-tauri/src/persona/motion_smooth.rs) | Non-destructive Gaussian smoothing (preview only) |

### 1.4 Self-Learning — the running loops

Three concurrent loops keep the companion's "self" current:

1. **Master-Echo** (every ~25 personal facts) proposes new persona
   traits and surfaces a review card in the persona panel.
2. **Drift detection** monitors trait alignment and offers
   "Echo noticed you've shifted toward …" cards.
3. **LLM-as-Animator** (Chunk 14.16, behind the camera consent flow)
   lets the brain *generate* motion clips as `provenance: 'generated'`
   so the user can teach without a webcam.

What was missing: a **measurement layer** that tracks how often each
taught artefact actually fires in real conversations and how the user
feels about it.

### 1.5 The Charisma stat

The `🎭 Charisma` skill in the quest tree
([src/stores/skill-tree.ts](../src/stores/skill-tree.ts)) lights up
when persona, expressions, and motions are all populated. Until 30.4
it was a binary "you have some" badge. Now it's a **maturity ladder**:
Untested → Learning → Proven → Canon.

---

## 2. The four maturity tiers

| Tier | Symbol | Rule | Meaning |
|---|---|---|---|
| **Untested** | ⏺ | Never used by the LLM since taught | Brand new — even the brain doesn't know about it yet |
| **Learning** | 📈 | At least 1 use, < 10 uses **OR** avg rating < 4.0 | The companion is using it, but we need more signal |
| **Proven** | ✨ | ≥ 10 uses **AND** avg rating ≥ 4.0 | Eligible for promotion to source-code defaults |
| **Canon** | 🏛️ | Promoted via the multi-agent workflow runner | Now part of the bundled defaults; future installs ship with it |

The thresholds are encoded once, in
[src-tauri/src/persona/charisma.rs](../src-tauri/src/persona/charisma.rs)
`CharismaStat::maturity()`, and mirrored in TypeScript via
`deriveMaturity()` in [src/stores/charisma.ts](../src/stores/charisma.ts)
so the UI never disagrees with the backend.

---

## 3. The Charisma management panel

**Open:** right-click the pet character → **Charisma — Teach me…**

The panel has three tabs (😊 Expressions, 💃 Motions, 📝 Traits) and a
top-of-panel summary dashboard with one cell per tier so the user can
see at a glance:

```
┌────────┬──────────┬────────┬───────┐
│   3    │    7     │   2    │   1   │
│Untested│ Learning │ Proven │ Canon │
└────────┴──────────┴────────┴───────┘
```

Each row shows:

- The asset's icon + display name
- "Used 14× · last 3h ago" usage meta
- A maturity badge coloured by tier
- A 5-star rating control (immediate save on click)
- Action buttons:
  - **▶ Test** — preview the expression / play the motion in-place
  - **⭐ Promote to source** — *only when Proven*; opens a coding
    workflow plan
  - **🏛️ Canon** — replaces Promote when the asset has been promoted
  - **Delete** — remove the stat row (the underlying asset stays)

Canon items are pinned to the bottom of the list so the user's eye
naturally falls on actionable Proven items.

---

## 4. How to teach — three worked examples

### 4.1 Teaching a facial expression ("Smug")

1. Right-click the pet → **Persona Teacher** (existing flow).
2. Grant camera consent (session-scoped — never persisted).
3. Make your smug face. Watch the live VRM mirror match it.
4. Click **Capture**. Name it "Smug" and pick a trigger word, e.g.
   `smug`.
5. The expression now lives in
   `<app_data>/persona/expressions/lex_…json` and shows up in the
   Persona panel.
6. **Open the Charisma panel.** "Smug" appears under
   *Expressions* with maturity **Untested**.
7. In chat, say something witty. When the brain detects pride / smug
   tone it emits the `smug` trigger; the runtime calls
   `charisma_record_usage` and the maturity flips to **Learning**.
8. After each use, click 4 or 5 stars on the row to record satisfaction.
9. After ≥ 10 uses with avg ≥ 4.0, the badge turns ✨ **Proven** and
   the **⭐ Promote to source** button appears.

### 4.2 Teaching a motion ("Bow")

Identical to above but using `PersonaTeacher.vue`'s **motion** flow:
record up to 1 s of pose data, name the clip, save. Charisma stats
accrue per clip the same way.

### 4.3 Teaching a persona trait

Persona trait teaching is *implicit* — you never click "save trait" for
charisma. Instead:

1. Edit the persona panel: add `"says 'indeed' a lot"` to **quirks**.
2. Each time the brain emits "indeed" the runtime calls
   `charisma_record_usage` with `kind: trait`, `asset_id: 'quirk_indeed'`.
3. Rate the trait through the Charisma panel like any other asset.
4. When Proven → Promote pushes the trait into the bundled persona
   default in source.

> **Tip — bulk-rate from chat.** Assistant turns that fired taught
> Charisma traits, expressions, or motions show a 1–5 star strip directly
> in chat. Clicking a star rates the *whole turn* and applies that same
> rating to every fired asset once.

---

## 5. Self-Improve integration — Promote → Workflow → Source

The promote button does **not** edit source files directly. Instead it
delegates to the **multi-agent workflow runner** built in Chunk 30.3.
The result is a regular YAML workflow plan in
`<data_dir>/workflow_plans/<id>.yaml` that the user (or, with
self-improve enabled, the autonomous loop) runs through the same
research-code-test-review pipeline as any other workflow.

```
                  ┌──────────────────────────┐
  Charisma row    │  charisma_promote() RPC  │
  ⭐ Promote ───▶│                          │
                  │  build_promotion_plan()  │
                  │  (4-step coding-kind     │
                  │   WorkflowPlan)          │
                  └──────────────┬───────────┘
                                 ▼
                  ┌──────────────────────────┐
                  │   Researcher (Llama 8B)   │
                  │   "Find the defaults file" │
                  └──────────────┬───────────┘
                                 ▼
                  ┌──────────────────────────┐
                  │   Coder (Claude Sonnet)   │
                  │   "<file path=…>…</file>" │  ← apply_file pipeline
                  │   *requires_approval*     │
                  └──────────────┬───────────┘
                                 ▼
                  ┌──────────────────────────┐
                  │   Tester (qwen-coder)     │
                  │   vitest + cargo test     │
                  └──────────────┬───────────┘
                                 ▼
                  ┌──────────────────────────┐
                  │   Reviewer (Claude Opus)  │
                  │   security + style audit  │
                  │   *requires_approval*     │
                  └──────────────────────────┘
```

The plan is **not auto-executed**. Two `requires_approval: true` gates
(the Coder step and the Reviewer step) ensure the user always sees
exactly what's about to be written into the repository. With
self-improve disabled, the user clicks Run on each step manually.
With self-improve **enabled**, the engine schedules the workflow into
the existing autonomous loop, but the same approval gates fire — they
just appear as actionable cards in the Self-Improve progress panel.

### 5.1 What ends up in source

Roughly, three target locations depending on `kind`:

| Kind | Likely target file (Researcher's job to confirm) |
|---|---|
| Expression | `src/renderer/face-mirror.ts` (DEFAULT_LEARNED_EXPRESSIONS table) **or** `public/animations/learned-expressions.json` (bundled asset) |
| Motion | `public/animations/learned/<trigger>.vrma` (bake from frames) **or** `src/renderer/vrma-manager.ts` `VRMA_ANIMATIONS` registry |
| Trait | `src-tauri/src/commands/persona.rs` `default_persona_json()` **or** a new `bundled_traits.json` reference |

The Researcher always reads the existing file first so the Coder
appends rather than overwrites.

### 5.2 Safety — why this is OK to ship

- **`apply_file` validates paths**: no `..`, no `.git/`, no symlinks,
  no absolute paths. See
  [src-tauri/src/coding/apply_file.rs](../src-tauri/src/coding/apply_file.rs).
- **Atomic writes**: temp file + rename — a crash mid-write never
  produces a torn file.
- **Two human approval gates**: Coder before write, Reviewer before
  acceptance.
- **Git stages** the change; nothing is pushed.
- **Tester runs the targeted CI slice** before Reviewer can approve.
- **Revertable**: the standard `git restore` works; the workflow plan
  records its own id in the asset's `last_promotion_plan_id`.

---

## 6. Worked end-to-end example — "indeed"

> A chat user says *"You should pepper 'indeed' into your responses."*
> By the end of this example, the word "indeed" is shipping with every
> future copy of TerranSoul.

1. **Day 0** — User edits the Persona panel, adds `says 'indeed'` to
   **quirks**. Persona schema now has the quirk; no source change.
2. **Day 0–14** — Each time the brain emits "indeed", the streaming
   path calls
   ```ts
   charismaStore.recordUsage('trait', 'quirk_indeed', "says 'indeed'");
   ```
   The Charisma panel shows the count tick up. After 5 emissions the
   user awards 5★ once. Maturity = **Learning**.
3. **Day 14** — usage_count crosses 10, average rating is 4.5.
   Maturity flips to **Proven**, the **⭐ Promote** button lights up.
4. **Day 14** — User clicks Promote. A workflow plan
   `wfp_a3f…` is created with title
   *"Promote persona trait 'says 'indeed'' to source defaults"*.
   The Charisma row's badge flips to **Canon** immediately (the plan
   is created — even if the user never runs it, the asset is
   considered locally promoted; re-running Promote is a no-op).
5. **Day 14** — User opens the **Multi-Agent Workflows** panel. The
   plan appears under Coding workflows. They click Run on the
   Researcher step. Llama 8B locates
   `src-tauri/src/commands/persona.rs` and reports the existing
   `default_persona_json()` constant.
6. **Day 14** — The Coder step is `requires_approval`. User reviews
   the proposed `<file path="...">` block, sees the new quirk being
   appended to the default JSON, clicks **Approve & Apply**.
7. **Day 14** — The Tester step runs `cargo test --lib persona` and
   `npx vitest run src/stores/persona`. All green.
8. **Day 14** — The Reviewer step (Claude Opus) reports: *"No PII in
   the new default. Style consistent with existing entries. No schema
   change. Approve."* User clicks **Approve & Merge**.
9. **Day 14** — `git status` shows the modified file already staged.
   User runs their normal commit + push.
10. **Day 15** — A friend installs TerranSoul fresh. Their default
    persona ships with `says 'indeed'` already in quirks. The
    companion has *learned*, and the learning has been *encoded into
    the artefact itself*.

This is the closing of the loop the user asked for: **runtime
learning → measurement → promotion → bundled default → next install**.

---

## 7. Removing / forgetting things

The Charisma panel has a **Delete** button per row that removes only
the stats — the underlying `LearnedExpression` / `LearnedMotion` /
trait quirk stays. To delete the underlying asset, use the Persona
panel's per-asset delete button (existing flow).

If you want to **demote** a Canon item (already promoted but you
changed your mind), the supported path is:

1. Open the Multi-Agent Workflows panel.
2. Find the promotion plan (its id is shown on the Canon row).
3. Run a *reverse* coding-kind workflow: Researcher locates the
   committed change, Coder generates the inverse `<file>` block,
   Tester + Reviewer same as forward.
4. Delete the Charisma stat row.

We deliberately do not provide an "undo Canon" button — promoting to
source code is intentionally weighty so the user feels the gravity of
shipping something to every future install.

---

## 8. Where Charisma lives in the codebase

| File | Role |
|---|---|
| [src-tauri/src/persona/charisma.rs](../src-tauri/src/persona/charisma.rs) | Data model + maturity rules + atomic JSON persistence + `build_promotion_plan()` |
| [src-tauri/src/commands/charisma.rs](../src-tauri/src/commands/charisma.rs) | 7 Tauri commands (list / record_usage / set_rating / rate_turn / delete / promote / summary) + inner helpers for tests |
| [src/stores/charisma.ts](../src/stores/charisma.ts) | Pinia store, helpers (`deriveMaturity`, `kindIcon`, etc.), one-to-one TS mirrors of Rust types, and turn-level usage/rating actions |
| [src/stores/conversation.ts](../src/stores/conversation.ts) | Annotates assistant turns with fired Charisma assets and distributes chat-level ratings |
| [src/components/ChatMessageList.vue](../src/components/ChatMessageList.vue) | Shows 1–5 turn rating controls on assistant messages that fired Charisma assets |
| [src/components/CharismaPanel.vue](../src/components/CharismaPanel.vue) | The management UI — 3-tab list, summary dashboard, rating stars, Promote button |
| [src/components/PetContextMenu.vue](../src/components/PetContextMenu.vue) | Adds the "Charisma — Teach me…" entry |
| [src/views/PetOverlayView.vue](../src/views/PetOverlayView.vue) | Mounts the modal panel |

Persisted state: `<app_data_dir>/persona/charisma_stats.json` (single
file, atomic write, schema-versioned).

---

## 9. Design rationale — why this shape?

- **Why a separate stats file?** Because we want to delete a learned
  expression *without* losing the lesson "this expression was
  promoted". `charisma_stats.json` survives independent of
  `expressions/lex_*.json`.
- **Why piggy-back on multi-agent workflows for promotion?** Three
  reasons. (a) The DAG runner already has the apply_file +
  approval-gate plumbing we need. (b) It keeps the Charisma panel
  focused — promotion is an event, not a feature surface. (c) It
  composes: a future "promote *all* Proven items at once" button is
  just a workflow plan that fans out the four steps per asset.
- **Why 10 uses + 4.0 rating?** Round numbers; conservative enough that
  a single hostile rating can't tank a real teaching, generous enough
  that an evening's chat can produce one or two Proven assets. The
  thresholds are constants, not configurable, so the meaning of
  "Proven" is the same on every install.

---

## 10. See also

- [docs/persona-design.md](../docs/persona-design.md) — full persona system
  design (§ 2 traits, § 5 camera consent, § 8 learned artefacts).
- [docs/persona-pack-schema.md](../docs/persona-pack-schema.md) — pack
  envelope format for sharing taught artefacts across machines.
- [tutorials/multi-agent-workflows-tutorial.md](multi-agent-workflows-tutorial.md)
  — the workflow runner that promotion plans use.
- [docs/coding-workflow-design.md](../docs/coding-workflow-design.md) §3.10 —
  Charisma promotion as a workflow class.
- `rules/architecture-rules.md` — the brain documentation sync rule
  this chunk respects (Charisma does not touch RAG / cognitive-kind
  surfaces).
