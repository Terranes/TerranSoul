<template>
  <form class="chat-input-bar" @submit.prevent="handleSubmit">
    <div class="input-wrapper">
      <input
        ref="inputRef"
        v-model="inputText"
        type="text"
        class="chat-input"
        placeholder="Type a message…"
        :disabled="disabled"
        autocomplete="off"
        @focus="handleFocus"
      />
      <button
        type="submit"
        class="send-btn"
        :disabled="disabled || !inputText.trim()"
        aria-label="Send"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
        </svg>
      </button>
    </div>
  </form>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { burstResetScroll } from '../utils/scroll-reset';

const props = defineProps<{ disabled: boolean }>();
const emit = defineEmits<{ submit: [message: string] }>();

const inputText = ref('');
const inputRef = ref<HTMLInputElement | null>(null);

function handleSubmit() {
  const text = inputText.value.trim();
  if (!text || props.disabled) return;
  emit('submit', text);
  inputText.value = '';
}

/**
 * When the input gains focus on mobile, iOS Safari tries to scroll the page
 * to keep the focused element visible.  We counteract this with a burst of
 * scroll resets across multiple frames and timers to cover the various points
 * where iOS may apply its auto-scroll.
 */
function handleFocus() {
  burstResetScroll();
}
</script>

<style scoped>
.chat-input-bar {
  display: flex;
  align-items: center;
  width: 100%;
}

.input-wrapper {
  display: flex;
  align-items: center;
  flex: 1;
  background: rgba(255, 255, 255, 0.07);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: var(--ts-radius-pill);
  padding: 4px 4px 4px 16px;
  transition: border-color var(--ts-transition-normal), box-shadow var(--ts-transition-normal), background var(--ts-transition-normal);
}

.input-wrapper:focus-within {
  border-color: var(--ts-accent);
  box-shadow: 0 0 0 3px var(--ts-accent-glow);
  background: rgba(255, 255, 255, 0.10);
}

.chat-input {
  flex: 1;
  padding: 8px 4px;
  border: none;
  background: transparent;
  color: var(--ts-text-primary);
  font-size: var(--ts-text-base);
  outline: none;
  min-width: 0;
}

.chat-input::placeholder {
  color: var(--ts-text-dim);
}

.chat-input:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.send-btn {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: none;
  background: var(--ts-accent);
  color: #fff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background var(--ts-transition-normal), opacity var(--ts-transition-normal), transform var(--ts-transition-fast), box-shadow var(--ts-transition-fast);
  box-shadow: 0 2px 8px rgba(124, 111, 255, 0.3);
}

.send-btn:hover:not(:disabled) {
  background: var(--ts-accent-hover);
  transform: scale(1.06);
  box-shadow: 0 4px 14px rgba(124, 111, 255, 0.4);
}

.send-btn:active:not(:disabled) {
  transform: scale(0.95);
}

.send-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

/* Mobile refinements */
@media (max-width: 640px) {
  .input-wrapper {
    padding: 3px 3px 3px 12px;
  }

  .chat-input {
    font-size: 0.88rem;
    padding: 7px 4px;
  }

  .send-btn {
    width: 34px;
    height: 34px;
  }

  .send-btn svg {
    width: 16px;
    height: 16px;
  }
}
</style>
