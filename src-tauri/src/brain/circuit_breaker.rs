// SPDX-License-Identifier: MIT
//! Circuit breaker for LLM provider resilience.
//!
//! Implements a CLOSED → OPEN → HALF_OPEN state machine that fast-fails
//! requests to providers experiencing repeated failures. Inspired by the
//! circuit-breaker pattern in the open-source agentmemory project (credited
//! in CREDITS.md).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    /// Normal operation — requests pass through.
    Closed,
    /// Provider has failed repeatedly — requests are immediately rejected.
    Open,
    /// Recovery probe — one request is allowed through to test recovery.
    HalfOpen,
}

/// Configuration for the circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures within `failure_window` to trip the breaker.
    pub failure_threshold: u32,
    /// Time window within which failures are counted.
    pub failure_window: Duration,
    /// How long the breaker stays OPEN before transitioning to HALF_OPEN.
    pub recovery_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            failure_window: Duration::from_secs(60),
            recovery_timeout: Duration::from_secs(30),
        }
    }
}

/// A circuit breaker that tracks consecutive failures and fast-fails when
/// a provider is known to be unhealthy.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitState,
    /// Timestamps of recent failures (within the failure window).
    failures: VecDeque<Instant>,
    /// When the breaker transitioned to OPEN (used for recovery timeout).
    opened_at: Option<Instant>,
    /// Total number of times the breaker has tripped to OPEN.
    pub trip_count: u64,
    /// Total number of requests rejected while OPEN.
    pub rejected_count: u64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration.
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitState::Closed,
            failures: VecDeque::new(),
            opened_at: None,
            trip_count: 0,
            rejected_count: 0,
        }
    }

    /// Create a circuit breaker with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Current state of the breaker.
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Check whether a request should be allowed through.
    ///
    /// Returns `true` if the request is allowed (CLOSED or HALF_OPEN probe).
    /// Returns `false` if the breaker is OPEN (request should be rejected).
    ///
    /// Automatically transitions OPEN → HALF_OPEN after recovery timeout.
    pub fn is_allowed(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if recovery timeout has elapsed.
                if let Some(opened_at) = self.opened_at {
                    if opened_at.elapsed() >= self.config.recovery_timeout {
                        self.state = CircuitState::HalfOpen;
                        return true; // Allow one probe request
                    }
                }
                self.rejected_count += 1;
                false
            }
            CircuitState::HalfOpen => {
                // Only one request at a time in half-open; subsequent requests
                // are rejected until the probe resolves.
                // For simplicity, we allow it (the caller should use
                // record_success/record_failure promptly).
                true
            }
        }
    }

    /// Record a successful request. Resets the breaker to CLOSED.
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                // Probe succeeded — close the breaker.
                self.state = CircuitState::Closed;
                self.failures.clear();
                self.opened_at = None;
            }
            CircuitState::Closed => {
                // Success in closed state — clear any accumulated failures.
                // (Only clear stale ones; fresh failures might still be relevant.)
                self.prune_stale_failures();
            }
            CircuitState::Open => {
                // Shouldn't happen (requests are rejected), but handle gracefully.
                self.state = CircuitState::Closed;
                self.failures.clear();
                self.opened_at = None;
            }
        }
    }

    /// Record a failed request. May trip the breaker to OPEN.
    pub fn record_failure(&mut self) {
        let now = Instant::now();
        match self.state {
            CircuitState::Closed => {
                self.failures.push_back(now);
                self.prune_stale_failures();
                if self.failures.len() as u32 >= self.config.failure_threshold {
                    self.trip();
                }
            }
            CircuitState::HalfOpen => {
                // Probe failed — go back to OPEN.
                self.trip();
            }
            CircuitState::Open => {
                // Already open — just note the failure.
                self.failures.push_back(now);
            }
        }
    }

    /// Trip the breaker to OPEN.
    fn trip(&mut self) {
        self.state = CircuitState::Open;
        self.opened_at = Some(Instant::now());
        self.trip_count += 1;
    }

    /// Remove failures that are older than the failure window.
    fn prune_stale_failures(&mut self) {
        let cutoff = Instant::now() - self.config.failure_window;
        while let Some(&front) = self.failures.front() {
            if front < cutoff {
                self.failures.pop_front();
            } else {
                break;
            }
        }
    }

    /// Manually reset the breaker to CLOSED (e.g. after a health check).
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failures.clear();
        self.opened_at = None;
    }

    /// Number of recent failures in the current window.
    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn starts_closed() {
        let cb = CircuitBreaker::with_defaults();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn allows_when_closed() {
        let mut cb = CircuitBreaker::with_defaults();
        assert!(cb.is_allowed());
    }

    #[test]
    fn trips_after_threshold_failures() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            failure_window: Duration::from_secs(60),
            recovery_timeout: Duration::from_secs(1),
        });
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.trip_count, 1);
    }

    #[test]
    fn rejects_when_open() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            failure_window: Duration::from_secs(60),
            recovery_timeout: Duration::from_secs(60), // long timeout
        });
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_allowed());
        assert_eq!(cb.rejected_count, 1);
    }

    #[test]
    fn transitions_to_half_open_after_recovery() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            failure_window: Duration::from_secs(60),
            recovery_timeout: Duration::from_millis(50),
        });
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for recovery timeout.
        thread::sleep(Duration::from_millis(60));
        assert!(cb.is_allowed());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn half_open_success_closes() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            failure_window: Duration::from_secs(60),
            recovery_timeout: Duration::from_millis(10),
        });
        cb.record_failure();
        cb.record_failure();
        thread::sleep(Duration::from_millis(15));
        cb.is_allowed(); // transitions to HalfOpen
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn half_open_failure_reopens() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            failure_window: Duration::from_secs(60),
            recovery_timeout: Duration::from_millis(10),
        });
        cb.record_failure();
        cb.record_failure();
        thread::sleep(Duration::from_millis(15));
        cb.is_allowed();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.trip_count, 2);
    }

    #[test]
    fn stale_failures_are_pruned() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            failure_window: Duration::from_millis(50),
            recovery_timeout: Duration::from_secs(30),
        });
        cb.record_failure();
        cb.record_failure();
        // Wait for failure window to expire.
        thread::sleep(Duration::from_millis(60));
        cb.record_failure();
        // Only 1 failure in the current window — should still be CLOSED.
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn manual_reset_works() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            failure_window: Duration::from_secs(60),
            recovery_timeout: Duration::from_secs(60),
        });
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_allowed());
    }

    #[test]
    fn success_in_closed_prunes_stale() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 5,
            failure_window: Duration::from_millis(20),
            recovery_timeout: Duration::from_secs(30),
        });
        cb.record_failure();
        cb.record_failure();
        thread::sleep(Duration::from_millis(25));
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
    }
}
