//! Self-healing embedding retry queue (Chunk 38.2).
//!
//! When memories are stored without embeddings (Ollama down, rate-limited,
//! or transient failure), they are enqueued here. A background worker drains
//! the queue in batches using the batch embedding endpoint from Chunk 38.1,
//! with exponential backoff on per-row failures (cap at 1 hour).
//!
//! The queue is a SQLite table (`pending_embeddings`) living in the same
//! database as the memory store. This module provides the queue CRUD and the
//! background worker loop.

use rusqlite::{params, Connection, Result as SqlResult};
use std::time::Duration;

use crate::brain::embed_batch_for_mode;

/// Maximum backoff between retries (1 hour).
const MAX_BACKOFF_SECS: u64 = 3600;

/// Default batch size for the retry worker.
const RETRY_BATCH_SIZE: usize = 32;

/// Worker tick interval (10 seconds).
pub const WORKER_TICK_INTERVAL: Duration = Duration::from_secs(10);

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

/// Spawn the embedding retry worker. Runs in a loop every 10 seconds,
/// draining the queue in batches of 32.
pub fn spawn_worker(state: crate::AppState) -> tauri::async_runtime::JoinHandle<()> {
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
        loop {
            interval.tick().await;

            // Fetch the due batch
            let batch = {
                let Ok(s) = state.memory_store.lock() else {
                    continue;
                };
                match fetch_due_batch(s.conn(), RETRY_BATCH_SIZE) {
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

            // Read brain mode/model
            let mode = state.brain_mode.lock().ok().and_then(|m| m.clone());
            let model = state.active_brain.lock().ok().and_then(|m| m.clone());

            // Build text slice for batch embed
            let texts: Vec<&str> = batch.iter().map(|(_, content)| content.as_str()).collect();

            let results = embed_batch_for_mode(
                &texts,
                mode.as_ref(),
                model.as_deref(),
                Some(RETRY_BATCH_SIZE),
            )
            .await;

            // Process results
            let mut success_count = 0u32;
            let mut fail_count = 0u32;
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
                            let _ = record_failure(conn, memory_id, "embed returned None");
                            fail_count += 1;
                        }
                    }
                }
            }

            if success_count > 0 || fail_count > 0 {
                eprintln!(
                    "[embed-queue] processed batch: {success_count} embedded, {fail_count} failed"
                );
            }
        }
    })
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
}
