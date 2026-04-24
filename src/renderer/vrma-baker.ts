/**
 * vrma-baker.ts — Bake a LearnedMotion (JSON frame timeline) into a
 * THREE.AnimationClip that VrmaManager can play.
 *
 * The pure `bakeMotionToClip()` function is the unit-tested seam.
 * It builds VRM-compatible bone rotation tracks from the recorded frame
 * timeline without requiring a VRM model (the clip is model-independent
 * until bound to a mixer).
 *
 * This eliminates per-frame retargeting cost and unlocks sharing learned
 * motions between devices via the existing Soul Link sync surface.
 */

import * as THREE from 'three';
import type { LearnedMotion } from '../stores/persona-types';

// ── Constants ─────────────────────────────────────────────────────────────────

/**
 * VRM humanoid bone names used in LearnedMotion frames.
 * These match the `ANIMATED_BONES` list in character-animator.ts.
 */
const BONE_NAMES = [
  'head', 'neck', 'spine', 'chest', 'hips',
  'leftUpperArm', 'rightUpperArm',
  'leftLowerArm', 'rightLowerArm',
  'leftShoulder', 'rightShoulder',
] as const;

// ── Pure baker (unit-testable seam) ───────────────────────────────────────────

/**
 * Bake a `LearnedMotion` clip into a `THREE.AnimationClip`.
 *
 * Each bone with at least one keyframe becomes a `QuaternionKeyframeTrack`
 * (VRM humanoid bones use quaternion rotations internally). The Euler angles
 * from the recorded frames are converted to quaternions.
 *
 * @param motion — The recorded LearnedMotion clip.
 * @param name — Optional clip name (defaults to motion.trigger).
 * @returns A THREE.AnimationClip ready for a mixer, or null if no valid frames.
 */
export function bakeMotionToClip(
  motion: LearnedMotion,
  name?: string,
): THREE.AnimationClip | null {
  if (motion.frames.length === 0) return null;

  const clipName = name ?? motion.trigger ?? motion.name;
  const tracks: THREE.KeyframeTrack[] = [];

  // For each bone, collect timestamps and quaternion values across all frames
  for (const boneName of BONE_NAMES) {
    const times: number[] = [];
    const values: number[] = [];

    for (const frame of motion.frames) {
      const euler = frame.bones[boneName];
      if (!euler) continue;

      times.push(frame.t);

      // Convert Euler [x, y, z] radians to quaternion [x, y, z, w]
      const q = new THREE.Quaternion();
      const e = new THREE.Euler(euler[0], euler[1], euler[2], 'XYZ');
      q.setFromEuler(e);
      values.push(q.x, q.y, q.z, q.w);
    }

    if (times.length === 0) continue;

    // VRM normalized bone track name pattern:
    // The mixer resolves tracks by name matching against scene graph node names.
    // For VRM humanoid bones, the convention is the bone name directly.
    const trackName = `${boneName}.quaternion`;

    tracks.push(
      new THREE.QuaternionKeyframeTrack(trackName, times, values),
    );
  }

  if (tracks.length === 0) return null;

  const duration = motion.duration_s > 0
    ? motion.duration_s
    : motion.frames[motion.frames.length - 1].t;

  return new THREE.AnimationClip(clipName, duration, tracks);
}

/**
 * Bake multiple learned motions into a Map of trigger → AnimationClip.
 * Skips any that fail to bake (empty frames, etc.).
 */
export function bakeAllMotions(
  motions: LearnedMotion[],
): Map<string, THREE.AnimationClip> {
  const result = new Map<string, THREE.AnimationClip>();
  for (const motion of motions) {
    const clip = bakeMotionToClip(motion);
    if (clip) {
      result.set(motion.trigger, clip);
    }
  }
  return result;
}
