//! LLM-driven memory refinement (write-back deduplication).
//!
//! When the chat-driven extractor produces a new candidate fact, the
//! brain previously called `MemoryStore::add` unconditionally. That made
//! long sessions accumulate near-duplicate or stale entries — the
//! semantic-RAG pool grew without ever being *adjusted*.
//!
//! This module turns every "add fact" call into a small refinement
//! decision, executed by the active brain:
//!
//! 1. Pull keyword-similar candidates from the store
//!    ([`MemoryStore::hybrid_search`] — keyword-only, no embedding
//!    work needed at write time).
//! 2. Ask the LLM whether the new fact is `KEEP` (already covered),
//!    `UPDATE { id, content }` (rewrite an existing entry to incorporate
//!    the new info), or `NEW` (genuinely novel).
//! 3. Apply the decision via [`MemoryStore::update`] (which also writes
//!    a non-destructive version snapshot — see `versioning.rs`) or
//!    [`MemoryStore::add`].
//!
//! The whole pipeline degrades gracefully:
//!   - No brain configured / LLM unreachable → fall back to pure insert.
//!   - No keyword candidates above the floor → insert as `NEW`.
//!   - Malformed LLM reply → insert as `NEW` so we never lose data.
//!
//! Pure prompt + parse helpers are kept separate from the I/O so they
//! are exhaustively unit-testable without a running brain or DB.

use serde::{Deserialize, Serialize};

use crate::brain::{BrainMode, ProviderRotator};
use crate::memory::{MemoryEntry, MemoryStore, MemoryType, MemoryUpdate, NewMemory};

/// Maximum number of keyword-similar candidates we ask the LLM to consider
/// per new fact. Keeping this small (default 3) bounds prompt length and
/// keeps each refine call fast.
pub const DEFAULT_REFINE_CANDIDATES: usize = 3;

/// What [`refine_and_save_fact`] decided to do for a single fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefineOutcome {
    /// New entry inserted via [`MemoryStore::add`].
    Inserted { id: i64 },
    /// Existing entry rewritten via [`MemoryStore::update`].
    Updated { id: i64 },
    /// Existing knowledge already covers the fact — nothing changed.
    Kept { id: i64 },
    /// Brain unreachable / no candidates / malformed reply — caller
    /// should treat the fact as new and insert it. Returned with the
    /// id of the inserted entry when the caller has already done so.
    FallbackInserted { id: i64 },
    /// Filtered before reaching the brain (too short, empty after
    /// trim, etc.).
    SkippedShort,
}

impl RefineOutcome {
    pub fn is_write(self) -> bool {
        matches!(
            self,
            RefineOutcome::Inserted { .. }
                | RefineOutcome::Updated { .. }
                | RefineOutcome::FallbackInserted { .. }
        )
    }
}

/// Aggregate counters returned by [`save_facts_refined`]. Useful for
/// surfacing "we kept N, updated M, added K" telemetry to the UI.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefineStats {
    pub inserted: usize,
    pub updated: usize,
    pub kept: usize,
    pub fallback_inserted: usize,
    pub skipped: usize,
}

impl RefineStats {
    pub fn total_writes(&self) -> usize {
        self.inserted + self.updated + self.fallback_inserted
    }

    fn record(&mut self, outcome: RefineOutcome) {
        match outcome {
            RefineOutcome::Inserted { .. } => self.inserted += 1,
            RefineOutcome::Updated { .. } => self.updated += 1,
            RefineOutcome::Kept { .. } => self.kept += 1,
            RefineOutcome::FallbackInserted { .. } => self.fallback_inserted += 1,
            RefineOutcome::SkippedShort => self.skipped += 1,
        }
    }
}

/// Parsed LLM refine reply (pure data, no I/O). The brain returns one
/// of these three shapes for each new candidate fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum RefineDecision {
    /// Existing entry already covers the fact — keep both as-is.
    Keep { id: i64 },
    /// Rewrite the existing entry with merged content. The new content
    /// should be a single concise statement that incorporates the old
    /// entry plus the new fact (no duplication, no stale claims).
    Update { id: i64, content: String },
    /// Insert as a brand-new entry — none of the candidates is close
    /// enough to merge.
    New,
}

/// Build the (system, user) prompt pair that asks the brain to choose
/// between Keep / Update / New for a single candidate fact.
///
/// `candidates` is the short-list pulled from the store; each tuple is
/// `(id, content)`. `new_fact` is the freshly-extracted fact under
/// consideration.
pub fn build_refine_prompt(new_fact: &str, candidates: &[(i64, String)]) -> (String, String) {
    let system = "You are a knowledge-base curator for a personal AI companion. \
Your job is to keep the knowledge base concise: prefer updating an existing entry \
over adding a near-duplicate. Reply with ONLY a JSON object — no prose, no markdown fences."
        .to_string();

    let mut candidates_block = String::new();
    for (id, content) in candidates {
        candidates_block.push_str(&format!("- id={id}: {}\n", content.trim()));
    }
    if candidates_block.is_empty() {
        candidates_block.push_str("(none)\n");
    }

    let user = format!(
        "EXISTING ENTRIES (keyword-similar to the new fact):\n{candidates_block}\n\
        NEW CANDIDATE FACT:\n{}\n\n\
        Decide which action best preserves the knowledge base without duplication.\n\n\
        OUTPUT FORMAT — reply with exactly one JSON object:\n\
        {{\n\
        \x20 \"action\": \"keep\" | \"update\" | \"new\",\n\
        \x20 \"id\": <existing entry id>,            // required for keep/update, omit for new\n\
        \x20 \"content\": \"<rewritten entry>\"        // required for update only\n\
        }}\n\n\
        Rules:\n\
        - Use \"keep\" when the new fact adds nothing — an existing entry already says it.\n\
        - Use \"update\" when an existing entry partially covers the fact — rewrite it as a \
          single concise statement that incorporates BOTH the old entry AND the new fact, \
          dropping anything the new fact contradicts. The rewrite must stand alone.\n\
        - Use \"new\" only when none of the existing entries is close enough to merge.\n\
        - Never invent ids that aren't in the list above.\n\
        - Reply with ONLY the JSON object.",
        new_fact.trim(),
    );

    (system, user)
}

/// Strip common Markdown fences from a raw LLM reply.
fn strip_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let rest = rest.strip_prefix("json").unwrap_or(rest);
        let rest = rest.trim_start_matches('\n');
        if let Some(body) = rest.strip_suffix("```") {
            return body.trim().to_string();
        }
    }
    trimmed.to_string()
}

/// Parse a brain reply into a [`RefineDecision`].
///
/// Tolerant of fences, leading prose, missing fields, and ids the LLM
/// invented — `valid_ids` is consulted to reject any decision that
/// references an entry not in the candidate set.
///
/// Returns `None` when the reply cannot be interpreted; callers should
/// treat this as "insert as new" so a flaky LLM never silently drops a
/// fact.
pub fn parse_refine_reply(raw: &str, valid_ids: &[i64]) -> Option<RefineDecision> {
    let body = strip_fences(raw);
    let start = body.find('{')?;
    let end = body.rfind('}')? + 1;
    if start >= end {
        return None;
    }
    let json_str = &body[start..end];
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;

    let action = v.get("action")?.as_str()?.to_ascii_lowercase();
    match action.as_str() {
        "new" => Some(RefineDecision::New),
        "keep" => {
            let id = v.get("id")?.as_i64()?;
            if valid_ids.contains(&id) {
                Some(RefineDecision::Keep { id })
            } else {
                None
            }
        }
        "update" => {
            let id = v.get("id")?.as_i64()?;
            if !valid_ids.contains(&id) {
                return None;
            }
            let content = v.get("content")?.as_str()?.trim().to_string();
            if content.is_empty() {
                return None;
            }
            Some(RefineDecision::Update { id, content })
        }
        _ => None,
    }
}

/// Default minimum length for a fact to be considered for refinement.
/// Mirrors the existing filter in `save_facts` so behaviour stays
/// consistent between the legacy insert-everything path and the new
/// refined path.
pub const MIN_FACT_LEN: usize = 5;

/// Pull a short-list of keyword-similar existing entries for `new_fact`.
/// Returns at most `limit` (id, content) pairs. Pure synchronous DB
/// work — no LLM call. Acquires the store lock briefly and releases it
/// before any `.await` so callers can be `Send`.
fn keyword_candidates(
    store: &std::sync::Mutex<MemoryStore>,
    new_fact: &str,
    limit: usize,
) -> Vec<(i64, String)> {
    let Ok(store) = store.lock() else {
        return Vec::new();
    };
    match store.hybrid_search(new_fact, None, limit.max(1)) {
        Ok(entries) => entries
            .into_iter()
            .map(|e: MemoryEntry| (e.id, e.content))
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Refine and persist a single fact via the active brain.
///
/// The function never panics and returns a [`RefineOutcome`] describing
/// what was actually written. When the brain is unreachable or returns
/// an unparseable reply, the fact is inserted as new (FallbackInserted)
/// so we never silently lose extracted knowledge.
///
/// The store mutex is acquired briefly per phase and never held across
/// the LLM `.await`, so this future stays `Send`.
pub async fn refine_and_save_fact(
    fact: &str,
    brain_mode: &BrainMode,
    rotator: &std::sync::Mutex<ProviderRotator>,
    store: &std::sync::Mutex<MemoryStore>,
    candidate_limit: usize,
) -> RefineOutcome {
    let trimmed = fact.trim();
    if trimmed.len() < MIN_FACT_LEN {
        return RefineOutcome::SkippedShort;
    }

    // Keyword candidates first — cheap, offline, no embedding work.
    let candidates = keyword_candidates(store, trimmed, candidate_limit);

    // No similar entries → straight insert.
    if candidates.is_empty() {
        return insert_fact_locked(trimmed, store)
            .map(|id| RefineOutcome::Inserted { id })
            .unwrap_or(RefineOutcome::SkippedShort);
    }

    // Ask the brain to decide between keep / update / new.
    let (system, user) = build_refine_prompt(trimmed, &candidates);
    let reply =
        super::brain_memory::complete_via_mode(brain_mode, &system, &user, rotator).await;

    let valid_ids: Vec<i64> = candidates.iter().map(|(id, _)| *id).collect();
    let decision = match reply {
        Ok(text) => parse_refine_reply(&text, &valid_ids),
        Err(_) => None,
    };

    match decision {
        Some(RefineDecision::Keep { id }) => RefineOutcome::Kept { id },
        Some(RefineDecision::Update { id, content }) => {
            let updated = match store.lock() {
                Ok(guard) => guard
                    .update(
                        id,
                        MemoryUpdate {
                            content: Some(content),
                            tags: None,
                            importance: None,
                            memory_type: None,
                        },
                    )
                    .is_ok(),
                Err(_) => false,
            };
            if updated {
                RefineOutcome::Updated { id }
            } else {
                // Update failed (row deleted between search and update,
                // or lock poisoned) — fall back to insert rather than
                // losing the fact.
                insert_fact_locked(trimmed, store)
                    .map(|id| RefineOutcome::FallbackInserted { id })
                    .unwrap_or(RefineOutcome::SkippedShort)
            }
        }
        Some(RefineDecision::New) => insert_fact_locked(trimmed, store)
            .map(|id| RefineOutcome::Inserted { id })
            .unwrap_or(RefineOutcome::SkippedShort),
        None => insert_fact_locked(trimmed, store)
            .map(|id| RefineOutcome::FallbackInserted { id })
            .unwrap_or(RefineOutcome::SkippedShort),
    }
}

fn insert_fact_locked(content: &str, store: &std::sync::Mutex<MemoryStore>) -> Option<i64> {
    let guard = store.lock().ok()?;
    guard
        .add(NewMemory {
            content: content.to_string(),
            tags: "auto-extracted".to_string(),
            importance: 3,
            memory_type: MemoryType::Fact,
            ..Default::default()
        })
        .ok()
        .map(|e| e.id)
}

/// Refined replacement for [`super::brain_memory::save_facts`]. Iterates
/// over `facts` and, for each one, asks the brain whether to keep/update
/// existing knowledge or insert as new. Returns aggregate counts.
pub async fn save_facts_refined(
    facts: &[String],
    brain_mode: &BrainMode,
    rotator: &std::sync::Mutex<ProviderRotator>,
    store: &std::sync::Mutex<MemoryStore>,
) -> RefineStats {
    let mut stats = RefineStats::default();
    for fact in facts {
        let outcome =
            refine_and_save_fact(fact, brain_mode, rotator, store, DEFAULT_REFINE_CANDIDATES)
                .await;
        stats.record(outcome);
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refine_prompt_lists_all_candidates_and_new_fact() {
        let candidates = vec![
            (1, "User prefers Python".to_string()),
            (2, "User dislikes Java".to_string()),
        ];
        let (system, user) = build_refine_prompt("User mostly codes in Python 3", &candidates);
        assert!(system.contains("knowledge-base curator"));
        assert!(user.contains("id=1"));
        assert!(user.contains("User prefers Python"));
        assert!(user.contains("id=2"));
        assert!(user.contains("User mostly codes in Python 3"));
        assert!(user.contains("\"action\""));
        assert!(user.contains("keep"));
        assert!(user.contains("update"));
        assert!(user.contains("new"));
    }

    #[test]
    fn refine_prompt_handles_empty_candidates() {
        let (_, user) = build_refine_prompt("anything", &[]);
        assert!(user.contains("(none)"));
    }

    #[test]
    fn parse_keep_decision() {
        let raw = r#"{"action":"keep","id":7}"#;
        let decision = parse_refine_reply(raw, &[7]).unwrap();
        assert_eq!(decision, RefineDecision::Keep { id: 7 });
    }

    #[test]
    fn parse_update_decision() {
        let raw = r#"{"action":"update","id":3,"content":"User codes mainly in Python 3"}"#;
        let decision = parse_refine_reply(raw, &[3, 9]).unwrap();
        assert_eq!(
            decision,
            RefineDecision::Update {
                id: 3,
                content: "User codes mainly in Python 3".to_string()
            }
        );
    }

    #[test]
    fn parse_new_decision() {
        let raw = r#"{"action":"new"}"#;
        let decision = parse_refine_reply(raw, &[]).unwrap();
        assert_eq!(decision, RefineDecision::New);
    }

    #[test]
    fn parse_tolerates_fences_and_prose() {
        let raw = "Here is my answer:\n```json\n{\"action\":\"keep\",\"id\":12}\n```\nThanks!";
        let decision = parse_refine_reply(raw, &[12]).unwrap();
        assert_eq!(decision, RefineDecision::Keep { id: 12 });
    }

    #[test]
    fn parse_rejects_invented_id() {
        // Brain hallucinated id=999 which isn't in the candidate list.
        let raw = r#"{"action":"keep","id":999}"#;
        assert!(parse_refine_reply(raw, &[1, 2, 3]).is_none());
    }

    #[test]
    fn parse_rejects_update_without_content() {
        let raw = r#"{"action":"update","id":3}"#;
        assert!(parse_refine_reply(raw, &[3]).is_none());
    }

    #[test]
    fn parse_rejects_unknown_action() {
        let raw = r#"{"action":"merge","id":1}"#;
        assert!(parse_refine_reply(raw, &[1]).is_none());
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse_refine_reply("not json at all", &[]).is_none());
        assert!(parse_refine_reply("", &[]).is_none());
    }

    #[test]
    fn refine_stats_total_writes_excludes_keep_and_skip() {
        let mut s = RefineStats::default();
        s.record(RefineOutcome::Inserted { id: 1 });
        s.record(RefineOutcome::Updated { id: 2 });
        s.record(RefineOutcome::Kept { id: 3 });
        s.record(RefineOutcome::FallbackInserted { id: 4 });
        s.record(RefineOutcome::SkippedShort);
        assert_eq!(s.inserted, 1);
        assert_eq!(s.updated, 1);
        assert_eq!(s.kept, 1);
        assert_eq!(s.fallback_inserted, 1);
        assert_eq!(s.skipped, 1);
        assert_eq!(s.total_writes(), 3);
    }

    // ── DB-backed integration tests using the in-memory MemoryStore ──

    fn store_with(facts: &[(&str, &str)]) -> std::sync::Mutex<MemoryStore> {
        let s = MemoryStore::in_memory();
        for (content, tags) in facts {
            s.add(NewMemory {
                content: (*content).to_string(),
                tags: (*tags).to_string(),
                importance: 3,
                memory_type: MemoryType::Fact,
                ..Default::default()
            })
            .unwrap();
        }
        std::sync::Mutex::new(s)
    }

    #[test]
    fn keyword_candidates_returns_existing_matches() {
        let store = store_with(&[
            ("User prefers Python language", "lang"),
            ("User dislikes Java", "lang"),
            ("User has a cat named Mochi", "pets"),
        ]);
        let cands = keyword_candidates(&store, "User mostly codes in Python", 5);
        // The Python entry should be in the top results.
        assert!(
            cands.iter().any(|(_, c)| c.contains("Python")),
            "expected a Python-matching candidate, got {cands:?}"
        );
    }

    #[test]
    fn keyword_candidates_empty_when_store_empty() {
        let store = store_with(&[]);
        assert!(keyword_candidates(&store, "anything", 5).is_empty());
    }

    #[test]
    fn insert_fact_locked_persists_entry() {
        let store = store_with(&[]);
        let id = insert_fact_locked("Hello world", &store).expect("insert");
        assert!(id > 0);
        let guard = store.lock().unwrap();
        let entry = guard.get_by_id(id).expect("entry exists");
        assert_eq!(entry.content, "Hello world");
        assert_eq!(entry.tags, "auto-extracted");
    }

    /// Skipping the LLM (no brain reachable) must still insert the fact
    /// when there are no keyword candidates.
    #[tokio::test]
    async fn refine_inserts_when_no_candidates() {
        let store = store_with(&[]);
        // BrainMode::LocalOllama with a fake model — `complete_via_mode`
        // will fail (Ollama not running) but we never reach that path
        // because there are no candidates. Use a free-API config that
        // points nowhere; the no-candidates branch short-circuits before
        // any HTTP call.
        let brain = BrainMode::LocalOllama {
            model: "ts-refine-test".to_string(),
        };
        let rotator = std::sync::Mutex::new(ProviderRotator::default());
        let outcome = refine_and_save_fact(
            "User just adopted a corgi named Pip",
            &brain,
            &rotator,
            &store,
            3,
        )
        .await;
        assert!(matches!(outcome, RefineOutcome::Inserted { .. }));
        let guard = store.lock().unwrap();
        assert_eq!(guard.count(), 1);
    }

    /// When the LLM is unreachable but there ARE candidates, the
    /// refine path must still insert the fact (FallbackInserted) so we
    /// never silently lose extracted knowledge.
    #[tokio::test]
    async fn refine_falls_back_to_insert_when_brain_unreachable() {
        let store = store_with(&[("User prefers Python language", "lang")]);
        let brain = BrainMode::LocalOllama {
            // Bogus model — Ollama call will time out / 404.
            model: "ts-refine-test-unreachable".to_string(),
        };
        let rotator = std::sync::Mutex::new(ProviderRotator::default());
        let outcome = refine_and_save_fact(
            "User mostly codes in Python 3",
            &brain,
            &rotator,
            &store,
            3,
        )
        .await;
        assert!(
            matches!(
                outcome,
                RefineOutcome::FallbackInserted { .. } | RefineOutcome::Inserted { .. }
            ),
            "expected fallback/insert, got {outcome:?}"
        );
        // The original Python entry must still exist — fallback never
        // deletes existing knowledge.
        let guard = store.lock().unwrap();
        assert!(guard.count() >= 1);
    }
}
