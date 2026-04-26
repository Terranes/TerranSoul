import { ref, computed, readonly } from 'vue';
import {
  BUILTIN_THEMES,
  DEFAULT_THEME_ID,
  THEME_MAP,
  type ThemeDefinition,
} from '../config/themes';

// ── Module-level singleton state ──────────────────────────────────────────────

const activeThemeId = ref<string>(DEFAULT_THEME_ID);

// ── Helpers ───────────────────────────────────────────────────────────────────

function resolveTheme(id: string): ThemeDefinition {
  return THEME_MAP.get(id) ?? BUILTIN_THEMES[0];
}

/**
 * Apply a theme by setting the data-theme attribute on <html>.
 * All CSS token overrides are declared as html[data-theme="*"] blocks in
 * src/style.css — no inline style manipulation needed.
 */
function applyThemeToDOM(theme: ThemeDefinition): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.dataset.theme = theme.id;
  root.style.colorScheme = theme.category === 'light' ? 'light' : 'dark';
  root.style.fontFamily = ''; // reset any stale override; CSS handles font via --ts-font-family
}

// ── Persistence ───────────────────────────────────────────────────────────────

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
activeThemeId.value = loadFromStorage();
applyThemeToDOM(resolveTheme(activeThemeId.value));

// ── Internal helper ───────────────────────────────────────────────────────────

function activateTheme(id: string): void {
  activeThemeId.value = id;
  applyThemeToDOM(resolveTheme(id));
  saveToStorage(id);
}

// ── Composable ────────────────────────────────────────────────────────────────

export function useTheme() {
  const activeTheme = computed<ThemeDefinition>(() => resolveTheme(activeThemeId.value));
  const isLight = computed(() => activeTheme.value.category === 'light');

  function setTheme(id: string): void {
    if (THEME_MAP.has(id) && id !== activeThemeId.value) {
      activateTheme(id);
    }
  }

  function nextTheme(): void {
    const idx = BUILTIN_THEMES.findIndex((t) => t.id === activeThemeId.value);
    activateTheme(BUILTIN_THEMES[(idx + 1) % BUILTIN_THEMES.length].id);
  }

  return {
    themeId: readonly(activeThemeId),
    activeTheme,
    isLight,
    themes: BUILTIN_THEMES,
    setTheme,
    nextTheme,
  };
}
