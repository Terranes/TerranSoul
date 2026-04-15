import type { CharacterState } from '../types';

export interface BlendConfig {
  amplitude: number;
  frequency: number;
  crossFadeDuration: number;
  boneWeights?: Record<string, number>;
}

export const STATE_BLEND_CONFIGS: Record<CharacterState, BlendConfig> = {
  idle:      { amplitude: 0.01,  frequency: 0.3,  crossFadeDuration: 0.6 },
  thinking:  { amplitude: 0.02,  frequency: 0.5,  crossFadeDuration: 0.5 },
  talking:   { amplitude: 0.025, frequency: 0.8,  crossFadeDuration: 0.35 },
  happy:     { amplitude: 0.04,  frequency: 1.2,  crossFadeDuration: 0.4 },
  sad:       { amplitude: 0.008, frequency: 0.2,  crossFadeDuration: 0.8 },
  angry:     { amplitude: 0.03,  frequency: 1.5,  crossFadeDuration: 0.3 },
  relaxed:   { amplitude: 0.012, frequency: 0.25, crossFadeDuration: 0.7 },
  surprised: { amplitude: 0.035, frequency: 1.0,  crossFadeDuration: 0.25 },
};

const BONE_PHASE_OFFSETS: Record<string, number> = {
  head: 0,
  spine: 1.7,
  chest: 3.1,
  hips: 4.8,
  neck: 6.3,
  leftUpperArm: 8.2,
  rightUpperArm: 10.5,
};

function valueNoise(t: number): number {
  return Math.sin(t) * 0.5 + Math.sin(t * 2.3) * 0.25 + Math.sin(t * 5.7) * 0.125;
}

function cosineInterpolate(a: number, b: number, t: number): number {
  const f = (1 - Math.cos(t * Math.PI)) * 0.5;
  return a * (1 - f) + b * f;
}

export class GestureBlender {
  private configs: Record<CharacterState, BlendConfig>;
  private currentState: CharacterState = 'idle';
  private previousState: CharacterState | null = null;
  private transitionStart = -1;

  constructor(overrides?: Partial<Record<CharacterState, Partial<BlendConfig>>>) {
    this.configs = {} as Record<CharacterState, BlendConfig>;
    for (const key of Object.keys(STATE_BLEND_CONFIGS) as CharacterState[]) {
      const base = STATE_BLEND_CONFIGS[key];
      const over = overrides?.[key];
      this.configs[key] = {
        amplitude: over?.amplitude ?? base.amplitude,
        frequency: over?.frequency ?? base.frequency,
        crossFadeDuration: over?.crossFadeDuration ?? base.crossFadeDuration,
        boneWeights: over?.boneWeights ?? base.boneWeights,
      };
    }
  }

  computeOffset(boneName: string, state: CharacterState, time: number): [number, number, number] {
    const config = this.configs[state] ?? this.configs.idle;
    const phase = BONE_PHASE_OFFSETS[boneName] ?? 0;
    const weight = config.boneWeights?.[boneName] ?? 1;
    const amp = config.amplitude * weight;
    const freq = config.frequency;

    const dx = valueNoise((time * freq) + phase) * amp;
    const dy = valueNoise((time * freq) + phase + 100) * amp;
    const dz = valueNoise((time * freq) + phase + 200) * amp;

    return [dx, dy, dz];
  }

  transitionTo(newState: CharacterState, currentTime: number): void {
    if (newState === this.currentState) return;
    this.previousState = this.currentState;
    this.currentState = newState;
    this.transitionStart = currentTime;
  }

  getTransitionAlpha(currentTime: number): number {
    if (this.previousState === null || this.transitionStart < 0) return 1;
    const config = this.configs[this.currentState];
    const elapsed = currentTime - this.transitionStart;
    if (elapsed >= config.crossFadeDuration) return 1;
    return cosineInterpolate(0, 1, elapsed / config.crossFadeDuration);
  }

  getPreviousState(): CharacterState | null {
    return this.previousState;
  }
}
