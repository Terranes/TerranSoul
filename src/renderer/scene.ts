import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

export interface RendererInfo {
  triangles: number;
  calls: number;
  programs: number;
}

/** Distance (metres) in front of the camera to place the eye-tracking target. */
export const EYE_TARGET_DISTANCE = 1.5;

export interface SceneContext {
  renderer: THREE.WebGLRenderer;
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  clock: THREE.Clock;
  controls: OrbitControls;
  lookAtTarget: THREE.Object3D;
  /** Pre-allocated scratch Vector3 used to read camera.getWorldDirection() each
   *  frame without allocating.  Owned by SceneContext so the render loop can
   *  reuse it for eye tracking. */
  _eyeForward: THREE.Vector3;
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
  /**
   * Register the current VRM scene so the ResizeObserver can re-frame the
   * camera when the canvas transitions from hidden (0×0) to visible.
   * This fixes the case where the model loads while the DOM is display:none
   * (e.g. v-show="!appLoading") and frameCameraToCharacter runs with
   * degenerate 1×1 dimensions.
   */
  setCurrentModel: (vrmScene: THREE.Object3D | null) => void;
  /**
   * Per-frame auto-resize check.  Compares the canvas display-size against
   * the last known dimensions and, if they changed, resizes the renderer
   * and camera.  Returns true when the size actually changed.
   * Call this every frame in the animation loop so the model is never
   * invisible due to a stale 1×1 backbuffer after a v-show transition.
   */
  checkResize: () => boolean;
  /** Shift the orbit-camera focus vertically by `offset` metres.  Used by
   *  the sitting-idle prop system to keep the seated character centred in
   *  the viewport when the animator translates the body downward. */
  setFocusYOffset: (offset: number) => void;
  /** Toggle visibility of the decorative pedestal (floor disc + ring) so
   *  the character stands on nothing in pet mode.  Accepts `true` to show
   *  the pedestal in desktop mode, `false` to hide it in pet mode. */
  setPedestalVisible: (visible: boolean) => void;
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

  renderer.setSize(canvas.clientWidth || 1, canvas.clientHeight || 1, false);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

  const scene = new THREE.Scene();

  const camera = new THREE.PerspectiveCamera(
    30,
    (canvas.clientWidth || 1) / (canvas.clientHeight || 1),
    0.02,
    1000,
  );
  // Full-body framing — camera at body centre height, pulled back.
  // On portrait screens (aspect < 1), pull back further so
  // the character's arms don't extend beyond the viewport edges.
  const CAMERA_Z_LANDSCAPE = 2.8;
  const CAMERA_Z_PORTRAIT = 3.8;
  const aspect = (canvas.clientWidth || 1) / (canvas.clientHeight || 1);
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
  // Additional Y shift applied on top of faceY/bodyY.  Tracks the animator's
  // body translation (e.g. for the seated idle) so the camera follows the
  // character down and keeps them centred.
  let focusYOffset = 0;

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
    if (isNaN(dist)) return; // camera not positioned yet
    const t = Math.max(0, Math.min(1, (dist - MIN_DIST) / (MAX_DIST - MIN_DIST)));
    const newY = faceY + t * (bodyY - faceY) + focusYOffset;
    if (isNaN(newY)) return; // faceY/bodyY not computed yet
    controls.target.y = newY;
  }

  /** Smoothly shift the camera orbit focus vertically.  Used so the seated
   *  idle keeps the character centred when the animator translates the body
   *  down.  Damped by the frame loop's per-frame updateZoomTarget. */
  let focusYOffsetTarget = 0;
  function setFocusYOffset(offset: number) {
    focusYOffsetTarget = offset;
  }
  // Damp focus offset each frame so transitions are smooth.
  // Invoked from updateZoomTarget below via a separate tick.
  function tickFocusYOffset() {
    const lambda = 6;
    // Frame time approximated by clock.getDelta() is consumed by the caller;
    // here we use a fixed 1/60 for deterministic easing.
    const dt = 1 / 60;
    focusYOffset += (focusYOffsetTarget - focusYOffset) * (1 - Math.exp(-lambda * dt));
  }
  // Hook into updateZoomTarget — call tickFocusYOffset before recomputing.
  const _origUpdateZoomTarget = updateZoomTarget;
  function updateZoomTargetWithFocus() {
    tickFocusYOffset();
    _origUpdateZoomTarget();
  }

  /**
   * Reframe the camera to fit the given VRM scene object.
   * Computes the world-space bounding box to get the character's real height,
   * then recalculates the face/body orbit-target heights and the full-body
   * camera distance based on the camera's vertical FOV.
   * Should be called each time a new model is loaded.
   */
  function frameCameraToCharacter(vrmScene: THREE.Object3D) {
    // Guard: if the canvas is hidden (display:none via v-show) its dimensions
    // are 0×0 which makes the aspect ratio NaN and poisons the camera position.
    // Bail out — the deferred-reframe mechanism in checkResize() will call us
    // again once the canvas has real dimensions.
    if (canvas.clientWidth <= 1 || canvas.clientHeight <= 1) return;

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

  // LookAt target — placed in scene (not on camera) for VRM eye tracking.
  // Positioned each frame a fixed distance in front of the camera using
  // camera.getWorldDirection() so the character's gaze tracks the viewer's
  // direction of view rather than the camera position itself.
  const lookAtTarget = new THREE.Object3D();
  scene.add(lookAtTarget);

  // Pre-allocated scratch vector used by the render loop to read
  // camera.getWorldDirection() each frame without allocation.
  const _eyeForward = new THREE.Vector3();

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

  function setPedestalVisible(visible: boolean) {
    pedestal.visible = visible;
    pedestalRing.visible = visible;
  }

  const clock = new THREE.Clock();

  // ── Deferred reframe state ────────────────────────────────────────
  // Tracks the current VRM scene so the ResizeObserver can re-frame the
  // camera when the canvas transitions from hidden (0×0) to real dimensions.
  let pendingReframeModel: THREE.Object3D | null = null;
  /** True once the canvas has had valid (>1) dimensions and been framed. */
  let hasValidSize = false;
  /** True when a model is loaded but hasn't been properly framed yet. */
  let needsReframe = false;
  /** Last known canvas dimensions — used by the render-loop auto-resize. */
  let lastKnownWidth = 0;
  let lastKnownHeight = 0;

  function setCurrentModel(vrmScene: THREE.Object3D | null) {
    pendingReframeModel = vrmScene;
    // If the canvas currently has degenerate dimensions, mark for deferred reframe
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    if (vrmScene && (w <= 1 || h <= 1)) {
      hasValidSize = false;
      needsReframe = true;
    }
  }

  /**
   * Called every frame from the animation loop.  Acts as a bulletproof
   * fallback for ResizeObserver — if the canvas display-size changed
   * (e.g. v-show transition, window resize, orientation change) the
   * renderer and camera are updated immediately so the model is never
   * invisible due to a stale 1×1 backbuffer.
   */
  function checkResize(): boolean {
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    if (w <= 0 || h <= 0) return false;

    const changed = w !== lastKnownWidth || h !== lastKnownHeight;
    if (changed) {
      lastKnownWidth = w;
      lastKnownHeight = h;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    }

    // Deferred reframe: model loaded while canvas was hidden/tiny
    if ((!hasValidSize || needsReframe) && w > 1 && h > 1 && pendingReframeModel) {
      hasValidSize = true;
      needsReframe = false;
      frameCameraToCharacter(pendingReframeModel);
      return true;
    }

    if (changed) {
      // Adjust camera distance for portrait vs landscape
      const currentDist = controls.getDistance();
      const targetZ = (w / h) < 1 ? defaultCameraZPortrait : defaultCameraZLandscape;
      const ZOOM_TOLERANCE = 0.1;
      if (Math.abs(currentDist - defaultCameraZLandscape) < ZOOM_TOLERANCE ||
          Math.abs(currentDist - defaultCameraZPortrait)  < ZOOM_TOLERANCE) {
        camera.position.setZ(targetZ);
      }
    }
    return changed;
  }

  // Use ResizeObserver for accurate per-element resize handling.
  // checkResize() in the render loop is the primary mechanism; this
  // observer triggers a render tick so idle-throttled frames pick up
  // the change promptly.
  const resizeObserver = new ResizeObserver(() => {
    // Handled by checkResize() in the render loop — just poke a render.
    checkResize();
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

  return { renderer, scene, camera, clock, controls, lookAtTarget, _eyeForward, getRendererInfo, updateZoomTarget: updateZoomTargetWithFocus, frameCameraToCharacter, setCurrentModel, checkResize, setFocusYOffset, setPedestalVisible, dispose, onCameraChange };
}
