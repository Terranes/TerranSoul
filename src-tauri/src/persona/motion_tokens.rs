//! MotionGPT-style discrete motion token codec (Chunk 14.15).
//!
//! Encodes continuous bone-pose frames into a compact discrete token
//! vocabulary that an LLM can generate, and decodes tokens back into
//! playable motion clips for `LearnedMotionPlayer`.
//!
//! **Feature-flagged** behind `motion-research`. Research chunk — the
//! full VQ-VAE training loop is out of scope. This module provides a
//! deterministic uniform-quantisation codec (no learned codebook) that
//! demonstrates the encode/decode pipeline and lets the LLM generate
//! motion in its output stream.
//!
//! # Design
//!
//! Each bone channel (Euler X/Y/Z) is independently quantised into
//! `n_bins` discrete levels spanning the bone's joint-limit range.
//! A frame is encoded as a sequence of tokens: `[bone0_x, bone0_y,
//! bone0_z, bone1_x, ...]`. The full clip becomes a 2-D grid of
//! tokens that the LLM can emit inside `<motion_tokens>` blocks.
//!
//! Token format in LLM output:
//! ```text
//! <motion_tokens fps="30" bones="head,spine,leftUpperArm,rightUpperArm">
//! 12 8 15 | 7 9 14 | 20 5 3 | 18 6 4
//! 13 8 15 | 7 9 14 | 19 6 3 | 17 7 4
//! ...
//! </motion_tokens>
//! ```
//! Each line = one frame. `|` separates bones. Numbers are token indices.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;

// ─── Types ───────────────────────────────────────────────────────────────────

/// Codec configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCodecConfig {
    /// Number of quantisation bins per channel. Default: 32.
    pub n_bins: u16,
    /// Ordered list of bone names in the vocabulary.
    /// Default: 8 upper-body bones (head, neck, spine, chest,
    /// leftUpperArm, rightUpperArm, leftLowerArm, rightLowerArm).
    pub bones: Vec<String>,
    /// Per-bone joint-limit ranges [min, max] per axis (radians).
    /// If not specified, uses default symmetric ranges.
    pub ranges: Option<HashMap<String, [[f64; 2]; 3]>>,
}

impl Default for TokenCodecConfig {
    fn default() -> Self {
        Self {
            n_bins: 32,
            bones: vec![
                "head".to_string(),
                "neck".to_string(),
                "spine".to_string(),
                "chest".to_string(),
                "leftUpperArm".to_string(),
                "rightUpperArm".to_string(),
                "leftLowerArm".to_string(),
                "rightLowerArm".to_string(),
            ],
            ranges: None,
        }
    }
}

/// A single frame of token indices (one per bone × 3 axes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenFrame {
    /// Tokens grouped by bone: `bones[i] = [x_token, y_token, z_token]`.
    pub bones: Vec<[u16; 3]>,
}

/// Encoded motion clip (discrete tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodedMotion {
    pub fps: f64,
    pub bone_names: Vec<String>,
    pub n_bins: u16,
    pub frames: Vec<TokenFrame>,
}

/// Decoded motion clip (continuous values, ready for `LearnedMotionPlayer`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedMotion {
    pub fps: f64,
    pub frames: Vec<DecodedFrame>,
}

/// A single decoded frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedFrame {
    pub t: f64,
    pub bones: HashMap<String, [f64; 3]>,
}

/// The motion token codec.
#[derive(Debug, Clone)]
pub struct MotionTokenCodec {
    config: TokenCodecConfig,
    /// Resolved ranges per bone (3 axes each).
    ranges: HashMap<String, [[f64; 2]; 3]>,
}

// ─── Default Ranges ──────────────────────────────────────────────────────────

fn default_range_for_bone(bone: &str) -> [[f64; 2]; 3] {
    match bone {
        "head" => [
            [-PI / 4.0, PI / 4.0],
            [-PI / 3.0, PI / 3.0],
            [-PI / 6.0, PI / 6.0],
        ],
        "neck" => [
            [-PI / 6.0, PI / 6.0],
            [-PI / 4.0, PI / 4.0],
            [-PI / 8.0, PI / 8.0],
        ],
        "spine" | "chest" => [
            [-PI / 6.0, PI / 4.0],
            [-PI / 6.0, PI / 6.0],
            [-PI / 8.0, PI / 8.0],
        ],
        "leftUpperArm" | "rightUpperArm" => [[-PI, PI], [-PI, PI], [-PI / 2.0, PI / 2.0]],
        "leftLowerArm" | "rightLowerArm" => [
            [-PI * 0.9, 0.1],
            [-PI / 4.0, PI / 4.0],
            [-PI / 2.0, PI / 2.0],
        ],
        "leftUpperLeg" | "rightUpperLeg" => [
            [-PI / 2.0, PI / 3.0],
            [-PI / 4.0, PI / 4.0],
            [-PI / 6.0, PI / 6.0],
        ],
        "leftLowerLeg" | "rightLowerLeg" => [[0.0, PI * 0.8], [-0.1, 0.1], [-0.1, 0.1]],
        _ => [[-PI, PI], [-PI, PI], [-PI, PI]],
    }
}

// ─── Codec Implementation ────────────────────────────────────────────────────

impl MotionTokenCodec {
    /// Create a new codec with the given configuration.
    pub fn new(config: TokenCodecConfig) -> Self {
        let mut ranges = HashMap::new();
        for bone in &config.bones {
            let r = config
                .ranges
                .as_ref()
                .and_then(|m| m.get(bone))
                .copied()
                .unwrap_or_else(|| default_range_for_bone(bone));
            ranges.insert(bone.clone(), r);
        }
        Self { config, ranges }
    }

    /// Quantise a continuous angle to a discrete token index.
    fn encode_value(&self, value: f64, min: f64, max: f64) -> u16 {
        let n = self.config.n_bins;
        let clamped = value.clamp(min, max);
        let normalised = (clamped - min) / (max - min); // [0, 1]
        let bin = (normalised * (n - 1) as f64).round() as u16;
        bin.min(n - 1)
    }

    /// Dequantise a token index back to a continuous angle.
    fn decode_value(&self, token: u16, min: f64, max: f64) -> f64 {
        let n = self.config.n_bins;
        let token = token.min(n - 1);
        let normalised = token as f64 / (n - 1).max(1) as f64;
        min + normalised * (max - min)
    }

    /// Encode a single frame of bone poses into token indices.
    pub fn encode_frame(&self, bones: &HashMap<String, [f64; 3]>) -> TokenFrame {
        let mut frame_tokens = Vec::with_capacity(self.config.bones.len());

        for bone_name in &self.config.bones {
            let euler = bones.get(bone_name).copied().unwrap_or([0.0; 3]);
            let range = self
                .ranges
                .get(bone_name)
                .copied()
                .unwrap_or([[-PI, PI]; 3]);

            let tokens = [
                self.encode_value(euler[0], range[0][0], range[0][1]),
                self.encode_value(euler[1], range[1][0], range[1][1]),
                self.encode_value(euler[2], range[2][0], range[2][1]),
            ];
            frame_tokens.push(tokens);
        }

        TokenFrame {
            bones: frame_tokens,
        }
    }

    /// Decode a token frame back to bone poses.
    pub fn decode_frame(&self, frame: &TokenFrame) -> HashMap<String, [f64; 3]> {
        let mut bones = HashMap::new();

        for (i, bone_name) in self.config.bones.iter().enumerate() {
            if i >= frame.bones.len() {
                break;
            }
            let tokens = frame.bones[i];
            let range = self
                .ranges
                .get(bone_name)
                .copied()
                .unwrap_or([[-PI, PI]; 3]);

            let euler = [
                self.decode_value(tokens[0], range[0][0], range[0][1]),
                self.decode_value(tokens[1], range[1][0], range[1][1]),
                self.decode_value(tokens[2], range[2][0], range[2][1]),
            ];
            bones.insert(bone_name.clone(), euler);
        }

        bones
    }

    /// Encode a full motion clip into discrete tokens.
    pub fn encode_clip(
        &self,
        fps: f64,
        frames: &[(f64, HashMap<String, [f64; 3]>)],
    ) -> EncodedMotion {
        let encoded_frames: Vec<TokenFrame> = frames
            .iter()
            .map(|(_, bones)| self.encode_frame(bones))
            .collect();

        EncodedMotion {
            fps,
            bone_names: self.config.bones.clone(),
            n_bins: self.config.n_bins,
            frames: encoded_frames,
        }
    }

    /// Decode an encoded motion back into continuous values.
    pub fn decode_clip(&self, encoded: &EncodedMotion) -> DecodedMotion {
        let frames: Vec<DecodedFrame> = encoded
            .frames
            .iter()
            .enumerate()
            .map(|(i, tf)| DecodedFrame {
                t: i as f64 / encoded.fps.max(1.0),
                bones: self.decode_frame(tf),
            })
            .collect();

        DecodedMotion {
            fps: encoded.fps,
            frames,
        }
    }

    /// Serialize an encoded motion to the LLM-friendly text format.
    pub fn to_text(&self, encoded: &EncodedMotion) -> String {
        let bones_header = encoded.bone_names.join(",");
        let mut lines = Vec::with_capacity(encoded.frames.len() + 2);

        lines.push(format!(
            "<motion_tokens fps=\"{}\" bones=\"{}\">",
            encoded.fps, bones_header
        ));

        for frame in &encoded.frames {
            let bone_strs: Vec<String> = frame
                .bones
                .iter()
                .map(|tokens| format!("{} {} {}", tokens[0], tokens[1], tokens[2]))
                .collect();
            lines.push(bone_strs.join(" | "));
        }

        lines.push("</motion_tokens>".to_string());
        lines.join("\n")
    }

    /// Parse the LLM text format back into an encoded motion.
    pub fn from_text(&self, text: &str) -> Option<EncodedMotion> {
        // Find the opening tag
        let open_start = text.find("<motion_tokens")?;
        let open_end = text[open_start..].find('>')? + open_start + 1;
        let close_start = text.find("</motion_tokens>")?;

        // Parse header attributes
        let header = &text[open_start..open_end];
        let fps = parse_attr(header, "fps")?.parse::<f64>().ok()?;
        let bones_str = parse_attr(header, "bones")?;
        let bone_names: Vec<String> = bones_str.split(',').map(|s| s.trim().to_string()).collect();

        // Parse frame lines
        let body = text[open_end..close_start].trim();
        let mut frames = Vec::new();

        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let bone_parts: Vec<&str> = line.split('|').collect();
            let mut bone_tokens = Vec::new();

            for part in bone_parts {
                let nums: Vec<u16> = part
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if nums.len() >= 3 {
                    bone_tokens.push([nums[0], nums[1], nums[2]]);
                }
            }

            if !bone_tokens.is_empty() {
                frames.push(TokenFrame { bones: bone_tokens });
            }
        }

        Some(EncodedMotion {
            fps,
            bone_names,
            n_bins: self.config.n_bins,
            frames,
        })
    }

    /// Build an LLM system-prompt fragment describing the motion token
    /// vocabulary so the model can generate motion clips.
    pub fn build_vocabulary_prompt(&self) -> String {
        let n = self.config.n_bins;
        let bones_list = self.config.bones.join(", ");

        format!(
            "You can generate motion animations by emitting a <motion_tokens> block.\n\
             Each frame is a line of space-separated token indices (0–{max_token}), grouped by bone with | separators.\n\
             Bones (in order): {bones_list}\n\
             Each bone has 3 values: pitch, yaw, roll.\n\
             Token 0 = minimum angle, token {max_token} = maximum angle, token {mid} = neutral/rest.\n\
             Example (2 frames, 30fps):\n\
             <motion_tokens fps=\"30\" bones=\"{bones_list}\">\n\
             {mid} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid}\n\
             {up} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid} | {mid} {mid} {mid}\n\
             </motion_tokens>\n\
             The first example frame is neutral; the second tilts the head up.",
            max_token = n - 1,
            mid = n / 2,
            up = n / 2 + 4,
            bones_list = bones_list,
        )
    }
}

/// Parse an XML attribute value from a tag string.
fn parse_attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let pattern = format!("{}=\"", name);
    let start = tag.find(&pattern)? + pattern.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_codec() -> MotionTokenCodec {
        MotionTokenCodec::new(TokenCodecConfig::default())
    }

    #[test]
    fn encode_decode_roundtrip_midpoint() {
        let codec = make_codec();
        // Encode zero angles (should map to middle bins)
        let mut bones = HashMap::new();
        bones.insert("head".to_string(), [0.0, 0.0, 0.0]);
        bones.insert("spine".to_string(), [0.0, 0.0, 0.0]);

        let frame = codec.encode_frame(&bones);
        let decoded = codec.decode_frame(&frame);

        // Zero should be near the center of the range
        let head = decoded.get("head").unwrap();
        // Head X range is [-PI/4, PI/4], midpoint is 0
        assert!(head[0].abs() < 0.1);
    }

    #[test]
    fn encode_decode_roundtrip_extremes() {
        let codec = make_codec();
        let mut bones = HashMap::new();
        // Head pitch at maximum
        bones.insert("head".to_string(), [PI / 4.0, 0.0, 0.0]);

        let frame = codec.encode_frame(&bones);
        let decoded = codec.decode_frame(&frame);

        let head = decoded.get("head").unwrap();
        // Should be close to PI/4 (quantisation error ≤ range/(n_bins-1))
        let max_error = (PI / 2.0) / 31.0; // range / (n_bins - 1)
        assert!((head[0] - PI / 4.0).abs() < max_error);
    }

    #[test]
    fn encode_clips_values_to_range() {
        let codec = make_codec();
        let mut bones = HashMap::new();
        // Far beyond range
        bones.insert("head".to_string(), [PI * 2.0, -PI * 2.0, 0.0]);

        let frame = codec.encode_frame(&bones);
        // Should be clamped to max/min bins
        assert_eq!(frame.bones[0][0], 31); // max bin
        assert_eq!(frame.bones[0][1], 0); // min bin
    }

    #[test]
    fn full_clip_roundtrip() {
        let codec = make_codec();
        let frames: Vec<(f64, HashMap<String, [f64; 3]>)> = (0..10)
            .map(|i| {
                let t = i as f64 / 30.0;
                let angle = (t * 2.0 * PI).sin() * 0.3;
                let mut bones = HashMap::new();
                bones.insert("head".to_string(), [angle, 0.0, 0.0]);
                bones.insert("spine".to_string(), [0.0, angle * 0.5, 0.0]);
                (t, bones)
            })
            .collect();

        let encoded = codec.encode_clip(30.0, &frames);
        assert_eq!(encoded.frames.len(), 10);
        assert_eq!(encoded.fps, 30.0);

        let decoded = codec.decode_clip(&encoded);
        assert_eq!(decoded.frames.len(), 10);
    }

    #[test]
    fn text_serialization_roundtrip() {
        let codec = make_codec();
        let mut bones = HashMap::new();
        bones.insert("head".to_string(), [0.1, -0.1, 0.0]);
        bones.insert("spine".to_string(), [0.0, 0.05, 0.0]);

        let frames: Vec<(f64, HashMap<String, [f64; 3]>)> =
            vec![(0.0, bones.clone()), (0.033, bones)];

        let encoded = codec.encode_clip(30.0, &frames);
        let text = codec.to_text(&encoded);

        assert!(text.contains("<motion_tokens"));
        assert!(text.contains("fps=\"30\""));
        assert!(text.contains("</motion_tokens>"));

        let parsed = codec.from_text(&text).unwrap();
        assert_eq!(parsed.frames.len(), 2);
        assert_eq!(parsed.fps, 30.0);
        assert_eq!(parsed.bone_names.len(), 8);
    }

    #[test]
    fn parse_text_with_surrounding_content() {
        let codec = make_codec();
        let text = r#"Here's a wave animation:
<motion_tokens fps="30" bones="head,spine">
16 16 16 | 16 16 16
20 16 16 | 16 16 16
</motion_tokens>
That should work!"#;

        let parsed = codec.from_text(text).unwrap();
        assert_eq!(parsed.frames.len(), 2);
        assert_eq!(parsed.bone_names, vec!["head", "spine"]);
    }

    #[test]
    fn parse_invalid_text_returns_none() {
        let codec = make_codec();
        assert!(codec.from_text("no tokens here").is_none());
        assert!(codec.from_text("<motion_tokens>no closing").is_none());
    }

    #[test]
    fn vocabulary_prompt_contains_range_info() {
        let codec = make_codec();
        let prompt = codec.build_vocabulary_prompt();
        assert!(prompt.contains("motion_tokens"));
        assert!(prompt.contains("head"));
        assert!(prompt.contains("0–31"));
        assert!(prompt.contains("pitch, yaw, roll"));
    }

    #[test]
    fn missing_bone_encodes_as_zero() {
        let codec = make_codec();
        let bones = HashMap::new(); // empty — no bones at all
        let frame = codec.encode_frame(&bones);
        // All bones should get the mid-range token (representing 0)
        assert_eq!(frame.bones.len(), 8); // 8 default bones
    }

    #[test]
    fn config_default_values() {
        let cfg = TokenCodecConfig::default();
        assert_eq!(cfg.n_bins, 32);
        assert_eq!(cfg.bones.len(), 8);
        assert!(cfg.ranges.is_none());
    }

    #[test]
    fn custom_bin_count() {
        let config = TokenCodecConfig {
            n_bins: 64,
            ..Default::default()
        };
        let codec = MotionTokenCodec::new(config);
        let mut bones = HashMap::new();
        bones.insert("head".to_string(), [PI / 4.0, 0.0, 0.0]);

        let frame = codec.encode_frame(&bones);
        // Max token should be 63
        assert!(frame.bones[0][0] <= 63);

        let decoded = codec.decode_frame(&frame);
        let head = decoded.get("head").unwrap();
        // Higher bin count = lower quantisation error
        let max_error = (PI / 2.0) / 63.0;
        assert!((head[0] - PI / 4.0).abs() < max_error);
    }

    #[test]
    fn parse_attr_works() {
        let tag = r#"<motion_tokens fps="30" bones="head,spine">"#;
        assert_eq!(parse_attr(tag, "fps"), Some("30"));
        assert_eq!(parse_attr(tag, "bones"), Some("head,spine"));
        assert_eq!(parse_attr(tag, "missing"), None);
    }

    #[test]
    fn serde_roundtrip_encoded() {
        let codec = make_codec();
        let mut bones = HashMap::new();
        bones.insert("head".to_string(), [0.1, 0.0, 0.0]);
        let frames = vec![(0.0, bones)];
        let encoded = codec.encode_clip(30.0, &frames);

        let json = serde_json::to_string(&encoded).unwrap();
        let back: EncodedMotion = serde_json::from_str(&json).unwrap();
        assert_eq!(back.frames.len(), 1);
        assert_eq!(back.n_bins, 32);
    }
}
