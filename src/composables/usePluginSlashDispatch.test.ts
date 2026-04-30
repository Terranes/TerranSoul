/**
 * Tests for `usePluginSlashDispatch` (Chunk 22.4).
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';

import {
  parseSlashIntent,
  usePluginSlashDispatch,
} from './usePluginSlashDispatch';
import { usePluginStore } from '../stores/plugins';

const mockInvoke = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => mockInvoke(cmd, args),
}));

describe('parseSlashIntent', () => {
  const known = new Set(['translate', 'summarize']);

  it('returns null for non-slash messages', () => {
    expect(parseSlashIntent('hello world', known)).toBeNull();
    expect(parseSlashIntent('', known)).toBeNull();
  });

  it('returns null for unknown slash names', () => {
    expect(parseSlashIntent('/unknown thing', known)).toBeNull();
  });

  it('returns null for double-slash (URLs)', () => {
    expect(parseSlashIntent('//escape', known)).toBeNull();
  });

  it('returns null for bare slash', () => {
    expect(parseSlashIntent('/', known)).toBeNull();
  });

  it('parses a known slash with args', () => {
    expect(parseSlashIntent('/translate hello there', known)).toEqual({
      name: 'translate',
      args: 'hello there',
    });
  });

  it('parses a known slash without args', () => {
    expect(parseSlashIntent('/summarize', known)).toEqual({
      name: 'summarize',
      args: '',
    });
  });

  it('tolerates leading whitespace', () => {
    expect(parseSlashIntent('   /translate text', known)).toEqual({
      name: 'translate',
      args: 'text',
    });
  });

  it('collapses extra whitespace in args', () => {
    expect(parseSlashIntent('/translate    hello   ', known)).toEqual({
      name: 'translate',
      args: 'hello',
    });
  });
});

describe('usePluginSlashDispatch', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  function seedStoreWithSlash(name: string, commandId: string) {
    const store = usePluginStore();
    store.slashCommands = [
      { plugin_id: 'p', slash_command: { name, description: 'd', command_id: commandId } },
    ];
  }

  it('returns handled=false when no plugin claims the name', async () => {
    const { tryDispatchSlashCommand } = usePluginSlashDispatch();
    const r = await tryDispatchSlashCommand('hello there');
    expect(r.handled).toBe(false);
  });

  it('returns handled=false for unknown slash command (lets host handle)', async () => {
    const { tryDispatchSlashCommand } = usePluginSlashDispatch();
    const r = await tryDispatchSlashCommand('/unknown text');
    expect(r.handled).toBe(false);
  });

  it('dispatches a known slash to invokeSlashCommand and returns output', async () => {
    seedStoreWithSlash('translate', 'p.translate');
    mockInvoke.mockResolvedValueOnce({
      success: true,
      output: '[p] Translate — args: {"text":"hello"}',
      error: null,
    });

    const { tryDispatchSlashCommand } = usePluginSlashDispatch();
    const r = await tryDispatchSlashCommand('/translate hello');

    expect(r.handled).toBe(true);
    expect(r.name).toBe('translate');
    expect(r.args).toBe('hello');
    expect(r.output).toContain('Translate');
    expect(mockInvoke).toHaveBeenCalledWith('plugin_invoke_slash_command', {
      name: 'translate',
      args: { text: 'hello' },
    });
  });

  it('passes null args when slash has no argument string', async () => {
    seedStoreWithSlash('summarize', 'p.summarize');
    mockInvoke.mockResolvedValueOnce({
      success: true,
      output: 'ok',
      error: null,
    });

    const { tryDispatchSlashCommand } = usePluginSlashDispatch();
    await tryDispatchSlashCommand('/summarize');

    expect(mockInvoke).toHaveBeenCalledWith('plugin_invoke_slash_command', {
      name: 'summarize',
      args: null,
    });
  });

  it('captures backend errors as error string with handled=true', async () => {
    seedStoreWithSlash('translate', 'p.translate');
    mockInvoke.mockRejectedValueOnce('boom');

    const { tryDispatchSlashCommand } = usePluginSlashDispatch();
    const r = await tryDispatchSlashCommand('/translate hi');

    expect(r.handled).toBe(true);
    expect(r.error).toBe('boom');
    expect(r.output).toBeUndefined();
  });

  it('treats success=false as handled with error', async () => {
    seedStoreWithSlash('translate', 'p.translate');
    mockInvoke.mockResolvedValueOnce({
      success: false,
      output: null,
      error: 'plugin failed',
    });

    const { tryDispatchSlashCommand } = usePluginSlashDispatch();
    const r = await tryDispatchSlashCommand('/translate hi');

    expect(r.handled).toBe(true);
    expect(r.error).toBe('plugin failed');
  });
});
