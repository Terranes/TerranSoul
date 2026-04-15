/**
 * Tests for useVisionCapture composable.
 *
 * Tests cover:
 * - captureScreen calls invoke and returns frame
 * - captureScreen sets lastFrame on success
 * - captureScreen sets error on failure
 * - captureScreen manages isCapturing state
 * - analyzeScreen calls invoke with frame
 * - analyzeScreen uses lastFrame if no frame provided
 * - analyzeScreen errors when no frame available
 * - analyzeScreen manages isAnalyzing state
 * - analyzeScreen sets error on failure
 * - captureAndAnalyze does both in sequence
 * - captureAndAnalyze returns null if capture fails
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useVisionCapture } from './useVisionCapture';
import type { ScreenFrame, VisionAnalysis } from '../types';

// ── Tauri IPC mock ────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

const stubFrame: ScreenFrame = {
  image_b64: 'iVBORw0KGgo=',
  width: 1,
  height: 1,
  captured_at: 1000,
  active_window_title: 'TerranSoul',
};

const stubAnalysis: VisionAnalysis = {
  description: 'User is working in TerranSoul (1×1)',
  activity: 'working',
  confidence: 0.85,
  analyzed_at: 2000,
};

describe('useVisionCapture', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('initialises with null state and no errors', () => {
    const { lastFrame, lastAnalysis, isCapturing, isAnalyzing, error } = useVisionCapture();
    expect(lastFrame.value).toBeNull();
    expect(lastAnalysis.value).toBeNull();
    expect(isCapturing.value).toBe(false);
    expect(isAnalyzing.value).toBe(false);
    expect(error.value).toBeNull();
  });

  it('captureScreen calls invoke and returns frame', async () => {
    mockInvoke.mockResolvedValue(stubFrame);

    const { captureScreen } = useVisionCapture();
    const result = await captureScreen();

    expect(mockInvoke).toHaveBeenCalledWith('capture_screen');
    expect(result).toEqual(stubFrame);
  });

  it('captureScreen sets lastFrame on success', async () => {
    mockInvoke.mockResolvedValue(stubFrame);

    const { lastFrame, captureScreen } = useVisionCapture();
    await captureScreen();

    expect(lastFrame.value).toEqual(stubFrame);
  });

  it('captureScreen sets error on failure', async () => {
    mockInvoke.mockRejectedValue('capture failed');

    const { error, captureScreen } = useVisionCapture();
    const result = await captureScreen();

    expect(result).toBeNull();
    expect(error.value).toBe('capture failed');
  });

  it('captureScreen resets isCapturing after completion', async () => {
    mockInvoke.mockResolvedValue(stubFrame);

    const { isCapturing, captureScreen } = useVisionCapture();
    await captureScreen();

    expect(isCapturing.value).toBe(false);
  });

  it('captureScreen resets isCapturing after error', async () => {
    mockInvoke.mockRejectedValue('fail');

    const { isCapturing, captureScreen } = useVisionCapture();
    await captureScreen();

    expect(isCapturing.value).toBe(false);
  });

  it('analyzeScreen calls invoke with provided frame', async () => {
    mockInvoke.mockResolvedValue(stubAnalysis);

    const { analyzeScreen } = useVisionCapture();
    const customFrame: ScreenFrame = {
      image_b64: 'dGVzdA==',
      width: 1920,
      height: 1080,
      captured_at: 500,
      active_window_title: 'VS Code',
    };
    const result = await analyzeScreen(customFrame);

    expect(mockInvoke).toHaveBeenCalledWith('analyze_screen', { frame: customFrame });
    expect(result).toEqual(stubAnalysis);
  });

  it('analyzeScreen uses lastFrame if no frame provided', async () => {
    mockInvoke
      .mockResolvedValueOnce(stubFrame)    // capture_screen
      .mockResolvedValueOnce(stubAnalysis); // analyze_screen

    const { captureScreen, analyzeScreen } = useVisionCapture();
    await captureScreen();
    const result = await analyzeScreen();

    expect(mockInvoke).toHaveBeenCalledWith('analyze_screen', { frame: stubFrame });
    expect(result).toEqual(stubAnalysis);
  });

  it('analyzeScreen errors when no frame available', async () => {
    const { error, analyzeScreen } = useVisionCapture();
    const result = await analyzeScreen();

    expect(result).toBeNull();
    expect(error.value).toBe('No frame to analyze');
  });

  it('analyzeScreen resets isAnalyzing after completion', async () => {
    mockInvoke.mockResolvedValue(stubAnalysis);

    const { isAnalyzing, analyzeScreen } = useVisionCapture();
    await analyzeScreen(stubFrame);

    expect(isAnalyzing.value).toBe(false);
  });

  it('analyzeScreen sets error on failure', async () => {
    mockInvoke.mockRejectedValue('analysis failed');

    const { error, analyzeScreen } = useVisionCapture();
    const result = await analyzeScreen(stubFrame);

    expect(result).toBeNull();
    expect(error.value).toBe('analysis failed');
  });

  it('analyzeScreen sets lastAnalysis on success', async () => {
    mockInvoke.mockResolvedValue(stubAnalysis);

    const { lastAnalysis, analyzeScreen } = useVisionCapture();
    await analyzeScreen(stubFrame);

    expect(lastAnalysis.value).toEqual(stubAnalysis);
  });

  it('captureAndAnalyze does both in sequence', async () => {
    mockInvoke
      .mockResolvedValueOnce(stubFrame)    // capture_screen
      .mockResolvedValueOnce(stubAnalysis); // analyze_screen

    const { lastFrame, lastAnalysis, captureAndAnalyze } = useVisionCapture();
    const result = await captureAndAnalyze();

    expect(mockInvoke).toHaveBeenCalledTimes(2);
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'capture_screen');
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'analyze_screen', { frame: stubFrame });
    expect(lastFrame.value).toEqual(stubFrame);
    expect(lastAnalysis.value).toEqual(stubAnalysis);
    expect(result).toEqual(stubAnalysis);
  });

  it('captureAndAnalyze returns null if capture fails', async () => {
    mockInvoke.mockRejectedValue('capture failed');

    const { captureAndAnalyze } = useVisionCapture();
    const result = await captureAndAnalyze();

    expect(result).toBeNull();
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });
});
