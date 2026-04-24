import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  tokenizeToVisemes,
  buildVisemeTimeline,
  VisemeScheduler,
} from './phoneme-viseme';

// ── tokenizeToVisemes ───────────────────────────────────────────────────────

describe('tokenizeToVisemes', () => {
  it('returns empty for empty string', () => {
    expect(tokenizeToVisemes('')).toEqual([]);
  });

  it('maps single vowels to expected visemes', () => {
    // Use spaces to prevent 'ou' digraph matching
    const tokens = tokenizeToVisemes('a e i o u');
    const visemes = tokens.filter(t => t.viseme !== 'silent').map(t => t.viseme);
    expect(visemes).toEqual(['aa', 'ee', 'ih', 'oh', 'ou']);
  });

  it('maps labial consonants to silent (lips close)', () => {
    const tokens = tokenizeToVisemes('b m p');
    const visemes = tokens.filter(t => t.viseme !== 'silent').map(t => t.viseme);
    // b, m, p are all silent (lips close) — only whitespace is also silent
    expect(visemes).toEqual([]);
  });

  it('handles digraphs before single chars', () => {
    const tokens = tokenizeToVisemes('sh');
    expect(tokens.length).toBe(1);
    expect(tokens[0].viseme).toBe('ou'); // sh → ou (rounded lips)
  });

  it('handles "th" digraph', () => {
    const tokens = tokenizeToVisemes('the');
    expect(tokens[0].viseme).toBe('ee'); // th → ee
    expect(tokens[1].viseme).toBe('ee'); // e → ee
  });

  it('handles "oo" vowel digraph', () => {
    const tokens = tokenizeToVisemes('moon');
    const visemes = tokens.map(t => t.viseme);
    // m → silent, oo → ou, n → ee
    expect(visemes).toEqual(['silent', 'ou', 'ee']);
  });

  it('skips numbers and unknown chars', () => {
    const tokens = tokenizeToVisemes('a1b');
    expect(tokens.length).toBe(2); // 'a' and 'b', '1' skipped
  });

  it('produces silent tokens for spaces and punctuation', () => {
    const tokens = tokenizeToVisemes('hi, there!');
    const silentCount = tokens.filter(t => t.viseme === 'silent').length;
    expect(silentCount).toBeGreaterThanOrEqual(2); // comma + space + exclaim
  });

  it('is case insensitive', () => {
    const lower = tokenizeToVisemes('hello');
    const upper = tokenizeToVisemes('HELLO');
    expect(lower.map(t => t.viseme)).toEqual(upper.map(t => t.viseme));
  });
});

// ── buildVisemeTimeline ─────────────────────────────────────────────────────

describe('buildVisemeTimeline', () => {
  it('returns empty for empty text', () => {
    expect(buildVisemeTimeline('', 2.0)).toEqual([]);
  });

  it('returns empty for zero duration', () => {
    expect(buildVisemeTimeline('hello', 0)).toEqual([]);
  });

  it('builds timeline spanning the duration', () => {
    const kf = buildVisemeTimeline('hello', 1.0);
    expect(kf.length).toBeGreaterThan(1);
    expect(kf[0].time).toBe(0);
    expect(kf[kf.length - 1].time).toBe(1.0); // ends with silence at duration
  });

  it('ends with silent keyframe', () => {
    const kf = buildVisemeTimeline('test', 0.5);
    const last = kf[kf.length - 1];
    expect(last.weights.aa).toBe(0);
    expect(last.weights.ih).toBe(0);
    expect(last.weights.ou).toBe(0);
    expect(last.weights.ee).toBe(0);
    expect(last.weights.oh).toBe(0);
  });

  it('all keyframe times are ascending', () => {
    const kf = buildVisemeTimeline('hello world', 2.0);
    for (let i = 1; i < kf.length; i++) {
      expect(kf[i].time).toBeGreaterThanOrEqual(kf[i - 1].time);
    }
  });

  it('all weights are in [0, 1] range', () => {
    const kf = buildVisemeTimeline('the quick brown fox', 3.0);
    for (const frame of kf) {
      expect(frame.weights.aa).toBeGreaterThanOrEqual(0);
      expect(frame.weights.aa).toBeLessThanOrEqual(1);
      expect(frame.weights.ih).toBeGreaterThanOrEqual(0);
      expect(frame.weights.ou).toBeGreaterThanOrEqual(0);
      expect(frame.weights.ee).toBeGreaterThanOrEqual(0);
      expect(frame.weights.oh).toBeGreaterThanOrEqual(0);
    }
  });
});

// ── VisemeScheduler ─────────────────────────────────────────────────────────

describe('VisemeScheduler', () => {
  let scheduler: VisemeScheduler;

  beforeEach(() => {
    scheduler = new VisemeScheduler();
  });

  it('is inactive by default', () => {
    expect(scheduler.active).toBe(false);
  });

  it('becomes active after schedule()', () => {
    scheduler.schedule('hello', 1.0);
    expect(scheduler.active).toBe(true);
  });

  it('stays inactive for empty text', () => {
    scheduler.schedule('', 1.0);
    expect(scheduler.active).toBe(false);
  });

  it('sample() returns silent when inactive', () => {
    const w = scheduler.sample();
    expect(w.aa).toBe(0);
    expect(w.ih).toBe(0);
    expect(w.ou).toBe(0);
    expect(w.ee).toBe(0);
    expect(w.oh).toBe(0);
  });

  it('sample() returns valid weights when active', () => {
    // Mock performance.now so the scheduler sees us at time 0
    vi.spyOn(performance, 'now').mockReturnValue(0);
    scheduler.schedule('hello', 1.0);
    vi.spyOn(performance, 'now').mockReturnValue(200); // 200ms into 1s
    const w = scheduler.sample();
    // Should have some non-zero weight (we're in the middle of "hello")
    const total = w.aa + w.ih + w.ou + w.ee + w.oh;
    expect(total).toBeGreaterThan(0);
  });

  it('becomes inactive after duration elapses', () => {
    vi.spyOn(performance, 'now').mockReturnValue(0);
    scheduler.schedule('hi', 0.5);
    // Jump past the end
    vi.spyOn(performance, 'now').mockReturnValue(600); // 600ms > 500ms duration
    scheduler.sample();
    expect(scheduler.active).toBe(false);
  });

  it('stop() clears the schedule', () => {
    scheduler.schedule('hello', 1.0);
    expect(scheduler.active).toBe(true);
    scheduler.stop();
    expect(scheduler.active).toBe(false);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });
});
