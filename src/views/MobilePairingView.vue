<template>
  <div
    class="bp-shell mobile-pairing-view"
    data-density="cozy"
  >
    <!-- ── Breadcrumb ──────────────────────────────────────────────────────── -->
    <AppBreadcrumb
      here="PHONE LINK"
      @navigate="emit('navigate', $event)"
    />

    <header class="mobile-header">
      <h2>Phone Link</h2>
      <span :class="['status-pill', store.status]">{{ statusLabel }}</span>
    </header>

    <div
      v-if="store.error"
      class="error-banner"
      data-test="pairing-error"
    >
      {{ store.error }}
      <button
        type="button"
        @click="store.clearError()"
      >
        x
      </button>
    </div>

    <section class="scan-panel">
      <div class="vault-row">
        <input
          v-model="vaultPassword"
          class="pairing-input"
          type="password"
          placeholder="Vault password"
          data-test="pairing-vault-password"
        >
        <button
          class="primary-button"
          type="button"
          data-test="scan-pairing"
          :disabled="store.isLoading"
          @click="scan"
        >
          Scan QR
        </button>
      </div>

      <textarea
        v-model="manualUri"
        class="pairing-textarea"
        rows="3"
        placeholder="terransoul://pair?..."
        data-test="manual-pairing-uri"
      />
      <button
        class="secondary-button"
        type="button"
        @click="reviewManualUri"
      >
        Review URI
      </button>
    </section>

    <section
      v-if="store.draft"
      class="review-panel"
      data-test="pairing-review"
    >
      <div>
        <span class="label">Desktop</span>
        <strong>{{ store.endpoint }}</strong>
      </div>
      <div>
        <span class="label">Fingerprint</span>
        <code>{{ draftFingerprint }}</code>
      </div>
      <div>
        <span class="label">Expires</span>
        <span>{{ draftExpiry }}</span>
      </div>

      <p
        v-if="store.fingerprintMismatch"
        class="warning-banner"
      >
        Desktop fingerprint changed since the last saved pairing.
      </p>

      <button
        class="primary-button confirm-button"
        type="button"
        data-test="confirm-pairing"
        :disabled="!store.canConfirm || !vaultPassword"
        @click="confirm"
      >
        {{ store.fingerprintMismatch ? 'Trust Fingerprint' : 'Confirm Pairing' }}
      </button>
    </section>

    <MobileSettingsView :vault-password="vaultPassword" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useMobilePairingStore } from '../stores/mobile-pairing';
import { fingerprintPreview } from '../utils/mobile-pairing';
import MobileSettingsView from './MobileSettingsView.vue';
import AppBreadcrumb from '../components/ui/AppBreadcrumb.vue';

const emit = defineEmits<{
  navigate: [target: string];
}>();

const store = useMobilePairingStore();
const vaultPassword = ref('');
const manualUri = ref('');

const statusLabel = computed(() => {
  const labels: Record<typeof store.status, string> = {
    idle: 'Ready',
    scanning: 'Scanning',
    reviewing: 'Review',
    pairing: 'Pairing',
    paired: 'Paired',
    fingerprint_mismatch: 'Verify',
  };
  return labels[store.status];
});
const draftFingerprint = computed(() => (store.draft ? fingerprintPreview(store.draft.fingerprintB64) : ''));
const draftExpiry = computed(() => (store.draft ? new Date(store.draft.expiresAtUnixMs).toLocaleTimeString() : ''));

async function scan() {
  const payload = await store.scanQr();
  manualUri.value = payload.rawUri;
}

function reviewManualUri() {
  store.setManualUri(manualUri.value);
}

async function confirm() {
  await store.confirm(vaultPassword.value, { trustNewFingerprint: store.fingerprintMismatch });
}
</script>

<style scoped>
/*
 * Phone Link uses the shared `.bp-shell` chrome (padding / max-width / gap /
 * scrollbar) so the breadcrumb sits at the exact same position as every
 * other panel (Brain, Memory, Marketplace, Skill Tree, Voice). The rules
 * below only add Phone-Link-specific extras: mobile safe-area insets and a
 * bottom inset so the mobile bottom-nav bar doesn't cover content.
 */
.mobile-pairing-view {
  /* Reserve room for the mobile bottom-nav on small screens; .bp-shell
     already provides padding: 28px 32px 80px so we only extend the bottom
     when the mobile nav is present. */
  padding-bottom: calc(var(--ts-mobile-nav-total-height, 0px) + var(--ts-space-xl));
  /* Safe-area top inset stacks on top of .bp-shell's 28px. */
  padding-top: calc(28px + var(--ts-safe-area-top, 0px));
}

.mobile-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ts-space-md);
}

h2 {
  margin: 0;
  font-size: var(--ts-text-xl);
}

.status-pill {
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-pill);
  color: var(--ts-text-secondary);
  font-size: var(--ts-text-xs);
  padding: 4px 10px;
}

.status-pill.paired {
  color: var(--ts-success);
  background: var(--ts-success-bg);
}

.status-pill.fingerprint_mismatch {
  color: var(--ts-warning-text);
  background: var(--ts-warning-bg);
}

.scan-panel,
.review-panel {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-md);
}

.vault-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: var(--ts-space-sm);
}

.pairing-input,
.pairing-textarea {
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  font: inherit;
  min-width: 0;
  padding: 10px 12px;
}

.pairing-textarea {
  resize: vertical;
  font-family: var(--ts-font-mono);
  font-size: var(--ts-text-xs);
}

.review-panel {
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

.primary-button,
.secondary-button {
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm);
  cursor: pointer;
  font-weight: 600;
  padding: 10px 14px;
}

.primary-button {
  background: var(--ts-accent-blue-hover);
  color: var(--ts-text-on-accent);
  border-color: transparent;
}

.secondary-button {
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
}

.confirm-button {
  align-self: flex-start;
}

.primary-button:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.error-banner,
.warning-banner {
  border-radius: var(--ts-radius-sm);
  margin: 0;
  padding: 10px 12px;
}

.error-banner {
  display: flex;
  justify-content: space-between;
  gap: var(--ts-space-md);
  color: var(--ts-error);
  background: var(--ts-error-bg);
}

.error-banner button {
  background: transparent;
  border: 0;
  color: inherit;
  cursor: pointer;
}

.warning-banner {
  color: var(--ts-warning-text);
  background: var(--ts-warning-bg);
}

@media (max-width: 640px) {
  .mobile-pairing-view {
    /* Drop side padding to phone-friendly values, honouring safe-area. */
    padding-left: max(var(--ts-space-md), var(--ts-safe-area-left, 0px));
    padding-right: max(var(--ts-space-md), var(--ts-safe-area-right, 0px));
  }

  .vault-row {
    grid-template-columns: 1fr;
  }
}
</style>