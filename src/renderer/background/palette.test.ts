import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Color } from 'three';
import { clonePalette, lerpPalette, readPalette, watchTheme } from './palette';

function setTokens(tokens: Record<string, string>): void {
  for (const [k, v] of Object.entries(tokens)) {
    document.documentElement.style.setProperty(k, v);
  }
}

function clearTokens(): void {
  const html = document.documentElement;
  for (const t of ['--ts-bg-c1', '--ts-bg-c2', '--ts-bg-c3', '--ts-bg-accent']) {
    html.style.removeProperty(t);
  }
  delete html.dataset.theme;
}

describe('background palette reader', () => {
  afterEach(() => clearTokens());

  it('reads four colours from the live <html> computed style', () => {
    setTokens({
      '--ts-bg-c1':     '#112233',
      '--ts-bg-c2':     '#445566',
      '--ts-bg-c3':     '#778899',
      '--ts-bg-accent': '#aabbcc',
    });
    const p = readPalette();
    // THREE.Color stores normalised RGB; round-trip through getHexString()
    // for a stable comparison.
    expect(`#${p.c1.getHexString()}`).toBe('#112233');
    expect(`#${p.c2.getHexString()}`).toBe('#445566');
    expect(`#${p.c3.getHexString()}`).toBe('#778899');
    expect(`#${p.accent.getHexString()}`).toBe('#aabbcc');
  });

  it('falls back to a default palette when tokens are missing', () => {
    clearTokens();
    const p = readPalette();
    // Colors must still be valid (not NaN) so the shader does not blow up.
    expect(Number.isFinite(p.c1.r)).toBe(true);
    expect(Number.isFinite(p.accent.b)).toBe(true);
  });

  it('falls back when a token contains an unparseable value', () => {
    setTokens({ '--ts-bg-c1': 'not-a-color' });
    const p = readPalette();
    expect(Number.isFinite(p.c1.r)).toBe(true);
  });
});

describe('background palette tween helpers', () => {
  it('clonePalette produces independent THREE.Color instances', () => {
    const src = {
      c1:     new Color('#100000'),
      c2:     new Color('#001000'),
      c3:     new Color('#000010'),
      accent: new Color('#ffffff'),
    };
    const dup = clonePalette(src);
    dup.c1.set('#ff0000');
    expect(`#${src.c1.getHexString()}`).toBe('#100000');
    expect(`#${dup.c1.getHexString()}`).toBe('#ff0000');
  });

  it('lerpPalette interpolates each channel toward target', () => {
    const cur = {
      c1:     new Color(0, 0, 0),
      c2:     new Color(0, 0, 0),
      c3:     new Color(0, 0, 0),
      accent: new Color(0, 0, 0),
    };
    const tgt = {
      c1:     new Color(1, 1, 1),
      c2:     new Color(1, 1, 1),
      c3:     new Color(1, 1, 1),
      accent: new Color(1, 1, 1),
    };
    lerpPalette(cur, tgt, 0.5);
    expect(cur.c1.r).toBeCloseTo(0.5, 5);
    expect(cur.accent.b).toBeCloseTo(0.5, 5);
  });

  it('lerpPalette clamps alpha to 0..1', () => {
    const cur     = { c1: new Color(0, 0, 0), c2: new Color(0, 0, 0), c3: new Color(0, 0, 0), accent: new Color(0, 0, 0) };
    const tgt     = { c1: new Color(1, 0, 0), c2: new Color(0, 1, 0), c3: new Color(0, 0, 1), accent: new Color(1, 1, 1) };
    lerpPalette(cur, tgt, 5);
    expect(cur.c1.r).toBeCloseTo(1, 5);
  });
});

describe('watchTheme', () => {
  beforeEach(() => clearTokens());
  afterEach(() => clearTokens());

  it('fires the callback when data-theme changes on <html>', async () => {
    const cb = vi.fn();
    const stop = watchTheme(cb);
    document.documentElement.dataset.theme = 'sakura';
    // MutationObserver fires asynchronously
    await new Promise((r) => setTimeout(r, 0));
    expect(cb).toHaveBeenCalledTimes(1);
    stop();
  });

  it('does not fire after disposal', async () => {
    const cb = vi.fn();
    const stop = watchTheme(cb);
    stop();
    document.documentElement.dataset.theme = 'aurora';
    await new Promise((r) => setTimeout(r, 0));
    expect(cb).not.toHaveBeenCalled();
  });

  it('returns a no-op disposer when MutationObserver is unavailable', () => {
    const original = globalThis.MutationObserver;
    // Simulate an environment without MutationObserver
    // (e.g. some SSR stubs).
    // @ts-expect-error — intentional removal for the test
    delete globalThis.MutationObserver;
    try {
      const stop = watchTheme(() => {});
      expect(typeof stop).toBe('function');
      // Should not throw
      stop();
    } finally {
      globalThis.MutationObserver = original;
    }
  });
});
