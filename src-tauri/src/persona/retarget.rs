//! Full-body retarget from sparse keypoints (Chunk 14.14).
//!
//! Reconstructs a dense VRM bone pose from 33 MediaPipe BlazePose
//! landmarks using geometric IK and anatomical constraints. This is
//! the Rust-side complement to the frontend's `pose-mirror.ts` — it
//! enables offline batch retargeting (e.g. processing recorded clips)
//! without needing the browser.
//!
//! **Feature-flagged** behind `motion-research`. Research chunk — the
//! full MoMask / SMPL-X neural retarget is deferred until an ONNX
//! runtime dependency is justified. This module provides the geometric
//! foundation that a future ML pass would refine.
//!
//! # Approach
//!
//! 1. Convert 33 BlazePose landmarks to a skeleton hierarchy.
//! 2. Compute bone orientations via two-bone IK (shoulder→elbow→wrist,
//!    hip→knee→ankle) and single-bone look-at (head, spine, hips).
//! 3. Apply anatomical joint limits to prevent impossible poses.
//! 4. Output as `VrmBonePose` (Euler triples per named bone).

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

// ─── Types ───────────────────────────────────────────────────────────────────

/// A single 3D landmark from MediaPipe BlazePose (33 total).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Landmark {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Visibility confidence [0, 1]. Landmarks below threshold are ignored.
    pub visibility: f64,
}

/// Euler angles (radians) in XYZ order.
pub type EulerTriple = [f64; 3];

/// Full VRM bone pose — 17 bones (extended from the frontend's 11).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VrmBonePose {
    pub hips: Option<EulerTriple>,
    pub spine: Option<EulerTriple>,
    pub chest: Option<EulerTriple>,
    pub neck: Option<EulerTriple>,
    pub head: Option<EulerTriple>,
    pub left_shoulder: Option<EulerTriple>,
    pub right_shoulder: Option<EulerTriple>,
    pub left_upper_arm: Option<EulerTriple>,
    pub right_upper_arm: Option<EulerTriple>,
    pub left_lower_arm: Option<EulerTriple>,
    pub right_lower_arm: Option<EulerTriple>,
    pub left_upper_leg: Option<EulerTriple>,
    pub right_upper_leg: Option<EulerTriple>,
    pub left_lower_leg: Option<EulerTriple>,
    pub right_lower_leg: Option<EulerTriple>,
    pub left_foot: Option<EulerTriple>,
    pub right_foot: Option<EulerTriple>,
}

/// Retargeting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetargetConfig {
    /// Minimum visibility threshold for a landmark to be used.
    /// Default: 0.5.
    pub visibility_threshold: f64,
    /// Whether to apply anatomical joint limits.
    /// Default: true.
    pub apply_joint_limits: bool,
    /// Scale factor for depth (Z). MediaPipe Z is often noisy.
    /// Default: 0.5 (halve Z influence).
    pub depth_scale: f64,
}

impl Default for RetargetConfig {
    fn default() -> Self {
        Self {
            visibility_threshold: 0.5,
            apply_joint_limits: true,
            depth_scale: 0.5,
        }
    }
}

/// BlazePose landmark indices (33 total).
#[allow(dead_code)]
mod idx {
    pub const NOSE: usize = 0;
    pub const LEFT_EYE_INNER: usize = 1;
    pub const LEFT_EYE: usize = 2;
    pub const LEFT_EYE_OUTER: usize = 3;
    pub const RIGHT_EYE_INNER: usize = 4;
    pub const RIGHT_EYE: usize = 5;
    pub const RIGHT_EYE_OUTER: usize = 6;
    pub const LEFT_EAR: usize = 7;
    pub const RIGHT_EAR: usize = 8;
    pub const MOUTH_LEFT: usize = 9;
    pub const MOUTH_RIGHT: usize = 10;
    pub const LEFT_SHOULDER: usize = 11;
    pub const RIGHT_SHOULDER: usize = 12;
    pub const LEFT_ELBOW: usize = 13;
    pub const RIGHT_ELBOW: usize = 14;
    pub const LEFT_WRIST: usize = 15;
    pub const RIGHT_WRIST: usize = 16;
    pub const LEFT_PINKY: usize = 17;
    pub const RIGHT_PINKY: usize = 18;
    pub const LEFT_INDEX: usize = 19;
    pub const RIGHT_INDEX: usize = 20;
    pub const LEFT_THUMB: usize = 21;
    pub const RIGHT_THUMB: usize = 22;
    pub const LEFT_HIP: usize = 23;
    pub const RIGHT_HIP: usize = 24;
    pub const LEFT_KNEE: usize = 25;
    pub const RIGHT_KNEE: usize = 26;
    pub const LEFT_ANKLE: usize = 27;
    pub const RIGHT_ANKLE: usize = 28;
    pub const LEFT_HEEL: usize = 29;
    pub const RIGHT_HEEL: usize = 30;
    pub const LEFT_FOOT_INDEX: usize = 31;
    pub const RIGHT_FOOT_INDEX: usize = 32;
}

// ─── Vector Math ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

#[allow(dead_code)]
impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn from_landmark(lm: &Landmark, depth_scale: f64) -> Self {
        Self {
            x: lm.x,
            y: lm.y,
            z: lm.z * depth_scale,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    fn scale(self, s: f64) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }

    fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    fn normalize(self) -> Self {
        let len = self.length();
        if len < 1e-10 {
            Self::new(0.0, 1.0, 0.0)
        } else {
            self.scale(1.0 / len)
        }
    }

    fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn midpoint(self, other: Self) -> Self {
        self.add(other).scale(0.5)
    }
}

// ─── Geometric Helpers ───────────────────────────────────────────────────────

/// Compute pitch/yaw from a direction vector (Y-up convention).
fn direction_to_euler(dir: Vec3) -> EulerTriple {
    let d = dir.normalize();
    let pitch = (-d.y).asin(); // rotation around X
    let yaw = d.x.atan2(d.z); // rotation around Y
    [pitch, yaw, 0.0]
}

/// Two-bone IK: given shoulder→elbow→wrist positions, compute upper and
/// lower arm Euler angles.
fn two_bone_ik(shoulder: Vec3, elbow: Vec3, wrist: Vec3) -> (EulerTriple, EulerTriple) {
    let upper_dir = elbow.sub(shoulder).normalize();
    let lower_dir = wrist.sub(elbow).normalize();

    let upper_euler = direction_to_euler(upper_dir);
    let lower_euler = direction_to_euler(lower_dir);

    (upper_euler, lower_euler)
}

/// Clamp an angle to the given range.
fn clamp_angle(angle: f64, min: f64, max: f64) -> f64 {
    angle.clamp(min, max)
}

/// Apply anatomical joint limits to a bone's Euler angles.
fn apply_limits(euler: EulerTriple, limits: &JointLimits) -> EulerTriple {
    [
        clamp_angle(euler[0], limits.pitch_min, limits.pitch_max),
        clamp_angle(euler[1], limits.yaw_min, limits.yaw_max),
        clamp_angle(euler[2], limits.roll_min, limits.roll_max),
    ]
}

struct JointLimits {
    pitch_min: f64,
    pitch_max: f64,
    yaw_min: f64,
    yaw_max: f64,
    roll_min: f64,
    roll_max: f64,
}

/// Anatomical limits for various bones.
fn head_limits() -> JointLimits {
    JointLimits {
        pitch_min: -PI / 4.0,
        pitch_max: PI / 4.0,
        yaw_min: -PI / 3.0,
        yaw_max: PI / 3.0,
        roll_min: -PI / 6.0,
        roll_max: PI / 6.0,
    }
}

fn spine_limits() -> JointLimits {
    JointLimits {
        pitch_min: -PI / 6.0,
        pitch_max: PI / 4.0,
        yaw_min: -PI / 6.0,
        yaw_max: PI / 6.0,
        roll_min: -PI / 8.0,
        roll_max: PI / 8.0,
    }
}

fn upper_arm_limits() -> JointLimits {
    JointLimits {
        pitch_min: -PI,
        pitch_max: PI,
        yaw_min: -PI,
        yaw_max: PI,
        roll_min: -PI / 2.0,
        roll_max: PI / 2.0,
    }
}

fn lower_arm_limits() -> JointLimits {
    JointLimits {
        pitch_min: -PI * 0.9,
        pitch_max: 0.1,
        yaw_min: -PI / 4.0,
        yaw_max: PI / 4.0,
        roll_min: -PI / 2.0,
        roll_max: PI / 2.0,
    }
}

fn upper_leg_limits() -> JointLimits {
    JointLimits {
        pitch_min: -PI / 2.0,
        pitch_max: PI / 3.0,
        yaw_min: -PI / 4.0,
        yaw_max: PI / 4.0,
        roll_min: -PI / 6.0,
        roll_max: PI / 6.0,
    }
}

fn lower_leg_limits() -> JointLimits {
    JointLimits {
        pitch_min: 0.0,
        pitch_max: PI * 0.8,
        yaw_min: -0.1,
        yaw_max: 0.1,
        roll_min: -0.1,
        roll_max: 0.1,
    }
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Retarget 33 BlazePose landmarks to a full VRM bone pose.
///
/// Returns `None` if insufficient landmarks are visible (need at least
/// both shoulders and both hips visible to establish torso reference).
pub fn retarget_pose(landmarks: &[Landmark], config: &RetargetConfig) -> Option<VrmBonePose> {
    if landmarks.len() < 33 {
        return None;
    }

    let thresh = config.visibility_threshold;
    let ds = config.depth_scale;

    // Check minimum: shoulders + hips
    let ls = &landmarks[idx::LEFT_SHOULDER];
    let rs = &landmarks[idx::RIGHT_SHOULDER];
    let lh = &landmarks[idx::LEFT_HIP];
    let rh = &landmarks[idx::RIGHT_HIP];

    if ls.visibility < thresh || rs.visibility < thresh {
        return None;
    }
    if lh.visibility < thresh || rh.visibility < thresh {
        return None;
    }

    let left_shoulder = Vec3::from_landmark(ls, ds);
    let right_shoulder = Vec3::from_landmark(rs, ds);
    let left_hip = Vec3::from_landmark(lh, ds);
    let right_hip = Vec3::from_landmark(rh, ds);

    let mid_shoulder = left_shoulder.midpoint(right_shoulder);
    let mid_hip = left_hip.midpoint(right_hip);

    let mut pose = VrmBonePose::default();

    // Spine: direction from hips to shoulders
    let spine_dir = mid_shoulder.sub(mid_hip).normalize();
    let spine_euler = direction_to_euler(spine_dir);
    pose.spine = Some(if config.apply_joint_limits {
        apply_limits(spine_euler, &spine_limits())
    } else {
        spine_euler
    });

    // Hips: lateral tilt from hip positions
    let hip_dir = right_hip.sub(left_hip).normalize();
    let hip_roll = hip_dir.y.asin();
    pose.hips = Some([0.0, 0.0, hip_roll]);

    // Chest: between spine and shoulders
    let chest_euler = [spine_euler[0] * 0.5, spine_euler[1] * 0.5, 0.0];
    pose.chest = Some(chest_euler);

    // Head: if nose visible
    let nose = &landmarks[idx::NOSE];
    if nose.visibility >= thresh {
        let nose_pos = Vec3::from_landmark(nose, ds);
        let head_dir = nose_pos.sub(mid_shoulder).normalize();
        let head_euler = direction_to_euler(head_dir);
        pose.head = Some(if config.apply_joint_limits {
            apply_limits(head_euler, &head_limits())
        } else {
            head_euler
        });

        // Neck: halfway between spine and head
        let neck_euler = [
            (spine_euler[0] + head_euler[0]) * 0.5,
            (spine_euler[1] + head_euler[1]) * 0.5,
            0.0,
        ];
        pose.neck = Some(neck_euler);
    }

    // Arms
    let le = &landmarks[idx::LEFT_ELBOW];
    let lw = &landmarks[idx::LEFT_WRIST];
    if le.visibility >= thresh && lw.visibility >= thresh {
        let elbow = Vec3::from_landmark(le, ds);
        let wrist = Vec3::from_landmark(lw, ds);
        let (upper, lower) = two_bone_ik(left_shoulder, elbow, wrist);

        pose.left_upper_arm = Some(if config.apply_joint_limits {
            apply_limits(upper, &upper_arm_limits())
        } else {
            upper
        });
        pose.left_lower_arm = Some(if config.apply_joint_limits {
            apply_limits(lower, &lower_arm_limits())
        } else {
            lower
        });

        // Shoulder shrug
        let shrug = (left_shoulder.y - mid_shoulder.y).atan2(
            left_shoulder.sub(mid_shoulder).length().max(0.01),
        );
        pose.left_shoulder = Some([0.0, 0.0, shrug]);
    }

    let re = &landmarks[idx::RIGHT_ELBOW];
    let rw = &landmarks[idx::RIGHT_WRIST];
    if re.visibility >= thresh && rw.visibility >= thresh {
        let elbow = Vec3::from_landmark(re, ds);
        let wrist = Vec3::from_landmark(rw, ds);
        let (upper, lower) = two_bone_ik(right_shoulder, elbow, wrist);

        pose.right_upper_arm = Some(if config.apply_joint_limits {
            apply_limits(upper, &upper_arm_limits())
        } else {
            upper
        });
        pose.right_lower_arm = Some(if config.apply_joint_limits {
            apply_limits(lower, &lower_arm_limits())
        } else {
            lower
        });

        let shrug = (right_shoulder.y - mid_shoulder.y).atan2(
            right_shoulder.sub(mid_shoulder).length().max(0.01),
        );
        pose.right_shoulder = Some([0.0, 0.0, shrug]);
    }

    // Legs
    let lk = &landmarks[idx::LEFT_KNEE];
    let la = &landmarks[idx::LEFT_ANKLE];
    if lk.visibility >= thresh && la.visibility >= thresh {
        let knee = Vec3::from_landmark(lk, ds);
        let ankle = Vec3::from_landmark(la, ds);
        let (upper, lower) = two_bone_ik(left_hip, knee, ankle);

        pose.left_upper_leg = Some(if config.apply_joint_limits {
            apply_limits(upper, &upper_leg_limits())
        } else {
            upper
        });
        pose.left_lower_leg = Some(if config.apply_joint_limits {
            apply_limits(lower, &lower_leg_limits())
        } else {
            lower
        });

        // Foot
        let lfi = &landmarks[idx::LEFT_FOOT_INDEX];
        if lfi.visibility >= thresh {
            let foot_tip = Vec3::from_landmark(lfi, ds);
            let foot_dir = foot_tip.sub(ankle).normalize();
            pose.left_foot = Some(direction_to_euler(foot_dir));
        }
    }

    let rk = &landmarks[idx::RIGHT_KNEE];
    let ra = &landmarks[idx::RIGHT_ANKLE];
    if rk.visibility >= thresh && ra.visibility >= thresh {
        let knee = Vec3::from_landmark(rk, ds);
        let ankle = Vec3::from_landmark(ra, ds);
        let (upper, lower) = two_bone_ik(right_hip, knee, ankle);

        pose.right_upper_leg = Some(if config.apply_joint_limits {
            apply_limits(upper, &upper_leg_limits())
        } else {
            upper
        });
        pose.right_lower_leg = Some(if config.apply_joint_limits {
            apply_limits(lower, &lower_leg_limits())
        } else {
            lower
        });

        let rfi = &landmarks[idx::RIGHT_FOOT_INDEX];
        if rfi.visibility >= thresh {
            let foot_tip = Vec3::from_landmark(rfi, ds);
            let foot_dir = foot_tip.sub(ankle).normalize();
            pose.right_foot = Some(direction_to_euler(foot_dir));
        }
    }

    Some(pose)
}

/// Batch-retarget a sequence of landmark frames into a motion clip.
pub fn retarget_sequence(
    frames: &[(f64, Vec<Landmark>)],
    config: &RetargetConfig,
) -> Vec<(f64, Option<VrmBonePose>)> {
    frames
        .iter()
        .map(|(t, lms)| (*t, retarget_pose(lms, config)))
        .collect()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tpose_landmarks() -> Vec<Landmark> {
        // Minimal T-pose: shoulders level, arms out, legs down
        let mut lms = vec![
            Landmark {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                visibility: 0.0,
            };
            33
        ];

        // Set key landmarks with high visibility
        let vis = 0.9;

        // Nose (top center)
        lms[idx::NOSE] = Landmark {
            x: 0.5,
            y: 0.15,
            z: 0.0,
            visibility: vis,
        };
        // Shoulders
        lms[idx::LEFT_SHOULDER] = Landmark {
            x: 0.35,
            y: 0.35,
            z: 0.0,
            visibility: vis,
        };
        lms[idx::RIGHT_SHOULDER] = Landmark {
            x: 0.65,
            y: 0.35,
            z: 0.0,
            visibility: vis,
        };
        // Elbows (arms out to sides)
        lms[idx::LEFT_ELBOW] = Landmark {
            x: 0.2,
            y: 0.35,
            z: 0.0,
            visibility: vis,
        };
        lms[idx::RIGHT_ELBOW] = Landmark {
            x: 0.8,
            y: 0.35,
            z: 0.0,
            visibility: vis,
        };
        // Wrists
        lms[idx::LEFT_WRIST] = Landmark {
            x: 0.05,
            y: 0.35,
            z: 0.0,
            visibility: vis,
        };
        lms[idx::RIGHT_WRIST] = Landmark {
            x: 0.95,
            y: 0.35,
            z: 0.0,
            visibility: vis,
        };
        // Hips
        lms[idx::LEFT_HIP] = Landmark {
            x: 0.4,
            y: 0.55,
            z: 0.0,
            visibility: vis,
        };
        lms[idx::RIGHT_HIP] = Landmark {
            x: 0.6,
            y: 0.55,
            z: 0.0,
            visibility: vis,
        };
        // Knees
        lms[idx::LEFT_KNEE] = Landmark {
            x: 0.4,
            y: 0.75,
            z: 0.0,
            visibility: vis,
        };
        lms[idx::RIGHT_KNEE] = Landmark {
            x: 0.6,
            y: 0.75,
            z: 0.0,
            visibility: vis,
        };
        // Ankles
        lms[idx::LEFT_ANKLE] = Landmark {
            x: 0.4,
            y: 0.95,
            z: 0.0,
            visibility: vis,
        };
        lms[idx::RIGHT_ANKLE] = Landmark {
            x: 0.6,
            y: 0.95,
            z: 0.0,
            visibility: vis,
        };
        // Feet
        lms[idx::LEFT_FOOT_INDEX] = Landmark {
            x: 0.4,
            y: 0.98,
            z: -0.05,
            visibility: vis,
        };
        lms[idx::RIGHT_FOOT_INDEX] = Landmark {
            x: 0.6,
            y: 0.98,
            z: -0.05,
            visibility: vis,
        };

        lms
    }

    #[test]
    fn retarget_tpose_succeeds() {
        let lms = make_tpose_landmarks();
        let pose = retarget_pose(&lms, &RetargetConfig::default());
        assert!(pose.is_some());
        let pose = pose.unwrap();
        assert!(pose.spine.is_some());
        assert!(pose.head.is_some());
        assert!(pose.left_upper_arm.is_some());
        assert!(pose.right_upper_arm.is_some());
        assert!(pose.left_upper_leg.is_some());
        assert!(pose.right_upper_leg.is_some());
    }

    #[test]
    fn retarget_insufficient_landmarks_returns_none() {
        let lms = vec![
            Landmark {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                visibility: 0.0,
            };
            10
        ];
        assert!(retarget_pose(&lms, &RetargetConfig::default()).is_none());
    }

    #[test]
    fn retarget_low_visibility_returns_none() {
        let mut lms = make_tpose_landmarks();
        // Zero out shoulder visibility
        lms[idx::LEFT_SHOULDER].visibility = 0.1;
        assert!(retarget_pose(&lms, &RetargetConfig::default()).is_none());
    }

    #[test]
    fn joint_limits_clamp_angles() {
        let extreme = [PI, PI, PI];
        let limited = apply_limits(extreme, &head_limits());
        assert!(limited[0] <= PI / 4.0 + f64::EPSILON);
        assert!(limited[1] <= PI / 3.0 + f64::EPSILON);
        assert!(limited[2] <= PI / 6.0 + f64::EPSILON);
    }

    #[test]
    fn disable_joint_limits() {
        let mut lms = make_tpose_landmarks();
        // Put arm in extreme position
        lms[idx::LEFT_ELBOW] = Landmark {
            x: 0.35,
            y: 0.1,
            z: -0.5,
            visibility: 0.9,
        };
        let config = RetargetConfig {
            apply_joint_limits: false,
            ..Default::default()
        };
        let pose = retarget_pose(&lms, &config).unwrap();
        assert!(pose.left_upper_arm.is_some());
    }

    #[test]
    fn batch_retarget_sequence() {
        let lms = make_tpose_landmarks();
        let frames: Vec<(f64, Vec<Landmark>)> =
            (0..5).map(|i| (i as f64 / 30.0, lms.clone())).collect();

        let result = retarget_sequence(&frames, &RetargetConfig::default());
        assert_eq!(result.len(), 5);
        for (_, pose) in &result {
            assert!(pose.is_some());
        }
    }

    #[test]
    fn partial_visibility_gives_partial_pose() {
        let mut lms = make_tpose_landmarks();
        // Hide legs
        lms[idx::LEFT_KNEE].visibility = 0.1;
        lms[idx::RIGHT_KNEE].visibility = 0.1;
        lms[idx::LEFT_ANKLE].visibility = 0.1;
        lms[idx::RIGHT_ANKLE].visibility = 0.1;

        let pose = retarget_pose(&lms, &RetargetConfig::default()).unwrap();
        assert!(pose.spine.is_some()); // torso works
        assert!(pose.left_upper_arm.is_some()); // arms work
        assert!(pose.left_upper_leg.is_none()); // legs missing
        assert!(pose.right_upper_leg.is_none());
    }

    #[test]
    fn config_defaults() {
        let cfg = RetargetConfig::default();
        assert!((cfg.visibility_threshold - 0.5).abs() < f64::EPSILON);
        assert!(cfg.apply_joint_limits);
        assert!((cfg.depth_scale - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn direction_to_euler_straight_down() {
        // Pointing straight down (Y+) = pitch of -PI/2
        let euler = direction_to_euler(Vec3::new(0.0, 1.0, 0.0));
        assert!((euler[0] - (-PI / 2.0)).abs() < 1e-10);
    }

    #[test]
    fn direction_to_euler_forward() {
        // Pointing forward (Z+) = pitch 0, yaw 0
        let euler = direction_to_euler(Vec3::new(0.0, 0.0, 1.0));
        assert!(euler[0].abs() < 1e-10);
        assert!(euler[1].abs() < 1e-10);
    }

    #[test]
    fn serde_roundtrip_pose() {
        let lms = make_tpose_landmarks();
        let pose = retarget_pose(&lms, &RetargetConfig::default()).unwrap();
        let json = serde_json::to_string(&pose).unwrap();
        let back: VrmBonePose = serde_json::from_str(&json).unwrap();
        assert_eq!(pose, back);
    }

    #[test]
    fn serde_roundtrip_config() {
        let cfg = RetargetConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: RetargetConfig = serde_json::from_str(&json).unwrap();
        assert!((back.sigma_equivalent() - cfg.sigma_equivalent()).abs() < f64::EPSILON);
    }

    impl RetargetConfig {
        fn sigma_equivalent(&self) -> f64 {
            self.visibility_threshold + self.depth_scale
        }
    }
}
