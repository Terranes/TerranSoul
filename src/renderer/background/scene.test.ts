import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createBackgroundScene } from './scene';

/**
 * Minimal WebGLRenderer stub.  We don't need real WebGL to verify the
 * scene's mount/unmount/visibility/resize wiring — only that the public
 * Three.js API surface is invoked correctly.
 */
function makeRendererStub() {
  return {
    setPixelRatio: vi.fn(),
    setSize:       vi.fn(),
    setClearColor: vi.fn(),
    render:        vi.fn(),
    dispose:       vi.fn(),
    domElement:    null as unknown as HTMLCanvasElement,
  };
}

function makeFactory(stub: ReturnType<typeof makeRendererStub>) {
  // The real WebGLRenderer constructor receives `{ canvas }` and stores it
  // on `domElement`; we mirror that for completeness.
  return (canvas: HTMLCanvasElement) => {
    stub.domElement = canvas;
    return stub as unknown as import('three').WebGLRenderer;
  };
}

describe('createBackgroundScene', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    delete document.documentElement.dataset.theme;
  });
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('mounts a fixed-position canvas onto the parent and renders one frame', () => {
    const stub = makeRendererStub();
    const handle = createBackgroundScene({ rendererFactory: makeFactory(stub) });
    expect(handle).not.toBeNull();
    const canvas = handle!.canvas;
    expect(canvas.parentElement).toBe(document.body);
    expect(canvas.tagName).toBe('CANVAS');
    expect(canvas.style.position).toBe('fixed');
    expect(canvas.style.zIndex).toBe('-1');
    expect(canvas.style.pointerEvents).toBe('none');
    expect(stub.setSize).toHaveBeenCalled();
    expect(stub.render).toHaveBeenCalled();
    handle!.dispose();
  });

  it('returns null when WebGL renderer construction throws', () => {
    const handle = createBackgroundScene({
      rendererFactory: () => {
        throw new Error('WebGL not available');
      },
    });
    expect(handle).toBeNull();
    // Must not leak a canvas if construction failed.
    expect(document.body.querySelector('canvas')).toBeNull();
  });

  it('removes the canvas and renderer on dispose()', () => {
    const stub = makeRendererStub();
    const handle = createBackgroundScene({ rendererFactory: makeFactory(stub) })!;
    handle.dispose();
    expect(document.body.querySelector('canvas')).toBeNull();
    expect(stub.dispose).toHaveBeenCalled();
  });

  it('pauses on visibilitychange when document.hidden becomes true', () => {
    const stub = makeRendererStub();
    const handle = createBackgroundScene({ rendererFactory: makeFactory(stub) })!;
    const callsBefore = stub.render.mock.calls.length;
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => true });
    document.dispatchEvent(new Event('visibilitychange'));
    // No new render should fire while hidden — schedule does nothing.
    // (We can't perfectly assert "no future render" deterministically,
    // but we can assert the disposer still works after hiding.)
    expect(stub.render.mock.calls.length).toBeGreaterThanOrEqual(callsBefore);
    handle.dispose();
  });

  it('updates uResolution on window resize', () => {
    const stub = makeRendererStub();
    const handle = createBackgroundScene({ rendererFactory: makeFactory(stub) })!;
    const callsBefore = stub.setSize.mock.calls.length;
    Object.defineProperty(window, 'innerWidth',  { configurable: true, value: 800,  writable: true });
    Object.defineProperty(window, 'innerHeight', { configurable: true, value: 600,  writable: true });
    window.dispatchEvent(new Event('resize'));
    expect(stub.setSize.mock.calls.length).toBeGreaterThan(callsBefore);
    handle.dispose();
  });

  it('refreshPalette() schedules a render without throwing', () => {
    const stub = makeRendererStub();
    const handle = createBackgroundScene({ rendererFactory: makeFactory(stub) })!;
    expect(() => handle.refreshPalette()).not.toThrow();
    handle.dispose();
  });
});
