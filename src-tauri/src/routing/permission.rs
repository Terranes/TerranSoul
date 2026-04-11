/// Permission management for remote commands.
///
/// The first remote command from a new device requires explicit user approval.
/// Once approved, that device is "trusted for remote" and subsequent commands
/// are auto-approved. The user can revoke trust at any time.
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Per-device permission policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionPolicy {
    /// Always allow remote commands from this device.
    Allow,
    /// Always deny remote commands from this device.
    Deny,
    /// Ask the user every time (default for unknown devices).
    Ask,
}

/// In-memory permission store.
pub struct PermissionStore {
    /// device_id → policy
    policies: HashMap<String, PermissionPolicy>,
    /// Commands pending user approval: command_id set
    pending: HashSet<String>,
}

impl PermissionStore {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            pending: HashSet::new(),
        }
    }

    /// Get the policy for a device (defaults to `Ask` if unknown).
    pub fn policy_for(&self, device_id: &str) -> PermissionPolicy {
        self.policies
            .get(device_id)
            .copied()
            .unwrap_or(PermissionPolicy::Ask)
    }

    /// Set the policy for a device.
    pub fn set_policy(&mut self, device_id: &str, policy: PermissionPolicy) {
        self.policies.insert(device_id.to_string(), policy);
    }

    /// Check whether a command from `device_id` should be allowed.
    /// Returns `Some(true)` for Allow, `Some(false)` for Deny, `None` for Ask.
    pub fn check(&self, device_id: &str) -> Option<bool> {
        match self.policy_for(device_id) {
            PermissionPolicy::Allow => Some(true),
            PermissionPolicy::Deny => Some(false),
            PermissionPolicy::Ask => None,
        }
    }

    /// Mark a command as pending user approval.
    pub fn add_pending(&mut self, command_id: &str) {
        self.pending.insert(command_id.to_string());
    }

    /// Check if a command is pending approval.
    pub fn is_pending(&self, command_id: &str) -> bool {
        self.pending.contains(command_id)
    }

    /// Approve a pending command and optionally remember the device.
    /// Returns `true` if the command was pending and is now approved.
    pub fn approve(&mut self, command_id: &str, device_id: &str, remember: bool) -> bool {
        let was_pending = self.pending.remove(command_id);
        if remember {
            self.set_policy(device_id, PermissionPolicy::Allow);
        }
        was_pending
    }

    /// Deny a pending command and optionally block the device.
    pub fn deny(&mut self, command_id: &str, device_id: &str, block: bool) -> bool {
        let was_pending = self.pending.remove(command_id);
        if block {
            self.set_policy(device_id, PermissionPolicy::Deny);
        }
        was_pending
    }

    /// List all pending command IDs.
    pub fn pending_commands(&self) -> Vec<String> {
        self.pending.iter().cloned().collect()
    }

    /// List all devices and their policies.
    pub fn all_policies(&self) -> Vec<(String, PermissionPolicy)> {
        self.policies
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }
}

impl Default for PermissionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_device_defaults_to_ask() {
        let store = PermissionStore::new();
        assert_eq!(store.policy_for("unknown"), PermissionPolicy::Ask);
        assert_eq!(store.check("unknown"), None);
    }

    #[test]
    fn set_and_check_allow() {
        let mut store = PermissionStore::new();
        store.set_policy("phone", PermissionPolicy::Allow);
        assert_eq!(store.check("phone"), Some(true));
    }

    #[test]
    fn set_and_check_deny() {
        let mut store = PermissionStore::new();
        store.set_policy("phone", PermissionPolicy::Deny);
        assert_eq!(store.check("phone"), Some(false));
    }

    #[test]
    fn pending_command_lifecycle() {
        let mut store = PermissionStore::new();
        store.add_pending("cmd-1");
        assert!(store.is_pending("cmd-1"));
        assert!(!store.is_pending("cmd-2"));
        assert_eq!(store.pending_commands().len(), 1);
    }

    #[test]
    fn approve_removes_pending_and_remembers() {
        let mut store = PermissionStore::new();
        store.add_pending("cmd-1");
        let approved = store.approve("cmd-1", "phone", true);
        assert!(approved);
        assert!(!store.is_pending("cmd-1"));
        assert_eq!(store.policy_for("phone"), PermissionPolicy::Allow);
    }

    #[test]
    fn approve_without_remember_does_not_set_policy() {
        let mut store = PermissionStore::new();
        store.add_pending("cmd-1");
        store.approve("cmd-1", "phone", false);
        assert_eq!(store.policy_for("phone"), PermissionPolicy::Ask);
    }

    #[test]
    fn deny_removes_pending_and_blocks() {
        let mut store = PermissionStore::new();
        store.add_pending("cmd-1");
        let denied = store.deny("cmd-1", "phone", true);
        assert!(denied);
        assert!(!store.is_pending("cmd-1"));
        assert_eq!(store.policy_for("phone"), PermissionPolicy::Deny);
    }

    #[test]
    fn deny_without_block_does_not_set_policy() {
        let mut store = PermissionStore::new();
        store.add_pending("cmd-1");
        store.deny("cmd-1", "phone", false);
        assert_eq!(store.policy_for("phone"), PermissionPolicy::Ask);
    }

    #[test]
    fn approve_non_pending_returns_false() {
        let mut store = PermissionStore::new();
        let approved = store.approve("non-existent", "phone", true);
        assert!(!approved);
    }

    #[test]
    fn all_policies_lists_set_policies() {
        let mut store = PermissionStore::new();
        store.set_policy("phone", PermissionPolicy::Allow);
        store.set_policy("tablet", PermissionPolicy::Deny);
        let policies = store.all_policies();
        assert_eq!(policies.len(), 2);
    }
}
