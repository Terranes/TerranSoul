<template>
  <div class="pairing-view">
    <div class="pairing-header">
      <h2>Device Pairing</h2>
      <p class="subtitle">
        Scan the QR code on another device to pair with this one.
      </p>
    </div>

    <div
      v-if="store.error"
      class="error-banner"
    >
      {{ store.error }}
      <button
        class="dismiss-btn"
        @click="store.clearError()"
      >
        ✕
      </button>
    </div>

    <section class="identity-section">
      <h3>This Device</h3>
      <div
        v-if="store.deviceInfo"
        class="identity-card"
      >
        <div class="identity-row">
          <span class="label">Name</span>
          <span class="value">{{ store.deviceInfo.name }}</span>
        </div>
        <div class="identity-row">
          <span class="label">Device ID</span>
          <span class="value mono truncate">{{ store.deviceInfo.device_id }}</span>
        </div>
        <div class="identity-row">
          <span class="label">Public Key</span>
          <span class="value mono truncate">{{ store.deviceInfo.public_key_b64 }}</span>
        </div>
      </div>
      <div
        v-else-if="store.isLoading"
        class="loading"
      >
        Loading identity…
      </div>
    </section>

    <section class="qr-section">
      <h3>Pairing QR Code</h3>
      <div
        v-if="store.qrSvg"
        class="qr-container"
        v-html="store.qrSvg"
      />
      <div
        v-else-if="store.isLoading"
        class="loading"
      >
        Generating QR…
      </div>
      <button
        v-else
        class="btn-primary"
        @click="store.loadPairingQr()"
      >
        Show QR Code
      </button>
    </section>

    <section class="trusted-section">
      <h3>Trusted Devices</h3>
      <ul
        v-if="store.trustedDevices.length"
        class="device-list"
      >
        <li
          v-for="device in store.trustedDevices"
          :key="device.device_id"
          class="device-item"
        >
          <div class="device-info">
            <span class="device-name">{{ device.name }}</span>
            <span class="device-id mono">{{ device.device_id }}</span>
          </div>
          <button
            class="btn-remove"
            @click="removeDevice(device.device_id)"
          >
            Remove
          </button>
        </li>
      </ul>
      <p
        v-else
        class="empty-state"
      >
        No trusted devices yet. Pair another device using the QR code above.
      </p>
    </section>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue';
import { useIdentityStore } from '../stores/identity';

const store = useIdentityStore();

onMounted(async () => {
  await Promise.all([
    store.loadIdentity(),
    store.loadPairingQr(),
    store.loadTrustedDevices(),
  ]);
});

async function removeDevice(deviceId: string) {
  await store.removeTrustedDevice(deviceId);
}
</script>

<style scoped>
.pairing-view {
  padding: 1.5rem;
  max-width: 560px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.pairing-header h2 {
  margin: 0 0 0.25rem;
  font-size: 1.25rem;
}

.subtitle {
  margin: 0;
  opacity: 0.7;
  font-size: 0.875rem;
}

.error-banner {
  background: var(--ts-error-bg);
  color: var(--ts-error);
  border-radius: 6px;
  padding: 0.75rem 1rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.875rem;
}

.dismiss-btn {
  background: none;
  border: none;
  cursor: pointer;
  color: inherit;
  font-size: 1rem;
  padding: 0 0.25rem;
}

section h3 {
  margin: 0 0 0.75rem;
  font-size: 1rem;
  font-weight: 600;
}

.identity-card {
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border);
  border-radius: 8px;
  padding: 0.75rem 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.identity-row {
  display: flex;
  gap: 1rem;
  font-size: 0.875rem;
}

.label {
  width: 80px;
  flex-shrink: 0;
  opacity: 0.6;
}

.mono {
  font-family: monospace;
  font-size: 0.8rem;
}

.truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.qr-container {
  display: flex;
  justify-content: center;
}

.qr-container :deep(svg) {
  max-width: 220px;
  height: auto;
}

.loading {
  opacity: 0.6;
  font-size: 0.875rem;
}

.btn-primary {
  background: var(--ts-accent-blue-hover);
  color: var(--ts-text-on-accent);
  border: none;
  border-radius: 6px;
  padding: 0.5rem 1.25rem;
  cursor: pointer;
  font-size: 0.875rem;
  transition: background var(--ts-transition-fast);
}

.btn-primary:hover {
  background: var(--ts-accent-blue);
}

.device-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.device-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--ts-bg-surface);
  border-radius: 6px;
  padding: 0.5rem 0.75rem;
}

.device-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.device-name {
  font-size: 0.875rem;
  font-weight: 500;
}

.device-id {
  font-size: 0.75rem;
  opacity: 0.6;
}

.btn-remove {
  background: none;
  border: 1px solid rgba(239, 68, 68, 0.4);
  color: var(--ts-error);
  border-radius: 4px;
  padding: 0.25rem 0.5rem;
  cursor: pointer;
  font-size: 0.75rem;
  transition: background var(--ts-transition-fast);
}

.btn-remove:hover {
  background: var(--ts-error-bg);
}

.empty-state {
  opacity: 0.5;
  font-size: 0.875rem;
  margin: 0;
}
</style>
