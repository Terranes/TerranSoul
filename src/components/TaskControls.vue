<template>
  <Transition name="task-controls">
    <div
      v-if="visible"
      class="task-controls"
    >
      <div class="task-controls__actions">
        <button
          class="task-controls__btn task-controls__btn--stop"
          aria-label="Stop generation"
          :title="TOOLTIPS.stop"
          @click="$emit('stop')"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 16 16"
            fill="currentColor"
          >
            <rect
              x="3"
              y="3"
              width="10"
              height="10"
              rx="1"
            />
          </svg>
          <span class="task-controls__label">Stop</span>
        </button>

        <button
          class="task-controls__btn task-controls__btn--stop-send"
          aria-label="Stop and send partial response"
          :title="TOOLTIPS.stopAndSend"
          @click="$emit('stop-and-send')"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 16 16"
            fill="currentColor"
          >
            <rect
              x="2"
              y="3"
              width="5"
              height="10"
              rx="1"
            />
            <path d="M9 8l5-3v6z" />
          </svg>
          <span class="task-controls__label">Stop &amp; Send</span>
        </button>

        <button
          class="task-controls__btn task-controls__btn--queue"
          aria-label="Add message to queue"
          :title="TOOLTIPS.addToQueue"
          @click="handleAddToQueue"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 16 16"
            fill="currentColor"
          >
            <path d="M8 2v12M2 8h12" stroke="currentColor" stroke-width="2" fill="none" />
          </svg>
          <span class="task-controls__label">Queue</span>
          <span
            v-if="queueCount > 0"
            class="task-controls__badge"
          >{{ queueCount }}</span>
        </button>

        <button
          class="task-controls__btn task-controls__btn--steer"
          aria-label="Steer with message"
          :title="TOOLTIPS.steer"
          @click="handleSteer"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 16 16"
            fill="currentColor"
          >
            <path d="M1 8h10M8 4l4 4-4 4" stroke="currentColor" stroke-width="2" fill="none" />
            <circle cx="14" cy="8" r="1.5" />
          </svg>
          <span class="task-controls__label">Steer</span>
        </button>

        <div class="task-controls__help-wrapper">
          <button
            class="task-controls__help-btn"
            aria-label="Help: what do these controls do?"
            @mouseenter="showHelp = true"
            @mouseleave="showHelp = false"
            @focus="showHelp = true"
            @blur="showHelp = false"
          >
            ?
          </button>
          <Transition name="help-tooltip">
            <div
              v-if="showHelp"
              class="task-controls__tooltip"
              role="tooltip"
            >
              <div class="task-controls__tooltip-title">
                Long-Running Task Controls
              </div>
              <dl class="task-controls__tooltip-list">
                <dt>⏹ Stop</dt>
                <dd>{{ TOOLTIPS.stop }}</dd>
                <dt>⏹▶ Stop &amp; Send</dt>
                <dd>{{ TOOLTIPS.stopAndSend }}</dd>
                <dt>＋ Queue</dt>
                <dd>{{ TOOLTIPS.addToQueue }}</dd>
                <dt>→● Steer</dt>
                <dd>{{ TOOLTIPS.steer }}</dd>
              </dl>
            </div>
          </Transition>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref } from 'vue';

defineProps<{
  visible: boolean;
  queueCount: number;
}>();

const emit = defineEmits<{
  stop: [];
  'stop-and-send': [];
  'add-to-queue': [message: string];
  steer: [message: string];
}>();

const showHelp = ref(false);

/** Tooltip descriptions matching VS Code Copilot's agent-mode behaviour. */
const TOOLTIPS = {
  stop:
    'Cancel the current generation and discard any partial output. ' +
    'Like pressing Escape in VS Code Copilot — the AI stops immediately and nothing is added to the conversation.',
  stopAndSend:
    'Cancel the current generation but keep what has been generated so far as the response. ' +
    'Useful when the AI is on the right track but you have enough — similar to VS Code Copilot\'s "Stop and keep partial" behaviour.',
  addToQueue:
    'Type a follow-up message that will be sent automatically after the current generation finishes. ' +
    'Messages are queued in order (FIFO). Like VS Code Copilot\'s message queue — you can keep typing while the AI works.',
  steer:
    'Send a message that redirects the current generation without waiting for it to finish. ' +
    'The AI stops, keeps partial output, and immediately processes your new instruction. ' +
    'Like VS Code Copilot\'s "steer" — course-correct the AI mid-stream.',
} as const;

function handleAddToQueue() {
  const message = prompt('Message to queue (sent after current generation finishes):');
  if (message?.trim()) {
    emit('add-to-queue', message.trim());
  }
}

function handleSteer() {
  const message = prompt('Steering message (redirects current generation):');
  if (message?.trim()) {
    emit('steer', message.trim());
  }
}
</script>

<style scoped>
.task-controls {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--ts-space-xs) var(--ts-space-sm);
}

.task-controls__actions {
  display: flex;
  align-items: center;
  gap: var(--ts-space-xs);
  padding: var(--ts-space-xs) var(--ts-space-sm);
  background: var(--ts-bg-elevated);
  border: 1px solid var(--ts-border-subtle);
  border-radius: var(--ts-radius-pill);
  box-shadow: var(--ts-shadow-sm);
}

.task-controls__btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border: none;
  border-radius: var(--ts-radius-sm);
  background: transparent;
  color: var(--ts-text-secondary);
  font-size: var(--ts-text-xs);
  font-family: var(--ts-font-family);
  cursor: pointer;
  transition: all var(--ts-transition-fast);
  white-space: nowrap;
  position: relative;
}

.task-controls__btn:hover {
  background: var(--ts-bg-hover);
  color: var(--ts-text-primary);
}

.task-controls__btn:active {
  transform: scale(0.95);
}

.task-controls__btn--stop:hover {
  color: var(--ts-error);
}

.task-controls__btn--stop-send:hover {
  color: var(--ts-warning);
}

.task-controls__btn--queue:hover {
  color: var(--ts-accent);
}

.task-controls__btn--steer:hover {
  color: var(--ts-success);
}

.task-controls__label {
  font-weight: 500;
}

.task-controls__badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  border-radius: var(--ts-radius-pill);
  background: var(--ts-accent);
  color: var(--ts-text-on-accent);
  font-size: 10px;
  font-weight: 700;
  line-height: 1;
}

/* ── Help button + tooltip ───────────────────────────────────────── */

.task-controls__help-wrapper {
  position: relative;
  margin-left: 2px;
}

.task-controls__help-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border: 1px solid var(--ts-border-subtle);
  border-radius: 50%;
  background: transparent;
  color: var(--ts-text-muted);
  font-size: 11px;
  font-weight: 700;
  font-family: var(--ts-font-family);
  cursor: help;
  transition: all var(--ts-transition-fast);
}

.task-controls__help-btn:hover,
.task-controls__help-btn:focus {
  background: var(--ts-bg-hover);
  color: var(--ts-text-primary);
  border-color: var(--ts-border-medium);
}

.task-controls__tooltip {
  position: absolute;
  bottom: calc(100% + 8px);
  right: -8px;
  width: 340px;
  padding: var(--ts-space-sm) var(--ts-space-md);
  background: var(--ts-bg-overlay);
  border: 1px solid var(--ts-border-medium);
  border-radius: var(--ts-radius-md);
  box-shadow: var(--ts-shadow-lg);
  z-index: 1000;
  pointer-events: none;
}

.task-controls__tooltip-title {
  font-size: var(--ts-text-sm);
  font-weight: 600;
  color: var(--ts-text-primary);
  margin-bottom: var(--ts-space-xs);
  padding-bottom: var(--ts-space-xs);
  border-bottom: 1px solid var(--ts-border-subtle);
}

.task-controls__tooltip-list {
  margin: 0;
  padding: 0;
}

.task-controls__tooltip-list dt {
  font-size: var(--ts-text-xs);
  font-weight: 600;
  color: var(--ts-text-primary);
  margin-top: var(--ts-space-xs);
}

.task-controls__tooltip-list dd {
  font-size: 11px;
  color: var(--ts-text-secondary);
  margin: 2px 0 0 0;
  line-height: 1.4;
}

/* ── Transitions ─────────────────────────────────────────────────── */

.task-controls-enter-active,
.task-controls-leave-active {
  transition: all var(--ts-transition-normal);
}

.task-controls-enter-from,
.task-controls-leave-to {
  opacity: 0;
  transform: translateY(8px);
}

.help-tooltip-enter-active,
.help-tooltip-leave-active {
  transition: all var(--ts-transition-fast);
}

.help-tooltip-enter-from,
.help-tooltip-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
