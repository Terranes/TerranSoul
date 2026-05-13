<template>
  <div
    class="viewport-wrapper"
    :class="{ 'viewport-wrapper--pet': isPetMode }"
  >
    <!-- Background layer: only shown when a user-imported local image is active.
         In Auto mode the body CSS gradient (--ts-bg-gradient) shows through. -->
    <div
      v-if="!isPetMode && backgroundStore.currentBackground.kind !== 'auto'"
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
    <!-- ── Top bubble strip — Settings (left), Model (middle), Status (right) ── -->
    <div
      v-if="!isPetMode"
      ref="settingsRef"
      class="corner-cluster"
    >
      <!-- Settings host: own positioned wrapper so the dropdown anchors to
           the trigger, not to the whole cluster. -->
      <div class="settings-host">
        <FloatingChip
          as="button"
          class="settings-toggle"
          interactive
          type="button"
          aria-label="Settings"
          @click.stop="toggleSettingsDialog"
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
            v-if="settingsOpen && !props.hideSettingsDialog"
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
            <!-- View mode selector — 3D / Chat / Pet -->
            <div class="dropdown-section">
              <label class="dropdown-label">View Mode</label>
              <div class="settings-mode-row">
                <button
                  class="settings-mode-btn"
                  :class="{ active: !isPetMode }"
                  @click="emit('set-display-mode', 'desktop'); settingsOpen = false"
                >
                  🖥 3D
                </button>
                <button
                  class="settings-mode-btn"
                  @click="emit('set-display-mode', 'chatbox'); settingsOpen = false"
                >
                  💬 Chat
                </button>
                <button
                  class="settings-mode-btn"
                  @click="emit('toggle-pet-mode'); settingsOpen = false"
                >
                  🐾 Pet
                </button>
              </div>
            </div>
            <!-- Quest progress — inline inside settings -->
            <div class="dropdown-section">
              <label class="dropdown-label">Quests</label>
              <div
                id="corner-cluster-portal"
                class="settings-quest-portal"
              />
            </div>
            <!-- Model selector -->
            <div class="dropdown-section">
              <label class="dropdown-label">Character</label>
              <select
                class="model-selector"
                :value="characterStore.selectedModelId"
                @change="handleModelChange"
              >
                <optgroup label="Bundled">
                  <option
                    v-for="model in characterStore.defaultModels"
                    :key="model.id"
                    :value="model.id"
                  >
                    {{ characterStore.resolveModelProfile(model).name }}
                  </option>
                </optgroup>
                <optgroup
                  v-if="characterStore.userModels.length > 0"
                  label="Imported"
                >
                  <option
                    v-for="model in characterStore.userModels"
                    :key="model.id"
                    :value="model.id"
                  >
                    {{ characterStore.resolveModelProfile(model).name }}
                  </option>
                </optgroup>
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
              <div class="character-profile-editor">
                <label class="profile-field">
                  <span>Name</span>
                  <input
                    v-model="profileDraftName"
                    type="text"
                    maxlength="60"
                    placeholder="Character name"
                  >
                </label>
                <label class="profile-field">
                  <span>Gender</span>
                  <select
                    v-model="profileDraftGender"
                    @change="handleProfileGenderChange"
                  >
                    <option
                      v-for="option in genderOptions"
                      :key="option.value"
                      :value="option.value"
                    >
                      {{ option.label }}
                    </option>
                  </select>
                </label>
                <label class="profile-field profile-field-full">
                  <span>Persona / role</span>
                  <input
                    v-model="profileDraftPersona"
                    type="text"
                    maxlength="80"
                    placeholder="e.g. field researcher"
                  >
                </label>
                <label class="profile-field profile-field-full">
                  <span>Voice</span>
                  <input
                    v-model="profileDraftVoiceName"
                    type="text"
                    maxlength="120"
                    placeholder="e.g. en-US-AnaNeural"
                  >
                </label>
                <div class="profile-grid">
                  <label class="profile-field">
                    <span>Age</span>
                    <select v-model="profileDraftAge">
                      <option
                        v-for="option in ageOptions"
                        :key="option.value"
                        :value="option.value"
                      >
                        {{ option.label }}
                      </option>
                    </select>
                  </label>
                  <label class="profile-field">
                    <span>Pitch</span>
                    <select v-model="profileDraftPitch">
                      <option
                        v-for="option in pitchOptions"
                        :key="option.value"
                        :value="option.value"
                      >
                        {{ option.label }}
                      </option>
                    </select>
                  </label>
                  <label class="profile-field">
                    <span>Style</span>
                    <select v-model="profileDraftStyle">
                      <option
                        v-for="option in styleOptions"
                        :key="option.value"
                        :value="option.value"
                      >
                        {{ option.label }}
                      </option>
                    </select>
                  </label>
                  <label class="profile-field">
                    <span>Accent</span>
                    <select v-model="profileDraftEnglishAccent">
                      <option
                        v-for="option in englishAccentOptions"
                        :key="option.value"
                        :value="option.value"
                      >
                        {{ option.label }}
                      </option>
                    </select>
                  </label>
                </div>
                <label class="profile-field profile-field-full">
                  <span>Chinese dialect</span>
                  <select v-model="profileDraftChineseDialect">
                    <option
                      v-for="option in chineseDialectOptions"
                      :key="option.value"
                      :value="option.value"
                    >
                      {{ option.label }}
                    </option>
                  </select>
                </label>
                <div class="profile-actions">
                  <button
                    class="profile-save-btn"
                    :disabled="profileSaving"
                    @click="saveCurrentCharacterProfile"
                  >
                    {{ profileSaving ? 'Saving…' : 'Save profile' }}
                  </button>
                  <button
                    class="profile-reset-btn"
                    :disabled="profileSaving"
                    @click="resetProfileDraft"
                  >
                    Discard
                  </button>
                </div>
                <p
                  v-if="profileStatus"
                  class="profile-status"
                >
                  {{ profileStatus }}
                </p>
                <p
                  v-if="profileError"
                  class="profile-error"
                >
                  {{ profileError }}
                </p>
              </div>
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
                v-show="bgmEnabled"
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

            <div class="dropdown-section">
              <label class="dropdown-label">Karaoke Dialog</label>
              <div class="bgm-toggle-row">
                <label class="bgm-switch">
                  <input
                    type="checkbox"
                    :checked="karaokeDialogEnabled"
                    @change="handleKaraokeToggle"
                  >
                  <span class="bgm-slider" />
                </label>
                <span class="bgm-status">{{ karaokeDialogEnabled ? 'On' : 'Off' }}</span>
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
      </div> <!-- /.settings-host -->
    </div>

    <div
      v-if="backgroundStore.importError"
      class="background-error-banner"
    >
      {{ backgroundStore.importError }}
    </div>
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
import { listen } from '@tauri-apps/api/event';
import { useCharacterStore } from '../stores/character';
import { useBackgroundStore } from '../stores/background';
import { useSettingsStore } from '../stores/settings';
import { useAudioStore } from '../stores/audio';
import { useWindowStore } from '../stores/window';
import { usePersonaStore } from '../stores/persona';
import { DEFAULT_MODELS, GENDER_VOICES, type ModelGender } from '../config/default-models';
import {
  PERSONA_CHINESE_DIALECT_OPTIONS,
  PERSONA_ENGLISH_ACCENT_OPTIONS,
  PERSONA_VOICE_AGE_OPTIONS,
  PERSONA_VOICE_GENDER_OPTIONS,
  PERSONA_VOICE_PITCH_OPTIONS,
  PERSONA_VOICE_STYLE_OPTIONS,
  type PersonaChineseDialect,
  type PersonaEnglishAccent,
  type PersonaVoiceAge,
  type PersonaVoicePitch,
  type PersonaVoiceStyle,
} from '../stores/persona-types';
import { initScene, type RendererInfo, type SceneContext } from '../renderer/scene';
import { loadVRMSafe, createPlaceholderCharacter } from '../renderer/vrm-loader';
import { CharacterAnimator } from '../renderer/character-animator';
import { VrmaManager, getAnimationForMood, getAnimationForMotion, getIdleAnimationForGender, getStandingAnimationForMood, SITTING_ANIMATION_PATHS } from '../renderer/vrma-manager';
import { LearnedMotionPlayer, applyLearnedExpression, clearExpressionPreview } from '../renderer/learned-motion-player';
import { PoseAnimator, type LlmPoseFrame } from '../renderer/pose-animator';
import { EmotionPoseBias, type BiasEmotion } from '../renderer/emotion-pose-bias';
import { SittingPropController } from '../renderer/sitting-props-controller';
import { useBgmPlayer, BGM_TRACKS, type BgmTrack } from '../composables/useBgmPlayer';
import { MOOD_ENTRIES, isMoodActive, applyMood, type MoodEntry } from '../config/moods';
import { subscribeLlmPoseFrames, type LlmPoseListen } from '../utils/llm-pose-events';
import SystemInfoPanel from './SystemInfoPanel.vue';
import AudioControlsPanel from './AudioControlsPanel.vue';
import FloatingChip from './ui/FloatingChip.vue';
import FloatingMenu from './ui/FloatingMenu.vue';
import ThemePicker from './ThemePicker.vue';

const emit = defineEmits<{
  'request-add-music': [];
  'overlay-open': [open: boolean];
  'set-display-mode': [mode: 'desktop' | 'chatbox'];
  'toggle-pet-mode': [];
}>();

const props = withDefaults(defineProps<{
  /** Force transparent pet rendering even when the app window is in normal mode. */
  forcePet?: boolean;
  /** Hide/close the settings dialog while chat history is expanded. */
  hideSettingsDialog?: boolean;
}>(), {
  forcePet: false,
  hideSettingsDialog: false,
});

const canvasRef = ref<HTMLCanvasElement | null>(null);
const characterStore = useCharacterStore();
const backgroundStore = useBackgroundStore();
const settingsStore = useSettingsStore();
const audioStore = useAudioStore();
const windowStoreLocal = useWindowStore();
const personaStore = usePersonaStore();
/** Viewport behaves differently in pet mode: no background, no chrome,
 *  transparent clear colour, and pedestal hidden in the 3D scene. */
const isPetMode = computed(() => props.forcePet || windowStoreLocal.mode === 'pet');

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
const bgmFileInputRef = ref<HTMLInputElement | null>(null);
const showUrlDialog = ref(false);
const urlInput = ref('');
const karaokeDialogEnabled = computed(() => settingsStore.settings.karaoke_dialog_enabled !== false);

// ── Current character profile editor ────────────────────────────────────────
const genderOptions = PERSONA_VOICE_GENDER_OPTIONS;
const ageOptions = PERSONA_VOICE_AGE_OPTIONS;
const pitchOptions = PERSONA_VOICE_PITCH_OPTIONS;
const styleOptions = PERSONA_VOICE_STYLE_OPTIONS;
const englishAccentOptions = PERSONA_ENGLISH_ACCENT_OPTIONS;
const chineseDialectOptions = PERSONA_CHINESE_DIALECT_OPTIONS;
const profileDraftName = ref('');
const profileDraftGender = ref<ModelGender>('female');
const profileDraftPersona = ref('');
const profileDraftVoiceName = ref('');
const profileDraftAge = ref<PersonaVoiceAge>('adult');
const profileDraftPitch = ref<PersonaVoicePitch>('medium');
const profileDraftStyle = ref<PersonaVoiceStyle>('natural');
const profileDraftEnglishAccent = ref<PersonaEnglishAccent>('american');
const profileDraftChineseDialect = ref<PersonaChineseDialect>('mandarin');
const profileSaving = ref(false);
const profileStatus = ref('');
const profileError = ref('');

function resetProfileDraft() {
  const profile = characterStore.currentModelProfile();
  profileDraftName.value = profile.name;
  profileDraftGender.value = profile.gender;
  profileDraftPersona.value = profile.persona;
  profileDraftVoiceName.value = profile.voiceProfile.voiceName;
  profileDraftAge.value = profile.voiceProfile.age;
  profileDraftPitch.value = profile.voiceProfile.pitch;
  profileDraftStyle.value = profile.voiceProfile.style;
  profileDraftEnglishAccent.value = profile.voiceProfile.englishAccent;
  profileDraftChineseDialect.value = profile.voiceProfile.chineseDialect;
  profileStatus.value = '';
  profileError.value = '';
}

function usesDefaultVoice(voiceName: string): boolean {
  const trimmed = voiceName.trim();
  return !trimmed || Object.values(GENDER_VOICES).some(voice => voice.edgeVoice === trimmed);
}

function handleProfileGenderChange() {
  if (usesDefaultVoice(profileDraftVoiceName.value)) {
    profileDraftVoiceName.value = GENDER_VOICES[profileDraftGender.value].edgeVoice;
  }
}

async function saveCurrentCharacterProfile() {
  profileSaving.value = true;
  profileStatus.value = '';
  profileError.value = '';
  try {
    await characterStore.updateModelProfile(characterStore.selectedModelId, {
      name: profileDraftName.value || undefined,
      gender: profileDraftGender.value,
      persona: profileDraftPersona.value,
      voiceProfile: {
        gender: profileDraftGender.value,
        age: profileDraftAge.value,
        pitch: profileDraftPitch.value,
        style: profileDraftStyle.value,
        englishAccent: profileDraftEnglishAccent.value,
        chineseDialect: profileDraftChineseDialect.value,
        voiceName: profileDraftVoiceName.value,
      },
    });
    profileStatus.value = 'Saved for this character.';
  } catch (err) {
    profileError.value = `Profile save failed: ${err}`;
  } finally {
    profileSaving.value = false;
  }
}

watch(
  () => [characterStore.selectedModelId, characterStore.modelProfiles] as const,
  () => resetProfileDraft(),
  { immediate: true, deep: true },
);

function handleKaraokeToggle(e: Event) {
  const checked = (e.target as HTMLInputElement).checked;
  void settingsStore.setKaraokeDialogEnabled(checked);
}

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

// React to mute toggles — silence BGM immediately and restore prior volume
// on unmute. Voice (TTS) is muted independently inside `useTtsPlayback`
// via the `mutedRef` option wired up in ChatView / PetOverlayView.
watch(
  () => audioStore.muted,
  (isMuted) => {
    bgm.setVolume(isMuted ? 0 : bgmVolume.value);
  },
);

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

const backgroundStyle = computed(() => {
  const bg = backgroundStore.currentBackground;
  // Auto: no inline style — body CSS gradient (via --ts-bg-gradient) shows through.
  if (bg.kind === 'auto') return {};
  return { backgroundImage: `url("${bg.url}")` };
});

let animFrameId = 0;
let disposeScene: (() => void) | null = null;
let getRendererInfo: (() => RendererInfo) | null = null;
let sceneCtx: SceneContext | null = null;
let currentVrmScene: THREE.Object3D | null = null;
const animator = new CharacterAnimator();
const vrmaManager = new VrmaManager();
const motionPlayer = new LearnedMotionPlayer(vrmaManager);
const poseAnimator = new PoseAnimator();
const emotionBias = new EmotionPoseBias();
let expressionPreviewTimer: ReturnType<typeof setTimeout> | null = null;
let unlistenLlmPose: (() => void) | null = null;
let viewportUnmounted = false;

// ── Sitting chair prop ───────────────────────────────────────────────
const sittingPropController = new SittingPropController();
// In pet / forcePet mode the floating preview has no floor — a chair would
// visibly hover beside the avatar. Disable the prop entirely in that case.
sittingPropController.disabled = isPetMode.value;
watch(isPetMode, (pet) => {
  sittingPropController.disabled = pet;
  if (pet) {
    sittingPropController.dispose(sceneCtx?.scene ?? null);
  }
});

function disposeSittingProps() {
  sittingPropController.dispose(sceneCtx?.scene ?? null);
}

async function subscribeToLlmPoseEvents() {
  try {
    const unlisten = await subscribeLlmPoseFrames(listen as LlmPoseListen, (frame) => {
      poseAnimator.applyFrame(frame);
    });
    if (viewportUnmounted) {
      unlisten();
    } else {
      unlistenLlmPose = unlisten;
    }
  } catch {
    // Browser mode has no Tauri event bus; pose frames arrive only in native shells.
  }
}

function syncSittingProps(playing: boolean) {
  sittingPropController.sync(sceneCtx?.scene ?? null, playing, vrmaManager.currentPath);
}

// Wire VRMA playback state to the animator + lazy-load sitting props.
// When a non-looping mood animation finishes, return the character to idle
// so the idle VRMA loop restarts and the character doesn't appear frozen.
vrmaManager.onPlaybackChange((playing) => {
  animator.setVrmaPlaying(playing);
  poseAnimator.setVrmaPlaying(playing);
  syncSittingProps(playing);

  if (!playing) {
    // A VRMA clip just ended — if the character is in an emotional state
    // (not idle/talking/thinking) that means a one-shot mood animation finished.
    // Transition back to idle so the idle loop restarts.
    const s = characterStore.state;
    if (s !== 'idle' && s !== 'talking' && s !== 'thinking') {
      characterStore.setState('idle');
    }
  }
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
  /**
   * Apply an LLM-generated pose frame (chunk 14.16b3). The frame is
   * additively layered on top of the procedural idle animation; a
   * VRMA-driven body animation, when active, wins automatically.
   */
  playPose(frame: LlmPoseFrame) {
    poseAnimator.applyFrame(frame);
  },
  /** Clear any in-flight LLM pose and fade back to procedural idle. */
  clearPose() {
    poseAnimator.reset();
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

function toggleSettingsDialog() {
  if (props.hideSettingsDialog) return;
  settingsOpen.value = !settingsOpen.value;
}

watch(
  () => props.hideSettingsDialog,
  (hidden) => {
    if (hidden && settingsOpen.value) settingsOpen.value = false;
  },
  { immediate: true },
);

watch(
  [settingsOpen, showSystemInfo, showAudioControls, showUrlDialog],
  ([settingsDialogOpen, systemInfoOpen, audioControlsOpen, urlDialogOpen]) => {
    const overlayOpen = (settingsDialogOpen && !props.hideSettingsDialog)
      || systemInfoOpen
      || audioControlsOpen
      || urlDialogOpen;
    emit('overlay-open', overlayOpen);
  },
  { immediate: true },
);

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
  viewportUnmounted = false;
  window.addEventListener('keydown', handleKeyDown);
  document.addEventListener('click', handleClickOutside);
  void subscribeToLlmPoseEvents();

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
    // Browser landing preview keeps full LEFT-drag rotation so visitors
    // can interact; real desktop pet mode disables LEFT/RIGHT so the OS
    // window-drag handler can claim them. (Mirrors the watcher below.)
    ctx.controls.mouseButtons = props.forcePet
      ? {
          LEFT: THREE.MOUSE.ROTATE,
          MIDDLE: THREE.MOUSE.DOLLY,
          RIGHT: THREE.MOUSE.PAN,
        }
      : {
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
    // Layer the LLM-as-Animator pose blender on top of the procedural
    // bones written by `animator.update`. Runs after so its additive
    // offsets sit on the most recent rotation values.
    poseAnimator.apply(vrmaManager.vrm, delta);
    // Emotion-reactive procedural pose bias (Chunk 14.16d). Yields
    // when a baked VRMA clip or an LLM pose is in charge so it never
    // fights the higher-priority animation source.
    emotionBias.apply(
      vrmaManager.vrm,
      delta,
      vrmaManager.isPlaying || poseAnimator.isActive,
    );
    ctx.renderer.render(ctx.scene, ctx.camera);

    if (showDebug.value && getRendererInfo) {
      debugInfo.value = getRendererInfo();
    }
  }
  loop();
});

onUnmounted(() => {
  viewportUnmounted = true;
  cancelAnimationFrame(animFrameId);
  unlistenLlmPose?.();
  unlistenLlmPose = null;
  disposeSittingProps();
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

// Track last mood animation to prevent re-triggering the same one
// (e.g. multiple <anim> tags with the same emotion in one response).
let lastMoodAnimState: string | null = null;

watch(
  () => characterStore.state,
  (newState) => {
    animator.setState(newState, characterStore.emotionIntensity);
    // Drive the emotion-reactive procedural pose bias (Chunk 14.16d).
    // `thinking` / `talking` are body states with no postural mood —
    // map them to neutral so we don't double-stack with talking gestures.
    const biasMap: Record<typeof newState, BiasEmotion> = {
      idle: 'neutral',
      thinking: 'neutral',
      talking: 'neutral',
      happy: 'happy',
      sad: 'sad',
      angry: 'angry',
      relaxed: 'relaxed',
      surprised: 'surprised',
    };
    emotionBias.setEmotion(biasMap[newState] ?? 'neutral', characterStore.emotionIntensity);
    // Skip mood auto-play when an explicit motion is active (e.g. LLM said "clapping")
    if (vrmaManager.isMoodSuppressed) return;
    // Idle special-case: use character-gender weighted loop selection.
    if (newState === 'idle') {
      lastMoodAnimState = null; // reset so next emotion can play
      const idleEntry = getIdleAnimationForGender(
        characterStore.currentGender(),
        Math.random,
        isPetMode.value,
      );
      if (idleEntry) {
        // Keep looping until mood/state changes away from idle.
        vrmaManager.play(idleEntry.path, true, 0.4);
      } else {
        vrmaManager.stop(0.4);
      }
      return;
    }
    // Try to play a VRMA animation mapped to this mood (one-shot, then return to procedural).
    // Skip if the same mood animation is already playing — prevents
    // re-triggering per sentence when multiple <anim> tags emit the
    // same emotion during a streamed response.
    if (newState === lastMoodAnimState) return;
    // In pet/forcePet mode, prefer the standing variant so we don't spawn a chair
    // floating in mid-air next to the small floating preview.
    const entry = isPetMode.value
      ? (getStandingAnimationForMood(newState) ?? getAnimationForMood(newState))
      : getAnimationForMood(newState);
    if (entry && (!isPetMode.value || !SITTING_ANIMATION_PATHS.has(entry.path))) {
      lastMoodAnimState = newState;
      vrmaManager.suppressMoodAnimation();
      vrmaManager.play(entry.path, false, 0.4);
    } else if (newState === 'talking') {
      lastMoodAnimState = null; // reset so emotion after talking can play
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
// Remap mouse buttons:
//   • Real pet mode (desktop overlay): left-drag moves the OS window
//     (handled by PetOverlayView), middle-button rotates the model.
//   • forcePet preview (browser landing): there is no draggable window,
//     so left-drag MUST rotate the model — otherwise visitors cannot
//     interact with the character at all.
watch(
  () => isPetMode.value,
  (pet) => {
    if (sceneCtx) {
      sceneCtx.setPedestalVisible(!pet);
      if (pet) {
        const isLandingPreview = props.forcePet;
        sceneCtx.controls.mouseButtons = isLandingPreview
          ? {
              // Browser landing preview — full interaction.
              LEFT: THREE.MOUSE.ROTATE,
              MIDDLE: THREE.MOUSE.DOLLY,
              RIGHT: THREE.MOUSE.PAN,
            }
          : {
              // Desktop pet overlay — left/right reserved for OS window drag.
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
          const idleEntry = getIdleAnimationForGender(
            characterStore.currentGender(),
            Math.random,
            isPetMode.value,
          );
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

/* ── Top bubble strip: single Settings button ── */
.corner-cluster {
  position: absolute;
  top: 18px;
  right: 16px;
  z-index: 40;
  display: flex;
  align-items: center;
}

/* Settings host — own positioned wrapper so the dropdown anchors to the
   trigger button, not to the whole flex cluster. */
.settings-host {
  position: relative;
  display: flex;
  justify-self: start;
}

/* Quest portal inside settings dropdown — no absolute positioning needed. */
.settings-quest-portal {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* View mode selector row inside settings dropdown */
.settings-mode-row {
  display: flex;
  gap: 4px;
}
.settings-mode-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 6px 10px;
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  border-radius: var(--ts-radius-pill, 999px);
  background: transparent;
  color: var(--ts-text-dim);
  font-size: 0.72rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s ease, color 0.2s ease;
  white-space: nowrap;
}
.settings-mode-btn:hover {
  background: var(--ts-bg-hover);
  color: var(--ts-text-bright, var(--ts-text-primary));
}
.settings-mode-btn.active {
  background: var(--ts-accent, #7c6fff);
  color: var(--ts-text-on-accent, #fff);
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
  top: 44px;
  right: 0;
  width: 260px;
  max-width: min(260px, 90vw);
  max-height: min(500px, 70vh);
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: var(--ts-accent, #7c6fff) transparent;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  z-index: 50;
}
.settings-dropdown::-webkit-scrollbar { width: 5px; }
.settings-dropdown::-webkit-scrollbar-track { background: transparent; }
.settings-dropdown::-webkit-scrollbar-thumb {
  background: var(--ts-accent, #7c6fff);
  border-radius: 4px;
}
.settings-state-badge.talking { background: rgba(34, 197, 94, 0.15); color: var(--ts-success); }
.settings-state-badge.talking .ai-state-dot { background: #22c55e; }
.settings-state-badge.happy { background: rgba(6, 182, 212, 0.15); color: var(--ts-info); }
.settings-state-badge.happy .ai-state-dot { background: #06b6d4; }
.settings-state-badge.sad { background: rgba(168, 85, 247, 0.15); color: var(--ts-accent-violet); }
.settings-state-badge.sad .ai-state-dot { background: #a855f7; }
.settings-state-badge.angry { background: rgba(239, 68, 68, 0.15); color: var(--ts-error); }
.settings-state-badge.angry .ai-state-dot { background: #ef4444; }
.settings-state-badge.relaxed { background: rgba(20, 184, 166, 0.15); color: var(--ts-success-dim); }
.settings-state-badge.relaxed .ai-state-dot { background: #14b8a6; }
.settings-state-badge.surprised { background: rgba(245, 158, 11, 0.15); color: var(--ts-warning); }
.settings-state-badge.surprised .ai-state-dot { background: #f59e0b; }

@keyframes pulse-dot {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.4; transform: scale(0.85); }
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
  background: var(--ts-bg-hover);
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
  background: var(--ts-bg-hover);
  transform: translateY(-1px);
}
.mood-chip.active {
  background: var(--ts-accent, #7c6fff);
  border-color: var(--ts-accent, #7c6fff);
  color: var(--ts-text-on-accent, #fff);
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
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  font-size: 0.82rem;
  cursor: pointer;
  outline: none;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='%239ca3af'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
  transition: border-color var(--ts-transition-fast), background var(--ts-transition-fast);
}
.model-selector:hover {
  border-color: var(--ts-accent);
  background: var(--ts-bg-hover);
}
.model-selector:focus-visible {
  border-color: var(--ts-accent);
  box-shadow: 0 0 0 2px rgba(108, 99, 255, 0.3);
}
.model-selector option {
  background: var(--ts-bg-surface);
  color: var(--ts-text-primary);
}

.character-profile-editor {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  background: var(--ts-bg-surface);
}
.profile-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}
.profile-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}
.profile-field-full {
  width: 100%;
}
.profile-field span {
  font-size: 0.66rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  color: var(--ts-text-muted);
  text-transform: uppercase;
}
.profile-field input,
.profile-field select {
  width: 100%;
  min-width: 0;
  padding: 7px 9px;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  font-size: 0.78rem;
  outline: none;
}
.profile-field input:focus,
.profile-field select:focus {
  border-color: var(--ts-accent);
  box-shadow: 0 0 0 2px var(--ts-accent-glow);
}
.profile-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.profile-save-btn,
.profile-reset-btn {
  flex: 1 1 90px;
  padding: 7px 10px;
  border-radius: var(--ts-radius-sm);
  font-size: 0.76rem;
  font-weight: 700;
  cursor: pointer;
}
.profile-save-btn {
  border: 1px solid var(--ts-accent);
  background: var(--ts-accent);
  color: var(--ts-text-on-accent);
}
.profile-reset-btn {
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-input);
  color: var(--ts-text-secondary);
}
.profile-save-btn:disabled,
.profile-reset-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.profile-status,
.profile-error {
  margin: 0;
  font-size: 0.72rem;
}
.profile-status { color: var(--ts-success); }
.profile-error { color: var(--ts-error); }

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
  background: var(--ts-bg-hover);
  transform: translateY(-1px);
}
.background-chip.active {
  background: var(--ts-accent, #7c6fff);
  border-color: var(--ts-accent, #7c6fff);
  color: var(--ts-text-on-accent, #fff);
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
  .corner-cluster {
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
  border: 3px solid var(--ts-border, rgba(255, 255, 255, 0.15));
  border-top-color: var(--ts-accent, rgba(108, 99, 255, 0.9));
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
  border: 1px solid var(--ts-accent, rgba(108, 99, 255, 0.5));
  border-radius: 6px;
  background: var(--ts-accent, rgba(108, 99, 255, 0.6));
  color: var(--ts-text-on-accent, #fff);
  font-size: 0.8rem;
  cursor: pointer;
  transition: background 0.2s;
}
.load-error-retry:hover {
  opacity: 0.85;
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
  color: var(--ts-text-secondary);
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
  background: var(--ts-border, rgba(255, 255, 255, 0.22));
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
  background: #fff;
  border-radius: 50%;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
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
  opacity: 1;
  filter: contrast(1.2);
}

.bgm-volume-slider {
  flex: 1;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: var(--ts-border, rgba(255, 255, 255, 0.25));
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
  background: var(--ts-bg-hover, rgba(255, 255, 255, 0.05));
}
.bgm-custom-name {
  font-size: 0.68rem;
  color: var(--ts-text-secondary, var(--ts-viewport-text-med));
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
  background: var(--ts-bg-backdrop, rgba(0, 0, 0, 0.5));
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(4px);
}
.url-dialog {
  background: var(--ts-bg-overlay);
  border: 1px solid var(--ts-border);
  border-radius: 12px;
  padding: 20px;
  min-width: 320px;
  max-width: 90%;
  box-shadow: var(--ts-shadow-lg);
}
.url-dialog-label {
  display: block;
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--ts-text-primary);
  margin-bottom: 10px;
}
.url-dialog-input {
  width: 100%;
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  font-size: 0.8rem;
  outline: none;
  box-sizing: border-box;
}
.url-dialog-input:focus {
  border-color: var(--ts-accent);
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
  background: var(--ts-bg-hover);
  color: var(--ts-text-muted);
}
.url-dialog-btn.cancel:hover {
  background: var(--ts-bg-input);
}
.url-dialog-btn.confirm {
  background: var(--ts-accent);
  color: var(--ts-text-on-accent);
}
.url-dialog-btn.confirm:hover {
  opacity: 0.85;
}
.url-dialog-btn.confirm:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

</style>
