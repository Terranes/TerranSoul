import { ref } from 'vue';
import type { useVoiceStore } from '../stores/voice';

/**
 * Composable wrapping the Supertonic first-run consent → download → finish
 * state machine. Extracted from VoiceSetupView so the view stays inside the
 * project's `max-lines` budget and so the same machine can be reused from
 * other entry points (settings panel, first-launch wizard) later.
 *
 * The composable does NOT own the visibility decision — callers pass in
 * `openIfNeeded` to inspect the current provider and decide whether to show
 * the consent dialog. The composable owns: the modal's stage, the accept
 * handler that drives `downloadSupertonic`, and the cancel/dismiss helpers.
 */
export function useSupertonicConsent(voice: ReturnType<typeof useVoiceStore>) {
  const consentVisible = ref(false);
  const consentStage = ref<'consent' | 'downloading' | 'error' | 'done'>(
    'consent',
  );

  /**
   * Open the consent dialog when the user is trying to switch to Supertonic
   * but the model is not yet installed. Returns `true` when the dialog was
   * opened (caller should NOT call `setTtsProvider` itself in that case).
   */
  function openIfNeeded(providerId: string | null): boolean {
    if (providerId !== 'supertonic') return false;
    const provider = voice.ttsProviders.find((p) => p.id === 'supertonic');
    if (!provider || !provider.requires_install || provider.installed) {
      return false;
    }
    consentStage.value = 'consent';
    consentVisible.value = true;
    return true;
  }

  async function onAccept(): Promise<void> {
    consentStage.value = 'downloading';
    try {
      await voice.downloadSupertonic();
      consentStage.value = 'done';
      await voice.autoPromoteSupertonicIfReady();
    } catch {
      // `voice.supertonicError` is now populated; the dialog will display it.
      consentStage.value = 'error';
    }
  }

  function onCancel(): void {
    consentVisible.value = false;
    consentStage.value = 'consent';
  }

  function onDismiss(): void {
    consentVisible.value = false;
    if (consentStage.value === 'done' || consentStage.value === 'error') {
      consentStage.value = 'consent';
    }
  }

  return {
    consentVisible,
    consentStage,
    openIfNeeded,
    onAccept,
    onCancel,
    onDismiss,
  };
}
