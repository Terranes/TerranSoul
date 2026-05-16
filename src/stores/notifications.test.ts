/**
 * Tests for the unified notifications + job-tracking store.
 *
 * Validates:
 *  - `trackJob` adds a job and pushes a started notification
 *  - `handleCliEvent` updates job state line-by-line
 *  - `handleWorkflowCompleted` flips status and pushes terminal notif
 *  - `unreadCount` / `activeJobs` / `recentJobs` derived state
 *  - `dispatchHermesJob` round-trips the Tauri contract
 *  - panel open marks all read
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async () => () => {}),
}));

vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn(async () => true),
  requestPermission: vi.fn(async () => 'granted'),
  sendNotification: vi.fn(),
}));

import { useNotificationsStore } from './notifications';

beforeEach(() => {
  setActivePinia(createPinia());
  mockInvoke.mockReset();
  // Simulate non-Tauri environment to avoid network/IPC side effects
  // in the tests that don't explicitly want it.
  // The store still functions for everything except `dispatchHermesJob`
  // and `cancelJob`, which we cover separately.
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

function withTauri(fn: () => Promise<void>) {
  return async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    try {
      await fn();
    } finally {
      delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
    }
  };
}

describe('useNotificationsStore', () => {
  it('trackJob adds an active job and pushes a started notification', () => {
    const store = useNotificationsStore();
    store.trackJob({
      workflowId: 'w1',
      agentId: 'hermes-staff',
      agentName: 'Hermes Staff',
      label: 'refactor auth flow',
    });
    expect(store.activeJobs).toHaveLength(1);
    expect(store.activeJobs[0]).toMatchObject({
      workflowId: 'w1',
      agentName: 'Hermes Staff',
      status: 'queued',
      label: 'refactor auth flow',
    });
    expect(store.notifications).toHaveLength(1);
    expect(store.notifications[0].kind).toBe('job-started');
    expect(store.unreadCount).toBe(1);
  });

  it('handleCliEvent flips queued → running and records last line', () => {
    const store = useNotificationsStore();
    store.trackJob({
      workflowId: 'w1',
      agentId: 'a',
      agentName: 'Staff',
      label: 'job',
    });
    store.handleCliEvent({
      workflow_id: 'w1',
      event: { type: 'started', pid: 1234 },
    });
    expect(store.jobs['w1'].status).toBe('running');
    store.handleCliEvent({
      workflow_id: 'w1',
      event: { type: 'line', stream: 'stdout', text: 'Hello, world!' },
    });
    expect(store.jobs['w1'].lastLine).toBe('Hello, world!');
    expect(store.jobs['w1'].lineCount).toBe(1);
  });

  it('handleCliEvent ignores unknown workflow ids', () => {
    const store = useNotificationsStore();
    store.handleCliEvent({
      workflow_id: 'unknown',
      event: { type: 'started', pid: 1 },
    });
    expect(Object.keys(store.jobs)).toHaveLength(0);
  });

  it('handleWorkflowCompleted flips to completed and pushes ✓ notification', () => {
    const store = useNotificationsStore();
    store.trackJob({
      workflowId: 'w1',
      agentId: 'a',
      agentName: 'Hermes Staff',
      label: 'finish task',
    });
    store.handleWorkflowCompleted({
      workflow_id: 'w1',
      status: 'completed',
      message: null,
    });
    expect(store.jobs['w1'].status).toBe('completed');
    expect(store.recentJobs).toHaveLength(1);
    expect(store.activeJobs).toHaveLength(0);
    const completedNotif = store.notifications.find(
      (n) => n.kind === 'job-completed',
    );
    expect(completedNotif).toBeDefined();
    expect(completedNotif?.title).toContain('Hermes Staff');
  });

  it('handleWorkflowCompleted records error message on failure', () => {
    const store = useNotificationsStore();
    store.trackJob({
      workflowId: 'w1',
      agentId: 'a',
      agentName: 'Staff',
      label: 'broken job',
    });
    store.handleWorkflowCompleted({
      workflow_id: 'w1',
      status: 'failed',
      message: 'CLI exited with status 2',
    });
    expect(store.jobs['w1'].status).toBe('failed');
    expect(store.jobs['w1'].errorMessage).toBe('CLI exited with status 2');
    const failedNotif = store.notifications.find(
      (n) => n.kind === 'job-failed',
    );
    expect(failedNotif?.body).toContain('CLI exited');
  });

  it('opening the panel marks notifications as read', () => {
    const store = useNotificationsStore();
    store.trackJob({
      workflowId: 'w1',
      agentId: 'a',
      agentName: 'Staff',
      label: 'x',
    });
    expect(store.unreadCount).toBe(1);
    store.openPanel();
    expect(store.unreadCount).toBe(0);
    expect(store.panelOpen).toBe(true);
  });

  it('dismiss removes a single notification, clearAll removes all', () => {
    const store = useNotificationsStore();
    store.pushNotification({ kind: 'info', title: 'A', body: '1' });
    store.pushNotification({ kind: 'info', title: 'B', body: '2' });
    expect(store.notifications).toHaveLength(2);
    const idA = store.notifications[1].id;
    store.dismiss(idA);
    expect(store.notifications).toHaveLength(1);
    store.clearAll();
    expect(store.notifications).toHaveLength(0);
  });

  it(
    'dispatchHermesJob invokes the Tauri command and tracks the result',
    withTauri(async () => {
      mockInvoke.mockResolvedValueOnce({
        workflow_id: 'wf-42',
        agent_id: 'hermes-staff',
        label: 'do the thing',
      });
      const store = useNotificationsStore();
      const job = await store.dispatchHermesJob({
        prompt: 'do the thing',
        working_folder: 'C:\\repo',
      });
      expect(mockInvoke).toHaveBeenCalledWith('dispatch_hermes_job', {
        request: { prompt: 'do the thing', working_folder: 'C:\\repo' },
      });
      expect(job.workflowId).toBe('wf-42');
      expect(store.activeJobs).toHaveLength(1);
    }),
  );

  it('dispatchHermesJob throws when running outside Tauri', async () => {
    const store = useNotificationsStore();
    await expect(
      store.dispatchHermesJob({ prompt: 'p', working_folder: 'f' }),
    ).rejects.toThrow(/desktop app/);
  });

  it(
    'fetchHermesOfficeStatus invokes the Tauri command and returns the snapshot',
    withTauri(async () => {
      mockInvoke.mockResolvedValueOnce({
        installed: true,
        install_path: 'C:\\Users\\u\\AppData\\Local\\Programs\\hermes-desktop\\hermes-agent.exe',
        gateway_running: true,
        gateway_url: 'http://127.0.0.1:8642',
        message: null,
      });
      const store = useNotificationsStore();
      const status = await store.fetchHermesOfficeStatus();
      expect(mockInvoke).toHaveBeenCalledWith('hermes_office_status');
      expect(status.installed).toBe(true);
      expect(status.gateway_running).toBe(true);
    }),
  );

  it('fetchHermesOfficeStatus returns a not-installed snapshot outside Tauri', async () => {
    const store = useNotificationsStore();
    const status = await store.fetchHermesOfficeStatus();
    expect(status.installed).toBe(false);
    expect(status.gateway_running).toBe(false);
    expect(status.message).toMatch(/desktop app/);
  });
});
