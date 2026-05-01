/**
 * LLM-as-Animator pose blender (Chunk 14.16b3).
 *
 * Receives `LlmPoseFrame` payloads from the Rust streaming pipeline
 * (via the `llm-pose` Tauri event) and additively layers them on top
 * of the procedural idle animation produced by `CharacterAnimator`.
 *
 * Lifecycle of a single pose frame:
 *
 * ```
 *  ┌───────────┐  fadeIn 0.3s   ┌───────┐  hold = duration_s   ┌────────────┐  fadeOut 0.5s  ┌──────┐
 *  │ idle      │ ─────────────► │ pose  │ ───────────────────► │ pose hold  │ ─────────────► │ idle │
 *  └───────────┘                └───────┘                      └────────────┘                └──────┘
 * ```
 *
 * The blender holds **only a single active pose at a time**. A new
 * `apply()` call instantly replaces the active pose (with the same
 * fadeIn — the previous pose damps toward the new target).
 *
 * VRMA yield: when the underlying `VrmaManager` reports `playing=true`,
 * the blender clears its target and ramps the offset to zero so the
 * baked clip drives bones unmodified.
 */

import type { VRM } from '@pixiv/three-vrm';
import type { VRMHumanBoneName } from '@pixiv/three-vrm';

/** Easing modes the LLM may request. Matches `pose_frame::PoseEasing`. */
export type PoseEasing = 'linear' | 'ease-in-out' | 'spring';

/**
 * Frontend mirror of Rust `pose_frame::LlmPoseFrame`. Only `bones` is
 * required; everything else has a sensible default.
 *
 * Bone Euler triplets are XYZ radians, hard-clamped by the Rust parser
 * to ±0.5 rad before reaching the frontend.
 */
export interface LlmPoseFrame {
  bones: Record<string, [number, number, number]>;
  duration_s?: number;
  easing?: PoseEasing;
  expression?: Record<string, number>;
}

/** Same canonical 11-bone rig the Rust parser validates against. */
export const POSE_BONES: VRMHumanBoneName[] = [
  'head', 'neck', 'spine', 'chest', 'hips',
  'leftUpperArm', 'rightUpperArm',
  'leftLowerArm', 'rightLowerArm',
  'leftShoulder', 'rightShoulder',
];

const FADE_IN_S = 0.3;
const FADE_OUT_S = 0.5;
const SPRING_LAMBDA = 6;       // exp damping rate (matches CharacterAnimator)
const DEFAULT_DURATION = 2.0;
const MIN_DURATION = 0.05;
const MAX_DURATION = 10.0;

/** One bone's current + target Euler offsets (radians). */
interface BoneEntry {
  current: [number, number, number];
  target: [number, number, number];
}

type Phase = 'idle' | 'fadeIn' | 'hold' | 'fadeOut';

interface ActivePose {
  duration: number;
  easing: PoseEasing;
  expression: Record<string, number>;
  /** Seconds elapsed in the current pose lifecycle. */
  elapsed: number;
  phase: Phase;
}

/** Frame-rate-independent exponential damping toward `target`. */
function damp(current: number, target: number, lambda: number, dt: number): number {
  return target + (current - target) * Math.exp(-lambda * dt);
}

/** Linear interp clamped to [0, 1]. */
function lerp01(a: number, b: number, t: number): number {
  const s = Math.max(0, Math.min(1, t));
  return a + (b - a) * s;
}

/** smoothstep — used for `ease-in-out`. */
function smoothstep01(t: number): number {
  const s = Math.max(0, Math.min(1, t));
  return s * s * (3 - 2 * s);
}

/**
 * The blender. Owns no THREE state by itself; `apply(vrm, dt)` mutates
 * bone rotations on the supplied VRM at the end of each frame.
 */
export class PoseAnimator {
  private bones = new Map<VRMHumanBoneName, BoneEntry>();
  private active: ActivePose | null = null;
  private vrmaPlaying = false;
  /** Current overall blend weight (0 = idle, 1 = full pose). */
  private weight = 0;

  /** True while a pose is active (fading in, holding, or fading out). */
  get isActive(): boolean {
    return this.active !== null && this.active.phase !== 'idle';
  }

  /** Current blend weight in [0, 1]. Useful for tests + telemetry. */
  get currentWeight(): number {
    return this.weight;
  }

  /** Notify the blender that a VRMA clip is/was driving the bones. */
  setVrmaPlaying(playing: boolean) {
    this.vrmaPlaying = playing;
    if (playing) {
      // Yield immediately — start fading out so the VRMA clip can
      // take over without competing for the same bones.
      this.beginFadeOut();
    }
  }

  /**
   * Apply a new pose frame. Sanitises inputs, replaces any in-flight
   * pose, and starts a fresh fade-in.
   */
  applyFrame(frame: LlmPoseFrame): void {
    if (this.vrmaPlaying) {
      // VRMA owns the bones — silently drop. A future stream-end
      // signal can re-trigger; we don't queue here to keep behaviour
      // predictable.
      return;
    }
    const sanitisedBones = this.sanitiseBones(frame.bones);
    if (sanitisedBones.size === 0) {
      // Nothing to animate; ignore the frame entirely.
      return;
    }
    // Set or reset every canonical bone's target. Bones not present in
    // this frame return to zero offset (so successive frames feel
    // independent rather than additive).
    for (const name of POSE_BONES) {
      const entry = this.boneEntry(name);
      const next = sanitisedBones.get(name) ?? [0, 0, 0];
      entry.target = next;
    }
    const duration = clampDuration(frame.duration_s);
    this.active = {
      duration,
      easing: frame.easing ?? 'spring',
      expression: { ...(frame.expression ?? {}) },
      elapsed: 0,
      phase: 'fadeIn',
    };
  }

  /**
   * Advance the blender by `dt` seconds and apply the resulting bone
   * offsets to `vrm`. Safe to call when `vrm` is null (no-op) or when
   * no pose is active (continues damping back to zero).
   */
  apply(vrm: VRM | null, dt: number): void {
    this.tickPhase(dt);
    this.tickWeight(dt);
    this.tickBones(dt);
    if (vrm) {
      this.applyToVrm(vrm);
    }
  }

  /** Drop the active pose and fade back to idle. */
  reset(): void {
    if (this.active && this.active.phase !== 'fadeOut') {
      this.beginFadeOut();
    }
    for (const entry of this.bones.values()) {
      entry.target = [0, 0, 0];
    }
  }

  // ── internals ──────────────────────────────────────────────────────

  private sanitiseBones(
    raw: Record<string, [number, number, number]>,
  ): Map<VRMHumanBoneName, [number, number, number]> {
    const out = new Map<VRMHumanBoneName, [number, number, number]>();
    const allowed = new Set<string>(POSE_BONES);
    for (const [name, val] of Object.entries(raw)) {
      if (!allowed.has(name)) continue;
      if (!Array.isArray(val) || val.length !== 3) continue;
      const sanitised: [number, number, number] = [
        sanitiseNumber(val[0]),
        sanitiseNumber(val[1]),
        sanitiseNumber(val[2]),
      ];
      out.set(name as VRMHumanBoneName, sanitised);
    }
    return out;
  }

  private boneEntry(name: VRMHumanBoneName): BoneEntry {
    let entry = this.bones.get(name);
    if (!entry) {
      entry = { current: [0, 0, 0], target: [0, 0, 0] };
      this.bones.set(name, entry);
    }
    return entry;
  }

  private tickPhase(dt: number) {
    if (!this.active) return;
    this.active.elapsed += dt;
    switch (this.active.phase) {
      case 'fadeIn':
        if (this.active.elapsed >= FADE_IN_S) {
          this.active.elapsed = 0;
          this.active.phase = 'hold';
        }
        break;
      case 'hold':
        if (this.active.elapsed >= this.active.duration) {
          this.active.elapsed = 0;
          this.active.phase = 'fadeOut';
        }
        break;
      case 'fadeOut':
        if (this.active.elapsed >= FADE_OUT_S) {
          this.active = null;
        }
        break;
      case 'idle':
        break;
    }
  }

  private tickWeight(dt: number) {
    let targetWeight: number;
    if (!this.active) {
      targetWeight = 0;
    } else if (this.active.phase === 'fadeIn') {
      const t = this.active.elapsed / FADE_IN_S;
      targetWeight = this.active.easing === 'linear'
        ? lerp01(0, 1, t)
        : smoothstep01(t);
    } else if (this.active.phase === 'hold') {
      targetWeight = 1;
    } else if (this.active.phase === 'fadeOut') {
      const t = 1 - this.active.elapsed / FADE_OUT_S;
      targetWeight = this.active.easing === 'linear'
        ? lerp01(0, 1, t)
        : smoothstep01(t);
    } else {
      targetWeight = 0;
    }
    if (this.active?.easing === 'spring' || !this.active) {
      this.weight = damp(this.weight, targetWeight, SPRING_LAMBDA, dt);
    } else {
      this.weight = targetWeight;
    }
  }

  private tickBones(dt: number) {
    for (const entry of this.bones.values()) {
      // Damp bones toward target (or toward zero when idle).
      const target = this.active ? entry.target : [0, 0, 0] as const;
      entry.current[0] = damp(entry.current[0], target[0], SPRING_LAMBDA, dt);
      entry.current[1] = damp(entry.current[1], target[1], SPRING_LAMBDA, dt);
      entry.current[2] = damp(entry.current[2], target[2], SPRING_LAMBDA, dt);
    }
  }

  private applyToVrm(vrm: VRM) {
    if (!vrm.humanoid) return;
    for (const [name, entry] of this.bones) {
      const node = vrm.humanoid.getNormalizedBoneNode(name);
      if (!node) continue;
      // Additive offset on top of whatever CharacterAnimator already wrote.
      node.rotation.x += entry.current[0] * this.weight;
      node.rotation.y += entry.current[1] * this.weight;
      node.rotation.z += entry.current[2] * this.weight;
    }
    // Apply expression weights (additive, weight-scaled). Best-effort —
    // an unknown expression name is silently ignored.
    if (this.active && vrm.expressionManager) {
      for (const [name, raw] of Object.entries(this.active.expression)) {
        const v = sanitiseExpressionWeight(raw);
        try {
          vrm.expressionManager.setValue(name, v * this.weight);
        } catch { /* unknown expression on this VRM */ }
      }
    }
  }

  private beginFadeOut() {
    if (!this.active) return;
    if (this.active.phase === 'fadeOut') return;
    this.active.phase = 'fadeOut';
    this.active.elapsed = 0;
  }
}

function sanitiseNumber(v: unknown): number {
  if (typeof v !== 'number' || !Number.isFinite(v)) return 0;
  // The Rust parser already clamps to ±0.5, but defend in depth in case
  // the frame was constructed in JS without going through the IPC layer.
  return Math.max(-0.5, Math.min(0.5, v));
}

function sanitiseExpressionWeight(v: unknown): number {
  if (typeof v !== 'number' || !Number.isFinite(v)) return 0;
  return Math.max(0, Math.min(1, v));
}

function clampDuration(v: unknown): number {
  if (typeof v !== 'number' || !Number.isFinite(v)) return DEFAULT_DURATION;
  return Math.max(MIN_DURATION, Math.min(MAX_DURATION, v));
}
