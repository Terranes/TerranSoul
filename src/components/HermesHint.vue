<script setup lang="ts">
/**
 * HermesHint — Dismissable one-line suggestion banner.
 *
 * Shows when a chat turn exceeds the Hermes threshold: heavyweight
 * workloads (deep research, multi-day workflows, full-IDE coding) are
 * better served by Hermes Desktop + Hermes Agent.
 *
 * The hint never auto-launches anything. Clicking "Learn more" navigates
 * to the Marketplace → Companions panel where the user can install.
 */
import { ref, computed } from 'vue';
import type { HermesHintIntent } from '../utils/hermes-hint';

const props = defineProps<{
  intent: HermesHintIntent;
  visible: boolean;
}>();

const emit = defineEmits<{
  dismiss: [];
  navigate: [];
}>();

const intentLabels: Record<HermesHintIntent, string> = {
  deep_research: 'deep research',
  long_running_workflow: 'long-running workflows',
  full_ide_coding: 'full-IDE coding sessions',
};

const hintText = computed(() =>
  `For ${intentLabels[props.intent]}, Hermes Desktop offers a dedicated agent surface with your TerranSoul brain.`,
);

const dismissed = ref(false);

function dismiss(): void {
  dismissed.value = true;
  emit('dismiss');
}

function navigateToCompanions(): void {
  emit('navigate');
}
</script>

<template>
  <Transition name="hermes-hint">
    <div
      v-if="visible && !dismissed"
      class="hermes-hint"
      role="status"
      aria-live="polite"
      data-testid="hermes-hint-banner"
    >
      <span
        class="hermes-hint__icon"
        aria-hidden="true"
      >💡</span>
      <span class="hermes-hint__text">{{ hintText }}</span>
      <button
        class="hermes-hint__action"
        data-testid="hermes-hint-learn"
        @click="navigateToCompanions"
      >
        Learn more
      </button>
      <button
        class="hermes-hint__close"
        aria-label="Dismiss hint"
        data-testid="hermes-hint-dismiss"
        @click="dismiss"
      >
        ✕
      </button>
    </div>
  </Transition>
</template>

<style scoped>
.hermes-hint {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  margin: 0.5rem 1rem;
  background: var(--ts-galaxy-hud-bg, rgba(20, 18, 28, 0.62));
  border: 1px solid var(--ts-galaxy-hud-border, rgba(255, 255, 255, 0.08));
  border-radius: 10px;
  backdrop-filter: blur(12px);
  font-size: 0.82rem;
  color: var(--ts-galaxy-hud-fg, rgba(255, 255, 255, 0.88));
  line-height: 1.4;
}

.hermes-hint__icon {
  flex-shrink: 0;
  font-size: 1rem;
}

.hermes-hint__text {
  flex: 1;
  min-width: 0;
}

.hermes-hint__action {
  flex-shrink: 0;
  background: rgba(185, 172, 232, 0.12);
  border: 1px solid rgba(185, 172, 232, 0.3);
  border-radius: 6px;
  color: var(--ts-galaxy-violet, #b9ace8);
  font-size: 0.78rem;
  padding: 0.25rem 0.6rem;
  cursor: pointer;
  transition: background 0.15s ease;
}

.hermes-hint__action:hover {
  background: rgba(185, 172, 232, 0.22);
}

.hermes-hint__close {
  flex-shrink: 0;
  background: none;
  border: none;
  color: var(--ts-galaxy-hud-fg-dim, rgba(255, 255, 255, 0.48));
  font-size: 0.9rem;
  cursor: pointer;
  padding: 0.15rem 0.3rem;
  border-radius: 4px;
  transition: color 0.15s ease;
}

.hermes-hint__close:hover {
  color: var(--ts-galaxy-hud-fg, rgba(255, 255, 255, 0.88));
}

/* Transition */
.hermes-hint-enter-active,
.hermes-hint-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}

.hermes-hint-enter-from,
.hermes-hint-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
</style>
