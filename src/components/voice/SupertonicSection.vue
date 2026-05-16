<template>
  <section
    v-if="voice.supertonicPromotion"
    class="bp-module vs-card vs-promote-banner"
    data-testid="supertonic-promotion-banner"
  >
    <div class="vs-promote-banner__text">
      <strong>Supertonic is now your default voice.</strong>
      Auto-switched from the browser default to the on-device model.
    </div>
    <button
      type="button"
      class="bp-btn bp-btn--ghost"
      data-testid="supertonic-revert"
      @click="voice.revertSupertonicPromotion"
    >
      Revert
    </button>
  </section>
  <SupertonicConsentDialog
    :visible="consent.consentVisible.value"
    :stage="consent.consentStage.value"
    :progress="voice.supertonicProgress"
    :error-message="voice.supertonicError ?? ''"
    @accept="consent.onAccept"
    @cancel="consent.onCancel"
    @dismiss="consent.onDismiss"
  />
</template>

<script setup lang="ts">
import { useVoiceStore } from '../../stores/voice';
import { useSupertonicConsent } from '../../composables/useSupertonicConsent';
import SupertonicConsentDialog from './SupertonicConsentDialog.vue';

const voice = useVoiceStore();
const consent = useSupertonicConsent(voice);

// Expose the consent helper so VoiceSetupView can route Supertonic clicks
// through `consent.openIfNeeded` without reaching into private state.
defineExpose({ consent });
</script>

<style scoped>
.vs-promote-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.875rem 1rem;
}

.vs-promote-banner__text {
  color: var(--ts-text-secondary, #c8cdda);
  font-size: 0.9rem;
  line-height: 1.5;
}
</style>
