import { describe, it, expect } from 'vitest';
import {
  retargetPoseToVRM,
  zeroBonePose,
  smoothBonePose,
  type Landmark,
  type VrmBonePose,
  MP,
  VRM_BONE_NAMES,
} from './pose-mirror';

// ── Helpers ─────────────────────────────────────────────────────────────────

/** Create a default 33-landmark array with all zeros and full visibility. */
function emptyLandmarks(): Landmark[] {
  return Array.from({ length: 33 }, () => ({ x: 0, y: 0, z: 0, visibility: 1.0 }));
}

/** Set a specific landmark. */
function setLm(lms: Landmark[], idx: number, x: number, y: number, z: number, vis = 1.0): void {
  lms[idx] = { x, y, z, visibility: vis };
}

// ── Tests ───────────────────────────────────────────────────────────────────

describe('retargetPoseToVRM', () => {
  it('returns empty pose for too-short array', () => {
    const pose = retargetPoseToVRM([]);
    expect(Object.keys(pose)).toHaveLength(0);
  });

  it('returns empty pose for array with 20 landmarks', () => {
    const lms = Array.from({ length: 20 }, () => ({ x: 0, y: 0, z: 0, visibility: 1.0 }));
    const pose = retargetPoseToVRM(lms);
    expect(Object.keys(pose)).toHaveLength(0);
  });

  it('produces bone rotations for a neutral standing pose', () => {
    const lms = emptyLandmarks();
    // Shoulders level, hips level, nose above shoulders
    setLm(lms, MP.LEFT_SHOULDER, -0.15, 0.4, 0);
    setLm(lms, MP.RIGHT_SHOULDER, 0.15, 0.4, 0);
    setLm(lms, MP.LEFT_HIP, -0.1, 0.7, 0);
    setLm(lms, MP.RIGHT_HIP, 0.1, 0.7, 0);
    setLm(lms, MP.NOSE, 0, 0.2, 0);
    setLm(lms, MP.LEFT_ELBOW, -0.3, 0.5, 0);
    setLm(lms, MP.RIGHT_ELBOW, 0.3, 0.5, 0);

    const pose = retargetPoseToVRM(lms);
    expect(pose.spine).toBeDefined();
    expect(pose.head).toBeDefined();
    expect(pose.leftUpperArm).toBeDefined();
    expect(pose.rightUpperArm).toBeDefined();
  });

  it('detects head tilt when nose is off-center', () => {
    const lms = emptyLandmarks();
    setLm(lms, MP.LEFT_SHOULDER, -0.15, 0.5, 0);
    setLm(lms, MP.RIGHT_SHOULDER, 0.15, 0.5, 0);
    setLm(lms, MP.LEFT_HIP, -0.1, 0.8, 0);
    setLm(lms, MP.RIGHT_HIP, 0.1, 0.8, 0);
    setLm(lms, MP.NOSE, 0.1, 0.3, 0); // Nose shifted right
    setLm(lms, MP.LEFT_ELBOW, -0.3, 0.6, 0);
    setLm(lms, MP.RIGHT_ELBOW, 0.3, 0.6, 0);

    const pose = retargetPoseToVRM(lms);
    expect(pose.head).toBeDefined();
    // Head Y rotation should be non-zero (tilted right)
    expect(pose.head![1]).not.toBe(0);
  });

  it('skips arm when elbow is not visible', () => {
    const lms = emptyLandmarks();
    setLm(lms, MP.LEFT_SHOULDER, -0.15, 0.5, 0);
    setLm(lms, MP.RIGHT_SHOULDER, 0.15, 0.5, 0);
    setLm(lms, MP.LEFT_HIP, -0.1, 0.8, 0);
    setLm(lms, MP.RIGHT_HIP, 0.1, 0.8, 0);
    setLm(lms, MP.NOSE, 0, 0.3, 0);
    // Left elbow invisible
    setLm(lms, MP.LEFT_ELBOW, -0.3, 0.6, 0, 0.1);
    // Right elbow visible
    setLm(lms, MP.RIGHT_ELBOW, 0.3, 0.6, 0, 0.9);

    const pose = retargetPoseToVRM(lms);
    expect(pose.leftUpperArm).toBeUndefined();
    expect(pose.rightUpperArm).toBeDefined();
  });

  it('produces elbow bend when wrist is visible', () => {
    const lms = emptyLandmarks();
    setLm(lms, MP.LEFT_SHOULDER, -0.15, 0.5, 0);
    setLm(lms, MP.RIGHT_SHOULDER, 0.15, 0.5, 0);
    setLm(lms, MP.LEFT_HIP, -0.1, 0.8, 0);
    setLm(lms, MP.RIGHT_HIP, 0.1, 0.8, 0);
    setLm(lms, MP.NOSE, 0, 0.3, 0);
    setLm(lms, MP.LEFT_ELBOW, -0.3, 0.55, 0);
    setLm(lms, MP.LEFT_WRIST, -0.35, 0.4, 0);
    setLm(lms, MP.RIGHT_ELBOW, 0.3, 0.55, 0);
    setLm(lms, MP.RIGHT_WRIST, 0.35, 0.4, 0);

    const pose = retargetPoseToVRM(lms);
    expect(pose.leftLowerArm).toBeDefined();
    expect(pose.rightLowerArm).toBeDefined();
  });

  it('clamps all angles within safe ranges', () => {
    const lms = emptyLandmarks();
    // Extreme positions
    setLm(lms, MP.LEFT_SHOULDER, -0.8, 0.1, 0.5);
    setLm(lms, MP.RIGHT_SHOULDER, 0.8, 0.1, -0.5);
    setLm(lms, MP.LEFT_HIP, -0.5, 0.9, 0.3);
    setLm(lms, MP.RIGHT_HIP, 0.5, 0.9, -0.3);
    setLm(lms, MP.NOSE, 0, 0, 0.8);
    setLm(lms, MP.LEFT_ELBOW, -1, 0.3, 0.5);
    setLm(lms, MP.RIGHT_ELBOW, 1, 0.3, -0.5);
    setLm(lms, MP.LEFT_WRIST, -1.2, 0.1, 0.3);
    setLm(lms, MP.RIGHT_WRIST, 1.2, 0.1, -0.3);

    const pose = retargetPoseToVRM(lms);
    // All angles should be within [-3, 3] radians (< π)
    for (const boneName of VRM_BONE_NAMES) {
      const triple = pose[boneName];
      if (triple) {
        for (const v of triple) {
          expect(Math.abs(v)).toBeLessThanOrEqual(3);
        }
      }
    }
  });

  it('splits torso rotation across spine/chest/hips', () => {
    const lms = emptyLandmarks();
    setLm(lms, MP.LEFT_SHOULDER, -0.2, 0.4, 0.1);
    setLm(lms, MP.RIGHT_SHOULDER, 0.2, 0.4, -0.1);
    setLm(lms, MP.LEFT_HIP, -0.1, 0.8, 0);
    setLm(lms, MP.RIGHT_HIP, 0.1, 0.8, 0);
    setLm(lms, MP.NOSE, 0, 0.2, 0);
    setLm(lms, MP.LEFT_ELBOW, -0.3, 0.5, 0);
    setLm(lms, MP.RIGHT_ELBOW, 0.3, 0.5, 0);

    const pose = retargetPoseToVRM(lms);
    expect(pose.spine).toBeDefined();
    expect(pose.chest).toBeDefined();
    expect(pose.hips).toBeDefined();
    // All three should share the torso rotation
    for (const bone of ['spine', 'chest', 'hips'] as const) {
      const triple = pose[bone]!;
      expect(triple).toHaveLength(3);
    }
  });
});

describe('zeroBonePose', () => {
  it('creates a pose with all 11 bones set to zero', () => {
    const pose = zeroBonePose();
    for (const name of VRM_BONE_NAMES) {
      expect(pose[name]).toBeDefined();
      expect(pose[name]).toEqual([0, 0, 0]);
    }
  });
});

describe('smoothBonePose', () => {
  it('converges smoothed pose toward raw target', () => {
    const smoothed = zeroBonePose();
    const raw: VrmBonePose = {
      head: [0.5, 0.3, 0],
      leftUpperArm: [0, 0, 1.0],
    };

    for (let i = 0; i < 300; i++) {
      smoothBonePose(smoothed, raw, 10, 0.016);
    }

    expect(smoothed.head![0]).toBeCloseTo(0.5, 2);
    expect(smoothed.head![1]).toBeCloseTo(0.3, 2);
    expect(smoothed.leftUpperArm![2]).toBeCloseTo(1.0, 2);
  });

  it('decays missing bones toward zero', () => {
    const smoothed = zeroBonePose();
    smoothed.head = [0.5, 0.3, 0];

    // Update with empty raw (no head) — should decay toward zero
    for (let i = 0; i < 300; i++) {
      smoothBonePose(smoothed, {}, 10, 0.016);
    }

    expect(smoothed.head![0]).toBeCloseTo(0, 2);
    expect(smoothed.head![1]).toBeCloseTo(0, 2);
  });
});
