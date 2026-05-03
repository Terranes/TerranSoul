<template>
  <section class="mobile-settings-view">
    <div class="section-heading">
      <h3>Trusted Desktop</h3>
      <button
        class="icon-button"
        title="Refresh paired devices"
        type="button"
        @click="refresh"
      >
        <svg
          aria-hidden="true"
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" />
          <path d="M3 21v-5h5" />
          <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />
          <path d="M16 8h5V3" />
        </svg>
      </button>
    </div>

    <div class="vault-row">
      <input
        v-model="localVaultPassword"
        class="pairing-input"
        type="password"
        placeholder="Vault password"
        data-test="settings-vault-password"
      >
      <button
        class="secondary-button"
        type="button"
        @click="loadStored"
      >
        Unlock
      </button>
    </div>

    <div
      v-if="store.storedRecord"
      class="trust-panel"
    >
      <div>
        <span class="label">Endpoint</span>
        <strong>{{ storedEndpoint }}</strong>
      </div>
      <div>
        <span class="label">Fingerprint</span>
        <code>{{ storedFingerprint }}</code>
      </div>
      <div>
        <span class="label">Saved</span>
        <span>{{ savedAt }}</span>
      </div>
      <button
        class="danger-button"
        type="button"
        @click="forget"
      >
        Forget
      </button>
    </div>
    <p
      v-else
      class="empty-state"
    >
      No saved desktop pairing is unlocked.
    </p>

    <div class="section-heading compact">
      <h3>Desktop Trust List</h3>
    </div>
    <ul
      v-if="store.pairedDevices.length"
      class="device-list"
    >
      <li
        v-for="device in store.pairedDevices"
        :key="device.device_id"
      >
        <span>{{ device.display_name }}</span>
        <code>{{ device.device_id }}</code>
      </li>
    </ul>
    <p
      v-else
      class="empty-state"
    >
      No desktop trust-list entries are available yet.
    </p>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useMobilePairingStore } from '../stores/mobile-pairing';
import { fingerprintPreview } from '../utils/mobile-pairing';

const props = defineProps<{ vaultPassword?: string }>();
const store = useMobilePairingStore();
const localVaultPassword = ref('');

const effectiveVaultPassword = computed(() => props.vaultPassword || localVaultPassword.value);
const storedEndpoint = computed(() => {
  const credentials = store.storedRecord?.credentials;
  if (!credentials?.desktopHost || !credentials.desktopPort) return 'Unknown';
  return `${credentials.desktopHost}:${credentials.desktopPort}`;
});
const storedFingerprint = computed(() => {
  const fingerprint = store.storedRecord?.credentials.desktopFingerprintB64;
  return fingerprint ? fingerprintPreview(fingerprint) : 'Unknown';
});
const savedAt = computed(() => {
  const timestamp = store.storedRecord?.savedAt;
  return timestamp ? new Date(timestamp).toLocaleString() : 'Unknown';
});

onMounted(() => {
  void store.loadPairedDevices();
});

async function loadStored() {
  await store.loadStoredPairing(effectiveVaultPassword.value);
}

async function refresh() {
  await store.loadPairedDevices();
}

async function forget() {
  await store.forgetStoredPairing(effectiveVaultPassword.value);
}
</script>

<style scoped>
.mobile-settings-view {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-md);
}

.section-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ts-space-md);
}

.section-heading.compact {
  margin-top: var(--ts-space-md);
}

h3 {
  margin: 0;
  font-size: var(--ts-text-lg);
}

.vault-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: var(--ts-space-sm);
}

.pairing-input {
  min-width: 0;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  padding: 10px 12px;
}

.trust-panel {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: var(--ts-space-sm) var(--ts-space-md);
  align-items: center;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  background: var(--ts-bg-surface);
  padding: var(--ts-space-md);
}

.label {
  display: block;
  color: var(--ts-text-secondary);
  font-size: var(--ts-text-xs);
}

code {
  color: var(--ts-text-bright);
  font-family: var(--ts-font-mono);
  font-size: var(--ts-text-xs);
  overflow-wrap: anywhere;
}

.device-list {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-sm);
  list-style: none;
  margin: 0;
  padding: 0;
}

.device-list li {
  display: flex;
  justify-content: space-between;
  gap: var(--ts-space-md);
  border: 1px solid var(--ts-border-subtle);
  border-radius: var(--ts-radius-sm);
  padding: var(--ts-space-sm) var(--ts-space-md);
}

.empty-state {
  margin: 0;
  color: var(--ts-text-secondary);
  font-size: var(--ts-text-sm);
}

.secondary-button,
.danger-button,
.icon-button {
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  cursor: pointer;
  padding: 8px 12px;
}

.danger-button {
  color: var(--ts-error);
  border-color: color-mix(in srgb, var(--ts-error) 50%, var(--ts-border));
}

.icon-button {
  width: 36px;
  height: 36px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
</style>