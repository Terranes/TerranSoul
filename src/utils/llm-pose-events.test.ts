import { describe, expect, it, vi } from 'vitest';
import type { LlmPoseFrame } from '../renderer/pose-animator';
import { subscribeLlmPoseFrames, type LlmPoseEvent, type LlmPoseListen } from './llm-pose-events';

describe('llm pose event subscription', () => {
  it('passes mock llm-pose payloads to applyFrame', async () => {
    const captured: { handler?: (event: LlmPoseEvent<LlmPoseFrame>) => void } = {};
    const unlisten = vi.fn();
    const listen: LlmPoseListen = vi.fn(async (eventName, nextHandler) => {
      expect(eventName).toBe('llm-pose');
      captured.handler = nextHandler as (event: LlmPoseEvent<LlmPoseFrame>) => void;
      return unlisten;
    });
    const applyFrame = vi.fn();
    const frame: LlmPoseFrame = {
      bones: {
        head: [0.1, 0.2, 0.3],
      },
      duration_s: 0.5,
    };

    const stop = await subscribeLlmPoseFrames(listen, applyFrame);
    if (!captured.handler) throw new Error('mock listen did not receive a handler');
    captured.handler({ payload: frame });
    stop();

    expect(applyFrame).toHaveBeenCalledWith(frame);
    expect(unlisten).toHaveBeenCalledTimes(1);
  });
});