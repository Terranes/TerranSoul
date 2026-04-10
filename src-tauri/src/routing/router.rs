/// Command router — receives remote command envelopes, runs permission
/// checks, executes allowed commands, and produces results.
use super::command_envelope::{CommandEnvelope, CommandResult, CommandStatus};
use super::permission::{PermissionPolicy, PermissionStore};

/// Routes remote commands through permission checks and execution.
pub struct CommandRouter {
    /// This device's device_id (the target).
    device_id: String,
    /// Permission store.
    permissions: PermissionStore,
    /// Commands awaiting execution after approval.
    pending_envelopes: Vec<CommandEnvelope>,
}

impl CommandRouter {
    pub fn new(device_id: &str) -> Self {
        Self {
            device_id: device_id.to_string(),
            permissions: PermissionStore::new(),
            pending_envelopes: Vec::new(),
        }
    }

    /// Handle an incoming command from a remote device.
    /// Returns the immediate result or `None` if the command is pending approval.
    pub fn handle_incoming(&mut self, mut envelope: CommandEnvelope) -> Option<CommandResult> {
        // Verify this device is the target
        if envelope.target_device != self.device_id {
            return Some(CommandResult::denied(
                &envelope.command_id,
                &format!("wrong target: expected {}, got {}", self.device_id, envelope.target_device),
            ));
        }

        match self.permissions.check(&envelope.origin_device) {
            Some(true) => {
                // Allowed — execute immediately
                envelope.status = CommandStatus::Executing;
                Some(self.execute(&envelope))
            }
            Some(false) => {
                // Denied
                Some(CommandResult::denied(&envelope.command_id, "device is blocked"))
            }
            None => {
                // Ask — put it in pending
                envelope.status = CommandStatus::PendingApproval;
                self.permissions.add_pending(&envelope.command_id);
                self.pending_envelopes.push(envelope);
                None
            }
        }
    }

    /// User approves a pending command. Executes and returns the result.
    pub fn approve_command(&mut self, command_id: &str, remember: bool) -> Option<CommandResult> {
        // Find the pending envelope
        let idx = self
            .pending_envelopes
            .iter()
            .position(|e| e.command_id == command_id)?;

        let mut envelope = self.pending_envelopes.remove(idx);
        self.permissions.approve(command_id, &envelope.origin_device, remember);
        envelope.status = CommandStatus::Executing;
        Some(self.execute(&envelope))
    }

    /// User denies a pending command.
    pub fn deny_command(&mut self, command_id: &str, block: bool) -> Option<CommandResult> {
        let idx = self
            .pending_envelopes
            .iter()
            .position(|e| e.command_id == command_id)?;

        let envelope = self.pending_envelopes.remove(idx);
        self.permissions.deny(command_id, &envelope.origin_device, block);
        Some(CommandResult::denied(&envelope.command_id, "user denied"))
    }

    /// Execute a command (after permission has been granted).
    fn execute(&self, envelope: &CommandEnvelope) -> CommandResult {
        match envelope.command_type.as_str() {
            "ping" => CommandResult::success(&envelope.command_id, serde_json::json!("pong")),
            "list_agents" => {
                // Stub — in production this would call the orchestrator
                CommandResult::success(
                    &envelope.command_id,
                    serde_json::json!(["stub"]),
                )
            }
            "send_message" => {
                let text = envelope.payload.get("text").and_then(|v| v.as_str()).unwrap_or("");
                if text.is_empty() {
                    return CommandResult::failed(&envelope.command_id, "empty message");
                }
                CommandResult::success(
                    &envelope.command_id,
                    serde_json::json!({"queued": true, "text": text}),
                )
            }
            other => CommandResult::failed(
                &envelope.command_id,
                &format!("unknown command type: {other}"),
            ),
        }
    }

    /// List pending commands awaiting approval.
    pub fn pending_commands(&self) -> &[CommandEnvelope] {
        &self.pending_envelopes
    }

    /// Get the permission store (for Tauri commands).
    pub fn permissions(&self) -> &PermissionStore {
        &self.permissions
    }

    /// Mutable permission store access.
    pub fn permissions_mut(&mut self) -> &mut PermissionStore {
        &mut self.permissions
    }

    /// This device's ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::command_envelope::CommandEnvelope;

    fn make_router() -> CommandRouter {
        CommandRouter::new("pc-device")
    }

    fn make_envelope(origin: &str, cmd_type: &str, payload: serde_json::Value) -> CommandEnvelope {
        CommandEnvelope::new(origin, "pc-device", cmd_type, payload)
    }

    #[test]
    fn wrong_target_is_denied() {
        let mut router = make_router();
        let env = CommandEnvelope::new("phone", "other-device", "ping", serde_json::json!(null));
        let result = router.handle_incoming(env).unwrap();
        assert_eq!(result.status, CommandStatus::Denied);
        assert!(result.payload["reason"].as_str().unwrap().contains("wrong target"));
    }

    #[test]
    fn unknown_device_goes_to_pending() {
        let mut router = make_router();
        let env = make_envelope("phone", "ping", serde_json::json!(null));
        let result = router.handle_incoming(env);
        assert!(result.is_none()); // pending
        assert_eq!(router.pending_commands().len(), 1);
    }

    #[test]
    fn allowed_device_executes_immediately() {
        let mut router = make_router();
        router.permissions_mut().set_policy("phone", PermissionPolicy::Allow);
        let env = make_envelope("phone", "ping", serde_json::json!(null));
        let result = router.handle_incoming(env).unwrap();
        assert_eq!(result.status, CommandStatus::Completed);
        assert_eq!(result.payload, "pong");
    }

    #[test]
    fn blocked_device_is_denied() {
        let mut router = make_router();
        router.permissions_mut().set_policy("phone", PermissionPolicy::Deny);
        let env = make_envelope("phone", "ping", serde_json::json!(null));
        let result = router.handle_incoming(env).unwrap();
        assert_eq!(result.status, CommandStatus::Denied);
    }

    #[test]
    fn approve_pending_command() {
        let mut router = make_router();
        let env = make_envelope("phone", "ping", serde_json::json!(null));
        let cmd_id = env.command_id.clone();
        router.handle_incoming(env);

        let result = router.approve_command(&cmd_id, false).unwrap();
        assert_eq!(result.status, CommandStatus::Completed);
        assert_eq!(result.payload, "pong");
        assert!(router.pending_commands().is_empty());
    }

    #[test]
    fn approve_with_remember_sets_allow_policy() {
        let mut router = make_router();
        let env = make_envelope("phone", "ping", serde_json::json!(null));
        let cmd_id = env.command_id.clone();
        router.handle_incoming(env);
        router.approve_command(&cmd_id, true);

        assert_eq!(
            router.permissions().policy_for("phone"),
            PermissionPolicy::Allow
        );
    }

    #[test]
    fn deny_pending_command() {
        let mut router = make_router();
        let env = make_envelope("phone", "ping", serde_json::json!(null));
        let cmd_id = env.command_id.clone();
        router.handle_incoming(env);

        let result = router.deny_command(&cmd_id, false).unwrap();
        assert_eq!(result.status, CommandStatus::Denied);
        assert!(router.pending_commands().is_empty());
    }

    #[test]
    fn deny_with_block_sets_deny_policy() {
        let mut router = make_router();
        let env = make_envelope("phone", "ping", serde_json::json!(null));
        let cmd_id = env.command_id.clone();
        router.handle_incoming(env);
        router.deny_command(&cmd_id, true);

        assert_eq!(
            router.permissions().policy_for("phone"),
            PermissionPolicy::Deny
        );
    }

    #[test]
    fn execute_list_agents() {
        let mut router = make_router();
        router.permissions_mut().set_policy("phone", PermissionPolicy::Allow);
        let env = make_envelope("phone", "list_agents", serde_json::json!(null));
        let result = router.handle_incoming(env).unwrap();
        assert_eq!(result.status, CommandStatus::Completed);
        assert!(result.payload.as_array().unwrap().contains(&serde_json::json!("stub")));
    }

    #[test]
    fn execute_send_message() {
        let mut router = make_router();
        router.permissions_mut().set_policy("phone", PermissionPolicy::Allow);
        let env = make_envelope("phone", "send_message", serde_json::json!({"text": "hello"}));
        let result = router.handle_incoming(env).unwrap();
        assert_eq!(result.status, CommandStatus::Completed);
        assert_eq!(result.payload["queued"], true);
        assert_eq!(result.payload["text"], "hello");
    }

    #[test]
    fn execute_send_message_empty_fails() {
        let mut router = make_router();
        router.permissions_mut().set_policy("phone", PermissionPolicy::Allow);
        let env = make_envelope("phone", "send_message", serde_json::json!({"text": ""}));
        let result = router.handle_incoming(env).unwrap();
        assert_eq!(result.status, CommandStatus::Failed);
    }

    #[test]
    fn unknown_command_type_fails() {
        let mut router = make_router();
        router.permissions_mut().set_policy("phone", PermissionPolicy::Allow);
        let env = make_envelope("phone", "hack_the_planet", serde_json::json!(null));
        let result = router.handle_incoming(env).unwrap();
        assert_eq!(result.status, CommandStatus::Failed);
        assert!(result.payload["error"].as_str().unwrap().contains("unknown command type"));
    }

    #[test]
    fn approve_nonexistent_returns_none() {
        let mut router = make_router();
        assert!(router.approve_command("no-such-cmd", false).is_none());
    }

    #[test]
    fn multiple_pending_commands() {
        let mut router = make_router();
        let env1 = make_envelope("phone", "ping", serde_json::json!(null));
        let env2 = make_envelope("phone", "list_agents", serde_json::json!(null));
        let id1 = env1.command_id.clone();
        let id2 = env2.command_id.clone();
        router.handle_incoming(env1);
        router.handle_incoming(env2);
        assert_eq!(router.pending_commands().len(), 2);

        // Approve first, deny second
        let r1 = router.approve_command(&id1, false).unwrap();
        assert_eq!(r1.status, CommandStatus::Completed);

        let r2 = router.deny_command(&id2, false).unwrap();
        assert_eq!(r2.status, CommandStatus::Denied);

        assert!(router.pending_commands().is_empty());
    }
}
