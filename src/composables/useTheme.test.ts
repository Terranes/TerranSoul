import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { useTheme } from './useTheme';
import { BUILTIN_THEMES, DEFAULT_THEME_ID } from '../config/themes';

describe('useTheme', () => {
  beforeEach(() => {
    // Reset to default before each test
    localStorage.removeItem('ts-active-theme');
    const { setTheme } = useTheme();
    setTheme('default');
  });

  afterEach(() => {
    // Clean up any inline styles written by applyTokensToDOM
    const root = document.documentElement;
    root.removeAttribute('data-theme');
    root.style.cssText = '';
    localStorage.removeItem('ts-active-theme');
  });

  it('returns the default theme initially', () => {
    const { themeId, activeTheme } = useTheme();
    expect(themeId.value).toBe(DEFAULT_THEME_ID);
    expect(activeTheme.value.id).toBe('default');
  });

  it('exposes the full list of built-in themes', () => {
    const { themes } = useTheme();
    expect(themes.length).toBeGreaterThanOrEqual(9);
    expect(themes).toBe(BUILTIN_THEMES);
  });

  it('setTheme switches the active theme', () => {
    const { themeId, setTheme, activeTheme } = useTheme();
    setTheme('corporate');
    expect(themeId.value).toBe('corporate');
    expect(activeTheme.value.label).toBe('Corporate');
  });

  it('setTheme is a no-op for unknown IDs', () => {
    const { themeId, setTheme } = useTheme();
    setTheme('nonexistent-theme');
    expect(themeId.value).toBe('default');
  });

  it('applies data-theme attribute instead of CSS custom properties', () => {
    const { setTheme } = useTheme();
    setTheme('corporate');
    const root = document.documentElement;
    // CSS-first: tokens are in the stylesheet, not inline styles.
    // The composable sets data-theme; the cascade handles the rest.
    expect(root.dataset.theme).toBe('corporate');
    expect(root.style.getPropertyValue('--ts-bg-base')).toBe('');
  });

  it('clears previous tokens when switching themes', () => {
    const { setTheme } = useTheme();
    setTheme('corporate');
    // CSS-first: tokens are not written to inline styles.
    expect(document.documentElement.dataset.theme).toBe('corporate');

    setTheme('default');
    expect(document.documentElement.dataset.theme).toBe('default');
  });

  it('sets data-theme attribute on root', () => {
    const { setTheme } = useTheme();
    setTheme('sakura');
    expect(document.documentElement.dataset.theme).toBe('sakura');
  });

  it('sets color-scheme to light for light themes', () => {
    const { setTheme } = useTheme();
    setTheme('corporate');
    expect(document.documentElement.style.colorScheme).toBe('light');
  });

  it('sets color-scheme to dark for dark themes', () => {
    const { setTheme } = useTheme();
    setTheme('midnight');
    expect(document.documentElement.style.colorScheme).toBe('dark');
  });

  it('isLight computed is true for light themes', () => {
    const { setTheme, isLight } = useTheme();
    setTheme('corporate');
    expect(isLight.value).toBe(true);

    setTheme('midnight');
    expect(isLight.value).toBe(false);
  });

  it('persists theme choice to localStorage', () => {
    const { setTheme } = useTheme();
    setTheme('brain');
    expect(localStorage.getItem('ts-active-theme')).toBe('brain');
  });

  it('nextTheme cycles through all themes', () => {
    const { themeId, nextTheme } = useTheme();
    const seen = new Set<string>();
    for (let i = 0; i < BUILTIN_THEMES.length; i++) {
      seen.add(themeId.value);
      nextTheme();
    }
    expect(seen.size).toBe(BUILTIN_THEMES.length);
  });

  it('nextTheme wraps around to the first theme', () => {
    const { themeId, nextTheme } = useTheme();
    for (let i = 0; i < BUILTIN_THEMES.length; i++) {
      nextTheme();
    }
    expect(themeId.value).toBe(BUILTIN_THEMES[0].id);
  });

  it('singleton state is shared across multiple useTheme() calls', () => {
    const a = useTheme();
    const b = useTheme();
    a.setTheme('cat');
    expect(b.themeId.value).toBe('cat');
    expect(b.activeTheme.value.id).toBe('cat');
  });

  it('applies font overrides when theme specifies them via CSS cascade', () => {
    const { setTheme } = useTheme();
    setTheme('kids');
    const root = document.documentElement;
    // CSS-first: font overrides live in html[data-theme="kids"] in style.css.
    // The composable only sets the data-theme attribute; no inline style.
    expect(root.dataset.theme).toBe('kids');
    expect(root.style.getPropertyValue('--ts-font-family')).toBe('');
  });
});
