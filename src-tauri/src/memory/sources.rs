//! Memory sources registry (BRAIN-REPO-RAG-1a).
//!
//! Each row in `memory_sources` represents one logical "brain origin"
//! the user can flip between in the Memory panel and in MCP queries.
//!
//! * `kind = 'self'` — the always-present TerranSoul brain. There is
//!   exactly one row with `id = 'self'`, seeded by the v22 migration.
//! * `kind = 'repo'` — a per-source repository checkout that lives in
//!   `<app_data>/mcp-data/repos/<id>/` with its own SQLite + ANN sidecars
//!   (populated by chunks BRAIN-REPO-RAG-1b and 1c).
//! * `kind = 'topic'` — reserved for the future "topical brain" cluster
//!   feature; not yet exposed to UI.
//!
//! This module owns only the registry CRUD. Per-source data (checkout,
//! `memories.db`, `ann.usearch`, `manifest.json`) is materialized by the
//! ingest pipeline in later chunks. Deleting the `'self'` row is rejected
//! at the application layer; the schema permits it only so administrative
//! repair tools can still operate on the table.

use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};

/// The canonical id of the built-in TerranSoul brain. This row is seeded
/// by the v22 migration and may not be deleted via the public API.
pub const SELF_SOURCE_ID: &str = "self";

/// Kind discriminator for `memory_sources.kind`. Matches the SQL `CHECK`
/// constraint.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemorySourceKind {
    /// The built-in TerranSoul brain (singleton, id = `"self"`).
    #[serde(rename = "self")]
    SelfBrain,
    /// A user-added repository, cloned under `mcp-data/repos/<id>/`.
    Repo,
    /// Reserved for future topical brains.
    Topic,
}

impl MemorySourceKind {
    fn as_sql(self) -> &'static str {
        match self {
            Self::SelfBrain => "self",
            Self::Repo => "repo",
            Self::Topic => "topic",
        }
    }

    fn from_sql(s: &str) -> SqlResult<Self> {
        match s {
            "self" => Ok(Self::SelfBrain),
            "repo" => Ok(Self::Repo),
            "topic" => Ok(Self::Topic),
            other => Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown memory_source kind {other:?}"),
                )),
            )),
        }
    }
}

/// One memory source row.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemorySource {
    pub id: String,
    pub kind: MemorySourceKind,
    pub label: String,
    pub repo_url: Option<String>,
    pub repo_ref: Option<String>,
    pub created_at: i64,
    pub last_synced_at: Option<i64>,
}

fn map_row(row: &rusqlite::Row<'_>) -> SqlResult<MemorySource> {
    let kind_str: String = row.get(1)?;
    Ok(MemorySource {
        id: row.get(0)?,
        kind: MemorySourceKind::from_sql(&kind_str)?,
        label: row.get(2)?,
        repo_url: row.get(3)?,
        repo_ref: row.get(4)?,
        created_at: row.get(5)?,
        last_synced_at: row.get(6)?,
    })
}

/// List all registered sources, `'self'` first then alphabetical by label.
pub fn list_sources(conn: &Connection) -> SqlResult<Vec<MemorySource>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, label, repo_url, repo_ref, created_at, last_synced_at
         FROM memory_sources
         ORDER BY (id = 'self') DESC, lower(label) ASC",
    )?;
    let rows = stmt.query_map([], map_row)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Fetch a single source by id.
pub fn get_source(conn: &Connection, id: &str) -> SqlResult<Option<MemorySource>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, label, repo_url, repo_ref, created_at, last_synced_at
         FROM memory_sources WHERE id = ?1",
    )?;
    stmt.query_row(params![id], map_row).optional()
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Create a new source. `id` must be unique; caller is responsible for
/// generating a stable slug (e.g. `repo:<host>/<owner>/<name>@<ref>`).
///
/// Rejects attempts to create a row with `id = "self"` (already seeded)
/// and attempts to create another `kind = SelfBrain` row.
pub fn create_source(
    conn: &Connection,
    id: &str,
    kind: MemorySourceKind,
    label: &str,
    repo_url: Option<&str>,
    repo_ref: Option<&str>,
) -> SqlResult<MemorySource> {
    if kind == MemorySourceKind::SelfBrain {
        return Err(rusqlite::Error::InvalidParameterName(
            "cannot create additional 'self' source rows".into(),
        ));
    }
    if id == SELF_SOURCE_ID {
        return Err(rusqlite::Error::InvalidParameterName(
            "id 'self' is reserved for the built-in brain".into(),
        ));
    }
    let now = now_millis();
    conn.execute(
        "INSERT INTO memory_sources (id, kind, label, repo_url, repo_ref, created_at, last_synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
        params![id, kind.as_sql(), label, repo_url, repo_ref, now],
    )?;
    Ok(MemorySource {
        id: id.to_string(),
        kind,
        label: label.to_string(),
        repo_url: repo_url.map(str::to_string),
        repo_ref: repo_ref.map(str::to_string),
        created_at: now,
        last_synced_at: None,
    })
}

/// Update the `last_synced_at` timestamp on a source. Returns the number
/// of rows touched (0 if the id is unknown).
pub fn touch_synced(conn: &Connection, id: &str) -> SqlResult<usize> {
    conn.execute(
        "UPDATE memory_sources SET last_synced_at = ?1 WHERE id = ?2",
        params![now_millis(), id],
    )
}

/// Delete a source. Rejects deletion of the `'self'` row at this layer
/// so that callers (Tauri commands, MCP tools, tests) get a consistent
/// `InvalidParameterName` error rather than silently succeeding.
///
/// Returns `true` if a row was deleted, `false` if the id was unknown.
pub fn delete_source(conn: &Connection, id: &str) -> SqlResult<bool> {
    if id == SELF_SOURCE_ID {
        return Err(rusqlite::Error::InvalidParameterName(
            "the built-in 'self' source cannot be deleted".into(),
        ));
    }
    let n = conn.execute("DELETE FROM memory_sources WHERE id = ?1", params![id])?;
    Ok(n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::schema::create_canonical_schema;

    fn fresh() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_canonical_schema(&c).unwrap();
        c
    }

    #[test]
    fn self_row_is_seeded_by_migration() {
        let c = fresh();
        let s = get_source(&c, "self").unwrap().expect("self row missing");
        assert_eq!(s.kind, MemorySourceKind::SelfBrain);
        assert_eq!(s.label, "TerranSoul");
        assert!(s.repo_url.is_none());
    }

    #[test]
    fn list_orders_self_first_then_alpha() {
        let c = fresh();
        create_source(&c, "repo:b", MemorySourceKind::Repo, "Bravo", None, None).unwrap();
        create_source(&c, "repo:a", MemorySourceKind::Repo, "Alpha", None, None).unwrap();
        let rows = list_sources(&c).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].id, "self");
        assert_eq!(rows[1].label, "Alpha");
        assert_eq!(rows[2].label, "Bravo");
    }

    #[test]
    fn create_round_trip() {
        let c = fresh();
        let s = create_source(
            &c,
            "repo:gh/foo/bar@main",
            MemorySourceKind::Repo,
            "foo/bar @ main",
            Some("https://github.com/foo/bar"),
            Some("main"),
        )
        .unwrap();
        let back = get_source(&c, &s.id).unwrap().unwrap();
        assert_eq!(back.label, "foo/bar @ main");
        assert_eq!(back.repo_url.as_deref(), Some("https://github.com/foo/bar"));
        assert_eq!(back.repo_ref.as_deref(), Some("main"));
        assert!(back.last_synced_at.is_none());
    }

    #[test]
    fn cannot_create_second_self() {
        let c = fresh();
        let err =
            create_source(&c, "self2", MemorySourceKind::SelfBrain, "x", None, None).unwrap_err();
        assert!(matches!(err, rusqlite::Error::InvalidParameterName(_)));
    }

    #[test]
    fn cannot_reuse_self_id() {
        let c = fresh();
        let err = create_source(&c, "self", MemorySourceKind::Repo, "x", None, None).unwrap_err();
        assert!(matches!(err, rusqlite::Error::InvalidParameterName(_)));
    }

    #[test]
    fn cannot_delete_self() {
        let c = fresh();
        let err = delete_source(&c, "self").unwrap_err();
        assert!(matches!(err, rusqlite::Error::InvalidParameterName(_)));
        assert!(get_source(&c, "self").unwrap().is_some());
    }

    #[test]
    fn delete_unknown_returns_false() {
        let c = fresh();
        assert!(!delete_source(&c, "repo:nope").unwrap());
    }

    #[test]
    fn delete_repo_works() {
        let c = fresh();
        create_source(&c, "repo:x", MemorySourceKind::Repo, "X", None, None).unwrap();
        assert!(delete_source(&c, "repo:x").unwrap());
        assert!(get_source(&c, "repo:x").unwrap().is_none());
    }

    #[test]
    fn touch_synced_updates_timestamp() {
        let c = fresh();
        create_source(&c, "repo:t", MemorySourceKind::Repo, "T", None, None).unwrap();
        assert_eq!(touch_synced(&c, "repo:t").unwrap(), 1);
        let s = get_source(&c, "repo:t").unwrap().unwrap();
        assert!(s.last_synced_at.is_some());
    }

    #[test]
    fn migration_is_idempotent() {
        let c = fresh();
        // Running the canonical schema again must not double-seed.
        create_canonical_schema(&c).unwrap();
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM memory_sources WHERE id='self'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }
}
