/**
 * Audio analysis Web Worker — performs RMS volume calculation and
 * 5-band frequency energy extraction off the main thread.
 *
 * Message protocol:
 *
 * Main → Worker:
 *   { type: 'analyze', timeDomain: Float32Array, frequency: Uint8Array, sensitivity: number }
 *   (Float32Array sent as transferable for zero-copy)
 *
 * Worker → Main:
 *   { type: 'result', volume: number, visemes: { aa, ih, ou, ee, oh } }
 *
 * Main → Worker:
 *   { type: 'configure', silenceThreshold: number, sensitivity: number }
 *
 * Worker → Main:
 *   { type: 'configured' }
 */

// ── Types (duplicated to avoid import issues in worker scope) ────────────────

export interface AnalyzeMessage {
  type: 'analyze';
  timeDomain: Float32Array;
  frequency: Uint8Array;
  sensitivity: number;
}

export interface ConfigureMessage {
  type: 'configure';
  silenceThreshold: number;
  sensitivity: number;
}

export type WorkerInMessage = AnalyzeMessage | ConfigureMessage;

export interface AnalyzeResult {
  type: 'result';
  volume: number;
  visemes: { aa: number; ih: number; ou: number; ee: number; oh: number };
}

export interface ConfiguredResult {
  type: 'configured';
}

export type WorkerOutMessage = AnalyzeResult | ConfiguredResult;

// ── Frequency band boundaries (fraction of Nyquist) ─────────────────────────
const BAND_EDGES = [0, 0.12, 0.25, 0.45, 0.65, 1.0] as const;

// ── Worker state ─────────────────────────────────────────────────────────────
let silenceThreshold = 0.01;
let defaultSensitivity = 1.5;

// ── Pure computation functions (exported for testing) ────────────────────────

export function calculateRMS(data: Float32Array): number {
  let sum = 0;
  for (let i = 0; i < data.length; i++) {
    sum += data[i] * data[i];
  }
  return Math.sqrt(sum / data.length);
}

export function computeBandEnergies(
  freqData: Uint8Array,
  binCount: number,
): [number, number, number, number, number] {
  const result: [number, number, number, number, number] = [0, 0, 0, 0, 0];

  for (let b = 0; b < 5; b++) {
    const startBin = Math.floor(BAND_EDGES[b] * binCount);
    const endBin = Math.min(binCount, Math.floor(BAND_EDGES[b + 1] * binCount));
    const count = endBin - startBin;
    if (count <= 0) continue;

    let sum = 0;
    for (let i = startBin; i < endBin; i++) {
      sum += freqData[i];
    }
    // freqData values are 0–255; normalize to 0–1
    result[b] = (sum / count) / 255;
  }

  return result;
}

function clamp01(v: number): number {
  return v < 0 ? 0 : v > 1 ? 1 : v;
}

export function analyzeAudio(
  timeDomain: Float32Array,
  frequency: Uint8Array,
  sensitivity: number,
  threshold: number,
): AnalyzeResult {
  const volume = calculateRMS(timeDomain);
  const zeroVisemes = { aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 };

  if (volume < threshold) {
    return { type: 'result', volume, visemes: zeroVisemes };
  }

  const binCount = frequency.length;
  const bands = computeBandEnergies(frequency, binCount);
  const sens = sensitivity;

  return {
    type: 'result',
    volume,
    visemes: {
      aa: clamp01(bands[0] * sens),
      ou: clamp01(bands[1] * sens),
      oh: clamp01(bands[2] * sens),
      ee: clamp01(bands[3] * sens),
      ih: clamp01(bands[4] * sens),
    },
  };
}

// ── Worker message handler ───────────────────────────────────────────────────

const ctx = globalThis as unknown as { onmessage: ((e: MessageEvent) => void) | null; postMessage: (msg: WorkerOutMessage) => void };

if (typeof ctx.postMessage === 'function') {
  ctx.onmessage = (e: MessageEvent<WorkerInMessage>) => {
    const msg = e.data;

    if (msg.type === 'configure') {
      silenceThreshold = msg.silenceThreshold;
      defaultSensitivity = msg.sensitivity;
      ctx.postMessage({ type: 'configured' });
      return;
    }

    if (msg.type === 'analyze') {
      const result = analyzeAudio(
        msg.timeDomain,
        msg.frequency,
        msg.sensitivity ?? defaultSensitivity,
        silenceThreshold,
      );
      ctx.postMessage(result);
    }
  };
}
