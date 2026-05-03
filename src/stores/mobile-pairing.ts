import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { DeviceInfo } from '../types';
import { createLocalRemoteHost, type RemoteHost } from '../transport';
import {
  createSecurePairingStore,
  type PairingCredentialBundle,
  type SecurePairingRecord,
  type SecurePairingStore,
  type SecurePairingStoreOptions,
} from '../utils/secure-pairing-store';
import {
  formatPairingEndpoint,
  pairingFingerprintMismatch,
  parsePairingUri,
  scanPairingQrCode,
  type MobilePairPayload,
} from '../utils/mobile-pairing';

export interface PairedDeviceRecord {
  device_id: string;
  display_name: string;
  cert_fingerprint: string;
  capabilities: string[];
  paired_at: number;
  last_seen_at: number | null;
}

export interface PairingResultResponse {
  device: PairedDeviceRecord;
  client_cert_pem: string;
  client_key_pem: string;
  ca_cert_pem: string;
}

export interface ConfirmPairingInput {
  payload: MobilePairPayload;
  deviceId: string;
  displayName: string;
}

export interface MobilePairingAdapters {
  scanQr: () => Promise<string>;
  getDeviceInfo: () => Promise<DeviceInfo>;
  confirmPairing: (input: ConfirmPairingInput) => Promise<PairingResultResponse>;
  remoteHost: () => RemoteHost;
  createStore: (options: SecurePairingStoreOptions) => Promise<SecurePairingStore>;
  listPairedDevices: () => Promise<PairedDeviceRecord[]>;
  now: () => number;
}

export type MobilePairingStatus = 'idle' | 'scanning' | 'reviewing' | 'pairing' | 'paired' | 'fingerprint_mismatch';

let adapterOverrides: Partial<MobilePairingAdapters> = {};

export function configureMobilePairingAdapters(overrides: Partial<MobilePairingAdapters>): void {
  adapterOverrides = overrides;
}

export function resetMobilePairingAdapters(): void {
  adapterOverrides = {};
}

export const useMobilePairingStore = defineStore('mobile-pairing', () => {
  const manualUri = ref('');
  const draft = ref<MobilePairPayload | null>(null);
  const storedRecord = ref<SecurePairingRecord | null>(null);
  const pairedDevices = ref<PairedDeviceRecord[]>([]);
  const status = ref<MobilePairingStatus>('idle');
  const error = ref<string | null>(null);
  const isLoading = ref(false);
  const fingerprintMismatch = ref(false);

  const canConfirm = computed(() => Boolean(draft.value && !isLoading.value));
  const endpoint = computed(() => (draft.value ? formatPairingEndpoint(draft.value) : ''));

  async function scanQr(): Promise<MobilePairPayload> {
    isLoading.value = true;
    status.value = 'scanning';
    error.value = null;
    try {
      const uri = await currentAdapters().scanQr();
      return setManualUri(uri);
    } catch (scanError) {
      status.value = 'idle';
      error.value = errorMessage(scanError);
      throw scanError;
    } finally {
      isLoading.value = false;
    }
  }

  function setManualUri(uri: string): MobilePairPayload {
    manualUri.value = uri;
    const payload = parsePairingUri(uri, currentAdapters().now());
    draft.value = payload;
    error.value = null;
    updateFingerprintState();
    if (!fingerprintMismatch.value) status.value = 'reviewing';
    return payload;
  }

  async function loadStoredPairing(vaultPassword: string): Promise<SecurePairingRecord | null> {
    const secureStore = await openSecureStore(vaultPassword);
    storedRecord.value = await secureStore.load();
    updateFingerprintState();
    return storedRecord.value;
  }

  async function confirm(vaultPassword: string, options: { trustNewFingerprint?: boolean } = {}): Promise<SecurePairingRecord> {
    if (!draft.value) throw new Error('Scan or paste a pairing QR first.');
    if (fingerprintMismatch.value && !options.trustNewFingerprint) {
      status.value = 'fingerprint_mismatch';
      throw new Error('Desktop fingerprint changed. Confirm re-pairing before saving new credentials.');
    }

    isLoading.value = true;
    status.value = 'pairing';
    error.value = null;
    try {
      const adapters = currentAdapters();
      const deviceInfo = await adapters.getDeviceInfo();
      const result = await adapters.confirmPairing({
        payload: draft.value,
        deviceId: deviceInfo.device_id,
        displayName: deviceInfo.name,
      });
      const credentials: PairingCredentialBundle = {
        deviceId: result.device.device_id,
        displayName: result.device.display_name,
        capabilities: result.device.capabilities,
        desktopHost: draft.value.host,
        desktopPort: draft.value.port,
        desktopFingerprintB64: draft.value.fingerprintB64,
        pairingTokenB64: draft.value.tokenB64,
        clientCertPem: result.client_cert_pem,
        clientKeyPem: result.client_key_pem,
        caCertPem: result.ca_cert_pem,
        pairedAt: result.device.paired_at,
      };
      const secureStore = await openSecureStore(vaultPassword);
      storedRecord.value = await secureStore.save(credentials);
      fingerprintMismatch.value = false;
      status.value = 'paired';
      await loadPairedDevices();
      return storedRecord.value;
    } catch (confirmError) {
      status.value = fingerprintMismatch.value ? 'fingerprint_mismatch' : 'reviewing';
      error.value = errorMessage(confirmError);
      throw confirmError;
    } finally {
      isLoading.value = false;
    }
  }

  async function forgetStoredPairing(vaultPassword: string): Promise<void> {
    const secureStore = await openSecureStore(vaultPassword);
    await secureStore.remove();
    storedRecord.value = null;
    updateFingerprintState();
  }

  async function loadPairedDevices(): Promise<void> {
    try {
      pairedDevices.value = await currentAdapters().listPairedDevices();
    } catch (listError) {
      error.value = errorMessage(listError);
    }
  }

  function clearError(): void {
    error.value = null;
  }

  function updateFingerprintState(): void {
    fingerprintMismatch.value = pairingFingerprintMismatch(storedRecord.value, draft.value);
    if (fingerprintMismatch.value) status.value = 'fingerprint_mismatch';
  }

  async function openSecureStore(vaultPassword: string): Promise<SecurePairingStore> {
    return currentAdapters().createStore({ vaultPassword });
  }

  return {
    manualUri,
    draft,
    storedRecord,
    pairedDevices,
    status,
    error,
    isLoading,
    fingerprintMismatch,
    canConfirm,
    endpoint,
    scanQr,
    setManualUri,
    loadStoredPairing,
    confirm,
    forgetStoredPairing,
    loadPairedDevices,
    clearError,
  };
});

function currentAdapters(): MobilePairingAdapters {
  return {
    scanQr: adapterOverrides.scanQr ?? scanPairingQrCode,
    getDeviceInfo: adapterOverrides.getDeviceInfo ?? (() => invoke<DeviceInfo>('get_device_identity')),
    confirmPairing: adapterOverrides.confirmPairing ?? defaultConfirmPairing,
    remoteHost: adapterOverrides.remoteHost ?? (() => createLocalRemoteHost()),
    createStore: adapterOverrides.createStore ?? createSecurePairingStore,
    listPairedDevices: adapterOverrides.listPairedDevices ?? (async () => {
      const devices = await currentAdapters().remoteHost().listPairedDevices();
      return devices.map((device) => ({
        device_id: device.deviceId,
        display_name: device.displayName,
        cert_fingerprint: device.certFingerprint ?? '',
        capabilities: device.capabilities,
        paired_at: device.pairedAtUnixMs,
        last_seen_at: device.lastSeenAtUnixMs,
      }));
    }),
    now: adapterOverrides.now ?? Date.now,
  };
}

async function defaultConfirmPairing(input: ConfirmPairingInput): Promise<PairingResultResponse> {
  return invoke<PairingResultResponse>('confirm_pairing', {
    deviceId: input.deviceId,
    displayName: input.displayName,
    tokenB64: input.payload.tokenB64,
  });
}

function errorMessage(value: unknown): string {
  return value instanceof Error ? value.message : String(value);
}