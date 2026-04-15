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
   * Reframe the camera to fit a newly-loaded VRM scene.
   * Computes the actual bounding box of the model, derives the correct
   * face/body orbit-target heights, and resets the camera to a full-body
   * view — so every character appears centred regardless of their height.
   */
  frameCameraToCharacter: (vrmScene: THREE.Object3D) => void;
  dispose: () => void;
  /** Register a callback that fires after the user finishes orbiting or zooming.
   *  Receives (azimuth, distance) so the caller can persist the camera state. */
  onCameraChange: (cb: (azimuth: number, distance: number) => void) => void;
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

  // Heights for zoom-dependent orbit target — updated by frameCameraToCharacter
  // once the real model is loaded and its bounding box is known.
  let faceY = 1.45;    // orbit target Y when zoomed in (face)
  let bodyY = 0.65;    // orbit target Y when zoomed out (full body, head to toes)

  // Default full-body camera distances used by the ResizeObserver — updated by
  // frameCameraToCharacter so portrait/landscape switching still works after load.
  let defaultCameraZLandscape = CAMERA_Z_LANDSCAPE;
  let defaultCameraZPortrait  = CAMERA_Z_PORTRAIT;

  /**
   * Smoothly adjusts the orbit target height based on zoom distance so
   * zooming in frames the face and zooming out shows the entire body.
   * Must be called each frame before controls.update().
   */
  function updateZoomTarget() {
    const dist = controls.getDistance();
    const t = Math.max(0, Math.min(1, (dist - MIN_DIST) / (MAX_DIST - MIN_DIST)));
    controls.target.y = faceY + t * (bodyY - faceY);
  }

  /**
   * Reframe the camera to fit the given VRM scene object.
   * Computes the world-space bounding box to get the character's real height,
   * then recalculates the face/body orbit-target heights and the full-body
   * camera distance based on the camera's vertical FOV.
   * Should be called each time a new model is loaded.
   */
  function frameCameraToCharacter(vrmScene: THREE.Object3D) {
    const box = new THREE.Box3().setFromObject(vrmScene);
    if (box.isEmpty()) return;

    const size = new THREE.Vector3();
    box.getSize(size);
    const charHeight = size.y;
    const charTop    = box.max.y;
    const charBottom = box.min.y;

    // Orbit target heights derived from the character's actual dimensions.
    // Face target: 12% below the crown (roughly eye/chin level).
    // Body target: vertical midpoint (keeps the full body centred when zoomed out).
    faceY = charTop - charHeight * 0.12;
    bodyY = charBottom + charHeight * 0.50;

    // Minimum camera distance to fit the full character height inside the
    // vertical FOV, with 10% padding above and below.
    const vFovRad = THREE.MathUtils.degToRad(camera.fov);
    const padding = 1.10;
    const heightBasedDist = (charHeight * padding / 2) / Math.tan(vFovRad / 2);

    // In portrait mode the character's arm span can clip the horizontal edges —
    // also check the width-based minimum and take the larger of the two.
    const clamp = (v: number) => Math.min(Math.max(v, MIN_DIST + 0.1), MAX_DIST);

    const aspectNow = canvas.clientWidth / canvas.clientHeight;
    const widthBasedLandscape = size.x > 0
      ? (size.x * padding / 2) / (Math.tan(vFovRad / 2) * Math.max(aspectNow, 1))
      : 0;
    const widthBasedPortrait = size.x > 0
      ? (size.x * padding / 2) / (Math.tan(vFovRad / 2) * Math.min(aspectNow, 1))
      : 0;

    defaultCameraZLandscape = clamp(Math.max(heightBasedDist, widthBasedLandscape));
    defaultCameraZPortrait  = clamp(Math.max(heightBasedDist, widthBasedPortrait));

    // Reset camera to the full-body view, preserving the current azimuth so
    // the character's facing direction is unchanged.
    const fullBodyDist = aspectNow < 1 ? defaultCameraZPortrait : defaultCameraZLandscape;
    const azimuth = Math.atan2(camera.position.x, camera.position.z);
    camera.position.set(
      fullBodyDist * Math.sin(azimuth),
      bodyY,
      fullBodyDist * Math.cos(azimuth),
    );
    controls.target.set(0, bodyY, 0);
    controls.update();
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
    const currentDist = controls.getDistance();
    const targetZ = (w / h) < 1 ? defaultCameraZPortrait : defaultCameraZLandscape;
    const ZOOM_TOLERANCE = 0.1;
    // Only adjust if the user hasn't manually zoomed away from the default.
    if (Math.abs(currentDist - defaultCameraZLandscape) < ZOOM_TOLERANCE ||
        Math.abs(currentDist - defaultCameraZPortrait)  < ZOOM_TOLERANCE) {
      camera.position.setZ(targetZ);
    }
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

  let cameraChangeCallback: ((azimuth: number, distance: number) => void) | null = null;

  // Fire the camera change callback when the user finishes orbiting/zooming.
  controls.addEventListener('end', () => {
    if (cameraChangeCallback) {
      const sph = new THREE.Spherical().setFromVector3(
        camera.position.clone().sub(controls.target),
      );
      cameraChangeCallback(sph.theta, sph.radius);
    }
  });

  function onCameraChange(cb: (azimuth: number, distance: number) => void) {
    cameraChangeCallback = cb;
  }

  function dispose() {
    resizeObserver.disconnect();
    controls.dispose();
    renderer.dispose();
  }

  return { renderer, scene, camera, clock, controls, lookAtTarget, getRendererInfo, updateZoomTarget, frameCameraToCharacter, dispose, onCameraChange };
}
