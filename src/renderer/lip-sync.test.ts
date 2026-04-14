import { describe, it, expect } from 'vitest';
import { LipSync, type MouthValues } from './lip-sync';

describe('LipSync', () => {
  it('creates with default options', () => {
    const lipSync = new LipSync();
    expect(lipSync.active).toBe(false);
    expect(lipSync.volume).toBe(0);
  });

  it('creates with custom options', () => {
    const lipSync = new LipSync({
      fftSize: 512,
      smoothingTimeConstant: 0.8,
      silenceThreshold: 0.05,
      sensitivity: 2.0,
    });
    expect(lipSync.active).toBe(false);
  });

  it('getMouthValues returns zero when not active', () => {
    const lipSync = new LipSync();
    const values = lipSync.getMouthValues();
    expect(values.aa).toBe(0);
    expect(values.oh).toBe(0);
  });

  it('disconnect resets state', () => {
    const lipSync = new LipSync();
    lipSync.disconnect(); // Should not throw even when not connected
    expect(lipSync.active).toBe(false);
    expect(lipSync.volume).toBe(0);
  });

  it('disconnect can be called multiple times safely', () => {
    const lipSync = new LipSync();
    lipSync.disconnect();
    lipSync.disconnect();
    expect(lipSync.active).toBe(false);
  });

  // ── Static volume-to-mouth conversion ──────────────────────────────

  it('mouthValuesFromVolume returns zero for silence', () => {
    const values = LipSync.mouthValuesFromVolume(0);
    expect(values.aa).toBe(0);
    expect(values.oh).toBe(0);
  });

  it('mouthValuesFromVolume returns zero for very low volume', () => {
    const values = LipSync.mouthValuesFromVolume(0.005);
    expect(values.aa).toBe(0);
    expect(values.oh).toBe(0);
  });

  it('mouthValuesFromVolume returns proportional values for moderate volume', () => {
    const values = LipSync.mouthValuesFromVolume(0.3);
    expect(values.aa).toBeGreaterThan(0);
    expect(values.aa).toBeLessThanOrEqual(1);
    expect(values.oh).toBeGreaterThan(0);
    expect(values.oh).toBeLessThanOrEqual(1);
  });

  it('mouthValuesFromVolume aa is greater than oh (aa is primary)', () => {
    const values = LipSync.mouthValuesFromVolume(0.5);
    expect(values.aa).toBeGreaterThan(values.oh);
  });

  it('mouthValuesFromVolume clamps at 1.0', () => {
    const values = LipSync.mouthValuesFromVolume(10.0);
    expect(values.aa).toBeLessThanOrEqual(1);
    expect(values.oh).toBeLessThanOrEqual(1);
  });

  it('mouthValuesFromVolume handles negative volume', () => {
    const values = LipSync.mouthValuesFromVolume(-1.0);
    expect(values.aa).toBe(0);
    expect(values.oh).toBe(0);
  });

  it('mouthValuesFromVolume respects custom sensitivity', () => {
    const low = LipSync.mouthValuesFromVolume(0.2, 1.0);
    const high = LipSync.mouthValuesFromVolume(0.2, 3.0);
    expect(high.aa).toBeGreaterThan(low.aa);
  });

  it('mouthValuesFromVolume increases monotonically with volume', () => {
    const volumes = [0.1, 0.2, 0.3, 0.4, 0.5];
    let prevAa = 0;
    for (const v of volumes) {
      const values = LipSync.mouthValuesFromVolume(v);
      expect(values.aa).toBeGreaterThanOrEqual(prevAa);
      prevAa = values.aa;
    }
  });

  // ── MouthValues shape ──────────────────────────────────────────────

  it('MouthValues type has aa and oh fields', () => {
    const values: MouthValues = { aa: 0.5, oh: 0.3 };
    expect(values.aa).toBe(0.5);
    expect(values.oh).toBe(0.3);
  });
});
