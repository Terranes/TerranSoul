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
const SENTENCE_END_RE = /[.!?…]\s+|\n/;

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
  /**
   * Generation counter — incremented on every stop() call.
   * Each async operation captures the generation at enqueue time and aborts
   * if the current generation has changed when it executes.
   */
  let generation = 0;
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
    if (trimmed.length < MIN_SENTENCE_CHARS) return;

    // Capture generation at enqueue time so this sentence aborts if stop() is called.
    const myGen = generation;

    const synthPromise = invoke<number[]>('synthesize_tts', { text: trimmed })
      .then((bytes) => (generation === myGen ? new Uint8Array(bytes) : null))
      .catch(() => null);

    synthQueue.push(synthPromise);

    // Drain queue if nothing is playing
    if (synthQueue.length === 1) {
      drainQueue(myGen);
    }
  }

  /** Play synthesized sentences from the queue one after another. */
  async function drainQueue(myGen: number): Promise<void> {
    while (synthQueue.length > 0 && generation === myGen) {
      const wavBytes = await synthQueue[0];
      synthQueue.shift();

      if (!wavBytes || generation !== myGen) continue;

      isSpeaking.value = true;
      await playWavBytes(wavBytes, myGen);
    }

    if (generation === myGen) {
      isSpeaking.value = false;
    }
  }

  /** Play WAV bytes via HTMLAudioElement. Returns a Promise that resolves when done. */
  function playWavBytes(bytes: Uint8Array, myGen: number): Promise<void> {
    return new Promise((resolve) => {
      if (generation !== myGen) {
        resolve();
        return;
      }

      const blob = new Blob([bytes.buffer as ArrayBuffer], { type: 'audio/wav' });
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
    if (!text) return;

    buffer += text;
    const { sentences, remainder } = extractSentences(buffer);
    buffer = remainder;

    for (const sentence of sentences) {
      enqueueSentence(sentence);
    }
  }

  function flush(): void {
    const leftover = buffer.trim();
    buffer = '';
    if (leftover.length >= MIN_SENTENCE_CHARS) {
      enqueueSentence(leftover);
    }
  }

  function stop(): void {
    // Increment generation — all in-flight async operations from previous generation
    // will see the generation mismatch and abort before playing audio.
    generation++;
    buffer = '';
    synthQueue.length = 0;
    isSpeaking.value = false;

    if (currentAudio) {
      currentAudio.pause();
      currentAudio.src = '';
      currentAudio = null;
    }

    cleanupBlobUrls();
  }

  return {
    isSpeaking: readonly(isSpeaking),
    feedChunk,
    flush,
    stop,
  };
}
