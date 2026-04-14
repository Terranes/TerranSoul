/**
 * Web Audio API-based lip sync engine.
 *
 * Analyses audio output (from TTS) in real-time using an `AnalyserNode`
 * and maps volume levels to VRM mouth morph targets (`aa`, `oh`).
 *
 * Provider-agnostic — works with any audio source that can be connected to
 * the Web Audio API (HTMLAudioElement, MediaStream, AudioBufferSource, etc.).
 *
 * Usage:
 *   const lipSync = new LipSync();
 *   lipSync.connectAudioElement(audioElement);
 *   // In your animation loop:
 *   const { aa, oh } = lipSync.getMouthValues();
 *   // Apply to VRM expression manager
 */

export interface MouthValues {
  /** Mouth open (primary morph: wide open, "ah" shape). Range 0–1. */
  aa: number;
  /** Mouth round (secondary morph: rounded, "oh" shape). Range 0–1. */
  oh: number;
}

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

export class LipSync {
  private context: AudioContext | null = null;
  private analyser: AnalyserNode | null = null;
  private source: MediaElementAudioSourceNode | null = null;
  private timeDomainData: Float32Array<ArrayBuffer> | null = null;
  private options: Required<LipSyncOptions>;
  private _active = false;
  private _volume = 0;

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
  }

  /**
   * Compute current mouth morph values from audio analysis.
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
}
