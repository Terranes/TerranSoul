//! Persistent SQLite task queue for the coding self-improve loop
//! (Chunk 28.6 — see `rules/milestones.md` Phase 28).
//!
//! Today the loop reads the next chunk by re-parsing
//! `rules/milestones.md` every cycle. That works for a single
//! interactive session but breaks down for the Phase 24 phone surface
//! and the MCP / CLI surfaces, which need to enqueue tasks
//! asynchronously and pick them up later. This module is the
//! durable, FIFO+priority+retry queue that backs all three.
//!
//! ## Storage
//!
//! `<data_dir>/coding_tasks.sqlite` — single table `coding_tasks`.
//! Schema is created on first open via `CREATE TABLE IF NOT EXISTS`,
//! so opening a fresh path is a no-op against an existing DB.
//!
//! ## Concurrency
//!
//! [`TaskQueue::claim_next`] uses a single SQL `UPDATE … RETURNING`
//! statement so two callers racing for the same task can never both
//! claim it — the loser sees `None`. Per-row state transitions
//! (`pending → in_progress → done|failed`) are also single statements,
//! making this safe under WAL with multiple concurrent claimers (each
//! worker process / thread).
//!
//! ## Why not Tokio channels?
//!
//! Channels are great for in-process pub/sub but lose state on
//! restart. A coding task may take an hour to plan/apply/review; we
//! need crash-safe persistence so a phone-enqueued "implement chunk
//! 28.5" survives a desktop restart and the user opening the laptop
//! the next morning still sees it pending.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
/// Errors surfaced by [`TaskQueue`].
#[derive(Debug, Error)]
pub enum QueueError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid status transition: {0}")]
    InvalidTransition(String),
    #[error("task not found: {0}")]
    NotFound(String),
}

/// Status of a queued task. Persisted as a lowercase string so SQL
/// inspection is human-readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Waiting to be claimed.
    Pending,
    /// Claimed by a worker but not yet finished.
    InProgress,
    /// Finished successfully — `result` populated.
    Done,
    /// Exhausted retries — `error` populated.
    Failed,
    /// Cancelled by user or supervisor before completion.
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }

    fn from_str(s: &str) -> Result<Self, QueueError> {
        match s {
            "pending" => Ok(TaskStatus::Pending),
            "in_progress" => Ok(TaskStatus::InProgress),
            "done" => Ok(TaskStatus::Done),
            "failed" => Ok(TaskStatus::Failed),
            "cancelled" => Ok(TaskStatus::Cancelled),
            other => Err(QueueError::InvalidTransition(format!(
                "unknown status `{other}`"
            ))),
        }
    }
}

/// A row in the queue. `priority`: higher fires first; ties broken by
/// `enqueued_at ASC` (FIFO within the same priority).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskRow {
    pub id: String,
    pub description: String,
    /// Free-form output-shape hint passed through to
    /// `coding::workflow::run_coding_task`. Not parsed here.
    pub output_shape: String,
    pub priority: i64,
    pub status: TaskStatus,
    pub attempts: u32,
    pub max_attempts: u32,
    pub enqueued_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub error: Option<String>,
    pub result: Option<String>,
    /// Free-form actor identifier (e.g. `"phone"`, `"mcp"`, `"local"`).
    pub enqueued_by: String,
}

/// Caller-supplied parameters for [`TaskQueue::enqueue`]. The queue
/// fills in `id`, `enqueued_at`, `attempts`, and `status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    pub description: String,
    pub output_shape: String,
    #[serde(default)]
    pub priority: i64,
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_enqueued_by")]
    pub enqueued_by: String,
}

fn default_max_attempts() -> u32 {
    3
}

fn default_enqueued_by() -> String {
    "local".to_string()
}

/// SQLite-backed task queue.
pub struct TaskQueue {
    conn: Connection,
}

impl std::fmt::Debug for TaskQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskQueue").finish_non_exhaustive()
    }
}

impl TaskQueue {
    /// Open (or create) a queue at `path`. Creates the schema on first
    /// use; subsequent opens are no-ops. Idempotent.
    pub fn open(path: &Path) -> Result<Self, QueueError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        Self::init(&conn)?;
        Ok(Self { conn })
    }

    /// In-memory queue for tests.
    pub fn open_in_memory() -> Result<Self, QueueError> {
        let conn = Connection::open_in_memory()?;
        Self::init(&conn)?;
        Ok(Self { conn })
    }

    fn init(conn: &Connection) -> Result<(), QueueError> {
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous  = NORMAL;
            CREATE TABLE IF NOT EXISTS coding_tasks (
                id            TEXT PRIMARY KEY,
                description   TEXT    NOT NULL,
                output_shape  TEXT    NOT NULL,
                priority      INTEGER NOT NULL DEFAULT 0,
                status        TEXT    NOT NULL DEFAULT 'pending',
                attempts      INTEGER NOT NULL DEFAULT 0,
                max_attempts  INTEGER NOT NULL DEFAULT 3,
                enqueued_at   INTEGER NOT NULL,
                started_at    INTEGER,
                finished_at   INTEGER,
                error         TEXT,
                result        TEXT,
                enqueued_by   TEXT    NOT NULL DEFAULT 'local'
            );
            CREATE INDEX IF NOT EXISTS idx_coding_tasks_pickup
                ON coding_tasks(status, priority DESC, enqueued_at ASC);
            "#,
        )?;
        Ok(())
    }

    /// Enqueue a new task. Returns the assigned id.
    pub fn enqueue(&self, task: NewTask) -> Result<String, QueueError> {
        self.enqueue_with_now(task, now_unix_ms())
    }

    /// Test-friendly variant that takes the timestamp explicitly.
    pub fn enqueue_with_now(&self, task: NewTask, now_ms: i64) -> Result<String, QueueError> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO coding_tasks (id, description, output_shape, priority,
                status, attempts, max_attempts, enqueued_at, enqueued_by)
             VALUES (?1, ?2, ?3, ?4, 'pending', 0, ?5, ?6, ?7)",
            params![
                id,
                task.description,
                task.output_shape,
                task.priority,
                task.max_attempts,
                now_ms,
                task.enqueued_by,
            ],
        )?;
        Ok(id)
    }

    /// Atomically pick the highest-priority `pending` task and flip it
    /// to `in_progress`. Returns `None` when the queue is empty.
    ///
    /// Two concurrent callers can race; SQLite serialises the
    /// `UPDATE` so at most one wins per row.
    pub fn claim_next(&self) -> Result<Option<TaskRow>, QueueError> {
        self.claim_next_with_now(now_unix_ms())
    }

    pub fn claim_next_with_now(&self, now_ms: i64) -> Result<Option<TaskRow>, QueueError> {
        // SQLite's UPDATE … RETURNING makes the claim atomic under
        // WAL: the row is locked for the duration of the update and
        // any racing caller sees the new status before considering it.
        let row = self
            .conn
            .query_row(
                "UPDATE coding_tasks
             SET status      = 'in_progress',
                 started_at  = ?1,
                 attempts    = attempts + 1
             WHERE id = (
                 SELECT id FROM coding_tasks
                 WHERE status = 'pending'
                 ORDER BY priority DESC, enqueued_at ASC
                 LIMIT 1
             )
             RETURNING id, description, output_shape, priority, status,
                       attempts, max_attempts, enqueued_at, started_at,
                       finished_at, error, result, enqueued_by",
                params![now_ms],
                row_to_task,
            )
            .optional()?;
        Ok(row)
    }

    /// Mark a claimed task as completed successfully.
    pub fn complete(&self, id: &str, result: Option<&str>) -> Result<(), QueueError> {
        self.complete_with_now(id, result, now_unix_ms())
    }

    pub fn complete_with_now(
        &self,
        id: &str,
        result: Option<&str>,
        now_ms: i64,
    ) -> Result<(), QueueError> {
        let n = self.conn.execute(
            "UPDATE coding_tasks
             SET status      = 'done',
                 finished_at = ?1,
                 result      = ?2,
                 error       = NULL
             WHERE id = ?3 AND status = 'in_progress'",
            params![now_ms, result, id],
        )?;
        if n == 0 {
            return Err(QueueError::InvalidTransition(format!(
                "complete: task `{id}` not in_progress"
            )));
        }
        Ok(())
    }

    /// Record a failed attempt. If `attempts < max_attempts`, the row
    /// goes back to `pending` for retry; otherwise to `failed`.
    pub fn fail(&self, id: &str, error: &str) -> Result<TaskStatus, QueueError> {
        self.fail_with_now(id, error, now_unix_ms())
    }

    pub fn fail_with_now(
        &self,
        id: &str,
        error: &str,
        now_ms: i64,
    ) -> Result<TaskStatus, QueueError> {
        // Read the current row to decide retry vs terminal-failure,
        // then issue a single UPDATE. This is two statements but the
        // window between them is harmless — only the worker that
        // claimed this row is supposed to call `fail`/`complete`, so
        // there is no race against another claimer.
        let (attempts, max_attempts, status) = self
            .conn
            .query_row(
                "SELECT attempts, max_attempts, status FROM coding_tasks WHERE id = ?1",
                params![id],
                |r| {
                    Ok((
                        r.get::<_, u32>(0)?,
                        r.get::<_, u32>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| QueueError::NotFound(id.to_string()))?;

        if status != "in_progress" {
            return Err(QueueError::InvalidTransition(format!(
                "fail: task `{id}` not in_progress (was `{status}`)"
            )));
        }

        let new_status = if attempts >= max_attempts {
            TaskStatus::Failed
        } else {
            TaskStatus::Pending
        };

        let n = self.conn.execute(
            "UPDATE coding_tasks
             SET status      = ?1,
                 finished_at = CASE WHEN ?1 = 'failed' THEN ?2 ELSE NULL END,
                 started_at  = CASE WHEN ?1 = 'pending' THEN NULL ELSE started_at END,
                 error       = ?3
             WHERE id = ?4",
            params![new_status.as_str(), now_ms, error, id],
        )?;
        if n == 0 {
            return Err(QueueError::NotFound(id.to_string()));
        }
        Ok(new_status)
    }

    /// Cancel a task, regardless of current status. No-op if already
    /// terminal (`done` / `failed` / `cancelled`).
    pub fn cancel(&self, id: &str) -> Result<bool, QueueError> {
        self.cancel_with_now(id, now_unix_ms())
    }

    pub fn cancel_with_now(&self, id: &str, now_ms: i64) -> Result<bool, QueueError> {
        let n = self.conn.execute(
            "UPDATE coding_tasks
             SET status      = 'cancelled',
                 finished_at = ?1
             WHERE id = ?2 AND status IN ('pending', 'in_progress')",
            params![now_ms, id],
        )?;
        Ok(n > 0)
    }

    pub fn get(&self, id: &str) -> Result<Option<TaskRow>, QueueError> {
        let row = self
            .conn
            .query_row(
                "SELECT id, description, output_shape, priority, status,
                    attempts, max_attempts, enqueued_at, started_at,
                    finished_at, error, result, enqueued_by
             FROM coding_tasks WHERE id = ?1",
                params![id],
                row_to_task,
            )
            .optional()?;
        Ok(row)
    }

    /// List rows in the given status, newest-enqueued first. Pass
    /// `None` for "all statuses".
    pub fn list(&self, status: Option<TaskStatus>) -> Result<Vec<TaskRow>, QueueError> {
        let mut stmt;
        let rows = if let Some(s) = status {
            stmt = self.conn.prepare(
                "SELECT id, description, output_shape, priority, status,
                        attempts, max_attempts, enqueued_at, started_at,
                        finished_at, error, result, enqueued_by
                 FROM coding_tasks WHERE status = ?1
                 ORDER BY enqueued_at DESC",
            )?;
            stmt.query_map(params![s.as_str()], row_to_task)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt = self.conn.prepare(
                "SELECT id, description, output_shape, priority, status,
                        attempts, max_attempts, enqueued_at, started_at,
                        finished_at, error, result, enqueued_by
                 FROM coding_tasks ORDER BY enqueued_at DESC",
            )?;
            stmt.query_map([], row_to_task)?
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }

    /// Number of rows in each status. Useful for the Brain panel
    /// "queue depth" display and for tests.
    pub fn counts_by_status(&self) -> Result<std::collections::BTreeMap<String, i64>, QueueError> {
        let mut stmt = self
            .conn
            .prepare("SELECT status, COUNT(*) FROM coding_tasks GROUP BY status")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        let mut out = std::collections::BTreeMap::new();
        for r in rows {
            let (k, v) = r?;
            out.insert(k, v);
        }
        Ok(out)
    }

    /// Delete all `done`/`failed`/`cancelled` rows finished before
    /// `cutoff_ms`. Returns the number of rows removed.
    pub fn purge_finished_before(&self, cutoff_ms: i64) -> Result<usize, QueueError> {
        let n = self.conn.execute(
            "DELETE FROM coding_tasks
             WHERE status IN ('done', 'failed', 'cancelled')
               AND finished_at IS NOT NULL
               AND finished_at < ?1",
            params![cutoff_ms],
        )?;
        Ok(n)
    }
}

fn row_to_task(r: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRow> {
    let status_s: String = r.get(4)?;
    let status = TaskStatus::from_str(&status_s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::other(e.to_string())),
        )
    })?;
    Ok(TaskRow {
        id: r.get(0)?,
        description: r.get(1)?,
        output_shape: r.get(2)?,
        priority: r.get(3)?,
        status,
        attempts: r.get(5)?,
        max_attempts: r.get(6)?,
        enqueued_at: r.get(7)?,
        started_at: r.get(8)?,
        finished_at: r.get(9)?,
        error: r.get(10)?,
        result: r.get(11)?,
        enqueued_by: r.get(12)?,
    })
}

fn now_unix_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(desc: &str, prio: i64) -> NewTask {
        NewTask {
            description: desc.to_string(),
            output_shape: "text".to_string(),
            priority: prio,
            max_attempts: 3,
            enqueued_by: "test".to_string(),
        }
    }

    #[test]
    fn open_in_memory_creates_schema() {
        let q = TaskQueue::open_in_memory().unwrap();
        assert!(q.list(None).unwrap().is_empty());
        assert!(q.claim_next().unwrap().is_none());
    }

    #[test]
    fn open_is_idempotent() {
        let q = TaskQueue::open_in_memory().unwrap();
        // Re-init shouldn't fail or wipe data.
        TaskQueue::init(&q.conn).unwrap();
        TaskQueue::init(&q.conn).unwrap();
        assert!(q.list(None).unwrap().is_empty());
    }

    #[test]
    fn enqueue_returns_unique_ids() {
        let q = TaskQueue::open_in_memory().unwrap();
        let a = q.enqueue(task("a", 0)).unwrap();
        let b = q.enqueue(task("b", 0)).unwrap();
        assert_ne!(a, b);
        assert_eq!(q.list(None).unwrap().len(), 2);
    }

    #[test]
    fn fifo_within_same_priority() {
        let q = TaskQueue::open_in_memory().unwrap();
        let a = q.enqueue_with_now(task("a", 0), 100).unwrap();
        let b = q.enqueue_with_now(task("b", 0), 200).unwrap();
        let claimed = q.claim_next().unwrap().unwrap();
        assert_eq!(claimed.id, a, "older should win FIFO tie-break");
        let claimed2 = q.claim_next().unwrap().unwrap();
        assert_eq!(claimed2.id, b);
    }

    #[test]
    fn higher_priority_wins() {
        let q = TaskQueue::open_in_memory().unwrap();
        let _low = q.enqueue_with_now(task("low", 0), 100).unwrap();
        let high = q.enqueue_with_now(task("high", 10), 200).unwrap();
        let claimed = q.claim_next().unwrap().unwrap();
        assert_eq!(claimed.id, high);
        assert_eq!(claimed.status, TaskStatus::InProgress);
        assert!(claimed.started_at.is_some());
        assert_eq!(claimed.attempts, 1);
    }

    #[test]
    fn claim_next_is_none_when_no_pending() {
        let q = TaskQueue::open_in_memory().unwrap();
        let _id = q.enqueue(task("a", 0)).unwrap();
        let _ = q.claim_next().unwrap().unwrap();
        // Now nothing pending.
        assert!(q.claim_next().unwrap().is_none());
    }

    #[test]
    fn complete_marks_done_and_records_result() {
        let q = TaskQueue::open_in_memory().unwrap();
        let id = q.enqueue(task("a", 0)).unwrap();
        let _ = q.claim_next().unwrap().unwrap();
        q.complete(&id, Some("OK")).unwrap();
        let row = q.get(&id).unwrap().unwrap();
        assert_eq!(row.status, TaskStatus::Done);
        assert_eq!(row.result.as_deref(), Some("OK"));
        assert!(row.finished_at.is_some());
        assert!(row.error.is_none());
    }

    #[test]
    fn complete_rejects_non_in_progress() {
        let q = TaskQueue::open_in_memory().unwrap();
        let id = q.enqueue(task("a", 0)).unwrap();
        // Not yet claimed.
        let err = q.complete(&id, Some("OK")).unwrap_err();
        assert!(matches!(err, QueueError::InvalidTransition(_)));
    }

    #[test]
    fn fail_retries_until_max_attempts() {
        let q = TaskQueue::open_in_memory().unwrap();
        let id = q
            .enqueue(NewTask {
                description: "a".into(),
                output_shape: "text".into(),
                priority: 0,
                max_attempts: 2,
                enqueued_by: "test".into(),
            })
            .unwrap();

        // Attempt 1 — re-queue.
        let _ = q.claim_next().unwrap().unwrap();
        let s1 = q.fail(&id, "boom").unwrap();
        assert_eq!(s1, TaskStatus::Pending);
        let row = q.get(&id).unwrap().unwrap();
        assert_eq!(row.attempts, 1);
        assert_eq!(row.status, TaskStatus::Pending);
        assert!(row.started_at.is_none(), "started_at cleared on retry");
        assert_eq!(row.error.as_deref(), Some("boom"));

        // Attempt 2 (== max) — terminal failure.
        let _ = q.claim_next().unwrap().unwrap();
        let s2 = q.fail(&id, "boom2").unwrap();
        assert_eq!(s2, TaskStatus::Failed);
        let row = q.get(&id).unwrap().unwrap();
        assert_eq!(row.status, TaskStatus::Failed);
        assert_eq!(row.attempts, 2);
        assert_eq!(row.error.as_deref(), Some("boom2"));
        assert!(row.finished_at.is_some());
    }

    #[test]
    fn fail_rejects_non_in_progress() {
        let q = TaskQueue::open_in_memory().unwrap();
        let id = q.enqueue(task("a", 0)).unwrap();
        // Not claimed.
        let err = q.fail(&id, "x").unwrap_err();
        assert!(matches!(err, QueueError::InvalidTransition(_)));
    }

    #[test]
    fn fail_unknown_id_returns_not_found() {
        let q = TaskQueue::open_in_memory().unwrap();
        let err = q.fail("nope", "x").unwrap_err();
        assert!(matches!(err, QueueError::NotFound(_)));
    }

    #[test]
    fn cancel_pending_or_in_progress_only() {
        let q = TaskQueue::open_in_memory().unwrap();
        let id_p = q.enqueue(task("pending", 0)).unwrap();
        let id_i = q.enqueue(task("inflight", 0)).unwrap();
        // Claim the second so it becomes in_progress.
        // Claim order is FIFO so first claim returns id_p.
        let first = q.claim_next().unwrap().unwrap();
        assert_eq!(first.id, id_p);
        let second = q.claim_next().unwrap().unwrap();
        assert_eq!(second.id, id_i);

        // Move id_p to done so cancel is a no-op for it.
        q.complete(&id_p, None).unwrap();
        assert!(!q.cancel(&id_p).unwrap(), "done is terminal");

        // id_i is in_progress → cancel works.
        assert!(q.cancel(&id_i).unwrap());
        let row = q.get(&id_i).unwrap().unwrap();
        assert_eq!(row.status, TaskStatus::Cancelled);
    }

    #[test]
    fn list_filters_by_status() {
        let q = TaskQueue::open_in_memory().unwrap();
        let id1 = q.enqueue(task("a", 0)).unwrap();
        let _id2 = q.enqueue(task("b", 0)).unwrap();
        let _ = q.claim_next().unwrap();
        q.complete(&id1, None).unwrap();

        let pending = q.list(Some(TaskStatus::Pending)).unwrap();
        assert_eq!(pending.len(), 1);
        let done = q.list(Some(TaskStatus::Done)).unwrap();
        assert_eq!(done.len(), 1);
        let all = q.list(None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn counts_by_status_aggregates() {
        let q = TaskQueue::open_in_memory().unwrap();
        let id = q.enqueue(task("a", 0)).unwrap();
        let _ = q.enqueue(task("b", 0)).unwrap();
        let _ = q.claim_next().unwrap(); // claims a
        q.complete(&id, None).unwrap();

        let counts = q.counts_by_status().unwrap();
        assert_eq!(counts.get("done").copied().unwrap_or(0), 1);
        assert_eq!(counts.get("pending").copied().unwrap_or(0), 1);
    }

    #[test]
    fn purge_finished_before_keeps_active() {
        let q = TaskQueue::open_in_memory().unwrap();
        let id_done = q.enqueue_with_now(task("done", 0), 100).unwrap();
        let _id_pending = q.enqueue_with_now(task("pending", 0), 200).unwrap();
        let _ = q.claim_next_with_now(150).unwrap(); // claims done
        q.complete_with_now(&id_done, None, 300).unwrap();

        // Purge cutoff = 1000 → only the done row qualifies.
        let removed = q.purge_finished_before(1000).unwrap();
        assert_eq!(removed, 1);
        let remaining = q.list(None).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].status, TaskStatus::Pending);
    }

    #[test]
    fn purge_respects_cutoff() {
        let q = TaskQueue::open_in_memory().unwrap();
        let id = q.enqueue_with_now(task("a", 0), 100).unwrap();
        let _ = q.claim_next_with_now(150).unwrap();
        q.complete_with_now(&id, None, 300).unwrap();

        // Cutoff before finished_at → keeps the row.
        let removed = q.purge_finished_before(200).unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn enqueue_persists_to_disk_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("q.sqlite");
        {
            let q = TaskQueue::open(&path).unwrap();
            q.enqueue(task("a", 0)).unwrap();
        }
        // Re-open — row survives.
        let q = TaskQueue::open(&path).unwrap();
        assert_eq!(q.list(None).unwrap().len(), 1);
    }

    #[test]
    fn task_status_round_trips_string() {
        for s in [
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::Done,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ] {
            assert_eq!(TaskStatus::from_str(s.as_str()).unwrap(), s);
        }
        assert!(TaskStatus::from_str("garbage").is_err());
    }

    /// Even with two queue handles claiming concurrently, each row
    /// can only be claimed once thanks to the atomic UPDATE…RETURNING.
    #[test]
    fn concurrent_claim_does_not_double_pick() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("q.sqlite");
        let q1 = TaskQueue::open(&path).unwrap();
        for i in 0..5 {
            q1.enqueue(task(&format!("t{i}"), 0)).unwrap();
        }
        let q2 = TaskQueue::open(&path).unwrap();

        // Drain via both handles, alternating.
        let mut seen = std::collections::HashSet::new();
        loop {
            let claimed = q1.claim_next().unwrap();
            if let Some(t) = claimed {
                assert!(seen.insert(t.id));
            } else if q2.claim_next().unwrap().is_none() {
                break;
            }
            let claimed = q2.claim_next().unwrap();
            if let Some(t) = claimed {
                assert!(seen.insert(t.id));
            }
        }
        assert_eq!(seen.len(), 5);
    }
}
