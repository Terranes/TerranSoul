/**
 * generate-bgm.cjs — Generate JRPG-inspired BGM WAV files for TerranSoul.
 *
 * Creates 3 original compositions evoking classic JRPG soundtracks:
 *   1. Crystal Prelude — ascending/descending harp arpeggios
 *   2. Dream of Zanarkand — emotional piano ballad
 *   3. Promised Land — ethereal orchestral rest/save theme
 *
 * All tracks are original synthesized compositions — no copyrighted music.
 * They are designed to loop seamlessly as background ambient music.
 *
 * Run:  node scripts/generate-bgm.cjs
 * Output: public/audio/prelude.wav, moonflow.wav, sanctuary.wav
 *   (file IDs kept for backward compatibility with persisted settings)
 */

const fs = require('fs');
const path = require('path');

const SAMPLE_RATE = 44100;
const DURATION_S = 40; // 40-second seamless loops
const NUM_SAMPLES = Math.floor(SAMPLE_RATE * DURATION_S);

// ── WAV writer ────────────────────────────────────────────────────────────────

function writeWav(filePath, samplesL, samplesR, numSamples) {
  const numChannels = 2;
  const bitsPerSample = 16;
  const byteRate = SAMPLE_RATE * numChannels * (bitsPerSample / 8);
  const blockAlign = numChannels * (bitsPerSample / 8);
  const dataSize = numSamples * blockAlign;
  const buf = Buffer.alloc(44 + dataSize);

  buf.write('RIFF', 0);
  buf.writeUInt32LE(36 + dataSize, 4);
  buf.write('WAVE', 8);
  buf.write('fmt ', 12);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20); // PCM
  buf.writeUInt16LE(numChannels, 22);
  buf.writeUInt32LE(SAMPLE_RATE, 24);
  buf.writeUInt32LE(byteRate, 28);
  buf.writeUInt16LE(blockAlign, 32);
  buf.writeUInt16LE(bitsPerSample, 34);
  buf.write('data', 36);
  buf.writeUInt32LE(dataSize, 40);

  let offset = 44;
  for (let i = 0; i < numSamples; i++) {
    const l = Math.max(-1, Math.min(1, samplesL[i])) * 32767;
    const r = Math.max(-1, Math.min(1, samplesR[i])) * 32767;
    buf.writeInt16LE(Math.round(l), offset); offset += 2;
    buf.writeInt16LE(Math.round(r), offset); offset += 2;
  }

  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, buf);
  console.log(`  ✓ ${path.basename(filePath)} (${(buf.length / 1024).toFixed(0)} KB)`);
}

// ── Synthesis core ────────────────────────────────────────────────────────────

const TAU = 2 * Math.PI;
const sine = (p) => Math.sin(p);
const tri = (p) => { const x = ((p / TAU) % 1 + 1) % 1; return 4 * Math.abs(x - 0.5) - 1; };

/** Band-limited sawtooth via additive synthesis. */
function softSaw(phase, harmonics = 12) {
  let v = 0;
  for (let k = 1; k <= harmonics; k++) v += Math.sin(phase * k) / k * (k % 2 === 0 ? -1 : 1);
  return v * (2 / Math.PI) * 0.6;
}

/** Plucked-string model: bright attack decaying to sine. */
function pluck(phase, brightness) {
  const s = sine(phase);
  const t2 = tri(phase);
  return s * (1 - brightness) + t2 * brightness;
}

/** Stereo chorus with per-voice panning. */
function stereoChorus(t, freq, waveform, voices, spreadCents) {
  let l = 0, r = 0;
  for (let v = 0; v < voices; v++) {
    const detune = (v - (voices - 1) / 2) * spreadCents;
    const val = waveform(TAU * freq * Math.pow(2, detune / 1200) * t);
    const pan = v / (voices - 1 || 1); // 0=left, 1=right
    l += val * (1 - pan * 0.6);
    r += val * (0.4 + pan * 0.6);
  }
  return { l: l / voices, r: r / voices };
}

/** One-pole lowpass filter. */
function createLPF(cutoffHz) {
  const alpha = (1 / SAMPLE_RATE) / ((1 / (TAU * cutoffHz)) + (1 / SAMPLE_RATE));
  let prev = 0;
  return (x) => { prev += alpha * (x - prev); return prev; };
}

/** Two-pole (resonant) lowpass filter. */
function createResonantLPF(cutoffHz, resonance = 0.3) {
  const w = TAU * cutoffHz / SAMPLE_RATE;
  const cosw = Math.cos(w);
  const alpha = Math.sin(w) / (2 * (1 - resonance * 0.99));
  const b0 = (1 - cosw) / 2, b1 = 1 - cosw, b2 = (1 - cosw) / 2;
  const a0 = 1 + alpha, a1 = -2 * cosw, a2 = 1 - alpha;
  let x1 = 0, x2 = 0, y1 = 0, y2 = 0;
  return (x) => {
    const y = (b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0;
    x2 = x1; x1 = x; y2 = y1; y1 = y;
    return y;
  };
}

/** ADSR envelope. */
function adsr(t, noteStart, a, d, s, r, noteEnd) {
  const elapsed = t - noteStart;
  if (elapsed < 0) return 0;
  if (t > noteEnd) {
    const rel = t - noteEnd;
    return rel > r ? 0 : s * (1 - rel / r);
  }
  if (elapsed < a) return elapsed / a;
  if (elapsed < a + d) return 1 - (1 - s) * ((elapsed - a) / d);
  return s;
}

/** Exponential decay envelope for percussive/plucked sounds. */
function expDecay(t, noteStart, attack, decayRate) {
  const elapsed = t - noteStart;
  if (elapsed < 0) return 0;
  if (elapsed < attack) return elapsed / attack;
  return Math.exp(-(elapsed - attack) * decayRate);
}

/** Multi-tap delay for lush reverb. */
function createReverb(params) {
  const taps = params.map(p => ({
    buf: new Float32Array(Math.floor(SAMPLE_RATE * p.delay)),
    feedback: p.feedback,
    mix: p.mix,
    writePos: 0,
  }));
  return (x) => {
    let wet = 0;
    for (const tap of taps) {
      const delayed = tap.buf[tap.writePos];
      tap.writePos = (tap.writePos + 1) % tap.buf.length;
      wet += delayed * tap.mix;
    }
    return x * 0.6 + wet;
  };
}

/** Crossfade end into beginning for seamless loop. */
function crossfadeLoop(samples, fadeSamples) {
  for (let i = 0; i < fadeSamples; i++) {
    const fi = i / fadeSamples;
    const endIdx = samples.length - fadeSamples + i;
    samples[i] = samples[i] * fi + samples[endIdx] * (1 - fi);
  }
}

/** Soft limiter to prevent clipping while preserving dynamics. */
function softLimit(samples) {
  for (let i = 0; i < samples.length; i++) {
    const x = samples[i];
    samples[i] = x / (1 + Math.abs(x) * 0.3);
  }
}

/** Note name to frequency. Supports sharps/flats: C#4, Bb3, etc. */
function noteFreq(note, octave) {
  const names = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };
  let semitone = names[note[0]] || 0;
  if (note.includes('#')) semitone++;
  if (note.includes('b')) semitone--;
  const midi = 12 * (octave + 1) + semitone;
  return 440 * Math.pow(2, (midi - 69) / 12);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Track 1: CRYSTAL PRELUDE — Ascending/descending harp arpeggios
//   The iconic ascending C-D-E-G arpeggio through multiple octaves,
//   then descending, over warm string pads. Evokes the FF title screen.
// ═══════════════════════════════════════════════════════════════════════════════

function generateCrystalPrelude() {
  const L = new Float32Array(NUM_SAMPLES);
  const R = new Float32Array(NUM_SAMPLES);

  // Classic ascending arpeggio: C-D-E-G pattern through octaves, then descending
  const arpUp = [
    'C3','D3','E3','G3',
    'C4','D4','E4','G4',
    'C5','D5','E5','G5',
    'C6','D6','E6','G6',
  ];
  const arpDown = [
    'G6','E6','D6','C6',
    'G5','E5','D5','C5',
    'G4','E4','D4','C4',
    'G3','E3','D3','C3',
  ];
  // Second phrase with Am feel
  const arpUp2 = [
    'A2','C3','E3','A3',
    'C4','E4','A4','C5',
    'E5','A5','C6','E6',
  ];
  const arpDown2 = [
    'E6','C6','A5','E5',
    'C5','A4','E4','C4',
    'A3','E3','C3','A2',
  ];

  // Pattern sequence: each ~5s
  const patterns = [arpUp, arpDown, arpUp2, arpDown2, arpUp, arpDown, arpUp2, arpDown];
  const patternDur = DURATION_S / patterns.length;

  function parseNote(s) { return noteFreq(s.slice(0, -1), parseInt(s.slice(-1))); }

  // String pad: very soft, sustained Cmaj → Am alternation
  const padChords = [
    [noteFreq('C', 2), noteFreq('E', 2), noteFreq('G', 2)],
    [noteFreq('C', 2), noteFreq('E', 2), noteFreq('G', 2)],
    [noteFreq('A', 1), noteFreq('C', 2), noteFreq('E', 2)],
    [noteFreq('A', 1), noteFreq('C', 2), noteFreq('E', 2)],
    [noteFreq('C', 2), noteFreq('E', 2), noteFreq('G', 2)],
    [noteFreq('C', 2), noteFreq('E', 2), noteFreq('G', 2)],
    [noteFreq('A', 1), noteFreq('C', 2), noteFreq('E', 2)],
    [noteFreq('C', 2), noteFreq('E', 2), noteFreq('G', 2)],
  ];

  const reverbL = createReverb([
    { delay: 0.15, feedback: 0.35, mix: 0.20 },
    { delay: 0.33, feedback: 0.40, mix: 0.25 },
    { delay: 0.57, feedback: 0.30, mix: 0.18 },
    { delay: 0.83, feedback: 0.20, mix: 0.12 },
  ]);
  const reverbR = createReverb([
    { delay: 0.19, feedback: 0.35, mix: 0.20 },
    { delay: 0.41, feedback: 0.40, mix: 0.25 },
    { delay: 0.67, feedback: 0.30, mix: 0.18 },
    { delay: 0.91, feedback: 0.20, mix: 0.12 },
  ]);
  const lpfL = createResonantLPF(6000, 0.08);
  const lpfR = createResonantLPF(5800, 0.08);

  for (let i = 0; i < NUM_SAMPLES; i++) {
    const t = i / SAMPLE_RATE;
    let sl = 0, sr = 0;

    const patIdx = Math.min(Math.floor(t / patternDur), patterns.length - 1);
    const pat = patterns[patIdx];
    const patStart = patIdx * patternDur;
    const patTime = t - patStart;

    // ── String pad ──────────────────────────────────────────────────────
    const chord = padChords[patIdx];
    for (let v = 0; v < chord.length; v++) {
      const freq = chord[v];
      const lfo = 1 + 0.003 * Math.sin(TAU * 0.08 * t + v * 1.7);
      const sc = stereoChorus(t, freq * lfo, softSaw, 6, 8);
      const vol = 0.025 * (0.5 + 0.5 * Math.sin(TAU * 0.03 * t + v));
      sl += sc.l * vol;
      sr += sc.r * vol;
    }

    // ── Harp arpeggios — fast ascending/descending notes ────────────────
    const noteInterval = patternDur / pat.length;
    for (let n = 0; n < pat.length; n++) {
      const nStart = n * noteInterval;
      const freq = parseNote(pat[n]);
      const env = expDecay(patTime, nStart, 0.002, 2.0);
      if (env > 0.001) {
        const brightness = Math.max(0, 1 - (patTime - nStart) * 1.2);
        const val = pluck(TAU * freq * t, brightness * 0.5) * env * 0.16;
        // Stereo spread: low notes left, high notes right
        const pan = 0.1 + 0.8 * (n / pat.length);
        sl += val * (1 - pan);
        sr += val * pan;
        // Octave shimmer
        const shimmer = sine(TAU * freq * 2 * t) * env * 0.04 * brightness;
        sl += shimmer * (1 - pan);
        sr += shimmer * pan;
      }
    }

    // ── Deep bass pulse ─────────────────────────────────────────────────
    const bassFreq = chord[0] * 0.5;
    sl += sine(TAU * bassFreq * t) * 0.015;
    sr += sine(TAU * bassFreq * t) * 0.015;

    L[i] = lpfL(reverbL(sl));
    R[i] = lpfR(reverbR(sr));
  }

  crossfadeLoop(L, Math.floor(SAMPLE_RATE * 3));
  crossfadeLoop(R, Math.floor(SAMPLE_RATE * 3));
  softLimit(L);
  softLimit(R);
  return { L, R };
}

// ═══════════════════════════════════════════════════════════════════════════════
// Track 2: DREAM OF ZANARKAND — Emotional piano ballad
//   A melancholic, intimate piano piece with gentle string accompaniment.
//   Key: Ab major, slow tempo, expressive dynamics.
// ═══════════════════════════════════════════════════════════════════════════════

function generateDreamOfZanarkand() {
  const L = new Float32Array(NUM_SAMPLES);
  const R = new Float32Array(NUM_SAMPLES);

  // Chord progression: Ab → Fm → Db → Eb (×2), then Bbm → Eb → Ab → Ab
  const chordProg = [
    { notes: ['Ab2','C3','Eb3','Ab3'], dur: 5 },
    { notes: ['F2','Ab2','C3','F3'], dur: 5 },
    { notes: ['Db2','F2','Ab2','Db3'], dur: 5 },
    { notes: ['Eb2','G2','Bb2','Eb3'], dur: 5 },
    { notes: ['Ab2','C3','Eb3','Ab3'], dur: 5 },
    { notes: ['F2','Ab2','C3','Eb3'], dur: 5 },
    { notes: ['Bb1','Db2','F2','Bb2'], dur: 5 },
    { notes: ['Eb2','G2','Bb2','Eb3'], dur: 5 },
  ];

  function parseNote(s) {
    const m = s.match(/^([A-G][b#]?)(\d)$/);
    return noteFreq(m[1], parseInt(m[2]));
  }

  // Piano melody — melancholic, lyrical, with long phrases
  const melody = [
    // Phrase 1: gentle opening
    { n: 'Eb5', s: 0.5, d: 2.0 }, { n: 'C5', s: 2.8, d: 1.5 },
    { n: 'Ab4', s: 4.5, d: 2.5 }, { n: 'Bb4', s: 7.2, d: 1.5 },
    { n: 'C5', s: 9.0, d: 2.0 },
    // Phrase 2: ascending emotion
    { n: 'Db5', s: 11.5, d: 2.5 }, { n: 'Eb5', s: 14.2, d: 1.5 },
    { n: 'F5', s: 16.0, d: 2.0 }, { n: 'Eb5', s: 18.5, d: 1.5 },
    // Phrase 3: emotional peak
    { n: 'Ab5', s: 20.5, d: 3.0 }, { n: 'G5', s: 23.8, d: 1.5 },
    { n: 'F5', s: 25.5, d: 2.0 }, { n: 'Eb5', s: 28.0, d: 1.5 },
    // Phrase 4: gentle resolve
    { n: 'Db5', s: 30.0, d: 2.5 }, { n: 'C5', s: 32.8, d: 2.0 },
    { n: 'Ab4', s: 35.0, d: 3.5 }, { n: 'Eb4', s: 38.5, d: 1.5 },
  ];

  // Build chord timeline
  let chordOffset = 0;
  const chordTimeline = chordProg.map(c => {
    const entry = { ...c, start: chordOffset, freqs: c.notes.map(parseNote) };
    chordOffset += c.dur;
    return entry;
  });

  // Piano left-hand broken chord pattern (gentle, not too busy)
  function pianoLeftHand(t, chordFreqs, chordStart, chordDur) {
    let sl = 0, sr = 0;
    const noteInterval = chordDur / 6;
    const pattern = [0, 2, 1, 3, 2, 1]; // gentle arpeggiation
    for (let p = 0; p < 6; p++) {
      const pickStart = chordStart + p * noteInterval;
      const noteIdx = pattern[p];
      const freq = chordFreqs[Math.min(noteIdx, chordFreqs.length - 1)];
      const env = expDecay(t, pickStart, 0.005, 1.8);
      if (env > 0.001) {
        // Piano timbre: sine + harmonics with quick attack
        const val = (sine(TAU * freq * t) * 0.7
          + sine(TAU * freq * 2 * t) * 0.2
          + sine(TAU * freq * 3 * t) * 0.08
          + sine(TAU * freq * 4 * t) * 0.02) * env * 0.10;
        // Slightly left-of-center for bass register
        sl += val * 0.65;
        sr += val * 0.45;
      }
    }
    return { sl, sr };
  }

  const reverbL = createReverb([
    { delay: 0.13, feedback: 0.35, mix: 0.22 },
    { delay: 0.31, feedback: 0.30, mix: 0.18 },
    { delay: 0.53, feedback: 0.25, mix: 0.14 },
    { delay: 0.79, feedback: 0.20, mix: 0.10 },
    { delay: 1.1, feedback: 0.15, mix: 0.06 },
  ]);
  const reverbR = createReverb([
    { delay: 0.17, feedback: 0.35, mix: 0.22 },
    { delay: 0.37, feedback: 0.30, mix: 0.18 },
    { delay: 0.61, feedback: 0.25, mix: 0.14 },
    { delay: 0.89, feedback: 0.20, mix: 0.10 },
    { delay: 1.2, feedback: 0.15, mix: 0.06 },
  ]);
  const lpfL = createResonantLPF(5500, 0.1);
  const lpfR = createResonantLPF(5300, 0.1);

  for (let i = 0; i < NUM_SAMPLES; i++) {
    const t = i / SAMPLE_RATE;
    let sl = 0, sr = 0;

    // ── Piano left hand (broken chords) ─────────────────────────────────
    for (const ch of chordTimeline) {
      if (t >= ch.start && t < ch.start + ch.dur + 1.5) {
        const lh = pianoLeftHand(t, ch.freqs, ch.start, ch.dur);
        sl += lh.sl;
        sr += lh.sr;
      }
    }

    // ── Soft string pad (very background, for warmth) ───────────────────
    const chordIdx = chordTimeline.findIndex(c => t >= c.start && t < c.start + c.dur);
    const activeChord = chordTimeline[Math.max(0, chordIdx === -1 ? 0 : chordIdx)];
    if (activeChord) {
      for (const freq of activeChord.freqs) {
        const lfo = 1 + 0.003 * Math.sin(TAU * 0.07 * t + freq * 0.01);
        const sc = stereoChorus(t, freq * 2 * lfo, softSaw, 4, 6);
        sl += sc.l * 0.008;
        sr += sc.r * 0.008;
      }
    }

    // ── Piano melody (right hand) ───────────────────────────────────────
    for (const m of melody) {
      const freq = parseNote(m.n);
      const noteEnd = m.s + m.d;
      const env = adsr(t, m.s, 0.01, 0.4, 0.55, 1.2, noteEnd);
      if (env > 0.001) {
        // Piano timbre with expressive dynamics
        const vibrato = 1 + 0.003 * Math.sin(TAU * 5 * t) * Math.min(1, (t - m.s) * 0.5);
        const val = (sine(TAU * freq * vibrato * t) * 0.65
          + sine(TAU * freq * 2 * vibrato * t) * 0.22
          + sine(TAU * freq * 3 * vibrato * t) * 0.08
          + sine(TAU * freq * 5 * vibrato * t) * 0.03) * env * 0.13;
        // Slightly right-of-center for treble register
        sl += val * 0.45;
        sr += val * 0.65;
      }
    }

    // ── Gentle sustain pedal resonance ───────────────────────────────────
    if (activeChord) {
      const bassFreq = activeChord.freqs[0];
      const resonance = sine(TAU * bassFreq * 0.5 * t) * 0.008
        * (0.5 + 0.5 * Math.sin(TAU * 0.02 * t));
      sl += resonance;
      sr += resonance;
    }

    L[i] = lpfL(reverbL(sl));
    R[i] = lpfR(reverbR(sr));
  }

  crossfadeLoop(L, Math.floor(SAMPLE_RATE * 3));
  crossfadeLoop(R, Math.floor(SAMPLE_RATE * 3));
  softLimit(L);
  softLimit(R);
  return { L, R };
}

// ═══════════════════════════════════════════════════════════════════════════════
// Track 3: PROMISED LAND — Ethereal orchestral rest/save theme
//   Gentle music box melody, warm orchestral pads, and celeste bells.
//   Key: Db major / Bbm, spacious and serene.
// ═══════════════════════════════════════════════════════════════════════════════

function generatePromisedLand() {
  const L = new Float32Array(NUM_SAMPLES);
  const R = new Float32Array(NUM_SAMPLES);

  // Orchestral pad progression: Db → Bbm → Gb → Ab
  const pads = [
    { freqs: [noteFreq('Db', 2), noteFreq('F', 2), noteFreq('Ab', 2), noteFreq('C', 3), noteFreq('Eb', 3)], dur: 10 },
    { freqs: [noteFreq('Bb', 1), noteFreq('Db', 2), noteFreq('F', 2), noteFreq('Ab', 2), noteFreq('C', 3)], dur: 10 },
    { freqs: [noteFreq('Gb', 1), noteFreq('Bb', 1), noteFreq('Db', 2), noteFreq('F', 2), noteFreq('Ab', 2)], dur: 10 },
    { freqs: [noteFreq('Ab', 1), noteFreq('C', 2), noteFreq('Eb', 2), noteFreq('Ab', 2), noteFreq('Bb', 2)], dur: 10 },
  ];

  // Music box melody — gentle, nostalgic, slightly melancholic
  const musicBox = [
    { n: 'Ab5', s: 1.0, d: 2.0 }, { n: 'F5', s: 3.5, d: 1.5 },
    { n: 'Db5', s: 5.5, d: 2.5 }, { n: 'Eb5', s: 8.5, d: 1.5 },
    { n: 'F5', s: 10.5, d: 2.5 }, { n: 'Ab5', s: 13.5, d: 2.0 },
    { n: 'Bb5', s: 16.0, d: 3.0 }, { n: 'Ab5', s: 19.5, d: 1.5 },
    { n: 'F5', s: 21.5, d: 2.5 }, { n: 'Db5', s: 24.5, d: 2.0 },
    { n: 'C5', s: 27.0, d: 2.5 }, { n: 'Db5', s: 30.0, d: 3.0 },
    { n: 'Eb5', s: 33.5, d: 2.0 }, { n: 'Ab4', s: 36.0, d: 3.5 },
  ];

  function parseNote(s) {
    const m = s.match(/^([A-G][b#]?)(\d)$/);
    const m = s.match(/^([A-G][b#]?)(\d)$/);
  }

  // Build pad timeline
  let padOffset = 0;
  const padTimeline = pads.map(p => {
    const entry = { ...p, start: padOffset };
    padOffset += p.dur;
    return entry;
  });

  const reverbL = createReverb([
    { delay: 0.21, feedback: 0.42, mix: 0.28 },
    { delay: 0.43, feedback: 0.38, mix: 0.22 },
    { delay: 0.67, feedback: 0.32, mix: 0.18 },
    { delay: 0.97, feedback: 0.25, mix: 0.14 },
    { delay: 1.3, feedback: 0.18, mix: 0.08 },
  ]);
  const reverbR = createReverb([
    { delay: 0.25, feedback: 0.42, mix: 0.28 },
    { delay: 0.49, feedback: 0.38, mix: 0.22 },
    { delay: 0.77, feedback: 0.32, mix: 0.18 },
    { delay: 1.07, feedback: 0.25, mix: 0.14 },
    { delay: 1.4, feedback: 0.18, mix: 0.08 },
  ]);
  const lpfL = createLPF(3800);
  const lpfR = createLPF(3600);

  for (let i = 0; i < NUM_SAMPLES; i++) {
    const t = i / SAMPLE_RATE;
    let sl = 0, sr = 0;

    // ── Orchestral string pad ───────────────────────────────────────────
    for (const pad of padTimeline) {
      if (t < pad.start - 1 || t > pad.start + pad.dur + 2) continue;
      const padEnv = adsr(t, pad.start, 2.5, 1.5, 0.65, 2.5, pad.start + pad.dur);
      if (padEnv < 0.001) continue;

      for (let v = 0; v < pad.freqs.length; v++) {
        const freq = pad.freqs[v];
        const lfo = 1 + 0.004 * Math.sin(TAU * 0.05 * t + v * 1.3);
        const breathe = 0.5 + 0.5 * Math.sin(TAU * 0.025 * t + v * 0.7);
        const sc = stereoChorus(t, freq * lfo, softSaw, 6, 10);
        const vol = 0.032 * padEnv * breathe;
        sl += sc.l * vol;
        sr += sc.r * vol;
        // Warm sine layer
        sl += sine(TAU * freq * t) * 0.018 * padEnv * breathe;
        sr += sine(TAU * freq * t) * 0.018 * padEnv * breathe;
      }
    }

    // ── Music box melody ────────────────────────────────────────────────
    for (const note of musicBox) {
      const freq = parseNote(note.n);
      const env = expDecay(t, note.s, 0.008, 0.9);
      if (env < 0.001) continue;

      // Music box: bright sine + inharmonic partials
      const fundamental = sine(TAU * freq * t);
      const partial2 = sine(TAU * freq * 2.01 * t) * 0.4;
      const partial3 = sine(TAU * freq * 3.02 * t) * 0.15;
      const partial5 = sine(TAU * freq * 5.03 * t) * 0.06;
      const boxVal = (fundamental + partial2 + partial3 + partial5) * env * 0.09;

      const pan = 0.3 + 0.4 * Math.sin(freq * 0.008 + 0.5);
      sl += boxVal * (1 - pan);
      sr += boxVal * pan;
    }

    // ── Celeste bell accents (sparse, on chord changes) ─────────────────
    for (const pad of padTimeline) {
      const bellStart = pad.start + 0.5;
      const bellFreq = pad.freqs[pad.freqs.length - 1] * 2;
      const bellEnv = expDecay(t, bellStart, 0.005, 0.5);
      if (bellEnv > 0.001) {
        const bell = sine(TAU * bellFreq * t) * bellEnv * 0.04;
        sl += bell * 0.4;
        sr += bell * 0.7;
      }
    }

    // ── Ethereal high shimmer ───────────────────────────────────────────
    const shimmerFreq = noteFreq('Db', 7) + 40 * Math.sin(TAU * 0.018 * t);
    const shimmer = sine(TAU * shimmerFreq * t) * 0.005
      * (0.3 + 0.7 * Math.sin(TAU * 0.04 * t))
      * (0.5 + 0.5 * Math.sin(TAU * 0.06 * t + 1.2));
    sl += shimmer * 0.5;
    sr += shimmer;

    // ── Deep bass warmth ────────────────────────────────────────────────
    const currentPad = padTimeline.find(p => t >= p.start && t < p.start + p.dur);
    if (currentPad) {
      const bassFreq = currentPad.freqs[0] * 0.5;
      const bassEnv = 0.5 + 0.5 * Math.sin(TAU * 0.018 * t);
      sl += sine(TAU * bassFreq * t) * 0.015 * bassEnv;
      sr += sine(TAU * bassFreq * t) * 0.015 * bassEnv;
    }

    // ── Gentle breath texture ───────────────────────────────────────────
    const breath = (Math.random() * 2 - 1) * 0.004
      * (0.2 + 0.8 * Math.sin(TAU * 0.012 * t))
      * (0.3 + 0.7 * Math.sin(TAU * 0.035 * t + 1.5));
    sl += breath;
    sr += breath * 0.6;

    L[i] = lpfL(reverbL(sl));
    R[i] = lpfR(reverbR(sr));
  }

  crossfadeLoop(L, Math.floor(SAMPLE_RATE * 3));
  crossfadeLoop(R, Math.floor(SAMPLE_RATE * 3));
  softLimit(L);
  softLimit(R);
  return { L, R };
}

// ── Main ──────────────────────────────────────────────────────────────────────

const outDir = path.join(__dirname, '..', 'public', 'audio');
console.log('Generating JRPG-inspired BGM tracks (40s seamless loops)...\n');

console.log('  1/3 Crystal Prelude (ascending/descending harp arpeggios)...');
const crystal = generateCrystalPrelude();
writeWav(path.join(outDir, 'prelude.wav'), crystal.L, crystal.R, NUM_SAMPLES);

console.log('  2/3 Dream of Zanarkand (emotional piano ballad)...');
const zanarkand = generateDreamOfZanarkand();
writeWav(path.join(outDir, 'moonflow.wav'), zanarkand.L, zanarkand.R, NUM_SAMPLES);

console.log('  3/3 Promised Land (ethereal orchestral rest theme)...');
const promised = generatePromisedLand();
writeWav(path.join(outDir, 'sanctuary.wav'), promised.L, promised.R, NUM_SAMPLES);

console.log('\nDone! Files saved to public/audio/');
