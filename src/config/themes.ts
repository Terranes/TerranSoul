/**
 * Built-in UI themes for TerranSoul.
 *
 * Each theme entry contains only presentation metadata.  All CSS token values
 * live in the html[data-theme="*"] override blocks in src/style.css — that is
 * the single source of truth for theme colours.  useTheme.ts sets the
 * data-theme attribute on <html> and the cascade handles the rest.
 *
 * The `preview` field provides the three swatch colours shown in ThemePicker.
 */

// ── Types ─────────────────────────────────────────────────────────────────────

export interface ThemeDefinition {
  /** Unique machine-readable ID (kebab-case). */
  id: string;
  /** Human-readable display name. */
  label: string;
  /** Short description shown in the theme picker. */
  description: string;
  /** Emoji or icon shown in the picker. */
  icon: string;
  /** Category for grouping in the picker. */
  category: 'dark' | 'light' | 'colorful';
  /**
   * Three swatch colours for the ThemePicker preview dots.
   * Must match the html[data-theme="*"] values in style.css.
   */
  preview: { bg: string; accent: string; text: string };
}

// ── Theme Definitions ─────────────────────────────────────────────────────────

export const THEME_DEFAULT: ThemeDefinition = {
  id: 'default',
  label: 'Adventurer',
  description: 'The original dark RPG aesthetic — deep navy, violet accents, quest-style UI.',
  icon: '⚔️',
  category: 'dark',
  preview: { bg: '#0f172a', accent: '#7c6fff', text: '#f1f5f9' },
};

export const THEME_CORPORATE: ThemeDefinition = {
  id: 'corporate',
  label: 'Corporate',
  description: 'Teams/Slack-inspired professional light theme — clean whites, indigo accent, subtle elevation.',
  icon: '💼',
  category: 'light',
  preview: { bg: '#f5f5f5', accent: '#5b5fc7', text: '#242424' },
};

export const THEME_CAT: ThemeDefinition = {
  id: 'cat',
  label: 'Neko',
  description: 'Cozy cat café vibes — warm browns, amber accents, paw-pad softness.',
  icon: '🐱',
  category: 'dark',
  preview: { bg: '#1a1410', accent: '#f59e0b', text: '#fef3e2' },
};

export const THEME_SAKURA: ThemeDefinition = {
  id: 'sakura',
  label: 'Sakura',
  description: 'Soft pink & rose aesthetic — pastel blossoms, gentle gradients, dreamy glow.',
  icon: '🌸',
  category: 'colorful',
  preview: { bg: '#1e1520', accent: '#f472b6', text: '#fdf2f8' },
};

export const THEME_KIDS: ThemeDefinition = {
  id: 'kids',
  label: 'Playground',
  description: 'Bright, fun, and friendly — bold primaries, large radii, big text, playful feel.',
  icon: '🎈',
  category: 'colorful',
  preview: { bg: '#1a1638', accent: '#818cf8', text: '#f8fafc' },
};

export const THEME_BRAIN: ThemeDefinition = {
  id: 'brain',
  label: 'Neural',
  description: 'Cyberpunk neural-net aesthetic — electric teal, matrix green, circuit-board precision.',
  icon: '🧠',
  category: 'dark',
  preview: { bg: '#0a0f14', accent: '#00e6b4', text: '#d0f0e8' },
};

export const THEME_MIDNIGHT: ThemeDefinition = {
  id: 'midnight',
  label: 'Midnight',
  description: 'True OLED-dark — near-black surfaces, muted accents, ultra-minimal glow.',
  icon: '🌙',
  category: 'dark',
  preview: { bg: '#050505', accent: '#9382ff', text: '#c0caf5' },
};

export const THEME_AURORA: ThemeDefinition = {
  id: 'aurora',
  label: 'Aurora',
  description: 'Northern-lights inspired — deep teal base, shifting green-blue-purple accents.',
  icon: '🌌',
  category: 'dark',
  preview: { bg: '#0c1a1e', accent: '#34d399', text: '#e0f2f1' },
};

export const THEME_PASTEL: ThemeDefinition = {
  id: 'pastel',
  label: 'Pastel',
  description: 'Soft cream & lavender light theme — gentle, calming, easy on the eyes.',
  icon: '☁️',
  category: 'light',
  preview: { bg: '#faf8f5', accent: '#a78bfa', text: '#3f3f46' },
};

// ── Registry ──────────────────────────────────────────────────────────────────

/** All built-in themes, ordered by display priority. */
export const BUILTIN_THEMES: readonly ThemeDefinition[] = [
  THEME_DEFAULT,
  THEME_CORPORATE,
  THEME_MIDNIGHT,
  THEME_AURORA,
  THEME_BRAIN,
  THEME_SAKURA,
  THEME_CAT,
  THEME_KIDS,
  THEME_PASTEL,
] as const;

/** Quick lookup by ID. */
export const THEME_MAP: ReadonlyMap<string, ThemeDefinition> = new Map(
  BUILTIN_THEMES.map((t) => [t.id, t]),
);

/** The default theme ID applied on first launch. */
export const DEFAULT_THEME_ID = 'default';
