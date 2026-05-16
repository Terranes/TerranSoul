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
}

// ── Persistence ───────────────────────────────────────────────────────────────

const LS_KEY = 'ts-active-theme';
/**
 * One-time migration marker. When DEFAULT_THEME_ID changes between releases,
 * bump this string so users who were silently riding the previous default
 * (here `corporate-dark`) get moved onto the new default exactly once. Users
 * who explicitly picked a non-default theme after the migration ran keep
 * their choice forever.
 */
const LS_DEFAULT_MIGRATION_KEY = 'ts-active-theme-default-migration';
const CURRENT_DEFAULT_MIGRATION = 'v2-soul-of-terransoul';
const PREVIOUS_DEFAULT_ID = 'corporate-dark';

function loadFromStorage(): string {
  try {
    const saved = localStorage.getItem(LS_KEY);
    const lastMigration = localStorage.getItem(LS_DEFAULT_MIGRATION_KEY);
    // Migrate exactly once: if the user never explicitly picked a theme
    // (saved === null) OR they were on the old silent default, snap to the
    // new default and remember we did it.
    if (lastMigration !== CURRENT_DEFAULT_MIGRATION) {
      localStorage.setItem(LS_DEFAULT_MIGRATION_KEY, CURRENT_DEFAULT_MIGRATION);
      if (saved === null || saved === PREVIOUS_DEFAULT_ID) {
        localStorage.setItem(LS_KEY, DEFAULT_THEME_ID);
        return DEFAULT_THEME_ID;
      }
    }
    return saved ?? DEFAULT_THEME_ID;
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
