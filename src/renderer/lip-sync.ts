/**
 * Web Audio API-based lip sync engine.
 *
 * Analyses audio output (from TTS) in real-time using an `AnalyserNode`
 * and maps volume/frequency levels to VRM mouth morph targets.
 *
 * Supports two modes:
 *   - **5-channel viseme** (`aa`, `ih`, `ou`, `ee`, `oh`) via FFT frequency-band
 *     analysis. Each band drives a specific mouth shape.
 *   - **2-channel fallback** (`aa`, `oh`) via simple RMS volume mapping.
 *
 * Provider-agnostic — works with any audio source that can be connected to
 * the Web Audio API (HTMLAudioElement, MediaStream, AudioBufferSource, etc.).
 *
 * Usage:
 *   const lipSync = new LipSync();
 *   lipSync.connectAudioElement(audioElement);
 *   // In your animation loop:
 *   const visemes = lipSync.getVisemeValues();
 *   // Feed into AvatarState or apply directly to VRM expression manager
 */

import type { VisemeWeights } from './avatar-state';
import type { WorkerInMessage, WorkerOutMessage } from '../workers/audio-analyzer.worker';

export interface MouthValues {
  /** Mouth open (primary morph: wide open, "ah" shape). Range 0–1. */
  aa: number;
  /** Mouth round (secondary morph: rounded, "oh" shape). Range 0–1. */
  oh: number;
}

/** Full 5-channel viseme weights matching VRM viseme blend shapes. */
export type VisemeValues = VisemeWeights;

export interface LipSyncOptions {
  /** FFT size for the analyser. Default: 256 (128 frequency bins). */
  fftSize?: number;
  /** Smoothing constant for the analyser. Default: 0.5. */
  smoothingTimeConstant?: number;
  /** Volume threshold below which mouth is closed. Default: 0.01. */
  silenceThreshold?: number;
  /** Sensitivity multiplier for volume → mouth mapping. Default: 1.5. */
  sensitivity?: number;
}

const DEFAULT_OPTIONS: Required<LipSyncOptions> = {
  fftSize: 256,
  smoothingTimeConstant: 0.5,
  silenceThreshold: 0.01,
  sensitivity: 1.5,
};

/**
 * Frequency band boundaries (as fraction of Nyquist frequency).
 * Tuned for speech:
 *   low  (0–0.12)  → aa  (open jaw, "ah")
 *   mLow (0.12–0.25) → ou (round lips, "oo")
 *   mid  (0.25–0.45) → oh (half-round, "oh")
 *   mHi  (0.45–0.65) → ee (spread lips, "ee")
 *   hi   (0.65–1.0)  → ih (narrow, "ih")
 */
const BAND_EDGES = [0, 0.12, 0.25, 0.45, 0.65, 1.0] as const;

export class LipSync {
  private context: AudioContext | null = null;
  private analyser: AnalyserNode | null = null;
  private source: MediaElementAudioSourceNode | null = null;
  private timeDomainData: Float32Array<ArrayBuffer> | null = null;
  private frequencyData: Uint8Array<ArrayBuffer> | null = null;
  private options: Required<LipSyncOptions>;
  private _active = false;
  private _volume = 0;

  // Worker-based off-thread analysis
  private worker: Worker | null = null;
  private _workerReady = false;
  private _lastWorkerVisemes: VisemeValues = { aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 };
  private _lastWorkerVolume = 0;
  private _pendingWorkerResult = false;

  constructor(options: LipSyncOptions = {}) {
    this.options = { ...DEFAULT_OPTIONS, ...options };
  }

  /** Whether the lip sync is currently connected to an audio source. */
  get active(): boolean {
    return this._active;
  }

  /** Current raw volume level (0–1 range after clamping). */
  get volume(): number {
    return this._volume;
  }

  /** Whether the worker-based off-thread analysis is active. */
  get workerReady(): boolean {
    return this._workerReady;
  }

  /**
   * Enable off-thread audio analysis via Web Worker.
   * Call once at startup. If the worker fails to load, lip sync
   * falls back to main-thread analysis transparently.
   */
  enableWorker(): void {
    if (this.worker) return;
    try {
      this.worker = new Worker(
        new URL('../workers/audio-analyzer.worker.ts', import.meta.url),
        { type: 'module' },
      );
      this.worker.onmessage = (e: MessageEvent<WorkerOutMessage>) => {
        const msg = e.data;
        if (msg.type === 'configured') {
          this._workerReady = true;
        } else if (msg.type === 'result') {
          this._lastWorkerVolume = msg.volume;
          this._lastWorkerVisemes = msg.visemes;
          this._pendingWorkerResult = false;
        }
      };
      this.worker.onerror = () => {
        this.disableWorker();
      };
      // Send initial configuration
      const cfgMsg: WorkerInMessage = {
        type: 'configure',
        silenceThreshold: this.options.silenceThreshold,
        sensitivity: this.options.sensitivity,
      };
      this.worker.postMessage(cfgMsg);
    } catch {
      // Worker not supported or URL resolution failed — stay on main thread
      this.worker = null;
    }
  }

  /** Disable the Web Worker and revert to main-thread analysis. */
  disableWorker(): void {
    if (this.worker) {
      this.worker.terminate();
      this.worker = null;
    }
    this._workerReady = false;
    this._pendingWorkerResult = false;
  }

  /**
   * Connect to an HTMLAudioElement and start analysing.
   * The audio element must be playing or about to play.
   */
  connectAudioElement(audioElement: HTMLAudioElement): void {
    this.disconnect();

    this.context = new AudioContext();
    this.analyser = this.context.createAnalyser();
    this.analyser.fftSize = this.options.fftSize;
    this.analyser.smoothingTimeConstant = this.options.smoothingTimeConstant;

    this.source = this.context.createMediaElementSource(audioElement);
    this.source.connect(this.analyser);
    this.analyser.connect(this.context.destination);

    this.timeDomainData = new Float32Array(this.analyser.fftSize);
    this.frequencyData = new Uint8Array(this.analyser.frequencyBinCount);
    this._active = true;
  }

  /**
   * Connect to an existing AnalyserNode (e.g. from an AudioContext you manage).
   * Does NOT create its own AudioContext — you must manage the graph.
   */
  connectAnalyser(analyser: AnalyserNode): void {
    this.disconnect();

    this.analyser = analyser;
    this.timeDomainData = new Float32Array(analyser.fftSize);
    this.frequencyData = new Uint8Array(analyser.frequencyBinCount);
    this._active = true;
  }

  /** Disconnect from the current audio source and release resources. */
  disconnect(): void {
    this._active = false;
    this._volume = 0;

    if (this.source) {
      try {
        this.source.disconnect();
      } catch {
        // Already disconnected
      }
      this.source = null;
    }

    if (this.context) {
      try {
        this.context.close();
      } catch {
        // Already closed
      }
      this.context = null;
    }

    this.analyser = null;
    this.timeDomainData = null;
    this.frequencyData = null;
    this.disableWorker();
  }

  /**
   * Compute current mouth morph values from audio analysis (2-channel fallback).
   * Call this every frame in your requestAnimationFrame loop.
   *
   * Returns { aa: 0, oh: 0 } when no audio is connected or below threshold.
   */
  getMouthValues(): MouthValues {
    if (!this._active || !this.analyser || !this.timeDomainData) {
      return { aa: 0, oh: 0 };
    }

    // Read time-domain data (waveform)
    this.analyser.getFloatTimeDomainData(this.timeDomainData);

    // Calculate RMS (root mean square) volume
    const rms = this.calculateRMS(this.timeDomainData);
    this._volume = rms;

    // Apply silence threshold
    if (rms < this.options.silenceThreshold) {
      return { aa: 0, oh: 0 };
    }

    // Map volume to mouth values with sensitivity
    const scaledVolume = Math.min(1, rms * this.options.sensitivity);

    // aa (wide open) tracks the overall volume
    // oh (rounded) responds to higher frequencies / stronger volume
    const aa = scaledVolume * 0.8;
    const oh = scaledVolume * scaledVolume * 0.5; // Quadratic for more subtle rounding

    return {
      aa: Math.min(1, aa),
      oh: Math.min(1, oh),
    };
  }

  /**
   * Compute 5-channel VRM viseme weights from FFT frequency-band analysis.
   * Call this every frame in your requestAnimationFrame loop.
   *
   * Returns all-zero visemes when no audio is connected or below threshold.
   *
   * Frequency bands (fraction of Nyquist):
   *   low  (0–12%)   → aa  (open jaw, "ah")
   *   mLow (12–25%)  → ou  (round lips, "oo")
   *   mid  (25–45%)  → oh  (half-round, "oh")
   *   mHi  (45–65%)  → ee  (spread lips, "ee")
   *   hi   (65–100%) → ih  (narrow, "ih")
   */
  getVisemeValues(): VisemeValues {
    const zero: VisemeValues = { aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 };

    if (!this._active || !this.analyser || !this.timeDomainData || !this.frequencyData) {
      return zero;
    }

    // ── Worker path: send raw data off-thread, return last result ────
    if (this._workerReady && this.worker && !this._pendingWorkerResult) {
      this.analyser.getFloatTimeDomainData(this.timeDomainData);
      this.analyser.getByteFrequencyData(this.frequencyData);

      // Copy buffers for transfer (originals stay with main thread)
      const tdCopy = new Float32Array(this.timeDomainData);
      const fqCopy = new Uint8Array(this.frequencyData);

      const msg: WorkerInMessage = {
        type: 'analyze',
        timeDomain: tdCopy,
        frequency: fqCopy,
        sensitivity: this.options.sensitivity,
      };
      this.worker.postMessage(msg, [tdCopy.buffer]);
      this._pendingWorkerResult = true;

      // Update main-thread volume for the `volume` getter
      this._volume = this._lastWorkerVolume;
      return this._lastWorkerVisemes;
    }

    // If worker is busy, return last worker result
    if (this._workerReady && this._pendingWorkerResult) {
      return this._lastWorkerVisemes;
    }

    // ── Main-thread fallback ─────────────────────────────────────────
    // Read time-domain for overall volume
    this.analyser.getFloatTimeDomainData(this.timeDomainData);
    const rms = this.calculateRMS(this.timeDomainData);
    this._volume = rms;

    if (rms < this.options.silenceThreshold) {
      return zero;
    }

    // Read frequency data for band analysis
    this.analyser.getByteFrequencyData(this.frequencyData);

    const binCount = this.frequencyData.length;
    const bandEnergies = this.computeBandEnergies(this.frequencyData, binCount);
    const sens = this.options.sensitivity;

    return {
      aa: clamp01(bandEnergies[0] * sens),
      ou: clamp01(bandEnergies[1] * sens),
      oh: clamp01(bandEnergies[2] * sens),
      ee: clamp01(bandEnergies[3] * sens),
      ih: clamp01(bandEnergies[4] * sens),
    };
  }

  /**
   * Convenience: compute 5-channel visemes from pre-supplied frequency data.
   * Useful when frequency data is computed off-thread (Web Worker).
   */
  static visemeValuesFromBands(bandEnergies: readonly [number, number, number, number, number], sensitivity = 1.5): VisemeValues {
    return {
      aa: clamp01(bandEnergies[0] * sensitivity),
      ou: clamp01(bandEnergies[1] * sensitivity),
      oh: clamp01(bandEnergies[2] * sensitivity),
      ee: clamp01(bandEnergies[3] * sensitivity),
      ih: clamp01(bandEnergies[4] * sensitivity),
    };
  }

  /**
   * Get mouth values from pre-computed volume levels.
   * Useful when the audio source provides per-frame RMS volumes
   * alongside the audio data. Call with the current volume at playback position.
   */
  static mouthValuesFromVolume(volume: number, sensitivity: number = 1.5): MouthValues {
    const scaledVolume = Math.min(1, Math.max(0, volume) * sensitivity);
    if (scaledVolume < 0.01) {
      return { aa: 0, oh: 0 };
    }
    return {
      aa: Math.min(1, scaledVolume * 0.8),
      oh: Math.min(1, scaledVolume * scaledVolume * 0.5),
    };
  }

  /** Calculate RMS of a Float32Array (time-domain waveform). */
  private calculateRMS(data: Float32Array<ArrayBuffer>): number {
    let sum = 0;
    for (let i = 0; i < data.length; i++) {
      sum += data[i] * data[i];
    }
    return Math.sqrt(sum / data.length);
  }

  /**
   * Compute normalized energy for each of the 5 frequency bands.
   * Returns [low, mLow, mid, mHi, hi] each in 0–1 range (before sensitivity).
   */
  private computeBandEnergies(freqData: Uint8Array<ArrayBuffer>, binCount: number): [number, number, number, number, number] {
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
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function clamp01(v: number): number {
  return v < 0 ? 0 : v > 1 ? 1 : v;
}
