# Coding Workflow Reliability Rules

> **Mandatory invariant** — Every coding workflow in TerranSoul (the
> reusable `run_coding_task` runner, the autonomous self-improve loop,
> any future agent that reuses `coding::workflow`, and the Tauri/Pinia
> surface that drives them) **must be 100% durable, reliable, atomic,
> and resilient**. These are not aspirational adjectives — they are
> defined operationally below, and every PR that touches a coding
> workflow must satisfy every clause or be rejected.

This file is a peer of `rules/architecture-rules.md` and is enforced by
the milestones gate and the CI gate (`npx vitest run && npx vue-tsc
--noEmit && cargo clippy -- -D warnings && cargo test`).

---

## 1. Durability — state survives crashes, kills, and reboots

A workflow is **durable** when no acknowledged user action is ever lost
to a process crash, OS reboot, hard kill, or disk pressure.

### Required practices

1. **Persist before acknowledge.** Any command that returns `Ok(())` to
   the UI for a state-changing action MUST have already flushed the new
   state to disk before returning. Never acknowledge an in-memory mutation.
2. **Atomic file writes.** All JSON config files (`coding_llm_config.json`,
   `self_improve.json`, `coding_workflow_config.json`, metrics logs,
   self-improve state) MUST be written via the `write-temp-then-rename`
   pattern: serialise to `*.json.tmp` in the same directory, `fsync`,
   then `rename` over the destination. Never write directly to the
   live path. Helper: `crate::coding::atomic_write_json`.
3. **No silent writes.** Every persistence call site MUST `?`-propagate
   the error. Swallowing a write error with `let _ =` or `.ok()` is
   forbidden in workflow code paths.
4. **Backups on schema migration.** When a config struct gains/loses a
   field, the loader MUST keep backwards compatibility via `#[serde(default)]`
   and MUST NOT delete the old file. If a destructive migration is
   unavoidable, copy the old file to `*.bak` first.
5. **Idempotent on resume.** A workflow that was killed mid-run MUST be
   safely re-runnable from the last persisted checkpoint with no
   duplicate side effects (no double commits, double PRs, double
   metrics rows).

### Forbidden anti-patterns

- Writing config to a file you also read from in the same call (race).
- Using `tempfile::NamedTempFile::persist` across volumes (silently
  copies on Windows; use same-directory temp + rename).
- Holding a `MutexGuard` across an `await` while a write is in flight.

---

## 2. Atomicity — operations either fully apply or fully revert

A workflow is **atomic** when every observable side effect of a single
logical operation either *all* lands or *none* lands. Partial state is
never exposed to another process or to the UI.

### Required practices

1. **Single source of truth per setting.** Every persistent setting has
   exactly one disk file and exactly one `Mutex<...>` field on
   `AppState`. The Tauri command updates *both* in a single critical
   section using this exact order:

   ```text
   1. Validate input.
   2. Write file via atomic_write_json (temp + fsync + rename).
   3. On success, swap the in-memory value under the Mutex.
   4. Return Ok.
   ```

   If step 2 fails, step 3 MUST NOT execute. The in-memory value stays
   on the previous good state, and the command returns an `Err`.
2. **Git operations are transactional.** The self-improve loop MUST
   never leave a half-applied change in the user's working tree. Clean
   active checkouts can stage generated changes only after review and
   tests pass; dirty active checkouts MUST run autonomous apply/test in
   a temporary git worktree and save a patch artifact instead of mixing
   generated edits with user changes. No "uncommitted dirty branch"
   terminal states.
3. **Two-phase tray toggle.** Tray-driven self-improve toggles MUST
   persist the new `SelfImproveSettings` *before* starting/stopping
   the engine. If the engine fails to start, the persisted state is
   reverted via the same atomic helper.
4. **Database multi-row writes use transactions.** Any SQLite write
   that touches more than one row of memory/metrics/versioning data
   MUST be wrapped in `BEGIN ... COMMIT`. No partially-committed
   batches.

---

## 3. Reliability — the same input produces the same outcome

A workflow is **reliable** when, given the same persisted state and
the same user input, it produces the same observable behaviour every
run, on every supported platform, regardless of timing.

### Required practices

1. **Deterministic prompts.** The `<documents>` block produced by
   `load_workflow_context` MUST be sorted (filename ascending). No
   `HashMap` iteration order in prompt assembly.
2. **No wall-clock branching in core logic.** Time-sensitive behaviour
   (decay, recency boosts, cooldowns) MUST take an injectable clock
   so tests can pin it. Production passes `SystemTime::now()`; tests
   pass a fixed instant.
3. **Bounded retries with explicit budgets.** Network calls (LLM, git,
   GitHub API) use exponential backoff with a hard retry cap (default
   3) and a hard total-time budget (default 60s). After the budget
   expires, surface the error — do not retry forever.
4. **Cross-platform file paths.** Use `std::path::Path::join` and
   `to_string_lossy().replace('\\', "/")` when emitting labels for
   prompts. Never hand-build paths with `format!("{dir}/{file}")`.
5. **No flaky tests.** Any test that touches the filesystem uses
   `tempfile::TempDir`. Any test that touches time uses an injected
   clock. A test that relies on timing being "fast enough" is banned.

---

## 4. Resilience — the system degrades gracefully under failure

A workflow is **resilient** when external failures (network down,
disk full, LLM 5xx, git conflict, missing repo, malformed user input)
never crash the process and always produce a recoverable error visible
to the user.

### Required practices

1. **No `unwrap`/`expect` in workflow code paths.** All `Result` types
   propagate via `?`. The only exceptions are `OnceLock` initialisation
   in `main` and infallible `from_str` on compile-time literals.
   Existing call sites violating this rule MUST be fixed when touched.
2. **Typed errors at module boundaries.** Each coding submodule
   (`workflow`, `engine`, `git_ops`, `github`, `metrics`) defines a
   `thiserror`-derived error enum. `String` errors are allowed only
   at the Tauri command boundary (where `Result<T, String>` is
   required by Tauri serde).
3. **Cancellation is observed everywhere.** Every long-running loop
   (`SelfImproveEngine`, the planner, any spawn) MUST check the
   `cancel: Arc<AtomicBool>` flag at every iteration boundary and
   between phases. A cancelled loop returns within 2 seconds.
4. **Graceful degradation for missing context.** If a configured
   `include_dir` or `include_file` does not exist on disk, the loader
   MUST emit a structured warning event (`workflow.warn`) and
   continue with the remaining sources — never abort the task.
5. **UI never freezes.** Every Tauri command that may take >100 ms
   MUST be `async` and MUST emit progress events at a minimum cadence
   of one event per phase. The Vue layer MUST handle `Err` returns
   without throwing — every store action wraps the `invoke` call in
   `try/catch` and surfaces a toast.
6. **Backoff on provider failure.** When the coding LLM returns 429,
   503, or a network error, the engine sleeps with exponential
   backoff (1s, 2s, 4s, capped at 60s) and emits an `info` event so
   the user sees the retry. After 3 consecutive failures, the loop
   pauses and emits an `error` event.

---

## 5. Verification — how to prove a change satisfies this rule

Every PR that adds or changes a coding-workflow code path MUST include:

1. **A unit test for the happy path** of the new code.
2. **A unit test for at least one failure path** — disk write fails,
   network returns 5xx, file missing, invalid input. Use `tempfile`
   + a faulty path (e.g. `/dev/full` on Linux, a read-only dir on
   Windows) or a stub HTTP client.
3. **A cancellation test** when adding a new long-running loop.
4. **An atomicity test** when adding a new persisted field — kill
   the write halfway (truncate the temp file before rename) and
   verify the on-disk state is still the previous good value.
5. **The full CI gate green:**
   `npx vitest run && npx vue-tsc --noEmit && cargo clippy -- -D warnings && cargo test`.

A PR that adds a coding-workflow code path with no failure-path test
is incomplete and MUST be rejected on review.

---

## 6. Enforcement checklist (paste into PR description)

- [ ] All persisted state uses `atomic_write_json` (temp + fsync + rename).
- [ ] No `unwrap`/`expect` added to workflow code paths.
- [ ] All long-running loops observe the `cancel` flag every iteration.
- [ ] Every new Tauri command emits progress events for >100 ms work.
- [ ] Every new module boundary error is a `thiserror` enum.
- [ ] Every new test uses `tempfile::TempDir` for filesystem work.
- [ ] At least one failure-path test exists for every new code path.
- [ ] Cross-platform paths use `Path::join`, never `format!`.
- [ ] `<documents>` ordering is deterministic (sorted).
- [ ] Full CI gate is green locally.

---

## 7. References

- `rules/architecture-rules.md` — overall architecture invariants.
- `rules/coding-standards.md` — language-level coding rules.
- `rules/quality-pillars.md` — broader quality posture.
- `docs/brain-advanced-design.md` — brain subsystem design (when the
  workflow touches brain state).
