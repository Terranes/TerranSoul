import { describe, it, expect, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { usePromptCommandsStore } from '../stores/prompt-commands';
import { usePromptCommandDispatch } from './usePromptCommandDispatch';

describe('usePromptCommandDispatch', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('returns handled:false for non-slash messages', () => {
    const { tryDispatchPromptCommand } = usePromptCommandDispatch();
    expect(tryDispatchPromptCommand('hello world')).toEqual({ handled: false });
  });

  it('returns handled:false for unknown slash commands', () => {
    const { tryDispatchPromptCommand } = usePromptCommandDispatch();
    expect(tryDispatchPromptCommand('/unknown-cmd')).toEqual({ handled: false });
  });

  it('dispatches a loaded prompt command', () => {
    const store = usePromptCommandsStore();
    store.commands = [
      {
        name: 'test-cmd',
        description: 'A test command',
        content: '# Test\nDo the thing with {{input}}',
        source: '/fake/path/test-cmd.md',
        mode: 'all',
      },
    ];

    const { tryDispatchPromptCommand } = usePromptCommandDispatch();
    const result = tryDispatchPromptCommand('/test-cmd some argument');

    expect(result.handled).toBe(true);
    expect(result.name).toBe('test-cmd');
    expect(result.prompt).toContain('Do the thing with some argument');
  });

  it('replaces {{date}} template variable', () => {
    const store = usePromptCommandsStore();
    store.commands = [
      {
        name: 'daily',
        description: 'Daily prompt',
        content: 'Today is {{date}}',
        source: '/fake/daily.md',
        mode: 'all',
      },
    ];

    const { tryDispatchPromptCommand } = usePromptCommandDispatch();
    const result = tryDispatchPromptCommand('/daily');

    expect(result.handled).toBe(true);
    expect(result.prompt).toMatch(/\d{4}-\d{2}-\d{2}/);
  });

  it('dispatches with no args (empty input)', () => {
    const store = usePromptCommandsStore();
    store.commands = [
      {
        name: 'reflect',
        description: 'Reflect on session',
        content: 'Reflect on the conversation. Input: {{input}}',
        source: '/fake/reflect.md',
        mode: 'all',
      },
    ];

    const { tryDispatchPromptCommand } = usePromptCommandDispatch();
    const result = tryDispatchPromptCommand('/reflect');

    expect(result.handled).toBe(true);
    expect(result.prompt).toContain('Input: ');
  });

  it('getAvailableCommands returns loaded commands', () => {
    const store = usePromptCommandsStore();
    store.commands = [
      { name: 'alpha', description: 'First', content: '', source: '', mode: 'all' as const },
      { name: 'beta', description: 'Second', content: '', source: '', mode: 'all' as const },
    ];

    const { getAvailableCommands } = usePromptCommandDispatch();
    const cmds = getAvailableCommands();

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toEqual({ name: 'alpha', description: 'First' });
    expect(cmds[1]).toEqual({ name: 'beta', description: 'Second' });
  });
});
