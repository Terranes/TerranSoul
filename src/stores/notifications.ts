/**
 * Unified notifications + agent-job ledger.
 *
 * Tracks every external-CLI workflow (Codex, Claude, Gemini, Hermes,
 * Custom) that TerranSoul has dispatched, plus a queue of dismissible
 * notifications that the UI bubble and panel render.
 *
 * Inputs:
 *  - `agent-cli-output` Tauri events — line-by-line stdout/stderr from
 *    every running staff. Used to update each job's `lastLine` preview.
 *  - `workflow-completed` Tauri events — emitted exactly once per
 *    workflow when it transitions to a terminal state. Flips job status
 *    and pushes a "Job done" notification.
 *
 * Outputs:
 *  - `useNotificationsStore().notifications` — unread + read items for
 *    the NotificationPanel.
 *  - `unreadCount` — for the NotificationBubble badge.
 *  - `activeJobs` / `recentJobs` — derived job ledger.
 *  - Native system notifications via `@tauri-apps/plugin-notification`
 *    when the app window is unfocused.
 */

import { defineStore } from 'pinia';
import { ref, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ── Types ───────────────────────────────────────────────────────────────

export type JobStatus =
  | 'queued'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type NotificationKind =
  | 'job-started'
  | 'job-completed'
  | 'job-failed'
  | 'job-cancelled'
  | 'info';

export interface AgentJob {
  workflowId: string;
  agentId: string;
  agentName: string;
  label: string;
  status: JobStatus;
  startedAt: number;
  completedAt: number | null;
  lastLine: string;
  lineCount: number;
  errorMessage: string | null;
}

export interface AppNotification {
  id: string;
  kind: NotificationKind;
  title: string;
  body: string;
  timestamp: number;
  read: boolean;
  /** Optional workflow id this notification is about. */
  workflowId?: string;
}

interface CliEventPayload {
  workflow_id: string;
  event:
    | { type: 'started'; pid: number }
    | { type: 'line'; stream: 'stdout' | 'stderr'; text: string }
    | { type: 'exited'; code: number | null }
    | { type: 'spawn_error'; message: string };
}

interface WorkflowCompletedPayload {
  workflow_id: string;
  status: 'completed' | 'failed' | 'cancelled';
  message: string | null;
}

interface DispatchHermesJobResult {
  workflow_id: string;
  agent_id: string;
  label: string;
}

export interface DispatchHermesJobRequest {
  prompt: string;
  working_folder: string;
  label?: string;
  binary?: string;
  extra_args?: string[];
}

export interface HermesOfficeStatus {
  installed: boolean;
  install_path: string | null;
  gateway_running: boolean;
  gateway_url: string;
  message: string | null;
}

const MAX_NOTIFICATIONS = 100;
const MAX_LINE_PREVIEW = 240;

function isTauriAvailable(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function freshId(): string {
  return `n-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export const useNotificationsStore = defineStore('notifications', () => {
  const jobs = ref<Record<string, AgentJob>>({});
  const notifications = ref<AppNotification[]>([]);
  const panelOpen = ref(false);
  const initialized = ref(false);

  let unlistenCli: UnlistenFn | null = null;
  let unlistenCompleted: UnlistenFn | null = null;

  // ── Derived ───────────────────────────────────────────────────────

  const allJobs = computed<AgentJob[]>(() =>
    Object.values(jobs.value).sort((a, b) => b.startedAt - a.startedAt),
  );
  const activeJobs = computed<AgentJob[]>(() =>
    allJobs.value.filter(
      (j) => j.status === 'queued' || j.status === 'running',
    ),
  );
  const recentJobs = computed<AgentJob[]>(() =>
    allJobs.value
      .filter(
        (j) =>
          j.status === 'completed' ||
          j.status === 'failed' ||
          j.status === 'cancelled',
      )
      .slice(0, 20),
  );
  const unreadCount = computed(
    () => notifications.value.filter((n) => !n.read).length,
  );

  // ── Mutations ─────────────────────────────────────────────────────

  function pushNotification(n: Omit<AppNotification, 'id' | 'timestamp' | 'read'>) {
    notifications.value.unshift({
      id: freshId(),
      timestamp: Date.now(),
      read: false,
      ...n,
    });
    if (notifications.value.length > MAX_NOTIFICATIONS) {
      notifications.value.length = MAX_NOTIFICATIONS;
    }
    // System notification when window not focused. Best-effort.
    if (typeof document !== 'undefined' && !document.hasFocus()) {
      void sendSystemNotification(n.title, n.body);
    }
  }

  async function sendSystemNotification(title: string, body: string) {
    if (!isTauriAvailable()) return;
    try {
      const pn = await import('@tauri-apps/plugin-notification');
      let granted = await pn.isPermissionGranted();
      if (!granted) {
        const perm = await pn.requestPermission();
        granted = perm === 'granted';
      }
      if (granted) {
        pn.sendNotification({ title, body });
      }
    } catch {
      /* notifications are best-effort */
    }
  }

  function trackJob(input: {
    workflowId: string;
    agentId: string;
    agentName: string;
    label: string;
  }) {
    if (jobs.value[input.workflowId]) return;
    jobs.value[input.workflowId] = {
      workflowId: input.workflowId,
      agentId: input.agentId,
      agentName: input.agentName,
      label: input.label,
      status: 'queued',
      startedAt: Date.now(),
      completedAt: null,
      lastLine: '',
      lineCount: 0,
      errorMessage: null,
    };
    pushNotification({
      kind: 'job-started',
      title: `${input.agentName} started`,
      body: input.label,
      workflowId: input.workflowId,
    });
  }

  function handleCliEvent(payload: CliEventPayload) {
    const job = jobs.value[payload.workflow_id];
    if (!job) return;
    const ev = payload.event;
    if (ev.type === 'started') {
      job.status = 'running';
    } else if (ev.type === 'line') {
      job.status = 'running';
      const text = ev.text.length > MAX_LINE_PREVIEW
        ? ev.text.slice(0, MAX_LINE_PREVIEW) + '…'
        : ev.text;
      job.lastLine = text;
      job.lineCount += 1;
    } else if (ev.type === 'spawn_error') {
      job.status = 'failed';
      job.errorMessage = ev.message;
      job.completedAt = Date.now();
    }
  }

  function handleWorkflowCompleted(payload: WorkflowCompletedPayload) {
    const job = jobs.value[payload.workflow_id];
    if (!job) return;
    job.status = payload.status;
    job.completedAt = Date.now();
    if (payload.message) job.errorMessage = payload.message;

    if (payload.status === 'completed') {
      pushNotification({
        kind: 'job-completed',
        title: `✓ ${job.agentName} finished`,
        body: job.label,
        workflowId: job.workflowId,
      });
    } else if (payload.status === 'failed') {
      pushNotification({
        kind: 'job-failed',
        title: `✗ ${job.agentName} failed`,
        body: payload.message ?? job.label,
        workflowId: job.workflowId,
      });
    } else if (payload.status === 'cancelled') {
      pushNotification({
        kind: 'job-cancelled',
        title: `⊘ ${job.agentName} cancelled`,
        body: payload.message ?? job.label,
        workflowId: job.workflowId,
      });
    }
  }

  // ── Public API ────────────────────────────────────────────────────

  async function dispatchHermesJob(req: DispatchHermesJobRequest): Promise<AgentJob> {
    if (!isTauriAvailable()) {
      throw new Error('Hermes dispatch is only available in the desktop app');
    }
    const result = await invoke<DispatchHermesJobResult>('dispatch_hermes_job', {
      request: req,
    });
    trackJob({
      workflowId: result.workflow_id,
      agentId: result.agent_id,
      agentName: 'Hermes Staff',
      label: result.label,
    });
    return jobs.value[result.workflow_id];
  }

  async function fetchHermesOfficeStatus(): Promise<HermesOfficeStatus> {
    if (!isTauriAvailable()) {
      return {
        installed: false,
        install_path: null,
        gateway_running: false,
        gateway_url: 'http://127.0.0.1:8642',
        message: 'Hermes Office status is only available in the desktop app.',
      };
    }
    return invoke<HermesOfficeStatus>('hermes_office_status');
  }

  async function cancelJob(workflowId: string, reason?: string) {
    if (!isTauriAvailable()) return;
    try {
      await invoke('roster_cancel_workflow', { workflowId, reason });
    } catch (e) {
      pushNotification({
        kind: 'info',
        title: 'Cancel failed',
        body: String(e),
        workflowId,
      });
    }
  }

  function markAllRead() {
    for (const n of notifications.value) n.read = true;
  }
  function dismiss(id: string) {
    notifications.value = notifications.value.filter((n) => n.id !== id);
  }
  function clearAll() {
    notifications.value = [];
  }
  function openPanel() {
    panelOpen.value = true;
    markAllRead();
  }
  function closePanel() {
    panelOpen.value = false;
  }
  function togglePanel() {
    if (panelOpen.value) closePanel();
    else openPanel();
  }

  async function initialize() {
    if (initialized.value) return;
    initialized.value = true;
    if (!isTauriAvailable()) return;
    try {
      unlistenCli = await listen<CliEventPayload>('agent-cli-output', (e) =>
        handleCliEvent(e.payload),
      );
      unlistenCompleted = await listen<WorkflowCompletedPayload>(
        'workflow-completed',
        (e) => handleWorkflowCompleted(e.payload),
      );
    } catch {
      /* listening is best-effort */
    }
  }

  async function teardown() {
    if (unlistenCli) {
      unlistenCli();
      unlistenCli = null;
    }
    if (unlistenCompleted) {
      unlistenCompleted();
      unlistenCompleted = null;
    }
    initialized.value = false;
  }

  // Auto-clear panel-open badge when panel is open
  watch(panelOpen, (open) => {
    if (open) markAllRead();
  });

  return {
    // state
    jobs,
    notifications,
    panelOpen,
    // derived
    allJobs,
    activeJobs,
    recentJobs,
    unreadCount,
    // actions
    initialize,
    teardown,
    dispatchHermesJob,
    fetchHermesOfficeStatus,
    cancelJob,
    trackJob,
    pushNotification,
    handleCliEvent,
    handleWorkflowCompleted,
    markAllRead,
    dismiss,
    clearAll,
    openPanel,
    closePanel,
    togglePanel,
  };
});
