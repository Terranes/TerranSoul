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
  /** The sentence currently being spoken (empty when not speaking). */
  currentSentence: Readonly<ReturnType<typeof ref<string>>>;
  /** All text that has been fully spoken so far in this generation. */
  spokenText: Readonly<ReturnType<typeof ref<string>>>;
  /** Feed an LLM token chunk into the sentence buffer. */
  feedChunk(text: string): void;
  /** Flush the remaining buffer as the final TTS sentence (call on stream done). */
  flush(): void;
  /** Stop all pending synthesis and cancel audio playback. */
  stop(): void;
  /**
   * Register a callback fired when a sentence starts playing.
   * Receives the HTMLAudioElement that is about to play.
   */
  onAudioStart(cb: (audio: HTMLAudioElement) => void): void;
  /** Register a callback fired when a sentence finishes playing. */
  onAudioEnd(cb: () => void): void;
  /** Register a callback fired when stop() is called (all playback cancelled). */
  onPlaybackStop(cb: () => void): void;
}

export interface TtsPlaybackOptions {
  /** Returns the browser speech pitch to use (called per utterance). */
  getBrowserPitch?: () => number;
  /** Returns the browser speech rate to use (called per utterance). */
  getBrowserRate?: () => number;
}

export function useTtsPlayback(options?: TtsPlaybackOptions): TtsPlaybackHandle {
  const isSpeaking = ref(false);
  /** The sentence currently being spoken. */
  const currentSentence = ref('');
  /** All text fully spoken so far in this generation. */
  const spokenText = ref('');

  /** Accumulated text not yet sent to TTS. */
  let buffer = '';
  /**
   * Generation counter — incremented on every stop() call.
   * Each async operation captures the generation at enqueue time and aborts
   * if the current generation has changed when it executes.
   */
  let generation = 0;
  /** Queue of pending sentences — each carries text and a WAV synthesis promise. */
  const synthQueue: { text: string; wav: Promise<Uint8Array | null> }[] = [];
  /** Current HTMLAudioElement being played, if any. */
  let currentAudio: HTMLAudioElement | null = null;
  /** Blob URLs created for audio elements, tracked for cleanup. */
  const blobUrls: string[] = [];

  // Callback hooks for LipSync integration
  let audioStartCb: ((audio: HTMLAudioElement) => void) | null = null;
  let audioEndCb: (() => void) | null = null;
  let playbackStopCb: (() => void) | null = null;

  // ── Internal helpers ───────────────────────────────────────────────────────

  /** Revoke all created blob URLs to free memory. */
  function cleanupBlobUrls() {
    for (const url of blobUrls.splice(0)) {
      URL.revokeObjectURL(url);
    }
  }

  /** Strip markdown formatting before TTS synthesis. */
  function stripMarkdown(text: string): string {
    return text
      .replace(/\*\*([^*]+)\*\*/g, '$1')  // Remove **bold**
      .replace(/\*([^*]+)\*/g, '$1')     // Remove *italic*
      .replace(/`([^`]+)`/g, '$1')       // Remove `code`
      .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1') // Remove [links](url)
      .replace(/#+ /g, '')               // Remove markdown headers
      .replace(/^[-*+] /gm, '')          // Remove list bullets
      .replace(/\n{2,}/g, '. ')          // Replace multiple newlines with periods
      .replace(/\n/g, ' ')               // Replace single newlines with spaces
      .trim();
  }

  /** Enqueue a sentence for synthesis and sequential playback. */
  function enqueueSentence(sentence: string): void {
    const trimmed = stripMarkdown(sentence.trim());
    if (trimmed.length < MIN_SENTENCE_CHARS) return;

    // Capture generation at enqueue time so this sentence aborts if stop() is called.
    const myGen = generation;

    const synthPromise = invoke<number[]>('synthesize_tts', { text: trimmed })
      .then((bytes) => (generation === myGen ? new Uint8Array(bytes) : null))
      .catch((err) => {
        console.warn('synthesize_tts failed, will use browser fallback:', err);
        return null;
      });

    synthQueue.push({ text: trimmed, wav: synthPromise });

    // Drain queue if nothing is playing
    if (synthQueue.length === 1) {
      drainQueue(myGen);
    }
  }

  /** Play synthesized sentences from the queue one after another. */
  async function drainQueue(myGen: number): Promise<void> {
    while (synthQueue.length > 0 && generation === myGen) {
      const item = synthQueue[0];
      const wavBytes = await item.wav;
      synthQueue.shift();

      if (generation !== myGen) continue;

      isSpeaking.value = true;
      currentSentence.value = item.text;

      if (wavBytes && wavBytes.length > 44) {
        // WAV synthesis succeeded — play through HTMLAudioElement (enables lip sync).
        await playWavBytes(wavBytes, myGen);
      } else {
        // Synthesis failed or returned empty audio — fall back to browser speech.
        await speakWithBrowserTts(item.text, myGen);
      }

      // Mark sentence as fully spoken
      if (generation === myGen) {
        spokenText.value += (spokenText.value ? ' ' : '') + item.text;
      }
    }

    if (generation === myGen) {
      isSpeaking.value = false;
      currentSentence.value = '';
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
        audioEndCb?.();
        resolve();
      };

      audio.onerror = () => {
        currentAudio = null;
        audioEndCb?.();
        resolve();
      };

      audioStartCb?.(audio);
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

  /**
   * Browser-native fallback when backend TTS synthesis fails.
   * Uses the Web Speech API (SpeechSynthesis) which works on all platforms,
   * offline, and without any backend dependency.
   * Note: does not produce an HTMLAudioElement so lip sync is unavailable.
   */
  function speakWithBrowserTts(text: string, myGen: number): Promise<void> {
    return new Promise<void>((resolve) => {
      if (!('speechSynthesis' in window) || generation !== myGen) {
        audioEndCb?.();
        resolve();
        return;
      }

      const utterance = new SpeechSynthesisUtterance(text);
      utterance.pitch = options?.getBrowserPitch?.() ?? 1.0;
      utterance.rate = options?.getBrowserRate?.() ?? 1.0;
      utterance.onend = () => {
        audioEndCb?.();
        resolve();
      };
      utterance.onerror = () => {
        audioEndCb?.();
        resolve();
      };
      speechSynthesis.speak(utterance);
    });
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
    generation++;
    buffer = '';
    synthQueue.length = 0;
    isSpeaking.value = false;
    currentSentence.value = '';
    spokenText.value = '';

    if (currentAudio) {
      currentAudio.pause();
      currentAudio.src = '';
      currentAudio = null;
    }

    // Cancel any browser-native speech synthesis in progress.
    if (typeof speechSynthesis !== 'undefined') {
      speechSynthesis.cancel();
    }

    cleanupBlobUrls();
    playbackStopCb?.();
  }

  return {
    isSpeaking: readonly(isSpeaking),
    currentSentence: readonly(currentSentence),
    spokenText: readonly(spokenText),
    feedChunk,
    flush,
    stop,
    onAudioStart(cb: (audio: HTMLAudioElement) => void) { audioStartCb = cb; },
    onAudioEnd(cb: () => void) { audioEndCb = cb; },
    onPlaybackStop(cb: () => void) { playbackStopCb = cb; },
  };
}
