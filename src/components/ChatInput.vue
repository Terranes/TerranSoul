<template>
  <form class="chat-input-bar" @submit.prevent="handleSubmit">
    <input
      v-model="inputText"
      type="text"
      class="chat-input"
      placeholder="Type a message…"
      :disabled="disabled"
      autocomplete="off"
    />
    <button
      type="submit"
      class="send-btn"
      :disabled="disabled || !inputText.trim()"
      aria-label="Send"
    >
      <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
        <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
      </svg>
    </button>
  </form>
</template>

<script setup lang="ts">
import { ref } from 'vue';

const props = defineProps<{ disabled: boolean }>();
const emit = defineEmits<{ submit: [message: string] }>();

const inputText = ref('');

function handleSubmit() {
  const text = inputText.value.trim();
  if (!text || props.disabled) return;
  emit('submit', text);
  inputText.value = '';
}
</script>

<style scoped>
.chat-input-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 14px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(0, 0, 0, 0.25);
}

.chat-input {
  flex: 1;
  padding: 10px 16px;
  border-radius: 22px;
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  font-size: var(--ts-text-base);
  outline: none;
  transition: border-color var(--ts-transition-normal), box-shadow var(--ts-transition-normal);
}

.chat-input:focus {
  border-color: var(--ts-accent);
  box-shadow: 0 0 0 2px var(--ts-accent-glow);
}

.chat-input::placeholder {
  color: var(--ts-text-dim);
}

.chat-input:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.send-btn {
  width: 38px;
  height: 38px;
  border-radius: 50%;
  border: none;
  background: var(--ts-accent);
  color: #fff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background var(--ts-transition-normal), opacity var(--ts-transition-normal), transform var(--ts-transition-fast);
}

.send-btn:hover:not(:disabled) {
  background: var(--ts-accent-hover);
  transform: scale(1.05);
}

.send-btn:active:not(:disabled) {
  transform: scale(0.95);
}

.send-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
</style>
