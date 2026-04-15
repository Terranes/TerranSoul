import { describe, it, expect } from 'vitest';
import { createRenderer } from './renderer-abstraction';
import type { CharacterRenderer } from './renderer-abstraction';
import { Live2DStubRenderer } from './live2d-stub';
import type { CharacterState } from '../types';

function makeCanvas(): HTMLCanvasElement {
  return document.createElement('canvas');
}

describe('Live2DStubRenderer', () => {
  it('initializes with correct type', () => {
    const renderer = new Live2DStubRenderer();
    expect(renderer.type).toBe('live2d');
  });

  it('init sets canvas reference', () => {
    const renderer = new Live2DStubRenderer();
    const canvas = makeCanvas();
    renderer.init(canvas);
    // No throw means it accepted the canvas
    expect(renderer.type).toBe('live2d');
  });

  it('loadModel sets loaded state', async () => {
    const renderer = new Live2DStubRenderer();
    expect(renderer.isModelLoaded()).toBe(false);
    await renderer.loadModel('test.model3.json');
    expect(renderer.isModelLoaded()).toBe(true);
  });

  it('setState / getState work correctly', () => {
    const renderer = new Live2DStubRenderer();
    expect(renderer.getState()).toBe('idle');
    renderer.setState('happy');
    expect(renderer.getState()).toBe('happy');
  });

  it('setMouthValues does not throw', () => {
    const renderer = new Live2DStubRenderer();
    expect(() => renderer.setMouthValues(0.5, 0.3)).not.toThrow();
  });

  it('clearMouthValues does not throw', () => {
    const renderer = new Live2DStubRenderer();
    expect(() => renderer.clearMouthValues()).not.toThrow();
  });

  it('update does not throw', () => {
    const renderer = new Live2DStubRenderer();
    expect(() => renderer.update(0.016)).not.toThrow();
  });

  it('resize does not throw', () => {
    const renderer = new Live2DStubRenderer();
    expect(() => renderer.resize(800, 600)).not.toThrow();
  });

  it('dispose cleans up state', async () => {
    const renderer = new Live2DStubRenderer();
    renderer.init(makeCanvas());
    await renderer.loadModel('model.json');
    expect(renderer.isModelLoaded()).toBe(true);
    renderer.dispose();
    expect(renderer.isModelLoaded()).toBe(false);
  });

  it('isModelLoaded returns false initially and true after load', async () => {
    const renderer = new Live2DStubRenderer();
    expect(renderer.isModelLoaded()).toBe(false);
    await renderer.loadModel(new ArrayBuffer(8));
    expect(renderer.isModelLoaded()).toBe(true);
  });

  it('all CharacterState values work with setState', () => {
    const renderer = new Live2DStubRenderer();
    const states: CharacterState[] = [
      'idle', 'thinking', 'talking', 'happy', 'sad', 'angry', 'relaxed', 'surprised',
    ];
    for (const s of states) {
      renderer.setState(s);
      expect(renderer.getState()).toBe(s);
    }
  });
});

describe('createRenderer', () => {
  it('returns Live2DStubRenderer for live2d type', () => {
    const renderer = createRenderer('live2d');
    expect(renderer).toBeInstanceOf(Live2DStubRenderer);
    expect(renderer.type).toBe('live2d');
  });

  it('throws for vrm type (managed separately)', () => {
    expect(() => createRenderer('vrm')).toThrow(
      'VRM renderer is managed by CharacterViewport directly',
    );
  });

  it('returned renderer satisfies CharacterRenderer interface', async () => {
    const renderer: CharacterRenderer = createRenderer('live2d');
    renderer.init(makeCanvas());
    await renderer.loadModel('test.model3.json');
    renderer.setState('talking');
    expect(renderer.getState()).toBe('talking');
    renderer.setMouthValues(1, 0);
    renderer.clearMouthValues();
    renderer.update(0.016);
    renderer.resize(640, 480);
    expect(renderer.isModelLoaded()).toBe(true);
    renderer.dispose();
    expect(renderer.isModelLoaded()).toBe(false);
  });
});
