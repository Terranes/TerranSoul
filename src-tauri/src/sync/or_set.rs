/// OR-Set (Observed-Remove Set) CRDT for agent status tracking.
///
/// Elements are added with a unique tag (HLC + site). Remove operations
/// only remove elements with specific tags that were observed, so
/// concurrent add + remove both survive (add wins for unseen tags).
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{CrdtState, SyncOp, HLC, SiteId};

/// A unique tag identifying a specific add operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag {
    pub hlc: HLC,
    pub site: SiteId,
}

/// OR-Set: each element can be present multiple times (with different tags).
/// An element is "in the set" if it has at least one live tag.
pub struct OrSet {
    crdt_id: String,
    /// key → set of tags that added this key.
    entries: HashMap<String, HashSet<Tag>>,
}

impl OrSet {
    pub fn new(crdt_id: &str) -> Self {
        Self {
            crdt_id: crdt_id.to_string(),
            entries: HashMap::new(),
        }
    }

    /// Add an element. Returns the sync op.
    pub fn add(&mut self, key: &str, hlc: HLC, site: &str) -> SyncOp {
        let tag = Tag {
            hlc,
            site: site.to_string(),
        };
        self.entries
            .entry(key.to_string())
            .or_default()
            .insert(tag);

        SyncOp {
            crdt_id: self.crdt_id.clone(),
            kind: "add".to_string(),
            hlc,
            site: site.to_string(),
            payload: serde_json::json!({ "key": key }),
        }
    }

    /// Remove an element by removing all currently observed tags for it.
    /// Returns the sync op containing the tags to remove.
    pub fn remove(&mut self, key: &str, hlc: HLC, site: &str) -> SyncOp {
        let removed_tags: Vec<Tag> = self
            .entries
            .get(key)
            .map(|tags| tags.iter().cloned().collect())
            .unwrap_or_default();

        if let Some(tags) = self.entries.get_mut(key) {
            tags.clear();
        }

        SyncOp {
            crdt_id: self.crdt_id.clone(),
            kind: "remove".to_string(),
            hlc,
            site: site.to_string(),
            payload: serde_json::json!({
                "key": key,
                "tags": removed_tags,
            }),
        }
    }

    /// Check if an element is in the set.
    pub fn contains(&self, key: &str) -> bool {
        self.entries
            .get(key)
            .map(|tags| !tags.is_empty())
            .unwrap_or(false)
    }

    /// Return all elements currently in the set.
    pub fn elements(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|(_, tags)| !tags.is_empty())
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Number of distinct elements in the set.
    pub fn len(&self) -> usize {
        self.entries.values().filter(|tags| !tags.is_empty()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl CrdtState for OrSet {
    fn apply(&mut self, op: &SyncOp) -> Result<(), String> {
        match op.kind.as_str() {
            "add" => {
                let key = op.payload["key"]
                    .as_str()
                    .ok_or("missing key in add payload")?;
                let tag = Tag {
                    hlc: op.hlc,
                    site: op.site.clone(),
                };
                self.entries.entry(key.to_string()).or_default().insert(tag);
                Ok(())
            }
            "remove" => {
                let key = op.payload["key"]
                    .as_str()
                    .ok_or("missing key in remove payload")?;
                let tags_json = op.payload["tags"]
                    .as_array()
                    .ok_or("missing tags in remove payload")?;
                let tags_to_remove: Vec<Tag> = tags_json
                    .iter()
                    .map(|t| serde_json::from_value(t.clone()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| format!("invalid tag: {e}"))?;

                if let Some(tags) = self.entries.get_mut(key) {
                    for t in &tags_to_remove {
                        tags.remove(t);
                    }
                }
                Ok(())
            }
            other => Err(format!("unexpected op kind: {other}")),
        }
    }

    fn snapshot_ops(&self) -> Vec<SyncOp> {
        let mut ops = Vec::new();
        for (key, tags) in &self.entries {
            for tag in tags {
                ops.push(SyncOp {
                    crdt_id: self.crdt_id.clone(),
                    kind: "add".to_string(),
                    hlc: tag.hlc,
                    site: tag.site.clone(),
                    payload: serde_json::json!({ "key": key }),
                });
            }
        }
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_set() {
        let set = OrSet::new("agents");
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert!(!set.contains("stub"));
    }

    #[test]
    fn add_and_contains() {
        let mut set = OrSet::new("agents");
        set.add("stub", HLC::new(1, 0), "a");
        assert!(set.contains("stub"));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn remove_element() {
        let mut set = OrSet::new("agents");
        set.add("stub", HLC::new(1, 0), "a");
        set.remove("stub", HLC::new(2, 0), "a");
        assert!(!set.contains("stub"));
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn add_after_remove_readds() {
        let mut set = OrSet::new("agents");
        set.add("stub", HLC::new(1, 0), "a");
        set.remove("stub", HLC::new(2, 0), "a");
        set.add("stub", HLC::new(3, 0), "a");
        assert!(set.contains("stub"));
    }

    #[test]
    fn concurrent_add_and_remove_add_wins() {
        // Device A adds "stub" with tag (1,0)
        let mut set_a = OrSet::new("agents");
        let op_add = set_a.add("stub", HLC::new(1, 0), "device-a");

        // Device B also adds "stub" with tag (1,1)
        let mut set_b = OrSet::new("agents");
        let op_add_b = set_b.add("stub", HLC::new(1, 1), "device-b");

        // Now device A removes "stub" — it only removes the tags it has seen: {(1,0)}
        let op_remove = set_a.remove("stub", HLC::new(2, 0), "device-a");

        // Apply all ops to both sets
        // set_a: already has op_add applied, apply op_add_b and op_remove
        set_a.apply(&op_add_b).unwrap();
        // set_a.remove already happened locally above

        // set_b: already has op_add_b, apply op_add and op_remove
        set_b.apply(&op_add).unwrap();
        set_b.apply(&op_remove).unwrap();

        // Both sets should still contain "stub" because device B's tag (1,1) was
        // not included in the remove op (it hadn't been observed yet)
        assert!(set_a.contains("stub"));
        assert!(set_b.contains("stub"));
    }

    #[test]
    fn apply_add_op() {
        let mut set = OrSet::new("agents");
        let op = SyncOp {
            crdt_id: "agents".to_string(),
            kind: "add".to_string(),
            hlc: HLC::new(1, 0),
            site: "remote".to_string(),
            payload: serde_json::json!({"key": "gpt-agent"}),
        };
        set.apply(&op).unwrap();
        assert!(set.contains("gpt-agent"));
    }

    #[test]
    fn apply_remove_op() {
        let mut set = OrSet::new("agents");
        // First add
        set.add("stub", HLC::new(1, 0), "a");
        // Build a remove op that targets the tag we just added
        let tag = Tag {
            hlc: HLC::new(1, 0),
            site: "a".to_string(),
        };
        let op = SyncOp {
            crdt_id: "agents".to_string(),
            kind: "remove".to_string(),
            hlc: HLC::new(2, 0),
            site: "a".to_string(),
            payload: serde_json::json!({"key": "stub", "tags": [tag]}),
        };
        set.apply(&op).unwrap();
        assert!(!set.contains("stub"));
    }

    #[test]
    fn apply_rejects_wrong_kind() {
        let mut set = OrSet::new("agents");
        let op = SyncOp {
            crdt_id: "agents".to_string(),
            kind: "append".to_string(),
            hlc: HLC::new(1, 0),
            site: "a".to_string(),
            payload: serde_json::json!(null),
        };
        assert!(set.apply(&op).is_err());
    }

    #[test]
    fn elements_returns_all_live_keys() {
        let mut set = OrSet::new("agents");
        set.add("stub", HLC::new(1, 0), "a");
        set.add("gpt", HLC::new(2, 0), "a");
        set.add("claude", HLC::new(3, 0), "a");
        let mut elems = set.elements();
        elems.sort();
        assert_eq!(elems, vec!["claude", "gpt", "stub"]);
    }

    #[test]
    fn snapshot_and_restore() {
        let mut set = OrSet::new("agents");
        set.add("stub", HLC::new(1, 0), "a");
        set.add("gpt", HLC::new(2, 1), "b");

        let ops = set.snapshot_ops();
        assert_eq!(ops.len(), 2);

        let mut restored = OrSet::new("agents");
        for op in &ops {
            restored.apply(op).unwrap();
        }
        assert!(restored.contains("stub"));
        assert!(restored.contains("gpt"));
        assert_eq!(restored.len(), 2);
    }

    #[test]
    fn convergence_after_cross_apply() {
        let mut set_a = OrSet::new("agents");
        let mut set_b = OrSet::new("agents");

        let op_a1 = set_a.add("agent-x", HLC::new(1, 0), "a");
        let op_b1 = set_b.add("agent-y", HLC::new(1, 1), "b");

        // Cross-apply
        set_a.apply(&op_b1).unwrap();
        set_b.apply(&op_a1).unwrap();

        // Both should contain both agents
        assert!(set_a.contains("agent-x"));
        assert!(set_a.contains("agent-y"));
        assert!(set_b.contains("agent-x"));
        assert!(set_b.contains("agent-y"));
    }
}
