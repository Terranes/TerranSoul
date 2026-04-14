/**
 * GesturePlayer — plays timed gesture sequences on a VRM character.
 *
 * Gestures are short additive bone-rotation animations defined in gestures.ts.
 * They layer *on top of* both the AnimationMixer output and pose blending.
 *
 * Integration in CharacterAnimator.applyVRMAnimation():
 *   1. mixer.update(delta)
 *   2. poseBlender.apply(vrm, delta)
 *   3. gesturePlayer.apply(vrm, delta)   ← here
 *   4. vrm.update(delta)
 *
 * Queue semantics:
 * - Gestures can be queued; the next starts immediately after the current ends.
 * - If the same gesture is queued while already playing, it is ignored.
 * - Maximum queue depth is 4 to prevent backlog.
 */

import * as THREE from 'three';
import type { VRM } from '@pixiv/three-vrm';
import { getGesture, type GestureDefinition, type GestureKeyframe } from './gestures';

const MAX_QUEUE = 4;

export class GesturePlayer {
  private active: GestureDefinition | null = null;
  private elapsed = 0;
  private queue: GestureDefinition[] = [];

  /**
   * Request a gesture by id. If the gesture is already playing it is ignored.
   * If another gesture is playing, the new one is queued (up to MAX_QUEUE).
   *
   * @param gestureId  The id of a built-in gesture.
   * @returns true if the gesture was started or queued; false if unknown id.
   */
  play(gestureId: string): boolean {
    const def = getGesture(gestureId);
    if (!def) return false;

    if (!this.active) {
      this.startGesture(def);
    } else if (this.active.id !== gestureId && this.queue.length < MAX_QUEUE) {
      this.queue.push(def);
    }
    return true;
  }

  /** Immediately stop the current gesture and clear the queue. */
  stop() {
    this.active = null;
    this.elapsed = 0;
    this.queue = [];
  }

  /** Whether a gesture is currently playing or queued. */
  get isPlaying(): boolean {
    return this.active !== null;
  }

  /** Id of the currently playing gesture, or null. */
  get currentId(): string | null {
    return this.active?.id ?? null;
  }

  /** Number of gestures waiting in the queue. */
  get queueLength(): number {
    return this.queue.length;
  }

  /**
   * Advance the gesture player and apply bone rotation offsets to the VRM.
   * Call *after* mixer.update() and poseBlender.apply(), before vrm.update().
   */
  apply(vrm: VRM, delta: number) {
    if (!this.active) return;

    this.elapsed += delta;

    if (this.elapsed >= this.active.duration) {
      // Gesture complete — advance to next queued gesture
      this.active = null;
      this.elapsed = 0;
      if (this.queue.length > 0) {
        this.startGesture(this.queue.shift()!);
      }
      return;
    }

    // Interpolate between keyframes
    const offsets = this.interpolateKeyframes(this.active.keyframes, this.elapsed);
    this.applyOffsets(vrm, offsets);
  }

  // ── Private helpers ────────────────────────────────────────────────────────

  private startGesture(def: GestureDefinition) {
    this.active = def;
    this.elapsed = 0;
  }

  /**
   * Linear interpolation between keyframes at time `t`.
   * Returns blended bone rotation offsets.
   */
  private interpolateKeyframes(
    keyframes: GestureKeyframe[],
    t: number,
  ): Map<string, { x: number; y: number; z: number }> {
    const result: Map<string, { x: number; y: number; z: number }> = new Map();

    // Find surrounding keyframe pair
    let fromIdx = 0;
    let toIdx = keyframes.length - 1;

    for (let i = 0; i < keyframes.length - 1; i++) {
      if (t >= keyframes[i].time && t <= keyframes[i + 1].time) {
        fromIdx = i;
        toIdx = i + 1;
        break;
      }
    }

    const from = keyframes[fromIdx];
    const to = keyframes[toIdx];

    let alpha = 0;
    if (to.time > from.time) {
      alpha = Math.max(0, Math.min(1, (t - from.time) / (to.time - from.time)));
    }

    // Collect all bone names across both keyframes
    const boneNames = new Set([
      ...Object.keys(from.bones),
      ...Object.keys(to.bones),
    ]);

    for (const bone of boneNames) {
      const f = from.bones[bone] ?? { x: 0, y: 0, z: 0 };
      const tt = to.bones[bone] ?? { x: 0, y: 0, z: 0 };
      result.set(bone, {
        x: f.x + (tt.x - f.x) * alpha,
        y: f.y + (tt.y - f.y) * alpha,
        z: f.z + (tt.z - f.z) * alpha,
      });
    }

    return result;
  }

  private applyOffsets(
    vrm: VRM,
    offsets: Map<string, { x: number; y: number; z: number }>,
  ) {
    const _euler = new THREE.Euler();
    const _quat = new THREE.Quaternion();

    for (const [boneName, offset] of offsets) {
      const absMax = Math.max(Math.abs(offset.x), Math.abs(offset.y), Math.abs(offset.z));
      if (absMax < 0.0001) continue;
      try {
        const node = vrm.humanoid?.getNormalizedBoneNode(boneName as any);
        if (!node) continue;
        _euler.set(offset.x, offset.y, offset.z, 'XYZ');
        _quat.setFromEuler(_euler);
        node.quaternion.multiply(_quat);
      } catch {
        // Bone unavailable — skip
      }
    }
  }
}
