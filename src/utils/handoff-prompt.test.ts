import { describe, it, expect } from 'vitest';
import {
  buildHandoffBlock,
  HANDOFF_MAX_CHARS,
  HANDOFF_MAX_LINES,
} from './handoff-prompt';

describe('buildHandoffBlock', () => {
  it('returns empty string for null/undefined input', () => {
    expect(buildHandoffBlock(null)).toBe('');
    expect(buildHandoffBlock(undefined)).toBe('');
  });

  it('returns empty string when prevAgentName is blank', () => {
    expect(buildHandoffBlock({ prevAgentName: '', context: 'hi' })).toBe('');
    expect(buildHandoffBlock({ prevAgentName: '   ', context: 'hi' })).toBe('');
  });

  it('returns empty string when context is blank or whitespace-only', () => {
    expect(buildHandoffBlock({ prevAgentName: 'Aria', context: '' })).toBe('');
    expect(buildHandoffBlock({ prevAgentName: 'Aria', context: '   \n  \t  ' })).toBe('');
  });

  it('renders a basic single-line block', () => {
    const out = buildHandoffBlock({
      prevAgentName: 'Aria',
      context: '[user] Hello there',
    });
    expect(out).toBe('\n\n[HANDOFF FROM Aria]\n[user] Hello there\n[/HANDOFF]');
  });

  it('renders multi-line context preserving order', () => {
    const out = buildHandoffBlock({
      prevAgentName: 'Aria',
      context: '[user] question one\n[assistant] answer one\n[user] question two',
    });
    expect(out).toContain('[user] question one\n[assistant] answer one\n[user] question two');
    expect(out.startsWith('\n\n[HANDOFF FROM Aria]\n')).toBe(true);
    expect(out.endsWith('\n[/HANDOFF]')).toBe(true);
  });

  it('drops empty lines and trims trailing whitespace per line', () => {
    const out = buildHandoffBlock({
      prevAgentName: 'Aria',
      context: '[user] alpha   \n\n\n[assistant] beta\t\t\n   \n[user] gamma',
    });
    expect(out).toContain('[user] alpha\n[assistant] beta\n[user] gamma');
    expect(out).not.toMatch(/\n\n\[user\] gamma/);
  });

  it('normalises CRLF to LF', () => {
    const out = buildHandoffBlock({
      prevAgentName: 'Aria',
      context: 'line one\r\nline two\r\nline three',
    });
    expect(out).toContain('line one\nline two\nline three');
    expect(out).not.toContain('\r');
  });

  it('strips control characters from the agent name', () => {
    const out = buildHandoffBlock({
      prevAgentName: 'Ar\x00ia',
      context: 'hi',
    });
    expect(out).toContain('[HANDOFF FROM Ar ia]');
  });

  it('keeps only the last HANDOFF_MAX_LINES non-empty lines', () => {
    const lines = Array.from({ length: HANDOFF_MAX_LINES + 10 }, (_, i) => `[user] line ${i}`);
    const out = buildHandoffBlock({
      prevAgentName: 'Aria',
      context: lines.join('\n'),
    });
    // The first 10 lines should be dropped (kept the *tail*).
    expect(out).not.toContain('[user] line 0\n');
    expect(out).not.toContain('[user] line 9\n');
    // The most-recent line must survive.
    expect(out).toContain(`[user] line ${HANDOFF_MAX_LINES + 9}`);
    // And the line just inside the window too.
    expect(out).toContain('[user] line 10\n');
  });

  it('hard-caps body at HANDOFF_MAX_CHARS with a truncation marker', () => {
    // Single huge line so line-cap can't help — must hit the char cap.
    const big = 'x'.repeat(HANDOFF_MAX_CHARS * 2);
    const out = buildHandoffBlock({
      prevAgentName: 'Aria',
      context: big,
    });
    // Body length (between header and footer) ≤ cap.
    const bodyMatch = out.match(/\[HANDOFF FROM Aria\]\n([\s\S]*)\n\[\/HANDOFF\]/);
    expect(bodyMatch).not.toBeNull();
    const body = bodyMatch![1];
    expect(body.length).toBeLessThanOrEqual(HANDOFF_MAX_CHARS);
    expect(body.startsWith('…(truncated)\n')).toBe(true);
    // Tail of the original input is preserved (the last x's).
    expect(body.endsWith('x')).toBe(true);
  });

  it('does not truncate when body is exactly at the cap', () => {
    const exact = 'a'.repeat(HANDOFF_MAX_CHARS);
    const out = buildHandoffBlock({
      prevAgentName: 'Aria',
      context: exact,
    });
    expect(out).not.toContain('…(truncated)');
    expect(out).toContain(exact);
  });

  it('ignores non-string context gracefully', () => {
    // @ts-expect-error — testing runtime guard against bad callers.
    expect(buildHandoffBlock({ prevAgentName: 'Aria', context: 42 })).toBe('');
    // @ts-expect-error — testing runtime guard against bad callers.
    expect(buildHandoffBlock({ prevAgentName: 'Aria', context: { foo: 'bar' } })).toBe('');
  });

  it('block format is stable for snapshot-style assertion', () => {
    const out = buildHandoffBlock({
      prevAgentName: 'Aria',
      context: '[user] hi\n[assistant] hello',
    });
    expect(out).toBe('\n\n[HANDOFF FROM Aria]\n[user] hi\n[assistant] hello\n[/HANDOFF]');
  });

  it('nextAgentName is accepted but never rendered', () => {
    const out = buildHandoffBlock({
      prevAgentName: 'Aria',
      nextAgentName: 'Beat',
      context: 'hi',
    });
    expect(out).not.toContain('Beat');
    expect(out).toContain('[HANDOFF FROM Aria]');
  });
});
