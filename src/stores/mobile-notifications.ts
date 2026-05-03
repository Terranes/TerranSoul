import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { createGrpcWebRemoteHost, remoteBaseUrl } from '../transport/grpc_web';
import type { RemoteCopilotSessionStatus, RemoteHost, RemoteWorkflowRun } from '../transport/remote-host';
import type { TaskInfo } from './tasks';
import { useMobilePairingStore } from './mobile-pairing';
import { useSettingsStore } from './settings';
import { shouldUseRemoteConversation } from '../utils/runtime-target';

export const DEFAULT_MOBILE_NOTIFICATION_THRESHOLD_MS = 30_000;
export const DEFAULT_MOBILE_NOTIFICATION_POLL_MS = 10_000;

export type MobileNotificationKind = 'workflow' | 'task' | 'copilot';

export interface MobileNotificationPayload {
  id: string;
  kind: MobileNotificationKind;
  title: string;
  body: string;
}

interface WorkflowRecord {
  firstSeenAt: number;
  status: string;
  wasTerminal: boolean;
  notifiedTerminal: boolean;
}

interface TaskRecord {
  firstSeenAt: number;
  status: string;
  wasTerminal: boolean;
  notifiedTerminal: boolean;
}

interface CopilotRecord {
  firstSeenAt: number;
  notifiedThreshold: boolean;
  eventCount: number;
}

interface TrackedTask {
  id: string;
  status: string;
  name?: string;
  description?: string;
}

export interface MobileNotificationTracker {
  observeWorkflows(runs: RemoteWorkflowRun[], now: number, thresholdMs: number): MobileNotificationPayload[];
  observeTask(task: TrackedTask, now: number, thresholdMs: number): MobileNotificationPayload[];
  observeCopilot(status: RemoteCopilotSessionStatus, now: number, thresholdMs: number): MobileNotificationPayload[];
  reset(): void;
}

export interface MobileNotificationAdapters {
  now(): number;
  setInterval(callback: () => void, ms: number): ReturnType<typeof setInterval>;
  clearInterval(handle: ReturnType<typeof setInterval>): void;
  notify(payload: MobileNotificationPayload): Promise<void>;
  remoteHost(): RemoteHost | null;
  listenTaskProgress(callback: (task: TaskInfo) => void): Promise<UnlistenFn>;
}

const TERMINAL_STATUSES = new Set(['completed', 'complete', 'done', 'failed', 'cancelled', 'canceled', 'success', 'finished']);
let adapterOverrides: Partial<MobileNotificationAdapters> = {};

export function configureMobileNotificationAdapters(overrides: Partial<MobileNotificationAdapters>): void {
  adapterOverrides = { ...adapterOverrides, ...overrides };
}

export function resetMobileNotificationAdapters(): void {
  adapterOverrides = {};
}

export function createMobileNotificationTracker(): MobileNotificationTracker {
  const workflows = new Map<string, WorkflowRecord>();
  const tasks = new Map<string, TaskRecord>();
  const copilotSessions = new Map<string, CopilotRecord>();

  return {
    observeWorkflows(runs, now, thresholdMs) {
      const notifications: MobileNotificationPayload[] = [];
      for (const run of runs) {
        const workflowId = run.workflowId || run.name;
        if (!workflowId) continue;
        const status = normaliseStatus(run.status);
        const terminal = isTerminalStatus(status);
        const previous = workflows.get(workflowId);
        const firstSeenAt = previous?.firstSeenAt ?? now;
        const startedAt = normaliseUnixMs(run.startedAtUnixMs) || firstSeenAt;
        const durationMs = Math.max(0, now - startedAt);
        const shouldNotify = previous && !previous.wasTerminal && terminal && !previous.notifiedTerminal && durationMs >= thresholdMs;

        if (shouldNotify) {
          notifications.push({
            id: `workflow:${workflowId}:${status}`,
            kind: 'workflow',
            title: 'TerranSoul workflow finished',
            body: `${run.name || workflowId} ${status} after ${formatDuration(durationMs)}.`,
          });
        }

        workflows.set(workflowId, {
          firstSeenAt,
          status,
          wasTerminal: terminal,
          notifiedTerminal: Boolean(previous?.notifiedTerminal || shouldNotify),
        });
      }
      return notifications;
    },

    observeTask(task, now, thresholdMs) {
      const status = normaliseStatus(task.status);
      const terminal = isTerminalStatus(status);
      const previous = tasks.get(task.id);
      const firstSeenAt = previous?.firstSeenAt ?? now;
      const durationMs = Math.max(0, now - firstSeenAt);
      const shouldNotify = previous && !previous.wasTerminal && terminal && !previous.notifiedTerminal && durationMs >= thresholdMs;

      tasks.set(task.id, {
        firstSeenAt,
        status,
        wasTerminal: terminal,
        notifiedTerminal: Boolean(previous?.notifiedTerminal || shouldNotify),
      });

      return shouldNotify
        ? [{
            id: `task:${task.id}:${status}`,
            kind: 'task',
            title: 'TerranSoul task finished',
            body: `${task.name || task.description || task.id} ${status} after ${formatDuration(durationMs)}.`,
          }]
        : [];
    },

    observeCopilot(status, now, thresholdMs) {
      if (!status.found || status.eventCount <= 0) return [];
      const sessionKey = status.sessionId || status.workspaceFolder || 'latest';
      const previous = copilotSessions.get(sessionKey);
      const firstSeenAt = previous?.firstSeenAt ?? now;
      const durationMs = Math.max(0, now - firstSeenAt);
      const shouldNotify = previous && !previous.notifiedThreshold && durationMs >= thresholdMs;

      copilotSessions.set(sessionKey, {
        firstSeenAt,
        notifiedThreshold: Boolean(previous?.notifiedThreshold || shouldNotify),
        eventCount: status.eventCount,
      });

      return shouldNotify
        ? [{
            id: `copilot:${sessionKey}:threshold`,
            kind: 'copilot',
            title: 'Copilot session update',
            body: copilotNotificationBody(status, durationMs),
          }]
        : [];
    },

    reset() {
      workflows.clear();
      tasks.clear();
      copilotSessions.clear();
    },
  };
}

export const useMobileNotificationsStore = defineStore('mobile-notifications', () => {
  const running = ref(false);
  const connected = ref(false);
  const lastPollAt = ref<number | null>(null);
  const error = ref<string | null>(null);
  const sentNotifications = ref<MobileNotificationPayload[]>([]);
  const tracker = createMobileNotificationTracker();
  let timer: ReturnType<typeof setInterval> | null = null;
  let unlistenTaskProgress: UnlistenFn | null = null;

  async function start(): Promise<void> {
    if (running.value || !shouldUseRemoteConversation()) return;
    running.value = true;
    error.value = null;
    await attachTaskProgressListener();
    timer = currentAdapters().setInterval(() => {
      void pollOnce();
    }, pollIntervalMs());
    await pollOnce();
  }

  function stop(): void {
    if (timer) currentAdapters().clearInterval(timer);
    timer = null;
    if (unlistenTaskProgress) unlistenTaskProgress();
    unlistenTaskProgress = null;
    running.value = false;
    connected.value = false;
    tracker.reset();
  }

  async function pollOnce(): Promise<void> {
    if (!notificationsEnabled()) return;
    const host = currentAdapters().remoteHost();
    if (!host) {
      connected.value = false;
      return;
    }

    const now = currentAdapters().now();
    const threshold = thresholdMs();
    try {
      const [workflows, copilot] = await Promise.allSettled([
        host.listWorkflowRuns(true),
        host.getCopilotSessionStatus(),
      ]);
      connected.value = true;
      lastPollAt.value = now;
      error.value = null;

      if (workflows.status === 'fulfilled') {
        await sendAll(tracker.observeWorkflows(workflows.value, now, threshold));
      }
      if (copilot.status === 'fulfilled') {
        await sendAll(tracker.observeCopilot(copilot.value, now, threshold));
      }
    } catch (pollError) {
      connected.value = false;
      error.value = errorMessage(pollError);
    }
  }

  async function observeTaskProgress(task: TaskInfo): Promise<void> {
    if (!notificationsEnabled()) return;
    await sendAll(tracker.observeTask(task, currentAdapters().now(), thresholdMs()));
  }

  async function attachTaskProgressListener(): Promise<void> {
    if (unlistenTaskProgress) return;
    try {
      unlistenTaskProgress = await currentAdapters().listenTaskProgress((task) => {
        void observeTaskProgress(task);
      });
    } catch (listenError) {
      error.value = errorMessage(listenError);
    }
  }

  async function sendAll(notifications: MobileNotificationPayload[]): Promise<void> {
    for (const notification of notifications) {
      sentNotifications.value.push(notification);
      try {
        await currentAdapters().notify(notification);
      } catch (notifyError) {
        error.value = errorMessage(notifyError);
      }
    }
  }

  return {
    running,
    connected,
    lastPollAt,
    error,
    sentNotifications,
    start,
    stop,
    pollOnce,
    observeTaskProgress,
  };
});

function currentAdapters(): MobileNotificationAdapters {
  return {
    now: adapterOverrides.now ?? Date.now,
    setInterval: adapterOverrides.setInterval ?? ((callback, ms) => setInterval(callback, ms)),
    clearInterval: adapterOverrides.clearInterval ?? ((handle) => clearInterval(handle)),
    notify: adapterOverrides.notify ?? defaultNotify,
    remoteHost: adapterOverrides.remoteHost ?? defaultRemoteHost,
    listenTaskProgress: adapterOverrides.listenTaskProgress ?? defaultListenTaskProgress,
  };
}

function defaultRemoteHost(): RemoteHost | null {
  const pairing = useMobilePairingStore();
  const credentials = pairing.storedRecord?.credentials;
  if (!credentials?.desktopHost || !credentials.desktopPort) return null;
  return createGrpcWebRemoteHost({
    baseUrl: remoteBaseUrl(credentials.desktopHost, credentials.desktopPort, false),
  });
}

async function defaultListenTaskProgress(callback: (task: TaskInfo) => void): Promise<UnlistenFn> {
  const { listen } = await import('@tauri-apps/api/event');
  return listen<TaskInfo>('task-progress', (event) => callback(event.payload));
}

async function defaultNotify(payload: MobileNotificationPayload): Promise<void> {
  const notifications = await import('@tauri-apps/plugin-notification');
  let granted = await notifications.isPermissionGranted();
  if (!granted) {
    granted = (await notifications.requestPermission()) === 'granted';
  }
  if (granted) {
    notifications.sendNotification({ title: payload.title, body: payload.body });
  }
}

function notificationsEnabled(): boolean {
  const settings = useSettingsStore();
  return settings.settings.mobile_notifications_enabled ?? true;
}

function thresholdMs(): number {
  const settings = useSettingsStore();
  const value = settings.settings.mobile_notification_threshold_ms ?? DEFAULT_MOBILE_NOTIFICATION_THRESHOLD_MS;
  return Number.isFinite(value) ? Math.max(1_000, value) : DEFAULT_MOBILE_NOTIFICATION_THRESHOLD_MS;
}

function pollIntervalMs(): number {
  const settings = useSettingsStore();
  const value = settings.settings.mobile_notification_poll_ms ?? DEFAULT_MOBILE_NOTIFICATION_POLL_MS;
  return Number.isFinite(value) ? Math.max(5_000, value) : DEFAULT_MOBILE_NOTIFICATION_POLL_MS;
}

function isTerminalStatus(status: string): boolean {
  return TERMINAL_STATUSES.has(status);
}

function normaliseStatus(status: string): string {
  return status.trim().toLowerCase().replace(/[^a-z]+/g, '_').replace(/^_+|_+$/g, '');
}

function normaliseUnixMs(value: number): number {
  if (!Number.isFinite(value) || value <= 0) return 0;
  return value < 10_000_000_000 ? value * 1000 : value;
}

function formatDuration(durationMs: number): string {
  const seconds = Math.max(1, Math.round(durationMs / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return remainingSeconds ? `${minutes}m ${remainingSeconds}s` : `${minutes}m`;
}

function copilotNotificationBody(status: RemoteCopilotSessionStatus, durationMs: number): string {
  const workspace = status.workspaceFolder ? ` in ${shortName(status.workspaceFolder)}` : '';
  const preview = status.lastAssistantPreview || status.lastUserPreview;
  const suffix = preview ? ` Latest: ${preview.slice(0, 96)}` : '';
  return `Copilot has been active${workspace} for ${formatDuration(durationMs)}.${suffix}`;
}

function shortName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

function errorMessage(value: unknown): string {
  return value instanceof Error ? value.message : String(value);
}
