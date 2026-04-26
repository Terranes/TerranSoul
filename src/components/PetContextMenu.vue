<template>
  <Teleport to="body">
    <Transition name="ctx-fade">
      <div
        v-if="visible"
        class="pet-ctx-backdrop"
        @click="close"
        @contextmenu.prevent="close"
      >
        <div
          ref="menuRef"
          class="pet-ctx-menu"
          :style="menuStyle"
          role="menu"
          @click.stop
          @contextmenu.prevent.stop
        >
          <!-- Mood toggle — expands inline rather than as a flyout submenu
               so it never clips outside the compact pet window. -->
          <div
            class="ctx-item"
            role="menuitem"
            :aria-expanded="moodOpen"
            @click="moodOpen = !moodOpen"
          >
            <span class="ctx-icon">🎭</span>
            <span class="ctx-label">Mood</span>
            <span
              class="ctx-chev"
              :class="{ 'ctx-chev--open': moodOpen }"
            >▸</span>
          </div>

          <!-- Inline mood list -->
          <Transition name="ctx-sub-fade">
            <div
              v-if="moodOpen"
              class="pet-ctx-inline-sub"
              role="menu"
            >
              <div
                v-for="mood in MOOD_ENTRIES"
                :key="mood.key"
                class="ctx-item ctx-item--sub"
                role="menuitemradio"
                :aria-checked="isActive(mood)"
                @click="onPickMood(mood)"
              >
                <span class="ctx-icon">{{ mood.emoji }}</span>
                <span class="ctx-label">{{ mood.label }}</span>
                <span
                  v-if="isActive(mood)"
                  class="ctx-check"
                >✓</span>
              </div>
            </div>
          </Transition>

          <div class="ctx-separator" />

          <!-- Model selector — inline submenu like Mood -->
          <div
            class="ctx-item"
            role="menuitem"
            :aria-expanded="modelOpen"
            @click="modelOpen = !modelOpen"
          >
            <span class="ctx-icon">🎭</span>
            <span class="ctx-label">Model</span>
            <span
              class="ctx-chev"
              :class="{ 'ctx-chev--open': modelOpen }"
            >▸</span>
          </div>

          <Transition name="ctx-sub-fade">
            <div
              v-if="modelOpen"
              class="pet-ctx-inline-sub"
              role="menu"
            >
              <div
                v-for="model in DEFAULT_MODELS"
                :key="model.id"
                class="ctx-item ctx-item--sub"
                role="menuitemradio"
                :aria-checked="model.id === characterStore.selectedModelId"
                @click="onPickModel(model.id)"
              >
                <span class="ctx-icon">{{ model.gender === 'male' ? '🧑' : '👩' }}</span>
                <span class="ctx-label">{{ model.name }}</span>
                <span
                  v-if="model.id === characterStore.selectedModelId"
                  class="ctx-check"
                >✓</span>
              </div>
            </div>
          </Transition>

          <div class="ctx-separator" />

          <div
            class="ctx-item"
            role="menuitem"
            @click="onToggleChat"
          >
            <span class="ctx-icon">💬</span>
            <span class="ctx-label">Toggle chat</span>
          </div>

          <!-- Panel windows — each opens as a separate floating window -->
          <div
            class="ctx-item"
            role="menuitem"
            :aria-expanded="panelsOpen"
            @click="panelsOpen = !panelsOpen"
          >
            <span class="ctx-icon">📋</span>
            <span class="ctx-label">Panels</span>
            <span
              class="ctx-chev"
              :class="{ 'ctx-chev--open': panelsOpen }"
            >▸</span>
          </div>

          <Transition name="ctx-sub-fade">
            <div
              v-if="panelsOpen"
              class="pet-ctx-inline-sub"
              role="menu"
            >
              <div
                v-for="p in PANEL_ENTRIES"
                :key="p.id"
                class="ctx-item ctx-item--sub"
                role="menuitem"
                @click="onOpenPanel(p.id)"
              >
                <span class="ctx-icon">{{ p.icon }}</span>
                <span class="ctx-label">{{ p.label }}</span>
              </div>
            </div>
          </Transition>

          <div class="ctx-separator" />

          <div
            class="ctx-item"
            role="menuitem"
            @click="onToggleMode"
          >
            <span class="ctx-icon">🖥</span>
            <span class="ctx-label">Switch to desktop mode</span>
          </div>

          <div
            class="ctx-item"
            role="menuitemcheckbox"
            :aria-checked="props.resizeActive"
            @click="onToggleResize"
          >
            <span class="ctx-icon">↔</span>
            <span class="ctx-label">Resize</span>
            <span
              v-if="props.resizeActive"
              class="ctx-check"
            >✓</span>
          </div>

          <div class="ctx-separator" />

          <div
            class="ctx-item ctx-item--danger"
            role="menuitem"
            @click="onExit"
          >
            <span class="ctx-icon">✕</span>
            <span class="ctx-label">Exit</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { useCharacterStore } from '../stores/character';
import { useWindowStore } from '../stores/window';
import { useChatExpansion } from '../composables/useChatExpansion';
import { MOOD_ENTRIES, isMoodActive, applyMood, type MoodEntry } from '../config/moods';
import { DEFAULT_MODELS } from '../config/default-models';

export type { MoodKey } from '../config/moods';

const props = defineProps<{
  visible: boolean;
  x: number;
  y: number;
  resizeActive: boolean;
}>();

const emit = defineEmits<{
  close: [];
  'toggle-resize': [];
}>();

const characterStore = useCharacterStore();
const windowStore = useWindowStore();
const { togglePetChat } = useChatExpansion();

const menuRef = ref<HTMLElement | null>(null);
const moodOpen = ref(false);
const modelOpen = ref(false);
const panelsOpen = ref(false);

const PANEL_ENTRIES = [
  { id: 'brain', icon: '🧠', label: 'Brain' },
  { id: 'memory', icon: '💡', label: 'Memory' },
  { id: 'skills', icon: '⭐', label: 'Quests' },
  { id: 'marketplace', icon: '🏪', label: 'Marketplace' },
  { id: 'voice', icon: '🎙', label: 'Voice' },
] as const;

// Measured menu dimensions for viewport-clamping.
const measuredWidth = ref(220);
const measuredHeight = ref(280);

/**
 * Get the CSS-pixel bounds of the monitor that contains the click point.
 * Falls back to the full viewport (window.innerWidth/Height) when no
 * monitor info is available (e.g. browser-only mode).
 */
function getMonitorBounds(clickX: number, clickY: number): { left: number; top: number; right: number; bottom: number } {
  const monitors = windowStore.monitors;
  const dpr = window.devicePixelRatio || 1;

  if (!monitors.length) {
    return { left: 0, top: 0, right: window.innerWidth, bottom: window.innerHeight };
  }

  // The pet-mode window origin is at the top-left of the bounding rect
  // spanning all monitors.  Convert physical coords → CSS coords.
  const minX = Math.min(...monitors.map(m => m.x));
  const minY = Math.min(...monitors.map(m => m.y));

  // Convert click CSS coords to physical space to find containing monitor
  const clickPhysX = clickX * dpr + minX;
  const clickPhysY = clickY * dpr + minY;

  // Find which monitor contains the click point
  let monitor = monitors[0]; // fallback: first monitor
  for (const m of monitors) {
    if (
      clickPhysX >= m.x && clickPhysX < m.x + m.width &&
      clickPhysY >= m.y && clickPhysY < m.y + m.height
    ) {
      monitor = m;
      break;
    }
  }

  // Convert that monitor's bounds to CSS pixels relative to window origin
  return {
    left: (monitor.x - minX) / dpr,
    top: (monitor.y - minY) / dpr,
    right: (monitor.x + monitor.width - minX) / dpr,
    bottom: (monitor.y + monitor.height - minY) / dpr,
  };
}

const menuStyle = computed(() => {
  const w = measuredWidth.value;
  const h = measuredHeight.value;
  const bounds = getMonitorBounds(props.x, props.y);
  // Horizontal: flip left if menu would overflow this monitor's right edge
  const flipLeft = props.x + w + 8 > bounds.right;
  const left = flipLeft
    ? Math.max(bounds.left + 8, props.x - w)
    : props.x;
  // Vertical: flip upward if menu would overflow this monitor's bottom edge
  const flipUp = props.y + h + 8 > bounds.bottom;
  let top = flipUp
    ? Math.max(bounds.top + 8, props.y - h)
    : props.y;
  // Final clamp: ensure menu doesn't extend beyond monitor bottom
  if (top + h + 8 > bounds.bottom) {
    top = Math.max(bounds.top + 8, bounds.bottom - h - 8);
  }
  return {
    left: `${left}px`,
    top: `${top}px`,
    maxHeight: `${bounds.bottom - bounds.top - 16}px`,
  };
});

function isActive(mood: MoodEntry): boolean {
  return isMoodActive(mood, characterStore);
}

function onPickMood(mood: MoodEntry) {
  applyMood(mood, characterStore);
  close();
}

function onToggleChat() {
  togglePetChat();
  close();
}

function onOpenPanel(panel: string) {
  windowStore.openPanelWindow(panel);
  close();
}

async function onToggleMode() {
  close();
  await windowStore.toggleMode();
}

function onPickModel(modelId: string) {
  characterStore.selectModel(modelId);
  close();
}

function onToggleResize() {
  emit('toggle-resize');
  close();
}

async function onExit() {
  close();
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('exit_app');
  } catch {
    window.close();
  }
}

function close() {
  moodOpen.value = false;
  modelOpen.value = false;
  panelsOpen.value = false;
  emit('close');
}

function onKeydown(e: KeyboardEvent) {
  if (!props.visible) return;
  if (e.key === 'Escape') close();
}

watch(
  () => props.visible,
  async (visible) => {
    if (visible) {
      moodOpen.value = false;
      modelOpen.value = false;
      panelsOpen.value = false;
      window.addEventListener('keydown', onKeydown);
      await nextTick();
      if (menuRef.value) {
        const rect = menuRef.value.getBoundingClientRect();
        measuredWidth.value = rect.width;
        measuredHeight.value = rect.height;
      }
    } else {
      window.removeEventListener('keydown', onKeydown);
    }
  },
);

// Re-measure when mood or model submenu opens/closes so positioning re-computes
watch([moodOpen, modelOpen, panelsOpen], async () => {
  await nextTick();
  if (menuRef.value) {
    const rect = menuRef.value.getBoundingClientRect();
    measuredWidth.value = rect.width;
    measuredHeight.value = rect.height;
  }
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown);
});
</script>

<style scoped>
.pet-ctx-backdrop {
  position: fixed;
  inset: 0;
  z-index: 9000;
  pointer-events: auto;
  background: transparent;
  /* Must override the parent's pointer-events: none in full-screen pet mode */
}

.pet-ctx-menu {
  position: fixed;
  min-width: 220px;
  max-width: min(320px, calc(100vw - 16px));
  overflow-y: auto;
  background: rgba(15, 23, 42, 0.96);
  border: 1px solid rgba(139, 92, 246, 0.3);
  border-radius: 10px;
  padding: 6px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(14px);
  color: var(--ts-text-bright, var(--ts-text-primary));
  font-size: 0.85rem;
  user-select: none;
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.12s;
  position: relative;
}
.ctx-item:hover {
  background: rgba(108, 99, 255, 0.22);
}
.ctx-item--danger:hover {
  background: rgba(248, 113, 113, 0.22);
}
.ctx-icon {
  width: 20px;
  text-align: center;
  font-size: 0.95rem;
}
.ctx-label {
  flex: 1;
}
.ctx-chev {
  opacity: 0.55;
  font-size: 0.8rem;
  transition: transform 0.15s;
}
.ctx-chev--open {
  transform: rotate(90deg);
}
.ctx-check {
  color: var(--ts-accent, #7c6fff);
  font-weight: 700;
}
.ctx-separator {
  height: 1px;
  background: rgba(255, 255, 255, 0.08);
  margin: 4px 2px;
}

/* Inline submenu — mood items expand within the menu */
.pet-ctx-inline-sub {
  padding: 2px 0 2px 10px;
}
.ctx-item--sub {
  padding: 6px 10px;
  font-size: 0.8rem;
}

.ctx-fade-enter-active,
.ctx-fade-leave-active {
  transition: opacity 0.12s;
}
.ctx-fade-enter-from,
.ctx-fade-leave-to {
  opacity: 0;
}

.ctx-sub-fade-enter-active,
.ctx-sub-fade-leave-active {
  transition: opacity 0.1s, max-height 0.15s;
}
.ctx-sub-fade-enter-from,
.ctx-sub-fade-leave-to {
  opacity: 0;
}
</style>
