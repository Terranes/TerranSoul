import * as THREE from 'three';
import type { VRM } from '@pixiv/three-vrm';
import type { AnimationPersona, CharacterState } from '../types';
import { setNaturalBonePose } from './vrm-loader';

export class CharacterAnimator {
  private vrm: VRM | null = null;
  private vrmScene: THREE.Object3D | null = null;
  private placeholder: THREE.Group | null = null;
  private state: CharacterState = 'idle';
  private elapsed = 0;
  private baseRotationY = 0;
  private persona: AnimationPersona = 'cool';
  private skipBonePose = false;

  setVRM(vrm: VRM, rotationY = 0, persona: AnimationPersona = 'cool', skipBonePose = false) {
    this.vrm = vrm;
    this.vrmScene = vrm.scene;
    this.baseRotationY = rotationY;
    this.persona = persona;
    this.skipBonePose = skipBonePose;
    this.placeholder = null;
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
   * Animate the VRM model. Delegates to persona-specific routines.
   */
  private applyVRMAnimation(t: number, delta: number) {
    if (!this.vrm || !this.vrmScene) return;

    if (!this.skipBonePose) {
      setNaturalBonePose(this.vrm);
    }

    // Reset scene root each frame
    this.vrmScene.position.set(0, 0, 0);
    this.vrmScene.rotation.set(0, this.baseRotationY, 0);

    // Periodic blink
    const blinkCycle = t % (this.persona === 'cute' ? 3.0 : 4.0);
    this.setExpression('blink', blinkCycle < 0.12 ? 1.0 : 0.0);

    if (this.persona === 'cute') {
      this.applyCuteAnimation(t);
    } else {
      this.applyCoolAnimation(t);
    }

    this.vrm.update(delta);
  }

  // ── Cool persona: smooth, confident, minimal movement ──────────────

  private applyCoolAnimation(t: number) {
    if (!this.vrmScene) return;

    switch (this.state) {
      case 'idle': {
        // Slow, deep breathing — barely perceptible sway. Calm confidence.
        this.vrmScene.position.y = Math.sin(t * 0.8) * 0.008;
        this.vrmScene.rotation.y = this.baseRotationY + Math.sin(t * 0.3) * 0.015;
        this.clearExpressions();
        // Slight smirk — "relaxed" expression for that cool composure
        this.setExpression('relaxed', 0.4);
        break;
      }
      case 'thinking': {
        // Slow deliberate head tilt, like considering carefully
        this.vrmScene.position.y = Math.sin(t * 0.6) * 0.01;
        this.vrmScene.rotation.y = this.baseRotationY + Math.sin(t * 0.4) * 0.12;
        this.vrmScene.rotation.z = Math.sin(t * 0.5) * 0.04;
        this.clearExpressions();
        break;
      }
      case 'talking': {
        // Measured nods with controlled lip sync — not hyper
        this.vrmScene.position.y = Math.sin(t * 1.8) * 0.012;
        this.vrmScene.rotation.y = this.baseRotationY + Math.sin(t * 0.7) * 0.05;
        this.vrmScene.rotation.x = Math.sin(t * 1.5) * 0.02; // subtle nod
        this.clearExpressions();
        this.setExpression('aa', ((Math.sin(t * 6.0) + 1.0) * 0.5) * 0.5);
        this.setExpression('oh', ((Math.cos(t * 4.5) + 1.0) * 0.5) * 0.2);
        break;
      }
      case 'happy': {
        // Cool-happy: confident lean back, slight smirk, not bouncy
        this.vrmScene.position.y = Math.sin(t * 1.2) * 0.02;
        this.vrmScene.rotation.y = this.baseRotationY + Math.sin(t * 0.8) * 0.06;
        this.vrmScene.rotation.x = -0.03; // lean back slightly
        this.clearExpressions();
        this.setExpression('happy', 0.6);
        this.setExpression('relaxed', 0.3);
        break;
      }
      case 'sad': {
        // Stoic disappointment — subtle droop, controlled
        this.vrmScene.position.y = -0.02 + Math.sin(t * 0.4) * 0.005;
        this.vrmScene.rotation.x = 0.04;
        this.vrmScene.rotation.z = Math.sin(t * 0.25) * 0.008;
        this.clearExpressions();
        this.setExpression('sad', 0.5);
        break;
      }
    }
  }

  // ── Cute persona: bouncy, expressive, lots of energy ───────────────

  private applyCuteAnimation(t: number) {
    if (!this.vrmScene) return;

    switch (this.state) {
      case 'idle': {
        // Gentle swaying side-to-side like humming a song
        this.vrmScene.position.y = Math.sin(t * 1.5) * 0.02;
        this.vrmScene.rotation.y = this.baseRotationY + Math.sin(t * 0.7) * 0.05;
        this.vrmScene.rotation.z = Math.sin(t * 0.9) * 0.025;
        this.clearExpressions();
        this.setExpression('relaxed', 0.5);
        this.setExpression('happy', 0.2); // resting cute smile
        break;
      }
      case 'thinking': {
        // Cute confused head tilts — tilting and swaying
        this.vrmScene.position.y = Math.sin(t * 2.2) * 0.025;
        this.vrmScene.rotation.y = this.baseRotationY + Math.sin(t * 1.0) * 0.18;
        this.vrmScene.rotation.z = Math.sin(t * 1.5) * 0.06;
        this.clearExpressions();
        this.setExpression('oh', 0.4); // cute "oh?" expression
        break;
      }
      case 'talking': {
        // Energetic bobbing with exaggerated mouth movement
        this.vrmScene.position.y = Math.sin(t * 3.5) * 0.025;
        this.vrmScene.rotation.y = this.baseRotationY + Math.sin(t * 1.8) * 0.1;
        this.vrmScene.rotation.z = Math.sin(t * 2.5) * 0.03;
        this.clearExpressions();
        this.setExpression('aa', ((Math.sin(t * 9.0) + 1.0) * 0.5) * 0.8);
        this.setExpression('oh', ((Math.cos(t * 7.0) + 1.0) * 0.5) * 0.4);
        this.setExpression('happy', 0.3); // talks with a smile
        break;
      }
      case 'happy': {
        // Super bouncy! Big smile, celebratory hops
        this.vrmScene.position.y = Math.abs(Math.sin(t * 5.0)) * 0.06;
        this.vrmScene.rotation.y = this.baseRotationY + Math.sin(t * 3.0) * 0.12;
        this.vrmScene.rotation.z = Math.sin(t * 4.0) * 0.06;
        this.clearExpressions();
        this.setExpression('happy', 1.0);
        this.setExpression('aa', 0.4 + Math.sin(t * 4.0) * 0.2);
        break;
      }
      case 'sad': {
        // Droopy, pouty — clearly upset, dramatic
        this.vrmScene.position.y = -0.04 + Math.sin(t * 0.6) * 0.012;
        this.vrmScene.rotation.x = 0.08;
        this.vrmScene.rotation.z = Math.sin(t * 0.4) * 0.02;
        this.clearExpressions();
        this.setExpression('sad', 1.0);
        break;
      }
    }
  }

  private setExpression(name: string, value: number) {
    try {
      this.vrm?.expressionManager?.setValue(name, value);
    } catch { /* expression not available on this model */ }
  }

  private clearExpressions() {
    for (const name of ['aa', 'oh', 'happy', 'sad', 'angry', 'relaxed']) {
      this.setExpression(name, 0);
    }
  }

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
