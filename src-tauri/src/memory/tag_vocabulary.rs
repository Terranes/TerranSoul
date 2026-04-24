//! Curated tag-prefix vocabulary for memory entries (Chunk 18.4).
//!
//! TerranSoul memories use free-form `tags`, but the *intended* convention
//! is a `<prefix>:<value>` shape so that retrieval and filtering can be
//! prefix-aware (the front-end "Memory View" filter, the upcoming
//! category-aware decay rates, and the persona extractor's
//! `personal:*` lookups all rely on it).
//!
//! Historical free-form tags ingested before this convention landed are
//! kept working via [`LEGACY_ALLOW_LIST`]; nothing here *rejects* a
//! tag — [`validate`] returns a [`TagValidation`] verdict that callers
//! (BrainView, ingest pipeline) can surface as a soft "review tag"
//! warning without breaking the write path.
//!
//! Maps to `docs/brain-advanced-design.md` §16 Phase 2 row "Tag-prefix
//! convention enforcement" (chunk 18.4).

/// The curated set of tag prefixes. Adding a prefix here is a small
/// design decision — please prefer reusing an existing prefix over
/// inventing a new one.
///
/// | Prefix      | Intent                                                |
/// |-------------|-------------------------------------------------------|
/// | `personal`  | Things about the user (name, preferences, goals).     |
/// | `domain`    | Subject-matter knowledge (law, medicine, programming).|
/// | `project`   | Per-project context (active codebases, deliverables). |
/// | `tool`      | Tool-specific facts (CLI flags, API keys-by-name).    |
/// | `code`      | Code snippets, design patterns, architecture notes.   |
/// | `external`  | External resources (URLs, cited sources, citations).  |
/// | `session`   | Conversation-scoped scratch facts (auto-evicted).     |
/// | `quest`     | Skill-tree quest progress + unlocks.                  |
pub const CURATED_PREFIXES: &[&str] = &[
    "personal", "domain", "project", "tool", "code", "external", "session", "quest",
];

/// Free-form tags that pre-date the prefix convention but are still
/// considered valid. Keep this list short — every entry here is a
/// debt item to be migrated to a `<prefix>:<value>` shape.
pub const LEGACY_ALLOW_LIST: &[&str] = &[
    // Phase 0–4 seed tags
    "user", "assistant", "system",
    // Common short tags from the early ingest fixtures
    "fact", "preference", "todo", "summary",
];

/// Verdict for a single tag. Always informational — never rejects writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagValidation {
    /// Tag follows the curated `<prefix>:<value>` convention.
    Curated { prefix: &'static str },
    /// Tag is in the legacy allow-list (pre-prefix convention).
    Legacy,
    /// Tag is non-conforming. Surfaces in BrainView as a soft warning.
    NonConforming { reason: NonConformingReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NonConformingReason {
    /// Has a `:` separator but the prefix isn't in [`CURATED_PREFIXES`].
    UnknownPrefix(String),
    /// No `:` separator and not in [`LEGACY_ALLOW_LIST`].
    MissingPrefix,
    /// Empty value after `prefix:` (e.g. `"personal:"`).
    EmptyValue { prefix: String },
    /// Empty / whitespace-only tag.
    Empty,
}

impl TagValidation {
    /// True for `Curated` and `Legacy`.
    pub fn is_acceptable(&self) -> bool {
        matches!(self, TagValidation::Curated { .. } | TagValidation::Legacy)
    }
}

/// Validate a single tag. Pure — no I/O.
///
/// Matching is case-insensitive on the prefix (so `Personal:foo` and
/// `personal:foo` both validate as `Curated { prefix: "personal" }`).
/// Values are not interpreted — `personal:🍕` is acceptable.
pub fn validate(tag: &str) -> TagValidation {
    let trimmed = tag.trim();
    if trimmed.is_empty() {
        return TagValidation::NonConforming { reason: NonConformingReason::Empty };
    }

    // Legacy allow-list (case-insensitive whole-tag match).
    if LEGACY_ALLOW_LIST.iter().any(|legacy| legacy.eq_ignore_ascii_case(trimmed)) {
        return TagValidation::Legacy;
    }

    match trimmed.split_once(':') {
        None => TagValidation::NonConforming { reason: NonConformingReason::MissingPrefix },
        Some((prefix, value)) => {
            let prefix_lc = prefix.trim().to_ascii_lowercase();
            // `&'static str` lookup — return the *canonical* casing from the
            // const slice so downstream callers can pattern-match safely.
            let canonical = CURATED_PREFIXES
                .iter()
                .copied()
                .find(|p| p.eq_ignore_ascii_case(&prefix_lc));
            match canonical {
                None => TagValidation::NonConforming {
                    reason: NonConformingReason::UnknownPrefix(prefix.trim().to_string()),
                },
                Some(p) => {
                    if value.trim().is_empty() {
                        TagValidation::NonConforming {
                            reason: NonConformingReason::EmptyValue { prefix: p.to_string() },
                        }
                    } else {
                        TagValidation::Curated { prefix: p }
                    }
                }
            }
        }
    }
}

/// Validate a comma-separated tag string (matches the on-disk shape stored
/// in `MemoryEntry.tags`). Returns one verdict per tag, in input order.
/// Empty entries from a trailing comma are dropped.
pub fn validate_csv(tags_csv: &str) -> Vec<TagValidation> {
    tags_csv
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(validate)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_prefixes_validate() {
        assert_eq!(
            validate("personal:loves_pho"),
            TagValidation::Curated { prefix: "personal" }
        );
        assert_eq!(
            validate("project:terransoul"),
            TagValidation::Curated { prefix: "project" }
        );
        assert_eq!(
            validate("code:rust"),
            TagValidation::Curated { prefix: "code" }
        );
    }

    #[test]
    fn case_insensitive_prefix_match() {
        assert_eq!(
            validate("Personal:Foo"),
            TagValidation::Curated { prefix: "personal" }
        );
        assert_eq!(
            validate("DOMAIN:law"),
            TagValidation::Curated { prefix: "domain" }
        );
    }

    #[test]
    fn legacy_allow_list_passes() {
        assert_eq!(validate("fact"), TagValidation::Legacy);
        assert_eq!(validate("Preference"), TagValidation::Legacy); // case-insensitive
    }

    #[test]
    fn unknown_prefix_is_non_conforming() {
        match validate("color:blue") {
            TagValidation::NonConforming { reason: NonConformingReason::UnknownPrefix(p) } => {
                assert_eq!(p, "color");
            }
            other => panic!("expected UnknownPrefix, got {other:?}"),
        }
    }

    #[test]
    fn no_separator_and_not_in_allow_list_is_non_conforming() {
        assert!(matches!(
            validate("randomtag"),
            TagValidation::NonConforming { reason: NonConformingReason::MissingPrefix }
        ));
    }

    #[test]
    fn empty_value_is_non_conforming() {
        match validate("personal:") {
            TagValidation::NonConforming { reason: NonConformingReason::EmptyValue { prefix } } => {
                assert_eq!(prefix, "personal");
            }
            other => panic!("expected EmptyValue, got {other:?}"),
        }
        // Whitespace value also empty.
        assert!(matches!(
            validate("personal:   "),
            TagValidation::NonConforming { reason: NonConformingReason::EmptyValue { .. } }
        ));
    }

    #[test]
    fn empty_or_whitespace_tag_is_non_conforming() {
        assert_eq!(validate(""), TagValidation::NonConforming { reason: NonConformingReason::Empty });
        assert_eq!(validate("   "), TagValidation::NonConforming { reason: NonConformingReason::Empty });
    }

    #[test]
    fn validate_csv_parses_each_tag_in_order() {
        let v = validate_csv("personal:foo, fact, color:blue,, project:bar");
        assert_eq!(v.len(), 4); // empty entry between commas is dropped
        assert_eq!(v[0], TagValidation::Curated { prefix: "personal" });
        assert_eq!(v[1], TagValidation::Legacy);
        assert!(matches!(v[2], TagValidation::NonConforming { .. }));
        assert_eq!(v[3], TagValidation::Curated { prefix: "project" });
    }

    #[test]
    fn is_acceptable_only_curated_or_legacy() {
        assert!(TagValidation::Curated { prefix: "personal" }.is_acceptable());
        assert!(TagValidation::Legacy.is_acceptable());
        assert!(!TagValidation::NonConforming {
            reason: NonConformingReason::Empty
        }
        .is_acceptable());
    }

    #[test]
    fn value_can_contain_colons_and_unicode() {
        // `split_once(':')` only splits on the first colon, so values with
        // additional colons (URLs, namespaced ids) pass through untouched.
        assert_eq!(
            validate("external:https://thuvienphapluat.vn/..."),
            TagValidation::Curated { prefix: "external" }
        );
        assert_eq!(
            validate("personal:🍕"),
            TagValidation::Curated { prefix: "personal" }
        );
    }
}
