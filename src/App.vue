<template>
  <div
    class="app-shell"
    @mousedown="onMouseDown"
    @mouseup="onMouseUp"
    @contextmenu.prevent="onRightClick"
  >
    <!-- Brain onboarding overlay -->
    <div v-if="!hasBrain && !skipSetup" class="onboarding-overlay">
      <BrainSetupView @done="onBrainDone" />
    </div>

    <template v-else>
      <!-- ChatView handles the character + chat panel -->
      <ChatView ref="chatViewRef" class="chat-layer" />

      <!-- Memory panel overlay -->
      <Transition name="slide-up">
        <div v-if="showMemory" class="overlay-panel" @mousedown.stop @contextmenu.stop.prevent>
          <div class="panel-header">
            <span>🧠 Memory</span>
            <button class="panel-close" @click="showMemory = false">✕</button>
          </div>
          <MemoryView />
        </div>
      </Transition>

      <!-- Right-click context menu -->
      <Transition name="fade">
        <div
          v-if="showMenu"
          class="context-menu"
          :style="menuStyle"
          @mousedown.stop
          @contextmenu.stop.prevent
        >
          <button class="menu-item" @click="toggleChat">💬 Chat</button>
          <button class="menu-item" @click="openMemory">🧠 Memory</button>

          <!-- Mood submenu -->
          <button class="menu-item submenu-toggle" @click="showMoodMenu = !showMoodMenu">
            😊 Mood
            <span class="submenu-arrow" :class="{ open: showMoodMenu }">›</span>
          </button>
          <div v-if="showMoodMenu" class="submenu-list">
            <button
              v-for="s in moodStates"
              :key="s.state"
              class="submenu-item"
              :class="{ active: characterStore.state === s.state }"
              @click="setMood(s.state)"
            >{{ s.label }}</button>
          </div>

          <button class="menu-item" @click="openBrainSetup">⚙ Brain Setup</button>
          <div class="menu-divider" />
          <button class="menu-item danger" @click="closeApp">✕ Quit</button>
        </div>
      </Transition>

      <div v-if="showMenu" class="menu-backdrop" @mousedown="showMenu = false; showMoodMenu = false" />
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useBrainStore } from './stores/brain';
import { useCharacterStore } from './stores/character';
import type { CharacterState } from './types';
import ChatView from './views/ChatView.vue';
import MemoryView from './views/MemoryView.vue';
import BrainSetupView from './views/BrainSetupView.vue';

const brain = useBrainStore();
const characterStore = useCharacterStore();
const skipSetup = ref(false);
const hasBrain = computed(() => brain.hasBrain);

const chatViewRef = ref<InstanceType<typeof ChatView> | null>(null);
const showMemory = ref(false);
const showMenu = ref(false);
const showMoodMenu = ref(false);
const menuStyle = ref({ top: '0px', left: '0px' });

const moodStates: { state: CharacterState; label: string }[] = [
  { state: 'idle',      label: '💤 Idle' },
  { state: 'happy',     label: '😊 Happy' },
  { state: 'sad',       label: '😢 Sad' },
  { state: 'angry',     label: '😠 Angry' },
  { state: 'surprised', label: '😲 Surprised' },
  { state: 'shy',       label: '☺️ Shy' },
  { state: 'sitting',   label: '🪑 Sitting' },
];

// ── Drag detection ──────────────────────────────────────────────
const DRAG_MS = 250;
let pressTime = 0;
let dragTimer: ReturnType<typeof setTimeout> | null = null;
let didDrag = false;

function onMouseDown(e: MouseEvent) {
  if (e.button !== 0) return;
  if (showMenu.value) { showMenu.value = false; return; }
  didDrag = false;
  pressTime = Date.now();
  dragTimer = setTimeout(async () => {
    didDrag = true;
    await invoke('start_drag');
  }, DRAG_MS);
}

async function snapToEdge() {
  try {
    const s = await invoke<{ x: number; y: number; width: number; height: number; screen_width: number; screen_height: number }>('get_window_state');
    if (!s) return;
    const SNAP = 40; // px threshold to trigger snap
    let nx = s.x;
    let ny = s.y;
    if (s.x < SNAP) nx = 0;
    else if (s.x + s.width > s.screen_width - SNAP) nx = s.screen_width - s.width;
    if (s.y < SNAP) ny = 0;
    else if (s.y + s.height > s.screen_height - SNAP) ny = s.screen_height - s.height;
    if (nx !== s.x || ny !== s.y) await invoke('move_window', { x: nx, y: ny });
  } catch { /* no backend */ }
}

async function onMouseUp(e: MouseEvent) {
  if (e.button !== 0) return;
  if (dragTimer) { clearTimeout(dragTimer); dragTimer = null; }
  const elapsed = Date.now() - pressTime;
  if (!didDrag && elapsed < DRAG_MS) {
    chatViewRef.value?.toggleDialog();
  } else if (didDrag) {
    // Snap to edge after drag
    await snapToEdge();
  }
  didDrag = false;
}

// ── Right-click menu ────────────────────────────────────────────
function onRightClick(e: MouseEvent) {
  // Keep menu inside window bounds (taller when mood submenu might open)
  const menuW = 170, menuH = 300;
  const x = Math.min(e.clientX, window.innerWidth - menuW);
  const y = Math.min(e.clientY, window.innerHeight - menuH);
  menuStyle.value = { top: y + 'px', left: x + 'px' };
  showMoodMenu.value = false;
  showMenu.value = true;
}

function setMood(state: CharacterState) {
  characterStore.setState(state);
  showMenu.value = false;
  showMoodMenu.value = false;
}

function toggleChat() {
  chatViewRef.value?.toggleDialog();
  showMenu.value = false;
}

function openMemory() {
  showMemory.value = true;
  showMenu.value = false;
}

function openBrainSetup() {
  skipSetup.value = false;
  showMenu.value = false;
}

async function closeApp() {
  await invoke('close_window');
}

async function onBrainDone() {
  skipSetup.value = true;
}

onMounted(async () => {
  try {
    await brain.loadActiveBrain();
  } catch {
    skipSetup.value = true;
    return;
  }
  if (brain.hasBrain) skipSetup.value = true;
});
</script>

<style>
*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; background: transparent; color: #f1f5f9; font-family: system-ui, sans-serif; }
</style>

<style scoped>
.app-shell {
  position: relative;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  background: transparent;
  cursor: default;
  user-select: none;
}

.chat-layer {
  position: absolute;
  inset: 0;
  z-index: 1;
}

.onboarding-overlay {
  position: absolute;
  inset: 0;
  z-index: 100;
  background: #0f172a;
  overflow-y: auto;
}

.overlay-panel {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 65%;
  background: rgba(10, 15, 30, 0.95);
  backdrop-filter: blur(16px);
  border-top: 1px solid rgba(255,255,255,0.1);
  border-radius: 16px 16px 0 0;
  z-index: 20;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px 8px;
  border-bottom: 1px solid rgba(255,255,255,0.08);
  font-weight: 600;
  font-size: 0.9rem;
  flex-shrink: 0;
}

.panel-close {
  background: none;
  border: none;
  color: #94a3b8;
  cursor: pointer;
  font-size: 1rem;
  padding: 2px 6px;
  border-radius: 4px;
}
.panel-close:hover { background: rgba(255,255,255,0.1); color: #f1f5f9; }

.menu-backdrop { position: fixed; inset: 0; z-index: 29; }

.context-menu {
  position: fixed;
  z-index: 30;
  background: rgba(15, 23, 42, 0.97);
  backdrop-filter: blur(16px);
  border: 1px solid rgba(255,255,255,0.12);
  border-radius: 12px;
  padding: 6px;
  min-width: 160px;
  box-shadow: 0 8px 32px rgba(0,0,0,0.6);
}

.menu-item {
  display: block;
  width: 100%;
  padding: 9px 14px;
  background: none;
  border: none;
  color: #f1f5f9;
  font-size: 0.88rem;
  text-align: left;
  cursor: pointer;
  border-radius: 8px;
  transition: background 0.12s;
}
.menu-item:hover { background: rgba(255,255,255,0.1); }
.menu-item.danger { color: #f87171; }
.menu-item.danger:hover { background: rgba(248,113,113,0.12); }
.menu-divider { height: 1px; background: rgba(255,255,255,0.08); margin: 4px 0; }

.submenu-toggle { display: flex; justify-content: space-between; align-items: center; }
.submenu-arrow {
  font-size: 1.1rem;
  line-height: 1;
  transition: transform 0.15s;
  color: #94a3b8;
}
.submenu-arrow.open { transform: rotate(90deg); }

.submenu-list {
  padding: 2px 0 2px 8px;
  border-left: 2px solid rgba(255,255,255,0.1);
  margin: 0 6px 2px 10px;
}
.submenu-item {
  display: block;
  width: 100%;
  padding: 6px 10px;
  background: none;
  border: none;
  color: #cbd5e1;
  font-size: 0.83rem;
  text-align: left;
  cursor: pointer;
  border-radius: 6px;
  transition: background 0.1s;
}
.submenu-item:hover { background: rgba(255,255,255,0.08); color: #f1f5f9; }
.submenu-item.active { color: #818cf8; font-weight: 600; }

.slide-up-enter-active, .slide-up-leave-active { transition: transform 0.25s ease, opacity 0.25s ease; }
.slide-up-enter-from, .slide-up-leave-to { transform: translateY(100%); opacity: 0; }
.fade-enter-active, .fade-leave-active { transition: opacity 0.15s ease, transform 0.15s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; transform: scale(0.95); }
</style>
