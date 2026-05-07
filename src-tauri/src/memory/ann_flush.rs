//! Debounced async flush for the ANN index (Chunk 41.10).
//!
//! When the ANN index becomes dirty (ops threshold or time threshold exceeded),
//! the flush handle coalesces concurrent save requests into a single background
//! task that acquires the `MemoryStore` lock and persists.
//!
//! Design:
//! - A `tokio::sync::Notify` signals that a flush may be needed.
//! - The background task waits for the notification, then sleeps a short
//!   debounce window (200 ms) to coalesce rapid-fire signals, then saves.
//! - An `AtomicBool` prevents duplicate flush scheduling within the window.
//! - Graceful shutdown via a `tokio::sync::watch` channel stops the task.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Notify};

/// Debounce window: after the first flush signal, wait this long to coalesce
/// additional signals before performing the actual save.
const DEBOUNCE_MS: u64 = 200;

/// Handle for scheduling debounced ANN index flushes.
///
/// Clone-cheap (all fields are `Arc`). The actual flush is performed by the
/// background task spawned via [`spawn_flush_task`].
#[derive(Clone)]
pub struct AnnFlushHandle {
    notify: Arc<Notify>,
    scheduled: Arc<AtomicBool>,
    shutdown_tx: watch::Sender<bool>,
}

/// Metrics exposed for observability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FlushStats {
    /// Total number of flush cycles completed since process start.
    pub flush_count: u64,
    /// Total number of ops flushed across all cycles.
    pub ops_flushed: u64,
}

impl AnnFlushHandle {
    /// Create a new flush handle. Call [`spawn_flush_task`] separately to
    /// start the background consumer.
    pub fn new() -> Self {
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            notify: Arc::new(Notify::new()),
            scheduled: Arc::new(AtomicBool::new(false)),
            shutdown_tx,
        }
    }

    /// Signal that the ANN index may need flushing.
    ///
    /// This is cheap and non-blocking. Multiple calls within the debounce
    /// window are coalesced into a single flush.
    pub fn signal_flush(&self) {
        if !self.scheduled.swap(true, Ordering::AcqRel) {
            self.notify.notify_one();
        }
    }

    /// Request graceful shutdown of the background flush task.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        // Wake the task so it can observe the shutdown signal.
        self.notify.notify_one();
    }

    /// Subscribe to the shutdown channel (used by the flush task).
    fn subscribe_shutdown(&self) -> watch::Receiver<bool> {
        self.shutdown_tx.subscribe()
    }
}

impl Default for AnnFlushHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn the background flush task.
///
/// The task runs until shutdown is signaled. On each notification it:
/// 1. Waits the debounce window to coalesce concurrent signals.
/// 2. Acquires the `MemoryStore` mutex and saves all dirty ANN indices.
/// 3. Resets the scheduled flag.
///
/// `store_mutex` must be the same `std::sync::Mutex<MemoryStore>` from
/// `AppStateInner`. We take it as a closure to avoid circular deps.
pub fn spawn_flush_task<F>(handle: AnnFlushHandle, flush_fn: F)
where
    F: Fn() -> (u64, u64) + Send + Sync + 'static,
{
    let notify = handle.notify.clone();
    let scheduled = handle.scheduled.clone();
    let mut shutdown_rx = handle.subscribe_shutdown();

    tokio::spawn(async move {
        loop {
            // Wait for a flush signal or shutdown.
            tokio::select! {
                _ = notify.notified() => {}
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        // Final flush before exit.
                        scheduled.store(false, Ordering::Release);
                        flush_fn();
                        return;
                    }
                }
            }

            // Debounce: wait a short window to coalesce rapid signals.
            tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;

            // Check shutdown again after sleep.
            if *shutdown_rx.borrow() {
                scheduled.store(false, Ordering::Release);
                flush_fn();
                return;
            }

            // Reset the scheduled flag so new signals can re-trigger.
            scheduled.store(false, Ordering::Release);

            // Perform the actual flush.
            flush_fn();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;

    #[tokio::test]
    async fn flush_handle_coalesces_signals() {
        let handle = AnnFlushHandle::new();
        let counter = Arc::new(AtomicU64::new(0));

        let counter_clone = counter.clone();
        spawn_flush_task(handle.clone(), move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            (1, 0)
        });

        // Send 10 rapid signals — they should coalesce into 1 flush.
        for _ in 0..10 {
            handle.signal_flush();
        }

        // Wait for debounce + flush to complete.
        tokio::time::sleep(Duration::from_millis(400)).await;

        let count = counter.load(Ordering::SeqCst);
        assert!(count <= 2, "Expected coalesced flushes, got {count}");
        assert!(count >= 1, "Expected at least 1 flush, got {count}");
    }

    #[tokio::test]
    async fn flush_handle_shutdown_performs_final_flush() {
        let handle = AnnFlushHandle::new();
        let counter = Arc::new(AtomicU64::new(0));

        let counter_clone = counter.clone();
        spawn_flush_task(handle.clone(), move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            (1, 0)
        });

        // Signal once then immediately shutdown.
        handle.signal_flush();
        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.shutdown();

        // Give the task time to finish.
        tokio::time::sleep(Duration::from_millis(400)).await;

        let count = counter.load(Ordering::SeqCst);
        assert!(count >= 1, "Expected final flush on shutdown, got {count}");
    }

    #[tokio::test]
    async fn no_flush_without_signal() {
        let handle = AnnFlushHandle::new();
        let counter = Arc::new(AtomicU64::new(0));

        let counter_clone = counter.clone();
        spawn_flush_task(handle.clone(), move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            (1, 0)
        });

        // Wait without signaling.
        tokio::time::sleep(Duration::from_millis(500)).await;

        let count = counter.load(Ordering::SeqCst);
        assert_eq!(count, 0, "No flush should occur without a signal");

        handle.shutdown();
    }
}
