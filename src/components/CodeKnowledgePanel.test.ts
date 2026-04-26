import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { invoke } from '@tauri-apps/api/core';
import CodeKnowledgePanel from './CodeKnowledgePanel.vue';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const invokeMock = invoke as unknown as ReturnType<typeof vi.fn>;

interface MirrorRow {
  edge_source: string;
  scope: string;
  edge_count: number;
  last_synced_at: number;
}

/**
 * Helper: build an `invoke` mock that routes per-command. Simulates the
 * Tauri command surface for `gitnexus_list_mirrors`, `gitnexus_sync`,
 * `gitnexus_unmirror`, and `gitnexus_impact`.
 */
function setupInvoke(mirrors: MirrorRow[] = []) {
  let listed: MirrorRow[] = [...mirrors];
  invokeMock.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    switch (cmd) {
      case 'gitnexus_list_mirrors':
        return [...listed];
      case 'gitnexus_sync': {
        const scope = String(args?.repoLabel ?? '');
        const row: MirrorRow = {
          edge_source: `gitnexus:${scope}`,
          scope,
          edge_count: 5,
          last_synced_at: Date.now(),
        };
        listed = [row, ...listed.filter((r) => r.scope !== scope)];
        return {
          edge_source: row.edge_source,
          nodes_inserted: 3,
          nodes_reused: 0,
          edges_inserted: 5,
          edges_skipped: 1,
        };
      }
      case 'gitnexus_unmirror': {
        const scope = String(args?.repoLabel ?? '');
        const before = listed.length;
        listed = listed.filter((r) => r.scope !== scope);
        return before - listed.length;
      }
      case 'gitnexus_impact':
        return { symbol: args?.symbol, dependents: ['callerA', 'callerB', 'callerC'] };
      default:
        throw new Error(`unexpected invoke: ${cmd}`);
    }
  });
}

describe('CodeKnowledgePanel', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('renders empty state when no mirrors exist', async () => {
    setupInvoke([]);
    const w = mount(CodeKnowledgePanel);
    await flushPromises();
    expect(w.text()).toContain('No repos mirrored yet');
    expect(invokeMock).toHaveBeenCalledWith('gitnexus_list_mirrors');
  });

  it('disables the sync button when scope is empty', async () => {
    setupInvoke();
    const w = mount(CodeKnowledgePanel);
    await flushPromises();
    const syncBtn = w.findAll('button').find((b) => b.text().includes('Sync KG'))!;
    expect(syncBtn.attributes('disabled')).toBeDefined();
  });

  it('lists mirrors loaded on mount in most-recent-first order', async () => {
    setupInvoke([
      { edge_source: 'gitnexus:repo:a/b@1', scope: 'repo:a/b@1', edge_count: 7, last_synced_at: 1700000000000 },
      { edge_source: 'gitnexus:repo:c/d@2', scope: 'repo:c/d@2', edge_count: 2, last_synced_at: 1700000005000 },
    ]);
    const w = mount(CodeKnowledgePanel);
    await flushPromises();
    const rows = w.findAll('.ck-mirror-row');
    expect(rows).toHaveLength(2);
    expect(rows[0].text()).toContain('repo:a/b@1');
    expect(rows[0].text()).toContain('7 edges');
    expect(rows[1].text()).toContain('2 edges');
    // Singular vs. plural rendering.
    expect(rows[1].text()).toContain('2 edges');
  });

  it('runs a sync and refreshes the mirror list with the report', async () => {
    setupInvoke([]);
    const w = mount(CodeKnowledgePanel);
    await flushPromises();

    await w.find('input.ck-input').setValue('repo:foo/bar@abc');
    const syncBtn = w.findAll('button').find((b) => b.text().includes('Sync KG'))!;
    await syncBtn.trigger('click');
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith('gitnexus_sync', {
      repoLabel: 'repo:foo/bar@abc',
    });
    expect(w.text()).toContain('Mirrored');
    expect(w.text()).toContain('3 new nodes');
    expect(w.text()).toContain('5 edges');
    // Refresh fired again.
    expect(w.findAll('.ck-mirror-row').length).toBe(1);
  });

  it('removes a row when Unmirror is clicked', async () => {
    setupInvoke([
      { edge_source: 'gitnexus:repo:a/b@1', scope: 'repo:a/b@1', edge_count: 4, last_synced_at: 1700000000000 },
    ]);
    const w = mount(CodeKnowledgePanel);
    await flushPromises();
    expect(w.findAll('.ck-mirror-row').length).toBe(1);

    const unmirrorBtn = w.findAll('button').find((b) => b.text().includes('Unmirror'))!;
    await unmirrorBtn.trigger('click');
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith('gitnexus_unmirror', {
      repoLabel: 'repo:a/b@1',
    });
    expect(w.text()).toContain('No repos mirrored yet');
  });

  it('surfaces a one-line impact summary when probe succeeds', async () => {
    setupInvoke();
    const w = mount(CodeKnowledgePanel);
    await flushPromises();

    const inputs = w.findAll('input.ck-input');
    // The blast-radius input is the second .ck-input.
    await inputs[1].setValue('module::path::Symbol');
    const probeBtn = w.findAll('button').find((b) => b.text().includes('Probe impact'))!;
    await probeBtn.trigger('click');
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith('gitnexus_impact', {
      symbol: 'module::path::Symbol',
    });
    expect(w.text()).toContain('3 dependents');
    expect(w.text()).toContain('module::path::Symbol');
  });

  it('shows an error banner when sync fails', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'gitnexus_list_mirrors') return [];
      if (cmd === 'gitnexus_sync') throw 'capability not granted';
      throw new Error('unexpected');
    });
    const w = mount(CodeKnowledgePanel);
    await flushPromises();
    await w.find('input.ck-input').setValue('repo:x/y@z');
    const syncBtn = w.findAll('button').find((b) => b.text().includes('Sync KG'))!;
    await syncBtn.trigger('click');
    await flushPromises();
    const alert = w.find('[role="alert"]');
    expect(alert.exists()).toBe(true);
    expect(alert.text()).toContain('capability not granted');
  });

  it('summariseImpact handles known and unknown response shapes', () => {
    setupInvoke();
    const w = mount(CodeKnowledgePanel);
    const cmp = w.vm as unknown as {
      summariseImpact: (raw: unknown) => string;
      formatTimestamp: (ms: number) => string;
    };
    expect(cmp.summariseImpact({ symbol: 'foo', dependents: ['a', 'b'] }))
      .toBe('2 dependents of foo');
    expect(cmp.summariseImpact({ items: [1] })).toContain('1 dependent ');
    expect(cmp.summariseImpact({ count: 9 })).toBe('9 dependents');
    // Unknown shape — falls back to JSON snippet, not an exception.
    const fallback = cmp.summariseImpact({ noisy: true });
    expect(fallback).toContain('noisy');
  });

  it('formatTimestamp returns a placeholder for falsy timestamps', () => {
    setupInvoke();
    const w = mount(CodeKnowledgePanel);
    const cmp = w.vm as unknown as { formatTimestamp: (ms: number) => string };
    expect(cmp.formatTimestamp(0)).toBe('—');
    expect(cmp.formatTimestamp(-1)).toBe('—');
    expect(cmp.formatTimestamp(1700000000000)).not.toBe('—');
  });
});
