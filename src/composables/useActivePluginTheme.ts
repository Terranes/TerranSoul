/**
 * **Chunk 22.3** — Plugin theme contribution applier.
 *
 * Plugins can declare CSS-token themes via
 * `manifest.contributes.themes[].tokens` (a `Record<string, string>` of
 * CSS custom properties → values, e.g. `{ "--ts-primary": "#ff0066" }`).
 * The TerranSoul shell already exposes a `--ts-*` token layer in
 * `src/style.css`; this composable wires the plugin contributions into
 * `document.documentElement.style.setProperty(...)` so they hot-swap
 * without a reload.
 *
 * Design notes
 * ------------
 * - **Singleton state at module scope** so every component that calls
 *   `useActivePluginTheme()` shares the same `activeThemeId` ref. This
 *   matches the existing `useTheme` pattern in this folder.
 * - **Hot-swap on plugin deactivation** — a `watch` on the plugin
 *   store's `themes + plugins` array detects when the active theme's
 *   plugin disappears (or moves to disabled) and resets to the
 *   no-plugin-theme baseline. Applied tokens are removed via
 *   `removeProperty(...)` so the underlying built-in `useTheme()` styles
 *   take over again — no flicker.
 * - **Idempotent reset** — calling `setActivePluginTheme(null)` a second
 *   time is a no-op. The set of "currently-applied keys" is tracked so
 *   we only call `removeProperty` once per key.
 * - **Frontend-only persistence** — stored in `localStorage` under
 *   `ts-active-plugin-theme`. Theme choice is cosmetic and a backend
 *   round-trip is overkill.
 */
import { computed, readonly, ref, watch } from 'vue';
import { usePluginStore, type InstalledPlugin } from '../stores/plugins';

// ── Module-level singleton state ─────────────────────────────────────────

const activeThemeId = ref<string | null>(null);
let appliedKeys = new Set<string>();
let initialised = false;

const LS_KEY = 'ts-active-plugin-theme';

// ── Helpers ──────────────────────────────────────────────────────────────

function loadFromStorage(): string | null {
  try {
    return localStorage.getItem(LS_KEY);
  } catch {
    return null;
  }
}

function saveToStorage(id: string | null): void {
  try {
    if (id == null) localStorage.removeItem(LS_KEY);
    else localStorage.setItem(LS_KEY, id);
  } catch {
    // Storage unavailable — ignore.
  }
}

function isPluginActive(plugin: InstalledPlugin): boolean {
  return typeof plugin.state === 'string' && plugin.state.toLowerCase() === 'active';
}

/**
 * Filter `themes` to only those contributed by an `Active` plugin.
 * Pure helper exported for unit testing.
 */
export function filterActiveThemes(
  themes: { id: string; label: string; tokens: Record<string, string> }[],
  plugins: InstalledPlugin[],
): { id: string; label: string; tokens: Record<string, string> }[] {
  // Build set of active plugin IDs that contribute themes.
  const activeIds = new Set(
    plugins.filter(isPluginActive).map((p) => p.manifest.id),
  );
  // A theme `id` is qualified as `<plugin_id>.<theme_id>` when stored in
  // `CommandEntry`-style tuples; but `plugin_list_themes` returns just
  // the contributed `ContributedTheme { id, label, tokens }` array
  // which doesn't carry the plugin id. We can only filter when we have
  // a way to attribute the theme — fall back to checking each plugin's
  // manifest contributions.
  const allowed = new Set<string>();
  for (const plugin of plugins) {
    if (!isPluginActive(plugin)) continue;
    for (const t of plugin.manifest.contributes.themes) {
      allowed.add(t.id);
    }
  }
  void activeIds;
  return themes.filter((t) => allowed.has(t.id));
}

/**
 * Apply a theme's tokens to `document.documentElement`. Pure DOM
 * mutation; idempotent.
 *
 * Returns the set of keys that were applied so the caller can track
 * what to remove on the next swap.
 */
export function applyTokens(tokens: Record<string, string>): Set<string> {
  const keys = new Set<string>();
  if (typeof document === 'undefined') return keys;
  const root = document.documentElement;
  for (const [key, value] of Object.entries(tokens)) {
    if (!key.startsWith('--')) continue; // defensive — must be a CSS custom prop
    root.style.setProperty(key, value);
    keys.add(key);
  }
  return keys;
}

/**
 * Remove every key in `keys` from the document root. Idempotent.
 */
export function removeTokens(keys: Set<string>): void {
  if (typeof document === 'undefined' || keys.size === 0) return;
  const root = document.documentElement;
  for (const key of keys) {
    root.style.removeProperty(key);
  }
}

function applyById(
  themeId: string | null,
  themes: { id: string; tokens: Record<string, string> }[],
): void {
  // Reset previous tokens first.
  removeTokens(appliedKeys);
  appliedKeys = new Set();
  if (themeId == null) return;
  const theme = themes.find((t) => t.id === themeId);
  if (!theme) return;
  appliedKeys = applyTokens(theme.tokens);
}

// ── Composable ───────────────────────────────────────────────────────────

export function useActivePluginTheme() {
  const store = usePluginStore();

  const availableThemes = computed(() =>
    filterActiveThemes(store.themes, store.plugins),
  );

  // First-call initialisation: read persisted choice and set up the
  // reactive watcher that hot-swaps on plugin activate/deactivate.
  if (!initialised) {
    initialised = true;
    activeThemeId.value = loadFromStorage();
    // Apply on the next tick — the store may not have refreshed yet.
    queueMicrotask(() => {
      applyById(activeThemeId.value, availableThemes.value);
    });
    watch(
      availableThemes,
      (themes) => {
        // Active theme was contributed by a plugin that is no longer
        // active → reset to baseline.
        if (
          activeThemeId.value != null &&
          !themes.some((t) => t.id === activeThemeId.value)
        ) {
          activeThemeId.value = null;
          saveToStorage(null);
          removeTokens(appliedKeys);
          appliedKeys = new Set();
        } else {
          // Re-apply in case tokens changed underneath us.
          applyById(activeThemeId.value, themes);
        }
      },
      { deep: true },
    );
  }

  function setActivePluginTheme(id: string | null): void {
    if (id != null && !availableThemes.value.some((t) => t.id === id)) {
      // Refuse to activate an unknown theme — preserves invariants.
      return;
    }
    activeThemeId.value = id;
    saveToStorage(id);
    applyById(id, availableThemes.value);
  }

  return {
    activeThemeId: readonly(activeThemeId),
    availableThemes,
    setActivePluginTheme,
  };
}

// ── Test-only escape hatch ───────────────────────────────────────────────

/**
 * Reset module-level state. Intended for unit tests only — production
 * code should never call this.
 *
 * @internal
 */
export function __resetActivePluginThemeForTests(): void {
  removeTokens(appliedKeys);
  appliedKeys = new Set();
  activeThemeId.value = null;
  initialised = false;
  try {
    localStorage.removeItem(LS_KEY);
  } catch {
    // ignore
  }
}
