import { ref, type Ref } from 'vue';
import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { VRMLoaderPlugin, VRMUtils, type VRM } from '@pixiv/three-vrm';
import { invoke } from '@tauri-apps/api/core';
import { applyNaturalPose } from '../renderer/vrm-loader';

/** Size of the rendered thumbnail in pixels. */
const THUMB_SIZE = 128;

/** IndexedDB database name and store for cached thumbnails. */
const DB_NAME = 'terransoul-vrm-thumbs';
const STORE_NAME = 'thumbnails';
const DB_VERSION = 1;

// ── IndexedDB cache ─────────────────────────────────────────────────────────

function openDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME);
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function getCached(key: string): Promise<string | null> {
  try {
    const db = await openDB();
    return new Promise((resolve) => {
      const tx = db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const req = store.get(key);
      req.onsuccess = () => resolve((req.result as string) ?? null);
      req.onerror = () => resolve(null);
    });
  } catch {
    return null;
  }
}

async function setCache(key: string, dataUrl: string): Promise<void> {
  try {
    const db = await openDB();
    const tx = db.transaction(STORE_NAME, 'readwrite');
    tx.objectStore(STORE_NAME).put(dataUrl, key);
  } catch {
    // Cache write failed — non-critical
  }
}

// ── Offscreen renderer (singleton — reused across calls) ────────────────────

let _offRenderer: THREE.WebGLRenderer | null = null;

function getOffscreenRenderer(): THREE.WebGLRenderer {
  if (_offRenderer) return _offRenderer;
  const canvas = document.createElement('canvas');
  canvas.width = THUMB_SIZE;
  canvas.height = THUMB_SIZE;
  _offRenderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: true,
    preserveDrawingBuffer: true,
  });
  _offRenderer.outputColorSpace = THREE.SRGBColorSpace;
  _offRenderer.toneMapping = THREE.NoToneMapping;
  _offRenderer.setClearColor(0x000000, 0);
  _offRenderer.setSize(THUMB_SIZE, THUMB_SIZE, false);
  _offRenderer.setPixelRatio(1);
  return _offRenderer;
}

// ── Core render logic ───────────────────────────────────────────────────────

async function renderVrmHeadshot(vrmPath: string): Promise<string> {
  const renderer = getOffscreenRenderer();
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(20, 1, 0.01, 100);

  // Lighting — simplified version of the main scene's setup
  scene.add(new THREE.AmbientLight(0xffffff, 0.9));
  scene.add(new THREE.HemisphereLight(0xffffff, 0xdcecff, 0.95));
  const key = new THREE.DirectionalLight(0xffffff, 1.0);
  key.position.set(0.4, 1.8, 3.4);
  scene.add(key);
  const fill = new THREE.DirectionalLight(0xffffff, 0.35);
  fill.position.set(-1.6, 1.4, 2.2);
  scene.add(fill);

  // Load VRM
  const loader = new GLTFLoader();
  loader.register((parser) => new VRMLoaderPlugin(parser));
  const url = vrmPath.startsWith('blob:') || vrmPath.startsWith('data:') ? vrmPath : encodeURI(vrmPath);
  const gltf = await loader.loadAsync(url);
  const vrm: VRM | undefined = gltf.userData.vrm;
  if (!vrm) throw new Error('No VRM data');

  const isVrm0 = String(vrm.meta?.metaVersion ?? '').startsWith('0');
  if (isVrm0) VRMUtils.rotateVRM0(vrm);
  VRMUtils.removeUnnecessaryVertices(gltf.scene);
  VRMUtils.combineSkeletons(gltf.scene);

  vrm.scene.traverse((obj) => { obj.frustumCulled = false; });
  scene.add(vrm.scene);
  applyNaturalPose(vrm);

  // Spring bone warmup so hair settles
  const sbm = vrm.springBoneManager;
  if (sbm) {
    sbm.reset();
    for (let i = 0; i < 60; i++) sbm.update(1 / 60);
  }

  // Frame the head: find the head bone or fall back to bounding box top
  const headBone = vrm.humanoid?.getNormalizedBoneNode('head');
  let headY: number;
  if (headBone) {
    const wp = new THREE.Vector3();
    headBone.getWorldPosition(wp);
    headY = wp.y;
  } else {
    const box = new THREE.Box3().setFromObject(vrm.scene);
    headY = box.max.y - (box.max.y - box.min.y) * 0.1;
  }

  // Position camera for a bust/headshot — slightly above centre of head,
  // pulled back enough to frame head + shoulders.
  camera.position.set(0, headY + 0.02, 0.55);
  camera.lookAt(0, headY - 0.02, 0);

  // Render
  renderer.render(scene, camera);
  const dataUrl = renderer.domElement.toDataURL('image/png');

  // Cleanup — remove VRM from scene and dispose geometry/materials
  scene.remove(vrm.scene);
  vrm.scene.traverse((obj) => {
    if ((obj as THREE.Mesh).isMesh) {
      const mesh = obj as THREE.Mesh;
      mesh.geometry?.dispose();
      const mat = mesh.material;
      if (Array.isArray(mat)) mat.forEach(m => m.dispose());
      else if (mat) mat.dispose();
    }
  });
  // Dispose lights
  scene.traverse((obj) => {
    if ((obj as THREE.Light).isLight) {
      (obj as THREE.Light).dispose?.();
    }
  });

  return dataUrl;
}

// ── Public composable ───────────────────────────────────────────────────────

/** In-flight generation promises keyed by cache key — prevents duplicate
 *  concurrent renders for the same model. */
const _inflight = new Map<string, Promise<string>>();

/** Keys that already failed — avoids retrying on every component mount. */
const _failed = new Set<string>();

/**
 * Read user-model bytes from the Rust backend, wrap in a temporary blob URL,
 * render a headshot, cache the result, and revoke the blob URL.
 */
async function renderUserModelHeadshot(userModelId: string): Promise<string> {
  const bytes = await invoke<number[] | Uint8Array<ArrayBuffer>>('read_user_model_bytes', { id: userModelId });
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  const blob = new Blob([u8], { type: 'model/gltf-binary' });
  const blobUrl = URL.createObjectURL(blob);
  try {
    return await renderVrmHeadshot(blobUrl);
  } finally {
    URL.revokeObjectURL(blobUrl);
  }
}

/**
 * Composable that provides a reactive thumbnail data URL for a VRM model.
 * On first call, checks IndexedDB cache. If missing, renders the VRM
 * offscreen and caches the result.
 *
 * For **default models**, pass `modelPath` (e.g. `/models/default/Shinra.vrm`).
 * For **user-imported models**, pass `userModelId` — bytes will be read from
 * the Rust backend via `read_user_model_bytes`, rendered offscreen, and cached.
 * Once cached, subsequent calls are instant (no backend round-trip).
 *
 * @param cacheKey     - Unique key for IndexedDB (e.g. model id like `shinra` or `u-1`)
 * @param modelPath    - Static VRM path for bundled models (mutually exclusive with userModelId)
 * @param userModelId  - User-model ID for backend byte fetch (mutually exclusive with modelPath)
 */
export function useVrmThumbnail(
  cacheKey: string,
  options: { modelPath?: string; userModelId?: string },
): {
  thumbnailUrl: Ref<string | null>;
  isGenerating: Ref<boolean>;
  generate: () => Promise<void>;
} {
  const thumbnailUrl = ref<string | null>(null);
  const isGenerating = ref(false);

  async function generate(): Promise<void> {
    if (thumbnailUrl.value || isGenerating.value || _failed.has(cacheKey)) return;

    // Try cache first
    const cached = await getCached(cacheKey);
    if (cached) {
      thumbnailUrl.value = cached;
      return;
    }

    // Check if another instance is already generating this thumbnail
    const existing = _inflight.get(cacheKey);
    if (existing) {
      isGenerating.value = true;
      try {
        thumbnailUrl.value = await existing;
      } catch {
        // Primary generator already logged the error
      } finally {
        isGenerating.value = false;
      }
      return;
    }

    isGenerating.value = true;

    // Choose render path based on model type
    const promise = options.userModelId
      ? renderUserModelHeadshot(options.userModelId)
      : options.modelPath
        ? renderVrmHeadshot(options.modelPath)
        : Promise.reject(new Error('Either modelPath or userModelId is required'));

    _inflight.set(cacheKey, promise);
    try {
      const dataUrl = await promise;
      await setCache(cacheKey, dataUrl);
      thumbnailUrl.value = dataUrl;
    } catch (err) {
      _failed.add(cacheKey);
      console.error(`[TerranSoul] VRM thumbnail generation failed for ${cacheKey}:`, err);
    } finally {
      _inflight.delete(cacheKey);
      isGenerating.value = false;
    }
  }

  return { thumbnailUrl, isGenerating, generate };
}

/**
 * Generate and cache a thumbnail for a user-imported model immediately
 * after import (while the blob URL may still be available). Call this
 * from the import flow so the thumbnail is ready before the user ever
 * opens the model picker again.
 */
export async function preGenerateUserThumbnail(userModelId: string): Promise<void> {
  const cached = await getCached(userModelId);
  if (cached) return; // Already cached

  const existing = _inflight.get(userModelId);
  if (existing) {
    await existing;
    return;
  }

  const task = (async () => {
    const dataUrl = await renderUserModelHeadshot(userModelId);
    await setCache(userModelId, dataUrl);
    return dataUrl;
  })();

  _inflight.set(userModelId, task);

  try {
    await task;
  } catch (err) {
    _failed.add(userModelId);
    console.error(`[TerranSoul] Pre-generate thumbnail failed for ${userModelId}:`, err);
  } finally {
    if (_inflight.get(userModelId) === task) _inflight.delete(userModelId);
  }
}

/** Dispose the shared offscreen renderer (call on app shutdown if needed). */
export function disposeVrmThumbnailRenderer(): void {
  if (_offRenderer) {
    _offRenderer.dispose();
    _offRenderer = null;
  }
}

/** Clear the failed-key cache (useful for tests or retry-after-fix scenarios). */
export function resetFailedThumbnails(): void {
  _failed.clear();
}
