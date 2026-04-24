<template>
  <Transition name="hotseat">
    <div v-if="choices.length > 0" class="hotseat-strip">
      <!-- Compact question row -->
      <div class="hotseat-header">
        <span class="hotseat-icon">🗡️</span>
        <span class="hotseat-question-text">{{ questionText }}</span>
        <button class="hotseat-dismiss" @click="$emit('dismiss')" aria-label="Dismiss">✕</button>
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
          <span v-if="choice.icon" class="hotseat-tile-icon">{{ choice.icon }}</span>
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
  padding: 6px 0 8px;
  border-bottom: 1px solid rgba(255, 215, 0, 0.15);
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
  color: #ffd700;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  line-height: 1.3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.hotseat-dismiss {
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.4);
  font-size: 0.75rem;
  cursor: pointer;
  padding: 2px 4px;
  line-height: 1;
  flex-shrink: 0;
}
.hotseat-dismiss:hover { color: rgba(255, 255, 255, 0.8); }

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
  border: 1px solid transparent;
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  color: #fff;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.4);
  transition: transform 0.15s ease, box-shadow 0.15s ease, border-color 0.15s ease;
  flex: 0 1 auto;
  justify-content: center;
  white-space: normal;
  text-align: center;
  line-height: 1.25;
  min-height: 32px;
}
.hotseat-tile:hover { transform: scale(1.03); }
.hotseat-tile:active { transform: scale(0.97); }

/* ── Color variants ── */
.hotseat-tile-orange {
  background: linear-gradient(135deg, rgba(255, 165, 0, 0.25) 0%, rgba(255, 120, 0, 0.15) 100%);
  border-color: rgba(255, 165, 0, 0.45);
}
.hotseat-tile-orange:hover {
  border-color: rgba(255, 165, 0, 0.8);
  box-shadow: 0 0 12px rgba(255, 165, 0, 0.2);
}
.hotseat-tile-blue {
  background: linear-gradient(135deg, rgba(56, 189, 248, 0.25) 0%, rgba(37, 99, 235, 0.15) 100%);
  border-color: rgba(56, 189, 248, 0.45);
}
.hotseat-tile-blue:hover {
  border-color: rgba(56, 189, 248, 0.8);
  box-shadow: 0 0 12px rgba(56, 189, 248, 0.2);
}
.hotseat-tile-green {
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.25) 0%, rgba(16, 150, 72, 0.15) 100%);
  border-color: rgba(34, 197, 94, 0.45);
}
.hotseat-tile-green:hover {
  border-color: rgba(34, 197, 94, 0.8);
  box-shadow: 0 0 12px rgba(34, 197, 94, 0.2);
}
.hotseat-tile-purple {
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.25) 0%, rgba(126, 34, 206, 0.15) 100%);
  border-color: rgba(168, 85, 247, 0.45);
}
.hotseat-tile-purple:hover {
  border-color: rgba(168, 85, 247, 0.8);
  box-shadow: 0 0 12px rgba(168, 85, 247, 0.2);
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
