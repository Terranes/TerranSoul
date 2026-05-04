import type { LlmPoseFrame } from '../renderer/pose-animator';

export interface LlmPoseEvent<T> {
  payload: T;
}

export type LlmPoseListen = <T>(
  eventName: string,
  handler: (event: LlmPoseEvent<T>) => void,
) => Promise<() => void>;

export function subscribeLlmPoseFrames(
  listen: LlmPoseListen,
  applyFrame: (frame: LlmPoseFrame) => void,
): Promise<() => void> {
  return listen<LlmPoseFrame>('llm-pose', (event) => {
    applyFrame(event.payload);
  });
}