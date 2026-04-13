/**
 * Voice Activity Detection (VAD) composable.
 *
 * Uses @ricky0123/vad-web (ONNX-based, runs in browser via WebAssembly) for
 * accurate speech detection. Provides:
 *   - onSpeechStart — speech detected, pause AI audio, capture mic
 *   - onSpeechEnd   — speech ended, audio data ready for ASR
 *   - Echo cancellation: mute TTS during mic capture
 *
 * Pattern reference: Open-LLM-VTuber-Web/src/renderer/src/context/vad-context.tsx
 */
import { ref, computed, onUnmounted } from 'vue';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface VadSettings {
  /** Speech start confidence threshold (0–1). Default: 0.5. */
  positiveSpeechThreshold: number;
  /** Speech end confidence threshold (0–1). Default: 0.35. */
  negativeSpeechThreshold: number;
  /** Milliseconds to wait before confirming speech end. Default: 300. */
  redemptionMs: number;
}

export interface VadCallbacks {
  /** Speech started — pause AI audio, begin recording. */
  onSpeechStart?: () => void;
  /** Speech ended — audio data available for ASR. */
  onSpeechEnd?: (audio: Float32Array) => void;
  /** VAD misfire — false positive detected. */
  onMisfire?: () => void;
  /** Frame-level speech probability (0–1). */
  onFrameProcessed?: (probability: number) => void;
}

const DEFAULT_SETTINGS: VadSettings = {
  positiveSpeechThreshold: 0.5,
  negativeSpeechThreshold: 0.35,
  redemptionMs: 300,
};

// ── Composable ────────────────────────────────────────────────────────────────

export function useVad(callbacks: VadCallbacks = {}) {
  const micOn = ref(false);
  const isSpeaking = ref(false);
  const lastProbability = ref(0);
  const settings = ref<VadSettings>({ ...DEFAULT_SETTINGS });
  const error = ref<string | null>(null);

  // Internal VAD instance (loaded dynamically)
  let vadInstance: { destroy: () => Promise<void>; pause: () => Promise<void>; start: () => Promise<void> } | null = null;

  const canStart = computed(() => !micOn.value);

  /**
   * Start the microphone and VAD.
   * Dynamically imports @ricky0123/vad-web to avoid bundling ONNX model
   * when voice is not used.
   */
  async function startMic(): Promise<void> {
    if (micOn.value) return;
    error.value = null;

    try {
      const { MicVAD } = await import('@ricky0123/vad-web');

      vadInstance = await MicVAD.new({
        positiveSpeechThreshold: settings.value.positiveSpeechThreshold,
        negativeSpeechThreshold: settings.value.negativeSpeechThreshold,
        redemptionMs: settings.value.redemptionMs,

        onSpeechStart: () => {
          isSpeaking.value = true;
          callbacks.onSpeechStart?.();
        },

        onSpeechEnd: (audio: Float32Array) => {
          isSpeaking.value = false;
          callbacks.onSpeechEnd?.(audio);
        },

        onVADMisfire: () => {
          isSpeaking.value = false;
          callbacks.onMisfire?.();
        },

        onFrameProcessed: (probs: { isSpeech: number }, _frame: Float32Array) => {
          lastProbability.value = probs.isSpeech;
          callbacks.onFrameProcessed?.(probs.isSpeech);
        },
      });

      vadInstance.start();
      micOn.value = true;
    } catch (e) {
      error.value = `Failed to start VAD: ${e instanceof Error ? e.message : String(e)}`;
      micOn.value = false;
    }
  }

  /** Stop the microphone and VAD. */
  function stopMic(): void {
    if (vadInstance) {
      try {
        vadInstance.pause();
        vadInstance.destroy();
      } catch {
        // Already destroyed
      }
      vadInstance = null;
    }
    micOn.value = false;
    isSpeaking.value = false;
    lastProbability.value = 0;
  }

  /** Update VAD settings. Takes effect on next startMic() call. */
  function updateSettings(newSettings: Partial<VadSettings>): void {
    settings.value = { ...settings.value, ...newSettings };
  }

  // Cleanup on unmount
  onUnmounted(() => {
    stopMic();
  });

  return {
    // state
    micOn,
    isSpeaking,
    lastProbability,
    settings,
    error,
    // computed
    canStart,
    // actions
    startMic,
    stopMic,
    updateSettings,
  };
}
