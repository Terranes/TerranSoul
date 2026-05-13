import {
  AdditiveBlending,
  BufferAttribute,
  BufferGeometry,
  CanvasTexture,
  Float32BufferAttribute,
  LineSegments,
  Points,
  ShaderMaterial,
  type Texture,
} from 'three';

/**
 * Radial alpha gradient used as the sprite texture for additive Points
 * layers (core / glow / halo). Produces the soft galaxy-star look.
 */
export function makeSpriteTexture(): CanvasTexture {
  const size = 128;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d')!;
  const gradient = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
  gradient.addColorStop(0.0, 'rgba(255,255,255,1)');
  gradient.addColorStop(0.25, 'rgba(255,255,255,0.85)');
  gradient.addColorStop(0.55, 'rgba(255,255,255,0.25)');
  gradient.addColorStop(1.0, 'rgba(255,255,255,0)');
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, size, size);
  return new CanvasTexture(canvas);
}

export const NODE_VS = `
  attribute float size;
  attribute vec3 nodeColor;
  attribute float emphasis;
  varying vec3 vColor;
  varying float vEmphasis;
  void main() {
    vColor = nodeColor;
    vEmphasis = emphasis;
    vec4 mv = modelViewMatrix * vec4(position, 1.0);
    gl_PointSize = size * (320.0 / -mv.z);
    gl_Position = projectionMatrix * mv;
  }
`;

export const NODE_FS = `
  uniform sampler2D map;
  varying vec3 vColor;
  varying float vEmphasis;
  void main() {
    vec2 uv = gl_PointCoord;
    vec4 tex = texture2D(map, uv);
    if (tex.a < 0.02) discard;
    vec3 c = vColor * (0.7 + vEmphasis * 0.9);
    gl_FragColor = vec4(c, tex.a);
  }
`;

export const GLOW_FS = `
  uniform sampler2D map;
  varying vec3 vColor;
  varying float vEmphasis;
  void main() {
    vec2 uv = gl_PointCoord;
    vec4 tex = texture2D(map, uv);
    if (tex.a < 0.01) discard;
    float a = tex.a * (0.18 + vEmphasis * 0.42);
    gl_FragColor = vec4(vColor, a);
  }
`;

export const HALO_FS = `
  uniform sampler2D map;
  varying vec3 vColor;
  varying float vEmphasis;
  void main() {
    vec2 uv = gl_PointCoord;
    vec4 tex = texture2D(map, uv);
    if (tex.a < 0.005) discard;
    gl_FragColor = vec4(vColor, tex.a * 0.55 * vEmphasis);
  }
`;

export function makeNodeMaterial(sprite: Texture, fragmentShader: string): ShaderMaterial {
  return new ShaderMaterial({
    uniforms: { map: { value: sprite } },
    vertexShader: NODE_VS,
    fragmentShader,
    transparent: true,
    depthWrite: false,
    blending: AdditiveBlending,
  });
}

/**
 * Builds a deep-space starfield (additive glow Points on a spherical shell)
 * for use as a background layer. Returned `Points` should be added directly
 * to the scene (not the rotating world group) so it remains stable.
 */
export function buildStarfield(sprite: Texture): Points {
  const count = 1400;
  const positions = new Float32Array(count * 3);
  const colors = new Float32Array(count * 3);
  const sizes = new Float32Array(count);
  const emphasis = new Float32Array(count);
  for (let i = 0; i < count; i++) {
    const r = 900 + Math.random() * 700;
    const phi = Math.acos(2 * Math.random() - 1);
    const theta = Math.random() * Math.PI * 2;
    positions[i * 3 + 0] = r * Math.sin(phi) * Math.cos(theta);
    positions[i * 3 + 1] = r * Math.cos(phi);
    positions[i * 3 + 2] = r * Math.sin(phi) * Math.sin(theta);
    const shade = 0.4 + Math.random() * 0.6;
    colors[i * 3 + 0] = shade * 0.9;
    colors[i * 3 + 1] = shade * 0.95;
    colors[i * 3 + 2] = shade;
    sizes[i] = 0.6 + Math.random() * 1.4;
    emphasis[i] = 0.4 + Math.random() * 0.4;
  }
  const geo = new BufferGeometry();
  geo.setAttribute('position', new BufferAttribute(positions, 3));
  geo.setAttribute('nodeColor', new BufferAttribute(colors, 3));
  geo.setAttribute('size', new BufferAttribute(sizes, 1));
  geo.setAttribute('emphasis', new BufferAttribute(emphasis, 1));
  const stars = new Points(geo, makeNodeMaterial(sprite, GLOW_FS));
  stars.frustumCulled = false;
  return stars;
}

// --- Twinkle starfield (multi-layer, time-driven shimmer) ---

/** Vertex shader for twinkling stars — accepts a uTime uniform for shimmer. */
export const TWINKLE_STAR_VS = `
  uniform float uTime;
  attribute float size;
  attribute vec3 nodeColor;
  attribute float emphasis;
  attribute float phase;
  varying vec3 vColor;
  varying float vAlpha;
  void main() {
    vColor = nodeColor;
    float twinkle = 0.82 + 0.18 * sin(uTime * 0.6 + phase);
    vAlpha = (0.18 + emphasis * 0.42) * twinkle;
    vec4 mv = modelViewMatrix * vec4(position, 1.0);
    gl_PointSize = size * (320.0 / -mv.z) * (0.92 + 0.08 * twinkle);
    gl_Position = projectionMatrix * mv;
  }
`;

export const TWINKLE_STAR_FS = `
  uniform sampler2D map;
  varying vec3 vColor;
  varying float vAlpha;
  void main() {
    vec2 uv = gl_PointCoord;
    vec4 tex = texture2D(map, uv);
    if (tex.a < 0.01) discard;
    gl_FragColor = vec4(vColor, tex.a * vAlpha);
  }
`;

export interface TwinkleStarfieldResult {
  bg: Points;
  fg: Points;
  /** Call each frame with performance.now() / 1000 to drive twinkle. */
  update(timeSec: number): void;
}

/**
 * Two-layer twinkling starfield for the galaxy aesthetic.
 * - Background layer: ~3000 stars, radius 600–1600, small, mostly white
 * - Foreground layer: ~600 stars, radius 80–300, slightly larger, tinted
 *
 * Both layers share a uTime uniform that drives per-star sinusoidal shimmer
 * via a per-vertex `phase` attribute seeded from position hash.
 */
export function buildTwinkleStarfield(sprite: Texture): TwinkleStarfieldResult {
  const bgCount = 3000;
  const fgCount = 600;

  function makeLayer(count: number, rMin: number, rMax: number, sizeMin: number, sizeMax: number, tints: [number, number, number][]) {
    const positions = new Float32Array(count * 3);
    const colors = new Float32Array(count * 3);
    const sizes = new Float32Array(count);
    const emphasis = new Float32Array(count);
    const phase = new Float32Array(count);
    for (let i = 0; i < count; i++) {
      const r = rMin + Math.random() * (rMax - rMin);
      const phi = Math.acos(2 * Math.random() - 1);
      const theta = Math.random() * Math.PI * 2;
      const px = r * Math.sin(phi) * Math.cos(theta);
      const py = r * Math.cos(phi);
      const pz = r * Math.sin(phi) * Math.sin(theta);
      positions[i * 3] = px;
      positions[i * 3 + 1] = py;
      positions[i * 3 + 2] = pz;
      const tint = tints[Math.floor(Math.random() * tints.length)];
      colors[i * 3] = tint[0];
      colors[i * 3 + 1] = tint[1];
      colors[i * 3 + 2] = tint[2];
      sizes[i] = sizeMin + Math.random() * (sizeMax - sizeMin);
      emphasis[i] = 0.35 + Math.random() * 0.55;
      phase[i] = px * 0.7 + pz * 0.4 + py * 0.3;
    }
    const geo = new BufferGeometry();
    geo.setAttribute('position', new BufferAttribute(positions, 3));
    geo.setAttribute('nodeColor', new BufferAttribute(colors, 3));
    geo.setAttribute('size', new BufferAttribute(sizes, 1));
    geo.setAttribute('emphasis', new BufferAttribute(emphasis, 1));
    geo.setAttribute('phase', new BufferAttribute(phase, 1));
    return geo;
  }

  const bgTints: [number, number, number][] = [
    [0.92, 0.94, 1.0],   // cool white
    [0.88, 0.90, 0.96],  // blue-white
    [1.0, 0.96, 0.90],   // warm white
  ];
  const fgTints: [number, number, number][] = [
    [0.72, 0.67, 0.91],  // violet (--ts-galaxy-violet family)
    [0.59, 0.73, 0.73],  // teal (--ts-galaxy-teal family)
    [0.91, 0.81, 0.59],  // gold (--ts-galaxy-gold family)
    [0.64, 0.79, 0.55],  // sage (--ts-galaxy-sage family)
    [0.92, 0.94, 1.0],   // white
  ];

  const bgGeo = makeLayer(bgCount, 600, 1600, 0.5, 1.5, bgTints);
  const fgGeo = makeLayer(fgCount, 80, 300, 1.0, 3.2, fgTints);

  const uTimeBg = { value: 0 };
  const uTimeFg = { value: 0 };

  const makeMat = (uTime: { value: number }) => new ShaderMaterial({
    uniforms: { map: { value: sprite }, uTime },
    vertexShader: TWINKLE_STAR_VS,
    fragmentShader: TWINKLE_STAR_FS,
    transparent: true,
    depthWrite: false,
    blending: AdditiveBlending,
  });

  const bg = new Points(bgGeo, makeMat(uTimeBg));
  bg.frustumCulled = false;
  const fg = new Points(fgGeo, makeMat(uTimeFg));
  fg.frustumCulled = false;

  return {
    bg,
    fg,
    update(timeSec: number) {
      uTimeBg.value = timeSec;
      uTimeFg.value = timeSec;
    },
  };
}

/**
 * Single bright additive sprite at the origin used as the "galactic core"
 * — gives the brain-graph swirl a luminous anchor point.
 */
export function buildCoreGlow(sprite: Texture): Points {
  const corePos = new Float32Array([0, 0, 0]);
  const coreCol = new Float32Array([1.0, 0.95, 0.85]);
  const coreSize = new Float32Array([260]);
  const coreEmph = new Float32Array([1.6]);
  const geo = new BufferGeometry();
  geo.setAttribute('position', new BufferAttribute(corePos, 3));
  geo.setAttribute('nodeColor', new BufferAttribute(coreCol, 3));
  geo.setAttribute('size', new BufferAttribute(coreSize, 1));
  geo.setAttribute('emphasis', new BufferAttribute(coreEmph, 1));
  const points = new Points(geo, makeNodeMaterial(sprite, GLOW_FS));
  points.frustumCulled = false;
  return points;
}

export interface RadialFilamentNode {
  x?: number | null;
  y?: number | null;
  z?: number | null;
  colour: string;
}

function safe(v: number | null | undefined): number {
  return (v != null && Number.isFinite(v)) ? v : 0;
}

/**
 * Additive line streamers from each node toward the galactic core. The line
 * stops ~18% from origin so it reads as a "streamer" rather than a hard spoke,
 * fading from bright at the node end to dim near the core.
 */
export function buildRadialFilaments(nodes: readonly RadialFilamentNode[]): LineSegments | null {
  if (nodes.length === 0) return null;
  const positions: number[] = [];
  const colors: number[] = [];
  const alphas: number[] = [];
  const tmp = { r: 0, g: 0, b: 0 };
  const parseHex = (hex: string) => {
    const h = hex.startsWith('#') ? hex.slice(1) : hex;
    const n = parseInt(h.length === 3 ? h.split('').map((c) => c + c).join('') : h, 16);
    tmp.r = ((n >> 16) & 0xff) / 255;
    tmp.g = ((n >> 8) & 0xff) / 255;
    tmp.b = (n & 0xff) / 255;
  };
  for (const node of nodes) {
    const x = safe(node.x), y = safe(node.y), z = safe(node.z);
    const d = Math.hypot(x, y, z);
    if (d < 60) continue;
    const t = 0.18;
    positions.push(x, y, z, x * t, y * t, z * t);
    parseHex(node.colour);
    colors.push(tmp.r, tmp.g, tmp.b, tmp.r, tmp.g, tmp.b);
    alphas.push(0.72, 0.04);
  }
  if (positions.length === 0) return null;
  const geometry = new BufferGeometry();
  geometry.setAttribute('position', new Float32BufferAttribute(positions, 3));
  geometry.setAttribute('color', new Float32BufferAttribute(colors, 3));
  geometry.setAttribute('alpha', new Float32BufferAttribute(alphas, 1));
  const material = new ShaderMaterial({
    vertexShader: `
      attribute float alpha;
      attribute vec3 color;
      varying vec3 vColor;
      varying float vAlpha;
      void main() {
        vColor = color;
        vAlpha = alpha;
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }
    `,
    fragmentShader: `
      varying vec3 vColor;
      varying float vAlpha;
      void main() { gl_FragColor = vec4(vColor, vAlpha); }
    `,
    transparent: true,
    depthWrite: false,
    blending: AdditiveBlending,
  });
  const lines = new LineSegments(geometry, material);
  lines.frustumCulled = false;
  return lines;
}

export interface EdgeLinkInput {
  src: { x?: number | null; y?: number | null; z?: number | null; colour: string };
  dst: { x?: number | null; y?: number | null; z?: number | null; colour: string };
  /** Alpha for both endpoints; usually a function of focus mode + gap status. */
  alpha: number;
}

/**
 * Additive line segments between connected graph nodes. Colours are sampled
 * from each endpoint and interpolated by the GPU; alpha is per-vertex so the
 * caller controls dimming for non-focused links.
 */
export function buildEdgeLines(edges: readonly EdgeLinkInput[]): LineSegments | null {
  if (edges.length === 0) return null;
  const positions: number[] = [];
  const colors: number[] = [];
  const alphas: number[] = [];
  const tmp = { r: 0, g: 0, b: 0 };
  const parseHex = (hex: string) => {
    const h = hex.startsWith('#') ? hex.slice(1) : hex;
    const n = parseInt(h.length === 3 ? h.split('').map((c) => c + c).join('') : h, 16);
    tmp.r = ((n >> 16) & 0xff) / 255;
    tmp.g = ((n >> 8) & 0xff) / 255;
    tmp.b = (n & 0xff) / 255;
  };
  for (const edge of edges) {
    positions.push(safe(edge.src.x), safe(edge.src.y), safe(edge.src.z));
    positions.push(safe(edge.dst.x), safe(edge.dst.y), safe(edge.dst.z));
    parseHex(edge.src.colour);
    colors.push(tmp.r, tmp.g, tmp.b);
    parseHex(edge.dst.colour);
    colors.push(tmp.r, tmp.g, tmp.b);
    alphas.push(edge.alpha, edge.alpha);
  }
  const geometry = new BufferGeometry();
  geometry.setAttribute('position', new Float32BufferAttribute(positions, 3));
  geometry.setAttribute('color', new Float32BufferAttribute(colors, 3));
  geometry.setAttribute('alpha', new Float32BufferAttribute(alphas, 1));
  const material = new ShaderMaterial({
    vertexShader: `
      attribute float alpha;
      attribute vec3 color;
      varying vec3 vColor;
      varying float vAlpha;
      void main() {
        vColor = color;
        vAlpha = alpha;
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }
    `,
    fragmentShader: `
      varying vec3 vColor;
      varying float vAlpha;
      void main() { gl_FragColor = vec4(vColor, vAlpha * 0.6); }
    `,
    transparent: true,
    depthWrite: false,
    blending: AdditiveBlending,
  });
  const lines = new LineSegments(geometry, material);
  lines.frustumCulled = false;
  return lines;
}
