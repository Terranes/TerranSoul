import { ref, computed, readonly } from 'vue';
import {
  BUILTIN_THEMES,
  DEFAULT_THEME_ID,
  THEME_MAP,
  type ThemeDefinition,
} from '../config/themes';

// ── Module-level singleton state ──────────────────────────────────────────────
// Shared across all call-sites so that the theme is consistent app-wide.

const activeThemeId = ref<string>(DEFAULT_THEME_ID);

/** Tokens that were applied to :root in the current session (for cleanup). */
let appliedTokenKeys: string[] = [];

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Resolve a theme ID to its definition, falling back to default. */
function resolveTheme(id: string): ThemeDefinition {
  return THEME_MAP.get(id) ?? BUILTIN_THEMES[0];
}

/** Write CSS custom properties onto documentElement and clean up stale ones. */
function applyTokensToDOM(theme: ThemeDefinition): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;

  // 1. Remove previously-applied overrides (restore :root stylesheet values).
  for (const key of appliedTokenKeys) {
    root.style.removeProperty(key);
  }

  // 2. Apply the new theme's token overrides.
  const keys = Object.keys(theme.tokens);
  for (const key of keys) {
    root.style.setProperty(key, theme.tokens[key]);
  }
  appliedTokenKeys = keys;

  // 3. Optional font overrides.
  if (theme.fontFamily) {
    root.style.setProperty('--ts-font-family', theme.fontFamily);
    appliedTokenKeys.push('--ts-font-family');
  }
  if (theme.fontMono) {
    root.style.setProperty('--ts-font-mono', theme.fontMono);
    appliedTokenKeys.push('--ts-font-mono');
  }

  // 4. Set a data attribute for CSS-level selectors (e.g. [data-theme="corporate"]).
  root.dataset.theme = theme.id;

  // 5. Set color-scheme for native browser widgets (scrollbars, inputs, etc.).
  root.style.colorScheme = theme.category === 'light' ? 'light' : 'dark';
}

// ── Persistence (localStorage — instant on cold-start, no async needed) ──────

const LS_KEY = 'ts-active-theme';

function loadFromStorage(): string {
  try {
    return localStorage.getItem(LS_KEY) ?? DEFAULT_THEME_ID;
  } catch {
    return DEFAULT_THEME_ID;
  }
}

function saveToStorage(id: string): void {
  try {
    localStorage.setItem(LS_KEY, id);
  } catch {
    // Storage unavailable — ignore.
  }
}

// ── Initialise on import ──────────────────────────────────────────────────────
// Apply the persisted theme before the first Vue render so there's no flash.
activeThemeId.value = loadFromStorage();
applyTokensToDOM(resolveTheme(activeThemeId.value));

// ── Sync helper ───────────────────────────────────────────────────────────────
// Applies tokens + persists in one shot (no deferred watch needed).

function activateTheme(id: string): void {
  activeThemeId.value = id;
  const theme = resolveTheme(id);
  applyTokensToDOM(theme);
  saveToStorage(id);
}

// ── Composable ────────────────────────────────────────────────────────────────

export function useTheme() {
  const activeTheme = computed<ThemeDefinition>(() => resolveTheme(activeThemeId.value));

  /** Whether the active theme uses a light background. */
  const isLight = computed(() => activeTheme.value.category === 'light');

  /** Switch to a different theme by ID. No-op if the ID is unknown. */
  function setTheme(id: string): void {
    if (THEME_MAP.has(id) && id !== activeThemeId.value) {
      activateTheme(id);
    }
  }

  /** Cycle to the next theme in the built-in list. */
  function nextTheme(): void {
    const idx = BUILTIN_THEMES.findIndex((t) => t.id === activeThemeId.value);
    const next = (idx + 1) % BUILTIN_THEMES.length;
    activateTheme(BUILTIN_THEMES[next].id);
  }

  return {
    /** Reactive ID of the active theme. */
    themeId: readonly(activeThemeId),
    /** The full resolved ThemeDefinition. */
    activeTheme,
    /** Whether the active theme is a light variant. */
    isLight,
    /** All available built-in themes. */
    themes: BUILTIN_THEMES,
    /** Switch theme by ID. */
    setTheme,
    /** Cycle to the next theme. */
    nextTheme,
  };
}
