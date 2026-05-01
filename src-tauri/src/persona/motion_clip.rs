//! LLM-generated motion-clip parser + validator (Chunk 14.16c1).
//!
//! Pure-logic foundation for the upcoming `generate_motion_from_text`
//! Tauri command. Parses the brain's JSON reply, validates against the
//! canonical 11-bone VRM rig (re-using [`crate::persona::pose_frame`]),
//! enforces monotonic non-decreasing timestamps, and clamps every Euler
//! component to ±[`pose_frame::CLAMP_RADIANS`] so non-anatomical clips
//! never reach the renderer.
//!
//! The output is a [`GeneratedMotion`] struct that serialises to the
//! same JSON shape the frontend `LearnedMotion` Pinia store expects
//! (see `src/stores/persona-types.ts`). The motion can be passed
//! straight to `save_learned_motion` without further validation.
//!
//! ## Forgiving by design
//!
//! Like the pose-frame parser, this module is forgiving:
//!
//! - Unknown bones inside frames are dropped with a telemetry warning.
//! - Out-of-range Eulers are clamped (counter incremented).
//! - Non-finite values are replaced with `0.0`.
//! - Frames that would yield zero recognised bones after sanitisation
//!   are skipped (rather than failing the whole clip).
//! - Non-monotonic timestamps are repaired by sorting the frames.
//!
//! Hard failures (returned as [`MotionParseError`]) only happen when:
//!
//! - the payload isn't valid JSON;
//! - the payload has no `frames` array;
//! - every frame is empty after sanitisation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::persona::pose_frame::{CANONICAL_BONES, CLAMP_RADIANS};

/// Minimum frames the LLM must produce. Below this, the clip can't
/// hold up against any procedural fallback.
pub const MIN_FRAMES: usize = 2;
/// Maximum frames per clip. Beyond this, the LLM's reply is almost
/// certainly hallucinated padding — and bigger clips belong in a
/// proper authoring tool, not a chat reply.
pub const MAX_FRAMES: usize = 240;
/// Default clip length when the caller doesn't pin one down.
pub const DEFAULT_DURATION_S: f32 = 3.0;
/// Hard upper bound on clip length (matches `LearnedMotion` UX).
pub const MAX_DURATION_S: f32 = 30.0;
/// Default frame rate. 24 fps is the cinematic floor and keeps the
/// total frame count well below [`MAX_FRAMES`] for normal clips.
pub const DEFAULT_FPS: u32 = 24;
/// Maximum supported frame rate.
pub const MAX_FPS: u32 = 60;

/// One frame in a generated clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedFrame {
    /// Seconds since clip start.
    pub t: f32,
    /// Bone-name → Euler XYZ radians (sorted for deterministic output).
    pub bones: BTreeMap<String, [f32; 3]>,
}

/// A complete LLM-generated motion clip, shaped to match the
/// frontend `LearnedMotion` Pinia type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedMotion {
    /// Stable id, generated server-side (not derived from the LLM
    /// reply — keeps storage paths predictable).
    pub id: String,
    /// Always `"motion"` so the frontend store classifier can route it.
    pub kind: String,
    /// Human-readable name (defaults to a slug of the description).
    pub name: String,
    /// Trigger key the LLM can later emit to play this clip.
    pub trigger: String,
    pub fps: u32,
    pub duration_s: f32,
    pub frames: Vec<GeneratedFrame>,
    /// ms epoch of generation (set by the caller — `chrono`-free here
    /// so the module stays unit-testable without a clock source).
    #[serde(rename = "learnedAt")]
    pub learned_at: i64,
}

/// Parser-level diagnostics. A successful parse returns this alongside
/// the [`GeneratedMotion`] so the caller can surface "we cleaned X
/// bones for you" UX hints + emit telemetry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionParseDiagnostics {
    /// Frames the LLM emitted that contributed zero canonical bones
    /// after sanitisation. Skipped from the output.
    pub dropped_frames: usize,
    /// Bone names not in [`CANONICAL_BONES`]. Counted, not stored,
    /// because the LLM tends to repeat the same misspellings.
    pub dropped_bone_components: usize,
    /// Euler components that were out of range and got clamped.
    pub clamped_components: usize,
    /// True iff the input frames had to be re-sorted to be monotonic.
    pub repaired_timestamps: bool,
    /// True iff the parser had to renormalise the duration to fit the
    /// actual frame timestamps.
    pub repaired_duration: bool,
}

/// Hard parse failures.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MotionParseError {
    #[error("payload is not valid JSON: {0}")]
    InvalidJson(String),
    #[error("payload missing required `frames` array")]
    MissingFrames,
    #[error("clip has fewer than {min} usable frames after sanitisation")]
    NotEnoughFrames { min: usize },
    #[error("clip has more than {max} frames; refusing to ingest")]
    TooManyFrames { max: usize },
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Inputs to [`build_motion_prompt`] and [`parse_motion_payload`].
#[derive(Debug, Clone)]
pub struct MotionRequest {
    pub description: String,
    pub duration_s: f32,
    pub fps: u32,
}

impl MotionRequest {
    /// Clamp every field into a sane range so the prompt template
    /// never renders garbage.
    pub fn sanitised(self) -> Self {
        let MotionRequest {
            description,
            duration_s,
            fps,
        } = self;
        Self {
            description: description.trim().to_string(),
            duration_s: clamp_duration(duration_s),
            fps: clamp_fps(fps),
        }
    }
}

/// Build the system + user prompt the brain should answer for a given
/// motion request. Kept pure so it can be unit-tested without the
/// network.
///
/// Returned tuple is `(system, user)`.
pub fn build_motion_prompt(req: &MotionRequest) -> (String, String) {
    build_motion_prompt_with_hint(req, "")
}

/// Like [`build_motion_prompt`] but appends an optional one-line hint
/// to the system prompt. Used by the self-improve feedback loop
/// (Chunk 14.16e) to feed the LLM the trusted-trigger leaderboard so
/// it can match the user's preferred movement vocabulary on the next
/// generation. Empty / whitespace-only hint = no-op (same prompt as
/// the parameter-less version).
pub fn build_motion_prompt_with_hint(req: &MotionRequest, hint: &str) -> (String, String) {
    let bone_list = CANONICAL_BONES.join(", ");
    let mut system = "You generate VRM body-animation clips from text descriptions. \
        Reply with a single valid JSON object — no prose, no markdown fences, \
        no commentary. The schema is documented in the user message."
        .to_string();
    let trimmed_hint = hint.trim();
    if !trimmed_hint.is_empty() {
        system.push_str("\n\n");
        system.push_str(trimmed_hint);
    }
    let user = format!(
        "Generate a VRM animation clip for: \"{description}\".\n\n\
        Output ONLY a JSON object of this exact shape:\n\
        {{\"frames\":[{{\"t\":0.0,\"bones\":{{\"head\":[0.0,0.0,0.0]}}}}, \
        {{\"t\":0.5,\"bones\":{{\"head\":[0.1,0.0,0.0]}}}}]}}\n\n\
        Rules:\n\
        - Duration: {duration:.2}s at {fps} FPS (so ~{count} frames).\n\
        - Allowed bones: {bones}.\n\
        - Bone values are Euler XYZ angles in radians, ±0.5 max.\n\
        - Timestamps `t` start at 0.0 and increase monotonically.\n\
        - Use sparse keyframes — you do NOT need a frame for every FPS tick.\n\
        - At least {min_frames} frames; at most {max_frames}.\n\
        - Output JSON only. No explanation. No markdown.\n",
        description = req.description,
        duration = req.duration_s,
        fps = req.fps,
        count = (req.duration_s * req.fps as f32).round() as i64,
        bones = bone_list,
        min_frames = MIN_FRAMES,
        max_frames = MAX_FRAMES,
    );
    (system, user)
}

/// Parse the brain's reply. Strips any leading/trailing markdown fences
/// (a common LLM tic) before attempting JSON deserialisation.
///
/// `id`, `name`, `trigger`, `learned_at` are caller-provided so the
/// pure module never needs a clock or ID generator.
pub fn parse_motion_payload(
    payload: &str,
    id: String,
    name: String,
    trigger: String,
    fps: u32,
    duration_s: f32,
    learned_at: i64,
) -> Result<(GeneratedMotion, MotionParseDiagnostics), MotionParseError> {
    let trimmed = strip_code_fences(payload).trim().to_string();
    if trimmed.is_empty() {
        return Err(MotionParseError::InvalidJson("empty payload".into()));
    }

    let raw: serde_json::Value =
        serde_json::from_str(&trimmed).map_err(|e| MotionParseError::InvalidJson(e.to_string()))?;
    let frames_value = raw
        .get("frames")
        .ok_or(MotionParseError::MissingFrames)?
        .as_array()
        .ok_or(MotionParseError::MissingFrames)?;

    if frames_value.len() > MAX_FRAMES {
        return Err(MotionParseError::TooManyFrames { max: MAX_FRAMES });
    }

    let canonical: std::collections::HashSet<&str> = CANONICAL_BONES.iter().copied().collect();
    let mut diagnostics = MotionParseDiagnostics::default();
    let mut frames: Vec<GeneratedFrame> = Vec::with_capacity(frames_value.len());

    for frame_raw in frames_value {
        let obj = match frame_raw.as_object() {
            Some(o) => o,
            None => {
                diagnostics.dropped_frames += 1;
                continue;
            }
        };
        let t = obj
            .get("t")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32)
            .unwrap_or(0.0)
            .clamp(0.0, MAX_DURATION_S);
        let bones_obj = match obj.get("bones").and_then(|v| v.as_object()) {
            Some(b) => b,
            None => {
                diagnostics.dropped_frames += 1;
                continue;
            }
        };

        let mut bones: BTreeMap<String, [f32; 3]> = BTreeMap::new();
        for (name, value) in bones_obj.iter() {
            if !canonical.contains(name.as_str()) {
                diagnostics.dropped_bone_components += 1;
                continue;
            }
            let arr = match value.as_array() {
                Some(a) if a.len() == 3 => a,
                _ => {
                    diagnostics.dropped_bone_components += 1;
                    continue;
                }
            };
            let mut euler = [0.0_f32; 3];
            for (i, v) in arr.iter().enumerate() {
                let raw = v.as_f64().unwrap_or(0.0) as f32;
                if !raw.is_finite() {
                    diagnostics.clamped_components += 1;
                    euler[i] = 0.0;
                    continue;
                }
                let clamped = raw.clamp(-CLAMP_RADIANS, CLAMP_RADIANS);
                if (clamped - raw).abs() > f32::EPSILON {
                    diagnostics.clamped_components += 1;
                }
                euler[i] = clamped;
            }
            bones.insert(name.clone(), euler);
        }

        if bones.is_empty() {
            diagnostics.dropped_frames += 1;
            continue;
        }
        frames.push(GeneratedFrame { t, bones });
    }

    if frames.len() < MIN_FRAMES {
        return Err(MotionParseError::NotEnoughFrames { min: MIN_FRAMES });
    }

    // Repair non-monotonic timestamps by sorting in-place.
    let was_monotonic = frames
        .windows(2)
        .all(|w| w[0].t <= w[1].t + f32::EPSILON);
    if !was_monotonic {
        frames.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        diagnostics.repaired_timestamps = true;
    }

    // Renormalise duration if the LLM produced timestamps past the
    // requested length (clamp upward). Shorter is fine — the player
    // simply ends earlier.
    let last_t = frames.last().map(|f| f.t).unwrap_or(0.0);
    let actual_duration = if last_t > duration_s {
        diagnostics.repaired_duration = true;
        last_t.min(MAX_DURATION_S)
    } else {
        duration_s.clamp(0.05, MAX_DURATION_S)
    };

    Ok((
        GeneratedMotion {
            id,
            kind: "motion".to_string(),
            name,
            trigger,
            fps: clamp_fps(fps),
            duration_s: actual_duration,
            frames,
            learned_at,
        },
        diagnostics,
    ))
}

/// Slug a free-form description into a stable `trigger` key. Strips
/// any non-alphanumeric run, lowercases, and prefixes `learned-` so the
/// triggers can never collide with the canonical built-in motion list
/// (`idle`, `wave`, etc.).
pub fn slugify_trigger(description: &str) -> String {
    let mut slug = String::with_capacity(description.len());
    let mut prev_was_dash = false;
    for ch in description.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_was_dash = false;
        } else if !prev_was_dash {
            slug.push('-');
            prev_was_dash = true;
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "learned-motion".to_string()
    } else {
        format!("learned-{trimmed}")
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn clamp_duration(v: f32) -> f32 {
    if !v.is_finite() {
        return DEFAULT_DURATION_S;
    }
    v.clamp(0.05, MAX_DURATION_S)
}

fn clamp_fps(v: u32) -> u32 {
    if v == 0 {
        DEFAULT_FPS
    } else {
        v.min(MAX_FPS)
    }
}

/// Strip `` ```json `` / `` ``` `` fences a chatty LLM may wrap its
/// reply in. Idempotent — passes plain JSON straight through.
fn strip_code_fences(s: &str) -> &str {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        return rest.trim_end_matches("```").trim();
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        return rest.trim_end_matches("```").trim();
    }
    trimmed
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn req(d: &str) -> MotionRequest {
        MotionRequest {
            description: d.to_string(),
            duration_s: 2.0,
            fps: 30,
        }
    }

    fn parse_ok(payload: &str) -> (GeneratedMotion, MotionParseDiagnostics) {
        parse_motion_payload(
            payload,
            "test-id".into(),
            "test-name".into(),
            "learned-test".into(),
            30,
            2.0,
            42,
        )
        .expect("payload should parse")
    }

    #[test]
    fn build_prompt_mentions_every_canonical_bone() {
        let (system, user) = build_motion_prompt(&req("wave hello"));
        assert!(!system.is_empty());
        for bone in CANONICAL_BONES {
            assert!(user.contains(bone), "prompt missing bone {bone}");
        }
    }

    #[test]
    fn build_prompt_includes_request_metadata() {
        let r = MotionRequest {
            description: "spin around".into(),
            duration_s: 4.0,
            fps: 24,
        };
        let (_s, u) = build_motion_prompt(&r);
        assert!(u.contains("spin around"));
        assert!(u.contains("4.00"));
        assert!(u.contains("24 FPS"));
    }

    #[test]
    fn parses_minimal_two_frame_clip() {
        let payload = r#"{
            "frames":[
                {"t":0.0,"bones":{"head":[0,0,0]}},
                {"t":0.5,"bones":{"head":[0.1,0,0]}}
            ]
        }"#;
        let (clip, diag) = parse_ok(payload);
        assert_eq!(clip.kind, "motion");
        assert_eq!(clip.frames.len(), 2);
        assert_eq!(clip.frames[1].bones["head"], [0.1, 0.0, 0.0]);
        assert_eq!(diag, MotionParseDiagnostics::default());
    }

    #[test]
    fn drops_unknown_bones_per_frame() {
        let payload = r#"{
            "frames":[
                {"t":0,"bones":{"head":[0,0,0],"tail":[1,1,1]}},
                {"t":1,"bones":{"head":[0.1,0,0]}}
            ]
        }"#;
        let (clip, diag) = parse_ok(payload);
        assert!(!clip.frames[0].bones.contains_key("tail"));
        assert_eq!(diag.dropped_bone_components, 1);
    }

    #[test]
    fn clamps_out_of_range_eulers() {
        let payload = r#"{
            "frames":[
                {"t":0,"bones":{"head":[5.0,-3.0,0]}},
                {"t":1,"bones":{"head":[0,0,0]}}
            ]
        }"#;
        let (clip, diag) = parse_ok(payload);
        assert_eq!(clip.frames[0].bones["head"][0], CLAMP_RADIANS);
        assert_eq!(clip.frames[0].bones["head"][1], -CLAMP_RADIANS);
        assert!(diag.clamped_components >= 2);
    }

    #[test]
    fn replaces_non_finite_with_zero() {
        let payload = r#"{
            "frames":[
                {"t":0,"bones":{"head":[1e40,0,0]}},
                {"t":1,"bones":{"head":[0,0,0]}}
            ]
        }"#;
        let (clip, diag) = parse_ok(payload);
        assert_eq!(clip.frames[0].bones["head"][0], 0.0);
        assert!(diag.clamped_components >= 1);
    }

    #[test]
    fn skips_frames_with_no_recognised_bones() {
        let payload = r#"{
            "frames":[
                {"t":0,"bones":{"tail":[0,0,0]}},
                {"t":0.5,"bones":{"head":[0.1,0,0]}},
                {"t":1.0,"bones":{"head":[0.2,0,0]}}
            ]
        }"#;
        let (clip, diag) = parse_ok(payload);
        assert_eq!(clip.frames.len(), 2);
        assert_eq!(diag.dropped_frames, 1);
    }

    #[test]
    fn repairs_non_monotonic_timestamps() {
        let payload = r#"{
            "frames":[
                {"t":0.5,"bones":{"head":[0.1,0,0]}},
                {"t":0.0,"bones":{"head":[0,0,0]}},
                {"t":1.0,"bones":{"head":[0.2,0,0]}}
            ]
        }"#;
        let (clip, diag) = parse_ok(payload);
        assert!(diag.repaired_timestamps);
        let timestamps: Vec<f32> = clip.frames.iter().map(|f| f.t).collect();
        assert_eq!(timestamps, vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn rejects_below_minimum_frames() {
        let payload = r#"{"frames":[{"t":0,"bones":{"head":[0,0,0]}}]}"#;
        let r = parse_motion_payload(
            payload,
            "id".into(),
            "n".into(),
            "t".into(),
            30,
            2.0,
            0,
        );
        assert!(matches!(
            r,
            Err(MotionParseError::NotEnoughFrames { min: MIN_FRAMES })
        ));
    }

    #[test]
    fn rejects_above_maximum_frames() {
        let mut frames = String::from("{\"frames\":[");
        for i in 0..(MAX_FRAMES + 1) {
            if i > 0 {
                frames.push(',');
            }
            frames.push_str(&format!(
                "{{\"t\":{i},\"bones\":{{\"head\":[0,0,0]}}}}"
            ));
        }
        frames.push_str("]}");
        let r = parse_motion_payload(
            &frames,
            "id".into(),
            "n".into(),
            "t".into(),
            30,
            2.0,
            0,
        );
        assert!(matches!(
            r,
            Err(MotionParseError::TooManyFrames { max: MAX_FRAMES })
        ));
    }

    #[test]
    fn rejects_invalid_json() {
        let r = parse_motion_payload(
            "not json",
            "id".into(),
            "n".into(),
            "t".into(),
            30,
            2.0,
            0,
        );
        assert!(matches!(r, Err(MotionParseError::InvalidJson(_))));
    }

    #[test]
    fn rejects_payload_without_frames() {
        let r = parse_motion_payload(
            r#"{"foo":"bar"}"#,
            "id".into(),
            "n".into(),
            "t".into(),
            30,
            2.0,
            0,
        );
        assert_eq!(r, Err(MotionParseError::MissingFrames));
    }

    #[test]
    fn strips_markdown_code_fences() {
        let payload = "```json\n{\"frames\":[{\"t\":0,\"bones\":{\"head\":[0,0,0]}},{\"t\":1,\"bones\":{\"head\":[0,0,0]}}]}\n```";
        let (clip, _) = parse_ok(payload);
        assert_eq!(clip.frames.len(), 2);
    }

    #[test]
    fn duration_renormalised_when_frames_overshoot() {
        let payload = r#"{
            "frames":[
                {"t":0,"bones":{"head":[0,0,0]}},
                {"t":5.0,"bones":{"head":[0,0,0]}}
            ]
        }"#;
        let (clip, diag) = parse_ok(payload);
        assert!(diag.repaired_duration);
        assert!(clip.duration_s >= 5.0);
    }

    #[test]
    fn slugify_handles_diverse_inputs() {
        assert_eq!(slugify_trigger("Wave Hello!"), "learned-wave-hello");
        assert_eq!(slugify_trigger("  spin   around  "), "learned-spin-around");
        assert_eq!(slugify_trigger("@@@"), "learned-motion");
        assert_eq!(slugify_trigger("UPPER lower"), "learned-upper-lower");
    }

    #[test]
    fn motion_request_sanitised_clamps_inputs() {
        let r = MotionRequest {
            description: "  hi ".into(),
            duration_s: 999.0,
            fps: 999,
        };
        let s = r.sanitised();
        assert_eq!(s.description, "hi");
        assert_eq!(s.duration_s, MAX_DURATION_S);
        assert_eq!(s.fps, MAX_FPS);
    }

    #[test]
    fn fps_zero_normalises_to_default() {
        assert_eq!(clamp_fps(0), DEFAULT_FPS);
    }

    #[test]
    fn motion_serde_roundtrip_matches_frontend_shape() {
        // Ensure the serialised JSON has the exact field names the
        // frontend `LearnedMotion` Pinia type expects.
        let payload = r#"{"frames":[
            {"t":0,"bones":{"head":[0,0,0]}},
            {"t":1,"bones":{"head":[0.1,0,0]}}
        ]}"#;
        let (clip, _) = parse_ok(payload);
        let json = serde_json::to_value(&clip).unwrap();
        for k in &["id", "kind", "name", "trigger", "fps", "duration_s", "frames", "learnedAt"] {
            assert!(json.get(*k).is_some(), "missing field {k}");
        }
        assert_eq!(json["kind"], "motion");
    }
}
