import { describe, expect, it } from 'vitest';
import {
  BRAIN_WIKI_HELP_TEXT,
  parseBrainWikiSlashCommand,
  parseSlashCommand,
  SLASH_HELP_TEXT,
} from './slash-commands';

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

describe('parseBrainWikiSlashCommand', () => {
  it('ignores plain chat', () => {
    expect(parseBrainWikiSlashCommand('tell me about memory')).toBeNull();
  });

  it('parses supported wiki commands case-insensitively', () => {
    const out = parseBrainWikiSlashCommand(' /SPOTLIGHT ');
    expect(out?.kind).toBe('spotlight');
    expect(out?.arg).toBe('');
  });

  it('preserves digest text as a single argument', () => {
    const out = parseBrainWikiSlashCommand('/digest The brain should remember this source note.');
    expect(out?.kind).toBe('digest');
    expect(out?.arg).toBe('The brain should remember this source note.');
  });

  it('parses path-style trace arguments', () => {
    const out = parseBrainWikiSlashCommand('/trace 12 42');
    expect(out?.kind).toBe('trace');
    expect(out?.arg).toBe('12 42');
  });

  it('does not claim plugin or self-improve commands', () => {
    expect(parseBrainWikiSlashCommand('/clear')).toBeNull();
    expect(parseBrainWikiSlashCommand('/plugin-command args')).toBeNull();
  });

  it('documents the active and planned wiki verbs', () => {
    expect(BRAIN_WIKI_HELP_TEXT).toContain('/ponder');
    expect(BRAIN_WIKI_HELP_TEXT).toContain('/spotlight');
    expect(BRAIN_WIKI_HELP_TEXT).toContain('/weave');
  });
});
