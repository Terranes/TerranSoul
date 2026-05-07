//! Postgres persistence layer for the Hive relay.
//!
//! Stores accepted bundles and manages the job queue.

use sqlx::PgPool;

/// Database handle for the relay.
#[derive(Clone)]
pub struct RelayDb {
    pool: PgPool,
}

impl RelayDb {
    /// Create a new RelayDb from a connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run schema migrations (idempotent).
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::query(SCHEMA_SQL).execute(&self.pool).await?;
        Ok(())
    }

    /// Store a verified envelope payload (bundles are persisted, OPs are ephemeral).
    pub async fn store_bundle(
        &self,
        sender_id: &str,
        bundle_id: &str,
        hlc_counter: u64,
        payload: &[u8],
        signature: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO hive_bundles (sender_id, bundle_id, hlc_counter, payload, signature, received_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (bundle_id) DO NOTHING
            "#,
        )
        .bind(sender_id)
        .bind(bundle_id)
        .bind(hlc_counter as i64)
        .bind(payload)
        .bind(signature)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Fetch bundles for a device since a given HLC counter.
    pub async fn get_bundles_since(
        &self,
        since_hlc: u64,
        limit: i64,
    ) -> Result<Vec<StoredBundle>, sqlx::Error> {
        let rows = sqlx::query_as::<_, StoredBundle>(
            r#"
            SELECT sender_id, bundle_id, hlc_counter, payload, signature
            FROM hive_bundles
            WHERE hlc_counter > $1
            ORDER BY hlc_counter ASC
            LIMIT $2
            "#,
        )
        .bind(since_hlc as i64)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Enqueue a job envelope.
    pub async fn enqueue_job(
        &self,
        job_id: &str,
        sender_id: &str,
        payload: &[u8],
        signature: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO hive_jobs (job_id, sender_id, payload, signature, status, enqueued_at)
            VALUES ($1, $2, $3, $4, 'pending', NOW())
            ON CONFLICT (job_id) DO NOTHING
            "#,
        )
        .bind(job_id)
        .bind(sender_id)
        .bind(payload)
        .bind(signature)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Claim the next available job matching the given capabilities.
    ///
    /// Returns `None` if no matching job is available.
    pub async fn claim_job(
        &self,
        worker_id: &str,
    ) -> Result<Option<StoredJob>, sqlx::Error> {
        // Simple FIFO claim — capability matching is done at the application layer
        // after fetching pending jobs. The relay is intentionally simple.
        let job = sqlx::query_as::<_, StoredJob>(
            r#"
            UPDATE hive_jobs
            SET status = 'claimed', worker_id = $1, claimed_at = NOW()
            WHERE job_id = (
                SELECT job_id FROM hive_jobs
                WHERE status = 'pending'
                ORDER BY enqueued_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING job_id, sender_id, payload, signature
            "#,
        )
        .bind(worker_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(job)
    }

    /// Mark a job as completed.
    pub async fn complete_job(&self, job_id: &str, worker_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE hive_jobs
            SET status = 'completed', completed_at = NOW()
            WHERE job_id = $1 AND worker_id = $2 AND status = 'claimed'
            "#,
        )
        .bind(job_id)
        .bind(worker_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Count pending jobs.
    pub async fn pending_job_count(&self) -> Result<u64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM hive_jobs WHERE status = 'pending'",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0 as u64)
    }

    /// Track the highest HLC seen per device (for replay protection).
    pub async fn update_hlc_watermark(
        &self,
        device_id: &str,
        hlc_counter: u64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO hive_hlc_watermarks (device_id, highest_hlc)
            VALUES ($1, $2)
            ON CONFLICT (device_id) DO UPDATE SET highest_hlc = GREATEST(hive_hlc_watermarks.highest_hlc, $2)
            "#,
        )
        .bind(device_id)
        .bind(hlc_counter as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get the highest HLC watermark for a device (for replay protection).
    pub async fn get_hlc_watermark(&self, device_id: &str) -> Result<u64, sqlx::Error> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT highest_hlc FROM hive_hlc_watermarks WHERE device_id = $1",
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0 as u64).unwrap_or(0))
    }
}

/// A bundle row from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StoredBundle {
    pub sender_id: String,
    pub bundle_id: String,
    pub hlc_counter: i64,
    pub payload: Vec<u8>,
    pub signature: Vec<u8>,
}

/// A job row from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StoredJob {
    pub job_id: String,
    pub sender_id: String,
    pub payload: Vec<u8>,
    pub signature: Vec<u8>,
}

/// Schema SQL — run once on startup.
const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS hive_bundles (
    bundle_id   TEXT PRIMARY KEY,
    sender_id   TEXT NOT NULL,
    hlc_counter BIGINT NOT NULL,
    payload     BYTEA NOT NULL,
    signature   BYTEA NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_hive_bundles_hlc ON hive_bundles (hlc_counter);
CREATE INDEX IF NOT EXISTS idx_hive_bundles_sender ON hive_bundles (sender_id);

CREATE TABLE IF NOT EXISTS hive_jobs (
    job_id       TEXT PRIMARY KEY,
    sender_id    TEXT NOT NULL,
    payload      BYTEA NOT NULL,
    signature    BYTEA NOT NULL,
    status       TEXT NOT NULL DEFAULT 'pending',
    worker_id    TEXT,
    enqueued_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_at   TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_hive_jobs_status ON hive_jobs (status, enqueued_at);

CREATE TABLE IF NOT EXISTS hive_hlc_watermarks (
    device_id   TEXT PRIMARY KEY,
    highest_hlc BIGINT NOT NULL DEFAULT 0
);
"#;
