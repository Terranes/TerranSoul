import * as THREE from 'three';
import type { VRM, VRMHumanBoneName } from '@pixiv/three-vrm';
import type { AnimationPersona, CharacterState } from '../types';

/**
 * Smooth interpolation helper — lerps a value toward a target each frame.
 * This produces exponential ease-out, which feels natural.
 */
function smoothStep(current: number, target: number, speed: number, delta: number): number {
  return current + (target - current) * Math.min(1, speed * delta);
}

/**
 * VRoid Hub–style VRM animator.
 *
 * Design principles (reverse-engineered from VRoid Hub character viewer and
 * official @pixiv/three-vrm examples):
 *
 * 1. **Direct assignment, not additive.**  Every animated bone is set via
 *    `.rotation.set(x, y, z)` each frame — never `+=`.  This eliminates the
 *    rotation-accumulation bug where bones that weren't explicitly reset
 *    would spin/fly over time.
 *
 * 2. **Minimal idle.**  VRoid Hub shows models virtually static — only very
 *    subtle breathing (spine/chest) and random blinks.  LookAt handles eye
 *    tracking.  No hip sway, no weight shifts, no head look-around.
 *
 * 3. **Expression-driven reactions.**  States like talking, happy, sad are
 *    conveyed primarily through VRM expressions (blendShapes), with only
 *    very small bone adjustments (head tilt for thinking, slight nod when
 *    talking).
 *
 * 4. **autoUpdateHumanBones = true** (library default).  `vrm.update(delta)`
 *    transfers normalized bone rotations to the raw skeleton automatically.
 */
export class CharacterAnimator {
  private vrm: VRM | null = null;
  private vrmScene: THREE.Object3D | null = null;
  private placeholder: THREE.Group | null = null;
  private state: CharacterState = 'idle';
  private elapsed = 0;
  private baseRotationY = 0;
  private persona: AnimationPersona = 'cool';

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

  private static randomBlinkInterval(): number {
    return CharacterAnimator.MIN_BLINK_INTERVAL +
      Math.random() * (CharacterAnimator.MAX_BLINK_INTERVAL - CharacterAnimator.MIN_BLINK_INTERVAL);
  }

  setVRM(vrm: VRM, rotationY = 0, persona: AnimationPersona = 'cool') {
    this.vrm = vrm;
    this.vrmScene = vrm.scene;
    this.baseRotationY = rotationY;
    this.persona = persona;
    this.placeholder = null;
    // Reset blink timing
    this.nextBlinkTime = CharacterAnimator.randomBlinkInterval();
    this.blinkValue = 0;
    this.isBlinking = false;
    this.blinkTimer = 0;
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
    this.state = state;
    this.elapsed = 0;
  }

  getState(): CharacterState {
    return this.state;
  }

  getPersona(): AnimationPersona {
    return this.persona;
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
   * VRoid Hub–style VRM animation.
   *
   * Every bone is set via direct assignment (.rotation.set) each frame so
   * no rotation accumulates.  The natural arm-down pose is baked into the
   * arm bone values.  Breathing and state-specific adjustments are layered
   * on top via simple sin() offsets.
   */
  private applyVRMAnimation(t: number, delta: number) {
    if (!this.vrm || !this.vrmScene) return;

    // Pin scene root — only preserve the loader's base rotation (e.g. VRM 0.x π flip)
    this.vrmScene.position.set(0, 0, 0);
    this.vrmScene.rotation.set(0, this.baseRotationY, 0);

    // Natural blinking with random intervals
    this.updateBlink(delta);

    // ── Breathing (always active, matches VRoid Hub's subtle chest rise/fall) ──
    const breathAmt = Math.sin(t * 1.2) * 0.015;
    this.setBone('spine', breathAmt, 0, 0);
    this.setBone('chest', breathAmt * 0.6, 0, 0);

    // ── Natural arm-down pose ──
    // VRM T-pose has arms horizontal; rotate upper arms ~70° down toward body.
    // These are direct-set each frame so they never accumulate.
    this.setBone('leftUpperArm', 0, 0, 1.1);
    this.setBone('rightUpperArm', 0, 0, -1.1);
    this.setBone('leftLowerArm', 0, 0, 0.15);
    this.setBone('rightLowerArm', 0, 0, -0.15);
    this.setBone('leftShoulder', 0, 0, 0.05);
    this.setBone('rightShoulder', 0, 0, -0.05);

    // ── Zero out bones that states may tweak (prevents stale values) ──
    this.setBone('head', 0, 0, 0);
    this.setBone('neck', 0, 0, 0);
    this.setBone('hips', 0, 0, 0);
    this.setBone('upperChest', 0, 0, 0);

    // ── State-specific overlays ──
    // Only small adjustments — expressions carry most of the emotional weight.
    this.applyStateAnimation(t);

    // Smoothly interpolate all expressions toward their targets
    this.flushExpressions(delta);

    // vrm.update() transfers normalized bones → raw skeleton (autoUpdateHumanBones=true),
    // then updates lookAt, expressions, and spring bones.
    this.vrm.update(delta);
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

  // ── Bone helper — direct assignment, never additive ────────────────

  /**
   * Set a normalized bone's rotation directly.
   * This replaces whatever value was there — no accumulation across frames.
   */
  private setBone(name: VRMHumanBoneName, x: number, y: number, z: number) {
    const bone = this.vrm?.humanoid?.getNormalizedBoneNode(name);
    if (bone) {
      bone.rotation.set(x, y, z);
    }
  }

  // ── State animation (minimal, VRoid Hub–inspired) ──────────────────

  private applyStateAnimation(t: number) {
    // Expressions are the primary reaction channel.
    // Bone adjustments are kept tiny — just enough to feel alive.
    this.clearExpressionTargets();

    switch (this.state) {
      case 'idle': {
        // VRoid Hub idle: virtually motionless.
        // A barely-perceptible head micro-motion prevents the "frozen" look.
        this.setBone('head', Math.sin(t * 0.3) * 0.008, Math.sin(t * 0.2) * 0.008, 0);
        this.setExpressionTarget('relaxed', 0.3);
        break;
      }
      case 'thinking': {
        // Slight head tilt — like pondering
        this.setBone('head', Math.sin(t * 0.4) * 0.02, 0.06, Math.sin(t * 0.5) * 0.03);
        this.setBone('neck', 0.02, 0, 0);
        break;
      }
      case 'talking': {
        // Gentle nods during speech
        this.setBone('head', Math.sin(t * 1.5) * 0.02, Math.sin(t * 0.6) * 0.015, 0);
        this.setBone('neck', Math.sin(t * 1.8) * 0.008, 0, 0);
        this.setExpressionTarget('aa', ((Math.sin(t * 6.0) + 1) * 0.5) * 0.5);
        break;
      }
      case 'happy': {
        // Slight upward head tilt, happy expression
        this.setBone('head', -0.03, Math.sin(t * 0.5) * 0.015, 0);
        this.setExpressionTarget('happy', 0.7);
        this.setExpressionTarget('relaxed', 0.2);
        break;
      }
      case 'sad': {
        // Head droops, subtle swaying
        this.setBone('head', 0.06, 0, Math.sin(t * 0.3) * 0.01);
        this.setBone('neck', 0.02, 0, 0);
        this.setBone('spine', 0.03 + Math.sin(t * 1.2) * 0.015, 0, 0);
        this.setExpressionTarget('sad', 0.6);
        break;
      }
    }
  }

  // ── Smooth expression system ───────────────────────────────────────

  private setExpressionTarget(name: string, value: number) {
    this.expressionTargets.set(name, value);
  }

  private clearExpressionTargets() {
    for (const name of ['aa', 'oh', 'happy', 'sad', 'angry', 'relaxed']) {
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
    }
  }
}
