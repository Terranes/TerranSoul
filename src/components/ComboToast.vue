<template>
  <Teleport to="body">
    <div
      class="combo-toast-stack"
      data-testid="combo-toast-stack"
      aria-live="polite"
    >
      <TransitionGroup name="combo-toast">
        <div
          v-for="toast in visibleToasts"
          :key="toast.key"
          class="combo-toast"
          :data-testid="`combo-toast-${comboTestId(toast.key)}`"
          @click="dismiss(toast.key)"
        >
          <div
            class="ct-burst"
            aria-hidden="true"
          >
            <span
              v-for="n in 6"
              :key="n"
              class="ct-spark"
              :style="{ '--ct-i': n }"
            />
          </div>
          <div class="ct-icon">
            {{ toast.icon }}
          </div>
          <div class="ct-body">
            <div class="ct-eyebrow">
              ⚡ Combo Unlocked
            </div>
            <div class="ct-name">
              {{ toast.name }}
            </div>
            <div class="ct-desc">
              {{ toast.description }}
            </div>
            <div class="ct-source">
              via <strong>{{ toast.sourceName }}</strong>
            </div>
          </div>
          <button
            class="ct-close"
            aria-label="Dismiss combo notification"
            @click.stop="dismiss(toast.key)"
          >
            ✕
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { useSkillTreeStore } from '../stores/skill-tree';

interface ComboToastItem {
  key: string;
  name: string;
  description: string;
  icon: string;
  sourceName: string;
  expiresAt: number;
}

const TOAST_DURATION_MS = 6000;

const skillTree = useSkillTreeStore();
const toasts = ref<ComboToastItem[]>([]);
let interval: ReturnType<typeof setInterval> | null = null;

const visibleToasts = computed(() => toasts.value.slice(0, 3));

function comboKey(sourceSkill: string, comboName: string): string {
  return `${sourceSkill}::${comboName}`;
}

/** A DOM-safe variant — `::` is treated as a pseudo-element selector. */
function comboTestId(key: string): string {
  return key.replace(/::/g, '__');
}

function dismiss(key: string) {
  toasts.value = toasts.value.filter(t => t.key !== key);
}

function pruneExpired() {
  const now = Date.now();
  if (toasts.value.some(t => t.expiresAt <= now)) {
    toasts.value = toasts.value.filter(t => t.expiresAt > now);
  }
}

watch(
  () => skillTree.activeCombos,
  (combos) => {
    if (!combos || combos.length === 0) return;
    // Suppressed during first-launch wizard / batch init.
    if (skillTree.notificationsSuppressed) return;
    const seen = new Set(skillTree.tracker.seenComboKeys);
    const newKeys: string[] = [];
    const queued: ComboToastItem[] = [];
    for (const entry of combos) {
      const key = comboKey(entry.sourceSkill, entry.combo.name);
      if (seen.has(key)) continue;
      newKeys.push(key);
      const sourceNode = skillTree.nodes.find(n => n.id === entry.sourceSkill);
      queued.push({
        key,
        name: entry.combo.name,
        description: entry.combo.description,
        icon: entry.combo.icon,
        sourceName: sourceNode?.name ?? entry.sourceSkill,
        expiresAt: Date.now() + TOAST_DURATION_MS,
      });
    }
    if (newKeys.length === 0) return;
    toasts.value = [...queued, ...toasts.value];
    skillTree.markCombosSeen(newKeys);
  },
  { deep: true, immediate: true },
);

onMounted(() => {
  interval = setInterval(pruneExpired, 1000);
});

onUnmounted(() => {
  if (interval) clearInterval(interval);
});
</script>

<style scoped>
.combo-toast-stack {
  position: fixed;
  /* Anchored on the LEFT so we never collide with the QuestBubble orb on the right. */
  bottom: 18px;
  left: 18px;
  z-index: 60;
  display: flex;
  flex-direction: column;
  gap: 10px;
  pointer-events: none;
  max-width: 360px;
}

.combo-toast {
  pointer-events: auto;
  position: relative;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 36px 12px 14px;
  background: linear-gradient(135deg, rgba(220, 195, 110, 0.15), rgba(20, 18, 40, 0.95));
  border: 1.5px solid rgba(220, 195, 110, 0.55);
  border-radius: 6px;
  box-shadow:
    0 8px 24px rgba(0, 0, 0, 0.45),
    0 0 24px rgba(220, 195, 110, 0.25),
    inset 0 0 12px rgba(220, 195, 110, 0.05);
  cursor: pointer;
  overflow: hidden;
  min-width: 280px;
}

.ct-burst {
  position: absolute;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
}
.ct-spark {
  position: absolute;
  top: 50%;
  left: 36px;
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: #dcc36e;
  box-shadow: 0 0 6px #dcc36e;
  opacity: 0;
  animation: ct-burst 1.2s ease-out forwards;
  animation-delay: calc(var(--ct-i) * 0.04s);
  --ct-angle: calc(var(--ct-i) * 60deg);
  transform-origin: center;
}
@keyframes ct-burst {
  0% {
    transform: translate(-50%, -50%) rotate(var(--ct-angle)) translateX(0);
    opacity: 1;
  }
  100% {
    transform: translate(-50%, -50%) rotate(var(--ct-angle)) translateX(34px);
    opacity: 0;
  }
}

.ct-icon {
  font-size: 1.8rem;
  width: 38px;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  filter: drop-shadow(0 0 8px rgba(220, 195, 110, 0.6));
  z-index: 1;
}
.ct-body { flex: 1; min-width: 0; z-index: 1; }
.ct-eyebrow {
  font-size: 0.6rem;
  font-weight: 700;
  letter-spacing: 0.14em;
  color: #dcc36e;
  text-transform: uppercase;
  text-shadow: 0 0 6px rgba(220, 195, 110, 0.4);
}
.ct-name {
  font-size: 0.9rem;
  font-weight: 700;
  color: #f0e8c8;
  margin-top: 1px;
}
.ct-desc {
  font-size: 0.7rem;
  color: rgba(220, 220, 235, 0.78);
  margin-top: 2px;
  line-height: 1.4;
}
.ct-source {
  font-size: 0.62rem;
  color: rgba(180, 180, 200, 0.55);
  margin-top: 4px;
}
.ct-source strong { color: #8ec8f6; font-weight: 600; }

.ct-close {
  position: absolute;
  top: 6px;
  right: 6px;
  background: none;
  border: none;
  color: rgba(220, 220, 235, 0.5);
  cursor: pointer;
  font-size: 0.75rem;
  padding: 4px 6px;
  line-height: 1;
  border-radius: 2px;
  z-index: 2;
}
.ct-close:hover { color: #dcc36e; background: rgba(255, 255, 255, 0.05); }

/* Slide-in / out transitions */
.combo-toast-enter-active { transition: transform 0.4s cubic-bezier(0.2, 0.8, 0.2, 1), opacity 0.3s ease; }
.combo-toast-leave-active { transition: transform 0.3s ease, opacity 0.3s ease; }
.combo-toast-enter-from { transform: translateX(-120%); opacity: 0; }
.combo-toast-leave-to { transform: translateX(-30px); opacity: 0; }

/* Don't fight the mobile bottom navigation. */
@media (max-width: 640px) {
  .combo-toast-stack {
    left: 8px;
    right: 8px;
    bottom: 64px; /* Above .mobile-bottom-nav (56px tall) */
    max-width: none;
  }
  .combo-toast { min-width: 0; }
}
</style>
