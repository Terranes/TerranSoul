/**
 * emotion-pose-bias.ts — Subtle emotion-driven additive pose offsets.
 *
 * Chunk 14.16d (LLM-as-Animator: Emotion-reactive procedural blending).
 *
 * Layers a tiny, continuous postural bias on top of whatever the
 * character animator + VRMA mixer produce, so a "happy" emotion gets a
 * small chest lift + relaxed shoulders, "sad" gets a soft head/chest
 * droop, "angry" tightens the shoulders inward, etc. The bias is
 * intentionally far smaller than `pose_frame::CLAMP_RADIANS` (0.5) —
 * we cap each component at 0.18 rad so the avatar is *expressive*
 * without ever looking puppeted.
 *
 * Yields control unconditionally to:
 *   1. A baked VRMA clip (`vrmaPlaying = true`)
 *   2. An active `PoseAnimator` pose (caller passes its `isActive`
 *      flag — the bias damps to zero so the LLM-driven pose doesn't
 *      fight the bias)
 *
 * Pure-data design: bone offsets live in plain `Float64Array`s so the
 * frame loop allocates nothing. The `EMOTION_BIAS_TABLE` is exported
 * for unit-testing the mapping without a VRM.
 */

import type { VRM, VRMHumanBoneName } from '@pixiv/three-vrm';

/** Same canonical 11-bone rig as `PoseAnimator` / Rust `pose_frame`. */
export const BIAS_BONES: VRMHumanBoneName[] = [
  'head', 'neck', 'spine', 'chest', 'hips',
  'leftUpperArm', 'rightUpperArm',
  'leftLowerArm', 'rightLowerArm',
  'leftShoulder', 'rightShoulder',
];

/** Recognised emotion labels — superset of `AvatarStateMachine.EmotionState`. */
export type BiasEmotion =
  | 'neutral'
  | 'happy'
  | 'sad'
  | 'angry'
  | 'relaxed'
  | 'surprised';

/** Hard cap on any single bone-component offset (radians). */
export const MAX_BIAS_RAD = 0.18;

/** Damping rate for the additive bias channel (frame-rate independent). */
const BIAS_LAMBDA = 4;

/**
 * Per-emotion target bone-offset table.
 *
 * Entries are XYZ Euler radians, **already capped at ±MAX_BIAS_RAD**.
 * Sign conventions match the rest of the VRM rig:
 *   - X+ = pitch forward (head/chest tilt down, shoulders forward)
 *   - Y+ = yaw left (head turning left)
 *   - Z+ = roll left for spine, body-side for shoulders/arms
 *
 * The values were hand-tuned against Soul's default VRM and verified
 * by the unit tests in `emotion-pose-bias.test.ts` — all entries stay
 * within `MAX_BIAS_RAD` and exhibit left/right symmetry where
 * appropriate (so a "happy" lift doesn't subtly lean the avatar to
 * one side).
 */
export const EMOTION_BIAS_TABLE: Record<
  BiasEmotion,
  Partial<Record<VRMHumanBoneName, [number, number, number]>>
> = {
  neutral: {},
  // happy: facial-expression-only — no postural bias
  happy: {},
  sad: {
    // Drop head + chest forward + shoulders pulled in.
    head: [0.07, 0, 0],
    neck: [0.04, 0, 0],
    chest: [0.06, 0, 0],
    spine: [0.04, 0, 0],
    leftShoulder: [0, 0, 0.05],
    rightShoulder: [0, 0, -0.05],
  },
  angry: {
    // Chin slightly down, shoulders pulled in, upper arms tightened.
    head: [0.03, 0, 0],
    neck: [0.02, 0, 0],
    chest: [0.02, 0, 0],
    leftShoulder: [0, 0, 0.08],
    rightShoulder: [0, 0, -0.08],
    leftUpperArm: [0, 0, 0.06],
    rightUpperArm: [0, 0, -0.06],
  },
  relaxed: {
    // Lazy chest droop, slight head tilt, shoulders down.
    head: [0.02, 0, 0.04],
    chest: [0.02, 0, 0],
    leftShoulder: [0, 0, 0.03],
    rightShoulder: [0, 0, -0.03],
  },
  surprised: {
    // Slight backward lean + raised chin + spread shoulders.
    head: [-0.06, 0, 0],
    neck: [-0.03, 0, 0],
    chest: [-0.04, 0, 0],
    spine: [-0.02, 0, 0],
    leftShoulder: [0, 0, -0.07],
    rightShoulder: [0, 0, 0.07],
  },
};

/** Frame-rate-independent exponential damping toward `target`. */
function damp(current: number, target: number, lambda: number, dt: number): number {
  return target + (current - target) * Math.exp(-lambda * dt);
}

function clampBias(v: number): number {
  if (!Number.isFinite(v)) return 0;
  if (v > MAX_BIAS_RAD) return MAX_BIAS_RAD;
  if (v < -MAX_BIAS_RAD) return -MAX_BIAS_RAD;
  return v;
}

/**
 * Build a fully-keyed bone-offset map for a given emotion + intensity.
 * Bones absent from the table get `[0,0,0]`. Pure / I/O-free — the
 * unit tests drive this directly without any VRM dependency.
 */
export function emotionTargetBias(
  emotion: BiasEmotion,
  intensity: number,
): Map<VRMHumanBoneName, [number, number, number]> {
  const out = new Map<VRMHumanBoneName, [number, number, number]>();
  const k = Math.max(0, Math.min(1, intensity));
  const table = EMOTION_BIAS_TABLE[emotion] ?? {};
  for (const bone of BIAS_BONES) {
    const t = table[bone] ?? [0, 0, 0];
    out.set(bone, [
      clampBias(t[0] * k),
      clampBias(t[1] * k),
      clampBias(t[2] * k),
    ]);
  }
  return out;
}

/**
 * Stateful per-frame applier. Caller mutates `targetEmotion` /
 * `targetIntensity` whenever the avatar's mood changes; the bias damps
 * smoothly toward that target over ~0.25 s.
 */
export class EmotionPoseBias {
  private current = new Map<VRMHumanBoneName, [number, number, number]>();
  private target = new Map<VRMHumanBoneName, [number, number, number]>();
  private weight = 0;
  private targetWeight = 0;

  constructor() {
    for (const bone of BIAS_BONES) {
      this.current.set(bone, [0, 0, 0]);
      this.target.set(bone, [0, 0, 0]);
    }
  }

  /** Update the desired emotion bias. Pass `intensity = 0` to fade out. */
  setEmotion(emotion: BiasEmotion, intensity: number = 1): void {
    this.target = emotionTargetBias(emotion, intensity);
    this.targetWeight = emotion === 'neutral' || intensity <= 0 ? 0 : 1;
  }

  /** Force fade-to-zero (used when VRMA / LLM pose takes over). */
  yield(): void {
    this.targetWeight = 0;
  }

  /**
   * Step the bias forward by `dt` and apply the current offsets to
   * `vrm`. Safe when `vrm` is null (no-op). When `suppress` is true the
   * weight damps to zero regardless of the configured emotion — used
   * by the caller to yield to a baked VRMA clip or an active
   * `PoseAnimator` pose.
   */
  apply(vrm: VRM | null, dt: number, suppress: boolean = false): void {
    const desiredWeight = suppress ? 0 : this.targetWeight;
    this.weight = damp(this.weight, desiredWeight, BIAS_LAMBDA, dt);

    for (const bone of BIAS_BONES) {
      const cur = this.current.get(bone)!;
      const tgt = this.target.get(bone)!;
      cur[0] = damp(cur[0], tgt[0], BIAS_LAMBDA, dt);
      cur[1] = damp(cur[1], tgt[1], BIAS_LAMBDA, dt);
      cur[2] = damp(cur[2], tgt[2], BIAS_LAMBDA, dt);
    }

    if (!vrm || this.weight < 1e-4) return;

    const humanoid = vrm.humanoid;
    if (!humanoid) return;
    for (const bone of BIAS_BONES) {
      const node = humanoid.getNormalizedBoneNode(bone);
      if (!node) continue;
      const cur = this.current.get(bone)!;
      // Additive: layered on top of whatever CharacterAnimator wrote.
      node.rotation.x += cur[0] * this.weight;
      node.rotation.y += cur[1] * this.weight;
      node.rotation.z += cur[2] * this.weight;
    }
  }

  /** Current blend weight in [0, 1]. Useful for tests + telemetry. */
  get currentWeight(): number {
    return this.weight;
  }
}
