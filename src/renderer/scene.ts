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
  /**
   * Smoothly zoom the camera to face-close (enabled=true) or back to the
   * default full-body distance (enabled=false).  Used when the mobile keyboard
   * opens so the character face stays visible while the input footer slides up.
   */
  zoomToFace: (enabled: boolean) => void;
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
  renderer.setClearColor(0x000000, 0);

  renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

  const scene = new THREE.Scene();

  const camera = new THREE.PerspectiveCamera(
    30,
    canvas.clientWidth / canvas.clientHeight,
    0.02,
    1000,
  );
  // Full-body framing — camera at body centre height, pulled back.
  // On portrait screens (aspect < 1), pull back further so
  // the character's arms don't extend beyond the viewport edges.
  const CAMERA_Z_LANDSCAPE = 2.8;
  const CAMERA_Z_PORTRAIT = 3.8;
  /** Camera distance when the mobile keyboard is open — zoomed to face. */
  const CAMERA_Z_KEYBOARD = 1.2;
  /** Orbit target Y when the mobile keyboard is open (face centre). */
  const CAMERA_TARGET_Y_KEYBOARD = 1.55;
  const aspect = canvas.clientWidth / canvas.clientHeight;
  const cameraZ = aspect < 1 ? CAMERA_Z_PORTRAIT : CAMERA_Z_LANDSCAPE;
  camera.position.set(0.0, 1.0, cameraZ);

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
  const MAX_DIST = 5.0;
  controls.minDistance = MIN_DIST;
  controls.maxDistance = MAX_DIST;
  controls.update();

  // Heights for zoom-dependent orbit target
  const FACE_Y = 1.45;    // orbit target Y when zoomed in (face)
  const BODY_Y = 0.65;    // orbit target Y when zoomed out (full body, head to toes)

  /**
   * Smoothly adjusts the orbit target height based on zoom distance so
   * zooming in frames the face and zooming out shows the entire body.
   * When the keyboard is open, lerps camera towards face-close distance.
   * Must be called each frame before controls.update().
   */
  function updateZoomTarget() {
    if (_keyboardOpen) {
      // Lerp camera Z towards face distance
      const currentZ = camera.position.z;
      const targetZ = CAMERA_Z_KEYBOARD;
      camera.position.z += (targetZ - currentZ) * 0.08;
      // Lerp orbit target Y towards face height
      controls.target.y += (CAMERA_TARGET_Y_KEYBOARD - controls.target.y) * 0.08;
    } else {
      // Normal zoom-target logic: portrait default or full-body
      const dist = controls.getDistance();
      const t = Math.max(0, Math.min(1, (dist - MIN_DIST) / (MAX_DIST - MIN_DIST)));
      controls.target.y = FACE_Y + t * (BODY_Y - FACE_Y);
    }
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

  // Rim/back light — stronger separation from background
  const rimLight = new THREE.DirectionalLight(0xffffff, 0.45);
  rimLight.position.set(0.3, 2.3, -2.2);
  scene.add(rimLight);

  // Secondary rim from opposite side for fuller edge definition
  const rimLight2 = new THREE.DirectionalLight(0xddeeff, 0.25);
  rimLight2.position.set(-0.5, 1.5, -1.8);
  scene.add(rimLight2);

  // Soft pedestal instead of grid floor
  const pedestalGeometry = new THREE.CircleGeometry(1.15, 64);
  const pedestalMaterial = new THREE.MeshStandardMaterial({
    color: 0xe6ecf5,
    transparent: true,
    opacity: 0.95,
    roughness: 0.92,
    metalness: 0.02,
  });
  const pedestal = new THREE.Mesh(pedestalGeometry, pedestalMaterial);
  pedestal.rotation.x = -Math.PI / 2;
  pedestal.position.y = 0.01;
  scene.add(pedestal);

  const pedestalRingGeometry = new THREE.RingGeometry(1.18, 1.28, 64);
  const pedestalRingMaterial = new THREE.MeshBasicMaterial({
    color: 0xc7d7eb,
    transparent: true,
    opacity: 0.5,
    side: THREE.DoubleSide,
  });
  const pedestalRing = new THREE.Mesh(pedestalRingGeometry, pedestalRingMaterial);
  pedestalRing.rotation.x = -Math.PI / 2;
  pedestalRing.position.y = 0.012;
  scene.add(pedestalRing);

  const clock = new THREE.Clock();

  // Use ResizeObserver for accurate per-element resize handling
  const resizeObserver = new ResizeObserver(() => {
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    if (w === 0 || h === 0) return;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();

    // Adjust camera distance for portrait vs landscape so the character
    // stays fully visible on narrow mobile screens.
    const currentDist = camera.position.length();
    const targetZ = (w / h) < 1 ? CAMERA_Z_PORTRAIT : CAMERA_Z_LANDSCAPE;
    const ZOOM_TOLERANCE = 0.1;
    // Only adjust if the user hasn't manually zoomed
    if (Math.abs(currentDist - CAMERA_Z_LANDSCAPE) < ZOOM_TOLERANCE || Math.abs(currentDist - CAMERA_Z_PORTRAIT) < ZOOM_TOLERANCE) {
      camera.position.setZ(targetZ);
    }
  });
  resizeObserver.observe(canvas.parentElement ?? canvas);

  // ── Keyboard-open face-zoom ─────────────────────────────────────────
  // When the mobile virtual keyboard opens, zoomToFace(true) is called.
  // We smoothly lerp the camera Z and orbit target Y towards face values
  // each animation frame via updateZoomTarget().
  let _keyboardOpen = false;

  function zoomToFace(enabled: boolean) {
    _keyboardOpen = enabled;
  }

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

  return { renderer, scene, camera, clock, controls, lookAtTarget, getRendererInfo, updateZoomTarget, zoomToFace, dispose };
}
