import { describe, it, expect, beforeEach, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useTaskStore, TASK_LOG_MAX_LINES } from './tasks';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

describe('useTaskStore — debug log ring buffer (BRAIN-REPO-RAG-2b)', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('appends log lines and exposes them per task', () => {
    const store = useTaskStore();
    store.appendTaskLog('task-a', 'walk (1/3): foo.rs');
    store.appendTaskLog('task-a', 'skip[binary]: bar.bin');
    store.appendTaskLog('task-b', 'walk (1/1): other.rs');

    expect(store.taskLogs.get('task-a')).toEqual([
      'walk (1/3): foo.rs',
      'skip[binary]: bar.bin',
    ]);
    expect(store.taskLogs.get('task-b')).toEqual(['walk (1/1): other.rs']);
  });

  it('caps each task ring buffer at TASK_LOG_MAX_LINES', () => {
    const store = useTaskStore();
    const total = TASK_LOG_MAX_LINES + 25;
    for (let i = 0; i < total; i += 1) {
      store.appendTaskLog('task-x', `line ${i}`);
    }
    const buf = store.taskLogs.get('task-x') ?? [];
    expect(buf.length).toBe(TASK_LOG_MAX_LINES);
    // Oldest entries are dropped.
    expect(buf[0]).toBe(`line ${total - TASK_LOG_MAX_LINES}`);
    expect(buf[buf.length - 1]).toBe(`line ${total - 1}`);
  });

  it('clearTaskLog removes a task buffer', () => {
    const store = useTaskStore();
    store.appendTaskLog('task-c', 'one');
    expect(store.taskLogs.get('task-c')?.length).toBe(1);
    store.clearTaskLog('task-c');
    expect(store.taskLogs.get('task-c')).toBeUndefined();
  });
});
