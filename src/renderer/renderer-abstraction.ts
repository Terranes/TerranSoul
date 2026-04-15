import type { CharacterState } from '../types';
import { Live2DStubRenderer } from './live2d-stub';

/** Supported renderer backends. */
export type RendererType = 'vrm' | 'live2d';

/** Common interface that all character renderers must implement. */
export interface CharacterRenderer {
  /** Renderer backend type. */
  readonly type: RendererType;

  /** Initialize the renderer with a canvas element. */
  init(canvas: HTMLCanvasElement): void;

  /** Load a model from the given path or ArrayBuffer. */
  loadModel(source: string | ArrayBuffer): Promise<void>;

  /** Set the character's emotional/animation state. */
  setState(state: CharacterState): void;

  /** Get the current state. */
  getState(): CharacterState;

  /** Set mouth values for lip-sync (aa=open, oh=round, 0–1). */
  setMouthValues(aa: number, oh: number): void;

  /** Clear lip-sync values. */
  clearMouthValues(): void;

  /** Update animation (called each frame with delta in seconds). */
  update(delta: number): void;

  /** Resize the renderer to fit the container. */
  resize(width: number, height: number): void;

  /** Dispose all resources. */
  dispose(): void;

  /** Whether a model is currently loaded. */
  isModelLoaded(): boolean;
}

/** Factory function to create a renderer by type. */
export function createRenderer(type: RendererType): CharacterRenderer {
  switch (type) {
    case 'live2d':
      return new Live2DStubRenderer();
    case 'vrm':
    default:
      // VRM renderer is handled by the existing scene.ts + CharacterAnimator.
      // This is a facade that wraps the existing VRM pipeline.
      throw new Error('VRM renderer is managed by CharacterViewport directly');
  }
}
