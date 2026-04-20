<template>
  <section class="brain-stat-sheet" data-testid="brain-stat-sheet">
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
          <span class="bss-stat-icon" aria-hidden="true">{{ desc.icon }}</span>
          <span class="bss-stat-abbr">{{ desc.abbr }}</span>
          <span class="bss-stat-name">{{ desc.label }}</span>
          <span class="bss-stat-value" :data-testid="`stat-value-${desc.id}`">{{ stats[desc.id] }}</span>
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
        <div class="bss-stat-desc">{{ desc.description }}</div>
      </div>
    </div>

    <!-- Active modifiers reflect Chunk 134 — show users that the stats actually
         change AI behaviour, not just decorate the panel. -->
    <div class="bss-modifiers" data-testid="bss-modifiers">
      <div class="bss-modifiers-title">⚙ Active Modifiers</div>
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
import { STAT_DESCRIPTORS, computeStats, type StatId } from '../utils/stats';
import { computeModifiers } from '../utils/stat-modifiers';

const skillTree = useSkillTreeStore();
const statDescriptors = STAT_DESCRIPTORS;

const activeSkillIds = computed(() =>
  skillTree.nodes.filter(n => skillTree.getSkillStatus(n.id) === 'active').map(n => n.id),
);

const stats = computed(() => computeStats(activeSkillIds.value));
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
  background: linear-gradient(160deg, rgba(20, 18, 40, 0.85) 0%, rgba(12, 10, 28, 0.92) 100%);
  border: 1px solid rgba(180, 160, 100, 0.25);
  border-radius: var(--ts-radius-lg, 8px);
  padding: var(--ts-space-md, 14px) var(--ts-space-lg, 18px);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.35), inset 0 0 12px rgba(124, 111, 255, 0.04);
}

.bss-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: var(--ts-space-md, 12px);
  border-bottom: 1px solid rgba(180, 160, 100, 0.15);
  padding-bottom: var(--ts-space-sm, 8px);
}
.bss-title {
  font-size: 0.95rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  color: #dcc36e;
  text-transform: uppercase;
  text-shadow: 0 0 8px rgba(220, 195, 110, 0.25);
}
.bss-level {
  font-size: 0.85rem;
  font-weight: 700;
  color: #8ec8f6;
  text-shadow: 0 0 6px rgba(142, 200, 246, 0.3);
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
  background: rgba(10, 8, 22, 0.5);
  border: 1px solid rgba(255, 255, 255, 0.04);
  transition: transform 0.2s ease, border-color 0.2s ease;
}
.bss-stat--pulse {
  animation: bss-stat-pulse 1.5s ease-out;
  border-color: rgba(220, 195, 110, 0.5);
}
@keyframes bss-stat-pulse {
  0% { transform: scale(1); box-shadow: 0 0 0 rgba(220, 195, 110, 0); }
  30% { transform: scale(1.03); box-shadow: 0 0 16px rgba(220, 195, 110, 0.5); }
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
  color: #dcc36e;
  background: rgba(220, 195, 110, 0.08);
  border: 1px solid rgba(220, 195, 110, 0.2);
  padding: 1px 5px;
  border-radius: 2px;
}
.bss-stat-name {
  font-size: 0.78rem;
  font-weight: 600;
  color: rgba(220, 220, 235, 0.85);
  flex: 1;
}
.bss-stat-value {
  font-size: 0.95rem;
  font-weight: 800;
  color: #f0e8c8;
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.6);
  font-variant-numeric: tabular-nums;
}

.bss-stat-bar {
  position: relative;
  height: 8px;
  background: rgba(255, 255, 255, 0.05);
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
  color: rgba(180, 180, 200, 0.5);
  line-height: 1.4;
}

.bss-modifiers {
  margin-top: var(--ts-space-md, 14px);
  padding-top: var(--ts-space-sm, 10px);
  border-top: 1px dashed rgba(180, 160, 100, 0.18);
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 4px 16px;
}
.bss-modifiers-title {
  grid-column: 1 / -1;
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  color: #dcc36e;
  text-transform: uppercase;
  margin-bottom: 4px;
}
.bss-modifier-row {
  display: flex;
  justify-content: space-between;
  font-size: 0.7rem;
  padding: 2px 0;
}
.bss-modifier-label { color: rgba(200, 200, 220, 0.6); }
.bss-modifier-value { color: #8ec8f6; font-weight: 600; font-variant-numeric: tabular-nums; }

@media (max-width: 640px) {
  .brain-stat-sheet { padding: 12px; }
  .bss-stats { grid-template-columns: 1fr; gap: 8px; }
  .bss-stat-desc { display: none; }
}
</style>
