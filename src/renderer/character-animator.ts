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
 * VRoid Hub–inspired VRM animator with persona-specific animations.
 *
 * Each character persona has unique idle behaviour and state reactions
 * that match their personality:
 *
 * - **witch** (Annabelle): slow, deliberate, mystical gestures.
 *   Head tilts curiously, gentle sway, one hand occasionally rises as if
 *   channelling energy. Composed and studious.
 *
 * - **idol** (M58): bouncy, friendly, energetic. Slight rhythmic sway,
 *   head bobs side to side, cheerful micro-bounces. Very expressive.
 *
 * - **fashionista** (Miyoura): confident posture, hip-shift weight sway,
 *   occasional head toss. Cool and stylish.
 *
 * - **gentleman** (Nogami): minimal, composed, stoic. Very slight breathing,
 *   rare slow head turns. Almost statue-like calm.
 *
 * All bones use direct `.rotation.set()` each frame — never `+=`.
 */
export class CharacterAnimator {
  private vrm: VRM | null = null;
  private vrmScene: THREE.Object3D | null = null;
  private placeholder: THREE.Group | null = null;
  private state: CharacterState = 'idle';
  private elapsed = 0;
  private baseRotationY = 0;
  private persona: AnimationPersona = 'gentleman';

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

  setVRM(vrm: VRM, rotationY = 0, persona: AnimationPersona = 'gentleman') {
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
   * VRoid Hub–style VRM animation with persona-specific behaviour.
   *
   * Every bone is set via direct assignment (.rotation.set) each frame so
   * no rotation accumulates.
   */
  private applyVRMAnimation(t: number, delta: number) {
    if (!this.vrm || !this.vrmScene) return;

    // Pin scene root — only preserve the loader's base rotation
    this.vrmScene.position.set(0, 0, 0);
    this.vrmScene.rotation.set(0, this.baseRotationY, 0);

    // Natural blinking with random intervals
    this.updateBlink(delta);

    // Apply persona-specific idle & state animation
    this.applyPersonaAnimation(t);

    // Smoothly interpolate all expressions toward their targets
    this.flushExpressions(delta);

    // vrm.update() transfers normalized bones → raw skeleton,
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

  private setBone(name: VRMHumanBoneName, x: number, y: number, z: number) {
    const bone = this.vrm?.humanoid?.getNormalizedBoneNode(name);
    if (bone) {
      bone.rotation.set(x, y, z);
    }
  }

  // ── Persona-specific animation dispatcher ──────────────────────────

  private applyPersonaAnimation(t: number) {
    // Clear expressions each frame — state methods set new targets
    this.clearExpressionTargets();

    switch (this.persona) {
      case 'witch':
        this.animateWitch(t);
        break;
      case 'idol':
        this.animateIdol(t);
        break;
      case 'fashionista':
        this.animateFashionista(t);
        break;
      case 'gentleman':
        this.animateGentleman(t);
        break;
    }
  }

  // ══════════════════════════════════════════════════════════════════════
  //  WITCH — Annabelle: studious, mystical, composed with slow gestures
  // ══════════════════════════════════════════════════════════════════════

  private animateWitch(t: number) {
    // ── Breathing — deep, slow, mystical ──
    const breath = Math.sin(t * 0.9) * 0.06;
    this.setBone('spine', breath, 0, 0);
    this.setBone('chest', breath * 0.7, 0, 0);

    // ── Base arm pose — relaxed ──
    this.setBone('leftUpperArm', 0, 0, 1.2);
    this.setBone('rightUpperArm', 0, 0, -1.2);
    this.setBone('leftLowerArm', 0, 0, 0.15);
    this.setBone('rightLowerArm', 0, 0, -0.15);
    this.setBone('leftShoulder', 0, 0, 0.05);
    this.setBone('rightShoulder', 0, 0, -0.05);

    // Defaults for bones that states override
    this.setBone('hips', 0, 0, 0);
    this.setBone('neck', 0, 0, 0);
    this.setBone('head', 0, 0, 0);
    this.setBone('upperChest', 0, 0, 0);

    switch (this.state) {
      case 'idle': {
        // Slow, curious head tilts — like reading an ancient tome
        const headX = Math.sin(t * 0.35) * 0.08;
        const headY = Math.sin(t * 0.2) * 0.12;
        const headZ = Math.sin(t * 0.28) * 0.05;
        this.setBone('head', headX, headY, headZ);
        this.setBone('neck', Math.sin(t * 0.25) * 0.04, Math.sin(t * 0.15) * 0.03, 0);
        // Body sway — mystical energy flowing through
        const sway = Math.sin(t * 0.3) * 0.04;
        this.setBone('spine', breath + sway * 0.5, Math.sin(t * 0.18) * 0.03, sway);
        this.setBone('hips', 0, Math.sin(t * 0.15) * 0.02, sway * 0.6);
        this.setBone('upperChest', Math.sin(t * 0.4) * 0.02, 0, -sway * 0.3);
        // Right arm lifts periodically as if channelling energy
        const armCycle = (Math.sin(t * 0.22) * 0.5 + 0.5);
        const armLift = armCycle * 0.35;
        this.setBone('rightUpperArm', -armLift * 0.4, 0, -1.2 + armLift);
        this.setBone('rightLowerArm', -armCycle * 0.15, 0, -0.15 - armLift * 0.6);
        // Left arm subtle pendulum
        this.setBone('leftUpperArm', Math.sin(t * 0.3) * 0.04, 0, 1.2 + Math.sin(t * 0.25) * 0.03);
        this.setExpressionTarget('relaxed', 0.3);
        break;
      }
      case 'thinking': {
        // Scholarly contemplation — head tilts deeply, hand near chin
        this.setBone('head', 0.1, 0.15, Math.sin(t * 0.5) * 0.08);
        this.setBone('neck', 0.06, Math.sin(t * 0.3) * 0.05, 0);
        this.setBone('spine', breath, Math.sin(t * 0.2) * 0.04, 0);
        this.setBone('hips', 0, 0, Math.sin(t * 0.3) * 0.02);
        // Right arm up near face — thinking pose
        this.setBone('rightUpperArm', -0.5, 0.15, -0.6);
        this.setBone('rightLowerArm', -0.4, 0, -0.8);
        this.setExpressionTarget('neutral', 0.3);
        break;
      }
      case 'talking': {
        // Explaining magic — animated head, gesturing hand
        this.setBone('head', Math.sin(t * 1.5) * 0.08, Math.sin(t * 0.8) * 0.1, 0);
        this.setBone('neck', Math.sin(t * 1.8) * 0.04, Math.sin(t * 0.6) * 0.03, 0);
        this.setBone('spine', breath, Math.sin(t * 0.5) * 0.04, Math.sin(t * 0.7) * 0.02);
        this.setBone('hips', 0, 0, Math.sin(t * 0.6) * 0.02);
        // Right arm gestures while explaining
        const gesture = Math.sin(t * 1.2) * 0.25;
        this.setBone('rightUpperArm', -0.35 + gesture, 0, -0.8);
        this.setBone('rightLowerArm', gesture * 0.4, 0, -0.3 + Math.sin(t * 1.5) * 0.1);
        this.setExpressionTarget('aa', ((Math.sin(t * 5.5) + 1) * 0.5) * 0.5);
        this.setExpressionTarget('relaxed', 0.15);
        break;
      }
      case 'happy': {
        // Pleased discovery — lean back, warm smile, arms lift
        this.setBone('head', -0.1, Math.sin(t * 0.6) * 0.06, 0);
        this.setBone('neck', -0.05, 0, 0);
        this.setBone('spine', breath - 0.06, 0, Math.sin(t * 0.8) * 0.03);
        this.setBone('hips', 0, 0, Math.sin(t * 0.5) * 0.02);
        // Arms spread slightly in delight
        this.setBone('leftUpperArm', -0.08, 0, 1.1 + Math.sin(t * 0.7) * 0.05);
        this.setBone('rightUpperArm', -0.08, 0, -1.1 - Math.sin(t * 0.7) * 0.05);
        this.setExpressionTarget('happy', 0.65);
        this.setExpressionTarget('relaxed', 0.25);
        break;
      }
      case 'sad': {
        // Worried about a failed spell — droops forward
        this.setBone('head', 0.15, 0, Math.sin(t * 0.25) * 0.04);
        this.setBone('neck', 0.08, 0, 0);
        this.setBone('spine', breath + 0.1, 0, 0);
        this.setBone('hips', 0, 0, Math.sin(t * 0.2) * 0.015);
        this.setBone('leftShoulder', 0.06, 0, 0.08);
        this.setBone('rightShoulder', 0.06, 0, -0.08);
        this.setExpressionTarget('sad', 0.6);
        break;
      }
    }
  }

  // ══════════════════════════════════════════════════════════════════════
  //  IDOL — M58: cute, bouncy, friendly, BTS-vibes
  // ══════════════════════════════════════════════════════════════════════

  private animateIdol(t: number) {
    // ── Breathing — quicker, energetic ──
    const breath = Math.sin(t * 1.5) * 0.05;
    this.setBone('spine', breath, 0, 0);
    this.setBone('chest', breath * 0.8, 0, 0);

    // ── Arms — relaxed but ready ──
    this.setBone('leftUpperArm', 0, 0, 1.2);
    this.setBone('rightUpperArm', 0, 0, -1.2);
    this.setBone('leftLowerArm', 0, 0, 0.15);
    this.setBone('rightLowerArm', 0, 0, -0.15);
    this.setBone('leftShoulder', 0, 0, 0.05);
    this.setBone('rightShoulder', 0, 0, -0.05);
    this.setBone('hips', 0, 0, 0);
    this.setBone('neck', 0, 0, 0);
    this.setBone('head', 0, 0, 0);
    this.setBone('upperChest', 0, 0, 0);

    switch (this.state) {
      case 'idle': {
        // Rhythmic sway — side to side like grooving to music
        const sway = Math.sin(t * 0.8) * 0.07;
        const bounce = Math.abs(Math.sin(t * 1.6)) * 0.03;
        this.setBone('hips', 0, Math.sin(t * 0.6) * 0.04, sway);
        this.setBone('spine', breath + bounce, 0, -sway * 0.4);
        this.setBone('chest', breath * 0.8, 0, -sway * 0.2);
        this.setBone('upperChest', 0, 0, -sway * 0.15);
        // Head bobs — friendly curiosity
        this.setBone('head',
          Math.sin(t * 0.7) * 0.07 + bounce,
          Math.sin(t * 0.5) * 0.12,
          Math.sin(t * 0.9) * 0.07,
        );
        this.setBone('neck', Math.sin(t * 0.55) * 0.04, Math.sin(t * 0.45) * 0.05, 0);
        // Arms swing with body sway — cute arm pendulums
        this.setBone('leftUpperArm', Math.sin(t * 0.8) * 0.08, 0, 1.2 + sway * 0.4);
        this.setBone('rightUpperArm', Math.sin(t * 0.8 + 0.5) * 0.08, 0, -1.2 + sway * 0.4);
        this.setBone('leftLowerArm', 0, 0, 0.15 + Math.sin(t * 0.6) * 0.05);
        this.setBone('rightLowerArm', 0, 0, -0.15 - Math.sin(t * 0.6 + 0.3) * 0.05);
        this.setExpressionTarget('happy', 0.25);
        this.setExpressionTarget('relaxed', 0.2);
        break;
      }
      case 'thinking': {
        // Cute confusion — head tilts far, pout
        this.setBone('head', Math.sin(t * 0.6) * 0.08, 0.2, Math.sin(t * 0.8) * 0.12);
        this.setBone('neck', 0.05, Math.sin(t * 0.4) * 0.08, 0);
        this.setBone('hips', 0, 0, Math.sin(t * 0.5) * 0.03);
        this.setBone('spine', breath, 0, Math.sin(t * 0.4) * 0.02);
        this.setExpressionTarget('oh', 0.4);
        break;
      }
      case 'talking': {
        // Animated chatting — bobbing, very expressive
        const bob = Math.sin(t * 2.0) * 0.06;
        this.setBone('head', bob, Math.sin(t * 1.0) * 0.12, Math.sin(t * 1.5) * 0.06);
        this.setBone('neck', Math.sin(t * 2.2) * 0.03, Math.sin(t * 0.8) * 0.04, 0);
        this.setBone('spine', breath + bob * 0.5, 0, Math.sin(t * 0.8) * 0.03);
        this.setBone('hips', 0, 0, Math.sin(t * 1.0) * 0.04);
        // Hand gestures while talking — active gesturing
        this.setBone('rightUpperArm', Math.sin(t * 1.5) * 0.2 - 0.2, 0, -1.0);
        this.setBone('rightLowerArm', Math.sin(t * 1.8) * 0.15, 0, -0.2 + Math.sin(t * 2.0) * 0.08);
        this.setBone('leftUpperArm', Math.sin(t * 1.3) * 0.1, 0, 1.1);
        this.setExpressionTarget('aa', ((Math.sin(t * 7.0) + 1) * 0.5) * 0.6);
        this.setExpressionTarget('happy', 0.2);
        break;
      }
      case 'happy': {
        // Excited bounce — big smile, energetic jumping vibes
        const bounce = Math.abs(Math.sin(t * 3.0)) * 0.05;
        this.setBone('spine', breath + bounce, 0, Math.sin(t * 2.0) * 0.07);
        this.setBone('chest', breath * 0.8 + bounce * 0.5, 0, 0);
        this.setBone('head', -0.1 + bounce, Math.sin(t * 1.5) * 0.1, Math.sin(t * 2.0) * 0.08);
        this.setBone('hips', 0, 0, Math.sin(t * 2.5) * 0.06);
        // Arms swing wider — celebratory wave
        this.setBone('leftUpperArm', -0.3, 0, 0.9 + Math.sin(t * 2.0) * 0.15);
        this.setBone('rightUpperArm', -0.3, 0, -0.9 - Math.sin(t * 2.0 + 0.5) * 0.15);
        this.setBone('leftLowerArm', 0, 0, 0.2 + Math.sin(t * 2.5) * 0.1);
        this.setBone('rightLowerArm', 0, 0, -0.2 - Math.sin(t * 2.5 + 0.3) * 0.1);
        this.setExpressionTarget('happy', 0.9);
        break;
      }
      case 'sad': {
        // Puppy-eyes sad — shoulders droop, head hangs
        this.setBone('head', 0.15, 0, Math.sin(t * 0.3) * 0.05);
        this.setBone('neck', 0.08, 0, 0);
        this.setBone('spine', breath + 0.08, 0, 0);
        this.setBone('hips', 0, 0, Math.sin(t * 0.25) * 0.015);
        this.setBone('leftShoulder', 0.08, 0, 0.1);
        this.setBone('rightShoulder', 0.08, 0, -0.1);
        this.setExpressionTarget('sad', 0.8);
        break;
      }
    }
  }

  // ══════════════════════════════════════════════════════════════════════
  //  FASHIONISTA — Miyoura: cool, confident, gen Z attitude
  // ══════════════════════════════════════════════════════════════════════

  private animateFashionista(t: number) {
    // ── Breathing — relaxed, confident ──
    const breath = Math.sin(t * 1.1) * 0.05;
    this.setBone('spine', breath, 0, 0);
    this.setBone('chest', breath * 0.6, 0, 0);

    // ── Arms — slightly away from body, attitude ──
    this.setBone('leftUpperArm', 0, 0, 1.15);
    this.setBone('rightUpperArm', 0, 0, -1.15);
    this.setBone('leftLowerArm', 0, 0, 0.18);
    this.setBone('rightLowerArm', 0, 0, -0.18);
    this.setBone('leftShoulder', 0, 0, 0.05);
    this.setBone('rightShoulder', 0, 0, -0.05);
    this.setBone('hips', 0, 0, 0);
    this.setBone('neck', 0, 0, 0);
    this.setBone('head', 0, 0, 0);
    this.setBone('upperChest', 0, 0, 0);

    switch (this.state) {
      case 'idle': {
        // Hip-shift weight sway — model-like contrapposto stance
        const hipSway = Math.sin(t * 0.5) * 0.08;
        this.setBone('hips', 0, Math.sin(t * 0.35) * 0.04, hipSway);
        this.setBone('spine', breath, 0, -hipSway * 0.35);
        this.setBone('chest', breath * 0.6, 0, -hipSway * 0.2);
        this.setBone('upperChest', 0, 0, -hipSway * 0.15);
        // Confident head — slow turns, chin slightly up
        this.setBone('head',
          -0.04 + Math.sin(t * 0.3) * 0.04,
          Math.sin(t * 0.22) * 0.15,
          Math.sin(t * 0.4) * 0.06,
        );
        this.setBone('neck', -0.03, Math.sin(t * 0.18) * 0.06, 0);
        // Weight on one leg — hip sway affects arms
        this.setBone('leftUpperArm', Math.sin(t * 0.4) * 0.04, 0, 1.15 + hipSway * 0.08);
        this.setBone('rightUpperArm', Math.sin(t * 0.4) * 0.04, 0, -1.15 - hipSway * 0.08);
        this.setBone('leftLowerArm', 0, 0, 0.18 + Math.sin(t * 0.35) * 0.04);
        this.setBone('rightLowerArm', 0, 0, -0.18 - Math.sin(t * 0.35 + 0.2) * 0.04);
        this.setExpressionTarget('relaxed', 0.35);
        break;
      }
      case 'thinking': {
        // Cool ponder — one hip out, hand on hip attitude
        this.setBone('hips', 0, 0, 0.08);
        this.setBone('head', Math.sin(t * 0.5) * 0.06, 0.12, 0.08);
        this.setBone('neck', 0, 0.06, 0);
        this.setBone('spine', breath, 0, -0.04);
        // Left hand on hip — attitude pose
        this.setBone('leftUpperArm', -0.15, 0.3, 0.7);
        this.setBone('leftLowerArm', 0, 0, 0.7);
        break;
      }
      case 'talking': {
        // Expressive but controlled — confident gestures
        this.setBone('head', Math.sin(t * 1.2) * 0.07, Math.sin(t * 0.7) * 0.12, Math.sin(t * 1.0) * 0.04);
        this.setBone('neck', Math.sin(t * 1.5) * 0.03, Math.sin(t * 0.5) * 0.04, 0);
        this.setBone('hips', 0, 0, Math.sin(t * 0.6) * 0.04);
        this.setBone('spine', breath, Math.sin(t * 0.4) * 0.04, Math.sin(t * 0.5) * 0.02);
        // Casual hand gesture — stylish
        this.setBone('rightUpperArm', Math.sin(t * 1.0) * 0.15 - 0.15, 0, -1.0);
        this.setBone('rightLowerArm', Math.sin(t * 1.3) * 0.1, 0, -0.2);
        this.setExpressionTarget('aa', ((Math.sin(t * 6.0) + 1) * 0.5) * 0.45);
        this.setExpressionTarget('relaxed', 0.1);
        break;
      }
      case 'happy': {
        // Pleased but cool about it — subtle smile, hair toss motion
        this.setBone('head', -0.08, Math.sin(t * 0.8) * 0.1, Math.sin(t * 1.2) * 0.07);
        this.setBone('neck', -0.04, Math.sin(t * 0.6) * 0.04, 0);
        this.setBone('hips', 0, 0, Math.sin(t * 1.0) * 0.06);
        this.setBone('spine', breath - 0.04, 0, Math.sin(t * 0.7) * 0.03);
        this.setExpressionTarget('happy', 0.6);
        this.setExpressionTarget('relaxed', 0.3);
        break;
      }
      case 'sad': {
        // Annoyed-sad — looks away with attitude
        this.setBone('head', 0.1, -0.15, 0.04);
        this.setBone('neck', 0.05, -0.08, 0);
        this.setBone('spine', breath + 0.06, 0, Math.sin(t * 0.3) * 0.02);
        this.setBone('hips', 0, 0, 0.04);
        this.setExpressionTarget('sad', 0.4);
        this.setExpressionTarget('angry', 0.2);
        break;
      }
    }
  }

  // ══════════════════════════════════════════════════════════════════════
  //  GENTLEMAN — Nogami: stoic, minimal, composed, strong silent type
  // ══════════════════════════════════════════════════════════════════════

  private animateGentleman(t: number) {
    // ── Breathing — slow, measured, controlled ──
    const breath = Math.sin(t * 0.8) * 0.04;
    this.setBone('spine', breath, 0, 0);
    this.setBone('chest', breath * 0.5, 0, 0);

    // ── Arms — proper, close to body, firmly at sides ──
    // Increased Z rotation (1.35 ≈ 77°) to push arms fully down for VRM 1.0
    this.setBone('leftUpperArm', 0.05, 0, 1.35);
    this.setBone('rightUpperArm', 0.05, 0, -1.35);
    this.setBone('leftLowerArm', 0, 0, 0.1);
    this.setBone('rightLowerArm', 0, 0, -0.1);
    this.setBone('leftShoulder', 0, 0, 0.04);
    this.setBone('rightShoulder', 0, 0, -0.04);
    this.setBone('hips', 0, 0, 0);
    this.setBone('neck', 0, 0, 0);
    this.setBone('head', 0, 0, 0);
    this.setBone('upperChest', 0, 0, 0);

    switch (this.state) {
      case 'idle': {
        // Deliberate, slow micro-movements — conveys quiet confidence
        // Still restrained compared to other personas but VISIBLE
        const headY = Math.sin(t * 0.15) * 0.08;
        const headX = Math.sin(t * 0.12) * 0.03;
        this.setBone('head', headX, headY, Math.sin(t * 0.1) * 0.015);
        this.setBone('neck', Math.sin(t * 0.1) * 0.02, Math.sin(t * 0.08) * 0.02, 0);
        // Slow weight shift — barely visible but present
        const shift = Math.sin(t * 0.2) * 0.03;
        this.setBone('hips', 0, Math.sin(t * 0.12) * 0.015, shift);
        this.setBone('spine', breath, 0, -shift * 0.3);
        this.setBone('upperChest', Math.sin(t * 0.15) * 0.01, 0, 0);
        // Very subtle arm pendulum with weight shift
        this.setBone('leftUpperArm', Math.sin(t * 0.2) * 0.02 + 0.05, 0, 1.35 + shift * 0.04);
        this.setBone('rightUpperArm', Math.sin(t * 0.2 + 0.3) * 0.02 + 0.05, 0, -1.35 + shift * 0.04);
        break;
      }
      case 'thinking': {
        // Slight brow furrow, chin down — deep pensive
        this.setBone('head', 0.08, Math.sin(t * 0.3) * 0.04, 0);
        this.setBone('neck', 0.04, 0, 0);
        this.setBone('spine', breath, Math.sin(t * 0.2) * 0.02, 0);
        this.setBone('hips', 0, 0, Math.sin(t * 0.2) * 0.015);
        this.setExpressionTarget('angry', 0.15);
        break;
      }
      case 'talking': {
        // Minimal nods — speaks with gravity, measured
        this.setBone('head', Math.sin(t * 1.2) * 0.05, Math.sin(t * 0.5) * 0.04, 0);
        this.setBone('neck', Math.sin(t * 1.5) * 0.02, 0, 0);
        this.setBone('spine', breath, 0, Math.sin(t * 0.4) * 0.015);
        this.setBone('hips', 0, 0, Math.sin(t * 0.5) * 0.012);
        this.setExpressionTarget('aa', ((Math.sin(t * 5.0) + 1) * 0.5) * 0.35);
        break;
      }
      case 'happy': {
        // Restrained satisfaction — subtle nod, slight lean back
        this.setBone('head', -0.06, Math.sin(t * 0.4) * 0.03, 0);
        this.setBone('neck', -0.03, 0, 0);
        this.setBone('spine', breath - 0.03, 0, 0);
        this.setExpressionTarget('happy', 0.35);
        this.setExpressionTarget('relaxed', 0.3);
        break;
      }
      case 'sad': {
        // Stoic sadness — head drops slightly, shoulders tense
        this.setBone('head', 0.1, 0, Math.sin(t * 0.2) * 0.02);
        this.setBone('neck', 0.05, 0, 0);
        this.setBone('spine', breath + 0.04, 0, 0);
        this.setBone('hips', 0, 0, Math.sin(t * 0.15) * 0.01);
        this.setExpressionTarget('sad', 0.35);
        break;
      }
    }
  }

  // ── Smooth expression system ───────────────────────────────────────

  private setExpressionTarget(name: string, value: number) {
    this.expressionTargets.set(name, value);
  }

  private clearExpressionTargets() {
    for (const name of ['aa', 'oh', 'happy', 'sad', 'angry', 'relaxed', 'neutral']) {
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
