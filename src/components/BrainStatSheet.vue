<template>
  <section
    class="brain-stat-sheet"
    data-testid="brain-stat-sheet"
  >
    <div class="bss-header">
      <span class="bss-title">⚔ Brain Stat Sheet</span>
      <span class="bss-level">Lv. {{ overallLevel }}</span>
    </div>

    <div class="bss-stats">
      <div
        v-for="desc in statDescriptors"
        :key="desc.id"
        class="bss-stat"
        :class="{ 'bss-stat--pulse': pulsingStats.includes(desc.id) }"
        :data-testid="`stat-${desc.id}`"
      >
        <div class="bss-stat-label-row">
          <span
            class="bss-stat-icon"
            aria-hidden="true"
          >{{ desc.icon }}</span>
          <span class="bss-stat-abbr">{{ desc.abbr }}</span>
          <span class="bss-stat-name">{{ desc.label }}</span>
          <span
            class="bss-stat-value"
            :data-testid="`stat-value-${desc.id}`"
          >{{ stats[desc.id] }}</span>
        </div>
        <div class="bss-stat-bar">
          <div
            class="bss-stat-bar-fill"
            :style="{
              width: stats[desc.id] + '%',
              background: `linear-gradient(90deg, ${desc.color}55, ${desc.color})`,
              boxShadow: `0 0 8px ${desc.color}66`,
            }"
          />
          <div class="bss-stat-bar-shimmer" />
        </div>
        <div class="bss-stat-desc">
          {{ desc.description }}
        </div>
      </div>
    </div>

    <!-- Active modifiers reflect Chunk 134 — show users that the stats actually
         change AI behaviour, not just decorate the panel. -->
    <div
      class="bss-modifiers"
      data-testid="bss-modifiers"
    >
      <div class="bss-modifiers-title">
        ⚙ Active Modifiers
      </div>
      <div class="bss-modifier-row">
        <span class="bss-modifier-label">Memory recall depth</span>
        <span class="bss-modifier-value">{{ modifiers.memoryRecallLimit }} fragments</span>
      </div>
      <div class="bss-modifier-row">
        <span class="bss-modifier-label">Chat history kept</span>
        <span class="bss-modifier-value">{{ modifiers.chatHistoryLimit }} turns ({{ formatMultiplier(modifiers.contextWindowMultiplier) }})</span>
      </div>
      <div class="bss-modifier-row">
        <span class="bss-modifier-label">Hotword sensitivity</span>
        <span class="bss-modifier-value">{{ formatMultiplier(modifiers.hotwordSensitivity) }}</span>
      </div>
      <div class="bss-modifier-row">
        <span class="bss-modifier-label">TTS expressiveness</span>
        <span class="bss-modifier-value">{{ Math.round(modifiers.ttsExpressiveness * 100) }}%</span>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useSkillTreeStore } from '../stores/skill-tree';
import { useBrainStore } from '../stores/brain';
import { STAT_DESCRIPTORS, computeStats, type StatId, type StatSnapshot } from '../utils/stats';
import { computeModifiers } from '../utils/stat-modifiers';
import { getModelBoost } from '../utils/model-benchmarks';

const skillTree = useSkillTreeStore();
const brain = useBrainStore();
const statDescriptors = STAT_DESCRIPTORS;

const activeSkillIds = computed(() =>
  skillTree.nodes.filter(n => skillTree.getSkillStatus(n.id) === 'active').map(n => n.id),
);

/**
 * Per-stat boost contributed by the *currently selected* AI model.
 *
 * Reads `brainMode` + `freeProviders` from the brain store and resolves the
 * model identifier (e.g. `claude-opus-4.7`, `gemma3:1b`, `llama-3.3-70b`)
 * against the benchmark table. Falls back to an empty object when the brain
 * isn't configured yet, so a fresh install reads the bare baseline.
 */
const brainBoost = computed<Partial<StatSnapshot>>(() => {
  const mode = brain.brainMode;
  if (!mode) return {};
  if (mode.mode === 'free_api') {
    const provider = brain.freeProviders.find(p => p.id === mode.provider_id);
    return getModelBoost(provider?.model);
  }
  if (mode.mode === 'paid_api' || mode.mode === 'local_ollama') {
    return getModelBoost(mode.model);
  }
  return {};
});

const stats = computed(() => computeStats(activeSkillIds.value, brainBoost.value));
const modifiers = computed(() => computeModifiers(stats.value));

/** "Level" is just the rounded average of all six stats — easy to read. */
const overallLevel = computed(() => {
  const values = Object.values(stats.value);
  const total = values.reduce((sum, v) => sum + v, 0);
  return Math.max(1, Math.round(total / values.length));
});

const pulsingStats = ref<StatId[]>([]);

// Pulse a stat bar briefly whenever its value increases.
watch(stats, (next, prev) => {
  if (!prev) return;
  const newlyPulsing: StatId[] = [];
  for (const desc of STAT_DESCRIPTORS) {
    if (next[desc.id] > prev[desc.id]) newlyPulsing.push(desc.id);
  }
  if (newlyPulsing.length === 0) return;
  pulsingStats.value = [...pulsingStats.value, ...newlyPulsing];
  setTimeout(() => {
    pulsingStats.value = pulsingStats.value.filter(id => !newlyPulsing.includes(id));
  }, 1500);
}, { deep: true });

function formatMultiplier(value: number): string {
  return `${value.toFixed(2)}×`;
}
</script>

<style scoped>
.brain-stat-sheet {
  background: var(--ts-quest-bg, linear-gradient(160deg, rgba(20, 18, 40, 0.85) 0%, rgba(12, 10, 28, 0.92) 100%));
  border: 1px solid var(--ts-quest-border);
  border-radius: var(--ts-radius-lg, 8px);
  padding: var(--ts-space-md, 14px) var(--ts-space-lg, 18px);
  box-shadow: var(--ts-shadow-md);
}

.bss-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: var(--ts-space-md, 12px);
  border-bottom: 1px solid var(--ts-quest-border);
  padding-bottom: var(--ts-space-sm, 8px);
}
.bss-title {
  font-size: 0.95rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  color: var(--ts-quest-gold);
  text-transform: uppercase;
  text-shadow: 0 0 8px var(--ts-quest-gold-dim);
}
.bss-level {
  font-size: 0.85rem;
  font-weight: 700;
  color: var(--ts-text-link, #8ec8f6);
  text-shadow: 0 0 6px var(--ts-accent-glow);
}

.bss-stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: var(--ts-space-md, 14px);
}

.bss-stat {
  position: relative;
  padding: 8px 10px;
  border-radius: 4px;
  background: var(--ts-bg-panel);
  border: 1px solid var(--ts-border-subtle);
  transition: transform 0.2s ease, border-color 0.2s ease;
}
.bss-stat--pulse {
  animation: bss-stat-pulse 1.5s ease-out;
  border-color: var(--ts-quest-gold-glow);
}
@keyframes bss-stat-pulse {
  0% { transform: scale(1); box-shadow: 0 0 0 rgba(220, 195, 110, 0); }
  30% { transform: scale(1.03); box-shadow: 0 0 16px var(--ts-quest-gold-glow); }
  100% { transform: scale(1); box-shadow: 0 0 0 rgba(220, 195, 110, 0); }
}

.bss-stat-label-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}
.bss-stat-icon { font-size: 0.95rem; line-height: 1; }
.bss-stat-abbr {
  font-size: 0.65rem;
  font-weight: 700;
  letter-spacing: 0.1em;
  color: var(--ts-quest-gold);
  background: var(--ts-quest-gold-dim);
  border: 1px solid var(--ts-quest-gold-dim);
  padding: 1px 5px;
  border-radius: 2px;
}
.bss-stat-name {
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--ts-text-primary);
  flex: 1;
}
.bss-stat-value {
  font-size: 0.95rem;
  font-weight: 800;
  color: var(--ts-quest-text);
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.6);
  font-variant-numeric: tabular-nums;
}

.bss-stat-bar {
  position: relative;
  height: 8px;
  background: var(--ts-bg-input);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 4px;
}
.bss-stat-bar-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
}
.bss-stat-bar-shimmer {
  position: absolute;
  top: 0;
  left: -50%;
  width: 50%;
  height: 100%;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.15), transparent);
  animation: bss-shimmer 4s ease-in-out infinite;
  pointer-events: none;
}
@keyframes bss-shimmer {
  0% { left: -50%; }
  60%, 100% { left: 110%; }
}

.bss-stat-desc {
  font-size: 0.65rem;
  color: var(--ts-quest-muted);
  line-height: 1.4;
}

.bss-modifiers {
  margin-top: var(--ts-space-md, 14px);
  padding-top: var(--ts-space-sm, 10px);
  border-top: 1px dashed var(--ts-quest-border);
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 4px 16px;
}
.bss-modifiers-title {
  grid-column: 1 / -1;
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  color: var(--ts-quest-gold);
  text-transform: uppercase;
  margin-bottom: 4px;
}
.bss-modifier-row {
  display: flex;
  justify-content: space-between;
  font-size: 0.7rem;
  padding: 2px 0;
}
.bss-modifier-label { color: var(--ts-text-secondary); }
.bss-modifier-value { color: var(--ts-text-link, #8ec8f6); font-weight: 600; font-variant-numeric: tabular-nums; }

@media (max-width: 640px) {
  .brain-stat-sheet { padding: 12px; }
  .bss-stats { grid-template-columns: 1fr; gap: 8px; }
  .bss-stat-desc { display: none; }
}
</style>
