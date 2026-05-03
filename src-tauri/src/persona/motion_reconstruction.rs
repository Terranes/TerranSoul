//! Motion reconstruction backend seam for saved landmark clips.
//!
//! The first backend is intentionally just a thin `geometric` wrapper around
//! the existing Rust retargeter. This gives future MotionBERT/MMPose sidecars a
//! stable interface to implement without changing the live camera mirror path.

use serde::{Deserialize, Serialize};

use super::retarget::{retarget_pose, Landmark, RetargetConfig, VrmBonePose};

/// Supported reconstruction backend identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionReconstructionBackendId {
    Geometric,
}

impl MotionReconstructionBackendId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Geometric => "geometric",
        }
    }
}

/// A saved 33-landmark frame. These frames come from persisted clips, not live camera input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedLandmarkFrame {
    pub timestamp_secs: f64,
    pub landmarks: Vec<Landmark>,
}

/// Runtime metadata for a reconstruction backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionReconstructionBackendInfo {
    pub id: MotionReconstructionBackendId,
    pub display_name: String,
    pub bundled: bool,
    pub requires_sidecar: bool,
    pub accepts_live_camera: bool,
}

/// User/config-selected reconstruction settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionReconstructionConfig {
    pub backend: MotionReconstructionBackendId,
    pub retarget: RetargetConfig,
}

impl Default for MotionReconstructionConfig {
    fn default() -> Self {
        Self {
            backend: MotionReconstructionBackendId::Geometric,
            retarget: RetargetConfig::default(),
        }
    }
}

/// One reconstructed pose frame with quality metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionReconstructedFrame {
    pub timestamp_secs: f64,
    pub pose: Option<VrmBonePose>,
    pub confidence: f64,
    pub warnings: Vec<String>,
}

/// Full reconstruction result for a saved landmark clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionReconstructionResult {
    pub backend: MotionReconstructionBackendId,
    pub frames: Vec<MotionReconstructedFrame>,
    pub warnings: Vec<String>,
}

/// Backend boundary for future non-geometric reconstruction adapters.
pub trait MotionReconstructionBackend {
    fn info(&self) -> MotionReconstructionBackendInfo;

    fn reconstruct(
        &self,
        frames: &[SavedLandmarkFrame],
        config: &RetargetConfig,
    ) -> MotionReconstructionResult;
}

/// No-op baseline backend: saved landmarks -> existing geometric retargeter.
#[derive(Debug, Default, Clone, Copy)]
pub struct GeometricMotionReconstructionBackend;

impl MotionReconstructionBackend for GeometricMotionReconstructionBackend {
    fn info(&self) -> MotionReconstructionBackendInfo {
        MotionReconstructionBackendInfo {
            id: MotionReconstructionBackendId::Geometric,
            display_name: "Geometric baseline".to_string(),
            bundled: true,
            requires_sidecar: false,
            accepts_live_camera: false,
        }
    }

    fn reconstruct(
        &self,
        frames: &[SavedLandmarkFrame],
        config: &RetargetConfig,
    ) -> MotionReconstructionResult {
        let reconstructed_frames = frames
            .iter()
            .map(|frame| reconstruct_geometric_frame(frame, config))
            .collect();

        MotionReconstructionResult {
            backend: MotionReconstructionBackendId::Geometric,
            frames: reconstructed_frames,
            warnings: Vec::new(),
        }
    }
}

pub fn available_reconstruction_backends() -> Vec<MotionReconstructionBackendInfo> {
    vec![GeometricMotionReconstructionBackend.info()]
}

pub fn reconstruct_saved_landmark_clip(
    frames: &[SavedLandmarkFrame],
    config: &MotionReconstructionConfig,
) -> MotionReconstructionResult {
    match config.backend {
        MotionReconstructionBackendId::Geometric => {
            GeometricMotionReconstructionBackend.reconstruct(frames, &config.retarget)
        }
    }
}

fn reconstruct_geometric_frame(
    frame: &SavedLandmarkFrame,
    config: &RetargetConfig,
) -> MotionReconstructedFrame {
    let pose = retarget_pose(&frame.landmarks, config);
    let confidence = pose.as_ref().map(pose_completeness).unwrap_or(0.0);
    let warnings = if pose.is_some() {
        Vec::new()
    } else {
        vec!["insufficient_visible_landmarks".to_string()]
    };

    MotionReconstructedFrame {
        timestamp_secs: frame.timestamp_secs,
        pose,
        confidence,
        warnings,
    }
}

fn pose_completeness(pose: &VrmBonePose) -> f64 {
    let present = [
        pose.hips,
        pose.spine,
        pose.chest,
        pose.neck,
        pose.head,
        pose.left_shoulder,
        pose.right_shoulder,
        pose.left_upper_arm,
        pose.right_upper_arm,
        pose.left_lower_arm,
        pose.right_lower_arm,
        pose.left_upper_leg,
        pose.right_upper_leg,
        pose.left_lower_leg,
        pose.right_lower_leg,
        pose.left_foot,
        pose.right_foot,
    ]
    .iter()
    .filter(|bone_pose| bone_pose.is_some())
    .count();

    present as f64 / 17.0
}

/// Synthetic saved-clip fixtures for exercising the backend seam without camera frames.
pub mod static_landmark_fixtures {
    use super::{Landmark, SavedLandmarkFrame};

    const LANDMARK_COUNT: usize = 33;
    const VISIBILITY: f64 = 0.9;

    pub fn t_pose_frame(timestamp_secs: f64) -> SavedLandmarkFrame {
        SavedLandmarkFrame {
            timestamp_secs,
            landmarks: t_pose_landmarks(),
        }
    }

    pub fn two_frame_clip() -> Vec<SavedLandmarkFrame> {
        vec![t_pose_frame(0.0), raised_left_arm_frame(1.0 / 30.0)]
    }

    pub fn t_pose_landmarks() -> Vec<Landmark> {
        let mut landmarks = empty_landmarks();
        set_core_torso(&mut landmarks);
        set_arm_landmarks(&mut landmarks);
        set_leg_landmarks(&mut landmarks);
        landmarks
    }

    pub fn raised_left_arm_frame(timestamp_secs: f64) -> SavedLandmarkFrame {
        let mut landmarks = t_pose_landmarks();
        landmarks[13] = landmark(0.25, 0.2, -0.05);
        landmarks[15] = landmark(0.2, 0.08, -0.1);
        SavedLandmarkFrame {
            timestamp_secs,
            landmarks,
        }
    }

    fn empty_landmarks() -> Vec<Landmark> {
        vec![
            Landmark {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                visibility: 0.0,
            };
            LANDMARK_COUNT
        ]
    }

    fn set_core_torso(landmarks: &mut [Landmark]) {
        landmarks[0] = landmark(0.5, 0.15, 0.0);
        landmarks[11] = landmark(0.35, 0.35, 0.0);
        landmarks[12] = landmark(0.65, 0.35, 0.0);
        landmarks[23] = landmark(0.4, 0.55, 0.0);
        landmarks[24] = landmark(0.6, 0.55, 0.0);
    }

    fn set_arm_landmarks(landmarks: &mut [Landmark]) {
        landmarks[13] = landmark(0.2, 0.35, 0.0);
        landmarks[14] = landmark(0.8, 0.35, 0.0);
        landmarks[15] = landmark(0.05, 0.35, 0.0);
        landmarks[16] = landmark(0.95, 0.35, 0.0);
    }

    fn set_leg_landmarks(landmarks: &mut [Landmark]) {
        landmarks[25] = landmark(0.4, 0.75, 0.0);
        landmarks[26] = landmark(0.6, 0.75, 0.0);
        landmarks[27] = landmark(0.4, 0.95, 0.0);
        landmarks[28] = landmark(0.6, 0.95, 0.0);
        landmarks[31] = landmark(0.4, 0.98, -0.05);
        landmarks[32] = landmark(0.6, 0.98, -0.05);
    }

    fn landmark(coordinate_x: f64, coordinate_y: f64, coordinate_z: f64) -> Landmark {
        Landmark {
            x: coordinate_x,
            y: coordinate_y,
            z: coordinate_z,
            visibility: VISIBILITY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometric_backend_reports_static_metadata() {
        let metadata = available_reconstruction_backends();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].id, MotionReconstructionBackendId::Geometric);
        assert!(metadata[0].bundled);
        assert!(!metadata[0].requires_sidecar);
        assert!(!metadata[0].accepts_live_camera);
    }

    #[test]
    fn geometric_backend_wraps_existing_retargeter() {
        let frames = static_landmark_fixtures::two_frame_clip();
        let config = MotionReconstructionConfig::default();

        let reconstructed = reconstruct_saved_landmark_clip(&frames, &config);

        assert_eq!(
            reconstructed.backend,
            MotionReconstructionBackendId::Geometric
        );
        assert_eq!(reconstructed.frames.len(), 2);
        assert!(reconstructed.warnings.is_empty());
        assert!(reconstructed
            .frames
            .iter()
            .all(|frame| frame.pose.is_some()));
        assert!(reconstructed
            .frames
            .iter()
            .all(|frame| frame.confidence > 0.5));
    }

    #[test]
    fn geometric_backend_preserves_retarget_failure_as_frame_warning() {
        let mut frame = static_landmark_fixtures::t_pose_frame(0.0);
        frame.landmarks[11].visibility = 0.0;
        let config = MotionReconstructionConfig::default();

        let reconstructed = reconstruct_saved_landmark_clip(&[frame], &config);

        assert_eq!(reconstructed.frames.len(), 1);
        assert!(reconstructed.frames[0].pose.is_none());
        assert_eq!(reconstructed.frames[0].confidence, 0.0);
        assert_eq!(
            reconstructed.frames[0].warnings,
            vec!["insufficient_visible_landmarks".to_string()]
        );
    }

    #[test]
    fn backend_id_serializes_as_geometric() {
        let json = serde_json::to_string(&MotionReconstructionBackendId::Geometric).unwrap();
        assert_eq!(json, "\"geometric\"");
    }
}
