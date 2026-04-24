/**
 * pose-mirror.ts — MediaPipe PoseLandmarker → VRM humanoid bone retargeting.
 *
 * The pure retargeter (`retargetPoseToVRM`) is the unit-tested seam.
 * The `PoseMirror` class wraps MediaPipe PoseLandmarker for real-time use.
 *
 * MediaPipe PoseLandmarker returns 33 keypoints; we use a subset to drive
 * the 11 VRM humanoid bones animated by CharacterAnimator.
 */

import { dampWeight } from './face-mirror';

// ── MediaPipe Pose landmark indices (from BlazePose topology) ─────────────

/** Subset of the 33 BlazePose landmarks used for upper-body retargeting. */
export const MP = {
  NOSE: 0,
  LEFT_SHOULDER: 11,
  RIGHT_SHOULDER: 12,
  LEFT_ELBOW: 13,
  RIGHT_ELBOW: 14,
  LEFT_WRIST: 15,
  RIGHT_WRIST: 16,
  LEFT_HIP: 23,
  RIGHT_HIP: 24,
} as const;

// ── Types ─────────────────────────────────────────────────────────────────

/** A 3D landmark from MediaPipe (normalized coords, 0–1). */
export interface Landmark {
  x: number;
  y: number;
  z: number;
  visibility?: number;
}

/** Euler-angle triplet [x, y, z] in radians for a VRM bone. */
export type EulerTriple = [number, number, number];

/** VRM bone rotations output by the retargeter. */
export interface VrmBonePose {
  head?: EulerTriple;
  neck?: EulerTriple;
  spine?: EulerTriple;
  chest?: EulerTriple;
  hips?: EulerTriple;
  leftUpperArm?: EulerTriple;
  rightUpperArm?: EulerTriple;
  leftLowerArm?: EulerTriple;
  rightLowerArm?: EulerTriple;
  leftShoulder?: EulerTriple;
  rightShoulder?: EulerTriple;
}

/** All VRM bone names that can appear in a VrmBonePose. */
export const VRM_BONE_NAMES = [
  'head', 'neck', 'spine', 'chest', 'hips',
  'leftUpperArm', 'rightUpperArm',
  'leftLowerArm', 'rightLowerArm',
  'leftShoulder', 'rightShoulder',
] as const;

export type VrmBoneName = (typeof VRM_BONE_NAMES)[number];

// ── Math helpers ──────────────────────────────────────────────────────────

function sub(a: Landmark, b: Landmark): [number, number, number] {
  return [a.x - b.x, a.y - b.y, a.z - b.z];
}

function atan2(y: number, x: number): number {
  return Math.atan2(y, x);
}

function clampRad(v: number, min: number, max: number): number {
  return v < min ? min : v > max ? max : v;
}

// ── Pure retargeter (unit-testable seam) ──────────────────────────────────

/** Minimum landmark visibility to trust the joint position. */
const VIS_THRESHOLD = 0.5;

/**
 * Is a landmark visible enough to use?
 */
function visible(lm: Landmark): boolean {
  return (lm.visibility ?? 0) >= VIS_THRESHOLD;
}

/**
 * Retarget 33 MediaPipe pose landmarks to VRM humanoid bone Euler angles.
 *
 * Uses simple inverse trigonometry on joint vectors — no IK solver needed
 * because we drive pre-rigged VRM humanoid bones directly.
 *
 * Joint angle limits match CharacterAnimator's dress-aware clamping ranges.
 *
 * @param landmarks — Array of 33 MediaPipe PoseLandmarker keypoints.
 * @returns VRM bone rotations for the visible subset of joints.
 */
export function retargetPoseToVRM(landmarks: Landmark[]): VrmBonePose {
  if (landmarks.length < 25) return {};

  const pose: VrmBonePose = {};

  const nose = landmarks[MP.NOSE];
  const lShoulder = landmarks[MP.LEFT_SHOULDER];
  const rShoulder = landmarks[MP.RIGHT_SHOULDER];
  const lElbow = landmarks[MP.LEFT_ELBOW];
  const rElbow = landmarks[MP.RIGHT_ELBOW];
  const lWrist = landmarks[MP.LEFT_WRIST];
  const rWrist = landmarks[MP.RIGHT_WRIST];
  const lHip = landmarks[MP.LEFT_HIP];
  const rHip = landmarks[MP.RIGHT_HIP];

  // ── Torso orientation (spine/chest/hips) ──────────────────────────
  if (visible(lShoulder) && visible(rShoulder) && visible(lHip) && visible(rHip)) {
    // Shoulder midpoint → hip midpoint = spine vector
    const shoulderMid: Landmark = {
      x: (lShoulder.x + rShoulder.x) / 2,
      y: (lShoulder.y + rShoulder.y) / 2,
      z: (lShoulder.z + rShoulder.z) / 2,
    };
    const hipMid: Landmark = {
      x: (lHip.x + rHip.x) / 2,
      y: (lHip.y + rHip.y) / 2,
      z: (lHip.z + rHip.z) / 2,
    };

    // Spine lean (forward/back = X rotation, side = Z rotation)
    const spineVec = sub(shoulderMid, hipMid);
    const spineX = clampRad(atan2(spineVec[2], -spineVec[1]), -0.4, 0.4);
    const spineZ = clampRad(atan2(spineVec[0], -spineVec[1]) * 0.5, -0.3, 0.3);

    // Shoulder twist (Y rotation)
    const shoulderDiff = sub(rShoulder, lShoulder);
    const spineY = clampRad(atan2(shoulderDiff[2], shoulderDiff[0]) * 0.6, -0.5, 0.5);

    pose.spine = [spineX * 0.4, spineY * 0.3, spineZ * 0.4];
    pose.chest = [spineX * 0.3, spineY * 0.4, spineZ * 0.3];
    pose.hips = [spineX * 0.3, spineY * 0.3, spineZ * 0.3];
  }

  // ── Head ──────────────────────────────────────────────────────────
  if (visible(nose) && visible(lShoulder) && visible(rShoulder)) {
    const shoulderMid: Landmark = {
      x: (lShoulder.x + rShoulder.x) / 2,
      y: (lShoulder.y + rShoulder.y) / 2,
      z: (lShoulder.z + rShoulder.z) / 2,
    };
    const headVec = sub(nose, shoulderMid);

    const headX = clampRad(atan2(headVec[2], -headVec[1]) * 0.8, -0.5, 0.5);
    const headY = clampRad(atan2(headVec[0], -headVec[1]) * 0.8, -0.6, 0.6);

    pose.head = [headX, headY, 0];
    pose.neck = [headX * 0.4, headY * 0.4, 0];
  }

  // ── Left arm ──────────────────────────────────────────────────────
  if (visible(lShoulder) && visible(lElbow)) {
    const upperArm = sub(lElbow, lShoulder);
    const armX = clampRad(atan2(upperArm[2], upperArm[1]), -1.5, 1.5);
    const armZ = clampRad(atan2(-upperArm[0], upperArm[1]), -0.3, 2.5);

    pose.leftUpperArm = [armX, 0, armZ];
    pose.leftShoulder = [0, 0, armZ * 0.2];

    // Lower arm (elbow bend)
    if (visible(lWrist)) {
      const forearm = sub(lWrist, lElbow);
      const elbowBend = clampRad(atan2(forearm[1], -forearm[0]), -2.5, 0);
      pose.leftLowerArm = [0, 0, elbowBend];
    }
  }

  // ── Right arm (mirrored) ──────────────────────────────────────────
  if (visible(rShoulder) && visible(rElbow)) {
    const upperArm = sub(rElbow, rShoulder);
    const armX = clampRad(atan2(upperArm[2], upperArm[1]), -1.5, 1.5);
    const armZ = clampRad(atan2(upperArm[0], upperArm[1]), -2.5, 0.3);

    pose.rightUpperArm = [armX, 0, armZ];
    pose.rightShoulder = [0, 0, armZ * 0.2];

    // Lower arm (elbow bend)
    if (visible(rWrist)) {
      const forearm = sub(rWrist, rElbow);
      const elbowBend = clampRad(atan2(forearm[1], forearm[0]), 0, 2.5);
      pose.rightLowerArm = [0, 0, elbowBend];
    }
  }

  return pose;
}

// ── Smoothed pose state ───────────────────────────────────────────────────

/** Create a zeroed VrmBonePose with all 11 bones set. */
export function zeroBonePose(): Required<VrmBonePose> {
  const z: EulerTriple = [0, 0, 0];
  return {
    head: [...z], neck: [...z], spine: [...z], chest: [...z], hips: [...z],
    leftUpperArm: [...z], rightUpperArm: [...z],
    leftLowerArm: [...z], rightLowerArm: [...z],
    leftShoulder: [...z], rightShoulder: [...z],
  };
}

/**
 * Smooth all bone channels of a VrmBonePose using EMA.
 * Mutates `smoothed` in-place and returns it.
 */
export function smoothBonePose(
  smoothed: Required<VrmBonePose>,
  raw: VrmBonePose,
  lambda: number,
  dt: number,
): Required<VrmBonePose> {
  for (const name of VRM_BONE_NAMES) {
    const target = raw[name];
    const current = smoothed[name]!;
    if (target) {
      for (let i = 0; i < 3; i++) {
        current[i] = dampWeight(current[i], target[i], lambda, dt);
      }
    } else {
      // Decay to rest when landmark lost
      for (let i = 0; i < 3; i++) {
        current[i] = dampWeight(current[i], 0, lambda * 0.5, dt);
      }
    }
  }
  return smoothed;
}

// ── PoseMirror class (wraps MediaPipe) ────────────────────────────────────

const DEFAULT_LAMBDA = 10;

/**
 * Real-time pose → VRM bone mirror.
 *
 * Lazy-loads `@mediapipe/tasks-vision` PoseLandmarker.
 *
 * Usage:
 *   const mirror = new PoseMirror();
 *   await mirror.init();
 *   // In rAF loop:
 *   const bones = mirror.update(videoElement, deltaSeconds);
 *   // When done:
 *   mirror.dispose();
 */
export class PoseMirror {
  private landmarker: import('@mediapipe/tasks-vision').PoseLandmarker | null = null;
  private smoothed = zeroBonePose();
  private _running = false;
  private _lastTimestamp = -1;

  get running(): boolean { return this._running; }

  /**
   * Initialise the PoseLandmarker.
   * Lazy-imports @mediapipe/tasks-vision on first call.
   */
  async init(): Promise<void> {
    const { PoseLandmarker, FilesetResolver } = await import('@mediapipe/tasks-vision');

    const vision = await FilesetResolver.forVisionTasks(
      'https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@latest/wasm',
    );

    this.landmarker = await PoseLandmarker.createFromOptions(vision, {
      baseOptions: {
        modelAssetPath: 'https://storage.googleapis.com/mediapipe-models/pose_landmarker/pose_landmarker_lite/float16/1/pose_landmarker_lite.task',
        delegate: 'GPU',
      },
      runningMode: 'VIDEO',
      numPoses: 1,
    });

    this.smoothed = zeroBonePose();
    this._running = true;
  }

  /**
   * Process a single video frame and return smoothed VRM bone rotations.
   */
  update(video: HTMLVideoElement, dt: number, lambda = DEFAULT_LAMBDA): Required<VrmBonePose> {
    if (!this.landmarker || !this._running) return this.smoothed;

    const timestamp = video.currentTime * 1000;
    if (timestamp === this._lastTimestamp) return this.smoothed;
    this._lastTimestamp = timestamp;

    const result = this.landmarker.detectForVideo(video, performance.now());

    if (result.landmarks && result.landmarks.length > 0) {
      const lm = result.landmarks[0];
      const raw = retargetPoseToVRM(lm);
      smoothBonePose(this.smoothed, raw, lambda, dt);
    }

    return this.smoothed;
  }

  /** Tear down the PoseLandmarker and release WASM resources. */
  dispose(): void {
    this._running = false;
    if (this.landmarker) {
      this.landmarker.close();
      this.landmarker = null;
    }
    this.smoothed = zeroBonePose();
  }
}
