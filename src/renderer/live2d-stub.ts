import type { CharacterState } from '../types';
import type { CharacterRenderer, RendererType } from './renderer-abstraction';

/**
 * Stub Live2D renderer that satisfies the CharacterRenderer interface
 * without bundling the Cubism SDK.  When Live2D support is added for real,
 * only this file needs to be replaced.
 */
export class Live2DStubRenderer implements CharacterRenderer {
  readonly type: RendererType = 'live2d';
  private state: CharacterState = 'idle';
  private loaded = false;
  private _canvas: HTMLCanvasElement | null = null;

  /** The canvas bound to this renderer (useful when the real SDK is wired in). */
  get canvas(): HTMLCanvasElement | null {
    return this._canvas;
  }

  init(canvas: HTMLCanvasElement): void {
    this._canvas = canvas;
  }

  async loadModel(_source: string | ArrayBuffer): Promise<void> {
    // Stub: would load a .model3.json via Cubism SDK
    this.loaded = true;
  }

  setState(state: CharacterState): void {
    this.state = state;
  }

  getState(): CharacterState {
    return this.state;
  }

  setMouthValues(_aa: number, _oh: number): void {
    // stub — would set Cubism mouth parameters
  }

  clearMouthValues(): void {
    // stub — would reset Cubism mouth parameters
  }

  update(_delta: number): void {
    // stub — would call Cubism update
  }

  resize(_w: number, _h: number): void {
    // stub — would resize Cubism viewport
  }

  dispose(): void {
    this._canvas = null;
    this.loaded = false;
  }

  isModelLoaded(): boolean {
    return this.loaded;
  }
}
