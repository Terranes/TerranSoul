<template>
  <div class="message-list" ref="listRef">
    <!-- Welcome state when no messages yet -->
    <div v-if="messages.length === 0 && !isThinking && !isStreaming" class="welcome-state">
      <div class="welcome-glow" />
      <div class="welcome-icon">✨</div>
      <p class="welcome-title">Welcome to TerranSoul</p>
      <p class="welcome-hint">Your AI companion is ready. Type a message below to start a conversation.</p>
      <div class="welcome-suggestions">
        <button
          v-for="suggestion in suggestions"
          :key="suggestion"
          class="suggestion-chip"
          @click="$emit('suggest', suggestion)"
        >{{ suggestion }}</button>
      </div>
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
          <div class="bubble" v-html="renderMarkdown(msg.content)" />
          <!-- RPG quest choice buttons -->
          <div v-if="msg.questChoices && msg.questChoices.length" class="quest-choices">
            <button
              v-for="choice in msg.questChoices"
              :key="choice.value"
              class="quest-choice-btn"
              @click="$emit('questChoice', msg.questId ?? '', choice.value)"
            >
              <span v-if="choice.icon" class="quest-choice-icon">{{ choice.icon }}</span>
              {{ choice.label }}
            </button>
          </div>
          <span class="timestamp">{{ formatTime(msg.timestamp) }}</span>
        </div>
      </div>
    </TransitionGroup>
    <!-- Live streaming response bubble -->
    <div v-if="isStreaming && streamingText" class="message-row assistant" key="streaming">
      <div class="bubble-wrapper">
        <AgentBadge name="TerranSoul" />
        <div class="bubble streaming-bubble" v-html="renderMarkdown(streamingText) + '<span class=\'cursor-blink\'>▎</span>'" />
      </div>
    </div>
    <TypingIndicator v-if="isThinking && !isStreaming" />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import type { Message } from '../types';
import AgentBadge from './AgentBadge.vue';
import TypingIndicator from './TypingIndicator.vue';
import { renderMarkdown } from '../utils/render-markdown';

const props = defineProps<{
  messages: Message[];
  isThinking: boolean;
  streamingText?: string;
  isStreaming?: boolean;
}>();

defineEmits<{ suggest: [message: string]; questChoice: [questId: string, value: string] }>();

const suggestions = [
  'Tell me about yourself',
  'How are you feeling today?',
  'What can you help me with?',
];

const listRef = ref<HTMLElement | null>(null);

function formatTime(ts: number) {
  return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

// renderMarkdown and escapeHtml imported from '../utils/render-markdown'

async function scrollToBottom() {
  await nextTick();
  if (listRef.value) {
    listRef.value.scrollTop = listRef.value.scrollHeight;
  }
}

watch(() => props.messages.length, scrollToBottom);
watch(() => props.isThinking, scrollToBottom);
watch(() => props.streamingText, scrollToBottom);
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
  border-radius: var(--ts-radius-xl);
  line-height: 1.5;
  font-size: var(--ts-text-base);
  word-break: break-word;
}

.message-row.user .bubble {
  background: linear-gradient(135deg, var(--ts-accent) 0%, #5a52e0 100%);
  color: #fff;
  border-bottom-right-radius: 4px;
  box-shadow: 0 2px 8px rgba(124, 111, 255, 0.28);
}

.message-row.assistant .bubble {
  background: rgba(255, 255, 255, 0.10);
  color: #eaecf4;
  border-bottom-left-radius: 4px;
  border: 1px solid rgba(255, 255, 255, 0.10);
}

.timestamp {
  font-size: var(--ts-text-xs);
  color: var(--ts-text-dim);
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
  gap: 10px;
  padding: 24px 16px;
  text-align: center;
  position: relative;
}

.welcome-glow {
  position: absolute;
  width: 200px;
  height: 200px;
  border-radius: 50%;
  background: radial-gradient(circle, var(--ts-accent-glow) 0%, transparent 70%);
  pointer-events: none;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -70%);
}

.welcome-icon {
  font-size: 2.8rem;
  margin-bottom: 4px;
  filter: drop-shadow(0 2px 8px rgba(108, 99, 255, 0.3));
}

.welcome-title {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--ts-text-primary);
  letter-spacing: 0.02em;
}

.welcome-hint {
  margin: 0;
  font-size: 0.82rem;
  color: var(--ts-text-dim);
  max-width: 280px;
  line-height: 1.5;
}

.welcome-suggestions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: center;
  margin-top: 8px;
  max-width: 340px;
}

.suggestion-chip {
  padding: 7px 16px;
  border-radius: var(--ts-radius-pill);
  border: 1px solid rgba(124, 111, 255, 0.35);
  background: rgba(124, 111, 255, 0.10);
  color: rgba(124, 111, 255, 0.95);
  font-size: 0.78rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--ts-transition-fast);
}

.suggestion-chip:hover {
  background: rgba(124, 111, 255, 0.22);
  border-color: rgba(124, 111, 255, 0.55);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(124, 111, 255, 0.15);
}

/* Streaming cursor blink */
.streaming-bubble {
  border: 1px solid rgba(124, 111, 255, 0.3);
}

:deep(.cursor-blink) {
  animation: blink 0.8s step-end infinite;
  color: var(--ts-accent);
  font-weight: 300;
}

@keyframes blink {
  50% { opacity: 0; }
}

/* Markdown styling inside bubbles */
:deep(.md-code-block) {
  background: rgba(0, 0, 0, 0.3);
  border-radius: var(--ts-radius-sm);
  padding: 8px 12px;
  margin: 6px 0;
  overflow-x: auto;
  font-family: var(--ts-font-mono);
  font-size: 0.82rem;
  line-height: 1.5;
  white-space: pre-wrap;
}

:deep(.md-inline-code) {
  background: rgba(0, 0, 0, 0.25);
  border-radius: 3px;
  padding: 1px 5px;
  font-family: var(--ts-font-mono);
  font-size: 0.85em;
}

:deep(strong) {
  font-weight: 600;
}

:deep(em) {
  font-style: italic;
  opacity: 0.9;
}

/* RPG quest choice buttons */
.quest-choices {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}

.quest-choice-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: var(--ts-radius-pill);
  border: 1px solid rgba(255, 215, 0, 0.4);
  background: linear-gradient(135deg, rgba(255, 215, 0, 0.12) 0%, rgba(255, 165, 0, 0.08) 100%);
  color: #ffd700;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--ts-transition-fast);
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}

.quest-choice-btn:hover {
  background: linear-gradient(135deg, rgba(255, 215, 0, 0.25) 0%, rgba(255, 165, 0, 0.18) 100%);
  border-color: rgba(255, 215, 0, 0.7);
  transform: translateY(-1px);
  box-shadow: 0 3px 12px rgba(255, 215, 0, 0.2);
}

.quest-choice-btn:active {
  transform: translateY(0);
}

.quest-choice-icon {
  font-size: 1rem;
}
</style>
