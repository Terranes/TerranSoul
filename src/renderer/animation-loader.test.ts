import { describe, it, expect } from 'vitest';
import type { AnimationPersona, CharacterState } from '../types';

// ── JSON data imports (same as animation-loader.ts) ──────────────────
import witchData from './animations/witch.json';
import idolData from './animations/idol.json';

const DATA_MAP: Record<string, any> = { witch: witchData, idol: idolData };

const PERSONAS: AnimationPersona[] = ['witch', 'idol'];
const STATES: CharacterState[] = ['idle', 'thinking', 'talking', 'happy', 'sad'];

describe('animation-loader JSON data', () => {
  // ── Anti-sinking regression test ─────────────────────────────────
  // Position keyframes are offsets from the bone rest position.
  // If any offset exceeds ±0.05 m the model would visibly sink or float.
  it('all hips position offsets stay within ±0.05 m (anti-sinking)', () => {
    for (const persona of PERSONAS) {
      const data = DATA_MAP[persona];
      for (const state of STATES) {
        const variants = Array.isArray(data[state]) ? data[state] : [data[state]];
        for (let vi = 0; vi < variants.length; vi++) {
          const clip = variants[vi];
          for (const track of clip.tracks) {
            if (!track.positions) continue;
            for (let i = 0; i < track.positions.length; i++) {
              const [x, y, z] = track.positions[i];
              const label = `${persona}/${state}/v${vi}/${track.bone}[${i}]`;
              expect(Math.abs(x), `${label} x`).toBeLessThanOrEqual(0.05);
              expect(Math.abs(y), `${label} y`).toBeLessThanOrEqual(0.05);
              expect(Math.abs(z), `${label} z`).toBeLessThanOrEqual(0.05);
            }
          }
        }
      }
    }
  });

  // ── Multi-variant presence test ──────────────────────────────────
  it('each persona × state has at least 2 animation variants', () => {
    for (const persona of PERSONAS) {
      const data = DATA_MAP[persona];
      for (const state of STATES) {
        const variants = Array.isArray(data[state]) ? data[state] : [data[state]];
        expect(
          variants.length,
          `${persona}/${state} should have ≥2 variants`,
        ).toBeGreaterThanOrEqual(2);
      }
    }
  });

  // ── Every clip has valid structure ───────────────────────────────
  it('every clip has positive duration and at least 1 track', () => {
    for (const persona of PERSONAS) {
      const data = DATA_MAP[persona];
      for (const state of STATES) {
        const variants = Array.isArray(data[state]) ? data[state] : [data[state]];
        for (let vi = 0; vi < variants.length; vi++) {
          const clip = variants[vi];
          const label = `${persona}/${state}/v${vi}`;
          expect(clip.duration, `${label} duration`).toBeGreaterThan(0);
          expect(clip.tracks.length, `${label} tracks`).toBeGreaterThanOrEqual(1);
          for (const track of clip.tracks) {
            expect(track.bone, `${label} bone name`).toBeTruthy();
            expect(track.times.length, `${label} keyframes`).toBeGreaterThanOrEqual(2);
            if (track.rotations) {
              expect(track.rotations.length, `${label} rotations`).toBe(track.times.length);
            }
            if (track.positions) {
              expect(track.positions.length, `${label} positions`).toBe(track.times.length);
            }
          }
        }
      }
    }
  });

  // ── Variant diversity: different durations show distinct clips ───
  it('variants within each state have different durations (not identical clips)', () => {
    for (const persona of PERSONAS) {
      const data = DATA_MAP[persona];
      for (const state of STATES) {
        const variants = Array.isArray(data[state]) ? data[state] : [data[state]];
        if (variants.length < 2) continue;
        const durations = variants.map((v: any) => v.duration);
        const unique = new Set(durations);
        expect(
          unique.size,
          `${persona}/${state} durations should vary`,
        ).toBeGreaterThanOrEqual(2);
      }
    }
  });
});
