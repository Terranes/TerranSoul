<template>
  <Teleport to="body">
    <Transition name="ts-notif-panel">
      <aside
        v-if="store.panelOpen"
        class="ts-notif-panel"
        role="dialog"
        aria-modal="false"
        aria-label="TerranSoul notifications and active staff"
        data-testid="notification-panel"
      >
        <header class="panel-header">
          <h3>Staff &amp; Notifications</h3>
          <div class="panel-actions">
            <button
              type="button"
              class="ghost"
              data-testid="dispatch-hermes-btn"
              :disabled="!tauriAvailable"
              @click="dispatchOpen = true"
            >
              + Dispatch Hermes Job
            </button>
            <button
              type="button"
              class="ghost"
              :disabled="store.notifications.length === 0"
              @click="store.clearAll()"
            >
              Clear
            </button>
            <button
              type="button"
              class="close"
              aria-label="Close notifications panel"
              @click="store.closePanel()"
            >
              ✕
            </button>
          </div>
        </header>

        <section class="section">
          <h4>
            Active staff
            <span class="count">{{ store.activeJobs.length }}</span>
          </h4>
          <p
            v-if="store.activeJobs.length === 0"
            class="empty"
          >
            No agents are currently running. Dispatch a job to Hermes to see
            multiple staff working in parallel.
          </p>
          <ul
            v-else
            class="job-list"
          >
            <li
              v-for="job in store.activeJobs"
              :key="job.workflowId"
              class="job-row running"
              :data-testid="`job-row-${job.workflowId}`"
            >
              <div class="job-head">
                <span
                  class="spinner"
                  aria-hidden="true"
                />
                <span class="name">{{ job.agentName }}</span>
                <span class="elapsed">{{ formatElapsed(job.startedAt) }}</span>
              </div>
              <div class="job-label">
                {{ job.label }}
              </div>
              <div
                v-if="job.lastLine"
                class="last-line"
              >
                {{ job.lastLine }}
              </div>
              <button
                type="button"
                class="cancel"
                @click="store.cancelJob(job.workflowId)"
              >
                Cancel
              </button>
            </li>
          </ul>
        </section>

        <section class="section">
          <h4>
            Recent activity
            <span class="count">{{ store.notifications.length }}</span>
          </h4>
          <p
            v-if="store.notifications.length === 0"
            class="empty"
          >
            No recent notifications.
          </p>
          <ul
            v-else
            class="notif-list"
          >
            <li
              v-for="n in store.notifications"
              :key="n.id"
              class="notif-row"
              :class="`kind-${n.kind}`"
              :data-testid="`notif-row-${n.id}`"
            >
              <div class="notif-head">
                <span class="title">{{ n.title }}</span>
                <span class="time">{{ formatTime(n.timestamp) }}</span>
              </div>
              <div class="notif-body">
                {{ n.body }}
              </div>
              <button
                type="button"
                class="dismiss"
                aria-label="Dismiss notification"
                @click="store.dismiss(n.id)"
              >
                ✕
              </button>
            </li>
          </ul>
        </section>

        <HermesDispatchDialog
          v-if="dispatchOpen"
          @close="dispatchOpen = false"
        />
      </aside>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { useNotificationsStore } from '../stores/notifications';
import HermesDispatchDialog from './HermesDispatchDialog.vue';

const store = useNotificationsStore();
const dispatchOpen = ref(false);
const tauriAvailable =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// Re-render elapsed labels every second while panel is open.
const tick = ref(0);
let tickHandle: ReturnType<typeof setInterval> | null = null;
onMounted(() => {
  tickHandle = setInterval(() => {
    tick.value = (tick.value + 1) % 1_000_000;
  }, 1000);
});
onUnmounted(() => {
  if (tickHandle) clearInterval(tickHandle);
});

function formatElapsed(startedAt: number): string {
  // Touch `tick` so the template re-renders.
  void tick.value;
  const sec = Math.max(0, Math.floor((Date.now() - startedAt) / 1000));
  if (sec < 60) return `${sec}s`;
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  if (m < 60) return `${m}m ${s}s`;
  const h = Math.floor(m / 60);
  return `${h}h ${m % 60}m`;
}

function formatTime(ts: number): string {
  try {
    return new Date(ts).toLocaleTimeString(undefined, {
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return '';
  }
}
</script>

<style scoped>
.ts-notif-panel {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  width: min(380px, 92vw);
  background: var(--ts-bg-panel);
  border-left: 1px solid var(--ts-border);
  box-shadow: -8px 0 24px rgba(0, 0, 0, 0.55);
  z-index: 1499;
  display: flex;
  flex-direction: column;
  color: var(--ts-text-primary);
  font-size: var(--ts-text-sm);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--ts-border-subtle);
  gap: 8px;
}

.panel-header h3 {
  margin: 0;
  font-size: 0.95rem;
  color: var(--ts-text-bright);
}

.panel-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.panel-actions .ghost {
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-hover);
  color: var(--ts-text-primary);
  padding: 4px 10px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.75rem;
}
.panel-actions .ghost:hover:not(:disabled) {
  border-color: var(--ts-accent);
  color: var(--ts-accent);
}
.panel-actions .ghost:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.panel-actions .close {
  background: transparent;
  border: none;
  color: var(--ts-text-muted);
  cursor: pointer;
  font-size: 1rem;
  padding: 4px 6px;
}
.panel-actions .close:hover {
  color: var(--ts-text-primary);
}

.section {
  padding: 10px 16px;
  border-bottom: 1px solid var(--ts-border-subtle);
  overflow-y: auto;
  flex: 0 0 auto;
}
.section:last-child {
  flex: 1 1 auto;
  border-bottom: none;
}

.section h4 {
  margin: 0 0 8px;
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--ts-text-muted);
  display: flex;
  align-items: center;
  gap: 6px;
}
.section h4 .count {
  background: var(--ts-bg-hover);
  border-radius: 999px;
  padding: 1px 7px;
  font-size: 0.65rem;
  color: var(--ts-text-secondary);
}

.empty {
  color: var(--ts-text-muted);
  font-size: 0.78rem;
  margin: 6px 0;
  line-height: 1.4;
}

.job-list,
.notif-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.job-row {
  position: relative;
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-border);
  border-radius: 8px;
  padding: 10px 36px 10px 12px;
}

.job-row.running {
  border-color: var(--ts-accent);
}

.job-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}
.job-head .name {
  font-weight: 600;
  color: var(--ts-text-bright);
}
.job-head .elapsed {
  margin-left: auto;
  font-size: 0.7rem;
  color: var(--ts-text-muted);
}

.spinner {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  border: 2px solid var(--ts-accent);
  border-top-color: transparent;
  animation: ts-spin 1s linear infinite;
}
@keyframes ts-spin {
  to { transform: rotate(360deg); }
}

.job-label {
  font-size: 0.8rem;
  color: var(--ts-text-secondary);
  margin-bottom: 2px;
  white-space: pre-wrap;
  word-break: break-word;
}

.last-line {
  font-family: var(--ts-font-mono, ui-monospace, SFMono-Regular, monospace);
  font-size: 0.7rem;
  color: var(--ts-text-muted);
  background: var(--ts-bg-input);
  border-radius: 4px;
  padding: 4px 6px;
  max-height: 60px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cancel {
  position: absolute;
  top: 8px;
  right: 8px;
  background: transparent;
  border: 1px solid var(--ts-border);
  color: var(--ts-text-muted);
  font-size: 0.65rem;
  padding: 2px 6px;
  border-radius: 4px;
  cursor: pointer;
}
.cancel:hover {
  color: var(--ts-accent-pink);
  border-color: var(--ts-accent-pink);
}

.notif-row {
  position: relative;
  padding: 8px 28px 8px 10px;
  border-radius: 6px;
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-border-subtle);
}

.notif-row.kind-job-completed { border-left: 3px solid var(--ts-success); }
.notif-row.kind-job-failed    { border-left: 3px solid var(--ts-accent-pink); }
.notif-row.kind-job-cancelled { border-left: 3px solid var(--ts-text-muted); }
.notif-row.kind-job-started   { border-left: 3px solid var(--ts-accent); }

.notif-head {
  display: flex;
  align-items: center;
  gap: 6px;
}
.notif-head .title {
  font-weight: 600;
  font-size: 0.8rem;
  color: var(--ts-text-bright);
}
.notif-head .time {
  margin-left: auto;
  font-size: 0.65rem;
  color: var(--ts-text-muted);
}
.notif-body {
  font-size: 0.74rem;
  color: var(--ts-text-secondary);
  margin-top: 2px;
  white-space: pre-wrap;
  word-break: break-word;
}
.dismiss {
  position: absolute;
  top: 6px;
  right: 6px;
  background: transparent;
  border: none;
  color: var(--ts-text-muted);
  cursor: pointer;
  font-size: 0.7rem;
  padding: 2px 4px;
}
.dismiss:hover {
  color: var(--ts-text-primary);
}

.ts-notif-panel-enter-active,
.ts-notif-panel-leave-active {
  transition: transform 0.22s ease, opacity 0.22s ease;
}
.ts-notif-panel-enter-from,
.ts-notif-panel-leave-to {
  transform: translateX(20px);
  opacity: 0;
}
</style>
