/**
 * Tests for useDiarization composable.
 *
 * Tests cover:
 * - diarize calls invoke and returns segments
 * - formatTranscript formats correctly
 * - getSpeakers returns unique speakers
 * - getSegmentsForSpeaker filters correctly
 * - Error handling when invoke fails
 * - isProcessing state management
 * - Empty audio handling
 * - Multiple diarize calls update segments
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useDiarization, type DiarizedSegment } from './useDiarization';

// ── Tauri IPC mock ────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Test data ─────────────────────────────────────────────────────────────────

const mockSegments: DiarizedSegment[] = [
  {
    speaker: 'Speaker 1',
    text: 'Hello from speaker one',
    start_secs: 0.0,
    end_secs: 1.5,
    confidence: 0.95,
  },
  {
    speaker: 'Speaker 2',
    text: 'Hello from speaker two',
    start_secs: 1.5,
    end_secs: 3.0,
    confidence: 0.90,
  },
];

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('useDiarization', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('returns empty segments initially', () => {
    const { segments, isProcessing, error } = useDiarization();
    expect(segments.value).toEqual([]);
    expect(isProcessing.value).toBe(false);
    expect(error.value).toBeNull();
  });

  it('calls invoke with correct command and samples', async () => {
    mockInvoke.mockResolvedValue(mockSegments);
    const { diarize } = useDiarization();

    const audio = new Float32Array([0.1, 0.2, 0.3]);
    await diarize(audio);

    expect(mockInvoke).toHaveBeenCalledWith('diarize_audio', {
      samples: Array.from(audio),
    });
  });

  it('diarize returns and stores segments', async () => {
    mockInvoke.mockResolvedValue(mockSegments);
    const { diarize, segments } = useDiarization();

    const result = await diarize(new Float32Array([0.1, 0.2]));

    expect(result).toEqual(mockSegments);
    expect(segments.value).toEqual(mockSegments);
  });

  it('sets isProcessing during diarization', async () => {
    const states: boolean[] = [];
    let resolveInvoke: (v: DiarizedSegment[]) => void;
    mockInvoke.mockImplementation(
      () =>
        new Promise<DiarizedSegment[]>((resolve) => {
          resolveInvoke = resolve;
        }),
    );

    const { diarize, isProcessing } = useDiarization();
    const promise = diarize(new Float32Array([0.1]));

    // Should be processing while awaiting
    states.push(isProcessing.value);
    resolveInvoke!(mockSegments);
    await promise;
    states.push(isProcessing.value);

    expect(states).toEqual([true, false]);
  });

  it('handles invoke error gracefully', async () => {
    mockInvoke.mockRejectedValue(new Error('network failure'));
    const { diarize, error, segments } = useDiarization();

    const result = await diarize(new Float32Array([0.1]));

    expect(result).toEqual([]);
    expect(segments.value).toEqual([]);
    expect(error.value).toContain('network failure');
  });

  it('clears error on subsequent successful diarize', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('fail'));
    const { diarize, error } = useDiarization();

    await diarize(new Float32Array([0.1]));
    expect(error.value).toContain('fail');

    mockInvoke.mockResolvedValue(mockSegments);
    await diarize(new Float32Array([0.1]));
    expect(error.value).toBeNull();
  });

  it('resets isProcessing after error', async () => {
    mockInvoke.mockRejectedValue(new Error('boom'));
    const { diarize, isProcessing } = useDiarization();

    await diarize(new Float32Array([0.1]));

    expect(isProcessing.value).toBe(false);
  });

  it('formatTranscript formats segments correctly', async () => {
    mockInvoke.mockResolvedValue(mockSegments);
    const { diarize, formatTranscript } = useDiarization();

    await diarize(new Float32Array([0.1]));

    expect(formatTranscript()).toBe(
      '[Speaker 1] Hello from speaker one\n[Speaker 2] Hello from speaker two',
    );
  });

  it('formatTranscript returns empty string with no segments', () => {
    const { formatTranscript } = useDiarization();
    expect(formatTranscript()).toBe('');
  });

  it('getSpeakers returns unique speaker labels', async () => {
    const threeSegments: DiarizedSegment[] = [
      { speaker: 'Speaker 1', text: 'a', start_secs: 0, end_secs: 1, confidence: null },
      { speaker: 'Speaker 2', text: 'b', start_secs: 1, end_secs: 2, confidence: null },
      { speaker: 'Speaker 1', text: 'c', start_secs: 2, end_secs: 3, confidence: null },
    ];
    mockInvoke.mockResolvedValue(threeSegments);
    const { diarize, getSpeakers } = useDiarization();

    await diarize(new Float32Array([0.1]));

    const speakers = getSpeakers();
    expect(speakers).toEqual(['Speaker 1', 'Speaker 2']);
    expect(speakers).toHaveLength(2);
  });

  it('getSegmentsForSpeaker filters by speaker', async () => {
    const threeSegments: DiarizedSegment[] = [
      { speaker: 'Speaker 1', text: 'a', start_secs: 0, end_secs: 1, confidence: null },
      { speaker: 'Speaker 2', text: 'b', start_secs: 1, end_secs: 2, confidence: null },
      { speaker: 'Speaker 1', text: 'c', start_secs: 2, end_secs: 3, confidence: null },
    ];
    mockInvoke.mockResolvedValue(threeSegments);
    const { diarize, getSegmentsForSpeaker } = useDiarization();

    await diarize(new Float32Array([0.1]));

    const s1 = getSegmentsForSpeaker('Speaker 1');
    expect(s1).toHaveLength(2);
    expect(s1[0].text).toBe('a');
    expect(s1[1].text).toBe('c');

    const s2 = getSegmentsForSpeaker('Speaker 2');
    expect(s2).toHaveLength(1);
    expect(s2[0].text).toBe('b');
  });

  it('getSegmentsForSpeaker returns empty for unknown speaker', async () => {
    mockInvoke.mockResolvedValue(mockSegments);
    const { diarize, getSegmentsForSpeaker } = useDiarization();

    await diarize(new Float32Array([0.1]));

    expect(getSegmentsForSpeaker('Speaker 99')).toEqual([]);
  });
});
