import * as THREE from 'three';
import type { VRM } from '@pixiv/three-vrm';
import type { CharacterState } from '../types';

export class CharacterAnimator {
  private vrm: VRM | null = null;
  private placeholder: THREE.Group | null = null;
  private state: CharacterState = 'idle';
  private elapsed = 0;

  setVRM(vrm: VRM) {
    this.vrm = vrm;
    this.placeholder = null;
  }

  setPlaceholder(group: THREE.Group) {
    this.placeholder = group;
    this.vrm = null;
  }

  setState(state: CharacterState) {
    this.state = state;
    this.elapsed = 0;
  }

  getState(): CharacterState {
    return this.state;
  }

  update(delta: number) {
    this.elapsed += delta;
    const t = this.elapsed;

    if (this.vrm) {
      this.vrm.update(delta);
      this.applyVRMAnimation(t);
    } else if (this.placeholder) {
      this.applyPlaceholderAnimation(t);
    }
  }

  private applyVRMAnimation(t: number) {
    if (!this.vrm) return;
    const hips = this.vrm.humanoid.getNormalizedBoneNode('hips');
    const head = this.vrm.humanoid.getNormalizedBoneNode('head');

    switch (this.state) {
      case 'idle':
        if (hips) {
          hips.position.y = Math.sin(t * 0.8) * 0.01;
        }
        if (head) {
          head.rotation.x = Math.sin(t * 0.6) * 0.02;
          head.rotation.z = Math.sin(t * 0.4) * 0.01;
        }
        this.setBlendShape('aa', 0);
        break;

      case 'thinking':
        if (hips) {
          hips.position.y = Math.sin(t * 2.0) * 0.015;
        }
        if (head) {
          head.rotation.x = -0.1 + Math.sin(t * 1.5) * 0.05;
          head.rotation.z = Math.sin(t * 0.8) * 0.03;
        }
        this.setBlendShape('aa', 0);
        break;

      case 'talking':
        if (hips) {
          hips.position.y = Math.sin(t * 4.0) * 0.012;
        }
        if (head) {
          head.rotation.x = Math.sin(t * 3.0) * 0.03;
        }
        this.setBlendShape('aa', ((Math.sin(t * 8.0) + 1.0) * 0.5) * 0.6);
        this.setBlendShape('oh', ((Math.sin(t * 8.0) + 1.0) * 0.5) * 0.2);
        break;

      case 'happy':
        if (hips) {
          hips.position.y = Math.abs(Math.sin(t * 5.0)) * 0.03;
        }
        if (head) {
          head.rotation.z = Math.sin(t * 3.0) * 0.04;
        }
        this.setBlendShape('aa', 0);
        this.setBlendShape('happy', 0.8);
        break;

      case 'sad':
        if (hips) {
          hips.position.y = Math.sin(t * 0.5) * 0.005;
        }
        if (head) {
          head.rotation.x = 0.15 + Math.sin(t * 0.3) * 0.02;
        }
        this.setBlendShape('aa', 0);
        this.setBlendShape('happy', 0);
        break;
    }
  }

  private setBlendShape(name: string, value: number) {
    if (!this.vrm) return;
    try {
      this.vrm.expressionManager?.setValue(name, value);
    } catch {
      // BlendShape not available on this model — silently ignore
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
