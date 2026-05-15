<template>
  <section
    class="wiki-panel"
    data-testid="wiki-panel"
  >
    <header class="wiki-panel__header">
      <div>
        <p class="wiki-panel__eyebrow">
          Knowledge Wiki
        </p>
        <h2>Brain graph operations</h2>
      </div>
      <button
        type="button"
        class="wiki-panel__refresh"
        :disabled="loading"
        data-testid="wiki-panel-refresh"
        @click="refresh"
      >
        {{ loading ? 'Refreshing' : 'Refresh' }}
      </button>
    </header>

    <div
      class="wiki-panel__tabs"
      role="tablist"
      aria-label="Knowledge wiki views"
    >
      <button
        v-for="tab in tabs"
        :key="tab.key"
        type="button"
        role="tab"
        :aria-selected="activeTab === tab.key"
        :class="['wiki-panel__tab', { active: activeTab === tab.key }]"
        @click="activeTab = tab.key"
      >
        {{ tab.label }}
      </button>
    </div>

    <p
      v-if="error"
      class="wiki-panel__error"
      data-testid="wiki-panel-error"
    >
      {{ error }}
    </p>

    <div
      v-if="activeTab === 'audit'"
      class="wiki-panel__body"
      data-testid="wiki-panel-audit"
    >
      <div class="wiki-panel__metric-grid">
        <div class="wiki-panel__metric">
          <span>{{ audit?.total_memories ?? 0 }}</span>
          <small>Memories</small>
        </div>
        <div class="wiki-panel__metric">
          <span>{{ audit?.total_edges ?? 0 }}</span>
          <small>Live edges</small>
        </div>
        <div class="wiki-panel__metric">
          <span>{{ audit?.open_conflicts.length ?? 0 }}</span>
          <small>Conflicts</small>
        </div>
        <div class="wiki-panel__metric">
          <span>{{ audit?.pending_embeddings ?? 0 }}</span>
          <small>Embedding queue</small>
        </div>
      </div>
      <div class="wiki-panel__audit-row">
        <span>Orphans</span>
        <strong>{{ audit?.orphan_ids.length ?? 0 }}</strong>
      </div>
      <div class="wiki-panel__audit-row">
        <span>Stale review candidates</span>
        <strong>{{ audit?.stale_ids.length ?? 0 }}</strong>
      </div>
    </div>

    <div
      v-else-if="activeTab === 'spotlight'"
      class="wiki-panel__body"
      data-testid="wiki-panel-spotlight"
    >
      <ol
        v-if="spotlight.length"
        class="wiki-panel__list"
      >
        <li
          v-for="node in spotlight"
          :key="node.entry.id"
        >
          <strong>#{{ node.entry.id }}</strong>
          <span>{{ node.degree }} edge{{ node.degree === 1 ? '' : 's' }}</span>
          <p>{{ compactMemory(node.entry) }}</p>
        </li>
      </ol>
      <p
        v-else
        class="wiki-panel__empty"
      >
        No connected memories found yet.
      </p>
    </div>

    <div
      v-else-if="activeTab === 'serendipity'"
      class="wiki-panel__body"
      data-testid="wiki-panel-serendipity"
    >
      <ol
        v-if="serendipity.length"
        class="wiki-panel__list wiki-panel__list--connections"
      >
        <li
          v-for="item in serendipity"
          :key="item.edge.id"
        >
          <strong>#{{ item.src.id }} to #{{ item.dst.id }}</strong>
          <span>{{ item.edge.rel_type }} · {{ Math.round(item.edge.confidence * 100) }}% · {{ item.label }}</span>
          <p>{{ compactMemory(item.src, 72) }}</p>
          <p>{{ compactMemory(item.dst, 72) }}</p>
        </li>
      </ol>
      <p
        v-else
        class="wiki-panel__empty"
      >
        No cross-topic links found yet.
      </p>
    </div>

    <div
      v-else
      class="wiki-panel__body"
      data-testid="wiki-panel-revisit"
    >
      <ol
        v-if="revisit.length"
        class="wiki-panel__list"
      >
        <li
          v-for="item in revisit"
          :key="item.entry.id"
        >
          <strong>#{{ item.entry.id }}</strong>
          <span>gravity {{ item.gravity.toFixed(2) }}</span>
          <p>{{ compactMemory(item.entry) }}</p>
        </li>
      </ol>
      <p
        v-else
        class="wiki-panel__empty"
      >
        No memories are ready for review right now.
      </p>
    </div>
  </section>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { onMounted, ref } from 'vue';
import type { MemoryEdge, MemoryEntry } from '../types';

type WikiTab = 'audit' | 'spotlight' | 'serendipity' | 'revisit';

interface BrainWikiAuditReport {
  open_conflicts: unknown[];
  orphan_ids: number[];
  stale_ids: number[];
  pending_embeddings: number;
  total_memories: number;
  total_edges: number;
  generated_at: number;
}

interface BrainWikiGodNode {
  entry: MemoryEntry;
  degree: number;
}

interface BrainWikiSurprisingConnection {
  edge: MemoryEdge;
  src: MemoryEntry;
  dst: MemoryEntry;
  label: string;
}

interface BrainWikiReviewItem {
  entry: MemoryEntry;
  gravity: number;
}

const tabs: Array<{ key: WikiTab; label: string }> = [
  { key: 'audit', label: 'Audit' },
  { key: 'spotlight', label: 'Spotlight' },
  { key: 'serendipity', label: 'Serendipity' },
  { key: 'revisit', label: 'Revisit' },
];

const activeTab = ref<WikiTab>('audit');
const loading = ref(false);
const error = ref('');
const audit = ref<BrainWikiAuditReport | null>(null);
const spotlight = ref<BrainWikiGodNode[]>([]);
const serendipity = ref<BrainWikiSurprisingConnection[]>([]);
const revisit = ref<BrainWikiReviewItem[]>([]);

function compactMemory(entry: MemoryEntry, max = 120): string {
  const text = entry.content.replace(/\s+/g, ' ').trim();
  return text.length > max ? `${text.slice(0, max - 1)}...` : text;
}

async function refresh() {
  loading.value = true;
  error.value = '';
  try {
    const [auditResult, spotlightResult, serendipityResult, revisitResult] = await Promise.all([
      invoke<BrainWikiAuditReport>('brain_wiki_audit', { limit: 50 }),
      invoke<BrainWikiGodNode[]>('brain_wiki_spotlight', { limit: 10 }),
      invoke<BrainWikiSurprisingConnection[]>('brain_wiki_serendipity', { limit: 10 }),
      invoke<BrainWikiReviewItem[]>('brain_wiki_revisit', { limit: 12 }),
    ]);
    audit.value = auditResult;
    spotlight.value = spotlightResult;
    serendipity.value = serendipityResult;
    revisit.value = revisitResult;
  } catch (err) {
    error.value = `Knowledge wiki refresh failed: ${String(err)}`;
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);

defineExpose({ refresh, activeTab });
</script>

<style scoped>
.wiki-panel {
  border: 1px solid var(--ts-glass-border);
  border-radius: 12px;
  background: var(--ts-glass-bg, rgba(15, 15, 30, 0.7));
  box-shadow: var(--ts-shadow-inset, inset 0 1px 0 rgba(255, 255, 255, 0.08)), var(--ts-shadow-glow, 0 16px 40px rgba(0, 0, 0, 0.22));
  backdrop-filter: blur(var(--ts-glass-blur, 12px)) saturate(130%);
  padding: 1rem;
  display: grid;
  gap: 0.9rem;
}

.wiki-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.wiki-panel__eyebrow {
  margin: 0 0 0.2rem;
  color: var(--ts-text-muted);
  font-size: 0.78rem;
  text-transform: uppercase;
  letter-spacing: 0;
}

.wiki-panel h2 {
  margin: 0;
  font-size: 1.1rem;
  color: var(--ts-text);
}

.wiki-panel__refresh,
.wiki-panel__tab {
  border: 1px solid var(--ts-glass-border);
  background: rgba(255, 255, 255, 0.06);
  color: var(--ts-text, #f0f0f0);
  border-radius: 8px;
  cursor: pointer;
  transition: transform var(--ts-transition-spring), border-color 0.2s ease, background 0.2s ease;
}

.wiki-panel__refresh {
  min-height: 2rem;
  padding: 0 0.8rem;
}

.wiki-panel__refresh:disabled {
  cursor: wait;
  opacity: 0.7;
}

.wiki-panel__tabs {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.4rem;
}

.wiki-panel__tab {
  min-height: 2.1rem;
  padding: 0 0.5rem;
  font-size: 0.88rem;
}

.wiki-panel__tab.active {
  border-color: var(--ts-accent);
  background: color-mix(in srgb, var(--ts-accent, #7c6fff) 18%, rgba(255, 255, 255, 0.06));
}

.wiki-panel__refresh:hover:not(:disabled),
.wiki-panel__tab:hover {
  transform: translateY(-1px);
  border-color: var(--ts-accent);
}

.wiki-panel__body {
  min-height: 8rem;
}

.wiki-panel__metric-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.5rem;
}

.wiki-panel__metric {
  border: 1px solid var(--ts-glass-border);
  border-radius: 8px;
  background: color-mix(in srgb, rgba(255, 255, 255, 0.06) 78%, transparent);
  padding: 0.65rem;
}

.wiki-panel__metric span {
  display: block;
  color: var(--ts-text);
  font-weight: 700;
  font-size: 1.1rem;
}

.wiki-panel__metric small,
.wiki-panel__list span,
.wiki-panel__empty,
.wiki-panel__audit-row span {
  color: var(--ts-text-muted);
}

.wiki-panel__audit-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--ts-glass-border);
  padding: 0.65rem 0;
}

.wiki-panel__list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 0.55rem;
}

.wiki-panel__list li {
  border: 1px solid var(--ts-glass-border);
  border-radius: 8px;
  background: color-mix(in srgb, rgba(255, 255, 255, 0.06) 72%, transparent);
  padding: 0.7rem;
  display: grid;
  gap: 0.25rem;
}

.wiki-panel__list strong {
  color: var(--ts-text);
}

.wiki-panel__list p {
  margin: 0;
  color: var(--ts-text);
  line-height: 1.4;
}

.wiki-panel__list--connections li {
  gap: 0.35rem;
}

.wiki-panel__empty,
.wiki-panel__error {
  margin: 0;
  padding: 0.75rem;
  border-radius: 8px;
}

.wiki-panel__empty {
  background: color-mix(in srgb, rgba(255, 255, 255, 0.06) 70%, transparent);
}

.wiki-panel__error {
  color: var(--ts-error, #f87171);
  background: var(--ts-error-bg, rgba(248, 113, 113, 0.12));
}

@media (max-width: 640px) {
  .wiki-panel__header {
    align-items: stretch;
    flex-direction: column;
  }

  .wiki-panel__tabs,
  .wiki-panel__metric-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
