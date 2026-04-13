import * as THREE from 'three';
import type { VRM, VRMHumanBoneName } from '@pixiv/three-vrm';
import type { AnimationPersona, CharacterState } from '../types';
import { setNaturalBonePose } from './vrm-loader';

/**
 * Smooth interpolation helper — lerps a value toward a target each frame.
 * This produces exponential ease-out, which feels natural.
 */
function smoothStep(current: number, target: number, speed: number, delta: number): number {
  return current + (target - current) * Math.min(1, speed * delta);
}

export class CharacterAnimator {
  private vrm: VRM | null = null;
  private vrmScene: THREE.Object3D | null = null;
  private placeholder: THREE.Group | null = null;
  private state: CharacterState = 'idle';
  private elapsed = 0;
  private baseRotationY = 0;
  private persona: AnimationPersona = 'cool';
  private skipBonePose = false;

  // Blink timing constants
  private static readonly BLINK_DURATION = 0.15;
  private static readonly MIN_BLINK_INTERVAL = 2.0;
  private static readonly MAX_BLINK_INTERVAL = 6.0;

  // Smooth blink state
  private nextBlinkTime = CharacterAnimator.MIN_BLINK_INTERVAL + Math.random() * (CharacterAnimator.MAX_BLINK_INTERVAL - CharacterAnimator.MIN_BLINK_INTERVAL);
  private blinkValue = 0;
  private isBlinking = false;
  private blinkTimer = 0;

  // Smooth expression targets (interpolated each frame)
  private expressionTargets: Map<string, number> = new Map();
  private expressionCurrent: Map<string, number> = new Map();

  setVRM(vrm: VRM, rotationY = 0, persona: AnimationPersona = 'cool', skipBonePose = false) {
    this.vrm = vrm;
    this.vrmScene = vrm.scene;
    this.baseRotationY = rotationY;
    this.persona = persona;
    this.skipBonePose = skipBonePose;
    this.placeholder = null;
    // Reset blink timing
    this.nextBlinkTime = CharacterAnimator.MIN_BLINK_INTERVAL + Math.random() * (CharacterAnimator.MAX_BLINK_INTERVAL - CharacterAnimator.MIN_BLINK_INTERVAL);
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
   * Animate the VRM model using bone-level transforms and smooth expressions.
   * This replaces the old approach of moving the entire scene root, which
   * looked artificial. Instead, individual bones (spine, chest, head, arms)
   * are manipulated for natural breathing, swaying, and gestures.
   */
  private applyVRMAnimation(t: number, delta: number) {
    if (!this.vrm || !this.vrmScene) return;

    if (!this.skipBonePose) {
      setNaturalBonePose(this.vrm);
    }

    // Reset scene root each frame — only apply rotation, no root movement
    this.vrmScene.position.set(0, 0, 0);
    this.vrmScene.rotation.set(0, this.baseRotationY, 0);

    // Natural blinking with random intervals
    this.updateBlink(t, delta);

    if (this.persona === 'cute') {
      this.applyCuteAnimation(t, delta);
    } else {
      this.applyCoolAnimation(t, delta);
    }

    // Smoothly interpolate all expressions toward their targets
    this.flushExpressions(delta);

    this.vrm.update(delta);
  }

  // ── Natural blink system with random timing ────────────────────────

  private updateBlink(_t: number, delta: number) {
    if (!this.isBlinking) {
      // Count down to next blink
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
        // Closing
        this.blinkValue = this.blinkTimer / half;
      } else if (this.blinkTimer < CharacterAnimator.BLINK_DURATION) {
        // Opening
        this.blinkValue = 1.0 - (this.blinkTimer - half) / half;
      } else {
        // Done blinking
        this.blinkValue = 0;
        this.isBlinking = false;
        // Random interval until next blink
        this.nextBlinkTime = CharacterAnimator.MIN_BLINK_INTERVAL + Math.random() * (CharacterAnimator.MAX_BLINK_INTERVAL - CharacterAnimator.MIN_BLINK_INTERVAL);
      }
    }

    this.setExpressionTarget('blink', this.blinkValue);
  }

  // ── Bone helpers ───────────────────────────────────────────────────

  private getBone(name: VRMHumanBoneName): THREE.Object3D | null {
    return this.vrm?.humanoid?.getNormalizedBoneNode(name) ?? null;
  }

  /**
   * Additively rotate a bone by small offsets, preserving the natural pose
   * that was set by setNaturalBonePose(). This is the key to smooth
   * bone-level animation without jerky hard-sets.
   */
  private addBoneRotation(name: VRMHumanBoneName, x: number, y: number, z: number) {
    const bone = this.getBone(name);
    if (bone) {
      bone.rotation.x += x;
      bone.rotation.y += y;
      bone.rotation.z += z;
    }
  }

  // ── Cool persona: smooth, confident, minimal movement ──────────────

  private applyCoolAnimation(t: number, _delta: number) {
    if (!this.vrmScene) return;

    // Subtle idle breathing on spine/chest bones (always active)
    const breathAmt = Math.sin(t * 1.2) * 0.008;
    this.addBoneRotation('spine', breathAmt, 0, 0);
    this.addBoneRotation('chest', breathAmt * 0.5, 0, 0);

    switch (this.state) {
      case 'idle': {
        // Minimal weight shift — gentle spine sway
        this.addBoneRotation('spine', 0, Math.sin(t * 0.3) * 0.01, 0);
        // Slight head movement — looking around subtly
        this.addBoneRotation('head', Math.sin(t * 0.5) * 0.01, Math.sin(t * 0.35) * 0.02, 0);
        this.clearExpressionTargets();
        this.setExpressionTarget('relaxed', 0.4);
        break;
      }
      case 'thinking': {
        // Deliberate head tilt, like considering
        this.addBoneRotation('head', Math.sin(t * 0.5) * 0.03, Math.sin(t * 0.4) * 0.08, Math.sin(t * 0.5) * 0.04);
        this.addBoneRotation('spine', 0, Math.sin(t * 0.3) * 0.02, 0);
        this.clearExpressionTargets();
        break;
      }
      case 'talking': {
        // Measured nods with controlled lip sync
        this.addBoneRotation('head', Math.sin(t * 1.5) * 0.02, Math.sin(t * 0.7) * 0.04, 0);
        this.addBoneRotation('spine', 0, Math.sin(t * 0.5) * 0.015, 0);
        this.clearExpressionTargets();
        this.setExpressionTarget('aa', ((Math.sin(t * 6.0) + 1.0) * 0.5) * 0.5);
        this.setExpressionTarget('oh', ((Math.cos(t * 4.5) + 1.0) * 0.5) * 0.2);
        break;
      }
      case 'happy': {
        // Confident lean back, head lifts
        this.addBoneRotation('spine', -0.03 + Math.sin(t * 1.2) * 0.01, Math.sin(t * 0.8) * 0.03, 0);
        this.addBoneRotation('head', -0.02, Math.sin(t * 0.6) * 0.03, 0);
        this.clearExpressionTargets();
        this.setExpressionTarget('happy', 0.6);
        this.setExpressionTarget('relaxed', 0.3);
        break;
      }
      case 'sad': {
        // Stoic disappointment — head droops, spine curves
        this.addBoneRotation('spine', 0.04, 0, Math.sin(t * 0.25) * 0.005);
        this.addBoneRotation('head', 0.06, 0, Math.sin(t * 0.3) * 0.01);
        this.clearExpressionTargets();
        this.setExpressionTarget('sad', 0.5);
        break;
      }
    }
  }

  // ── Cute persona: bouncy, expressive, lots of energy ───────────────

  private applyCuteAnimation(t: number, _delta: number) {
    if (!this.vrmScene) return;

    // Breathing — slightly more visible for cute persona
    const breathAmt = Math.sin(t * 1.5) * 0.012;
    this.addBoneRotation('spine', breathAmt, 0, 0);
    this.addBoneRotation('chest', breathAmt * 0.6, 0, 0);

    switch (this.state) {
      case 'idle': {
        // Gentle swaying side-to-side
        this.addBoneRotation('spine', 0, Math.sin(t * 0.7) * 0.03, Math.sin(t * 0.9) * 0.02);
        this.addBoneRotation('head', 0, Math.sin(t * 0.6) * 0.03, Math.sin(t * 0.8) * 0.02);
        this.clearExpressionTargets();
        this.setExpressionTarget('relaxed', 0.5);
        this.setExpressionTarget('happy', 0.2);
        break;
      }
      case 'thinking': {
        // Cute confused head tilts
        this.addBoneRotation('head', Math.sin(t * 1.0) * 0.04, Math.sin(t * 0.8) * 0.1, Math.sin(t * 1.2) * 0.06);
        this.addBoneRotation('spine', 0, Math.sin(t * 0.6) * 0.025, 0);
        this.clearExpressionTargets();
        this.setExpressionTarget('oh', 0.4);
        break;
      }
      case 'talking': {
        // Energetic bobbing with exaggerated mouth
        this.addBoneRotation('head', Math.sin(t * 2.5) * 0.03, Math.sin(t * 1.8) * 0.06, Math.sin(t * 2.0) * 0.02);
        this.addBoneRotation('spine', Math.sin(t * 3.5) * 0.015, Math.sin(t * 1.5) * 0.02, 0);
        this.clearExpressionTargets();
        this.setExpressionTarget('aa', ((Math.sin(t * 9.0) + 1.0) * 0.5) * 0.8);
        this.setExpressionTarget('oh', ((Math.cos(t * 7.0) + 1.0) * 0.5) * 0.4);
        this.setExpressionTarget('happy', 0.3);
        break;
      }
      case 'happy': {
        // Super bouncy! Big smile, celebratory head bobbing
        this.addBoneRotation('spine', Math.abs(Math.sin(t * 5.0)) * 0.04, Math.sin(t * 3.0) * 0.06, Math.sin(t * 4.0) * 0.04);
        this.addBoneRotation('head', -0.03, Math.sin(t * 2.5) * 0.05, Math.sin(t * 3.0) * 0.03);
        this.clearExpressionTargets();
        this.setExpressionTarget('happy', 1.0);
        this.setExpressionTarget('aa', 0.4 + Math.sin(t * 4.0) * 0.2);
        break;
      }
      case 'sad': {
        // Droopy, pouty — clearly upset
        this.addBoneRotation('spine', 0.06, 0, Math.sin(t * 0.4) * 0.01);
        this.addBoneRotation('head', 0.08, 0, Math.sin(t * 0.5) * 0.015);
        this.clearExpressionTargets();
        this.setExpressionTarget('sad', 1.0);
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
    const expressionSpeed = 8.0; // how fast expressions blend
    for (const [name, target] of this.expressionTargets) {
      const current = this.expressionCurrent.get(name) ?? 0;
      const next = smoothStep(current, target, expressionSpeed, delta);
      this.expressionCurrent.set(name, next);
      try {
        this.vrm?.expressionManager?.setValue(name, next);
      } catch { /* expression not available on this model */ }
    }
  }

  // ── Placeholder animation (same as before) ────────────────────────

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
