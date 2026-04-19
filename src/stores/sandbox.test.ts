/**
 * Integration tests for the sandbox store.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useSandboxStore } from './sandbox';
import type { ConsentInfo } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const sampleConsents: ConsentInfo[] = [
  { agent_name: 'test-agent', capability: 'file_read', granted: true },
  { agent_name: 'test-agent', capability: 'network', granted: false },
];

describe('sandbox store — IPC integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('initial state: empty consents, no error, not loading', () => {
    const store = useSandboxStore();
    expect(store.consents).toHaveLength(0);
    expect(store.error).toBeNull();
    expect(store.isLoading).toBe(false);
  });

  it('grantCapability calls invoke and returns true on success', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    const store = useSandboxStore();
    const result = await store.grantCapability('test-agent', 'file_read');
    expect(mockInvoke).toHaveBeenCalledWith('grant_agent_capability', {
      agentName: 'test-agent',
      capability: 'file_read',
    });
    expect(result).toBe(true);
    expect(store.error).toBeNull();
  });

  it('grantCapability returns false and sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('unknown capability'));
    const store = useSandboxStore();
    const result = await store.grantCapability('test-agent', 'network');
    expect(result).toBe(false);
    expect(store.error).toContain('unknown capability');
  });

  it('revokeCapability calls invoke and returns true on success', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    const store = useSandboxStore();
    const result = await store.revokeCapability('test-agent', 'file_write');
    expect(mockInvoke).toHaveBeenCalledWith('revoke_agent_capability', {
      agentName: 'test-agent',
      capability: 'file_write',
    });
    expect(result).toBe(true);
  });

  it('revokeCapability returns false and sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('revoke error'));
    const store = useSandboxStore();
    const result = await store.revokeCapability('test-agent', 'clipboard');
    expect(result).toBe(false);
    expect(store.error).toContain('revoke error');
  });

  it('listCapabilities populates consents and returns records', async () => {
    mockInvoke.mockResolvedValueOnce(sampleConsents);
    const store = useSandboxStore();
    const records = await store.listCapabilities('test-agent');
    expect(mockInvoke).toHaveBeenCalledWith('list_agent_capabilities', {
      agentName: 'test-agent',
    });
    expect(records).toHaveLength(2);
    expect(store.consents).toHaveLength(2);
    expect(store.isLoading).toBe(false);
  });

  it('listCapabilities returns empty array and sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('backend error'));
    const store = useSandboxStore();
    const records = await store.listCapabilities('test-agent');
    expect(records).toHaveLength(0);
    expect(store.error).toContain('backend error');
    expect(store.isLoading).toBe(false);
  });

  it('clearCapabilities removes agent consents from store', async () => {
    mockInvoke.mockResolvedValueOnce(sampleConsents);
    const store = useSandboxStore();
    await store.listCapabilities('test-agent');
    expect(store.consents).toHaveLength(2);

    mockInvoke.mockResolvedValueOnce(undefined);
    const result = await store.clearCapabilities('test-agent');
    expect(mockInvoke).toHaveBeenCalledWith('clear_agent_capabilities', {
      agentName: 'test-agent',
    });
    expect(result).toBe(true);
    expect(store.consents.filter((c) => c.agent_name === 'test-agent')).toHaveLength(0);
  });

  it('clearCapabilities returns false and sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('clear error'));
    const store = useSandboxStore();
    const result = await store.clearCapabilities('test-agent');
    expect(result).toBe(false);
    expect(store.error).toContain('clear error');
  });

  it('runInSandbox returns result on success', async () => {
    mockInvoke.mockResolvedValueOnce(42);
    const store = useSandboxStore();
    const result = await store.runInSandbox('test-agent', [0x00, 0x61, 0x73, 0x6d]);
    expect(mockInvoke).toHaveBeenCalledWith('run_agent_in_sandbox', {
      agentName: 'test-agent',
      wasmBytes: [0x00, 0x61, 0x73, 0x6d],
    });
    expect(result).toBe(42);
    expect(store.isLoading).toBe(false);
  });

  it('runInSandbox returns null and sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('wasm error'));
    const store = useSandboxStore();
    const result = await store.runInSandbox('test-agent', []);
    expect(result).toBeNull();
    expect(store.error).toContain('wasm error');
    expect(store.isLoading).toBe(false);
  });

  it('clearError resets error', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('some error'));
    const store = useSandboxStore();
    await store.grantCapability('test-agent', 'network');
    expect(store.error).not.toBeNull();
    store.clearError();
    expect(store.error).toBeNull();
  });
});

// ── IPC Contract Tests ─────────────────────────────────────────────────────

describe('sandbox store — IPC contract', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('grantCapability sends agentName (camelCase)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useSandboxStore();
    await store.grantCapability('test-agent', 'network');
    expect(mockInvoke).toHaveBeenCalledWith('grant_agent_capability', { agentName: 'test-agent', capability: 'network' });
  });

  it('revokeCapability sends agentName (camelCase)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useSandboxStore();
    await store.revokeCapability('test-agent', 'file_read');
    expect(mockInvoke).toHaveBeenCalledWith('revoke_agent_capability', { agentName: 'test-agent', capability: 'file_read' });
  });

  it('listCapabilities sends agentName (camelCase)', async () => {
    mockInvoke.mockResolvedValue([]);
    const store = useSandboxStore();
    await store.listCapabilities('test-agent');
    expect(mockInvoke).toHaveBeenCalledWith('list_agent_capabilities', { agentName: 'test-agent' });
  });

  it('clearCapabilities sends agentName (camelCase)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useSandboxStore();
    await store.clearCapabilities('test-agent');
    expect(mockInvoke).toHaveBeenCalledWith('clear_agent_capabilities', { agentName: 'test-agent' });
  });

  it('runInSandbox sends agentName and wasmBytes (camelCase)', async () => {
    mockInvoke.mockResolvedValue(0);
    const store = useSandboxStore();
    await store.runInSandbox('test-agent', [0, 1, 2]);
    expect(mockInvoke).toHaveBeenCalledWith('run_agent_in_sandbox', { agentName: 'test-agent', wasmBytes: [0, 1, 2] });
  });
});
