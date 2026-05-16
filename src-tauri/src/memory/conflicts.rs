//! Memory contradiction detection and resolution (Chunk 17.2).
//!
//! When a new memory is added and a near-duplicate (high cosine
//! similarity) is found that semantically *contradicts* the new entry,
//! a `MemoryConflict` row is opened. The user is notified in the Brain
//! view and can pick a winner — the loser is soft-closed via `valid_to`
//! (never deleted).
//!
//! The module is split into:
//! - **Prompt + parse** (`build_contradiction_prompt`, `parse_contradiction_reply`)
//!   — pure functions for LLM-based contradiction checking.
//! - **CRUD** (`MemoryConflict` struct + `MemoryStore` methods) — SQL
//!   operations on the `memory_conflicts` table (V9 schema).
//!
//! See `docs/brain-advanced-design.md` §16 Phase 5.

use rusqlite::{params, Result as SqlResult};
use serde::{Deserialize, Serialize};

use super::store::MemoryStore;

// ── Types ──────────────────────────────────────────────────────────────────────

/// Status of a memory conflict.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStatus {
    /// Awaiting user resolution.
    Open,
    /// User picked a winner.
    Resolved,
    /// User dismissed (not a real conflict).
    Dismissed,
}

impl ConflictStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
            Self::Dismissed => "dismissed",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "resolved" => Self::Resolved,
            "dismissed" => Self::Dismissed,
            _ => Self::Open,
        }
    }
}

/// A detected contradiction between two memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConflict {
    pub id: i64,
    /// First memory in the conflict pair.
    pub entry_a_id: i64,
    /// Second memory (usually the newer one).
    pub entry_b_id: i64,
    pub status: ConflictStatus,
    /// The winning memory id (set on resolution).
    pub winner_id: Option<i64>,
    pub created_at: i64,
    pub resolved_at: Option<i64>,
    /// LLM-provided reason for the contradiction.
    pub reason: String,
}

/// Result of the LLM contradiction check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContradictionResult {
    /// True if the two statements contradict each other.
    pub contradicts: bool,
    /// Short explanation of why they contradict (empty if they don't).
    pub reason: String,
}

// ── LLM prompt + parse (pure functions) ────────────────────────────────────────

/// Build the (system, user) prompt pair that asks the LLM whether two
/// memory statements contradict each other.
pub fn build_contradiction_prompt(content_a: &str, content_b: &str) -> (String, String) {
    let system = "You are a fact-checking assistant. \
Determine whether two statements contradict each other. \
Reply with ONLY a JSON object — no prose, no markdown fences."
        .to_string();

    let user = format!(
        "STATEMENT A:\n{}\n\nSTATEMENT B:\n{}\n\n\
        Do these two statements contradict each other? \
        A contradiction means they cannot both be true at the same time.\n\n\
        OUTPUT FORMAT — reply with exactly one JSON object:\n\
        {{\n\
        \x20 \"contradicts\": true/false,\n\
        \x20 \"reason\": \"<1-2 sentence explanation>\"\n\
        }}\n\n\
        Rules:\n\
        - Only flag genuine contradictions — not minor differences or complementary info.\n\
        - If they are compatible or just different aspects of the same topic, set contradicts=false.\n\
        - Reply with ONLY the JSON object.",
        content_a.trim(),
        content_b.trim(),
    );

    (system, user)
}

/// Parse a brain reply into a [`ContradictionResult`]. Tolerant of fences,
/// leading prose, and missing fields.
pub fn parse_contradiction_reply(raw: &str) -> Option<ContradictionResult> {
    let body = strip_fences(raw);
    let start = body.find('{')?;
    let end = body.rfind('}')? + 1;
    if start >= end {
        return None;
    }
    let json_str = &body[start..end];

    // Try strict parse first.
    if let Ok(result) = serde_json::from_str::<ContradictionResult>(json_str) {
        return Some(result);
    }

    // Fallback: parse as generic Value.
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let contradicts = v.get("contradicts")?.as_bool()?;
    let reason = v
        .get("reason")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    Some(ContradictionResult {
        contradicts,
        reason,
    })
}

/// Strip markdown fences from a raw reply.
fn strip_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let rest = rest.strip_prefix("json").unwrap_or(rest);
        let rest = rest.trim_start_matches('\n');
        if let Some(body) = rest.strip_suffix("```") {
            return body.trim().to_string();
        }
    }
    trimmed.to_string()
}

// ── Store methods (CRUD on memory_conflicts table) ─────────────────────────────

impl MemoryStore {
    /// Record a new contradiction between two memories.
    pub fn add_conflict(
        &self,
        entry_a_id: i64,
        entry_b_id: i64,
        reason: &str,
    ) -> SqlResult<MemoryConflict> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        self.conn().execute(
            "INSERT INTO memory_conflicts (entry_a_id, entry_b_id, status, reason, created_at)
             VALUES (?1, ?2, 'open', ?3, ?4)",
            params![entry_a_id, entry_b_id, reason, now],
        )?;

        let id = self.conn().last_insert_rowid();
        Ok(MemoryConflict {
            id,
            entry_a_id,
            entry_b_id,
            status: ConflictStatus::Open,
            winner_id: None,
            created_at: now,
            resolved_at: None,
            reason: reason.to_string(),
        })
    }

    /// List all conflicts matching the given status filter.
    /// Pass `None` to list all conflicts regardless of status.
    pub fn list_conflicts(
        &self,
        status_filter: Option<&ConflictStatus>,
    ) -> SqlResult<Vec<MemoryConflict>> {
        let (sql, filter_val): (&str, Option<String>) = match status_filter {
            Some(s) => (
                "SELECT id, entry_a_id, entry_b_id, status, winner_id, created_at, resolved_at, reason
                 FROM memory_conflicts WHERE status = ?1 ORDER BY created_at DESC",
                Some(s.as_str().to_string()),
            ),
            None => (
                "SELECT id, entry_a_id, entry_b_id, status, winner_id, created_at, resolved_at, reason
                 FROM memory_conflicts ORDER BY created_at DESC",
                None,
            ),
        };

        let mut stmt = self.conn().prepare(sql)?;
        let rows = if let Some(ref val) = filter_val {
            stmt.query_map(params![val], row_to_conflict)?
        } else {
            stmt.query_map([], row_to_conflict)?
        };
        rows.collect()
    }

    /// Resolve a conflict by picking a winner. The loser is soft-closed
    /// via `valid_to`. Returns the updated conflict.
    pub fn resolve_conflict(&self, conflict_id: i64, winner_id: i64) -> SqlResult<MemoryConflict> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        // Load the conflict to find the loser.
        let conflict: MemoryConflict = self.conn().query_row(
            "SELECT id, entry_a_id, entry_b_id, status, winner_id, created_at, resolved_at, reason
             FROM memory_conflicts WHERE id = ?1",
            params![conflict_id],
            row_to_conflict,
        )?;

        let loser_id = if winner_id == conflict.entry_a_id {
            conflict.entry_b_id
        } else {
            conflict.entry_a_id
        };

        // Soft-close the loser.
        self.close_memory(loser_id, now)?;

        // Mark the conflict as resolved.
        self.conn().execute(
            "UPDATE memory_conflicts SET status = 'resolved', winner_id = ?1, resolved_at = ?2
             WHERE id = ?3",
            params![winner_id, now, conflict_id],
        )?;

        // Save a version audit trail for the loser (best-effort).
        let _ = super::versioning::save_version(self.conn(), loser_id);

        Ok(MemoryConflict {
            id: conflict_id,
            entry_a_id: conflict.entry_a_id,
            entry_b_id: conflict.entry_b_id,
            status: ConflictStatus::Resolved,
            winner_id: Some(winner_id),
            created_at: conflict.created_at,
            resolved_at: Some(now),
            reason: conflict.reason,
        })
    }

    /// Dismiss a conflict (user says "not a real conflict").
    pub fn dismiss_conflict(&self, conflict_id: i64) -> SqlResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        self.conn().execute(
            "UPDATE memory_conflicts SET status = 'dismissed', resolved_at = ?1 WHERE id = ?2",
            params![now, conflict_id],
        )?;
        Ok(())
    }

    /// Count open (unresolved) conflicts.
    pub fn count_open_conflicts(&self) -> SqlResult<i64> {
        self.conn().query_row(
            "SELECT COUNT(*) FROM memory_conflicts WHERE status = 'open'",
            [],
            |r: &rusqlite::Row<'_>| r.get(0),
        )
    }

    /// CLAIM-VERIFY-2 (2026-05-17) — return the set of memory ids that
    /// participate in at least one OPEN `memory_conflicts` row (either
    /// side). Used by `hybrid_search_rrf` to apply a per-id score
    /// multiplier so contested claims rank lower than uncontested
    /// equally-relevant peers. Returns an empty set when no open
    /// conflicts exist — callers can early-return to skip the lookup
    /// hot-path overhead in the common case.
    pub fn contested_memory_ids(&self) -> SqlResult<std::collections::HashSet<i64>> {
        let mut stmt = self.conn().prepare(
            "SELECT entry_a_id FROM memory_conflicts WHERE status = 'open' \
             UNION SELECT entry_b_id FROM memory_conflicts WHERE status = 'open'",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
        let mut set = std::collections::HashSet::new();
        for row in rows {
            set.insert(row?);
        }
        Ok(set)
    }
}

/// Pure-logic gate for the chat-side auto-detect path (CHAT-PARITY-4).
///
/// Given a brain-produced [`ContradictionResult`] for the (`new_id`,
/// `dup_id`) pair, opens a `memory_conflicts` row only when the brain
/// confirmed an actual contradiction AND the two ids are different
/// (a memory cannot contradict itself — `find_duplicate` can return the
/// just-inserted row when its embedding round-trips through the ANN
/// index). Returns the new conflict id on success, `None` when the
/// gate filtered the call out.
///
/// This helper is hermetic: callers feed in a synthesised
/// [`ContradictionResult`] in unit tests so the trigger logic can be
/// validated without an Ollama hop. The async LLM call lives in
/// `OllamaAgent::check_contradiction`; the chat-side wrapper in
/// `commands::memory::extract_memories_from_session` chains the two.
///
/// See `docs/brain-advanced-design.md` § Phase-5 "Contradiction
/// resolution" and `rules/milestones.md` (CHAT-PARITY-4).
pub fn record_contradiction_if(
    store: &MemoryStore,
    new_id: i64,
    dup_id: i64,
    result: &ContradictionResult,
) -> Option<i64> {
    if !result.contradicts || new_id == dup_id {
        return None;
    }
    store
        .add_conflict(dup_id, new_id, &result.reason)
        .ok()
        .map(|c| c.id)
}

fn row_to_conflict(row: &rusqlite::Row<'_>) -> SqlResult<MemoryConflict> {
    Ok(MemoryConflict {
        id: row.get(0)?,
        entry_a_id: row.get(1)?,
        entry_b_id: row.get(2)?,
        status: ConflictStatus::parse(&row.get::<_, String>(3)?),
        winner_id: row.get(4)?,
        created_at: row.get(5)?,
        resolved_at: row.get(6)?,
        reason: row.get::<_, String>(7).unwrap_or_default(),
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Prompt + parse tests ───────────────────────────────────────────────

    #[test]
    fn build_contradiction_prompt_includes_both_statements() {
        let (system, user) =
            build_contradiction_prompt("The sky is blue.", "The sky is always red.");
        assert!(system.contains("fact-checking"));
        assert!(user.contains("The sky is blue"));
        assert!(user.contains("The sky is always red"));
        assert!(user.contains("contradicts"));
    }

    #[test]
    fn parse_contradiction_reply_clean_json() {
        let raw = r#"{"contradicts":true,"reason":"They disagree about the sky color."}"#;
        let result = parse_contradiction_reply(raw).unwrap();
        assert!(result.contradicts);
        assert!(result.reason.contains("sky color"));
    }

    #[test]
    fn parse_contradiction_reply_no_contradiction() {
        let raw = r#"{"contradicts":false,"reason":""}"#;
        let result = parse_contradiction_reply(raw).unwrap();
        assert!(!result.contradicts);
    }

    #[test]
    fn parse_contradiction_reply_with_fences() {
        let raw = "```json\n{\"contradicts\":true,\"reason\":\"Conflict.\"}\n```";
        let result = parse_contradiction_reply(raw).unwrap();
        assert!(result.contradicts);
    }

    #[test]
    fn parse_contradiction_reply_with_prose() {
        let raw = "Sure! Here's the analysis:\n{\"contradicts\":false,\"reason\":\"\"}";
        let result = parse_contradiction_reply(raw).unwrap();
        assert!(!result.contradicts);
    }

    #[test]
    fn parse_contradiction_reply_garbage() {
        assert!(parse_contradiction_reply("not json").is_none());
        assert!(parse_contradiction_reply("").is_none());
    }

    #[test]
    fn contradiction_result_serde_round_trip() {
        let r = ContradictionResult {
            contradicts: true,
            reason: "They disagree.".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: ContradictionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    // ── CRUD tests (in-memory SQLite) ──────────────────────────────────────

    fn test_store() -> MemoryStore {
        MemoryStore::in_memory()
    }

    #[test]
    fn add_and_list_conflict() {
        let store = test_store();
        let a = store
            .add(super::super::NewMemory {
                content: "The deadline is Monday.".into(),
                ..Default::default()
            })
            .unwrap();
        let b = store
            .add(super::super::NewMemory {
                content: "The deadline is Friday.".into(),
                ..Default::default()
            })
            .unwrap();

        let c = store
            .add_conflict(a.id, b.id, "Disagree about the deadline.")
            .unwrap();
        assert_eq!(c.status, ConflictStatus::Open);
        assert_eq!(c.entry_a_id, a.id);
        assert_eq!(c.entry_b_id, b.id);

        let all = store.list_conflicts(None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, c.id);

        let open = store.list_conflicts(Some(&ConflictStatus::Open)).unwrap();
        assert_eq!(open.len(), 1);

        let resolved = store
            .list_conflicts(Some(&ConflictStatus::Resolved))
            .unwrap();
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_conflict_closes_loser() {
        let store = test_store();
        let a = store
            .add(super::super::NewMemory {
                content: "Earth is flat.".into(),
                ..Default::default()
            })
            .unwrap();
        let b = store
            .add(super::super::NewMemory {
                content: "Earth is round.".into(),
                ..Default::default()
            })
            .unwrap();

        let c = store.add_conflict(a.id, b.id, "Shape of Earth").unwrap();
        let resolved = store.resolve_conflict(c.id, b.id).unwrap();

        assert_eq!(resolved.status, ConflictStatus::Resolved);
        assert_eq!(resolved.winner_id, Some(b.id));
        assert!(resolved.resolved_at.is_some());

        // Loser (a) should have valid_to set.
        let loser = store.get_by_id(a.id).unwrap();
        assert!(loser.valid_to.is_some());

        // Winner (b) should still be active.
        let winner = store.get_by_id(b.id).unwrap();
        assert!(winner.valid_to.is_none());
    }

    #[test]
    fn dismiss_conflict() {
        let store = test_store();
        let a = store
            .add(super::super::NewMemory {
                content: "A".into(),
                ..Default::default()
            })
            .unwrap();
        let b = store
            .add(super::super::NewMemory {
                content: "B".into(),
                ..Default::default()
            })
            .unwrap();

        let c = store.add_conflict(a.id, b.id, "Maybe a conflict").unwrap();
        store.dismiss_conflict(c.id).unwrap();

        let all = store
            .list_conflicts(Some(&ConflictStatus::Dismissed))
            .unwrap();
        assert_eq!(all.len(), 1);

        // Neither entry should be closed.
        assert!(store.get_by_id(a.id).unwrap().valid_to.is_none());
        assert!(store.get_by_id(b.id).unwrap().valid_to.is_none());
    }

    #[test]
    fn count_open_conflicts() {
        let store = test_store();
        let a = store
            .add(super::super::NewMemory {
                content: "X".into(),
                ..Default::default()
            })
            .unwrap();
        let b = store
            .add(super::super::NewMemory {
                content: "Y".into(),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(store.count_open_conflicts().unwrap(), 0);
        store.add_conflict(a.id, b.id, "test").unwrap();
        assert_eq!(store.count_open_conflicts().unwrap(), 1);
    }

    #[test]
    fn close_memory_sets_valid_to() {
        let store = test_store();
        let e = store
            .add(super::super::NewMemory {
                content: "Will be closed.".into(),
                ..Default::default()
            })
            .unwrap();
        assert!(e.valid_to.is_none());

        store.close_memory(e.id, 1234567890).unwrap();
        let after = store.get_by_id(e.id).unwrap();
        assert_eq!(after.valid_to, Some(1234567890));
    }

    // ── CHAT-PARITY-4 hermetic gate tests ───────────────────────────────

    #[test]
    fn record_contradiction_if_opens_row_on_negation() {
        // Positive case: the brain returned `contradicts: true` for a
        // semantic negation of an existing memory. The chat-side ingest
        // path must open a `memory_conflicts` row so the user can pick a
        // winner instead of letting two contradictory facts coexist.
        let store = test_store();
        let existing = store
            .add(super::super::NewMemory {
                content: "User prefers Python.".into(),
                ..Default::default()
            })
            .unwrap();
        let new_entry = store
            .add(super::super::NewMemory {
                content: "User does not like Python at all.".into(),
                ..Default::default()
            })
            .unwrap();

        let verdict = ContradictionResult {
            contradicts: true,
            reason: "Statements disagree about the user's preference for Python.".into(),
        };

        let conflict_id = record_contradiction_if(&store, new_entry.id, existing.id, &verdict);
        assert!(conflict_id.is_some(), "negation must open a conflict row");

        let open = store.list_conflicts(Some(&ConflictStatus::Open)).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].entry_a_id, existing.id);
        assert_eq!(open[0].entry_b_id, new_entry.id);
        assert!(open[0].reason.contains("Python"));
    }

    #[test]
    fn record_contradiction_if_does_not_open_row_on_paraphrase() {
        // Negative case 1: the brain returned `contradicts: false` for a
        // paraphrase. The gate must NOT open a conflict row, otherwise
        // the user gets spammed with false positives every time chat
        // restates an existing fact.
        let store = test_store();
        let existing = store
            .add(super::super::NewMemory {
                content: "User prefers Python.".into(),
                ..Default::default()
            })
            .unwrap();
        let new_entry = store
            .add(super::super::NewMemory {
                content: "Python is the user's preferred language.".into(),
                ..Default::default()
            })
            .unwrap();

        let verdict = ContradictionResult {
            contradicts: false,
            reason: String::new(),
        };

        let conflict_id = record_contradiction_if(&store, new_entry.id, existing.id, &verdict);
        assert!(
            conflict_id.is_none(),
            "paraphrase must NOT open a conflict row"
        );
        assert_eq!(store.count_open_conflicts().unwrap(), 0);

        // Negative case 2: even when `contradicts: true`, a self-pair
        // (`new_id == dup_id`) must be filtered. `find_duplicate` can
        // return the just-inserted row when its embedding round-trips
        // through the ANN index, and a memory cannot contradict itself.
        let self_pair = record_contradiction_if(&store, existing.id, existing.id, &ContradictionResult {
            contradicts: true,
            reason: "shouldn't reach the store".into(),
        });
        assert!(self_pair.is_none(), "self-pair must be filtered");
        assert_eq!(store.count_open_conflicts().unwrap(), 0);
    }
}
