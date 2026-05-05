<template>
  <section
    class="po-panel"
    data-testid="provider-observability-panel"
    aria-labelledby="po-title"
  >
    <header class="po-header">
      <span class="po-icon" aria-hidden="true">📡</span>
      <h2 id="po-title" class="po-title">Provider Status</h2>
      <button
        type="button"
        class="po-refresh-btn"
        title="Refresh provider health"
        :disabled="refreshing"
        @click="refreshAll"
      >
        {{ refreshing ? '...' : '↻' }}
      </button>
    </header>

    <!-- Summary stats -->
    <div class="po-summary">
      <div class="po-stat po-stat--ok">
        <span class="po-stat-num">{{ summary.healthy_count }}</span>
        <span class="po-stat-label">Healthy</span>
      </div>
      <div class="po-stat po-stat--warn">
        <span class="po-stat-num">{{ summary.rate_limited_count }}</span>
        <span class="po-stat-label">Rate-limited</span>
      </div>
      <div class="po-stat po-stat--err">
        <span class="po-stat-num">{{ summary.unhealthy_count }}</span>
        <span class="po-stat-label">Unhealthy</span>
      </div>
      <div class="po-stat">
        <span class="po-stat-num">{{ summary.selected_provider_id || '—' }}</span>
        <span class="po-stat-label">Active</span>
      </div>
    </div>

    <!-- Failover policy -->
    <div class="po-section">
      <h3 class="po-section-title">Failover Policy</h3>
      <div class="po-policy-grid">
        <label class="po-field">
          <span>Max attempts</span>
          <input
            type="number"
            min="1"
            max="10"
            :value="policy.max_attempts"
            @change="updatePolicy('max_attempts', Number(($event.target as HTMLInputElement).value))"
          >
        </label>
        <label class="po-field">
          <span>Cooldown (s)</span>
          <input
            type="number"
            min="0"
            max="600"
            :value="policy.min_cooldown_secs"
            @change="updatePolicy('min_cooldown_secs', Number(($event.target as HTMLInputElement).value))"
          >
        </label>
        <label class="po-field po-field--toggle">
          <span>Privacy mode</span>
          <input
            type="checkbox"
            :checked="policy.respect_privacy"
            @change="updatePolicy('respect_privacy', ($event.target as HTMLInputElement).checked)"
          >
        </label>
      </div>
    </div>

    <!-- Per-task model usage -->
    <div class="po-section">
      <h3 class="po-section-title">Per-Task Model Usage</h3>
      <div class="po-task-grid">
        <div
          v-for="task in taskResolutions"
          :key="task.kind"
          class="po-task-row"
        >
          <span class="po-task-kind">{{ task.label }}</span>
          <span class="po-task-provider">{{ task.resolved.provider_id }}</span>
          <span class="po-task-model">{{ task.resolved.model }}</span>
          <span class="po-task-source">{{ task.resolved.source }}</span>
        </div>
      </div>
    </div>

    <!-- Agent routing overview -->
    <div
      v-if="agentRoutes.length > 0"
      class="po-section"
    >
      <h3 class="po-section-title">Agent Routing</h3>
      <div class="po-task-grid">
        <div
          v-for="route in agentRoutes"
          :key="route.role"
          class="po-task-row"
        >
          <span class="po-task-kind">{{ route.role }}</span>
          <span class="po-task-provider">{{ route.preferred_provider || '(default)' }}</span>
          <span class="po-task-model">{{ route.preferred_model || `tier:${route.preferred_tier}` }}</span>
          <span class="po-task-source">{{ route.enabled ? 'active' : 'disabled' }}</span>
        </div>
      </div>
    </div>

    <!-- Recent failover events -->
    <div
      v-if="summary.recent_events.length > 0"
      class="po-section"
    >
      <h3 class="po-section-title">Recent Failover Events</h3>
      <ul class="po-events">
        <li
          v-for="(evt, i) in summary.recent_events.slice(0, 10)"
          :key="i"
          class="po-event"
        >
          <span class="po-event-provider">{{ evt.provider_id }}</span>
          <span class="po-event-reason" :class="`po-reason--${evt.reason}`">
            {{ formatReason(evt.reason) }}
          </span>
          <span class="po-event-time">{{ formatTime(evt.timestamp_ms) }}</span>
        </li>
      </ul>
    </div>

    <div
      v-if="summary.all_exhausted"
      class="po-alert"
    >
      All providers exhausted — waiting for rate limits to reset.
    </div>
  </section>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type {
  AgentRouteConfig,
  ResolvedProvider,
  TaskKind,
} from '../types';

interface FailoverEvent {
  provider_id: string;
  reason: string;
  timestamp_ms: number;
}

interface FailoverSummary {
  healthy_count: number;
  rate_limited_count: number;
  unhealthy_count: number;
  all_exhausted: boolean;
  recent_events: FailoverEvent[];
  selected_provider_id: string | null;
}

interface FailoverPolicy {
  max_attempts: number;
  respect_privacy: boolean;
  min_cooldown_secs: number;
}

interface TaskResolution {
  kind: TaskKind;
  label: string;
  resolved: ResolvedProvider;
}

const summary = ref<FailoverSummary>({
  healthy_count: 0,
  rate_limited_count: 0,
  unhealthy_count: 0,
  all_exhausted: false,
  recent_events: [],
  selected_provider_id: null,
});

const policy = ref<FailoverPolicy>({
  max_attempts: 3,
  respect_privacy: true,
  min_cooldown_secs: 60,
});

const taskResolutions = ref<TaskResolution[]>([]);
const agentRoutes = ref<AgentRouteConfig[]>([]);
const refreshing = ref(false);

const taskKinds: { kind: TaskKind; label: string }[] = [
  { kind: 'chat', label: 'Chat' },
  { kind: 'embeddings', label: 'Embeddings' },
  { kind: 'rerank', label: 'Rerank' },
  { kind: 'summarise', label: 'Summarise' },
  { kind: 'code_review', label: 'Code Review' },
  { kind: 'long_context', label: 'Long Context' },
];

async function loadSummary() {
  try {
    summary.value = await invoke<FailoverSummary>('get_failover_summary');
  } catch { /* ignore if not available */ }
}

async function loadPolicy() {
  try {
    policy.value = await invoke<FailoverPolicy>('get_failover_policy');
  } catch { /* ignore */ }
}

async function loadTaskResolutions() {
  const results: TaskResolution[] = [];
  for (const tk of taskKinds) {
    try {
      const resolved = await invoke<ResolvedProvider>('resolve_provider_for_task', { kind: tk.kind });
      results.push({ kind: tk.kind, label: tk.label, resolved });
    } catch {
      results.push({
        kind: tk.kind,
        label: tk.label,
        resolved: { source: 'error', provider_id: '—', model: '—', base_url: '', api_key: '', max_tokens: null },
      });
    }
  }
  taskResolutions.value = results;
}

async function loadAgentRoutes() {
  try {
    agentRoutes.value = await invoke<AgentRouteConfig[]>('get_agent_routing');
  } catch { /* ignore */ }
}

async function refreshAll() {
  refreshing.value = true;
  try {
    await invoke('health_check_providers');
  } catch { /* ignore */ }
  await Promise.all([loadSummary(), loadPolicy(), loadTaskResolutions(), loadAgentRoutes()]);
  refreshing.value = false;
}

async function updatePolicy(field: string, value: number | boolean) {
  try {
    policy.value = await invoke<FailoverPolicy>('set_failover_policy', { [field]: value });
  } catch { /* ignore */ }
}

function formatReason(reason: string | Record<string, unknown>): string {
  if (typeof reason === 'string') return reason.replace(/_/g, ' ');
  if ('rate_limit' in reason) return 'rate-limited';
  if ('unhealthy' in reason) return 'unhealthy';
  if ('context_overflow' in reason) return 'context overflow';
  if ('token_cap_exceeded' in reason) return 'token cap exceeded';
  if ('privacy_constraint' in reason) return 'privacy';
  if ('free_tier_exhausted' in reason) return 'exhausted';
  return JSON.stringify(reason);
}

function formatTime(ms: number): string {
  if (!ms) return '';
  const d = new Date(ms);
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

onMounted(async () => {
  await Promise.all([loadSummary(), loadPolicy(), loadTaskResolutions(), loadAgentRoutes()]);
});
</script>

<style scoped>
.po-panel {
  background: var(--ts-bg-panel);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-lg, 8px);
  padding: var(--ts-space-md, 14px) var(--ts-space-lg, 18px);
  box-shadow: var(--ts-shadow-md);
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-md, 14px);
}

.po-header {
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm, 8px);
}
.po-icon {
  font-size: 1.2rem;
}
.po-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 700;
  color: var(--ts-text-primary);
  flex: 1;
}
.po-refresh-btn {
  appearance: none;
  background: var(--ts-bg-selected);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm, 4px);
  color: var(--ts-text-secondary);
  cursor: pointer;
  padding: 4px 8px;
  font-size: 0.9rem;
}
.po-refresh-btn:hover {
  color: var(--ts-text-primary);
  border-color: var(--ts-accent);
}

/* Summary stats */
.po-summary {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(80px, 1fr));
  gap: var(--ts-space-sm, 8px);
}
.po-stat {
  text-align: center;
  padding: 8px;
  border-radius: var(--ts-radius-sm, 4px);
  background: var(--ts-bg-selected);
  border: 1px solid var(--ts-border-subtle);
}
.po-stat-num {
  display: block;
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--ts-text-primary);
}
.po-stat-label {
  font-size: 0.72rem;
  color: var(--ts-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.po-stat--ok .po-stat-num { color: var(--ts-success, #4caf50); }
.po-stat--warn .po-stat-num { color: var(--ts-warning, #f0c040); }
.po-stat--err .po-stat-num { color: var(--ts-error, #f44336); }

/* Sections */
.po-section {
  border-top: 1px solid var(--ts-border-subtle);
  padding-top: var(--ts-space-sm, 8px);
}
.po-section-title {
  margin: 0 0 8px;
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--ts-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

/* Policy controls */
.po-policy-grid {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ts-space-sm, 8px);
}
.po-field {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 0.8rem;
  color: var(--ts-text-secondary);
}
.po-field input[type="number"] {
  width: 70px;
  padding: 4px 6px;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm, 4px);
  background: var(--ts-bg-input, var(--ts-bg-selected));
  color: var(--ts-text-primary);
  font-size: 0.85rem;
}
.po-field--toggle {
  flex-direction: row;
  align-items: center;
  gap: 6px;
}

/* Task grid */
.po-task-grid {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.po-task-row {
  display: grid;
  grid-template-columns: 100px 1fr 1fr auto;
  gap: 8px;
  align-items: center;
  font-size: 0.78rem;
  padding: 4px 6px;
  border-radius: 3px;
  background: var(--ts-bg-selected);
}
.po-task-kind {
  font-weight: 600;
  color: var(--ts-text-primary);
}
.po-task-provider {
  color: var(--ts-text-link, var(--ts-accent));
}
.po-task-model {
  color: var(--ts-text-secondary);
  font-family: var(--ts-mono, monospace);
  font-size: 0.75rem;
}
.po-task-source {
  font-size: 0.7rem;
  color: var(--ts-text-muted);
  text-align: right;
}

/* Events list */
.po-events {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.po-event {
  display: flex;
  gap: 8px;
  align-items: center;
  font-size: 0.78rem;
  padding: 3px 6px;
  border-radius: 3px;
  background: var(--ts-bg-selected);
}
.po-event-provider {
  font-weight: 600;
  color: var(--ts-text-primary);
  min-width: 80px;
}
.po-event-reason {
  color: var(--ts-text-secondary);
  flex: 1;
}
.po-reason--rate_limit { color: var(--ts-warning, #f0c040); }
.po-reason--unhealthy { color: var(--ts-error, #f44336); }
.po-event-time {
  font-size: 0.7rem;
  color: var(--ts-text-muted);
}

/* Alert */
.po-alert {
  padding: 8px 12px;
  border-radius: var(--ts-radius-sm, 4px);
  background: color-mix(in srgb, var(--ts-error, #f44336) 15%, var(--ts-bg-panel));
  border: 1px solid var(--ts-error, #f44336);
  color: var(--ts-error, #f44336);
  font-size: 0.82rem;
  font-weight: 600;
  text-align: center;
}
</style>
