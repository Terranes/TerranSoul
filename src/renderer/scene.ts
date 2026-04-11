import * as THREE from 'three';

export type RendererType = 'webgpu' | 'webgl';

export interface RendererInfo {
  triangles: number;
  calls: number;
  programs: number;
}

export interface SceneContext {
  renderer: THREE.WebGLRenderer;
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  clock: THREE.Clock;
  rendererType: RendererType;
  getRendererInfo: () => RendererInfo;
  dispose: () => void;
}

async function tryCreateWebGPURenderer(
  canvas: HTMLCanvasElement,
): Promise<THREE.WebGLRenderer | null> {
  if (typeof navigator === 'undefined' || !('gpu' in navigator)) return null;
  try {
    const { WebGPURenderer } = await import('three/webgpu');
    const renderer = new WebGPURenderer({ canvas, antialias: true });
    await renderer.init();
    return renderer as unknown as THREE.WebGLRenderer;
  } catch {
    return null;
  }
}

export async function initScene(canvas: HTMLCanvasElement): Promise<SceneContext> {
  let renderer: THREE.WebGLRenderer;
  let rendererType: RendererType;

  const webgpuRenderer = await tryCreateWebGPURenderer(canvas);
  if (webgpuRenderer) {
    renderer = webgpuRenderer;
    rendererType = 'webgpu';
  } else {
    renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    rendererType = 'webgl';
  }

  renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

  const scene = new THREE.Scene();
  // No solid background — transparent for overlay window
  scene.background = null;

  const camera = new THREE.PerspectiveCamera(
    30,
    canvas.clientWidth / canvas.clientHeight,
    0.1,
    100,
  );
  // Frame the upper body: camera slightly above eye level, pulled back enough
  // to see head-to-waist. VRM origin is at feet (Y=0), typical height ~1.5m.
  camera.position.set(0, 1.25, 2.5);
  camera.lookAt(0, 1.15, 0);

  // Ambient light
  const ambient = new THREE.AmbientLight(0xffffff, 0.7);
  scene.add(ambient);

  // Key light — warm, from upper right
  const dirLight = new THREE.DirectionalLight(0xfff5ee, 1.2);
  dirLight.position.set(2, 3, 2);
  dirLight.castShadow = true;
  dirLight.shadow.mapSize.set(512, 512);
  scene.add(dirLight);

  // Fill light — cool, from left
  const fillLight = new THREE.DirectionalLight(0xc4d4ff, 0.4);
  fillLight.position.set(-2, 1, 1);
  scene.add(fillLight);

  // Rim light — helps separate character from background
  const rimLight = new THREE.DirectionalLight(0x8888ff, 0.5);
  rimLight.position.set(-1, 2, -2);
  scene.add(rimLight);

  // Subtle ground circle for visual grounding
  const groundGeo = new THREE.CircleGeometry(1.2, 48);
  const groundMat = new THREE.MeshBasicMaterial({
    color: 0x4444aa,
    transparent: true,
    opacity: 0.12,
  });
  const ground = new THREE.Mesh(groundGeo, groundMat);
  ground.rotation.x = -Math.PI / 2; // lay flat
  ground.position.y = 0.001; // just above origin to avoid z-fighting
  ground.receiveShadow = true;
  scene.add(ground);

  const clock = new THREE.Clock();

  // Use ResizeObserver for accurate per-element resize handling
  const resizeObserver = new ResizeObserver(() => {
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    if (w === 0 || h === 0) return;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  });
  resizeObserver.observe(canvas.parentElement ?? canvas);

  function getRendererInfo(): RendererInfo {
    const info = renderer.info;
    return {
      triangles: info.render?.triangles ?? 0,
      calls: info.render?.calls ?? 0,
      programs: (info.programs?.length) ?? 0,
    };
  }

  function dispose() {
    resizeObserver.disconnect();
    renderer.dispose();
  }

  return { renderer, scene, camera, clock, rendererType, getRendererInfo, dispose };
}
