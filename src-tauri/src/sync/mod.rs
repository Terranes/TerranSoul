pub mod append_log;
pub mod lww_register;
pub mod or_set;

use serde::{Deserialize, Serialize};

/// Globally-unique site identifier (device_id).
pub type SiteId = String;

/// Lamport-style logical timestamp.
/// Monotonically increasing per-site; ties broken by `SiteId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HLC {
    pub counter: u64,
    pub site_ord: u32,
}

impl HLC {
    pub fn new(counter: u64, site_ord: u32) -> Self {
        Self { counter, site_ord }
    }

    /// Increment the counter and return a new HLC for this site.
    pub fn tick(&self) -> Self {
        Self {
            counter: self.counter + 1,
            site_ord: self.site_ord,
        }
    }

    /// Merge with a remote HLC: take the max counter + 1.
    pub fn merge(&self, remote: &HLC) -> Self {
        let max_counter = self.counter.max(remote.counter);
        Self {
            counter: max_counter + 1,
            site_ord: self.site_ord,
        }
    }
}

/// A single sync operation that can be sent between devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOp {
    /// Which CRDT this op targets.
    pub crdt_id: String,
    /// The operation kind (e.g. "append", "set", "add", "remove").
    pub kind: String,
    /// Timestamp for ordering.
    pub hlc: HLC,
    /// Originating device.
    pub site: SiteId,
    /// Opaque payload.
    pub payload: serde_json::Value,
}

/// Trait implemented by all CRDT containers.
pub trait CrdtState: Send + Sync {
    /// Apply a remote operation.
    fn apply(&mut self, op: &SyncOp) -> Result<(), String>;

    /// Produce all operations needed to bring a new peer up to date.
    fn snapshot_ops(&self) -> Vec<SyncOp>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hlc_tick_increments_counter() {
        let hlc = HLC::new(5, 1);
        let next = hlc.tick();
        assert_eq!(next.counter, 6);
        assert_eq!(next.site_ord, 1);
    }

    #[test]
    fn hlc_merge_takes_max_plus_one() {
        let local = HLC::new(3, 1);
        let remote = HLC::new(7, 2);
        let merged = local.merge(&remote);
        assert_eq!(merged.counter, 8);
        assert_eq!(merged.site_ord, 1);
    }

    #[test]
    fn hlc_merge_when_local_is_higher() {
        let local = HLC::new(10, 1);
        let remote = HLC::new(2, 2);
        let merged = local.merge(&remote);
        assert_eq!(merged.counter, 11);
    }

    #[test]
    fn hlc_ordering() {
        let a = HLC::new(1, 0);
        let b = HLC::new(2, 0);
        assert!(a < b);
    }

    #[test]
    fn hlc_ordering_tiebreak_by_site() {
        let a = HLC::new(5, 1);
        let b = HLC::new(5, 2);
        assert!(a < b);
    }

    #[test]
    fn sync_op_json_roundtrip() {
        let op = SyncOp {
            crdt_id: "chat".to_string(),
            kind: "append".to_string(),
            hlc: HLC::new(1, 0),
            site: "device-a".to_string(),
            payload: serde_json::json!({"text": "hello"}),
        };
        let json = serde_json::to_string(&op).unwrap();
        let restored: SyncOp = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.crdt_id, "chat");
        assert_eq!(restored.kind, "append");
        assert_eq!(restored.hlc.counter, 1);
        assert_eq!(restored.site, "device-a");
        assert_eq!(restored.payload["text"], "hello");
    }
}
