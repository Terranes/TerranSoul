import { describe, it, expect } from 'vitest';
import { computeStats } from './stats';
import {
  getMemoryRecallLimit,
  getContextWindowMultiplier,
  getChatHistoryLimit,
  getHotwordSensitivity,
  getTtsExpressiveness,
  computeModifiers,
} from './stat-modifiers';

describe('stat-modifiers', () => {
  const BASELINE = computeStats([]);
  const LEVELLED = computeStats([
    'free-brain', 'paid-brain', 'local-brain',
    'memory', 'tts', 'asr', 'hotwords',
    'bgm', 'pet-mode', 'agents', 'vision',
    'translation', 'whisper-asr', 'voice-cloning',
  ]);

  it('memory recall scales monotonically with wisdom', () => {
    const baselineLimit = getMemoryRecallLimit(BASELINE);
    const levelledLimit = getMemoryRecallLimit(LEVELLED);
    expect(levelledLimit).toBeGreaterThan(baselineLimit);
    expect(baselineLimit).toBeGreaterThanOrEqual(8);
    expect(levelledLimit).toBeLessThanOrEqual(20);
  });

  it('context window multiplier ≥ 1 and grows with intelligence', () => {
    expect(getContextWindowMultiplier(BASELINE)).toBeGreaterThanOrEqual(1);
    expect(getContextWindowMultiplier(LEVELLED)).toBeGreaterThan(getContextWindowMultiplier(BASELINE));
    expect(getContextWindowMultiplier(LEVELLED)).toBeLessThanOrEqual(1.5);
  });

  it('chat history limit applies the multiplier to a 20-turn baseline', () => {
    expect(getChatHistoryLimit(BASELINE)).toBeGreaterThanOrEqual(20);
    expect(getChatHistoryLimit(BASELINE, 10)).toBeGreaterThanOrEqual(10);
    expect(getChatHistoryLimit(LEVELLED)).toBeGreaterThan(getChatHistoryLimit(BASELINE));
  });

  it('hotword sensitivity is in [0.5, 1.5]', () => {
    expect(getHotwordSensitivity(BASELINE)).toBeGreaterThanOrEqual(0.5);
    expect(getHotwordSensitivity(BASELINE)).toBeLessThanOrEqual(1.5);
    expect(getHotwordSensitivity(LEVELLED)).toBeGreaterThan(getHotwordSensitivity(BASELINE));
  });

  it('tts expressiveness is in [0, 1]', () => {
    expect(getTtsExpressiveness(BASELINE)).toBeGreaterThanOrEqual(0);
    expect(getTtsExpressiveness(BASELINE)).toBeLessThanOrEqual(1);
    expect(getTtsExpressiveness(LEVELLED)).toBeGreaterThan(getTtsExpressiveness(BASELINE));
    expect(getTtsExpressiveness(LEVELLED)).toBeLessThanOrEqual(1);
  });

  it('computeModifiers returns all five fields', () => {
    const mods = computeModifiers(LEVELLED);
    expect(mods).toHaveProperty('memoryRecallLimit');
    expect(mods).toHaveProperty('contextWindowMultiplier');
    expect(mods).toHaveProperty('chatHistoryLimit');
    expect(mods).toHaveProperty('hotwordSensitivity');
    expect(mods).toHaveProperty('ttsExpressiveness');
  });
});
