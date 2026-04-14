/**
 * Pose Blending Engine
 *
 * PoseBlender maintains a set of weighted pose presets and applies them
 * as additive Euler-angle offsets to VRM normalized bone nodes.
 *
 * Integration flow (inside CharacterAnimator.applyVRMAnimation):
 *   1. mixer.update(delta)  — sets base rotations from keyframe clips
 *   2. poseBlender.apply(vrm, delta) — adds pose offsets on top
 *   3. vrm.update(delta)    — normalizes → raw skeleton, expressions, spring
 *
 * Design choices:
 * - Weighted *average* of Euler angles works well for small additive offsets
 *   (< ±0.6 rad). For large angles, quaternion slerp is more accurate but
 *   not necessary at the magnitudes these presets use.
 * - Blend weight transitions are smoothed via exponential decay (lerp),
 *   preventing jarring pop-in when the LLM emits a new pose tag.
 * - Only bones explicitly listed in the active preset(s) are touched.
 *   All other bones remain exactly as the AnimationMixer set them.
 */

import * as THREE from 'three';
import type { VRM } from '@pixiv/three-vrm';
import { getAllPosePresets, type PoseBoneRotation } from './pose-presets';

// Build a flat lookup of id → boneRotations once at module load.
const PRESET_BONES: Map<string, Record<string, PoseBoneRotation>> = new Map(
  getAllPosePresets().map(p => [p.id, p.boneRotations as Record<string, PoseBoneRotation>]),
);

/** How fast blend weights interpolate toward targets (exponential decay rate). */
const BLEND_SPEED = 4.0;

/** Blend instruction: a preset id + desired weight [0,1]. */
export interface BlendInstruction {
  presetId: string;
  weight: number;
}

/**
 * PoseBlender manages weighted blending between pose presets.
 *
 * Usage:
 *   const blender = new PoseBlender();
 *   blender.setTarget([{ presetId: 'confident', weight: 0.7 }]);
 *   // each frame:
 *   blender.apply(vrm, delta);
 */
export class PoseBlender {
  /** Smoothed current weights for each preset. */
  private currentWeights: Map<string, number> = new Map();
  /** Target weights the blender is lerping toward. */
  private targetWeights: Map<string, number> = new Map();

  /**
   * Set the target pose blend. Any preset not listed gets weight → 0.
   *
   * @param instructions  Array of { presetId, weight } blend targets.
   *                      Weights need not sum to 1; they are applied
   *                      independently as additive offsets.
   */
  setTarget(instructions: BlendInstruction[]) {
    // Mark all current targets to fade out
    for (const id of this.targetWeights.keys()) {
      this.targetWeights.set(id, 0);
    }
    // Apply new targets (clamped to [0, 1])
    for (const { presetId, weight } of instructions) {
      if (PRESET_BONES.has(presetId)) {
        this.targetWeights.set(presetId, Math.max(0, Math.min(1, weight)));
      }
    }
  }

  /**
   * Immediately set all blend weights to zero without interpolation.
   * Call this when resetting the character to neutral.
   */
  reset() {
    this.currentWeights.clear();
    this.targetWeights.clear();
  }

  /**
   * Apply blended pose offsets to the VRM's normalized skeleton.
   * Must be called *after* mixer.update(delta) and *before* vrm.update(delta).
   *
   * @param vrm   The loaded VRM instance.
   * @param delta Frame delta time in seconds.
   */
  apply(vrm: VRM, delta: number) {
    // Smooth current weights toward targets
    const allIds = new Set([...this.currentWeights.keys(), ...this.targetWeights.keys()]);
    for (const id of allIds) {
      const current = this.currentWeights.get(id) ?? 0;
      const target = this.targetWeights.get(id) ?? 0;
      const next = current + (target - current) * Math.min(1, BLEND_SPEED * delta);
      if (next < 0.001 && target === 0) {
        this.currentWeights.delete(id);
      } else {
        this.currentWeights.set(id, next);
      }
    }

    // Accumulate bone offsets across all active presets
    const boneOffsets: Map<string, { x: number; y: number; z: number }> = new Map();

    for (const [id, weight] of this.currentWeights) {
      if (weight < 0.001) continue;
      const bones = PRESET_BONES.get(id);
      if (!bones) continue;
      for (const [boneName, rot] of Object.entries(bones)) {
        const existing = boneOffsets.get(boneName) ?? { x: 0, y: 0, z: 0 };
        existing.x += rot.x * weight;
        existing.y += rot.y * weight;
        existing.z += rot.z * weight;
        boneOffsets.set(boneName, existing);
      }
    }

    if (boneOffsets.size === 0) return;

    // Apply accumulated offsets to normalized bone nodes
    const _euler = new THREE.Euler();
    const _quat = new THREE.Quaternion();

    for (const [boneName, offset] of boneOffsets) {
      try {
        const node = vrm.humanoid?.getNormalizedBoneNode(boneName as any);
        if (!node) continue;
        // Compose offset quaternion and multiply onto existing bone rotation
        _euler.set(offset.x, offset.y, offset.z, 'XYZ');
        _quat.setFromEuler(_euler);
        node.quaternion.multiply(_quat);
      } catch {
        // Bone not available on this model — skip silently
      }
    }
  }

  /** Return current blend weights (for debugging / serialization). */
  getCurrentWeights(): Map<string, number> {
    return new Map(this.currentWeights);
  }

  /** Whether any preset is currently active (weight > threshold). */
  get isActive(): boolean {
    for (const w of this.currentWeights.values()) {
      if (w > 0.001) return true;
    }
    return false;
  }
}
