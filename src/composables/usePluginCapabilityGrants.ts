import { ref, type ComputedRef, type Ref } from 'vue';
import type { ConsentInfo, InstalledPlugin } from '../stores/plugins';

interface PluginCapabilityStore {
  listPluginCapabilities(pluginId: string): Promise<ConsentInfo[]>;
  grantPluginCapability(pluginId: string, capability: string): Promise<void>;
  revokePluginCapability(pluginId: string, capability: string): Promise<void>;
}
const SENSITIVE_CAPABILITIES = new Set([
  'filesystem',
  'clipboard',
  'network',
  'remote_exec',
]);

const SANDBOX_CAPABILITIES_BY_MANIFEST_CAP: Record<string, string[]> = {
  filesystem: ['file_read', 'file_write'],
  clipboard: ['clipboard'],
  network: ['network'],
  remote_exec: ['process_spawn'],
};

export function capabilityRequiresConsent(capability: string): boolean {
  return SENSITIVE_CAPABILITIES.has(capability);
}

export function sandboxCapabilitiesForManifestCap(capability: string): string[] {
  return SANDBOX_CAPABILITIES_BY_MANIFEST_CAP[capability] ?? [];
}

export function usePluginCapabilityGrants(
  plugins: ComputedRef<InstalledPlugin[]>,
  store: PluginCapabilityStore,
  actionErrors: Ref<Record<string, string>>,
) {
  const grants = ref<Record<string, Record<string, boolean>>>({});

  function canActivate(plugin: InstalledPlugin): boolean {
    const pluginGrants = grants.value[plugin.manifest.id] ?? {};
    return plugin.manifest.capabilities.every((capability) => {
      if (!capabilityRequiresConsent(capability)) return true;
      return pluginGrants[capability] === true;
    });
  }

  async function loadGrantsForPlugin(plugin: InstalledPlugin) {
    const id = plugin.manifest.id;
    const next = { ...(grants.value[id] ?? {}) };
    try {
      const records = await store.listPluginCapabilities(id);
      const granted = new Map(records.map((record) => [record.capability, record.granted]));
      for (const capability of plugin.manifest.capabilities) {
        if (!capabilityRequiresConsent(capability)) continue;
        const sandboxCaps = sandboxCapabilitiesForManifestCap(capability);
        next[capability] = sandboxCaps.length > 0
          && sandboxCaps.every((sandboxCap) => granted.get(sandboxCap) === true);
      }
      grants.value[id] = next;
    } catch (error) {
      actionErrors.value[id] = String(error);
    }
  }

  async function loadAllGrants() {
    await Promise.all(plugins.value.map(loadGrantsForPlugin));
  }

  async function onGrantToggle(pluginId: string, capability: string, checked: boolean) {
    if (!grants.value[pluginId]) grants.value[pluginId] = {};
    const previous = grants.value[pluginId][capability];
    grants.value[pluginId][capability] = checked;
    delete actionErrors.value[pluginId];
    try {
      await Promise.all(
        sandboxCapabilitiesForManifestCap(capability).map((sandboxCap) => checked
          ? store.grantPluginCapability(pluginId, sandboxCap)
          : store.revokePluginCapability(pluginId, sandboxCap)),
      );
    } catch (error) {
      grants.value[pluginId][capability] = previous ?? false;
      actionErrors.value[pluginId] = String(error);
    }
  }

  return {
    grants,
    capabilityRequiresConsent,
    canActivate,
    loadAllGrants,
    onGrantToggle,
  };
}
	