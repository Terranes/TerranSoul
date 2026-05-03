import { beforeEach, describe, expect, it } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import MobilePairingView from './MobilePairingView.vue';
import {
  configureMobilePairingAdapters,
  resetMobilePairingAdapters,
  type PairedDeviceRecord,
} from '../stores/mobile-pairing';
import type { SecurePairingStore } from '../utils/secure-pairing-store';

const futureUri = 'terransoul://pair?host=192.168.1.42&port=7422&token=tok_123&fp=fp_new&exp=9999999999999';

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

describe('MobilePairingView', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    resetMobilePairingAdapters();
  });

  it('scans, reviews, and confirms a pairing URI', async () => {
    const savedCredentials: unknown[] = [];
    const secureStore: SecurePairingStore = {
      async save(credentials) {
        savedCredentials.push(credentials);
        return { schemaVersion: 1, credentials, savedAt: 2222 };
      },
      async load() {
        return null;
      },
      async remove() {},
    };
    configureMobilePairingAdapters({
      scanQr: async () => futureUri,
      now: () => 1000,
      createStore: async () => secureStore,
      getDeviceInfo: async () => ({ device_id: 'ios-device-1', name: 'Ari iPhone', public_key_b64: 'pub' }),
      confirmPairing: async () => ({
        device: pairedDevice(),
        client_cert_pem: 'client-cert',
        client_key_pem: 'client-key',
        ca_cert_pem: 'ca-cert',
      }),
      listPairedDevices: async () => [pairedDevice()],
    });

    const wrapper = mount(MobilePairingView);
    await wrapper.find('[data-test="pairing-vault-password"]').setValue('vault-secret');
    await wrapper.find('[data-test="scan-pairing"]').trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('192.168.1.42:7422');

    await wrapper.find('[data-test="confirm-pairing"]').trigger('click');
    await flushPromises();

    expect(savedCredentials).toHaveLength(1);
    expect(wrapper.text()).toContain('Paired');
  });
});