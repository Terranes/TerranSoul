<template>
  <Teleport to="body">
    <Transition name="qrc">
      <div
        v-if="activeCeremony"
        class="quest-reward-ceremony"
        data-testid="quest-reward-ceremony"
        role="dialog"
        aria-label="Quest Complete"
        @click.self="dismiss"
      >
        <div
          class="qrc-bg"
          aria-hidden="true"
        >
          <div
            v-for="i in 18"
            :key="i"
            class="qrc-particle"
            :style="particleStyle(i)"
          />
        </div>

        <div class="qrc-card">
          <div class="qrc-eyebrow">
            ⚔ Quest Complete
          </div>
          <div class="qrc-title-row">
            <span class="qrc-icon">{{ activeCeremony.icon }}</span>
            <h2 class="qrc-title">
              {{ activeCeremony.name }}
            </h2>
          </div>
          <p class="qrc-tagline">
            {{ activeCeremony.tagline }}
          </p>

          <div class="qrc-section">
            <div class="qrc-section-label">
              ◆ Stat Changes
            </div>
            <div class="qrc-stat-grid">
              <div
                v-for="desc in statDescriptors"
                :key="desc.id"
                class="qrc-stat"
                :class="{ 'qrc-stat--up': activeCeremony.statDelta[desc.id] > 0 }"
              >
                <span class="qrc-stat-icon">{{ desc.icon }}</span>
                <span class="qrc-stat-abbr">{{ desc.abbr }}</span>
                <span class="qrc-stat-bar">
                  <span
                    class="qrc-stat-bar-fill"
                    :style="{
                      width: activeCeremony.statsAfter[desc.id] + '%',
                      background: desc.color,
                    }"
                  />
                </span>
                <span class="qrc-stat-value">
                  {{ activeCeremony.statsBefore[desc.id] }}
                  <template v-if="activeCeremony.statDelta[desc.id] !== 0">
                    <span class="qrc-stat-arrow">→</span>
                    <strong>{{ activeCeremony.statsAfter[desc.id] }}</strong>
                    <span
                      v-if="activeCeremony.statDelta[desc.id] > 0"
                      class="qrc-stat-delta"
                    >+{{ activeCeremony.statDelta[desc.id] }}</span>
                  </template>
                </span>
              </div>
            </div>
          </div>

          <div
            v-if="activeCeremony.rewards.length"
            class="qrc-section"
          >
            <div class="qrc-section-label">
              ◆ Rewards Granted
            </div>
            <div class="qrc-reward-list">
              <span
                v-for="(r, i) in activeCeremony.rewards"
                :key="i"
                class="qrc-reward"
              >
                {{ activeCeremony.rewardIcons[i] || '🎁' }} {{ r }}
              </span>
            </div>
          </div>

          <div
            v-if="activeCeremony.newCombos.length"
            class="qrc-section"
          >
            <div class="qrc-section-label">
              ◆ New Combos
            </div>
            <div class="qrc-combo-list">
              <div
                v-for="combo in activeCeremony.newCombos"
                :key="combo.name"
                class="qrc-combo"
              >
                <span class="qrc-combo-icon">{{ combo.icon }}</span>
                <div>
                  <div class="qrc-combo-name">
                    {{ combo.name }}
                  </div>
                  <div class="qrc-combo-desc">
                    {{ combo.description }}
                  </div>
                </div>
              </div>
            </div>
          </div>

          <button
            class="qrc-dismiss"
            data-testid="qrc-dismiss"
            @click="dismiss"
          >
            Continue ▸
          </button>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue';
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
const statDescriptors = STAT_DESCRIPTORS;

let autoDismissTimer: ReturnType<typeof setTimeout> | null = null;
const AUTO_DISMISS_MS = 8000;

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

function particleStyle(i: number): Record<string, string> {
  // Pseudo-random but deterministic positions for SSR-friendly tests.
  const angle = (i * 137.508) % 360;
  const distance = 30 + ((i * 53) % 50);
  return {
    '--qrc-x': `${Math.cos((angle * Math.PI) / 180) * distance}vw`,
    '--qrc-y': `${Math.sin((angle * Math.PI) / 180) * distance}vh`,
    '--qrc-d': `${1.5 + (i % 7) * 0.3}s`,
    '--qrc-delay': `${(i % 5) * 0.2}s`,
  } as Record<string, string>;
}

onUnmounted(() => {
  if (autoDismissTimer) clearTimeout(autoDismissTimer);
});
</script>

<style scoped>
.quest-reward-ceremony {
  position: fixed;
  inset: 0;
  z-index: 70;
  background: radial-gradient(ellipse at center, rgba(20, 18, 50, 0.85), rgba(4, 4, 14, 0.96));
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  backdrop-filter: blur(8px);
}

.qrc-bg { position: absolute; inset: 0; pointer-events: none; overflow: hidden; }
.qrc-particle {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #dcc36e;
  box-shadow: 0 0 12px rgba(220, 195, 110, 0.7);
  animation: qrc-burst var(--qrc-d, 2s) ease-out var(--qrc-delay, 0s) infinite;
}
@keyframes qrc-burst {
  0% { transform: translate(-50%, -50%); opacity: 1; }
  100% { transform: translate(calc(-50% + var(--qrc-x)), calc(-50% + var(--qrc-y))); opacity: 0; }
}

.qrc-card {
  position: relative;
  width: 100%;
  max-width: 520px;
  background: linear-gradient(170deg, rgba(28, 24, 56, 0.98), rgba(12, 10, 28, 0.98));
  border: 1.5px solid rgba(220, 195, 110, 0.5);
  border-radius: 8px;
  padding: 24px 28px;
  box-shadow:
    0 24px 80px rgba(0, 0, 0, 0.7),
    0 0 60px rgba(220, 195, 110, 0.18),
    inset 0 0 30px rgba(220, 195, 110, 0.04);
  text-align: center;
}

.qrc-eyebrow {
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.18em;
  color: #dcc36e;
  text-transform: uppercase;
  text-shadow: 0 0 10px rgba(220, 195, 110, 0.5);
}

.qrc-title-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin: 8px 0 4px;
}
.qrc-icon { font-size: 2.2rem; filter: drop-shadow(0 0 12px rgba(220, 195, 110, 0.6)); }
.qrc-title {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 800;
  color: #f0e8c8;
  text-shadow: 0 0 14px rgba(220, 195, 110, 0.4);
}
.qrc-tagline {
  margin: 0 0 18px;
  font-size: 0.85rem;
  color: rgba(220, 220, 235, 0.7);
  font-style: italic;
}

.qrc-section { text-align: left; margin-top: 14px; }
.qrc-section-label {
  font-size: 0.65rem;
  font-weight: 700;
  letter-spacing: 0.14em;
  color: #dcc36e;
  text-transform: uppercase;
  margin-bottom: 8px;
}

.qrc-stat-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 6px 14px; }
.qrc-stat {
  display: grid;
  grid-template-columns: 18px 30px 1fr auto;
  align-items: center;
  gap: 6px;
  font-size: 0.7rem;
  padding: 3px 0;
  color: rgba(200, 200, 220, 0.7);
}
.qrc-stat-icon { line-height: 1; text-align: center; }
.qrc-stat-abbr { font-weight: 700; color: #dcc36e; font-size: 0.6rem; letter-spacing: 0.05em; }
.qrc-stat-bar {
  height: 6px;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 3px;
  overflow: hidden;
}
.qrc-stat-bar-fill { display: block; height: 100%; transition: width 0.8s cubic-bezier(0.4, 0, 0.2, 1); }
.qrc-stat-value { font-variant-numeric: tabular-nums; white-space: nowrap; font-size: 0.7rem; }
.qrc-stat-arrow { color: rgba(200, 200, 220, 0.4); margin: 0 4px; }
.qrc-stat-delta {
  margin-left: 6px;
  font-weight: 700;
  color: #80c882;
  text-shadow: 0 0 6px rgba(128, 200, 130, 0.5);
}
.qrc-stat--up .qrc-stat-abbr { color: #80c882; }

.qrc-reward-list { display: flex; flex-wrap: wrap; gap: 6px; }
.qrc-reward {
  font-size: 0.75rem;
  padding: 4px 10px;
  border-radius: 3px;
  background: rgba(220, 195, 110, 0.1);
  color: #dcc36e;
  border: 1px solid rgba(220, 195, 110, 0.25);
}

.qrc-combo-list { display: flex; flex-direction: column; gap: 8px; }
.qrc-combo {
  display: flex; align-items: center; gap: 12px;
  padding: 8px 12px;
  background: rgba(142, 200, 246, 0.08);
  border: 1px solid rgba(142, 200, 246, 0.28);
  border-radius: 4px;
}
.qrc-combo-icon { font-size: 1.4rem; }
.qrc-combo-name { font-weight: 700; color: #8ec8f6; font-size: 0.8rem; }
.qrc-combo-desc { font-size: 0.7rem; color: rgba(200, 200, 220, 0.65); margin-top: 2px; }

.qrc-dismiss {
  margin-top: 22px;
  padding: 10px 28px;
  background: linear-gradient(180deg, rgba(220, 195, 110, 0.25), rgba(180, 160, 100, 0.1));
  border: 1px solid rgba(220, 195, 110, 0.55);
  color: #f0e8c8;
  font-size: 0.85rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s ease;
  text-shadow: 0 0 8px rgba(220, 195, 110, 0.5);
}
.qrc-dismiss:hover {
  background: linear-gradient(180deg, rgba(220, 195, 110, 0.4), rgba(180, 160, 100, 0.2));
  box-shadow: 0 0 18px rgba(220, 195, 110, 0.35);
}

/* Transitions */
.qrc-enter-active, .qrc-leave-active { transition: opacity 0.4s ease; }
.qrc-enter-active .qrc-card, .qrc-leave-active .qrc-card { transition: transform 0.4s cubic-bezier(0.2, 0.8, 0.2, 1); }
.qrc-enter-from { opacity: 0; }
.qrc-enter-from .qrc-card { transform: scale(0.85); }
.qrc-leave-to { opacity: 0; }
.qrc-leave-to .qrc-card { transform: scale(0.95); }

@media (max-width: 640px) {
  .qrc-card { padding: 18px 18px; }
  .qrc-title { font-size: 1.2rem; }
  .qrc-icon { font-size: 1.8rem; }
  .qrc-stat-grid { grid-template-columns: 1fr; }
}
</style>
