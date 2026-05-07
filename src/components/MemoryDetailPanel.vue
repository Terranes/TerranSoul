<template>
  <div class="memory-detail-panel">
    <h3 class="panel-title">Memory #{{ entry.id }}</h3>
    <div class="detail-row">
      <span class="label">Type:</span>
      <span class="value">{{ entry.memory_type }}</span>
    </div>
    <div class="detail-row">
      <span class="label">Tier:</span>
      <span class="value">{{ entry.tier }}</span>
    </div>
    <div class="detail-row">
      <span class="label">Importance:</span>
      <span class="value">{{ entry.importance }}/5</span>
    </div>
    <div class="detail-row">
      <span class="label">Confidence:</span>
      <span class="value">{{ (entry.confidence * 100).toFixed(1) }}%</span>
    </div>
    <div class="detail-row">
      <span class="label">Decay:</span>
      <span class="value">{{ (entry.decay_score * 100).toFixed(1) }}%</span>
    </div>
    <div class="detail-row">
      <span class="label">Created:</span>
      <span class="value">{{ formatDate(entry.created_at) }}</span>
    </div>
    <div class="detail-row content-row">
      <span class="label">Content:</span>
      <p class="content-text">{{ entry.content }}</p>
    </div>

    <div
      v-if="reinforcements.length > 0"
      class="reinforcements-section"
    >
      <h4 class="section-title">
        Reinforcements ({{ reinforcements.length }})
      </h4>
      <ul class="reinforcement-list">
        <li
          v-for="r in reinforcements"
          :key="`${r.session_id}-${r.message_index}`"
          class="reinforcement-item"
        >
          <span class="reinf-session">{{ r.session_id }}</span>
          <span class="reinf-time">{{ formatDate(r.ts) }}</span>
        </li>
      </ul>
    </div>
    <div
      v-else
      class="no-reinforcements"
    >
      No reinforcements recorded yet.
    </div>
  </div>
</template>

<script setup lang="ts">
import type { MemoryEntry, ReinforcementRecord } from '../types';

defineProps<{
  entry: MemoryEntry;
  reinforcements: ReinforcementRecord[];
}>();

function formatDate(ms: number): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(ms));
}
</script>

<style scoped>
.memory-detail-panel {
  padding: 1rem;
  background: var(--ts-surface);
  border-radius: 8px;
  color: var(--ts-text);
  font-size: 0.875rem;
}

.panel-title {
  margin: 0 0 0.75rem;
  font-size: 1rem;
  color: var(--ts-text-bright);
}

.detail-row {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.375rem;
}

.label {
  font-weight: 600;
  color: var(--ts-text-muted);
  min-width: 6rem;
}

.content-row {
  flex-direction: column;
}

.content-text {
  margin: 0.25rem 0 0;
  white-space: pre-wrap;
  line-height: 1.4;
}

.reinforcements-section {
  margin-top: 1rem;
  border-top: 1px solid var(--ts-border);
  padding-top: 0.75rem;
}

.section-title {
  margin: 0 0 0.5rem;
  font-size: 0.875rem;
  color: var(--ts-text-bright);
}

.reinforcement-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.reinforcement-item {
  display: flex;
  justify-content: space-between;
  padding: 0.25rem 0;
  border-bottom: 1px solid var(--ts-border-subtle, rgba(255, 255, 255, 0.05));
}

.reinf-session {
  color: var(--ts-accent);
  font-family: monospace;
  font-size: 0.8rem;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 60%;
}

.reinf-time {
  color: var(--ts-text-muted);
  font-size: 0.8rem;
}

.no-reinforcements {
  margin-top: 0.75rem;
  color: var(--ts-text-muted);
  font-style: italic;
}
</style>
