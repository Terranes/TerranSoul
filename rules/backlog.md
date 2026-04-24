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

❌ **Removed (2026-04-24).** Encryption-based VRM asset protection is
not feasible for an open-source Tauri desktop application: any
decryption key compiled into the binary is extractable by anyone who
downloads the app, reducing the scheme to obfuscation rather than real
DRM. VRM creators rely on copyright and the model's accompanying ToS
as their primary protection; TerranSoul will not pretend to offer
technical DRM it cannot actually deliver. Chunks 100–105 are removed
and will not be re-promoted.

---

## Phase 9 — Learned Features (From Reference Projects)

> **Source repos:** Open-LLM-VTuber, AI4Animation-js, VibeVoice, aituber-kit
> **Analysis:** See `rules/research-reverse-engineering.md` §9.
> **Principle:** Integrate proven patterns; don't reinvent.

### High Priority

📦 Promoted to `rules/milestones.md` — chunks 106–109.

### Medium Priority

📦 Promoted to `rules/milestones.md` — chunks 094–098.

### Lower Priority

📦 Promoted to `rules/milestones.md` — chunks 115–119 (renumbered from 110–114 to avoid conflict with Chunk 110 BGM).

### Demoted from Milestones

| Chunk | Description | Status | Notes |
|-------|-------------|--------|-------|
| 117 | **Docker Containerization** — Run TerranSoul in isolated containers for CI/testing and server deployment (Open-LLM-VTuber pattern). CPU/GPU variants. | `not-needed` | Re-analysis: TerranSoul is a Tauri desktop app — Docker is not applicable. If container orchestration for LLM inference servers is ever needed, use .NET Aspire to manage Docker instead of raw Dockerfiles. |
