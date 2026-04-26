/**
 * Theme-palette reader for the animated background shader.
 *
 * The background shader takes four `vec3` colour uniforms.  Their values
 * are sourced from per-theme CSS custom properties on `<html>`:
 *
 *   --ts-bg-c1     — primary base colour (low-fbm regions)
 *   --ts-bg-c2     — secondary colour   (mid-fbm regions)
 *   --ts-bg-c3     — tertiary highlight (high-fbm regions / glows)
 *   --ts-bg-accent — sparkle / pulse / beam highlight
 *
 * Themes that haven't defined the new tokens fall back to a sensible
 * dark palette so the scene still renders.
 */

import { Color } from 'three';

export interface Palette {
  c1: Color;
  c2: Color;
  c3: Color;
  accent: Color;
}

const FALLBACK: Palette = Object.freeze({
  c1:     new Color('#0a0f1f'),
  c2:     new Color('#1a1f3a'),
  c3:     new Color('#2a3060'),
  accent: new Color('#7c6fff'),
});

/**
 * Parse a CSS colour string into a THREE.Color.  Returns null when the
 * input is missing or unparseable so callers can fall back gracefully.
 *
 * THREE.Color() throws on unparseable strings (e.g. empty), so we wrap
 * in a try/catch — the constructor accepts hex, named colours and
 * hsl() / rgb() functional notation, which covers every value in our
 * design-token vocabulary.
 */
function parseCssColor(raw: string | undefined): Color | null {
  if (!raw) return null;
  const trimmed = raw.trim();
  if (!trimmed) return null;
  try {
    return new Color(trimmed);
  } catch {
    return null;
  }
}

/**
 * Read the four palette tokens from the live `<html>` computed style.
 *
 * Designed to be called from a `MutationObserver` watching the
 * `data-theme` attribute on `<html>` — the new theme's tokens have
 * already been applied by `useTheme.applyThemeToDOM` at that point.
 */
export function readPalette(root: HTMLElement = document.documentElement): Palette {
  const cs = getComputedStyle(root);
  const c1     = parseCssColor(cs.getPropertyValue('--ts-bg-c1'))     ?? FALLBACK.c1.clone();
  const c2     = parseCssColor(cs.getPropertyValue('--ts-bg-c2'))     ?? FALLBACK.c2.clone();
  const c3     = parseCssColor(cs.getPropertyValue('--ts-bg-c3'))     ?? FALLBACK.c3.clone();
  const accent = parseCssColor(cs.getPropertyValue('--ts-bg-accent')) ?? FALLBACK.accent.clone();
  return { c1, c2, c3, accent };
}

/**
 * Snapshot the four current target colours so a tween can interpolate
 * smoothly to a new palette without touching the originals.
 */
export function clonePalette(p: Palette): Palette {
  return {
    c1:     p.c1.clone(),
    c2:     p.c2.clone(),
    c3:     p.c3.clone(),
    accent: p.accent.clone(),
  };
}

/**
 * Linearly interpolate `current` toward `target` by `alpha` (0..1).
 * Mutates `current` in place — the typical caller is the per-frame
 * tween in `scene.ts`.
 */
export function lerpPalette(current: Palette, target: Palette, alpha: number): void {
  const a = Math.max(0, Math.min(1, alpha));
  current.c1.lerp(target.c1, a);
  current.c2.lerp(target.c2, a);
  current.c3.lerp(target.c3, a);
  current.accent.lerp(target.accent, a);
}

/**
 * Subscribe to `data-theme` changes on `<html>`.  The supplied callback
 * is invoked synchronously after each attribute change with the freshly
 * read palette.  Returns a disposer that disconnects the observer.
 *
 * No-op outside a browser environment (returns a no-op disposer) so
 * server-side rendering and Vitest do not blow up.
 */
export function watchTheme(
  onChange: (next: Palette) => void,
  root: HTMLElement = document.documentElement,
): () => void {
  if (typeof MutationObserver === 'undefined') return () => {};
  const observer = new MutationObserver((records) => {
    for (const r of records) {
      if (r.type === 'attributes' && r.attributeName === 'data-theme') {
        onChange(readPalette(root));
        return;
      }
    }
  });
  observer.observe(root, { attributes: true, attributeFilter: ['data-theme'] });
  return () => observer.disconnect();
}
