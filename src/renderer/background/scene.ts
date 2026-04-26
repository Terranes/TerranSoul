/**
 * Three.js-based animated background scene.
 *
 * Renders a single fullscreen quad with a fragment shader (see
 * `shader.glsl.ts`) whose colour palette comes from the active theme's
 * CSS tokens (see `palette.ts`) and whose visual personality is chosen
 * per-theme (see `preset.ts`).
 *
 * Performance / accessibility guardrails (see plan §4):
 *   • DPR clamped to min(devicePixelRatio, 1.5)
 *   • ~30 FPS frame gate via delta-time accumulator
 *   • Render loop pauses on document.visibilitychange / hidden
 *   • prefers-reduced-motion: render a single static frame, never animate
 *   • Graceful fallback when WebGLRenderer construction throws — caller
 *     keeps the body's --ts-bg-gradient as a backdrop.
 */

import {
  Color,
  Mesh,
  OrthographicCamera,
  PlaneGeometry,
  Scene,
  ShaderMaterial,
  Vector2,
  WebGLRenderer,
} from 'three';

import {
  clonePalette,
  lerpPalette,
  readPalette,
  watchTheme,
  type Palette,
} from './palette';
import { getPreset, MODE_INDEX } from './preset';
import { FRAGMENT_SHADER, VERTEX_SHADER } from './shader.glsl';

const FRAME_INTERVAL_MS = 1000 / 30;       // ~30 FPS gate
const MAX_DPR           = 1.5;
const TWEEN_MS          = 600;             // palette transition duration
/**
 * Per-frame ease-out factor for the palette lerp: each frame closes
 * 25 % of the remaining distance to the target palette, producing a
 * smooth exponential decay rather than a linear ramp.  Lower = slower.
 */
const PALETTE_LERP_RATE = 0.25;
const MIN_RENDERER_FACTORY = (canvas: HTMLCanvasElement): WebGLRenderer =>
  new WebGLRenderer({
    canvas,
    alpha: true,
    antialias: false,
    powerPreference: 'low-power',
    premultipliedAlpha: true,
  });

export interface BackgroundSceneOptions {
  /** DOM element receiving the canvas; defaults to `document.body`. */
  parent?: HTMLElement;
  /** Theme id; defaults to `<html data-theme>` or `'default'`. */
  themeId?: string;
  /**
   * Override the renderer factory — exclusively for tests.  Production
   * always uses the default `WebGLRenderer`.
   */
  rendererFactory?: (canvas: HTMLCanvasElement) => WebGLRenderer;
}

/** Public handle returned by {@link createBackgroundScene}. */
export interface BackgroundScene {
  /** The injected canvas element (so callers can style/test it). */
  readonly canvas: HTMLCanvasElement;
  /** Manually trigger a re-read of `<html data-theme>`'s palette. */
  refreshPalette(): void;
  /** Tear down everything — canvas, renderer, listeners, animation. */
  dispose(): void;
}

interface InternalState {
  renderer: WebGLRenderer;
  scene: Scene;
  camera: OrthographicCamera;
  material: ShaderMaterial;
  current: Palette;
  target: Palette;
  tweenStartMs: number;
  rafId: number | null;
  lastFrameMs: number;
  startMs: number;
  reducedMotion: boolean;
  paused: boolean;
}

function currentThemeId(): string {
  if (typeof document === 'undefined') return 'default';
  return document.documentElement.dataset.theme || 'default';
}

function makeCanvas(parent: HTMLElement): HTMLCanvasElement {
  const canvas = document.createElement('canvas');
  canvas.setAttribute('aria-hidden', 'true');
  canvas.dataset.tsBackground = '';
  // Inline styles so the canvas works even before any CSS has loaded.
  Object.assign(canvas.style, {
    position:      'fixed',
    inset:         '0',
    width:         '100%',
    height:        '100%',
    zIndex:        '-1',
    pointerEvents: 'none',
    display:       'block',
  } as Partial<CSSStyleDeclaration>);
  parent.appendChild(canvas);
  return canvas;
}

/**
 * Construct the renderer + scene + uniforms.  Returns null when WebGL
 * is unavailable so the caller can fall back to the CSS gradient.
 */
function buildState(
  canvas: HTMLCanvasElement,
  themeId: string,
  factory: (c: HTMLCanvasElement) => WebGLRenderer,
): InternalState | null {
  let renderer: WebGLRenderer;
  try {
    renderer = factory(canvas);
  } catch {
    return null;
  }
  renderer.setPixelRatio(Math.min(
    typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1,
    MAX_DPR,
  ));
  renderer.setClearColor(new Color(0x000000), 0);

  const scene  = new Scene();
  const camera = new OrthographicCamera(-1, 1, 1, -1, 0, 1);
  const preset = getPreset(themeId);
  const initialPalette = readPalette();

  const material = new ShaderMaterial({
    vertexShader:   VERTEX_SHADER,
    fragmentShader: FRAGMENT_SHADER,
    depthTest:      false,
    depthWrite:     false,
    transparent:    true,
    uniforms: {
      uTime:       { value: 0 },
      uResolution: { value: new Vector2(1, 1) },
      uC1:         { value: initialPalette.c1     },
      uC2:         { value: initialPalette.c2     },
      uC3:         { value: initialPalette.c3     },
      uAccent:     { value: initialPalette.accent },
      uIntensity:  { value: preset.intensity      },
      uSpeed:      { value: preset.speed          },
      uMode:       { value: MODE_INDEX[preset.mode] },
    },
  });
  scene.add(new Mesh(new PlaneGeometry(2, 2), material));

  const reducedMotion =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  return {
    renderer,
    scene,
    camera,
    material,
    current:      clonePalette(initialPalette),
    target:       initialPalette,
    tweenStartMs: 0,
    rafId:        null,
    lastFrameMs:  0,
    startMs:      typeof performance !== 'undefined' ? performance.now() : 0,
    reducedMotion,
    paused:       false,
  };
}

function applyPaletteToUniforms(state: InternalState): void {
  const u = state.material.uniforms;
  u.uC1.value     = state.current.c1;
  u.uC2.value     = state.current.c2;
  u.uC3.value     = state.current.c3;
  u.uAccent.value = state.current.accent;
}

function applyPresetToUniforms(state: InternalState, themeId: string): void {
  const preset = getPreset(themeId);
  state.material.uniforms.uIntensity.value = preset.intensity;
  state.material.uniforms.uSpeed.value     = preset.speed;
  state.material.uniforms.uMode.value      = MODE_INDEX[preset.mode];
}

/**
 * Resize the renderer to match the parent (window) dimensions.
 * Cheap to call repeatedly — Three.js skips the GL call when the
 * size is unchanged.
 */
function syncSize(state: InternalState): void {
  if (typeof window === 'undefined') return;
  const w = Math.max(1, window.innerWidth);
  const h = Math.max(1, window.innerHeight);
  state.renderer.setSize(w, h, false);
  state.material.uniforms.uResolution.value.set(w, h);
}

export function createBackgroundScene(
  options: BackgroundSceneOptions = {},
): BackgroundScene | null {
  if (typeof document === 'undefined') return null;
  const parent = options.parent ?? document.body;
  const themeId = options.themeId ?? currentThemeId();
  const factory = options.rendererFactory ?? MIN_RENDERER_FACTORY;

  const canvas = makeCanvas(parent);
  const state = buildState(canvas, themeId, factory);
  if (!state) {
    canvas.remove();
    return null;
  }

  syncSize(state);

  // ── Resize ───────────────────────────────────────────────────────
  const onResize = (): void => syncSize(state);
  window.addEventListener('resize', onResize, { passive: true });

  // ── Visibility ───────────────────────────────────────────────────
  const onVisibilityChange = (): void => {
    state.paused = document.hidden;
    if (!state.paused) scheduleFrame();
  };
  document.addEventListener('visibilitychange', onVisibilityChange);

  // ── Theme change tween ───────────────────────────────────────────
  const stopThemeWatch = watchTheme((next) => {
    state.target = next;
    state.tweenStartMs = (typeof performance !== 'undefined'
      ? performance.now()
      : 0);
    applyPresetToUniforms(state, currentThemeId());
    if (state.reducedMotion) {
      // Snap immediately and render a single frame.
      state.current = clonePalette(state.target);
      applyPaletteToUniforms(state);
      state.renderer.render(state.scene, state.camera);
    } else {
      scheduleFrame();
    }
  });

  // ── Render loop ──────────────────────────────────────────────────
  const renderOnce = (nowMs: number): void => {
    // Palette tween
    if (state.tweenStartMs > 0) {
      const tween = Math.min(1, (nowMs - state.tweenStartMs) / TWEEN_MS);
      lerpPalette(state.current, state.target, tween * PALETTE_LERP_RATE);
      applyPaletteToUniforms(state);
      if (tween >= 1) state.tweenStartMs = 0;
    }
    state.material.uniforms.uTime.value = (nowMs - state.startMs) / 1000;
    state.renderer.render(state.scene, state.camera);
  };

  // `scheduleFrame` is declared first so `tick` can call it without
  // tripping TS narrowing on the captured `state` const.
  let scheduleFrame: () => void = () => {};

  const tick = (nowMs: number): void => {
    state.rafId = null;
    if (state.paused) return;
    if (nowMs - state.lastFrameMs >= FRAME_INTERVAL_MS) {
      state.lastFrameMs = nowMs;
      renderOnce(nowMs);
    }
    if (!state.reducedMotion || state.tweenStartMs > 0) {
      scheduleFrame();
    }
  };

  scheduleFrame = (): void => {
    if (state.rafId !== null) return;
    if (typeof requestAnimationFrame === 'undefined') return;
    state.rafId = requestAnimationFrame(tick);
  };

  // First frame — always render once even under reduced-motion so the
  // palette is visible.
  renderOnce(state.startMs);
  if (!state.reducedMotion) scheduleFrame();

  // ── Public handle ────────────────────────────────────────────────
  return {
    canvas,
    refreshPalette(): void {
      state.target = readPalette();
      state.tweenStartMs = (typeof performance !== 'undefined'
        ? performance.now()
        : 0);
      applyPresetToUniforms(state, currentThemeId());
      scheduleFrame();
    },
    dispose(): void {
      window.removeEventListener('resize', onResize);
      document.removeEventListener('visibilitychange', onVisibilityChange);
      stopThemeWatch();
      if (state.rafId !== null && typeof cancelAnimationFrame !== 'undefined') {
        cancelAnimationFrame(state.rafId);
      }
      state.rafId = null;
      state.material.dispose();
      // Mesh is the only child; dispose its geometry.
      state.scene.traverse((obj) => {
        if ((obj as Mesh).geometry) (obj as Mesh).geometry.dispose();
      });
      state.renderer.dispose();
      canvas.remove();
    },
  };
}
