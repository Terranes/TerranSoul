import { describe, expect, it } from 'vitest';
import { parseSlashCommand, SLASH_HELP_TEXT } from './slash-commands';

describe('parseSlashCommand', () => {
  it('treats plain text as a chat message', () => {
    const out = parseSlashCommand('what does the brain store?');
    expect(out.kind).toBe('chat');
    expect(out.arg).toBe('what does the brain store?');
  });

  it('parses /clear with no argument', () => {
    const out = parseSlashCommand('  /clear  ');
    expect(out.kind).toBe('clear');
    expect(out.arg).toBe('');
  });

  it('extracts the argument after a command', () => {
    const out = parseSlashCommand('/rename auth-refactor');
    expect(out.kind).toBe('rename');
    expect(out.arg).toBe('auth-refactor');
  });

  it('preserves arguments containing spaces', () => {
    const out = parseSlashCommand('/resume my long session name');
    expect(out.kind).toBe('resume');
    expect(out.arg).toBe('my long session name');
  });

  it('is case-insensitive on the command token', () => {
    const out = parseSlashCommand('/HELP');
    expect(out.kind).toBe('help');
  });

  it('treats a bare slash as help', () => {
    expect(parseSlashCommand('/').kind).toBe('help');
  });

  it('flags unknown slash commands without falling back to chat', () => {
    const out = parseSlashCommand('/teapot brew');
    expect(out.kind).toBe('unknown');
    expect(out.command).toBe('teapot');
    expect(out.arg).toBe('brew');
  });

  it('exposes a non-empty help text', () => {
    expect(SLASH_HELP_TEXT.length).toBeGreaterThan(0);
    expect(SLASH_HELP_TEXT).toContain('/clear');
    expect(SLASH_HELP_TEXT).toContain('/rename');
    expect(SLASH_HELP_TEXT).toContain('/fork');
    expect(SLASH_HELP_TEXT).toContain('/resume');
  });
});
