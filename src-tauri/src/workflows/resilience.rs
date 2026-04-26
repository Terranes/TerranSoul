//! Workflow resilience primitives — retry, timeout, circuit breaker.
//!
//! Follows the same patterns as Temporal.io's activity policies but runs
//! entirely in-process (no external server required). Uses the `backon`
//! crate for battle-tested retry-with-backoff instead of hand-rolling.
//!
//! # Components
//!
//! | Primitive        | Temporal Equivalent       | Implementation        |
//! |------------------|---------------------------|-----------------------|
//! | `RetryPolicy`    | `RetryPolicy` on Activity | `backon` backoff      |
//! | `TimeoutPolicy`  | Schedule-to-close timeout | `tokio::time::timeout`|
//! | `CircuitBreaker` | N/A (message-queue pattern)| 3-state FSM          |
//! | `HeartbeatWatchdog` | Heartbeat timeout      | last-seen timestamp   |

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use backon::{ExponentialBuilder, Retryable};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// ── Retry Policy ─────────────────────────────────────────────────────────

/// Configurable retry policy for workflow activities.
///
/// Mirrors Temporal's `RetryPolicy`: max attempts, initial/max interval,
/// backoff multiplier, and non-retryable error filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including the first). 1 = no retry.
    pub max_attempts: u32,
    /// Initial backoff interval.
    pub initial_interval: Duration,
    /// Maximum backoff interval (caps exponential growth).
    pub max_interval: Duration,
    /// Multiplier applied each retry (e.g. 2.0 for exponential backoff).
    pub backoff_multiplier: f64,
    /// Error substrings that should NOT be retried (permanent failures).
    #[serde(default)]
    pub non_retryable_errors: Vec<String>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_interval: Duration::from_secs(1),
            max_interval: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            non_retryable_errors: Vec::new(),
        }
    }
}

impl RetryPolicy {
    /// No retries — execute exactly once.
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            ..Default::default()
        }
    }

    /// Returns `true` if the error message matches a non-retryable pattern.
    pub fn is_non_retryable(&self, error: &str) -> bool {
        self.non_retryable_errors
            .iter()
            .any(|pat| error.contains(pat))
    }

    /// Execute `op` with this retry policy. Returns the first success or
    /// the last error after all attempts are exhausted.
    pub async fn execute<F, Fut, T>(&self, op: F) -> Result<T, String>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, String>>,
    {
        let non_retryable = self.non_retryable_errors.clone();
        let backoff = ExponentialBuilder::default()
            .with_min_delay(self.initial_interval)
            .with_max_delay(self.max_interval)
            .with_factor(self.backoff_multiplier as f32)
            .with_max_times(self.max_attempts.saturating_sub(1) as usize);

        op.retry(backoff)
            .when(move |e: &String| !non_retryable.iter().any(|pat| e.contains(pat)))
            .await
    }
}

// ── Timeout Policy ───────────────────────────────────────────────────────

/// Timeout configuration for a workflow or individual activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    /// Overall time limit for the entire workflow (schedule-to-close).
    pub workflow_timeout: Option<Duration>,
    /// Time limit per individual activity execution (start-to-close).
    pub activity_timeout: Option<Duration>,
    /// Maximum time between heartbeats before considering the workflow
    /// stale. `None` = no heartbeat monitoring.
    pub heartbeat_timeout: Option<Duration>,
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            workflow_timeout: Some(Duration::from_secs(3600)), // 1 hour
            activity_timeout: Some(Duration::from_secs(300)),  // 5 min
            heartbeat_timeout: Some(Duration::from_secs(120)), // 2 min
        }
    }
}

impl TimeoutPolicy {
    /// No timeouts at all.
    pub fn unlimited() -> Self {
        Self {
            workflow_timeout: None,
            activity_timeout: None,
            heartbeat_timeout: None,
        }
    }

    /// Execute `fut` with the activity timeout. Returns `Err` on timeout.
    pub async fn run_activity<F, T>(&self, fut: F) -> Result<T, String>
    where
        F: Future<Output = Result<T, String>>,
    {
        match self.activity_timeout {
            Some(d) => tokio::time::timeout(d, fut)
                .await
                .map_err(|_| format!("activity timed out after {}s", d.as_secs()))?,
            None => fut.await,
        }
    }

    /// Execute `fut` with the workflow timeout. Returns `Err` on timeout.
    pub async fn run_workflow<F, T>(&self, fut: F) -> Result<T, String>
    where
        F: Future<Output = Result<T, String>>,
    {
        match self.workflow_timeout {
            Some(d) => tokio::time::timeout(d, fut)
                .await
                .map_err(|_| format!("workflow timed out after {}s", d.as_secs()))?,
            None => fut.await,
        }
    }
}

// ── Circuit Breaker ──────────────────────────────────────────────────────

/// Three-state circuit breaker (Closed → Open → HalfOpen → Closed).
///
/// When consecutive failures reach `failure_threshold`, the breaker
/// opens and rejects all calls for `recovery_timeout`. After that
/// window a single probe call is allowed; if it succeeds the breaker
/// closes, otherwise it re-opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BreakerState {
    /// Normal operation — calls pass through.
    Closed,
    /// Failures exceeded threshold — calls are rejected immediately.
    Open,
    /// Recovery window elapsed — one probe call allowed.
    HalfOpen,
}

/// Thread-safe circuit breaker.
#[derive(Debug)]
pub struct CircuitBreaker {
    inner: Arc<Mutex<BreakerInner>>,
}

#[derive(Debug)]
struct BreakerInner {
    state: BreakerState,
    consecutive_failures: u32,
    failure_threshold: u32,
    recovery_timeout: Duration,
    last_failure_at: Option<Instant>,
    total_successes: u64,
    total_failures: u64,
}

/// Snapshot of circuit breaker metrics (for monitoring / UI).
#[derive(Debug, Clone, Serialize)]
pub struct BreakerMetrics {
    pub state: BreakerState,
    pub consecutive_failures: u32,
    pub failure_threshold: u32,
    pub total_successes: u64,
    pub total_failures: u64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker.
    ///
    /// * `failure_threshold` — consecutive failures before opening.
    /// * `recovery_timeout` — how long to stay open before probing.
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BreakerInner {
                state: BreakerState::Closed,
                consecutive_failures: 0,
                failure_threshold,
                recovery_timeout,
                last_failure_at: None,
                total_successes: 0,
                total_failures: 0,
            })),
        }
    }

    /// Cheap clone for sharing across tasks.
    pub fn handle(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    /// Current metrics snapshot.
    pub async fn metrics(&self) -> BreakerMetrics {
        let g = self.inner.lock().await;
        BreakerMetrics {
            state: effective_state(&g),
            consecutive_failures: g.consecutive_failures,
            failure_threshold: g.failure_threshold,
            total_successes: g.total_successes,
            total_failures: g.total_failures,
        }
    }

    /// Execute `op` through the circuit breaker. If the breaker is open,
    /// returns an error immediately without calling `op`.
    pub async fn call<F, Fut, T>(&self, op: F) -> Result<T, String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, String>>,
    {
        // Check + transition
        {
            let mut g = self.inner.lock().await;
            match effective_state(&g) {
                BreakerState::Closed => { /* allow */ }
                BreakerState::HalfOpen => {
                    // Allow one probe — state stays HalfOpen until result
                }
                BreakerState::Open => {
                    return Err(format!(
                        "circuit breaker open: {} consecutive failures (threshold {})",
                        g.consecutive_failures, g.failure_threshold
                    ));
                }
            }
            // If entering HalfOpen from Open, stay in HalfOpen
            if g.state == BreakerState::Open {
                g.state = BreakerState::HalfOpen;
            }
        }

        let result = op().await;

        {
            let mut g = self.inner.lock().await;
            match &result {
                Ok(_) => {
                    g.consecutive_failures = 0;
                    g.state = BreakerState::Closed;
                    g.total_successes += 1;
                }
                Err(_) => {
                    g.consecutive_failures += 1;
                    g.total_failures += 1;
                    g.last_failure_at = Some(Instant::now());
                    if g.consecutive_failures >= g.failure_threshold {
                        g.state = BreakerState::Open;
                    }
                }
            }
        }

        result
    }
}

/// Compute the effective state accounting for recovery timeout elapsed.
fn effective_state(inner: &BreakerInner) -> BreakerState {
    match inner.state {
        BreakerState::Open => {
            if let Some(last) = inner.last_failure_at {
                if last.elapsed() >= inner.recovery_timeout {
                    return BreakerState::HalfOpen;
                }
            }
            BreakerState::Open
        }
        other => other,
    }
}

// ── Heartbeat Watchdog ───────────────────────────────────────────────────

/// Tracks the last heartbeat timestamp (epoch seconds) for each workflow
/// and detects stale workflows whose heartbeat gap exceeds the threshold.
#[derive(Debug)]
pub struct HeartbeatWatchdog {
    /// Map of workflow_id → last heartbeat epoch seconds.
    last_seen: Arc<tokio::sync::RwLock<std::collections::HashMap<String, i64>>>,
    /// Default heartbeat threshold — if no heartbeat arrives within this
    /// duration, the workflow is considered stale.
    default_threshold: AtomicU64,
}

/// A workflow detected as stale by the watchdog.
#[derive(Debug, Clone, Serialize)]
pub struct StaleWorkflow {
    pub workflow_id: String,
    pub last_heartbeat: i64,
    pub stale_since_secs: i64,
}

impl HeartbeatWatchdog {
    /// Create a watchdog with the given default heartbeat threshold.
    pub fn new(default_threshold: Duration) -> Self {
        Self {
            last_seen: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            default_threshold: AtomicU64::new(default_threshold.as_secs()),
        }
    }

    /// Record a heartbeat for a workflow.
    pub async fn touch(&self, workflow_id: &str) {
        let now = now_secs();
        self.last_seen
            .write()
            .await
            .insert(workflow_id.to_string(), now);
    }

    /// Remove a workflow from tracking (e.g. on completion).
    pub async fn remove(&self, workflow_id: &str) {
        self.last_seen.write().await.remove(workflow_id);
    }

    /// Return all workflows whose last heartbeat is older than the threshold.
    pub async fn stale_workflows(&self) -> Vec<StaleWorkflow> {
        let now = now_secs();
        let threshold = self.default_threshold.load(Ordering::Relaxed) as i64;
        let map = self.last_seen.read().await;
        map.iter()
            .filter_map(|(id, &last)| {
                let gap = now - last;
                if gap > threshold {
                    Some(StaleWorkflow {
                        workflow_id: id.clone(),
                        last_heartbeat: last,
                        stale_since_secs: gap - threshold,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Update the default threshold at runtime.
    pub fn set_threshold(&self, d: Duration) {
        self.default_threshold.store(d.as_secs(), Ordering::Relaxed);
    }

    /// Current threshold in seconds.
    pub fn threshold_secs(&self) -> u64 {
        self.default_threshold.load(Ordering::Relaxed)
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── Resilient Activity Runner ────────────────────────────────────────────

/// Combined runner that wraps an operation with retry + timeout + circuit
/// breaker. This is the main entry point callers should use.
pub struct ResilientRunner {
    pub retry: RetryPolicy,
    pub timeout: TimeoutPolicy,
    pub breaker: CircuitBreaker,
}

impl ResilientRunner {
    /// Create a runner with the given policies.
    pub fn new(
        retry: RetryPolicy,
        timeout: TimeoutPolicy,
        breaker: CircuitBreaker,
    ) -> Self {
        Self {
            retry,
            timeout,
            breaker,
        }
    }

    /// Execute an activity with retry + timeout + circuit breaker.
    ///
    /// Evaluation order (outermost → innermost):
    /// 1. Circuit breaker — rejects immediately if open
    /// 2. Retry — re-executes on failure (respects non-retryable filter)
    /// 3. Timeout — each individual attempt is time-bounded
    pub async fn run_activity<F, Fut, T>(&self, op: F) -> Result<T, String>
    where
        F: Fn() -> Fut + Clone,
        Fut: Future<Output = Result<T, String>>,
    {
        let timeout_policy = self.timeout.clone();
        let op_clone = op.clone();
        self.breaker
            .call(|| async {
                let tp = timeout_policy.clone();
                let op_inner = op_clone.clone();
                self.retry
                    .execute(move || {
                        let tp2 = tp.clone();
                        let op2 = op_inner.clone();
                        async move { tp2.run_activity(op2()).await }
                    })
                    .await
            })
            .await
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering as AtomicOrd};

    #[tokio::test]
    async fn retry_succeeds_after_transient_failures() {
        let count = Arc::new(AtomicU32::new(0));
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_interval: Duration::from_millis(10),
            max_interval: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            non_retryable_errors: vec![],
        };
        let c = count.clone();
        let result = policy
            .execute(move || {
                let c = c.clone();
                async move {
                    let n = c.fetch_add(1, AtomicOrd::Relaxed);
                    if n < 2 {
                        Err("transient".to_string())
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(count.load(AtomicOrd::Relaxed), 3);
    }

    #[tokio::test]
    async fn retry_respects_non_retryable() {
        let count = Arc::new(AtomicU32::new(0));
        let policy = RetryPolicy {
            max_attempts: 5,
            initial_interval: Duration::from_millis(1),
            max_interval: Duration::from_millis(10),
            backoff_multiplier: 1.0,
            non_retryable_errors: vec!["permanent".to_string()],
        };
        let c = count.clone();
        let result: Result<(), String> = policy
            .execute(move || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, AtomicOrd::Relaxed);
                    Err("permanent failure".to_string())
                }
            })
            .await;
        assert!(result.is_err());
        // Should stop after first attempt due to non-retryable match
        assert_eq!(count.load(AtomicOrd::Relaxed), 1);
    }

    #[tokio::test]
    async fn retry_exhausts_all_attempts() {
        let count = Arc::new(AtomicU32::new(0));
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_interval: Duration::from_millis(1),
            max_interval: Duration::from_millis(10),
            backoff_multiplier: 1.0,
            non_retryable_errors: vec![],
        };
        let c = count.clone();
        let result: Result<(), String> = policy
            .execute(move || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, AtomicOrd::Relaxed);
                    Err("always fails".to_string())
                }
            })
            .await;
        assert!(result.is_err());
        assert_eq!(count.load(AtomicOrd::Relaxed), 3);
    }

    #[tokio::test]
    async fn no_retry_runs_once() {
        let count = Arc::new(AtomicU32::new(0));
        let policy = RetryPolicy::no_retry();
        let c = count.clone();
        let _: Result<(), String> = policy
            .execute(move || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, AtomicOrd::Relaxed);
                    Err("fail".to_string())
                }
            })
            .await;
        assert_eq!(count.load(AtomicOrd::Relaxed), 1);
    }

    #[tokio::test]
    async fn timeout_fires_on_slow_activity() {
        let policy = TimeoutPolicy {
            workflow_timeout: None,
            activity_timeout: Some(Duration::from_millis(50)),
            heartbeat_timeout: None,
        };
        let result: Result<(), String> = policy
            .run_activity(async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                Ok(())
            })
            .await;
        assert!(result.unwrap_err().contains("timed out"));
    }

    #[tokio::test]
    async fn timeout_passes_fast_activity() {
        let policy = TimeoutPolicy {
            workflow_timeout: None,
            activity_timeout: Some(Duration::from_millis(500)),
            heartbeat_timeout: None,
        };
        let result = policy.run_activity(async { Ok(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(2, Duration::from_secs(60));
        // Two failures → opens
        let _: Result<(), String> = cb.call(|| async { Err("f1".into()) }).await;
        let _: Result<(), String> = cb.call(|| async { Err("f2".into()) }).await;

        // Third call should be rejected by the breaker
        let result: Result<(), String> = cb.call(|| async { Ok(()) }).await;
        assert!(result.unwrap_err().contains("circuit breaker open"));

        let m = cb.metrics().await;
        assert_eq!(m.state, BreakerState::Open);
        assert_eq!(m.total_failures, 2);
    }

    #[tokio::test]
    async fn circuit_breaker_recovers_after_timeout() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(50));
        let _: Result<(), String> = cb.call(|| async { Err("fail".into()) }).await;

        // Breaker is open
        assert_eq!(cb.metrics().await.state, BreakerState::Open);

        // Wait for recovery timeout
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should be half-open, probe succeeds → closes
        let result = cb.call(|| async { Ok(99) }).await;
        assert_eq!(result.unwrap(), 99);
        assert_eq!(cb.metrics().await.state, BreakerState::Closed);
    }

    #[tokio::test]
    async fn circuit_breaker_reopens_on_failed_probe() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(50));
        let _: Result<(), String> = cb.call(|| async { Err("fail".into()) }).await;

        tokio::time::sleep(Duration::from_millis(60)).await;

        // Probe fails → re-opens
        let _: Result<(), String> = cb.call(|| async { Err("still bad".into()) }).await;
        assert_eq!(cb.metrics().await.state, BreakerState::Open);
    }

    #[tokio::test]
    async fn watchdog_detects_stale() {
        let wd = HeartbeatWatchdog::new(Duration::from_secs(1));
        // Manually insert an old timestamp
        wd.last_seen.write().await.insert("wf-1".into(), now_secs() - 10);
        let stale = wd.stale_workflows().await;
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].workflow_id, "wf-1");
        assert!(stale[0].stale_since_secs >= 9);
    }

    #[tokio::test]
    async fn watchdog_fresh_not_stale() {
        let wd = HeartbeatWatchdog::new(Duration::from_secs(60));
        wd.touch("wf-1").await;
        let stale = wd.stale_workflows().await;
        assert!(stale.is_empty());
    }

    #[tokio::test]
    async fn watchdog_remove_stops_tracking() {
        let wd = HeartbeatWatchdog::new(Duration::from_secs(0));
        wd.touch("wf-1").await;
        wd.remove("wf-1").await;
        // Even with threshold 0, removed workflow should not appear
        // (it's simply not tracked anymore)
        let stale = wd.stale_workflows().await;
        assert!(stale.is_empty());
    }

    #[tokio::test]
    async fn resilient_runner_integrates_all() {
        let count = Arc::new(AtomicU32::new(0));
        let runner = ResilientRunner::new(
            RetryPolicy {
                max_attempts: 3,
                initial_interval: Duration::from_millis(10),
                max_interval: Duration::from_millis(50),
                backoff_multiplier: 2.0,
                non_retryable_errors: vec![],
            },
            TimeoutPolicy {
                workflow_timeout: None,
                activity_timeout: Some(Duration::from_secs(5)),
                heartbeat_timeout: None,
            },
            CircuitBreaker::new(5, Duration::from_secs(60)),
        );

        let c = count.clone();
        let result = runner
            .run_activity(move || {
                let c = c.clone();
                async move {
                    let n = c.fetch_add(1, AtomicOrd::Relaxed);
                    if n < 2 {
                        Err("transient".to_string())
                    } else {
                        Ok("done")
                    }
                }
            })
            .await;
        assert_eq!(result.unwrap(), "done");
        assert_eq!(count.load(AtomicOrd::Relaxed), 3);
    }
}
