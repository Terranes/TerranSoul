//! Million-memory retrieval and capacity benchmark (Chunk 38.5).
//!
//! Smoke run (10k vectors, CI-friendly):
//! `cargo bench --bench million_memory`
//!
//! Full run (1M vectors, local/nightly):
//! `cargo bench --bench million_memory --features bench-million`
//!
//! The JSON report is written to `target/bench-results/million_memory.json`.
//! The benchmark measures the HNSW candidate stage used by vector-backed
//! hybrid retrieval. The pure linear backend is explicitly reported as skipped
//! at 1M because it is out of design for that scale.

use criterion::{black_box, BenchmarkId, Criterion};
use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::System;
use terransoul_lib::memory::eviction::{enforce_capacity, DEFAULT_TARGET_RATIO};
use terransoul_lib::memory::schema::create_canonical_schema;

/// Dimensionality matching TerranSoul's default nomic-embed-text.
const DIM: usize = 768;

const DEFAULT_SMOKE_SCALE: usize = 10_000;
const ONE_MILLION: usize = 1_000_000;
const QUERY_COUNT: usize = 1_000;
const TOP_K: usize = 10;
const VECTOR_SEED: u64 = 0x38_05_0000_0000_0001;
const QUERY_SEED: u64 = 0x38_05_0000_0000_0002;
const P50_TARGET_MS: f64 = 30.0;
const P95_TARGET_MS: f64 = 60.0;
const P99_TARGET_MS: f64 = 100.0;
const CAPACITY_TARGET_SECONDS: f64 = 30.0;

#[derive(Debug, Clone, Serialize)]
struct MachineSpec {
    os_name: Option<String>,
    os_version: Option<String>,
    kernel_version: Option<String>,
    cpu_brand: Option<String>,
    logical_cpus: usize,
    total_memory_gib: f64,
    available_memory_gib_at_start: f64,
    current_exe: String,
}

#[derive(Debug, Clone, Serialize)]
struct HnswReport {
    scale: usize,
    status: String,
    retrieval_kind: String,
    query_count: usize,
    top_k: usize,
    dimensions: usize,
    seed: u64,
    raw_vector_gib: f64,
    estimated_hnsw_gib: f64,
    available_memory_gib_before: f64,
    build_seconds: Option<f64>,
    vectors_per_second: Option<f64>,
    p50_ms: Option<f64>,
    p95_ms: Option<f64>,
    p99_ms: Option<f64>,
    max_ms: Option<f64>,
    retrieval_total_seconds: Option<f64>,
    failure: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct LinearBackendReport {
    scale: usize,
    status: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct CapacityReport {
    scale: usize,
    status: String,
    initial_long_count: u64,
    cap: u64,
    target_ratio: f64,
    expected_target: u64,
    elapsed_seconds: Option<f64>,
    long_count_before: Option<u64>,
    dropped: Option<u64>,
    long_count_after: Option<u64>,
    kept_protected: Option<u64>,
    kept_important: Option<u64>,
    protected_after: Option<u64>,
    important_after: Option<u64>,
    failure: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BenchReport {
    generated_at_unix_ms: u128,
    command: String,
    machine: MachineSpec,
    hnsw: Vec<HnswReport>,
    linear_backend: Vec<LinearBackendReport>,
    capacity: Vec<CapacityReport>,
}

fn fill_vector(rng: &mut Xoshiro256PlusPlus, out: &mut [f32]) {
    for value in out.iter_mut() {
        let unit = (rng.next_u32() as f32) / (u32::MAX as f32);
        *value = unit * 2.0 - 1.0;
    }
    let norm: f32 = out.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in out.iter_mut() {
            *value /= norm;
        }
    }
}

fn parse_scales() -> Vec<usize> {
    if let Ok(value) = env::var("TS_BENCH_SCALES") {
        let scales: Vec<usize> = value
            .split(',')
            .filter_map(|part| part.trim().parse::<usize>().ok())
            .collect();
        if !scales.is_empty() {
            return scales;
        }
    }
    if cfg!(feature = "bench-million") {
        vec![ONE_MILLION]
    } else {
        vec![DEFAULT_SMOKE_SCALE]
    }
}

fn bytes_to_gib(bytes: u128) -> f64 {
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}

fn raw_vector_bytes(scale: usize) -> u128 {
    scale as u128 * DIM as u128 * std::mem::size_of::<f32>() as u128
}

fn estimated_hnsw_bytes(scale: usize) -> u128 {
    let raw = raw_vector_bytes(scale);
    let graph_and_metadata = scale as u128 * 128;
    raw + graph_and_metadata + raw / 3
}

fn machine_spec(system: &System) -> MachineSpec {
    MachineSpec {
        os_name: System::name(),
        os_version: System::long_os_version(),
        kernel_version: System::kernel_version(),
        cpu_brand: system.cpus().first().map(|cpu| cpu.brand().to_string()),
        logical_cpus: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or_else(|_| system.cpus().len()),
        total_memory_gib: bytes_to_gib(system.total_memory() as u128),
        available_memory_gib_at_start: bytes_to_gib(system.available_memory() as u128),
        current_exe: env::current_exe()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| String::from("unknown")),
    }
}

fn should_skip_for_capacity(scale: usize, estimated_bytes: u128, available_memory: u64) -> bool {
    let force = env::var("TS_BENCH_FORCE_LARGE").ok().as_deref() == Some("1");
    !force && scale >= ONE_MILLION && estimated_bytes > (available_memory as u128 * 90 / 100)
}

fn generate_queries() -> Vec<Vec<f32>> {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(QUERY_SEED);
    (0..QUERY_COUNT)
        .map(|_| {
            let mut vector = vec![0.0; DIM];
            fill_vector(&mut rng, &mut vector);
            vector
        })
        .collect()
}

fn run_hnsw_scale(scale: usize, system: &mut System, criterion: &mut Criterion) -> HnswReport {
    system.refresh_memory();
    let available_memory = system.available_memory();
    let raw_bytes = raw_vector_bytes(scale);
    let estimated_bytes = estimated_hnsw_bytes(scale);

    let mut report = HnswReport {
        scale,
        status: String::from("started"),
        retrieval_kind: String::from("hnsw_vector_stage_for_hybrid_search"),
        query_count: QUERY_COUNT,
        top_k: TOP_K,
        dimensions: DIM,
        seed: VECTOR_SEED,
        raw_vector_gib: bytes_to_gib(raw_bytes),
        estimated_hnsw_gib: bytes_to_gib(estimated_bytes),
        available_memory_gib_before: bytes_to_gib(available_memory as u128),
        build_seconds: None,
        vectors_per_second: None,
        p50_ms: None,
        p95_ms: None,
        p99_ms: None,
        max_ms: None,
        retrieval_total_seconds: None,
        failure: None,
    };

    if should_skip_for_capacity(scale, estimated_bytes, available_memory) {
        report.status = String::from("skipped_capacity_gate");
        report.failure = Some(format!(
            "Estimated HNSW footprint {:.2} GiB exceeds 85% of available memory {:.2} GiB. Set TS_BENCH_FORCE_LARGE=1 to force the run.",
            report.estimated_hnsw_gib, report.available_memory_gib_before
        ));
        eprintln!(
            "[bench] scale={scale}: skipped capacity gate ({:.2} GiB estimated, {:.2} GiB available)",
            report.estimated_hnsw_gib, report.available_memory_gib_before
        );
        return report;
    }

    eprintln!("[bench] Building HNSW index with {scale} vectors (dim={DIM})...");
    let index = match terransoul_lib::memory::ann_index::AnnIndex::new(DIM) {
        Ok(index) => index,
        Err(error) => {
            report.status = String::from("failed_create_index");
            report.failure = Some(error);
            return report;
        }
    };

    eprintln!("[bench] scale={scale}: reserving index capacity...");
    let reserve_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    if let Err(error) = index.reserve_capacity_with_threads(scale, reserve_threads) {
        report.status = String::from("failed_reserve");
        report.failure = Some(error);
        return report;
    }
    eprintln!("[bench] scale={scale}: reserve complete; inserting vectors...");

    let start = Instant::now();
    let mut vector = vec![0.0; DIM];
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(VECTOR_SEED);
    for id in 1..=(scale as i64) {
        fill_vector(&mut rng, &mut vector);
        if let Err(error) = index.add(id, &vector) {
            report.status = String::from("failed_add");
            report.failure = Some(format!("id={id}: {error}"));
            return report;
        }
        if id % 100_000 == 0 {
            eprintln!("[bench] scale={scale}: indexed {id} vectors...");
        }
    }
    let insert_elapsed = start.elapsed();
    report.build_seconds = Some(insert_elapsed.as_secs_f64());
    report.vectors_per_second = Some(scale as f64 / insert_elapsed.as_secs_f64().max(f64::EPSILON));
    eprintln!(
        "[bench] Inserted {scale} vectors in {:.2}s ({:.0} vec/s)",
        insert_elapsed.as_secs_f64(),
        scale as f64 / insert_elapsed.as_secs_f64().max(f64::EPSILON)
    );

    if index.len() != scale {
        report.status = String::from("failed_len_check");
        report.failure = Some(format!("index length {} != {scale}", index.len()));
        return report;
    }

    let queries = generate_queries();
    let mut latencies: Vec<f64> = Vec::with_capacity(QUERY_COUNT);
    let retrieval_start = Instant::now();

    for (i, query) in queries.iter().enumerate() {
        let t = Instant::now();
        let results = match index.search(query, TOP_K) {
            Ok(results) => results,
            Err(error) => {
                report.status = String::from("failed_search");
                report.failure = Some(format!("query #{i}: {error}"));
                return report;
            }
        };
        if results.is_empty() {
            report.status = String::from("failed_empty_results");
            report.failure = Some(format!("query #{i}: empty results"));
            return report;
        }
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        latencies.push(ms);
    }
    report.retrieval_total_seconds = Some(retrieval_start.elapsed().as_secs_f64());
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p50 = percentile(&latencies, 0.50);
    let p95 = percentile(&latencies, 0.95);
    let p99 = percentile(&latencies, 0.99);
    let max = *latencies.last().unwrap_or(&0.0);
    report.p50_ms = Some(p50);
    report.p95_ms = Some(p95);
    report.p99_ms = Some(p99);
    report.max_ms = Some(max);

    eprintln!(
        "[bench] scale={scale}: p50={p50:.2}ms, p95={p95:.2}ms, p99={p99:.2}ms, max={max:.2}ms"
    );

    record_criterion_query_batch(criterion, scale, &index, &queries);

    if p50 > P50_TARGET_MS || p95 > P95_TARGET_MS || p99 > P99_TARGET_MS {
        report.status = String::from("failed_latency_threshold");
        report.failure = Some(format!(
            "latency threshold exceeded: p50={p50:.2}ms (target {P50_TARGET_MS:.0}), p95={p95:.2}ms (target {P95_TARGET_MS:.0}), p99={p99:.2}ms (target {P99_TARGET_MS:.0})"
        ));
        return report;
    }

    report.status = String::from("completed");
    report
}

fn percentile(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = ((sorted.len() as f64) * pct).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn record_criterion_query_batch(
    criterion: &mut Criterion,
    scale: usize,
    index: &terransoul_lib::memory::ann_index::AnnIndex,
    queries: &[Vec<f32>],
) {
    let mut group = criterion.benchmark_group("million_memory_hnsw_query");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(2));
    group.bench_with_input(BenchmarkId::from_parameter(scale), &scale, |bench, _| {
        let mut cursor = 0usize;
        bench.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let query = &queries[cursor % queries.len()];
                let results = index.search(black_box(query), black_box(TOP_K));
                black_box(results.expect("HNSW query should succeed"));
                cursor = cursor.wrapping_add(1);
            }
            start.elapsed()
        });
    });
    group.finish();
}

fn linear_backend_report(scale: usize) -> LinearBackendReport {
    if scale >= ONE_MILLION {
        LinearBackendReport {
            scale,
            status: String::from("skipped_at_million_scale"),
            reason: String::from(
                "The pure linear cosine backend is O(n) and intentionally out of design at 1M entries; use native-ann HNSW.",
            ),
        }
    } else {
        LinearBackendReport {
            scale,
            status: String::from("not_run_for_smoke"),
            reason: String::from(
                "Smoke benchmarks track the HNSW path used by desktop large-store builds.",
            ),
        }
    }
}

fn run_capacity_benchmark(scale: usize) -> CapacityReport {
    let cap = scale as u64;
    let initial_long_count = cap + cap / 20;
    let expected_target = (cap as f64 * DEFAULT_TARGET_RATIO) as u64;
    let protected_seed = (scale / 1_000).clamp(10, 1_000) as u64;
    let important_seed = (scale / 1_000).clamp(10, 1_000) as u64;

    let mut report = CapacityReport {
        scale,
        status: String::from("started"),
        initial_long_count,
        cap,
        target_ratio: DEFAULT_TARGET_RATIO,
        expected_target,
        elapsed_seconds: None,
        long_count_before: None,
        dropped: None,
        long_count_after: None,
        kept_protected: None,
        kept_important: None,
        protected_after: None,
        important_after: None,
        failure: None,
    };

    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => {
            report.status = String::from("failed_tempdir");
            report.failure = Some(error.to_string());
            return report;
        }
    };

    let mut conn = match Connection::open_in_memory() {
        Ok(conn) => conn,
        Err(error) => {
            report.status = String::from("failed_open_db");
            report.failure = Some(error.to_string());
            return report;
        }
    };

    if let Err(error) = create_canonical_schema(&conn) {
        report.status = String::from("failed_schema");
        report.failure = Some(error.to_string());
        return report;
    }

    let _ = conn.execute_batch(
        "PRAGMA journal_mode=OFF; PRAGMA synchronous=OFF; PRAGMA temp_store=MEMORY;",
    );

    if let Err(error) = seed_capacity_rows(
        &mut conn,
        initial_long_count,
        protected_seed,
        important_seed,
    ) {
        report.status = String::from("failed_seed_rows");
        report.failure = Some(error);
        return report;
    }

    eprintln!(
        "[bench] enforce_capacity scale={scale}: {initial_long_count} -> {expected_target} target..."
    );
    let start = Instant::now();
    let eviction = match enforce_capacity(&conn, cap, DEFAULT_TARGET_RATIO, dir.path()) {
        Ok(Some(eviction)) => eviction,
        Ok(None) => {
            report.status = String::from("failed_no_eviction");
            report.failure = Some(String::from("expected an eviction report"));
            return report;
        }
        Err(error) => {
            report.status = String::from("failed_enforce_capacity");
            report.failure = Some(error.to_string());
            return report;
        }
    };
    let elapsed = start.elapsed().as_secs_f64();

    let protected_after: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories WHERE tier='long' AND protected = 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let important_after: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories WHERE tier='long' AND importance >= 4",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    report.elapsed_seconds = Some(elapsed);
    report.long_count_before = Some(eviction.long_count_before);
    report.dropped = Some(eviction.dropped);
    report.long_count_after = Some(eviction.long_count_after);
    report.kept_protected = Some(eviction.kept_protected);
    report.kept_important = Some(eviction.kept_important);
    report.protected_after = Some(protected_after);
    report.important_after = Some(important_after);

    if eviction.long_count_before != initial_long_count {
        report.status = String::from("failed_before_count");
        report.failure = Some(format!(
            "before count {} != expected {initial_long_count}",
            eviction.long_count_before
        ));
        return report;
    }
    if eviction.long_count_after > expected_target {
        report.status = String::from("failed_target_count");
        report.failure = Some(format!(
            "after count {} > target {expected_target}",
            eviction.long_count_after
        ));
        return report;
    }
    if protected_after != protected_seed || important_after != important_seed {
        report.status = String::from("failed_preservation");
        report.failure = Some(format!(
            "protected/important after {protected_after}/{important_after} != expected {protected_seed}/{important_seed}"
        ));
        return report;
    }
    if scale >= ONE_MILLION && elapsed > CAPACITY_TARGET_SECONDS {
        report.status = String::from("failed_capacity_threshold");
        report.failure = Some(format!(
            "enforce_capacity took {elapsed:.2}s, exceeds {CAPACITY_TARGET_SECONDS:.0}s target"
        ));
        return report;
    }

    report.status = String::from("completed");
    report
}

fn seed_capacity_rows(
    conn: &mut Connection,
    count: u64,
    protected_seed: u64,
    important_seed: u64,
) -> Result<(), String> {
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    {
        let mut stmt = tx
            .prepare_cached(
                "INSERT INTO memories
                   (id, content, tags, importance, memory_type, created_at, last_accessed, tier, decay_score, token_count, protected)
                 VALUES
                   (?1, ?2, 'bench', ?3, 'fact', ?4, NULL, 'long', ?5, 8, ?6)",
            )
            .map_err(|error| error.to_string())?;
        for id in 1..=count {
            let protected = u64::from(id <= protected_seed) as i64;
            let importance = if id > protected_seed && id <= protected_seed + important_seed {
                4i64
            } else {
                2i64
            };
            let created_at = 1_800_000_000_000i64 - id as i64;
            let decay_score = 0.05f64 + ((id % 100) as f64 / 1_000.0);
            stmt.execute(params![
                id as i64,
                format!("bench memory {id}"),
                importance,
                created_at,
                decay_score,
                protected,
            ])
            .map_err(|error| error.to_string())?;
        }
    }
    tx.commit().map_err(|error| error.to_string())
}

fn report_path() -> PathBuf {
    env::var_os("TS_BENCH_OUTPUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("target"))
        .join("bench-results")
        .join("million_memory.json")
}

fn write_report(report: &BenchReport) -> Result<(), String> {
    let target_path = report_path();
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
    fs::write(&target_path, &json).map_err(|e| e.to_string())?;
    eprintln!("[bench] wrote {}", target_path.display());
    Ok(())
}

fn main() {
    let command = env::args().collect::<Vec<_>>().join(" ");
    let mut system = System::new_all();
    system.refresh_all();
    let mut criterion = Criterion::default().configure_from_args();
    let mut report = BenchReport {
        generated_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default(),
        command,
        machine: machine_spec(&system),
        hnsw: Vec::new(),
        linear_backend: Vec::new(),
        capacity: Vec::new(),
    };

    let mut failed = false;
    for scale in parse_scales() {
        report.linear_backend.push(linear_backend_report(scale));

        let hnsw_report = run_hnsw_scale(scale, &mut system, &mut criterion);
        failed |= hnsw_report.status.starts_with("failed");
        report.hnsw.push(hnsw_report);

        let capacity_report = run_capacity_benchmark(scale);
        failed |= capacity_report.status.starts_with("failed");
        report.capacity.push(capacity_report);

        write_report(&report).expect("write benchmark report");
    }

    criterion.final_summary();
    assert!(!failed, "million_memory benchmark failed; see JSON report");
}
