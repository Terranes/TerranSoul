import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import {
  configureBrowserLanAdapters,
  resetBrowserLanAdapters,
  useBrowserLanStore,
} from './browser-lan';
import { BROWSER_LAN_HOST_STORAGE_KEY } from '../utils/browser-lan';
import type { RemoteHost } from '../transport';

describe('browser LAN store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    resetBrowserLanAdapters();
    localStorage.removeItem(BROWSER_LAN_HOST_STORAGE_KEY);
  });

  afterEach(() => {
    resetBrowserLanAdapters();
    localStorage.removeItem(BROWSER_LAN_HOST_STORAGE_KEY);
  });

  it('blocks plaintext private LAN probes from an HTTPS page before network calls', async () => {
    const createRemoteHost = vi.fn();
    configureBrowserLanAdapters({
      createRemoteHost,
      pageProtocol: () => 'https:',
    });

    const store = useBrowserLanStore();
    store.hostInput = '192.168.1.42';
    store.portInput = '7422';

    await expect(store.probeAndSave()).resolves.toBeNull();
    expect(createRemoteHost).not.toHaveBeenCalled();
    expect(store.status).toBe('blocked');
    expect(store.error).toContain('HTTPS page cannot call a plaintext private LAN host');
  });

  it('saves a host after a successful gRPC-Web health probe', async () => {
    configureBrowserLanAdapters({
      createRemoteHost: () => fakeRemoteHost(),
      now: () => 4444,
      pageProtocol: () => 'http:',
    });

    const store = useBrowserLanStore();
    store.hostInput = '192.168.1.42';
    store.portInput = '7422';

    const saved = await store.probeAndSave();

    expect(saved).toMatchObject({ baseUrl: 'http://192.168.1.42:7422', savedAt: 4444 });
    expect(store.status).toBe('connected');
    expect(store.remoteSummary).toContain('local_ollama');
    expect(JSON.parse(localStorage.getItem(BROWSER_LAN_HOST_STORAGE_KEY) ?? '{}')).toMatchObject({
      host: '192.168.1.42',
      port: 7422,
    });
  });
});

function fakeRemoteHost(): RemoteHost {
  return {
    kind: 'grpc-web',
    getSystemStatus: async () => ({
      totalMemoryBytes: 0,
      usedMemoryBytes: 0,
      cpuUsagePct: 0,
      brainProvider: 'local_ollama',
      brainModel: 'gemma3:4b',
      memoryEntryCount: 12,
    }),
    listVsCodeWorkspaces: async () => [],
    getCopilotSessionStatus: async () => ({
      found: false,
      workspaceFolder: '',
      sessionId: '',
      model: '',
      lastUserTurnTs: '',
      lastUserPreview: '',
      lastAssistantTurnTs: '',
      lastAssistantPreview: '',
      toolInvocationCount: 0,
      eventCount: 0,
    }),
    listWorkflowRuns: async () => [],
    getWorkflowProgress: async () => ({
      workflowId: '',
      name: '',
      status: '',
      startedAtUnixMs: 0,
      lastEventAtUnixMs: 0,
      eventCount: 0,
      summary: '',
    }),
    continueWorkflow: async () => ({ accepted: false, message: '' }),
    sendChatMessage: async () => '',
    streamChatMessage: async function* () {},
    listPairedDevices: async () => [],
    brainHealth: async () => ({
      version: 'test',
      brainProvider: 'local_ollama',
      brainModel: 'gemma3:4b',
      ragQualityPct: 80,
      memoryTotal: 12,
    }),
    searchMemories: async () => [],
    streamSearchMemories: async function* () {},
  };
}
