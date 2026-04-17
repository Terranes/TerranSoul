/**
 * Pinia store for streaming LLM responses.
 *
 * Listens to `llm-chunk` Tauri events and assembles text progressively.
 * Integrates with the main conversation store to add the final message.
 */
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { EmotionTag } from '../types';

export const useStreamingStore = defineStore('streaming', () => {
  /** Whether we are currently streaming a response. */
  const isStreaming = ref(false);
  /** Accumulated text so far (raw, with tags stripped). */
  const streamText = ref('');
  /** Raw accumulated text (tags not stripped). */
  const streamRawText = ref('');
  /** Latest emotion detected during streaming. */
  const currentEmotion = ref<EmotionTag | null>(null);
  /** Latest motion/gesture detected during streaming. */
  const currentMotion = ref<string | null>(null);
  /** Error, if any. */
  const error = ref<string | null>(null);

  /**
   * Send a message and stream the response.
   * The actual streaming events are received via Tauri event system.
   * This initiates the stream; call `handleChunk` to process each event.
   */
  async function sendStreaming(message: string): Promise<boolean> {
    isStreaming.value = true;
    streamText.value = '';
    streamRawText.value = '';
    currentEmotion.value = null;
    currentMotion.value = null;
    error.value = null;

    try {
      await invoke('send_message_stream', { message });
      return true;
    } catch (err) {
      error.value = String(err);
      isStreaming.value = false;
      return false;
    }
  }

  /**
   * Process a single LLM chunk from the Tauri event.
   * Text is already clean (anim blocks stripped by Rust StreamTagParser).
   */
  function handleChunk(chunk: { text: string; done: boolean }) {
    if (chunk.done) {
      isStreaming.value = false;
      return;
    }

    // Set isStreaming to true on first chunk with actual text
    if (!isStreaming.value && chunk.text) {
      isStreaming.value = true;
    }

    // Text from Rust is already clean — just accumulate.
    streamRawText.value += chunk.text;
    streamText.value += chunk.text;
  }

  /**
   * Process a structured animation command from the `llm-animation` event.
   * Called by the ChatView event listener.
   */
  function handleAnimation(cmd: { emotion?: string; motion?: string }) {
    if (cmd.emotion) {
      const lower = cmd.emotion.toLowerCase();
      const valid: ReadonlySet<string> = new Set([
        'happy', 'sad', 'angry', 'relaxed', 'surprised', 'neutral',
      ]);
      if (valid.has(lower)) {
        currentEmotion.value = lower as EmotionTag;
      }
    }
    if (cmd.motion) {
      currentMotion.value = cmd.motion.toLowerCase();
    }
  }

  /** Reset streaming state. */
  function reset() {
    isStreaming.value = false;
    streamText.value = '';
    streamRawText.value = '';
    currentEmotion.value = null;
    currentMotion.value = null;
    error.value = null;
  }

  return {
    isStreaming,
    streamText,
    streamRawText,
    currentEmotion,
    currentMotion,
    error,
    sendStreaming,
    handleChunk,
    handleAnimation,
    reset,
  };
});
