/**
 * Stat-driven AI behaviour modifiers (Chunk 134).
 *
 * Pure functions that translate the six RPG stats into knobs that downstream
 * subsystems can read to scale their behaviour. Kept side-effect-free so they
 * can be safely consumed from anywhere — Pinia stores, composables, tests.
 *
 * The intent is *gentle* scaling: a fresh install behaves identically to the
 * previous defaults, while a fully-levelled brain gets meaningfully larger
 * context windows, deeper memory recall, etc.
 */

import type { StatSnapshot } from './stats';

const DEFAULT_RECALL_LIMIT = 10;
const DEFAULT_HISTORY = 20;

/**
 * Wisdom-driven memory recall depth.
 *
 * Wisdom 0  → 8  results
 * Wisdom 50 → ~13 results
 * Wisdom 100 → 20 results
 */
export function getMemoryRecallLimit(stats: StatSnapshot): number {
  const baseline = Math.round(8 + (stats.wisdom / 100) * 12);
  return Math.max(DEFAULT_RECALL_LIMIT - 2, Math.min(20, baseline));
}

/**
 * Intelligence-driven multiplier for chat history kept in the prompt.
 *
 * INT  0 → 1.0× (default)
 * INT 50 → ~1.25×
 * INT 100 → 1.5×
 */
export function getContextWindowMultiplier(stats: StatSnapshot): number {
  return 1 + (stats.intelligence / 100) * 0.5;
}

/** Convenience helper that applies the multiplier to a base history count. */
export function getChatHistoryLimit(stats: StatSnapshot, base = DEFAULT_HISTORY): number {
  return Math.round(base * getContextWindowMultiplier(stats));
}

/**
 * Perception-driven hotword detection sensitivity.
 *
 * Returns a value in `[0.5, 1.5]` where >1 means "easier to trigger".
 * Detector code can multiply its score against this to get the effective
 * sensitivity.
 */
export function getHotwordSensitivity(stats: StatSnapshot): number {
  return 0.5 + (stats.perception / 100);
}

/**
 * Charisma-driven TTS expressiveness in `[0, 1]`. Voice backends that support
 * a "style strength" knob (Edge TTS prosody, ElevenLabs `stability`, etc.) can
 * map this directly.
 */
export function getTtsExpressiveness(stats: StatSnapshot): number {
  return Math.min(1, Math.max(0, stats.charisma / 100));
}

/** Single-call helper for UI surfaces that want to display all modifiers. */
export interface StatModifiers {
  memoryRecallLimit: number;
  contextWindowMultiplier: number;
  chatHistoryLimit: number;
  hotwordSensitivity: number;
  ttsExpressiveness: number;
}

export function computeModifiers(stats: StatSnapshot): StatModifiers {
  return {
    memoryRecallLimit: getMemoryRecallLimit(stats),
    contextWindowMultiplier: getContextWindowMultiplier(stats),
    chatHistoryLimit: getChatHistoryLimit(stats),
    hotwordSensitivity: getHotwordSensitivity(stats),
    ttsExpressiveness: getTtsExpressiveness(stats),
  };
}
