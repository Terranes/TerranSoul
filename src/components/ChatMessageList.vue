<template>
  <div
    ref="listRef"
    class="message-list"
  >
    <!-- Welcome state when no messages yet -->
    <div
      v-if="messages.length === 0 && !isThinking && !isStreaming"
      class="welcome-state"
    >
      <div class="welcome-glow" />
      <div class="welcome-icon">
        ✨
      </div>
      <p class="welcome-title">
        Welcome to TerranSoul
      </p>
      <p class="welcome-hint">
        Your AI companion is ready. Pick a quest to begin your adventure!
      </p>
      <div class="welcome-quests">
        <button
          class="welcome-quest-btn primary"
          @click="$emit('startQuest')"
        >
          <span class="wq-icon">⚔️</span>
          <span class="wq-text">
            <span class="wq-label">Start First Quest</span>
            <span class="wq-hint">Recommended for you</span>
          </span>
        </button>
        <button
          class="welcome-quest-btn"
          @click="$emit('suggest', 'Tell me about yourself')"
        >
          <span class="wq-icon">💬</span>
          <span class="wq-text">
            <span class="wq-label">Just Chat</span>
            <span class="wq-hint">Talk freely</span>
          </span>
        </button>
        <button
          class="welcome-quest-btn"
          @click="$emit('navigate', 'skills')"
        >
          <span class="wq-icon">🗺️</span>
          <span class="wq-text">
            <span class="wq-label">View All Quests</span>
            <span class="wq-hint">Browse the skill tree</span>
          </span>
        </button>
      </div>
    </div>
    <!-- Teams-style date separators -->
    <div
      v-for="item in messagesWithSeparators"
      :key="item.key"
      :class="item.type === 'separator' ? 'date-separator' : ['message-row', item.msg!.role]"
    >
      <!-- Date separator -->
      <span
        v-if="item.type === 'separator'"
        class="date-separator-text"
      >{{ item.label }}</span>
      <!-- Message bubble -->
      <div
        v-else
        class="bubble-wrapper"
      >
        <AgentBadge
          v-if="item.msg!.role === 'assistant'"
          :name="item.msg!.agentName ?? 'TerranSoul'"
        />
        <SafeMarkdown
          class="bubble"
          :text="item.msg!.content"
        />
        <span class="timestamp">{{ formatTime(item.msg!.timestamp) }}</span>
      </div>
    </div>
    <!-- Live streaming response bubble -->
    <div
      v-if="isStreaming && streamingText"
      key="streaming"
      class="message-row assistant"
    >
      <div class="bubble-wrapper">
        <AgentBadge name="TerranSoul" />
        <SafeMarkdown
          class="bubble streaming-bubble"
          :text="streamingText"
          cursor
        />
      </div>
    </div>
    <TypingIndicator v-if="isThinking && !isStreaming" />
    <!-- Quick-reply buttons when model asks a yes/no question -->
    <div
      v-if="showQuickReplies"
      class="quick-replies"
    >
      <button
        class="quick-reply-btn yes"
        @click="sendQuickReply('Yes, let\'s do it!')"
      >
        ✅ Yes
      </button>
      <button
        class="quick-reply-btn no"
        @click="sendQuickReply('No, not right now.')"
      >
        ❌ No
      </button>
      <button
        class="quick-reply-btn more"
        @click="sendQuickReply('Tell me more about it first.')"
      >
        💬 Tell me more
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue';
import type { Message } from '../types';
import AgentBadge from './AgentBadge.vue';
import TypingIndicator from './TypingIndicator.vue';
import SafeMarkdown from './SafeMarkdown.vue';
import { useAiDecisionPolicyStore } from '../stores/ai-decision-policy';

/**
 * Read the live `quickRepliesEnabled` flag from the user-controllable
 * AI-decision policy. Returns the documented default (`true`) when Pinia
 * isn't active — this lets the component mount in legacy unit tests that
 * don't install a Pinia instance.
 */
function isQuickRepliesEnabled(): boolean {
  try {
    return useAiDecisionPolicyStore().policy.quickRepliesEnabled;
  } catch {
    return true;
  }
}

const props = defineProps<{
  messages: Message[];
  isThinking: boolean;
  streamingText?: string;
  isStreaming?: boolean;
}>();

const emit = defineEmits<{
  suggest: [message: string];
  startQuest: [];
  navigate: [target: string];
}>();

const listRef = ref<HTMLElement | null>(null);

/** Detect if a message ends with a yes/no question. */
const YES_NO_PATTERN = /(?:shall we|would you like|want me to|ready to|do you want|should i|can i|activate it|shall i|want to try)\s*\??$/i;

/** Whether the last assistant message asks a yes/no question (and it's the most recent message). */
const showQuickReplies = computed(() => {
  // Respects the "Quick-reply suggestions" toggle in the Brain panel — when
  // off, never offer one-tap Yes/No buttons regardless of trailing phrasing.
  if (!isQuickRepliesEnabled()) return false;
  if (props.isThinking || props.isStreaming) return false;
  const msgs = props.messages;
  if (msgs.length === 0) return false;
  const last = msgs[msgs.length - 1];
  if (last.role !== 'assistant') return false;
  if (last.questChoices?.length) return false; // already has quest buttons
  return YES_NO_PATTERN.test(last.content.trim());
});

function sendQuickReply(text: string) {
  emit('suggest', text);
}

/** Format a date label like Microsoft Teams: "Today", "Yesterday", or "Wednesday, January 5". */
function formatDateLabel(ts: number): string {
  const date = new Date(ts);
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const msgDay = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  const diffDays = Math.round((today.getTime() - msgDay.getTime()) / 86_400_000);

  if (diffDays === 0) return 'Today';
  if (diffDays === 1) return 'Yesterday';
  if (diffDays < 7) return date.toLocaleDateString([], { weekday: 'long' });
  return date.toLocaleDateString([], { weekday: 'long', month: 'long', day: 'numeric', year: now.getFullYear() !== date.getFullYear() ? 'numeric' : undefined });
}

/** Get the day key (YYYY-MM-DD) for grouping. */
function dayKey(ts: number): string {
  const d = new Date(ts);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

type SeparatorItem = { type: 'separator'; key: string; label: string; msg?: undefined };
type MessageItem = { type: 'message'; key: string; msg: Message; label?: undefined };

const messagesWithSeparators = computed<(SeparatorItem | MessageItem)[]>(() => {
  const items: (SeparatorItem | MessageItem)[] = [];
  let lastDay = '';
  for (const msg of props.messages) {
    const dk = dayKey(msg.timestamp);
    if (dk !== lastDay) {
      items.push({ type: 'separator', key: `sep-${dk}`, label: formatDateLabel(msg.timestamp) });
      lastDay = dk;
    }
    items.push({ type: 'message', key: msg.id, msg });
  }
  return items;
});

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

/* Teams-style date separator */
.date-separator {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 0;
  margin: 4px 0;
}
.date-separator::before,
.date-separator::after {
  content: '';
  flex: 1;
  height: 1px;
  background: rgba(255, 255, 255, 0.08);
}
.date-separator-text {
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--ts-text-dim);
  white-space: nowrap;
  letter-spacing: 0.03em;
  text-transform: uppercase;
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
  background: linear-gradient(135deg, var(--ts-accent) 0%, var(--ts-accent-hover) 100%);
  color: var(--ts-text-on-accent);
  border-bottom-right-radius: 4px;
  box-shadow: 0 2px 8px var(--ts-accent-glow);
}

.message-row.assistant .bubble {
  background: var(--ts-bg-hover);
  color: var(--ts-text-primary);
  border-bottom-left-radius: 4px;
  border: 1px solid var(--ts-border);
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

.welcome-quests {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 10px;
  width: 100%;
  max-width: 300px;
}

.welcome-quest-btn {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-radius: var(--ts-radius-lg);
  border: 1px solid rgba(124, 111, 255, 0.25);
  background: rgba(124, 111, 255, 0.08);
  color: var(--ts-text-primary);
  cursor: pointer;
  text-align: left;
  transition: all var(--ts-transition-fast);
}

.welcome-quest-btn:hover {
  background: rgba(124, 111, 255, 0.18);
  border-color: rgba(124, 111, 255, 0.5);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(124, 111, 255, 0.2);
}

.welcome-quest-btn.primary {
  border-color: rgba(124, 111, 255, 0.5);
  background: rgba(124, 111, 255, 0.15);
}

.welcome-quest-btn.primary:hover {
  background: rgba(124, 111, 255, 0.28);
  box-shadow: 0 4px 16px rgba(124, 111, 255, 0.3);
}

.wq-icon {
  font-size: 1.4rem;
  flex-shrink: 0;
}

.wq-text {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.wq-label {
  font-size: 0.85rem;
  font-weight: 600;
}

.wq-hint {
  font-size: 0.72rem;
  color: var(--ts-text-dim);
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



/* ── Quick-reply buttons (yes/no/more) ── */
.quick-replies {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 6px 16px 10px;
  animation: quick-replies-in 0.25s ease;
}
@keyframes quick-replies-in {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}
.quick-reply-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 7px 16px;
  border-radius: var(--ts-radius-pill);
  border: 1px solid rgba(56, 189, 248, 0.35);
  background: rgba(56, 189, 248, 0.10);
  color: rgba(56, 189, 248, 0.95);
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
}
.quick-reply-btn:hover {
  background: rgba(56, 189, 248, 0.22);
  border-color: rgba(56, 189, 248, 0.55);
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(56, 189, 248, 0.15);
}
.quick-reply-btn.yes {
  border-color: rgba(34, 197, 94, 0.4);
  background: var(--ts-success-bg);
  color: var(--ts-success);
}
.quick-reply-btn.yes:hover {
  background: rgba(34, 197, 94, 0.22);
  border-color: rgba(34, 197, 94, 0.6);
  box-shadow: 0 2px 8px rgba(34, 197, 94, 0.15);
}
.quick-reply-btn.no {
  border-color: rgba(239, 68, 68, 0.3);
  background: var(--ts-error-bg);
  color: var(--ts-error);
}
.quick-reply-btn.no:hover {
  background: rgba(239, 68, 68, 0.18);
  border-color: rgba(239, 68, 68, 0.5);
  box-shadow: 0 2px 8px rgba(239, 68, 68, 0.15);
}
</style>
