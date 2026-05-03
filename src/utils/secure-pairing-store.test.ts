import { afterEach, describe, expect, it } from 'vitest';
import {
  createSecurePairingStore,
  isTauriRuntimeAvailable,
  type PairingCredentialBundle,
  type StrongholdClientLike,
  type StrongholdLoader,
  type StrongholdRecordStore,
  type StrongholdVaultLike,
} from './secure-pairing-store';

const sampleCredentials: PairingCredentialBundle = {
  deviceId: 'ios-device-1',
  displayName: 'Ari iPhone',
  clientCertPem: '-----BEGIN CERTIFICATE-----\nclient\n-----END CERTIFICATE-----',
  clientKeyPem: '-----BEGIN PRIVATE KEY-----\nclient\n-----END PRIVATE KEY-----',
  caCertPem: '-----BEGIN CERTIFICATE-----\nca\n-----END CERTIFICATE-----',
  pairedAt: 42,
};

function makeStrongholdFake(options: { loadClientFails?: boolean } = {}) {
  const records = new Map<string, number[] | Uint8Array>();
  const calls = {
    loadPath: '',
    loadPassword: '',
    loadClient: 0,
    createClient: 0,
    save: 0,
  };

  const recordStore: StrongholdRecordStore = {
    async insert(key, value) {
      records.set(key, value);
    },
    async get(key) {
      return records.get(key) ?? null;
    },
    async remove(key) {
      records.delete(key);
    },
  };
  const client: StrongholdClientLike = { getStore: () => recordStore };
  const vault: StrongholdVaultLike = {
    async loadClient() {
      calls.loadClient += 1;
      if (options.loadClientFails) throw new Error('missing client');
      return client;
    },
    async createClient() {
      calls.createClient += 1;
      return client;
    },
    async save() {
      calls.save += 1;
    },
  };
  const loader: StrongholdLoader = {
    async load(vaultPath, vaultPassword) {
      calls.loadPath = vaultPath;
      calls.loadPassword = vaultPassword;
      return vault;
    },
  };
  return { loader, records, calls };
}

afterEach(() => {
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
});

describe('secure pairing store', () => {
  it('saves and loads pairing credentials through Stronghold', async () => {
    const fake = makeStrongholdFake({ loadClientFails: true });
    const store = await createSecurePairingStore({
      vaultPassword: 'pair-secret',
      loader: fake.loader,
      appDataDir: async () => '/tmp/terransoul',
      now: () => 1234,
    });

    const saved = await store.save(sampleCredentials);
    const loaded = await store.load();

    expect(saved.savedAt).toBe(1234);
    expect(loaded).toEqual(saved);
    expect(fake.calls.loadPath).toBe('/tmp/terransoul/terransoul-pairing.hold');
    expect(fake.calls.loadPassword).toBe('pair-secret');
    expect(fake.calls.loadClient).toBe(1);
    expect(fake.calls.createClient).toBe(1);
    expect(fake.calls.save).toBe(1);
  });

  it('uses an existing Stronghold client when available', async () => {
    const fake = makeStrongholdFake();
    await createSecurePairingStore({
      vaultPassword: 'pair-secret',
      loader: fake.loader,
      vaultPath: 'custom.hold',
    });

    expect(fake.calls.loadClient).toBe(1);
    expect(fake.calls.createClient).toBe(0);
    expect(fake.calls.loadPath).toBe('custom.hold');
  });

  it('removes stored pairing credentials and saves the vault', async () => {
    const fake = makeStrongholdFake();
    const store = await createSecurePairingStore({
      vaultPassword: 'pair-secret',
      loader: fake.loader,
      vaultPath: 'custom.hold',
    });

    await store.save(sampleCredentials);
    await store.remove();

    expect(await store.load()).toBeNull();
    expect(fake.calls.save).toBe(2);
  });

  it('rejects empty vault passwords', async () => {
    const fake = makeStrongholdFake();
    await expect(createSecurePairingStore({ vaultPassword: ' ', loader: fake.loader }))
      .rejects
      .toThrow('vault password');
  });

  it('detects Tauri runtime availability', () => {
    expect(isTauriRuntimeAvailable()).toBe(false);
    (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {};
    expect(isTauriRuntimeAvailable()).toBe(true);
  });
});