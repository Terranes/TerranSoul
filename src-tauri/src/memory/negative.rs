//! Negative-memory trigger scanning (Chunk 43.7).
//!
//! Scans incoming context (user message, file content, diff) against active
//! trigger patterns stored in `memory_trigger_patterns`. Matching negative
//! memories are prepended as `[NEGATIVE — DO NOT DO THIS]` blocks.

use rusqlite::{params, Result as SqlResult};

use super::store::MemoryStore;

/// A negative memory with its trigger pattern that matched.
#[derive(Debug, Clone)]
pub struct NegativeMatch {
    pub memory_id: i64,
    pub content: String,
    pub pattern: String,
}

/// Scan `context` against all trigger patterns attached to negative memories.
///
/// Returns matching negative memories that should be injected as warnings.
pub fn scan_triggers(store: &MemoryStore, context: &str) -> SqlResult<Vec<NegativeMatch>> {
    let lower = context.to_lowercase();

    let mut stmt = store.conn.prepare_cached(
        "SELECT tp.memory_id, m.content, tp.pattern
         FROM memory_trigger_patterns tp
         JOIN memories m ON m.id = tp.memory_id
         WHERE m.valid_to IS NULL
           AND tp.kind = 'substring'
         ORDER BY tp.memory_id",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut matches = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    for r in rows.flatten() {
        let (memory_id, content, pattern) = r;
        if seen_ids.contains(&memory_id) {
            continue;
        }
        if lower.contains(&pattern.to_lowercase()) {
            seen_ids.insert(memory_id);
            matches.push(NegativeMatch {
                memory_id,
                content,
                pattern,
            });
        }
    }

    Ok(matches)
}

/// Format negative matches as a prompt injection block.
pub fn format_negative_block(matches: &[NegativeMatch]) -> String {
    if matches.is_empty() {
        return String::new();
    }

    let mut block = String::from("[NEGATIVE — DO NOT DO THIS]\n");
    for m in matches {
        block.push_str("- ");
        block.push_str(&m.content);
        block.push('\n');
    }
    block
}

/// Register a trigger pattern for a memory. Skips if an identical
/// (memory_id, pattern, kind) triple already exists.
pub fn add_trigger(
    store: &MemoryStore,
    memory_id: i64,
    pattern: &str,
    kind: &str,
) -> SqlResult<()> {
    let exists: bool = store.conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM memory_trigger_patterns
            WHERE memory_id = ?1 AND pattern = ?2 AND kind = ?3
        )",
        params![memory_id, pattern, kind],
        |row| row.get(0),
    )?;
    if !exists {
        store.conn.execute(
            "INSERT INTO memory_trigger_patterns (memory_id, pattern, kind)
             VALUES (?1, ?2, ?3)",
            params![memory_id, pattern, kind],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::store::{MemoryStore, MemoryType, NewMemory};

    fn setup() -> MemoryStore {
        let store = MemoryStore::in_memory();

        // Create a negative memory
        let entry = store
            .add(NewMemory {
                content: "Never use .unwrap() in library code".to_string(),
                tags: "negative,coding".to_string(),
                importance: 5,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        // Set cognitive_kind to negative in DB
        store
            .conn
            .execute(
                "UPDATE memories SET cognitive_kind = 'negative' WHERE id = ?1",
                params![entry.id],
            )
            .unwrap();

        // Register a trigger pattern
        add_trigger(&store, entry.id, ".unwrap()", "substring").unwrap();

        // Create another negative
        let entry2 = store
            .add(NewMemory {
                content: "Do not hardcode hex colors, use CSS tokens".to_string(),
                tags: "negative,css".to_string(),
                importance: 4,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();

        add_trigger(&store, entry2.id, "hardcode hex", "substring").unwrap();
        add_trigger(&store, entry2.id, "#[0-9a-f]{6}", "substring").unwrap();

        store
    }

    #[test]
    fn scan_finds_matching_trigger() {
        let store = setup();
        let matches = scan_triggers(&store, "We should use .unwrap() here").unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].content.contains("unwrap"));
    }

    #[test]
    fn scan_case_insensitive() {
        let store = setup();
        let matches = scan_triggers(&store, "We should use .UNWRAP() here").unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn scan_no_match_returns_empty() {
        let store = setup();
        let matches = scan_triggers(&store, "This code looks fine").unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn scan_deduplicates_by_memory_id() {
        let store = setup();
        // Both patterns for memory 2 could match, but we should only get one result
        let matches = scan_triggers(&store, "hardcode hex colors like #ff0000").unwrap();
        // The "hardcode hex" pattern matches; depending on overlap, should get 1 result for that memory
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn format_block_empty_for_no_matches() {
        let block = format_negative_block(&[]);
        assert!(block.is_empty());
    }

    #[test]
    fn format_block_includes_header_and_content() {
        let matches = vec![NegativeMatch {
            memory_id: 1,
            content: "Don't use goto".to_string(),
            pattern: "goto".to_string(),
        }];
        let block = format_negative_block(&matches);
        assert!(block.starts_with("[NEGATIVE"));
        assert!(block.contains("Don't use goto"));
    }

    #[test]
    fn add_trigger_is_idempotent() {
        let store = MemoryStore::in_memory();
        let entry = store
            .add(NewMemory {
                content: "test".to_string(),
                ..Default::default()
            })
            .unwrap();

        add_trigger(&store, entry.id, "test_pattern", "substring").unwrap();
        add_trigger(&store, entry.id, "test_pattern", "substring").unwrap();

        let count: i64 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM memory_trigger_patterns WHERE memory_id = ?1",
                params![entry.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "duplicate trigger should not be inserted");
    }
}
