<template>
  <div
    class="viewport-wrapper"
    :class="{ 'viewport-wrapper--pet': isPetMode }"
  >
    <!-- Background layers: scenic background + tint.  Hidden in pet mode so
         only the character floats above the desktop. -->
    <div
      v-if="!isPetMode"
      class="background-layer"
      :style="backgroundStyle"
    />
    <div
      v-if="!isPetMode"
      class="background-tint"
    />
    <canvas
      ref="canvasRef"
      class="viewport-canvas"
    />
    <!-- Loading overlay -->
    <Transition name="fade">
      <div
        v-if="characterStore.isLoading"
        class="loading-overlay"
      >
        <div class="loading-spinner" />
        <span class="loading-text">Loading model…</span>
      </div>
    </Transition>
    <!-- Error overlay -->
    <Transition name="fade">
      <div
        v-if="characterStore.loadError && !characterStore.isLoading"
        class="loading-overlay load-error-overlay"
      >
        <span class="load-error-icon">⚠️</span>
        <span class="loading-text">{{ characterStore.loadError }}</span>
        <button
          class="load-error-retry"
          @click="retryModelLoad"
        >
          Retry
        </button>
      </div>
    </Transition>
    <!-- Character name / author / settings — hidden in pet mode (which has
         its own minimal chrome anchored at the app level). -->
    <template v-if="!isPetMode">
      <div class="character-name-overlay">
        {{ characterName }}
      </div>
      <div
        v-if="characterStore.vrmMetadata"
        class="character-meta-overlay"
      >
        <span>by {{ characterStore.vrmMetadata.author }}</span>
      </div>
    </template>

    <!-- ── Corner settings button — hidden in pet mode (the PetContextMenu
         exposes pet-specific options instead) ── -->
    <div
      v-if="!isPetMode"
      ref="settingsRef"
      class="settings-corner"
    >
      <FloatingChip
        as="button"
        class="settings-toggle"
        interactive
        type="button"
        aria-label="Settings"
        @click.stop="settingsOpen = !settingsOpen"
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M19.14,12.94c0.04-0.3,0.06-0.61,0.06-0.94c0-0.32-0.02-0.64-0.07-0.94l2.03-1.58c0.18-0.14,0.23-0.41,0.12-0.61 l-1.92-3.32c-0.12-0.22-0.37-0.29-0.59-0.22l-2.39,0.96c-0.5-0.38-1.03-0.7-1.62-0.94L14.4,2.81c-0.04-0.24-0.24-0.41-0.48-0.41 h-3.84c-0.24,0-0.43,0.17-0.47,0.41L9.25,5.35C8.66,5.59,8.12,5.92,7.63,6.29L5.24,5.33c-0.22-0.08-0.47,0-0.59,0.22L2.74,8.87 C2.62,9.08,2.66,9.34,2.86,9.48l2.03,1.58C4.84,11.36,4.8,11.69,4.8,12s0.02,0.64,0.07,0.94l-2.03,1.58 c-0.18,0.14-0.23,0.41-0.12,0.61l1.92,3.32c0.12,0.22,0.37,0.29,0.59,0.22l2.39-0.96c0.5,0.38,1.03,0.7,1.62,0.94l0.36,2.54 c0.05,0.24,0.24,0.41,0.48,0.41h3.84c0.24,0,0.44-0.17,0.47-0.41l0.36-2.54c0.59-0.24,1.13-0.56,1.62-0.94l2.39,0.96 c0.22,0.08,0.47,0,0.59-0.22l1.92-3.32c0.12-0.22,0.07-0.47-0.12-0.61L19.14,12.94z M12,15.6c-1.98,0-3.6-1.62-3.6-3.6 s1.62-3.6,3.6-3.6s3.6,1.62,3.6,3.6S13.98,15.6,12,15.6z" />
        </svg>
        <span class="settings-label">Settings</span>
      </FloatingChip>
      <Transition name="dropdown">
        <FloatingMenu
          v-if="settingsOpen"
          class="settings-dropdown"
          @click.stop
        >
          <div class="settings-header">
            <span class="settings-header-title">Settings</span>
            <button
              class="settings-close-btn"
              aria-label="Close settings"
              @click="settingsOpen = false"
            >
              &times;
            </button>
          </div>
          <!-- Model selector -->
          <div class="dropdown-section">
            <label class="dropdown-label">Character</label>
            <select
              class="model-selector"
              :value="characterStore.selectedModelId"
              @change="handleModelChange"
            >
              <option
                v-for="model in characterStore.defaultModels"
                :key="model.id"
                :value="model.id"
              >
                {{ model.name }}
              </option>
            </select>
            <button
              class="dropdown-btn"
              @click="openVrmPicker"
            >
              📁 Import VRM
            </button>
            <input
              ref="vrmInputRef"
              class="hidden-file-input"
              type="file"
              accept=".vrm"
              @change="handleVrmImport"
            >
          </div>
          <!-- Mood / pose selector — matches the Mood submenu in PetContextMenu
               so desktop and pet modes offer the same configurable states. -->
          <div class="dropdown-section">
            <label class="dropdown-label">Mood / Pose</label>
            <div
              class="mood-grid"
              role="radiogroup"
              aria-label="Character mood"
            >
              <button
                v-for="mood in MOOD_ENTRIES"
                :key="mood.key"
                class="mood-chip"
                :class="{ active: isMoodActive(mood, characterStore) }"
                role="radio"
                :aria-checked="isMoodActive(mood, characterStore)"
                :title="mood.label"
                @click="handleMoodPick(mood)"
              >
                <span class="mood-chip-emoji">{{ mood.emoji }}</span>
                <span class="mood-chip-label">{{ mood.label }}</span>
              </button>
            </div>
          </div>
          <!-- Background selector -->
          <div class="dropdown-section">
            <label class="dropdown-label">Background</label>
            <div class="bg-chips">
              <button
                v-for="background in backgroundStore.allBackgrounds"
                :key="background.id"
                class="background-chip"
                :class="{ active: backgroundStore.selectedBackgroundId === background.id }"
                @click="backgroundStore.selectBackground(background.id)"
              >
                {{ background.name }}
              </button>
            </div>
            <button
              class="dropdown-btn"
              @click="openBackgroundPicker"
            >
              🖼 Import BG
            </button>
            <input
              ref="backgroundInputRef"
              class="hidden-file-input"
              type="file"
              accept="image/*"
              @change="handleBackgroundImport"
            >
          </div>
          <!-- Background music -->
          <div class="dropdown-section">
            <label class="dropdown-label">Music</label>
            <div class="bgm-toggle-row">
              <label class="bgm-switch">
                <input
                  type="checkbox"
                  :checked="bgmEnabled"
                  @change="handleBgmToggle"
                >
                <span class="bgm-slider" />
              </label>
              <span class="bgm-status">{{ bgmEnabled ? 'On' : 'Off' }}</span>
            </div>
            <select
              v-if="bgmEnabled"
              class="model-selector"
              :value="bgmTrackId"
              @change="handleBgmTrackChange"
            >
              <option
                v-for="track in bgm.allTracks.value"
                :key="track.id"
                :value="track.id"
              >
                {{ track.name }}
              </option>
            </select>
            <div
              v-if="bgmEnabled"
              class="bgm-track-actions"
            >
              <button
                class="dropdown-btn"
                @click="requestAddMusic"
              >
                🎵 Add File
              </button>
              <button
                class="dropdown-btn"
                @click="openUrlDialog"
              >
                🔗 Add URL
              </button>
              <input
                ref="bgmFileInputRef"
                class="hidden-file-input"
                type="file"
                accept="audio/*,video/*"
                @change="handleBgmFileImport"
              >
            </div>
            <!-- Custom track list with delete -->
            <div
              v-if="bgmEnabled && bgm.customTracks.value.length"
              class="bgm-custom-list"
            >
              <div
                v-for="track in bgm.customTracks.value"
                :key="track.id"
                class="bgm-custom-item"
              >
                <span class="bgm-custom-name">{{ track.name }}</span>
                <button
                  class="bgm-remove-btn"
                  title="Remove track"
                  @click="handleRemoveTrack(track.id)"
                >
                  ✕
                </button>
              </div>
            </div>
            <div
              v-if="bgmEnabled"
              class="bgm-volume-row"
            >
              <span class="bgm-vol-icon">🔈</span>
              <input
                type="range"
                class="bgm-volume-slider"
                min="0"
                max="100"
                :value="Math.round(bgmVolume * 100)"
                @input="handleBgmVolumeChange"
              >
              <span class="bgm-vol-icon">🔊</span>
            </div>
          </div>
          
          <!-- Appearance / Theme picker -->
          <div class="dropdown-section">
            <ThemePicker />
          </div>

          <!-- Toggle buttons for full-screen panels -->
          <div class="dropdown-section">
            <button
              class="dropdown-btn"
              @click="showSystemInfo = !showSystemInfo"
            >
              📊 System Information
            </button>
            <button
              class="dropdown-btn"
              @click="showAudioControls = !showAudioControls"
            >
              🎛️ Audio Controls
            </button>
          </div>
        </FloatingMenu>
      </Transition>

      <!-- Full-screen overlays (rendered outside the dropdown to avoid z-index issues) -->
      <SystemInfoPanel
        v-if="showSystemInfo"
        @close="showSystemInfo = false"
      />
      <AudioControlsPanel
        v-if="showAudioControls"
        @close="showAudioControls = false"
        @update:bgm-volume="handleAudioBgmVolumeChange"
        @update:bgm-track-id="handleAudioBgmTrackChange"
      />
    </div>

    <div
      v-if="backgroundStore.importError"
      class="background-error-banner"
    >
      {{ backgroundStore.importError }}
    </div>
    <!-- ── Floating Music Bar (teleported to left side of viewport) ──
         Hidden in pet mode — music playback continues but the UI chrome is
         desktop-mode only.  The Teleport is disabled in pet mode because
         #music-bar-portal only exists inside ChatView; trying to teleport
         to a missing target throws during Vue's component update phase and
         breaks subsequent reactivity. -->
    <Teleport
      to="#music-bar-portal"
      defer
      :disabled="isPetMode"
    >
      <div
        v-if="!isPetMode"
        class="music-bar"
        :class="{ expanded: bgmBarExpanded, playing: bgmEnabled }"
      >
        <button
          class="music-bar-toggle"
          :title="bgmBarExpanded ? 'Collapse' : 'Music'"
          @click.stop="bgmBarExpanded = !bgmBarExpanded"
        >
          <span
            class="music-bar-toggle-icon"
            :class="{ open: bgmBarExpanded }"
          >{{ bgmBarExpanded ? '▶' : '🎵' }}</span>
        </button>
        <Transition name="music-expand">
          <div
            v-if="bgmBarExpanded"
            class="music-bar-panel"
            @click.stop
          >
            <button
              class="music-btn play-btn"
              :title="bgmEnabled ? 'Stop music' : 'Play music'"
              @click="toggleBgmFromBar"
            >
              {{ bgmEnabled ? '⏸' : '▶️' }}
            </button>
            <div class="music-track-info">
              <span class="music-track-name">{{ currentTrackName }}</span>
            </div>
            <button
              class="music-btn"
              title="Next track"
              @click="nextTrack"
            >
              ⏭
            </button>
            <input
              type="range"
              class="music-vol-slider"
              min="0"
              max="100"
              :value="Math.round(bgmVolume * 100)"
              title="Volume"
              @input="handleBarVolumeChange"
            >
            <button
              class="music-btn add-btn"
              title="Add more music"
              @click="emit('request-add-music')"
            >
              ➕
            </button>
          </div>
        </Transition>
      </div>
    </Teleport>

    <!-- ── Add URL Dialog ── -->
    <Transition name="fade">
      <div
        v-if="showUrlDialog"
        class="url-dialog-backdrop"
        @click.self="cancelUrlDialog"
      >
        <div class="url-dialog">
          <label class="url-dialog-label">Add music from URL</label>
          <input
            v-model="urlInput"
            class="url-dialog-input"
            type="url"
            placeholder="https://example.com/music.mp3"
            @keydown.enter="confirmUrlAdd"
            @keydown.escape="cancelUrlDialog"
          >
          <div class="url-dialog-actions">
            <button
              class="url-dialog-btn cancel"
              @click="cancelUrlDialog"
            >
              Cancel
            </button>
            <button
              class="url-dialog-btn confirm"
              :disabled="!urlInput.trim()"
              @click="confirmUrlAdd"
            >
              Add
            </button>
          </div>
        </div>
      </div>
    </Transition>

    <div
      v-if="showDebug"
      class="debug-overlay"
    >
      <span>WebGL</span>
      <span>▲ {{ debugInfo.triangles }}</span>
      <span>⬡ {{ debugInfo.calls }} draws</span>
      <span>⚙ {{ debugInfo.programs }} progs</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import * as THREE from 'three';
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { useCharacterStore } from '../stores/character';
import { useBackgroundStore } from '../stores/background';
import { useSettingsStore } from '../stores/settings';
import { useWindowStore } from '../stores/window';
import { usePersonaStore } from '../stores/persona';
import { DEFAULT_MODELS } from '../config/default-models';
import { initScene, type RendererInfo, type SceneContext } from '../renderer/scene';
import { loadVRMSafe, createPlaceholderCharacter } from '../renderer/vrm-loader';
import { CharacterAnimator } from '../renderer/character-animator';
import { VrmaManager, getAnimationForMood, getAnimationForMotion, getIdleAnimationForGender } from '../renderer/vrma-manager';
import { LearnedMotionPlayer, applyLearnedExpression, clearExpressionPreview } from '../renderer/learned-motion-player';
import { useBgmPlayer, BGM_TRACKS, type BgmTrack } from '../composables/useBgmPlayer';
import { MOOD_ENTRIES, isMoodActive, applyMood, type MoodEntry } from '../config/moods';
import SystemInfoPanel from './SystemInfoPanel.vue';
import AudioControlsPanel from './AudioControlsPanel.vue';
import FloatingChip from './ui/FloatingChip.vue';
import FloatingMenu from './ui/FloatingMenu.vue';
import ThemePicker from './ThemePicker.vue';

const emit = defineEmits<{
  'request-add-music': [];
}>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const characterStore = useCharacterStore();
const backgroundStore = useBackgroundStore();
const settingsStore = useSettingsStore();
const windowStoreLocal = useWindowStore();
const personaStore = usePersonaStore();
/** Viewport behaves differently in pet mode: no background, no chrome,
 *  transparent clear colour, and pedestal hidden in the 3D scene. */
const isPetMode = computed(() => windowStoreLocal.mode === 'pet');
const showDebug = ref(false);
const debugInfo = ref<RendererInfo>({ triangles: 0, calls: 0, programs: 0 });
const backgroundInputRef = ref<HTMLInputElement | null>(null);
const vrmInputRef = ref<HTMLInputElement | null>(null);
const localVrmObjectUrl = ref<string | null>(null);
const settingsOpen = ref(false);
const settingsRef = ref<HTMLElement | null>(null);
const showSystemInfo = ref(false);
const showAudioControls = ref(false);

// ── BGM player ────────────────────────────────────────────────────────────────
const bgm = useBgmPlayer();
const bgmEnabled = ref(false);
const bgmVolume = ref(0.15);
const bgmTrackId = ref('prelude');
const bgmBarExpanded = ref(false);
const bgmFileInputRef = ref<HTMLInputElement | null>(null);
const showUrlDialog = ref(false);
const urlInput = ref('');

function handleBgmToggle(e: Event) {
  const checked = (e.target as HTMLInputElement).checked;
  bgmEnabled.value = checked;
  if (checked) {
    bgm.setVolume(bgmVolume.value);
    bgm.play(bgmTrackId.value);
  } else {
    bgm.stop();
  }
  settingsStore.saveBgmState(bgmEnabled.value, bgmVolume.value, bgmTrackId.value);
}

function handleBgmTrackChange(e: Event) {
  const id = (e.target as HTMLSelectElement).value;
  bgmTrackId.value = id;
  if (bgmEnabled.value) {
    bgm.play(id);
  }
  settingsStore.saveBgmState(bgmEnabled.value, bgmVolume.value, bgmTrackId.value);
}

function handleBgmVolumeChange(e: Event) {
  const v = parseInt((e.target as HTMLInputElement).value, 10) / 100;
  bgmVolume.value = v;
  bgm.setVolume(v);
  settingsStore.saveBgmState(bgmEnabled.value, bgmVolume.value, bgmTrackId.value);
}

/** Restore BGM state from persisted settings. */
function restoreBgmFromSettings() {
  const s = settingsStore.settings;
  bgmEnabled.value = s.bgm_enabled;
  bgmVolume.value = s.bgm_volume;
  bgmTrackId.value = s.bgm_track_id;
  // Load persisted custom tracks
  if (s.bgm_custom_tracks?.length) {
    bgm.loadCustomTracks(s.bgm_custom_tracks);
  }
  // Don't auto-play here — browser autoplay policy blocks AudioContext.resume()
  // without a user gesture. Instead, defer playback until the first interaction.
  if (bgmEnabled.value) {
    bgm.setVolume(bgmVolume.value);
    deferBgmPlayback();
  }
}

/** Wait for the first user interaction, then start BGM. */
let bgmDeferredCleanup: (() => void) | null = null;
function deferBgmPlayback() {
  if (bgmDeferredCleanup) return; // already deferred
  const startBgm = () => {
    if (bgmEnabled.value) {
      bgm.setVolume(bgmVolume.value);
      bgm.play(bgmTrackId.value);
    }
    cleanup();
  };
  const cleanup = () => {
    document.removeEventListener('click', startBgm, true);
    document.removeEventListener('keydown', startBgm, true);
    document.removeEventListener('touchstart', startBgm, true);
    bgmDeferredCleanup = null;
  };
  bgmDeferredCleanup = cleanup;
  document.addEventListener('click', startBgm, { capture: true, once: true });
  document.addEventListener('keydown', startBgm, { capture: true, once: true });
  document.addEventListener('touchstart', startBgm, { capture: true, once: true });
}

// Event handlers for audio controls panel
function handleAudioBgmVolumeChange(volume: number) {
  bgmVolume.value = volume;
  bgm.setVolume(volume);
  if (bgmEnabled.value) {
    settingsStore.saveBgmState(bgmEnabled.value, bgmVolume.value, bgmTrackId.value);
  }
}

// ── Floating music bar helpers ────────────────────────────────────────────────
const currentTrackName = computed(() => {
  return bgm.allTracks.value.find(t => t.id === bgmTrackId.value)?.name ?? 'Music';
});

function toggleBgmFromBar() {
  bgmEnabled.value = !bgmEnabled.value;
  if (bgmEnabled.value) {
    bgm.setVolume(bgmVolume.value);
    bgm.play(bgmTrackId.value);
  } else {
    bgm.stop();
  }
  settingsStore.saveBgmState(bgmEnabled.value, bgmVolume.value, bgmTrackId.value);
}

function nextTrack() {
  const tracks = bgm.allTracks.value;
  const currentIdx = tracks.findIndex(t => t.id === bgmTrackId.value);
  const nextIdx = (currentIdx + 1) % tracks.length;
  bgmTrackId.value = tracks[nextIdx].id;
  if (bgmEnabled.value) {
    bgm.play(bgmTrackId.value);
  }
  settingsStore.saveBgmState(bgmEnabled.value, bgmVolume.value, bgmTrackId.value);
}

function handleBarVolumeChange(e: Event) {
  const v = parseInt((e.target as HTMLInputElement).value, 10) / 100;
  bgmVolume.value = v;
  bgm.setVolume(v);
  settingsStore.saveBgmState(bgmEnabled.value, bgmVolume.value, bgmTrackId.value);
}

function requestAddMusic() {
  bgmFileInputRef.value?.click();
}

function handleBgmFileImport(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  const objectUrl = URL.createObjectURL(file);
  const name = file.name.replace(/\.[^.]+$/, '');
  const id = bgm.addCustomTrack(name, objectUrl);
  bgmTrackId.value = id;
  if (bgmEnabled.value) {
    bgm.play(id);
  }
  persistCustomTracks();
  input.value = '';
}

function openUrlDialog() {
  urlInput.value = '';
  showUrlDialog.value = true;
}

function confirmUrlAdd() {
  const url = urlInput.value.trim();
  if (!url) return;
  // Derive a name from the URL (last path segment or hostname)
  let name = 'Custom Track';
  try {
    const parsed = new URL(url);
    const seg = parsed.pathname.split('/').filter(Boolean).pop();
    if (seg) name = decodeURIComponent(seg).replace(/\.[^.]+$/, '');
  } catch { /* keep default name */ }
  const id = bgm.addCustomTrack(name, url);
  bgmTrackId.value = id;
  if (bgmEnabled.value) {
    bgm.play(id);
  }
  persistCustomTracks();
  showUrlDialog.value = false;
}

function cancelUrlDialog() {
  showUrlDialog.value = false;
}

function handleRemoveTrack(trackId: string) {
  const wasPlaying = bgmTrackId.value === trackId;
  bgm.removeTrack(trackId);
  if (wasPlaying) {
    bgmTrackId.value = BGM_TRACKS[0].id;
    if (bgmEnabled.value) {
      bgm.play(bgmTrackId.value);
    }
  }
  persistCustomTracks();
}

function persistCustomTracks() {
  // Save custom tracks (with src URLs) to settings.
  // Only persist tracks that have non-blob URLs (blob URLs don't survive restart).
  const persistable = bgm.customTracks.value
    .filter(t => t.src && !t.src.startsWith('blob:'))
    .map(({ id, name, src }) => ({ id, name, src }));
  settingsStore.saveSettings({ bgm_custom_tracks: persistable as BgmTrack[] });
}

function handleAudioBgmTrackChange(trackId: string) {
  bgmTrackId.value = trackId;
  if (bgmEnabled.value) {
    bgm.play(trackId);
    settingsStore.saveBgmState(bgmEnabled.value, bgmVolume.value, bgmTrackId.value);
  }
}

const characterName = computed(() => {
  return characterStore.vrmMetadata?.title ?? 'TerranSoul';
});

const backgroundStyle = computed(() => ({
  backgroundImage: `url("${backgroundStore.currentBackground.url}")`,
}));

let animFrameId = 0;
let disposeScene: (() => void) | null = null;
let getRendererInfo: (() => RendererInfo) | null = null;
let sceneCtx: SceneContext | null = null;
let currentVrmScene: THREE.Object3D | null = null;
const animator = new CharacterAnimator();
const vrmaManager = new VrmaManager();
const motionPlayer = new LearnedMotionPlayer(vrmaManager);
let expressionPreviewTimer: ReturnType<typeof setTimeout> | null = null;

// Wire VRMA playback state to the animator
vrmaManager.onPlaybackChange((playing) => {
  animator.setVrmaPlaying(playing);
});

// Expose the avatar state machine for direct mutation by ChatView (coarse state bridge)
defineExpose({
  /** The layered AvatarStateMachine — ChatView mutates body/emotion here. */
  get avatarStateMachine() {
    return animator.avatarStateMachine;
  },
  /** Enable BGM playback (called by ChatView when BGM quest is accepted). */
  enableBgm() {
    if (!bgmEnabled.value) {
      bgmEnabled.value = true;
      bgm.setVolume(bgmVolume.value);
      bgm.play(bgmTrackId.value);
      settingsStore.saveBgmState(true, bgmVolume.value, bgmTrackId.value);
    }
  },
  /** Scene context — used by PetOverlayView to project 3D positions and rotate. */
  get sceneContext() {
    return sceneCtx;
  },
  /**
   * Play a VRMA body animation by motion key (e.g. 'greeting', 'clapping').
   * Called by ChatView when the LLM emits a motion tag.
   * Suppresses mood-auto-play so the mood watcher doesn't override this.
   */
  playMotion(motionKey: string) {
    const entry = getAnimationForMotion(motionKey);
    if (entry) {
      vrmaManager.suppressMoodAnimation();
      vrmaManager.play(entry.path, entry.loop, 0.4);
    }
  },
  /** Stop any playing VRMA animation and return to procedural. */
  stopMotion() {
    vrmaManager.stop(0.4);
  },
  /** Whether a mood-suppressed VRMA animation is actively playing (e.g. angry.vrma). */
  get isAnimationActive(): boolean {
    return vrmaManager.isMoodSuppressed && vrmaManager.isPlaying;
  },
  /**
   * Play a learned motion clip on the avatar. Bakes the JSON frames
   * into an AnimationClip on the fly.
   */
  playLearnedMotion(motion: import('../stores/persona-types').LearnedMotion) {
    vrmaManager.suppressMoodAnimation();
    motionPlayer.play(motion, false, 0.4);
  },
  /**
   * Preview a learned expression by applying its weights to the VRM
   * for 3 seconds, then resetting.
   */
  previewExpression(expr: import('../stores/persona-types').LearnedExpression) {
    const vrm = vrmaManager.vrm;
    if (!vrm) return;
    if (expressionPreviewTimer) clearTimeout(expressionPreviewTimer);
    applyLearnedExpression(vrm, expr);
    expressionPreviewTimer = setTimeout(() => {
      clearExpressionPreview(vrm);
      expressionPreviewTimer = null;
    }, 3000);
  },
});

function handleModelChange(e: Event) {
  const select = e.target as HTMLSelectElement;
  characterStore.selectModel(select.value);
}

function handleMoodPick(mood: MoodEntry) {
  applyMood(mood, characterStore);
}

function retryModelLoad() {
  characterStore.selectModel(characterStore.selectedModelId);
}

function openVrmPicker() {
  vrmInputRef.value?.click();
}

async function handleVrmImport(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) {
    return;
  }

  if (!file.name.toLowerCase().endsWith('.vrm')) {
    characterStore.setLoadError('Please choose a .vrm file.');
    input.value = '';
    return;
  }

  characterStore.setLoadError(undefined);

  if (localVrmObjectUrl.value) {
    URL.revokeObjectURL(localVrmObjectUrl.value);
  }

  const objectUrl = URL.createObjectURL(file);
  localVrmObjectUrl.value = objectUrl;
  await characterStore.loadVrm(objectUrl);
  input.value = '';
}

function openBackgroundPicker() {
  backgroundInputRef.value?.click();
}

async function handleBackgroundImport(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (file) {
    await backgroundStore.importLocalBackground(file);
  }
  input.value = '';
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.ctrlKey && e.key === 'd') {
    e.preventDefault();
    showDebug.value = !showDebug.value;
  }
}

function handleClickOutside(e: MouseEvent) {
  if (settingsRef.value && e.target instanceof Node && !settingsRef.value.contains(e.target)) {
    settingsOpen.value = false;
  }
}

// WebGL context loss handlers (hoisted so onUnmounted can remove them)
function handleContextLost(e: Event) {
  e.preventDefault();
  console.warn('[TerranSoul] WebGL context lost');
}
function handleContextRestored() {
  console.warn('[TerranSoul] WebGL context restored — reloading model');
  if (characterStore.vrmPath) {
    loadModelIntoScene(characterStore.vrmPath);
  }
}

onMounted(async () => {
  window.addEventListener('keydown', handleKeyDown);
  document.addEventListener('click', handleClickOutside);

  backgroundStore.ensureValidSelection();

  const canvas = canvasRef.value;
  if (!canvas) return;

  const ctx = await initScene(canvas);
  sceneCtx = ctx;
  disposeScene = ctx.dispose;
  getRendererInfo = ctx.getRendererInfo;

  // Apply the current pet-mode state now that the scene is available.
  // The reactive watch below only fires on subsequent changes — this
  // initial call catches the case where the user mounted already in pet
  // mode (e.g. re-open to a saved state).
  ctx.setPedestalVisible(!isPetMode.value);
  if (isPetMode.value) {
    ctx.controls.mouseButtons = {
      LEFT: null as unknown as THREE.MOUSE,
      MIDDLE: THREE.MOUSE.ROTATE,
      RIGHT: null as unknown as THREE.MOUSE,
    };
  }

  // Persist camera state after user finishes orbiting or zooming.
  ctx.onCameraChange((azimuth, distance) => {
    settingsStore.saveCameraState(azimuth, distance);
  });

  // Restore persisted camera state (azimuth + distance).
  // Skip in pet mode — always start with full-body framing.
  const savedAzimuth = settingsStore.settings.camera_azimuth;
  const savedDistance = settingsStore.settings.camera_distance;
  if (savedDistance > 0 && !isPetMode.value) {
    // Set camera position from saved spherical coordinates (elevation = 0 = equatorial)
    const x = savedDistance * Math.sin(savedAzimuth);
    const z = savedDistance * Math.cos(savedAzimuth);
    ctx.camera.position.set(x, ctx.camera.position.y, z);
    ctx.controls.update();
  }

  // Auto-load the default VRM model (loading overlay shows until ready).
  // If vrmPath is already set (HMR re-mount), reload it directly since
  // the watcher won't fire for an unchanged value.
  if (characterStore.vrmPath) {
    loadModelIntoScene(characterStore.vrmPath);
  } else {
    characterStore.loadDefaultModel();
  }

  // Handle WebGL context loss — reload model when the GPU context is restored
  canvas.addEventListener('webglcontextlost', handleContextLost);
  canvas.addEventListener('webglcontextrestored', handleContextRestored);

  // ── Cursor tracking: head/eye follow mouse pointer ────────────────
  // Converts mouse position to normalised viewport coords (-1..1) and
  // feeds it into the animator each frame for head/eye tracking.
  function handleCursorMove(e: MouseEvent) {
    const rect = canvas!.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;
    const nx = ((e.clientX - rect.left) / rect.width) * 2 - 1;
    const ny = ((e.clientY - rect.top) / rect.height) * 2 - 1;
    animator.setCursorPosition(nx, ny);
  }
  function handleCursorLeave() {
    // Smoothly return to centre when cursor leaves viewport
    animator.setCursorPosition(0, 0);
  }
  canvas.addEventListener('mousemove', handleCursorMove);
  canvas.addEventListener('mouseleave', handleCursorLeave);

  // Restore BGM state (track, volume, enabled) from persisted settings.
  restoreBgmFromSettings();

  // On-demand rendering: throttle to ~15 FPS when idle & settled
  const IDLE_INTERVAL = 1 / 15; // ~66ms
  let idleAccum = 0;

  function loop() {
    animFrameId = requestAnimationFrame(loop);
    const delta = ctx.clock.getDelta();

    // ── Auto-resize: ensure renderer matches canvas display size ──
    // This is the primary mechanism that prevents the "model invisible"
    // bug.  It catches v-show transitions, window resizes, and any other
    // case where the canvas display-size changes after initScene ran
    // with degenerate (0×0 or 1×1) dimensions.
    ctx.checkResize();

    // Adjust orbit target height based on zoom (face ↔ full body)
    ctx.updateZoomTarget();
    // Update OrbitControls (damping requires per-frame update)
    ctx.controls.update();

    // Eye tracking is now handled by the CharacterAnimator via cursor position.
    // The animator uses VRM lookAt yaw/pitch directly based on mouse position,
    // providing cursor-following eyes in both procedural and VRMA modes.

    const asm = animator.avatarStateMachine;
    const settled = animator.isAnimationSettled();
    const idle = asm.state.body === 'idle';

    if (settled && idle && !asm.state.needsRender && !vrmaManager.isPlaying) {
      // Throttle: accumulate time, only render at ~15 FPS
      idleAccum += delta;
      if (idleAccum < IDLE_INTERVAL) return;
      idleAccum = 0;
    } else {
      idleAccum = 0;
    }

    // Clear the one-shot render flag
    if (asm.state.needsRender) asm.state.needsRender = false;

    // Tick VRMA animation mixer (must be before animator.update which calls vrm.update)
    vrmaManager.update(delta);
    animator.update(delta);
    ctx.renderer.render(ctx.scene, ctx.camera);

    if (showDebug.value && getRendererInfo) {
      debugInfo.value = getRendererInfo();
    }
  }
  loop();
});

onUnmounted(() => {
  cancelAnimationFrame(animFrameId);
  disposeScene?.();
  bgm.stop();
  bgmDeferredCleanup?.();
  window.removeEventListener('keydown', handleKeyDown);
  document.removeEventListener('click', handleClickOutside);
  // Remove WebGL context loss listeners
  const canvas = canvasRef.value;
  if (canvas) {
    canvas.removeEventListener('webglcontextlost', handleContextLost);
    canvas.removeEventListener('webglcontextrestored', handleContextRestored);
  }
  vrmaManager.dispose();
  if (localVrmObjectUrl.value) {
    URL.revokeObjectURL(localVrmObjectUrl.value);
  }
});

watch(
  () => characterStore.state,
  (newState) => {
    animator.setState(newState);
    // Skip mood auto-play when an explicit motion is active (e.g. LLM said "clapping")
    if (vrmaManager.isMoodSuppressed) return;
    // Idle special-case: use character-gender weighted loop selection.
    if (newState === 'idle') {
      const idleEntry = getIdleAnimationForGender(characterStore.currentGender());
      if (idleEntry) {
        // Keep looping until mood/state changes away from idle.
        vrmaManager.play(idleEntry.path, true, 0.4);
      } else {
        vrmaManager.stop(0.4);
      }
      return;
    }
    // Try to play a VRMA animation mapped to this mood (one-shot, then return to procedural)
    const entry = getAnimationForMood(newState);
    if (entry) {
      vrmaManager.suppressMoodAnimation();
      vrmaManager.play(entry.path, false, 0.4);
    } else if (newState === 'talking') {
      // Return to procedural animation for talking
      vrmaManager.stop(0.4);
    }
  },
);

// ── Persona preview bridge ────────────────────────────────────────────────
// PersonaPanel (BrainView) writes requests; we consume them here.

watch(
  () => personaStore.previewExpressionRequest,
  (expr) => {
    if (!expr) return;
    const vrm = vrmaManager.vrm;
    if (vrm) {
      if (expressionPreviewTimer) clearTimeout(expressionPreviewTimer);
      applyLearnedExpression(vrm, expr);
      expressionPreviewTimer = setTimeout(() => {
        clearExpressionPreview(vrm);
        expressionPreviewTimer = null;
      }, 3000);
    }
    personaStore.previewExpressionRequest = null;
  },
);

watch(
  () => personaStore.previewMotionRequest,
  (motion) => {
    if (!motion) return;
    vrmaManager.suppressMoodAnimation();
    motionPlayer.play(motion, false, 0.4);
    personaStore.previewMotionRequest = null;
  },
);

// Hide the pedestal (and any other floor decorations) in pet mode so the
// character floats cleanly on the desktop with nothing visible behind.
// Remap mouse buttons: left-drag moves the pet (handled by PetOverlayView),
// middle-click drag rotates the 3D model, scroll wheel zooms.
watch(
  () => isPetMode.value,
  (pet) => {
    if (sceneCtx) {
      sceneCtx.setPedestalVisible(!pet);
      if (pet) {
        // Disable left-click rotation; use middle-button drag to rotate instead
        sceneCtx.controls.mouseButtons = {
          LEFT: null as unknown as THREE.MOUSE,
          MIDDLE: THREE.MOUSE.ROTATE,
          RIGHT: null as unknown as THREE.MOUSE,
        };
        // Reset camera to full-body zoom so the entire character is visible
        // by default in pet mode (including legs and shoes).
        sceneCtx.resetToFullBody();
      } else {
        // Restore default OrbitControls mouse mapping
        sceneCtx.controls.mouseButtons = {
          LEFT: THREE.MOUSE.ROTATE,
          MIDDLE: THREE.MOUSE.DOLLY,
          RIGHT: THREE.MOUSE.PAN,
        };
      }
    }
  },
  { immediate: true },
);

// Watch for VRM path changes and load the model
watch(
  () => characterStore.vrmPath,
  (newPath) => { loadModelIntoScene(newPath); },
);

/** Load a VRM model into the active scene. Shared by the watcher and onMounted (HMR recovery). */
async function loadModelIntoScene(newPath: string | undefined) {
    if (!newPath || !sceneCtx) return;

    // Remove the previous VRM model from the scene before loading a new one
    if (currentVrmScene) {
      sceneCtx.scene.remove(currentVrmScene);
      currentVrmScene = null;
    }

    try {
      // Race the VRM load against a timeout to prevent infinite "Loading model…"
      const VRM_LOAD_TIMEOUT_MS = 30_000;
      const loadPromise = loadVRMSafe(sceneCtx.scene, newPath);
      const timeoutPromise = new Promise<null>((resolve) =>
        setTimeout(() => resolve(null), VRM_LOAD_TIMEOUT_MS),
      );
      const result = await Promise.race([loadPromise, timeoutPromise]);
      if (result) {
        currentVrmScene = result.vrm.scene;
        // Hide the model initially — loadVRM already added it to the scene,
        // but we keep it invisible until everything (textures, morphs, bones)
        // is fully parsed so the user never sees hair dropping or half-loaded
        // geometry.  We reveal it below after the animator is wired up.
        result.vrm.scene.visible = false;

        // rotateVRM0() sets vrm.scene.rotation.y = Math.PI for VRM 0.x.
        // Capture whatever rotation the loader left on the scene root so the
        // animator preserves it every frame instead of overwriting it to 0.
        const model = DEFAULT_MODELS.find(m => m.path === newPath);
        const rotY = result.vrm.scene.rotation.y + (model?.rotationY ?? 0);
        animator.setVRM(result.vrm, rotY);
        // Wire up eye tracking — lookAtTarget is in the scene, updated per frame
        animator.setLookAtTarget(sceneCtx.lookAtTarget);
        // Bind VRMA manager to the loaded VRM for animation playback
        vrmaManager.setVRM(result.vrm);
        characterStore.setMetadata(result.metadata);

        // Expose VRM for E2E testing — allows Playwright to verify bone positions
        (window as unknown as Record<string, unknown>).__terransoul_vrm__ = result.vrm;

        // Run one animation tick so bones settle into the natural pose before
        // the first visible frame — this prevents the T-pose flash.
        animator.update(0);

        // Reframe the camera to fit this specific character's height so every
        // model appears fully visible and centred regardless of their size.
        sceneCtx.frameCameraToCharacter(result.vrm.scene);

        // Register the model for deferred reframe — if the canvas is still
        // hidden (display:none via v-show), the ResizeObserver will re-frame
        // once the canvas becomes visible with real dimensions.
        sceneCtx.setCurrentModel(result.vrm.scene);

        // Now reveal the fully-posed model and dismiss the loading overlay
        result.vrm.scene.visible = true;

        // Kick off the initial idle animation — the state watcher only fires
        // on *changes*, but the character is already in 'idle' state at load
        // time, so we need an explicit trigger here.
        if (characterStore.state === 'idle') {
          const idleEntry = getIdleAnimationForGender(characterStore.currentGender());
          if (idleEntry) {
            vrmaManager.play(idleEntry.path, true, 0.4);
          }
        }

        characterStore.setLoaded();
      } else {
        // Show a placeholder character so the scene isn't empty (load failed or timed out)
        console.warn('[TerranSoul] VRM load returned null — showing placeholder');
        const placeholder = createPlaceholderCharacter(sceneCtx.scene);
        currentVrmScene = placeholder;
        characterStore.setLoadError('Failed to load VRM model — try retry or a different character');
        characterStore.setLoaded();
      }
    } catch (error) {
      console.error('[TerranSoul] Model setup failed after VRM load:', error);
      // Ensure loading overlay is dismissed even if post-load setup fails
      const placeholder = createPlaceholderCharacter(sceneCtx.scene);
      currentVrmScene = placeholder;
      characterStore.setLoadError('Model loaded but failed to initialise');
      characterStore.setLoaded();
    }
}
</script>

<style scoped>
.viewport-wrapper {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.background-layer {
  position: absolute;
  inset: 0;
  background-position: center;
  background-repeat: no-repeat;
  background-size: cover;
  z-index: 0;
}

.background-tint {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(ellipse at center, transparent 40%, rgba(10, 15, 30, 0.35) 100%),
    linear-gradient(180deg, rgba(15, 23, 42, 0.06) 0%, rgba(15, 23, 42, 0.20) 100%);
  z-index: 1;
  pointer-events: none;
}

.viewport-canvas {
  position: relative;
  z-index: 2;
  width: 100%;
  height: 100%;
  display: block;
}

/* Top-left chrome shares a row with the floating mode-toggle pill that
 * App.vue renders at (left: 82px). To prevent overlap every
 * in-viewport overlay is nudged right to start after the pill + a gap.
 * The settings button sits in the top-right corner instead. */
.character-name-overlay {
  position: absolute;
  top: 12px;
  left: 280px;
  font-size: var(--ts-text-lg);
  font-weight: 700;
  color: var(--ts-viewport-text);
  text-shadow: 0 1px 6px rgba(0, 0, 0, 0.7), 0 0 20px rgba(0, 0, 0, 0.3);
  pointer-events: none;
  letter-spacing: 0.05em;
  background: var(--ts-viewport-bg);
  padding: 2px 8px;
  border-radius: 4px;
}

.character-meta-overlay {
  position: absolute;
  top: 34px;
  left: 280px;
  font-size: 0.72rem;
  color: var(--ts-viewport-text-dim);
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
  pointer-events: none;
  letter-spacing: 0.02em;
  background: var(--ts-viewport-bg);
  padding: 2px 8px;
  border-radius: 4px;
}

/* ── Corner settings button ── */
/* Placed top-right on all viewports to avoid colliding with the
   App.vue mode-toggle pill that occupies the top-left zone. */
.settings-corner {
  position: absolute;
  top: 12px;
  right: 16px;
  z-index: 20;
}

.settings-toggle {
  appearance: none;
}
.settings-label {
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.03em;
}

.settings-dropdown {
  position: absolute;
  top: 42px;
  right: 0;
  width: 280px;
  max-width: min(280px, 90vw);
  max-height: min(500px, 70vh);
  overflow-y: auto;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  z-index: 30;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.settings-header-title {
  font-size: var(--ts-text-sm);
  font-weight: 700;
  color: var(--ts-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}
.settings-close-btn {
  background: none;
  border: none;
  color: var(--ts-text-dim);
  font-size: 1.4rem;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
  border-radius: var(--ts-radius-sm);
  transition: color 0.15s, background 0.15s;
}
.settings-close-btn:hover {
  color: var(--ts-text-primary);
  background: rgba(255, 255, 255, 0.1);
}

.dropdown-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.dropdown-label {
  font-size: var(--ts-text-xs);
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--ts-text-dim);
}

.dropdown-btn {
  padding: 6px 10px;
  border-radius: var(--ts-radius-sm);
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-input);
  color: var(--ts-text-secondary);
  font-size: var(--ts-text-sm);
  cursor: pointer;
  transition: background var(--ts-transition-fast);
  text-align: left;
}
.dropdown-btn:hover {
  background: var(--ts-bg-hover);
}

.bg-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

/* ── Mood grid ── */
.mood-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 4px;
}
.mood-chip {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 6px 4px;
  border-radius: var(--ts-radius-sm);
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-input);
  color: var(--ts-text-secondary);
  font-size: 0.66rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  cursor: pointer;
  transition: background var(--ts-transition-fast), border-color var(--ts-transition-fast), transform var(--ts-transition-fast);
}
.mood-chip:hover {
  background: rgba(255, 255, 255, 0.14);
  transform: translateY(-1px);
}
.mood-chip.active {
  background: rgba(124, 111, 255, 0.85);
  border-color: rgba(200, 210, 255, 0.85);
  color: #fff;
}
.mood-chip-emoji {
  font-size: 1.05rem;
  line-height: 1;
}
.mood-chip-label {
  font-size: 0.62rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}

/* Dropdown transition */
.dropdown-enter-active, .dropdown-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}
.dropdown-enter-from, .dropdown-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.96);
}

.model-selector {
  width: 100%;
  padding: 7px 28px 7px 10px;
  border-radius: var(--ts-radius-md);
  border: 1px solid rgba(255, 255, 255, 0.15);
  background: rgba(255, 255, 255, 0.06);
  color: var(--ts-viewport-text);
  font-size: 0.82rem;
  cursor: pointer;
  outline: none;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='rgba(255,255,255,0.7)'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
  transition: border-color var(--ts-transition-fast);
}
.model-selector:hover {
  border-color: rgba(108, 99, 255, 0.5);
}
.model-selector option {
  background: var(--ts-bg-surface);
  color: var(--ts-text-primary);
}

.hidden-file-input {
  display: none;
}

.background-chip {
  padding: 5px 10px;
  border-radius: var(--ts-radius-pill);
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  font-size: 0.72rem;
  font-weight: 600;
  cursor: pointer;
  transition: background var(--ts-transition-fast), border-color var(--ts-transition-fast), transform var(--ts-transition-fast);
}
.background-chip:hover {
  background: rgba(255, 255, 255, 0.18);
  transform: translateY(-1px);
}
.background-chip.active {
  background: rgba(124, 111, 255, 0.85);
  border-color: rgba(200, 210, 255, 0.85);
}

.background-error-banner {
  position: absolute;
  top: 56px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 6;
  padding: 8px 12px;
  border-radius: 10px;
  background: rgba(127, 29, 29, 0.82);
  color: #fee2e2;
  font-size: 0.76rem;
  font-weight: 600;
  backdrop-filter: blur(8px);
}

.debug-overlay {
  position: absolute;
  bottom: 10px;
  left: 10px;
  display: flex;
  gap: 10px;
  padding: 4px 10px;
  border-radius: var(--ts-radius-sm);
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  font-size: 0.7rem;
  font-family: var(--ts-font-mono);
  color: #7ef5a0;
  pointer-events: none;
  letter-spacing: 0.02em;
}

/* Mobile adjustments for viewport overlays.
 * On mobile (<= 640px) the sidebar is hidden and space is limited.
 * We hide character name/meta (the user already knows their character)
 * and show only the essential controls: mode toggle (top-left), settings
 * gear (top-right beside AI pill), with brain/music below. */
@media (max-width: 640px) {
  /* Hide character name & meta on mobile — screen is too narrow */
  .character-name-overlay { display: none; }
  .character-meta-overlay { display: none; }
  /* Settings gear: compact circle in top-right area */
  .settings-toggle {
    height: 32px;
    width: 32px;
    padding: 0;
    font-size: 0.7rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .settings-label { display: none; }
  .settings-corner {
    top: 6px;
    right: 10px;
  }
  /* Dropdown: narrower on mobile, already right-aligned */
  .settings-dropdown {
    width: min(280px, calc(100vw - 20px));
    padding: 10px;
    gap: 10px;
  }
}
.loading-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(6px);
  z-index: 10;
  pointer-events: none;
}

.loading-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(255, 255, 255, 0.15);
  border-top-color: rgba(108, 99, 255, 0.9);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loading-text {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--ts-viewport-text-med);
  letter-spacing: 0.05em;
}

.load-error-overlay {
  background: rgba(0, 0, 0, 0.45);
  pointer-events: auto;
}

.load-error-icon {
  font-size: 2rem;
}

.load-error-retry {
  margin-top: 4px;
  padding: 6px 20px;
  border: 1px solid rgba(255, 255, 255, 0.3);
  border-radius: 6px;
  background: rgba(108, 99, 255, 0.6);
  color: #fff;
  font-size: 0.8rem;
  cursor: pointer;
  transition: background 0.2s;
}
.load-error-retry:hover {
  background: rgba(108, 99, 255, 0.85);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.4s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* ── BGM Controls (settings dropdown) ── */
.bgm-toggle-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.bgm-status {
  font-size: 0.75rem;
  color: var(--ts-viewport-text-med);
  font-weight: 600;
}

.bgm-switch {
  position: relative;
  width: 36px;
  height: 20px;
  cursor: pointer;
}

.bgm-switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.bgm-slider {
  position: absolute;
  inset: 0;
  background: rgba(255, 255, 255, 0.22);
  border-radius: 10px;
  transition: background 0.3s;
}

.bgm-slider::before {
  content: '';
  position: absolute;
  width: 16px;
  height: 16px;
  left: 2px;
  bottom: 2px;
  background: white;
  border-radius: 50%;
  transition: transform 0.3s;
}

.bgm-switch input:checked + .bgm-slider {
  background: rgba(56, 189, 248, 0.85);
}

.bgm-switch input:checked + .bgm-slider::before {
  transform: translateX(16px);
}

.bgm-volume-row {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 4px;
}

.bgm-vol-icon {
  font-size: 0.7rem;
  opacity: 0.7;
}

.bgm-volume-slider {
  flex: 1;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: rgba(255, 255, 255, 0.25);
  border-radius: 2px;
  outline: none;
  cursor: pointer;
}

.bgm-volume-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  background: rgba(56, 189, 248, 0.95);
  border-radius: 50%;
  cursor: pointer;
}

.bgm-volume-slider::-moz-range-thumb {
  width: 14px;
  height: 14px;
  background: rgba(56, 189, 248, 0.95);
  border-radius: 50%;
  cursor: pointer;
  border: none;
}

/* ── Floating Music Bar (teleported to #music-bar-portal on the left) ── */
.music-bar {
  position: relative;
  display: flex;
  align-items: center;
  flex-direction: row;
  gap: 0;
}

.music-bar-toggle {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-overlay);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(8px);
  transition: background 0.2s, transform 0.15s;
  flex-shrink: 0;
  z-index: 2;
  box-shadow: var(--ts-shadow-md);
}
.music-bar-toggle:hover {
  background: var(--ts-bg-hover);
  transform: scale(1.1);
}
.music-bar-toggle-icon {
  font-size: 0.85rem;
  transition: transform 0.2s;
}
.music-bar-toggle-icon.open {
  font-size: 0.7rem;
}

@keyframes music-pulse {
  0%, 100% { transform: scale(1); }
  50% { transform: scale(1.15); }
}

.music-bar-panel {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: 6px;
  padding: 6px 10px;
  border-radius: 20px;
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-overlay);
  backdrop-filter: blur(16px);
  box-shadow: var(--ts-shadow-lg);
}

.music-btn {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  border: none;
  background: var(--ts-bg-input);
  color: var(--ts-viewport-text);
  font-size: 0.85rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s, transform 0.1s;
  flex-shrink: 0;
}
.music-btn:hover {
  background: var(--ts-accent-glow);
  transform: scale(1.1);
}
.music-btn.play-btn {
  background: var(--ts-accent-glow);
  width: 34px;
  height: 34px;
}
.music-btn.play-btn:hover {
  background: color-mix(in srgb, var(--ts-accent-blue) 35%, transparent);
}
.music-btn.add-btn {
  font-size: 0.9rem;
}

.music-track-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  max-width: 110px;
}
.music-track-name {
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--ts-viewport-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  letter-spacing: 0.02em;
}

.music-vol-slider {
  width: 60px;
  height: 3px;
  -webkit-appearance: none;
  appearance: none;
  background: var(--ts-border-strong);
  border-radius: 2px;
  outline: none;
  cursor: pointer;
  flex-shrink: 0;
}
.music-vol-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 12px;
  height: 12px;
  background: var(--ts-accent-blue);
  border-radius: 50%;
  cursor: pointer;
}
.music-vol-slider::-moz-range-thumb {
  width: 12px;
  height: 12px;
  background: var(--ts-accent-blue);
  border-radius: 50%;
  cursor: pointer;
  border: none;
}

/* Music bar expand transition */
.music-expand-enter-active, .music-expand-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.music-expand-enter-from, .music-expand-leave-to {
  opacity: 0;
  transform: translateX(12px) scale(0.9);
}

@media (max-width: 640px) {
  .music-bar-toggle { width: 30px; height: 30px; }
  .music-bar-panel { padding: 4px 8px; gap: 4px; }
  .music-vol-slider { width: 44px; }
  .music-track-info { max-width: 80px; }
}

/* ── BGM custom track controls ── */
.bgm-track-actions {
  display: flex;
  gap: 6px;
  margin-top: 4px;
}
.bgm-track-actions .dropdown-btn {
  flex: 1;
  font-size: 0.7rem;
  padding: 4px 6px;
}

.bgm-custom-list {
  margin-top: 6px;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.bgm-custom-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 3px 6px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.05);
}
.bgm-custom-name {
  font-size: 0.68rem;
  color: var(--ts-viewport-text-med);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
  flex: 1;
}
.bgm-remove-btn {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: none;
  background: rgba(239, 68, 68, 0.25);
  color: rgba(239, 68, 68, 0.9);
  font-size: 0.65rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-left: 6px;
  transition: background 0.15s;
}
.bgm-remove-btn:hover {
  background: rgba(239, 68, 68, 0.5);
}

/* ── URL dialog ── */
.url-dialog-backdrop {
  position: absolute;
  inset: 0;
  z-index: 50;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(4px);
}
.url-dialog {
  background: rgba(15, 23, 42, 0.96);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 12px;
  padding: 20px;
  min-width: 320px;
  max-width: 90%;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}
.url-dialog-label {
  display: block;
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--ts-viewport-text);
  margin-bottom: 10px;
}
.url-dialog-input {
  width: 100%;
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  background: rgba(255, 255, 255, 0.06);
  color: var(--ts-viewport-text);
  font-size: 0.8rem;
  outline: none;
  box-sizing: border-box;
}
.url-dialog-input:focus {
  border-color: rgba(56, 189, 248, 0.5);
}
.url-dialog-actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
  justify-content: flex-end;
}
.url-dialog-btn {
  padding: 6px 16px;
  border-radius: 8px;
  border: none;
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s;
}
.url-dialog-btn.cancel {
  background: rgba(255, 255, 255, 0.08);
  color: var(--ts-viewport-text-dim);
}
.url-dialog-btn.cancel:hover {
  background: rgba(255, 255, 255, 0.15);
}
.url-dialog-btn.confirm {
  background: rgba(56, 189, 248, 0.3);
  color: rgba(56, 189, 248, 1);
}
.url-dialog-btn.confirm:hover {
  background: rgba(56, 189, 248, 0.5);
}
.url-dialog-btn.confirm:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
