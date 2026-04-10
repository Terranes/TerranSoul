/// Last-Write-Wins Register CRDT for character selection and similar
/// single-value state that resolves to the most recent write.
///
/// Ties are broken by site_ord (higher wins).
use serde::{Deserialize, Serialize};

use super::{CrdtState, SyncOp, HLC, SiteId};

/// A timestamped value.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Stamped {
    hlc: HLC,
    site: SiteId,
    value: serde_json::Value,
}

/// LWW register — stores the "latest" value across all sites.
pub struct LwwRegister {
    crdt_id: String,
    current: Option<Stamped>,
}

impl LwwRegister {
    pub fn new(crdt_id: &str) -> Self {
        Self {
            crdt_id: crdt_id.to_string(),
            current: None,
        }
    }

    /// Set the value locally.  Returns the sync op to broadcast.
    pub fn set(&mut self, hlc: HLC, site: &str, value: serde_json::Value) -> SyncOp {
        let stamped = Stamped {
            hlc,
            site: site.to_string(),
            value: value.clone(),
        };
        self.merge_stamped(stamped);

        SyncOp {
            crdt_id: self.crdt_id.clone(),
            kind: "set".to_string(),
            hlc,
            site: site.to_string(),
            payload: value,
        }
    }

    /// Read the current value (if any).
    pub fn get(&self) -> Option<&serde_json::Value> {
        self.current.as_ref().map(|s| &s.value)
    }

    /// Merge a remote stamped value: keep the one with the higher HLC.
    fn merge_stamped(&mut self, incoming: Stamped) {
        match &self.current {
            None => {
                self.current = Some(incoming);
            }
            Some(existing) => {
                if incoming.hlc > existing.hlc {
                    self.current = Some(incoming);
                }
                // If equal HLC, existing wins (first-write-wins for ties at same HLC)
            }
        }
    }
}

impl CrdtState for LwwRegister {
    fn apply(&mut self, op: &SyncOp) -> Result<(), String> {
        if op.kind != "set" {
            return Err(format!("unexpected op kind: {}", op.kind));
        }
        let stamped = Stamped {
            hlc: op.hlc,
            site: op.site.clone(),
            value: op.payload.clone(),
        };
        self.merge_stamped(stamped);
        Ok(())
    }

    fn snapshot_ops(&self) -> Vec<SyncOp> {
        match &self.current {
            Some(s) => vec![SyncOp {
                crdt_id: self.crdt_id.clone(),
                kind: "set".to_string(),
                hlc: s.hlc,
                site: s.site.clone(),
                payload: s.value.clone(),
            }],
            None => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_register_returns_none() {
        let reg = LwwRegister::new("char_select");
        assert!(reg.get().is_none());
    }

    #[test]
    fn set_and_get() {
        let mut reg = LwwRegister::new("char_select");
        reg.set(HLC::new(1, 0), "a", serde_json::json!("model_a.vrm"));
        assert_eq!(reg.get().unwrap(), "model_a.vrm");
    }

    #[test]
    fn later_write_wins() {
        let mut reg = LwwRegister::new("char_select");
        reg.set(HLC::new(1, 0), "a", serde_json::json!("old"));
        reg.set(HLC::new(2, 0), "a", serde_json::json!("new"));
        assert_eq!(reg.get().unwrap(), "new");
    }

    #[test]
    fn earlier_write_does_not_overwrite() {
        let mut reg = LwwRegister::new("char_select");
        reg.set(HLC::new(5, 0), "a", serde_json::json!("latest"));
        reg.set(HLC::new(2, 0), "a", serde_json::json!("stale"));
        assert_eq!(reg.get().unwrap(), "latest");
    }

    #[test]
    fn concurrent_writes_higher_site_wins() {
        let mut reg = LwwRegister::new("char_select");
        // Same counter, site_ord 1 > site_ord 0
        reg.set(HLC::new(5, 0), "a", serde_json::json!("from A"));
        reg.set(HLC::new(5, 1), "b", serde_json::json!("from B"));
        assert_eq!(reg.get().unwrap(), "from B");
    }

    #[test]
    fn apply_remote_op() {
        let mut reg = LwwRegister::new("char_select");
        reg.set(HLC::new(1, 0), "a", serde_json::json!("local"));
        let remote_op = SyncOp {
            crdt_id: "char_select".to_string(),
            kind: "set".to_string(),
            hlc: HLC::new(3, 1),
            site: "b".to_string(),
            payload: serde_json::json!("remote"),
        };
        reg.apply(&remote_op).unwrap();
        assert_eq!(reg.get().unwrap(), "remote");
    }

    #[test]
    fn apply_stale_remote_op_has_no_effect() {
        let mut reg = LwwRegister::new("char_select");
        reg.set(HLC::new(10, 0), "a", serde_json::json!("current"));
        let stale = SyncOp {
            crdt_id: "char_select".to_string(),
            kind: "set".to_string(),
            hlc: HLC::new(2, 1),
            site: "b".to_string(),
            payload: serde_json::json!("old"),
        };
        reg.apply(&stale).unwrap();
        assert_eq!(reg.get().unwrap(), "current");
    }

    #[test]
    fn apply_rejects_wrong_kind() {
        let mut reg = LwwRegister::new("char_select");
        let op = SyncOp {
            crdt_id: "char_select".to_string(),
            kind: "append".to_string(),
            hlc: HLC::new(1, 0),
            site: "a".to_string(),
            payload: serde_json::json!(null),
        };
        assert!(reg.apply(&op).is_err());
    }

    #[test]
    fn snapshot_empty_register() {
        let reg = LwwRegister::new("x");
        assert!(reg.snapshot_ops().is_empty());
    }

    #[test]
    fn snapshot_and_restore() {
        let mut reg = LwwRegister::new("char_select");
        reg.set(HLC::new(5, 0), "a", serde_json::json!("chosen.vrm"));

        let ops = reg.snapshot_ops();
        assert_eq!(ops.len(), 1);

        let mut restored = LwwRegister::new("char_select");
        for op in &ops {
            restored.apply(op).unwrap();
        }
        assert_eq!(restored.get().unwrap(), "chosen.vrm");
    }

    #[test]
    fn concurrent_edits_converge() {
        // Simulate two devices making concurrent writes
        let mut reg_a = LwwRegister::new("char_select");
        let mut reg_b = LwwRegister::new("char_select");

        let op_a = reg_a.set(HLC::new(3, 0), "device-a", serde_json::json!("A's pick"));
        let op_b = reg_b.set(HLC::new(3, 1), "device-b", serde_json::json!("B's pick"));

        // Cross-apply
        reg_a.apply(&op_b).unwrap();
        reg_b.apply(&op_a).unwrap();

        // Both should converge to the same value (B wins: higher site_ord)
        assert_eq!(reg_a.get(), reg_b.get());
        assert_eq!(reg_a.get().unwrap(), "B's pick");
    }
}
