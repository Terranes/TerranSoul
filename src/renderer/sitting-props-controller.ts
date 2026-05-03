import * as THREE from 'three';
import { createSittingProps, type SittingProps } from './props';
import { SITTING_ANIMATION_PATHS } from './vrma-manager';

export class SittingPropController {
  private sittingProps: SittingProps | null = null;

  constructor(private readonly createProps: () => SittingProps = createSittingProps) {}

  get activeProps(): SittingProps | null {
    return this.sittingProps;
  }

  sync(scene: THREE.Scene | null, playing: boolean, currentPath: string | null): void {
    const isSitting = playing && currentPath != null && SITTING_ANIMATION_PATHS.has(currentPath);
    if (!isSitting) {
      this.dispose(scene);
      return;
    }
    if (!scene) return;
    if (!this.sittingProps) {
      this.sittingProps = this.createProps();
      scene.add(this.sittingProps.chair);
    }
    this.sittingProps.chair.visible = true;
  }

  dispose(scene: THREE.Scene | null): void {
    if (!this.sittingProps) return;
    scene?.remove(this.sittingProps.chair);
    disposeGroup(this.sittingProps.chair);
    disposeGroup(this.sittingProps.teacup);
    this.sittingProps = null;
  }
}

function disposeGroup(group: THREE.Group): void {
  group.traverse((obj) => {
    if ((obj as THREE.Mesh).isMesh) {
      const mesh = obj as THREE.Mesh;
      mesh.geometry?.dispose();
      const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
      for (const material of materials) {
        material.dispose();
      }
    }
  });
}
