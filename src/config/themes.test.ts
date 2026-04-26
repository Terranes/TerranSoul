import { describe, it, expect } from 'vitest';
import {
  BUILTIN_THEMES,
  DEFAULT_THEME_ID,
  THEME_MAP,
  THEME_DEFAULT,
  THEME_CORPORATE,
  THEME_CAT,
  THEME_SAKURA,
  THEME_KIDS,
  THEME_BRAIN,
  THEME_MIDNIGHT,
  THEME_AURORA,
  THEME_PASTEL,
  type ThemeDefinition,
} from './themes';

// ── Inline WCAG relative-luminance helpers ────────────────────────────────────

function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace('#', '');
  return [
    parseInt(h.slice(0, 2), 16) / 255,
    parseInt(h.slice(2, 4), 16) / 255,
    parseInt(h.slice(4, 6), 16) / 255,
  ];
}

function linearise(c: number): number {
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

function relativeLuminance(hex: string): number {
  const [r, g, b] = hexToRgb(hex).map(linearise);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrastRatio(hex1: string, hex2: string): number {
  const L1 = relativeLuminance(hex1);
  const L2 = relativeLuminance(hex2);
  const lighter = Math.max(L1, L2);
  const darker  = Math.min(L1, L2);
  return (lighter + 0.05) / (darker + 0.05);
}

// ── Registry / structural tests ───────────────────────────────────────────────

describe('theme definitions', () => {
  it('has at least 9 built-in themes', () => {
    expect(BUILTIN_THEMES.length).toBeGreaterThanOrEqual(9);
  });

  it('default theme ID is "default"', () => {
    expect(DEFAULT_THEME_ID).toBe('default');
  });

  it('every theme has a unique ID', () => {
    const ids = BUILTIN_THEMES.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('THEME_MAP contains all built-in themes', () => {
    for (const theme of BUILTIN_THEMES) {
      expect(THEME_MAP.get(theme.id)).toBe(theme);
    }
  });

  it('every theme has required fields', () => {
    for (const theme of BUILTIN_THEMES) {
      expect(theme.id).toBeTruthy();
      expect(theme.label).toBeTruthy();
      expect(theme.description).toBeTruthy();
      expect(theme.icon).toBeTruthy();
      expect(['dark', 'light', 'colorful']).toContain(theme.category);
    }
  });

  it('every non-default theme has a valid preview object', () => {
    const nonDefaults = BUILTIN_THEMES.filter((t) => t.id !== 'default');
    for (const theme of nonDefaults) {
      expect(theme.preview.bg).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(theme.preview.accent).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(theme.preview.text).toMatch(/^#[0-9a-fA-F]{6}$/);
    }
  });

  it('exports individual theme constants', () => {
    const expected: ThemeDefinition[] = [
      THEME_DEFAULT, THEME_CORPORATE, THEME_CAT, THEME_SAKURA,
      THEME_KIDS, THEME_BRAIN, THEME_MIDNIGHT, THEME_AURORA, THEME_PASTEL,
    ];
    for (const theme of expected) {
      expect(BUILTIN_THEMES).toContain(theme);
    }
  });

  it('corporate theme is professional with indigo accent', () => {
    expect(THEME_CORPORATE.category).toBe('light');
    expect(THEME_CORPORATE.preview.accent).toBe('#5b5fc7');
  });

  it('cat theme has warm amber accent', () => {
    expect(THEME_CAT.preview.accent).toBe('#f59e0b');
  });

  it('sakura theme has pink accent', () => {
    expect(THEME_SAKURA.preview.accent).toBe('#f472b6');
  });

  it('brain theme has teal/green accent', () => {
    expect(THEME_BRAIN.preview.accent).toBe('#00e6b4');
  });

  it('midnight theme has near-black bg', () => {
    expect(THEME_MIDNIGHT.preview.bg).toBe('#050505');
  });

  it('light themes have 2 light-category entries', () => {
    const lightThemes = BUILTIN_THEMES.filter((t) => t.category === 'light');
    expect(lightThemes.length).toBeGreaterThanOrEqual(2);
  });
});

// ── WCAG contrast tests ───────────────────────────────────────────────────────

describe('theme WCAG contrast', () => {
  it('corporate: text-primary on bg-card meets WCAG AA (≥4.5:1)', () => {
    // text #242424 on bg #ffffff
    expect(contrastRatio('#242424', '#ffffff')).toBeGreaterThanOrEqual(4.5);
  });

  it('pastel: text-primary on bg-surface meets WCAG AA (≥4.5:1)', () => {
    // text #3f3f46 on bg #ffffff
    expect(contrastRatio('#3f3f46', '#ffffff')).toBeGreaterThanOrEqual(4.5);
  });

  it('brain: text-on-accent is dark for WCAG AA on teal accent', () => {
    // #0a0f14 on #00e6b4 should pass; white would fail
    expect(contrastRatio('#0a0f14', '#00e6b4')).toBeGreaterThan(
      contrastRatio('#ffffff', '#00e6b4'),
    );
    expect(contrastRatio('#0a0f14', '#00e6b4')).toBeGreaterThanOrEqual(4.5);
  });

  it('aurora: text-on-accent is dark for WCAG AA on green accent', () => {
    // #0c1a1e on #34d399
    expect(contrastRatio('#0c1a1e', '#34d399')).toBeGreaterThanOrEqual(4.5);
  });

  it('cat: text-on-accent is dark for WCAG AA on amber accent', () => {
    // #1a1410 on #f59e0b
    expect(contrastRatio('#1a1410', '#f59e0b')).toBeGreaterThanOrEqual(4.5);
  });

  it('corporate: warning-text on bg-base has sufficient contrast', () => {
    // #835b00 on #f5f5f5
    expect(contrastRatio('#835b00', '#f5f5f5')).toBeGreaterThanOrEqual(4.5);
  });

  it('pastel: warning-text on bg-base has sufficient contrast', () => {
    // #92400e on #faf8f5
    expect(contrastRatio('#92400e', '#faf8f5')).toBeGreaterThanOrEqual(4.5);
  });

  it('default/adventurer: text-primary on bg-base meets WCAG AA', () => {
    expect(contrastRatio('#f1f5f9', '#0f172a')).toBeGreaterThanOrEqual(4.5);
  });

  it('midnight: text-primary on bg-base meets WCAG AA', () => {
    expect(contrastRatio('#c0caf5', '#050505')).toBeGreaterThanOrEqual(4.5);
  });

  it('brain: text-primary on bg-base meets WCAG AA', () => {
    expect(contrastRatio('#d0f0e8', '#0a0f14')).toBeGreaterThanOrEqual(4.5);
  });
});
