export interface PairingCredentialBundle {
  deviceId: string;
  displayName?: string;
  capabilities?: string[];
  desktopHost?: string;
  desktopPort?: number;
  desktopFingerprintB64?: string;
  pairingTokenB64?: string;
  clientCertPem: string;
  clientKeyPem: string;
  caCertPem: string;
  pairedAt?: number;
}

export interface SecurePairingRecord {
  schemaVersion: 1;
  credentials: PairingCredentialBundle;
  savedAt: number;
}

export interface SecurePairingStore {
  save(credentials: PairingCredentialBundle): Promise<SecurePairingRecord>;
  load(): Promise<SecurePairingRecord | null>;
  remove(): Promise<void>;
}

export interface StrongholdRecordStore {
  insert(key: string, value: number[]): Promise<void>;
  get(key: string): Promise<number[] | Uint8Array | null | undefined>;
  remove(key: string): Promise<unknown>;
}

export interface StrongholdClientLike {
  getStore(): StrongholdRecordStore;
}

export interface StrongholdVaultLike {
  loadClient(name: string): Promise<StrongholdClientLike>;
  createClient(name: string): Promise<StrongholdClientLike>;
  save(): Promise<void>;
}

export interface StrongholdLoader {
  load(vaultPath: string, vaultPassword: string): Promise<StrongholdVaultLike>;
}

export interface SecurePairingStoreOptions {
  vaultPassword: string;
  vaultPath?: string;
  clientName?: string;
  recordKey?: string;
  now?: () => number;
  loader?: StrongholdLoader;
  appDataDir?: () => Promise<string>;
}

const DEFAULT_CLIENT_NAME = 'terransoul-pairing';
const DEFAULT_RECORD_KEY = 'mobile.pairing.credentials';
const DEFAULT_VAULT_FILE = 'terransoul-pairing.hold';

export function isTauriRuntimeAvailable(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function createSecurePairingStore(
  options: SecurePairingStoreOptions,
): Promise<SecurePairingStore> {
  const vaultPassword = options.vaultPassword.trim();
  if (!vaultPassword) {
    throw new Error('A Stronghold vault password is required for pairing storage.');
  }
  if (!options.loader && !isTauriRuntimeAvailable()) {
    throw new Error('Secure pairing storage requires the Tauri Stronghold runtime.');
  }

  const vaultPath = await resolveVaultPath(options);
  const loader = options.loader ?? defaultStrongholdLoader;
  const vault = await loader.load(vaultPath, vaultPassword);
  const client = await loadOrCreateClient(vault, options.clientName ?? DEFAULT_CLIENT_NAME);
  return new StrongholdSecurePairingStore(
    vault,
    client.getStore(),
    options.recordKey ?? DEFAULT_RECORD_KEY,
    options.now ?? Date.now,
  );
}

class StrongholdSecurePairingStore implements SecurePairingStore {
  constructor(
    private readonly vault: StrongholdVaultLike,
    private readonly store: StrongholdRecordStore,
    private readonly recordKey: string,
    private readonly now: () => number,
  ) {}

  async save(credentials: PairingCredentialBundle): Promise<SecurePairingRecord> {
    const record: SecurePairingRecord = {
      schemaVersion: 1,
      credentials,
      savedAt: this.now(),
    };
    await this.store.insert(this.recordKey, encodeJson(record));
    await this.vault.save();
    return record;
  }

  async load(): Promise<SecurePairingRecord | null> {
    const bytes = await this.store.get(this.recordKey);
    if (!bytes || bytes.length === 0) return null;
    const record = decodeJson<SecurePairingRecord>(bytes);
    if (record.schemaVersion !== 1) {
      throw new Error(`Unsupported pairing record schema: ${record.schemaVersion}`);
    }
    return record;
  }

  async remove(): Promise<void> {
    await this.store.remove(this.recordKey);
    await this.vault.save();
  }
}

async function loadOrCreateClient(
  vault: StrongholdVaultLike,
  clientName: string,
): Promise<StrongholdClientLike> {
  try {
    return await vault.loadClient(clientName);
  } catch {
    return vault.createClient(clientName);
  }
}

async function resolveVaultPath(options: SecurePairingStoreOptions): Promise<string> {
  if (options.vaultPath) return options.vaultPath;
  const appDataDir = options.appDataDir ?? defaultAppDataDir;
  return joinPath(await appDataDir(), DEFAULT_VAULT_FILE);
}

async function defaultAppDataDir(): Promise<string> {
  const pathApi = await import('@tauri-apps/api/path');
  return pathApi.appDataDir();
}

const defaultStrongholdLoader: StrongholdLoader = {
  async load(vaultPath: string, vaultPassword: string): Promise<StrongholdVaultLike> {
    const strongholdApi = await import('@tauri-apps/plugin-stronghold');
    const vault = await strongholdApi.Stronghold.load(vaultPath, vaultPassword);
    return vault as unknown as StrongholdVaultLike;
  },
};

function joinPath(base: string, fileName: string): string {
  if (!base) return fileName;
  if (base.endsWith('/') || base.endsWith('\\')) return `${base}${fileName}`;
  return `${base}/${fileName}`;
}

function encodeJson(value: unknown): number[] {
  return Array.from(new TextEncoder().encode(JSON.stringify(value)));
}

function decodeJson<T>(bytes: number[] | Uint8Array): T {
  const data = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  return JSON.parse(new TextDecoder().decode(data)) as T;
}