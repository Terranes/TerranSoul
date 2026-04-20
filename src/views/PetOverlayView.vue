<template>
  <div
    class="pet-overlay"
    :class="{ 'pet-overlay--chat': petChatExpanded, 'pet-overlay--dragging': isDragging }"
  >
    <!-- Draggable 3D character viewport (transparent background).
         Use .capture so the wrapper intercepts mousedown before it reaches the
         Three.js canvas, and .stop to prevent OrbitControls from also reacting
         to what's really a pet-mode drag/click. -->
    <div
      class="pet-character"
      :style="characterStyle"
      @mousedown.capture.stop="onCharacterMouseDown"
      @contextmenu.prevent.stop="onCharacterContextMenu"
    >
      <CharacterViewport />
    </div>

    <!-- Floating chat bubble (shows recent message) -->
    <Transition name="bubble">
      <div
        v-if="showBubble && lastAssistantText && !petChatExpanded"
        class="pet-bubble"
        :style="bubbleStyle"
        @click.stop="toggleChat"
      >
        <p class="pet-bubble-text">{{ truncatedMessage }}</p>
      </div>
    </Transition>

    <!-- Expandable chat input -->
    <Transition name="chat-slide">
      <div v-if="petChatExpanded" class="pet-chat" @click.stop @mousedown.stop>
        <div class="pet-chat-header">
          <span class="pet-chat-title">Chat</span>
          <button class="pet-chat-close" @click.stop="toggleChat" title="Close chat">×</button>
        </div>
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

    <!-- Onboarding tooltip — shown once on first use of pet mode -->
    <Transition name="fade">
      <div v-if="showOnboarding" class="pet-onboarding" @click.stop @mousedown.stop>
        <p class="pet-onboarding-title">Welcome to pet mode</p>
        <ul class="pet-onboarding-list">
          <li><strong>Click</strong> character to toggle chat</li>
          <li><strong>Hold &amp; drag</strong> to move them around</li>
          <li><strong>Right-click</strong> for menu (mood, settings…)</li>
          <li>Use the <strong>top-right toggle</strong> to return to desktop</li>
        </ul>
        <button class="pet-onboarding-dismiss" @click.stop="dismissOnboarding">Got it</button>
      </div>
    </Transition>

    <!-- Emotion badge -->
    <Transition name="fade">
      <div
        v-if="characterStore.state !== 'idle'"
        class="pet-emotion"
        :style="emotionStyle"
      >
        {{ emotionEmoji }}
      </div>
    </Transition>

    <!-- Right-click context menu -->
    <PetContextMenu
      :visible="menuVisible"
      :x="menuX"
      :y="menuY"
      @close="menuVisible = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue';
import { useConversationStore } from '../stores/conversation';
import { useCharacterStore } from '../stores/character';
import { useBrainStore } from '../stores/brain';
import { useWindowStore } from '../stores/window';
import { useStreamingStore } from '../stores/streaming';
import { useChatExpansion } from '../composables/useChatExpansion';
import type { CharacterState } from '../types';
import CharacterViewport from '../components/CharacterViewport.vue';
import PetContextMenu from '../components/PetContextMenu.vue';

const conversationStore = useConversationStore();
const characterStore = useCharacterStore();
const brain = useBrainStore();
const windowStore = useWindowStore();
const streaming = useStreamingStore();
const { petChatExpanded, setPetChatExpanded, togglePetChat } = useChatExpansion();

// ── UI state ──────────────────────────────────────────────────────────────────
const inputText = ref('');
const showBubble = ref(false);
const showOnboarding = ref(false);
const messagesRef = ref<HTMLElement | null>(null);

// ── Character position (persisted via localStorage) ───────────────────────────
const CHARACTER_WIDTH = 350;
const CHARACTER_HEIGHT = 500;
const POSITION_KEY = 'ts.pet.character_position';
const ONBOARDING_KEY = 'ts.pet.onboarded';

interface CharacterPosition {
  x: number;
  y: number;
}

function defaultPosition(): CharacterPosition {
  // Default: bottom-right corner (preserves the original layout).
  const vw = typeof window !== 'undefined' ? window.innerWidth : 1920;
  const vh = typeof window !== 'undefined' ? window.innerHeight : 1080;
  return { x: vw - CHARACTER_WIDTH, y: vh - CHARACTER_HEIGHT };
}

function loadSavedPosition(): CharacterPosition {
  try {
    const raw = localStorage.getItem(POSITION_KEY);
    if (!raw) return defaultPosition();
    const parsed = JSON.parse(raw) as CharacterPosition;
    if (typeof parsed?.x !== 'number' || typeof parsed?.y !== 'number') {
      return defaultPosition();
    }
    return clampToViewport(parsed);
  } catch {
    return defaultPosition();
  }
}

function clampToViewport(pos: CharacterPosition): CharacterPosition {
  const vw = typeof window !== 'undefined' ? window.innerWidth : 1920;
  const vh = typeof window !== 'undefined' ? window.innerHeight : 1080;
  const maxX = Math.max(0, vw - CHARACTER_WIDTH);
  const maxY = Math.max(0, vh - CHARACTER_HEIGHT);
  return {
    x: Math.max(0, Math.min(maxX, pos.x)),
    y: Math.max(0, Math.min(maxY, pos.y)),
  };
}

const characterPos = ref<CharacterPosition>(defaultPosition());

const characterStyle = computed(() => ({
  left: `${characterPos.value.x}px`,
  top: `${characterPos.value.y}px`,
  width: `${CHARACTER_WIDTH}px`,
  height: `${CHARACTER_HEIGHT}px`,
}));

const bubbleStyle = computed(() => {
  const x = characterPos.value.x + CHARACTER_WIDTH / 2 - 140;
  const y = Math.max(8, characterPos.value.y - 80);
  return {
    left: `${Math.max(8, Math.min(window.innerWidth - 296, x))}px`,
    top: `${y}px`,
  };
});

const emotionStyle = computed(() => ({
  left: `${characterPos.value.x + CHARACTER_WIDTH - 56}px`,
  top: `${characterPos.value.y + 40}px`,
}));

// ── Drag handling ─────────────────────────────────────────────────────────────
const HOLD_THRESHOLD_MS = 150; // hold duration before drag starts
const CLICK_MOVE_TOLERANCE = 4; // pixels — treat as click if moved less than this

const isDragging = ref(false);
let pressStartX = 0;
let pressStartY = 0;
let pressStartTime = 0;
let pressOriginPosX = 0;
let pressOriginPosY = 0;
let pressActive = false;
let holdTimer: ReturnType<typeof setTimeout> | null = null;

function onCharacterMouseDown(e: MouseEvent) {
  // Only primary button (left). Ignore right-click (handled by contextmenu).
  if (e.button !== 0) return;
  pressActive = true;
  pressStartX = e.clientX;
  pressStartY = e.clientY;
  pressStartTime = performance.now();
  pressOriginPosX = characterPos.value.x;
  pressOriginPosY = characterPos.value.y;

  // Bind drag handlers at the document level so dragging continues even if
  // the cursor briefly leaves the character element.  These are removed on
  // mouseup to avoid listener leaks between presses.
  document.addEventListener('mousemove', onDocMouseMove);
  document.addEventListener('mouseup', onDocMouseUp, { once: true });

  // Arm a hold timer — if the user keeps the mouse down past the threshold
  // without moving much, we enter drag mode.
  if (holdTimer) clearTimeout(holdTimer);
  holdTimer = setTimeout(() => {
    if (pressActive && !isDragging.value) {
      isDragging.value = true;
    }
  }, HOLD_THRESHOLD_MS);
}

function onDocMouseMove(e: MouseEvent) {
  if (!pressActive) return;
  const dx = e.clientX - pressStartX;
  const dy = e.clientY - pressStartY;

  // Promote to drag when the user exceeds the click tolerance, even before
  // the hold timer fires.
  if (!isDragging.value && Math.hypot(dx, dy) > CLICK_MOVE_TOLERANCE * 2) {
    isDragging.value = true;
  }

  if (isDragging.value) {
    const next: CharacterPosition = {
      x: pressOriginPosX + dx,
      y: pressOriginPosY + dy,
    };
    characterPos.value = clampToViewport(next);
  }
}

function onDocMouseUp(e: MouseEvent) {
  document.removeEventListener('mousemove', onDocMouseMove);
  if (!pressActive) return;

  const dx = e.clientX - pressStartX;
  const dy = e.clientY - pressStartY;
  const duration = performance.now() - pressStartTime;
  const wasDragging = isDragging.value;

  if (holdTimer) {
    clearTimeout(holdTimer);
    holdTimer = null;
  }
  pressActive = false;
  isDragging.value = false;

  if (wasDragging) {
    try {
      localStorage.setItem(POSITION_KEY, JSON.stringify(characterPos.value));
    } catch {
      // localStorage unavailable — position lives for this session only
    }
    return;
  }

  // Treat as a click: short duration and minimal movement.
  if (
    duration < HOLD_THRESHOLD_MS &&
    Math.hypot(dx, dy) <= CLICK_MOVE_TOLERANCE
  ) {
    toggleChat();
  }
}

// ── Right-click menu ──────────────────────────────────────────────────────────
const menuVisible = ref(false);
const menuX = ref(0);
const menuY = ref(0);

function onCharacterContextMenu(e: MouseEvent) {
  menuX.value = e.clientX;
  menuY.value = e.clientY;
  menuVisible.value = true;
}

// ── Chat ──────────────────────────────────────────────────────────────────────
const recentMessages = computed(() => conversationStore.messages.slice(-20));

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
  const isExpanded = togglePetChat();
  if (isExpanded) {
    showBubble.value = false;
    nextTick(() => scrollToBottom());
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

function dismissOnboarding() {
  showOnboarding.value = false;
  try {
    localStorage.setItem(ONBOARDING_KEY, '1');
  } catch {
    // Ignore — tooltip will show again next session
  }
}

// Scroll on new messages
watch(
  () => conversationStore.messages.length,
  () => {
    if (petChatExpanded.value) {
      nextTick(() => scrollToBottom());
    }
    if (!petChatExpanded.value) {
      showBubble.value = true;
      setTimeout(() => {
        if (!petChatExpanded.value) showBubble.value = false;
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

// Keep character inside the viewport when the window resizes
function onWindowResize() {
  characterPos.value = clampToViewport(characterPos.value);
}

// ── Lifecycle ─────────────────────────────────────────────────────────────────
let unlistenLlmChunk: (() => void) | null = null;

onMounted(async () => {
  characterPos.value = loadSavedPosition();
  window.addEventListener('resize', onWindowResize);

  // Onboarding tooltip: show only if never dismissed before
  try {
    showOnboarding.value = !localStorage.getItem(ONBOARDING_KEY);
  } catch {
    showOnboarding.value = true;
  }

  // Collapse chat by default — the character is the focus in pet mode
  setPetChatExpanded(false);

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

  // Pet mode keeps click-through OFF: Tauri's set_ignore_cursor_events is
  // all-or-nothing for the whole window — toggling it on mouseenter/leave
  // creates a dead state where events are swallowed by the OS and the window
  // can never regain focus.  Instead the overlay captures all clicks; the
  // transparent areas show the desktop visually but don't forward clicks.
  windowStore.setCursorPassthrough(false);
});

onUnmounted(() => {
  window.removeEventListener('resize', onWindowResize);
  document.removeEventListener('mousemove', onDocMouseMove);
  if (holdTimer) clearTimeout(holdTimer);
  // Re-enable passthrough clean-up isn't needed — the window-mode change
  // back to desktop already makes the window fully interactive again.
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
  background: transparent;
  /* Pet-mode overlay is transparent; empty areas should let clicks through
   * to UI behind the overlay (e.g. the floating mode-toggle pill rendered
   * by App.vue).  Interactive children opt in with pointer-events: auto. */
  pointer-events: none;
}

.pet-overlay--dragging {
  cursor: grabbing;
}

.pet-character {
  position: absolute;
  pointer-events: auto;
  cursor: grab;
  user-select: none;
  -webkit-user-drag: none;
  will-change: left, top;
  transition: transform 0.15s ease-out;
}
.pet-overlay--dragging .pet-character {
  cursor: grabbing;
  transform: scale(1.01);
  transition: none;
}

/* ── Chat bubble ── */
.pet-bubble {
  position: absolute;
  max-width: 280px;
  width: fit-content;
  background: rgba(15, 23, 42, 0.92);
  border: 1px solid rgba(139, 92, 246, 0.3);
  border-radius: 16px 16px 16px 4px;
  padding: 12px 16px;
  color: #e2e8f0;
  font-size: 0.85rem;
  line-height: 1.4;
  cursor: pointer;
  backdrop-filter: blur(8px);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
  pointer-events: auto;
}
.pet-bubble-text { margin: 0; }

.bubble-enter-active,
.bubble-leave-active { transition: opacity 0.3s, transform 0.3s; }
.bubble-enter-from,
.bubble-leave-to { opacity: 0; transform: translateY(10px) scale(0.95); }

/* ── Expandable chat panel — anchored to left side so it never clips the character ── */
.pet-chat {
  position: absolute;
  bottom: 20px;
  left: 20px;
  width: 360px;
  max-height: 520px;
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

.pet-chat-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.pet-chat-title {
  font-size: 0.8rem;
  font-weight: 600;
  color: #cbd5e1;
  letter-spacing: 0.03em;
  text-transform: uppercase;
}
.pet-chat-close {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: none;
  background: rgba(255, 255, 255, 0.06);
  color: #e2e8f0;
  cursor: pointer;
  font-size: 1rem;
  line-height: 1;
}
.pet-chat-close:hover { background: rgba(255, 255, 255, 0.12); }

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
.pet-msg-text { color: #e2e8f0; }

.pet-thinking { animation: pet-pulse 1s ease-in-out infinite; }
@keyframes pet-pulse {
  0%, 100% { opacity: 0.4; }
  50%      { opacity: 1; }
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
.pet-chat-input input:focus { border-color: #6c63ff; }
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
.pet-chat-input button:disabled { opacity: 0.35; cursor: not-allowed; }

.chat-slide-enter-active,
.chat-slide-leave-active { transition: opacity 0.25s, transform 0.25s; }
.chat-slide-enter-from,
.chat-slide-leave-to { opacity: 0; transform: translateY(20px); }

/* ── Emotion badge ── */
.pet-emotion {
  position: absolute;
  font-size: 1.5rem;
  pointer-events: none;
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.45));
}

/* ── Onboarding ──
   Anchored bottom-left so it doesn't cover the character (who defaults to
   the bottom-right corner) and is clear of the top-right toggle pill. */
.pet-onboarding {
  position: absolute;
  bottom: 20px;
  left: 20px;
  max-width: 240px;
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(139, 92, 246, 0.35);
  border-radius: 12px;
  padding: 12px 14px;
  color: #e2e8f0;
  font-size: 0.78rem;
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(10px);
  pointer-events: auto;
  z-index: 100;
}
.pet-onboarding-title {
  margin: 0 0 8px;
  font-weight: 700;
  color: var(--ts-accent, #7c6fff);
}
.pet-onboarding-list {
  margin: 0 0 10px;
  padding-left: 18px;
  line-height: 1.5;
}
.pet-onboarding-dismiss {
  width: 100%;
  padding: 6px 10px;
  border-radius: 8px;
  border: none;
  background: #6c63ff;
  color: #fff;
  font-size: 0.78rem;
  cursor: pointer;
}
.pet-onboarding-dismiss:hover { background: #7c6fff; }

.fade-enter-active,
.fade-leave-active { transition: opacity 0.3s; }
.fade-enter-from,
.fade-leave-to { opacity: 0; }
</style>
