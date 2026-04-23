//! SQLite-backed durable workflow engine.
//!
//! Schema
//! ------
//! ```sql
//! CREATE TABLE workflow_events (
//!   id           INTEGER PRIMARY KEY AUTOINCREMENT,
//!   workflow_id  TEXT    NOT NULL,
//!   seq          INTEGER NOT NULL,
//!   kind         TEXT    NOT NULL,  -- discriminant of WorkflowEventKind
//!   payload      TEXT    NOT NULL,  -- JSON blob
//!   created_at   INTEGER NOT NULL,  -- epoch seconds
//!   UNIQUE (workflow_id, seq)
//! );
//! ```
//!
//! Every write is wrapped in a transaction so a crash mid-write never
//! produces a gap in `seq`. Callers observe `append()` as atomic.
//!
//! The in-memory side stores only a `(workflow_id -> last_status)` cache
//! so `query_status()` is O(1) after the first load.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// Opaque workflow identifier — hex UUID (32 chars).
pub type WorkflowId = String;

/// Every lifecycle event of a workflow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowEventKind {
    /// First event. Contains the workflow name (e.g. "cli_run") and an
    /// opaque input JSON the caller will use to resume.
    Started {
        name: String,
        input: serde_json::Value,
    },
    /// A sub-task was scheduled. `activity_id` is caller-defined and
    /// scoped to this workflow.
    ActivityScheduled { activity_id: String, kind: String },
    /// A sub-task succeeded.
    ActivityCompleted {
        activity_id: String,
        output: serde_json::Value,
    },
    /// A sub-task failed.
    ActivityFailed {
        activity_id: String,
        error: String,
    },
    /// Liveness marker so the UI can show "still running". Optional
    /// `message` lets the activity push human-readable progress.
    Heartbeat { message: Option<String> },
    /// Terminal: success.
    Completed { output: serde_json::Value },
    /// Terminal: failure.
    Failed { error: String },
    /// Terminal: user or system cancelled the workflow.
    Cancelled { reason: String },
}

impl WorkflowEventKind {
    /// Short discriminant stored in the `kind` column for fast filtering.
    pub fn tag(&self) -> &'static str {
        match self {
            WorkflowEventKind::Started { .. } => "started",
            WorkflowEventKind::ActivityScheduled { .. } => "activity_scheduled",
            WorkflowEventKind::ActivityCompleted { .. } => "activity_completed",
            WorkflowEventKind::ActivityFailed { .. } => "activity_failed",
            WorkflowEventKind::Heartbeat { .. } => "heartbeat",
            WorkflowEventKind::Completed { .. } => "completed",
            WorkflowEventKind::Failed { .. } => "failed",
            WorkflowEventKind::Cancelled { .. } => "cancelled",
        }
    }

    /// True if this event is a terminal state and no further events
    /// should be accepted for the workflow.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            WorkflowEventKind::Completed { .. }
                | WorkflowEventKind::Failed { .. }
                | WorkflowEventKind::Cancelled { .. }
        )
    }
}

/// A persisted event returned by `history()`.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowEvent {
    pub seq: i64,
    pub kind: WorkflowEventKind,
    pub created_at: i64,
}

/// Coarse-grained lifecycle state used by the UI.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    /// Event log exists but no terminal event yet — may or may not have
    /// an in-process handle attached.
    Running,
    Completed,
    Failed,
    Cancelled,
    /// Non-terminal workflow that the engine loaded from disk but has
    /// not yet re-attached an in-process handle to (e.g. right after app
    /// launch). Callers can still query `history()`.
    Resuming,
}

impl WorkflowStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled
        )
    }
}

/// A light summary returned by `list_workflows`.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowSummary {
    pub workflow_id: WorkflowId,
    pub name: String,
    pub status: WorkflowStatus,
    pub started_at: i64,
    pub last_event_at: i64,
    pub event_count: i64,
}

/// Durable workflow engine bound to a single SQLite file.
///
/// `attached` tracks whether the app has a live handle for the workflow
/// in-process. A `Running` workflow with `attached = false` is reported
/// as `Resuming` so the UI can tell the user "this was running when you
/// quit; do you want to resume it?".
pub struct WorkflowEngine {
    inner: Arc<Inner>,
}

struct Inner {
    conn: Mutex<Connection>,
    attached: Mutex<HashMap<WorkflowId, bool>>,
}

impl WorkflowEngine {
    /// Open (or create) the workflow log at `path`.
    ///
    /// If `path` is the string `":memory:"`, an in-memory SQLite database
    /// is used (tests only).
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = if path == Path::new(":memory:") {
            Connection::open_in_memory().map_err(|e| e.to_string())?
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            Connection::open(path).map_err(|e| e.to_string())?
        };
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS workflow_events (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                workflow_id TEXT    NOT NULL,
                seq         INTEGER NOT NULL,
                kind        TEXT    NOT NULL,
                payload     TEXT    NOT NULL,
                created_at  INTEGER NOT NULL,
                UNIQUE (workflow_id, seq)
            );
            CREATE INDEX IF NOT EXISTS ix_events_workflow
                ON workflow_events(workflow_id, seq);
            CREATE INDEX IF NOT EXISTS ix_events_kind
                ON workflow_events(kind);
            "#,
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            inner: Arc::new(Inner {
                conn: Mutex::new(conn),
                attached: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// Start a new workflow and record its `Started` event. Returns the
    /// fresh `WorkflowId`.
    pub async fn start(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<WorkflowId, String> {
        let id = uuid::Uuid::new_v4().simple().to_string();
        self.append(
            &id,
            WorkflowEventKind::Started {
                name: name.to_string(),
                input,
            },
        )
        .await?;
        self.inner.attached.lock().await.insert(id.clone(), true);
        Ok(id)
    }

    /// Append a new event to the workflow's history. Fails if the
    /// workflow already has a terminal event (nothing may be appended
    /// after `Completed`/`Failed`/`Cancelled`).
    pub async fn append(
        &self,
        workflow_id: &WorkflowId,
        kind: WorkflowEventKind,
    ) -> Result<i64, String> {
        let payload = serde_json::to_string(&kind).map_err(|e| e.to_string())?;
        let tag = kind.tag().to_string();
        let now = now_secs();
        let terminal = kind.is_terminal();
        let next_seq = {
            let mut guard = self.inner.conn.lock().await;
            let tx = guard.transaction().map_err(|e| e.to_string())?;
            // Enforce terminal lock.
            let last_kind: Option<String> = tx
                .query_row(
                    "SELECT kind FROM workflow_events
                        WHERE workflow_id = ?1
                        ORDER BY seq DESC LIMIT 1",
                    params![workflow_id],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            if let Some(last) = &last_kind {
                if matches!(last.as_str(), "completed" | "failed" | "cancelled") {
                    return Err(format!(
                        "workflow {workflow_id} is {last}; cannot append further events"
                    ));
                }
            }
            // Compute next seq.
            let next_seq: i64 = tx
                .query_row(
                    "SELECT COALESCE(MAX(seq), 0) + 1 FROM workflow_events
                        WHERE workflow_id = ?1",
                    params![workflow_id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO workflow_events (workflow_id, seq, kind, payload, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5)",
                params![workflow_id, next_seq, tag, payload, now],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            next_seq
        };
        // Transaction + connection guard are dropped here — safe to
        // await on the `attached` mutex without holding a !Send handle.
        if terminal {
            self.inner.attached.lock().await.remove(workflow_id);
        }
        Ok(next_seq)
    }

    /// Return the full event history for a workflow, ordered by `seq`.
    pub async fn history(&self, workflow_id: &str) -> Result<Vec<WorkflowEvent>, String> {
        let conn = self.inner.conn.lock().await;
        let mut stmt = conn
            .prepare(
                "SELECT seq, payload, created_at FROM workflow_events
                    WHERE workflow_id = ?1 ORDER BY seq ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![workflow_id], |row| {
                let seq: i64 = row.get(0)?;
                let payload: String = row.get(1)?;
                let created_at: i64 = row.get(2)?;
                let kind: WorkflowEventKind = serde_json::from_str(&payload).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(WorkflowEvent {
                    seq,
                    kind,
                    created_at,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    /// Coarse-grained status for a workflow. Returns `None` when the
    /// workflow is unknown.
    pub async fn status(&self, workflow_id: &str) -> Result<Option<WorkflowStatus>, String> {
        let conn = self.inner.conn.lock().await;
        let last: Option<String> = conn
            .query_row(
                "SELECT kind FROM workflow_events
                    WHERE workflow_id = ?1 ORDER BY seq DESC LIMIT 1",
                params![workflow_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        drop(conn);
        let Some(tag) = last else {
            return Ok(None);
        };
        let attached = self
            .inner
            .attached
            .lock()
            .await
            .get(workflow_id)
            .copied()
            .unwrap_or(false);
        Ok(Some(map_status(&tag, attached)))
    }

    /// All workflows that are not in a terminal state. Used on startup
    /// so the UI can offer "these workflows were running when you quit".
    pub async fn list_pending(&self) -> Result<Vec<WorkflowSummary>, String> {
        self.list_internal(false).await
    }

    /// All workflows ever recorded.
    pub async fn list_all(&self) -> Result<Vec<WorkflowSummary>, String> {
        self.list_internal(true).await
    }

    async fn list_internal(&self, include_terminal: bool) -> Result<Vec<WorkflowSummary>, String> {
        // Load row data with the SQLite lock held, then release it
        // before awaiting on the attached-set mutex — this keeps the
        // Future Send for tokio::spawn callers.
        let rows_data: Vec<(String, i64, i64, i64, String, String)> = {
            let conn = self.inner.conn.lock().await;
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT workflow_id,
                           MIN(created_at) AS started_at,
                           MAX(created_at) AS last_event_at,
                           COUNT(*)         AS event_count,
                           (SELECT kind FROM workflow_events e2
                               WHERE e2.workflow_id = workflow_events.workflow_id
                               ORDER BY seq DESC LIMIT 1) AS last_kind,
                           (SELECT payload FROM workflow_events e3
                               WHERE e3.workflow_id = workflow_events.workflow_id
                               ORDER BY seq ASC LIMIT 1) AS first_payload
                    FROM workflow_events
                    GROUP BY workflow_id
                    ORDER BY started_at DESC
                    "#,
                )
                .map_err(|e| e.to_string())?;
            let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                out.push((
                    row.get(0).map_err(|e| e.to_string())?,
                    row.get(1).map_err(|e| e.to_string())?,
                    row.get(2).map_err(|e| e.to_string())?,
                    row.get(3).map_err(|e| e.to_string())?,
                    row.get(4).map_err(|e| e.to_string())?,
                    row.get(5).map_err(|e| e.to_string())?,
                ));
            }
            out
        };
        let attached = self.inner.attached.lock().await.clone();
        let mut out = Vec::new();
        for (workflow_id, started_at, last_event_at, event_count, last_kind, first_payload) in
            rows_data
        {
            let attached_flag = attached.get(&workflow_id).copied().unwrap_or(false);
            let status = map_status(&last_kind, attached_flag);
            if !include_terminal && status.is_terminal() {
                continue;
            }
            let name = match serde_json::from_str::<WorkflowEventKind>(&first_payload) {
                Ok(WorkflowEventKind::Started { name, .. }) => name,
                _ => "unknown".to_string(),
            };
            out.push(WorkflowSummary {
                workflow_id,
                name,
                status,
                started_at,
                last_event_at,
                event_count,
            });
        }
        Ok(out)
    }

    /// Convenience wrapper that appends a `Heartbeat` event.
    pub async fn heartbeat(
        &self,
        workflow_id: &WorkflowId,
        message: Option<String>,
    ) -> Result<(), String> {
        self.append(workflow_id, WorkflowEventKind::Heartbeat { message })
            .await
            .map(|_| ())
    }

    /// Convenience wrapper that appends a `Completed` event.
    pub async fn complete(
        &self,
        workflow_id: &WorkflowId,
        output: serde_json::Value,
    ) -> Result<(), String> {
        self.append(workflow_id, WorkflowEventKind::Completed { output })
            .await
            .map(|_| ())
    }

    /// Convenience wrapper that appends a `Failed` event.
    pub async fn fail(&self, workflow_id: &WorkflowId, error: &str) -> Result<(), String> {
        self.append(
            workflow_id,
            WorkflowEventKind::Failed {
                error: error.to_string(),
            },
        )
        .await
        .map(|_| ())
    }

    /// Convenience wrapper that appends a `Cancelled` event. No-op if
    /// the workflow is already terminal.
    pub async fn cancel(&self, workflow_id: &WorkflowId, reason: &str) -> Result<(), String> {
        if let Some(s) = self.status(workflow_id).await? {
            if s.is_terminal() {
                return Ok(());
            }
        }
        self.append(
            workflow_id,
            WorkflowEventKind::Cancelled {
                reason: reason.to_string(),
            },
        )
        .await
        .map(|_| ())
    }

    /// The number of workflows currently attached (i.e. running in this
    /// process). Used by the RAM-aware concurrency cap.
    pub async fn attached_count(&self) -> usize {
        self.inner
            .attached
            .lock()
            .await
            .values()
            .filter(|v| **v)
            .count()
    }

    /// Clone a cheap handle for background tasks that need to append
    /// more events after the initial start.
    pub fn handle(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

fn map_status(tag: &str, attached: bool) -> WorkflowStatus {
    match tag {
        "completed" => WorkflowStatus::Completed,
        "failed" => WorkflowStatus::Failed,
        "cancelled" => WorkflowStatus::Cancelled,
        _ if attached => WorkflowStatus::Running,
        _ => WorkflowStatus::Resuming,
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    async fn new_engine() -> WorkflowEngine {
        WorkflowEngine::open(Path::new(":memory:")).unwrap()
    }

    #[tokio::test]
    async fn start_creates_started_event() {
        let e = new_engine().await;
        let id = e
            .start("test", serde_json::json!({"foo": "bar"}))
            .await
            .unwrap();
        let hist = e.history(&id).await.unwrap();
        assert_eq!(hist.len(), 1);
        assert!(matches!(hist[0].kind, WorkflowEventKind::Started { .. }));
        let status = e.status(&id).await.unwrap().unwrap();
        assert_eq!(status, WorkflowStatus::Running);
    }

    #[tokio::test]
    async fn append_sequences_events() {
        let e = new_engine().await;
        let id = e.start("seq", serde_json::json!({})).await.unwrap();
        e.heartbeat(&id, Some("tick".into())).await.unwrap();
        e.heartbeat(&id, None).await.unwrap();
        e.complete(&id, serde_json::json!({"result": 42}))
            .await
            .unwrap();
        let hist = e.history(&id).await.unwrap();
        assert_eq!(hist.len(), 4);
        assert_eq!(hist[0].seq, 1);
        assert_eq!(hist[3].seq, 4);
        assert_eq!(
            e.status(&id).await.unwrap().unwrap(),
            WorkflowStatus::Completed
        );
    }

    #[tokio::test]
    async fn cannot_append_after_terminal() {
        let e = new_engine().await;
        let id = e.start("term", serde_json::json!({})).await.unwrap();
        e.fail(&id, "boom").await.unwrap();
        let err = e.heartbeat(&id, None).await.unwrap_err();
        assert!(err.contains("failed") || err.contains("cannot"), "got: {err}");
    }

    #[tokio::test]
    async fn cancel_is_idempotent() {
        let e = new_engine().await;
        let id = e.start("cancel", serde_json::json!({})).await.unwrap();
        e.cancel(&id, "user").await.unwrap();
        e.cancel(&id, "again").await.unwrap(); // must not error
        assert_eq!(
            e.status(&id).await.unwrap().unwrap(),
            WorkflowStatus::Cancelled
        );
    }

    #[tokio::test]
    async fn restart_shows_pending_as_resuming() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = tmp.path().join("wf.sqlite");

        // First "app session" — start + heartbeat.
        let e1 = WorkflowEngine::open(&db).unwrap();
        let id = e1
            .start("long-run", serde_json::json!({"cmd": "codex"}))
            .await
            .unwrap();
        e1.heartbeat(&id, Some("compiling".into())).await.unwrap();
        assert_eq!(e1.attached_count().await, 1);
        drop(e1);

        // Second "app session" — event log survived on disk, but
        // `attached` is empty so status reports Resuming.
        let e2 = WorkflowEngine::open(&db).unwrap();
        assert_eq!(e2.attached_count().await, 0);
        let pending = e2.list_pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].workflow_id, id);
        assert_eq!(pending[0].status, WorkflowStatus::Resuming);
        assert_eq!(pending[0].name, "long-run");
        assert_eq!(pending[0].event_count, 2);

        // The caller can still append — e.g. finish the workflow.
        e2.complete(&id, serde_json::json!({"ok": true}))
            .await
            .unwrap();
        assert_eq!(
            e2.status(&id).await.unwrap().unwrap(),
            WorkflowStatus::Completed
        );
        // Completed workflows are excluded from list_pending.
        assert_eq!(e2.list_pending().await.unwrap().len(), 0);
        assert_eq!(e2.list_all().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn unknown_workflow_has_no_status() {
        let e = new_engine().await;
        assert!(e.status("nope").await.unwrap().is_none());
        assert!(e.history("nope").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn activity_events_roundtrip() {
        let e = new_engine().await;
        let id = e.start("acts", serde_json::json!({})).await.unwrap();
        e.append(
            &id,
            WorkflowEventKind::ActivityScheduled {
                activity_id: "a1".into(),
                kind: "cli_line".into(),
            },
        )
        .await
        .unwrap();
        e.append(
            &id,
            WorkflowEventKind::ActivityCompleted {
                activity_id: "a1".into(),
                output: serde_json::json!({"line": "hello"}),
            },
        )
        .await
        .unwrap();
        let hist = e.history(&id).await.unwrap();
        assert_eq!(hist.len(), 3);
        match &hist[1].kind {
            WorkflowEventKind::ActivityScheduled { activity_id, kind } => {
                assert_eq!(activity_id, "a1");
                assert_eq!(kind, "cli_line");
            }
            _ => panic!("wrong event kind at position 1"),
        }
    }

    #[tokio::test]
    async fn attached_count_drops_on_terminal() {
        let e = new_engine().await;
        let id = e.start("a", serde_json::json!({})).await.unwrap();
        assert_eq!(e.attached_count().await, 1);
        e.complete(&id, serde_json::json!(null)).await.unwrap();
        assert_eq!(e.attached_count().await, 0);
    }
}
