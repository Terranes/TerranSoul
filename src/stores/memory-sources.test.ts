import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import {
  useMemorySourcesStore,
  SELF_SOURCE_ID,
  ALL_SOURCES_ID,
  type MemorySource,
} from './memory-sources';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

function selfRow(): MemorySource {
  return {
    id: 'self',
    kind: 'self',
    label: 'TerranSoul',
    repo_url: null,
    repo_ref: null,
    created_at: 1_700_000_000_000,
    last_synced_at: null,
  };
}

function repoRow(id: string, label: string): MemorySource {
  return {
    id,
    kind: 'repo',
    label,
    repo_url: `https://github.com/example/${label}`,
    repo_ref: 'main',
    created_at: 1_700_000_000_001,
    last_synced_at: null,
  };
}

describe('memory-sources store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockInvoke.mockReset();
    localStorage.clear();
  });

  it('defaults to the self source when no localStorage value exists', () => {
    const store = useMemorySourcesStore();
    expect(store.activeId).toBe(SELF_SOURCE_ID);
    expect(store.isAllView).toBe(false);
  });

  it('fetchAll populates sources and falls back to self when persisted id vanishes', async () => {
    localStorage.setItem('terransoul.memory-sources.active.v1', 'repo:ghost');
    mockInvoke.mockResolvedValueOnce([selfRow(), repoRow('repo:alpha', 'Alpha')]);
    const store = useMemorySourcesStore();
    expect(store.activeId).toBe('repo:ghost');
    await store.fetchAll();
    expect(store.sources).toHaveLength(2);
    expect(store.activeId).toBe(SELF_SOURCE_ID);
    expect(store.activeSource?.id).toBe(SELF_SOURCE_ID);
  });

  it('setActive persists across reloads and supports the all-sources sentinel', () => {
    const store = useMemorySourcesStore();
    store.setActive(ALL_SOURCES_ID);
    expect(store.isAllView).toBe(true);
    expect(store.activeSource).toBeNull();
    expect(localStorage.getItem('terransoul.memory-sources.active.v1')).toBe(ALL_SOURCES_ID);
  });

  it('createSource appends, sorts, activates, and rejects on backend error', async () => {
    mockInvoke.mockResolvedValueOnce([selfRow()]);
    const store = useMemorySourcesStore();
    await store.fetchAll();

    const created = repoRow('repo:zeta', 'Zeta');
    mockInvoke.mockResolvedValueOnce(created);
    const ok = await store.createSource({
      id: 'repo:zeta',
      kind: 'repo',
      label: 'Zeta',
      repo_url: 'https://github.com/example/Zeta',
      repo_ref: 'main',
    });
    expect(ok?.id).toBe('repo:zeta');
    expect(store.activeId).toBe('repo:zeta');
    expect(store.sources.map((s) => s.id)).toEqual([SELF_SOURCE_ID, 'repo:zeta']);
    expect(store.repoSources).toHaveLength(1);

    mockInvoke.mockRejectedValueOnce(new Error('duplicate id'));
    const fail = await store.createSource({
      id: 'repo:zeta',
      kind: 'repo',
      label: 'Zeta dup',
    });
    expect(fail).toBeNull();
    expect(store.error).toContain('duplicate id');
  });

  it('deleteSource removes the row, resets active when deleted, and refuses to delete self', async () => {
    mockInvoke.mockResolvedValueOnce([selfRow(), repoRow('repo:beta', 'Beta')]);
    const store = useMemorySourcesStore();
    await store.fetchAll();
    store.setActive('repo:beta');

    mockInvoke.mockResolvedValueOnce(true);
    const removed = await store.deleteSource('repo:beta');
    expect(removed).toBe(true);
    expect(store.sources).toHaveLength(1);
    expect(store.activeId).toBe(SELF_SOURCE_ID);

    const refused = await store.deleteSource(SELF_SOURCE_ID);
    expect(refused).toBe(false);
    expect(store.error).toMatch(/built-in/i);
  });

  // ── BRAIN-REPO-RAG-1e: GitHub OAuth wrappers ──────────────────────

  it('startGitHubOAuth invokes repo_oauth_github_start and caches the device code', async () => {
    const store = useMemorySourcesStore();
    mockInvoke.mockResolvedValueOnce({
      device_code: 'dev-1',
      user_code: 'WXYZ-1234',
      verification_uri: 'https://github.com/login/device',
      expires_in: 900,
      interval: 5,
    });
    const resp = await store.startGitHubOAuth('repo');
    expect(mockInvoke).toHaveBeenCalledWith('repo_oauth_github_start', { scopes: 'repo' });
    expect(resp.user_code).toBe('WXYZ-1234');
    expect(store.oauthDeviceCode?.device_code).toBe('dev-1');
    expect(store.oauthPollResult).toBeNull();
  });

  it('pollGitHubOAuth refreshes status on success and stores poll result', async () => {
    const store = useMemorySourcesStore();
    store.oauthDeviceCode = {
      device_code: 'dev-1',
      user_code: 'WXYZ-1234',
      verification_uri: 'https://github.com/login/device',
      expires_in: 900,
      interval: 5,
    };
    // First poll: success — triggers a status refresh as a 2nd invoke.
    mockInvoke.mockResolvedValueOnce({
      status: 'success',
      access_token: 'ghs_secret',
      token_type: 'bearer',
      scope: 'repo',
    });
    mockInvoke.mockResolvedValueOnce({
      linked: true,
      token_type: 'bearer',
      scope: 'repo',
      created_at: 1_700_000_000,
      expires_at: null,
      expired: false,
    });
    const result = await store.pollGitHubOAuth();
    expect(mockInvoke).toHaveBeenNthCalledWith(1, 'repo_oauth_github_poll', {
      deviceCode: 'dev-1',
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, 'repo_oauth_github_status');
    expect(result.status).toBe('success');
    expect(store.oauthDeviceCode).toBeNull();
    expect(store.oauthStatus?.linked).toBe(true);
  });

  it('pollGitHubOAuth short-circuits with an error result when no device code is active', async () => {
    const store = useMemorySourcesStore();
    const result = await store.pollGitHubOAuth();
    expect(result.status).toBe('error');
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('clearGitHubOAuth wipes local state and invokes the backend', async () => {
    const store = useMemorySourcesStore();
    store.oauthStatus = {
      linked: true,
      token_type: 'bearer',
      scope: 'repo',
      created_at: 1,
      expires_at: null,
      expired: false,
    };
    mockInvoke.mockResolvedValueOnce(null);
    await store.clearGitHubOAuth();
    expect(mockInvoke).toHaveBeenCalledWith('repo_oauth_github_clear');
    expect(store.oauthStatus).toBeNull();
  });
});
