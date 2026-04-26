//! Persona pack codec (Chunk 14.7) — share an entire persona setup
//! (active traits + chosen learned-expression / learned-motion artifacts)
//! as a single self-describing JSON document the user can email,
//! version-control, or drop into Soul Link sync.
//!
//! This module is **pure** (no I/O, no lock acquisition, no Tauri
//! state) so the codec can be exhaustively unit-tested without a real
//! filesystem. The thin Tauri wrappers in [`crate::commands::persona`]
//! handle the disk side: `export_persona_pack` reads `persona.json` +
//! `expressions/*.json` + `motions/*.json` and feeds them into
//! [`build_pack`]; `import_persona_pack` takes a user-supplied JSON
//! string, runs it through [`parse_pack`], and writes the artifacts
//! back through the existing atomic-write helpers.
//!
//! ## Failure contract
//!
//! - Malformed JSON → `Err` with a human-readable reason; **nothing**
//!   is applied (no half-written state).
//! - Unknown `kind` for a learned asset → that single asset is skipped
//!   with a warning recorded in the import report; the rest of the
//!   pack still applies (consistent with the "skip corrupt artifacts"
//!   contract in `commands/persona.rs::list_assets`).
//! - Future `pack_version` higher than this binary supports → `Err`
//!   so the user knows to upgrade rather than silently losing fields.
//! - Empty traits / empty libraries are valid (a pack that exports
//!   only the persona traits, or only one motion, is allowed).
//! - Hard cap on pack size (1 MiB) so a hostile clipboard can't OOM
//!   the parser before it sees a single brace.

use serde::{Deserialize, Serialize};

/// Schema version of the export envelope. Bumped on **breaking**
/// changes; additive fields (e.g. a new optional list) do not bump it.
pub const PERSONA_PACK_VERSION: u32 = 1;

/// Hard cap on the input string accepted by [`parse_pack`]. The
/// largest realistic pack is ~250 KB (a few minutes of motion clips at
/// 30 fps with full bone state); 1 MiB leaves comfortable headroom and
/// blocks pathological clipboard payloads.
pub const PERSONA_PACK_MAX_BYTES: usize = 1024 * 1024;

/// One self-describing pack envelope. The frontend treats this as
/// opaque JSON; only this module knows the field layout, and it is
/// what gets serialised to disk / clipboard / Soul Link sync.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersonaPack {
    /// Pack-format version (NOT the persona-traits `version` field —
    /// that lives inside [`PersonaPack::traits`]).
    #[serde(rename = "packVersion")]
    pub pack_version: u32,
    /// ms epoch when the pack was created. Informational only —
    /// never used to override imported `updatedAt` fields.
    #[serde(rename = "exportedAt")]
    pub exported_at: i64,
    /// Free-form one-liner the exporter wrote ("My library setup"…).
    /// Optional; renderable in the import preview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The persona traits JSON, kept as an opaque [`serde_json::Value`]
    /// so future trait fields round-trip even when this binary doesn't
    /// know about them. Validated on import via the frontend
    /// `migratePersonaTraits` shim, the same way disk loads work.
    pub traits: serde_json::Value,
    /// Learned facial-expression presets, opaque per-entry to keep
    /// forward compatibility (the per-entry shape may evolve).
    #[serde(default)]
    pub expressions: Vec<serde_json::Value>,
    /// Learned motion clips, opaque per-entry for the same reason.
    #[serde(default)]
    pub motions: Vec<serde_json::Value>,
}

/// Build a pack from already-parsed disk artifacts. Caller is
/// responsible for reading `persona.json`, `expressions/*.json`, and
/// `motions/*.json`; this function only assembles the envelope.
///
/// `note` is the optional free-form description shown in the import
/// preview. Pass `None` for "untitled".
pub fn build_pack(
    traits: serde_json::Value,
    expressions: Vec<serde_json::Value>,
    motions: Vec<serde_json::Value>,
    note: Option<String>,
    now_ms: i64,
) -> PersonaPack {
    PersonaPack {
        pack_version: PERSONA_PACK_VERSION,
        exported_at: now_ms,
        note: note.and_then(|s| {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }),
        traits,
        expressions,
        motions,
    }
}

/// Serialise a pack to a pretty-printed JSON string suitable for
/// dumping into a clipboard or `.json` file. Returns `Err` only when
/// `serde_json` itself fails (essentially unreachable for the known
/// `PersonaPack` shape).
pub fn pack_to_string(pack: &PersonaPack) -> Result<String, String> {
    serde_json::to_string_pretty(pack)
        .map_err(|e| format!("Failed to serialise persona pack: {e}"))
}

/// Parse a user-supplied JSON string into a [`PersonaPack`].
///
/// Errors out (with a human-readable reason) when:
/// - input exceeds [`PERSONA_PACK_MAX_BYTES`],
/// - input is not valid JSON,
/// - the envelope is missing required fields (`packVersion`, `traits`),
/// - `packVersion` is higher than this binary supports,
/// - the `traits` field is not an object.
///
/// Per-asset validation (unknown `kind`, missing `id`) is **not** done
/// here — it happens at apply time so the importer can keep the rest
/// of the pack and surface a per-entry warning.
pub fn parse_pack(raw: &str) -> Result<PersonaPack, String> {
    if raw.len() > PERSONA_PACK_MAX_BYTES {
        return Err(format!(
            "Pack too large ({} bytes; max {})",
            raw.len(),
            PERSONA_PACK_MAX_BYTES
        ));
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Pack is empty".to_string());
    }
    let pack: PersonaPack = serde_json::from_str(trimmed)
        .map_err(|e| format!("Pack is not valid JSON: {e}"))?;

    if pack.pack_version > PERSONA_PACK_VERSION {
        return Err(format!(
            "Pack format version {} is newer than this build supports (max {}). Update TerranSoul to import.",
            pack.pack_version, PERSONA_PACK_VERSION
        ));
    }
    if !pack.traits.is_object() {
        return Err("Pack 'traits' must be a JSON object".to_string());
    }
    Ok(pack)
}

/// Plain summary of what an import would (or did) change. Used for the
/// "Preview" card and as the return value of the import command so the
/// UI can surface "imported 3 expressions, skipped 1 with unknown
/// kind" in a single response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ImportReport {
    /// Whether the persona traits would be / were replaced.
    pub traits_applied: bool,
    /// Number of expressions accepted by the validator.
    pub expressions_accepted: u32,
    /// Number of motions accepted by the validator.
    pub motions_accepted: u32,
    /// Per-entry skip reasons (max 32 entries to keep the JSON small).
    /// Each string is a single-line, user-facing explanation.
    #[serde(default)]
    pub skipped: Vec<String>,
}

/// Validate a single learned-asset JSON value. Returns `Ok(id)` when
/// the value is acceptable for writing to disk via the existing
/// `commands::persona::save_asset` path, or `Err` with the reason it
/// was rejected. The id rules mirror `commands::persona::validate_id`
/// (alphanumeric + `_-`, length 1..=128).
pub fn validate_asset(value: &serde_json::Value, expected_kind: &str) -> Result<String, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "asset is not a JSON object".to_string())?;
    let kind = obj
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "asset missing 'kind'".to_string())?;
    if kind != expected_kind {
        return Err(format!(
            "asset has wrong kind '{kind}' (expected '{expected_kind}')"
        ));
    }
    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "asset missing 'id'".to_string())?
        .to_string();
    if id.is_empty() || id.len() > 128 {
        return Err("asset 'id' length out of range".to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("asset 'id' contains illegal characters".to_string());
    }
    Ok(id)
}

/// Push a skip reason into the report, capping the total to keep the
/// payload bounded. Caller passes a one-line, user-facing message.
pub fn note_skip(report: &mut ImportReport, reason: String) {
    const MAX_SKIPS: usize = 32;
    if report.skipped.len() < MAX_SKIPS {
        report.skipped.push(reason);
    } else if report.skipped.len() == MAX_SKIPS {
        report.skipped.push("…(further skip messages truncated)".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_traits() -> serde_json::Value {
        json!({
            "version": 1,
            "name": "Lia",
            "role": "librarian",
            "bio": "Quiet bookworm.",
            "tone": ["warm"],
            "quirks": [],
            "avoid": ["medical advice"],
            "active": true,
            "updatedAt": 1700000000000_i64
        })
    }

    fn sample_expression(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "kind": "expression",
            "name": "Smug",
            "trigger": "smug",
            "weights": { "happy": 0.4 },
            "learnedAt": 1700000000000_i64
        })
    }

    fn sample_motion(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "kind": "motion",
            "name": "Shrug",
            "trigger": "shrug",
            "fps": 30,
            "duration_s": 1.0,
            "frames": [],
            "learnedAt": 1700000000000_i64
        })
    }

    #[test]
    fn build_pack_round_trips_through_parse() {
        let original = build_pack(
            sample_traits(),
            vec![sample_expression("lex_A")],
            vec![sample_motion("lmo_A")],
            Some("My setup".into()),
            1_700_000_001_000,
        );
        let raw = pack_to_string(&original).unwrap();
        let parsed = parse_pack(&raw).unwrap();
        assert_eq!(parsed, original);
        assert_eq!(parsed.pack_version, PERSONA_PACK_VERSION);
        assert_eq!(parsed.note.as_deref(), Some("My setup"));
    }

    #[test]
    fn build_pack_drops_empty_or_whitespace_note() {
        let p1 = build_pack(sample_traits(), vec![], vec![], Some("   ".into()), 0);
        assert!(p1.note.is_none());
        let p2 = build_pack(sample_traits(), vec![], vec![], Some("".into()), 0);
        assert!(p2.note.is_none());
        let p3 = build_pack(sample_traits(), vec![], vec![], None, 0);
        assert!(p3.note.is_none());
    }

    #[test]
    fn parse_pack_rejects_empty_input() {
        assert!(parse_pack("").is_err());
        assert!(parse_pack("   \n  ").is_err());
    }

    #[test]
    fn parse_pack_rejects_oversize_payload() {
        let huge = "x".repeat(PERSONA_PACK_MAX_BYTES + 1);
        let err = parse_pack(&huge).unwrap_err();
        assert!(err.contains("too large"));
    }

    #[test]
    fn parse_pack_rejects_garbage_json() {
        let err = parse_pack("not json").unwrap_err();
        assert!(err.contains("not valid JSON"));
    }

    #[test]
    fn parse_pack_rejects_future_pack_version() {
        let raw = serde_json::to_string(&json!({
            "packVersion": PERSONA_PACK_VERSION + 5,
            "exportedAt": 0,
            "traits": sample_traits(),
        }))
        .unwrap();
        let err = parse_pack(&raw).unwrap_err();
        assert!(err.contains("newer than this build"));
    }

    #[test]
    fn parse_pack_rejects_non_object_traits() {
        let raw = serde_json::to_string(&json!({
            "packVersion": PERSONA_PACK_VERSION,
            "exportedAt": 0,
            "traits": "not an object",
        }))
        .unwrap();
        let err = parse_pack(&raw).unwrap_err();
        assert!(err.contains("must be a JSON object"));
    }

    #[test]
    fn parse_pack_accepts_traits_only_pack() {
        let raw = serde_json::to_string(&json!({
            "packVersion": PERSONA_PACK_VERSION,
            "exportedAt": 1,
            "traits": sample_traits(),
        }))
        .unwrap();
        let p = parse_pack(&raw).unwrap();
        assert!(p.expressions.is_empty());
        assert!(p.motions.is_empty());
    }

    #[test]
    fn parse_pack_rejects_missing_required_envelope_fields() {
        // Missing packVersion.
        assert!(parse_pack(r#"{"traits":{}}"#).is_err());
        // Missing traits.
        let raw = format!(r#"{{"packVersion":{PERSONA_PACK_VERSION},"exportedAt":0}}"#);
        assert!(parse_pack(&raw).is_err());
    }

    #[test]
    fn validate_asset_accepts_well_formed_expression() {
        let id = validate_asset(&sample_expression("lex_OK"), "expression").unwrap();
        assert_eq!(id, "lex_OK");
    }

    #[test]
    fn validate_asset_rejects_wrong_kind() {
        let err = validate_asset(&sample_expression("lex_X"), "motion").unwrap_err();
        assert!(err.contains("wrong kind"));
    }

    #[test]
    fn validate_asset_rejects_traversal_id() {
        let bad = json!({"id": "../escape", "kind": "expression"});
        let err = validate_asset(&bad, "expression").unwrap_err();
        assert!(err.contains("illegal characters"));
    }

    #[test]
    fn validate_asset_rejects_empty_id() {
        let bad = json!({"id": "", "kind": "expression"});
        let err = validate_asset(&bad, "expression").unwrap_err();
        assert!(err.contains("length out of range"));
    }

    #[test]
    fn validate_asset_rejects_oversize_id() {
        let id: String = "a".repeat(200);
        let bad = json!({"id": id, "kind": "expression"});
        let err = validate_asset(&bad, "expression").unwrap_err();
        assert!(err.contains("length out of range"));
    }

    #[test]
    fn validate_asset_rejects_missing_id() {
        let bad = json!({"kind": "expression"});
        let err = validate_asset(&bad, "expression").unwrap_err();
        assert!(err.contains("missing 'id'"));
    }

    #[test]
    fn validate_asset_rejects_non_object() {
        let err = validate_asset(&json!("a string"), "expression").unwrap_err();
        assert!(err.contains("not a JSON object"));
    }

    #[test]
    fn validate_asset_accepts_well_formed_motion() {
        let id = validate_asset(&sample_motion("lmo_OK"), "motion").unwrap();
        assert_eq!(id, "lmo_OK");
    }

    #[test]
    fn note_skip_caps_at_thirty_two_entries_plus_truncation_marker() {
        let mut report = ImportReport::default();
        for i in 0..40 {
            note_skip(&mut report, format!("skip {i}"));
        }
        // 32 real entries + 1 truncation marker = 33.
        assert_eq!(report.skipped.len(), 33);
        assert!(report.skipped.last().unwrap().contains("truncated"));
    }
}
