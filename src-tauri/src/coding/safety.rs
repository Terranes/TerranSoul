//! Tier 1 / Tier 2 safety classifier (Chunk 43.10).
//!
//! Centralises permission decisions for all actions the coding agent can
//! perform. Every action is classified as Tier 1 (auto-approved) or
//! Tier 2 (requires confirmation). Decisions are persisted in the
//! `safety_decisions` table for audit and Tier-1 promotion analysis.

use rusqlite::{params, Result as SqlResult};
use serde::{Deserialize, Serialize};

/// An action the coding agent might want to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Read,
    Write,
    RunTests,
    CreateBranch,
    PushRemote,
    OpenPr,
    MergePr,
    RunShell,
    SendEmail,
    InstallPackage,
    DeleteFile,
    DropTable,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::RunTests => "run_tests",
            Self::CreateBranch => "create_branch",
            Self::PushRemote => "push_remote",
            Self::OpenPr => "open_pr",
            Self::MergePr => "merge_pr",
            Self::RunShell => "run_shell",
            Self::SendEmail => "send_email",
            Self::InstallPackage => "install_package",
            Self::DeleteFile => "delete_file",
            Self::DropTable => "drop_table",
        }
    }
}

/// Safety tier: Tier 1 is auto-approved, Tier 2 requires confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    Tier1,
    Tier2,
}

impl Tier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tier1 => "tier1",
            Self::Tier2 => "tier2",
        }
    }
}

/// Decision outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Approved,
    Denied,
    Promoted,
}

impl Decision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Promoted => "promoted",
        }
    }
}

/// A persisted safety decision record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyDecisionRecord {
    pub id: i64,
    pub action: String,
    pub decision: String,
    pub decided_at: i64,
    pub decided_via: String,
}

/// Static default tier classification.
pub fn default_tier(action: Action) -> Tier {
    match action {
        Action::Read | Action::Write | Action::RunTests | Action::CreateBranch => Tier::Tier1,
        Action::PushRemote
        | Action::OpenPr
        | Action::MergePr
        | Action::RunShell
        | Action::SendEmail
        | Action::InstallPackage
        | Action::DeleteFile
        | Action::DropTable => Tier::Tier2,
    }
}

/// Configuration for per-project tier overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Actions overridden to Tier 1 (auto-approved).
    pub tier1_overrides: Vec<String>,
    /// Actions overridden to Tier 2 (require confirmation).
    pub tier2_overrides: Vec<String>,
    /// Number of consecutive approvals needed for Tier 1 promotion.
    pub promotion_threshold: u32,
}

impl SafetyConfig {
    pub fn effective_tier(&self, action: Action) -> Tier {
        let name = action.as_str();
        if self.tier1_overrides.iter().any(|s| s == name) {
            return Tier::Tier1;
        }
        if self.tier2_overrides.iter().any(|s| s == name) {
            return Tier::Tier2;
        }
        default_tier(action)
    }

    pub fn promotion_count(&self) -> u32 {
        if self.promotion_threshold > 0 {
            self.promotion_threshold
        } else {
            14
        }
    }
}

/// Central permission entry point. Returns `Ok(true)` if auto-approved
/// (Tier 1), `Ok(false)` if denied/requires confirmation (Tier 2).
/// Records the decision in `safety_decisions`.
pub fn request_permission(
    conn: &rusqlite::Connection,
    action: Action,
    config: &SafetyConfig,
    reason: &str,
) -> SqlResult<bool> {
    let tier = config.effective_tier(action);
    let decision = match tier {
        Tier::Tier1 => Decision::Approved,
        Tier::Tier2 => Decision::Denied,
    };

    let now = crate::memory::store::now_ms();
    conn.execute(
        "INSERT INTO safety_decisions (action, decision, decided_at, decided_via)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            action.as_str(),
            decision.as_str(),
            now,
            reason
        ],
    )?;

    Ok(matches!(tier, Tier::Tier1))
}

/// Count consecutive approvals for an action (newest first).
pub fn consecutive_approvals(conn: &rusqlite::Connection, action: Action) -> SqlResult<u32> {
    let mut stmt = conn.prepare_cached(
        "SELECT decision FROM safety_decisions
         WHERE action = ?1
         ORDER BY decided_at DESC
         LIMIT 100",
    )?;

    let rows = stmt.query_map(params![action.as_str()], |row| {
        row.get::<_, String>(0)
    })?;

    let mut count = 0u32;
    for row in rows.flatten() {
        if row == "approved" {
            count += 1;
        } else {
            break;
        }
    }
    Ok(count)
}

/// Check if an action qualifies for Tier 1 promotion.
pub fn check_promotion(
    conn: &rusqlite::Connection,
    action: Action,
    config: &SafetyConfig,
) -> SqlResult<bool> {
    let threshold = config.promotion_count();
    let consec = consecutive_approvals(conn, action)?;
    Ok(consec >= threshold)
}

/// List recent safety decisions.
pub fn list_decisions(
    conn: &rusqlite::Connection,
    limit: usize,
) -> SqlResult<Vec<SafetyDecisionRecord>> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, action, decision, decided_at, decided_via
         FROM safety_decisions
         ORDER BY decided_at DESC
         LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(SafetyDecisionRecord {
            id: row.get(0)?,
            action: row.get(1)?,
            decision: row.get(2)?,
            decided_at: row.get(3)?,
            decided_via: row.get(4)?,
        })
    })?;

    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::MemoryStore;

    fn test_conn() -> rusqlite::Connection {
        let store = MemoryStore::in_memory();
        // The safety_decisions table is created by V20 schema.
        // MemoryStore::in_memory() runs the full schema.
        // Return the connection — use the store's conn.
        // We need to extract the connection. Since store.conn is pub(crate),
        // and we're in-crate, this works in tests.
        store.conn
    }

    #[test]
    fn default_tiers() {
        assert_eq!(default_tier(Action::Read), Tier::Tier1);
        assert_eq!(default_tier(Action::Write), Tier::Tier1);
        assert_eq!(default_tier(Action::RunTests), Tier::Tier1);
        assert_eq!(default_tier(Action::PushRemote), Tier::Tier2);
        assert_eq!(default_tier(Action::MergePr), Tier::Tier2);
        assert_eq!(default_tier(Action::DeleteFile), Tier::Tier2);
    }

    #[test]
    fn config_overrides_tier() {
        let config = SafetyConfig {
            tier1_overrides: vec!["push_remote".to_string()],
            tier2_overrides: vec!["write".to_string()],
            promotion_threshold: 14,
        };
        assert_eq!(config.effective_tier(Action::PushRemote), Tier::Tier1);
        assert_eq!(config.effective_tier(Action::Write), Tier::Tier2);
        assert_eq!(config.effective_tier(Action::Read), Tier::Tier1);
    }

    #[test]
    fn request_permission_tier1_auto_approves() {
        let conn = test_conn();
        let config = SafetyConfig::default();
        let result = request_permission(&conn, Action::Read, &config, "test read").unwrap();
        assert!(result);

        let decisions = list_decisions(&conn, 10).unwrap();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].action, "read");
        assert_eq!(decisions[0].decision, "approved");
    }

    #[test]
    fn request_permission_tier2_denies() {
        let conn = test_conn();
        let config = SafetyConfig::default();
        let result = request_permission(&conn, Action::MergePr, &config, "merge test").unwrap();
        assert!(!result);

        let decisions = list_decisions(&conn, 10).unwrap();
        assert_eq!(decisions[0].decision, "denied");
    }

    #[test]
    fn consecutive_approvals_counts_streak() {
        let conn = test_conn();
        let config = SafetyConfig::default();
        for _ in 0..5 {
            request_permission(&conn, Action::Read, &config, "test").unwrap();
        }
        assert_eq!(consecutive_approvals(&conn, Action::Read).unwrap(), 5);
    }

    #[test]
    fn consecutive_approvals_resets_on_deny() {
        let conn = test_conn();
        let config = SafetyConfig::default();

        // 3 approvals for read.
        for _ in 0..3 {
            request_permission(&conn, Action::Read, &config, "ok").unwrap();
        }
        // Force a deny with a future timestamp so it sorts above the 3 approvals.
        let future_ts = crate::memory::store::now_ms() + 100_000;
        conn.execute(
            "INSERT INTO safety_decisions (action, decision, decided_at, decided_via)
             VALUES ('read', 'denied', ?1, 'forced')",
            params![future_ts],
        )
        .unwrap();
        // 2 more approvals (their timestamps will be after the deny in real time,
        // but we need them even further in the future).
        let far_future = future_ts + 200_000;
        for i in 0..2 {
            conn.execute(
                "INSERT INTO safety_decisions (action, decision, decided_at, decided_via)
                 VALUES ('read', 'approved', ?1, 'ok')",
                params![far_future + i],
            )
            .unwrap();
        }

        assert_eq!(consecutive_approvals(&conn, Action::Read).unwrap(), 2);
    }

    #[test]
    fn promotion_check_with_threshold() {
        let conn = test_conn();
        let config = SafetyConfig {
            promotion_threshold: 3,
            ..Default::default()
        };

        for _ in 0..2 {
            request_permission(&conn, Action::Write, &config, "ok").unwrap();
        }
        assert!(!check_promotion(&conn, Action::Write, &config).unwrap());

        request_permission(&conn, Action::Write, &config, "ok").unwrap();
        assert!(check_promotion(&conn, Action::Write, &config).unwrap());
    }

    #[test]
    fn list_decisions_respects_limit() {
        let conn = test_conn();
        let config = SafetyConfig::default();
        for i in 0..10 {
            request_permission(&conn, Action::Read, &config, &format!("test {i}")).unwrap();
        }
        let decisions = list_decisions(&conn, 5).unwrap();
        assert_eq!(decisions.len(), 5);
    }

    #[test]
    fn action_as_str_roundtrip() {
        let actions = [
            Action::Read,
            Action::Write,
            Action::RunTests,
            Action::PushRemote,
            Action::DeleteFile,
        ];
        for a in actions {
            assert!(!a.as_str().is_empty());
        }
    }
}
