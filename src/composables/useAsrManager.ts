/**
 * Multi-ASR Provider Manager composable.
 *
 * Provider-agnostic speech-to-text factory that routes to the correct backend
 * based on the configured `asr_provider` in the voice store:
 *
 * - `web-speech`  → browser-native SpeechRecognition API (no Rust, no mic permission prompt)
 * - `stub` /
 *   `whisper-api` /
 *   `groq-whisper` → VAD-based mic capture → `transcribe_audio` Tauri IPC
 *
 * Usage:
 *   const asr = useAsrManager({ onTranscript: (text) => send(text) });
 *   asr.startListening();  // activate mic + ASR
 *   asr.stopListening();   // deactivate mic
 */

import { ref, readonly } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useVad } from '../utils/vad';
import { useVoiceStore } from '../stores/voice';

export interface AsrManagerOptions {
  /** Called when a transcript is ready. */
  onTranscript?: (text: string) => void;
  /** Called on error. */
  onError?: (message: string) => void;
}

export interface AsrManagerHandle {
  /** True while mic is active and listening. */
  isListening: Readonly<ReturnType<typeof ref<boolean>>>;
  /** Last error message, if any. */
  error: Readonly<ReturnType<typeof ref<string | null>>>;
  /** Start listening for speech. No-op if already listening. */
  startListening(): Promise<void>;
  /** Stop listening and release mic. */
  stopListening(): void;
}

// ── Narrow SpeechRecognition type for environments without lib.dom.d.ts ───────

interface SpeechRecognitionResult {
  readonly 0: { readonly transcript: string };
}

interface SpeechRecognitionEvent extends Event {
  readonly results: { readonly [index: number]: SpeechRecognitionResult };
}

interface SpeechRecognitionInstance extends EventTarget {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  onresult: ((event: SpeechRecognitionEvent) => void) | null;
  onerror: ((event: Event & { error: string }) => void) | null;
  onend: (() => void) | null;
  start(): void;
  stop(): void;
  abort(): void;
}

declare global {
  interface Window {
    SpeechRecognition?: new () => SpeechRecognitionInstance;
    webkitSpeechRecognition?: new () => SpeechRecognitionInstance;
  }
}

/** Convert Float32Array samples (16kHz, float) to a plain number array for IPC. */
function float32ToArray(audio: Float32Array): number[] {
  return Array.from(audio);
}

export function useAsrManager(options: AsrManagerOptions = {}): AsrManagerHandle {
  const voice = useVoiceStore();
  const isListening = ref(false);
  const error = ref<string | null>(null);

  // ── SpeechRecognition instance (web-speech path) ───────────────────────────
  let recognition: SpeechRecognitionInstance | null = null;

  // ── VAD instance (Tauri IPC path) ─────────────────────────────────────────
  const vad = useVad({
    onSpeechEnd: async (audio: Float32Array) => {
      if (!isListening.value) return;
      try {
        const result = await invoke<{ text: string; language: string | null; confidence: number | null }>(
          'transcribe_audio',
          { samples: float32ToArray(audio) },
        );
        if (result.text.trim()) {
          options.onTranscript?.(result.text.trim());
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        error.value = `Transcription failed: ${msg}`;
        options.onError?.(error.value);
      }
    },
    onSpeechStart: () => {
      // Speech activity detected — could emit a visual indicator here
    },
  });

  // ── Internal helpers ───────────────────────────────────────────────────────

  /** Start the browser Web Speech API recognition. */
  async function startWebSpeech(): Promise<void> {
    const SpeechRecognition = window.SpeechRecognition ?? window.webkitSpeechRecognition;
    if (!SpeechRecognition) {
      error.value = 'Web Speech API is not supported in this browser';
      options.onError?.(error.value);
      return;
    }

    recognition = new SpeechRecognition();
    recognition.continuous = false;
    recognition.interimResults = false;
    recognition.lang = 'en-US';

    recognition.onresult = (event: SpeechRecognitionEvent) => {
      const transcript = event.results[0]?.[0]?.transcript ?? '';
      if (transcript.trim()) {
        options.onTranscript?.(transcript.trim());
      }
      stopWebSpeech();
    };

    recognition.onerror = (event: Event & { error: string }) => {
      if (event.error !== 'no-speech' && event.error !== 'aborted') {
        error.value = `Speech recognition error: ${event.error}`;
        options.onError?.(error.value);
      }
      isListening.value = false;
    };

    recognition.onend = () => {
      isListening.value = false;
    };

    recognition.start();
    isListening.value = true;
  }

  function stopWebSpeech(): void {
    if (recognition) {
      recognition.abort();
      recognition = null;
    }
    isListening.value = false;
  }

  // ── Public API ─────────────────────────────────────────────────────────────

  async function startListening(): Promise<void> {
    if (isListening.value) return;
    error.value = null;

    const provider = voice.config.asr_provider;

    if (!provider) {
      error.value = 'No ASR provider configured';
      options.onError?.(error.value);
      return;
    }

    if (provider === 'web-speech') {
      await startWebSpeech();
    } else {
      // Tauri IPC path: stub / whisper-api / groq-whisper
      await vad.startMic();
      if (vad.error.value) {
        error.value = vad.error.value;
        options.onError?.(error.value);
        return;
      }
      isListening.value = true;
    }
  }

  function stopListening(): void {
    if (voice.config.asr_provider === 'web-speech') {
      stopWebSpeech();
    } else {
      vad.stopMic();
      isListening.value = false;
    }
  }

  return {
    isListening: readonly(isListening),
    error: readonly(error),
    startListening,
    stopListening,
  };
}
