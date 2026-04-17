/**
 * Integration tests for the link store.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC without the Rust backend.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useLinkStore } from './link';
import type { LinkStatusResponse } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const sampleStatus: LinkStatusResponse = {
  status: 'connected',
  transport: 'Quic',
  peer: { device_id: 'peer-1', name: 'Phone', addr: '192.168.1.5:4433' },
  server_port: 4433,
};

describe('link store — IPC integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('initial state: disconnected, no peer, no error', () => {
    const store = useLinkStore();
    expect(store.status).toBe('disconnected');
    expect(store.peer).toBeNull();
    expect(store.serverPort).toBeNull();
    expect(store.error).toBeNull();
    expect(store.isLoading).toBe(false);
    expect(store.isConnected).toBe(false);
  });

  it('fetchStatus updates all fields on success', async () => {
    mockInvoke.mockResolvedValueOnce(sampleStatus);
    const store = useLinkStore();
    await store.fetchStatus();
    expect(mockInvoke).toHaveBeenCalledWith('get_link_status');
    expect(store.status).toBe('connected');
    expect(store.transport).toBe('Quic');
    expect(store.peer).toEqual(sampleStatus.peer);
    expect(store.serverPort).toBe(4433);
    expect(store.isConnected).toBe(true);
  });

  it('fetchStatus sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('network error'));
    const store = useLinkStore();
    await store.fetchStatus();
    expect(store.error).toContain('network error');
  });

  it('startServer sets serverPort and status to connecting', async () => {
    mockInvoke.mockResolvedValueOnce(9999);
    const store = useLinkStore();
    await store.startServer(0);
    expect(mockInvoke).toHaveBeenCalledWith('start_link_server', { port: 0 });
    expect(store.serverPort).toBe(9999);
    expect(store.status).toBe('connecting');
    expect(store.isLoading).toBe(false);
  });

  it('startServer sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('bind failed'));
    const store = useLinkStore();
    await store.startServer(0);
    expect(store.error).toContain('bind failed');
    expect(store.isLoading).toBe(false);
  });

  it('connectToPeer sets peer and status to connected', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    const store = useLinkStore();
    await store.connectToPeer('192.168.1.5', 4433, 'peer-1', 'Phone');
    expect(mockInvoke).toHaveBeenCalledWith('connect_to_peer', {
      host: '192.168.1.5',
      port: 4433,
      deviceId: 'peer-1',
      name: 'Phone',
    });
    expect(store.status).toBe('connected');
    expect(store.peer).toEqual({
      device_id: 'peer-1',
      name: 'Phone',
      addr: '192.168.1.5:4433',
    });
    expect(store.isConnected).toBe(true);
  });

  it('connectToPeer sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('connection refused'));
    const store = useLinkStore();
    await store.connectToPeer('10.0.0.1', 4433, 'p-2', 'Tablet');
    expect(store.error).toContain('connection refused');
    expect(store.isConnected).toBe(false);
  });

  it('disconnect resets state', async () => {
    mockInvoke.mockResolvedValueOnce(undefined); // connect
    mockInvoke.mockResolvedValueOnce(undefined); // disconnect
    const store = useLinkStore();
    await store.connectToPeer('1.2.3.4', 4433, 'x', 'X');
    await store.disconnect();
    expect(mockInvoke).toHaveBeenCalledWith('disconnect_link');
    expect(store.status).toBe('disconnected');
    expect(store.peer).toBeNull();
    expect(store.serverPort).toBeNull();
    expect(store.isConnected).toBe(false);
  });

  it('clearError resets error to null', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('whoops'));
    const store = useLinkStore();
    await store.fetchStatus();
    expect(store.error).not.toBeNull();
    store.clearError();
    expect(store.error).toBeNull();
  });

  it('isLoading is true during startServer and false after', async () => {
    let resolve!: (value: number) => void;
    const pending = new Promise<number>((r) => { resolve = r; });
    mockInvoke.mockReturnValueOnce(pending);

    const store = useLinkStore();
    const promise = store.startServer(0);
    expect(store.isLoading).toBe(true);

    resolve(8888);
    await promise;
    expect(store.isLoading).toBe(false);
  });

  it('startServer with no port passes null', async () => {
    mockInvoke.mockResolvedValueOnce(5555);
    const store = useLinkStore();
    await store.startServer();
    expect(mockInvoke).toHaveBeenCalledWith('start_link_server', { port: null });
  });
});

// ── IPC Contract Tests ─────────────────────────────────────────────────────

describe('link store — IPC contract', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('connectToPeer sends deviceId (camelCase)', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const store = useLinkStore();
    await store.connectToPeer('192.168.1.1', 8080, 'dev-abc', 'MyPhone');
    expect(mockInvoke).toHaveBeenCalledWith('connect_to_peer', {
      host: '192.168.1.1',
      port: 8080,
      deviceId: 'dev-abc',
      name: 'MyPhone',
    });
  });
});
