/**
 * Vertex + fragment shaders for the animated theme-aware background.
 *
 * One shared fragment shader renders six different visual personalities,
 * selected at runtime by the `uMode` int uniform.  Palette is supplied
 * by three colour uniforms (uC1/uC2/uC3) that are read from the theme's
 * `--ts-bg-c1/c2/c3` CSS tokens (see `palette.ts`).  Movement intensity
 * and animation speed are themed via `uIntensity` and `uSpeed`.
 *
 * Algorithms used (all classic public-domain GLSL recipes — see Inigo
 * Quilez's noise tutorials at iquilezles.org for the source material):
 *
 *   • value-noise + 5-octave fbm
 *   • domain warping (Quilez 2002) for fluid colour blobs
 *   • ridged fbm for neural-trace mode
 *
 * The shader is designed to render at 30 FPS (gated by `scene.ts`) and
 * to never exceed ~3 ms GPU time on integrated graphics at 1080p.
 */

export const VERTEX_SHADER = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`;

export const FRAGMENT_SHADER = /* glsl */ `
  precision highp float;

  varying vec2 vUv;

  uniform float uTime;
  uniform vec2  uResolution;
  uniform vec3  uC1;
  uniform vec3  uC2;
  uniform vec3  uC3;
  uniform vec3  uAccent;
  uniform float uIntensity;
  uniform float uSpeed;
  uniform int   uMode;

  // ── Hash & noise primitives ────────────────────────────────────────
  // Classic GLSL value-hash (Quilez / ShaderToy idiom). The constants
  // 123.34 / 456.21 / 45.32 are arbitrary irrationals chosen for good
  // spatial decorrelation — any pair of large non-rational floats works.
  float hash21(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
  }

  float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    float a = hash21(i);
    float b = hash21(i + vec2(1.0, 0.0));
    float c = hash21(i + vec2(0.0, 1.0));
    float d = hash21(i + vec2(1.0, 1.0));
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
  }

  float fbm(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 5; i++) {
      v += a * vnoise(p);
      p *= 2.02;
      a *= 0.5;
    }
    return v;
  }

  // Ridged fbm — produces vein/trace-like patterns for neural mode.
  float ridged(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 5; i++) {
      float n = 1.0 - abs(vnoise(p) * 2.0 - 1.0);
      v += a * n * n;
      p *= 2.05;
      a *= 0.5;
    }
    return v;
  }

  // Domain-warped fbm — Inigo Quilez's classic recipe for fluid blobs.
  float warpedFbm(vec2 p, float t) {
    vec2 q = vec2(fbm(p + vec2(0.0, 0.0)),
                  fbm(p + vec2(5.2, 1.3)));
    vec2 r = vec2(fbm(p + 4.0 * q + vec2(1.7, 9.2) + 0.15 * t),
                  fbm(p + 4.0 * q + vec2(8.3, 2.8) + 0.13 * t));
    return fbm(p + 4.0 * r);
  }

  // ── Mode renderers — each returns a vec3 colour ────────────────────

  // 0: Nebula — slow drifting cloud blobs, the "Adventurer / Midnight" look.
  vec3 mNebula(vec2 uv, float t) {
    vec2 p = uv * 2.5;
    float n = warpedFbm(p, t * 0.25);
    vec3 col = mix(uC1, uC2, smoothstep(0.2, 0.8, n));
    col = mix(col, uC3, smoothstep(0.55, 0.95, n * n));
    // sparse parallax stars
    vec2 sp = uv * uResolution / 3.0;
    float star = step(0.997, hash21(floor(sp)));
    float twinkle = 0.5 + 0.5 * sin(t * 2.0 + hash21(floor(sp)) * 6.28);
    col += uAccent * star * twinkle * 0.6;
    return col;
  }

  // 1: Beams — slow vertical light shafts, the Linear / Stripe enterprise look.
  vec3 mBeams(vec2 uv, float t) {
    vec3 col = mix(uC1, uC2, uv.y);
    // moving vertical beams of subtle brightness
    float beams = 0.0;
    for (int i = 0; i < 3; i++) {
      float fi = float(i);
      float x  = fract((uv.x + t * 0.02 * (1.0 + fi * 0.3)) * (0.6 + fi * 0.2)) - 0.5;
      float w  = 0.04 + 0.02 * fi;
      beams += smoothstep(w, 0.0, abs(x)) * (0.18 - fi * 0.04);
    }
    // slow horizontal cloud wash
    float cloud = fbm(vec2(uv.x * 1.4 + t * 0.03, uv.y * 0.6));
    col += uAccent * (beams + cloud * 0.06);
    col = mix(col, uC3, smoothstep(0.0, 1.0, uv.y) * 0.25);
    return col;
  }

  // 2: Aurora — horizontal flowing ribbons across the upper half.
  vec3 mAurora(vec2 uv, float t) {
    vec3 col = mix(uC1, uC2, uv.y * 0.6 + 0.2);
    float a = 0.0;
    for (int i = 0; i < 3; i++) {
      float fi = float(i);
      float y0 = 0.65 - fi * 0.18;
      float wave = fbm(vec2(uv.x * 2.0 + t * (0.08 + fi * 0.04), fi * 13.0)) * 0.18;
      float band = smoothstep(0.07 + fi * 0.02, 0.0,
                              abs(uv.y - y0 - wave));
      a += band * (0.55 - fi * 0.12);
    }
    col += uAccent * a;
    col += uC3 * a * 0.5;
    return col;
  }

  // 3: Mist — very low-contrast drifting haze (sakura, pastel).
  vec3 mMist(vec2 uv, float t) {
    vec2 p = uv * 1.6;
    float n = warpedFbm(p, t * 0.18);
    vec3 col = mix(uC1, uC2, n);
    col = mix(col, uC3, smoothstep(0.45, 0.85, n) * 0.6);
    // soft floating motes
    vec2 mp = uv * uResolution / 6.0;
    float mote = step(0.992, hash21(floor(mp)));
    col += uAccent * mote * 0.25;
    return col;
  }

  // 4: Warm — warmer colour palette, slightly faster motion (cat, kids).
  vec3 mWarm(vec2 uv, float t) {
    vec2 p = uv * 2.2;
    float n = warpedFbm(p, t * 0.32);
    vec3 col = mix(uC1, uC2, smoothstep(0.15, 0.85, n));
    float glow = pow(smoothstep(0.55, 1.0, n), 1.5);
    col = mix(col, uC3, glow * 0.6);
    col += uAccent * glow * 0.35;
    return col;
  }

  // 5: Neural — ridged fbm + soft "synapse" pulses, for the Brain theme.
  vec3 mNeural(vec2 uv, float t) {
    vec2 p = uv * 3.5;
    p += 0.4 * vec2(fbm(p + t * 0.12), fbm(p - t * 0.10));
    float n = ridged(p);
    vec3 col = mix(uC1, uC2, smoothstep(0.05, 0.5, n));
    float trace = smoothstep(0.55, 0.85, n);
    col = mix(col, uC3, trace * 0.6);
    col += uAccent * trace * 0.5;
    // pulsing nodes
    vec2 sp = uv * 8.0;
    vec2 sid = floor(sp);
    float h = hash21(sid);
    float d = length(fract(sp) - 0.5);
    float pulse = smoothstep(0.4, 0.0, d) *
                  step(0.985, h) *
                  (0.5 + 0.5 * sin(t * 2.5 + h * 6.28));
    col += uAccent * pulse;
    return col;
  }

  void main() {
    // Aspect-correct UVs centred at 0,0 for radial maths; a separate
    // 0..1 uv is kept for vertical gradients.
    float aspect = uResolution.x / max(uResolution.y, 1.0);
    vec2 uv  = vUv;
    vec2 cuv = uv * 2.0 - 1.0;
    cuv.x *= aspect;

    float t = uTime * uSpeed;

    vec3 col;
    if      (uMode == 1) col = mBeams (uv,  t);
    else if (uMode == 2) col = mAurora(uv,  t);
    else if (uMode == 3) col = mMist  (uv,  t);
    else if (uMode == 4) col = mWarm  (uv,  t);
    else if (uMode == 5) col = mNeural(uv,  t);
    else                 col = mNebula(uv,  t);

    // Soft vignette — subtle edge darkening, intensity-aware.
    float v = 1.0 - smoothstep(0.55, 1.4, length(cuv));
    col *= mix(1.0, 0.55 + 0.45 * v, uIntensity);

    gl_FragColor = vec4(col, 1.0);
  }
`;
