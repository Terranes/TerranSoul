import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { VRMLoaderPlugin, type VRM } from '@pixiv/three-vrm';
import type { VrmMetadata } from '../types';

export type ProgressCallback = (loaded: number, total: number) => void;

export interface VrmLoadResult {
  vrm: VRM;
  metadata: VrmMetadata;
}

export function extractVrmMetadata(vrm: VRM): VrmMetadata {
  const meta = vrm.meta;
  if (!meta) {
    return { title: 'Unknown', author: 'Unknown', license: 'Unknown' };
  }

  if (meta.metaVersion === '1') {
    return {
      title: meta.name || 'Unknown',
      author: meta.authors?.[0] || 'Unknown',
      license: meta.licenseUrl || 'Unknown',
    };
  }

  if (meta.metaVersion === '0') {
    return {
      title: meta.title || 'Unknown',
      author: meta.author || 'Unknown',
      license: meta.licenseName || meta.otherLicenseUrl || 'Unknown',
    };
  }

  return { title: 'Unknown', author: 'Unknown', license: 'Unknown' };
}

export async function loadVRM(
  scene: THREE.Scene,
  path: string,
  onProgress?: ProgressCallback,
): Promise<VrmLoadResult> {
  if (!path || typeof path !== 'string') {
    throw new Error('VRM path must be a non-empty string');
  }

  const loader = new GLTFLoader();
  loader.register((parser) => new VRMLoaderPlugin(parser));

  const gltf = await loader.loadAsync(path, (event) => {
    if (onProgress && event.lengthComputable) {
      onProgress(event.loaded, event.total);
    }
  });

  const vrm: VRM | undefined = gltf.userData.vrm;
  if (!vrm) {
    throw new Error('File loaded but does not contain valid VRM data');
  }

  vrm.scene.rotation.y = Math.PI;
  scene.add(vrm.scene);

  return { vrm, metadata: extractVrmMetadata(vrm) };
}

export async function loadVRMSafe(
  scene: THREE.Scene,
  path: string,
  onProgress?: ProgressCallback,
): Promise<VrmLoadResult | null> {
  try {
    return await loadVRM(scene, path, onProgress);
  } catch (error) {
    console.error('VRM load failed, using placeholder:', error);
    return null;
  }
}

export function createPlaceholderCharacter(scene: THREE.Scene): THREE.Group {
  const group = new THREE.Group();

  // Body
  const bodyGeo = new THREE.CapsuleGeometry(0.25, 0.7, 8, 16);
  const bodyMat = new THREE.MeshStandardMaterial({ color: 0x6c63ff });
  const body = new THREE.Mesh(bodyGeo, bodyMat);
  body.position.y = 0.85;
  group.add(body);

  // Head
  const headGeo = new THREE.SphereGeometry(0.22, 16, 16);
  const headMat = new THREE.MeshStandardMaterial({ color: 0xf5c5a3 });
  const head = new THREE.Mesh(headGeo, headMat);
  head.position.y = 1.6;
  group.add(head);

  // Eyes
  const eyeGeo = new THREE.SphereGeometry(0.04, 8, 8);
  const eyeMat = new THREE.MeshStandardMaterial({ color: 0x222222 });
  const leftEye = new THREE.Mesh(eyeGeo, eyeMat);
  leftEye.position.set(-0.08, 1.64, 0.19);
  group.add(leftEye);
  const rightEye = new THREE.Mesh(eyeGeo, eyeMat);
  rightEye.position.set(0.08, 1.64, 0.19);
  group.add(rightEye);

  scene.add(group);
  return group;
}
