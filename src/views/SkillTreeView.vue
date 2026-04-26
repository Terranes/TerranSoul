<template>
  <div class="skill-tree-view">
    <!-- ── Progress Header ── -->
    <header class="st-header">
      <div class="st-title-row">
        <h2 class="st-title">
          ⚔️ Skill Tree
        </h2>
        <div class="st-progress-badge">
          <span class="st-progress-num">{{ skillTree.activeCount }}</span>
          <span class="st-progress-sep">/</span>
          <span class="st-progress-total">{{ skillTree.totalNodes }}</span>
          <span class="st-progress-label">unlocked</span>
        </div>
      </div>
      <div class="st-progress-bar-track">
        <div
          class="st-progress-bar-fill"
          :style="{ width: skillTree.progressPercent + '%' }"
        />
      </div>
    </header>

    <!-- ── Brain RPG Stat Sheet (Chunk 130) ── -->
    <BrainStatSheet />

    <!-- ── Daily Quests Banner ── -->
    <section
      v-if="dailySuggestions.length > 0"
      class="st-daily-section"
    >
      <div class="st-daily-header">
        <span class="st-daily-icon">📋</span>
        <span class="st-daily-title">Today's Quests</span>
        <button
          class="st-daily-refresh"
          :disabled="skillTree.isLoadingSuggestions"
          title="Ask AI for new suggestions"
          @click="skillTree.refreshDailySuggestions()"
        >
          {{ skillTree.isLoadingSuggestions ? '⏳' : '🔄' }}
        </button>
      </div>
      <p
        v-if="skillTree.tracker.dailySuggestionReason"
        class="st-daily-reason"
      >
        {{ skillTree.tracker.dailySuggestionReason }}
      </p>
      <div class="st-daily-cards">
        <button
          v-for="suggestion in dailySuggestions"
          :key="suggestion.node.id"
          class="st-daily-card"
          :class="'st-status-' + suggestion.status"
          @click="skillTree.openQuest(suggestion.node.id)"
        >
          <span class="st-daily-card-icon">{{ suggestion.node.icon }}</span>
          <div class="st-daily-card-body">
            <span class="st-daily-card-name">{{ suggestion.node.name }}</span>
            <span class="st-daily-card-tagline">{{ suggestion.node.tagline }}</span>
          </div>
          <span
            v-if="suggestion.status === 'active'"
            class="st-daily-card-badge st-badge-active"
          >✓</span>
          <span
            v-else-if="suggestion.status === 'available'"
            class="st-daily-card-badge st-badge-available"
          >!</span>
          <span
            v-else
            class="st-daily-card-badge st-badge-locked"
          >🔒</span>
        </button>
      </div>
    </section>

    <!-- ── Pinned Quests ── -->
    <section
      v-if="skillTree.pinnedQuests.length > 0"
      class="st-pinned-section"
    >
      <div class="st-section-header">
        <span>📌 Pinned Quests</span>
      </div>
      <div class="st-pinned-list">
        <div
          v-for="node in skillTree.pinnedQuests"
          :key="node.id"
          class="st-pinned-item"
          :class="'st-status-' + skillTree.getSkillStatus(node.id)"
          role="button"
          tabindex="0"
          @click="skillTree.openQuest(node.id)"
          @keydown.enter="skillTree.openQuest(node.id)"
        >
          <span class="st-pinned-icon">{{ node.icon }}</span>
          <span class="st-pinned-name">{{ node.name }}</span>
          <button
            class="st-unpin-btn"
            title="Unpin"
            @click.stop="skillTree.unpinQuest(node.id)"
          >
            ✕
          </button>
        </div>
      </div>
    </section>

    <!-- ── Active Combos Banner ── -->
    <section
      v-if="skillTree.activeCombos.length > 0"
      class="st-combos-section"
    >
      <div class="st-section-header">
        <span>🔥 Active Combos</span>
      </div>
      <div class="st-combo-list">
        <div
          v-for="c in skillTree.activeCombos"
          :key="c.combo.name"
          class="st-combo-card"
        >
          <span class="st-combo-icon">{{ c.combo.icon }}</span>
          <div class="st-combo-body">
            <span class="st-combo-name">{{ c.combo.name }}</span>
            <span class="st-combo-desc">{{ c.combo.description }}</span>
          </div>
        </div>
      </div>
    </section>

    <!-- ── Quest Tracker (management overview) ── -->
    <section class="st-tracker-section">
      <div class="st-section-header">
        <span>🗺️ Quest Tracker</span>
        <span class="st-tracker-summary">{{ trackerActiveCount }} active · {{ trackerAvailableCount }} available · {{ trackerLockedCount }} locked</span>
      </div>
      <div class="st-tracker-list">
        <div
          v-for="entry in trackerEntries"
          :key="entry.node.id"
          class="st-tracker-row"
          :class="'st-status-' + entry.status"
          @click="skillTree.openQuest(entry.node.id)"
        >
          <span class="st-tracker-status-icon">{{ entry.statusIcon }}</span>
          <span class="st-tracker-icon">{{ entry.node.icon }}</span>
          <div class="st-tracker-info">
            <span class="st-tracker-name">{{ entry.node.name }}</span>
            <span class="st-tracker-tagline">{{ entry.node.tagline }}</span>
          </div>
          <span
            v-if="entry.status === 'active' && entry.activatedAt"
            class="st-tracker-time"
          >{{ formatActivation(entry.activatedAt) }}</span>
          <span
            v-if="skillTree.tracker.pinnedQuestIds.includes(entry.node.id)"
            class="st-tracker-pin"
            title="Pinned"
          >📌</span>
        </div>
      </div>
    </section>

    <!-- ── Tech Tree by Tier ── -->
    <section
      v-for="tier in tiers"
      :key="tier.id"
      class="st-tier-section"
    >
      <div class="st-tier-header">
        <span class="st-tier-icon">{{ tier.icon }}</span>
        <span class="st-tier-name">{{ tier.label }}</span>
        <span class="st-tier-count">{{ tierActiveCount(tier.id) }}/{{ tierNodes(tier.id).length }}</span>
      </div>
      <div class="st-tier-grid">
        <button
          v-for="node in tierNodes(tier.id)"
          :key="node.id"
          class="st-node"
          :class="['st-status-' + skillTree.getSkillStatus(node.id), 'st-cat-' + node.category]"
          @click="skillTree.openQuest(node.id)"
        >
          <div class="st-node-icon-wrap">
            <span class="st-node-icon">{{ node.icon }}</span>
            <span
              v-if="skillTree.getSkillStatus(node.id) === 'active'"
              class="st-node-check"
            >✓</span>
          </div>
          <span class="st-node-name">{{ node.name }}</span>
          <span class="st-node-tagline">{{ node.tagline }}</span>
          <div
            v-if="node.combos.length > 0"
            class="st-node-combo-hint"
          >
            <span
              v-for="combo in node.combos"
              :key="combo.name"
              class="st-combo-pip"
              :title="combo.name"
            >
              {{ combo.icon }}
            </span>
          </div>
        </button>
      </div>
    </section>

    <!-- ── Quest Detail Dialog ── -->
    <QuestDialog
      v-if="activeQuest"
      :node="activeQuest"
      :status="skillTree.getSkillStatus(activeQuest.id)"
      :is-pinned="skillTree.tracker.pinnedQuestIds.includes(activeQuest.id)"
      :is-dismissed="skillTree.tracker.dismissedQuestIds.includes(activeQuest.id)"
      :activation-time="skillTree.tracker.activationTimestamps[activeQuest.id] ?? null"
      @close="skillTree.closeQuest()"
      @pin="skillTree.pinQuest(activeQuest!.id)"
      @unpin="skillTree.unpinQuest(activeQuest!.id)"
      @dismiss="skillTree.dismissQuest(activeQuest!.id); skillTree.closeQuest()"
      @navigate="handleNavigate"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useSkillTreeStore, type SkillTier, type SkillNode } from '../stores/skill-tree';
import QuestDialog from '../components/QuestDialog.vue';
import BrainStatSheet from '../components/BrainStatSheet.vue';

const emit = defineEmits<{
  navigate: [target: string];
}>();

const skillTree = useSkillTreeStore();

const tiers: { id: SkillTier; label: string; icon: string }[] = [
  { id: 'foundation', label: 'Foundation', icon: '🏗️' },
  { id: 'advanced', label: 'Advanced', icon: '⚡' },
  { id: 'ultimate', label: 'Ultimate', icon: '👑' },
];

const activeQuest = computed(() =>
  skillTree.activeQuestId
    ? skillTree.nodes.find(n => n.id === skillTree.activeQuestId) ?? null
    : null,
);

const dailySuggestions = computed(() => skillTree.dailySuggestions);

function tierNodes(tier: SkillTier) {
  return skillTree.nodes.filter(n => n.tier === tier);
}

function tierActiveCount(tier: SkillTier) {
  return tierNodes(tier).filter(n => skillTree.getSkillStatus(n.id) === 'active').length;
}

function handleNavigate(target: string) {
  emit('navigate', target);
  skillTree.closeQuest();
}

// ── Quest tracker computed ─────────────────
interface TrackerEntry {
  node: SkillNode;
  status: string;
  statusIcon: string;
  activatedAt: number | null;
}

const trackerEntries = computed<TrackerEntry[]>(() => {
  const statusOrder: Record<string, number> = { active: 0, available: 1, locked: 2 };
  return skillTree.nodes
    .map(node => {
      const status = skillTree.getSkillStatus(node.id);
      const statusIcon = status === 'active' ? '✅' : status === 'available' ? '🟡' : '🔒';
      return {
        node,
        status,
        statusIcon,
        activatedAt: skillTree.tracker.activationTimestamps[node.id] ?? null,
      };
    })
    .sort((a, b) => (statusOrder[a.status] ?? 9) - (statusOrder[b.status] ?? 9));
});

const trackerActiveCount = computed(() => trackerEntries.value.filter(e => e.status === 'active').length);
const trackerAvailableCount = computed(() => trackerEntries.value.filter(e => e.status === 'available').length);
const trackerLockedCount = computed(() => trackerEntries.value.filter(e => e.status === 'locked').length);

function formatActivation(ts: number): string {
  const date = new Date(ts);
  return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
}
</script>

<style scoped>
.skill-tree-view {
  padding: var(--ts-space-lg);
  max-width: 800px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-lg);
  overflow-y: auto;
  height: 100%;
  scrollbar-width: thin;
  scrollbar-color: var(--ts-accent-glow) transparent;
}

/* ── Header ── */
.st-header { display: flex; flex-direction: column; gap: var(--ts-space-sm); }
.st-title-row { display: flex; align-items: center; justify-content: space-between; }
.st-title { margin: 0; font-size: 1.3rem; color: var(--ts-text-primary); }
.st-progress-badge {
  display: flex; align-items: baseline; gap: 3px;
  padding: 4px 12px; border-radius: var(--ts-radius-pill);
  background: var(--ts-accent-glow); font-size: 0.85rem;
}
.st-progress-num { color: var(--ts-accent); font-weight: 700; font-size: 1.1rem; }
.st-progress-sep { color: var(--ts-text-muted); }
.st-progress-total { color: var(--ts-text-secondary); }
.st-progress-label { color: var(--ts-text-muted); font-size: 0.75rem; margin-left: 4px; }
.st-progress-bar-track {
  height: 6px; border-radius: 3px; background: var(--ts-bg-surface);
  overflow: hidden;
}
.st-progress-bar-fill {
  height: 100%; border-radius: 3px;
  background: linear-gradient(90deg, var(--ts-accent), var(--ts-accent-violet));
  transition: width 0.6s ease;
}

/* ── Daily Quests ── */
.st-daily-section {
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-accent-glow);
  border-radius: var(--ts-radius-lg);
  padding: var(--ts-space-md);
}
.st-daily-header { display: flex; align-items: center; gap: var(--ts-space-sm); margin-bottom: var(--ts-space-sm); }
.st-daily-icon { font-size: 1.1rem; }
.st-daily-title { font-weight: 600; color: var(--ts-text-primary); font-size: 0.9rem; flex: 1; }
.st-daily-refresh {
  background: none; border: 1px solid var(--ts-border); border-radius: var(--ts-radius-sm);
  padding: 2px 8px; font-size: 0.85rem; cursor: pointer; color: var(--ts-text-secondary);
  transition: background var(--ts-transition-fast);
}
.st-daily-refresh:hover { background: var(--ts-bg-hover); }
.st-daily-refresh:disabled { opacity: 0.5; cursor: default; }
.st-daily-reason {
  margin: 0 0 var(--ts-space-sm);
  font-size: 0.78rem; color: var(--ts-text-muted); font-style: italic;
}
.st-daily-cards { display: flex; flex-direction: column; gap: var(--ts-space-xs); }
.st-daily-card {
  display: flex; align-items: center; gap: var(--ts-space-sm);
  padding: var(--ts-space-sm) var(--ts-space-md);
  border: 1px solid var(--ts-border-subtle); border-radius: var(--ts-radius-md);
  background: transparent; cursor: pointer; text-align: left;
  transition: background var(--ts-transition-fast), border-color var(--ts-transition-fast);
}
.st-daily-card:hover { background: var(--ts-bg-hover); border-color: var(--ts-border); }
.st-daily-card-icon { font-size: 1.3rem; }
.st-daily-card-body { display: flex; flex-direction: column; flex: 1; min-width: 0; }
.st-daily-card-name { font-size: 0.85rem; font-weight: 600; color: var(--ts-text-primary); }
.st-daily-card-tagline { font-size: 0.75rem; color: var(--ts-text-muted); }
.st-daily-card-badge { font-size: 0.75rem; font-weight: 700; }
.st-badge-active { color: var(--ts-success); }
.st-badge-available { color: var(--ts-warning); }
.st-badge-locked { opacity: 0.5; }

/* ── Pinned & Combos ── */
.st-section-header { font-size: 0.9rem; font-weight: 600; color: var(--ts-text-primary); margin-bottom: var(--ts-space-sm); }
.st-pinned-section, .st-combos-section { display: flex; flex-direction: column; }
.st-pinned-list { display: flex; flex-wrap: wrap; gap: var(--ts-space-xs); }
.st-pinned-item {
  display: flex; align-items: center; gap: 6px;
  padding: 4px 10px; border: 1px solid var(--ts-border-subtle);
  border-radius: var(--ts-radius-pill); background: transparent;
  cursor: pointer; font-size: 0.8rem; color: var(--ts-text-secondary);
  transition: background var(--ts-transition-fast);
}
.st-pinned-item:hover { background: var(--ts-bg-hover); }
.st-pinned-icon { font-size: 1rem; }
.st-pinned-name { font-weight: 500; }
.st-unpin-btn {
  background: none; border: none; color: var(--ts-text-muted);
  cursor: pointer; font-size: 0.7rem; padding: 0 2px;
  transition: color var(--ts-transition-fast);
}
.st-unpin-btn:hover { color: var(--ts-error); }

.st-combo-list { display: flex; flex-direction: column; gap: var(--ts-space-xs); }
.st-combo-card {
  display: flex; align-items: center; gap: var(--ts-space-sm);
  padding: var(--ts-space-sm) var(--ts-space-md);
  border-radius: var(--ts-radius-md);
  background: linear-gradient(135deg, var(--ts-success-bg), var(--ts-accent-glow));
  border: 1px solid var(--ts-success);
}
.st-combo-icon { font-size: 1.2rem; }
.st-combo-body { display: flex; flex-direction: column; }
.st-combo-name { font-size: 0.85rem; font-weight: 600; color: var(--ts-success); }
.st-combo-desc { font-size: 0.75rem; color: var(--ts-text-muted); }

/* ── Tier Sections ── */
.st-tier-section { display: flex; flex-direction: column; gap: var(--ts-space-sm); }
.st-tier-header {
  display: flex; align-items: center; gap: var(--ts-space-sm);
  padding-bottom: var(--ts-space-xs);
  border-bottom: 1px solid var(--ts-border-subtle);
}
.st-tier-icon { font-size: 1.1rem; }
.st-tier-name { font-size: 0.95rem; font-weight: 700; color: var(--ts-text-primary); flex: 1; }
.st-tier-count { font-size: 0.8rem; color: var(--ts-text-muted); }

.st-tier-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: var(--ts-space-sm);
}

/* ── Skill Nodes ── */
.st-node {
  display: flex; flex-direction: column; align-items: center; text-align: center;
  gap: var(--ts-space-xs); padding: var(--ts-space-md);
  border: 1px solid var(--ts-border-subtle); border-radius: var(--ts-radius-lg);
  background: var(--ts-bg-surface); cursor: pointer;
  transition: transform var(--ts-transition-fast), border-color var(--ts-transition-fast),
    box-shadow var(--ts-transition-fast), background var(--ts-transition-fast);
  position: relative;
}
.st-node:hover { transform: translateY(-2px); box-shadow: var(--ts-shadow-md); }

/* Status styles */
.st-status-active {
  border-color: var(--ts-success) !important;
  background: linear-gradient(180deg, var(--ts-success-bg), var(--ts-bg-surface)) !important;
}
.st-status-active .st-node-name { color: var(--ts-success); }
.st-status-available {
  border-color: var(--ts-warning) !important;
}
.st-status-available .st-node-icon-wrap { animation: pulse-glow 2s infinite; }
.st-status-locked { opacity: 0.5; }
.st-status-locked .st-node-icon { filter: grayscale(1); }

@keyframes pulse-glow {
  0%, 100% { box-shadow: 0 0 0 0 transparent; }
  50% { box-shadow: 0 0 12px 2px var(--ts-accent-glow); }
}

.st-node-icon-wrap {
  width: 48px; height: 48px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  background: var(--ts-bg-card); border: 2px solid var(--ts-border-subtle);
  position: relative;
}
.st-node-icon { font-size: 1.5rem; }
.st-node-check {
  position: absolute; bottom: -2px; right: -2px;
  width: 18px; height: 18px; border-radius: 50%;
  background: var(--ts-success); color: var(--ts-text-on-accent);
  font-size: 0.65rem; font-weight: 700;
  display: flex; align-items: center; justify-content: center;
}
.st-node-name { font-size: 0.82rem; font-weight: 600; color: var(--ts-text-primary); }
.st-node-tagline { font-size: 0.7rem; color: var(--ts-text-muted); line-height: 1.2; }
.st-node-combo-hint { display: flex; gap: 2px; margin-top: 2px; }
.st-combo-pip { font-size: 0.75rem; }

/* ── Mobile ── */
@media (max-width: 640px) {
  .skill-tree-view { padding: var(--ts-space-md); padding-top: 52px; }
  .st-tier-grid { grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); }
}

/* ── Quest Tracker ── */
.st-tracker-section {
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-accent-glow);
  border-radius: var(--ts-radius-lg);
  padding: var(--ts-space-md);
}
.st-tracker-summary {
  font-size: 0.72rem;
  color: var(--ts-text-muted);
  margin-left: auto;
}
.st-tracker-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: var(--ts-space-sm);
}
.st-tracker-row {
  display: flex;
  align-items: center;
  gap: var(--ts-space-sm);
  padding: 6px var(--ts-space-sm);
  border-radius: var(--ts-radius-sm);
  cursor: pointer;
  transition: background var(--ts-transition-fast);
}
.st-tracker-row:hover { background: var(--ts-bg-hover); }
.st-tracker-status-icon { font-size: 0.85rem; width: 20px; text-align: center; flex-shrink: 0; }
.st-tracker-icon { font-size: 1.1rem; flex-shrink: 0; }
.st-tracker-info { display: flex; flex-direction: column; flex: 1; min-width: 0; }
.st-tracker-name { font-size: 0.82rem; font-weight: 600; color: var(--ts-text-primary); }
.st-tracker-tagline { font-size: 0.7rem; color: var(--ts-text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.st-tracker-time { font-size: 0.7rem; color: var(--ts-text-dim); flex-shrink: 0; }
.st-tracker-pin { font-size: 0.75rem; flex-shrink: 0; }
.st-tracker-row.st-status-locked { opacity: 0.5; }
</style>
