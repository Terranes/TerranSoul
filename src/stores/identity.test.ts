/**
 * Integration tests for the identity store.
 * Mocks @tauri-apps/api/core invoke() to simulate Tauri IPC without the Rust backend.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useIdentityStore } from './identity';
import type { DeviceInfo, TrustedDevice } from '../types';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const sampleDevice: DeviceInfo = {
  device_id: 'test-uuid-1234',
  public_key_b64: 'dGVzdA==',
  name: 'test-machine',
};

const sampleTrusted: TrustedDevice = {
  device_id: 'peer-uuid-5678',
  name: 'peer-device',
  public_key_b64: 'cGVlcg==',
  paired_at: 1000,
};

describe('identity store — IPC integration', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('initial state: deviceInfo is null, no devices, no error', () => {
    const store = useIdentityStore();
    expect(store.deviceInfo).toBeNull();
    expect(store.qrSvg).toBeNull();
    expect(store.trustedDevices).toHaveLength(0);
    expect(store.error).toBeNull();
    expect(store.isLoading).toBe(false);
  });

  it('loadIdentity sets deviceInfo on success', async () => {
    mockInvoke.mockResolvedValueOnce(sampleDevice);
    const store = useIdentityStore();
    await store.loadIdentity();
    expect(mockInvoke).toHaveBeenCalledWith('get_device_identity');
    expect(store.deviceInfo).toEqual(sampleDevice);
    expect(store.error).toBeNull();
    expect(store.isLoading).toBe(false);
  });

  it('loadIdentity sets error on failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('not initialised'));
    const store = useIdentityStore();
    await store.loadIdentity();
    expect(store.deviceInfo).toBeNull();
    expect(store.error).toContain('not initialised');
    expect(store.isLoading).toBe(false);
  });

  it('loadPairingQr sets qrSvg on success', async () => {
    const svgData = '<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>';
    mockInvoke.mockResolvedValueOnce(svgData);
    const store = useIdentityStore();
    await store.loadPairingQr();
    expect(mockInvoke).toHaveBeenCalledWith('get_pairing_qr');
    expect(store.qrSvg).toBe(svgData);
  });

  it('loadTrustedDevices populates trustedDevices', async () => {
    mockInvoke.mockResolvedValueOnce([sampleTrusted]);
    const store = useIdentityStore();
    await store.loadTrustedDevices();
    expect(mockInvoke).toHaveBeenCalledWith('list_trusted_devices');
    expect(store.trustedDevices).toHaveLength(1);
    expect(store.trustedDevices[0]).toEqual(sampleTrusted);
  });

  it('addTrustedDevice calls invoke and reloads trusted devices', async () => {
    mockInvoke
      .mockResolvedValueOnce(undefined) // add_trusted_device_cmd
      .mockResolvedValueOnce([sampleTrusted]); // list_trusted_devices
    const store = useIdentityStore();
    await store.addTrustedDevice(sampleTrusted);
    expect(mockInvoke).toHaveBeenCalledWith('add_trusted_device_cmd', { device: sampleTrusted });
    expect(store.trustedDevices).toHaveLength(1);
  });

  it('removeTrustedDevice calls invoke and reloads trusted devices', async () => {
    mockInvoke
      .mockResolvedValueOnce(undefined) // remove_trusted_device_cmd
      .mockResolvedValueOnce([]); // list_trusted_devices
    const store = useIdentityStore();
    await store.removeTrustedDevice('peer-uuid-5678');
    expect(mockInvoke).toHaveBeenCalledWith('remove_trusted_device_cmd', {
      deviceId: 'peer-uuid-5678',
    });
    expect(store.trustedDevices).toHaveLength(0);
  });

  it('clearError resets error to null', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('oops'));
    const store = useIdentityStore();
    await store.loadIdentity();
    expect(store.error).not.toBeNull();
    store.clearError();
    expect(store.error).toBeNull();
  });

  it('isLoading is true during loadIdentity and false after', async () => {
    let resolve!: (value: DeviceInfo) => void;
    const pending = new Promise<DeviceInfo>((r) => { resolve = r; });
    mockInvoke.mockReturnValueOnce(pending);

    const store = useIdentityStore();
    const loadPromise = store.loadIdentity();
    expect(store.isLoading).toBe(true);

    resolve(sampleDevice);
    await loadPromise;
    expect(store.isLoading).toBe(false);
  });
});
