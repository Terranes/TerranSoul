# TerranSoul — Backlog

> **Never-started work lives here.** Only move chunks from this file to
> `milestones.md` when the user explicitly says so. This file is the holding
> area for planned but unscheduled work.
>
> ⛔ **RULE: Never start chunks from this file.**
> All chunk implementation must begin from `rules/milestones.md`.
> If milestones.md has no `not-started` chunks, ask the user which backlog items to promote.
> See `rules/prompting-rules.md` for full enforcement rule.

---

## Phase 7 — VRM Model Security (Anti-Exploit & Asset Protection)

📦 Promoted to `rules/milestones.md` — chunks 070–075.

---

## Phase 9 — Learned Features (From Reference Projects)

> **Source repos:** Open-LLM-VTuber, AI4Animation-js, VibeVoice, aituber-kit
> **Analysis:** See `rules/research-reverse-engineering.md` §9.
> **Principle:** Integrate proven patterns; don't reinvent.

### High Priority

📦 Promoted to `rules/milestones.md` — chunks 090–093.

### Medium Priority

| Chunk | Description | Status |
|-------|-------------|--------|
| 094 | **Model Position Saving** — Persist camera orbit position, zoom, rotation per model. Resume user's preferred viewing angle on app restart. Store in Tauri settings alongside model selection. | `not-started` |
| 095 | **Procedural Gesture Blending (MANN-inspired)** — Learn from AI4Animation MANN approach: instead of hardcoded JSON keyframes, use lightweight ML or procedural blending to generate smooth transitions between emotion states. Train on existing gesture data. Replace stiff cross-fades with natural motion. | `not-started` |
| 096 | **Speaker Diarization** — Detect multiple speakers in room (VibeVoice-ASR-7B pattern). Tag "who said what" in conversation log. Useful for group scenarios or streaming. | `not-started` |
| 097 | **Hotword-Boosted ASR** — Let users define domain-specific keywords (character names, game terms) that ASR should recognize better. VibeVoice supports hotword injection. | `not-started` |
| 098 | **Presence / Greeting System** — Auto-greeting when user appears (timer-based or face detection), auto-goodbye when away. Track "away duration" for different responses (aituber-kit pattern). | `not-started` |

### Lower Priority

| Chunk | Description | Status |
|-------|-------------|--------|
| 099 | **Live2D Support** — Add Live2D rendering alongside VRM using `@cubism/cubism4-runtime-js` (aituber-kit pattern). Useful for users who prefer 2D or have only 2D models. Renderer abstraction layer. | `not-started` |
| 100 | **Screen Recording / Vision** — Extend beyond static context: real-time screen activity analysis (Open-LLM-VTuber pattern). Use Tauri window capture API. Character can comment on what user is doing. | `not-started` |
| 101 | **Docker Containerization** — Run TerranSoul in isolated containers for CI/testing and server deployment (Open-LLM-VTuber pattern). CPU/GPU variants. | `not-started` |
| 102 | **Chat Log Export** — JSON export with timestamps, sentiment tags, emotion metadata. Build on existing conversation persistence. | `not-started` |
| 103 | **Language Translation Layer** — Accept input in one language, TTS output in another. Use LLM for translation. Store original + translated text. | `not-started` |
