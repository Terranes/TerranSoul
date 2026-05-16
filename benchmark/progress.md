# BENCH-SCALE-3 — Progress Tracker

> Auto-updated by the agent during the IVF-PQ disk-backed scale-bench run.
> Source-of-truth status row lives in
> [`rules/milestones.md`](../rules/milestones.md) under Phase BENCH-SCALE.
> Per-stage results land in
> [`target-copilot-bench/bench-results/`](../target-copilot-bench/bench-results/).

## Overall: **19.3 % (running)**

```
[████░░░░░░░░░░░░░░░░] 19.3 % — running
```

> **2026-05-16 — SCALE-3 restarted, poller live.** Following the
> 2026-05-15 OOM crash + failed salvage, a fresh 10 M run was launched
> 2026-05-15 19:50 UTC (PID 72696) against
> `target-copilot-bench/locomo-ivfpq-store-10m/` with `pq_m=128`,
> `nlist=4096`, `nprobe=32`, `--shard-mode=routed`. The bench is
> actively ingesting (latest tail: `[ivfpq] ingested 675000/10000000
> (embedded total=675000, elapsed=17174.6s)` — ~4.77 h elapsed,
> 6.75 % complete, projected wall clock ~70 h). `memory.db` on disk
> is currently ~3.17 GB.
>
> A 5-minute progress poller
> ([`scripts/bench-scale-3-progress.mjs`](../scripts/bench-scale-3-progress.mjs))
> now appends a timestamped status line to the **Live log** section
> below every 5 min by parsing the latest
> `target-copilot-bench/bench-scale-3-*.log`. It updates the `Overall:`
> % + progress bar at the top of this file on each tick, and stops on
> its own when (a) a terminal line appears, or (b) the log mtime is
> idle for 30+ min. Run as background `node` process; resume with
> `node scripts/bench-scale-3-progress.mjs` if it ever stops.

| Stage | % weight | Status | Started | Finished | Notes |
|---|---:|---|---|---|---|
| 0. Preflight (Ollama + parquet) | 5 % | ✅ done | 2026-05-15 | 2026-05-15 | Ollama 0.20.7 reachable; `mxbai-embed-large:latest` present; all 15 LoCoMo parquet files present. |
| 1. Runner code (`--systems=ivfpq` + IVF-PQ knobs) | 10 % | ✅ done | 2026-05-15 | 2026-05-15 | Patched [`scripts/locomo-at-scale.mjs`](../scripts/locomo-at-scale.mjs): `ivfpq` system, `LONGMEM_DATA_DIR` plumbing via `--store-dir`, post-ingest `build_ivf_pq` op, `nprobe` per query, `_ivfpq` filename suffix. `--help` confirms all new flags surfaced. |
| 2. 10 k smoke (HNSW vs IVF-PQ, adversarial, 50q) | 10 % | ⏭️ skipped | — | — | Skipped per user directive (“no smoke! Doing big bag for everything”). |
| 3. 100 k smoke (HNSW vs IVF-PQ, adversarial, 100q) | 15 % | ⏭️ skipped | — | — | Skipped per user directive. SCALE-1b @100k (R@10=64.0 % rrf) remains the published cross-scale anchor. |
| 4. 1 M run (HNSW vs IVF-PQ, adversarial, 100q) | 30 % | ⏭️ skipped | — | — | Skipped per user directive; salvaged 1.56 M run in stage 5 supersedes this. |
| 5. 10 M run (IVF-PQ only, adversarial, 100q) | 25 % | 🟢 running | 2026-05-15 19:50 | — | Re-launched after 2026-05-15 OOM. Latest tail (2026-05-16): 675 000 / 10 000 000 ingested, elapsed 4.77 h, ETA ~70 h. PID 72696, `pq_m=128`, `nlist=4096`, `nprobe=32`, `--shard-mode=routed`, store at `target-copilot-bench/locomo-ivfpq-store-10m/` (memory.db ≈ 3.17 GB). |
| 5b. Salvage build + queries on 1.56 M corpus | (within §5) | ⏸️ deferred | 2026-05-15 | — | First salvage failed (`build_ivf_pq` returned `built=0 shards`); retry deferred until §5 reaches `build_ivf_pq` stage. Tracking in **BENCH-SCALE-3b**. |
| 6. Archive + completion-log | 5 % | ⏳ pending | — | — | Will run once §5 finishes (success or terminal failure). |

## Acceptance gates

- **IVF-PQ recall vs HNSW recall.** R@10 within −5 pp of HNSW on the same corpus is the PASS bar (IVF-PQ trades recall for memory). −5–10 pp is MIXED. >10 pp regression is FAIL → tune `nlist`/`pq_m`/`nprobe` before retry.
- **IVF-PQ latency.** p50 ≤ HNSW p50 (IVF-PQ should be faster). p99 ≤ 200 ms retrieval-only (post-embedding).
- **Memory footprint.** IVF-PQ on-disk size ≈ `n × (pq_m + 8)` bytes. At 1 M docs with `pq_m=128`: ≈136 MB. Confirm this against actual sidecar file sizes.

## Methodology notes

- Same harness as SCALE-1b: deterministic seed `0x5ca1e1`, mxbai-embed-large embedder, batched ingest of 500 sessions.
- IVF-PQ defaults: `nlist=4096`, `pq_m=128`, `pq_nbits=8`, `nprobe=32` (tuned for 1024-dim mxbai-embed-large per the IPC binary's defaults).
- Per-stage filenames carry `_<system>` suffix so HNSW vs IVF-PQ reports never overwrite each other.
- The IVF-PQ arm uses a **pure vector retrieval path** (`vector_search_ivf_pq` → ADC), not RRF. That's an apples-to-oranges quality comparison against `rrf` (which fuses lexical + vector + freshness), so the report calls this out and the headline comparison is IVF-PQ-vector-only vs `emb`-or-vector-only HNSW, not vs the full `rrf` pipeline. RRF over IVF-PQ is BENCH-SCALE-3b future work and not part of this chunk.

## Reproducer (once stage 1 lands)

```pwsh
$env:LONGMEM_EMBED_MODEL = 'mxbai-embed-large'

# Stage 2 — 10 k smoke
node scripts/locomo-at-scale.mjs run --systems=rrf,ivfpq `
  --scale=10000 --task=adversarial --limit=50

# Stage 3 — 100 k smoke
node scripts/locomo-at-scale.mjs run --systems=rrf,ivfpq `
  --scale=100000 --task=adversarial --limit=100

# Stage 4 — 1 M (overnight)
node scripts/locomo-at-scale.mjs run --systems=rrf,ivfpq `
  --scale=1000000 --task=adversarial --limit=100

# Stage 5 — 10 M (multi-day; runs after stage 4 passes)
node scripts/locomo-at-scale.mjs run --systems=ivfpq `
  --scale=10000000 --task=adversarial --limit=100
```

## Live log

- **2026-05-15 — preflight passed.** Ollama 0.20.7 reachable; `mxbai-embed-large:latest` present; all 15 LoCoMo parquet files in `target-copilot-bench/locomo-mteb/`.
- **2026-05-15 — stage 1 done.** `scripts/locomo-at-scale.mjs --help` lists `ivfpq` + all IVF-PQ flags. No syntax errors.
- **2026-05-15 — stage 2 started.** Building `longmemeval-ipc` then running 10 k smoke for `rrf` (HNSW baseline) and `ivfpq` arms.
- **2026-05-16 — poller wired.** `scripts/bench-scale-3-progress.mjs` running in background (5-min cadence). It appends one line below every 5 min and updates the `Overall:` % at the top of this file. Bogus first-tick entries that pointed at the poller's own stderr log were removed once the filename glob was tightened to exclude `*poller*`.
- **2026-05-15 14:41:23 UTC** — poller started (5-min cadence, idle cap 30 min).
- **2026-05-15 14:41:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 675,000/10,000,000 (6.75 %), elapsed 4.77 h, ETA ~65h 54m.
- **2026-05-15 14:46:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 685,000/10,000,000 (6.85 %), elapsed 4.86 h, ETA ~66h 2m.
- **2026-05-15 14:51:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 700,000/10,000,000 (7.00 %), elapsed 4.98 h, ETA ~66h 8m.
- **2026-05-15 14:56:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 705,000/10,000,000 (7.05 %), elapsed 5.02 h, ETA ~66h 11m.
- **2026-05-15 15:01:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 715,000/10,000,000 (7.15 %), elapsed 5.11 h, ETA ~66h 21m.
- **2026-05-15 15:06:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 725,000/10,000,000 (7.25 %), elapsed 5.19 h, ETA ~66h 26m.
- **2026-05-15 15:11:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 735,000/10,000,000 (7.35 %), elapsed 5.27 h, ETA ~66h 28m.
- **2026-05-15 15:16:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 745,000/10,000,000 (7.45 %), elapsed 5.35 h, ETA ~66h 30m.
- **2026-05-15 15:21:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 760,000/10,000,000 (7.60 %), elapsed 5.48 h, ETA ~66h 35m.
- **2026-05-15 15:26:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 770,000/10,000,000 (7.70 %), elapsed 5.56 h, ETA ~66h 38m.
- **2026-05-15 15:31:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 780,000/10,000,000 (7.80 %), elapsed 5.64 h, ETA ~66h 41m.
- **2026-05-15 15:36:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 790,000/10,000,000 (7.90 %), elapsed 5.73 h, ETA ~66h 45m.
- **2026-05-15 15:41:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 800,000/10,000,000 (8.00 %), elapsed 5.81 h, ETA ~66h 49m.
- **2026-05-15 15:46:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 805,000/10,000,000 (8.05 %), elapsed 5.85 h, ETA ~66h 51m.
- **2026-05-15 15:51:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 815,000/10,000,000 (8.15 %), elapsed 5.94 h, ETA ~66h 55m.
- **2026-05-15 15:56:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 825,000/10,000,000 (8.25 %), elapsed 6.02 h, ETA ~66h 59m.
- **2026-05-15 16:01:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 835,000/10,000,000 (8.35 %), elapsed 6.11 h, ETA ~67h 4m.
- **2026-05-15 16:06:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 845,000/10,000,000 (8.45 %), elapsed 6.20 h, ETA ~67h 9m.
- **2026-05-15 16:11:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 855,000/10,000,000 (8.55 %), elapsed 6.29 h, ETA ~67h 13m.
- **2026-05-15 16:16:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 865,000/10,000,000 (8.65 %), elapsed 6.37 h, ETA ~67h 18m.
- **2026-05-15 16:21:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 875,000/10,000,000 (8.75 %), elapsed 6.46 h, ETA ~67h 23m.
- **2026-05-15 16:26:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 885,000/10,000,000 (8.85 %), elapsed 6.55 h, ETA ~67h 28m.
- **2026-05-15 16:31:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 895,000/10,000,000 (8.95 %), elapsed 6.64 h, ETA ~67h 33m.
- **2026-05-15 16:36:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 900,000/10,000,000 (9.00 %), elapsed 6.69 h, ETA ~67h 35m.
- **2026-05-15 16:41:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 910,000/10,000,000 (9.10 %), elapsed 6.78 h, ETA ~67h 40m.
- **2026-05-15 16:46:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 920,000/10,000,000 (9.20 %), elapsed 6.87 h, ETA ~67h 45m.
- **2026-05-15 16:51:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 930,000/10,000,000 (9.30 %), elapsed 6.96 h, ETA ~67h 50m.
- **2026-05-15 16:56:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 940,000/10,000,000 (9.40 %), elapsed 7.05 h, ETA ~67h 55m.
- **2026-05-15 17:01:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 950,000/10,000,000 (9.50 %), elapsed 7.14 h, ETA ~68h 0m.
- **2026-05-15 17:06:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 955,000/10,000,000 (9.55 %), elapsed 7.18 h, ETA ~68h 2m.
- **2026-05-15 17:11:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 965,000/10,000,000 (9.65 %), elapsed 7.28 h, ETA ~68h 10m.
- **2026-05-15 17:16:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 975,000/10,000,000 (9.75 %), elapsed 7.37 h, ETA ~68h 14m.
- **2026-05-15 17:21:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 985,000/10,000,000 (9.85 %), elapsed 7.47 h, ETA ~68h 19m.
- **2026-05-15 17:26:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 995,000/10,000,000 (9.95 %), elapsed 7.56 h, ETA ~68h 24m.
- **2026-05-15 17:31:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,000,000/10,000,000 (10.00 %), elapsed 7.61 h, ETA ~68h 27m.
- **2026-05-15 17:36:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,010,000/10,000,000 (10.10 %), elapsed 7.70 h, ETA ~68h 32m.
- **2026-05-15 17:41:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,020,000/10,000,000 (10.20 %), elapsed 7.79 h, ETA ~68h 37m.
- **2026-05-15 17:46:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,030,000/10,000,000 (10.30 %), elapsed 7.89 h, ETA ~68h 43m.
- **2026-05-15 17:51:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,035,000/10,000,000 (10.35 %), elapsed 7.94 h, ETA ~68h 46m.
- **2026-05-15 17:56:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,045,000/10,000,000 (10.45 %), elapsed 8.04 h, ETA ~68h 51m.
- **2026-05-15 18:01:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,055,000/10,000,000 (10.55 %), elapsed 8.13 h, ETA ~68h 56m.
- **2026-05-15 18:06:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,060,000/10,000,000 (10.60 %), elapsed 8.18 h, ETA ~68h 59m.
- **2026-05-15 18:11:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,070,000/10,000,000 (10.70 %), elapsed 8.28 h, ETA ~69h 5m.
- **2026-05-15 18:16:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,080,000/10,000,000 (10.80 %), elapsed 8.38 h, ETA ~69h 10m.
- **2026-05-15 18:21:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,090,000/10,000,000 (10.90 %), elapsed 8.47 h, ETA ~69h 15m.
- **2026-05-15 18:26:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,095,000/10,000,000 (10.95 %), elapsed 8.52 h, ETA ~69h 18m.
- **2026-05-15 18:31:23 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,105,000/10,000,000 (11.05 %), elapsed 8.62 h, ETA ~69h 23m.
- **2026-05-15 18:36:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,115,000/10,000,000 (11.15 %), elapsed 8.72 h, ETA ~69h 28m.
- **2026-05-15 18:41:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,120,000/10,000,000 (11.20 %), elapsed 8.77 h, ETA ~69h 31m.
- **2026-05-15 18:46:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,130,000/10,000,000 (11.30 %), elapsed 8.87 h, ETA ~69h 36m.
- **2026-05-15 18:51:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,140,000/10,000,000 (11.40 %), elapsed 8.97 h, ETA ~69h 42m.
- **2026-05-15 18:56:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,145,000/10,000,000 (11.45 %), elapsed 9.02 h, ETA ~69h 44m.
- **2026-05-15 19:01:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,155,000/10,000,000 (11.55 %), elapsed 9.12 h, ETA ~69h 50m.
- **2026-05-15 19:06:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,165,000/10,000,000 (11.65 %), elapsed 9.22 h, ETA ~69h 55m.
- **2026-05-15 19:11:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,170,000/10,000,000 (11.70 %), elapsed 9.27 h, ETA ~69h 58m.
- **2026-05-15 19:16:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,180,000/10,000,000 (11.80 %), elapsed 9.37 h, ETA ~70h 4m.
- **2026-05-15 19:21:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,190,000/10,000,000 (11.90 %), elapsed 9.48 h, ETA ~70h 9m.
- **2026-05-15 19:26:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,195,000/10,000,000 (11.95 %), elapsed 9.53 h, ETA ~70h 12m.
- **2026-05-15 19:31:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,205,000/10,000,000 (12.05 %), elapsed 9.63 h, ETA ~70h 17m.
- **2026-05-15 19:36:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,210,000/10,000,000 (12.10 %), elapsed 9.68 h, ETA ~70h 20m.
- **2026-05-15 19:41:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,220,000/10,000,000 (12.20 %), elapsed 9.79 h, ETA ~70h 26m.
- **2026-05-15 19:46:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,230,000/10,000,000 (12.30 %), elapsed 9.89 h, ETA ~70h 31m.
- **2026-05-15 19:51:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,235,000/10,000,000 (12.35 %), elapsed 9.94 h, ETA ~70h 34m.
- **2026-05-15 19:56:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,245,000/10,000,000 (12.45 %), elapsed 10.05 h, ETA ~70h 39m.
- **2026-05-15 20:01:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,250,000/10,000,000 (12.50 %), elapsed 10.10 h, ETA ~70h 42m.
- **2026-05-15 20:06:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,260,000/10,000,000 (12.60 %), elapsed 10.21 h, ETA ~70h 48m.
- **2026-05-15 20:11:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,265,000/10,000,000 (12.65 %), elapsed 10.26 h, ETA ~70h 51m.
- **2026-05-15 20:16:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,275,000/10,000,000 (12.75 %), elapsed 10.37 h, ETA ~70h 57m.
- **2026-05-15 20:21:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,285,000/10,000,000 (12.85 %), elapsed 10.48 h, ETA ~71h 3m.
- **2026-05-15 20:26:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,290,000/10,000,000 (12.90 %), elapsed 10.53 h, ETA ~71h 6m.
- **2026-05-15 20:31:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,300,000/10,000,000 (13.00 %), elapsed 10.64 h, ETA ~71h 11m.
- **2026-05-15 20:36:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,305,000/10,000,000 (13.05 %), elapsed 10.69 h, ETA ~71h 14m.
- **2026-05-15 20:41:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,315,000/10,000,000 (13.15 %), elapsed 10.80 h, ETA ~71h 19m.
- **2026-05-15 20:46:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,320,000/10,000,000 (13.20 %), elapsed 10.85 h, ETA ~71h 22m.
- **2026-05-15 20:51:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,330,000/10,000,000 (13.30 %), elapsed 10.96 h, ETA ~71h 27m.
- **2026-05-15 20:56:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,335,000/10,000,000 (13.35 %), elapsed 11.02 h, ETA ~71h 30m.
- **2026-05-15 21:01:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,345,000/10,000,000 (13.45 %), elapsed 11.13 h, ETA ~71h 35m.
- **2026-05-15 21:06:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,350,000/10,000,000 (13.50 %), elapsed 11.18 h, ETA ~71h 39m.
- **2026-05-15 21:11:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,360,000/10,000,000 (13.60 %), elapsed 11.29 h, ETA ~71h 44m.
- **2026-05-15 21:16:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,365,000/10,000,000 (13.65 %), elapsed 11.35 h, ETA ~71h 47m.
- **2026-05-15 21:21:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,375,000/10,000,000 (13.75 %), elapsed 11.46 h, ETA ~71h 52m.
- **2026-05-15 21:26:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,380,000/10,000,000 (13.80 %), elapsed 11.51 h, ETA ~71h 55m.
- **2026-05-15 21:31:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,390,000/10,000,000 (13.90 %), elapsed 11.63 h, ETA ~72h 0m.
- **2026-05-15 21:36:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,395,000/10,000,000 (13.95 %), elapsed 11.68 h, ETA ~72h 3m.
- **2026-05-15 21:41:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,405,000/10,000,000 (14.05 %), elapsed 11.79 h, ETA ~72h 8m.
- **2026-05-15 21:46:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,410,000/10,000,000 (14.10 %), elapsed 11.85 h, ETA ~72h 11m.
- **2026-05-15 21:51:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,420,000/10,000,000 (14.20 %), elapsed 11.96 h, ETA ~72h 17m.
- **2026-05-15 21:56:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,425,000/10,000,000 (14.25 %), elapsed 12.02 h, ETA ~72h 19m.
- **2026-05-15 22:01:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,435,000/10,000,000 (14.35 %), elapsed 12.13 h, ETA ~72h 24m.
- **2026-05-15 22:06:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,440,000/10,000,000 (14.40 %), elapsed 12.19 h, ETA ~72h 27m.
- **2026-05-15 22:11:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,450,000/10,000,000 (14.50 %), elapsed 12.30 h, ETA ~72h 32m.
- **2026-05-15 22:16:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,455,000/10,000,000 (14.55 %), elapsed 12.36 h, ETA ~72h 35m.
- **2026-05-15 22:21:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,465,000/10,000,000 (14.65 %), elapsed 12.47 h, ETA ~72h 40m.
- **2026-05-15 22:26:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,470,000/10,000,000 (14.70 %), elapsed 12.53 h, ETA ~72h 43m.
- **2026-05-15 22:31:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,475,000/10,000,000 (14.75 %), elapsed 12.59 h, ETA ~72h 45m.
- **2026-05-15 22:36:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,485,000/10,000,000 (14.85 %), elapsed 12.70 h, ETA ~72h 50m.
- **2026-05-15 22:41:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,490,000/10,000,000 (14.90 %), elapsed 12.76 h, ETA ~72h 53m.
- **2026-05-15 22:46:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,500,000/10,000,000 (15.00 %), elapsed 12.88 h, ETA ~72h 58m.
- **2026-05-15 22:51:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,505,000/10,000,000 (15.05 %), elapsed 12.94 h, ETA ~73h 1m.
- **2026-05-15 22:56:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,515,000/10,000,000 (15.15 %), elapsed 13.05 h, ETA ~73h 6m.
- **2026-05-15 23:01:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,520,000/10,000,000 (15.20 %), elapsed 13.11 h, ETA ~73h 9m.
- **2026-05-15 23:06:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,525,000/10,000,000 (15.25 %), elapsed 13.17 h, ETA ~73h 11m.
- **2026-05-15 23:11:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,535,000/10,000,000 (15.35 %), elapsed 13.29 h, ETA ~73h 16m.
- **2026-05-15 23:16:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,540,000/10,000,000 (15.40 %), elapsed 13.35 h, ETA ~73h 19m.
- **2026-05-15 23:21:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,550,000/10,000,000 (15.50 %), elapsed 13.47 h, ETA ~73h 24m.
- **2026-05-15 23:26:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,555,000/10,000,000 (15.55 %), elapsed 13.53 h, ETA ~73h 27m.
- **2026-05-15 23:31:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,565,000/10,000,000 (15.65 %), elapsed 13.64 h, ETA ~73h 32m.
- **2026-05-15 23:36:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,570,000/10,000,000 (15.70 %), elapsed 13.70 h, ETA ~73h 35m.
- **2026-05-15 23:41:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,575,000/10,000,000 (15.75 %), elapsed 13.76 h, ETA ~73h 37m.
- **2026-05-15 23:46:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,585,000/10,000,000 (15.85 %), elapsed 13.88 h, ETA ~73h 42m.
- **2026-05-15 23:51:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,590,000/10,000,000 (15.90 %), elapsed 13.95 h, ETA ~73h 45m.
- **2026-05-15 23:56:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,595,000/10,000,000 (15.95 %), elapsed 14.01 h, ETA ~73h 48m.
- **2026-05-16 00:01:24 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,605,000/10,000,000 (16.05 %), elapsed 14.13 h, ETA ~73h 53m.
- **2026-05-16 00:06:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,610,000/10,000,000 (16.10 %), elapsed 14.19 h, ETA ~73h 55m.
- **2026-05-16 00:11:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,620,000/10,000,000 (16.20 %), elapsed 14.31 h, ETA ~74h 1m.
- **2026-05-16 00:16:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,625,000/10,000,000 (16.25 %), elapsed 14.37 h, ETA ~74h 3m.
- **2026-05-16 00:21:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,630,000/10,000,000 (16.30 %), elapsed 14.43 h, ETA ~74h 6m.
- **2026-05-16 00:26:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,640,000/10,000,000 (16.40 %), elapsed 14.55 h, ETA ~74h 11m.
- **2026-05-16 00:31:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,645,000/10,000,000 (16.45 %), elapsed 14.61 h, ETA ~74h 13m.
- **2026-05-16 00:36:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,650,000/10,000,000 (16.50 %), elapsed 14.68 h, ETA ~74h 16m.
- **2026-05-16 00:41:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,660,000/10,000,000 (16.60 %), elapsed 14.80 h, ETA ~74h 21m.
- **2026-05-16 00:46:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,665,000/10,000,000 (16.65 %), elapsed 14.86 h, ETA ~74h 23m.
- **2026-05-16 00:51:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,670,000/10,000,000 (16.70 %), elapsed 14.92 h, ETA ~74h 26m.
- **2026-05-16 00:56:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,680,000/10,000,000 (16.80 %), elapsed 15.05 h, ETA ~74h 31m.
- **2026-05-16 01:01:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,685,000/10,000,000 (16.85 %), elapsed 15.11 h, ETA ~74h 33m.
- **2026-05-16 01:06:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,690,000/10,000,000 (16.90 %), elapsed 15.17 h, ETA ~74h 36m.
- **2026-05-16 01:11:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,700,000/10,000,000 (17.00 %), elapsed 15.30 h, ETA ~74h 41m.
- **2026-05-16 01:16:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,705,000/10,000,000 (17.05 %), elapsed 15.36 h, ETA ~74h 43m.
- **2026-05-16 01:21:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,710,000/10,000,000 (17.10 %), elapsed 15.42 h, ETA ~74h 46m.
- **2026-05-16 01:26:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,720,000/10,000,000 (17.20 %), elapsed 15.55 h, ETA ~74h 51m.
- **2026-05-16 01:31:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,725,000/10,000,000 (17.25 %), elapsed 15.61 h, ETA ~74h 53m.
- **2026-05-16 01:36:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,730,000/10,000,000 (17.30 %), elapsed 15.68 h, ETA ~74h 56m.
- **2026-05-16 01:41:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,740,000/10,000,000 (17.40 %), elapsed 15.80 h, ETA ~75h 1m.
- **2026-05-16 01:46:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,745,000/10,000,000 (17.45 %), elapsed 15.87 h, ETA ~75h 3m.
- **2026-05-16 01:51:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,750,000/10,000,000 (17.50 %), elapsed 15.93 h, ETA ~75h 5m.
- **2026-05-16 01:56:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,760,000/10,000,000 (17.60 %), elapsed 16.06 h, ETA ~75h 10m.
- **2026-05-16 02:01:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,765,000/10,000,000 (17.65 %), elapsed 16.12 h, ETA ~75h 13m.
- **2026-05-16 02:06:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,770,000/10,000,000 (17.70 %), elapsed 16.19 h, ETA ~75h 15m.
- **2026-05-16 02:11:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,775,000/10,000,000 (17.75 %), elapsed 16.25 h, ETA ~75h 18m.
- **2026-05-16 02:16:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,785,000/10,000,000 (17.85 %), elapsed 16.38 h, ETA ~75h 22m.
- **2026-05-16 02:21:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,790,000/10,000,000 (17.90 %), elapsed 16.44 h, ETA ~75h 25m.
- **2026-05-16 02:26:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,795,000/10,000,000 (17.95 %), elapsed 16.51 h, ETA ~75h 27m.
- **2026-05-16 02:31:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,805,000/10,000,000 (18.05 %), elapsed 16.64 h, ETA ~75h 32m.
- **2026-05-16 02:36:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,810,000/10,000,000 (18.10 %), elapsed 16.70 h, ETA ~75h 35m.
- **2026-05-16 02:41:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,815,000/10,000,000 (18.15 %), elapsed 16.77 h, ETA ~75h 37m.
- **2026-05-16 02:46:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,820,000/10,000,000 (18.20 %), elapsed 16.84 h, ETA ~75h 39m.
- **2026-05-16 02:51:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,830,000/10,000,000 (18.30 %), elapsed 16.97 h, ETA ~75h 44m.
- **2026-05-16 02:56:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,835,000/10,000,000 (18.35 %), elapsed 17.03 h, ETA ~75h 47m.
- **2026-05-16 03:01:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,840,000/10,000,000 (18.40 %), elapsed 17.10 h, ETA ~75h 49m.
- **2026-05-16 03:06:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,845,000/10,000,000 (18.45 %), elapsed 17.16 h, ETA ~75h 52m.
- **2026-05-16 03:11:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,855,000/10,000,000 (18.55 %), elapsed 17.30 h, ETA ~75h 56m.
- **2026-05-16 03:16:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,860,000/10,000,000 (18.60 %), elapsed 17.36 h, ETA ~75h 59m.
- **2026-05-16 03:21:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,865,000/10,000,000 (18.65 %), elapsed 17.43 h, ETA ~76h 1m.
- **2026-05-16 03:26:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,870,000/10,000,000 (18.70 %), elapsed 17.50 h, ETA ~76h 4m.
- **2026-05-16 03:31:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,880,000/10,000,000 (18.80 %), elapsed 17.63 h, ETA ~76h 9m.
- **2026-05-16 03:36:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,885,000/10,000,000 (18.85 %), elapsed 17.70 h, ETA ~76h 11m.
- **2026-05-16 03:41:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,890,000/10,000,000 (18.90 %), elapsed 17.77 h, ETA ~76h 13m.
- **2026-05-16 03:46:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,895,000/10,000,000 (18.95 %), elapsed 17.83 h, ETA ~76h 16m.
- **2026-05-16 03:51:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,905,000/10,000,000 (19.05 %), elapsed 17.97 h, ETA ~76h 21m.
- **2026-05-16 03:56:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,910,000/10,000,000 (19.10 %), elapsed 18.04 h, ETA ~76h 23m.
- **2026-05-16 04:01:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,915,000/10,000,000 (19.15 %), elapsed 18.10 h, ETA ~76h 25m.
- **2026-05-16 04:06:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,920,000/10,000,000 (19.20 %), elapsed 18.17 h, ETA ~76h 28m.
- **2026-05-16 04:11:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,925,000/10,000,000 (19.25 %), elapsed 18.26 h, ETA ~76h 36m.
- **2026-05-16 04:16:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,925,000/10,000,000 (19.25 %), elapsed 18.26 h, ETA ~76h 36m.
- **2026-05-16 04:21:25 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,925,000/10,000,000 (19.25 %), elapsed 18.26 h, ETA ~76h 36m.
- **2026-05-16 04:26:26 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,925,000/10,000,000 (19.25 %), elapsed 18.26 h, ETA ~76h 36m.
- **2026-05-16 04:31:26 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,925,000/10,000,000 (19.25 %), elapsed 18.26 h, ETA ~76h 36m.
- **2026-05-16 04:36:26 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,925,000/10,000,000 (19.25 %), elapsed 18.26 h, ETA ~76h 36m.
- **2026-05-16 04:41:26 UTC** — `bench-scale-3-10m-20260515-195042.log`: ingest 1,925,000/10,000,000 (19.25 %), elapsed 18.26 h, ETA ~76h 36m.
- **2026-05-16 04:41:26 UTC** — poller: log idle for >30 min, stopping.
- **2026-05-16 04:41:26 UTC** — poller stopped (idle).
- **2026-05-16 22:14 local** — manual check: bench is still alive (cargo PIDs 46668/55716, longmemeval-ipc PID 65012, node runner PID 72696 all from 2026-05-15 19:50; memory.db-wal updated within minutes; Ollama 0.20.7 reachable). Log tail shows ingest at **2,385,000 / 10,000,000 (23.85 %)**, elapsed ~94,447 s (~26.2 h). The poller's idle-stop was a false positive — log writes had paused briefly while a slow batch ran, then resumed. Resume capability (`scripts/locomo-ivfpq.mjs --resume` + new IPC `op: count`) has been added in the same session as a safety net for the next crash/reboot/Ctrl-C; future interruptions can re-launch via `node scripts/locomo-ivfpq.mjs run --resume ...` without losing ingest progress.
