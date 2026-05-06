# Tutorial: Self-Improve — From a Charisma Teaching to a Merged Pull Request

> **What you'll do.** Take any **Proven** Charisma asset (a captured
> facial expression, body motion, or persona quirk) and walk it all the
> way through TerranSoul's self-improve coding workflow until it lives
> in a GitHub Pull Request waiting for human review. Verified against
> TerranSoul `0.1` on 2026-05-06.
>
> **Sister tutorial.** Capturing the asset and growing it from
> **Untested** to **Proven** is covered in
> [`teaching-animations-expressions-persona-tutorial.md`](./teaching-animations-expressions-persona-tutorial.md)
> and [`charisma-teaching-tutorial.md`](./charisma-teaching-tutorial.md).
> This file picks up at the **⭐ Promote to source** click.
>
> **Design references.**
> [`docs/coding-workflow-design.md`](../docs/coding-workflow-design.md) ·
> [`docs/multi-agent-orchestration-analysis-2026.md`](../docs/multi-agent-orchestration-analysis-2026.md) ·
> [`tutorials/multi-agent-workflows-tutorial.md`](./multi-agent-workflows-tutorial.md).

Maps to the Human-Brain ↔ AI-System ↔ RPG-Stat triple:

| Human cognition | AI subsystem | RPG stat |
|---|---|---|
| Habit formation | Self-improve loop + multi-agent workflow | 🛠️ Engineer |
| Peer review | Reviewer agent + GitHub PR review | 🛠️ Engineer |

---

## What You Are Building

```
Charisma row "Proven"
        │
        ▼
   ⭐ Promote to source        ◀── charisma_promote() Tauri command
        │
        ▼
   WorkflowPlan YAML          ◀── <data_dir>/workflow_plans/<id>.yaml
   (Researcher → Coder*       *  = requires_approval
    → Tester → Reviewer*)
        │
        ▼
  Multi-agent runner          ◀── apply_file pipeline (atomic + path-safe)
        │
        ▼
   git stage + commit         ◀── coding/git_ops.rs
        │
        ▼
   open_self_improve_pr       ◀── coding/github.rs
        │
        ▼
   GitHub PR (idempotent)
        │
        ▼
   Reviewer-requested merge
```

By the end of this tutorial you will have produced an actual GitHub
Pull Request whose body lists the Charisma asset, the targeted source
file, the test slice that ran green, and the reviewer verdict.

---

## Requirements

- TerranSoul desktop running with the **Self-Improve** panel visible
  (right-click pet → **Self-Improve…**, or open via Quest tree once the
  *Engineer* skill activates).
- A **Coding LLM** configured separately from your chat brain. Open
  **Settings → Coding workflow** and pick a provider/model; the panel
  surfaces ready-to-use defaults. Without this, the Researcher / Coder
  / Tester / Reviewer steps cannot run.
- A **Proven** Charisma asset. If you don't have one yet, follow
  [`teaching-animations-expressions-persona-tutorial.md`](./teaching-animations-expressions-persona-tutorial.md)
  for the capture and `charisma-teaching-tutorial.md` for the rating
  loop.
- A **git checkout** of TerranSoul (or your fork). The self-improve
  loop refuses to run outside a git repository.
- A **GitHub account** with write access to the repository you want
  the PR opened against. For OSS contributions, fork first and
  configure the fork as `origin`.
- *(Optional but recommended)* GitHub CLI `gh` installed for the
  fastest reviewer experience — TerranSoul does not call `gh`, but
  reviewers often do.

> **Cost note.** The Coder step is the only step that *must* be a
> high-quality model (default Claude Sonnet 4.5). Researcher / Tester
> can be a fast local model. See
> [`tutorials/multi-agent-workflows-tutorial.md`](./multi-agent-workflows-tutorial.md)
> § *Switching LLMs per agent* for the recommended tiering.

---

## 1. Authorize TerranSoul to talk to GitHub

There are two supported paths. Use whichever matches your security
posture.

### 1.1 Device flow (recommended for personal machines)

1. Open the **Self-Improve** panel.
2. Scroll to **GitHub**.
3. Click **Authorize with GitHub in browser**.
4. The panel shows a one-time `user_code` (e.g. `ABCD-1234`) and a
   **verification URL** button. Click it — your default browser opens
   `https://github.com/login/device`.
5. Paste the code, sign in, approve the requested `repo` scope.
6. Switch back to TerranSoul. The `Authorize…` button flips to
   **Authorized as @&lt;your-handle&gt;** and the code chip clears.

The token is persisted by
[`coding::save_github_config`](../src-tauri/src/coding/github.rs)
into `<app_data>/github_config.json` with `chmod 600` semantics on
POSIX. The panel never shows the raw token after the initial save.

### 1.2 Personal Access Token (recommended for CI / shared machines)

1. Generate a fine-grained PAT on GitHub with **`repo`** scope on the
   target repository.
2. In **Self-Improve → GitHub**, paste it into the **Token** field.
3. Fill **Owner**, **Repo**, **Default base branch** (defaults to
   `main`), and any **Reviewers** GitHub handles.
4. Click **Save**. The panel echoes the resolved owner/repo so you can
   confirm.

> **Owner/Repo blank?** Leave them empty and click **Save** — the
> backend infers them from `git remote get-url origin` on first use.
> The next reload of the panel shows the resolved values.

---

## 2. Promote the Proven asset

Pick any Charisma row whose maturity is **✨ Proven**.

1. Right-click the pet → **Charisma — Teach me…**.
2. Find the row, e.g. `says 'indeed' a lot` under **📝 Traits**.
3. Click **⭐ Promote to source**.
4. A toast appears: *"Workflow plan `wfp_…` created — open the
   Multi-Agent Workflows panel to run it."*

Behind the scenes, [`charisma_promote`](../src-tauri/src/commands/charisma.rs)
calls the shared [`build_promotion_plan`](../src-tauri/src/coding/promotion_plan.rs)
to assemble the 4-step DAG and writes it to
`<data_dir>/workflow_plans/<id>.yaml`. The Charisma row's badge flips
to **🏛️ Canon** immediately because *the plan exists*; whether it ever
runs is now a separate decision.

The DAG is fixed by design:

| Step | Agent | `requires_approval` | Default model |
|---|---|---|---|
| `research` | 📚 Researcher | `false` | Gemini 2.0 Flash *(or your Coding LLM if no Researcher override)* |
| `code` | ⌨️ Coder | **`true`** | Claude Sonnet 4.5 |
| `test` | 🧪 Tester | `false` | `qwen2.5-coder:7b` |
| `review` | 🔍 Reviewer | **`true`** | Claude Opus 4 |

Two human approval gates ensure nothing is written to your tree —
let alone pushed to GitHub — without your explicit click.

---

## 3. Run the workflow

1. Right-click the pet → **Multi-agent workflows…**.
2. The **Workflows** tab lists your new plan with status
   `pending_review`. Click it open.

### 3.1 Step 1 — Researcher

1. Click **▶ Run** on the `research` step.
2. The Researcher LLM loads the file hints embedded in the plan — for a
   trait promotion these typically point at
   [`src-tauri/src/commands/persona.rs`](../src-tauri/src/commands/persona.rs)
   `default_persona_json()` — and reports back a short prose memo:
   which file to edit, which line range, and what the new content
   should look like.
3. Read the memo. If it points at the wrong file, edit the plan's
   YAML directly (`code.description`) and Save. The runner re-validates
   the DAG via Kahn's topological sort before persisting.

### 3.2 Step 2 — Coder *(approval gate)*

1. Click **▶ Run** on the `code` step.
2. The Coder produces a `<file path="...">…</file>` block.
3. Status flips to **awaiting approval**. The diff is rendered side-by-side.
4. Read the diff carefully:
   - Path lives inside the repo (`apply_file` rejects `..`, `.git/`,
     symlinks, absolute paths — see
     [`src-tauri/src/coding/apply_file.rs`](../src-tauri/src/coding/apply_file.rs)).
   - The change *appends* rather than rewrites a default — for traits
     this means the new quirk is added to the JSON list, not replacing
     it.
5. Click **Approve & Apply**. `apply_file` writes via temp + rename
   (atomic) and stages the change with `git add`.
6. *Or* click **Reject** to send the Coder a refinement note and
   re-run.

### 3.3 Step 3 — Tester

1. Click **▶ Run** on the `test` step.
2. The Tester runs the targeted CI slice — for trait changes it's
   `cargo test --lib persona` plus `npx vitest run src/stores/persona`.
3. Output streams into the panel. Green ✅ means the step completes;
   red ❌ blocks the Reviewer step from running.

> **Failure mode.** If a test fails, do **not** click "approve anyway"
> — there is no such button. Re-open the `code` step, share the failure
> log with the Coder, click **▶ Run** again. The Coder reads the new
> context and produces an updated `<file>` block; you re-approve and
> the Tester re-runs.

### 3.4 Step 4 — Reviewer *(approval gate)*

1. Click **▶ Run** on the `review` step.
2. The Reviewer audits the staged diff for:
   - Path-safety regressions (anything outside the original target).
   - Style consistency (snake_case for Rust, camelCase for TS).
   - Schema changes (the Reviewer flags any breaking schema migration).
   - PII leakage (especially for trait promotions of free-text quirks).
3. The verdict appears as **APPROVE** / **REQUEST CHANGES** with a
   prose justification.
4. Click **Approve & Merge** (the button label is intentionally
   identical to GitHub's). This step **does not push** — see § 4.

---

## 4. Open the Pull Request

The workflow runner stages and commits locally; pushing and PR opening
is one separate human-driven action.

1. Back in the **Self-Improve** panel, scroll to **GitHub**.
2. Verify the panel shows your current branch (it must not be
   `main` / `default_base`).
3. Click **Open Pull Request**.
4. The backend command
   [`open_self_improve_pr`](../src-tauri/src/commands/coding.rs):
   - Pushes the current branch to `origin` (creating it if needed).
   - Calls
     [`open_or_update_pr`](../src-tauri/src/coding/github.rs) which
     first runs **`find_open_pr`** for the same head branch.
   - If a PR is already open against this head, it is **updated** in
     place (idempotent), preventing duplicate PR spam.
   - Otherwise it `POST /repos/:owner/:repo/pulls` with a structured
     body (built by `build_chunk_pr_body`) summarising:
     - Title: `self-improve: complete autonomous chunks (<branch>)`
     - Body: one section per `RunRecord` ran, with timing, test
       results, and the Charisma asset that triggered the chain.
     - Reviewers: every handle from `GitHubConfig.reviewers`.
5. The panel surfaces the resulting `html_url`. Click it — your browser
   opens the PR.

> **Idempotency receipt.** Click **Open Pull Request** twice in a row.
> The second click does **not** open a duplicate; it reuses the
> existing record (`created: false` in `PrSummary`). This is how the
> autonomous loop survives restarts without spamming reviewers.

---

## 5. Wait for review

This is a *human* phase. TerranSoul's job is done — the rest is
ordinary GitHub workflow.

1. The reviewers you listed in `GitHubConfig.reviewers` get a
   review-request notification.
2. **Self-improve cannot merge for you.** Branch protection on `main`
   (or your equivalent) should require at least one approving review
   and a passing CI check before merge — TerranSoul respects that
   contract.
3. While you wait, the **Self-Improve** panel polls the PR status
   (open / approved / changes requested / merged / closed) and shows
   a pill in the GitHub section.
4. When a reviewer **requests changes**, treat it like the Reviewer
   step in § 3.4: open the `code` step, paste the reviewer's note as
   refinement context, re-run, re-approve, push again. The same
   `open_self_improve_pr` call updates the existing PR with the new
   commit — no duplicate PR is created.
5. When the PR is **merged**, pull the change locally:

   ```pwsh
   git checkout main
   git pull --ff-only origin main
   ```

   The new bundled default ships with every future install of
   TerranSoul.

---

## 6. Worked end-to-end example — `says 'indeed' a lot`

The author of this tutorial walked the full flow as a dry-run:

1. Promoted the Proven `says 'indeed' a lot` trait. Plan `wfp_a3f…`
   appeared in `<data_dir>/workflow_plans/wfp_a3f….yaml`.
2. Researcher (gemini-2.0-flash) reported:
   *"Target file is `src-tauri/src/commands/persona.rs`,
   `default_persona_json()`. Append `\"says 'indeed' a lot\"` to the
   `quirks` array."*
3. Coder (claude-sonnet-4.5) emitted a `<file>` block adding the
   single line. Diff approved.
4. Tester (`qwen2.5-coder:7b`) ran `cargo test --lib persona` →
   8 passed, 0 failed.
5. Reviewer (claude-opus-4) verdict: *"APPROVE — additive,
   schema-compatible, no PII."*
6. Clicked **Open Pull Request**. PR
   `https://github.com/<owner>/<repo>/pull/482` was created with
   reviewer request for the configured admin handles.
7. Reviewer left one comment: *"Wrap the literal in single quotes for
   consistency with neighbouring entries."* Re-ran the Coder with the
   note, re-approved, clicked **Open Pull Request** again — same PR
   `#482`, new commit, `created: false`.
8. Second review: **Approve**. Merged via the GitHub UI.
9. Pulled `main` locally; the next fresh install ships with the new
   default.

Total wall-clock active time: ~6 minutes of human attention spread
over a half-day waiting on the reviewer.

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| **⭐ Promote to source** is greyed out | Asset is not yet **Proven** (needs ≥ 10 uses *and* avg rating ≥ 4.0) | Keep using and rating the asset; check the **Charisma** dashboard tier counts |
| Workflow plan not appearing in panel | Multi-agent workflows panel was open before the plan was created | Close & reopen the panel (it lazy-loads from `<data_dir>/workflow_plans/`) |
| **Researcher** can't find the target file | The asset `kind` doesn't match the file hints in the plan | Edit the plan YAML; update the `code.description` to point at the right file; re-run |
| **Coder** never finishes | Network drop or coding LLM provider rate-limited | Click **Cancel** then **▶ Run** again; the Researcher memo is reused |
| **Coder** writes outside the repo | Defensive — `apply_file` will refuse | The error toast cites the path; tighten the `code.description` and re-run |
| **Tester** fails on an unrelated test | Local working tree was dirty before the workflow started | `git status`; commit or stash unrelated changes; re-run from `code` |
| **Reviewer** verdict is **REQUEST CHANGES** | Reviewer found a real issue | Read the prose; refine the `code` step; the rest of the DAG re-runs |
| **Open Pull Request** errors with `Not inside a git repository` | App data dir resolved outside a checkout | Move/relink the app data directory under your TerranSoul checkout, or re-clone |
| **Open Pull Request** errors with `currently on the base branch` | You are still on `main` | `git switch -c self-improve/<asset>` first; the panel will then push that branch |
| PR opens but reviewers aren't tagged | `GitHubConfig.reviewers` is empty | Self-Improve → GitHub → fill the **Reviewers** field; click **Save**; click **Open Pull Request** again — same PR is updated with the new request |
| You clicked **Open Pull Request** twice | Idempotent on purpose | Check the PR — only one exists; the second call returned `created: false` |

---

## Where to next

- [`tutorials/teaching-animations-expressions-persona-tutorial.md`](./teaching-animations-expressions-persona-tutorial.md)
  — capture the asset that becomes the PR.
- [`tutorials/charisma-teaching-tutorial.md`](./charisma-teaching-tutorial.md)
  — the maturity ladder (Untested → Learning → Proven → Canon) that
  unlocks **⭐ Promote to source**.
- [`tutorials/multi-agent-workflows-tutorial.md`](./multi-agent-workflows-tutorial.md)
  — the underlying DAG runner; useful when you want to schedule promotions
  to run automatically (e.g., every Sunday afternoon).
- [`docs/coding-workflow-design.md`](../docs/coding-workflow-design.md)
  — design rationale, comparative study against other agentic-coding
  systems, and the output-shape contract.
- [`rules/coding-workflow-reliability.md`](../rules/coding-workflow-reliability.md)
  and [`rules/prompting-rules.md`](../rules/prompting-rules.md) —
  governance rules every step of the runner respects.
- [`rules/tutorial-template.md`](../rules/tutorial-template.md) —
  the format this tutorial follows, in case you want to add another.
