/**
 * Tests for useBgmPlayer composable.
 *
 * All tracks (built-in and custom) use HTMLAudioElement, so we mock `Audio`
 * globally and test reactive state transitions + API contract.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import { useBgmPlayer, BGM_TRACKS, DEFAULT_BGM_VOLUME } from './useBgmPlayer';

// ── HTMLAudioElement mock ─────────────────────────────────────────────────────

function stubAudio() {
  const instances: Record<string, unknown>[] = [];
  vi.stubGlobal('Audio', class {
    loop = false;
    volume = 1;
    src = '';
    play = vi.fn(() => Promise.resolve());
    pause = vi.fn();
    removeAttribute = vi.fn();
    load = vi.fn();
    constructor(src?: string) {
      if (src) (this as Record<string, unknown>).src = src;
      instances.push(this as unknown as Record<string, unknown>);
    }
  });
  return instances;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('useBgmPlayer', () => {
  it('exports preset tracks with src', () => {
    expect(BGM_TRACKS.length).toBeGreaterThanOrEqual(3);
    expect(BGM_TRACKS[0]).toHaveProperty('id');
    expect(BGM_TRACKS[0]).toHaveProperty('name');
    expect(BGM_TRACKS[0]).toHaveProperty('src');
  });

  it('has sensible default volume', () => {
    expect(DEFAULT_BGM_VOLUME).toBeGreaterThan(0);
    expect(DEFAULT_BGM_VOLUME).toBeLessThanOrEqual(0.5);
  });

  it('starts in stopped state', () => {
    const bgm = useBgmPlayer();
    expect(bgm.isPlaying.value).toBe(false);
    expect(bgm.currentTrackId.value).toBeNull();
    expect(bgm.volume.value).toBeCloseTo(DEFAULT_BGM_VOLUME);
  });

  it('play() sets isPlaying and currentTrackId', () => {
    stubAudio();
    const bgm = useBgmPlayer();
    bgm.play('prelude');
    expect(bgm.isPlaying.value).toBe(true);
    expect(bgm.currentTrackId.value).toBe('prelude');
  });

  it('stop() clears playing state', () => {
    stubAudio();
    const bgm = useBgmPlayer();
    bgm.play('prelude');
    bgm.stop();
    expect(bgm.isPlaying.value).toBe(false);
    expect(bgm.currentTrackId.value).toBeNull();
  });

  it('play() switches tracks when already playing', () => {
    stubAudio();
    const bgm = useBgmPlayer();
    bgm.play('prelude');
    expect(bgm.currentTrackId.value).toBe('prelude');
    bgm.play('moonflow');
    expect(bgm.currentTrackId.value).toBe('moonflow');
    expect(bgm.isPlaying.value).toBe(true);
  });

  it('setVolume() clamps to [0, 1]', () => {
    const bgm = useBgmPlayer();
    bgm.setVolume(0.5);
    expect(bgm.volume.value).toBeCloseTo(0.5);
    bgm.setVolume(-0.1);
    expect(bgm.volume.value).toBe(0);
    bgm.setVolume(1.5);
    expect(bgm.volume.value).toBe(1);
  });

  it('setVolume() updates while playing', () => {
    stubAudio();
    const bgm = useBgmPlayer();
    bgm.play('prelude');
    bgm.setVolume(0.7);
    expect(bgm.volume.value).toBeCloseTo(0.7);
    expect(bgm.isPlaying.value).toBe(true);
  });

  it('stop() is safe to call when already stopped', () => {
    const bgm = useBgmPlayer();
    expect(() => bgm.stop()).not.toThrow();
    expect(bgm.isPlaying.value).toBe(false);
  });

  it('all preset track IDs are unique', () => {
    const ids = BGM_TRACKS.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  // ── Custom track management ───────────────────────────────────────

  it('allTracks includes both builtins and custom tracks', () => {
    const bgm = useBgmPlayer();
    expect(bgm.allTracks.value.length).toBe(BGM_TRACKS.length);
    bgm.addCustomTrack('My Song', 'https://example.com/song.mp3');
    expect(bgm.allTracks.value.length).toBe(BGM_TRACKS.length + 1);
  });

  it('addCustomTrack returns a unique ID', () => {
    const bgm = useBgmPlayer();
    const id1 = bgm.addCustomTrack('Song 1', 'https://example.com/1.mp3');
    const id2 = bgm.addCustomTrack('Song 2', 'https://example.com/2.mp3');
    expect(id1).toBeTruthy();
    expect(id2).toBeTruthy();
    expect(id1).not.toBe(id2);
    expect(bgm.customTracks.value.length).toBe(2);
  });

  it('removeTrack removes a custom track and returns true', () => {
    const bgm = useBgmPlayer();
    const id = bgm.addCustomTrack('My Song', 'https://example.com/song.mp3');
    expect(bgm.removeTrack(id)).toBe(true);
    expect(bgm.customTracks.value.length).toBe(0);
  });

  it('removeTrack returns false for non-existent track', () => {
    const bgm = useBgmPlayer();
    expect(bgm.removeTrack('nonexistent')).toBe(false);
  });

  it('removeTrack stops playback if removed track was playing', () => {
    stubAudio();
    const bgm = useBgmPlayer();
    const id = bgm.addCustomTrack('My Song', 'https://example.com/song.mp3');
    bgm.play(id);
    expect(bgm.isPlaying.value).toBe(true);
    bgm.removeTrack(id);
    expect(bgm.isPlaying.value).toBe(false);
  });

  it('loadCustomTracks replaces custom tracks', () => {
    const bgm = useBgmPlayer();
    bgm.addCustomTrack('Old Track', 'https://example.com/old.mp3');
    bgm.loadCustomTracks([
      { id: 'c1', name: 'Track A', src: 'https://example.com/a.mp3' },
      { id: 'c2', name: 'Track B', src: 'https://example.com/b.mp3' },
    ]);
    expect(bgm.customTracks.value.length).toBe(2);
    expect(bgm.customTracks.value[0].name).toBe('Track A');
    expect(bgm.customTracks.value[0].removable).toBe(true);
  });

  it('play() uses HTMLAudioElement for tracks with src', () => {
    const instances = stubAudio();
    const bgm = useBgmPlayer();
    bgm.play('prelude');
    expect(bgm.isPlaying.value).toBe(true);
    expect(bgm.currentTrackId.value).toBe('prelude');
    const last = instances[instances.length - 1];
    expect(last.loop).toBe(true);
    expect(last.play).toHaveBeenCalled();
  });

  it('setVolume updates HTMLAudioElement volume', () => {
    const instances = stubAudio();
    const bgm = useBgmPlayer();
    bgm.play('prelude');
    bgm.setVolume(0.3);
    expect(bgm.volume.value).toBeCloseTo(0.3);
    const last = instances[instances.length - 1];
    expect(last.volume).toBeCloseTo(0.3);
  });

  it('play() does nothing for unknown track without src', () => {
    const bgm = useBgmPlayer();
    bgm.play('nonexistent');
    expect(bgm.isPlaying.value).toBe(false);
    expect(bgm.currentTrackId.value).toBeNull();
  });
});
