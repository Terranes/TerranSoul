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
        <details
          v-if="item.msg!.thinkingContent"
          class="thinking-details"
        >
          <summary class="thinking-summary">
            💭 Thinking…
          </summary>
          <SafeMarkdown
            class="thinking-content"
            :text="item.msg!.thinkingContent"
          />
        </details>
        <SafeMarkdown
          class="bubble"
          :text="item.msg!.content"
        />
        <span class="timestamp">{{ formatTime(item.msg!.timestamp) }}</span>
        <div
          v-if="canRateCharismaTurn(item.msg!)"
          class="turn-rating"
          role="group"
          :aria-label="`Rate Charisma turn for message ${item.msg!.id}`"
        >
          <button
            v-for="rating in 5"
            :key="rating"
            type="button"
            class="turn-rating-btn"
            :class="{ active: (item.msg!.charismaTurnRating ?? 0) >= rating }"
            :aria-label="`Rate ${rating} of 5`"
            :title="`Rate ${rating} of 5`"
            @click="rateCharismaTurn(item.msg!, rating)"
          >
            ★
          </button>
        </div>
      </div>
    </div>
    <!-- Live streaming response bubble -->
    <div
      v-if="isStreaming && (streamingText || streamingThinkingText)"
      key="streaming"
      class="message-row assistant"
    >
      <div class="bubble-wrapper">
        <AgentBadge name="TerranSoul" />
        <details
          v-if="streamingThinkingText"
          class="thinking-details"
          :open="isThinkingPhase"
        >
          <summary class="thinking-summary">
            💭 {{ isThinkingPhase ? 'Thinking…' : 'Thought process' }}
          </summary>
          <SafeMarkdown
            class="thinking-content"
            :text="streamingThinkingText"
            :cursor="isThinkingPhase"
          />
        </details>
        <SafeMarkdown
          v-if="streamingText"
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
  /** Accumulated extended-thinking text during streaming. */
  streamingThinkingText?: string;
  /** Whether the model is currently in the thinking phase. */
  isThinkingPhase?: boolean;
}>();

const emit = defineEmits<{
  suggest: [message: string];
  startQuest: [];
  navigate: [target: string];
  rateCharismaTurn: [messageId: string, rating: number];
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

function canRateCharismaTurn(message: Message): boolean {
  return message.role === 'assistant' && Boolean(message.charismaAssets?.length);
}

function rateCharismaTurn(message: Message, rating: number): void {
  emit('rateCharismaTurn', message.id, rating);
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
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  scroll-behavior: smooth;
}

.message-row {
  display: flex;
  animation: msg-appear 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

@keyframes msg-appear {
  from { opacity: 0; transform: translateY(8px) scale(0.98); }
  to { opacity: 1; transform: translateY(0) scale(1); }
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
  gap: 14px;
  padding: 10px 0;
  margin: 6px 0;
}
.date-separator::before,
.date-separator::after {
  content: '';
  flex: 1;
  height: 1px;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.08), transparent);
}
.date-separator-text {
  font-size: 0.68rem;
  font-weight: 700;
  color: var(--ts-text-dim);
  white-space: nowrap;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.bubble-wrapper {
  display: flex;
  flex-direction: column;
  max-width: 72%;
  gap: 4px;
}

.message-row.user .bubble-wrapper {
  align-items: flex-end;
}

.bubble {
  padding: 12px 16px;
  border-radius: var(--ts-radius-xl);
  line-height: 1.55;
  font-size: var(--ts-text-base);
  word-break: break-word;
  transition: transform var(--ts-transition-fast), box-shadow var(--ts-transition-fast);
}

.bubble:hover {
  transform: translateY(-0.5px);
}

.message-row.user .bubble {
  background: linear-gradient(135deg, var(--ts-accent) 0%, var(--ts-accent-violet) 100%);
  color: var(--ts-text-on-accent);
  border-bottom-right-radius: 6px;
  box-shadow: 0 4px 16px rgba(124, 111, 255, 0.25),
              inset 0 1px 0 rgba(255, 255, 255, 0.15);
}

.message-row.user .bubble:hover {
  box-shadow: 0 6px 24px rgba(124, 111, 255, 0.35),
              inset 0 1px 0 rgba(255, 255, 255, 0.15);
}

.message-row.assistant .bubble {
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  color: var(--ts-text-primary);
  border-bottom-left-radius: 6px;
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  backdrop-filter: blur(8px);
  box-shadow: var(--ts-shadow-sm),
              inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

.message-row.assistant .bubble:hover {
  border-color: rgba(255, 255, 255, 0.12);
  box-shadow: var(--ts-shadow-md),
              inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

.timestamp {
  font-size: var(--ts-text-xs);
  color: var(--ts-text-dim);
  padding: 0 6px;
  opacity: 0;
  transition: opacity var(--ts-transition-fast);
}

.bubble-wrapper:hover .timestamp {
  opacity: 1;
}

.turn-rating {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 0 4px;
  min-height: 24px;
  opacity: 0;
  transition: opacity var(--ts-transition-fast);
}

.bubble-wrapper:hover .turn-rating {
  opacity: 1;
}

.turn-rating-btn {
  width: 22px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid transparent;
  border-radius: var(--ts-radius-sm);
  background: transparent;
  color: var(--ts-text-dim);
  cursor: pointer;
  font-size: 0.86rem;
  line-height: 1;
  transition: color var(--ts-transition-fast), background var(--ts-transition-fast),
              border-color var(--ts-transition-fast), transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.turn-rating-btn:hover,
.turn-rating-btn.active {
  color: var(--ts-warning);
  background: var(--ts-warning-bg);
  border-color: var(--ts-warning);
  transform: scale(1.15);
}

.msg-enter-active,
.msg-leave-active {
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.msg-enter-from {
  opacity: 0;
  transform: translateY(12px) scale(0.96);
}

.msg-leave-to {
  opacity: 0;
  transform: scale(0.96);
}

/* Welcome / empty state */
.welcome-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 32px 20px;
  text-align: center;
  position: relative;
}

.welcome-glow {
  position: absolute;
  width: 260px;
  height: 260px;
  border-radius: 50%;
  background: radial-gradient(circle,
    rgba(124, 111, 255, 0.2) 0%,
    rgba(167, 139, 250, 0.08) 40%,
    transparent 70%);
  pointer-events: none;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -65%);
  animation: welcome-pulse 4s ease-in-out infinite;
}

@keyframes welcome-pulse {
  0%, 100% { opacity: 0.7; transform: translate(-50%, -65%) scale(1); }
  50% { opacity: 1; transform: translate(-50%, -65%) scale(1.08); }
}

.welcome-icon {
  font-size: 3rem;
  margin-bottom: 6px;
  filter: drop-shadow(0 4px 12px rgba(124, 111, 255, 0.35));
  animation: welcome-float 3s ease-in-out infinite;
}

@keyframes welcome-float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-6px); }
}

.welcome-title {
  margin: 0;
  font-size: 1.2rem;
  font-weight: 700;
  color: var(--ts-text-primary);
  letter-spacing: -0.01em;
  background: linear-gradient(135deg, var(--ts-text-primary), var(--ts-accent-violet));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.welcome-hint {
  margin: 0;
  font-size: 0.84rem;
  color: var(--ts-text-secondary);
  max-width: 300px;
  line-height: 1.55;
}

.welcome-quests {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: 14px;
  width: 100%;
  max-width: 320px;
}

.welcome-quest-btn {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 18px;
  border-radius: var(--ts-radius-lg);
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  backdrop-filter: blur(8px);
  color: var(--ts-text-primary);
  cursor: pointer;
  text-align: left;
  transition: all 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow: var(--ts-shadow-sm);
}

.welcome-quest-btn:hover {
  background: rgba(124, 111, 255, 0.14);
  border-color: rgba(124, 111, 255, 0.4);
  transform: translateY(-2px) scale(1.01);
  box-shadow: 0 6px 20px rgba(124, 111, 255, 0.2);
}

.welcome-quest-btn.primary {
  border-color: rgba(124, 111, 255, 0.35);
  background: rgba(124, 111, 255, 0.12);
  box-shadow: 0 2px 12px rgba(124, 111, 255, 0.12);
}

.welcome-quest-btn.primary:hover {
  background: rgba(124, 111, 255, 0.22);
  box-shadow: 0 8px 28px rgba(124, 111, 255, 0.28);
  border-color: rgba(124, 111, 255, 0.55);
}

.wq-icon {
  font-size: 1.5rem;
  flex-shrink: 0;
}

.wq-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.wq-label {
  font-size: 0.86rem;
  font-weight: 600;
  letter-spacing: 0.01em;
}

.wq-hint {
  font-size: 0.72rem;
  color: var(--ts-text-muted);
}

/* Streaming cursor blink */
.streaming-bubble {
  border: 1px solid rgba(124, 111, 255, 0.25);
  box-shadow: 0 0 20px rgba(124, 111, 255, 0.08),
              inset 0 1px 0 rgba(255, 255, 255, 0.04);
  animation: stream-pulse 2s ease-in-out infinite;
}

@keyframes stream-pulse {
  0%, 100% { border-color: rgba(124, 111, 255, 0.25); }
  50% { border-color: rgba(124, 111, 255, 0.45); }
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
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: var(--ts-radius-md);
  padding: 10px 14px;
  margin: 8px 0;
  overflow-x: auto;
  font-family: var(--ts-font-mono);
  font-size: 0.82rem;
  line-height: 1.55;
  white-space: pre-wrap;
  box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.15);
}

:deep(.md-inline-code) {
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 4px;
  padding: 2px 6px;
  font-family: var(--ts-font-mono);
  font-size: 0.84em;
}

/* Thinking / chain-of-thought */
.thinking-details {
  width: 100%;
  margin-bottom: 4px;
}

.thinking-summary {
  cursor: pointer;
  font-size: 0.8rem;
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.5));
  padding: 4px 8px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.04);
  user-select: none;
  list-style: none;
}

.thinking-summary::marker,
.thinking-summary::-webkit-details-marker {
  display: none;
}

.thinking-summary::before {
  content: '▶ ';
  font-size: 0.7em;
  transition: transform 0.2s;
  display: inline-block;
}

.thinking-details[open] > .thinking-summary::before {
  content: '▼ ';
}

.thinking-content {
  font-size: 0.78rem;
  line-height: 1.5;
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.5));
  padding: 6px 10px;
  border-left: 2px solid var(--ts-accent, #7c3aed);
  margin: 4px 0 4px 4px;
  background: rgba(124, 58, 237, 0.06);
  border-radius: 0 6px 6px 0;
  max-height: 300px;
  overflow-y: auto;
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
  gap: 8px;
  padding: 8px 20px 12px;
  animation: quick-replies-in 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
}
@keyframes quick-replies-in {
  from { opacity: 0; transform: translateY(8px) scale(0.95); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
.quick-reply-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 8px 18px;
  border-radius: var(--ts-radius-pill);
  border: 1px solid rgba(56, 189, 248, 0.3);
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  backdrop-filter: blur(8px);
  color: rgba(56, 189, 248, 0.95);
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow: var(--ts-shadow-sm);
}
.quick-reply-btn:hover {
  background: rgba(56, 189, 248, 0.15);
  border-color: rgba(56, 189, 248, 0.55);
  transform: translateY(-2px) scale(1.03);
  box-shadow: 0 4px 14px rgba(56, 189, 248, 0.18);
}
.quick-reply-btn.yes {
  border-color: rgba(34, 197, 94, 0.35);
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  color: var(--ts-success);
}
.quick-reply-btn.yes:hover {
  background: rgba(34, 197, 94, 0.15);
  border-color: rgba(34, 197, 94, 0.6);
  box-shadow: 0 4px 14px rgba(34, 197, 94, 0.18);
}
.quick-reply-btn.no {
  border-color: rgba(239, 68, 68, 0.25);
  background: var(--ts-glass-bg, rgba(15, 23, 42, 0.72));
  color: var(--ts-error);
}
.quick-reply-btn.no:hover {
  background: rgba(239, 68, 68, 0.12);
  border-color: rgba(239, 68, 68, 0.5);
  box-shadow: 0 4px 14px rgba(239, 68, 68, 0.18);
}
</style>
