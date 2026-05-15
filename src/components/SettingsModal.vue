<template>
  <Teleport to="body">
    <Transition name="settings-modal">
      <div
        v-if="open"
        class="settings-modal-backdrop"
        data-testid="settings-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-modal-title"
        @click.self="emit('update:open', false)"
      >
        <div
          class="settings-modal"
          @click.stop
        >
          <header class="sm-header">
            <h2
              id="settings-modal-title"
              class="sm-title"
            >
              Quick Settings
            </h2>
            <button
              type="button"
              class="sm-close"
              aria-label="Close quick settings"
              data-testid="settings-modal-close"
              @click="emit('update:open', false)"
            >
              &times;
            </button>
          </header>

          <!-- Full settings surface. SettingsModal now hosts the same
               configuration sections as the inline SettingsPanel inside
               CharacterViewport (view mode, quests, character + profile,
               mood/pose, background, BGM, karaoke, theme, system info,
               audio controls). BGM is driven by the app-wide shared
               player so the modal and the inline panel stay in sync. -->
          <div class="sm-panel-host">
            <SettingsPanel
              :is-pet-mode="isPetMode"
              :bgm="bgm"
              v-model:bgm-enabled="bgmEnabled"
              v-model:bgm-volume="bgmVolume"
              v-model:bgm-track-id="bgmTrackId"
              @close="emit('update:open', false)"
              @request-set-display-mode="(mode) => onSetDisplayMode(mode)"
              @request-toggle-pet-mode="onTogglePetMode"
              @toggle-system-info="showSystemInfo = !showSystemInfo"
              @toggle-audio-controls="showAudioControls = !showAudioControls"
              @url-dialog-toggle="(open) => { urlDialogOpen = open; }"
            />
          </div>

          <footer class="sm-footer">
            <button
              type="button"
              class="sm-link"
              data-testid="settings-modal-open-full"
              @click="onOpenFullSettings"
            >
              Open full Settings panel →
            </button>
          </footer>

          <!-- System info / audio controls panels are toggled by buttons
               inside SettingsPanel. They render inside the modal so the
               user can stay in context. -->
          <SystemInfoPanel
            v-if="showSystemInfo"
            class="sm-overlay-panel"
            @close="showSystemInfo = false"
          />
          <AudioControlsPanel
            v-if="showAudioControls"
            class="sm-overlay-panel"
            @close="showAudioControls = false"
            @update:bgm-enabled="(v) => (bgmEnabled = v)"
            @update:bgm-volume="(v) => (bgmVolume = v)"
            @update:bgm-track-id="(v) => (bgmTrackId = v)"
          />
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useSettingsStore } from '../stores/settings';
import { useWindowStore } from '../stores/window';
import { getSharedBgmPlayer, DEFAULT_BGM_VOLUME } from '../composables/useBgmPlayer';
import SettingsPanel from './SettingsPanel.vue';
import SystemInfoPanel from './SystemInfoPanel.vue';
import AudioControlsPanel from './AudioControlsPanel.vue';

withDefaults(
  defineProps<{ open: boolean }>(),
  { open: false },
);

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'open-full-settings'): void;
}>();

const settingsStore = useSettingsStore();
const windowStore = useWindowStore();

const isPetMode = computed(() => windowStore.mode === 'pet');

// ── Shared BGM state ────────────────────────────────────────────────────────
// Bound through SettingsPanel + AudioControlsPanel. Use the shared player
// so the modal stays in sync with CharacterViewport's inline panel.
const bgm = getSharedBgmPlayer();
const bgmEnabled = ref(false);
const bgmVolume = ref(DEFAULT_BGM_VOLUME);
const bgmTrackId = ref('prelude');

// ── Sub-panel toggles + state ──────────────────────────────────────────────
const showSystemInfo = ref(false);
const showAudioControls = ref(false);
// Tracked so SettingsPanel can warn the parent that a modal child is open
// (used by SettingsPanel's `url-dialog-toggle` event).
const urlDialogOpen = ref(false);
void urlDialogOpen;

async function onSetDisplayMode(mode: 'desktop' | 'chatbox') {
  if (isPetMode.value) {
    await windowStore.setMode('window');
  }
  await settingsStore.setChatboxMode(mode === 'chatbox');
}

async function onTogglePetMode() {
  await windowStore.toggleMode();
}

function onOpenFullSettings() {
  emit('open-full-settings');
  emit('update:open', false);
}
</script>

<style scoped>
.settings-modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(4px);
  z-index: 2100;
  display: flex;
  align-items: flex-start;
  justify-content: flex-end;
  padding: 64px 16px 16px;
}

.settings-modal {
  width: min(420px, 96vw);
  max-height: calc(100vh - 96px);
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-lg, 14px);
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.45);
  color: var(--ts-text-primary);
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-md, 12px);
  padding: var(--ts-space-lg, 16px);
  overflow-y: auto;
}

.sm-panel-host {
  /* The embedded SettingsPanel uses its own FloatingMenu chrome which
     comes with absolute positioning. Reset so it lays out inline. */
  position: relative;
  width: 100%;
}

/* Neutralise FloatingMenu's floating chrome when embedded. */
.sm-panel-host :deep(.floating-menu) {
  position: static;
  margin: 0;
  width: 100%;
  max-width: none;
  box-shadow: none;
  border: 0;
  padding: 0;
  background: transparent;
}

.sm-overlay-panel {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 2200;
}

.sm-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ts-space-sm, 8px);
}

.sm-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  color: var(--ts-text-bright);
  letter-spacing: 0.03em;
}

.sm-close {
  background: transparent;
  border: 0;
  color: var(--ts-text-muted);
  font-size: 1.4rem;
  line-height: 1;
  padding: 4px 8px;
  cursor: pointer;
  border-radius: var(--ts-radius-sm, 6px);
}

.sm-close:hover {
  color: var(--ts-text-bright);
  background: var(--ts-bg-hover);
}

.sm-section {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-sm, 8px);
}

.sm-section-title {
  margin: 0;
  font-size: 0.72rem;
  color: var(--ts-text-muted);
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.sm-row {
  display: flex;
  gap: var(--ts-space-sm, 8px);
  flex-wrap: wrap;
}

.sm-btn {
  flex: 1 1 auto;
  min-width: 90px;
  padding: var(--ts-space-sm, 8px) var(--ts-space-md, 12px);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm, 8px);
  background: var(--ts-bg-elevated);
  color: var(--ts-text-primary);
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.15s ease, background 0.15s ease,
    transform 0.15s ease;
}

.sm-btn:hover {
  border-color: var(--ts-accent);
  transform: translateY(-1px);
}

.sm-btn.active {
  border-color: var(--ts-accent);
  background: var(--ts-bg-hover);
  box-shadow: inset 0 0 0 1px var(--ts-accent);
}

.sm-footer {
  display: flex;
  justify-content: flex-end;
  padding-top: var(--ts-space-xs, 4px);
  border-top: 1px solid var(--ts-border);
}

.sm-link {
  background: transparent;
  border: 0;
  color: var(--ts-accent);
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  padding: var(--ts-space-xs, 4px) var(--ts-space-sm, 8px);
  border-radius: var(--ts-radius-sm, 6px);
}

.sm-link:hover {
  background: var(--ts-bg-hover);
}

.settings-modal-enter-active,
.settings-modal-leave-active {
  transition: opacity 0.18s ease;
}

.settings-modal-enter-active .settings-modal,
.settings-modal-leave-active .settings-modal {
  transition: transform 0.18s ease, opacity 0.18s ease;
}

.settings-modal-enter-from,
.settings-modal-leave-to {
  opacity: 0;
}

.settings-modal-enter-from .settings-modal,
.settings-modal-leave-to .settings-modal {
  transform: translateY(-8px);
  opacity: 0;
}
</style>
