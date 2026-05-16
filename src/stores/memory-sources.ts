/**
 * Pinia store for the memory sources registry (BRAIN-REPO-RAG-1a).
 *
 * Mirrors the backend `memory_sources` SQLite table created in schema v22
 * and the Tauri commands defined in `commands/memory_sources.rs`:
 *
 * - `list_memory_sources` → `MemorySource[]`
 * - `get_memory_source(id)` → `MemorySource | null`
 * - `create_memory_source(id, kind, label, repo_url?, repo_ref?)` → `MemorySource`
 * - `delete_memory_source(id)` → `boolean`
 *
 * The active source id is persisted in localStorage so the user's last
 * selection survives reloads (key: `terransoul.memory-sources.active.v1`).
 *
 * NOTE — at 1a no backend ingest exists yet. Add-source rows only
 * persist metadata; chunk 1b is the per-repo clone + chunk + embed
 * pipeline. UI consumers should fall back to "this source has not been
 * ingested yet" empty-state when the active source has `kind === 'repo'`.
 */
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

/** Discriminator matching the SQL CHECK constraint + serde enum. */
export type MemorySourceKind = 'self' | 'repo' | 'topic';

/** One row in `memory_sources`. Field names match the Rust struct. */
export interface MemorySource {
  id: string;
  kind: MemorySourceKind;
  label: string;
  repo_url: string | null;
  repo_ref: string | null;
  created_at: number;
  last_synced_at: number | null;
}

/** Canonical id of the built-in TerranSoul brain — seeded by v22. */
export const SELF_SOURCE_ID = 'self';

/** Sentinel id for the "all sources" cross-DB view (BRAIN-REPO-RAG-1c). */
export const ALL_SOURCES_ID = '__all__';

const ACTIVE_STORAGE_KEY = 'terransoul.memory-sources.active.v1';

function readActiveFromStorage(): string {
  try {
    const raw = localStorage.getItem(ACTIVE_STORAGE_KEY);
    if (typeof raw === 'string' && raw.length > 0) return raw;
  } catch {
    /* localStorage unavailable — fall through */
  }
  return SELF_SOURCE_ID;
}

function persistActive(id: string): void {
  try {
    localStorage.setItem(ACTIVE_STORAGE_KEY, id);
  } catch {
    /* non-fatal — same-session ref still holds the value */
  }
}

export const useMemorySourcesStore = defineStore('memory-sources', () => {
  const sources = ref<MemorySource[]>([]);
  const activeId = ref<string>(readActiveFromStorage());
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  /** The currently selected source row, or `null` for the "all" view. */
  const activeSource = computed<MemorySource | null>(() => {
    if (activeId.value === ALL_SOURCES_ID) return null;
    return sources.value.find((s) => s.id === activeId.value) ?? null;
  });

  /** True when the user is viewing the cross-source aggregate. */
  const isAllView = computed(() => activeId.value === ALL_SOURCES_ID);

  /** Convenience filter used by the source-picker UI. */
  const repoSources = computed(() => sources.value.filter((s) => s.kind === 'repo'));

  async function fetchAll(): Promise<void> {
    isLoading.value = true;
    error.value = null;
    try {
      const result = await invoke<MemorySource[]>('list_memory_sources');
      sources.value = Array.isArray(result) ? result : [];
      // If the persisted active id no longer exists, fall back to self.
      if (
        activeId.value !== ALL_SOURCES_ID &&
        !sources.value.some((s) => s.id === activeId.value)
      ) {
        setActive(SELF_SOURCE_ID);
      }
    } catch (e) {
      error.value = String(e);
      sources.value = [];
    } finally {
      isLoading.value = false;
    }
  }

  function setActive(id: string): void {
    activeId.value = id;
    persistActive(id);
  }

  async function createSource(input: {
    id: string;
    kind: Exclude<MemorySourceKind, 'self'>;
    label: string;
    repo_url?: string | null;
    repo_ref?: string | null;
  }): Promise<MemorySource | null> {
    error.value = null;
    try {
      const created = await invoke<MemorySource>('create_memory_source', {
        id: input.id,
        kind: input.kind,
        label: input.label,
        repoUrl: input.repo_url ?? null,
        repoRef: input.repo_ref ?? null,
      });
      if (!created || typeof created.id !== 'string') {
        throw new Error('invalid create_memory_source response');
      }
      sources.value.push(created);
      sources.value.sort(sortSelfFirstThenLabel);
      setActive(created.id);
      return created;
    } catch (e) {
      error.value = String(e);
      return null;
    }
  }

  async function deleteSource(id: string): Promise<boolean> {
    if (id === SELF_SOURCE_ID) {
      error.value = "the built-in 'self' source cannot be deleted";
      return false;
    }
    error.value = null;
    try {
      const ok = await invoke<boolean>('delete_memory_source', { id });
      if (ok) {
        sources.value = sources.value.filter((s) => s.id !== id);
        if (activeId.value === id) setActive(SELF_SOURCE_ID);
      }
      return Boolean(ok);
    } catch (e) {
      error.value = String(e);
      return false;
    }
  }

  function sortSelfFirstThenLabel(a: MemorySource, b: MemorySource): number {
    if (a.id === SELF_SOURCE_ID) return -1;
    if (b.id === SELF_SOURCE_ID) return 1;
    return a.label.toLowerCase().localeCompare(b.label.toLowerCase());
  }

  // ── BRAIN-REPO-RAG-1e: GitHub OAuth device flow for private repos ──

  /** Mirrors `coding::DeviceCodeResponse` from the backend. */
  interface RepoOAuthDeviceCode {
    device_code: string;
    user_code: string;
    verification_uri: string;
    expires_in: number;
    interval: number;
  }

  /** Mirrors `coding::DevicePollResult`. */
  type RepoOAuthPollResult =
    | { status: 'pending' }
    | { status: 'success'; access_token: string; token_type: string; scope: string }
    | { status: 'expired' }
    | { status: 'denied' }
    | { status: 'error'; message: string };

  /** Mirrors `commands::repos::RepoOAuthStatus` — never includes the token. */
  interface RepoOAuthStatus {
    linked: boolean;
    token_type: string;
    scope: string;
    created_at: number;
    expires_at: number | null;
    expired: boolean;
  }

  const oauthDeviceCode = ref<RepoOAuthDeviceCode | null>(null);
  const oauthPollResult = ref<RepoOAuthPollResult | null>(null);
  const oauthStatus = ref<RepoOAuthStatus | null>(null);

  async function startGitHubOAuth(scopes = 'repo'): Promise<RepoOAuthDeviceCode> {
    error.value = null;
    oauthPollResult.value = null;
    try {
      const resp = await invoke<RepoOAuthDeviceCode>('repo_oauth_github_start', { scopes });
      oauthDeviceCode.value = resp;
      return resp;
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  async function pollGitHubOAuth(deviceCode?: string): Promise<RepoOAuthPollResult> {
    const code = deviceCode ?? oauthDeviceCode.value?.device_code ?? '';
    if (!code) {
      const result: RepoOAuthPollResult = { status: 'error', message: 'no active device code' };
      oauthPollResult.value = result;
      return result;
    }
    try {
      const result = await invoke<RepoOAuthPollResult>('repo_oauth_github_poll', {
        deviceCode: code,
      });
      oauthPollResult.value = result;
      if (result.status === 'success') {
        oauthDeviceCode.value = null;
        await fetchGitHubOAuthStatus();
      }
      return result;
    } catch (e) {
      error.value = String(e);
      const result: RepoOAuthPollResult = { status: 'error', message: String(e) };
      oauthPollResult.value = result;
      return result;
    }
  }

  async function fetchGitHubOAuthStatus(): Promise<RepoOAuthStatus | null> {
    try {
      const status = await invoke<RepoOAuthStatus>('repo_oauth_github_status');
      oauthStatus.value = status;
      return status;
    } catch (e) {
      error.value = String(e);
      return null;
    }
  }

  async function clearGitHubOAuth(): Promise<void> {
    try {
      await invoke('repo_oauth_github_clear');
      oauthStatus.value = null;
      oauthDeviceCode.value = null;
      oauthPollResult.value = null;
    } catch (e) {
      error.value = String(e);
    }
  }

  return {
    sources,
    activeId,
    activeSource,
    isAllView,
    repoSources,
    isLoading,
    error,
    fetchAll,
    setActive,
    createSource,
    deleteSource,
    oauthDeviceCode,
    oauthPollResult,
    oauthStatus,
    startGitHubOAuth,
    pollGitHubOAuth,
    fetchGitHubOAuthStatus,
    clearGitHubOAuth,
  };
});
