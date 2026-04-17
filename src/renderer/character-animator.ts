import * as THREE from 'three';
import type { VRM, VRMHumanBoneName } from '@pixiv/three-vrm';
import type { CharacterState } from '../types';
import { GestureBlender } from './gesture-blender';
import { AvatarStateMachine } from './avatar-state';

/**
 * Exponential damping — frame-rate-independent smooth interpolation.
 * Produces identical visual results regardless of frame rate.
 */
export function damp(current: number, target: number, lambda: number, delta: number): number {
  return current + (target - current) * (1 - Math.exp(-lambda * delta));
}

/**
 * Soft-clamp helpers — exported for testing.
 */
export function softClampMin(value: number, minVal: number, margin: number): number {
  if (value >= minVal + margin) return value;
  if (value <= minVal) return minVal;
  const t = (value - minVal) / margin;
  const smooth = t * t * (3 - 2 * t);
  return minVal + smooth * margin;
}

export function softClampMax(value: number, maxVal: number, margin: number): number {
  if (value <= maxVal - margin) return value;
  if (value >= maxVal) return maxVal;
  const t = (maxVal - value) / margin;
  const smooth = t * t * (3 - 2 * t);
  return maxVal - smooth * margin;
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
  leftLowerArm?: [number, number, number];
  rightLowerArm?: [number, number, number];
  leftShoulder?: [number, number, number];
  rightShoulder?: [number, number, number];
  neck?: [number, number, number];
}

// ── Multiple idle pose variations for natural randomization ──
const IDLE_POSES: BonePose[] = [
  // Pose 1: Neutral standing (original idle pose)
  {
    head:  [0, 0, 0],
    spine: [0, 0, 0],
    chest: [0, 0, 0],
    hips:  [0, 0, 0],
    neck:  [0, 0, 0],
    leftUpperArm:  [0, 0, 1.35],    // arms down (natural pose)
    rightUpperArm: [0, 0, -1.35],
    leftLowerArm:  [0, 0, 0.15],    // slight elbow bend
    rightLowerArm: [0, 0, -0.15],
    leftShoulder:  [0, 0, 0.05],
    rightShoulder: [0, 0, -0.05],
  },
  // Pose 2: Weight shifted to left hip with slight body lean
  {
    head:  [0, -0.03, 0.02],        // head angles to balance body shift
    spine: [0, -0.02, 0],
    chest: [0, -0.015, 0],
    hips:  [0, -0.04, 0],           // shifted left
    neck:  [0, -0.01, 0],
    leftUpperArm:  [0, 0, 1.36],    // left arm slightly more relaxed
    rightUpperArm: [0, 0, -1.30],   // right arm tighter to body
    leftLowerArm:  [0, 0, 0.20],
    rightLowerArm: [0, 0, -0.10],
    leftShoulder:  [0, 0, 0.08],
    rightShoulder: [0, 0, -0.03],
  },
  // Pose 3: Weight shifted to right hip with crossed stance
  {
    head:  [0, 0.02, -0.02],        // head angles opposite to hip shift
    spine: [0, 0.02, 0],
    chest: [0, 0.015, 0],
    hips:  [0, 0.04, 0],            // shifted right
    neck:  [0, 0.01, -0.01],
    leftUpperArm:  [0, 0, 1.30],    // left arm closer to body
    rightUpperArm: [0, 0, -1.36],   // right arm more relaxed
    leftLowerArm:  [0, 0, 0.10],
    rightLowerArm: [0, 0, -0.20],
    leftShoulder:  [0, 0, 0.03],
    rightShoulder: [0, 0, -0.08],
  },
  // Pose 4: Slight forward lean (attentive stance)
  {
    head:  [-0.02, 0, 0],           // slight forward nod
    spine: [0.03, 0, 0],            // leaning forward
    chest: [0.02, 0, 0],
    hips:  [0.01, 0, 0],
    neck:  [-0.01, 0, 0],
    leftUpperArm:  [0, 0, 1.25],    // arms slightly forward
    rightUpperArm: [0, 0, -1.25],
    leftLowerArm:  [0, 0, 0.20],
    rightLowerArm: [0, 0, -0.20],
    leftShoulder:  [0, 0, 0.06],
    rightShoulder: [0, 0, -0.06],
  },
  // Pose 5: One hand on hip (confident stance)
  // Note: right arm akimbo — upper arm stays outside dress edge (≥1.10)
  {
    head:  [0, 0.01, 0.01],         // slight head tilt
    spine: [0, 0, 0],
    chest: [0, 0, 0],
    hips:  [0, 0.01, 0],
    neck:  [0, 0.01, 0],
    leftUpperArm:  [0, 0, 1.35],    // left arm neutral
    rightUpperArm: [0.15, -0.08, -1.12], // right arm akimbo — kept outside dress
    leftLowerArm:  [0, 0, 0.15],
    rightLowerArm: [0, 0, -0.50],   // hand on hip — limited to avoid clipping
    leftShoulder:  [0, 0, 0.05],
    rightShoulder: [0, 0, -0.10],
  },
  // Pose 6: Clasped hands in front (at ease stance)
  // Note: arms kept forward/resting at sides instead of behind back
  // to avoid clipping through the flared dress from behind.
  {
    head:  [0, 0, 0],
    spine: [-0.01, 0, 0],           // chest out slightly
    chest: [-0.01, 0, 0],
    hips:  [0, 0, 0],
    neck:  [0, 0, 0],
    leftUpperArm:  [-0.05, 0, 1.25],  // arms slightly forward
    rightUpperArm: [-0.05, 0, -1.25],
    leftLowerArm:  [0, 0, 0.30],    // hands clasped in front
    rightLowerArm: [0, 0, -0.30],
    leftShoulder:  [-0.02, 0, 0.08],
    rightShoulder: [-0.02, 0, -0.08],
  },
];

const STATE_BONE_POSES: Record<CharacterState, BonePose> = {
  // idle will be dynamically selected from IDLE_POSES array
  idle: IDLE_POSES[0], // default to first pose
  thinking: {
    head:  [0.08, 0.12, 0.04],     // tilted slightly, looking up-right
    spine: [0.02, 0, 0],
    chest: [0, 0, 0],
    hips:  [0, 0, 0],
    neck:  [0.04, 0.08, 0],
    leftUpperArm:  [0, 0, 1.35],    // left arm at side
    rightUpperArm: [0.2, -0.1, -1.20], // right hand toward chin
    leftLowerArm:  [0, 0, 0.15],
    rightLowerArm: [0, 0, -0.45],   // forearm raised toward face
    leftShoulder:  [0, 0, 0.05],
    rightShoulder: [0, 0, -0.05],
  },
  talking: {
    head:  [0, 0, 0],
    spine: [0.02, 0, 0],           // slight lean forward
    chest: [0.01, 0, 0],
    hips:  [0, 0, 0],
    neck:  [0, 0, 0],
    leftUpperArm:  [0, 0, 1.25],   // arms slightly out for gesturing
    rightUpperArm: [0, 0, -1.25],
    leftLowerArm:  [0, 0, 0.2],
    rightLowerArm: [0, 0, -0.2],
    leftShoulder:  [0, 0, 0.05],
    rightShoulder: [0, 0, -0.05],
  },
  happy: {
    head:  [-0.06, 0, 0.04],       // head up, slight tilt
    spine: [-0.04, 0, 0],          // chest out
    chest: [-0.02, 0, 0],
    hips:  [0, 0, 0],
    neck:  [-0.04, 0, 0.02],
    leftUpperArm:  [-0.1, 0, 1.15],  // arms wider (open body language)
    rightUpperArm: [-0.1, 0, -1.15],
    leftLowerArm:  [0, 0, 0.25],
    rightLowerArm: [0, 0, -0.25],
    leftShoulder:  [0, 0, 0.08],
    rightShoulder: [0, 0, -0.08],
  },
  sad: {
    head:  [0.15, 0, -0.02],       // head down
    spine: [0.08, 0, 0],           // slouched
    chest: [0.04, 0, 0],
    hips:  [0, 0, 0],
    neck:  [0.08, 0, 0],
    leftUpperArm:  [0.05, 0, 1.35], // arms at sides, slightly hunched
    rightUpperArm: [0.05, 0, -1.35],
    leftLowerArm:  [0, 0, 0.1],
    rightLowerArm: [0, 0, -0.1],
    leftShoulder:  [0, 0.05, 0.08],
    rightShoulder: [0, -0.05, -0.08],
  },
  angry: {
    head:  [0.06, 0, 0],           // chin down, intense
    spine: [-0.03, 0, 0],          // chest puffed
    chest: [-0.02, 0, 0],
    hips:  [0, 0, 0],
    neck:  [0.04, 0, 0],
    leftUpperArm:  [0, 0.1, 1.20],  // arms tense, slightly out
    rightUpperArm: [0, -0.1, -1.20],
    leftLowerArm:  [0, 0, 0.3],
    rightLowerArm: [0, 0, -0.3],
    leftShoulder:  [0, 0.03, 0.06],
    rightShoulder: [0, -0.03, -0.06],
  },
  relaxed: {
    head:  [-0.04, 0.03, 0.02],    // head slightly back, gentle tilt
    spine: [-0.02, 0, 0],
    chest: [0, 0, 0],
    hips:  [0, 0.02, 0],           // slight sway
    neck:  [-0.02, 0, 0.01],
    leftUpperArm:  [0, 0, 1.30],
    rightUpperArm: [0, 0, -1.30],
    leftLowerArm:  [0, 0, 0.12],
    rightLowerArm: [0, 0, -0.12],
    leftShoulder:  [0, 0, 0.05],
    rightShoulder: [0, 0, -0.05],
  },
  surprised: {
    head:  [-0.10, 0, 0],          // head back (recoil)
    spine: [-0.06, 0, 0],          // lean back
    chest: [-0.03, 0, 0],
    hips:  [0, 0, 0],
    neck:  [-0.06, 0, 0],
    leftUpperArm:  [-0.15, 0, 1.10], // arms up/out
    rightUpperArm: [-0.15, 0, -1.10],
    leftLowerArm:  [0, 0, 0.35],
    rightLowerArm: [0, 0, -0.35],
    leftShoulder:  [0, 0, 0.1],
    rightShoulder: [0, 0, -0.1],
  },
};

// All bone names we animate — typed as VRMHumanBoneName
const ANIMATED_BONES: VRMHumanBoneName[] = [
  'head', 'spine', 'chest', 'hips', 'neck',
  'leftUpperArm', 'rightUpperArm',
  'leftLowerArm', 'rightLowerArm',
  'leftShoulder', 'rightShoulder',
];

// ── Expression channel layout (flat typed arrays, zero-alloc frame loop) ──

// Emotion channels (named for readability; used via EXPR_NAMES lookup)
// const EXPR_HAPPY = 0;
// const EXPR_SAD = 1;
// const EXPR_ANGRY = 2;
// const EXPR_RELAXED = 3;
// const EXPR_SURPRISED = 4;
// const EXPR_NEUTRAL = 5;
// Viseme channels
const EXPR_AA = 6;
const EXPR_IH = 7;
const EXPR_OU = 8;
const EXPR_EE = 9;
const EXPR_OH = 10;
// Blink channel
const EXPR_BLINK = 11;
const EXPR_COUNT = 12;

/** VRM expression names corresponding to each channel index. */
const EXPR_NAMES: readonly string[] = [
  'happy', 'sad', 'angry', 'relaxed', 'surprised', 'neutral',
  'aa', 'ih', 'ou', 'ee', 'oh',
  'blink',
];

/** Per-channel damping rates: λ=8 emotions, λ=18 visemes, λ=25 blink. */
const EXPR_LAMBDAS = new Float64Array([
  8, 8, 8, 8, 8, 8,
  18, 18, 18, 18, 18,
  25,
]);

/** Damping rate for bone rotations. */
const BONE_LAMBDA = 6;
const BONE_STRIDE = 3;

// ── Dress-aware arm clamp boundaries ──────────────────────────────────
// For characters wearing flared dresses (e.g. Annabelle), the arms must
// not rotate too tightly against the body or the hands/forearms clip
// through the dress volume.
//
// VRM convention:  left upper arm Z > 0 → arm toward body (higher = tighter);
//                  right upper arm Z < 0 → arm toward body (lower = tighter).
//
// A flared skirt extends outward at the hip/wrist level.  When the
// upper arm |Z| exceeds ~1.38 rad the hands enter the dress silhouette.
// We enforce a soft max at 1.38 rad with a smooth transition zone.

/** Maximum absolute Z-rotation for upper arms (rad).  Arms tighter to the
 *  body than this angle will clip through a flared dress. */
const DRESS_UPPER_ARM_Z_MAX = 1.38;

/** Width of the soft transition zone (rad). */
const DRESS_CLAMP_MARGIN = 0.08;

/** Maximum inward bend for the lower arm Z-rotation (rad).
 *  Prevents the forearm from folding into the dress volume. */
const DRESS_LOWER_ARM_Z_MAX = 0.50;

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

  // AvatarStateMachine — canonical animation state (read in frame loop)
  private _asm = new AvatarStateMachine();

  // Idle pose randomization
  private currentIdlePoseIndex = 0;
  private idlePoseChangeTime = 0;
  private nextIdlePoseChangeAt = 8;

  /** Exposes the layered AvatarStateMachine for external mutation. */
  get avatarStateMachine(): AvatarStateMachine { return this._asm; }

  // Flat typed arrays for zero-alloc per-frame expression damping
  private exprTargets = new Float64Array(EXPR_COUNT);
  private exprCurrent = new Float64Array(EXPR_COUNT);

  // Flat typed arrays for bone rotation damping
  private boneTargetArr = new Float64Array(ANIMATED_BONES.length * BONE_STRIDE);
  private boneCurrentArr = new Float64Array(ANIMATED_BONES.length * BONE_STRIDE);

  // Mouth animation elapsed for talking state
  private mouthElapsed = 0;

  // External lip-sync values (from LipSync class or Open-LLM-VTuber volumes)
  private externalMouthAa = 0;
  private externalMouthOh = 0;
  private useExternalLipSync = false;

  private blender = new GestureBlender();

  setVRM(vrm: VRM, rotationY = 0) {
    this.vrm = vrm;
    this.vrmScene = vrm.scene;
    this.baseRotationY = rotationY;
    this.placeholder = null;
    // Reset flat arrays
    this.exprCurrent.fill(0);
    this.exprTargets.fill(0);
    this.boneTargetArr.fill(0);
    // Initialize bone current values to idle pose so we don't damp from T-pose
    const idlePose = STATE_BONE_POSES.idle;
    for (let i = 0; i < ANIMATED_BONES.length; i++) {
      const boneName = ANIMATED_BONES[i];
      const base = idlePose[boneName as keyof BonePose] ?? [0, 0, 0];
      this.boneCurrentArr[i * BONE_STRIDE]     = base[0];
      this.boneCurrentArr[i * BONE_STRIDE + 1] = base[1];
      this.boneCurrentArr[i * BONE_STRIDE + 2] = base[2];
    }
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
    this.blender.transitionTo(state, this.elapsed);
    this.state = state;
    this.elapsed = 0;
    this.mouthElapsed = 0;
    // Bridge to AvatarStateMachine
    this.bridgeStateToAvatar(state);
  }

  getState(): CharacterState {
    return this.state;
  }

  /** Map legacy CharacterState to body + emotion on the AvatarStateMachine. */
  private bridgeStateToAvatar(state: CharacterState): void {
    const asm = this._asm;
    switch (state) {
      case 'idle':      asm.forceBody('idle');  asm.setEmotion('neutral');   break;
      case 'thinking':  asm.forceBody('think'); asm.setEmotion('neutral');   break;
      case 'talking':   asm.forceBody('talk');  asm.setEmotion('neutral');   break;
      case 'happy':     asm.forceBody('idle');  asm.setEmotion('happy');     break;
      case 'sad':       asm.forceBody('idle');  asm.setEmotion('sad');       break;
      case 'angry':     asm.forceBody('idle');  asm.setEmotion('angry');     break;
      case 'relaxed':   asm.forceBody('idle');  asm.setEmotion('relaxed');   break;
      case 'surprised': asm.forceBody('idle');  asm.setEmotion('surprised'); break;
    }
  }

  /**
   * Check if all damped expression/bone values have converged to their targets
   * (within epsilon). Used for on-demand rendering — when settled, render rate drops.
   */
  isAnimationSettled(epsilon = 0.002): boolean {
    // Check AvatarStateMachine layer
    if (!this._asm.isSettled()) return false;
    // Check all expression channels
    for (let i = 0; i < EXPR_COUNT; i++) {
      if (Math.abs(this.exprCurrent[i] - this.exprTargets[i]) > epsilon) return false;
    }
    // Check all bone channels
    for (let i = 0; i < this.boneCurrentArr.length; i++) {
      if (Math.abs(this.boneCurrentArr[i] - this.boneTargetArr[i]) > epsilon) return false;
    }
    return true;
  }

  update(delta: number) {
    this.elapsed += delta;
    const t = this.elapsed;

    // Handle idle pose randomization when in idle state
    if (this.state === 'idle') {
      this.idlePoseChangeTime += delta;
      if (this.idlePoseChangeTime >= this.nextIdlePoseChangeAt) {
        this.selectNextIdlePose();
      }
    } else {
      // Reset idle pose timer when not in idle state
      this.idlePoseChangeTime = 0; 
      this.nextIdlePoseChangeAt = this.getRandomIdleInterval();
    }

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

  /**
   * Select the next idle pose randomly, avoiding repeating the same pose.
   * Updates the STATE_BONE_POSES.idle reference to point to the new pose.
   */
  private selectNextIdlePose() {
    let newIndex = this.currentIdlePoseIndex;
    // Ensure we don't repeat the same pose (unless there's only one pose)
    if (IDLE_POSES.length > 1) {
      do {
        newIndex = Math.floor(Math.random() * IDLE_POSES.length);
      } while (newIndex === this.currentIdlePoseIndex);
    }
    
    this.currentIdlePoseIndex = newIndex;
    STATE_BONE_POSES.idle = IDLE_POSES[newIndex];
    
    // Reset timer with some randomization
    this.idlePoseChangeTime = 0;
    this.nextIdlePoseChangeAt = this.getRandomIdleInterval();
  }

  /**
   * Get a random interval for the next idle pose change between 3-8 seconds.
   */
  private getRandomIdleInterval(): number {
    return 3 + Math.random() * 5; // 3-8 seconds for more frequent variation
  }

  // ── VRM animation (bones + expressions + blink) ────────────────────

  private applyVRMAnimation(t: number, delta: number) {
    if (!this.vrm || !this.vrmScene) return;

    // Pin scene root — only preserve the loader's base rotation
    this.vrmScene.position.set(0, 0, 0);
    this.vrmScene.rotation.set(0, this.baseRotationY, 0);

    // Tick the AvatarStateMachine's auto-blink cycle
    this._asm.tickBlink(delta);

    // Compute per-channel expression targets from current state
    this.computeExpressionTargets(t, delta);

    // Damp all expression channels toward their targets
    this.flushExpressions(delta);

    // Set bone pose targets for the current state (with idle breathing overlay)
    this.applyStateBonePose(t);

    // Damp bones toward their targets
    this.flushBones(delta);

    // vrm.update() transfers normalized bones → raw skeleton,
    // then updates lookAt, expressions, and spring bones.
    this.vrm.update(delta);
  }

  // ── Expression target computation (writes to flat exprTargets) ────────

  private computeExpressionTargets(_t: number, delta: number) {
    // Zero all targets
    this.exprTargets.fill(0);

    // ── Emotion layer: base expression weights from STATE_EXPRESSIONS ──
    const stateExprs = STATE_EXPRESSIONS[this.state];
    for (const [name, value] of Object.entries(stateExprs)) {
      const idx = EXPR_NAMES.indexOf(name);
      if (idx >= 0) this.exprTargets[idx] = value;
    }

    // ── Viseme layer: read from AvatarState mutable ref ───────────────
    const viseme = this._asm.state.viseme;
    if (this.state === 'talking') {
      if (this.useExternalLipSync) {
        this.exprTargets[EXPR_AA] = this.externalMouthAa;
        this.exprTargets[EXPR_OH] = this.externalMouthOh;
      } else if (viseme.aa > 0 || viseme.ih > 0 || viseme.ou > 0 ||
                 viseme.ee > 0 || viseme.oh > 0) {
        // AvatarState visemes take priority over procedural fallback
        this.exprTargets[EXPR_AA] = viseme.aa;
        this.exprTargets[EXPR_IH] = viseme.ih;
        this.exprTargets[EXPR_OU] = viseme.ou;
        this.exprTargets[EXPR_EE] = viseme.ee;
        this.exprTargets[EXPR_OH] = viseme.oh;
      } else {
        // Procedural sine-wave mouth fallback
        this.mouthElapsed += delta;
        const mouth = ((Math.sin(this.mouthElapsed * 5.5) + 1) * 0.5) * 0.5;
        this.exprTargets[EXPR_AA] = mouth;
      }
    }

    // ── Blink layer: read from AvatarState (driven by AvatarStateMachine.tickBlink)
    this.exprTargets[EXPR_BLINK] = this._asm.state.blink;
  }

  // ── State-based bone pose targets (with breathing overlay) ─────────

  private applyStateBonePose(t: number) {
    const pose = STATE_BONE_POSES[this.state] ?? STATE_BONE_POSES.idle;

    // Idle breathing cycle — subtle sine wave layered on all states
    // to keep the character feeling alive
    const breathCycle = Math.sin(t * 1.2);     // ~0.6 Hz breathing rate
    const breathAmt = 0.015;                   // subtle breath amplitude

    // Per-state additional movement overlays
    let headOscX = 0;
    let headOscY = 0;
    let headOscZ = 0;
    let spineOscX = 0;
    let spineOscY = 0;
    let hipsOscY = 0;
    let armLOscZ = 0;
    let armROscZ = 0;
    let lowerArmLOscZ = 0;
    let lowerArmROscZ = 0;
    let shoulderOscZ = 0;

    switch (this.state) {
      case 'idle': {
        // ── Multi-layered idle animation with pose-specific variations ──
        // Layer 1: Breathing — diaphragmatic breath cycle ~0.6Hz
        const breathPhase = t * 1.2;   // ~0.6 Hz
        const breathIn = Math.sin(breathPhase);
        // Layer 2: Weight shift — slow lateral hip/spine sway ~0.08Hz
        const shiftPhase = t * 0.16;
        const weightShift = Math.sin(shiftPhase);
        const weightShift2 = Math.cos(shiftPhase * 0.7);
        // Layer 3: Look-around — periodic slow head drift ~0.1Hz
        const lookPhase = t * 0.22;
        const lookY = Math.sin(lookPhase) * 0.6 + Math.sin(lookPhase * 2.3) * 0.4;
        const lookZ = Math.sin(lookPhase * 0.7 + 1.2) * 0.5;
        // Layer 4: Micro-fidget — very subtle fast noise for liveliness
        const fidgetX = Math.sin(t * 3.7) * 0.003;
        const fidgetY = Math.sin(t * 4.3) * 0.003;
        // Layer 5: Pose-specific modulation based on current pose index
        const poseVariation = Math.sin(t * 0.1 + this.currentIdlePoseIndex) * 0.02;

        headOscY = lookY * 0.04 + fidgetY + poseVariation;
        headOscZ = lookZ * 0.02;
        headOscX = fidgetX;
        spineOscX = breathIn * 0.003;
        spineOscY = weightShift * 0.012 + poseVariation * 0.5;
        hipsOscY = weightShift * 0.008 + weightShift2 * 0.005;
        armLOscZ = Math.sin(t * 0.3 + 0.5 + this.currentIdlePoseIndex) * 0.02;   // pose-aware arm sway
        armROscZ = Math.sin(t * 0.3 + 0.5 + this.currentIdlePoseIndex) * -0.02;
        lowerArmLOscZ = Math.sin(t * 0.25) * 0.015;
        lowerArmROscZ = Math.sin(t * 0.25) * -0.015;
        shoulderOscZ = breathIn * 0.008;               // shoulders rise on inhale
        break;
      }
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

    // Compose final bone targets: base pose + breathing + per-state oscillation + gesture blending
    const alpha = this.blender.getTransitionAlpha(t);
    const prevState = this.blender.getPreviousState();

    for (let i = 0; i < ANIMATED_BONES.length; i++) {
      const boneName = ANIMATED_BONES[i];
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
      } else if (boneName === 'leftUpperArm') {
        z += armLOscZ;
      } else if (boneName === 'rightUpperArm') {
        z += armROscZ;
      } else if (boneName === 'leftLowerArm') {
        z += lowerArmLOscZ;
      } else if (boneName === 'rightLowerArm') {
        z += lowerArmROscZ;
      } else if (boneName === 'leftShoulder') {
        z += shoulderOscZ;
      } else if (boneName === 'rightShoulder') {
        z += -shoulderOscZ;
      }

      const curOffset = this.blender.computeOffset(boneName, this.state, t);
      if (prevState !== null && alpha < 1) {
        const prevOffset = this.blender.computeOffset(boneName, prevState, t);
        x += prevOffset[0] * (1 - alpha) + curOffset[0] * alpha;
        y += prevOffset[1] * (1 - alpha) + curOffset[1] * alpha;
        z += prevOffset[2] * (1 - alpha) + curOffset[2] * alpha;
      } else {
        x += curOffset[0];
        y += curOffset[1];
        z += curOffset[2];
      }

      // ── Dress-aware arm clamping ──────────────────────────────────
      // Prevent hands / forearms from clipping through a flared dress.
      // Higher |Z| on upper arms = tighter to body = more clipping risk.
      // Applied after all composition so breathing, oscillations and
      // gesture noise are already folded in.  The soft clamp keeps the
      // motion looking natural rather than hitting a hard wall.
      if (boneName === 'leftUpperArm') {
        // Left upper arm: Z > 0 toward body; cap max to dress edge
        z = softClampMax(z, DRESS_UPPER_ARM_Z_MAX, DRESS_CLAMP_MARGIN);
      } else if (boneName === 'rightUpperArm') {
        // Right upper arm: Z < 0 toward body; |Z| must stay ≤ threshold
        z = -softClampMax(-z, DRESS_UPPER_ARM_Z_MAX, DRESS_CLAMP_MARGIN);
      } else if (boneName === 'leftLowerArm') {
        // Left lower arm: Z > 0 bends inward; limit max bend
        z = softClampMax(z, DRESS_LOWER_ARM_Z_MAX, DRESS_CLAMP_MARGIN);
      } else if (boneName === 'rightLowerArm') {
        // Right lower arm: Z < 0 bends inward; |Z| limited
        z = -softClampMax(-z, DRESS_LOWER_ARM_Z_MAX, DRESS_CLAMP_MARGIN);
      }

      this.boneTargetArr[i * BONE_STRIDE]     = x;
      this.boneTargetArr[i * BONE_STRIDE + 1] = y;
      this.boneTargetArr[i * BONE_STRIDE + 2] = z;
    }
  }

  // ── Exponential-damped bone interpolation (λ=6) ────────────────────

  private flushBones(delta: number) {
    if (!this.vrm) return;

    for (let i = 0; i < ANIMATED_BONES.length; i++) {
      const off = i * BONE_STRIDE;
      this.boneCurrentArr[off]     = damp(this.boneCurrentArr[off],     this.boneTargetArr[off],     BONE_LAMBDA, delta);
      this.boneCurrentArr[off + 1] = damp(this.boneCurrentArr[off + 1], this.boneTargetArr[off + 1], BONE_LAMBDA, delta);
      this.boneCurrentArr[off + 2] = damp(this.boneCurrentArr[off + 2], this.boneTargetArr[off + 2], BONE_LAMBDA, delta);

      // Apply to VRM humanoid normalized bone via the standard API
      const node = this.vrm.humanoid?.getNormalizedBoneNode(ANIMATED_BONES[i]);
      if (node) {
        node.rotation.set(
          this.boneCurrentArr[off],
          this.boneCurrentArr[off + 1],
          this.boneCurrentArr[off + 2],
        );
      }
    }
  }

  // ── Exponential-damped expression interpolation (per-channel λ) ────

  /**
   * Damp all expression channels toward their targets each frame.
   * Uses per-channel lambda from EXPR_LAMBDAS for differentiated smoothing.
   */
  private flushExpressions(delta: number) {
    for (let i = 0; i < EXPR_COUNT; i++) {
      this.exprCurrent[i] = damp(this.exprCurrent[i], this.exprTargets[i], EXPR_LAMBDAS[i], delta);
      try {
        this.vrm?.expressionManager?.setValue(EXPR_NAMES[i], this.exprCurrent[i]);
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
