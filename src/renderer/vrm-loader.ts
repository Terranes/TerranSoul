import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { VRMLoaderPlugin, VRMUtils, type VRM, type VRMHumanBoneName } from '@pixiv/three-vrm';
import type { VrmMetadata } from '../types';

export type ProgressCallback = (loaded: number, total: number) => void;

export interface VrmLoadResult {
  vrm: VRM;
  metadata: VrmMetadata;
  isVrm0: boolean;
}

function shouldLogVrmLoadEvents(): boolean {
  return import.meta.env.MODE !== 'test';
}

/**
 * Set the normalized bone rotations into a natural relaxed standing pose.
 * Call this before vrm.update() to reset bones from T-pose/A-pose.
 * Does NOT call vrm.update() — the caller is responsible for that.
 */
export function setNaturalBonePose(vrm: VRM): void {
  const bone = (name: VRMHumanBoneName) => vrm.humanoid?.getNormalizedBoneNode(name);

  // Arms down — rotate upper arms ~77° toward body
  const leftUpperArm = bone('leftUpperArm');
  if (leftUpperArm) {
    leftUpperArm.rotation.set(0, 0, 1.35); // Z+ = toward body for left
  }
  const rightUpperArm = bone('rightUpperArm');
  if (rightUpperArm) {
    rightUpperArm.rotation.set(0, 0, -1.35); // Z- = toward body for right
  }

  // Slight bend in elbows so arms don't look stiff
  const leftLowerArm = bone('leftLowerArm');
  if (leftLowerArm) {
    leftLowerArm.rotation.set(0, 0, 0.15);
  }
  const rightLowerArm = bone('rightLowerArm');
  if (rightLowerArm) {
    rightLowerArm.rotation.set(0, 0, -0.15);
  }

  // Relax shoulders slightly
  const leftShoulder = bone('leftShoulder');
  if (leftShoulder) {
    leftShoulder.rotation.set(0, 0, 0.05);
  }
  const rightShoulder = bone('rightShoulder');
  if (rightShoulder) {
    rightShoulder.rotation.set(0, 0, -0.05);
  }

  // Head straight
  const head = bone('head');
  if (head) {
    head.rotation.set(0, 0, 0);
  }

  // Spine straight
  const spine = bone('spine');
  if (spine) {
    spine.rotation.set(0, 0, 0);
  }
}

/**
 * Apply a natural relaxed standing pose and push it to the raw skeleton.
 * Use this once after loading. The animator uses setNaturalBonePose() per frame.
 */
export function applyNaturalPose(vrm: VRM): void {
  setNaturalBonePose(vrm);
  // vrm.update() calls humanoid.update() internally, which transfers
  // normalized bone rotations to the raw skeleton (autoUpdateHumanBones=true).
  vrm.update(0);
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
  // autoUpdateHumanBones must be true (default) so that humanoid.update()
  // transfers normalized bone rotations to the raw skeleton each frame.
  // When false, humanoid.update() is a no-op and the model stays in T-pose.
  loader.register((parser) => new VRMLoaderPlugin(parser));

  // Encode spaces/special chars for HTTP paths; leave blob:/data: URLs untouched
  const url = path.startsWith('blob:') || path.startsWith('data:') ? path : encodeURI(path);

  // Suppress known harmless warnings from three-vrm during VRM 0.0 loading:
  // - "Curves of LookAtDegreeMap defined in VRM 0.0 are not supported"
  // - "THREE.InterleavedBufferAttribute.clone(): Cloning an interleaved buffer attribute will de-interleave buffer data."
  const origWarn = console.warn;
  console.warn = (...args: unknown[]) => {
    if (typeof args[0] === 'string' && (
      args[0].includes('LookAtDegreeMap') ||
      args[0].includes('InterleavedBufferAttribute')
    )) return;
    origWarn.apply(console, args);
  };

  const gltf = await loader.loadAsync(url, (event) => {
    if (onProgress && event.lengthComputable) {
      onProgress(event.loaded, event.total);
    }
  });

  const vrm: VRM | undefined = gltf.userData.vrm;
  if (!vrm) {
    throw new Error('File loaded but does not contain valid VRM data');
  }

  // Rotate VRM 0.x models so they face the camera (VRM 0 faces -Z, VRM 1 faces +Z)
  const isVrm0 = String(vrm.meta?.metaVersion ?? '').startsWith('0');
  if (isVrm0) {
    VRMUtils.rotateVRM0(vrm);
  }

  // Performance optimizations (from official three-vrm examples)
  VRMUtils.removeUnnecessaryVertices(gltf.scene);
  VRMUtils.combineSkeletons(gltf.scene);
  VRMUtils.combineMorphs(vrm);

  // Restore original console.warn after load + post-processing
  console.warn = origWarn;

  // Disable frustum culling to prevent clipping when parts are near screen edge;
  // also recompute bounding geometry for correct depth sorting.
  vrm.scene.traverse((obj) => {
    obj.frustumCulled = false;
    if ((obj as THREE.Mesh).isMesh) {
      const mesh = obj as THREE.Mesh;
      mesh.geometry?.computeBoundingBox?.();
      mesh.geometry?.computeBoundingSphere?.();
    }
  });

  scene.add(vrm.scene);

  // Apply a natural relaxed pose so the character doesn't stand in T-pose
  applyNaturalPose(vrm);

  // ── Spring bone warmup ────────────────────────────────────────────
  // VRM spring bones (hair, clothing, accessories) start in their rest
  // position, which may be far from where gravity would settle them.
  // Without warmup the hair visibly "flies" or "drops" over the first
  // second.  VRoid Hub solves this by running physics ticks off-screen.
  //
  // We reset the spring bone state to match the current pose, then
  // simulate ~1 second of physics (60 ticks at 1/60s) so the hair and
  // cloth settle into their gravity-affected rest position before the
  // first visible frame.
  const sbm = vrm.springBoneManager;
  if (sbm) {
    sbm.reset();
    const warmupDt = 1 / 60;
    for (let i = 0; i < 60; i++) {
      sbm.update(warmupDt);
    }
  }

  return { vrm, metadata: extractVrmMetadata(vrm), isVrm0 };
}

export async function loadVRMSafe(
  scene: THREE.Scene,
  path: string,
  onProgress?: ProgressCallback,
): Promise<VrmLoadResult | null> {
  try {
    if (shouldLogVrmLoadEvents()) {
      console.log('[TerranSoul] Loading VRM:', path);
    }
    const result = await loadVRM(scene, path, onProgress);
    if (shouldLogVrmLoadEvents()) {
      console.log('[TerranSoul] VRM loaded successfully:', path);
    }
    return result;
  } catch (error) {
    if (shouldLogVrmLoadEvents()) {
      console.error('[TerranSoul] VRM load failed, using placeholder:', error);
    }
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
