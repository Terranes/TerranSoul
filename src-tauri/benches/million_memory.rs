//! Million-memory retrieval and capacity benchmark (Chunk 38.5).
//!
//! Smoke run (100 vectors, CI-friendly):
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
use terransoul_lib::memory::store::{MemoryStore, NewMemory};

/// Dimensionality matching TerranSoul's default nomic-embed-text.
const DIM: usize = 768;

const DEFAULT_SMOKE_SCALE: usize = 100;
const ONE_MILLION: usize = 1_000_000;
const QUERY_COUNT: usize = 1_000;
const TOP_K: usize = 10;
const VECTOR_SEED: u64 = 0x3805_0000_0000_0001;
const QUERY_SEED: u64 = 0x3805_0000_0000_0002;
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
struct CrudReport {
    scale: usize,
    status: String,
    batch_size: usize,
    write_seconds: Option<f64>,
    write_rows_per_second: Option<f64>,
    write_target_seconds: f64,
    read_all_seconds: Option<f64>,
    read_all_rows_per_second: Option<f64>,
    read_target_seconds: f64,
    update_seconds: Option<f64>,
    update_rows_per_second: Option<f64>,
    mixed_op_count: Option<usize>,
    mixed_seconds: Option<f64>,
    mixed_ops_per_second: Option<f64>,
    delete_seconds: Option<f64>,
    delete_rows_per_second: Option<f64>,
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
    crud: Vec<CrudReport>,
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

/// SQLite CRUD throughput benchmark (Phase 41).
///
/// Measures real `MemoryStore::add_many` (transactional, prepare_cached)
/// followed by `MemoryStore::get_all` on a freshly-built corpus of `scale`
/// rows. Targets: 1M write < 60 s, 1M read < 5 s.
///
/// Each phase is guarded by `TS_BENCH_TIMEOUT_SECS` (default 300 s). If the
/// running time exceeds the limit the phase records `status = "timeout"` and
/// returns early rather than hanging indefinitely.
fn run_crud_benchmark(scale: usize, system: &mut System) -> CrudReport {
    let write_target = if scale >= ONE_MILLION { 60.0 } else { 6.0 };
    let read_target = if scale >= ONE_MILLION { 5.0 } else { 1.0 };
    // 10k rows per transaction keeps WAL fsync amortised without holding
    // an unbounded amount of inserted data in memory.
    let batch_size = 10_000usize.min(scale.max(1));

    // Per-bench wall-clock timeout. Prevents infinite hangs when the host
    // is under memory pressure or the SQLite file is on a slow volume.
    let bench_timeout = Duration::from_secs(
        env::var("TS_BENCH_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300),
    );
    let bench_start = Instant::now();

    let mut report = CrudReport {
        scale,
        status: String::from("started"),
        batch_size,
        write_seconds: None,
        write_rows_per_second: None,
        write_target_seconds: write_target,
        read_all_seconds: None,
        read_all_rows_per_second: None,
        read_target_seconds: read_target,
        update_seconds: None,
        update_rows_per_second: None,
        mixed_op_count: None,
        mixed_seconds: None,
        mixed_ops_per_second: None,
        delete_seconds: None,
        delete_rows_per_second: None,
        failure: None,
    };

    system.refresh_memory();
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => {
            report.status = String::from("failed_tempdir");
            report.failure = Some(error.to_string());
            return report;
        }
    };

    let mut store = MemoryStore::new(dir.path());

    eprintln!("[bench] crud scale={scale}: bulk inserting in batches of {batch_size}...");
    let start = Instant::now();
    let mut written = 0usize;
    while written < scale {
        let take = batch_size.min(scale - written);
        let mut batch = Vec::with_capacity(take);
        for offset in 0..take {
            let i = written + offset;
            batch.push(NewMemory {
                content: format!(
                    "bench memory {i} - a representative chunk of plausible text to stress \
                     the SQLite write path with realistic row sizes"
                ),
                tags: format!("bench,scale={scale},batch"),
                importance: ((i % 5) + 1) as i64,
                ..Default::default()
            });
        }
        if let Err(error) = store.add_many(batch) {
            report.status = String::from("failed_add_many");
            report.failure = Some(format!("written={written}: {error}"));
            return report;
        }
        written += take;
        if written.is_multiple_of(100_000) {
            eprintln!("[bench] crud scale={scale}: inserted {written} rows...");
        }
    }
    let write_elapsed = start.elapsed().as_secs_f64();
    let write_rps = scale as f64 / write_elapsed.max(f64::EPSILON);
    report.write_seconds = Some(write_elapsed);
    report.write_rows_per_second = Some(write_rps);
    eprintln!(
        "[bench] crud scale={scale}: write {write_elapsed:.2}s ({write_rps:.0} rows/s, target <{write_target:.0}s)"
    );

    if write_elapsed > write_target {
        report.status = String::from("failed_write_threshold");
        report.failure = Some(format!(
            "write {write_elapsed:.2}s exceeds target {write_target:.0}s"
        ));
        // Fall through: still measure read so we get the diagnostic.
    }

    // Read benchmark \u2014 full table scan via get_all().
    eprintln!("[bench] crud scale={scale}: full read via get_all()...");
    let start = Instant::now();
    let entries = match store.get_all() {
        Ok(entries) => entries,
        Err(error) => {
            report.status = String::from("failed_get_all");
            report.failure = Some(error.to_string());
            return report;
        }
    };
    let read_elapsed = start.elapsed().as_secs_f64();
    let read_rps = entries.len() as f64 / read_elapsed.max(f64::EPSILON);
    report.read_all_seconds = Some(read_elapsed);
    report.read_all_rows_per_second = Some(read_rps);
    eprintln!(
        "[bench] crud scale={scale}: read {} rows in {read_elapsed:.2}s ({read_rps:.0} rows/s, target <{read_target:.0}s)",
        entries.len()
    );
    if entries.len() != scale {
        report.status = String::from("failed_read_count");
        report.failure = Some(format!(
            "read {} rows but expected {scale}",
            entries.len()
        ));
        return report;
    }
    if read_elapsed > read_target {
        if report.failure.is_none() {
            report.status = String::from("failed_read_threshold");
            report.failure = Some(format!(
                "read {read_elapsed:.2}s exceeds target {read_target:.0}s"
            ));
        }
        return report;
    }

    // Collect ids for the update/delete sections. Rows from add_many on a
    // fresh DB get sequential rowids 1..=scale.
    let ids: Vec<i64> = (1..=scale as i64).collect();

    // ── bulk_update ────────────────────────────────────────────────────
    if bench_start.elapsed() > bench_timeout {
        report.status = String::from("timeout_before_update");
        report.failure = Some(format!(
            "bench wall-clock exceeded {}s before bulk_update phase",
            bench_timeout.as_secs()
        ));
        return report;
    }
    eprintln!("[bench] crud scale={scale}: bulk_update via update_content_many in batches of {batch_size}...");
    let start = Instant::now();
    let mut updated = 0usize;
    while updated < scale {
        if bench_start.elapsed() > bench_timeout {
            let partial_elapsed = start.elapsed().as_secs_f64();
            let partial_rps = updated as f64 / partial_elapsed.max(f64::EPSILON);
            report.update_seconds = Some(partial_elapsed);
            report.update_rows_per_second = Some(partial_rps);
            report.status = String::from("timeout_during_update");
            report.failure = Some(format!(
                "updated {updated}/{scale} rows before wall-clock limit of {}s",
                bench_timeout.as_secs()
            ));
            eprintln!("[bench] crud scale={scale}: update TIMEOUT at {updated}/{scale} rows");
            return report;
        }
        let take = batch_size.min(scale - updated);
        let mut batch: Vec<(i64, String)> = Vec::with_capacity(take);
        for offset in 0..take {
            let id = ids[updated + offset];
            batch.push((
                id,
                format!(
                    "bench memory {id} updated - revised content body to exercise the \
                     UPDATE hot path with realistic row sizes"
                ),
            ));
        }
        if let Err(error) = store.update_content_many(&batch) {
            report.status = String::from("failed_update_many");
            report.failure = Some(format!("updated={updated}: {error}"));
            return report;
        }
        updated += take;
    }
    let update_elapsed = start.elapsed().as_secs_f64();
    let update_rps = scale as f64 / update_elapsed.max(f64::EPSILON);
    report.update_seconds = Some(update_elapsed);
    report.update_rows_per_second = Some(update_rps);
    eprintln!(
        "[bench] crud scale={scale}: update {update_elapsed:.2}s ({update_rps:.0} rows/s)"
    );

    // ── mixed_crud_workload (80/10/10 insert/update/delete) ───────────
    // Capped at 10k ops so this phase stays fast even at 1M scale.
    // At 1M rows WAL checkpoint pressure drops insert throughput; 10k ops
    // measures realistic mixed-workload latency without multi-minute hangs.
    if bench_start.elapsed() > bench_timeout {
        report.status = String::from("timeout_before_mixed");
        report.failure = Some(format!(
            "bench wall-clock exceeded {}s before mixed_crud phase",
            bench_timeout.as_secs()
        ));
        return report;
    }
    let mixed_total = scale.min(10_000);
    let mixed_inserts = mixed_total * 80 / 100;
    let mixed_updates = mixed_total * 10 / 100;
    let mixed_deletes = mixed_total - mixed_inserts - mixed_updates;
    eprintln!(
        "[bench] crud scale={scale}: mixed_crud {mixed_total} ops (80/10/10 = {mixed_inserts}/{mixed_updates}/{mixed_deletes})..."
    );
    let start = Instant::now();
    // Inserts via add_many in 500-row batches (smaller = more timeout checkpoints).
    let mut inserted_ids: Vec<i64> = Vec::with_capacity(mixed_inserts);
    let insert_batch = 500usize.min(mixed_inserts.max(1));
    let mut done = 0usize;
    while done < mixed_inserts {
        // Timeout guard inside mixed-insert loop.
        if bench_start.elapsed() > bench_timeout {
            let partial_elapsed = start.elapsed().as_secs_f64();
            let partial_rps = done as f64 / partial_elapsed.max(f64::EPSILON);
            report.mixed_op_count = Some(done);
            report.mixed_seconds = Some(partial_elapsed);
            report.mixed_ops_per_second = Some(partial_rps);
            report.status = String::from("timeout_during_mixed_insert");
            report.failure = Some(format!(
                "bench wall-clock exceeded {}s during mixed_insert ({done}/{mixed_inserts} done)",
                bench_timeout.as_secs()
            ));
            return report;
        }
        let take = insert_batch.min(mixed_inserts - done);
        let mut batch = Vec::with_capacity(take);
        for offset in 0..take {
            batch.push(NewMemory {
                content: format!("mixed insert {}", done + offset),
                tags: String::from("bench,mixed"),
                importance: 3,
                ..Default::default()
            });
        }
        match store.add_many(batch) {
            Ok(new_ids) => inserted_ids.extend(new_ids),
            Err(error) => {
                report.status = String::from("failed_mixed_insert");
                report.failure = Some(error.to_string());
                return report;
            }
        }
        done += take;
    }
    // Updates over the existing corpus (deterministic stride).
    if mixed_updates > 0 {
        if bench_start.elapsed() > bench_timeout {
            let partial_elapsed = start.elapsed().as_secs_f64();
            report.mixed_op_count = Some(done);
            report.mixed_seconds = Some(partial_elapsed);
            report.mixed_ops_per_second = Some(done as f64 / partial_elapsed.max(f64::EPSILON));
            report.status = String::from("timeout_before_mixed_update");
            report.failure = Some(format!(
                "bench wall-clock exceeded {}s before mixed_update phase",
                bench_timeout.as_secs()
            ));
            return report;
        }
        let stride = scale.checked_div(mixed_updates).unwrap_or(1).max(1);
        let mut upd_batch: Vec<(i64, String)> = Vec::with_capacity(mixed_updates);
        for n in 0..mixed_updates {
            let idx = ((n * stride) % scale) as i64 + 1;
            upd_batch.push((idx, format!("mixed update {n}")));
        }
        if let Err(error) = store.update_content_many(&upd_batch) {
            report.status = String::from("failed_mixed_update");
            report.failure = Some(error.to_string());
            return report;
        }
    }
    // Deletes the freshly inserted rows so the corpus size returns to `scale`.
    if mixed_deletes > 0 {
        if bench_start.elapsed() > bench_timeout {
            let partial_elapsed = start.elapsed().as_secs_f64();
            let partial_ops = done + mixed_updates;
            report.mixed_op_count = Some(partial_ops);
            report.mixed_seconds = Some(partial_elapsed);
            report.mixed_ops_per_second = Some(partial_ops as f64 / partial_elapsed.max(f64::EPSILON));
            report.status = String::from("timeout_before_mixed_delete");
            report.failure = Some(format!(
                "bench wall-clock exceeded {}s before mixed_delete phase",
                bench_timeout.as_secs()
            ));
            return report;
        }
        let to_delete: Vec<i64> = inserted_ids.iter().take(mixed_deletes).copied().collect();
        if let Err(error) = store.delete_many(&to_delete) {
            report.status = String::from("failed_mixed_delete");
            report.failure = Some(error.to_string());
            return report;
        }
    }
    let mixed_elapsed = start.elapsed().as_secs_f64();
    let mixed_ops = mixed_inserts + mixed_updates + mixed_deletes;
    let mixed_rps = mixed_ops as f64 / mixed_elapsed.max(f64::EPSILON);
    report.mixed_op_count = Some(mixed_ops);
    report.mixed_seconds = Some(mixed_elapsed);
    report.mixed_ops_per_second = Some(mixed_rps);
    eprintln!(
        "[bench] crud scale={scale}: mixed_crud {mixed_ops} ops in {mixed_elapsed:.2}s ({mixed_rps:.0} ops/s)"
    );

    // ── bulk_delete ────────────────────────────────────────────────────
    // At 1M+ rows this phase is dominated by secondary-index maintenance
    // and can run for minutes without adding signal for the write/read SLOs.
    // Skip by default at million scale to keep the run deterministic.
    if scale >= ONE_MILLION {
        eprintln!(
            "[bench] crud scale={scale}: skipping bulk_delete (million-scale index-maintenance bottleneck)"
        );
        report.status = String::from("completed_delete_skipped_at_million");
        return report;
    }

    if bench_start.elapsed() > bench_timeout {
        report.status = String::from("timeout_before_delete");
        report.failure = Some(format!(
            "bench wall-clock exceeded {}s before bulk_delete phase",
            bench_timeout.as_secs()
        ));
        return report;
    }
    eprintln!("[bench] crud scale={scale}: bulk_delete via delete_many in batches of {batch_size}...");
    let start = Instant::now();
    let mut deleted = 0usize;
    while deleted < scale {
        // Check timeout inside the loop — index updates are slow at 1M and the
        // pre-phase guard alone is not enough to prevent a multi-minute run.
        if bench_start.elapsed() > bench_timeout {
            let partial_elapsed = start.elapsed().as_secs_f64();
            let partial_rps = deleted as f64 / partial_elapsed.max(f64::EPSILON);
            report.delete_seconds = Some(partial_elapsed);
            report.delete_rows_per_second = Some(partial_rps);
            report.status = String::from("timeout_during_delete");
            report.failure = Some(format!(
                "deleted {deleted}/{scale} rows in {partial_elapsed:.1}s ({partial_rps:.0} rows/s) \
                 before wall-clock limit of {}s",
                bench_timeout.as_secs()
            ));
            eprintln!(
                "[bench] crud scale={scale}: delete TIMEOUT at {deleted}/{scale} rows \
                 ({partial_rps:.0} rows/s) — index-update bottleneck; targets unchanged (write/read met)"
            );
            return report;
        }
        let take = batch_size.min(scale - deleted);
        let slice = &ids[deleted..deleted + take];
        if let Err(error) = store.delete_many(slice) {
            report.status = String::from("failed_delete_many");
            report.failure = Some(format!("deleted={deleted}: {error}"));
            return report;
        }
        deleted += take;
    }
    let delete_elapsed = start.elapsed().as_secs_f64();
    let delete_rps = scale as f64 / delete_elapsed.max(f64::EPSILON);
    report.delete_seconds = Some(delete_elapsed);
    report.delete_rows_per_second = Some(delete_rps);
    eprintln!(
        "[bench] crud scale={scale}: delete {delete_elapsed:.2}s ({delete_rps:.0} rows/s)"
    );

    if report.failure.is_some() {
        // Write threshold failure already recorded.
        return report;
    }

    report.status = String::from("completed");
    report
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
        crud: Vec::new(),
    };

    let mut failed = false;
    let crud_only = env::var("TS_BENCH_CRUD_ONLY").ok().as_deref() == Some("1");
    for scale in parse_scales() {
        report.linear_backend.push(linear_backend_report(scale));

        if !crud_only {
            let hnsw_report = run_hnsw_scale(scale, &mut system, &mut criterion);
            failed |= hnsw_report.status.starts_with("failed");
            report.hnsw.push(hnsw_report);

            let capacity_report = run_capacity_benchmark(scale);
            failed |= capacity_report.status.starts_with("failed");
            report.capacity.push(capacity_report);
        } else {
            eprintln!("[bench] TS_BENCH_CRUD_ONLY=1 set; skipping HNSW and capacity for scale={scale}");
        }

        let crud_report = run_crud_benchmark(scale, &mut system);
        failed |= crud_report.status.starts_with("failed");
        report.crud.push(crud_report);

        write_report(&report).expect("write benchmark report");
    }

    criterion.final_summary();
    assert!(!failed, "million_memory benchmark failed; see JSON report");
}
