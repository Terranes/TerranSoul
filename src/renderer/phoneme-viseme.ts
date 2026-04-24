/**
 * phoneme-viseme.ts — Text-driven viseme scheduling.
 *
 * Maps English text → grapheme → approximate viseme shapes, then
 * distributes them evenly across an audio duration to produce a
 * timeline that can be sampled per animation frame.
 *
 * This replaces the FFT band-energy approach in `lip-sync.ts` with a
 * deterministic, phoneme-aware mapping. No LLM involved — purely a
 * lookup table + timing heuristic.
 *
 * The 5-channel viseme output (`aa`, `ih`, `ou`, `ee`, `oh`) matches
 * the existing `VisemeWeights` interface in `avatar-state.ts`.
 */

import type { VisemeWeights } from './avatar-state';

// ── Types ─────────────────────────────────────────────────────────────────────

export interface VisemeKeyframe {
  /** Time offset in seconds from the start of the utterance. */
  time: number;
  /** The viseme weights at this keyframe. */
  weights: VisemeWeights;
}

/** A single grapheme unit mapped to a viseme shape. */
interface GraphemeViseme {
  /** How many characters this grapheme consumes from the input text. */
  len: number;
  /** The viseme key this grapheme maps to, or 'silent' for closed mouth. */
  viseme: VisemeKey;
  /** Relative duration weight (vowels are longer, plosives shorter). */
  durationWeight: number;
}

type VisemeKey = 'aa' | 'ih' | 'ou' | 'ee' | 'oh' | 'silent';

// ── Viseme weight presets ─────────────────────────────────────────────────────

const VISEME_PRESETS: Record<VisemeKey, VisemeWeights> = {
  aa:     { aa: 0.85, ih: 0,    ou: 0,    ee: 0,    oh: 0.15 },
  ih:     { aa: 0,    ih: 0.85, ou: 0,    ee: 0.15, oh: 0    },
  ou:     { aa: 0,    ih: 0,    ou: 0.85, ee: 0,    oh: 0.15 },
  ee:     { aa: 0,    ih: 0.15, ou: 0,    ee: 0.85, oh: 0    },
  oh:     { aa: 0.15, ih: 0,    ou: 0.15, ee: 0,    oh: 0.70 },
  silent: { aa: 0,    ih: 0,    ou: 0,    ee: 0,    oh: 0    },
};

// ── Grapheme → viseme lookup ──────────────────────────────────────────────────

/**
 * Common English digraphs/trigraphs that should be handled before
 * single-character fallback. Checked in order (longest first).
 */
const DIGRAPH_MAP: Array<{ pattern: string; viseme: VisemeKey; dw: number }> = [
  // Trigraphs
  { pattern: 'tch', viseme: 'ee',     dw: 0.6 },
  { pattern: 'igh', viseme: 'ih',     dw: 1.2 },
  // Digraphs — consonants
  { pattern: 'th',  viseme: 'ee',     dw: 0.7 },
  { pattern: 'sh',  viseme: 'ou',     dw: 0.7 },
  { pattern: 'ch',  viseme: 'ee',     dw: 0.6 },
  { pattern: 'ph',  viseme: 'ih',     dw: 0.6 },
  { pattern: 'wh',  viseme: 'ou',     dw: 0.7 },
  { pattern: 'ck',  viseme: 'silent', dw: 0.3 },
  { pattern: 'ng',  viseme: 'silent', dw: 0.5 },
  { pattern: 'qu',  viseme: 'ou',     dw: 0.7 },
  // Digraphs — vowels
  { pattern: 'oo',  viseme: 'ou',     dw: 1.4 },
  { pattern: 'ee',  viseme: 'ee',     dw: 1.4 },
  { pattern: 'ea',  viseme: 'ee',     dw: 1.3 },
  { pattern: 'ai',  viseme: 'ee',     dw: 1.3 },
  { pattern: 'ay',  viseme: 'ee',     dw: 1.3 },
  { pattern: 'ou',  viseme: 'ou',     dw: 1.3 },
  { pattern: 'ow',  viseme: 'oh',     dw: 1.3 },
  { pattern: 'oi',  viseme: 'oh',     dw: 1.3 },
  { pattern: 'oy',  viseme: 'oh',     dw: 1.3 },
  { pattern: 'au',  viseme: 'oh',     dw: 1.3 },
  { pattern: 'aw',  viseme: 'oh',     dw: 1.3 },
];

/**
 * Single-character fallback map. Covers all 26 letters + common punctuation.
 * Vowels get higher duration weights (they're held longer in speech).
 */
const CHAR_MAP: Record<string, { viseme: VisemeKey; dw: number }> = {
  // Vowels
  a: { viseme: 'aa', dw: 1.2 },
  e: { viseme: 'ee', dw: 1.0 },
  i: { viseme: 'ih', dw: 1.0 },
  o: { viseme: 'oh', dw: 1.2 },
  u: { viseme: 'ou', dw: 1.0 },
  y: { viseme: 'ih', dw: 0.8 },
  // Labial consonants (lips close)
  b: { viseme: 'silent', dw: 0.3 },
  m: { viseme: 'silent', dw: 0.5 },
  p: { viseme: 'silent', dw: 0.3 },
  // Labiodental (lower lip + teeth)
  f: { viseme: 'ih', dw: 0.5 },
  v: { viseme: 'ih', dw: 0.5 },
  // Alveolar (tongue tip)
  d: { viseme: 'ee', dw: 0.4 },
  l: { viseme: 'ee', dw: 0.5 },
  n: { viseme: 'ee', dw: 0.5 },
  s: { viseme: 'ee', dw: 0.5 },
  t: { viseme: 'ee', dw: 0.3 },
  z: { viseme: 'ee', dw: 0.5 },
  // Post-alveolar / palatal (lips slightly rounded or spread)
  j: { viseme: 'ee', dw: 0.4 },
  r: { viseme: 'oh', dw: 0.6 },
  // Velar / glottal (back of mouth — minimal visible change)
  c: { viseme: 'ee', dw: 0.4 },
  g: { viseme: 'silent', dw: 0.3 },
  h: { viseme: 'aa', dw: 0.3 },
  k: { viseme: 'silent', dw: 0.3 },
  q: { viseme: 'ou', dw: 0.4 },
  w: { viseme: 'ou', dw: 0.6 },
  x: { viseme: 'ee', dw: 0.5 },
};

// ── Grapheme tokenizer ────────────────────────────────────────────────────────

/**
 * Tokenize text into grapheme-viseme pairs. Handles digraphs first,
 * then falls back to single characters.
 */
export function tokenizeToVisemes(text: string): GraphemeViseme[] {
  const result: GraphemeViseme[] = [];
  const lower = text.toLowerCase();
  let i = 0;

  while (i < lower.length) {
    // Skip non-alphabetic characters (spaces, punctuation, digits)
    if (!/[a-z]/.test(lower[i])) {
      // Whitespace / punctuation → micro-pause (silent)
      if (lower[i] === ' ' || lower[i] === ',' || lower[i] === '.' ||
          lower[i] === '!' || lower[i] === '?' || lower[i] === ';') {
        result.push({ len: 1, viseme: 'silent', durationWeight: 0.4 });
      }
      i++;
      continue;
    }

    // Try digraphs/trigraphs (longest match first)
    let matched = false;
    for (const dg of DIGRAPH_MAP) {
      if (lower.startsWith(dg.pattern, i)) {
        result.push({ len: dg.pattern.length, viseme: dg.viseme, durationWeight: dg.dw });
        i += dg.pattern.length;
        matched = true;
        break;
      }
    }
    if (matched) continue;

    // Single character
    const ch = lower[i];
    const entry = CHAR_MAP[ch];
    if (entry) {
      result.push({ len: 1, viseme: entry.viseme, durationWeight: entry.dw });
    }
    i++;
  }

  return result;
}

// ── Timeline builder ──────────────────────────────────────────────────────────

/**
 * Build a viseme timeline from text and audio duration.
 * Distributes grapheme-visemes proportionally across the duration
 * using their duration weights.
 */
export function buildVisemeTimeline(
  text: string,
  durationS: number,
): VisemeKeyframe[] {
  const tokens = tokenizeToVisemes(text);
  if (tokens.length === 0 || durationS <= 0) return [];

  const totalWeight = tokens.reduce((sum, t) => sum + t.durationWeight, 0);
  if (totalWeight <= 0) return [];

  const keyframes: VisemeKeyframe[] = [];
  let currentTime = 0;

  for (const token of tokens) {
    const segmentDuration = (token.durationWeight / totalWeight) * durationS;
    keyframes.push({
      time: currentTime,
      weights: { ...VISEME_PRESETS[token.viseme] },
    });
    currentTime += segmentDuration;
  }

  // Always end with silence
  keyframes.push({
    time: durationS,
    weights: { ...VISEME_PRESETS.silent },
  });

  return keyframes;
}

// ── Scheduler ─────────────────────────────────────────────────────────────────

/**
 * VisemeScheduler — samples a pre-built viseme timeline at arbitrary
 * timestamps. Used per-frame in the animation loop.
 */
export class VisemeScheduler {
  private timeline: VisemeKeyframe[];
  private startTime: number = 0;
  private _active: boolean = false;

  constructor() {
    this.timeline = [];
  }

  /** Load a new utterance. Call when TTS starts speaking a sentence. */
  schedule(text: string, durationS: number): void {
    this.timeline = buildVisemeTimeline(text, durationS);
    this.startTime = performance.now() / 1000;
    this._active = this.timeline.length > 0;
  }

  /** Whether the scheduler has an active utterance. */
  get active(): boolean {
    return this._active;
  }

  /**
   * Sample the viseme weights at the current time. Returns the
   * interpolated weights between the two nearest keyframes.
   */
  sample(): VisemeWeights {
    if (!this._active || this.timeline.length === 0) {
      return { ...VISEME_PRESETS.silent };
    }

    const elapsed = (performance.now() / 1000) - this.startTime;

    // Past the end → finished
    if (elapsed >= this.timeline[this.timeline.length - 1].time) {
      this._active = false;
      return { ...VISEME_PRESETS.silent };
    }

    // Find the two keyframes bracketing the current time
    let lo = 0;
    for (let i = 1; i < this.timeline.length; i++) {
      if (this.timeline[i].time > elapsed) break;
      lo = i;
    }
    const hi = Math.min(lo + 1, this.timeline.length - 1);

    if (lo === hi) return { ...this.timeline[lo].weights };

    // Lerp between lo and hi
    const loKf = this.timeline[lo];
    const hiKf = this.timeline[hi];
    const range = hiKf.time - loKf.time;
    const t = range > 0 ? (elapsed - loKf.time) / range : 0;

    return {
      aa: loKf.weights.aa + (hiKf.weights.aa - loKf.weights.aa) * t,
      ih: loKf.weights.ih + (hiKf.weights.ih - loKf.weights.ih) * t,
      ou: loKf.weights.ou + (hiKf.weights.ou - loKf.weights.ou) * t,
      ee: loKf.weights.ee + (hiKf.weights.ee - loKf.weights.ee) * t,
      oh: loKf.weights.oh + (hiKf.weights.oh - loKf.weights.oh) * t,
    };
  }

  /** Stop and clear the current utterance. */
  stop(): void {
    this.timeline = [];
    this._active = false;
  }
}
