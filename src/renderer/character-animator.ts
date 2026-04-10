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
    if (!hips) return;

    switch (this.state) {
      case 'idle':
        hips.position.y = Math.sin(t * 0.8) * 0.01;
        break;
      case 'thinking':
        hips.position.y = Math.sin(t * 2.0) * 0.015;
        break;
      case 'talking':
        hips.position.y = Math.sin(t * 4.0) * 0.012;
        break;
      case 'happy':
        hips.position.y = Math.abs(Math.sin(t * 5.0)) * 0.03;
        break;
      case 'sad':
        hips.position.y = Math.sin(t * 0.5) * 0.005;
        break;
    }
  }

  private applyPlaceholderAnimation(t: number) {
    if (!this.placeholder) return;

    switch (this.state) {
      case 'idle':
        this.placeholder.position.y = Math.sin(t * 0.8) * 0.03;
        this.placeholder.rotation.y = Math.sin(t * 0.4) * 0.1;
        break;
      case 'thinking':
        this.placeholder.rotation.y += 0.02;
        this.placeholder.position.y = Math.sin(t * 2.0) * 0.04;
        break;
      case 'talking':
        this.placeholder.position.y = Math.sin(t * 6.0) * 0.025;
        this.placeholder.rotation.z = Math.sin(t * 6.0) * 0.04;
        break;
      case 'happy':
        this.placeholder.position.y = Math.abs(Math.sin(t * 5.0)) * 0.08;
        this.placeholder.rotation.z = Math.sin(t * 5.0) * 0.08;
        break;
      case 'sad':
        this.placeholder.position.y = -Math.abs(Math.sin(t * 0.5)) * 0.02;
        this.placeholder.rotation.z = Math.sin(t * 0.5) * 0.02;
        break;
    }
  }
}
