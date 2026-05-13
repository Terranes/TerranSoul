<template>
  <Transition name="hotseat">
    <div
      v-if="choices.length > 0"
      class="hotseat-strip"
    >
      <!-- Compact question row -->
      <div class="hotseat-header">
        <span class="hotseat-icon">🗡️</span>
        <span class="hotseat-question-text">{{ questionText }}</span>
        <button
          class="hotseat-dismiss"
          aria-label="Dismiss"
          @click="$emit('dismiss')"
        >
          ✕
        </button>
      </div>

      <!-- Choice buttons — horizontal row -->
      <div class="hotseat-choices">
        <button
          v-for="(choice, idx) in choices"
          :key="choice.value"
          class="hotseat-tile"
          :class="[`hotseat-tile-${TILE_COLORS[idx] ?? 'blue'}`]"
          @click="$emit('pick', questId, choice.value)"
        >
          <span
            v-if="choice.icon"
            class="hotseat-tile-icon"
          >{{ choice.icon }}</span>
          <span class="hotseat-tile-label">{{ choice.label }}</span>
        </button>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import type { QuestChoice } from '../types';

defineProps<{
  choices: QuestChoice[];
  questId: string;
  questionText: string;
}>();

defineEmits<{
  pick: [questId: string, value: string];
  dismiss: [];
}>();

const TILE_COLORS = ['orange', 'blue', 'green', 'purple'];
</script>

<style scoped>
/* ── Compact strip inside the input footer ── */
.hotseat-strip {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px;
  border: 1px solid var(--ts-border-subtle, var(--ts-border));
  border-radius: var(--ts-radius-md);
  background: color-mix(in srgb, var(--ts-bg-elevated) 82%, transparent);
}

/* ── Header row ── */
.hotseat-header {
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 24px;
}
.hotseat-icon {
  font-size: 0.95rem;
  flex-shrink: 0;
}
.hotseat-question-text {
  flex: 1;
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--ts-text-primary);
  line-height: 1.3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.hotseat-dismiss {
  background: none;
  border: none;
  color: var(--ts-text-dim);
  font-size: 0.75rem;
  cursor: pointer;
  padding: 2px 4px;
  line-height: 1;
  flex-shrink: 0;
}
.hotseat-dismiss:hover { color: var(--ts-text-secondary); }

/* ── Choice buttons row ── */
.hotseat-choices {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

/*
 * Inline buttons — each tile sizes to its label so the full text is always
 * visible. Tiles wrap onto a second row only when they don't fit on one line;
 * if every label fits, all buttons sit on a single line. Long labels may wrap
 * inside a tile rather than being truncated with an ellipsis.
 */
.hotseat-tile {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px solid var(--ts-border);
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  color: var(--ts-text-primary);
  background: var(--ts-bg-input);
  transition: transform 0.15s ease, box-shadow 0.15s ease, border-color 0.15s ease;
  flex: 0 1 auto;
  justify-content: center;
  white-space: normal;
  text-align: center;
  line-height: 1.25;
  min-height: 32px;
}
.hotseat-tile:hover {
  transform: scale(1.03);
  background: var(--ts-bg-hover);
  border-color: var(--ts-border);
}
.hotseat-tile:active { transform: scale(0.97); }

/* ── Color variants ── */
.hotseat-tile-orange {
  border-color: color-mix(in srgb, var(--ts-warning) 50%, var(--ts-border));
  background: color-mix(in srgb, var(--ts-warning-bg, var(--ts-bg-hover)) 75%, transparent);
}
.hotseat-tile-orange:hover {
  border-color: var(--ts-warning);
  box-shadow: 0 0 10px color-mix(in srgb, var(--ts-warning) 30%, transparent);
}
.hotseat-tile-blue {
  border-color: color-mix(in srgb, var(--ts-accent-blue) 50%, var(--ts-border));
  background: color-mix(in srgb, var(--ts-info-bg, var(--ts-bg-hover)) 75%, transparent);
}
.hotseat-tile-blue:hover {
  border-color: var(--ts-accent-blue);
  box-shadow: 0 0 10px color-mix(in srgb, var(--ts-accent-blue) 30%, transparent);
}
.hotseat-tile-green {
  border-color: color-mix(in srgb, var(--ts-success) 50%, var(--ts-border));
  background: color-mix(in srgb, var(--ts-success-bg, var(--ts-bg-hover)) 75%, transparent);
}
.hotseat-tile-green:hover {
  border-color: var(--ts-success);
  box-shadow: 0 0 10px color-mix(in srgb, var(--ts-success) 30%, transparent);
}
.hotseat-tile-purple {
  border-color: color-mix(in srgb, var(--ts-accent) 50%, var(--ts-border));
  background: color-mix(in srgb, var(--ts-accent-glow, var(--ts-bg-hover)) 65%, transparent);
}
.hotseat-tile-purple:hover {
  border-color: var(--ts-accent);
  box-shadow: 0 0 10px color-mix(in srgb, var(--ts-accent) 30%, transparent);
}

.hotseat-tile-icon {
  font-size: 0.9rem;
  flex-shrink: 0;
}
.hotseat-tile-label {
  /* Allow the label to wrap inside the button instead of getting truncated. */
  overflow-wrap: anywhere;
  word-break: break-word;
}

/* ── Transitions ── */
.hotseat-enter-active { transition: opacity 0.25s ease, transform 0.25s ease; }
.hotseat-leave-active { transition: opacity 0.2s ease, transform 0.2s ease; }
.hotseat-enter-from { opacity: 0; transform: translateY(10px); }
.hotseat-leave-to { opacity: 0; transform: translateY(10px); }
</style>
