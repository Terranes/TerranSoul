import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface DiarizedSegment {
  speaker: string;
  text: string;
  start_secs: number;
  end_secs: number;
  confidence: number | null;
}

export function useDiarization() {
  const segments = ref<DiarizedSegment[]>([]);
  const isProcessing = ref(false);
  const error = ref<string | null>(null);

  async function diarize(samples: Float32Array): Promise<DiarizedSegment[]> {
    isProcessing.value = true;
    error.value = null;
    try {
      const result = await invoke<DiarizedSegment[]>('diarize_audio', {
        samples: Array.from(samples),
      });
      segments.value = result;
      return result;
    } catch (e) {
      error.value = String(e);
      return [];
    } finally {
      isProcessing.value = false;
    }
  }

  function formatTranscript(): string {
    return segments.value
      .map(s => `[${s.speaker}] ${s.text}`)
      .join('\n');
  }

  function getSpeakers(): string[] {
    return [...new Set(segments.value.map(s => s.speaker))];
  }

  function getSegmentsForSpeaker(speaker: string): DiarizedSegment[] {
    return segments.value.filter(s => s.speaker === speaker);
  }

  return {
    segments,
    isProcessing,
    error,
    diarize,
    formatTranscript,
    getSpeakers,
    getSegmentsForSpeaker,
  };
}
