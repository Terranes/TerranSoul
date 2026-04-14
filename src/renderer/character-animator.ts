import * as THREE from 'three';
import type { VRM } from '@pixiv/three-vrm';
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

/**
 * VRM animator that drives facial expressions and blinking.
 *
 * Body/bone animation is expected to come from external VRMA or Mixamo
 * animation clips loaded at runtime — this class does **not** generate
 * procedural bone keyframes.
 *
 * Expressions (morph targets) and blinking are procedural.
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

  // ── VRM animation (expressions + blink) ────────────────────────────

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
