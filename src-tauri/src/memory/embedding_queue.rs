//! Self-healing embedding retry queue (Chunks 38.2, 41.6, 41.7).
//!
//! When memories are stored without embeddings (Ollama down, rate-limited,
//! or transient failure), they are enqueued here. A background worker drains
//! the queue in batches using the batch embedding endpoint from Chunk 38.1,
//! with exponential backoff on per-row failures (cap at 1 hour).
//!
//! Chunk 41.7 enhancements:
//! - Configurable batch size per provider (8/32/128).
//! - Concurrency semaphore to cap concurrent embed calls.
//! - Soft pause on rate-limit (429/busy) vs hard fail distinction.
//! - WorkerStatus surfaced through brain_health.
//! - Graceful shutdown via CancellationToken.
//!
//! The queue is a SQLite table (`pending_embeddings`) living in the same
//! database as the memory store. This module provides the queue CRUD and the
//! background worker loop.

use rusqlite::{params, Connection, Result as SqlResult};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

use crate::brain::embed_batch_for_mode;

/// Shutdown signal type — a watch channel receiver for graceful stop.
pub type ShutdownRx = watch::Receiver<bool>;

/// Maximum backoff between retries (1 hour).
const MAX_BACKOFF_SECS: u64 = 3600;

/// Default batch size for the retry worker.
const DEFAULT_BATCH_SIZE: usize = 32;

/// Worker tick interval (10 seconds).
pub const WORKER_TICK_INTERVAL: Duration = Duration::from_secs(10);

/// Maximum concurrent embed calls.
const MAX_CONCURRENCY: usize = 4;

/// Pause duration after detecting a rate limit (starts at 30s, doubles).
const INITIAL_PAUSE_SECS: u64 = 30;

/// Maximum pause duration (10 minutes).
const MAX_PAUSE_SECS: u64 = 600;

/// Batch sizes per provider category.
pub fn batch_size_for_provider(provider: Option<&str>) -> usize {
    match provider {
        Some("ollama") | Some("local") => 8,
        Some("free") | Some("pollinations") => 32,
        Some("paid") | Some("openai") | Some("anthropic") => 128,
        _ => DEFAULT_BATCH_SIZE,
    }
}

/// Status of the embedding retry queue.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingQueueStatus {
    /// Number of rows pending a retry.
    pub pending: u64,
    /// Number of rows that have failed at least once.
    pub failing: u64,
    /// Earliest `next_retry_at` timestamp (ms since epoch), or None if queue is empty.
    pub next_retry_at: Option<u64>,
}

/// Live worker status, exposed through `brain_health`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WorkerStatus {
    /// Whether the worker is currently paused due to rate limiting.
    pub rate_limited: bool,
    /// Seconds remaining in the current rate-limit pause (0 if not paused).
    pub pause_remaining_secs: u64,
    /// Cumulative count of hard failures (non-rate-limit errors).
    pub hard_failures: u32,
    /// Cumulative count of successful embeddings since last restart.
    pub total_embedded: u64,
    /// Cumulative count of rate-limit pauses.
    pub rate_limit_pauses: u32,
}

/// Shared atomic counters for worker status (lock-free, cheaply clonable).
#[derive(Debug, Clone)]
pub struct WorkerMetrics {
    pub hard_failures: Arc<AtomicU32>,
    pub total_embedded: Arc<AtomicU64>,
    pub rate_limit_pauses: Arc<AtomicU32>,
    /// Epoch-ms when the current pause ends (0 = not paused).
    pub pause_until_ms: Arc<AtomicU64>,
}

impl Default for WorkerMetrics {
    fn default() -> Self {
        Self {
            hard_failures: Arc::new(AtomicU32::new(0)),
            total_embedded: Arc::new(AtomicU64::new(0)),
            rate_limit_pauses: Arc::new(AtomicU32::new(0)),
            pause_until_ms: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl WorkerMetrics {
    pub fn snapshot(&self) -> WorkerStatus {
        let now_ms = now_epoch_ms();
        let pause_until = self.pause_until_ms.load(Ordering::Relaxed);
        let pause_remaining_secs = if pause_until > now_ms {
            (pause_until - now_ms) / 1000
        } else {
            0
        };
        WorkerStatus {
            rate_limited: pause_until > now_ms,
            pause_remaining_secs,
            hard_failures: self.hard_failures.load(Ordering::Relaxed),
            total_embedded: self.total_embedded.load(Ordering::Relaxed),
            rate_limit_pauses: self.rate_limit_pauses.load(Ordering::Relaxed),
        }
    }

    /// Returns true if the worker is currently rate-limit paused.
    pub fn is_paused(&self) -> bool {
        self.pause_until_ms.load(Ordering::Relaxed) > now_epoch_ms()
    }

    /// Set a pause until now + duration_secs.
    pub fn set_pause(&self, duration_secs: u64) {
        let until = now_epoch_ms() + duration_secs * 1000;
        self.pause_until_ms.store(until, Ordering::Relaxed);
        self.rate_limit_pauses.fetch_add(1, Ordering::Relaxed);
    }

    /// Clear the pause.
    pub fn clear_pause(&self) {
        self.pause_until_ms.store(0, Ordering::Relaxed);
    }
}

/// Detect whether an error message indicates a rate limit (soft pause).
pub fn is_rate_limit_error(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("429")
        || lower.contains("rate limit")
        || lower.contains("rate_limit")
        || lower.contains("too many requests")
        || lower.contains("model busy")
        || lower.contains("resource exhausted")
        || lower.contains("quota exceeded")
}

// ── Queue CRUD ─────────────────────────────────────────────────────────────────

/// Enqueue a memory ID for embedding retry. Idempotent — if already
/// present, this is a no-op.
pub fn enqueue(conn: &Connection, memory_id: i64) -> SqlResult<()> {
    let now_ms = now_epoch_ms();
    conn.execute(
        "INSERT OR IGNORE INTO pending_embeddings (memory_id, attempts, next_retry_at)
         VALUES (?1, 0, ?2)",
        params![memory_id, now_ms],
    )?;
    Ok(())
}

/// Enqueue multiple memory IDs at once. Idempotent per row.
pub fn enqueue_many(conn: &Connection, memory_ids: &[i64]) -> SqlResult<()> {
    if memory_ids.is_empty() {
        return Ok(());
    }
    let now_ms = now_epoch_ms();
    let mut stmt = conn.prepare_cached(
        "INSERT OR IGNORE INTO pending_embeddings (memory_id, attempts, next_retry_at)
         VALUES (?1, 0, ?2)",
    )?;
    for &id in memory_ids {
        stmt.execute(params![id, now_ms])?;
    }
    Ok(())
}

/// Remove a memory ID from the queue (embedding succeeded).
pub fn dequeue(conn: &Connection, memory_id: i64) -> SqlResult<()> {
    conn.execute(
        "DELETE FROM pending_embeddings WHERE memory_id = ?1",
        params![memory_id],
    )?;
    Ok(())
}

/// Record a failed embedding attempt: increment attempts, set error, and
/// compute next retry with exponential backoff (capped at 1 hour).
pub fn record_failure(conn: &Connection, memory_id: i64, error: &str) -> SqlResult<()> {
    let now_ms = now_epoch_ms();
    // Read current attempts to compute backoff
    let attempts: i64 = conn
        .query_row(
            "SELECT attempts FROM pending_embeddings WHERE memory_id = ?1",
            params![memory_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let new_attempts = attempts + 1;
    let backoff_secs = compute_backoff(new_attempts as u32);
    let next_retry = now_ms + backoff_secs * 1000;

    conn.execute(
        "UPDATE pending_embeddings
         SET attempts = ?1, last_error = ?2, next_retry_at = ?3
         WHERE memory_id = ?4",
        params![new_attempts, error, next_retry as i64, memory_id],
    )?;
    Ok(())
}

/// Fetch the next batch of memory IDs ready for retry (where
/// `next_retry_at <= now`). Returns `(memory_id, content)` pairs.
pub fn fetch_due_batch(conn: &Connection, limit: usize) -> SqlResult<Vec<(i64, String)>> {
    let now_ms = now_epoch_ms() as i64;
    let mut stmt = conn.prepare_cached(
        "SELECT pe.memory_id, m.content
         FROM pending_embeddings pe
         JOIN memories m ON m.id = pe.memory_id
         WHERE pe.next_retry_at <= ?1 AND m.embedding IS NULL
         ORDER BY pe.next_retry_at ASC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![now_ms, limit as i64], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    rows.collect()
}

/// Get the current queue status.
pub fn queue_status(conn: &Connection) -> SqlResult<EmbeddingQueueStatus> {
    let pending: u64 =
        conn.query_row("SELECT COUNT(*) FROM pending_embeddings", [], |r| r.get(0))?;
    let failing: u64 = conn.query_row(
        "SELECT COUNT(*) FROM pending_embeddings WHERE attempts > 0",
        [],
        |r| r.get(0),
    )?;
    let next_retry_at: Option<u64> = conn
        .query_row(
            "SELECT MIN(next_retry_at) FROM pending_embeddings",
            [],
            |r| r.get::<_, Option<i64>>(0),
        )?
        .map(|v| v as u64);
    Ok(EmbeddingQueueStatus {
        pending,
        failing,
        next_retry_at,
    })
}

/// Scan all memories with NULL embeddings that are NOT in the queue and enqueue them.
/// This is the "bootstrap" operation for existing databases that already have
/// unembedded memories from before Chunk 38.2 was deployed.
pub fn backfill_queue(conn: &Connection) -> SqlResult<u64> {
    let now_ms = now_epoch_ms() as i64;
    let count = conn.execute(
        "INSERT OR IGNORE INTO pending_embeddings (memory_id, attempts, next_retry_at)
         SELECT m.id, 0, ?1
         FROM memories m
         LEFT JOIN pending_embeddings pe ON pe.memory_id = m.id
         WHERE m.embedding IS NULL AND pe.memory_id IS NULL",
        params![now_ms],
    )?;
    Ok(count as u64)
}

// ── Background Worker ──────────────────────────────────────────────────────────

/// Spawn the embedding retry worker with concurrency control and graceful
/// shutdown. Returns the join handle and the shared metrics for health reporting.
pub fn spawn_worker(
    state: crate::AppState,
    shutdown: ShutdownRx,
) -> (tauri::async_runtime::JoinHandle<()>, WorkerMetrics) {
    let metrics = WorkerMetrics::default();
    let handle = spawn_worker_inner(state, shutdown, metrics.clone());
    (handle, metrics)
}

/// Spawn using externally-owned metrics (e.g. from AppState).
pub fn spawn_worker_with_metrics(
    state: crate::AppState,
    shutdown: ShutdownRx,
    metrics: WorkerMetrics,
) -> tauri::async_runtime::JoinHandle<()> {
    spawn_worker_inner(state, shutdown, metrics)
}

fn spawn_worker_inner(
    state: crate::AppState,
    mut shutdown: ShutdownRx,
    metrics_clone: WorkerMetrics,
) -> tauri::async_runtime::JoinHandle<()> {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENCY));

    tauri::async_runtime::spawn(async move {
        // Initial backfill: pick up any memories that were stored without
        // embeddings before this worker was introduced.
        {
            if let Ok(s) = state.memory_store.lock() {
                match backfill_queue(s.conn()) {
                    Ok(n) if n > 0 => {
                        eprintln!("[embed-queue] backfilled {n} unembedded memories into queue");
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("[embed-queue] backfill error: {e}"),
                }
            }
        }

        let mut interval = tokio::time::interval(WORKER_TICK_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut consecutive_rate_limits: u32 = 0;

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    eprintln!("[embed-queue] shutting down gracefully");
                    break;
                }
                _ = interval.tick() => {}
            }

            // If paused due to rate limiting, skip this tick.
            if metrics_clone.is_paused() {
                continue;
            }

            // Determine batch size from provider.
            let provider_name = state
                .brain_mode
                .lock()
                .ok()
                .and_then(|m| m.as_ref().map(provider_category));
            let batch_size = batch_size_for_provider(provider_name.as_deref());

            // Fetch the due batch.
            let batch = {
                let Ok(s) = state.memory_store.lock() else {
                    continue;
                };
                match fetch_due_batch(s.conn(), batch_size) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("[embed-queue] fetch error: {e}");
                        continue;
                    }
                }
            };

            if batch.is_empty() {
                continue;
            }

            // Acquire concurrency permit.
            let _permit = match semaphore.acquire().await {
                Ok(p) => p,
                Err(_) => break, // Semaphore closed — shutting down.
            };

            // Read brain mode/model.
            let mode = state.brain_mode.lock().ok().and_then(|m| m.clone());
            let model = state.active_brain.lock().ok().and_then(|m| m.clone());

            // Build text slice for batch embed.
            let texts: Vec<&str> = batch.iter().map(|(_, content)| content.as_str()).collect();

            let results = embed_batch_for_mode(
                &texts,
                mode.as_ref(),
                model.as_deref(),
                Some(batch_size),
            )
            .await;

            // Process results.
            let mut success_count = 0u32;
            let mut fail_count = 0u32;
            let mut batch_rate_limited = false;

            // If all results are None, treat it as a potential rate limit.
            let all_failed = results.iter().all(|r| r.is_none());
            if all_failed && !results.is_empty() {
                batch_rate_limited = true;
            }

            {
                let Ok(s) = state.memory_store.lock() else {
                    continue;
                };
                let conn = s.conn();
                for (i, emb_opt) in results.into_iter().enumerate() {
                    let memory_id = batch[i].0;
                    match emb_opt {
                        Some(emb) => {
                            if s.set_embedding(memory_id, &emb).is_ok() {
                                let _ = dequeue(conn, memory_id);
                                success_count += 1;
                            }
                        }
                        None => {
                            if batch_rate_limited {
                                // Soft pause: don't increment failure attempts.
                                let _ = record_soft_pause(conn, memory_id);
                            } else {
                                let _ = record_failure(conn, memory_id, "embed returned None");
                                metrics_clone.hard_failures.fetch_add(1, Ordering::Relaxed);
                            }
                            fail_count += 1;
                        }
                    }
                }
            }

            if success_count > 0 {
                metrics_clone
                    .total_embedded
                    .fetch_add(u64::from(success_count), Ordering::Relaxed);
                consecutive_rate_limits = 0;
                metrics_clone.clear_pause();
            }

            if batch_rate_limited {
                consecutive_rate_limits += 1;
                let pause_secs = (INITIAL_PAUSE_SECS
                    * 1u64.checked_shl(consecutive_rate_limits.saturating_sub(1))
                        .unwrap_or(u64::MAX))
                .min(MAX_PAUSE_SECS);
                metrics_clone.set_pause(pause_secs);
                eprintln!(
                    "[embed-queue] rate-limited, pausing for {pause_secs}s (consecutive: {consecutive_rate_limits})"
                );
            }

            if success_count > 0 || fail_count > 0 {
                eprintln!(
                    "[embed-queue] batch: {success_count} embedded, {fail_count} failed{}",
                    if batch_rate_limited { " (rate-limited)" } else { "" }
                );
            }
        }
    })
}

/// Record a soft pause for a memory (rate-limited, don't increment attempts).
/// Just push `next_retry_at` forward without incrementing the failure counter.
pub fn record_soft_pause(conn: &Connection, memory_id: i64) -> SqlResult<()> {
    let now_ms = now_epoch_ms();
    let next_retry = now_ms + INITIAL_PAUSE_SECS * 1000;
    conn.execute(
        "UPDATE pending_embeddings SET next_retry_at = ?1 WHERE memory_id = ?2",
        params![next_retry as i64, memory_id],
    )?;
    Ok(())
}

/// Determine a provider category string from BrainMode for batch size selection.
fn provider_category(mode: &crate::brain::BrainMode) -> String {
    match mode {
        crate::brain::BrainMode::LocalOllama { .. } => "ollama".to_string(),
        crate::brain::BrainMode::LocalLmStudio { .. } => "local".to_string(),
        crate::brain::BrainMode::PaidApi { .. } => "paid".to_string(),
        crate::brain::BrainMode::FreeApi { .. } => "free".to_string(),
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Compute exponential backoff: 10s * 2^(attempts-1), capped at MAX_BACKOFF_SECS.
fn compute_backoff(attempts: u32) -> u64 {
    let base = 10u64;
    let exp = base.saturating_mul(
        1u64.checked_shl(attempts.saturating_sub(1))
            .unwrap_or(u64::MAX),
    );
    exp.min(MAX_BACKOFF_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::schema::create_canonical_schema;
    use rusqlite::Connection;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_canonical_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn enqueue_and_dequeue() {
        let conn = test_db();
        // Insert a memory first
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
             VALUES (1, 'hello world', 'test', 3, 'fact', 1000, 'long', 1.0, 5)",
            [],
        )
        .unwrap();

        enqueue(&conn, 1).unwrap();
        let status = queue_status(&conn).unwrap();
        assert_eq!(status.pending, 1);
        assert_eq!(status.failing, 0);

        dequeue(&conn, 1).unwrap();
        let status = queue_status(&conn).unwrap();
        assert_eq!(status.pending, 0);
    }

    #[test]
    fn enqueue_is_idempotent() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
             VALUES (1, 'hello', 'test', 3, 'fact', 1000, 'long', 1.0, 5)",
            [],
        )
        .unwrap();

        enqueue(&conn, 1).unwrap();
        enqueue(&conn, 1).unwrap(); // Should not fail or duplicate
        let status = queue_status(&conn).unwrap();
        assert_eq!(status.pending, 1);
    }

    #[test]
    fn record_failure_increments_attempts_and_delays() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
             VALUES (1, 'hello', 'test', 3, 'fact', 1000, 'long', 1.0, 5)",
            [],
        )
        .unwrap();

        enqueue(&conn, 1).unwrap();
        record_failure(&conn, 1, "timeout").unwrap();

        let status = queue_status(&conn).unwrap();
        assert_eq!(status.failing, 1);

        // Verify attempts increased
        let attempts: i64 = conn
            .query_row(
                "SELECT attempts FROM pending_embeddings WHERE memory_id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(attempts, 1);
    }

    #[test]
    fn fetch_due_batch_respects_next_retry_at() {
        let conn = test_db();
        // Insert two memories
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
             VALUES (1, 'hello', 'test', 3, 'fact', 1000, 'long', 1.0, 5)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
             VALUES (2, 'world', 'test', 3, 'fact', 1000, 'long', 1.0, 5)",
            [],
        )
        .unwrap();

        // Enqueue both; id=2 with a far-future retry time
        enqueue(&conn, 1).unwrap();
        conn.execute(
            "INSERT INTO pending_embeddings (memory_id, attempts, next_retry_at)
             VALUES (2, 3, 9999999999999)",
            [],
        )
        .unwrap();

        let batch = fetch_due_batch(&conn, 10).unwrap();
        // Only id=1 should be due (id=2 is in the future)
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].0, 1);
        assert_eq!(batch[0].1, "hello");
    }

    #[test]
    fn backfill_queue_picks_up_unembedded() {
        let conn = test_db();
        // Insert 3 memories: 2 without embedding, 1 with
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
             VALUES (1, 'no embed', 'test', 3, 'fact', 1000, 'long', 1.0, 5)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
             VALUES (2, 'also no embed', 'test', 3, 'fact', 1000, 'long', 1.0, 5)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count, embedding)
             VALUES (3, 'has embed', 'test', 3, 'fact', 1000, 'long', 1.0, 5, X'00000000')",
            [],
        )
        .unwrap();

        let count = backfill_queue(&conn).unwrap();
        assert_eq!(count, 2);

        let status = queue_status(&conn).unwrap();
        assert_eq!(status.pending, 2);
    }

    #[test]
    fn backfill_skips_already_queued() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
             VALUES (1, 'no embed', 'test', 3, 'fact', 1000, 'long', 1.0, 5)",
            [],
        )
        .unwrap();

        enqueue(&conn, 1).unwrap();
        let count = backfill_queue(&conn).unwrap();
        assert_eq!(count, 0); // Already in queue
    }

    #[test]
    fn compute_backoff_caps_at_max() {
        assert_eq!(compute_backoff(1), 10); // 10 * 2^0
        assert_eq!(compute_backoff(2), 20); // 10 * 2^1
        assert_eq!(compute_backoff(3), 40); // 10 * 2^2
        assert_eq!(compute_backoff(7), 640); // 10 * 2^6
        assert_eq!(compute_backoff(10), MAX_BACKOFF_SECS); // capped
        assert_eq!(compute_backoff(50), MAX_BACKOFF_SECS); // still capped
    }

    #[test]
    fn fetch_due_excludes_already_embedded() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count, embedding)
             VALUES (1, 'already done', 'test', 3, 'fact', 1000, 'long', 1.0, 5, X'00000000')",
            [],
        )
        .unwrap();
        // Queue this despite having an embedding (edge case: someone re-embedded it)
        conn.execute(
            "INSERT INTO pending_embeddings (memory_id, attempts, next_retry_at) VALUES (1, 0, 0)",
            [],
        )
        .unwrap();

        let batch = fetch_due_batch(&conn, 10).unwrap();
        assert!(batch.is_empty()); // Excluded because embedding IS NOT NULL
    }

    // ── Chunk 41.7 tests ──────────────────────────────────────────────────────

    #[test]
    fn batch_size_per_provider_category() {
        assert_eq!(batch_size_for_provider(Some("ollama")), 8);
        assert_eq!(batch_size_for_provider(Some("local")), 8);
        assert_eq!(batch_size_for_provider(Some("free")), 32);
        assert_eq!(batch_size_for_provider(Some("pollinations")), 32);
        assert_eq!(batch_size_for_provider(Some("paid")), 128);
        assert_eq!(batch_size_for_provider(Some("openai")), 128);
        assert_eq!(batch_size_for_provider(Some("anthropic")), 128);
        assert_eq!(batch_size_for_provider(None), DEFAULT_BATCH_SIZE);
        assert_eq!(batch_size_for_provider(Some("unknown")), DEFAULT_BATCH_SIZE);
    }

    #[test]
    fn is_rate_limit_error_detects_patterns() {
        assert!(is_rate_limit_error("HTTP 429 Too Many Requests"));
        assert!(is_rate_limit_error("rate limit exceeded"));
        assert!(is_rate_limit_error("rate_limit_reached"));
        assert!(is_rate_limit_error("too many requests, retry later"));
        assert!(is_rate_limit_error("model busy, try again"));
        assert!(is_rate_limit_error("resource exhausted"));
        assert!(is_rate_limit_error("quota exceeded for this billing period"));
        // Negatives
        assert!(!is_rate_limit_error("connection refused"));
        assert!(!is_rate_limit_error("model not found"));
        assert!(!is_rate_limit_error("invalid input"));
    }

    #[test]
    fn record_soft_pause_does_not_increment_attempts() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO memories (id, content, tags, importance, memory_type, created_at, tier, decay_score, token_count)
             VALUES (1, 'hello', 'test', 3, 'fact', 1000, 'long', 1.0, 5)",
            [],
        )
        .unwrap();

        enqueue(&conn, 1).unwrap();
        record_soft_pause(&conn, 1).unwrap();

        let attempts: i64 = conn
            .query_row(
                "SELECT attempts FROM pending_embeddings WHERE memory_id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(attempts, 0, "soft pause must NOT increment attempts");

        // The next_retry_at should be pushed into the future
        let next_retry: i64 = conn
            .query_row(
                "SELECT next_retry_at FROM pending_embeddings WHERE memory_id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let now_ms = now_epoch_ms() as i64;
        assert!(next_retry > now_ms, "next_retry_at should be in the future after soft pause");
    }

    #[test]
    fn worker_metrics_snapshot_reports_pause() {
        let m = WorkerMetrics::default();
        let snap = m.snapshot();
        assert!(!snap.rate_limited);
        assert_eq!(snap.pause_remaining_secs, 0);
        assert_eq!(snap.hard_failures, 0);
        assert_eq!(snap.total_embedded, 0);
        assert_eq!(snap.rate_limit_pauses, 0);

        // Set a 60-second pause
        m.set_pause(60);
        let snap = m.snapshot();
        assert!(snap.rate_limited);
        assert!(snap.pause_remaining_secs >= 58); // allow 2s slack
        assert_eq!(snap.rate_limit_pauses, 1);

        // Clear
        m.clear_pause();
        let snap = m.snapshot();
        assert!(!snap.rate_limited);
        assert_eq!(snap.pause_remaining_secs, 0);
    }

    #[test]
    fn worker_metrics_accumulate_atomically() {
        let m = WorkerMetrics::default();
        m.hard_failures.fetch_add(3, Ordering::Relaxed);
        m.total_embedded.fetch_add(100, Ordering::Relaxed);
        m.set_pause(10);
        m.set_pause(20); // Second pause increments counter again

        let snap = m.snapshot();
        assert_eq!(snap.hard_failures, 3);
        assert_eq!(snap.total_embedded, 100);
        assert_eq!(snap.rate_limit_pauses, 2);
    }

    #[test]
    fn provider_category_maps_brain_modes() {
        use crate::brain::BrainMode;
        let ollama = BrainMode::LocalOllama {
            model: "nomic-embed-text".into(),
        };
        assert_eq!(provider_category(&ollama), "ollama");

        let paid = BrainMode::PaidApi {
            api_key: "sk-test".into(),
            model: "gpt-4".into(),
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
        };
        assert_eq!(provider_category(&paid), "paid");

        let free = BrainMode::FreeApi {
            model: Some("gemma-3".into()),
            provider_id: "pollinations".into(),
            api_key: None,
        };
        assert_eq!(provider_category(&free), "free");
    }
}
