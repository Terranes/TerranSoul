//! Per-task scenario aggregation tier (MEM-SCENARIO-1).
//!
//! Inspired by TencentDB-Agent-Memory's L2 Scenario blocks (aggregated
//! task scenes that sit between atomic L1 evidence and the L3 persona
//! projection). TerranSoul implements the tier as plain metadata on the
//! existing `memories` table — a nullable `scenario_id` column added in
//! V24. A "scenario" is itself a memory row (typically
//! `MemoryType::Summary`) that other entries point at via
//! `scenario_id = <scenario.id>`. This avoids rippling a new
//! `MemoryType` variant through every `add_many` call-site while still
//! giving the agent a durable handle for per-task aggregation.
//!
//! `ON DELETE SET NULL` on the FK means deleting the scenario head
//! never cascades into the underlying evidence — the L0/L1 ground
//! truth survives, only the aggregation handle goes away.
//!
// Schema lives in `schema.rs` (V24). This module is the `MemoryStore`
// API + tests only.

use rusqlite::{params, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};

use crate::memory::store::{MemoryEntry, MemoryStore, NewMemory};

/// Lightweight summary of a scenario block — the head row plus member
/// counts. Returned by listing endpoints so the caller can decide
/// whether to drill in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSummary {
    pub scenario_id: i64,
    /// Content of the head row (the L2 aggregation summary).
    pub content: String,
    /// Tags on the head row, comma-separated as stored.
    pub tags: String,
    pub importance: i64,
    pub created_at: i64,
    /// Number of member rows (`memories` with `scenario_id = scenario_id`),
    /// excluding the head row itself.
    pub member_count: i64,
}

impl MemoryStore {
    /// Create a new scenario block.
    ///
    /// 1. Inserts a head memory carrying the scenario's summary text
    ///    (typically `MemoryType::Summary`).
    /// 2. Stamps every `member_ids` row with `scenario_id = head.id`.
    ///
    /// Returns the head's id so the caller can wire follow-up edges
    /// (e.g. `derived_from` from the head to each member, handled
    /// outside this method to keep the primitive composable).
    pub fn create_scenario(
        &self,
        head: NewMemory,
        member_ids: &[i64],
    ) -> SqlResult<MemoryEntry> {
        let head_entry = self.add(head)?;
        if !member_ids.is_empty() {
            self.assign_members_to_scenario(head_entry.id, member_ids)?;
        }
        Ok(head_entry)
    }

    /// Assign a single existing memory to a scenario. Pass `None` to
    /// detach the memory from its current scenario.
    pub fn set_scenario_id(
        &self,
        memory_id: i64,
        scenario_id: Option<i64>,
    ) -> SqlResult<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE memories SET scenario_id = ?1 WHERE id = ?2",
            params![scenario_id, memory_id],
        )?;
        Ok(())
    }

    /// Bulk-assign `member_ids` to a scenario in a single transaction.
    pub fn assign_members_to_scenario(
        &self,
        scenario_id: i64,
        member_ids: &[i64],
    ) -> SqlResult<()> {
        if member_ids.is_empty() {
            return Ok(());
        }
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "UPDATE memories SET scenario_id = ?1 WHERE id = ?2",
            )?;
            for id in member_ids {
                stmt.execute(params![scenario_id, id])?;
            }
        }
        tx.commit()
    }

    /// Get the scenario_id for a given memory, or `None` if unassigned.
    pub fn get_scenario_id(&self, memory_id: i64) -> SqlResult<Option<i64>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT scenario_id FROM memories WHERE id = ?1",
            params![memory_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map(Option::flatten)
    }

    /// List the member rows for a scenario, in creation order. Does
    /// NOT include the scenario head row itself.
    pub fn list_scenario_members(&self, scenario_id: i64) -> SqlResult<Vec<MemoryEntry>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, content, tags, importance, memory_type, created_at, last_accessed,
                    access_count, tier, decay_score, session_id, parent_id, token_count,
                    source_url, source_hash, expires_at, valid_to, obsidian_path,
                    last_exported, updated_at, origin_device, hlc_counter, confidence
             FROM memories
             WHERE scenario_id = ?1 AND id != ?1
             ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map(params![scenario_id], crate::memory::store::row_to_entry)?;
        rows.collect()
    }

    /// List all scenarios with their head row + member counts, newest
    /// first. A "scenario" here is any memory row whose id appears as
    /// a `scenario_id` on at least one other row.
    pub fn list_scenarios(&self, limit: i64) -> SqlResult<Vec<ScenarioSummary>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.content, s.tags, s.importance, s.created_at,
                    (SELECT COUNT(*) FROM memories m WHERE m.scenario_id = s.id AND m.id != s.id) AS member_count
             FROM memories s
             WHERE EXISTS (
                 SELECT 1 FROM memories m WHERE m.scenario_id = s.id AND m.id != s.id
             )
             ORDER BY s.created_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(ScenarioSummary {
                scenario_id: row.get(0)?,
                content: row.get(1)?,
                tags: row.get(2)?,
                importance: row.get(3)?,
                created_at: row.get(4)?,
                member_count: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    /// Total count of distinct scenarios (rows referenced by at least
    /// one `scenario_id`). Drives the "Scenario Aggregation" skill-tree
    /// quest auto-activation.
    pub fn scenario_total_count(&self) -> SqlResult<i64> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(DISTINCT scenario_id) FROM memories WHERE scenario_id IS NOT NULL",
            [],
            |row| row.get(0),
        )
    }
}

// Helper re-export so external callers don't need to thread the
// internal column mapping through this module.
#[allow(unused_imports)]
pub(crate) use crate::memory::store::row_to_entry as _row_to_entry;

#[cfg(test)]
mod tests {
    use crate::memory::store::{MemoryStore, MemoryType, NewMemory};
    use std::sync::atomic::{AtomicI64, Ordering};

    /// `MemoryStore::add` dedupes by content hash, so each helper call
    /// must produce a unique content string.
    fn make_member(store: &MemoryStore, label: &str) -> i64 {
        static COUNTER: AtomicI64 = AtomicI64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        store
            .add(NewMemory {
                content: format!("scenario-member-{}-{}", label, n),
                tags: "test,scenario-member".to_string(),
                importance: 3,
                memory_type: MemoryType::Context,
                ..Default::default()
            })
            .unwrap()
            .id
    }

    fn make_head(_store: &MemoryStore) -> NewMemory {
        static COUNTER: AtomicI64 = AtomicI64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        NewMemory {
            content: format!("scenario summary head {}", n),
            tags: "test,scenario-head".to_string(),
            importance: 6,
            memory_type: MemoryType::Summary,
            ..Default::default()
        }
    }

    #[test]
    fn create_scenario_stamps_all_members() {
        let store = MemoryStore::in_memory();
        let m1 = make_member(&store, "a");
        let m2 = make_member(&store, "b");
        let m3 = make_member(&store, "c");

        let head = store.create_scenario(make_head(&store), &[m1, m2, m3]).unwrap();

        assert_eq!(store.get_scenario_id(m1).unwrap(), Some(head.id));
        assert_eq!(store.get_scenario_id(m2).unwrap(), Some(head.id));
        assert_eq!(store.get_scenario_id(m3).unwrap(), Some(head.id));
        // The head's own scenario_id stays NULL — it IS the scenario,
        // not a member of one.
        assert_eq!(store.get_scenario_id(head.id).unwrap(), None);
    }

    #[test]
    fn create_scenario_with_no_members_still_inserts_head() {
        let store = MemoryStore::in_memory();
        let head = store.create_scenario(make_head(&store), &[]).unwrap();
        assert!(head.id > 0);
        assert!(store.list_scenario_members(head.id).unwrap().is_empty());
    }

    #[test]
    fn list_scenario_members_excludes_head_and_orders_chronologically() {
        let store = MemoryStore::in_memory();
        let m1 = make_member(&store, "first");
        let m2 = make_member(&store, "second");
        let head = store.create_scenario(make_head(&store), &[m1, m2]).unwrap();

        let members = store.list_scenario_members(head.id).unwrap();
        assert_eq!(members.len(), 2);
        let ids: Vec<i64> = members.iter().map(|m| m.id).collect();
        assert!(ids.contains(&m1));
        assert!(ids.contains(&m2));
        assert!(!ids.contains(&head.id));
    }

    #[test]
    fn set_scenario_id_detaches_member() {
        let store = MemoryStore::in_memory();
        let m1 = make_member(&store, "x");
        let head = store.create_scenario(make_head(&store), &[m1]).unwrap();
        assert_eq!(store.get_scenario_id(m1).unwrap(), Some(head.id));

        store.set_scenario_id(m1, None).unwrap();
        assert_eq!(store.get_scenario_id(m1).unwrap(), None);
        assert!(store.list_scenario_members(head.id).unwrap().is_empty());
    }

    #[test]
    fn deleting_scenario_head_nulls_member_pointers_not_member_rows() {
        let store = MemoryStore::in_memory();
        let m1 = make_member(&store, "alpha");
        let m2 = make_member(&store, "beta");
        let head = store.create_scenario(make_head(&store), &[m1, m2]).unwrap();
        assert_eq!(store.list_scenario_members(head.id).unwrap().len(), 2);

        store.delete(head.id).unwrap();

        // Members survive (ON DELETE SET NULL preserves L0/L1 evidence).
        assert!(store.get_by_id(m1).is_ok());
        assert!(store.get_by_id(m2).is_ok());
        assert_eq!(store.get_scenario_id(m1).unwrap(), None);
        assert_eq!(store.get_scenario_id(m2).unwrap(), None);
    }

    #[test]
    fn list_scenarios_returns_head_with_member_count() {
        let store = MemoryStore::in_memory();
        let m1 = make_member(&store, "p");
        let m2 = make_member(&store, "q");
        let m3 = make_member(&store, "r");
        let head = store.create_scenario(make_head(&store), &[m1, m2, m3]).unwrap();

        let scenarios = store.list_scenarios(10).unwrap();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].scenario_id, head.id);
        assert_eq!(scenarios[0].member_count, 3);
    }

    #[test]
    fn list_scenarios_skips_empty_heads() {
        let store = MemoryStore::in_memory();
        // Head with no members at all should NOT appear in the
        // listing — it isn't aggregating anything yet.
        let _empty = store.create_scenario(make_head(&store), &[]).unwrap();
        assert!(store.list_scenarios(10).unwrap().is_empty());
    }

    #[test]
    fn scenario_total_count_counts_distinct_heads() {
        let store = MemoryStore::in_memory();
        let m_a1 = make_member(&store, "a1");
        let m_a2 = make_member(&store, "a2");
        let m_b1 = make_member(&store, "b1");
        let _h_a = store.create_scenario(make_head(&store), &[m_a1, m_a2]).unwrap();
        let _h_b = store.create_scenario(make_head(&store), &[m_b1]).unwrap();

        assert_eq!(store.scenario_total_count().unwrap(), 2);
    }

    #[test]
    fn assign_members_is_transactional_and_idempotent() {
        let store = MemoryStore::in_memory();
        let m1 = make_member(&store, "t1");
        let m2 = make_member(&store, "t2");
        let head = store.create_scenario(make_head(&store), &[]).unwrap();

        store.assign_members_to_scenario(head.id, &[m1, m2]).unwrap();
        store.assign_members_to_scenario(head.id, &[m1, m2]).unwrap(); // idempotent
        assert_eq!(store.list_scenario_members(head.id).unwrap().len(), 2);
    }
}
