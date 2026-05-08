# Tutorial Template

> **Why this rule exists.** TerranSoul ships a growing library of
> end-user tutorials under [`tutorials/`](../tutorials/). They are
> the *first* surface a new user touches after the README. Drift between
> tutorial steps and shipping code creates a worse onboarding experience
> than no tutorial at all. This rule pins the shape every new tutorial
> must follow so the whole set stays consistent, navigable, and verifiable.
>
> Last updated: 2026-05-06.

---

## 1. Where tutorials live

- **All user-facing tutorials** live under [`tutorials/`](../tutorials/),
  one Markdown file per topic, named `<topic>-tutorial.md`
  (kebab-case, always ends in `-tutorial.md`).
- Screenshots go under `tutorials/screenshots/<topic>/NN-step.png`
  (numeric prefix, kebab-case label) so they sort by walkthrough order.
- Architecture / design docs that *explain how something is built*
  live under [`docs/`](../docs/) and are linked **from** the tutorial
  in the header — never the other way around. Tutorials read
  top-to-bottom; design docs are the reference companion.
- Internal-only tooling notes (e.g. `instructions/IMPORTING-MODELS.md`)
  stay in [`instructions/`](../instructions/) and are not subject to
  this template.

## 2. Required structure

Every new tutorial under `tutorials/` must include, in order:

1. **`# Tutorial: <Title>`** or **`# <Title> — Tutorial`** as the H1.
2. A **block-quote intro** (≤ 4 lines) stating *what* the reader will
   build/learn and the project version it was last verified against.
3. Optional but recommended: a **Human-Brain ↔ AI-System ↔ RPG-Stat**
   table when the feature maps to TerranSoul's cognitive triple
   (see `tutorials/charisma-teaching-tutorial.md` § 1).
4. A **"What You Are Building"** ASCII or Mermaid diagram, *or* a
   bullet list of concrete artifacts the reader will produce.
5. A **Requirements** section (OS, hardware, optional services,
   configuration toggles, prior tutorials).
6. **Numbered walkthrough steps**, each one:
   - Names the **exact UI surface** (panel, menu, right-click target)
     or the **exact Tauri command** / file path being touched.
   - Quotes button labels and field names verbatim from the running app.
   - Links to the relevant source file(s) using a workspace-relative
     path so reviewers can verify drift in one click.
7. At least one **worked end-to-end example** with realistic, named
   inputs (no `foo` / `bar`).
8. A **Troubleshooting** section listing the failure modes the author
   actually hit while writing the tutorial.
9. A **Where to next** footer linking to related tutorials, design docs,
   and the `rules/` files that govern the feature.

## 3. Verification before merge

- The tutorial author **must dry-run every numbered step** against a
  current build of TerranSoul (or the headless MCP runner for backend
  tutorials) and fix code drift discovered during the run *in the same
  PR* — never leave a "this step is currently broken" note.
- Every internal link must resolve (workspace-relative paths only;
  no `https://github.com/<owner>/<repo>/blob/...` for in-repo files).
- Every quoted button label, command id, file path, and CLI flag must
  match the running app exactly. When the implementation changes,
  the tutorial changes in the same PR.
- Screenshots must reflect the current UI within one minor version.
  Outdated screenshots are worse than missing ones — delete first,
  re-capture later.

## 4. Cross-cutting rules that still apply

- **Third-party naming hygiene** (`rules/coding-standards.md` §
  Third-Party Naming & Licensing Hygiene) — no creator/channel
  branding in tutorial titles, file names, character names, or step
  labels. Attribution belongs in [`CREDITS.md`](../CREDITS.md).
- **Markdown is not MCP memory** (`rules/coding-standards.md` §
  MCP Markdown Memory Boundary) — durable lessons learned while
  writing the tutorial sync into `mcp-data/shared/memory-seed.sql`,
  not into the tutorial body.
- **Brain documentation sync** (`rules/architecture-rules.md`
  rule 10) — tutorials that touch brain/RAG/memory must keep
  `docs/brain-advanced-design.md` and `README.md` aligned.
- **Always credit external influences** in `CREDITS.md` when an
  outside source (paper, video, project, doc) shaped the tutorial's
  approach.

## 5. Tutorial inventory

The canonical inventory lives in
[`memory-seed.sql`](../mcp-data/shared/memory-seed.sql) (memory id
covering "Design docs and tutorials"). When a tutorial is added,
removed, or renamed, update that seed entry in the same PR so the
brain stays authoritative.
