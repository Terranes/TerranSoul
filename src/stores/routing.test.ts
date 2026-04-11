/**
 * Integration tests for the routing store.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useRoutingStore } from './routing';
import type { PendingCommand, CommandResultResponse } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const samplePending: PendingCommand = {
  command_id: 'cmd-1',
  origin_device: 'phone',
  command_type: 'send_message',
  payload: { text: 'hello' },
};

const sampleResult: CommandResultResponse = {
  command_id: 'cmd-1',
  status: 'completed',
  payload: { queued: true },
};

describe('routing store — IPC integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('initial state: no pending, no result, no error', () => {
    const store = useRoutingStore();
    expect(store.pendingCommands).toHaveLength(0);
    expect(store.lastResult).toBeNull();
    expect(store.error).toBeNull();
    expect(store.isLoading).toBe(false);
    expect(store.hasPending).toBe(false);
  });

  it('fetchPendingCommands populates list', async () => {
    mockInvoke.mockResolvedValueOnce([samplePending]);
    const store = useRoutingStore();
    await store.fetchPendingCommands();
    expect(mockInvoke).toHaveBeenCalledWith('list_pending_commands');
    expect(store.pendingCommands).toHaveLength(1);
    expect(store.pendingCommands[0]).toEqual(samplePending);
    expect(store.hasPending).toBe(true);
  });

  it('fetchPendingCommands sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('fetch failed'));
    const store = useRoutingStore();
    await store.fetchPendingCommands();
    expect(store.error).toContain('fetch failed');
  });

  it('approveCommand sets lastResult and refreshes pending', async () => {
    mockInvoke
      .mockResolvedValueOnce(sampleResult) // approve_remote_command
      .mockResolvedValueOnce([]); // list_pending_commands (refresh)
    const store = useRoutingStore();
    await store.approveCommand('cmd-1', true);
    expect(mockInvoke).toHaveBeenCalledWith('approve_remote_command', {
      commandId: 'cmd-1',
      remember: true,
    });
    expect(store.lastResult).toEqual(sampleResult);
    expect(store.pendingCommands).toHaveLength(0);
    expect(store.isLoading).toBe(false);
  });

  it('approveCommand sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('not found'));
    const store = useRoutingStore();
    await store.approveCommand('bad-id');
    expect(store.error).toContain('not found');
    expect(store.isLoading).toBe(false);
  });

  it('denyCommand sets lastResult and refreshes pending', async () => {
    const deniedResult: CommandResultResponse = {
      command_id: 'cmd-2',
      status: 'denied',
      payload: { reason: 'user denied' },
    };
    mockInvoke
      .mockResolvedValueOnce(deniedResult)
      .mockResolvedValueOnce([]);
    const store = useRoutingStore();
    await store.denyCommand('cmd-2', true);
    expect(mockInvoke).toHaveBeenCalledWith('deny_remote_command', {
      commandId: 'cmd-2',
      block: true,
    });
    expect(store.lastResult).toEqual(deniedResult);
  });

  it('setDevicePermission calls invoke with correct args', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    const store = useRoutingStore();
    await store.setDevicePermission('phone', 'allow');
    expect(mockInvoke).toHaveBeenCalledWith('set_device_permission', {
      deviceId: 'phone',
      policy: 'allow',
    });
    expect(store.error).toBeNull();
  });

  it('getDevicePermissions returns policy list', async () => {
    mockInvoke.mockResolvedValueOnce([['phone', 'allow'], ['tablet', 'deny']]);
    const store = useRoutingStore();
    const policies = await store.getDevicePermissions();
    expect(mockInvoke).toHaveBeenCalledWith('get_device_permissions');
    expect(policies).toHaveLength(2);
    expect(policies[0]).toEqual(['phone', 'allow']);
  });

  it('getDevicePermissions returns empty array on error', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('oops'));
    const store = useRoutingStore();
    const policies = await store.getDevicePermissions();
    expect(policies).toHaveLength(0);
    expect(store.error).toContain('oops');
  });

  it('clearError resets error to null', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('boom'));
    const store = useRoutingStore();
    await store.fetchPendingCommands();
    expect(store.error).not.toBeNull();
    store.clearError();
    expect(store.error).toBeNull();
  });
});
