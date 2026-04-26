import { describe, expect, it } from 'vitest';
import { getPreset, knownThemeIds, MODE_INDEX } from './preset';

const REGISTERED_THEME_IDS = [
  'default',
  'corporate',
  'corporate-dark',
  'cat',
  'sakura',
  'kids',
  'brain',
  'midnight',
  'aurora',
  'pastel',
] as const;

describe('background preset table', () => {
  it('returns a registered preset for every built-in theme', () => {
    for (const id of REGISTERED_THEME_IDS) {
      const preset = getPreset(id);
      // Every entry in the preset table must reference a defined mode.
      expect(MODE_INDEX[preset.mode]).toBeTypeOf('number');
      expect(preset.speed).toBeGreaterThanOrEqual(0);
      expect(preset.speed).toBeLessThanOrEqual(2);
      expect(preset.intensity).toBeGreaterThanOrEqual(0);
      expect(preset.intensity).toBeLessThanOrEqual(1);
    }
  });

  it('exposes every built-in theme via knownThemeIds()', () => {
    const known = new Set(knownThemeIds());
    for (const id of REGISTERED_THEME_IDS) {
      expect(known.has(id)).toBe(true);
    }
  });

  it('falls back to the default preset for unknown themes', () => {
    const preset = getPreset('not-a-real-theme');
    expect(preset.mode).toBe('nebula');
    expect(preset.speed).toBeGreaterThan(0);
  });

  it('falls back to the default preset for null / empty inputs', () => {
    expect(getPreset(null).mode).toBe('nebula');
    expect(getPreset(undefined).mode).toBe('nebula');
    expect(getPreset('').mode).toBe('nebula');
  });

  it('uses Linear-style beams for both corporate themes', () => {
    expect(getPreset('corporate').mode).toBe('beams');
    expect(getPreset('corporate-dark').mode).toBe('beams');
  });

  it('uses neural traces for the brain theme', () => {
    expect(getPreset('brain').mode).toBe('neural');
  });

  it('uses aurora ribbons for the aurora theme', () => {
    expect(getPreset('aurora').mode).toBe('aurora');
  });

  it('uses soft mist for the sakura theme (no cartoon petals)', () => {
    expect(getPreset('sakura').mode).toBe('mist');
  });

  it('mode index covers exactly six personalities (0..5)', () => {
    const indices = Object.values(MODE_INDEX).sort();
    expect(indices).toEqual([0, 1, 2, 3, 4, 5]);
  });
});
