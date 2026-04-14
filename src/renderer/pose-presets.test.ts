import { describe, it, expect } from 'vitest';
import {
  getAllPosePresets,
  getPosePreset,
  VALID_POSE_BONES,
  EMOTION_TO_POSE,
  type PosePreset,
} from './pose-presets';

const TWO_PI = Math.PI * 2;

describe('PosePresets', () => {
  it('exports exactly 10 presets', () => {
    expect(getAllPosePresets()).toHaveLength(10);
  });

  it('all 10 expected preset ids exist', () => {
    const ids = getAllPosePresets().map(p => p.id);
    expect(ids).toContain('confident');
    expect(ids).toContain('shy');
    expect(ids).toContain('excited');
    expect(ids).toContain('thoughtful');
    expect(ids).toContain('relaxed');
    expect(ids).toContain('defensive');
    expect(ids).toContain('attentive');
    expect(ids).toContain('playful');
    expect(ids).toContain('bored');
    expect(ids).toContain('empathetic');
  });

  it('each preset has a non-empty id and label', () => {
    for (const preset of getAllPosePresets()) {
      expect(preset.id.length).toBeGreaterThan(0);
      expect(preset.label.length).toBeGreaterThan(0);
    }
  });

  it('each preset has at least one bone rotation', () => {
    for (const preset of getAllPosePresets()) {
      expect(Object.keys(preset.boneRotations).length).toBeGreaterThan(0);
    }
  });

  it('all bone names in presets are valid VRM humanoid bones', () => {
    for (const preset of getAllPosePresets()) {
      for (const boneName of Object.keys(preset.boneRotations)) {
        expect(VALID_POSE_BONES.has(boneName)).toBe(true);
      }
    }
  });

  it('all rotation values are within [-2π, 2π]', () => {
    for (const preset of getAllPosePresets()) {
      for (const rot of Object.values(preset.boneRotations)) {
        expect(Math.abs(rot!.x)).toBeLessThanOrEqual(TWO_PI);
        expect(Math.abs(rot!.y)).toBeLessThanOrEqual(TWO_PI);
        expect(Math.abs(rot!.z)).toBeLessThanOrEqual(TWO_PI);
        expect(Number.isFinite(rot!.x)).toBe(true);
        expect(Number.isFinite(rot!.y)).toBe(true);
        expect(Number.isFinite(rot!.z)).toBe(true);
      }
    }
  });

  it('all bone rotation values are small offsets (|value| < 1.1 rad)', () => {
    for (const preset of getAllPosePresets()) {
      for (const rot of Object.values(preset.boneRotations)) {
        expect(Math.abs(rot!.x)).toBeLessThan(1.1);
        expect(Math.abs(rot!.y)).toBeLessThan(1.1);
        expect(Math.abs(rot!.z)).toBeLessThan(1.1);
      }
    }
  });

  it('getPosePreset returns correct preset by id', () => {
    const preset = getPosePreset('confident') as PosePreset;
    expect(preset).toBeDefined();
    expect(preset.id).toBe('confident');
    expect(preset.label).toBe('Confident');
  });

  it('getPosePreset returns undefined for unknown id', () => {
    expect(getPosePreset('nonexistent')).toBeUndefined();
  });

  it('no two presets share the same id', () => {
    const ids = getAllPosePresets().map(p => p.id);
    const unique = new Set(ids);
    expect(unique.size).toBe(ids.length);
  });

  it('confident preset has spine and head bones', () => {
    const preset = getPosePreset('confident')!;
    expect(preset.boneRotations['spine']).toBeDefined();
    expect(preset.boneRotations['head']).toBeDefined();
  });

  it('shy preset has forward spine lean (spine.x > 0)', () => {
    const preset = getPosePreset('shy')!;
    expect(preset.boneRotations['spine']!.x).toBeGreaterThan(0);
  });

  it('confident preset has backward spine lean (spine.x < 0)', () => {
    const preset = getPosePreset('confident')!;
    expect(preset.boneRotations['spine']!.x).toBeLessThan(0);
  });

  it('EMOTION_TO_POSE covers all standard character states', () => {
    const states = ['idle', 'thinking', 'talking', 'happy', 'sad', 'angry', 'relaxed', 'surprised', 'neutral'];
    for (const state of states) {
      expect(EMOTION_TO_POSE[state]).toBeDefined();
      expect(EMOTION_TO_POSE[state].length).toBeGreaterThan(0);
    }
  });

  it('EMOTION_TO_POSE references only valid preset ids', () => {
    const validIds = new Set(getAllPosePresets().map(p => p.id));
    for (const instructions of Object.values(EMOTION_TO_POSE)) {
      for (const instr of instructions) {
        expect(validIds.has(instr.presetId)).toBe(true);
        expect(instr.weight).toBeGreaterThan(0);
        expect(instr.weight).toBeLessThanOrEqual(1);
      }
    }
  });
});
