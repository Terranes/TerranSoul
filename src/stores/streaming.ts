/**
 * Pinia store for streaming LLM responses.
 *
 * Listens to `llm-chunk` Tauri events and assembles text progressively.
 * Integrates with the main conversation store to add the final message.
 */
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { parseTags } from '../utils/emotion-parser';
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
   * Call this from the event listener.
   */
  function handleChunk(chunk: { text: string; done: boolean }) {
    if (chunk.done) {
      // Trim final accumulated text
      streamText.value = streamText.value.trim();
      isStreaming.value = false;
      return;
    }

    streamRawText.value += chunk.text;

    // Parse tags from the incoming chunk
    const parsed = parseTags(chunk.text);

    if (parsed.emotion) {
      currentEmotion.value = parsed.emotion;
    }

    // Accumulate display text
    streamText.value += parsed.text;
  }

  /** Reset streaming state. */
  function reset() {
    isStreaming.value = false;
    streamText.value = '';
    streamRawText.value = '';
    currentEmotion.value = null;
    error.value = null;
  }

  return {
    isStreaming,
    streamText,
    streamRawText,
    currentEmotion,
    error,
    sendStreaming,
    handleChunk,
    reset,
  };
});
