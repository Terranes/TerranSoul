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
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.07);
  color: #e8e8f0;
  font-size: 0.9rem;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.chat-input:focus {
  border-color: #6c63ff;
  box-shadow: 0 0 0 2px rgba(108, 99, 255, 0.2);
}

.chat-input::placeholder {
  color: rgba(255, 255, 255, 0.35);
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
  background: #6c63ff;
  color: #fff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.2s, opacity 0.2s, transform 0.15s;
}

.send-btn:hover:not(:disabled) {
  background: #8078ff;
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
