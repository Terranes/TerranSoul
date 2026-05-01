//! Code-RAG fusion — Tier 2 of the GitNexus integration (Chunk 2.2).
//!
//! Normalises a JSON response from the GitNexus MCP `query` tool into a
//! list of pseudo-[`MemoryEntry`] records that the existing
//! [`crate::memory::fusion::reciprocal_rank_fuse`] pipeline can consume.
//! These pseudo-entries:
//!
//! * Carry **negative ids** so they can never collide with real SQLite
//!   primary keys (which are always positive). Downstream code that needs
//!   to write back to the DB can detect them with `id < 0` and skip.
//! * Live in the [`MemoryTier::Working`] tier — they are ephemeral context
//!   pulled in for one query and never persisted.
//! * Are typed as [`MemoryType::Context`] so the rest of the system treats
//!   them as transient retrieval context, not personal facts or
//!   preferences.
//! * Carry a `code:gitnexus` tag and optionally a `code:<source-path>`
//!   tag so the BrainView "Source" column (Chunk 2.4) can attribute them.
//!
//! The normaliser is **lenient** by design — GitNexus's response schema
//! is not stable across versions. We accept any of these shapes:
//!
//! ```text
//! { "snippets": [ { "content": "...", "path": "..." }, ... ] }
//! { "answer": "...", "sources": [ { "content": "...", "path": "..." } ] }
//! { "results": [ { "content": "...", "path": "..." } ] }
//! [ { "content": "...", "path": "..." }, ... ]   // top-level array
//! { "answer": "...sentence..." }                 // single answer only
//! ```
//!
//! Anything we don't recognise is silently dropped — the search pipeline
//! must always degrade gracefully.

use crate::memory::{MemoryEntry, MemoryTier, MemoryType};
use serde_json::Value;

/// Marker tag every code-intelligence pseudo-entry carries. Used by
/// downstream filters to identify entries that came from GitNexus rather
/// than from the local SQLite store.
pub const CODE_RAG_TAG: &str = "code:gitnexus";

/// Maximum number of entries we will surface from a single GitNexus
/// response. Defensive bound — runaway responses must not flood the
/// rerank stage or blow up LLM-as-judge token usage.
pub const MAX_CODE_RAG_ENTRIES: usize = 16;

/// Convert a GitNexus MCP response into a list of pseudo-`MemoryEntry`
/// records suitable for RRF fusion with SQLite hits.
///
/// `base_id_offset` is the most-negative id the caller has already used
/// for ephemeral entries in this query — typically `-1` for the first
/// call. The function returns entries with ids
/// `base_id_offset, base_id_offset - 1, base_id_offset - 2, ...` so the
/// caller can always know the next free id from
/// `base_id_offset - returned.len() as i64`.
///
/// Returns an empty `Vec` for any unrecognised shape — never errors. The
/// search pipeline must continue working even if GitNexus speaks a
/// future schema we don't understand.
pub fn gitnexus_response_to_entries(value: &Value, base_id_offset: i64) -> Vec<MemoryEntry> {
    debug_assert!(
        base_id_offset < 0,
        "base_id_offset must be negative so pseudo-ids never collide with real primary keys"
    );

    let mut snippets = extract_snippets(value);
    if snippets.len() > MAX_CODE_RAG_ENTRIES {
        snippets.truncate(MAX_CODE_RAG_ENTRIES);
    }

    let now = current_unix_ms();
    snippets
        .into_iter()
        .enumerate()
        .map(|(idx, snip)| build_entry(snip, base_id_offset - idx as i64, now))
        .collect()
}

/// Returns true iff the entry is a GitNexus-derived pseudo-entry — used by
/// callers that want to skip non-persistent entries (e.g. write-back
/// loops, edge extractors).
pub fn is_code_rag_entry(entry: &MemoryEntry) -> bool {
    entry.id < 0 && entry.tags.split(',').any(|t| t.trim() == CODE_RAG_TAG)
}

// ---------------------------------------------------------------------------
// Internal: shape-tolerant snippet extraction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Snippet {
    content: String,
    path: Option<String>,
}

fn extract_snippets(value: &Value) -> Vec<Snippet> {
    // 1. Top-level array of snippets / strings.
    if let Some(arr) = value.as_array() {
        return arr.iter().filter_map(value_to_snippet).collect();
    }
    // 2. Object with one of the known list keys.
    if let Some(obj) = value.as_object() {
        for key in ["snippets", "sources", "results", "items", "matches"] {
            if let Some(arr) = obj.get(key).and_then(Value::as_array) {
                let mut out: Vec<Snippet> = arr.iter().filter_map(value_to_snippet).collect();
                // Some shapes carry an additional top-level `answer` —
                // surface it as the lead snippet so the LLM sees the
                // synthesised answer first.
                if let Some(answer) = obj.get("answer").and_then(Value::as_str) {
                    if !answer.trim().is_empty() {
                        out.insert(
                            0,
                            Snippet {
                                content: answer.to_string(),
                                path: None,
                            },
                        );
                    }
                }
                return out;
            }
        }
        // 3. Lone "answer" with no list — return it as a single snippet.
        if let Some(answer) = obj.get("answer").and_then(Value::as_str) {
            if !answer.trim().is_empty() {
                return vec![Snippet {
                    content: answer.to_string(),
                    path: None,
                }];
            }
        }
    }
    // 4. Plain string.
    if let Some(s) = value.as_str() {
        if !s.trim().is_empty() {
            return vec![Snippet {
                content: s.to_string(),
                path: None,
            }];
        }
    }
    Vec::new()
}

fn value_to_snippet(v: &Value) -> Option<Snippet> {
    // Plain string array element.
    if let Some(s) = v.as_str() {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }
        return Some(Snippet {
            content: trimmed.to_string(),
            path: None,
        });
    }
    // Object: prefer `content` / `text` / `snippet` / `body` for the
    // body, and `path` / `file` / `location` / `uri` for the source.
    let obj = v.as_object()?;
    let content = ["content", "text", "snippet", "body", "code"]
        .iter()
        .find_map(|k| obj.get(*k).and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    let path = ["path", "file", "location", "uri", "source"]
        .iter()
        .find_map(|k| obj.get(*k).and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Some(Snippet { content, path })
}

fn build_entry(snippet: Snippet, id: i64, now_ms: i64) -> MemoryEntry {
    let tags = match &snippet.path {
        Some(p) => format!("{CODE_RAG_TAG},code:{}", sanitize_tag(p)),
        None => CODE_RAG_TAG.to_string(),
    };
    let token_count = (snippet.content.len() / 4).max(1) as i64;
    MemoryEntry {
        id,
        content: snippet.content,
        tags,
        importance: 3,
        memory_type: MemoryType::Context,
        created_at: now_ms,
        last_accessed: None,
        access_count: 0,
        embedding: None,
        tier: MemoryTier::Working,
        decay_score: 1.0,
        session_id: None,
        parent_id: None,
        token_count,
        source_url: snippet.path.clone(),
        source_hash: None,
        expires_at: None,
        valid_to: None,
        obsidian_path: None,
        last_exported: None,
    }
}

fn sanitize_tag(raw: &str) -> String {
    // Tags are stored comma-separated, so commas in a path would break
    // round-trips. Replace them and a few other comma-like separators
    // with `_` so the tag remains greppable.
    raw.replace([',', ';', '\n', '\r', '\t'], "_")
}

fn current_unix_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_value_yields_no_entries() {
        assert!(gitnexus_response_to_entries(&Value::Null, -1).is_empty());
        assert!(gitnexus_response_to_entries(&json!({}), -1).is_empty());
        assert!(gitnexus_response_to_entries(&json!([]), -1).is_empty());
        assert!(gitnexus_response_to_entries(&json!(""), -1).is_empty());
        assert!(gitnexus_response_to_entries(&json!("   "), -1).is_empty());
    }

    #[test]
    fn snippets_shape_is_normalised() {
        let v = json!({
            "snippets": [
                { "content": "fn parse() {}", "path": "src/parser.rs" },
                { "content": "fn serialise() {}", "path": "src/serialiser.rs" },
            ]
        });
        let entries = gitnexus_response_to_entries(&v, -1);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, -1);
        assert_eq!(entries[1].id, -2);
        assert_eq!(entries[0].content, "fn parse() {}");
        assert_eq!(entries[0].source_url.as_deref(), Some("src/parser.rs"));
        assert!(entries[0].tags.contains("code:gitnexus"));
        assert!(entries[0].tags.contains("code:src/parser.rs"));
        assert!(entries.iter().all(is_code_rag_entry));
    }

    #[test]
    fn answer_plus_sources_puts_answer_first() {
        let v = json!({
            "answer": "parse() lives in src/parser.rs:42",
            "sources": [
                { "text": "fn parse() {}", "file": "src/parser.rs" },
            ]
        });
        let entries = gitnexus_response_to_entries(&v, -1);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content, "parse() lives in src/parser.rs:42");
        assert_eq!(entries[0].source_url, None);
        assert_eq!(entries[1].content, "fn parse() {}");
        assert_eq!(entries[1].source_url.as_deref(), Some("src/parser.rs"));
    }

    #[test]
    fn results_shape_is_supported() {
        let v = json!({
            "results": [
                { "snippet": "let x = 1;", "location": "src/lib.rs:10" },
                { "body": "let y = 2;", "uri": "src/lib.rs:11" },
            ]
        });
        let entries = gitnexus_response_to_entries(&v, -1);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content, "let x = 1;");
        assert_eq!(entries[1].source_url.as_deref(), Some("src/lib.rs:11"));
    }

    #[test]
    fn top_level_array_is_supported() {
        let v = json!([
            { "content": "alpha", "path": "a.rs" },
            "raw string snippet",
            { "content": "beta" },
        ]);
        let entries = gitnexus_response_to_entries(&v, -1);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].content, "alpha");
        assert_eq!(entries[1].content, "raw string snippet");
        assert_eq!(entries[2].content, "beta");
        assert_eq!(entries[2].source_url, None);
    }

    #[test]
    fn lone_answer_becomes_single_entry() {
        let v = json!({ "answer": "yes — see line 17" });
        let entries = gitnexus_response_to_entries(&v, -1);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "yes — see line 17");
    }

    #[test]
    fn plain_string_value_becomes_single_entry() {
        let v = json!("hello world");
        let entries = gitnexus_response_to_entries(&v, -1);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "hello world");
    }

    #[test]
    fn ids_are_strictly_decreasing_starting_from_offset() {
        let v = json!({
            "snippets": (0..5).map(|i| json!({ "content": format!("s{i}") })).collect::<Vec<_>>()
        });
        let entries = gitnexus_response_to_entries(&v, -10);
        assert_eq!(entries.len(), 5);
        let ids: Vec<i64> = entries.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![-10, -11, -12, -13, -14]);
    }

    #[test]
    fn entries_are_capped_at_max() {
        let v = json!({
            "snippets": (0..(MAX_CODE_RAG_ENTRIES * 3))
                .map(|i| json!({ "content": format!("s{i}") }))
                .collect::<Vec<_>>()
        });
        let entries = gitnexus_response_to_entries(&v, -1);
        assert_eq!(entries.len(), MAX_CODE_RAG_ENTRIES);
    }

    #[test]
    fn whitespace_only_snippets_are_dropped() {
        let v = json!({
            "snippets": [
                { "content": "   " },
                { "content": "real" },
                { "content": "" },
            ]
        });
        let entries = gitnexus_response_to_entries(&v, -1);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "real");
    }

    #[test]
    fn entries_are_ephemeral_working_tier_context() {
        let v = json!({ "snippets": [{ "content": "x", "path": "a.rs" }] });
        let entry = &gitnexus_response_to_entries(&v, -1)[0];
        assert!(entry.id < 0);
        assert_eq!(entry.tier, MemoryTier::Working);
        assert_eq!(entry.memory_type, MemoryType::Context);
        assert_eq!(entry.decay_score, 1.0);
        assert!(entry.embedding.is_none());
        assert!(entry.session_id.is_none());
        assert!(entry.parent_id.is_none());
        assert!(entry.expires_at.is_none());
    }

    #[test]
    fn comma_in_path_does_not_break_tag_round_trip() {
        let v = json!({
            "snippets": [{ "content": "x", "path": "weird,name.rs" }]
        });
        let entry = &gitnexus_response_to_entries(&v, -1)[0];
        // Tag list should still be parseable as comma-separated.
        let tags: Vec<&str> = entry.tags.split(',').map(str::trim).collect();
        assert!(tags.contains(&"code:gitnexus"));
        assert!(tags.iter().any(|t| t.starts_with("code:weird_name.rs")));
    }

    #[test]
    fn is_code_rag_entry_matches_only_code_rag_pseudo_entries() {
        let v = json!({ "snippets": [{ "content": "x" }] });
        let pseudo = &gitnexus_response_to_entries(&v, -1)[0];
        assert!(is_code_rag_entry(pseudo));

        // Real entry: positive id, no code:gitnexus tag.
        let real = MemoryEntry {
            id: 42,
            content: "real".into(),
            tags: "personal".into(),
            importance: 3,
            memory_type: MemoryType::Fact,
            created_at: 0,
            last_accessed: None,
            access_count: 0,
            embedding: None,
            tier: MemoryTier::Long,
            decay_score: 1.0,
            session_id: None,
            parent_id: None,
            token_count: 1,
            source_url: None,
            source_hash: None,
            expires_at: None,
            valid_to: None,
        obsidian_path: None,
        last_exported: None,
        };
        assert!(!is_code_rag_entry(&real));

        // Negative id but no code tag — must still return false.
        let mut neg = real.clone();
        neg.id = -5;
        assert!(!is_code_rag_entry(&neg));
    }

    #[test]
    fn unknown_shape_yields_empty_vec() {
        let v = json!({ "totally": "unrelated", "fields": 42 });
        assert!(gitnexus_response_to_entries(&v, -1).is_empty());
    }
}
