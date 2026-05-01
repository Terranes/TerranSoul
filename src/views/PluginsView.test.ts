/**
 * Tests for PluginsView.vue (Chunk 22.2).
 *
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { setActivePinia, createPinia } from 'pinia';
import PluginsView from './PluginsView.vue';
import type { InstalledPlugin } from '../stores/plugins';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

function buildPlugin(overrides: Partial<InstalledPlugin> = {}): InstalledPlugin {
  const base: InstalledPlugin = {
    manifest: {
      id: 'sample-plugin',
      display_name: 'Sample Plugin',
      version: '1.0.0',
      description: 'A sample plugin for testing.',
      kind: 'tool',
      install_method: 'native',
      capabilities: [],
      activation_events: [],
      contributes: {
        commands: [],
        views: [],
        settings: [],
        themes: [],
        slash_commands: [],
        memory_hooks: [],
      },
      api_version: 1,
      dependencies: [],
    },
    state: 'installed',
    installed_at: 1_700_000_000,
    last_active_at: null,
  };
  return { ...base, ...overrides };
}

function setupMocks(plugins: InstalledPlugin[] = []) {
  const settingsStore = new Map<string, unknown>();
  mockInvoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    switch (cmd) {
      case 'plugin_list':
        return plugins;
      case 'plugin_list_commands':
      case 'plugin_list_slash_commands':
      case 'plugin_list_themes':
        return [];
      case 'plugin_host_status':
        return {
          total_plugins: plugins.length,
          active_plugins: plugins.filter((p) => p.state === 'active').length,
          disabled_plugins: 0,
          error_plugins: 0,
          total_commands: 0,
          total_slash_commands: 0,
          total_themes: 0,
        };
      case 'plugin_install':
        return buildPlugin({
          manifest: { ...buildPlugin().manifest, id: 'new-plugin', display_name: 'Newly Installed' },
        });
      case 'plugin_activate':
      case 'plugin_deactivate':
      case 'plugin_uninstall':
        return undefined;
      case 'plugin_get_setting': {
        const key = (args as { key: string }).key;
        return settingsStore.has(key) ? settingsStore.get(key) : null;
      }
      case 'plugin_set_setting': {
        const { key, value } = args as { key: string; value: unknown };
        settingsStore.set(key, value);
        return undefined;
      }
      default:
        throw new Error(`Unmocked command: ${cmd} ${JSON.stringify(args)}`);
    }
  });
  return { settingsStore };
}

describe('PluginsView', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  afterEach(() => {
    mockInvoke.mockReset();
  });

  it('renders empty state when no plugins are installed', async () => {
    setupMocks([]);
    const w = mount(PluginsView);
    await flushPromises();
    expect(w.find('[data-testid="pv-empty"]').exists()).toBe(true);
    expect(w.find('[data-testid="pv-list"]').exists()).toBe(false);
    expect(w.find('[data-testid="pv-count-total"]').text()).toContain('0');
  });

  it('lists installed plugins with their state pills', async () => {
    setupMocks([
      buildPlugin({ state: 'active', last_active_at: 1_700_000_500 }),
      buildPlugin({
        manifest: { ...buildPlugin().manifest, id: 'other', display_name: 'Other' },
        state: 'disabled',
      }),
    ]);
    const w = mount(PluginsView);
    await flushPromises();

    const cards = w.findAll('.pv-card');
    expect(cards.length).toBe(2);
    expect(w.find('[data-testid="pv-state-sample-plugin"]').text()).toBe('Active');
    expect(w.find('[data-testid="pv-state-other"]').text()).toBe('Disabled');
    expect(w.find('[data-testid="pv-count-active"]').text()).toContain('1');
  });

  it('disables Activate when sensitive capabilities are not granted', async () => {
    setupMocks([
      buildPlugin({
        manifest: {
          ...buildPlugin().manifest,
          capabilities: ['chat', 'network'],
        },
      }),
    ]);
    const w = mount(PluginsView);
    await flushPromises();

    const btn = w.find('[data-testid="pv-activate-sample-plugin"]');
    expect(btn.exists()).toBe(true);
    expect(btn.attributes('disabled')).toBeDefined();
  });

  it('enables Activate after the user grants the sensitive capability', async () => {
    setupMocks([
      buildPlugin({
        manifest: {
          ...buildPlugin().manifest,
          capabilities: ['network'],
        },
      }),
    ]);
    const w = mount(PluginsView);
    await flushPromises();

    const btn = () => w.find('[data-testid="pv-activate-sample-plugin"]');
    expect(btn().attributes('disabled')).toBeDefined();

    const grantBox = w.find('[data-testid="pv-grant-sample-plugin-network"]');
    await grantBox.setValue(true);
    expect(btn().attributes('disabled')).toBeUndefined();
  });

  it('always allows Activate when only auto-granted capabilities are declared', async () => {
    setupMocks([
      buildPlugin({
        manifest: {
          ...buildPlugin().manifest,
          capabilities: ['chat', 'character'],
        },
      }),
    ]);
    const w = mount(PluginsView);
    await flushPromises();

    expect(w.find('[data-testid="pv-activate-sample-plugin"]').attributes('disabled')).toBeUndefined();
  });

  it('shows Disable when the plugin is active and calls plugin_deactivate', async () => {
    setupMocks([buildPlugin({ state: 'active' })]);
    const w = mount(PluginsView);
    await flushPromises();

    expect(w.find('[data-testid="pv-disable-sample-plugin"]').exists()).toBe(true);
    expect(w.find('[data-testid="pv-activate-sample-plugin"]').exists()).toBe(false);

    await w.find('[data-testid="pv-disable-sample-plugin"]').trigger('click');
    await flushPromises();
    expect(mockInvoke).toHaveBeenCalledWith('plugin_deactivate', { pluginId: 'sample-plugin' });
  });

  it('renders activation error message from PluginState::Error', async () => {
    setupMocks([
      buildPlugin({
        state: { Error: { message: 'capability denied' } } as unknown as string,
      }),
    ]);
    const w = mount(PluginsView);
    await flushPromises();
    expect(w.find('[data-testid="pv-state-error-sample-plugin"]').text())
      .toContain('capability denied');
  });

  it('install button calls plugin_install with file contents', async () => {
    setupMocks([]);
    const w = mount(PluginsView);
    await flushPromises();

    const fileText = JSON.stringify({ id: 'new-plugin' });
    const file = new File([fileText], 'manifest.json', { type: 'application/json' });

    // Hit the picker handler directly via the file input change event.
    const input = w.find('[data-testid="pv-file-input"]').element as HTMLInputElement;
    Object.defineProperty(input, 'files', { value: [file] });
    await w.find('[data-testid="pv-file-input"]').trigger('change');
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith('plugin_install', { json: fileText });
    expect(w.find('[data-testid="pv-install-ok"]').exists()).toBe(true);
  });

  it('rejects non-json files with a clear error message', async () => {
    setupMocks([]);
    const w = mount(PluginsView);
    await flushPromises();

    const file = new File(['not json'], 'manifest.txt', { type: 'text/plain' });
    const input = w.find('[data-testid="pv-file-input"]').element as HTMLInputElement;
    Object.defineProperty(input, 'files', { value: [file] });
    await w.find('[data-testid="pv-file-input"]').trigger('change');
    await flushPromises();

    expect(w.find('[data-testid="pv-install-error"]').text()).toContain('.json');
    expect(mockInvoke).not.toHaveBeenCalledWith('plugin_install', expect.anything());
  });

  // ── Settings (Chunk 22.6) ─────────────────────────────────────────

  function buildPluginWithSettings(): InstalledPlugin {
    const base = buildPlugin();
    return {
      ...base,
      manifest: {
        ...base.manifest,
        id: 'cfg',
        display_name: 'Cfg',
        contributes: {
          ...base.manifest.contributes,
          settings: [
            {
              key: 'enabled',
              label: 'Enabled',
              description: 'Whether the plugin is enabled.',
              default_value: true,
              value_type: 'boolean',
            },
            {
              key: 'maxRetries',
              label: 'Max retries',
              description: 'Retry count.',
              default_value: 3,
              value_type: 'number',
            },
            {
              key: 'apiKey',
              label: 'API Key',
              description: 'Auth token.',
              default_value: '',
              value_type: 'string',
            },
            {
              key: 'mode',
              label: 'Mode',
              description: 'Operating mode.',
              default_value: 'fast',
              value_type: { enum: { values: ['fast', 'slow'] } },
            },
          ],
        },
      },
    };
  }

  it('renders a settings section per plugin with each control kind', async () => {
    setupMocks([buildPluginWithSettings()]);
    const w = mount(PluginsView);
    await flushPromises();

    expect(w.find('[data-testid="pv-settings-cfg"]').exists()).toBe(true);
    const boolInput = w.find('[data-testid="pv-setting-cfg-enabled"]')
      .element as HTMLInputElement;
    expect(boolInput.type).toBe('checkbox');
    expect(boolInput.checked).toBe(true);

    const numInput = w.find('[data-testid="pv-setting-cfg-maxRetries"]')
      .element as HTMLInputElement;
    expect(numInput.type).toBe('number');
    expect(numInput.value).toBe('3');

    const strInput = w.find('[data-testid="pv-setting-cfg-apiKey"]')
      .element as HTMLInputElement;
    expect(strInput.type).toBe('text');

    const enumSelect = w.find('[data-testid="pv-setting-cfg-mode"]')
      .element as HTMLSelectElement;
    expect(enumSelect.tagName).toBe('SELECT');
    expect(enumSelect.value).toBe('fast');
    expect(enumSelect.options.length).toBe(2);
  });

  it('persists boolean toggle via plugin_set_setting', async () => {
    setupMocks([buildPluginWithSettings()]);
    const w = mount(PluginsView);
    await flushPromises();

    const cb = w.find('[data-testid="pv-setting-cfg-enabled"]');
    (cb.element as HTMLInputElement).checked = false;
    await cb.trigger('change');
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith('plugin_set_setting', {
      key: 'cfg.enabled',
      value: false,
    });
  });

  it('persists number input as a Number, not a string', async () => {
    setupMocks([buildPluginWithSettings()]);
    const w = mount(PluginsView);
    await flushPromises();

    const num = w.find('[data-testid="pv-setting-cfg-maxRetries"]');
    (num.element as HTMLInputElement).value = '7';
    await num.trigger('change');
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith('plugin_set_setting', {
      key: 'cfg.maxRetries',
      value: 7,
    });
  });

  it('persists enum value via select change', async () => {
    setupMocks([buildPluginWithSettings()]);
    const w = mount(PluginsView);
    await flushPromises();

    const sel = w.find('[data-testid="pv-setting-cfg-mode"]');
    (sel.element as HTMLSelectElement).value = 'slow';
    await sel.trigger('change');
    await flushPromises();

    expect(mockInvoke).toHaveBeenCalledWith('plugin_set_setting', {
      key: 'cfg.mode',
      value: 'slow',
    });
  });

  it('reads existing setting values from backend instead of default', async () => {
    const { settingsStore } = setupMocks([buildPluginWithSettings()]);
    settingsStore.set('cfg.maxRetries', 99);

    const w = mount(PluginsView);
    await flushPromises();

    const num = w.find('[data-testid="pv-setting-cfg-maxRetries"]')
      .element as HTMLInputElement;
    expect(num.value).toBe('99');
  });

  it('surfaces backend errors per setting key', async () => {
    setupMocks([buildPluginWithSettings()]);
    const w = mount(PluginsView);
    await flushPromises();

    // Make the next plugin_set_setting reject.
    mockInvoke.mockImplementationOnce(async () => {
      throw new Error('write denied');
    });

    const cb = w.find('[data-testid="pv-setting-cfg-enabled"]');
    (cb.element as HTMLInputElement).checked = false;
    await cb.trigger('change');
    await flushPromises();

    expect(
      w.find('[data-testid="pv-setting-error-cfg-enabled"]').text(),
    ).toContain('write denied');
  });

  it('does not render a settings section when plugin contributes none', async () => {
    setupMocks([buildPlugin()]); // plain plugin, no settings
    const w = mount(PluginsView);
    await flushPromises();

    expect(w.find('[data-testid="pv-settings-sample-plugin"]').exists()).toBe(false);
  });
});
