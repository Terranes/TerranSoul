//! Offline motion-clip smoothing (Chunk 14.13).
//!
//! Applies a configurable low-pass filter to recorded `LearnedMotion`
//! frame sequences to remove jitter from camera-captured clips.
//!
//! **Feature-flagged** behind `motion-research`. This is research code
//! — the pipeline works but the ML-accelerated variant (Hunyuan-Motion /
//! MimicMotion style) is deferred until real demand materialises.
//!
//! # Algorithm
//!
//! Two-pass Gaussian smoothing (forward + backward to eliminate phase
//! shift). Each bone channel (3 Euler angles per bone per frame) is
//! independently convolved with a 1-D Gaussian kernel of configurable
//! sigma. Edge frames are handled via reflection padding.
//!
//! This is the same approach used by motion-capture cleanup tools
//! (MotionBuilder, Blender's F-Curve smooth) — simple, effective, and
//! no ML runtime dependency.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;

// ─── Types ───────────────────────────────────────────────────────────────────

/// A single motion frame (mirrors the frontend `LearnedMotion.frames[]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionFrame {
    /// Time in seconds since clip start.
    pub t: f64,
    /// Bone name → Euler triple (radians).
    pub bones: HashMap<String, [f64; 3]>,
}

/// A motion clip to be smoothed (subset of the full `LearnedMotion`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionClip {
    pub fps: f64,
    pub frames: Vec<MotionFrame>,
}

/// Smoothing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmoothConfig {
    /// Gaussian kernel sigma in frames. Higher = more smoothing.
    /// Typical range: 1.0 (light) to 5.0 (heavy).
    /// Default: 2.0.
    pub sigma: f64,
    /// Kernel radius in frames (derived from sigma if `None`).
    /// When `None`, uses `ceil(3 * sigma)` which captures 99.7% of the
    /// Gaussian weight.
    pub radius: Option<usize>,
    /// Whether to preserve the first and last frame exactly (pin endpoints).
    /// Default: true.
    pub pin_endpoints: bool,
}

impl Default for SmoothConfig {
    fn default() -> Self {
        Self {
            sigma: 2.0,
            radius: None,
            pin_endpoints: true,
        }
    }
}

/// Result of smoothing: the cleaned clip + per-channel statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmoothResult {
    pub clip: MotionClip,
    /// Mean absolute displacement (radians) applied per bone channel.
    pub displacement_stats: HashMap<String, f64>,
}

// ─── Gaussian Kernel ─────────────────────────────────────────────────────────

/// Build a normalised 1-D Gaussian kernel of the given radius and sigma.
fn gaussian_kernel(sigma: f64, radius: usize) -> Vec<f64> {
    let size = 2 * radius + 1;
    let mut kernel = Vec::with_capacity(size);
    let two_sigma_sq = 2.0 * sigma * sigma;
    let norm = 1.0 / (two_sigma_sq * PI).sqrt();

    for i in 0..size {
        let x = i as f64 - radius as f64;
        kernel.push(norm * (-x * x / two_sigma_sq).exp());
    }

    // Normalise so weights sum to 1
    let sum: f64 = kernel.iter().sum();
    if sum > 0.0 {
        for w in &mut kernel {
            *w /= sum;
        }
    }
    kernel
}

// ─── 1-D Convolution (reflection-padded) ─────────────────────────────────────

/// Convolve a single channel with the Gaussian kernel.
/// Uses reflection padding at boundaries.
fn convolve_channel(data: &[f64], kernel: &[f64]) -> Vec<f64> {
    let n = data.len();
    if n == 0 {
        return vec![];
    }
    let radius = kernel.len() / 2;
    let mut out = Vec::with_capacity(n);

    for i in 0..n {
        let mut sum = 0.0;
        for (ki, &weight) in kernel.iter().enumerate() {
            let offset = ki as isize - radius as isize;
            let idx = i as isize + offset;
            // Reflection padding
            let clamped = if idx < 0 {
                (-idx) as usize
            } else if idx >= n as isize {
                2 * n - 2 - idx as usize
            } else {
                idx as usize
            };
            let clamped = clamped.min(n - 1);
            sum += data[clamped] * weight;
        }
        out.push(sum);
    }
    out
}

// ─── Two-pass (forward + backward) zero-phase filter ─────────────────────────

/// Apply zero-phase Gaussian filter (forward convolution + backward
/// convolution) to eliminate phase shift.
fn zero_phase_smooth(data: &[f64], kernel: &[f64]) -> Vec<f64> {
    if data.len() <= 1 {
        return data.to_vec();
    }
    // Forward pass
    let forward = convolve_channel(data, kernel);
    // Reverse
    let reversed: Vec<f64> = forward.into_iter().rev().collect();
    // Backward pass
    let backward = convolve_channel(&reversed, kernel);
    // Reverse back
    backward.into_iter().rev().collect()
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Smooth a motion clip using zero-phase Gaussian filtering.
///
/// Returns the smoothed clip and displacement statistics. The input
/// clip is not modified.
pub fn smooth_clip(clip: &MotionClip, config: &SmoothConfig) -> SmoothResult {
    let n = clip.frames.len();
    if n <= 2 {
        return SmoothResult {
            clip: clip.clone(),
            displacement_stats: HashMap::new(),
        };
    }

    let sigma = config.sigma.max(0.1);
    let radius = config
        .radius
        .unwrap_or_else(|| (3.0 * sigma).ceil() as usize);
    let kernel = gaussian_kernel(sigma, radius);

    // Collect all bone names across all frames
    let mut bone_names: Vec<String> = clip
        .frames
        .iter()
        .flat_map(|f| f.bones.keys().cloned())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    bone_names.sort();

    // For each bone, extract 3 channels, smooth, store back
    let mut smoothed_frames: Vec<MotionFrame> = clip.frames.clone();
    let mut displacement_stats: HashMap<String, f64> = HashMap::new();

    for bone in &bone_names {
        // Extract channels
        let mut channels: [Vec<f64>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for frame in &clip.frames {
            let euler = frame.bones.get(bone).copied().unwrap_or([0.0; 3]);
            channels[0].push(euler[0]);
            channels[1].push(euler[1]);
            channels[2].push(euler[2]);
        }

        // Smooth each channel
        let mut total_displacement = 0.0;
        for (axis, channel) in channels.iter().enumerate() {
            let smoothed = zero_phase_smooth(channel, &kernel);

            // Apply back to frames
            for (i, frame) in smoothed_frames.iter_mut().enumerate() {
                let entry = frame.bones.entry(bone.clone()).or_insert([0.0; 3]);

                // Pin endpoints if configured
                if config.pin_endpoints && (i == 0 || i == n - 1) {
                    // Keep original
                } else {
                    let displacement = (smoothed[i] - entry[axis]).abs();
                    total_displacement += displacement;
                    entry[axis] = smoothed[i];
                }
            }
        }

        let avg_displacement = if n > 2 {
            total_displacement / ((n - 2) * 3) as f64
        } else {
            0.0
        };
        displacement_stats.insert(bone.clone(), avg_displacement);
    }

    SmoothResult {
        clip: MotionClip {
            fps: clip.fps,
            frames: smoothed_frames,
        },
        displacement_stats,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_noisy_clip(n: usize) -> MotionClip {
        let mut frames = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / 30.0;
            // Smooth sine + noise
            let base = (t * 2.0 * PI).sin();
            let noise = if i % 2 == 0 { 0.1 } else { -0.1 };
            let mut bones = HashMap::new();
            bones.insert("head".to_string(), [base + noise, 0.0, 0.0]);
            bones.insert("spine".to_string(), [0.0, base * 0.5 + noise * 0.5, 0.0]);
            frames.push(MotionFrame { t, bones });
        }
        MotionClip { fps: 30.0, frames }
    }

    #[test]
    fn gaussian_kernel_sums_to_one() {
        let k = gaussian_kernel(2.0, 6);
        let sum: f64 = k.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn gaussian_kernel_is_symmetric() {
        let k = gaussian_kernel(1.5, 4);
        let n = k.len();
        for i in 0..n / 2 {
            assert!((k[i] - k[n - 1 - i]).abs() < 1e-12);
        }
    }

    #[test]
    fn convolve_identity_for_constant() {
        let data = vec![5.0; 10];
        let kernel = gaussian_kernel(2.0, 6);
        let out = convolve_channel(&data, &kernel);
        for v in &out {
            assert!((v - 5.0).abs() < 1e-10);
        }
    }

    #[test]
    fn smooth_reduces_noise() {
        let clip = make_noisy_clip(60);
        let result = smooth_clip(&clip, &SmoothConfig::default());

        // Compute variance of the head channel before and after
        let before: Vec<f64> = clip.frames.iter().map(|f| f.bones["head"][0]).collect();
        let after: Vec<f64> = result
            .clip
            .frames
            .iter()
            .map(|f| f.bones["head"][0])
            .collect();

        let var_before = variance(&before);
        let var_after = variance(&after);

        // After smoothing, the high-frequency noise should be reduced
        // but the overall signal preserved (variance shouldn't drop too much)
        assert!(var_after < var_before, "smoothing should reduce variance");
        assert!(var_after > var_before * 0.3, "shouldn't over-smooth");
    }

    #[test]
    fn pin_endpoints_preserves_first_last() {
        let clip = make_noisy_clip(30);
        let config = SmoothConfig {
            pin_endpoints: true,
            ..Default::default()
        };
        let result = smooth_clip(&clip, &config);

        assert_eq!(
            result.clip.frames[0].bones["head"],
            clip.frames[0].bones["head"]
        );
        assert_eq!(
            result.clip.frames[29].bones["head"],
            clip.frames[29].bones["head"]
        );
    }

    #[test]
    fn unpin_endpoints_may_change_them() {
        let clip = make_noisy_clip(30);
        let config = SmoothConfig {
            pin_endpoints: false,
            sigma: 3.0,
            radius: None,
        };
        let result = smooth_clip(&clip, &config);
        // With heavy smoothing and unpinned endpoints, they can differ
        // (just verify it runs without panic)
        assert_eq!(result.clip.frames.len(), 30);
    }

    #[test]
    fn empty_clip_passthrough() {
        let clip = MotionClip {
            fps: 30.0,
            frames: vec![],
        };
        let result = smooth_clip(&clip, &SmoothConfig::default());
        assert!(result.clip.frames.is_empty());
    }

    #[test]
    fn two_frame_clip_passthrough() {
        let clip = MotionClip {
            fps: 30.0,
            frames: vec![
                MotionFrame {
                    t: 0.0,
                    bones: HashMap::new(),
                },
                MotionFrame {
                    t: 0.033,
                    bones: HashMap::new(),
                },
            ],
        };
        let result = smooth_clip(&clip, &SmoothConfig::default());
        assert_eq!(result.clip.frames.len(), 2);
    }

    #[test]
    fn displacement_stats_populated() {
        let clip = make_noisy_clip(30);
        let result = smooth_clip(&clip, &SmoothConfig::default());
        assert!(result.displacement_stats.contains_key("head"));
        assert!(result.displacement_stats.contains_key("spine"));
        assert!(*result.displacement_stats.get("head").unwrap() > 0.0);
    }

    #[test]
    fn higher_sigma_means_more_smoothing() {
        let clip = make_noisy_clip(60);
        let light = smooth_clip(
            &clip,
            &SmoothConfig {
                sigma: 1.0,
                ..Default::default()
            },
        );
        let heavy = smooth_clip(
            &clip,
            &SmoothConfig {
                sigma: 4.0,
                ..Default::default()
            },
        );

        let light_disp = light.displacement_stats["head"];
        let heavy_disp = heavy.displacement_stats["head"];
        assert!(heavy_disp > light_disp);
    }

    #[test]
    fn config_default_values() {
        let cfg = SmoothConfig::default();
        assert!((cfg.sigma - 2.0).abs() < f64::EPSILON);
        assert!(cfg.pin_endpoints);
        assert!(cfg.radius.is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let clip = make_noisy_clip(5);
        let json = serde_json::to_string(&clip).unwrap();
        let back: MotionClip = serde_json::from_str(&json).unwrap();
        assert_eq!(back.frames.len(), 5);
        assert!((back.fps - 30.0).abs() < f64::EPSILON);
    }

    fn variance(data: &[f64]) -> f64 {
        let n = data.len() as f64;
        let mean = data.iter().sum::<f64>() / n;
        data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n
    }
}
