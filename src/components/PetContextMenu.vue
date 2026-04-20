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
          <!-- Mood submenu trigger -->
          <div
            class="ctx-item ctx-has-sub"
            role="menuitem"
            :aria-expanded="moodOpen"
            @mouseenter="moodOpen = true"
            @mouseleave="onMoodLeave"
            @click="moodOpen = !moodOpen"
          >
            <span class="ctx-icon">🎭</span>
            <span class="ctx-label">Mood</span>
            <span class="ctx-chev">▸</span>

            <!-- Submenu -->
            <Transition name="ctx-sub-fade">
              <div
                v-if="moodOpen"
                class="pet-ctx-submenu"
                :class="{ 'flip-up': flipSubmenuUp }"
                role="menu"
                @mouseenter="moodOpen = true"
              >
                <div
                  v-for="mood in MOOD_ENTRIES"
                  :key="mood.key"
                  class="ctx-item"
                  role="menuitemradio"
                  :aria-checked="isActive(mood)"
                  @click="onPickMood(mood)"
                >
                  <span class="ctx-icon">{{ mood.emoji }}</span>
                  <span class="ctx-label">{{ mood.label }}</span>
                  <span v-if="isActive(mood)" class="ctx-check">✓</span>
                </div>
              </div>
            </Transition>
          </div>

          <div class="ctx-separator" />

          <div class="ctx-item" role="menuitem" @click="onToggleChat">
            <span class="ctx-icon">💬</span>
            <span class="ctx-label">Toggle chat</span>
          </div>

          <div class="ctx-item" role="menuitem" @click="onToggleMode">
            <span class="ctx-icon">🖥</span>
            <span class="ctx-label">Switch to desktop mode</span>
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

export type { MoodKey } from '../config/moods';

const props = defineProps<{
  visible: boolean;
  x: number;
  y: number;
}>();

const emit = defineEmits<{
  close: [];
}>();

const characterStore = useCharacterStore();
const windowStore = useWindowStore();
const { togglePetChat } = useChatExpansion();

const menuRef = ref<HTMLElement | null>(null);
const moodOpen = ref(false);
const flipSubmenuUp = ref(false);

// Measured menu dimensions for edge-flip. Updated after mount.
const measuredWidth = ref(220);
const measuredHeight = ref(200);

const menuStyle = computed(() => {
  const vw = typeof window !== 'undefined' ? window.innerWidth : 1920;
  const vh = typeof window !== 'undefined' ? window.innerHeight : 1080;
  const w = measuredWidth.value;
  const h = measuredHeight.value;
  // Flip horizontally / vertically if the menu would overflow the viewport.
  const left = props.x + w > vw ? Math.max(0, vw - w - 4) : props.x;
  const top  = props.y + h > vh ? Math.max(0, vh - h - 4) : props.y;
  return {
    left: `${left}px`,
    top: `${top}px`,
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

async function onToggleMode() {
  close();
  await windowStore.toggleMode();
}

function close() {
  moodOpen.value = false;
  emit('close');
}

let moodLeaveTimer: ReturnType<typeof setTimeout> | null = null;
function onMoodLeave() {
  // Short grace period so the user can travel into the submenu
  if (moodLeaveTimer) clearTimeout(moodLeaveTimer);
  moodLeaveTimer = setTimeout(() => {
    moodOpen.value = false;
  }, 180);
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
      flipSubmenuUp.value = false;
      window.addEventListener('keydown', onKeydown);
      await nextTick();
      if (menuRef.value) {
        const rect = menuRef.value.getBoundingClientRect();
        measuredWidth.value = rect.width;
        measuredHeight.value = rect.height;
        // If the submenu would overflow downward, flip it upward.
        const estimatedSubHeight = MOOD_ENTRIES.length * 34 + 16;
        flipSubmenuUp.value =
          props.y + estimatedSubHeight > (window.innerHeight - 8);
      }
    } else {
      window.removeEventListener('keydown', onKeydown);
      if (moodLeaveTimer) {
        clearTimeout(moodLeaveTimer);
        moodLeaveTimer = null;
      }
    }
  },
);

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown);
  if (moodLeaveTimer) clearTimeout(moodLeaveTimer);
});
</script>

<style scoped>
.pet-ctx-backdrop {
  position: fixed;
  inset: 0;
  z-index: 9000;
  pointer-events: auto;
  background: transparent;
}

.pet-ctx-menu {
  position: fixed;
  min-width: 220px;
  background: rgba(15, 23, 42, 0.96);
  border: 1px solid rgba(139, 92, 246, 0.3);
  border-radius: 10px;
  padding: 6px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(14px);
  color: #e2e8f0;
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

.pet-ctx-submenu {
  position: absolute;
  top: -4px;
  left: 100%;
  margin-left: 4px;
  min-width: 180px;
  background: rgba(15, 23, 42, 0.97);
  border: 1px solid rgba(139, 92, 246, 0.3);
  border-radius: 10px;
  padding: 6px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(14px);
}
.pet-ctx-submenu.flip-up {
  top: auto;
  bottom: -4px;
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
  transition: opacity 0.1s, transform 0.1s;
}
.ctx-sub-fade-enter-from,
.ctx-sub-fade-leave-to {
  opacity: 0;
  transform: translateX(-4px);
}
</style>
