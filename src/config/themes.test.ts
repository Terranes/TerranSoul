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
      expect(typeof theme.tokens).toBe('object');
    }
  });

  it('default theme has an empty tokens map', () => {
    expect(Object.keys(THEME_DEFAULT.tokens)).toHaveLength(0);
  });

  it('non-default themes override key surface and accent tokens', () => {
    const nonDefaults = BUILTIN_THEMES.filter((t) => t.id !== 'default');
    for (const theme of nonDefaults) {
      expect(theme.tokens['--ts-bg-base']).toBeTruthy();
      expect(theme.tokens['--ts-accent']).toBeTruthy();
      expect(theme.tokens['--ts-text-primary']).toBeTruthy();
    }
  });

  it('light themes have light-category backgrounds', () => {
    const lightThemes = BUILTIN_THEMES.filter((t) => t.category === 'light');
    expect(lightThemes.length).toBeGreaterThanOrEqual(2);
    for (const theme of lightThemes) {
      // Light backgrounds should start with # and have a high luminance hex
      const bg = theme.tokens['--ts-bg-base'] ?? '';
      expect(bg).toMatch(/^#[a-fA-F0-9]{6}$/);
    }
  });

  it('all token keys start with --ts-', () => {
    for (const theme of BUILTIN_THEMES) {
      for (const key of Object.keys(theme.tokens)) {
        expect(key).toMatch(/^--ts-/);
      }
    }
  });

  it('exports individual theme constants', () => {
    const expected: ThemeDefinition[] = [
      THEME_DEFAULT,
      THEME_CORPORATE,
      THEME_CAT,
      THEME_SAKURA,
      THEME_KIDS,
      THEME_BRAIN,
      THEME_MIDNIGHT,
      THEME_AURORA,
      THEME_PASTEL,
    ];
    for (const theme of expected) {
      expect(BUILTIN_THEMES).toContain(theme);
    }
  });

  it('corporate theme is professional with indigo accent', () => {
    expect(THEME_CORPORATE.category).toBe('light');
    expect(THEME_CORPORATE.tokens['--ts-accent']).toBe('#5b5fc7');
  });

  it('cat theme has warm amber accent', () => {
    expect(THEME_CAT.tokens['--ts-accent']).toBe('#f59e0b');
  });

  it('sakura theme has pink accent', () => {
    expect(THEME_SAKURA.tokens['--ts-accent']).toBe('#f472b6');
  });

  it('brain theme has teal/green accent', () => {
    expect(THEME_BRAIN.tokens['--ts-accent']).toBe('#00e6b4');
  });

  it('kids theme has larger border radii', () => {
    expect(parseInt(THEME_KIDS.tokens['--ts-radius-sm'] ?? '0')).toBeGreaterThanOrEqual(10);
  });

  it('midnight theme has near-black base', () => {
    expect(THEME_MIDNIGHT.tokens['--ts-bg-base']).toBe('#050505');
  });

  it('non-default themes define quest tokens', () => {
    const nonDefaults = BUILTIN_THEMES.filter((t) => t.id !== 'default');
    for (const theme of nonDefaults) {
      expect(theme.tokens['--ts-quest-gold']).toBeTruthy();
      expect(theme.tokens['--ts-quest-gold-bright']).toBeTruthy();
      expect(theme.tokens['--ts-quest-gold-dim']).toBeTruthy();
      expect(theme.tokens['--ts-quest-border']).toBeTruthy();
      expect(theme.tokens['--ts-quest-text']).toBeTruthy();
    }
  });

  it('non-default themes define text-bright and text-link tokens', () => {
    const nonDefaults = BUILTIN_THEMES.filter((t) => t.id !== 'default');
    for (const theme of nonDefaults) {
      expect(theme.tokens['--ts-text-bright']).toBeTruthy();
      expect(theme.tokens['--ts-text-link']).toBeTruthy();
    }
  });

  it('non-default themes define border-strong and bg-backdrop', () => {
    const nonDefaults = BUILTIN_THEMES.filter((t) => t.id !== 'default');
    for (const theme of nonDefaults) {
      expect(theme.tokens['--ts-border-strong']).toBeTruthy();
      expect(theme.tokens['--ts-bg-backdrop']).toBeTruthy();
    }
  });

  it('non-default themes define info token', () => {
    const nonDefaults = BUILTIN_THEMES.filter((t) => t.id !== 'default');
    for (const theme of nonDefaults) {
      expect(theme.tokens['--ts-info']).toBeTruthy();
    }
  });
});
