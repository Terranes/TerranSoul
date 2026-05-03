import { describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';
import type { SittingProps } from './props';
import { SittingPropController } from './sitting-props-controller';

function testProps(): SittingProps {
  const material = new THREE.MeshBasicMaterial();
  const geometry = new THREE.BoxGeometry(1, 1, 1);
  vi.spyOn(material, 'dispose');
  vi.spyOn(geometry, 'dispose');
  const chair = new THREE.Group();
  chair.name = 'test-chair';
  chair.visible = false;
  chair.add(new THREE.Mesh(geometry, material));

  const teacupMaterial = new THREE.MeshBasicMaterial();
  const teacupGeometry = new THREE.SphereGeometry(1);
  vi.spyOn(teacupMaterial, 'dispose');
  vi.spyOn(teacupGeometry, 'dispose');
  const teacup = new THREE.Group();
  teacup.name = 'test-teacup';
  teacup.visible = false;
  teacup.add(new THREE.Mesh(teacupGeometry, teacupMaterial));

  return { chair, teacup };
}

describe('SittingPropController', () => {
  it('does not create chair props by default or for non-sitting animations', () => {
    const scene = new THREE.Scene();
    const createProps = vi.fn(testProps);
    const controller = new SittingPropController(createProps);

    controller.sync(scene, false, null);
    controller.sync(scene, true, '/animations/walk.vrma');

    expect(createProps).not.toHaveBeenCalled();
    expect(scene.children).toHaveLength(0);
    expect(controller.activeProps).toBeNull();
  });

  it('creates and shows the chair only for sitting animations', () => {
    const scene = new THREE.Scene();
    const controller = new SittingPropController(testProps);

    controller.sync(scene, true, '/animations/relax.vrma');

    expect(controller.activeProps?.chair.visible).toBe(true);
    expect(scene.children).toContain(controller.activeProps?.chair);
  });

  it('removes and disposes props after sitting ends', () => {
    const scene = new THREE.Scene();
    const controller = new SittingPropController(testProps);

    controller.sync(scene, true, '/animations/ladylike.vrma');
    const props = controller.activeProps!;
    const chairMesh = props.chair.children[0] as THREE.Mesh;
    const teacupMesh = props.teacup.children[0] as THREE.Mesh;

    controller.sync(scene, false, null);

    expect(scene.children).not.toContain(props.chair);
    expect(controller.activeProps).toBeNull();
    expect(chairMesh.geometry.dispose).toHaveBeenCalled();
    expect((chairMesh.material as THREE.Material).dispose).toHaveBeenCalled();
    expect(teacupMesh.geometry.dispose).toHaveBeenCalled();
    expect((teacupMesh.material as THREE.Material).dispose).toHaveBeenCalled();
  });
});
