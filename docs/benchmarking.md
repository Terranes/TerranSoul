# TerranSoul — Benchmarking Guide

> Last updated: 2026-05-06
>
> This document explains how the existing **Million-memory benchmark**
> works, how to run it, how to read its JSON report, and how to add
> **new benchmarks** for other parts of TerranSoul (RAG retrieval,
> ingestion, dedup, capacity pruning, KG traversal, etc.).
>
> Benchmark harness file: [src-tauri/benches/million_memory.rs](../src-tauri/benches/million_memory.rs)
> Bench wiring: [src-tauri/Cargo.toml](../src-tauri/Cargo.toml)
> Report output: `src-tauri/target/bench-results/*.json`

---

## 1. Why we benchmark

TerranSoul's brain is a long-lived store: memory keeps growing across
chat sessions, ingests, and quests. We need reproducible evidence that
core paths stay fast as the store scales toward 1M+ entries:

- **Hybrid search latency** stays within p99 budgets so chat UI feels
  instant.
- **Capacity eviction** keeps the long tier bounded without blocking
  user-visible flows.
- **Future ingestion / dedup / KG paths** have measurable budgets
  before regressions reach users.

Benchmarks live next to the Rust code they exercise (`src-tauri/benches/`),
use [Criterion](https://bheisler.github.io/criterion.rs/book/), and emit
machine-readable JSON reports under
`src-tauri/target/bench-results/` so CI and humans can diff numbers
across runs.

---

## 2. Million-memory benchmark (Chunk 38.5)

The harness exercises the **HNSW vector stage** used by hybrid search
plus the **`enforce_capacity` eviction path** used by the long-tier GC.

### 2.1 What it measures

| Path | What is measured | Hard threshold |
| --- | --- | --- |
| HNSW retrieval | p50 / p95 / p99 / max latency over 1,000 queries, top-10, 768-dim vectors | p50 <= 30 ms · p95 <= 60 ms · p99 <= 100 ms (full tier only) |
| HNSW build | total seconds + vectors/sec to populate the index | reported, no hard gate |
| Capacity pruning | seconds to enforce `cap * 1.05 -> cap * 0.95`, dropped count, kept protected/important | <= 30 s at 1M (full tier) |
| Linear backend | explicitly skipped at 1M (out of design) | reported as `not_run_for_smoke` / skip reason |

Vectors are deterministic (seeded `Xoshiro256PlusPlus`,
`VECTOR_SEED = 0x38_05_0000_0000_0001`,
`QUERY_SEED  = 0x38_05_0000_0000_0002`), so two runs on the same machine
are directly comparable.

### 2.2 Running it

Always run benches from the workspace root with a **separate Cargo
target dir** so the running TerranSoul MCP binary is not relocked:

```powershell
# Smoke tier — 10,000 vectors, CI-friendly, ~5 minutes cold compile
Set-Location src-tauri
cargo bench --bench million_memory --target-dir ../target-copilot-bench
Set-Location ..
```

```powershell
# Full tier — 1,000,000 vectors, local/nightly only
Set-Location src-tauri
cargo bench --bench million_memory --features bench-million `
  --target-dir ../target-copilot-bench
Set-Location ..
```

The full tier requires the `bench-million` feature (which pulls in
`native-ann` / `usearch`). On Windows, the bench refuses to allocate
1M vectors unless ~90% of available RAM can hold the estimated HNSW
footprint — set `TS_BENCH_FORCE_LARGE=1` to override.

> **Windows / MCP rule.** Never kill the running TerranSoul MCP
> terminal. Always pass `--target-dir ../target-copilot-bench` (or
> another non-default path) so the bench doesn't try to overwrite the
> live `terransoul.exe` artifact. Clean the temporary dir after
> validation.

### 2.3 Environment knobs

| Variable | Purpose | Default |
| --- | --- | --- |
| `TS_BENCH_SCALES` | Comma-separated list of integer scales to run (e.g. `1000,10000,100000`). | `10000` (smoke) or `1000000` (with `--features bench-million`) |
| `TS_BENCH_FORCE_LARGE` | Set to `1` to bypass the available-RAM gate at 1M+. | unset |
| `TS_BENCH_OUTPUT_DIR` | Override the report output base. JSON is written to `<dir>/bench-results/million_memory.json`. | `src-tauri/target/` |

Examples:

```powershell
$env:TS_BENCH_SCALES = "1000,10000,100000"
cargo bench --bench million_memory --target-dir ../target-copilot-bench
Remove-Item Env:TS_BENCH_SCALES
```

### 2.4 Reading the JSON report

Every run overwrites `src-tauri/target/bench-results/million_memory.json`
(or `<TS_BENCH_OUTPUT_DIR>/bench-results/million_memory.json`). Schema:

```jsonc
{
  "generated_at_unix_ms": 1778069443446,
  "command": "<absolute path to the bench exe> --bench",
  "machine": {
    "os_name": "Windows",
    "os_version": "Windows 11 Pro",
    "kernel_version": "26200",
    "cpu_brand": "12th Gen Intel(R) Core(TM) i9-12900K",
    "logical_cpus": 24,
    "total_memory_gib": 63.71,
    "available_memory_gib_at_start": 33.61,
    "current_exe": "<...>"
  },
  "hnsw": [
    {
      "scale": 10000,
      "status": "completed",            // or "skipped_for_capacity" / "failed"
      "retrieval_kind": "hnsw_vector_stage_for_hybrid_search",
      "query_count": 1000,
      "top_k": 10,
      "dimensions": 768,
      "seed": 4036632641007517697,
      "raw_vector_gib": 0.0286,
      "estimated_hnsw_gib": 0.0393,
      "available_memory_gib_before": 33.81,
      "build_seconds": 11.39,
      "vectors_per_second": 877.71,
      "p50_ms": 0.57, "p95_ms": 0.74, "p99_ms": 0.86, "max_ms": 1.03,
      "retrieval_total_seconds": 0.58,
      "failure": null
    }
  ],
  "linear_backend": [
    { "scale": 10000, "status": "not_run_for_smoke", "reason": "..." }
  ],
  "capacity": [
    {
      "scale": 10000, "status": "completed",
      "initial_long_count": 10500, "cap": 10000, "target_ratio": 0.95,
      "expected_target": 9500, "elapsed_seconds": 0.26,
      "long_count_before": 10500, "dropped": 1000, "long_count_after": 9500,
      "kept_protected": 10, "kept_important": 10,
      "protected_after": 10, "important_after": 10,
      "failure": null
    }
  ]
}
```

Status strings to look for:

- `"completed"` — measurement succeeded; latency/elapsed fields are populated.
- `"skipped_for_capacity"` — RAM gate refused this scale; set
  `TS_BENCH_FORCE_LARGE=1` to override on a beefier machine.
- `"not_run_for_smoke"` — linear backend is intentionally skipped at
  large scales because it is O(n) and out of design.
- `"failed"` — see `failure` for the error message.

### 2.5 Latest measured smoke (reference baseline)

Windows 11 Pro · 12th Gen Intel(R) Core(TM) i9-12900K · 24 logical CPUs · 63.7 GiB RAM
(2026-05-07, 10k vectors, 1,000 queries, top-10):

| Metric | Value | Threshold |
| --- | --- | --- |
| HNSW p50 | 0.57 ms | <= 30 ms |
| HNSW p95 | 0.74 ms | <= 60 ms |
| HNSW p99 | 0.86 ms | <= 100 ms |
| HNSW max | 1.03 ms | informational |
| Build | 11.39 s · 877 vectors/s | informational |
| Capacity 10,500 -> 9,500 | 0.26 s | <= 30 s |

These are smoke-tier numbers; the **HNSW p99 <= 100 ms hard gate**
applies to the **full 1M tier**, which must be re-run on a development
machine with enough RAM whenever code touching `AnnIndex`,
`enforce_capacity`, or the embedding shape changes.

---

## 3. Adding a new benchmark

Use this section as a step-by-step recipe whenever you need a
reproducible latency / throughput number for any other TerranSoul path
(RAG end-to-end, wiki dedup, KG neighbour traversal, ingestion, etc.).

### 3.1 Decide what to measure

Before writing code, write down:

1. **The path under test** in one sentence (e.g. "wiki dedup decision
   for a single 4 KB candidate against a 100k-entry store").
2. **Inputs and seed strategy** — every input must be deterministic
   (`rand_xoshiro` with a constant seed, or a fixture file).
3. **Hard thresholds** — concrete p50/p95/p99 budgets or throughput
   floors. If you cannot defend a threshold yet, mark the bench
   "informational" and emit numbers without asserting.
4. **Tiers** — at least a CI-friendly **smoke tier** and an optional
   **full tier** behind a feature flag (mirror `bench-million`).

### 3.2 Wire a new bench in `Cargo.toml`

Edit [src-tauri/Cargo.toml](../src-tauri/Cargo.toml) and add an entry
next to the existing `[[bench]]`:

```toml
[features]
# Heavy variant of the new bench, off by default.
bench-wiki-dedup = ["native-ann"]

[[bench]]
name = "wiki_dedup"
harness = false
required-features = ["native-ann"]
```

Reuse existing dev-deps (`criterion = "0.5"`, `rand = "0.9"`,
`rand_xoshiro = "0.7"`, `tempfile = "3"`); only add new dev-deps when a
crate already used by the runtime cannot satisfy the bench.

### 3.3 Skeleton bench file

Create `src-tauri/benches/<your_bench>.rs` and follow the same shape
as [million_memory.rs](../src-tauri/benches/million_memory.rs):

```rust
//! <One-line summary>.
//!
//! Smoke run:  cargo bench --bench <your_bench>
//! Full run:   cargo bench --bench <your_bench> --features bench-<feature>
//! Report:     target/bench-results/<your_bench>.json

use criterion::{black_box, BenchmarkId, Criterion};
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use serde::Serialize;
use std::{env, fs, path::PathBuf, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

const SEED: u64 = 0x__BENCH_ID__;
const QUERY_COUNT: usize = 1_000;
const SMOKE_SCALE: usize = 10_000;
const FULL_SCALE: usize = 1_000_000;

#[derive(Serialize)]
struct Report {
    generated_at_unix_ms: u128,
    command: String,
    machine: MachineSpec,    // copy from million_memory.rs
    runs: Vec<RunReport>,
}

#[derive(Serialize)]
struct RunReport {
    scale: usize,
    status: String,           // "completed" | "skipped_*" | "failed"
    p50_ms: Option<f64>,
    p95_ms: Option<f64>,
    p99_ms: Option<f64>,
    failure: Option<String>,
}

fn run_scale(scale: usize, c: &mut Criterion) -> RunReport { /* ... */ }

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    let scales = parse_scales();   // honour TS_BENCH_SCALES
    let mut runs = Vec::new();
    for scale in scales {
        runs.push(run_scale(scale, &mut criterion));
    }
    write_report("<your_bench>.json", runs);
    criterion.final_summary();
}
```

Required behaviours (lifted from the million-memory harness):

- Use **`Criterion::iter_custom`** when you need honest wall-clock
  percentiles; raw `bench_function` is fine for steady-state throughput.
- Always pass inputs through **`black_box`** so the optimiser cannot
  fold the hot path away.
- Encode pass/fail as **status strings + assertions** at the end of
  `main`. Example: `assert!(p99 <= 100.0, "p99 {} ms exceeds budget", p99);`.
- Honour `TS_BENCH_SCALES`, `TS_BENCH_FORCE_LARGE`, and
  `TS_BENCH_OUTPUT_DIR` via the same parser pattern as
  [million_memory.rs](../src-tauri/benches/million_memory.rs) so all
  benches share one operator interface.
- Write the JSON report to
  `<TS_BENCH_OUTPUT_DIR or src-tauri/target/>/bench-results/<your_bench>.json`.
  Do **not** emit reports into the source tree.

### 3.4 Run, validate, clean up

```powershell
Set-Location src-tauri
cargo bench --bench <your_bench> --target-dir ../target-copilot-bench
Set-Location ..
# Inspect the report:
Get-Content src-tauri/target/bench-results/<your_bench>.json
# Clean the temporary target after validation:
Remove-Item -Recurse -Force target-copilot-bench
```

### 3.5 Sync the result into project memory

For every new bench, in the same PR:

1. Update [README.md](../README.md) with a one-line bullet under the
   relevant component section (latest measured number + threshold).
2. If the bench touches the brain surface (memory store, RAG, KG,
   embeddings), also update
   [docs/brain-advanced-design.md](brain-advanced-design.md) — the
   brain doc-sync rule is mandatory (see
   [rules/architecture-rules.md](../rules/architecture-rules.md) rule 10).
3. Append a durable lesson row to
   [mcp-data/shared/memory-seed.sql](../mcp-data/shared/memory-seed.sql)
   describing the bench name, command, thresholds, and report path so
   future agents discover it via `brain_search`.
4. Log the chunk in [rules/completion-log.md](../rules/completion-log.md)
   and remove its row from [rules/milestones.md](../rules/milestones.md).

### 3.6 Checklist

- [ ] `[[bench]]` entry added with `harness = false` and the right
      `required-features`.
- [ ] Deterministic seeds (constants, not `thread_rng()`).
- [ ] Smoke tier runs without a feature flag; heavy tier is gated.
- [ ] Honours `TS_BENCH_SCALES` / `TS_BENCH_FORCE_LARGE` /
      `TS_BENCH_OUTPUT_DIR`.
- [ ] JSON report under `target/bench-results/`, never in source.
- [ ] Hard thresholds asserted in `main`, or bench marked
      "informational" with rationale.
- [ ] README + brain design doc + seed SQL updated; chunk archived in
      `completion-log.md`.

---

## 4. Troubleshooting

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| `Access is denied (os error 5)` overwriting `terransoul.exe` | Default Cargo target dir collides with running MCP binary. | Always pass `--target-dir ../target-copilot-bench`. Never kill the MCP terminal. |
| Bench reports `status: "skipped_for_capacity"` at 1M | RAM gate refused the run (estimated HNSW > 90% of available RAM). | Run on a host with more free RAM, or set `TS_BENCH_FORCE_LARGE=1` to override. |
| Cold compile takes several minutes | Criterion + sysinfo + usearch in a fresh target dir. | Reuse `target-copilot-bench` between runs; only delete it after a clean validation. |
| JSON report missing | Bench crashed before `write_report`. | Inspect terminal output; failures are also recorded as `status: "failed"` with `failure: "..."` when the bench survives the panic. |
| Numbers swing run-to-run | Background load (browser, build, antivirus). | Close other workloads; rerun. Criterion's own warm-up will smooth most jitter. |

---

## 5. Where to next

- Brain architecture and scaling envelope:
  [docs/brain-advanced-design.md](brain-advanced-design.md)
- Memory store internals:
  [src-tauri/src/memory/](../src-tauri/src/memory/)
- Completed bench-related chunks (history):
  [rules/completion-log.md](../rules/completion-log.md)
- Project-wide coding rules:
  [rules/coding-standards.md](../rules/coding-standards.md)
