/// Command envelope — the wire format for remote command routing.
///
/// A secondary device (e.g. phone) sends a `CommandEnvelope` targeting
/// a primary device (e.g. PC).  The target device executes the command
/// and returns a `CommandResult`.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a routed command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    /// Awaiting permission approval on the target device.
    PendingApproval,
    /// Approved and being executed.
    Executing,
    /// Completed successfully.
    Completed,
    /// Denied by the target device's permission policy.
    Denied,
    /// Execution failed.
    Failed,
}

/// A command sent from one device to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEnvelope {
    /// Unique command identifier.
    pub command_id: String,
    /// Device that originated the command.
    pub origin_device: String,
    /// Target device that should execute the command.
    pub target_device: String,
    /// The command type (e.g. "send_message", "load_vrm", "list_agents").
    pub command_type: String,
    /// Opaque JSON payload for the command.
    pub payload: serde_json::Value,
    /// Current status.
    pub status: CommandStatus,
}

impl CommandEnvelope {
    /// Create a new command envelope with a fresh UUID.
    pub fn new(
        origin_device: &str,
        target_device: &str,
        command_type: &str,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4().to_string(),
            origin_device: origin_device.to_string(),
            target_device: target_device.to_string(),
            command_type: command_type.to_string(),
            payload,
            status: CommandStatus::PendingApproval,
        }
    }
}

/// The result returned to the originating device after execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// The command_id this result corresponds to.
    pub command_id: String,
    /// Final status.
    pub status: CommandStatus,
    /// Result payload (command output or error message).
    pub payload: serde_json::Value,
}

impl CommandResult {
    pub fn success(command_id: &str, payload: serde_json::Value) -> Self {
        Self {
            command_id: command_id.to_string(),
            status: CommandStatus::Completed,
            payload,
        }
    }

    pub fn denied(command_id: &str, reason: &str) -> Self {
        Self {
            command_id: command_id.to_string(),
            status: CommandStatus::Denied,
            payload: serde_json::json!({ "reason": reason }),
        }
    }

    pub fn failed(command_id: &str, error: &str) -> Self {
        Self {
            command_id: command_id.to_string(),
            status: CommandStatus::Failed,
            payload: serde_json::json!({ "error": error }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_new_has_pending_status() {
        let env = CommandEnvelope::new("phone", "pc", "send_message", serde_json::json!({"text": "hi"}));
        assert_eq!(env.status, CommandStatus::PendingApproval);
        assert_eq!(env.origin_device, "phone");
        assert_eq!(env.target_device, "pc");
        assert_eq!(env.command_type, "send_message");
        assert!(!env.command_id.is_empty());
    }

    #[test]
    fn envelope_json_roundtrip() {
        let env = CommandEnvelope::new("a", "b", "list_agents", serde_json::json!(null));
        let json = serde_json::to_string(&env).unwrap();
        let restored: CommandEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.command_id, env.command_id);
        assert_eq!(restored.origin_device, "a");
        assert_eq!(restored.target_device, "b");
        assert_eq!(restored.command_type, "list_agents");
        assert_eq!(restored.status, CommandStatus::PendingApproval);
    }

    #[test]
    fn command_status_serde() {
        for status in [
            CommandStatus::PendingApproval,
            CommandStatus::Executing,
            CommandStatus::Completed,
            CommandStatus::Denied,
            CommandStatus::Failed,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let restored: CommandStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, status);
        }
    }

    #[test]
    fn command_status_snake_case() {
        assert_eq!(
            serde_json::to_string(&CommandStatus::PendingApproval).unwrap(),
            "\"pending_approval\""
        );
    }

    #[test]
    fn result_success() {
        let result = CommandResult::success("cmd-1", serde_json::json!({"agents": ["stub"]}));
        assert_eq!(result.command_id, "cmd-1");
        assert_eq!(result.status, CommandStatus::Completed);
        assert_eq!(result.payload["agents"][0], "stub");
    }

    #[test]
    fn result_denied() {
        let result = CommandResult::denied("cmd-2", "first remote command not approved");
        assert_eq!(result.status, CommandStatus::Denied);
        assert_eq!(result.payload["reason"], "first remote command not approved");
    }

    #[test]
    fn result_failed() {
        let result = CommandResult::failed("cmd-3", "agent not found");
        assert_eq!(result.status, CommandStatus::Failed);
        assert_eq!(result.payload["error"], "agent not found");
    }
}
