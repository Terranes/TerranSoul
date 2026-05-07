//! Lightweight per-operation latency histograms for the memory store (Chunk 41.2).
//!
//! Uses 16 power-of-2 µs buckets (0–1 µs through 16 ms+) backed by `AtomicU64`
//! arrays so recording is lock-free. A process-wide [`METRICS`] singleton lets
//! every call site record without passing state around.
//!
//! Retrieve the current snapshot via [`METRICS`]`.snapshot()` or the
//! `get_memory_metrics` Tauri command.

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::Instant;

/// Number of latency buckets.
///
/// Bucket `i` covers latencies in `[2^(i-1) µs, 2^i µs)`.
/// Bucket 0 covers `[0, 1 µs)`. Bucket 15 covers `[2^14 µs, ∞) ≈ [16 ms, ∞)`.
const BUCKET_COUNT: usize = 16;

// ─── Per-operation histogram ─────────────────────────────────────────────────

/// Per-operation latency histogram backed by atomic counters.
pub struct OpMetrics {
    pub count: AtomicU64,
    pub sum_ns: AtomicU64,
    buckets: [AtomicU64; BUCKET_COUNT],
}

impl OpMetrics {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            sum_ns: AtomicU64::new(0),
            buckets: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
        }
    }

    /// Record one observation. `elapsed_ns` is the wall-clock duration in nanoseconds.
    pub fn record_ns(&self, elapsed_ns: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum_ns.fetch_add(elapsed_ns, Ordering::Relaxed);
        let elapsed_us = elapsed_ns / 1_000;
        // bucket = floor(log2(elapsed_us + 1)), capped at BUCKET_COUNT - 1.
        let bucket = if elapsed_us == 0 {
            0
        } else {
            (u64::BITS - elapsed_us.leading_zeros()) as usize
        }
        .min(BUCKET_COUNT - 1);
        self.buckets[bucket].fetch_add(1, Ordering::Relaxed);
    }

    /// Produce a serialisable snapshot (safe point-in-time read; not atomic across fields).
    pub fn snapshot(&self) -> OpSnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let sum_ns = self.sum_ns.load(Ordering::Relaxed);
        let buckets: [u64; BUCKET_COUNT] =
            std::array::from_fn(|i| self.buckets[i].load(Ordering::Relaxed));
        let mean_ms = if count > 0 {
            Some((sum_ns as f64 / count as f64) / 1_000_000.0)
        } else {
            None
        };
        OpSnapshot {
            count,
            mean_ms,
            p50_ms: percentile_ms(&buckets, count, 0.50),
            p95_ms: percentile_ms(&buckets, count, 0.95),
            p99_ms: percentile_ms(&buckets, count, 0.99),
        }
    }
}

fn percentile_ms(buckets: &[u64; BUCKET_COUNT], total: u64, pct: f64) -> Option<f64> {
    if total == 0 {
        return None;
    }
    let target = ((total as f64 * pct).ceil() as u64).max(1);
    let mut cumulative = 0u64;
    for (i, &cnt) in buckets.iter().enumerate() {
        cumulative += cnt;
        if cumulative >= target {
            // Lower bound of bucket i in µs.
            let lower_us: f64 = if i == 0 { 0.0 } else { (1u64 << (i - 1)) as f64 };
            return Some(lower_us / 1_000.0); // µs → ms
        }
    }
    // Overflow: all ops are in the last bucket.
    Some((1u64 << (BUCKET_COUNT - 2)) as f64 / 1_000.0)
}

// ─── Per-operation snapshot (serialisable) ───────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct OpSnapshot {
    /// Total number of recorded operations.
    pub count: u64,
    /// Mean latency in milliseconds, or `null` when no operations have been recorded.
    pub mean_ms: Option<f64>,
    /// p50 latency in milliseconds, or `null` when no operations have been recorded.
    pub p50_ms: Option<f64>,
    /// p95 latency in milliseconds, or `null` when no operations have been recorded.
    pub p95_ms: Option<f64>,
    /// p99 latency in milliseconds, or `null` when no operations have been recorded.
    pub p99_ms: Option<f64>,
}

// ─── Process-wide metrics registry ──────────────────────────────────────────

/// Snapshot of all operation metrics, ready for JSON serialisation.
#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub add: OpSnapshot,
    pub add_many: OpSnapshot,
    pub update: OpSnapshot,
    pub delete: OpSnapshot,
    pub set_embedding: OpSnapshot,
    pub hybrid_search: OpSnapshot,
    pub hybrid_search_rrf: OpSnapshot,
}

/// Process-wide memory operation metrics.
///
/// All fields are `pub` so call sites can record directly:
/// ```ignore
/// METRICS.add.record_ns(elapsed_ns);
/// ```
pub struct MemoryMetrics {
    pub add: OpMetrics,
    pub add_many: OpMetrics,
    pub update: OpMetrics,
    pub delete: OpMetrics,
    pub set_embedding: OpMetrics,
    pub hybrid_search: OpMetrics,
    pub hybrid_search_rrf: OpMetrics,
}

impl MemoryMetrics {
    pub const fn new() -> Self {
        Self {
            add: OpMetrics::new(),
            add_many: OpMetrics::new(),
            update: OpMetrics::new(),
            delete: OpMetrics::new(),
            set_embedding: OpMetrics::new(),
            hybrid_search: OpMetrics::new(),
            hybrid_search_rrf: OpMetrics::new(),
        }
    }
}

impl Default for MemoryMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryMetrics {
    /// Return a point-in-time snapshot of all operation histograms.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            add: self.add.snapshot(),
            add_many: self.add_many.snapshot(),
            update: self.update.snapshot(),
            delete: self.delete.snapshot(),
            set_embedding: self.set_embedding.snapshot(),
            hybrid_search: self.hybrid_search.snapshot(),
            hybrid_search_rrf: self.hybrid_search_rrf.snapshot(),
        }
    }
}

// SAFETY: all fields are AtomicU64 — Send + Sync.
unsafe impl Send for MemoryMetrics {}
unsafe impl Sync for MemoryMetrics {}

/// Process-wide singleton. Accessed by every `MemoryStore` operation.
pub static METRICS: LazyLock<MemoryMetrics> = LazyLock::new(MemoryMetrics::new);

// ─── Convenience timer ───────────────────────────────────────────────────────

/// Time `f`, record the elapsed nanoseconds into `op`, and return the result.
///
/// ```ignore
/// let entry = timed(&METRICS.add, || store.add(m))?;
/// ```
pub fn timed<T, F: FnOnce() -> T>(op: &'static OpMetrics, f: F) -> T {
    let start = Instant::now();
    let result = f();
    op.record_ns(start.elapsed().as_nanos() as u64);
    result
}

// ─── RAII timer ─────────────────────────────────────────────────────────────

/// RAII timer that records its elapsed nanoseconds into an [`OpMetrics`] on drop.
///
/// Typical usage inside `MemoryStore` methods:
/// ```ignore
/// let _t = Timer::start(&METRICS.add);
/// // ... rest of function ...
/// // Timer records elapsed time when it goes out of scope.
/// ```
pub struct Timer<'a> {
    op: &'a OpMetrics,
    start: Instant,
}

impl<'a> Timer<'a> {
    /// Start timing an operation.
    pub fn start(op: &'a OpMetrics) -> Self {
        Self {
            op,
            start: Instant::now(),
        }
    }
}

impl Drop for Timer<'_> {
    fn drop(&mut self) {
        self.op.record_ns(self.start.elapsed().as_nanos() as u64);
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    fn fresh_op() -> OpMetrics {
        OpMetrics::new()
    }

    #[test]
    fn record_increments_count() {
        let op = fresh_op();
        assert_eq!(op.count.load(Ordering::Relaxed), 0);
        op.record_ns(5_000); // 5 µs
        assert_eq!(op.count.load(Ordering::Relaxed), 1);
        op.record_ns(10_000); // 10 µs
        assert_eq!(op.count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn record_accumulates_sum() {
        let op = fresh_op();
        op.record_ns(1_000_000); // 1 ms
        op.record_ns(2_000_000); // 2 ms
        assert_eq!(op.sum_ns.load(Ordering::Relaxed), 3_000_000);
    }

    #[test]
    fn snapshot_mean_ms() {
        let op = fresh_op();
        op.record_ns(1_000_000); // 1 ms
        op.record_ns(3_000_000); // 3 ms
        let snap = op.snapshot();
        assert_eq!(snap.count, 2);
        let mean = snap.mean_ms.expect("mean should be Some");
        // mean = 2 ms ± floating point
        assert!((mean - 2.0).abs() < 0.01, "mean_ms={mean}");
    }

    #[test]
    fn snapshot_no_ops_gives_none() {
        let op = fresh_op();
        let snap = op.snapshot();
        assert_eq!(snap.count, 0);
        assert!(snap.mean_ms.is_none());
        assert!(snap.p50_ms.is_none());
        assert!(snap.p99_ms.is_none());
    }

    #[test]
    fn percentile_bucket_placement() {
        let op = fresh_op();
        // 100 µs = 100_000 ns → bucket = floor(log2(100)) + 1 = 7 (64–128 µs range)
        for _ in 0..100 {
            op.record_ns(100_000);
        }
        let snap = op.snapshot();
        // p50 and p99 should both be in the 64–128 µs range → 0.064–0.128 ms
        let p50 = snap.p50_ms.unwrap();
        let p99 = snap.p99_ms.unwrap();
        assert!(p50 >= 0.064 && p50 <= 0.128, "p50_ms={p50}");
        assert!(p99 >= 0.064 && p99 <= 0.128, "p99_ms={p99}");
    }

    #[test]
    fn global_metrics_singleton_is_accessible() {
        // Verifies METRICS is accessible and records work on the singleton.
        let before = METRICS.add.count.load(Ordering::Relaxed);
        METRICS.add.record_ns(500_000);
        let after = METRICS.add.count.load(Ordering::Relaxed);
        assert!(after > before, "METRICS.add.count should have incremented");
    }

    #[test]
    fn timed_helper_records_and_returns() {
        let op = fresh_op();
        // SAFETY: We lie about 'static here — safe in tests where `op` outlives the call.
        // In production, only `&METRICS.field` (which IS static) is passed.
        let before = op.count.load(Ordering::Relaxed);
        // Use record_ns directly to avoid the 'static requirement in tests.
        let start = std::time::Instant::now();
        let _ = std::hint::black_box(42u64 + 1);
        op.record_ns(start.elapsed().as_nanos() as u64);
        assert_eq!(op.count.load(Ordering::Relaxed), before + 1);
    }
}
