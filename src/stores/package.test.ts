/**
 * Integration tests for the package store.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { usePackageStore } from './package';
import type { AgentSearchResult, ManifestInfo, InstalledAgentInfo } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const sampleManifest: ManifestInfo = {
  name: 'stub-agent',
  version: '1.0.0',
  description: 'A stub agent for testing',
  capabilities: ['chat', 'network'],
  sensitive_capabilities: ['network'],
  install_type: 'binary',
  ipc_protocol_version: 1,
  author: 'TerranSoul Team',
  license: 'MIT',
  homepage: null,
};

const sampleInstalled: InstalledAgentInfo = {
  name: 'stub-agent',
  version: '1.0.0',
  description: 'A stub agent for testing',
  install_path: '/data/agents/stub-agent',
};

describe('package store — IPC integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('initial state: no manifest, no installed, no error, not loading', () => {
    const store = usePackageStore();
    expect(store.currentManifest).toBeNull();
    expect(store.installedAgents).toHaveLength(0);
    expect(store.error).toBeNull();
    expect(store.isLoading).toBe(false);
  });

  it('parseManifest sets currentManifest on success', async () => {
    mockInvoke.mockResolvedValueOnce(sampleManifest);
    const store = usePackageStore();
    const result = await store.parseManifest('{"valid":"json"}');
    expect(mockInvoke).toHaveBeenCalledWith('parse_agent_manifest', {
      json: '{"valid":"json"}',
    });
    expect(result).toEqual(sampleManifest);
    expect(store.currentManifest).toEqual(sampleManifest);
    expect(store.error).toBeNull();
    expect(store.isLoading).toBe(false);
  });

  it('parseManifest sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('manifest: invalid name'));
    const store = usePackageStore();
    const result = await store.parseManifest('{"bad":"json"}');
    expect(result).toBeNull();
    expect(store.currentManifest).toBeNull();
    expect(store.error).toContain('invalid name');
    expect(store.isLoading).toBe(false);
  });

  it('validateManifest returns true on success', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    const store = usePackageStore();
    const valid = await store.validateManifest('{"valid":"json"}');
    expect(mockInvoke).toHaveBeenCalledWith('validate_agent_manifest', {
      json: '{"valid":"json"}',
    });
    expect(valid).toBe(true);
    expect(store.error).toBeNull();
  });

  it('validateManifest returns false and sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('manifest: parse error'));
    const store = usePackageStore();
    const valid = await store.validateManifest('not json');
    expect(valid).toBe(false);
    expect(store.error).toContain('parse error');
  });

  it('getIpcProtocolRange returns the range', async () => {
    mockInvoke.mockResolvedValueOnce([1, 1]);
    const store = usePackageStore();
    const range = await store.getIpcProtocolRange();
    expect(mockInvoke).toHaveBeenCalledWith('get_ipc_protocol_range');
    expect(range).toEqual([1, 1]);
    expect(store.error).toBeNull();
  });

  it('getIpcProtocolRange returns null on error', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('backend error'));
    const store = usePackageStore();
    const range = await store.getIpcProtocolRange();
    expect(range).toBeNull();
    expect(store.error).toContain('backend error');
  });

  it('installAgent calls invoke and refreshes list', async () => {
    mockInvoke
      .mockResolvedValueOnce(sampleInstalled)       // install_agent
      .mockResolvedValueOnce([sampleInstalled]);     // list_installed_agents
    const store = usePackageStore();
    const result = await store.installAgent('stub-agent');
    expect(mockInvoke).toHaveBeenCalledWith('install_agent', { agentName: 'stub-agent' });
    expect(result).toEqual(sampleInstalled);
    expect(store.installedAgents).toHaveLength(1);
    expect(store.isLoading).toBe(false);
  });

  it('installAgent sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('already installed'));
    const store = usePackageStore();
    const result = await store.installAgent('stub-agent');
    expect(result).toBeNull();
    expect(store.error).toContain('already installed');
  });

  it('updateAgent calls invoke and refreshes list', async () => {
    const updated: InstalledAgentInfo = { ...sampleInstalled, version: '2.0.0' };
    mockInvoke
      .mockResolvedValueOnce(updated)               // update_agent
      .mockResolvedValueOnce([updated]);             // list_installed_agents
    const store = usePackageStore();
    const result = await store.updateAgent('stub-agent');
    expect(mockInvoke).toHaveBeenCalledWith('update_agent', { agentName: 'stub-agent' });
    expect(result).toEqual(updated);
  });

  it('updateAgent sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('not installed'));
    const store = usePackageStore();
    const result = await store.updateAgent('stub-agent');
    expect(result).toBeNull();
    expect(store.error).toContain('not installed');
  });

  it('removeAgent calls invoke and refreshes list', async () => {
    mockInvoke
      .mockResolvedValueOnce(undefined)   // remove_agent
      .mockResolvedValueOnce([]);         // list_installed_agents
    const store = usePackageStore();
    const result = await store.removeAgent('stub-agent');
    expect(mockInvoke).toHaveBeenCalledWith('remove_agent', { agentName: 'stub-agent' });
    expect(result).toBe(true);
    expect(store.installedAgents).toHaveLength(0);
  });

  it('removeAgent sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('not installed'));
    const store = usePackageStore();
    const result = await store.removeAgent('nonexistent');
    expect(result).toBe(false);
    expect(store.error).toContain('not installed');
  });

  it('fetchInstalledAgents populates list', async () => {
    mockInvoke.mockResolvedValueOnce([sampleInstalled]);
    const store = usePackageStore();
    await store.fetchInstalledAgents();
    expect(mockInvoke).toHaveBeenCalledWith('list_installed_agents');
    expect(store.installedAgents).toHaveLength(1);
    expect(store.installedAgents[0]).toEqual(sampleInstalled);
  });

  it('fetchInstalledAgents sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('backend error'));
    const store = usePackageStore();
    await store.fetchInstalledAgents();
    expect(store.error).toContain('backend error');
  });

  it('clearManifest resets manifest and error', async () => {
    mockInvoke.mockResolvedValueOnce(sampleManifest);
    const store = usePackageStore();
    await store.parseManifest('{}');
    expect(store.currentManifest).not.toBeNull();
    store.clearManifest();
    expect(store.currentManifest).toBeNull();
    expect(store.error).toBeNull();
  });

  it('clearError resets only error', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('boom'));
    const store = usePackageStore();
    await store.parseManifest('bad');
    expect(store.error).not.toBeNull();
    store.clearError();
    expect(store.error).toBeNull();
  });

  it('parseManifest clears previous error on new success', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('first error'));
    const store = usePackageStore();
    await store.parseManifest('bad');
    expect(store.error).not.toBeNull();

    mockInvoke.mockResolvedValueOnce(sampleManifest);
    await store.parseManifest('good');
    expect(store.error).toBeNull();
    expect(store.currentManifest).toEqual(sampleManifest);
  });
});

// ── Registry server tests ─────────────────────────────────────────────────────

const sampleSearchResult: AgentSearchResult = {
  name: 'stub-agent',
  version: '1.0.0',
  description: 'Built-in TerranSoul stub agent for testing',
  capabilities: ['chat'],
  homepage: 'https://terranes.dev/agents/stub',
};

describe('package store — registry server', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('initial state: registryPort null, searchResults empty', () => {
    const store = usePackageStore();
    expect(store.registryPort).toBeNull();
    expect(store.searchResults).toHaveLength(0);
  });

  it('startRegistryServer returns port and updates registryPort', async () => {
    mockInvoke.mockResolvedValueOnce(12345);
    const store = usePackageStore();
    const port = await store.startRegistryServer();
    expect(mockInvoke).toHaveBeenCalledWith('start_registry_server');
    expect(port).toBe(12345);
    expect(store.registryPort).toBe(12345);
    expect(store.error).toBeNull();
  });

  it('startRegistryServer sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('server error'));
    const store = usePackageStore();
    const port = await store.startRegistryServer();
    expect(port).toBeNull();
    expect(store.error).toContain('server error');
  });

  it('stopRegistryServer clears registryPort', async () => {
    mockInvoke.mockResolvedValueOnce(12345);
    const store = usePackageStore();
    await store.startRegistryServer();
    expect(store.registryPort).toBe(12345);

    mockInvoke.mockResolvedValueOnce(undefined);
    const result = await store.stopRegistryServer();
    expect(mockInvoke).toHaveBeenCalledWith('stop_registry_server');
    expect(result).toBe(true);
    expect(store.registryPort).toBeNull();
  });

  it('stopRegistryServer sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('stop error'));
    const store = usePackageStore();
    const result = await store.stopRegistryServer();
    expect(result).toBe(false);
    expect(store.error).toContain('stop error');
  });

  it('getRegistryServerPort returns null when not running', async () => {
    mockInvoke.mockResolvedValueOnce(null);
    const store = usePackageStore();
    const port = await store.getRegistryServerPort();
    expect(mockInvoke).toHaveBeenCalledWith('get_registry_server_port');
    expect(port).toBeNull();
    expect(store.registryPort).toBeNull();
  });

  it('getRegistryServerPort returns port when running', async () => {
    mockInvoke.mockResolvedValueOnce(8080);
    const store = usePackageStore();
    const port = await store.getRegistryServerPort();
    expect(port).toBe(8080);
    expect(store.registryPort).toBe(8080);
  });

  it('searchAgents returns results and updates searchResults', async () => {
    mockInvoke.mockResolvedValueOnce([sampleSearchResult]);
    const store = usePackageStore();
    const results = await store.searchAgents('stub');
    expect(mockInvoke).toHaveBeenCalledWith('search_agents', { query: 'stub' });
    expect(results).toHaveLength(1);
    expect(results[0]).toEqual(sampleSearchResult);
    expect(store.searchResults).toHaveLength(1);
    expect(store.isLoading).toBe(false);
  });

  it('searchAgents returns empty array and sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('search error'));
    const store = usePackageStore();
    const results = await store.searchAgents('foo');
    expect(results).toHaveLength(0);
    expect(store.error).toContain('search error');
    expect(store.isLoading).toBe(false);
  });
});
