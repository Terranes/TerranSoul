import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export type PluginSettingValueType =
  | string
  | { enum?: { values: string[] }; Enum?: { values: string[] } }

/**
 * Plugin manifest as returned by the Rust backend.
 * Mirrors `plugins::manifest::PluginManifest`.
 */
export interface PluginManifest {
  id: string
  display_name: string
  version: string
  description: string
  kind: string
  install_method: string
  capabilities: string[]
  activation_events: unknown[]
  contributes: {
    commands?: { id: string; title: string; icon?: string; keybinding?: string; category?: string }[]
    views?: { id: string; label: string; location: string; icon?: string }[]
    settings?: { key: string; label: string; description: string; default_value: unknown; value_type: PluginSettingValueType }[]
    themes?: { id: string; label: string; tokens: Record<string, string> }[]
    slash_commands?: { name: string; description: string; command_id: string }[]
    memory_hooks?: { id: string; stage: string; description: string }[]
  }
  system_requirements?: unknown
  api_version: number
  homepage?: string
  license?: string
  author?: string
  icon?: string
  publisher?: string
  signature?: string
  sha256?: string
  dependencies: { plugin_id: string; version_req: string }[]
}

export interface InstalledPlugin {
  manifest: PluginManifest
  state: string | { Error: { message: string } }
  installed_at: number
  last_active_at: number | null
}

export interface CommandEntry {
  plugin_id: string
  command: { id: string; title: string; icon?: string; keybinding?: string; category?: string }
}

export interface SlashCommandEntry {
  plugin_id: string
  slash_command: { name: string; description: string; command_id: string }
}

export interface PluginHostStatus {
  total_plugins: number
  active_plugins: number
  disabled_plugins: number
  error_plugins: number
  total_commands: number
  total_slash_commands: number
  total_themes: number
}

export interface ConsentInfo {
  agent_name: string
  capability: string
  granted: boolean
}

export interface CommandResult {
  success: boolean
  output: string | null
  error: string | null
  exit_code?: number | null
  stderr?: string | null
}

export const usePluginStore = defineStore('plugins', () => {
  const plugins = ref<InstalledPlugin[]>([])
  const commands = ref<CommandEntry[]>([])
  const slashCommands = ref<SlashCommandEntry[]>([])
  const themes = ref<{ id: string; label: string; tokens: Record<string, string> }[]>([])
  const status = ref<PluginHostStatus | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const activePlugins = computed(() =>
    // Backend serializes `PluginState` with `rename_all = "snake_case"` →
    // 'installed' | 'active' | 'disabled'.
    plugins.value.filter((p) => typeof p.state === 'string' && p.state.toLowerCase() === 'active'),
  )

  async function refresh() {
    loading.value = true
    error.value = null
    try {
      const [p, c, sc, th, st] = await Promise.all([
        invoke<InstalledPlugin[]>('plugin_list'),
        invoke<CommandEntry[]>('plugin_list_commands'),
        invoke<SlashCommandEntry[]>('plugin_list_slash_commands'),
        invoke<{ id: string; label: string; tokens: Record<string, string> }[]>('plugin_list_themes'),
        invoke<PluginHostStatus>('plugin_host_status'),
      ])
      plugins.value = Array.isArray(p) ? p : []
      commands.value = Array.isArray(c) ? c : []
      slashCommands.value = Array.isArray(sc) ? sc : []
      themes.value = Array.isArray(th) ? th : []
      status.value = st
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function install(manifestJson: string) {
    const installed = await invoke<InstalledPlugin>('plugin_install', { json: manifestJson })
    await refresh()
    return installed
  }

  async function activate(pluginId: string) {
    await invoke<void>('plugin_activate', { pluginId })
    await refresh()
  }

  async function deactivate(pluginId: string) {
    await invoke<void>('plugin_deactivate', { pluginId })
    await refresh()
  }

  async function uninstall(pluginId: string) {
    await invoke<void>('plugin_uninstall', { pluginId })
    await refresh()
  }

  async function getSetting(key: string) {
    return invoke<unknown | null>('plugin_get_setting', { key })
  }

  async function setSetting(key: string, value: unknown) {
    await invoke<void>('plugin_set_setting', { key, value })
  }

  async function parseManifest(json: string) {
    return invoke<PluginManifest>('plugin_parse_manifest', { json })
  }

  async function listPluginCapabilities(pluginId: string) {
    return invoke<ConsentInfo[]>('list_agent_capabilities', { agentName: pluginId })
  }

  async function grantPluginCapability(pluginId: string, capability: string) {
    await invoke<void>('grant_agent_capability', { agentName: pluginId, capability })
  }

  async function revokePluginCapability(pluginId: string, capability: string) {
    await invoke<void>('revoke_agent_capability', { agentName: pluginId, capability })
  }

  /** Invoke a contributed command by its `command_id`. */
  async function invokeCommand(commandId: string, args?: unknown) {
    return invoke<CommandResult>(
      'plugin_invoke_command',
      { commandId, args: args ?? null },
    )
  }

  /** Invoke a slash-command by name (without `/`). */
  async function invokeSlashCommand(name: string, args?: unknown) {
    return invoke<CommandResult>(
      'plugin_invoke_slash_command',
      { name, args: args ?? null },
    )
  }

  return {
    plugins,
    commands,
    slashCommands,
    themes,
    status,
    loading,
    error,
    activePlugins,
    refresh,
    install,
    activate,
    deactivate,
    uninstall,
    getSetting,
    setSetting,
    listPluginCapabilities,
    grantPluginCapability,
    revokePluginCapability,
    parseManifest,
    invokeCommand,
    invokeSlashCommand,
  }
})
