<template>
  <div
    class="pet-overlay"
    @mouseenter="onMouseEnter"
    @mouseleave="onMouseLeave"
  >
    <!-- 3D character viewport (transparent background) -->
    <div class="pet-character">
      <CharacterViewport />
    </div>

    <!-- Floating chat bubble (shows recent message) -->
    <Transition name="bubble">
      <div
        v-if="showBubble && lastAssistantText"
        class="pet-bubble"
        @click.stop="toggleChat"
      >
        <p class="pet-bubble-text">{{ truncatedMessage }}</p>
      </div>
    </Transition>

    <!-- Expandable chat input (visible on hover or click) -->
    <Transition name="chat-slide">
      <div v-if="chatExpanded" class="pet-chat" @click.stop>
        <div class="pet-chat-messages" ref="messagesRef">
          <div
            v-for="msg in recentMessages"
            :key="msg.id"
            :class="['pet-msg', msg.role]"
          >
            <span class="pet-msg-text">{{ msg.content }}</span>
          </div>
          <div v-if="conversationStore.isThinking" class="pet-msg assistant">
            <span class="pet-msg-text pet-thinking">…</span>
          </div>
          <div v-if="conversationStore.isStreaming && conversationStore.streamingText" class="pet-msg assistant">
            <span class="pet-msg-text">{{ conversationStore.streamingText }}</span>
          </div>
        </div>
        <form class="pet-chat-input" @submit.prevent="handleSend">
          <input
            v-model="inputText"
            type="text"
            placeholder="Say something…"
            :disabled="conversationStore.isThinking"
            autocomplete="off"
          />
          <button type="submit" :disabled="conversationStore.isThinking || !inputText.trim()">
            ➤
          </button>
        </form>
      </div>
    </Transition>

    <!-- Pet mode controls (bottom-right) — always visible initially, then hover-only -->
    <div class="pet-controls" :class="{ visible: hovered || chatExpanded || showInitialHint }">
      <button class="pet-ctrl-btn" title="Toggle chat" @click.stop="toggleChat">💬</button>
      <button class="pet-ctrl-btn" title="Exit pet mode" @click.stop="exitPetMode">✕</button>
    </div>

    <!-- Emotion display -->
    <Transition name="fade">
      <div v-if="characterStore.state !== 'idle'" class="pet-emotion">
        {{ emotionEmoji }}
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue';
import { useConversationStore } from '../stores/conversation';
import { useCharacterStore } from '../stores/character';
import { useBrainStore } from '../stores/brain';
import { useWindowStore } from '../stores/window';
import { useStreamingStore } from '../stores/streaming';
import type { CharacterState } from '../types';
import CharacterViewport from '../components/CharacterViewport.vue';

const conversationStore = useConversationStore();
const characterStore = useCharacterStore();
const brain = useBrainStore();
const windowStore = useWindowStore();
const streaming = useStreamingStore();

const inputText = ref('');
const chatExpanded = ref(true);
const hovered = ref(false);
const showBubble = ref(false);
const showInitialHint = ref(true);
const messagesRef = ref<HTMLElement | null>(null);

let unlistenLlmChunk: (() => void) | null = null;

// Show only the last 20 messages in pet mode
const recentMessages = computed(() => {
  const msgs = conversationStore.messages;
  return msgs.slice(-20);
});

const lastAssistantText = computed(() => {
  const assistantMsgs = conversationStore.messages.filter((m) => m.role === 'assistant');
  return assistantMsgs.length > 0 ? assistantMsgs[assistantMsgs.length - 1].content : '';
});

const truncatedMessage = computed(() => {
  const text = lastAssistantText.value;
  return text.length > 120 ? text.substring(0, 117) + '…' : text;
});

const EMOTION_MAP: Record<CharacterState, string> = {
  idle: '',
  thinking: '🤔',
  talking: '💬',
  happy: '😊',
  sad: '😢',
  angry: '😠',
  relaxed: '😌',
  surprised: '😮',
};

const emotionEmoji = computed(() => EMOTION_MAP[characterStore.state] || '');

function toggleChat() {
  chatExpanded.value = !chatExpanded.value;
  if (chatExpanded.value) {
    showBubble.value = false;
    // Enable click events on this window
    windowStore.setCursorPassthrough(false);
    nextTick(() => scrollToBottom());
  } else {
    showBubble.value = true;
  }
}

function onMouseEnter() {
  hovered.value = true;
  // Ensure clicks work on the character/controls
  windowStore.setCursorPassthrough(false);
}

function onMouseLeave() {
  hovered.value = false;
  // If chat is not expanded, allow click-through
  if (!chatExpanded.value) {
    windowStore.setCursorPassthrough(true);
  }
}

function scrollToBottom() {
  if (messagesRef.value) {
    messagesRef.value.scrollTop = messagesRef.value.scrollHeight;
  }
}

async function handleSend() {
  const text = inputText.value.trim();
  if (!text || conversationStore.isThinking) return;
  inputText.value = '';
  await conversationStore.sendMessage(text);
  nextTick(() => scrollToBottom());
}

async function exitPetMode() {
  chatExpanded.value = false;
  await windowStore.setMode('window');
}

// Scroll on new messages
watch(
  () => conversationStore.messages.length,
  () => {
    if (chatExpanded.value) {
      nextTick(() => scrollToBottom());
    }
    // Show bubble on new assistant message
    if (!chatExpanded.value) {
      showBubble.value = true;
      // Auto-hide bubble after 8 seconds
      setTimeout(() => {
        if (!chatExpanded.value) showBubble.value = false;
      }, 8000);
    }
  },
);

// Map streaming emotion to character state
watch(
  () => streaming.currentEmotion,
  (emotion) => {
    if (emotion) {
      characterStore.setState(emotion as CharacterState);
      setTimeout(() => characterStore.setState('idle'), 6000);
    }
  },
);

// Set up streaming event listener
onMounted(async () => {
  try {
    await brain.loadActiveBrain();
  } catch {
    brain.autoConfigureFreeApi();
  }

  try {
    const { listen } = await import('@tauri-apps/api/event');
    unlistenLlmChunk = await listen<{ text: string; done: boolean }>('llm-chunk', (event) => {
      streaming.handleChunk(event.payload);
    });
  } catch {
    // No Tauri — browser-side streaming is handled by conversation store
  }

  // Start with click-through disabled since chat is expanded
  windowStore.setCursorPassthrough(false);

  // Auto-dismiss initial hint after 5 seconds
  setTimeout(() => {
    showInitialHint.value = false;
  }, 5000);
});

onUnmounted(() => {
  if (unlistenLlmChunk) {
    unlistenLlmChunk();
    unlistenLlmChunk = null;
  }
});
</script>

<style scoped>
.pet-overlay {
  position: fixed;
  inset: 0;
  overflow: hidden;
  /* Transparent background — the character floats on the desktop */
  background: transparent;
}

.pet-character {
  position: absolute;
  bottom: 0;
  right: 0;
  width: 350px;
  height: 500px;
  pointer-events: auto;
}

/* ── Chat bubble ── */
.pet-bubble {
  position: absolute;
  bottom: 420px;
  right: 20px;
  max-width: 280px;
  background: rgba(15, 23, 42, 0.92);
  border: 1px solid rgba(139, 92, 246, 0.3);
  border-radius: 16px 16px 4px 16px;
  padding: 12px 16px;
  color: #e2e8f0;
  font-size: 0.85rem;
  line-height: 1.4;
  cursor: pointer;
  backdrop-filter: blur(8px);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
  pointer-events: auto;
}

.pet-bubble-text {
  margin: 0;
}

/* Bubble transitions */
.bubble-enter-active,
.bubble-leave-active {
  transition: opacity 0.3s, transform 0.3s;
}
.bubble-enter-from,
.bubble-leave-to {
  opacity: 0;
  transform: translateY(10px) scale(0.95);
}

/* ── Expandable chat ── */
.pet-chat {
  position: absolute;
  bottom: 20px;
  left: 20px;
  width: 340px;
  max-height: 500px;
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(139, 92, 246, 0.25);
  border-radius: 16px;
  display: flex;
  flex-direction: column;
  backdrop-filter: blur(12px);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  overflow: hidden;
  pointer-events: auto;
}

.pet-chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 380px;
}

.pet-msg {
  padding: 8px 12px;
  border-radius: 12px;
  font-size: 0.82rem;
  line-height: 1.4;
  max-width: 85%;
  word-wrap: break-word;
}

.pet-msg.user {
  background: rgba(108, 99, 255, 0.25);
  align-self: flex-end;
  border-radius: 12px 12px 4px 12px;
}

.pet-msg.assistant {
  background: rgba(255, 255, 255, 0.08);
  align-self: flex-start;
  border-radius: 12px 12px 12px 4px;
}

.pet-msg-text {
  color: #e2e8f0;
}

.pet-thinking {
  animation: pet-pulse 1s ease-in-out infinite;
}

@keyframes pet-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 1; }
}

.pet-chat-input {
  display: flex;
  gap: 8px;
  padding: 10px 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.pet-chat-input input {
  flex: 1;
  padding: 8px 14px;
  border-radius: 20px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.07);
  color: #e2e8f0;
  font-size: 0.82rem;
  outline: none;
}

.pet-chat-input input:focus {
  border-color: #6c63ff;
}

.pet-chat-input button {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  border: none;
  background: #6c63ff;
  color: #fff;
  cursor: pointer;
  font-size: 0.9rem;
  flex-shrink: 0;
}

.pet-chat-input button:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

/* Chat slide transitions */
.chat-slide-enter-active,
.chat-slide-leave-active {
  transition: opacity 0.25s, transform 0.25s;
}
.chat-slide-enter-from,
.chat-slide-leave-to {
  opacity: 0;
  transform: translateY(20px);
}

/* ── Controls ── */
.pet-controls {
  position: absolute;
  bottom: 10px;
  right: 10px;
  display: flex;
  gap: 6px;
  opacity: 0;
  transition: opacity 0.2s;
  pointer-events: auto;
}

.pet-controls.visible {
  opacity: 1;
}

.pet-ctrl-btn {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.15);
  background: rgba(15, 23, 42, 0.85);
  color: #e2e8f0;
  cursor: pointer;
  font-size: 0.85rem;
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(4px);
  transition: background 0.15s, transform 0.15s;
}

.pet-ctrl-btn:hover {
  background: rgba(108, 99, 255, 0.35);
  transform: scale(1.1);
}

/* ── Emotion badge ── */
.pet-emotion {
  position: absolute;
  top: calc(100% - 530px);
  right: 140px;
  font-size: 1.5rem;
  pointer-events: none;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
