import { describe, it, expect } from 'vitest';
import { LipSync, type MouthValues, type VisemeValues } from './lip-sync';

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

  // ── 5-channel viseme values ────────────────────────────────────────

  it('getVisemeValues returns all-zero when not active', () => {
    const lipSync = new LipSync();
    const v = lipSync.getVisemeValues();
    expect(v.aa).toBe(0);
    expect(v.ih).toBe(0);
    expect(v.ou).toBe(0);
    expect(v.ee).toBe(0);
    expect(v.oh).toBe(0);
  });

  it('VisemeValues type has all 5 channels', () => {
    const v: VisemeValues = { aa: 0.1, ih: 0.2, ou: 0.3, ee: 0.4, oh: 0.5 };
    expect(v.aa).toBe(0.1);
    expect(v.ih).toBe(0.2);
    expect(v.ou).toBe(0.3);
    expect(v.ee).toBe(0.4);
    expect(v.oh).toBe(0.5);
  });

  // ── visemeValuesFromBands (static) ─────────────────────────────────

  it('visemeValuesFromBands returns clamped values for band energies', () => {
    const v = LipSync.visemeValuesFromBands([0.5, 0.3, 0.2, 0.4, 0.1]);
    expect(v.aa).toBeGreaterThan(0);
    expect(v.aa).toBeLessThanOrEqual(1);
    expect(v.ou).toBeGreaterThan(0);
    expect(v.oh).toBeGreaterThan(0);
    expect(v.ee).toBeGreaterThan(0);
    expect(v.ih).toBeGreaterThan(0);
  });

  it('visemeValuesFromBands clamps at 1.0', () => {
    const v = LipSync.visemeValuesFromBands([10, 10, 10, 10, 10], 2.0);
    expect(v.aa).toBe(1);
    expect(v.ou).toBe(1);
    expect(v.oh).toBe(1);
    expect(v.ee).toBe(1);
    expect(v.ih).toBe(1);
  });

  it('visemeValuesFromBands returns zero for zero energies', () => {
    const v = LipSync.visemeValuesFromBands([0, 0, 0, 0, 0]);
    expect(v.aa).toBe(0);
    expect(v.ih).toBe(0);
    expect(v.ou).toBe(0);
    expect(v.ee).toBe(0);
    expect(v.oh).toBe(0);
  });

  it('visemeValuesFromBands maps bands to correct channels', () => {
    // Only low band has energy → should only produce aa
    const v = LipSync.visemeValuesFromBands([0.8, 0, 0, 0, 0], 1.0);
    expect(v.aa).toBeGreaterThan(0);
    expect(v.ou).toBe(0);
    expect(v.oh).toBe(0);
    expect(v.ee).toBe(0);
    expect(v.ih).toBe(0);
  });

  it('visemeValuesFromBands high band maps to ih', () => {
    const v = LipSync.visemeValuesFromBands([0, 0, 0, 0, 0.6], 1.0);
    expect(v.aa).toBe(0);
    expect(v.ih).toBeGreaterThan(0);
  });

  it('visemeValuesFromBands respects sensitivity', () => {
    const low = LipSync.visemeValuesFromBands([0.3, 0.3, 0.3, 0.3, 0.3], 1.0);
    const high = LipSync.visemeValuesFromBands([0.3, 0.3, 0.3, 0.3, 0.3], 2.5);
    expect(high.aa).toBeGreaterThan(low.aa);
    expect(high.ou).toBeGreaterThan(low.ou);
  });

  it('visemeValuesFromBands does not produce negative values', () => {
    const v = LipSync.visemeValuesFromBands([-0.5, -1, -0.3, -2, -0.1], 1.5);
    expect(v.aa).toBeGreaterThanOrEqual(0);
    expect(v.ih).toBeGreaterThanOrEqual(0);
    expect(v.ou).toBeGreaterThanOrEqual(0);
    expect(v.ee).toBeGreaterThanOrEqual(0);
    expect(v.oh).toBeGreaterThanOrEqual(0);
  });

  // ── Worker integration ─────────────────────────────────────────────

  it('workerReady is false by default', () => {
    const lipSync = new LipSync();
    expect(lipSync.workerReady).toBe(false);
  });

  it('enableWorker does not throw in jsdom (no real Worker)', () => {
    const lipSync = new LipSync();
    // In jsdom, Worker constructor may not be available or may throw
    // enableWorker should handle gracefully and stay on main thread
    expect(() => lipSync.enableWorker()).not.toThrow();
  });

  it('disableWorker is safe to call when no worker is active', () => {
    const lipSync = new LipSync();
    expect(() => lipSync.disableWorker()).not.toThrow();
    expect(lipSync.workerReady).toBe(false);
  });

  it('disconnect cleans up worker state', () => {
    const lipSync = new LipSync();
    lipSync.disconnect();
    expect(lipSync.workerReady).toBe(false);
  });
});
