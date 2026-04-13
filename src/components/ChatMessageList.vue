<template>
  <div class="message-list" ref="listRef">
    <!-- Welcome state when no messages yet -->
    <div v-if="messages.length === 0 && !isThinking" class="welcome-state">
      <div class="welcome-icon">💬</div>
      <p class="welcome-title">Welcome to TerranSoul</p>
      <p class="welcome-hint">Type a message below to start chatting with your AI companion.</p>
    </div>
    <TransitionGroup name="msg">
      <div
        v-for="msg in messages"
        :key="msg.id"
        class="message-row"
        :class="msg.role"
      >
        <div class="bubble-wrapper">
          <AgentBadge v-if="msg.role === 'assistant'" :name="msg.agentName ?? 'TerranSoul'" />
          <div class="bubble">{{ msg.content }}</div>
          <span class="timestamp">{{ formatTime(msg.timestamp) }}</span>
        </div>
      </div>
    </TransitionGroup>
    <TypingIndicator v-if="isThinking" />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import type { Message } from '../types';
import AgentBadge from './AgentBadge.vue';
import TypingIndicator from './TypingIndicator.vue';

const props = defineProps<{
  messages: Message[];
  isThinking: boolean;
}>();

const listRef = ref<HTMLElement | null>(null);

function formatTime(ts: number) {
  return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

async function scrollToBottom() {
  await nextTick();
  if (listRef.value) {
    listRef.value.scrollTop = listRef.value.scrollHeight;
  }
}

watch(() => props.messages.length, scrollToBottom);
watch(() => props.isThinking, scrollToBottom);
</script>

<style scoped>
.message-list {
  flex: 1;
  overflow-y: auto;
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  scroll-behavior: smooth;
}

.message-row {
  display: flex;
}

.message-row.user {
  justify-content: flex-end;
}

.message-row.assistant {
  justify-content: flex-start;
}

.bubble-wrapper {
  display: flex;
  flex-direction: column;
  max-width: 72%;
  gap: 3px;
}

.message-row.user .bubble-wrapper {
  align-items: flex-end;
}

.bubble {
  padding: 10px 14px;
  border-radius: 18px;
  line-height: 1.5;
  font-size: 0.9rem;
  word-break: break-word;
  white-space: pre-wrap;
}

.message-row.user .bubble {
  background: linear-gradient(135deg, #6c63ff 0%, #5a52e0 100%);
  color: #fff;
  border-bottom-right-radius: 4px;
  box-shadow: 0 1px 3px rgba(108, 99, 255, 0.25);
}

.message-row.assistant .bubble {
  background: rgba(255, 255, 255, 0.08);
  color: #e8e8f0;
  border-bottom-left-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.timestamp {
  font-size: 0.68rem;
  color: rgba(255, 255, 255, 0.3);
  padding: 0 4px;
}

.msg-enter-active,
.msg-leave-active {
  transition: all 0.25s ease;
}

.msg-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.msg-leave-to {
  opacity: 0;
}

/* Welcome / empty state */
.welcome-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px 16px;
  opacity: 0.7;
  text-align: center;
}

.welcome-icon {
  font-size: 2.2rem;
  margin-bottom: 4px;
}

.welcome-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  color: #e8e8f0;
}

.welcome-hint {
  margin: 0;
  font-size: 0.82rem;
  color: rgba(255, 255, 255, 0.45);
  max-width: 260px;
  line-height: 1.4;
}
</style>
