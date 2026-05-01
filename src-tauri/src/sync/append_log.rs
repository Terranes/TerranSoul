/// Append-only log CRDT for conversation history.
///
/// Each entry has an HLC timestamp and site. Entries from all sites are
/// merged into a single total order: `(hlc.counter, hlc.site_ord, insert_index)`.
/// This is idempotent — re-applying an already-seen entry is a no-op.
use serde::{Deserialize, Serialize};

use super::{CrdtState, SiteId, SyncOp, HLC};

/// A single entry in the append-only log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub hlc: HLC,
    pub site: SiteId,
    pub value: serde_json::Value,
}

/// Append-only log: entries are globally ordered by HLC.
/// Duplicates (same hlc + site) are rejected.
pub struct AppendLog {
    crdt_id: String,
    entries: Vec<LogEntry>,
}

impl AppendLog {
    pub fn new(crdt_id: &str) -> Self {
        Self {
            crdt_id: crdt_id.to_string(),
            entries: Vec::new(),
        }
    }

    /// Append a locally-originated entry.
    pub fn append(&mut self, hlc: HLC, site: &str, value: serde_json::Value) -> SyncOp {
        let entry = LogEntry {
            hlc,
            site: site.to_string(),
            value: value.clone(),
        };
        self.insert_sorted(entry);

        SyncOp {
            crdt_id: self.crdt_id.clone(),
            kind: "append".to_string(),
            hlc,
            site: site.to_string(),
            payload: value,
        }
    }

    /// Read the entire log in order.
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert an entry in sorted position; reject duplicates.
    fn insert_sorted(&mut self, entry: LogEntry) {
        // Check for duplicates
        if self
            .entries
            .iter()
            .any(|e| e.hlc == entry.hlc && e.site == entry.site)
        {
            return; // idempotent — already seen
        }

        // Binary search for insertion point
        let pos = self
            .entries
            .binary_search_by(|e| e.hlc.cmp(&entry.hlc))
            .unwrap_or_else(|p| p);
        self.entries.insert(pos, entry);
    }
}

impl CrdtState for AppendLog {
    fn apply(&mut self, op: &SyncOp) -> Result<(), String> {
        if op.kind != "append" {
            return Err(format!("unexpected op kind: {}", op.kind));
        }
        let entry = LogEntry {
            hlc: op.hlc,
            site: op.site.clone(),
            value: op.payload.clone(),
        };
        self.insert_sorted(entry);
        Ok(())
    }

    fn snapshot_ops(&self) -> Vec<SyncOp> {
        self.entries
            .iter()
            .map(|e| SyncOp {
                crdt_id: self.crdt_id.clone(),
                kind: "append".to_string(),
                hlc: e.hlc,
                site: e.site.clone(),
                payload: e.value.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_read() {
        let mut log = AppendLog::new("chat");
        let hlc = HLC::new(1, 0);
        log.append(hlc, "device-a", serde_json::json!("hello"));
        assert_eq!(log.len(), 1);
        assert_eq!(log.entries()[0].value, "hello");
    }

    #[test]
    fn entries_are_ordered_by_hlc() {
        let mut log = AppendLog::new("chat");
        log.append(HLC::new(3, 0), "a", serde_json::json!("third"));
        log.append(HLC::new(1, 0), "a", serde_json::json!("first"));
        log.append(HLC::new(2, 0), "a", serde_json::json!("second"));
        let entries = log.entries();
        assert_eq!(entries[0].hlc.counter, 1);
        assert_eq!(entries[1].hlc.counter, 2);
        assert_eq!(entries[2].hlc.counter, 3);
    }

    #[test]
    fn duplicate_entries_are_rejected() {
        let mut log = AppendLog::new("chat");
        let hlc = HLC::new(1, 0);
        log.append(hlc, "a", serde_json::json!("hello"));
        log.append(hlc, "a", serde_json::json!("hello"));
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn apply_remote_op() {
        let mut log = AppendLog::new("chat");
        let op = SyncOp {
            crdt_id: "chat".to_string(),
            kind: "append".to_string(),
            hlc: HLC::new(5, 1),
            site: "device-b".to_string(),
            payload: serde_json::json!("remote message"),
        };
        log.apply(&op).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log.entries()[0].site, "device-b");
    }

    #[test]
    fn apply_rejects_wrong_kind() {
        let mut log = AppendLog::new("chat");
        let op = SyncOp {
            crdt_id: "chat".to_string(),
            kind: "set".to_string(),
            hlc: HLC::new(1, 0),
            site: "a".to_string(),
            payload: serde_json::json!(null),
        };
        let result = log.apply(&op);
        assert!(result.is_err());
    }

    #[test]
    fn concurrent_edits_from_two_devices_merge_correctly() {
        let mut log_a = AppendLog::new("chat");
        let mut log_b = AppendLog::new("chat");

        // Device A appends at HLC(1,0)
        let op_a = log_a.append(HLC::new(1, 0), "device-a", serde_json::json!("from A"));
        // Device B appends at HLC(1,1) — same counter, different site
        let op_b = log_b.append(HLC::new(1, 1), "device-b", serde_json::json!("from B"));

        // Apply each other's ops
        log_a.apply(&op_b).unwrap();
        log_b.apply(&op_a).unwrap();

        // Both logs should have the same 2 entries in the same order
        assert_eq!(log_a.len(), 2);
        assert_eq!(log_b.len(), 2);

        // Order: HLC(1,0) < HLC(1,1) — site_ord breaks the tie
        assert_eq!(log_a.entries()[0].site, "device-a");
        assert_eq!(log_a.entries()[1].site, "device-b");
        assert_eq!(log_b.entries()[0].site, "device-a");
        assert_eq!(log_b.entries()[1].site, "device-b");
    }

    #[test]
    fn snapshot_ops_reproduces_log() {
        let mut log = AppendLog::new("chat");
        log.append(HLC::new(1, 0), "a", serde_json::json!("one"));
        log.append(HLC::new(2, 0), "a", serde_json::json!("two"));

        let ops = log.snapshot_ops();
        assert_eq!(ops.len(), 2);

        let mut restored = AppendLog::new("chat");
        for op in &ops {
            restored.apply(op).unwrap();
        }
        assert_eq!(restored.len(), 2);
        assert_eq!(restored.entries()[0].value, "one");
        assert_eq!(restored.entries()[1].value, "two");
    }

    #[test]
    fn idempotent_apply() {
        let mut log = AppendLog::new("chat");
        let op = SyncOp {
            crdt_id: "chat".to_string(),
            kind: "append".to_string(),
            hlc: HLC::new(1, 0),
            site: "a".to_string(),
            payload: serde_json::json!("msg"),
        };
        log.apply(&op).unwrap();
        log.apply(&op).unwrap(); // duplicate
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn is_empty_on_new_log() {
        let log = AppendLog::new("chat");
        assert!(log.is_empty());
    }
}
