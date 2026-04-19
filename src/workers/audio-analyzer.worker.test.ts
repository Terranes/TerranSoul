import { describe, it, expect } from 'vitest';
import {
  calculateRMS,
  computeBandEnergies,
  analyzeAudio,
  type AnalyzeResult,
  type AnalyzeMessage,
  type ConfigureMessage,
  type WorkerOutMessage,
} from './audio-analyzer.worker';

// ── calculateRMS ─────────────────────────────────────────────────────────────

describe('calculateRMS', () => {
  it('returns 0 for silent signal', () => {
    const data = new Float32Array(256);
    expect(calculateRMS(data)).toBe(0);
  });

  it('returns correct RMS for known signal', () => {
    // Constant signal of 0.5 → RMS = 0.5
    const data = new Float32Array(128).fill(0.5);
    expect(calculateRMS(data)).toBeCloseTo(0.5, 6);
  });

  it('handles single-sample array', () => {
    const data = new Float32Array([0.8]);
    expect(calculateRMS(data)).toBeCloseTo(0.8, 6);
  });

  it('handles negative values (RMS is always positive)', () => {
    const data = new Float32Array(128).fill(-0.3);
    expect(calculateRMS(data)).toBeCloseTo(0.3, 6);
  });

  it('returns expected value for sine wave', () => {
    const length = 1024;
    const data = new Float32Array(length);
    for (let i = 0; i < length; i++) {
      data[i] = Math.sin((2 * Math.PI * i) / length);
    }
    // RMS of a sine wave = 1/√2 ≈ 0.7071
    expect(calculateRMS(data)).toBeCloseTo(1 / Math.sqrt(2), 2);
  });
});

// ── computeBandEnergies ──────────────────────────────────────────────────────

describe('computeBandEnergies', () => {
  it('returns all zeros for empty frequency data', () => {
    const data = new Uint8Array(128);
    const bands = computeBandEnergies(data, 128);
    expect(bands).toEqual([0, 0, 0, 0, 0]);
  });

  it('returns 5 bands as a tuple', () => {
    const data = new Uint8Array(128).fill(128);
    const bands = computeBandEnergies(data, 128);
    expect(bands).toHaveLength(5);
    for (const b of bands) {
      expect(b).toBeGreaterThan(0);
      expect(b).toBeLessThanOrEqual(1);
    }
  });

  it('low band responds to low-frequency energy', () => {
    const data = new Uint8Array(128);
    // Fill only the first 12% of bins (low band)
    const lowEnd = Math.floor(0.12 * 128);
    for (let i = 0; i < lowEnd; i++) data[i] = 200;
    const bands = computeBandEnergies(data, 128);
    expect(bands[0]).toBeGreaterThan(0.5); // low band (aa)
    expect(bands[4]).toBe(0); // high band (ih) should be 0
  });

  it('high band responds to high-frequency energy', () => {
    const data = new Uint8Array(128);
    // Fill only bins > 65% (high band)
    const hiStart = Math.floor(0.65 * 128);
    for (let i = hiStart; i < 128; i++) data[i] = 200;
    const bands = computeBandEnergies(data, 128);
    expect(bands[4]).toBeGreaterThan(0.5); // high band (ih)
    expect(bands[0]).toBe(0); // low band (aa) should be 0
  });

  it('uniform energy distributes across all bands', () => {
    const data = new Uint8Array(128).fill(128);
    const bands = computeBandEnergies(data, 128);
    // All bands should be approximately equal for uniform data
    const avg = bands.reduce((a, b) => a + b, 0) / 5;
    for (const b of bands) {
      expect(Math.abs(b - avg)).toBeLessThan(0.05);
    }
  });

  it('normalizes to 0–1 range', () => {
    const data = new Uint8Array(128).fill(255);
    const bands = computeBandEnergies(data, 128);
    for (const b of bands) {
      expect(b).toBeCloseTo(1.0, 2);
    }
  });
});

// ── analyzeAudio ─────────────────────────────────────────────────────────────

describe('analyzeAudio', () => {
  it('returns zero visemes for silent audio', () => {
    const td = new Float32Array(256);
    const freq = new Uint8Array(128);
    const result = analyzeAudio(td, freq, 1.5, 0.01);
    expect(result.type).toBe('result');
    expect(result.volume).toBe(0);
    expect(result.visemes).toEqual({ aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 });
  });

  it('returns volume below threshold with zero visemes', () => {
    const td = new Float32Array(256).fill(0.005);
    const freq = new Uint8Array(128).fill(128);
    const result = analyzeAudio(td, freq, 1.5, 0.01);
    expect(result.volume).toBeCloseTo(0.005, 4);
    expect(result.visemes).toEqual({ aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 });
  });

  it('produces non-zero visemes when volume above threshold', () => {
    const td = new Float32Array(256).fill(0.3);
    const freq = new Uint8Array(128).fill(128);
    const result = analyzeAudio(td, freq, 1.5, 0.01);
    expect(result.volume).toBeGreaterThan(0.01);
    expect(result.visemes.aa).toBeGreaterThan(0);
    expect(result.visemes.ou).toBeGreaterThan(0);
    expect(result.visemes.oh).toBeGreaterThan(0);
    expect(result.visemes.ee).toBeGreaterThan(0);
    expect(result.visemes.ih).toBeGreaterThan(0);
  });

  it('clamps viseme values to [0, 1]', () => {
    const td = new Float32Array(256).fill(0.5);
    const freq = new Uint8Array(128).fill(255);
    const result = analyzeAudio(td, freq, 5.0, 0.01); // high sensitivity
    for (const v of Object.values(result.visemes)) {
      expect(v).toBeGreaterThanOrEqual(0);
      expect(v).toBeLessThanOrEqual(1);
    }
  });

  it('matches the expected message shape', () => {
    const td = new Float32Array(256).fill(0.2);
    const freq = new Uint8Array(128).fill(100);
    const result = analyzeAudio(td, freq, 1.5, 0.01);
    expect(result).toHaveProperty('type', 'result');
    expect(result).toHaveProperty('volume');
    expect(result).toHaveProperty('visemes');
    expect(result.visemes).toHaveProperty('aa');
    expect(result.visemes).toHaveProperty('ih');
    expect(result.visemes).toHaveProperty('ou');
    expect(result.visemes).toHaveProperty('ee');
    expect(result.visemes).toHaveProperty('oh');
  });

  it('sensitivity scales viseme values', () => {
    const td = new Float32Array(256).fill(0.2);
    const freq = new Uint8Array(128).fill(100);
    const low = analyzeAudio(td, freq, 1.0, 0.01);
    const high = analyzeAudio(td, freq, 3.0, 0.01);
    expect(high.visemes.aa).toBeGreaterThanOrEqual(low.visemes.aa);
  });
});

// ── Worker message protocol types ────────────────────────────────────────────

describe('worker message protocol', () => {
  it('AnalyzeMessage has correct shape', () => {
    const msg: AnalyzeMessage = {
      type: 'analyze',
      timeDomain: new Float32Array(256),
      frequency: new Uint8Array(128),
      sensitivity: 1.5,
    };
    expect(msg.type).toBe('analyze');
    expect(msg.timeDomain).toBeInstanceOf(Float32Array);
    expect(msg.frequency).toBeInstanceOf(Uint8Array);
    expect(msg.sensitivity).toBe(1.5);
  });

  it('ConfigureMessage has correct shape', () => {
    const msg: ConfigureMessage = {
      type: 'configure',
      silenceThreshold: 0.02,
      sensitivity: 2.0,
    };
    expect(msg.type).toBe('configure');
    expect(msg.silenceThreshold).toBe(0.02);
    expect(msg.sensitivity).toBe(2.0);
  });

  it('AnalyzeResult has correct shape', () => {
    const msg: AnalyzeResult = {
      type: 'result',
      volume: 0.35,
      visemes: { aa: 0.5, ih: 0.2, ou: 0.3, ee: 0.4, oh: 0.1 },
    };
    expect(msg.type).toBe('result');
    expect(msg.volume).toBe(0.35);
    expect(Object.keys(msg.visemes)).toEqual(
      expect.arrayContaining(['aa', 'ih', 'ou', 'ee', 'oh']),
    );
  });

  it('WorkerOutMessage union includes result type', () => {
    const msg: WorkerOutMessage = { type: 'configured' };
    expect(msg.type).toBe('configured');
  });
});
