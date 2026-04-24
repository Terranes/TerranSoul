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
    </div>
  </div>
</template>

<script setup lang="ts">
import { useTaskStore } from '../stores/tasks';

const taskStore = useTaskStore();

function kindLabel(kind: string): string {
  const labels: Record<string, string> = {
    ingest: '📄 Import', crawl: '🕸️ Crawl',
    quest: '⚔️ Quest', extract: '🔍 Extract', custom: '⚙️ Task',
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
  color: #fff;
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
  color: #fff;
}

.task-btn.resume {
  background: var(--ts-accent, #7c3aed);
  color: #fff;
}

.task-error {
  font-size: 0.7rem;
  color: var(--ts-accent-error, #e53e3e);
  margin-top: 4px;
}
</style>
