import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import {
  configureMobileNotificationAdapters,
  createMobileNotificationTracker,
  resetMobileNotificationAdapters,
  type MobileNotificationPayload,
  useMobileNotificationsStore,
} from './mobile-notifications';
import { useSettingsStore } from './settings';
import type { RemoteHost } from '../transport';

describe('mobile notification tracker', () => {
  it('notifies when an observed workflow finishes after the threshold', () => {
    const tracker = createMobileNotificationTracker();
    expect(tracker.observeWorkflows([{ workflowId: 'wf-1', name: 'Copilot loop', status: 'running', startedAtUnixMs: 1, lastEventAtUnixMs: 1, eventCount: 1 }], 32_000, 30_000)).toEqual([]);

    const notifications = tracker.observeWorkflows([
      { workflowId: 'wf-1', name: 'Copilot loop', status: 'completed', startedAtUnixMs: 1, lastEventAtUnixMs: 40_000, eventCount: 4 },
    ], 40_000, 30_000);

    expect(notifications).toHaveLength(1);
    expect(notifications[0]).toMatchObject({
      kind: 'workflow',
      title: 'TerranSoul workflow finished',
      body: 'Copilot loop completed after 39s.',
    });
    expect(tracker.observeWorkflows([
      { workflowId: 'wf-1', name: 'Copilot loop', status: 'completed', startedAtUnixMs: 1, lastEventAtUnixMs: 41_000, eventCount: 5 },
    ], 41_000, 30_000)).toEqual([]);
  });

  it('does not notify short workflow completions', () => {
    const tracker = createMobileNotificationTracker();
    tracker.observeWorkflows([{ workflowId: 'wf-2', name: 'Quick run', status: 'running', startedAtUnixMs: 10, lastEventAtUnixMs: 10, eventCount: 1 }], 12_000, 30_000);

    expect(tracker.observeWorkflows([
      { workflowId: 'wf-2', name: 'Quick run', status: 'failed', startedAtUnixMs: 10, lastEventAtUnixMs: 20_000, eventCount: 2 },
    ], 20_000, 30_000)).toEqual([]);
  });

  it('notifies when an observed task completes after the threshold', () => {
    const tracker = createMobileNotificationTracker();
    tracker.observeTask({ id: 'task-1', status: 'running', description: 'Ingest docs' }, 0, 30_000);

    const notifications = tracker.observeTask({ id: 'task-1', status: 'completed', description: 'Ingest docs' }, 31_000, 30_000);

    expect(notifications).toHaveLength(1);
    expect(notifications[0]).toMatchObject({
      kind: 'task',
      title: 'TerranSoul task finished',
      body: 'Ingest docs completed after 31s.',
    });
  });

  it('notifies once when a Copilot session crosses the threshold', () => {
    const tracker = createMobileNotificationTracker();
    const status = {
      found: true,
      workspaceFolder: 'D:/Git/TerranSoul',
      sessionId: 'session-1',
      model: 'gpt-5-mini',
      lastUserTurnTs: '10:00',
      lastUserPreview: 'Continue next chunks',
      lastAssistantTurnTs: '10:01',
      lastAssistantPreview: 'Running focused tests',
      toolInvocationCount: 1,
      eventCount: 4,
    };

    expect(tracker.observeCopilot(status, 0, 30_000)).toEqual([]);
    const notifications = tracker.observeCopilot(status, 31_000, 30_000);

    expect(notifications).toHaveLength(1);
    expect(notifications[0].body).toContain('Copilot has been active in TerranSoul for 31s.');
    expect(tracker.observeCopilot(status, 40_000, 30_000)).toEqual([]);
  });
});

describe('mobile notifications store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    resetMobileNotificationAdapters();
  });

  it('polls the paired remote host and sends workflow notifications', async () => {
    const sent: MobileNotificationPayload[] = [];
    let now = 32_000;
    let workflowStatus = 'running';
    const listWorkflowRuns = vi.fn(async () => [{
      workflowId: 'wf-remote',
      name: 'Copilot loop',
      status: workflowStatus,
      startedAtUnixMs: 1,
      lastEventAtUnixMs: now,
      eventCount: 2,
    }]);

    configureMobileNotificationAdapters({
      now: () => now,
      remoteHost: () => fakeRemoteHost({ listWorkflowRuns }),
      notify: async (payload) => {
        sent.push(payload);
      },
      listenTaskProgress: async () => () => {},
    });
    const settings = useSettingsStore();
    settings.settings.mobile_notifications_enabled = true;
    settings.settings.mobile_notification_threshold_ms = 30_000;

    const store = useMobileNotificationsStore();
    await store.pollOnce();

    workflowStatus = 'completed';
    now = 40_000;
    await store.pollOnce();

    expect(listWorkflowRuns).toHaveBeenCalledWith(true);
    expect(sent).toHaveLength(1);
    expect(sent[0]).toMatchObject({ kind: 'workflow', body: 'Copilot loop completed after 39s.' });
    expect(store.connected).toBe(true);
  });
});

function fakeRemoteHost(overrides: Partial<RemoteHost> = {}): RemoteHost {
  return {
    kind: 'grpc-web',
    getSystemStatus: async () => ({
      totalMemoryBytes: 0,
      usedMemoryBytes: 0,
      cpuUsagePct: 0,
      brainProvider: 'stub',
      brainModel: 'stub',
      memoryEntryCount: 0,
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
      workflowId: 'wf',
      name: 'wf',
      status: 'completed',
      startedAtUnixMs: 0,
      lastEventAtUnixMs: 0,
      eventCount: 0,
      summary: '',
    }),
    continueWorkflow: async () => ({ accepted: false, message: '' }),
    sendChatMessage: async () => 'reply',
    streamChatMessage: async function* () {
      yield { text: 'reply', done: false };
      yield { text: '', done: true };
    },
    listPairedDevices: async () => [],
    brainHealth: async () => ({
      version: 'test',
      brainProvider: 'stub',
      brainModel: null,
      ragQualityPct: 0,
      memoryTotal: 0,
    }),
    searchMemories: async () => [],
    streamSearchMemories: async function* () {},
    ...overrides,
  };
}
