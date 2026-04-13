import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

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
  controls: OrbitControls;
  lookAtTarget: THREE.Object3D;
  getRendererInfo: () => RendererInfo;
  /** Call each frame before controls.update() — smoothly adjusts the orbit
   *  target height so zooming in frames the face and zooming out shows the
   *  full body. */
  updateZoomTarget: () => void;
  dispose: () => void;
}

export async function initScene(canvas: HTMLCanvasElement): Promise<SceneContext> {
  // Force WebGL2 — VRM MToon materials use custom GLSL shaders (ShaderMaterial)
  // that only render correctly under WebGL2.  The Three.js WebGPU renderer
  // cannot handle MToonMaterial (it requires MToonNodeMaterial for WebGPU,
  // which is experimental and produces different visual results).
  // VRoid Hub also uses WebGL, so this ensures visual parity.
  const renderer = new THREE.WebGLRenderer({
    canvas,
    antialias: true,
    alpha: true,
    // preserveDrawingBuffer is required so that canvas.toDataURL(),
    // Playwright screenshots, and video recording capture the actual
    // rendered frame instead of reading a cleared/stale back-buffer.
    preserveDrawingBuffer: true,
  });

  // sRGB color space for correct output; NoToneMapping preserves MToon material
  // colors exactly as authored — ACES/other tone mappers desaturate & shift hues
  // which breaks VRM toon-shaded looks.  This matches VRoid Hub's renderer config.
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.NoToneMapping;

  renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

  const scene = new THREE.Scene();
  // Light-blue backdrop matching VRoid Hub's viewer
  scene.background = new THREE.Color(0xe3f4ff);

  const camera = new THREE.PerspectiveCamera(
    30,
    canvas.clientWidth / canvas.clientHeight,
    0.02,
    1000,
  );
  // Full-body framing — camera at body centre height, pulled back
  camera.position.set(0.0, 1.0, 2.8);

  // ── OrbitControls — locked viewport ────────────────────────────────
  // Horizontal-only rotation (360° azimuth, no vertical tilt).
  // Zoom maps to face (close) ↔ full body (far).
  const controls = new OrbitControls(camera, canvas);
  controls.screenSpacePanning = true;
  controls.target.set(0.0, 1.0, 0.0);
  controls.enableDamping = true;
  controls.dampingFactor = 0.1;

  // Lock vertical rotation — polar angle π/2 = camera stays level with target
  controls.minPolarAngle = Math.PI / 2;
  controls.maxPolarAngle = Math.PI / 2;

  // Disable panning so the model stays centred
  controls.enablePan = false;

  // Zoom limits: close = face, far = full body
  const MIN_DIST = 0.5;
  const MAX_DIST = 3.5;
  controls.minDistance = MIN_DIST;
  controls.maxDistance = MAX_DIST;
  controls.update();

  // Heights for zoom-dependent orbit target
  const FACE_Y = 1.45;    // orbit target Y when zoomed in (face)
  const BODY_Y = 0.85;    // orbit target Y when zoomed out (full body)

  /**
   * Smoothly adjusts the orbit target height based on zoom distance so
   * zooming in frames the face and zooming out shows the entire body.
   * Must be called each frame before controls.update().
   */
  function updateZoomTarget() {
    const dist = controls.getDistance();
    const t = Math.max(0, Math.min(1, (dist - MIN_DIST) / (MAX_DIST - MIN_DIST)));
    controls.target.y = FACE_Y + t * (BODY_Y - FACE_Y);
  }

  // LookAt target — placed in scene (not on camera) for VRM eye tracking
  const lookAtTarget = new THREE.Object3D();
  scene.add(lookAtTarget);

  // ── Lighting: matches VRoid Hub's 5-light setup ───────────────────────────
  // Ambient fill — ensures no part of the model is completely dark
  const ambientLight = new THREE.AmbientLight(0xffffff, 0.9);
  scene.add(ambientLight);

  // Hemisphere sky light — subtle blue-tinted ground bounce
  const skyLight = new THREE.HemisphereLight(0xffffff, 0xdcecff, 0.95);
  scene.add(skyLight);

  // Key light — slightly off-center and above, main illumination
  const keyLight = new THREE.DirectionalLight(0xffffff, 1.0);
  keyLight.position.set(0.4, 1.8, 3.4);
  scene.add(keyLight);

  // Fill light — softer, from the opposite side to reduce harsh shadows
  const fillLight = new THREE.DirectionalLight(0xffffff, 0.35);
  fillLight.position.set(-1.6, 1.4, 2.2);
  scene.add(fillLight);

  // Rim/back light — subtle separation from background
  const rimLight = new THREE.DirectionalLight(0xffffff, 0.18);
  rimLight.position.set(0.3, 2.3, -2.2);
  scene.add(rimLight);

  // Grid helper — visual grounding (like VRoid Hub)
  const gridHelper = new THREE.GridHelper(10, 20, 0x8fb0d2, 0xc4d7eb);
  scene.add(gridHelper);

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
    controls.dispose();
    renderer.dispose();
  }

  return { renderer, scene, camera, clock, controls, lookAtTarget, getRendererInfo, updateZoomTarget, dispose };
}
