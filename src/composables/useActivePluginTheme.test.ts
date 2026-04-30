/**
 * Tests for `useActivePluginTheme` (Chunk 22.3).
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { nextTick } from 'vue';
import {
  __resetActivePluginThemeForTests,
  applyTokens,
  filterActiveThemes,
  removeTokens,
  useActivePluginTheme,
} from './useActivePluginTheme';
import { usePluginStore, type InstalledPlugin } from '../stores/plugins';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

function buildPlugin(
  id: string,
  state: 'installed' | 'active' | 'disabled',
  themes: { id: string; label: string; tokens: Record<string, string> }[] = [],
): InstalledPlugin {
  return {
    manifest: {
      id,
      display_name: id,
      version: '1.0.0',
      description: '',
      kind: 'theme',
      install_method: 'native',
      capabilities: [],
      activation_events: [],
      contributes: {
        commands: [],
        views: [],
        settings: [],
        themes,
        slash_commands: [],
        memory_hooks: [],
      },
      api_version: 1,
      dependencies: [],
    },
    state,
    installed_at: 1_700_000_000,
    last_active_at: null,
  };
}

describe('useActivePluginTheme', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    __resetActivePluginThemeForTests();
    document.documentElement.removeAttribute('style');
    localStorage.clear();
  });

  afterEach(() => {
    __resetActivePluginThemeForTests();
    document.documentElement.removeAttribute('style');
  });

  it('filterActiveThemes only returns themes from active plugins', () => {
    const themes = [
      { id: 'a-theme', label: 'A', tokens: {} },
      { id: 'b-theme', label: 'B', tokens: {} },
      { id: 'c-theme', label: 'C', tokens: {} },
    ];
    const plugins = [
      buildPlugin('a', 'active', [{ id: 'a-theme', label: 'A', tokens: {} }]),
      buildPlugin('b', 'disabled', [{ id: 'b-theme', label: 'B', tokens: {} }]),
      buildPlugin('c', 'active', [{ id: 'c-theme', label: 'C', tokens: {} }]),
    ];
    const filtered = filterActiveThemes(themes, plugins);
    expect(filtered.map((t) => t.id).sort()).toEqual(['a-theme', 'c-theme']);
  });

  it('applyTokens writes only --ts-* CSS custom properties', () => {
    const keys = applyTokens({
      '--ts-primary': '#ff0066',
      '--ts-bg': '#000',
      'invalid-key': 'oops', // should be ignored
    });
    expect(keys.has('--ts-primary')).toBe(true);
    expect(keys.has('--ts-bg')).toBe(true);
    expect(keys.has('invalid-key')).toBe(false);
    expect(document.documentElement.style.getPropertyValue('--ts-primary')).toBe('#ff0066');
  });

  it('removeTokens clears every key idempotently', () => {
    const keys = applyTokens({ '--ts-x': 'red' });
    expect(document.documentElement.style.getPropertyValue('--ts-x')).toBe('red');
    removeTokens(keys);
    expect(document.documentElement.style.getPropertyValue('--ts-x')).toBe('');
    // Second call is a no-op.
    removeTokens(keys);
    expect(document.documentElement.style.getPropertyValue('--ts-x')).toBe('');
  });

  it('setActivePluginTheme applies the chosen theme tokens', async () => {
    const store = usePluginStore();
    store.plugins = [
      buildPlugin('p', 'active', [
        { id: 'cool', label: 'Cool', tokens: { '--ts-primary': 'cyan' } },
      ]),
    ];
    store.themes = [{ id: 'cool', label: 'Cool', tokens: { '--ts-primary': 'cyan' } }];

    const { setActivePluginTheme, activeThemeId } = useActivePluginTheme();
    await nextTick();
    setActivePluginTheme('cool');
    expect(activeThemeId.value).toBe('cool');
    expect(document.documentElement.style.getPropertyValue('--ts-primary')).toBe('cyan');
  });

  it('refuses to activate a theme not contributed by any active plugin', async () => {
    const store = usePluginStore();
    store.plugins = [
      buildPlugin('p', 'disabled', [
        { id: 'unavailable', label: 'X', tokens: { '--ts-primary': 'red' } },
      ]),
    ];
    store.themes = [{ id: 'unavailable', label: 'X', tokens: { '--ts-primary': 'red' } }];

    const { setActivePluginTheme, activeThemeId } = useActivePluginTheme();
    await nextTick();
    setActivePluginTheme('unavailable');
    expect(activeThemeId.value).toBeNull();
    expect(document.documentElement.style.getPropertyValue('--ts-primary')).toBe('');
  });

  it('hot-swaps off when the contributing plugin is deactivated', async () => {
    const store = usePluginStore();
    const themes = [{ id: 'cool', label: 'Cool', tokens: { '--ts-primary': 'cyan' } }];
    store.plugins = [buildPlugin('p', 'active', themes)];
    store.themes = themes;

    const { setActivePluginTheme, activeThemeId } = useActivePluginTheme();
    await nextTick();
    setActivePluginTheme('cool');
    expect(document.documentElement.style.getPropertyValue('--ts-primary')).toBe('cyan');

    // Plugin deactivates → token must be removed and active id reset.
    store.plugins = [buildPlugin('p', 'disabled', themes)];
    await nextTick();
    expect(activeThemeId.value).toBeNull();
    expect(document.documentElement.style.getPropertyValue('--ts-primary')).toBe('');
  });

  it('persists the selection across composable re-instantiation', async () => {
    const store = usePluginStore();
    const themes = [{ id: 'persist', label: 'P', tokens: { '--ts-primary': 'gold' } }];
    store.plugins = [buildPlugin('p', 'active', themes)];
    store.themes = themes;

    const { setActivePluginTheme } = useActivePluginTheme();
    await nextTick();
    setActivePluginTheme('persist');

    expect(localStorage.getItem('ts-active-plugin-theme')).toBe('persist');

    // Reset module state but keep localStorage — simulates reload.
    __resetActivePluginThemeForTests();
    // Manually re-set localStorage since __reset clears it.
    localStorage.setItem('ts-active-plugin-theme', 'persist');

    setActivePinia(createPinia());
    const store2 = usePluginStore();
    store2.plugins = [buildPlugin('p', 'active', themes)];
    store2.themes = themes;

    const { activeThemeId } = useActivePluginTheme();
    await nextTick();
    // Wait an extra microtask for the deferred apply.
    await Promise.resolve();
    expect(activeThemeId.value).toBe('persist');
  });

  it('null clears the active theme', async () => {
    const store = usePluginStore();
    const themes = [{ id: 't', label: 'T', tokens: { '--ts-primary': 'red' } }];
    store.plugins = [buildPlugin('p', 'active', themes)];
    store.themes = themes;

    const { setActivePluginTheme, activeThemeId } = useActivePluginTheme();
    await nextTick();
    setActivePluginTheme('t');
    expect(document.documentElement.style.getPropertyValue('--ts-primary')).toBe('red');

    setActivePluginTheme(null);
    expect(activeThemeId.value).toBeNull();
    expect(document.documentElement.style.getPropertyValue('--ts-primary')).toBe('');
    expect(localStorage.getItem('ts-active-plugin-theme')).toBeNull();
  });
});
