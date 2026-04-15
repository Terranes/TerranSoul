/**
 * Streaming TTS playback composable.
 *
 * Consumes LLM token chunks as they stream in, detects sentence boundaries,
 * and synthesizes + plays each sentence sequentially via the Rust
 * `synthesize_tts` Tauri command. Voice starts ~200ms after the first
 * sentence completes — a major UX improvement over batched post-response TTS.
 *
 * Usage in ChatView.vue:
 *   const tts = useTtsPlayback();
 *   // Feed each llm-chunk text:
 *   tts.feedChunk(chunk.text);
 *   // When stream ends:
 *   tts.flush();
 *   // When user sends a new message:
 *   tts.stop();
 */

import { ref, readonly } from 'vue';
import { invoke } from '@tauri-apps/api/core';

/** Sentence-ending punctuation patterns that trigger TTS synthesis. */
const SENTENCE_END_RE = /[.!?…]\s+|[\n]/;

/** Minimum sentence length to bother synthesizing (filters out stray punctuation). */
const MIN_SENTENCE_CHARS = 4;

export interface TtsPlaybackHandle {
  /** Whether TTS audio is currently playing or pending. */
  isSpeaking: Readonly<ReturnType<typeof ref<boolean>>>;
  /** Feed an LLM token chunk into the sentence buffer. */
  feedChunk(text: string): void;
  /** Flush the remaining buffer as the final TTS sentence (call on stream done). */
  flush(): void;
  /** Stop all pending synthesis and cancel audio playback. */
  stop(): void;
}

export function useTtsPlayback(): TtsPlaybackHandle {
  const isSpeaking = ref(false);

  /** Accumulated text not yet sent to TTS. */
  let buffer = '';
  /** Whether we are currently stopped (user interrupted). */
  let stopped = false;
  /** Queue of WAV data Promises — each item is one sentence being synthesized. */
  const synthQueue: Promise<Uint8Array | null>[] = [];
  /** Current HTMLAudioElement being played, if any. */
  let currentAudio: HTMLAudioElement | null = null;
  /** Blob URLs created for audio elements, tracked for cleanup. */
  const blobUrls: string[] = [];

  // ── Internal helpers ───────────────────────────────────────────────────────

  /** Revoke all created blob URLs to free memory. */
  function cleanupBlobUrls() {
    for (const url of blobUrls.splice(0)) {
      URL.revokeObjectURL(url);
    }
  }

  /** Enqueue a sentence for synthesis and sequential playback. */
  function enqueueSentence(sentence: string): void {
    const trimmed = sentence.trim();
    if (trimmed.length < MIN_SENTENCE_CHARS || stopped) return;

    const synthPromise = invoke<number[]>('synthesize_tts', { text: trimmed })
      .then((bytes) => new Uint8Array(bytes))
      .catch(() => null);

    synthQueue.push(synthPromise);

    // Drain queue if nothing is playing
    if (synthQueue.length === 1) {
      drainQueue();
    }
  }

  /** Play synthesized sentences from the queue one after another. */
  async function drainQueue(): Promise<void> {
    while (synthQueue.length > 0 && !stopped) {
      const wavBytes = await synthQueue[0];
      synthQueue.shift();

      if (!wavBytes || stopped) continue;

      isSpeaking.value = true;
      await playWavBytes(wavBytes);
    }

    if (!stopped) {
      isSpeaking.value = false;
    }
  }

  /** Play WAV bytes via HTMLAudioElement. Returns a Promise that resolves when done. */
  function playWavBytes(bytes: Uint8Array): Promise<void> {
    return new Promise((resolve) => {
      if (stopped) {
        resolve();
        return;
      }

      const blob = new Blob([bytes], { type: 'audio/wav' });
      const url = URL.createObjectURL(blob);
      blobUrls.push(url);

      const audio = new Audio(url);
      currentAudio = audio;

      audio.onended = () => {
        currentAudio = null;
        resolve();
      };

      audio.onerror = () => {
        currentAudio = null;
        resolve();
      };

      audio.play().catch(() => resolve());
    });
  }

  /** Extract complete sentences from the buffer, returning leftover text. */
  function extractSentences(text: string): { sentences: string[]; remainder: string } {
    const sentences: string[] = [];
    let remainder = text;

    while (true) {
      const match = SENTENCE_END_RE.exec(remainder);
      if (!match) break;

      const sentence = remainder.slice(0, match.index + match[0].length);
      remainder = remainder.slice(sentence.length);
      if (sentence.trim().length >= MIN_SENTENCE_CHARS) {
        sentences.push(sentence);
      }
    }

    return { sentences, remainder };
  }

  // ── Public API ─────────────────────────────────────────────────────────────

  function feedChunk(text: string): void {
    if (stopped || !text) return;

    buffer += text;
    const { sentences, remainder } = extractSentences(buffer);
    buffer = remainder;

    for (const sentence of sentences) {
      enqueueSentence(sentence);
    }
  }

  function flush(): void {
    if (stopped) return;

    const leftover = buffer.trim();
    buffer = '';
    if (leftover.length >= MIN_SENTENCE_CHARS) {
      enqueueSentence(leftover);
    }
  }

  function stop(): void {
    stopped = true;
    buffer = '';
    synthQueue.length = 0;
    isSpeaking.value = false;

    if (currentAudio) {
      currentAudio.pause();
      currentAudio.src = '';
      currentAudio = null;
    }

    cleanupBlobUrls();

    // Reset stopped flag so the composable can be reused for the next message
    stopped = false;
  }

  return {
    isSpeaking: readonly(isSpeaking),
    feedChunk,
    flush,
    stop,
  };
}
