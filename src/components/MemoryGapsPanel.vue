<template>
  <div class="memory-gaps-panel">
    <h3 class="panel-title">
      Memory Gaps
      <span
        v-if="gaps.length > 0"
        class="gap-count"
      >({{ gaps.length }})</span>
    </h3>
    <p class="panel-desc">
      Queries where retrieval found no strong match — potential blind spots in the knowledge base.
    </p>

    <div
      v-if="loading"
      class="loading"
    >
      Loading gaps...
    </div>

    <div
      v-else-if="gaps.length === 0"
      class="empty-state"
    >
      No memory gaps detected yet. Gaps are recorded when search queries return low-confidence results.
    </div>

    <ul
      v-else
      class="gap-list"
    >
      <li
        v-for="gap in gaps"
        :key="gap.id"
        class="gap-item"
      >
        <div class="gap-header">
          <span class="gap-snippet">{{ gap.context_snippet }}</span>
          <button
            class="dismiss-btn"
            title="Dismiss this gap"
            @click="dismissGap(gap.id)"
          >
            ✕
          </button>
        </div>
        <div class="gap-meta">
          <span class="gap-time">{{ formatDate(gap.ts) }}</span>
          <span
            v-if="gap.session_id"
            class="gap-session"
          >{{ gap.session_id }}</span>
        </div>
      </li>
    </ul>

    <button
      v-if="!loading"
      class="refresh-btn"
      @click="loadGaps"
    >
      Refresh
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface MemoryGap {
  id: number;
  context_snippet: string;
  session_id: string | null;
  ts: number;
}

const gaps = ref<MemoryGap[]>([]);
const loading = ref(false);

async function loadGaps() {
  loading.value = true;
  try {
    gaps.value = await invoke<MemoryGap[]>('brain_review_gaps', { limit: 50 });
  } catch {
    // Silently handle — panel is informational.
    gaps.value = [];
  } finally {
    loading.value = false;
  }
}

async function dismissGap(id: number) {
  try {
    await invoke('brain_review_gaps', { dismiss: id });
    gaps.value = gaps.value.filter(g => g.id !== id);
  } catch {
    // No-op on failure.
  }
}

function formatDate(ms: number): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(ms));
}

onMounted(loadGaps);
</script>

<style scoped>
.memory-gaps-panel {
  padding: 1rem;
  background: var(--ts-surface);
  border-radius: 8px;
  color: var(--ts-text);
  font-size: 0.875rem;
}

.panel-title {
  margin: 0 0 0.25rem;
  font-size: 1rem;
  color: var(--ts-text-bright);
}

.gap-count {
  font-weight: 400;
  color: var(--ts-text-muted);
  font-size: 0.875rem;
}

.panel-desc {
  margin: 0 0 0.75rem;
  color: var(--ts-text-muted);
  font-size: 0.8rem;
}

.loading,
.empty-state {
  padding: 1rem 0;
  text-align: center;
  color: var(--ts-text-muted);
}

.gap-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.gap-item {
  padding: 0.5rem 0;
  border-bottom: 1px solid var(--ts-border-subtle, rgba(255, 255, 255, 0.05));
}

.gap-item:last-child {
  border-bottom: none;
}

.gap-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.5rem;
}

.gap-snippet {
  flex: 1;
  color: var(--ts-text);
  line-height: 1.4;
  word-break: break-word;
}

.dismiss-btn {
  background: none;
  border: none;
  color: var(--ts-text-muted);
  cursor: pointer;
  padding: 0.125rem 0.375rem;
  font-size: 0.75rem;
  border-radius: 4px;
  flex-shrink: 0;
}

.dismiss-btn:hover {
  background: var(--ts-danger-bg, rgba(220, 38, 38, 0.15));
  color: var(--ts-danger, #ef4444);
}

.gap-meta {
  display: flex;
  gap: 0.75rem;
  margin-top: 0.25rem;
  font-size: 0.75rem;
  color: var(--ts-text-muted);
}

.gap-session {
  font-family: monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 50%;
}

.refresh-btn {
  margin-top: 0.75rem;
  padding: 0.375rem 0.75rem;
  background: var(--ts-surface-hover, rgba(255, 255, 255, 0.05));
  border: 1px solid var(--ts-border);
  border-radius: 4px;
  color: var(--ts-text);
  cursor: pointer;
  font-size: 0.8rem;
}

.refresh-btn:hover {
  background: var(--ts-accent-bg, rgba(96, 165, 250, 0.15));
  border-color: var(--ts-accent);
}
</style>
