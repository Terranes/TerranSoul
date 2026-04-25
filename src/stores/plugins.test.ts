import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { usePluginStore } from './plugins'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

import { invoke } from '@tauri-apps/api/core'
const mockInvoke = vi.mocked(invoke)

describe('usePluginStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts with empty state', () => {
    const store = usePluginStore()
    expect(store.plugins).toEqual([])
    expect(store.commands).toEqual([])
    expect(store.slashCommands).toEqual([])
    expect(store.themes).toEqual([])
    expect(store.status).toBeNull()
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
  })

  it('refresh fetches all data', async () => {
    const fakePlugins = [
      {
        manifest: { id: 'test.plugin', display_name: 'Test', version: '1.0.0', description: 'Desc', kind: 'Tool', install_method: 'BuiltIn', capabilities: [], activation_events: [], contributes: { commands: [], views: [], settings: [], themes: [], slash_commands: [], memory_hooks: [] }, api_version: 1, dependencies: [] },
        state: 'Active',
        installed_at: 1000,
        last_active_at: 2000,
      },
    ]
    const fakeCommands = [{ plugin_id: 'test.plugin', command: { id: 'test.plugin.run', title: 'Run' } }]
    const fakeSlash = [{ plugin_id: 'test.plugin', slash_command: { name: 'run', description: 'Run', command_id: 'test.plugin.run' } }]
    const fakeThemes = [{ id: 'test.plugin.dark', label: 'Dark', tokens: {} }]
    const fakeStatus = { total_plugins: 1, active_plugins: 1, disabled_plugins: 0, error_plugins: 0, total_commands: 1, total_slash_commands: 1, total_themes: 1 }

    mockInvoke
      .mockResolvedValueOnce(fakePlugins)
      .mockResolvedValueOnce(fakeCommands)
      .mockResolvedValueOnce(fakeSlash)
      .mockResolvedValueOnce(fakeThemes)
      .mockResolvedValueOnce(fakeStatus)

    const store = usePluginStore()
    await store.refresh()

    expect(store.plugins).toEqual(fakePlugins)
    expect(store.commands).toEqual(fakeCommands)
    expect(store.slashCommands).toEqual(fakeSlash)
    expect(store.themes).toEqual(fakeThemes)
    expect(store.status).toEqual(fakeStatus)
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
  })

  it('activePlugins filters correctly', async () => {
    const store = usePluginStore()
    store.plugins = [
      { manifest: { id: 'a' } as never, state: 'Active', installed_at: 0, last_active_at: null },
      { manifest: { id: 'b' } as never, state: 'Installed', installed_at: 0, last_active_at: null },
      { manifest: { id: 'c' } as never, state: 'Active', installed_at: 0, last_active_at: null },
    ]
    expect(store.activePlugins.length).toBe(2)
  })

  it('install calls invoke and refreshes', async () => {
    const installed = { manifest: { id: 'new' }, state: 'Installed', installed_at: 0, last_active_at: null }
    mockInvoke
      .mockResolvedValueOnce(installed) // plugin_install
      .mockResolvedValueOnce([installed]) // refresh: plugin_list
      .mockResolvedValueOnce([]) // refresh: plugin_list_commands
      .mockResolvedValueOnce([]) // refresh: plugin_list_slash_commands
      .mockResolvedValueOnce([]) // refresh: plugin_list_themes
      .mockResolvedValueOnce({ total_plugins: 1, active_plugins: 0, disabled_plugins: 0, error_plugins: 0, total_commands: 0, total_slash_commands: 0, total_themes: 0 }) // refresh: plugin_host_status

    const store = usePluginStore()
    const result = await store.install('{"id":"new"}')
    expect(result).toEqual(installed)
    expect(mockInvoke).toHaveBeenCalledWith('plugin_install', { json: '{"id":"new"}' })
  })

  it('activate calls invoke', async () => {
    mockInvoke
      .mockResolvedValueOnce(undefined) // plugin_activate
      .mockResolvedValueOnce([]) // refresh
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce({ total_plugins: 0, active_plugins: 0, disabled_plugins: 0, error_plugins: 0, total_commands: 0, total_slash_commands: 0, total_themes: 0 })

    const store = usePluginStore()
    await store.activate('my-plugin')
    expect(mockInvoke).toHaveBeenCalledWith('plugin_activate', { pluginId: 'my-plugin' })
  })

  it('deactivate calls invoke', async () => {
    mockInvoke
      .mockResolvedValueOnce(undefined) // plugin_deactivate
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce({ total_plugins: 0, active_plugins: 0, disabled_plugins: 0, error_plugins: 0, total_commands: 0, total_slash_commands: 0, total_themes: 0 })

    const store = usePluginStore()
    await store.deactivate('my-plugin')
    expect(mockInvoke).toHaveBeenCalledWith('plugin_deactivate', { pluginId: 'my-plugin' })
  })

  it('uninstall calls invoke', async () => {
    mockInvoke
      .mockResolvedValueOnce(undefined) // plugin_uninstall
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce({ total_plugins: 0, active_plugins: 0, disabled_plugins: 0, error_plugins: 0, total_commands: 0, total_slash_commands: 0, total_themes: 0 })

    const store = usePluginStore()
    await store.uninstall('my-plugin')
    expect(mockInvoke).toHaveBeenCalledWith('plugin_uninstall', { pluginId: 'my-plugin' })
  })

  it('getSetting and setSetting call invoke', async () => {
    mockInvoke.mockResolvedValueOnce(42)
    const store = usePluginStore()
    const val = await store.getSetting('my.key')
    expect(val).toBe(42)
    expect(mockInvoke).toHaveBeenCalledWith('plugin_get_setting', { key: 'my.key' })

    mockInvoke.mockResolvedValueOnce(undefined)
    await store.setSetting('my.key', 99)
    expect(mockInvoke).toHaveBeenCalledWith('plugin_set_setting', { key: 'my.key', value: 99 })
  })

  it('parseManifest calls invoke', async () => {
    const manifest = { id: 'test.plugin' }
    mockInvoke.mockResolvedValueOnce(manifest)
    const store = usePluginStore()
    const result = await store.parseManifest('{}')
    expect(result).toEqual(manifest)
    expect(mockInvoke).toHaveBeenCalledWith('plugin_parse_manifest', { json: '{}' })
  })

  it('refresh sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('network fail'))
    const store = usePluginStore()
    await store.refresh()
    expect(store.error).toContain('network fail')
    expect(store.loading).toBe(false)
  })
})
