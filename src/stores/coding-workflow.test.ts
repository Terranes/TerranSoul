import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';

const { mockInvoke } = vi.hoisted(() => ({ mockInvoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }));

import {
  useCodingWorkflowStore,
  defaultCodingWorkflowConfig,
} from './coding-workflow';

describe('coding-workflow store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('exposes a working set of defaults matching the Rust struct', () => {
    const store = useCodingWorkflowStore();
    expect(store.config.include_dirs).toEqual(['rules', 'instructions', 'docs']);
    expect(store.config.include_files).toEqual(['README.md', 'AGENTS.md']);
    expect(store.config.exclude_paths).toEqual([]);
    expect(store.config.max_file_chars).toBe(4_000);
    expect(store.config.max_total_chars).toBe(30_000);
    expect(store.isDirty).toBe(false);
  });

  it('marks the form as dirty after editing and clean after save', async () => {
    const cfg = defaultCodingWorkflowConfig();
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_coding_workflow_config') return Promise.resolve(cfg);
      if (cmd === 'set_coding_workflow_config')
        return Promise.resolve({ ...cfg, max_file_chars: 6_000 });
      return Promise.reject(new Error(`unexpected ${cmd}`));
    });

    const store = useCodingWorkflowStore();
    await store.load();
    expect(store.isDirty).toBe(false);

    store.config.max_file_chars = 6_000;
    expect(store.isDirty).toBe(true);

    await store.save();
    expect(store.isDirty).toBe(false);
    expect(store.config.max_file_chars).toBe(6_000);
  });

  it('addEntry rejects empty + duplicates and trims input', () => {
    const store = useCodingWorkflowStore();
    expect(store.addEntry('include_dirs', '   ')).toBe(false);
    expect(store.addEntry('include_dirs', 'rules')).toBe(false); // duplicate
    expect(store.addEntry('include_dirs', '  src/agents  ')).toBe(true);
    expect(store.config.include_dirs).toContain('src/agents');
  });

  it('removeEntry deletes only the matching value', () => {
    const store = useCodingWorkflowStore();
    store.removeEntry('include_files', 'README.md');
    expect(store.config.include_files).toEqual(['AGENTS.md']);
    expect(store.isDirty).toBe(true);
  });

  it('discardChanges restores the pristine snapshot', () => {
    const store = useCodingWorkflowStore();
    store.config.max_total_chars = 12_345;
    expect(store.isDirty).toBe(true);
    store.discardChanges();
    expect(store.isDirty).toBe(false);
    expect(store.config.max_total_chars).toBe(30_000);
  });

  it('reset clears config via the backend and resets pristine', async () => {
    mockInvoke.mockResolvedValueOnce(defaultCodingWorkflowConfig());
    const store = useCodingWorkflowStore();
    store.config.exclude_paths = ['something.md'];
    expect(store.isDirty).toBe(true);
    await store.reset();
    expect(store.isDirty).toBe(false);
    expect(store.config.exclude_paths).toEqual([]);
    expect(mockInvoke).toHaveBeenCalledWith('reset_coding_workflow_config');
  });

  it('refreshPreview surfaces preview totals from the backend', async () => {
    mockInvoke.mockResolvedValueOnce({
      documents: [
        { label: 'rules/a.md', char_count: 100 },
        { label: 'README.md', char_count: 50 },
      ],
      total_chars: 150,
      file_count: 2,
      repo_root: '/repo',
    });
    const store = useCodingWorkflowStore();
    await store.refreshPreview();
    expect(store.preview?.file_count).toBe(2);
    expect(store.preview?.total_chars).toBe(150);
  });

  it('surfaces errors via lastError', async () => {
    mockInvoke.mockRejectedValueOnce('disk full');
    const store = useCodingWorkflowStore();
    await expect(store.load()).rejects.toBeDefined();
    expect(store.lastError).toContain('disk full');
  });
});
