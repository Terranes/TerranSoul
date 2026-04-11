/**
 * Integration tests for the package store.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { usePackageStore } from './package';
import type { ManifestInfo } from '../types';

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

describe('package store — IPC integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('initial state: no manifest, no error, not loading', () => {
    const store = usePackageStore();
    expect(store.currentManifest).toBeNull();
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
