import * as THREE from 'three';
import type { VRM, VRMHumanBoneName } from '@pixiv/three-vrm';
import type { CharacterState } from '../types';

/**
 * Smooth interpolation helper — lerps a value toward a target each frame.
 * This produces exponential ease-out, which feels natural.
 */
function smoothStep(current: number, target: number, speed: number, delta: number): number {
  return current + (target - current) * Math.min(1, speed * delta);
}

// ── Expression targets per state ──────────────────────────────────────
const STATE_EXPRESSIONS: Record<CharacterState, Record<string, number>> = {
  idle:      { relaxed: 0.25 },
  thinking:  { neutral: 0.3 },
  talking:   { relaxed: 0.15 },
  happy:     { happy: 0.7, relaxed: 0.2 },
  sad:       { sad: 0.6 },
  angry:     { angry: 0.7 },
  relaxed:   { relaxed: 0.6, happy: 0.15 },
  surprised: { surprised: 0.8 },
};

// ── Bone pose targets per state (Euler angles in radians) ─────────────
// Uses the standard VRM humanoid bone names from @pixiv/three-vrm.
// Each state defines target rotations layered on top of a gentle
// idle breathing cycle.  Bones not listed stay at their natural pose.
interface BonePose {
  head?: [number, number, number];
  spine?: [number, number, number];
  chest?: [number, number, number];
  hips?: [number, number, number];
  leftUpperArm?: [number, number, number];
  rightUpperArm?: [number, number, number];
  neck?: [number, number, number];
}

const STATE_BONE_POSES: Record<CharacterState, BonePose> = {
  idle: {
    head:  [0, 0, 0],
    spine: [0, 0, 0],
    chest: [0, 0, 0],
    hips:  [0, 0, 0],
    neck:  [0, 0, 0],
  },
  thinking: {
    head:  [0.08, 0.12, 0.04],     // tilted slightly, looking up-right
    spine: [0.02, 0, 0],
    chest: [0, 0, 0],
    hips:  [0, 0, 0],
    neck:  [0.04, 0.08, 0],
    leftUpperArm:  [0, 0, 1.45],    // left arm crosses slightly more
    rightUpperArm: [0.2, -0.1, -1.20], // right hand toward chin
  },
  talking: {
    head:  [0, 0, 0],
    spine: [0.02, 0, 0],           // slight lean forward
    chest: [0.01, 0, 0],
    hips:  [0, 0, 0],
    neck:  [0, 0, 0],
    leftUpperArm:  [0, 0, 1.25],   // arms slightly out for gesturing
    rightUpperArm: [0, 0, -1.25],
  },
  happy: {
    head:  [-0.06, 0, 0.04],       // head up, slight tilt
    spine: [-0.04, 0, 0],          // chest out
    chest: [-0.02, 0, 0],
    hips:  [0, 0, 0],
    neck:  [-0.04, 0, 0.02],
    leftUpperArm:  [-0.1, 0, 1.15],  // arms wider (open body language)
    rightUpperArm: [-0.1, 0, -1.15],
  },
  sad: {
    head:  [0.15, 0, -0.02],       // head down
    spine: [0.08, 0, 0],           // slouched
    chest: [0.04, 0, 0],
    hips:  [0, 0, 0],
    neck:  [0.08, 0, 0],
    leftUpperArm:  [0.05, 0, 1.45], // arms closer to body
    rightUpperArm: [0.05, 0, -1.45],
  },
  angry: {
    head:  [0.06, 0, 0],           // chin down, intense
    spine: [-0.03, 0, 0],          // chest puffed
    chest: [-0.02, 0, 0],
    hips:  [0, 0, 0],
    neck:  [0.04, 0, 0],
    leftUpperArm:  [0, 0.1, 1.20],  // arms tense, slightly out
    rightUpperArm: [0, -0.1, -1.20],
  },
  relaxed: {
    head:  [-0.04, 0.03, 0.02],    // head slightly back, gentle tilt
    spine: [-0.02, 0, 0],
    chest: [0, 0, 0],
    hips:  [0, 0.02, 0],           // slight sway
    neck:  [-0.02, 0, 0.01],
    leftUpperArm:  [0, 0, 1.30],
    rightUpperArm: [0, 0, -1.30],
  },
  surprised: {
    head:  [-0.10, 0, 0],          // head back (recoil)
    spine: [-0.06, 0, 0],          // lean back
    chest: [-0.03, 0, 0],
    hips:  [0, 0, 0],
    neck:  [-0.06, 0, 0],
    leftUpperArm:  [-0.15, 0, 1.10], // arms up/out
    rightUpperArm: [-0.15, 0, -1.10],
  },
};

// All bone names we animate — typed as VRMHumanBoneName
const ANIMATED_BONES: VRMHumanBoneName[] = [
  'head', 'spine', 'chest', 'hips', 'neck',
  'leftUpperArm', 'rightUpperArm',
];

/**
 * VRM animator that drives procedural bone animation, facial expressions,
 * and blinking using only standard Three.js and @pixiv/three-vrm APIs.
 *
 * - Body animation: per-frame procedural bone rotations via VRM humanoid
 *   normalized bone nodes. Idle breathing is layered with state-specific
 *   poses, smoothly interpolated for natural transitions.
 * - Face animation: morph-target expressions driven by emotion state.
 * - Blink: randomised natural blink cycle.
 * - Lip-sync: external or procedural sine-wave mouth flap.
 */
export class CharacterAnimator {
  private vrm: VRM | null = null;
  private vrmScene: THREE.Object3D | null = null;
  private placeholder: THREE.Group | null = null;
  private state: CharacterState = 'idle';
  private elapsed = 0;
  private baseRotationY = 0;

  // Blink timing constants
  private static readonly BLINK_DURATION = 0.15;
  private static readonly MIN_BLINK_INTERVAL = 2.0;
  private static readonly MAX_BLINK_INTERVAL = 6.0;

  // Smooth blink state
  private nextBlinkTime = CharacterAnimator.randomBlinkInterval();
  private blinkValue = 0;
  private isBlinking = false;
  private blinkTimer = 0;

  // Smooth expression targets (interpolated each frame)
  private expressionTargets: Map<string, number> = new Map();
  private expressionCurrent: Map<string, number> = new Map();

  // Smooth bone rotation targets (interpolated each frame)
  // Key = VRMHumanBoneName, Value = target Euler [x, y, z]
  private boneTargets: Map<string, [number, number, number]> = new Map();
  private boneCurrent: Map<string, [number, number, number]> = new Map();

  // Mouth animation elapsed for talking state
  private mouthElapsed = 0;

  // External lip-sync values (from LipSync class or Open-LLM-VTuber volumes)
  private externalMouthAa = 0;
  private externalMouthOh = 0;
  private useExternalLipSync = false;

  private static randomBlinkInterval(): number {
    return CharacterAnimator.MIN_BLINK_INTERVAL +
      Math.random() * (CharacterAnimator.MAX_BLINK_INTERVAL - CharacterAnimator.MIN_BLINK_INTERVAL);
  }

  setVRM(vrm: VRM, rotationY = 0) {
    this.vrm = vrm;
    this.vrmScene = vrm.scene;
    this.baseRotationY = rotationY;
    this.placeholder = null;
    // Reset blink timing
    this.nextBlinkTime = CharacterAnimator.randomBlinkInterval();
    this.blinkValue = 0;
    this.isBlinking = false;
    this.blinkTimer = 0;
    // Reset bone interpolation state
    this.boneCurrent.clear();
    this.boneTargets.clear();
  }

  /** Configure the VRM lookAt target so the model's eyes track the camera. */
  setLookAtTarget(target: THREE.Object3D) {
    if (this.vrm?.lookAt) {
      this.vrm.lookAt.target = target;
    }
  }

  setPlaceholder(group: THREE.Group) {
    this.placeholder = group;
    this.vrm = null;
    this.vrmScene = null;
  }

  setState(state: CharacterState) {
    if (this.state === state) return;
    this.state = state;
    this.elapsed = 0;
    this.mouthElapsed = 0;
  }

  getState(): CharacterState {
    return this.state;
  }

  update(delta: number) {
    this.elapsed += delta;
    const t = this.elapsed;

    if (this.vrm && this.vrmScene) {
      this.applyVRMAnimation(t, delta);
    } else if (this.placeholder) {
      this.applyPlaceholderAnimation(t);
    }
  }

  /**
   * Set mouth morph values from an external lip-sync source.
   * When called with non-zero values, overrides the procedural sine-wave
   * mouth animation for the talking state.
   *
   * @param aa — mouth open "ah" (0–1)
   * @param oh — mouth round "oh" (0–1)
   */
  setMouthValues(aa: number, oh: number) {
    this.externalMouthAa = Math.max(0, Math.min(1, aa));
    this.externalMouthOh = Math.max(0, Math.min(1, oh));
    this.useExternalLipSync = aa > 0 || oh > 0;
  }

  /** Clear external lip-sync, reverting to procedural mouth animation. */
  clearMouthValues() {
    this.externalMouthAa = 0;
    this.externalMouthOh = 0;
    this.useExternalLipSync = false;
  }

  // ── VRM animation (bones + expressions + blink) ────────────────────

  private applyVRMAnimation(t: number, delta: number) {
    if (!this.vrm || !this.vrmScene) return;

    // Pin scene root — only preserve the loader's base rotation
    this.vrmScene.position.set(0, 0, 0);
    this.vrmScene.rotation.set(0, this.baseRotationY, 0);

    // Natural blinking with random intervals
    this.updateBlink(delta);

    // Set expression targets for the current state
    this.applyStateExpressions(t, delta);

    // Smoothly interpolate all expressions toward their targets
    this.flushExpressions(delta);

    // Set bone pose targets for the current state (with idle breathing overlay)
    this.applyStateBonePose(t);

    // Smoothly interpolate bones toward their targets
    this.flushBones(delta);

    // vrm.update() transfers normalized bones → raw skeleton,
    // then updates lookAt, expressions, and spring bones.
    this.vrm.update(delta);
  }

  // ── State-based expression targets ─────────────────────────────────

  private applyStateExpressions(_t: number, delta: number) {
    // Clear all expression targets first
    this.clearExpressionTargets();

    // Apply per-state base expressions
    const targets = STATE_EXPRESSIONS[this.state];
    for (const [name, value] of Object.entries(targets)) {
      this.setExpressionTarget(name, value);
    }

    // Mouth flap for talking state — use external lip sync when available,
    // otherwise fall back to procedural sine wave
    if (this.state === 'talking') {
      if (this.useExternalLipSync) {
        this.setExpressionTarget('aa', this.externalMouthAa);
        this.setExpressionTarget('oh', this.externalMouthOh);
      } else {
        this.mouthElapsed += delta;
        const mouth = ((Math.sin(this.mouthElapsed * 5.5) + 1) * 0.5) * 0.5;
        this.setExpressionTarget('aa', mouth);
      }
    }
  }

  // ── State-based bone pose targets (with breathing overlay) ─────────

  private applyStateBonePose(t: number) {
    const pose = STATE_BONE_POSES[this.state] ?? STATE_BONE_POSES.idle;

    // Idle breathing cycle — subtle sine wave layered on all states
    // to keep the character feeling alive
    const breathCycle = Math.sin(t * 1.2);     // ~0.6 Hz breathing rate
    const breathAmt = 0.015;                   // subtle breath amplitude
    const swayCycle = Math.sin(t * 0.4);       // slow idle sway

    // Per-state additional movement overlays
    let headOscX = 0;
    let headOscY = 0;
    let headOscZ = 0;
    let spineOscX = 0;
    let spineOscY = 0;
    let hipsOscY = 0;

    switch (this.state) {
      case 'idle':
        // Gentle head sway + breathing
        headOscY = Math.sin(t * 0.5) * 0.03;
        headOscZ = Math.sin(t * 0.35) * 0.015;
        spineOscY = swayCycle * 0.01;
        hipsOscY = swayCycle * 0.008;
        break;
      case 'thinking':
        // Head tilting rhythmically as if pondering
        headOscX = Math.sin(t * 0.8) * 0.04;
        headOscY = Math.sin(t * 0.6) * 0.06;
        break;
      case 'talking':
        // Subtle gesturing movement
        headOscY = Math.sin(t * 2.5) * 0.04;
        headOscX = Math.sin(t * 1.8) * 0.02;
        spineOscY = Math.sin(t * 1.5) * 0.015;
        break;
      case 'happy':
        // Bouncy, energetic
        headOscZ = Math.sin(t * 3.0) * 0.04;
        spineOscX = Math.sin(t * 2.5) * -0.02;
        hipsOscY = Math.sin(t * 2.0) * 0.02;
        break;
      case 'sad':
        // Slow, droopy
        headOscX = Math.sin(t * 0.3) * 0.02;
        spineOscX = Math.sin(t * 0.4) * 0.01;
        break;
      case 'angry':
        // Tense, vibrating
        headOscX = Math.sin(t * 6.0) * 0.01;
        spineOscX = Math.sin(t * 5.0) * 0.008;
        break;
      case 'relaxed':
        // Slow, peaceful sway
        headOscY = Math.sin(t * 0.35) * 0.04;
        headOscZ = Math.sin(t * 0.25) * 0.02;
        spineOscY = Math.sin(t * 0.3) * 0.015;
        hipsOscY = Math.sin(t * 0.25) * 0.01;
        break;
      case 'surprised':
        // Quick recoil that settles
        const recoilDecay = Math.exp(-t * 2.0);
        headOscX = -0.08 * recoilDecay;
        spineOscX = -0.04 * recoilDecay;
        break;
    }

    // Compose final bone targets: base pose + breathing + per-state oscillation
    for (const boneName of ANIMATED_BONES) {
      const base = pose[boneName as keyof BonePose] ?? [0, 0, 0];
      let x = base[0];
      let y = base[1];
      let z = base[2];

      if (boneName === 'head') {
        x += headOscX;
        y += headOscY;
        z += headOscZ;
      } else if (boneName === 'spine') {
        x += breathCycle * breathAmt + spineOscX; // breathing = spine tilt
        y += spineOscY;
      } else if (boneName === 'chest') {
        x += breathCycle * breathAmt * 0.5;
      } else if (boneName === 'hips') {
        y += hipsOscY;
        x += breathCycle * breathAmt * 0.3;
      } else if (boneName === 'neck') {
        x += headOscX * 0.4; // neck follows head partially
        y += headOscY * 0.3;
      }

      this.boneTargets.set(boneName, [x, y, z]);
    }
  }

  // ── Smooth bone interpolation ──────────────────────────────────────

  private flushBones(delta: number) {
    if (!this.vrm) return;

    const boneSpeed = 4.0; // lerp speed for bone transitions

    for (const [boneName, target] of this.boneTargets) {
      const current = this.boneCurrent.get(boneName) ?? [0, 0, 0];
      const next: [number, number, number] = [
        smoothStep(current[0], target[0], boneSpeed, delta),
        smoothStep(current[1], target[1], boneSpeed, delta),
        smoothStep(current[2], target[2], boneSpeed, delta),
      ];
      this.boneCurrent.set(boneName, next);

      // Apply to VRM humanoid normalized bone via the standard API
      const node = this.vrm.humanoid?.getNormalizedBoneNode(boneName as VRMHumanBoneName);
      if (node) {
        node.rotation.set(next[0], next[1], next[2]);
      }
    }
  }

  // ── Natural blink system with random timing ────────────────────────

  private updateBlink(delta: number) {
    if (!this.isBlinking) {
      this.nextBlinkTime -= delta;
      if (this.nextBlinkTime <= 0) {
        this.isBlinking = true;
        this.blinkTimer = 0;
      }
    }

    if (this.isBlinking) {
      this.blinkTimer += delta;
      const half = CharacterAnimator.BLINK_DURATION / 2;
      if (this.blinkTimer < half) {
        this.blinkValue = this.blinkTimer / half;
      } else if (this.blinkTimer < CharacterAnimator.BLINK_DURATION) {
        this.blinkValue = 1.0 - (this.blinkTimer - half) / half;
      } else {
        this.blinkValue = 0;
        this.isBlinking = false;
        this.nextBlinkTime = CharacterAnimator.randomBlinkInterval();
      }
    }

    this.setExpressionTarget('blink', this.blinkValue);
  }

  // ── Smooth expression system ───────────────────────────────────────

  private setExpressionTarget(name: string, value: number) {
    this.expressionTargets.set(name, value);
  }

  private clearExpressionTargets() {
    for (const name of ['aa', 'oh', 'happy', 'sad', 'angry', 'relaxed', 'surprised', 'neutral']) {
      this.expressionTargets.set(name, 0);
    }
  }

  /**
   * Smoothly interpolate current expression values toward targets each frame.
   * This prevents harsh snapping between expression states.
   */
  private flushExpressions(delta: number) {
    const expressionSpeed = 8.0;
    for (const [name, target] of this.expressionTargets) {
      const current = this.expressionCurrent.get(name) ?? 0;
      const next = smoothStep(current, target, expressionSpeed, delta);
      this.expressionCurrent.set(name, next);
      try {
        this.vrm?.expressionManager?.setValue(name, next);
      } catch { /* expression not available on this model */ }
    }
  }

  // ── Placeholder animation (fallback when no VRM loaded) ────────────

  private applyPlaceholderAnimation(t: number) {
    if (!this.placeholder) return;

    switch (this.state) {
      case 'idle':
        this.placeholder.position.y = Math.sin(t * 0.8) * 0.03;
        this.placeholder.rotation.y = Math.sin(t * 0.4) * 0.1;
        this.placeholder.scale.setScalar(1.0);
        break;

      case 'thinking':
        this.placeholder.rotation.y += 0.02;
        this.placeholder.position.y = Math.sin(t * 2.0) * 0.04;
        this.placeholder.scale.setScalar(1.0);
        break;

      case 'talking':
        this.placeholder.position.y = Math.sin(t * 6.0) * 0.025;
        this.placeholder.rotation.z = Math.sin(t * 6.0) * 0.04;
        this.placeholder.scale.setScalar(1.0 + Math.sin(t * 8.0) * 0.04);
        break;

      case 'happy':
        this.placeholder.position.y = Math.abs(Math.sin(t * 5.0)) * 0.08;
        this.placeholder.rotation.z = Math.sin(t * 5.0) * 0.08;
        this.placeholder.scale.setScalar(1.0 + Math.abs(Math.sin(t * 5.0)) * 0.05);
        break;

      case 'sad':
        this.placeholder.position.y = -Math.abs(Math.sin(t * 0.5)) * 0.04;
        this.placeholder.rotation.z = Math.sin(t * 0.5) * 0.02;
        this.placeholder.rotation.x = 0.1;
        this.placeholder.scale.setScalar(0.95 + Math.sin(t * 0.3) * 0.02);
        break;

      case 'angry':
        this.placeholder.position.y = Math.sin(t * 8.0) * 0.015;
        this.placeholder.rotation.z = Math.sin(t * 6.0) * 0.06;
        this.placeholder.scale.setScalar(1.0 + Math.abs(Math.sin(t * 4.0)) * 0.03);
        break;

      case 'relaxed':
        this.placeholder.position.y = Math.sin(t * 0.5) * 0.02;
        this.placeholder.rotation.y = Math.sin(t * 0.3) * 0.06;
        this.placeholder.scale.setScalar(1.0);
        break;

      case 'surprised':
        this.placeholder.position.y = Math.max(0, Math.sin(t * 3.0)) * 0.06;
        this.placeholder.rotation.z = Math.sin(t * 4.0) * 0.05;
        this.placeholder.scale.setScalar(1.0 + Math.abs(Math.sin(t * 3.0)) * 0.06);
        break;
    }
  }
}
