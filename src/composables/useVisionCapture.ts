import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { ScreenFrame, VisionAnalysis } from '../types';

export function useVisionCapture() {
  const lastFrame = ref<ScreenFrame | null>(null);
  const lastAnalysis = ref<VisionAnalysis | null>(null);
  const isCapturing = ref(false);
  const isAnalyzing = ref(false);
  const error = ref<string | null>(null);

  async function captureScreen(): Promise<ScreenFrame | null> {
    isCapturing.value = true;
    error.value = null;
    try {
      const frame = await invoke<ScreenFrame>('capture_screen');
      lastFrame.value = frame;
      return frame;
    } catch (e) {
      error.value = String(e);
      return null;
    } finally {
      isCapturing.value = false;
    }
  }

  async function analyzeScreen(frame?: ScreenFrame): Promise<VisionAnalysis | null> {
    const target = frame ?? lastFrame.value;
    if (!target) {
      error.value = 'No frame to analyze';
      return null;
    }
    isAnalyzing.value = true;
    error.value = null;
    try {
      const analysis = await invoke<VisionAnalysis>('analyze_screen', { frame: target });
      lastAnalysis.value = analysis;
      return analysis;
    } catch (e) {
      error.value = String(e);
      return null;
    } finally {
      isAnalyzing.value = false;
    }
  }

  async function captureAndAnalyze(): Promise<VisionAnalysis | null> {
    const frame = await captureScreen();
    if (!frame) return null;
    return analyzeScreen(frame);
  }

  return {
    lastFrame,
    lastAnalysis,
    isCapturing,
    isAnalyzing,
    error,
    captureScreen,
    analyzeScreen,
    captureAndAnalyze,
  };
}
