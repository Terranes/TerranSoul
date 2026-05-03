import { beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import {
  configureMobilePairingAdapters,
  resetMobilePairingAdapters,
  useMobilePairingStore,
  type PairedDeviceRecord,
} from './mobile-pairing';
import type { SecurePairingRecord, SecurePairingStore } from '../utils/secure-pairing-store';

const futureUri = 'terransoul://pair?host=192.168.1.42&port=7422&token=tok_123&fp=fp_new&exp=9999999999999';

function makeSecureStore(initial: SecurePairingRecord | null = null) {
  let record = initial;
  const saved: SecurePairingRecord[] = [];
  const secureStore: SecurePairingStore = {
    async save(credentials) {
      record = { schemaVersion: 1, credentials, savedAt: 2222 };
      saved.push(record);
      return record;
    },
    async load() {
      return record;
    },
    async remove() {
      record = null;
    },
  };
  return { secureStore, saved };
}

function pairedDevice(): PairedDeviceRecord {
  return {
    device_id: 'ios-device-1',
    display_name: 'Ari iPhone',
    cert_fingerprint: 'client-fp',
    capabilities: [],
    paired_at: 1234,
    last_seen_at: null,
  };
}

describe('mobile pairing store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    resetMobilePairingAdapters();
  });

  it('scans a QR URI and saves confirmed credentials to secure storage', async () => {
    const fake = makeSecureStore();
    configureMobilePairingAdapters({
      scanQr: async () => futureUri,
      now: () => 1000,
      createStore: async () => fake.secureStore,
      getDeviceInfo: async () => ({ device_id: 'ios-device-1', name: 'Ari iPhone', public_key_b64: 'pub' }),
      confirmPairing: async () => ({
        device: pairedDevice(),
        client_cert_pem: 'client-cert',
        client_key_pem: 'client-key',
        ca_cert_pem: 'ca-cert',
      }),
      listPairedDevices: async () => [pairedDevice()],
    });

    const store = useMobilePairingStore();
    await store.scanQr();
    const saved = await store.confirm('vault-secret');

    expect(store.endpoint).toBe('192.168.1.42:7422');
    expect(saved.credentials.desktopHost).toBe('192.168.1.42');
    expect(saved.credentials.pairingTokenB64).toBe('tok_123');
    expect(fake.saved[0].credentials.clientCertPem).toBe('client-cert');
    expect(store.pairedDevices).toHaveLength(1);
    expect(store.status).toBe('paired');
  });

  it('blocks re-pairing when the desktop fingerprint changes until trusted', async () => {
    const fake = makeSecureStore({
      schemaVersion: 1,
      savedAt: 1,
      credentials: {
        deviceId: 'ios-device-1',
        clientCertPem: 'old-cert',
        clientKeyPem: 'old-key',
        caCertPem: 'old-ca',
        desktopFingerprintB64: 'fp_old',
      },
    });
    configureMobilePairingAdapters({
      now: () => 1000,
      createStore: async () => fake.secureStore,
      getDeviceInfo: async () => ({ device_id: 'ios-device-1', name: 'Ari iPhone', public_key_b64: 'pub' }),
      confirmPairing: async () => ({
        device: pairedDevice(),
        client_cert_pem: 'new-cert',
        client_key_pem: 'new-key',
        ca_cert_pem: 'new-ca',
      }),
      listPairedDevices: async () => [],
    });

    const store = useMobilePairingStore();
    await store.loadStoredPairing('vault-secret');
    store.setManualUri(futureUri);

    await expect(store.confirm('vault-secret')).rejects.toThrow('fingerprint changed');
    expect(store.status).toBe('fingerprint_mismatch');

    const saved = await store.confirm('vault-secret', { trustNewFingerprint: true });
    expect(saved.credentials.clientCertPem).toBe('new-cert');
  });
});