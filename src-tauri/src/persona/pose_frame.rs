//! LLM pose-frame parser and clamp (Chunk 14.16b foundation).
//!
//! Pure-logic Rust module that 14.16b's frontend `PoseAnimator` will
//! consume: parses `<pose>{ ... }</pose>` JSON payloads emitted by the
//! brain, validates the bone names against the canonical 11-bone VRM
//! rig (per `docs/llm-animation-research.md`), and clamps each Euler
//! component to the safe range so non-anatomical poses can never reach
//! the renderer regardless of LLM behaviour.
//!
//! This module is **frontend-agnostic**: it does not depend on Three.js
//! or any DOM type. The same parsing/clamp pipeline is used by:
//!
//! 1. The (future) `generate_motion_from_text` Tauri command (chunk
//!    14.16c) for offline clip generation.
//! 2. The (future) stream-tag parser when it forwards `<pose>` payloads
//!    to the frontend (chunk 14.16b).
//! 3. Tests + golden-vector harnesses for the self-improve loop (chunk
//!    14.16f).
//!
//! Per the research doc:
//! - Canonical bones: 11 upper-body bones (head, neck, spine, chest,
//!   hips, left/right Upper/Lower Arm, left/right Shoulder).
//! - Coordinate system: right-handed, Y-up, Euler XYZ in radians.
//! - Hard clamp: ±0.5 rad per component (~28°). Larger values are
//!   silently clamped, not rejected, so a noisy LLM still produces a
//!   valid pose.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The 11 upper-body VRM bones the LLM-as-animator pipeline accepts.
/// Order is canonical for serialisation / golden vectors.
pub const CANONICAL_BONES: &[&str] = &[
    "head",
    "neck",
    "spine",
    "chest",
    "hips",
    "leftUpperArm",
    "rightUpperArm",
    "leftLowerArm",
    "rightLowerArm",
    "leftShoulder",
    "rightShoulder",
];

/// Hard clamp on every Euler-radian component. Values outside this
/// symmetric range are silently clamped so non-anatomical poses cannot
/// reach the renderer.
pub const CLAMP_RADIANS: f32 = 0.5;

/// Default frame duration when the LLM omits `duration_s`. 2 seconds is
/// long enough for the spring to settle from idle without feeling sticky.
pub const DEFAULT_DURATION_SECS: f32 = 2.0;

/// Maximum frame duration. Anything longer is treated as a clip and
/// belongs in `LearnedMotion`, not a single-frame pose tag.
pub const MAX_DURATION_SECS: f32 = 10.0;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Easing modes the frontend `PoseAnimator` understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PoseEasing {
    Linear,
    EaseInOut,
    /// Critically-damped spring (default). Best for natural motion.
    #[default]
    Spring,
}

/// A single LLM-emitted pose frame. The frontend converts this into a
/// damped-spring target for each bone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmPoseFrame {
    /// Bone name → Euler XYZ radians. Use a sorted map so serialised
    /// output is deterministic for golden tests.
    pub bones: BTreeMap<String, [f32; 3]>,
    /// How long to hold this pose before fading back to procedural idle.
    /// Falls back to [`DEFAULT_DURATION_SECS`] when omitted.
    #[serde(default = "default_duration")]
    pub duration_s: f32,
    /// Easing curve. Defaults to [`PoseEasing::Spring`].
    #[serde(default)]
    pub easing: PoseEasing,
    /// Optional VRM expression weights (0.0–1.0) merged into the
    /// face-blendshape pipeline. Empty by default.
    #[serde(default)]
    pub expression: BTreeMap<String, f32>,
}

fn default_duration() -> f32 {
    DEFAULT_DURATION_SECS
}

impl Default for LlmPoseFrame {
    fn default() -> Self {
        Self {
            bones: BTreeMap::new(),
            duration_s: DEFAULT_DURATION_SECS,
            easing: PoseEasing::Spring,
            expression: BTreeMap::new(),
        }
    }
}

/// Result of attempting to parse a pose payload. The parser is
/// **forgiving** — unknown bones are dropped, out-of-range values are
/// clamped, and missing fields use defaults. The only hard failure is
/// non-JSON or JSON that doesn't have a `bones` map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoseParseResult {
    pub frame: LlmPoseFrame,
    /// Bone names the LLM emitted that are not in [`CANONICAL_BONES`].
    /// Returned (rather than silently dropped) so callers can log a
    /// telemetry warning.
    pub dropped_bones: Vec<String>,
    /// Number of Euler components that were out of range and got
    /// clamped to ±[`CLAMP_RADIANS`]. Useful as a telemetry signal —
    /// a high count usually means the model needs a better system
    /// prompt.
    pub clamped_components: usize,
}

/// Parse error variants. Only the structural problems trigger an `Err`;
/// everything else is recovered into a [`PoseParseResult`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PoseParseError {
    #[error("payload is not valid JSON: {0}")]
    InvalidJson(String),
    #[error("payload missing required `bones` map")]
    MissingBones,
    #[error("payload has no recognised bones (all dropped or empty)")]
    NoRecognisedBones,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse a `<pose>...</pose>` JSON payload (without the tags themselves).
///
/// Forgiving by design — see [`PoseParseResult`] for what gets recovered.
pub fn parse_pose_payload(payload: &str) -> Result<PoseParseResult, PoseParseError> {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return Err(PoseParseError::InvalidJson("empty payload".into()));
    }

    let raw: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| PoseParseError::InvalidJson(e.to_string()))?;

    // The LLM may emit either a bare bones map (forgiving) or the full
    // shape `{ bones: {...}, duration_s: ..., easing: ..., expression: ... }`.
    // Detect by checking whether the top-level object has a `bones` key.
    let obj = raw
        .as_object()
        .ok_or_else(|| PoseParseError::InvalidJson("top-level must be a JSON object".into()))?;

    let (bones_value, duration_s, easing, expression) = if let Some(b) = obj.get("bones") {
        (
            b.clone(),
            obj.get("duration_s")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32)
                .unwrap_or(DEFAULT_DURATION_SECS),
            obj.get("easing")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            obj.get("expression")
                .and_then(|v| v.as_object())
                .map(parse_expression_map)
                .unwrap_or_default(),
        )
    } else {
        // Bare bones map — every key must look like a bone.
        (
            serde_json::Value::Object(obj.clone()),
            DEFAULT_DURATION_SECS,
            PoseEasing::default(),
            BTreeMap::new(),
        )
    };

    let bones_obj = bones_value
        .as_object()
        .ok_or(PoseParseError::MissingBones)?;

    let canonical: std::collections::HashSet<&str> = CANONICAL_BONES.iter().copied().collect();

    let mut bones: BTreeMap<String, [f32; 3]> = BTreeMap::new();
    let mut dropped: Vec<String> = Vec::new();
    let mut clamped_count: usize = 0;

    for (name, value) in bones_obj.iter() {
        if !canonical.contains(name.as_str()) {
            dropped.push(name.clone());
            continue;
        }
        let arr = match value.as_array() {
            Some(a) => a,
            None => {
                dropped.push(name.clone());
                continue;
            }
        };
        if arr.len() != 3 {
            dropped.push(name.clone());
            continue;
        }
        let mut euler = [0.0_f32; 3];
        for (i, v) in arr.iter().enumerate() {
            let raw = v.as_f64().unwrap_or(0.0) as f32;
            if !raw.is_finite() {
                clamped_count += 1;
                euler[i] = 0.0;
                continue;
            }
            let clamped = raw.clamp(-CLAMP_RADIANS, CLAMP_RADIANS);
            if (clamped - raw).abs() > f32::EPSILON {
                clamped_count += 1;
            }
            euler[i] = clamped;
        }
        bones.insert(name.clone(), euler);
    }

    if bones.is_empty() {
        return Err(PoseParseError::NoRecognisedBones);
    }

    let duration_clamped = duration_s.clamp(0.05, MAX_DURATION_SECS);
    let frame = LlmPoseFrame {
        bones,
        duration_s: duration_clamped,
        easing,
        expression,
    };

    Ok(PoseParseResult {
        frame,
        dropped_bones: dropped,
        clamped_components: clamped_count,
    })
}

fn parse_expression_map(obj: &serde_json::Map<String, serde_json::Value>) -> BTreeMap<String, f32> {
    obj.iter()
        .filter_map(|(k, v)| {
            let f = v.as_f64()? as f32;
            if !f.is_finite() {
                return None;
            }
            Some((k.clone(), f.clamp(0.0, 1.0)))
        })
        .collect()
}

/// Extract every `<pose>...</pose>` payload from a streamed chat
/// response. Tag-aware (case-insensitive); does **not** require the
/// brain to emit only one pose per response. Empty / malformed
/// payloads are silently skipped so a single bad pose tag never breaks
/// the rest of the stream.
pub fn extract_pose_payloads(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = text.to_lowercase();
    let mut cursor = 0_usize;
    while cursor < text.len() {
        let Some(open_rel) = lower[cursor..].find("<pose>") else {
            break;
        };
        let payload_start = cursor + open_rel + "<pose>".len();
        let Some(close_rel) = lower[payload_start..].find("</pose>") else {
            break;
        };
        let payload_end = payload_start + close_rel;
        out.push(text[payload_start..payload_end].to_string());
        cursor = payload_end + "</pose>".len();
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_bones_has_eleven() {
        assert_eq!(CANONICAL_BONES.len(), 11);
    }

    #[test]
    fn canonical_bones_has_no_duplicates() {
        let mut sorted: Vec<&&str> = CANONICAL_BONES.iter().collect();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), CANONICAL_BONES.len());
    }

    #[test]
    fn parses_full_frame_shape() {
        let payload = r#"{
            "bones": { "head": [0.1, -0.05, 0.0], "spine": [0.0, 0.0, 0.03] },
            "duration_s": 1.5,
            "easing": "ease-in-out",
            "expression": { "happy": 0.7 }
        }"#;
        let r = parse_pose_payload(payload).unwrap();
        assert_eq!(r.frame.bones.len(), 2);
        assert_eq!(r.frame.bones["head"], [0.1, -0.05, 0.0]);
        assert_eq!(r.frame.duration_s, 1.5);
        assert_eq!(r.frame.easing, PoseEasing::EaseInOut);
        assert_eq!(r.frame.expression.get("happy").copied(), Some(0.7));
        assert!(r.dropped_bones.is_empty());
        assert_eq!(r.clamped_components, 0);
    }

    #[test]
    fn parses_bare_bones_map() {
        let payload = r#"{ "head": [0.1, 0.0, 0.0] }"#;
        let r = parse_pose_payload(payload).unwrap();
        assert_eq!(r.frame.bones.len(), 1);
        assert_eq!(r.frame.duration_s, DEFAULT_DURATION_SECS);
        assert_eq!(r.frame.easing, PoseEasing::Spring);
    }

    #[test]
    fn drops_unknown_bones() {
        let payload = r#"{ "bones": { "head": [0.1, 0, 0], "tail": [1, 1, 1] } }"#;
        let r = parse_pose_payload(payload).unwrap();
        assert_eq!(r.frame.bones.len(), 1);
        assert_eq!(r.dropped_bones, vec!["tail".to_string()]);
    }

    #[test]
    fn clamps_out_of_range_components() {
        let payload = r#"{ "bones": { "head": [2.0, -3.0, 0.4] } }"#;
        let r = parse_pose_payload(payload).unwrap();
        let head = r.frame.bones["head"];
        assert_eq!(head[0], CLAMP_RADIANS);
        assert_eq!(head[1], -CLAMP_RADIANS);
        assert_eq!(head[2], 0.4);
        assert_eq!(r.clamped_components, 2);
    }

    #[test]
    fn replaces_non_finite_with_zero() {
        // 1e40 is valid f64 (finite) but overflows f32::MAX (~3.4e38) to
        // +Inf when cast. Our parser must replace +/-Inf and NaN with 0
        // and increment clamped_count.
        let payload = r#"{ "bones": { "head": [1e40, 0, 0] } }"#;
        let r = parse_pose_payload(payload).unwrap();
        assert_eq!(r.frame.bones["head"][0], 0.0);
        assert!(r.clamped_components >= 1);
    }

    #[test]
    fn rejects_invalid_json() {
        let r = parse_pose_payload("not json");
        assert!(matches!(r, Err(PoseParseError::InvalidJson(_))));
    }

    #[test]
    fn rejects_empty_payload() {
        let r = parse_pose_payload("   ");
        assert!(matches!(r, Err(PoseParseError::InvalidJson(_))));
    }

    #[test]
    fn rejects_when_no_recognised_bones() {
        let payload = r#"{ "bones": { "tail": [0, 0, 0], "wing": [0, 0, 0] } }"#;
        let r = parse_pose_payload(payload);
        assert_eq!(r, Err(PoseParseError::NoRecognisedBones));
    }

    #[test]
    fn rejects_top_level_array() {
        let r = parse_pose_payload("[1,2,3]");
        assert!(matches!(r, Err(PoseParseError::InvalidJson(_))));
    }

    #[test]
    fn drops_bones_with_wrong_arity() {
        let payload = r#"{ "bones": { "head": [0.1, 0.0], "spine": [0.0, 0.0, 0.03] } }"#;
        let r = parse_pose_payload(payload).unwrap();
        assert_eq!(r.frame.bones.len(), 1);
        assert!(r.dropped_bones.contains(&"head".to_string()));
    }

    #[test]
    fn duration_below_floor_is_clamped() {
        let payload = r#"{ "bones": { "head": [0,0,0] }, "duration_s": 0.001 }"#;
        let r = parse_pose_payload(payload).unwrap();
        assert!(r.frame.duration_s >= 0.05);
    }

    #[test]
    fn duration_above_ceiling_is_clamped() {
        let payload = r#"{ "bones": { "head": [0,0,0] }, "duration_s": 9999 }"#;
        let r = parse_pose_payload(payload).unwrap();
        assert_eq!(r.frame.duration_s, MAX_DURATION_SECS);
    }

    #[test]
    fn expression_weights_clamped_to_unit_interval() {
        let payload = r#"{
            "bones": { "head": [0,0,0] },
            "expression": { "happy": 1.5, "sad": -0.4, "neutral": 0.5 }
        }"#;
        let r = parse_pose_payload(payload).unwrap();
        assert_eq!(r.frame.expression["happy"], 1.0);
        assert_eq!(r.frame.expression["sad"], 0.0);
        assert_eq!(r.frame.expression["neutral"], 0.5);
    }

    #[test]
    fn easing_default_is_spring() {
        let payload = r#"{ "bones": { "head": [0,0,0] } }"#;
        let r = parse_pose_payload(payload).unwrap();
        assert_eq!(r.frame.easing, PoseEasing::Spring);
    }

    #[test]
    fn unknown_easing_falls_back_to_default() {
        let payload = r#"{ "bones": { "head": [0,0,0] }, "easing": "wibble" }"#;
        let r = parse_pose_payload(payload).unwrap();
        assert_eq!(r.frame.easing, PoseEasing::Spring);
    }

    #[test]
    fn extract_pose_payloads_finds_single_tag() {
        let text = "Some text <pose>{\"head\":[0,0,0]}</pose> more.";
        let payloads = extract_pose_payloads(text);
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0], r#"{"head":[0,0,0]}"#);
    }

    #[test]
    fn extract_pose_payloads_finds_multiple_tags() {
        let text = "<pose>{\"head\":[0,0,0]}</pose> mid <pose>{\"spine\":[0,0,0]}</pose>";
        let payloads = extract_pose_payloads(text);
        assert_eq!(payloads.len(), 2);
    }

    #[test]
    fn extract_pose_payloads_is_case_insensitive() {
        let text = "<POSE>{\"head\":[0,0,0]}</POSE>";
        let payloads = extract_pose_payloads(text);
        assert_eq!(payloads.len(), 1);
    }

    #[test]
    fn extract_pose_payloads_handles_no_tags() {
        let payloads = extract_pose_payloads("plain text");
        assert!(payloads.is_empty());
    }

    #[test]
    fn extract_pose_payloads_skips_unclosed_tag() {
        let text = "<pose>{\"head\":[0,0,0]} no close";
        let payloads = extract_pose_payloads(text);
        assert!(payloads.is_empty());
    }

    #[test]
    fn frame_serde_roundtrip() {
        let payload = r#"{
            "bones": { "head": [0.1, 0.0, 0.0], "spine": [0.0, 0.0, 0.05] },
            "duration_s": 2.0,
            "easing": "spring"
        }"#;
        let parsed = parse_pose_payload(payload).unwrap().frame;
        let json = serde_json::to_string(&parsed).unwrap();
        let round: LlmPoseFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(round, parsed);
    }

    #[test]
    fn end_to_end_extract_then_parse() {
        // Simulate a streaming response from the brain: prose + a pose
        // tag. The full pipeline (extract → parse) must round-trip
        // cleanly into a usable `LlmPoseFrame`.
        let response = "I'll tilt my head curiously. \
                        <pose>{\"head\":[0.2,0,0]}</pose> \
                        Now what would you like to know?";
        let payloads = extract_pose_payloads(response);
        assert_eq!(payloads.len(), 1);
        let frame = parse_pose_payload(&payloads[0]).unwrap().frame;
        assert_eq!(frame.bones["head"], [0.2, 0.0, 0.0]);
    }
}
