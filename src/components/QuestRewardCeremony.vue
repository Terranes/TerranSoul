<template>
  <Teleport to="body">
    <Transition name="qrc">
      <div
        v-if="activeCeremony"
        class="quest-reward-toast"
        data-testid="quest-reward-ceremony"
        role="status"
        aria-label="Quest Complete"
      >
        <div class="qrc-toast-inner">
          <span class="qrc-toast-icon">{{ activeCeremony.icon }}</span>
          <div class="qrc-toast-body">
            <div class="qrc-toast-eyebrow">
              ⚔ Quest Complete
            </div>
            <div class="qrc-toast-title">
              {{ activeCeremony.name }}
            </div>
            <div
              v-if="changedStats.length"
              class="qrc-toast-stats"
            >
              <span
                v-for="s in changedStats"
                :key="s.id"
                class="qrc-toast-stat"
              >
                {{ s.icon }} {{ s.abbr }} +{{ s.delta }}
              </span>
            </div>
          </div>
          <button
            class="qrc-toast-close"
            data-testid="qrc-dismiss"
            aria-label="Dismiss"
            @click="dismiss"
          >
            ✕
          </button>
        </div>
        <div
          class="qrc-toast-progress"
          :style="{ animationDuration: AUTO_DISMISS_MS + 'ms' }"
        />
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue';
import { useSkillTreeStore } from '../stores/skill-tree';
import { computeStats, STAT_DESCRIPTORS, type StatSnapshot } from '../utils/stats';
import type { ComboRef, SkillNode } from '../stores/skill-tree';

interface Ceremony {
  questId: string;
  name: string;
  tagline: string;
  icon: string;
  timestamp: number;
  rewards: string[];
  rewardIcons: string[];
  statsBefore: StatSnapshot;
  statsAfter: StatSnapshot;
  statDelta: StatSnapshot;
  newCombos: ComboRef[];
}

const skillTree = useSkillTreeStore();
const queue = ref<Ceremony[]>([]);
const activeCeremony = ref<Ceremony | null>(null);

const AUTO_DISMISS_MS = 6000;

let autoDismissTimer: ReturnType<typeof setTimeout> | null = null;

/** Stats that actually changed for the current ceremony (for compact display). */
const changedStats = computed(() => {
  const c = activeCeremony.value;
  if (!c) return [];
  return STAT_DESCRIPTORS
    .filter(d => c.statDelta[d.id] > 0)
    .map(d => ({ id: d.id, icon: d.icon, abbr: d.abbr, delta: c.statDelta[d.id] }));
});

/** Activate the next queued ceremony, if any. */
function showNext() {
  if (activeCeremony.value || queue.value.length === 0) return;
  const next = queue.value.shift()!;
  activeCeremony.value = next;
  if (autoDismissTimer) clearTimeout(autoDismissTimer);
  autoDismissTimer = setTimeout(() => dismiss(), AUTO_DISMISS_MS);
}

function dismiss() {
  if (!activeCeremony.value) return;
  skillTree.setLastSeenActivationTimestamp(activeCeremony.value.timestamp);
  activeCeremony.value = null;
  if (autoDismissTimer) { clearTimeout(autoDismissTimer); autoDismissTimer = null; }
  // Show the next one in the queue (if any) on the next tick.
  setTimeout(showNext, 250);
}

/** Build a Ceremony entry for a freshly-activated node. */
function buildCeremony(node: SkillNode, timestamp: number, before: StatSnapshot, after: StatSnapshot): Ceremony {
  const newCombos: ComboRef[] = [];
  for (const c of skillTree.activeCombos) {
    if (c.sourceSkill === node.id || c.combo.withSkills.includes(node.id)) {
      const key = `${c.sourceSkill}::${c.combo.name}`;
      if (!skillTree.tracker.seenComboKeys.includes(key)) newCombos.push(c.combo);
    }
  }
  return {
    questId: node.id,
    name: node.name,
    tagline: node.tagline,
    icon: node.icon,
    timestamp,
    rewards: node.rewards,
    rewardIcons: node.rewardIcons ?? [],
    statsBefore: before,
    statsAfter: after,
    statDelta: {
      intelligence: after.intelligence - before.intelligence,
      wisdom:       after.wisdom       - before.wisdom,
      charisma:     after.charisma     - before.charisma,
      perception:   after.perception   - before.perception,
      dexterity:    after.dexterity    - before.dexterity,
      endurance:    after.endurance    - before.endurance,
    },
    newCombos,
  };
}

function activeIdsExcept(excludeIds: string[]): string[] {
  const exclude = new Set(excludeIds);
  return skillTree.nodes
    .filter(n => skillTree.getSkillStatus(n.id) === 'active' && !exclude.has(n.id))
    .map(n => n.id);
}

watch(
  () => skillTree.tracker.activationTimestamps,
  (next) => {
    if (!next) return;
    // Suppressed during first-launch wizard / batch init.
    if (skillTree.notificationsSuppressed) return;
    const lastSeen = skillTree.tracker.lastSeenActivationTimestamp ?? 0;
    // First-launch grace: if there's no high-water mark yet, treat the current
    // active set as "already seen" so we don't blast the user with a stack of
    // ceremonies for skills that were already on.
    if (lastSeen === 0) {
      const max = Math.max(0, ...Object.values(next));
      if (max > 0) skillTree.setLastSeenActivationTimestamp(max);
      return;
    }
    const fresh = Object.entries(next)
      .filter(([, ts]) => typeof ts === 'number' && ts > lastSeen)
      .sort((a, b) => a[1] - b[1]);

    if (fresh.length === 0) return;

    for (const [skillId, ts] of fresh) {
      const node = skillTree.nodes.find(n => n.id === skillId);
      if (!node) continue;
      // Compute "before" by removing this skill from the active set, "after" by
      // including it. The diff is therefore an honest representation of what
      // unlocking this skill contributed.
      const otherActives = activeIdsExcept([skillId]);
      const before = computeStats(otherActives);
      const after = computeStats([...otherActives, skillId]);
      queue.value.push(buildCeremony(node, ts, before, after));
    }
    showNext();
  },
  { deep: true, immediate: true },
);

onUnmounted(() => {
  if (autoDismissTimer) clearTimeout(autoDismissTimer);
});
</script>

<style scoped>
/* ── Compact top-of-screen toast notification ── */
.quest-reward-toast {
  position: fixed;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1100;
  width: min(420px, calc(100vw - 32px));
  pointer-events: auto;
}

.qrc-toast-inner {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: var(--ts-bg-elevated, linear-gradient(170deg, rgba(28, 24, 56, 0.96), rgba(12, 10, 28, 0.96)));
  backdrop-filter: blur(20px) saturate(1.3);
  -webkit-backdrop-filter: blur(20px) saturate(1.3);
  border: 1.5px solid var(--ts-quest-border, rgba(220, 195, 110, 0.4));
  border-radius: var(--ts-radius-lg, 14px);
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.35),
    0 0 24px var(--ts-quest-gold-glow, rgba(220, 195, 110, 0.12));
}

.qrc-toast-icon {
  font-size: 1.8rem;
  flex-shrink: 0;
  filter: drop-shadow(0 0 8px var(--ts-quest-gold-glow, rgba(220, 195, 110, 0.5)));
}

.qrc-toast-body {
  flex: 1;
  min-width: 0;
}

.qrc-toast-eyebrow {
  font-size: 0.6rem;
  font-weight: 700;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--ts-quest-gold, #dcc36e);
  text-shadow: 0 0 8px var(--ts-quest-gold-glow, rgba(220, 195, 110, 0.4));
  line-height: 1;
  margin-bottom: 2px;
}

.qrc-toast-title {
  font-size: 0.92rem;
  font-weight: 700;
  color: var(--ts-text-primary);
  line-height: 1.25;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.qrc-toast-stats {
  display: flex;
  gap: 8px;
  margin-top: 3px;
  flex-wrap: wrap;
}

.qrc-toast-stat {
  font-size: 0.68rem;
  font-weight: 600;
  color: #80c882;
  text-shadow: 0 0 4px rgba(128, 200, 130, 0.3);
}

.qrc-toast-close {
  width: 24px;
  height: 24px;
  flex-shrink: 0;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: var(--ts-text-muted);
  font-size: 0.8rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s, color 0.15s;
}

.qrc-toast-close:hover {
  background: var(--ts-bg-hover);
  color: var(--ts-text-primary);
}

/* Auto-dismiss progress bar */
.qrc-toast-progress {
  height: 3px;
  margin: 0 4px;
  border-radius: 0 0 var(--ts-radius-lg, 14px) var(--ts-radius-lg, 14px);
  background: var(--ts-quest-gold, #dcc36e);
  transform-origin: left;
  animation: qrc-countdown linear forwards;
  opacity: 0.6;
}

@keyframes qrc-countdown {
  from { transform: scaleX(1); }
  to { transform: scaleX(0); }
}

/* Transitions */
.qrc-enter-active {
  transition: opacity 0.3s ease, transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.qrc-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}
.qrc-enter-from {
  opacity: 0;
  transform: translateX(-50%) translateY(-20px);
}
.qrc-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-12px);
}

@media (max-width: 640px) {
  .quest-reward-toast {
    top: 8px;
    width: calc(100vw - 16px);
  }
}
</style>
