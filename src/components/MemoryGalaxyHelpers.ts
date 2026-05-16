/**
 * MemoryGalaxy helper utilities — pure functions split out of
 * `MemoryGalaxy.vue` to keep the SFC under the project's `max-lines`
 * budget. All functions here are stateless or self-contained.
 */
import * as THREE from 'three';
import type { MemoryEntry } from '../types';
import type { CognitiveKind } from '../utils/cognitive-kind';

// ── Planet specs (one per cognitive kind) ────────────────────────────────
export interface PlanetSpec {
  key: CognitiveKind;
  name: string;
  eyebrow: string;
  blurb: string;
  color: number;
  glow: number;
  memories: MemoryEntry[];
  orbitRadius: number;
  orbitSpeed: number;
  planetRadius: number;
  group?: THREE.Group;
  pivot?: THREE.Group;
  mesh?: THREE.Mesh;
}

export type PlanetDef = Omit<PlanetSpec, 'memories' | 'orbitRadius' | 'orbitSpeed' | 'planetRadius'>;

export const PLANET_DEFS: PlanetDef[] = [
  {
    key: 'episodic',
    name: 'Episodic',
    eyebrow: 'Events · Timeline',
    blurb: 'Time-anchored memories of what happened, when, and where.',
    color: 0xe89b6b,
    glow: 0xffb37a,
  },
  {
    key: 'semantic',
    name: 'Semantic',
    eyebrow: 'Concepts · Facts',
    blurb: 'Durable concepts, facts, and preferences that hold across time.',
    color: 0x6bb5e8,
    glow: 0x9ecdf2,
  },
  {
    key: 'procedural',
    name: 'Procedural',
    eyebrow: 'How-to · Skills',
    blurb: 'Step-by-step procedures, workflows, and learned methods.',
    color: 0x8ccd9e,
    glow: 0xb6e1c2,
  },
  {
    key: 'judgment',
    name: 'Judgment',
    eyebrow: 'Decisions · Principles',
    blurb: 'Considered decisions, principles, and reflective conclusions.',
    color: 0xb9ace8,
    glow: 0xd1c5f5,
  },
];

// ── Deterministic PRNG (mulberry32) ──────────────────────────────────────
export function mulberry32(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x6D2B79F5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// ── Texture builders ─────────────────────────────────────────────────────
export function makeStarTexture(): THREE.Texture {
  const size = 64;
  const c = document.createElement('canvas');
  c.width = c.height = size;
  const ctx = c.getContext('2d')!;
  const g = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
  g.addColorStop(0, 'rgba(255,255,255,1)');
  g.addColorStop(0.25, 'rgba(255,255,255,0.85)');
  g.addColorStop(0.55, 'rgba(255,255,255,0.18)');
  g.addColorStop(1, 'rgba(255,255,255,0)');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, size, size);
  const tex = new THREE.CanvasTexture(c);
  tex.colorSpace = THREE.SRGBColorSpace;
  return tex;
}

export function makeGlowTexture(hex: number): THREE.Texture {
  const size = 256;
  const c = document.createElement('canvas');
  c.width = c.height = size;
  const ctx = c.getContext('2d')!;
  const col = new THREE.Color(hex);
  const r = Math.round(col.r * 255);
  const g = Math.round(col.g * 255);
  const b = Math.round(col.b * 255);
  const grd = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
  grd.addColorStop(0.0, `rgba(${r},${g},${b},0.9)`);
  grd.addColorStop(0.35, `rgba(${r},${g},${b},0.35)`);
  grd.addColorStop(0.75, `rgba(${r},${g},${b},0.08)`);
  grd.addColorStop(1.0, `rgba(${r},${g},${b},0)`);
  ctx.fillStyle = grd;
  ctx.fillRect(0, 0, size, size);
  const tex = new THREE.CanvasTexture(c);
  tex.colorSpace = THREE.SRGBColorSpace;
  return tex;
}

export function makeNebulaTexture(hue: number): THREE.Texture {
  const size = 512;
  const c = document.createElement('canvas');
  c.width = c.height = size;
  const ctx = c.getContext('2d')!;
  const grd = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
  grd.addColorStop(0.0, `hsla(${hue},70%,65%,0.55)`);
  grd.addColorStop(0.4, `hsla(${hue},65%,40%,0.18)`);
  grd.addColorStop(1.0, `hsla(${hue},60%,20%,0)`);
  ctx.fillStyle = grd;
  ctx.fillRect(0, 0, size, size);
  const tex = new THREE.CanvasTexture(c);
  tex.colorSpace = THREE.SRGBColorSpace;
  return tex;
}

// ── Starfield builder ────────────────────────────────────────────────────
export function buildStars(
  count: number,
  radius: number,
  sizeMin: number,
  sizeMax: number,
  tints: number[],
): THREE.Points {
  const positions = new Float32Array(count * 3);
  const colors = new Float32Array(count * 3);
  const sizes = new Float32Array(count);
  const rng = mulberry32(0xc0ffee ^ count);
  const tmp = new THREE.Color();
  for (let i = 0; i < count; i++) {
    const u = rng() * 2 - 1;
    const t = rng() * Math.PI * 2;
    const r = radius * (0.55 + rng() * 0.45);
    const s = Math.sqrt(1 - u * u);
    positions[i * 3 + 0] = r * s * Math.cos(t);
    positions[i * 3 + 1] = r * u;
    positions[i * 3 + 2] = r * s * Math.sin(t);
    const tint = tints[Math.floor(rng() * tints.length)];
    tmp.setHex(tint);
    colors[i * 3 + 0] = tmp.r;
    colors[i * 3 + 1] = tmp.g;
    colors[i * 3 + 2] = tmp.b;
    sizes[i] = sizeMin + rng() * (sizeMax - sizeMin);
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  geo.setAttribute('size', new THREE.BufferAttribute(sizes, 1));

  const mat = new THREE.ShaderMaterial({
    uniforms: { uMap: { value: makeStarTexture() } },
    vertexShader: `
      attribute float size;
      varying vec3 vColor;
      void main() {
        vColor = color;
        vec4 mv = modelViewMatrix * vec4(position, 1.0);
        gl_PointSize = size * (300.0 / -mv.z);
        gl_Position = projectionMatrix * mv;
      }
    `,
    fragmentShader: `
      uniform sampler2D uMap;
      varying vec3 vColor;
      void main() {
        vec4 t = texture2D(uMap, gl_PointCoord);
        if (t.a < 0.02) discard;
        gl_FragColor = vec4(vColor, 1.0) * t;
      }
    `,
    vertexColors: true,
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
  });
  return new THREE.Points(geo, mat);
}

// ── Planet shader (fbm + fresnel rim) ────────────────────────────────────
const PLANET_VS = /* glsl */ `
  varying vec3 vN;
  varying vec3 vV;
  varying vec3 vP;
  void main() {
    vN = normalize(normalMatrix * normal);
    vec4 mv = modelViewMatrix * vec4(position, 1.0);
    vV = normalize(-mv.xyz);
    vP = position;
    gl_Position = projectionMatrix * mv;
  }
`;

const PLANET_FS = /* glsl */ `
  uniform vec3 uColor;
  uniform vec3 uGlow;
  uniform float uTime;
  uniform float uOpacity;
  varying vec3 vN;
  varying vec3 vV;
  varying vec3 vP;

  float hash(vec3 p) {
    p = fract(p * 0.3183099 + 0.1);
    p *= 17.0;
    return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
  }
  float noise(vec3 x) {
    vec3 i = floor(x);
    vec3 f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
      mix(mix(hash(i + vec3(0,0,0)), hash(i + vec3(1,0,0)), f.x),
          mix(hash(i + vec3(0,1,0)), hash(i + vec3(1,1,0)), f.x), f.y),
      mix(mix(hash(i + vec3(0,0,1)), hash(i + vec3(1,0,1)), f.x),
          mix(hash(i + vec3(0,1,1)), hash(i + vec3(1,1,1)), f.x), f.y),
      f.z);
  }
  float fbm(vec3 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 5; i++) {
      v += a * noise(p);
      p *= 2.05;
      a *= 0.5;
    }
    return v;
  }

  void main() {
    float n = fbm(vP * 1.4 + vec3(uTime * 0.04, 0.0, uTime * 0.02));
    vec3 base = mix(uColor * 0.45, uColor, smoothstep(0.35, 0.85, n));
    base += uGlow * 0.18 * smoothstep(0.7, 1.0, n);
    float fres = pow(1.0 - max(dot(vN, vV), 0.0), 2.6);
    vec3 rim = uGlow * fres * 1.4;
    vec3 col = base + rim;
    gl_FragColor = vec4(col, uOpacity);
  }
`;

export function makePlanetMaterial(color: number, glow: number): THREE.ShaderMaterial {
  return new THREE.ShaderMaterial({
    uniforms: {
      uColor: { value: new THREE.Color(color) },
      uGlow: { value: new THREE.Color(glow) },
      uTime: { value: 0 },
      uOpacity: { value: 1.0 },
    },
    vertexShader: PLANET_VS,
    fragmentShader: PLANET_FS,
    transparent: true,
  });
}

// ── Easing ───────────────────────────────────────────────────────────────
export function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

// ── Text label sprite (HUD-style overlay above planets / nodes) ──────────
export function makeLabelSprite(text: string, color = '#ffffff', scale = 1): THREE.Sprite {
  const canvas = document.createElement('canvas');
  const padX = 18;
  const padY = 8;
  const fontPx = 28;
  const c = canvas.getContext('2d')!;
  c.font = `600 ${fontPx}px system-ui, -apple-system, "Segoe UI", sans-serif`;
  const metrics = c.measureText(text);
  const w = Math.ceil(metrics.width) + padX * 2;
  const h = fontPx + padY * 2;
  canvas.width = w;
  canvas.height = h;
  // Re-set font after canvas resize (it clears context state)
  c.font = `600 ${fontPx}px system-ui, -apple-system, "Segoe UI", sans-serif`;
  c.fillStyle = 'rgba(8,10,22,0.78)';
  const r = 10;
  c.beginPath();
  c.moveTo(r, 0);
  c.lineTo(w - r, 0); c.quadraticCurveTo(w, 0, w, r);
  c.lineTo(w, h - r); c.quadraticCurveTo(w, h, w - r, h);
  c.lineTo(r, h); c.quadraticCurveTo(0, h, 0, h - r);
  c.lineTo(0, r); c.quadraticCurveTo(0, 0, r, 0);
  c.closePath();
  c.fill();
  c.strokeStyle = 'rgba(255,255,255,0.18)';
  c.lineWidth = 1;
  c.stroke();
  c.fillStyle = color;
  c.textBaseline = 'middle';
  c.fillText(text, padX, h / 2);
  const tex = new THREE.CanvasTexture(canvas);
  tex.needsUpdate = true;
  const mat = new THREE.SpriteMaterial({ map: tex, transparent: true, depthWrite: false });
  const sprite = new THREE.Sprite(mat);
  // Keep the sprite's aspect, scale by world units.
  const worldH = 0.5 * scale;
  sprite.scale.set((w / h) * worldH, worldH, 1);
  return sprite;
}
