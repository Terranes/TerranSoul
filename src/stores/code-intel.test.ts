import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useCodeIntelStore } from './code-intel';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => mockInvoke(...args) }));

describe('code-intel store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
  });

  it('starts with empty state', () => {
    const store = useCodeIntelStore();
    expect(store.repos).toEqual([]);
    expect(store.activeRepoId).toBeNull();
    expect(store.clusters).toEqual([]);
    expect(store.processes).toEqual([]);
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it('fetchRepos populates repos and sets active', async () => {
    const repos = [
      { id: 1, path: '/tmp/a', label: 'repo-a', indexed_at: '2026-05-06' },
      { id: 2, path: '/tmp/b', label: 'repo-b', indexed_at: '2026-05-05' },
    ];
    mockInvoke.mockResolvedValueOnce(repos);

    const store = useCodeIntelStore();
    await store.fetchRepos();

    expect(mockInvoke).toHaveBeenCalledWith('code_list_repos');
    expect(store.repos).toEqual(repos);
    expect(store.activeRepoId).toBe(1);
  });

  it('fetchRepos handles errors gracefully', async () => {
    mockInvoke.mockRejectedValueOnce('no index');

    const store = useCodeIntelStore();
    await store.fetchRepos();

    expect(store.error).toBe('no index');
    expect(store.repos).toEqual([]);
  });

  it('fetchClusters calls with active repo id', async () => {
    mockInvoke.mockResolvedValueOnce([{ id: 1, label: 'core', size: 10, symbol_ids: [] }]);

    const store = useCodeIntelStore();
    store.activeRepoId = 5;
    await store.fetchClusters();

    expect(mockInvoke).toHaveBeenCalledWith('code_list_clusters', { repoId: 5 });
    expect(store.clusters).toHaveLength(1);
  });

  it('fetchClusters skips when no active repo', async () => {
    const store = useCodeIntelStore();
    await store.fetchClusters();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('fetchProcesses calls with active repo id', async () => {
    mockInvoke.mockResolvedValueOnce([
      { entry_point: 'main', entry_file: 'src/main.rs', entry_line: 1, steps: [] },
    ]);

    const store = useCodeIntelStore();
    store.activeRepoId = 3;
    await store.fetchProcesses();

    expect(mockInvoke).toHaveBeenCalledWith('code_list_processes', { repoId: 3 });
    expect(store.processes).toHaveLength(1);
  });

  it('analyzeImpact invokes correct command', async () => {
    const result = { symbol: 'foo', total_affected: 5, by_depth: [] };
    mockInvoke.mockResolvedValueOnce(result);

    const store = useCodeIntelStore();
    store.activeRepoId = 1;
    await store.analyzeImpact('foo', 3);

    expect(mockInvoke).toHaveBeenCalledWith('code_analyze_impact', {
      symbol: 'foo',
      depth: 3,
      repoId: 1,
    });
    expect(store.impactResult).toEqual(result);
  });

  it('analyzeDiffImpact invokes correct command', async () => {
    const report = {
      diff_ref: 'HEAD~1',
      files_changed: 2,
      symbols_changed: 3,
      total_affected: 10,
      risk_summary: { critical: 1, high: 2, moderate: 0, low: 0 },
      impacts: [],
    };
    mockInvoke.mockResolvedValueOnce(report);

    const store = useCodeIntelStore();
    store.activeRepoId = 1;
    await store.analyzeDiffImpact('main..feature');

    expect(mockInvoke).toHaveBeenCalledWith('code_analyze_diff_impact', {
      diffRef: 'main..feature',
      repoId: 1,
    });
    expect(store.diffImpact).toEqual(report);
  });

  it('setActiveRepo resets dependent state', () => {
    const store = useCodeIntelStore();
    store.clusters = [{ id: 1, label: 'x', size: 1, symbol_ids: [] }];
    store.processes = [
      { entry_point: 'y', entry_file: 'a.rs', entry_line: 1, steps: [] },
    ];
    store.impactResult = { symbol: 'z', total_affected: 0, by_depth: [] };

    store.setActiveRepo(99);

    expect(store.activeRepoId).toBe(99);
    expect(store.clusters).toEqual([]);
    expect(store.processes).toEqual([]);
    expect(store.impactResult).toBeNull();
  });

  it('selectCluster toggles selection', () => {
    const store = useCodeIntelStore();
    store.selectCluster(5);
    expect(store.selectedClusterId).toBe(5);
    store.selectCluster(null);
    expect(store.selectedClusterId).toBeNull();
  });

  it('selectedCluster computed returns matching cluster', () => {
    const store = useCodeIntelStore();
    store.clusters = [
      { id: 1, label: 'a', size: 5, symbol_ids: [] },
      { id: 2, label: 'b', size: 3, symbol_ids: [] },
    ];
    store.selectedClusterId = 2;
    expect(store.selectedCluster?.label).toBe('b');
  });
});
