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

  renderer.setSize(canvas.clientWidth, canvas.clientHeight);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x1a1a2e);

  const camera = new THREE.PerspectiveCamera(
    45,
    canvas.clientWidth / canvas.clientHeight,
    0.1,
    100,
  );
  camera.position.set(0, 1.4, 3);
  camera.lookAt(0, 1.0, 0);

  // Ambient light
  const ambient = new THREE.AmbientLight(0xffffff, 0.6);
  scene.add(ambient);

  // Directional light
  const dirLight = new THREE.DirectionalLight(0xffffff, 1.0);
  dirLight.position.set(1, 2, 2);
  dirLight.castShadow = true;
  scene.add(dirLight);

  // Rim light
  const rimLight = new THREE.DirectionalLight(0x8888ff, 0.4);
  rimLight.position.set(-2, 1, -1);
  scene.add(rimLight);

  const clock = new THREE.Clock();

  // Use ResizeObserver for accurate per-element resize handling
  const resizeObserver = new ResizeObserver(() => {
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    if (w === 0 || h === 0) return;
    renderer.setSize(w, h);
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
