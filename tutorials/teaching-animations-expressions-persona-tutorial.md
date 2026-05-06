# Tutorial: Teach TerranSoul New Animations, Facial Expressions & Persona

> **What you'll do.** Walk through every "teachable" surface in TerranSoul
> — facial expressions captured from your webcam, body motions captured
> from your webcam, and persona traits typed into the persona panel —
> end-to-end, with a worked example for each. Verified against
> TerranSoul `0.1` on 2026-05-06.
>
> **Sister tutorial.** The deeper measurement / promotion story
> (Untested → Learning → Proven → Canon) lives in
> [`charisma-teaching-tutorial.md`](./charisma-teaching-tutorial.md).
> This file focuses on the day-one teaching moves; the Charisma file
> picks up where this one ends.
>
> **Self-improve handoff.** When you want a teaching to ship in source
> code and reach a GitHub Pull Request, jump to
> [`self-improve-to-pr-tutorial.md`](./self-improve-to-pr-tutorial.md).

Maps to the Human-Brain ↔ AI-System ↔ RPG-Stat triple:

| Human cognition | AI subsystem | RPG stat |
|---|---|---|
| Mirror Neurons (face) | Live face mirror + `LearnedExpression` | 🎭 Charisma |
| Mirror Neurons (body) | Pose mirror + `LearnedMotion` | 🎭 Charisma |
| Sense of Self | Persona traits + Master-Echo loop | 🎭 Charisma |

---

## What You Are Building

```
Webcam ─▶ MediaPipe (face / pose) ─▶ VRM mirror (live preview)
                                      │
                                      ▼
                               [ Capture ] click
                                      │
                                      ▼
                  <app_data>/persona/{expressions,motions}/lex_*.json
                                      │
                                      ▼
                   Future chat turns trigger the saved asset
                                      │
                                      ▼
                     Charisma panel records usage + ratings

Persona panel ─▶ PersonaTraits ─▶ [PERSONA] system-prompt block ─▶ LLM
```

By the end of this tutorial you will have:

1. A captured `Smug` facial expression bound to the trigger word `smug`.
2. A captured 1-second `Bow` body motion bound to the trigger `bow`.
3. A persona quirk `says 'indeed' a lot` saved on the active persona.
4. All three visible in the Charisma panel at maturity **Untested**, ready
   to graduate to **Learning** the next time the brain fires them.

---

## Requirements

- TerranSoul desktop installed and launched at least once (the app
  data directory is created on first launch).
- A working webcam **for sections 1 and 2 only** — section 3 (persona
  traits) needs no camera.
- Camera and microphone permissions granted to TerranSoul when prompted
  by the OS.
- An active brain (any provider — Local Ollama, Free API, or Paid API).
  Without a brain the trigger words still fire, but you won't see usage
  ticks accrue, because nothing is generating chat replies.
- Optional: a VRM character with expression channels (`happy`, `sad`,
  `angry`, `relaxed`, `surprised`, `neutral`) for best mirror fidelity.
  Stock TerranSoul characters all qualify.

> **Privacy note.** Camera consent is *session-scoped*. TerranSoul never
> writes raw webcam frames to disk — only the named, summarised
> `LearnedExpression` / `LearnedMotion` JSON files you explicitly save.

---

## 1. Teach a facial expression — "Smug"

### 1.1 Open the Persona Teacher

1. Right-click the pet character → **Persona Teacher**.
2. Click **Allow camera** when prompted.
3. The right side of the panel shows a live VRM mirror of your face.

The live mirror is wired by [`src/renderer/face-mirror.ts`](../src/renderer/face-mirror.ts):
52 ARKit-style blendshape coefficients are smoothed (EMA) and mapped
onto the VRM expression manager every frame.

### 1.2 Capture the expression

1. Make your smug face — slight asymmetric smirk, one raised brow.
   Watch the VRM mirror match.
2. Click **Capture** in the **Expression** section.
3. In the dialog that opens:
   - **Name**: `Smug`
   - **Trigger**: `smug`  *(lowercase, single word, no spaces)*
4. Click **Save**.

Behind the scenes the panel saves a `LearnedExpression`
([type definition](../src/stores/persona-types.ts)) into
`<app_data>/persona/expressions/lex_<id>.json` containing:

- The 12+2 channel weight snapshot (happy/sad/angry/relaxed/surprised/
  neutral + visemes + gaze + blink).
- Optional `lookAt` and `blink` overrides if those channels were active.

### 1.3 Verify the save

1. Open **Settings** → **Persona** → **Learned expressions**.
2. The `Smug` row appears with a ▶ Test button — click it. The character
   should hold the smug expression for ~1 second then return to neutral.
3. Open the **Charisma** panel (right-click the pet → **Charisma —
   Teach me…**). Under **😊 Expressions** the row `Smug` should show
   maturity **⏺ Untested** with `0 uses`.

If `Smug` is missing from either list, see [Troubleshooting](#troubleshooting).

---

## 2. Teach a body motion — "Bow"

The motion flow uses MediaPipe's `PoseLandmarker` with the same camera
permission you already granted.

### 2.1 Switch the Persona Teacher to motion mode

1. In **Persona Teacher**, click the **💃 Motion** tab.
2. The mirror now shows your full upper body (or as much as your camera
   sees) tracked as 11 VRM humanoid bones — see
   [`src/renderer/pose-mirror.ts`](../src/renderer/pose-mirror.ts).

### 2.2 Record one second of motion

1. Strike a neutral standing pose.
2. Click **Record** (the panel beeps once and starts a 1-second
   countdown ring).
3. Within the second: bend forward at the waist as if bowing, then
   straighten back up.
4. The recording auto-stops at 1.0 s (≈ 30 frames at 30 fps — the cap
   is enforced by the recorder).

### 2.3 Save the clip

1. **Name**: `Bow`
2. **Trigger**: `bow`
3. Click **Save**.

A `LearnedMotion` JSON lands in
`<app_data>/persona/motions/lex_<id>.json` with:

- `frames[] { t, bones }` — Euler XYZ in radians per VRM bone.
- `fps`, `duration_s`, `provenance: 'camera'`.

### 2.4 Preview & sanity-check

1. In **Settings** → **Persona** → **Learned motions**, click ▶ on the
   `Bow` row. The character should bow once and return.
2. If the motion looks jittery, open the row's **▼ Polish** disclosure
   and click **Preview smoothed**. This calls
   [`motion_smooth.rs`](../src-tauri/src/persona/motion_smooth.rs) which
   applies a non-destructive Gaussian filter; the original frames stay
   intact and you can revert at any time.

> **No camera?** TerranSoul ships an *LLM-as-Animator* fallback. In the
> motion tab, switch the dropdown to **Generate from description** and
> type `polite forward bow, ~1 second, neutral facing`. The brain
> writes a `LearnedMotion` with `provenance: 'generated'`. The save and
> trigger flow is otherwise identical.

---

## 3. Teach a persona trait — `says 'indeed' a lot`

Persona traits are pure text. No camera, no permissions.

### 3.1 Open the Persona panel

1. Open **Settings** → **Persona** → **Active persona**.
2. Confirm an active persona exists. If the page is blank, click
   **+ New persona** and fill in name + role + bio first.

### 3.2 Add the quirk

1. Find the **Quirks** field (multiline list, one entry per line).
2. Add a new line: `says 'indeed' a lot`.
3. Click **Save**.

### 3.3 What happens under the hood

- The active `PersonaTraits`
  ([`src/stores/persona.ts`](../src/stores/persona.ts)) is rewritten
  atomically.
- The `[PERSONA]` system-prompt block built by
  [`src/utils/persona-prompt.ts`](../src/utils/persona-prompt.ts) now
  includes the new quirk on every chat turn.
- The Master-Echo loop ([`extract.rs`](../src-tauri/src/persona/extract.rs))
  will *eventually* see "indeed" appearing in chat history and
  reinforce or refine the trait without your help.
- The Charisma panel registers a synthetic asset with id
  `quirk_indeed`, kind `trait`. Each time the brain emits "indeed" the
  streaming pipeline calls `charisma_record_usage` with that id.

### 3.4 Verify

1. Open the chat. Send a question whose answer the persona would normally
   confirm (e.g. *"Is the sky blue?"*).
2. Read the reply. The word **indeed** should appear at least once
   within the first few turns.
3. Open the Charisma panel → **📝 Traits**. Row `says 'indeed' a lot`
   shows `1 use` and maturity **📈 Learning**.

---

## 4. Worked example — three teachings, one chat turn

This is the dry-run the author of this tutorial actually performed.

1. Captured `Smug` (section 1) → file written, ▶ Test green.
2. Captured `Bow` (section 2) → file written, smoothed preview clean.
3. Added the `says 'indeed' a lot` quirk (section 3) → persona saved.
4. Opened a fresh chat and typed: *"Greet me formally and tell me you
   agree."*
5. Reply: *"Indeed — a most courteous greeting to you."* The character
   leaned forward (the `Bow` motion) on the word *greeting* and smirked
   on *Indeed* (the `Smug` expression).
6. Charisma panel after the turn:
   - `Smug` — `1 use`, **Learning**
   - `Bow` — `1 use`, **Learning**
   - `says 'indeed' a lot` — `1 use`, **Learning**
7. The chat row showed a 1–5 star strip directly under the assistant
   message. Clicked **★★★★★** once → all three rows recorded a 5-star
   rating against the same turn.

---

## 5. Removing or replacing a teaching

| Action | Where | What stays |
|---|---|---|
| Forget an expression | **Settings → Persona → Learned expressions** → row **🗑️** | The Charisma stat row stays for history; click **Delete** there too if you want it gone |
| Forget a motion | **Settings → Persona → Learned motions** → row **🗑️** | Same as above |
| Remove a trait | Edit the persona quirks list; remove the line; **Save** | Charisma row remains until you delete it |

Deleting through the Charisma panel only deletes the *stat row*; the
underlying expression/motion file is untouched. Combine the persona-panel
delete and the Charisma-panel delete when you want a clean wipe.

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| Mirror is black, no face/pose tracked | Camera consent missing or another app is holding the device | Close the other app; right-click the pet → **Persona Teacher** → **Allow camera** again |
| **Capture** button is greyed out | The mirror has not received any landmark frames yet | Move into the camera frame; wait for the green tracking outline |
| Saved expression doesn't fire in chat | Trigger word doesn't appear in the brain's reply | Edit the trigger to a more common word, or add an `exampleDialogue` to the persona that uses the trigger |
| **▶ Test** plays the wrong character | The Window/Pet mode toggle is on the *other* character | Switch active VRM in **Settings → Character** before testing |
| Persona changes don't show in chat | An older chat thread cached the previous persona block | Open a new conversation, or click **Reload** in the chat header |
| Charisma panel never leaves **Untested** | Brain is offline or the trigger is not firing | Check **Brain** status in the toolbar; fix the provider; replay the trigger |
| Captured motion is jerky on playback | Camera dropped frames during the 1 s window | Re-record under steadier light, or apply **▼ Polish → Preview smoothed** |

---

## Where to next

- **Measure & promote** — [`charisma-teaching-tutorial.md`](./charisma-teaching-tutorial.md)
  covers the four maturity tiers, the rating UI, and the
  `⭐ Promote to source` button that turns a Proven asset into a
  multi-agent workflow plan.
- **Ship a teaching as a Pull Request** —
  [`self-improve-to-pr-tutorial.md`](./self-improve-to-pr-tutorial.md)
  picks up at the Promote click and walks through every step from
  workflow plan to merged PR.
- **Architecture deep-dive** —
  [`docs/persona-design.md`](../docs/persona-design.md) and
  [`docs/persona-pack-schema.md`](../docs/persona-pack-schema.md).
- **Rules that govern this surface** —
  [`rules/architecture-rules.md`](../rules/architecture-rules.md),
  [`rules/ui-ux-standards.md`](../rules/ui-ux-standards.md),
  [`rules/tutorial-template.md`](../rules/tutorial-template.md).
