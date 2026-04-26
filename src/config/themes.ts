/**
 * Built-in UI themes for TerranSoul.
 *
 * Each theme overrides the `--ts-*` CSS custom properties defined in style.css.
 * Only tokens that differ from the default RPG theme need to be specified —
 * the applier falls back to the :root values for any missing token.
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
  /** CSS custom-property overrides (key includes the `--ts-` prefix). */
  tokens: Record<string, string>;
  /** Optional: font family override. */
  fontFamily?: string;
  /** Optional: monospace font override. */
  fontMono?: string;
}

// ── Default / RPG Theme ───────────────────────────────────────────────────────
// The :root values in style.css serve as the RPG theme baseline.
// This entry carries an empty tokens map so the applier clears all overrides.

export const THEME_DEFAULT: ThemeDefinition = {
  id: 'default',
  label: 'Adventurer',
  description: 'The original dark RPG aesthetic — deep navy, violet accents, quest-style UI.',
  icon: '⚔️',
  category: 'dark',
  tokens: {},
};

// ── Corporate Theme ───────────────────────────────────────────────────────────

export const THEME_CORPORATE: ThemeDefinition = {
  id: 'corporate',
  label: 'Corporate',
  description: 'Clean, professional light theme — neutral grays, blue accents, minimal glow.',
  icon: '💼',
  category: 'light',
  fontFamily: "'Inter', 'Segoe UI', system-ui, sans-serif",
  tokens: {
    '--ts-bg-base': '#f0f2f5',
    '--ts-bg-surface': '#ffffff',
    '--ts-bg-elevated': '#f8f9fb',
    '--ts-bg-nav': '#e8ecf1',
    '--ts-bg-overlay': 'rgba(240, 242, 245, 0.96)',
    '--ts-bg-input': 'rgba(0, 0, 0, 0.04)',
    '--ts-bg-hover': 'rgba(0, 0, 0, 0.06)',
    '--ts-bg-card': 'rgba(255, 255, 255, 0.96)',
    '--ts-bg-panel': 'rgba(248, 249, 251, 0.98)',
    '--ts-bg-selected': 'rgba(37, 99, 235, 0.08)',

    '--ts-accent': '#2563eb',
    '--ts-accent-hover': '#1d4ed8',
    '--ts-accent-glow': 'rgba(37, 99, 235, 0.12)',
    '--ts-accent-blue': '#3b82f6',
    '--ts-accent-blue-hover': '#2563eb',
    '--ts-accent-violet': '#6366f1',
    '--ts-accent-violet-hover': '#4f46e5',

    '--ts-success': '#16a34a',
    '--ts-success-dim': '#15803d',
    '--ts-success-bg': 'rgba(22, 163, 74, 0.08)',
    '--ts-warning': '#d97706',
    '--ts-warning-bg': 'rgba(217, 119, 6, 0.08)',
    '--ts-warning-text': '#92400e',
    '--ts-error': '#dc2626',
    '--ts-error-bg': 'rgba(220, 38, 38, 0.08)',

    '--ts-text-primary': '#1e293b',
    '--ts-text-secondary': '#475569',
    '--ts-text-muted': '#94a3b8',
    '--ts-text-dim': 'rgba(0, 0, 0, 0.35)',
    '--ts-text-on-accent': '#ffffff',

    '--ts-border': 'rgba(0, 0, 0, 0.10)',
    '--ts-border-subtle': 'rgba(0, 0, 0, 0.05)',
    '--ts-border-medium': '#cbd5e1',
    '--ts-border-focus': '#2563eb',

    '--ts-info': '#0369a1',
    '--ts-info-bg': 'rgba(3, 105, 161, 0.08)',

    '--ts-quest-gold': '#b8860b',
    '--ts-quest-gold-bright': '#d4a017',
    '--ts-quest-gold-dim': 'rgba(184, 134, 11, 0.15)',
    '--ts-quest-gold-glow': 'rgba(184, 134, 11, 0.35)',
    '--ts-quest-bg': 'linear-gradient(135deg, rgba(248, 249, 251, 0.95), rgba(241, 245, 249, 0.98))',
    '--ts-quest-border': 'rgba(184, 134, 11, 0.25)',
    '--ts-quest-text': '#44403c',
    '--ts-quest-muted': '#78716c',

    '--ts-text-bright': '#0f172a',
    '--ts-text-link': '#2563eb',
    '--ts-border-strong': 'rgba(0, 0, 0, 0.14)',
    '--ts-bg-backdrop': 'rgba(0, 0, 0, 0.4)',

    '--ts-shadow-sm': '0 1px 3px rgba(0, 0, 0, 0.08)',
    '--ts-shadow-md': '0 4px 12px rgba(0, 0, 0, 0.10)',
    '--ts-shadow-lg': '0 12px 40px rgba(0, 0, 0, 0.15)',

    '--ts-radius-sm': '4px',
    '--ts-radius-md': '8px',
    '--ts-radius-lg': '12px',
    '--ts-radius-xl': '16px',
  },
};

// ── Cat / Neko Theme ──────────────────────────────────────────────────────────

export const THEME_CAT: ThemeDefinition = {
  id: 'cat',
  label: 'Neko',
  description: 'Cozy cat café vibes — warm browns, amber accents, paw-pad softness.',
  icon: '🐱',
  category: 'dark',
  tokens: {
    '--ts-bg-base': '#1a1410',
    '--ts-bg-surface': '#2a2218',
    '--ts-bg-elevated': '#362c20',
    '--ts-bg-nav': '#140f0a',
    '--ts-bg-overlay': 'rgba(20, 15, 10, 0.94)',
    '--ts-bg-input': 'rgba(255, 220, 170, 0.08)',
    '--ts-bg-hover': 'rgba(255, 200, 120, 0.12)',
    '--ts-bg-card': 'rgba(42, 34, 24, 0.94)',
    '--ts-bg-panel': 'rgba(26, 20, 16, 0.96)',
    '--ts-bg-selected': 'rgba(245, 158, 11, 0.12)',

    '--ts-accent': '#f59e0b',
    '--ts-accent-hover': '#d97706',
    '--ts-accent-glow': 'rgba(245, 158, 11, 0.20)',
    '--ts-accent-blue': '#fbbf24',
    '--ts-accent-blue-hover': '#f59e0b',
    '--ts-accent-violet': '#e8a87c',
    '--ts-accent-violet-hover': '#d4956a',

    '--ts-success': '#86efac',
    '--ts-success-dim': '#4ade80',
    '--ts-warning': '#fcd34d',
    '--ts-warning-text': '#fcd34d',
    '--ts-error': '#fca5a5',

    '--ts-text-primary': '#fef3e2',
    '--ts-text-secondary': '#c8b89a',
    '--ts-text-muted': '#8d7b66',
    '--ts-text-dim': 'rgba(255, 243, 226, 0.40)',

    '--ts-border': 'rgba(255, 200, 120, 0.12)',
    '--ts-border-subtle': 'rgba(255, 200, 120, 0.06)',
    '--ts-border-medium': '#4a3c2e',
    '--ts-border-focus': '#f59e0b',

    '--ts-shadow-sm': '0 1px 3px rgba(0, 0, 0, 0.35)',
    '--ts-shadow-md': '0 4px 16px rgba(0, 0, 0, 0.40)',
    '--ts-shadow-lg': '0 12px 48px rgba(0, 0, 0, 0.55)',

    '--ts-info': '#fbbf24',
    '--ts-info-bg': 'rgba(251, 191, 36, 0.10)',

    '--ts-quest-gold': '#f59e0b',
    '--ts-quest-gold-bright': '#fbbf24',
    '--ts-quest-gold-dim': 'rgba(245, 158, 11, 0.20)',
    '--ts-quest-gold-glow': 'rgba(245, 158, 11, 0.45)',
    '--ts-quest-bg': 'linear-gradient(135deg, rgba(42, 34, 24, 0.90), rgba(26, 20, 16, 0.95))',
    '--ts-quest-border': 'rgba(245, 158, 11, 0.25)',
    '--ts-quest-text': '#fef3e2',
    '--ts-quest-muted': 'rgba(200, 184, 154, 0.55)',

    '--ts-text-bright': '#fef3e2',
    '--ts-text-link': '#fbbf24',
    '--ts-border-strong': 'rgba(255, 200, 120, 0.18)',
    '--ts-bg-backdrop': 'rgba(0, 0, 0, 0.65)',

    '--ts-radius-sm': '8px',
    '--ts-radius-md': '14px',
    '--ts-radius-lg': '18px',
    '--ts-radius-xl': '24px',
  },
};

// ── Girl / Sakura Theme ───────────────────────────────────────────────────────

export const THEME_SAKURA: ThemeDefinition = {
  id: 'sakura',
  label: 'Sakura',
  description: 'Soft pink & rose aesthetic — pastel blossoms, gentle gradients, dreamy glow.',
  icon: '🌸',
  category: 'colorful',
  tokens: {
    '--ts-bg-base': '#1e1520',
    '--ts-bg-surface': '#2a1f2e',
    '--ts-bg-elevated': '#362a3a',
    '--ts-bg-nav': '#160f18',
    '--ts-bg-overlay': 'rgba(22, 15, 24, 0.94)',
    '--ts-bg-input': 'rgba(255, 182, 193, 0.08)',
    '--ts-bg-hover': 'rgba(255, 182, 193, 0.12)',
    '--ts-bg-card': 'rgba(42, 31, 46, 0.94)',
    '--ts-bg-panel': 'rgba(30, 21, 32, 0.96)',
    '--ts-bg-selected': 'rgba(244, 114, 182, 0.12)',

    '--ts-accent': '#f472b6',
    '--ts-accent-hover': '#ec4899',
    '--ts-accent-glow': 'rgba(244, 114, 182, 0.22)',
    '--ts-accent-blue': '#f9a8d4',
    '--ts-accent-blue-hover': '#f472b6',
    '--ts-accent-violet': '#e879f9',
    '--ts-accent-violet-hover': '#d946ef',

    '--ts-success': '#86efac',
    '--ts-success-dim': '#4ade80',
    '--ts-warning': '#fde68a',
    '--ts-warning-text': '#fde68a',
    '--ts-error': '#fda4af',

    '--ts-text-primary': '#fdf2f8',
    '--ts-text-secondary': '#d4a0b9',
    '--ts-text-muted': '#9a7189',
    '--ts-text-dim': 'rgba(253, 242, 248, 0.40)',

    '--ts-border': 'rgba(244, 114, 182, 0.14)',
    '--ts-border-subtle': 'rgba(244, 114, 182, 0.06)',
    '--ts-border-medium': '#4a3650',
    '--ts-border-focus': '#f472b6',

    '--ts-shadow-sm': '0 1px 4px rgba(236, 72, 153, 0.15)',
    '--ts-shadow-md': '0 4px 16px rgba(236, 72, 153, 0.20)',
    '--ts-shadow-lg': '0 12px 48px rgba(236, 72, 153, 0.25)',

    '--ts-info': '#67e8f9',
    '--ts-info-bg': 'rgba(103, 232, 249, 0.10)',

    '--ts-quest-gold': '#f9a8d4',
    '--ts-quest-gold-bright': '#f472b6',
    '--ts-quest-gold-dim': 'rgba(244, 114, 182, 0.20)',
    '--ts-quest-gold-glow': 'rgba(244, 114, 182, 0.45)',
    '--ts-quest-bg': 'linear-gradient(135deg, rgba(42, 31, 46, 0.90), rgba(30, 21, 32, 0.95))',
    '--ts-quest-border': 'rgba(244, 114, 182, 0.25)',
    '--ts-quest-text': '#fdf2f8',
    '--ts-quest-muted': 'rgba(212, 160, 185, 0.55)',

    '--ts-text-bright': '#fdf2f8',
    '--ts-text-link': '#f9a8d4',
    '--ts-border-strong': 'rgba(244, 114, 182, 0.20)',
    '--ts-bg-backdrop': 'rgba(0, 0, 0, 0.65)',

    '--ts-radius-sm': '8px',
    '--ts-radius-md': '12px',
    '--ts-radius-lg': '16px',
    '--ts-radius-xl': '22px',
    '--ts-radius-pill': '999px',
  },
};

// ── Kids Theme ────────────────────────────────────────────────────────────────

export const THEME_KIDS: ThemeDefinition = {
  id: 'kids',
  label: 'Playground',
  description: 'Bright, fun, and friendly — bold primaries, large radii, big text, playful feel.',
  icon: '🎈',
  category: 'colorful',
  fontFamily: "'Nunito', 'Comic Neue', 'Inter', system-ui, sans-serif",
  tokens: {
    '--ts-bg-base': '#1a1638',
    '--ts-bg-surface': '#252050',
    '--ts-bg-elevated': '#302968',
    '--ts-bg-nav': '#120e2a',
    '--ts-bg-overlay': 'rgba(18, 14, 42, 0.94)',
    '--ts-bg-input': 'rgba(255, 255, 255, 0.10)',
    '--ts-bg-hover': 'rgba(255, 255, 255, 0.14)',
    '--ts-bg-card': 'rgba(37, 32, 80, 0.94)',
    '--ts-bg-panel': 'rgba(26, 22, 56, 0.96)',
    '--ts-bg-selected': 'rgba(99, 102, 241, 0.14)',

    '--ts-accent': '#818cf8',
    '--ts-accent-hover': '#6366f1',
    '--ts-accent-glow': 'rgba(129, 140, 248, 0.25)',
    '--ts-accent-blue': '#38bdf8',
    '--ts-accent-blue-hover': '#0ea5e9',
    '--ts-accent-violet': '#c084fc',
    '--ts-accent-violet-hover': '#a855f7',

    '--ts-success': '#4ade80',
    '--ts-success-dim': '#22c55e',
    '--ts-success-bg': 'rgba(74, 222, 128, 0.14)',
    '--ts-warning': '#fbbf24',
    '--ts-warning-text': '#fbbf24',
    '--ts-error': '#fb7185',
    '--ts-error-bg': 'rgba(251, 113, 133, 0.14)',

    '--ts-text-primary': '#f8fafc',
    '--ts-text-secondary': '#a5b4fc',
    '--ts-text-muted': '#7c82b8',
    '--ts-text-dim': 'rgba(255, 255, 255, 0.45)',

    '--ts-border': 'rgba(129, 140, 248, 0.16)',
    '--ts-border-subtle': 'rgba(129, 140, 248, 0.08)',
    '--ts-border-medium': '#3b3580',
    '--ts-border-focus': '#818cf8',

    '--ts-shadow-sm': '0 2px 6px rgba(99, 102, 241, 0.15)',
    '--ts-shadow-md': '0 6px 20px rgba(99, 102, 241, 0.20)',
    '--ts-shadow-lg': '0 14px 52px rgba(99, 102, 241, 0.28)',

    '--ts-radius-sm': '10px',
    '--ts-radius-md': '16px',
    '--ts-radius-lg': '22px',
    '--ts-radius-xl': '28px',
    '--ts-radius-pill': '999px',

    '--ts-info': '#38bdf8',
    '--ts-info-bg': 'rgba(56, 189, 248, 0.12)',

    '--ts-quest-gold': '#fbbf24',
    '--ts-quest-gold-bright': '#fde68a',
    '--ts-quest-gold-dim': 'rgba(251, 191, 36, 0.22)',
    '--ts-quest-gold-glow': 'rgba(251, 191, 36, 0.50)',
    '--ts-quest-bg': 'linear-gradient(135deg, rgba(37, 32, 80, 0.90), rgba(26, 22, 56, 0.95))',
    '--ts-quest-border': 'rgba(251, 191, 36, 0.25)',
    '--ts-quest-text': '#fef9c3',
    '--ts-quest-muted': 'rgba(165, 180, 252, 0.55)',

    '--ts-text-bright': '#f8fafc',
    '--ts-text-link': '#38bdf8',
    '--ts-border-strong': 'rgba(129, 140, 248, 0.22)',
    '--ts-bg-backdrop': 'rgba(0, 0, 0, 0.65)',

    '--ts-text-sm': '0.85rem',
    '--ts-text-base': '0.95rem',
    '--ts-text-lg': '1.2rem',
  },
};

// ── Brain / Neural Theme ──────────────────────────────────────────────────────

export const THEME_BRAIN: ThemeDefinition = {
  id: 'brain',
  label: 'Neural',
  description: 'Cyberpunk neural-net aesthetic — electric teal, matrix green, circuit-board precision.',
  icon: '🧠',
  category: 'dark',
  fontMono: "'JetBrains Mono', 'Fira Code', monospace",
  tokens: {
    '--ts-bg-base': '#0a0f14',
    '--ts-bg-surface': '#111a22',
    '--ts-bg-elevated': '#1a2633',
    '--ts-bg-nav': '#060a0e',
    '--ts-bg-overlay': 'rgba(6, 10, 14, 0.95)',
    '--ts-bg-input': 'rgba(0, 255, 200, 0.06)',
    '--ts-bg-hover': 'rgba(0, 255, 200, 0.10)',
    '--ts-bg-card': 'rgba(17, 26, 34, 0.94)',
    '--ts-bg-panel': 'rgba(10, 15, 20, 0.97)',
    '--ts-bg-selected': 'rgba(0, 230, 180, 0.10)',

    '--ts-accent': '#00e6b4',
    '--ts-accent-hover': '#00cc9e',
    '--ts-accent-glow': 'rgba(0, 230, 180, 0.24)',
    '--ts-accent-blue': '#22d3ee',
    '--ts-accent-blue-hover': '#06b6d4',
    '--ts-accent-violet': '#67e8f9',
    '--ts-accent-violet-hover': '#22d3ee',

    '--ts-success': '#00ff88',
    '--ts-success-dim': '#00e676',
    '--ts-success-bg': 'rgba(0, 255, 136, 0.10)',
    '--ts-warning': '#ffea00',
    '--ts-warning-bg': 'rgba(255, 234, 0, 0.10)',
    '--ts-warning-text': '#ffea00',
    '--ts-error': '#ff5252',
    '--ts-error-bg': 'rgba(255, 82, 82, 0.10)',

    '--ts-text-primary': '#d0f0e8',
    '--ts-text-secondary': '#6eb8a0',
    '--ts-text-muted': '#3d7a66',
    '--ts-text-dim': 'rgba(208, 240, 232, 0.40)',

    '--ts-border': 'rgba(0, 230, 180, 0.14)',
    '--ts-border-subtle': 'rgba(0, 230, 180, 0.06)',
    '--ts-border-medium': '#1a3830',
    '--ts-border-focus': '#00e6b4',

    '--ts-shadow-sm': '0 1px 3px rgba(0, 230, 180, 0.10)',
    '--ts-shadow-md': '0 4px 16px rgba(0, 230, 180, 0.14)',
    '--ts-shadow-lg': '0 12px 48px rgba(0, 230, 180, 0.18)',

    '--ts-info': '#22d3ee',
    '--ts-info-bg': 'rgba(34, 211, 238, 0.10)',

    '--ts-quest-gold': '#00e6b4',
    '--ts-quest-gold-bright': '#00ff88',
    '--ts-quest-gold-dim': 'rgba(0, 230, 180, 0.18)',
    '--ts-quest-gold-glow': 'rgba(0, 230, 180, 0.45)',
    '--ts-quest-bg': 'linear-gradient(135deg, rgba(17, 26, 34, 0.90), rgba(10, 15, 20, 0.95))',
    '--ts-quest-border': 'rgba(0, 230, 180, 0.22)',
    '--ts-quest-text': '#d0f0e8',
    '--ts-quest-muted': 'rgba(110, 184, 160, 0.55)',

    '--ts-text-bright': '#e0faf2',
    '--ts-text-link': '#22d3ee',
    '--ts-border-strong': 'rgba(0, 230, 180, 0.20)',
    '--ts-bg-backdrop': 'rgba(0, 0, 0, 0.70)',

    '--ts-radius-sm': '4px',
    '--ts-radius-md': '6px',
    '--ts-radius-lg': '10px',
    '--ts-radius-xl': '14px',
  },
};

// ── Midnight Theme ────────────────────────────────────────────────────────────

export const THEME_MIDNIGHT: ThemeDefinition = {
  id: 'midnight',
  label: 'Midnight',
  description: 'True OLED-dark — near-black surfaces, muted accents, ultra-minimal glow.',
  icon: '🌙',
  category: 'dark',
  tokens: {
    '--ts-bg-base': '#050505',
    '--ts-bg-surface': '#111111',
    '--ts-bg-elevated': '#1a1a1a',
    '--ts-bg-nav': '#000000',
    '--ts-bg-overlay': 'rgba(0, 0, 0, 0.96)',
    '--ts-bg-input': 'rgba(255, 255, 255, 0.05)',
    '--ts-bg-hover': 'rgba(255, 255, 255, 0.08)',
    '--ts-bg-card': 'rgba(17, 17, 17, 0.96)',
    '--ts-bg-panel': 'rgba(5, 5, 5, 0.98)',
    '--ts-bg-selected': 'rgba(147, 130, 255, 0.10)',

    '--ts-accent': '#9382ff',
    '--ts-accent-hover': '#7a68e6',
    '--ts-accent-glow': 'rgba(147, 130, 255, 0.16)',
    '--ts-accent-blue': '#7aa2f7',
    '--ts-accent-blue-hover': '#5c88e0',
    '--ts-accent-violet': '#bb9af7',
    '--ts-accent-violet-hover': '#a37ee0',

    '--ts-success': '#9ece6a',
    '--ts-success-dim': '#73daca',
    '--ts-warning': '#e0af68',
    '--ts-warning-text': '#e0af68',
    '--ts-error': '#f7768e',

    '--ts-text-primary': '#c0caf5',
    '--ts-text-secondary': '#7982a9',
    '--ts-text-muted': '#4a506b',
    '--ts-text-dim': 'rgba(192, 202, 245, 0.35)',

    '--ts-border': 'rgba(255, 255, 255, 0.06)',
    '--ts-border-subtle': 'rgba(255, 255, 255, 0.03)',
    '--ts-border-medium': '#222233',
    '--ts-border-focus': '#9382ff',

    '--ts-info': '#7aa2f7',
    '--ts-info-bg': 'rgba(122, 162, 247, 0.10)',

    '--ts-quest-gold': '#e0af68',
    '--ts-quest-gold-bright': '#ffcf80',
    '--ts-quest-gold-dim': 'rgba(224, 175, 104, 0.18)',
    '--ts-quest-gold-glow': 'rgba(224, 175, 104, 0.40)',
    '--ts-quest-bg': 'linear-gradient(135deg, rgba(17, 17, 17, 0.92), rgba(5, 5, 5, 0.96))',
    '--ts-quest-border': 'rgba(224, 175, 104, 0.22)',
    '--ts-quest-text': '#c0caf5',
    '--ts-quest-muted': 'rgba(121, 130, 169, 0.55)',

    '--ts-text-bright': '#d5daf8',
    '--ts-text-link': '#7aa2f7',
    '--ts-border-strong': 'rgba(255, 255, 255, 0.10)',
    '--ts-bg-backdrop': 'rgba(0, 0, 0, 0.80)',

    '--ts-shadow-sm': '0 1px 2px rgba(0, 0, 0, 0.6)',
    '--ts-shadow-md': '0 4px 12px rgba(0, 0, 0, 0.7)',
    '--ts-shadow-lg': '0 12px 40px rgba(0, 0, 0, 0.8)',
  },
};

// ── Aurora Theme ──────────────────────────────────────────────────────────────

export const THEME_AURORA: ThemeDefinition = {
  id: 'aurora',
  label: 'Aurora',
  description: 'Northern-lights inspired — deep teal base, shifting green-blue-purple accents.',
  icon: '🌌',
  category: 'dark',
  tokens: {
    '--ts-bg-base': '#0c1a1e',
    '--ts-bg-surface': '#142830',
    '--ts-bg-elevated': '#1c3440',
    '--ts-bg-nav': '#081216',
    '--ts-bg-overlay': 'rgba(8, 18, 22, 0.94)',
    '--ts-bg-input': 'rgba(110, 231, 183, 0.06)',
    '--ts-bg-hover': 'rgba(110, 231, 183, 0.10)',
    '--ts-bg-card': 'rgba(20, 40, 48, 0.94)',
    '--ts-bg-panel': 'rgba(12, 26, 30, 0.96)',
    '--ts-bg-selected': 'rgba(52, 211, 153, 0.12)',

    '--ts-accent': '#34d399',
    '--ts-accent-hover': '#10b981',
    '--ts-accent-glow': 'rgba(52, 211, 153, 0.22)',
    '--ts-accent-blue': '#22d3ee',
    '--ts-accent-blue-hover': '#06b6d4',
    '--ts-accent-violet': '#a78bfa',
    '--ts-accent-violet-hover': '#8b5cf6',

    '--ts-success': '#6ee7b7',
    '--ts-success-dim': '#34d399',
    '--ts-warning': '#fde68a',
    '--ts-warning-text': '#fde68a',
    '--ts-error': '#fca5a5',

    '--ts-text-primary': '#e0f2f1',
    '--ts-text-secondary': '#80cbc4',
    '--ts-text-muted': '#4d8a82',
    '--ts-text-dim': 'rgba(224, 242, 241, 0.40)',

    '--ts-border': 'rgba(52, 211, 153, 0.14)',
    '--ts-border-subtle': 'rgba(52, 211, 153, 0.06)',
    '--ts-border-medium': '#1e4038',
    '--ts-border-focus': '#34d399',

    '--ts-info': '#22d3ee',
    '--ts-info-bg': 'rgba(34, 211, 238, 0.10)',

    '--ts-quest-gold': '#fde68a',
    '--ts-quest-gold-bright': '#fbbf24',
    '--ts-quest-gold-dim': 'rgba(253, 230, 138, 0.20)',
    '--ts-quest-gold-glow': 'rgba(253, 230, 138, 0.45)',
    '--ts-quest-bg': 'linear-gradient(135deg, rgba(20, 40, 48, 0.90), rgba(12, 26, 30, 0.95))',
    '--ts-quest-border': 'rgba(253, 230, 138, 0.25)',
    '--ts-quest-text': '#e0f2f1',
    '--ts-quest-muted': 'rgba(128, 203, 196, 0.55)',

    '--ts-text-bright': '#ecfdf5',
    '--ts-text-link': '#6ee7b7',
    '--ts-border-strong': 'rgba(52, 211, 153, 0.20)',
    '--ts-bg-backdrop': 'rgba(0, 0, 0, 0.65)',

    '--ts-shadow-sm': '0 1px 4px rgba(16, 185, 129, 0.12)',
    '--ts-shadow-md': '0 4px 16px rgba(16, 185, 129, 0.16)',
    '--ts-shadow-lg': '0 12px 48px rgba(16, 185, 129, 0.22)',
  },
};

// ── Pastel Light Theme ────────────────────────────────────────────────────────

export const THEME_PASTEL: ThemeDefinition = {
  id: 'pastel',
  label: 'Pastel',
  description: 'Soft cream & lavender light theme — gentle, calming, easy on the eyes.',
  icon: '☁️',
  category: 'light',
  tokens: {
    '--ts-bg-base': '#faf8f5',
    '--ts-bg-surface': '#ffffff',
    '--ts-bg-elevated': '#f5f0fa',
    '--ts-bg-nav': '#f0ebe5',
    '--ts-bg-overlay': 'rgba(250, 248, 245, 0.96)',
    '--ts-bg-input': 'rgba(0, 0, 0, 0.03)',
    '--ts-bg-hover': 'rgba(0, 0, 0, 0.05)',
    '--ts-bg-card': 'rgba(255, 255, 255, 0.98)',
    '--ts-bg-panel': 'rgba(250, 248, 245, 0.98)',
    '--ts-bg-selected': 'rgba(168, 139, 250, 0.10)',

    '--ts-accent': '#a78bfa',
    '--ts-accent-hover': '#8b5cf6',
    '--ts-accent-glow': 'rgba(168, 139, 250, 0.15)',
    '--ts-accent-blue': '#93c5fd',
    '--ts-accent-blue-hover': '#60a5fa',
    '--ts-accent-violet': '#c4b5fd',
    '--ts-accent-violet-hover': '#a78bfa',

    '--ts-success': '#34d399',
    '--ts-success-dim': '#10b981',
    '--ts-success-bg': 'rgba(52, 211, 153, 0.10)',
    '--ts-warning': '#f59e0b',
    '--ts-warning-bg': 'rgba(245, 158, 11, 0.10)',
    '--ts-warning-text': '#92400e',
    '--ts-error': '#f87171',
    '--ts-error-bg': 'rgba(248, 113, 113, 0.10)',

    '--ts-text-primary': '#3f3f46',
    '--ts-text-secondary': '#71717a',
    '--ts-text-muted': '#a1a1aa',
    '--ts-text-dim': 'rgba(0, 0, 0, 0.30)',
    '--ts-text-on-accent': '#ffffff',

    '--ts-border': 'rgba(0, 0, 0, 0.08)',
    '--ts-border-subtle': 'rgba(0, 0, 0, 0.04)',
    '--ts-border-medium': '#e4e4e7',
    '--ts-border-focus': '#a78bfa',

    '--ts-shadow-sm': '0 1px 3px rgba(0, 0, 0, 0.06)',
    '--ts-shadow-md': '0 4px 12px rgba(0, 0, 0, 0.08)',
    '--ts-shadow-lg': '0 12px 40px rgba(0, 0, 0, 0.12)',

    '--ts-info': '#6366f1',
    '--ts-info-bg': 'rgba(99, 102, 241, 0.08)',

    '--ts-quest-gold': '#a78bfa',
    '--ts-quest-gold-bright': '#c4b5fd',
    '--ts-quest-gold-dim': 'rgba(168, 139, 250, 0.15)',
    '--ts-quest-gold-glow': 'rgba(168, 139, 250, 0.35)',
    '--ts-quest-bg': 'linear-gradient(135deg, rgba(255, 255, 255, 0.96), rgba(245, 240, 250, 0.98))',
    '--ts-quest-border': 'rgba(168, 139, 250, 0.25)',
    '--ts-quest-text': '#3f3f46',
    '--ts-quest-muted': '#71717a',

    '--ts-text-bright': '#27272a',
    '--ts-text-link': '#7c3aed',
    '--ts-border-strong': 'rgba(0, 0, 0, 0.12)',
    '--ts-bg-backdrop': 'rgba(0, 0, 0, 0.35)',

    '--ts-radius-sm': '8px',
    '--ts-radius-md': '12px',
    '--ts-radius-lg': '16px',
    '--ts-radius-xl': '22px',
  },
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
