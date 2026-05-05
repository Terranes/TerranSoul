//! Session reflection persistence.

use serde::{Deserialize, Serialize};

use crate::memory::{EdgeSource, MemoryStore, MemoryType, NewMemory, NewMemoryEdge};

const MAX_TURN_CHARS: usize = 4_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionReflectionReport {
    pub facts_saved: usize,
    pub summary: String,
    pub reflection_id: i64,
    pub source_turn_count: usize,
    pub derived_edge_count: usize,
}

pub fn persist_session_reflection(
    store: &MemoryStore,
    history: &[(String, String)],
    facts: &[String],
    summary: &str,
) -> Result<SessionReflectionReport, String> {
    let facts_saved = save_reflection_facts(store, facts)?;
    let source_turns = filtered_history(history);
    let reflection = store
        .add(NewMemory {
            content: summary.trim().to_string(),
            tags: "session_reflection,session-summary,episodic:reflection".to_string(),
            importance: 4,
            memory_type: MemoryType::Summary,
            ..Default::default()
        })
        .map_err(|error| error.to_string())?;

    let mut derived_edge_count = 0;
    for (index, (role, content)) in source_turns.iter().enumerate() {
        let turn_entry = store
            .add(NewMemory {
                content: format_session_turn(index + 1, role, content),
                tags: "session_turn,reflection_source,episodic:turn".to_string(),
                importance: 2,
                memory_type: MemoryType::Context,
                ..Default::default()
            })
            .map_err(|error| error.to_string())?;
        if store
            .add_edge(NewMemoryEdge {
                src_id: reflection.id,
                dst_id: turn_entry.id,
                rel_type: "derived_from".to_string(),
                confidence: 1.0,
                source: EdgeSource::Auto,
                valid_from: None,
                valid_to: None,
                edge_source: Some("reflect_on_session".to_string()),
            })
            .is_ok()
        {
            derived_edge_count += 1;
        }
    }

    Ok(SessionReflectionReport {
        facts_saved,
        summary: summary.trim().to_string(),
        reflection_id: reflection.id,
        source_turn_count: source_turns.len(),
        derived_edge_count,
    })
}

fn save_reflection_facts(store: &MemoryStore, facts: &[String]) -> Result<usize, String> {
    let mut saved = 0;
    for fact in facts
        .iter()
        .map(|fact| fact.trim())
        .filter(|fact| fact.len() >= 5)
    {
        store
            .add(NewMemory {
                content: fact.to_string(),
                tags: "auto-extracted,session_reflection_fact".to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .map_err(|error| error.to_string())?;
        saved += 1;
    }
    Ok(saved)
}

fn filtered_history(history: &[(String, String)]) -> Vec<(String, String)> {
    history
        .iter()
        .filter_map(|(role, content)| {
            let trimmed = content.trim();
            if trimmed.is_empty() || trimmed == "/reflect" {
                None
            } else {
                Some((role.trim().to_string(), trimmed.to_string()))
            }
        })
        .collect()
}

fn format_session_turn(index: usize, role: &str, content: &str) -> String {
    format!(
        "Session turn {index} ({role}): {}",
        truncate_chars(content, MAX_TURN_CHARS)
    )
}

fn truncate_chars(content: &str, max_chars: usize) -> String {
    let mut iter = content.chars();
    let truncated: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> MemoryStore {
        MemoryStore::in_memory()
    }

    #[test]
    fn reflection_persists_summary_facts_turns_and_edges() {
        let store = test_store();
        let history = vec![
            (
                "user".to_string(),
                "I need help with Rust tests".to_string(),
            ),
            (
                "assistant".to_string(),
                "We should run cargo test".to_string(),
            ),
        ];
        let facts = vec!["User is working on Rust tests".to_string()];

        let report = persist_session_reflection(
            &store,
            &history,
            &facts,
            "The session focused on Rust test workflow.",
        )
        .unwrap();

        assert_eq!(report.facts_saved, 1);
        assert_eq!(report.source_turn_count, 2);
        assert_eq!(report.derived_edge_count, 2);

        let all = store.get_all().unwrap();
        assert_eq!(all.len(), 4);
        assert!(all
            .iter()
            .any(|entry| entry.tags.contains("session_reflection")));
        assert!(all.iter().any(|entry| entry.tags.contains("session_turn")));
    }

    #[test]
    fn reflection_filters_empty_and_command_turns() {
        let store = test_store();
        let history = vec![
            ("user".to_string(), "".to_string()),
            ("user".to_string(), "/reflect".to_string()),
            ("assistant".to_string(), "A useful answer".to_string()),
        ];

        let report =
            persist_session_reflection(&store, &history, &[], "One answer was given.").unwrap();

        assert_eq!(report.source_turn_count, 1);
        assert_eq!(report.derived_edge_count, 1);
    }
}
