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

const props = defineProps<{
  messages: Message[];
  isThinking: boolean;
  streamingText?: string;
  isStreaming?: boolean;
}>();

defineEmits<{ suggest: [message: string] }>();

const suggestions = [
  'Tell me about yourself',
  'How are you feeling today?',
  'What can you help me with?',
];

const listRef = ref<HTMLElement | null>(null);

function formatTime(ts: number) {
  return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

/**
 * Lightweight markdown renderer — handles **bold**, *italic*, `code`,
 * ```code blocks```, and line breaks. No external dependency needed
 * for the subset we support.
 */
function renderMarkdown(text: string): string {
  /**
   * XSS Safety: Content is first escaped via escapeHtml() which replaces
   * all &, <, >, " characters with HTML entities. Only then are markdown
   * patterns converted to safe, known HTML tags (<strong>, <em>, <code>,
   * <pre>). No raw user content is ever inserted as HTML.
   */
  let html = escapeHtml(text);
  // Code blocks (```...```)
  html = html.replace(/```(\w*)\n?([\s\S]*?)```/g, '<pre class="md-code-block"><code>$2</code></pre>');
  // Inline code (`...`)
  html = html.replace(/`([^`]+)`/g, '<code class="md-inline-code">$1</code>');
  // Bold (**...** or __...__) — must come before italic
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/__(.+?)__/g, '<strong>$1</strong>');
  // Italic (*...* or _..._) — uses simple non-greedy match for broad
  // browser compatibility (avoids lookbehind which Safari <16.4 lacks).
  html = html.replace(/\*([^*]+)\*/g, '<em>$1</em>');
  html = html.replace(/\b_([^_]+)_\b/g, '<em>$1</em>');
  // Line breaks
  html = html.replace(/\n/g, '<br/>');
  return html;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

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
</style>
