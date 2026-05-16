use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum task duration before auto-pausing (30 minutes).
const MAX_TASK_DURATION_MS: u64 = 30 * 60 * 1000;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── Types ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Ingest,
    Crawl,
    Quest,
    Extract,
    ModelPull,
    Custom,
}

/// A background task with progress tracking and resume support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub kind: TaskKind,
    pub status: TaskStatus,
    /// 0–100 progress percentage.
    pub progress: u8,
    /// Human-readable description.
    pub description: String,
    /// Source identifier (file path, URL, quest ID, etc.).
    pub source: String,
    /// Serialized checkpoint state for resuming — opaque JSON.
    pub checkpoint: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    /// Total items to process (pages, chunks, etc.).
    pub total_items: usize,
    /// Items processed so far.
    pub processed_items: usize,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Progress event emitted to the frontend via Tauri events.
#[derive(Debug, Clone, Serialize)]
pub struct TaskProgressEvent {
    pub id: String,
    pub kind: TaskKind,
    pub status: TaskStatus,
    pub progress: u8,
    pub description: String,
    pub processed_items: usize,
    pub total_items: usize,
    pub error: Option<String>,
    /// BRAIN-REPO-RAG-2b: optional human-readable log line appended to the
    /// frontend's per-task debug log ring buffer. `None` for legacy events
    /// (skipped during serialization to keep payloads small).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_line: Option<String>,
}

impl From<&TaskInfo> for TaskProgressEvent {
    fn from(t: &TaskInfo) -> Self {
        TaskProgressEvent {
            id: t.id.clone(),
            kind: t.kind.clone(),
            status: t.status.clone(),
            progress: t.progress,
            description: t.description.clone(),
            processed_items: t.processed_items,
            total_items: t.total_items,
            error: t.error.clone(),
            log_line: None,
        }
    }
}

/// Serializable checkpoint for crawl tasks (used for resume).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlCheckpoint {
    pub visited: Vec<String>,
    pub queue: Vec<(String, usize)>,
    pub collected_text: Vec<String>,
    pub base_domain: String,
    pub max_depth: usize,
    pub max_pages: usize,
}

/// Serializable checkpoint for ingest tasks (used for resume).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestCheckpoint {
    pub source: String,
    pub tags: String,
    pub importance: i64,
    pub chunks: Vec<String>,
    pub next_chunk_index: usize,
}

// ── Task Manager ───────────────────────────────────────────────────────────────

/// Manages background tasks with SQLite persistence for resume.
pub struct TaskManager {
    /// In-flight tasks keyed by task ID.
    active: HashMap<String, TaskInfo>,
    /// Cancellation tokens — set to true to request cancellation.
    cancel_flags: HashMap<String, Arc<std::sync::atomic::AtomicBool>>,
    /// SQLite connection for persisting unfinished tasks.
    conn: Connection,
}

impl TaskManager {
    pub fn new(data_dir: &Path) -> Self {
        let conn = Connection::open(data_dir.join("tasks.db"))
            .unwrap_or_else(|_| Connection::open_in_memory().expect("SQLite fallback"));
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");
        Self::ensure_schema(&conn);
        let mut mgr = TaskManager {
            active: HashMap::new(),
            cancel_flags: HashMap::new(),
            conn,
        };
        mgr.load_unfinished();
        mgr
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        let conn = Connection::open_in_memory().expect("SQLite in-memory");
        Self::ensure_schema(&conn);
        TaskManager {
            active: HashMap::new(),
            cancel_flags: HashMap::new(),
            conn,
        }
    }

    fn ensure_schema(conn: &Connection) {
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id          TEXT PRIMARY KEY,
                kind        TEXT NOT NULL,
                status      TEXT NOT NULL,
                progress    INTEGER NOT NULL DEFAULT 0,
                description TEXT NOT NULL DEFAULT '',
                source      TEXT NOT NULL DEFAULT '',
                checkpoint  TEXT,
                created_at  INTEGER NOT NULL,
                updated_at  INTEGER NOT NULL,
                total_items INTEGER NOT NULL DEFAULT 0,
                processed_items INTEGER NOT NULL DEFAULT 0,
                error       TEXT
            );",
        );
    }

    /// Load paused/running tasks from SQLite on startup.
    fn load_unfinished(&mut self) {
        let mut stmt = match self.conn.prepare(
            "SELECT id, kind, status, progress, description, source, checkpoint, \
             created_at, updated_at, total_items, processed_items, error \
             FROM tasks WHERE status IN ('running', 'paused') ORDER BY created_at DESC",
        ) {
            Ok(s) => s,
            Err(_) => return,
        };

        let rows = stmt.query_map([], |row| {
            Ok(TaskInfo {
                id: row.get(0)?,
                kind: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(TaskKind::Custom),
                status: TaskStatus::Paused, // auto-pause on load (was interrupted)
                progress: row.get::<_, i64>(3)? as u8,
                description: row.get(4)?,
                source: row.get(5)?,
                checkpoint: row.get(6)?,
                created_at: row.get::<_, i64>(7)? as u64,
                updated_at: row.get::<_, i64>(8)? as u64,
                total_items: row.get::<_, i64>(9)? as usize,
                processed_items: row.get::<_, i64>(10)? as usize,
                error: row.get(11)?,
            })
        });

        if let Ok(rows) = rows {
            for task in rows.flatten() {
                self.active.insert(task.id.clone(), task);
            }
        }
    }

    /// Create and register a new task. Returns the task ID.
    pub fn create_task(&mut self, kind: TaskKind, description: &str, source: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let task = TaskInfo {
            id: id.clone(),
            kind: kind.clone(),
            status: TaskStatus::Running,
            progress: 0,
            description: description.to_string(),
            source: source.to_string(),
            checkpoint: None,
            created_at: now,
            updated_at: now,
            total_items: 0,
            processed_items: 0,
            error: None,
        };
        self.persist_task(&task);
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.cancel_flags.insert(id.clone(), flag);
        self.active.insert(id.clone(), task);
        id
    }

    /// Update a task's progress. Returns the updated TaskInfo.
    pub fn update_progress(
        &mut self,
        id: &str,
        progress: u8,
        processed: usize,
        total: usize,
    ) -> Option<TaskInfo> {
        if let Some(task) = self.active.get_mut(id) {
            task.progress = progress.min(100);
            task.processed_items = processed;
            task.total_items = total;
            task.updated_at = now_ms();

            // Auto-pause if running too long
            if task.updated_at - task.created_at > MAX_TASK_DURATION_MS
                && task.status == TaskStatus::Running
            {
                task.status = TaskStatus::Paused;
                task.error =
                    Some("Auto-paused: exceeded 30-minute limit. Resume to continue.".to_string());
            }

            let snapshot = task.clone();
            self.persist_task(&snapshot);
            Some(snapshot)
        } else {
            None
        }
    }

    /// Save a checkpoint for resuming later.
    pub fn save_checkpoint(&mut self, id: &str, checkpoint: &str) {
        if let Some(task) = self.active.get_mut(id) {
            task.checkpoint = Some(checkpoint.to_string());
            task.updated_at = now_ms();
            let snapshot = task.clone();
            self.persist_task(&snapshot);
        }
    }

    /// Mark a task as paused (saves state for resume).
    pub fn pause_task(&mut self, id: &str) -> Option<TaskInfo> {
        if let Some(task) = self.active.get_mut(id) {
            if task.status == TaskStatus::Running {
                task.status = TaskStatus::Paused;
                task.updated_at = now_ms();
            }
            let snapshot = task.clone();
            self.persist_task(&snapshot);
            Some(snapshot)
        } else {
            None
        }
    }

    /// Mark a task as completed.
    pub fn complete_task(&mut self, id: &str) -> Option<TaskInfo> {
        if let Some(task) = self.active.get_mut(id) {
            task.status = TaskStatus::Completed;
            task.progress = 100;
            task.updated_at = now_ms();
            let snapshot = task.clone();
            self.persist_task(&snapshot);
            self.cancel_flags.remove(id);
            Some(snapshot)
        } else {
            None
        }
    }

    /// Mark a task as failed.
    pub fn fail_task(&mut self, id: &str, error: &str) -> Option<TaskInfo> {
        if let Some(task) = self.active.get_mut(id) {
            task.status = TaskStatus::Failed;
            task.error = Some(error.to_string());
            task.updated_at = now_ms();
            let snapshot = task.clone();
            self.persist_task(&snapshot);
            self.cancel_flags.remove(id);
            Some(snapshot)
        } else {
            None
        }
    }

    /// Request cancellation of a running task.
    pub fn cancel_task(&mut self, id: &str) -> Option<TaskInfo> {
        if let Some(flag) = self.cancel_flags.get(id) {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if let Some(task) = self.active.get_mut(id) {
            task.status = TaskStatus::Cancelled;
            task.updated_at = now_ms();
            let snapshot = task.clone();
            self.persist_task(&snapshot);
            self.cancel_flags.remove(id);
            Some(snapshot)
        } else {
            None
        }
    }

    /// Get a cancellation flag for checking in the background loop.
    pub fn get_cancel_flag(&self, id: &str) -> Option<Arc<std::sync::atomic::AtomicBool>> {
        self.cancel_flags.get(id).cloned()
    }

    /// Resume a paused task — sets status back to Running.
    pub fn resume_task(&mut self, id: &str) -> Option<TaskInfo> {
        if let Some(task) = self.active.get_mut(id) {
            if task.status == TaskStatus::Paused {
                task.status = TaskStatus::Running;
                task.created_at = now_ms();
                task.updated_at = now_ms();
                task.error = None;
                let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
                self.cancel_flags.insert(id.to_string(), flag);
            }
            let snapshot = task.clone();
            self.persist_task(&snapshot);
            Some(snapshot)
        } else {
            None
        }
    }

    /// Get all tasks (active + recently completed).
    pub fn list_tasks(&self) -> Vec<TaskInfo> {
        self.active.values().cloned().collect()
    }

    /// Get a specific task by ID.
    pub fn get_task(&self, id: &str) -> Option<TaskInfo> {
        self.active.get(id).cloned()
    }

    /// Check if any task of a given kind is currently running.
    pub fn has_running_task(&self, kind: &TaskKind) -> Option<String> {
        self.active
            .values()
            .find(|t| t.kind == *kind && t.status == TaskStatus::Running)
            .map(|t| t.id.clone())
    }

    /// Check if the agent is busy with any running task.
    pub fn is_busy(&self) -> bool {
        self.active
            .values()
            .any(|t| t.status == TaskStatus::Running)
    }

    /// Remove completed/cancelled/failed tasks older than `max_age_ms`.
    pub fn cleanup(&mut self, max_age_ms: u64) {
        let now = now_ms();
        let to_remove: Vec<String> = self
            .active
            .iter()
            .filter(|(_, t)| {
                matches!(
                    t.status,
                    TaskStatus::Completed | TaskStatus::Cancelled | TaskStatus::Failed
                ) && now - t.updated_at > max_age_ms
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in &to_remove {
            self.active.remove(id);
            let _ = self
                .conn
                .execute("DELETE FROM tasks WHERE id = ?1", params![id]);
        }
    }

    fn persist_task(&self, task: &TaskInfo) {
        let kind_str = serde_json::to_string(&task.kind).unwrap_or_default();
        let status_str = serde_json::to_string(&task.status).unwrap_or_default();
        let _ = self.conn.execute(
            "INSERT OR REPLACE INTO tasks \
             (id, kind, status, progress, description, source, checkpoint, \
              created_at, updated_at, total_items, processed_items, error) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                task.id,
                kind_str,
                status_str,
                task.progress as i64,
                task.description,
                task.source,
                task.checkpoint,
                task.created_at as i64,
                task.updated_at as i64,
                task.total_items as i64,
                task.processed_items as i64,
                task.error,
            ],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_complete_task() {
        let mut mgr = TaskManager::in_memory();
        let id = mgr.create_task(TaskKind::Ingest, "Test ingest", "test.md");
        assert!(mgr.is_busy());
        assert_eq!(mgr.get_task(&id).unwrap().status, TaskStatus::Running);

        mgr.update_progress(&id, 50, 5, 10);
        assert_eq!(mgr.get_task(&id).unwrap().progress, 50);

        mgr.complete_task(&id);
        assert_eq!(mgr.get_task(&id).unwrap().status, TaskStatus::Completed);
        assert!(!mgr.is_busy());
    }

    #[test]
    fn cancel_task() {
        let mut mgr = TaskManager::in_memory();
        let id = mgr.create_task(TaskKind::Crawl, "Test crawl", "https://example.com");
        let flag = mgr.get_cancel_flag(&id).unwrap();
        assert!(!flag.load(std::sync::atomic::Ordering::Relaxed));

        mgr.cancel_task(&id);
        assert!(flag.load(std::sync::atomic::Ordering::Relaxed));
        assert_eq!(mgr.get_task(&id).unwrap().status, TaskStatus::Cancelled);
    }

    #[test]
    fn pause_and_resume() {
        let mut mgr = TaskManager::in_memory();
        let id = mgr.create_task(TaskKind::Ingest, "Long task", "big.pdf");
        mgr.save_checkpoint(&id, r#"{"next_chunk_index": 5}"#);
        mgr.pause_task(&id);

        let task = mgr.get_task(&id).unwrap();
        assert_eq!(task.status, TaskStatus::Paused);
        assert!(task.checkpoint.is_some());

        mgr.resume_task(&id);
        assert_eq!(mgr.get_task(&id).unwrap().status, TaskStatus::Running);
    }

    #[test]
    fn has_running_task_by_kind() {
        let mut mgr = TaskManager::in_memory();
        assert!(mgr.has_running_task(&TaskKind::Ingest).is_none());

        let id = mgr.create_task(TaskKind::Ingest, "test", "file.md");
        assert_eq!(mgr.has_running_task(&TaskKind::Ingest), Some(id.clone()));
        assert!(mgr.has_running_task(&TaskKind::Crawl).is_none());
    }

    #[test]
    fn fail_task_records_error() {
        let mut mgr = TaskManager::in_memory();
        let id = mgr.create_task(TaskKind::Crawl, "failing", "bad.url");
        mgr.fail_task(&id, "Network timeout");

        let task = mgr.get_task(&id).unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(task.error.as_deref(), Some("Network timeout"));
    }

    #[test]
    fn cleanup_removes_old_completed() {
        let mut mgr = TaskManager::in_memory();
        let id = mgr.create_task(TaskKind::Ingest, "done", "x");
        mgr.complete_task(&id);
        assert_eq!(mgr.list_tasks().len(), 1);

        // Won't remove recent
        mgr.cleanup(10_000);
        assert_eq!(mgr.list_tasks().len(), 1);

        // Force age by setting updated_at to 0
        if let Some(task) = mgr.active.get_mut(&id) {
            task.updated_at = 0;
        }
        mgr.cleanup(10_000);
        assert_eq!(mgr.list_tasks().len(), 0);
    }
}
