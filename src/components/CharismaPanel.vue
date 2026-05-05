<template>
  <section class="ch-panel" aria-labelledby="ch-panel-title">
    <header class="ch-panel-header">
      <div class="ch-panel-title-row">
        <span class="ch-panel-icon" aria-hidden="true">🎭</span>
        <h2 id="ch-panel-title" class="ch-panel-title">Charisma — Teach TerranSoul</h2>
        <span class="ch-pill" :class="{ 'ch-pill-active': summary.proven > 0 }">
          {{ summary.proven }} proven
        </span>
      </div>
      <p class="ch-panel-sub">
        Manage taught persona traits, facial expressions, and body motions.
        Items used often and rated highly become <strong>Proven</strong> and
        can be promoted into source-code defaults via a multi-agent workflow.
      </p>

      <!-- Maturity dashboard -->
      <div class="ch-summary">
        <div class="ch-summary-cell ch-tier-untested">
          <span class="ch-summary-num">{{ summary.untested }}</span>
          <span class="ch-summary-lbl">Untested</span>
        </div>
        <div class="ch-summary-cell ch-tier-learning">
          <span class="ch-summary-num">{{ summary.learning }}</span>
          <span class="ch-summary-lbl">Learning</span>
        </div>
        <div class="ch-summary-cell ch-tier-proven">
          <span class="ch-summary-num">{{ summary.proven }}</span>
          <span class="ch-summary-lbl">Proven</span>
        </div>
        <div class="ch-summary-cell ch-tier-canon">
          <span class="ch-summary-num">{{ summary.canon }}</span>
          <span class="ch-summary-lbl">Canon</span>
        </div>
      </div>

      <nav class="ch-tabs" role="tablist">
        <button
          v-for="tab in tabs"
          :key="tab.id"
          role="tab"
          :aria-selected="activeTab === tab.id"
          :class="['ch-tab', { 'ch-tab-active': activeTab === tab.id }]"
          @click="activeTab = tab.id"
        >
          <span class="ch-tab-icon">{{ tab.icon }}</span>
          {{ tab.label }}
          <span class="ch-tab-count">{{ store.byKind[tab.kind].length }}</span>
        </button>
      </nav>
    </header>

    <div class="ch-tab-body">
      <p v-if="store.loading" class="ch-status">Loading…</p>
      <p v-else-if="visibleStats.length === 0" class="ch-empty">
        No {{ kindLabel(currentKind).toLowerCase() }}s taught yet.
        <span v-if="currentKind === 'expression' || currentKind === 'motion'">
          Open the <strong>Persona Teacher</strong> to capture one from
          your webcam.
        </span>
        <span v-else>
          Edit the <strong>Persona panel</strong> and rate individual
          traits to build up Charisma data.
        </span>
      </p>

      <ul v-else class="ch-list">
        <li v-for="s in visibleStats" :key="s.kind + s.asset_id" class="ch-row">
          <div class="ch-row-main">
            <span class="ch-kind-icon">{{ kindIcon(s.kind) }}</span>
            <div class="ch-row-text">
              <strong class="ch-name">{{ s.display_name }}</strong>
              <span class="ch-meta">
                Used {{ s.usage_count }}× · last
                {{ s.last_used_at > 0 ? formatRelative(s.last_used_at) : 'never' }}
              </span>
            </div>
            <span
              class="ch-maturity-badge"
              :style="{ background: maturityColor(maturityOf(s)) + '22', color: maturityColor(maturityOf(s)) }"
              :title="maturityHint(maturityOf(s))"
            >
              {{ maturityLabel(maturityOf(s)) }}
            </span>
          </div>

          <div class="ch-row-bottom">
            <!-- Rating stars -->
            <div class="ch-rating" role="radiogroup" :aria-label="`Rate ${s.display_name}`">
              <button
                v-for="n in 5"
                :key="n"
                type="button"
                role="radio"
                :aria-checked="Math.round(avg(s)) >= n"
                :class="['ch-star', { 'ch-star-on': Math.round(avg(s)) >= n }]"
                :title="`Rate ${n}/5`"
                @click="onRate(s, n)"
              >★</button>
              <span v-if="s.rating_count > 0" class="ch-rating-num">
                {{ avg(s).toFixed(1) }} ({{ s.rating_count }})
              </span>
            </div>

            <div class="ch-row-actions">
              <button
                v-if="maturityOf(s) === 'proven'"
                class="ch-btn ch-btn-promote"
                @click="onPromote(s)"
                :title="'Promote into source-code defaults via a coding workflow plan'"
              >
                ⭐ Promote to source
              </button>
              <span
                v-else-if="maturityOf(s) === 'canon'"
                class="ch-canon-stamp"
                :title="s.last_promotion_plan_id ? `Workflow plan ${s.last_promotion_plan_id}` : ''"
              >
                Canon
              </span>
              <button class="ch-btn" @click="onTest(s)" v-if="s.kind !== 'trait'">
                ▶ Test
              </button>
              <button class="ch-btn ch-btn-danger" @click="onDelete(s)">Delete</button>
            </div>
          </div>
        </li>
      </ul>

      <!-- Last promotion result -->
      <div v-if="lastPromotionPlanId" class="ch-promotion-toast">
        ✅ Created workflow plan
        <code>{{ lastPromotionPlanId }}</code>
        — open the
        <strong>Multi-agent workflows</strong> panel to review and run it.
      </div>
    </div>

    <p v-if="store.error" class="ch-error">{{ store.error }}</p>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import {
  avgRating,
  deriveMaturity,
  kindIcon,
  kindLabel,
  maturityColor,
  maturityLabel,
  type CharismaAssetKind,
  type CharismaStat,
  type Maturity,
  useCharismaStore,
} from '../stores/charisma';
import { formatRelativeTime as formatRelative } from '../utils/teaching-maturity';

const store = useCharismaStore();

type TabId = 'expression' | 'motion' | 'trait';

const tabs: { id: TabId; kind: CharismaAssetKind; label: string; icon: string }[] = [
  { id: 'expression', kind: 'expression', label: 'Expressions', icon: '😊' },
  { id: 'motion', kind: 'motion', label: 'Motions', icon: '💃' },
  { id: 'trait', kind: 'trait', label: 'Traits', icon: '📝' },
];

const activeTab = ref<TabId>('expression');
const lastPromotionPlanId = ref<string | null>(null);

const currentKind = computed<CharismaAssetKind>(() => activeTab.value);

const summary = computed(() => store.summary);

const visibleStats = computed<CharismaStat[]>(() =>
  store.byKind[currentKind.value],
);

const emit = defineEmits<{
  'preview-expression': [assetId: string];
  'preview-motion': [assetId: string];
}>();

onMounted(async () => {
  await store.load();
});

function maturityOf(s: CharismaStat): Maturity {
  return deriveMaturity(s);
}

function avg(s: CharismaStat): number {
  return avgRating(s);
}

function maturityHint(m: Maturity): string {
  switch (m) {
    case 'untested':
      return 'Never used. Mention it in chat to start collecting data.';
    case 'learning':
      return 'Used at least once. Needs ≥10 uses and avg rating ≥4 to become Proven.';
    case 'proven':
      return 'Eligible for promotion to source-code defaults.';
    case 'canon':
      return 'Already promoted into the bundled defaults.';
  }
}

async function onRate(s: CharismaStat, n: number): Promise<void> {
  await store.setRating(s.kind, s.asset_id, s.display_name, n);
}

async function onDelete(s: CharismaStat): Promise<void> {
  if (!confirm(`Delete charisma stats for "${s.display_name}"? The underlying asset stays.`)) return;
  await store.remove(s.kind, s.asset_id);
}

async function onPromote(s: CharismaStat): Promise<void> {
  lastPromotionPlanId.value = null;
  const resp = await store.promote(s.kind, s.asset_id);
  if (resp) {
    lastPromotionPlanId.value = resp.plan_id;
  }
}

function onTest(s: CharismaStat): void {
  if (s.kind === 'expression') emit('preview-expression', s.asset_id);
  else if (s.kind === 'motion') emit('preview-motion', s.asset_id);
}
</script>

<style scoped>
.ch-panel {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem;
  background: var(--ts-surface, #1a1a1a);
  color: var(--ts-text, #e5e7eb);
  border-radius: 0.75rem;
  min-height: 600px;
}
.ch-panel-header {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  border-bottom: 1px solid var(--ts-border, #333);
  padding-bottom: 0.75rem;
}
.ch-panel-title-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.ch-panel-icon { font-size: 1.5rem; }
.ch-panel-title { margin: 0; font-size: 1.25rem; }
.ch-panel-sub {
  margin: 0;
  font-size: 0.875rem;
  color: var(--ts-text-muted);
}
.ch-pill {
  margin-left: auto;
  padding: 0.125rem 0.5rem;
  border-radius: 999px;
  background: var(--ts-text-muted);
  color: var(--ts-surface);
  font-size: 0.75rem;
}
.ch-pill-active { background: var(--ts-success); }

/* Maturity dashboard */
.ch-summary {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 0.5rem;
}
.ch-summary-cell {
  padding: 0.5rem;
  background: var(--ts-surface-2, #222);
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.5rem;
  text-align: center;
}
.ch-summary-num {
  display: block;
  font-size: 1.5rem;
  font-weight: 700;
  line-height: 1.1;
}
.ch-summary-lbl {
  display: block;
  font-size: 0.6875rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--ts-text-muted);
}
.ch-tier-untested { border-color: var(--ts-text-muted); }
.ch-tier-learning { border-color: var(--ts-info); }
.ch-tier-learning .ch-summary-num { color: var(--ts-info); }
.ch-tier-proven { border-color: var(--ts-success); }
.ch-tier-proven .ch-summary-num { color: var(--ts-success); }
.ch-tier-canon { border-color: var(--ts-accent-violet); }
.ch-tier-canon .ch-summary-num { color: var(--ts-accent-violet); }

.ch-tabs {
  display: flex;
  gap: 0.25rem;
}
.ch-tab {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 0.875rem;
  background: transparent;
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.375rem;
  color: var(--ts-text-muted);
  cursor: pointer;
  font-size: 0.875rem;
}
.ch-tab:hover { color: var(--ts-text); }
.ch-tab-active {
  background: var(--ts-accent-glow);
  color: var(--ts-accent);
  border-color: var(--ts-accent);
}
.ch-tab-count {
  margin-left: 0.25rem;
  padding: 0 0.375rem;
  background: var(--ts-surface, #1a1a1a);
  border-radius: 999px;
  font-size: 0.6875rem;
}

.ch-tab-body { display: flex; flex-direction: column; gap: 0.5rem; }
.ch-status, .ch-empty {
  margin: 0;
  padding: 1.5rem;
  text-align: center;
  color: var(--ts-text-muted);
  font-style: italic;
  border: 1px dashed var(--ts-border, #333);
  border-radius: 0.5rem;
}

.ch-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.ch-row {
  background: var(--ts-surface-2, #222);
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.5rem;
  padding: 0.625rem 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.ch-row-main { display: flex; align-items: center; gap: 0.5rem; }
.ch-kind-icon { font-size: 1.25rem; }
.ch-row-text { flex: 1; display: flex; flex-direction: column; gap: 0.125rem; }
.ch-name { font-size: 0.9375rem; }
.ch-meta { font-size: 0.75rem; color: var(--ts-text-muted); }
.ch-maturity-badge {
  font-size: 0.6875rem;
  padding: 0.125rem 0.5rem;
  border-radius: 0.25rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.ch-row-bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  flex-wrap: wrap;
}
.ch-rating { display: flex; align-items: center; gap: 0.125rem; }
.ch-star {
  background: transparent;
  border: none;
  color: var(--ts-text-muted);
  font-size: 1.125rem;
  cursor: pointer;
  padding: 0 0.125rem;
  line-height: 1;
}
.ch-star-on { color: var(--ts-warning); }
.ch-rating-num {
  margin-left: 0.5rem;
  font-size: 0.75rem;
  color: var(--ts-text-muted);
}

.ch-row-actions { display: flex; gap: 0.375rem; }
.ch-btn {
  padding: 0.25rem 0.625rem;
  background: transparent;
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.375rem;
  color: var(--ts-text);
  cursor: pointer;
  font-size: 0.8125rem;
}
.ch-btn:hover:not(:disabled) { background: var(--ts-surface, #1a1a1a); }
.ch-btn-promote {
  background: var(--ts-success);
  border-color: var(--ts-success);
  color: white;
  font-weight: 600;
}
.ch-btn-promote:hover { filter: brightness(1.1); }
.ch-btn-danger {
  border-color: var(--ts-error);
  color: var(--ts-error);
}

.ch-canon-stamp {
  padding: 0.25rem 0.625rem;
  background: var(--ts-accent-violet);
  border-radius: 0.375rem;
  color: white;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.ch-promotion-toast {
  margin-top: 0.5rem;
  padding: 0.625rem;
  background: var(--ts-success-bg, rgba(52, 211, 153, 0.12));
  border: 1px solid var(--ts-success);
  border-radius: 0.5rem;
  color: var(--ts-success);
  font-size: 0.875rem;
}
.ch-promotion-toast code {
  background: rgba(0, 0, 0, 0.25);
  padding: 0.0625rem 0.375rem;
  border-radius: 0.25rem;
  font-family: monospace;
  font-size: 0.8125rem;
}

.ch-error {
  padding: 0.5rem;
  background: var(--ts-error-bg);
  color: var(--ts-error);
  border-radius: 0.375rem;
  font-size: 0.875rem;
}
</style>
