/**
 * Per-theme preset table for the animated background.
 *
 * Each registered theme id maps to:
 *   • `mode`      — which fragment-shader visual personality to render
 *                   (see `shader.glsl.ts` for the integer→algorithm table);
 *   • `speed`     — animation time multiplier (0 = static, 1 = baseline);
 *   • `intensity` — vignette + colour intensity scaler.
 *
 * Adding a new theme: register it here AND add `--ts-bg-c1/c2/c3/accent`
 * tokens to its `html[data-theme="*"]` block in `src/style.css`.  The
 * preset table is the single source of truth — `getPreset` returns the
 * default preset for unknown themes so the scene degrades gracefully.
 */

export type BackgroundMode =
  | 'nebula'
  | 'beams'
  | 'aurora'
  | 'mist'
  | 'warm'
  | 'neural';

export interface BackgroundPreset {
  /** Fragment-shader personality. */
  readonly mode: BackgroundMode;
  /** Animation speed multiplier (0..2). */
  readonly speed: number;
  /** Vignette / contrast intensity (0..1). */
  readonly intensity: number;
}

/** Mode-name → uMode integer (must match the if-chain in `shader.glsl.ts`). */
export const MODE_INDEX: Readonly<Record<BackgroundMode, number>> = {
  nebula: 0,
  beams: 1,
  aurora: 2,
  mist: 3,
  warm: 4,
  neural: 5,
};

const DEFAULT_PRESET: BackgroundPreset = {
  mode: 'nebula',
  speed: 0.6,
  intensity: 0.85,
};

/**
 * Theme-id → preset.  Only themes registered in `BUILTIN_THEMES` should
 * appear here.  Keep entries alphabetical on the theme id so audits are
 * trivial.
 */
const PRESETS: Readonly<Record<string, BackgroundPreset>> = {
  // Aurora — wide flowing ribbons.
  aurora:           { mode: 'aurora', speed: 0.55, intensity: 0.75 },
  // Brain — flowing neural traces with synapse pulses.
  brain:            { mode: 'neural', speed: 0.50, intensity: 0.90 },
  // Cat — warm candlelight blobs.
  cat:              { mode: 'warm',   speed: 0.65, intensity: 0.85 },
  // Corporate (light) — restrained vertical light beams over white.
  corporate:        { mode: 'beams',  speed: 0.30, intensity: 0.35 },
  // Corporate Dark — slow VS Code blue beams.
  'corporate-dark': { mode: 'beams',  speed: 0.30, intensity: 0.55 },
  // Adventurer (default) — drifting cosmic nebula + sparse stars.
  default:          { mode: 'nebula', speed: 0.60, intensity: 0.85 },
  // Kids — slightly faster warm play.
  kids:             { mode: 'warm',   speed: 0.85, intensity: 0.85 },
  // Midnight — minimal dark nebula.
  midnight:         { mode: 'nebula', speed: 0.45, intensity: 0.95 },
  // Pastel — soft drifting mist (light theme).
  pastel:           { mode: 'mist',   speed: 0.40, intensity: 0.30 },
  // Sakura — soft pink mist (no cartoon petals).
  sakura:           { mode: 'mist',   speed: 0.55, intensity: 0.75 },
};

/** Return the preset for a theme id, or the default preset if unknown. */
export function getPreset(themeId: string | null | undefined): BackgroundPreset {
  if (!themeId) return DEFAULT_PRESET;
  return PRESETS[themeId] ?? DEFAULT_PRESET;
}

/** Read-only list of theme ids that have a registered preset. */
export function knownThemeIds(): readonly string[] {
  return Object.keys(PRESETS);
}
