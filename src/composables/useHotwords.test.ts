/**
 * Tests for useHotwords composable.
 *
 * Tests cover:
 * - loadHotwords fetches from backend
 * - addHotword calls invoke with correct args
 * - addHotword uses default boost of 5.0
 * - removeHotword calls invoke
 * - clearHotwords calls invoke
 * - Error handling
 * - isLoading state
 * - hotwords ref updates
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useHotwords } from './useHotwords';

// ── Tauri IPC mock ────────────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('useHotwords', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('initialises with empty hotwords, not loading, no error', () => {
    const { hotwords, isLoading, error } = useHotwords();
    expect(hotwords.value).toEqual([]);
    expect(isLoading.value).toBe(false);
    expect(error.value).toBeNull();
  });

  it('loadHotwords fetches from backend and updates ref', async () => {
    const data = [
      { phrase: 'Kerrigan', boost: 8.0 },
      { phrase: 'Zeratul', boost: 6.0 },
    ];
    mockInvoke.mockResolvedValue(data);

    const { hotwords, loadHotwords } = useHotwords();
    await loadHotwords();

    expect(mockInvoke).toHaveBeenCalledWith('get_hotwords');
    expect(hotwords.value).toEqual(data);
  });

  it('loadHotwords sets error on failure', async () => {
    mockInvoke.mockRejectedValue('backend error');

    const { error, loadHotwords } = useHotwords();
    await loadHotwords();

    expect(error.value).toBe('backend error');
  });

  it('loadHotwords resets isLoading after completion', async () => {
    mockInvoke.mockResolvedValue([]);

    const { isLoading, loadHotwords } = useHotwords();
    await loadHotwords();

    expect(isLoading.value).toBe(false);
  });

  it('addHotword calls invoke with correct args', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { addHotword } = useHotwords();
    await addHotword('Artanis', 7.0);

    expect(mockInvoke).toHaveBeenCalledWith('add_hotword', { phrase: 'Artanis', boost: 7.0 });
  });

  it('addHotword uses default boost of 5.0', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { addHotword } = useHotwords();
    await addHotword('Protoss');

    expect(mockInvoke).toHaveBeenCalledWith('add_hotword', { phrase: 'Protoss', boost: 5.0 });
  });

  it('addHotword reloads hotwords after adding', async () => {
    const data = [{ phrase: 'Zealot', boost: 5.0 }];
    // First call: add_hotword, second call: get_hotwords (reload)
    mockInvoke
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(data);

    const { hotwords, addHotword } = useHotwords();
    await addHotword('Zealot');

    expect(mockInvoke).toHaveBeenCalledTimes(2);
    expect(hotwords.value).toEqual(data);
  });

  it('addHotword sets error on failure', async () => {
    mockInvoke.mockRejectedValue(new Error('duplicate'));

    const { error, addHotword } = useHotwords();
    await addHotword('Kerrigan', 8.0);

    expect(error.value).toBe('duplicate');
  });

  it('removeHotword calls invoke with phrase', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { removeHotword } = useHotwords();
    await removeHotword('Zeratul');

    expect(mockInvoke).toHaveBeenCalledWith('remove_hotword', { phrase: 'Zeratul' });
  });

  it('removeHotword reloads hotwords after removing', async () => {
    mockInvoke
      .mockResolvedValueOnce(undefined) // remove_hotword
      .mockResolvedValueOnce([]); // get_hotwords (reload)

    const { hotwords, removeHotword } = useHotwords();
    await removeHotword('Zeratul');

    expect(mockInvoke).toHaveBeenCalledTimes(2);
    expect(hotwords.value).toEqual([]);
  });

  it('clearHotwords calls invoke and empties ref', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { hotwords, clearHotwords } = useHotwords();
    // Pre-populate
    hotwords.value = [{ phrase: 'test', boost: 1.0 }];

    await clearHotwords();

    expect(mockInvoke).toHaveBeenCalledWith('clear_hotwords');
    expect(hotwords.value).toEqual([]);
  });

  it('clearHotwords sets error on failure', async () => {
    mockInvoke.mockRejectedValue('clear failed');

    const { error, clearHotwords } = useHotwords();
    await clearHotwords();

    expect(error.value).toBe('clear failed');
  });

  it('isLoading is false after error', async () => {
    mockInvoke.mockRejectedValue('fail');

    const { isLoading, loadHotwords } = useHotwords();
    await loadHotwords();

    expect(isLoading.value).toBe(false);
  });
});
