<template>
  <div class="pet-overlay">
    <!-- Character container — positioned absolutely within the full-screen window.
         Dragging moves the CSS position.  Click toggles chat.  Right-click opens menu. -->
    <div
      ref="charRef"
      class="pet-character"
      :style="charStyle"
      @mousedown.capture="onCharacterMouseDown"
      @wheel.capture="onCharacterWheel"
      @contextmenu.prevent.stop="onCharacterContextMenu"
    >
      <CharacterViewport ref="viewportRef" />

      <!-- Floating chat bubble (shows recent message) -->
      <Transition name="bubble">
        <div
          v-if="showBubble && lastAssistantText && !petChatExpanded"
          class="pet-bubble"
          @click.stop="toggleChat"
        >
          <p class="pet-bubble-text">
            {{ truncatedMessage }}
          </p>
        </div>
      </Transition>

      <!-- Expandable chat panel — manga-style speech bubble positioned
           relative to model head so it never covers the character -->
      <Transition name="chat-slide">
        <div
          v-if="petChatExpanded"
          class="pet-chat"
          :class="petChatClasses"
          :style="petChatStyle"
          @click.stop
          @mousedown.stop
        >
          <div class="pet-chat-header">
            <span class="pet-chat-title">Chat</span>
            <div class="pet-chat-actions">
              <button
                class="pet-chat-action-btn"
                title="Copy chat history"
                @click.stop="copyChatHistoryToClipboard"
              >
                Copy
              </button>
              <button
                class="pet-chat-action-btn"
                title="Paste clipboard into input"
                @click.stop="pasteClipboardToInput"
              >
                Paste
              </button>
              <button
                v-if="canSkipDialog"
                class="pet-chat-action-btn skip"
                title="Skip dialog and TTS"
                @click.stop="skipCurrentDialog"
              >
                Skip
              </button>
              <button
                class="pet-chat-close"
                title="Close chat"
                @click.stop="closeChat"
              >
                ×
              </button>
            </div>
          </div>
          <div
            ref="messagesRef"
            class="pet-chat-messages"
          >
            <template
              v-for="(msg, idx) in recentMessages"
              :key="msg.id"
            >
              <div
                v-if="showDateSep(idx)"
                class="pet-date-sep"
              >
                {{ dateSepLabel(msg.timestamp) }}
              </div>
              <div :class="['pet-msg', msg.role]">
                <span class="pet-msg-text">{{ msg.content }}</span>
                <span class="pet-msg-time">{{ formatPetTime(msg.timestamp) }}</span>
              </div>
            </template>
            <div
              v-if="conversationStore.isThinking"
              class="pet-msg assistant"
            >
              <span class="pet-msg-text pet-thinking">…</span>
            </div>
            <div
              v-if="conversationStore.isStreaming && conversationStore.streamingText"
              class="pet-msg assistant"
            >
              <span class="pet-msg-text">{{ conversationStore.streamingText }}</span>
            </div>
          </div>
          <form
            class="pet-chat-input"
            @submit.prevent="handleSend"
          >
            <input
              v-model="inputText"
              type="text"
              placeholder="Say something…"
              :disabled="conversationStore.isThinking"
              autocomplete="off"
            >
            <button
              type="submit"
              :disabled="conversationStore.isThinking || !inputText.trim()"
            >
              ➤
            </button>
          </form>
        </div>
      </Transition>

      <!-- Onboarding tooltip — shown once on first use of pet mode -->
      <Transition name="fade">
        <div
          v-if="showOnboarding"
          class="pet-onboarding"
          @click.stop
          @mousedown.stop
        >
          <p class="pet-onboarding-title">
            Welcome to pet mode
          </p>
          <ul class="pet-onboarding-list">
            <li><strong>Click</strong> character to toggle chat</li>
            <li><strong>Drag</strong> to move</li>
            <li><strong>Scroll wheel</strong> to zoom in/out</li>
            <li><strong>Hold click + scroll</strong> to rotate</li>
            <li><strong>Middle-click drag</strong> to rotate</li>
            <li><strong>Right-click</strong> for menu (mood, settings…)</li>
          </ul>
          <button
            class="pet-onboarding-dismiss"
            @click.stop="dismissOnboarding"
          >
            Got it
          </button>
        </div>
      </Transition>

      <!-- Manga-style emotion speech bubble — positioned near model's head,
           flips to left side when character is near the right screen edge -->
      <Transition name="fade">
        <div
          v-if="characterStore.state !== 'idle'"
          class="pet-emotion-bubble"
          :class="{ 'pet-emotion-bubble--left': emotionOnLeft }"
          :style="emotionBubbleStyle"
        >
          <span class="pet-emotion-emoji">{{ emotionEmoji }}</span>
          <div class="pet-emotion-tail" />
        </div>
      </Transition>

      <!-- Resize handle — visible only when resize mode is toggled on via context menu -->
      <div
        v-if="resizeActive"
        class="pet-resize-handle"
        title="Drag to resize"
        @mousedown.stop.prevent="onResizeMouseDown"
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 18 18"
        >
          <path
            d="M14 4L4 14M14 8L8 14M14 12L12 14"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
        </svg>
      </div>
    </div>

    <!-- Right-click context menu — rendered on the full-screen overlay so it
         can extend beyond the character bounds in any direction. -->
    <PetContextMenu
      :visible="menuVisible"
      :x="menuX"
      :y="menuY"
      :resize-active="resizeActive"
      @close="menuVisible = false"
      @toggle-resize="resizeActive = !resizeActive"
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
import { useVoiceStore } from '../stores/voice';
import { useChatExpansion } from '../composables/useChatExpansion';
import { useTtsPlayback } from '../composables/useTtsPlayback';
import { useLipSyncBridge } from '../composables/useLipSyncBridge';
import { GENDER_VOICES } from '../config/default-models';
import type { CharacterState } from '../types';
import type { AvatarStateMachine } from '../renderer/avatar-state';
import { copyChatHistory, readClipboardText } from '../utils/chat-history-clipboard';
import * as THREE from 'three';
import CharacterViewport from '../components/CharacterViewport.vue';
import PetContextMenu from '../components/PetContextMenu.vue';

const conversationStore = useConversationStore();
const characterStore = useCharacterStore();
const brain = useBrainStore();
const windowStore = useWindowStore();
const streaming = useStreamingStore();
const voice = useVoiceStore();
const { petChatExpanded, setPetChatExpanded, togglePetChat } = useChatExpansion();

// ── TTS + LipSync (same pipeline as ChatView) ────────────────────────────────
const tts = useTtsPlayback({
  getBrowserPitch: () => GENDER_VOICES[characterStore.currentGender()].browserPitch,
  getBrowserRate: () => GENDER_VOICES[characterStore.currentGender()].browserRate,
});

function getAsm(): AvatarStateMachine | null {
  return viewportRef.value?.avatarStateMachine ?? null;
}
const lipSyncBridge = useLipSyncBridge(tts, getAsm);

function sentimentToState(sentiment?: string): CharacterState {
  switch (sentiment) {
    case 'happy': return 'happy';
    case 'sad': return 'sad';
    case 'angry': return 'angry';
    case 'relaxed': return 'relaxed';
    case 'surprised': return 'surprised';
    default: return 'talking';
  }
}

function setAvatarState(charState: CharacterState): void {
  // When a VRMA mood animation is actively playing (e.g. angry.vrma),
  // don't let transient states like 'talking' override the emotion or
  // trigger mood watcher state changes that would kill the animation.
  const vrmaActive = viewportRef.value?.isAnimationActive ?? false;
  if (vrmaActive && (charState === 'talking' || charState === 'idle' || charState === 'thinking')) {
    return;
  }
  characterStore.setState(charState);
  const asm = getAsm();
  if (!asm) return;
  switch (charState) {
    case 'idle':      asm.forceBody('idle');  asm.setEmotion('neutral');   break;
    case 'thinking':  asm.forceBody('think'); asm.setEmotion('neutral');   break;
    case 'talking':   asm.forceBody('talk');  asm.setEmotion('neutral');   break;
    case 'happy':     asm.setEmotion('happy');     break;
    case 'sad':       asm.setEmotion('sad');       break;
    case 'angry':     asm.setEmotion('angry');     break;
    case 'relaxed':   asm.setEmotion('relaxed');   break;
    case 'surprised': asm.setEmotion('surprised'); break;
  }
}

// ── UI state ──────────────────────────────────────────────────────────────────
const inputText = ref('');
const showBubble = ref(false);
const showOnboarding = ref(false);
const messagesRef = ref<HTMLElement | null>(null);
const charRef = ref<HTMLElement | null>(null);
const viewportRef = ref<InstanceType<typeof CharacterViewport> | null>(null);

const ONBOARDING_KEY = 'ts.pet.onboarded';
const POS_KEY = 'ts.pet.charPos';
const SIZE_KEY = 'ts.pet.charSize';

// ── Character position within the full-screen window ──────────────────────────
// Starts bottom-right of the primary monitor, can be dragged anywhere.
const charX = ref(0);
const charY = ref(0);
const charW = ref(350);
const charH = ref(500);
const resizeActive = ref(false);

const charStyle = computed(() => ({
  left: `${charX.value}px`,
  top: `${charY.value}px`,
  width: `${charW.value}px`,
  height: `${charH.value}px`,
}));

function initCharPosition() {
  // Restore saved size
  try {
    const savedSize = localStorage.getItem(SIZE_KEY);
    if (savedSize) {
      const sz = JSON.parse(savedSize);
      charW.value = sz.w ?? 350;
      charH.value = sz.h ?? 500;
    }
  } catch { /* use default */ }
  // Restore saved position
  try {
    const saved = localStorage.getItem(POS_KEY);
    if (saved) {
      const pos = JSON.parse(saved);
      charX.value = pos.x ?? 0;
      charY.value = pos.y ?? 0;
      return;
    }
  } catch { /* use default */ }
  // Default: bottom-right corner of the viewport
  charX.value = Math.max(0, window.innerWidth - charW.value - 40);
  charY.value = Math.max(0, window.innerHeight - charH.value - 40);
}

function saveCharPosition() {
  try {
    localStorage.setItem(POS_KEY, JSON.stringify({ x: charX.value, y: charY.value }));
  } catch { /* best-effort */ }
}

function saveCharSize() {
  try {
    localStorage.setItem(SIZE_KEY, JSON.stringify({ w: charW.value, h: charH.value }));
  } catch { /* best-effort */ }
}
// ── Resize handling ─────────────────────────────────────────────────────────────────
// Bottom-right drag handle to change character container dimensions.
const MIN_SIZE = 150;
const MAX_SIZE = 1200;
let resizeStartX = 0;
let resizeStartY = 0;
let resizeStartW = 0;
let resizeStartH = 0;

function onResizeMouseDown(e: MouseEvent) {
  resizeStartX = e.clientX;
  resizeStartY = e.clientY;
  resizeStartW = charW.value;
  resizeStartH = charH.value;
  document.addEventListener('mousemove', onResizeMouseMove);
  document.addEventListener('mouseup', onResizeMouseUp, { once: true });
}

function onResizeMouseMove(e: MouseEvent) {
  const dx = e.clientX - resizeStartX;
  const dy = e.clientY - resizeStartY;
  charW.value = Math.min(MAX_SIZE, Math.max(MIN_SIZE, resizeStartW + dx));
  charH.value = Math.min(MAX_SIZE, Math.max(MIN_SIZE, resizeStartH + dy));
}

function onResizeMouseUp() {
  document.removeEventListener('mousemove', onResizeMouseMove);
  saveCharSize();
}
// ── Drag handling ─────────────────────────────────────────────────────────────
// Quick click = toggle chat.  Drag moves the character within the full-screen
// transparent window (CSS-level movement, not OS window drag).
const CLICK_MOVE_TOLERANCE = 6;

let pressStartX = 0;
let pressStartY = 0;
let charStartX = 0;
let charStartY = 0;
let pressStartTime = 0;
let pressActive = false;
let dragStarted = false;

function onCharacterMouseDown(e: MouseEvent) {
  // Only handle left-click; let middle-click pass through to canvas for OrbitControls rotation
  if (e.button !== 0) return;

  // Don't process events originating from the chat panel, bubble, onboarding, or resize handle
  const target = e.target as HTMLElement;
  const interactiveChild = target.closest('.pet-chat, .pet-bubble, .pet-onboarding, .pet-resize-handle');
  if (interactiveChild) return;

  e.stopPropagation();
  pressActive = true;
  dragStarted = false;
  pressStartX = e.clientX;
  pressStartY = e.clientY;
  charStartX = charX.value;
  charStartY = charY.value;
  pressStartTime = performance.now();

  document.addEventListener('mousemove', onDocMouseMove);
  document.addEventListener('mouseup', onDocMouseUp, { once: true });
}

function onDocMouseMove(e: MouseEvent) {
  if (!pressActive) return;
  const dx = e.clientX - pressStartX;
  const dy = e.clientY - pressStartY;

  if (!dragStarted && Math.hypot(dx, dy) > CLICK_MOVE_TOLERANCE) {
    dragStarted = true;
  }
  if (dragStarted) {
    charX.value = charStartX + dx;
    charY.value = charStartY + dy;
  }
}

function onDocMouseUp(e: MouseEvent) {
  document.removeEventListener('mousemove', onDocMouseMove);
  if (!pressActive) return;

  const dx = e.clientX - pressStartX;
  const dy = e.clientY - pressStartY;
  const duration = performance.now() - pressStartTime;
  const wasDrag = dragStarted;

  pressActive = false;
  dragStarted = false;

  if (wasDrag) {
    saveCharPosition();
  } else if (duration < 300 && Math.hypot(dx, dy) <= CLICK_MOVE_TOLERANCE) {
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

// ── Left-click + scroll wheel = rotate ────────────────────────────────────────
// When the user holds the left mouse button and scrolls the wheel, rotate the
// camera azimuth.  This is the pet-mode rotation gesture since left-drag moves
// the character and middle-click may not be available on all mice.
function onCharacterWheel(e: WheelEvent) {
  // Any wheel interaction (zoom or rotate) changes the head's screen position
  scheduleHeadScan();
  // Also rescan after OrbitControls damping animation settles
  setTimeout(scheduleHeadScan, 300);

  if (!pressActive) return; // Only rotate when left button is held
  e.preventDefault();
  e.stopPropagation();
  const ctx = viewportRef.value?.sceneContext;
  if (!ctx) return;
  // deltaY > 0 = scroll down = rotate clockwise
  const rotateSpeed = 0.005;
  const angle = e.deltaY * rotateSpeed;
  // Rotate camera around the orbit target
  const offset = ctx.camera.position.clone().sub(ctx.controls.target);
  const sph = new THREE.Spherical().setFromVector3(offset);
  sph.theta += angle;
  offset.setFromSpherical(sph);
  ctx.camera.position.copy(ctx.controls.target).add(offset);
  ctx.controls.update();
}

// ── Cursor tracking + click-through ─────────────────
// Rust polls OS cursor position (~33 Hz) and emits `pet-cursor-pos` events.
// We check if the cursor is inside any interactive element (character, menu,
// chat panel).  If yes → passthrough OFF (accept clicks).  If no → passthrough
// ON (clicks fall through to the desktop underneath).
let unlistenCursorPos: (() => void) | null = null;
let lastPassthrough = true;

/** Check if a point is over a visible (non-transparent) part of the 3D model
 *  by reading the canvas pixel alpha at the cursor position, or over any
 *  interactive UI overlay (chat panel, bubble, onboarding, resize handle). */
function isPointOverInteractive(x: number, y: number): boolean {
  const el = charRef.value;
  if (!el) return false;
  const charRect = el.getBoundingClientRect();

  // First check if the cursor is over any overlay UI child (chat, bubble, etc.)
  // These are opaque DOM elements that should always be interactive.
  const overlays = el.querySelectorAll(
    '.pet-bubble, .pet-chat, .pet-onboarding, .pet-resize-handle',
  );
  for (const overlay of overlays) {
    const r = overlay.getBoundingClientRect();
    if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) {
      return true;
    }
  }

  // Not over a UI overlay within the container — but the chat panel may
  // extend outside the character bounds (manga-style speech bubble).
  // Check the pet-chat element directly in case it's positioned outside.
  const chatPanel = el.querySelector('.pet-chat');
  if (chatPanel) {
    const chatRect = chatPanel.getBoundingClientRect();
    if (x >= chatRect.left && x <= chatRect.right && y >= chatRect.top && y <= chatRect.bottom) {
      return true;
    }
  }

  // Not over a UI overlay — check if over the bounding rect at all
  if (x < charRect.left || x > charRect.right || y < charRect.top || y > charRect.bottom) {
    return false;
  }

  // Inside the bounding rect — read the canvas pixel alpha to see if the
  // cursor is over a visible part of the 3D model (not transparent background).
  const canvas = el.querySelector('canvas');
  if (!canvas) return true; // fallback: treat entire rect as interactive
  const canvasRect = canvas.getBoundingClientRect();
  const cx = x - canvasRect.left;
  const cy = y - canvasRect.top;
  if (cx < 0 || cy < 0 || cx >= canvasRect.width || cy >= canvasRect.height) {
    return false;
  }

  const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
  if (!gl) return true; // fallback
  const dpr = window.devicePixelRatio || 1;
  const px = Math.round(cx * dpr);
  const py = Math.round((canvasRect.height - cy) * dpr); // WebGL y is flipped
  const pixel = new Uint8Array(4);
  gl.readPixels(px, py, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
  return pixel[3] > 10; // alpha > threshold → over the model
}

function handleCursorPos(payload: { x: number; y: number; inside: boolean }) {
  // When the context menu is open its backdrop covers the whole window,
  // so we always need to accept clicks to let the user dismiss/use it.
  if (menuVisible.value) {
    if (lastPassthrough) {
      windowStore.setCursorPassthrough(false);
      lastPassthrough = false;
    }
    return;
  }

  const { x, y, inside } = payload;
  if (!inside) {
    // Cursor is completely outside our window — passthrough ON.
    if (!lastPassthrough) {
      windowStore.setCursorPassthrough(true);
      lastPassthrough = true;
    }
    return;
  }

  // Check if cursor is over a visible part of the model or a UI overlay
  const overInteractive = isPointOverInteractive(x, y);

  if (overInteractive && lastPassthrough) {
    windowStore.setCursorPassthrough(false);
    lastPassthrough = false;
  } else if (!overInteractive && !lastPassthrough) {
    windowStore.setCursorPassthrough(true);
    lastPassthrough = true;
  }
}

// ── Chat ──────────────────────────────────────────────────────────────────────
const recentMessages = computed(() => conversationStore.messages.slice(-20));

function formatPetTime(ts: number): string {
  return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function dateSepLabel(ts: number): string {
  const date = new Date(ts);
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const msgDay = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  const diffDays = Math.round((today.getTime() - msgDay.getTime()) / 86_400_000);
  if (diffDays === 0) return 'Today';
  if (diffDays === 1) return 'Yesterday';
  if (diffDays < 7) return date.toLocaleDateString([], { weekday: 'long' });
  return date.toLocaleDateString([], { weekday: 'long', month: 'long', day: 'numeric' });
}

function showDateSep(idx: number): boolean {
  const msgs = recentMessages.value;
  if (idx === 0) return true;
  const prev = new Date(msgs[idx - 1].timestamp);
  const curr = new Date(msgs[idx].timestamp);
  return prev.getFullYear() !== curr.getFullYear()
    || prev.getMonth() !== curr.getMonth()
    || prev.getDate() !== curr.getDate();
}

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

// ── Emotion bubble positioning ────────────────────────────────────────────────
// Project the VRM head bone to screen coordinates via the Three.js camera.
// This is model-independent, works at any zoom, and doesn't need a rendered frame.
const headY = ref(0.15); // fraction of charH (0 = top, 1 = bottom)
const headScreenX = ref(0.5); // fraction of charW (0 = left, 1 = right)

// Scratch vector for 3D→2D projection (reused to avoid GC)
const _projVec = new THREE.Vector3();

function updateHeadPosition() {
  const vrm = (window as unknown as Record<string, unknown>).__terransoul_vrm__ as
    { humanoid?: { getNormalizedBoneNode(name: string): { getWorldPosition(v: THREE.Vector3): THREE.Vector3 } | null } } | undefined;
  const ctx = viewportRef.value?.sceneContext;
  if (!vrm?.humanoid || !ctx) return;

  const headBone = vrm.humanoid.getNormalizedBoneNode('head');
  if (!headBone) return;

  headBone.getWorldPosition(_projVec);
  _projVec.project(ctx.camera);
  // NDC: x,y are -1..+1 where (-1,-1) is bottom-left and (1,1) is top-right
  // Convert to 0..1 fractions of container (0,0 = top-left in CSS)
  headScreenX.value = Math.max(0.1, Math.min(0.9, (_projVec.x + 1) / 2));
  // y: NDC +1 = top, but CSS 0 = top, so invert. Subtract a small offset
  // to position the bubble at forehead/hat level rather than bone center.
  const rawY = (1 - _projVec.y) / 2;
  headY.value = Math.max(0.02, Math.min(0.6, rawY - 0.06));
}

// Update head position when emotion activates, model loads, zoom/rotate changes
let headScanTimer: ReturnType<typeof setTimeout> | null = null;
function scheduleHeadScan() {
  if (headScanTimer) clearTimeout(headScanTimer);
  headScanTimer = setTimeout(updateHeadPosition, 60);
}
watch(() => characterStore.state, (s) => { if (s !== 'idle') scheduleHeadScan(); });
// When model finishes loading (isLoading transitions false), rescan after the
// camera reframes and the first render completes.  Multiple delays catch
// different model load speeds and camera animation settling.
watch(() => characterStore.isLoading, (loading) => {
  if (!loading) {
    setTimeout(scheduleHeadScan, 300);
    setTimeout(scheduleHeadScan, 800);
    setTimeout(scheduleHeadScan, 1500);
  }
});

// Hook into OrbitControls 'change' event so the bubble tracks during zoom damping
let orbitChangeCleanup: (() => void) | null = null;
function hookOrbitControls() {
  if (orbitChangeCleanup) orbitChangeCleanup();
  const ctx = viewportRef.value?.sceneContext;
  if (!ctx) return;
  const handler = () => scheduleHeadScan();
  ctx.controls.addEventListener('change', handler);
  orbitChangeCleanup = () => ctx.controls.removeEventListener('change', handler);
}

/** Whether the emotion bubble should appear on the left side of the character.
 *  Uses monitor-aware bounds so it flips at the current screen's right edge,
 *  not the full spanning window's right edge. */
const emotionOnLeft = computed(() => {
  const monitors = windowStore.monitors;
  const dpr = window.devicePixelRatio || 1;
  const charRight = charX.value + charW.value + 60;
  if (!monitors.length) {
    return charRight > (typeof window !== 'undefined' ? window.innerWidth : 1920);
  }
  // Find which monitor contains the character's center
  const minX = Math.min(...monitors.map(m => m.x));
  const minY = Math.min(...monitors.map(m => m.y));
  const charCenterPhysX = (charX.value + charW.value / 2) * dpr + minX;
  const charCenterPhysY = (charY.value + charH.value / 2) * dpr + minY;
  let monitor = monitors[0];
  for (const m of monitors) {
    if (charCenterPhysX >= m.x && charCenterPhysX < m.x + m.width &&
        charCenterPhysY >= m.y && charCenterPhysY < m.y + m.height) {
      monitor = m;
      break;
    }
  }
  const monRight = (monitor.x + monitor.width - minX) / dpr;
  return charRight > monRight;
});

const emotionBubbleStyle = computed(() => {
  // The bubble is 48px wide with transform: translateX(-50%).
  // Position it beside the head bone's projected screen position.
  // A fixed pixel offset from the head center works because the head bone
  // is always at the skull center — no body/hat/hair contamination.
  const OFFSET = 80; // px from head center to bubble center (~half head width + gap)
  const containerW = charW.value || 350;
  const headCenterPx = headScreenX.value * containerW;
  if (emotionOnLeft.value) {
    return {
      top: `${headY.value * 100}%`,
      left: `${Math.max(0, headCenterPx - OFFSET)}px`,
    };
  } else {
    return {
      top: `${headY.value * 100}%`,
      left: `${Math.min(containerW, headCenterPx + OFFSET)}px`,
    };
  }
});

// ── Chat panel positioning — manga-style speech bubble ────────────────────────
// Places the chat panel beside the character model (like a manga dialog) so
// it never covers the character.  Flips side based on screen-edge proximity
// and flips vertically when the character is near the top of the screen.

/** Whether the chat bubble should appear on the left side of the character. */
const chatOnLeft = computed(() => {
  // Chat panel is ~300px wide.  If the character's right side + panel width
  // exceeds the available screen width, flip to the left.
  const monitors = windowStore.monitors ?? [];
  const dpr = window.devicePixelRatio || 1;
  const CHAT_WIDTH = 300;
  const charRight = charX.value + charW.value + CHAT_WIDTH + 16;
  if (!monitors.length) {
    return charRight > (typeof window !== 'undefined' ? window.innerWidth : 1920);
  }
  const minX = Math.min(...monitors.map(m => m.x));
  const minY = Math.min(...monitors.map(m => m.y));
  const charCenterPhysX = (charX.value + charW.value / 2) * dpr + minX;
  const charCenterPhysY = (charY.value + charH.value / 2) * dpr + minY;
  let monitor = monitors[0];
  for (const m of monitors) {
    if (charCenterPhysX >= m.x && charCenterPhysX < m.x + m.width &&
        charCenterPhysY >= m.y && charCenterPhysY < m.y + m.height) {
      monitor = m;
      break;
    }
  }
  const monRight = (monitor.x + monitor.width - minX) / dpr;
  return charRight > monRight;
});

const petChatClasses = computed(() => ({
  'pet-chat--left': chatOnLeft.value,
}));

const petChatStyle = computed(() => {
  // Anchor the chat beside the character's head bone, like a speech bubble.
  // Clamp vertically so the panel never extends beyond the visible monitor.
  const CHAT_MAX_H = 400; // matches CSS max-height
  const headTopPx = headY.value * charH.value;
  const chatAbsTop = charY.value + headTopPx;
  const screenH = typeof window !== 'undefined' ? window.innerHeight : 1080;

  // If the chat would overflow the bottom of the screen, shift it up
  let topPx = headTopPx - (charH.value * 0.05);
  if (chatAbsTop + CHAT_MAX_H > screenH - 16) {
    topPx = Math.max(0, screenH - 16 - CHAT_MAX_H - charY.value);
  }
  // Also ensure it doesn't go above the screen
  if (charY.value + topPx < 0) {
    topPx = -charY.value;
  }

  const style: Record<string, string> = {
    top: `${topPx}px`,
  };
  if (chatOnLeft.value) {
    // Position to the left of the character container
    style.right = '100%';
    style.left = 'auto';
    style.marginRight = '12px';
  } else {
    // Position to the right of the character container
    style.left = '100%';
    style.right = 'auto';
    style.marginLeft = '12px';
  }
  return style;
});

function toggleChat() {
  // If chat is already open, don't close it — only open or toggle from closed state
  if (petChatExpanded.value) return;
  const isExpanded = togglePetChat();
  if (isExpanded) {
    showBubble.value = false;
    scheduleHeadScan(); // Update head position so the chat bubble is placed correctly
    nextTick(() => scrollToBottom());
  }
}

function closeChat() {
  if (petChatExpanded.value) {
    setPetChatExpanded(false);
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

  // Stop any ongoing TTS before sending a new message
  tts.stop();

  setAvatarState('thinking');
  await conversationStore.sendMessage(text);

  const lastMsg = conversationStore.messages[conversationStore.messages.length - 1];
  const reactionState = lastMsg?.role === 'assistant'
    ? sentimentToState(lastMsg.sentiment)
    : 'idle';

  setAvatarState(reactionState);

  if (lastMsg?.role === 'assistant') {
    // Trigger VRMA body animation from the LLM's motion tag (browser-side path)
    if (lastMsg.motion) {
      viewportRef.value?.playMotion(lastMsg.motion);
    }
    // Speak non-streamed responses (quest messages, fallback)
    if (lastMsg.questChoices?.length) {
      tts.stop();
      tts.feedChunk(lastMsg.content);
      tts.flush();
    }
  }

  nextTick(() => scrollToBottom());

  setTimeout(() => {
    if (!tts.isSpeaking.value) {
      setAvatarState('idle');
      viewportRef.value?.stopMotion?.();
    }
  }, 6000);
}

const canSkipDialog = computed(
  () => conversationStore.isThinking || conversationStore.isStreaming || tts.isSpeaking.value,
);

function skipCurrentDialog() {
  tts.stop();
  streamTtsActive = false;
  streaming.reset();
  conversationStore.isStreaming = false;
  conversationStore.streamingText = '';
  setAvatarState('idle');
  viewportRef.value?.stopMotion?.();
}

async function copyChatHistoryToClipboard() {
  try {
    await copyChatHistory(conversationStore.messages);
  } catch {
    // Clipboard unavailable
  }
}

async function pasteClipboardToInput() {
  try {
    const text = (await readClipboardText()).trim();
    if (!text) return;
    inputText.value = text;
  } catch {
    // Clipboard unavailable
  }
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
      setAvatarState(sentimentToState(emotion));
      setTimeout(() => setAvatarState('idle'), 6000);
    }
  },
);

// Show 'thinking' animation during thinking
watch(
  () => conversationStore.isThinking,
  (thinking) => {
    if (thinking) setAvatarState('thinking');
  },
);

// Show 'talking' animation during streaming
watch(
  () => conversationStore.isStreaming,
  (active) => {
    if (active) {
      setAvatarState('talking');
    } else if (streaming.currentEmotion) {
      setAvatarState(sentimentToState(streaming.currentEmotion));
    }
  },
);

// TTS speaking state → body='talk', done → body='idle'
watch(tts.isSpeaking, (speaking) => {
  // Don't override state when a VRMA mood animation is active
  if (viewportRef.value?.isAnimationActive) return;
  const asm = getAsm();
  if (!asm) return;
  if (speaking) {
    asm.forceBody('talk');
    characterStore.setState('talking');
  } else {
    asm.forceBody('idle');
    characterStore.setState('idle');
  }
});

// ── Lifecycle ─────────────────────────────────────────────────────────────────
let unlistenLlmChunk: (() => void) | null = null;
let unlistenLlmAnimation: (() => void) | null = null;
let streamTtsActive = false;

onMounted(async () => {
  // Onboarding tooltip: show only if never dismissed before
  try {
    showOnboarding.value = !localStorage.getItem(ONBOARDING_KEY);
  } catch {
    showOnboarding.value = true;
  }

  // Collapse chat by default — the character is the focus in pet mode
  setPetChatExpanded(false);

  // Position the character (from localStorage or default bottom-right)
  initCharPosition();

  // Span the window across all monitors so the character can be dragged
  // anywhere and the context menu can appear without clipping.
  // full-screen transparent window
  // with cursor-tracking for click-through on blank areas.
  windowStore.spanAllMonitors();
  // Load monitor info so the context menu and emotion bubble can detect
  // which physical screen the cursor is on and avoid crossing monitor edges.
  windowStore.loadMonitors();

  try {
    await brain.loadActiveBrain();
  } catch {
    // Only auto-configure if brain isn't already set (avoid overwriting local_ollama/paid_api)
    if (!brain.hasBrain) {
      brain.autoConfigureFreeApi();
    }
  }

  try {
    const { listen } = await import('@tauri-apps/api/event');
    unlistenLlmChunk = await listen<{ text: string; done: boolean }>('llm-chunk', (event) => {
      streaming.handleChunk(event.payload);

      // Feed text into TTS (same as ChatView)
      if (voice.config.tts_provider) {
        if (event.payload.done) {
          tts.flush();
          streamTtsActive = false;
        } else if (event.payload.text) {
          if (!streamTtsActive) {
            // New AI response started: stop previous speech and only speak latest.
            tts.stop();
            streamTtsActive = true;
          }
          tts.feedChunk(event.payload.text);
        }
      }
    });

    // Animation stream — structured JSON from Rust parser (same as ChatView)
    unlistenLlmAnimation = await listen<{ emotion?: string; motion?: string }>('llm-animation', (event) => {
      streaming.handleAnimation(event.payload);

      if (event.payload.emotion) {
        const state = sentimentToState(event.payload.emotion);
        if (state !== 'idle') {
          setAvatarState(state);
        }
      }

      if (event.payload.motion) {
        viewportRef.value?.playMotion(event.payload.motion);
      }
    });

    // Start cursor-position polling from Rust.  On each event we decide
    // whether the cursor is over an interactive component and toggle
    // set_ignore_cursor_events accordingly.
    unlistenCursorPos = await listen<{ x: number; y: number; inside: boolean }>(
      'pet-cursor-pos',
      (event) => handleCursorPos(event.payload),
    );
    windowStore.startPetCursorPoll();
    // Default: pass-through ON so clicks on empty space go to the desktop.
    windowStore.setCursorPassthrough(true);
    lastPassthrough = true;
  } catch {
    // No Tauri — browser fallback: passthrough off so we can interact.
    windowStore.setCursorPassthrough(false);
    lastPassthrough = false;
  }

  // Hook into OrbitControls once the viewport is ready so the emotion
  // bubble tracks head position during zoom/rotate damping.
  setTimeout(hookOrbitControls, 500);

  // Initialise voice store so TTS provider is available
  try {
    await voice.initialise();
  } catch {
    // No Tauri backend — voice stays in text-only mode
  }

  // Start the LipSync ↔ TTS bridge
  lipSyncBridge.start();
});

onUnmounted(() => {
  document.removeEventListener('mousemove', onDocMouseMove);
  if (orbitChangeCleanup) { orbitChangeCleanup(); orbitChangeCleanup = null; }
  windowStore.stopPetCursorPoll();
  if (unlistenCursorPos) {
    unlistenCursorPos();
    unlistenCursorPos = null;
  }
  if (unlistenLlmChunk) {
    unlistenLlmChunk();
    unlistenLlmChunk = null;
  }
  if (unlistenLlmAnimation) {
    unlistenLlmAnimation();
    unlistenLlmAnimation = null;
  }
  tts.stop();
  lipSyncBridge.dispose();
  // Restore non-passthrough so the next mode isn't stuck.
  windowStore.setCursorPassthrough(false);
});
</script>

<style scoped>
.pet-overlay {
  position: fixed;
  inset: 0;
  overflow: visible;
  background: transparent;
  /* The full-screen overlay must not capture events by default — only the
     character container and its children should be interactive. */
  pointer-events: none;
}

.pet-character {
  position: absolute;
  cursor: grab;
  user-select: none;
  -webkit-user-drag: none;
  overflow: visible;
  /* Re-enable pointer events on the character container itself */
  pointer-events: auto;
}
.pet-character:active {
  cursor: grabbing;
}

/* ── Chat bubble ── */
.pet-bubble {
  position: absolute;
  top: 0;
  left: 50%;
  transform: translate(-50%, -100%);
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
}
.pet-bubble-text { margin: 0; }

.bubble-enter-active,
.bubble-leave-active { transition: opacity 0.3s, transform 0.3s; }
.bubble-enter-from,
.bubble-leave-to { opacity: 0; transform: translate(-50%, -90%) scale(0.95); }

/* ── Expandable chat panel — manga-style speech bubble beside character ── */
.pet-chat {
  position: absolute;
  /* top/left/right set dynamically via :style based on head bone position */
  width: 300px;
  max-height: 400px;
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(139, 92, 246, 0.25);
  border-radius: 16px;
  display: flex;
  flex-direction: column;
  backdrop-filter: blur(12px);
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.4);
  overflow: visible;
  z-index: 10;
}
/* Speech bubble tail — triangle pointing toward the character */
.pet-chat::after {
  content: '';
  position: absolute;
  top: 24px;
  width: 0;
  height: 0;
  border-top: 8px solid transparent;
  border-bottom: 8px solid transparent;
  /* Default: tail points left (chat is on the right side) */
  left: -8px;
  border-right: 10px solid rgba(15, 23, 42, 0.95);
}
/* When chat is on the left side, flip the tail to point right */
.pet-chat--left::after {
  left: auto;
  right: -8px;
  border-right: none;
  border-left: 10px solid rgba(15, 23, 42, 0.95);
}

.pet-chat-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 16px 16px 0 0;
  background: rgba(15, 23, 42, 0.95);
}
.pet-chat-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}
.pet-chat-action-btn {
  border: 1px solid rgba(255, 255, 255, 0.16);
  background: rgba(255, 255, 255, 0.08);
  color: #e2e8f0;
  font-size: 0.66rem;
  font-weight: 700;
  border-radius: 8px;
  padding: 3px 7px;
  cursor: pointer;
}
.pet-chat-action-btn:hover {
  background: rgba(255, 255, 255, 0.16);
}
.pet-chat-action-btn.skip {
  border-color: rgba(239, 68, 68, 0.45);
  color: #fca5a5;
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
  max-height: 280px;
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

.pet-msg-time {
  display: block;
  font-size: 0.62rem;
  color: rgba(203, 213, 225, 0.5);
  margin-top: 2px;
}
.pet-msg.user .pet-msg-time { text-align: right; }

.pet-date-sep {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
}
.pet-date-sep::before,
.pet-date-sep::after {
  content: '';
  flex: 1;
  height: 1px;
  background: rgba(255, 255, 255, 0.08);
}
.pet-date-sep {
  font-size: 0.6rem;
  font-weight: 600;
  color: rgba(203, 213, 225, 0.5);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  white-space: nowrap;
}

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
.chat-slide-leave-to { opacity: 0; transform: scale(0.92); }

/* ── Manga-style emotion speech bubble ── */
.pet-emotion-bubble {
  position: absolute;
  /* top and left are set dynamically via :style */
  transform: translateX(-50%) translateY(-50%);
  background: #fff;
  border-radius: 50%;
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.25);
  pointer-events: none;
  z-index: 5;
}
/* Flip tail to the other side when bubble is on the left */
.pet-emotion-bubble--left {
  /* No extra positioning needed — left is set dynamically */
}
.pet-emotion-emoji {
  font-size: 1.5rem;
  line-height: 1;
}
.pet-emotion-tail {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 0;
  height: 0;
  border-top: 6px solid transparent;
  border-bottom: 6px solid transparent;
  /* Default: bubble is to the right, tail points left toward head */
  left: -6px;
  border-right: 8px solid #fff;
}
/* When bubble is on the left, tail points right toward head */
.pet-emotion-bubble--left .pet-emotion-tail {
  left: auto;
  right: -6px;
  border-right: none;
  border-left: 8px solid #fff;
}

/* ── Onboarding ── */
.pet-onboarding {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  max-width: 240px;
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(139, 92, 246, 0.35);
  border-radius: 12px;
  padding: 12px 14px;
  color: #e2e8f0;
  font-size: 0.78rem;
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(10px);
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

/* ── Resize handle ── */
.pet-resize-handle {
  position: absolute;
  bottom: 2px;
  right: 2px;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: nwse-resize;
  color: rgba(139, 92, 246, 0.7);
  background: rgba(15, 23, 42, 0.7);
  border-radius: 6px 0 0 0;
  border-left: 1px solid rgba(139, 92, 246, 0.3);
  border-top: 1px solid rgba(139, 92, 246, 0.3);
  z-index: 20;
  transition: color 0.15s, background 0.15s;
}
.pet-resize-handle:hover {
  color: rgba(139, 92, 246, 1);
  background: rgba(15, 23, 42, 0.9);
}
</style>
