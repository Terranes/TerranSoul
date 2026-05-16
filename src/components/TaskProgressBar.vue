<template>
  <div
    v-if="taskStore.activeTasks.length > 0"
    class="task-progress-panel"
  >
    <div
      v-for="task in taskStore.activeTasks"
      :key="task.id"
      class="task-card"
      :class="task.status"
    >
      <div class="task-header">
        <span class="task-kind">{{ kindLabel(task.kind) }}</span>
        <span
          class="task-status"
          :class="task.status"
        >{{ statusLabel(task.status) }}</span>
      </div>
      <p class="task-desc">
        {{ task.description }}
      </p>
      <div class="progress-track">
        <div
          class="progress-fill"
          :style="{ width: task.progress + '%' }"
          :class="{ pulsing: task.status === 'running' }"
        />
      </div>
      <div class="task-footer">
        <span class="task-items">
          {{ task.processed_items }}/{{ task.total_items }} items
        </span>
        <span class="task-percent">{{ task.progress }}%</span>
      </div>
      <div class="task-actions">
        <button
          v-if="task.status === 'running'"
          class="task-btn cancel"
          @click="taskStore.cancelTask(task.id)"
        >
          Cancel
        </button>
        <button
          v-if="task.status === 'paused'"
          class="task-btn resume"
          @click="taskStore.resumeTask(task.id)"
        >
          Resume
        </button>
      </div>
      <p
        v-if="task.error"
        class="task-error"
      >
        {{ task.error }}
      </p>
      <!-- BRAIN-REPO-RAG-2b: collapsible debug log fed by `task-progress`
           events that carry a `log_line`. Useful for repo-RAG ingest so the
           user can see every file scanned + skipped + indexed in real time. -->
      <div
        v-if="(taskStore.taskLogs.get(task.id)?.length ?? 0) > 0"
        class="task-log"
      >
        <button
          class="task-log-toggle"
          type="button"
          @click="toggleLog(task.id)"
        >
          <span class="task-log-caret">{{ expandedLogs.has(task.id) ? '▼' : '▶' }}</span>
          Debug log ({{ taskStore.taskLogs.get(task.id)?.length ?? 0 }} lines)
          <span
            v-if="logCounters(task.id) as Record<string, number>"
            class="task-log-counters"
          >
            <template
              v-for="(count, label) in logCounters(task.id)"
              :key="label"
            >
              <span
                v-if="count > 0"
                class="task-log-chip"
                :class="`chip-${label}`"
              >{{ label }}: {{ count }}</span>
            </template>
          </span>
        </button>
        <pre
          v-if="expandedLogs.has(task.id)"
          ref="logScrollEl"
          class="task-log-body"
        >{{ taskStore.taskLogs.get(task.id)?.join('\n') }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, nextTick, watch } from 'vue';
import { useTaskStore } from '../stores/tasks';

const taskStore = useTaskStore();

/** BRAIN-REPO-RAG-2b: which task log panels are open. */
const expandedLogs = ref<Set<string>>(new Set());
const logScrollEl = ref<HTMLPreElement | HTMLPreElement[] | null>(null);

function toggleLog(taskId: string) {
  const next = new Set(expandedLogs.value);
  if (next.has(taskId)) {
    next.delete(taskId);
  } else {
    next.add(taskId);
  }
  expandedLogs.value = next;
}

/** Count log lines per category for the sticky chip row. */
function logCounters(taskId: string): Record<string, number> {
  const lines = taskStore.taskLogs.get(taskId) ?? [];
  const counts: Record<string, number> = {
    indexed: 0, 'skip-size': 0, 'skip-binary': 0,
    'skip-unchanged': 0, 'skip-secret': 0,
  };
  for (const line of lines) {
    if (line.startsWith('persist ')) counts.indexed += 1;
    else if (line.startsWith('skip[too_large]')) counts['skip-size'] += 1;
    else if (line.startsWith('skip[binary]')) counts['skip-binary'] += 1;
    else if (line.startsWith('skip[unchanged]')) counts['skip-unchanged'] += 1;
    else if (line.startsWith('skip[secret]')) counts['skip-secret'] += 1;
  }
  return counts;
}

// Auto-scroll the open log panel as new lines arrive.
watch(
  () => [...taskStore.taskLogs.values()].reduce((sum, b) => sum + b.length, 0),
  async () => {
    await nextTick();
    const el = logScrollEl.value;
    const nodes = Array.isArray(el) ? el : el ? [el] : [];
    for (const n of nodes) n.scrollTop = n.scrollHeight;
  },
);

function kindLabel(kind: string): string {
  const labels: Record<string, string> = {
    ingest: '📄 Import', crawl: '🕸️ Crawl',
    quest: '⚔️ Quest', extract: '🔍 Extract',
    model_pull: '🧠 Model Pull', custom: '⚙️ Task',
  };
  return labels[kind] ?? kind;
}

function statusLabel(status: string): string {
  const labels: Record<string, string> = {
    running: 'Running', paused: 'Paused',
    completed: 'Done', failed: 'Failed', cancelled: 'Cancelled',
  };
  return labels[status] ?? status;
}
</script>

<style scoped>
.task-progress-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px 12px;
}

.task-card {
  background: var(--ts-bg-surface, rgba(30, 30, 45, 0.85));
  border-radius: var(--ts-radius-md, 10px);
  padding: 10px 12px;
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.08));
}

.task-card.paused {
  border-color: var(--ts-accent-warning, #f0a020);
}

.task-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

.task-kind {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--ts-text-primary, #eee);
}

.task-status {
  font-size: 0.7rem;
  padding: 2px 8px;
  border-radius: var(--ts-radius-pill, 100px);
  background: var(--ts-accent, #7c3aed);
  color: var(--ts-text-on-accent);
}

.task-status.paused {
  background: var(--ts-accent-warning, #f0a020);
}

.task-status.failed {
  background: var(--ts-accent-error, #e53e3e);
}

.task-desc {
  font-size: 0.75rem;
  color: var(--ts-text-secondary, #aaa);
  margin: 4px 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.progress-track {
  width: 100%;
  height: 6px;
  background: var(--ts-bg-inset, rgba(0, 0, 0, 0.3));
  border-radius: 3px;
  overflow: hidden;
  margin: 6px 0 4px;
}

.progress-fill {
  height: 100%;
  background: var(--ts-accent, #7c3aed);
  border-radius: 3px;
  transition: width 0.4s ease;
}

.progress-fill.pulsing {
  animation: pulse-glow 1.5s ease-in-out infinite;
}

@keyframes pulse-glow {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}

.task-footer {
  display: flex;
  justify-content: space-between;
  font-size: 0.7rem;
  color: var(--ts-text-muted, #888);
}

.task-actions {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}

.task-btn {
  font-size: 0.7rem;
  padding: 3px 10px;
  border-radius: var(--ts-radius-pill, 100px);
  border: none;
  cursor: pointer;
  font-weight: 600;
  transition: opacity 0.2s;
}

.task-btn:hover {
  opacity: 0.85;
}

.task-btn.cancel {
  background: var(--ts-accent-error, #e53e3e);
  color: var(--ts-text-on-accent);
}

.task-btn.resume {
  background: var(--ts-accent, #7c3aed);
  color: var(--ts-text-on-accent);
}

.task-error {
  font-size: 0.7rem;
  color: var(--ts-accent-error, #e53e3e);
  margin-top: 4px;
}

/* BRAIN-REPO-RAG-2b: debug log surface. */
.task-log {
  margin-top: 8px;
  border-top: 1px solid var(--ts-border, rgba(255, 255, 255, 0.08));
  padding-top: 6px;
}

.task-log-toggle {
  background: transparent;
  border: none;
  color: var(--ts-text-secondary, #aaa);
  font-size: 0.7rem;
  font-weight: 600;
  cursor: pointer;
  padding: 2px 0;
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  text-align: left;
  flex-wrap: wrap;
}

.task-log-toggle:hover {
  color: var(--ts-text-primary, #eee);
}

.task-log-caret {
  font-size: 0.65rem;
  opacity: 0.8;
}

.task-log-counters {
  display: inline-flex;
  gap: 4px;
  margin-left: auto;
  flex-wrap: wrap;
}

.task-log-chip {
  font-size: 0.65rem;
  padding: 1px 6px;
  border-radius: var(--ts-radius-pill, 100px);
  background: var(--ts-bg-inset, rgba(0, 0, 0, 0.3));
  color: var(--ts-text-muted, #888);
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.08));
}

.task-log-chip.chip-indexed {
  color: var(--ts-accent, #7c3aed);
  border-color: var(--ts-accent, #7c3aed);
}

.task-log-chip.chip-skip-size,
.task-log-chip.chip-skip-binary,
.task-log-chip.chip-skip-unchanged {
  color: var(--ts-warning, #f0a020);
  border-color: var(--ts-warning, #f0a020);
}

.task-log-chip.chip-skip-secret {
  color: var(--ts-accent-error, #e53e3e);
  border-color: var(--ts-accent-error, #e53e3e);
}

.task-log-body {
  margin: 6px 0 0;
  padding: 8px;
  max-height: 220px;
  overflow-y: auto;
  background: var(--ts-bg-inset, rgba(0, 0, 0, 0.35));
  border-radius: var(--ts-radius-sm, 6px);
  font-family: var(--ts-font-mono, ui-monospace, monospace);
  font-size: 0.7rem;
  line-height: 1.4;
  color: var(--ts-text-secondary, #aaa);
  white-space: pre-wrap;
  word-break: break-all;
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.08));
}
</style>
