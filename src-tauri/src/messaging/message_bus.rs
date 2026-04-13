/// Topic-based publish/subscribe message bus for agent-to-agent communication.
/// Agents can subscribe to topics and publish messages that are fanned out to
/// all subscribers of that topic.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An agent-to-agent message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Unique message ID.
    pub id: String,
    /// Name of the agent that sent the message.
    pub sender: String,
    /// Topic the message was published to.
    pub topic: String,
    /// Arbitrary JSON payload.
    pub payload: serde_json::Value,
    /// Unix timestamp (ms) when the message was created.
    pub timestamp: u64,
}

impl AgentMessage {
    pub fn new(sender: &str, topic: &str, payload: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            sender: sender.to_string(),
            topic: topic.to_string(),
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

/// A subscription record — an agent subscribed to a topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub agent_name: String,
    pub topic: String,
}

/// The message bus: manages subscriptions and delivers messages.
pub struct MessageBus {
    /// topic → set of subscribed agent names
    subscriptions: HashMap<String, Vec<String>>,
    /// agent_name → inbox of unread messages
    inboxes: HashMap<String, Vec<AgentMessage>>,
    /// Maximum messages to retain per agent inbox
    max_inbox_size: usize,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
            inboxes: HashMap::new(),
            max_inbox_size: 100,
        }
    }

    /// Subscribe an agent to a topic.
    pub fn subscribe(&mut self, agent_name: &str, topic: &str) {
        let subscribers = self
            .subscriptions
            .entry(topic.to_string())
            .or_default();
        if !subscribers.contains(&agent_name.to_string()) {
            subscribers.push(agent_name.to_string());
        }
    }

    /// Unsubscribe an agent from a topic.
    pub fn unsubscribe(&mut self, agent_name: &str, topic: &str) {
        if let Some(subscribers) = self.subscriptions.get_mut(topic) {
            subscribers.retain(|s| s != agent_name);
            if subscribers.is_empty() {
                self.subscriptions.remove(topic);
            }
        }
    }

    /// Publish a message to a topic. Fans out to all subscribers except the sender.
    pub fn publish(&mut self, sender: &str, topic: &str, payload: serde_json::Value) -> AgentMessage {
        let msg = AgentMessage::new(sender, topic, payload);

        if let Some(subscribers) = self.subscriptions.get(topic) {
            for sub in subscribers {
                // Don't deliver to the sender
                if sub == sender {
                    continue;
                }
                let inbox = self.inboxes.entry(sub.clone()).or_default();
                inbox.push(msg.clone());
                // Trim inbox if too large
                if inbox.len() > self.max_inbox_size {
                    inbox.remove(0);
                }
            }
        }

        msg
    }

    /// Get and drain unread messages for an agent.
    pub fn get_messages(&mut self, agent_name: &str) -> Vec<AgentMessage> {
        self.inboxes
            .remove(agent_name)
            .unwrap_or_default()
    }

    /// Peek at messages without draining.
    pub fn peek_messages(&self, agent_name: &str) -> Vec<AgentMessage> {
        self.inboxes
            .get(agent_name)
            .cloned()
            .unwrap_or_default()
    }

    /// List all subscriptions for an agent.
    pub fn subscriptions_for(&self, agent_name: &str) -> Vec<String> {
        self.subscriptions
            .iter()
            .filter_map(|(topic, subs)| {
                if subs.contains(&agent_name.to_string()) {
                    Some(topic.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// List all subscribers for a topic.
    pub fn subscribers_for(&self, topic: &str) -> Vec<String> {
        self.subscriptions
            .get(topic)
            .cloned()
            .unwrap_or_default()
    }

    /// Get total pending message count for an agent.
    pub fn pending_count(&self, agent_name: &str) -> usize {
        self.inboxes
            .get(agent_name)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_and_publish_delivers_to_subscriber() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-b", "events");
        bus.publish("agent-a", "events", serde_json::json!({"action": "hello"}));

        let msgs = bus.get_messages("agent-b");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].sender, "agent-a");
        assert_eq!(msgs[0].topic, "events");
        assert_eq!(msgs[0].payload["action"], "hello");
    }

    #[test]
    fn sender_does_not_receive_own_message() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-a", "events");
        bus.subscribe("agent-b", "events");
        bus.publish("agent-a", "events", serde_json::json!("test"));

        assert_eq!(bus.get_messages("agent-a").len(), 0);
        assert_eq!(bus.get_messages("agent-b").len(), 1);
    }

    #[test]
    fn unsubscribe_stops_delivery() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-b", "events");
        bus.unsubscribe("agent-b", "events");
        bus.publish("agent-a", "events", serde_json::json!("test"));

        assert_eq!(bus.get_messages("agent-b").len(), 0);
    }

    #[test]
    fn multiple_subscribers_all_receive() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-b", "topic1");
        bus.subscribe("agent-c", "topic1");
        bus.publish("agent-a", "topic1", serde_json::json!("data"));

        assert_eq!(bus.get_messages("agent-b").len(), 1);
        assert_eq!(bus.get_messages("agent-c").len(), 1);
    }

    #[test]
    fn get_messages_drains_inbox() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-b", "events");
        bus.publish("agent-a", "events", serde_json::json!("msg1"));
        bus.publish("agent-a", "events", serde_json::json!("msg2"));

        let msgs = bus.get_messages("agent-b");
        assert_eq!(msgs.len(), 2);
        // Second call returns empty
        assert_eq!(bus.get_messages("agent-b").len(), 0);
    }

    #[test]
    fn peek_messages_does_not_drain() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-b", "events");
        bus.publish("agent-a", "events", serde_json::json!("msg1"));

        assert_eq!(bus.peek_messages("agent-b").len(), 1);
        assert_eq!(bus.peek_messages("agent-b").len(), 1);
    }

    #[test]
    fn subscriptions_for_agent() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-a", "topic1");
        bus.subscribe("agent-a", "topic2");
        bus.subscribe("agent-b", "topic1");

        let subs = bus.subscriptions_for("agent-a");
        assert_eq!(subs.len(), 2);
        assert!(subs.contains(&"topic1".to_string()));
        assert!(subs.contains(&"topic2".to_string()));
    }

    #[test]
    fn subscribers_for_topic() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-a", "events");
        bus.subscribe("agent-b", "events");

        let subs = bus.subscribers_for("events");
        assert_eq!(subs.len(), 2);
        assert!(subs.contains(&"agent-a".to_string()));
        assert!(subs.contains(&"agent-b".to_string()));
    }

    #[test]
    fn pending_count() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-b", "events");
        assert_eq!(bus.pending_count("agent-b"), 0);
        bus.publish("agent-a", "events", serde_json::json!("m1"));
        bus.publish("agent-a", "events", serde_json::json!("m2"));
        assert_eq!(bus.pending_count("agent-b"), 2);
    }

    #[test]
    fn duplicate_subscribe_is_idempotent() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-b", "events");
        bus.subscribe("agent-b", "events");

        bus.publish("agent-a", "events", serde_json::json!("test"));
        // Should only receive once
        assert_eq!(bus.get_messages("agent-b").len(), 1);
    }

    #[test]
    fn publish_to_empty_topic_no_error() {
        let mut bus = MessageBus::new();
        let msg = bus.publish("agent-a", "no-subscribers", serde_json::json!("test"));
        assert_eq!(msg.topic, "no-subscribers");
    }

    #[test]
    fn inbox_size_limit() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-b", "events");
        // Publish more than max_inbox_size (100)
        for i in 0..105 {
            bus.publish("agent-a", "events", serde_json::json!(i));
        }
        assert_eq!(bus.pending_count("agent-b"), 100);
        let msgs = bus.get_messages("agent-b");
        // Oldest messages should have been trimmed
        assert_eq!(msgs[0].payload, serde_json::json!(5));
    }

    #[test]
    fn multiple_topics_isolated() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-b", "topic1");
        bus.subscribe("agent-c", "topic2");
        bus.publish("agent-a", "topic1", serde_json::json!("for-b"));
        bus.publish("agent-a", "topic2", serde_json::json!("for-c"));

        let msgs_b = bus.get_messages("agent-b");
        assert_eq!(msgs_b.len(), 1);
        assert_eq!(msgs_b[0].payload, serde_json::json!("for-b"));

        let msgs_c = bus.get_messages("agent-c");
        assert_eq!(msgs_c.len(), 1);
        assert_eq!(msgs_c[0].payload, serde_json::json!("for-c"));
    }

    #[test]
    fn message_has_valid_fields() {
        let mut bus = MessageBus::new();
        bus.subscribe("agent-b", "events");
        let msg = bus.publish("agent-a", "events", serde_json::json!({"key": "value"}));

        assert!(!msg.id.is_empty());
        assert_eq!(msg.sender, "agent-a");
        assert_eq!(msg.topic, "events");
        assert!(msg.timestamp > 0);
    }

    #[test]
    fn unsubscribe_from_nonexistent_topic_is_noop() {
        let mut bus = MessageBus::new();
        bus.unsubscribe("agent-a", "nonexistent");
        assert_eq!(bus.subscriptions_for("agent-a").len(), 0);
    }
}
