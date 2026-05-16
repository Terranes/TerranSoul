<template>
  <FloatingMenu
    class="settings-dropdown"
    data-testid="settings-panel"
    @click.stop
  >
    <div class="settings-header">
      <span class="settings-header-title">Settings</span>
      <button
        class="settings-close-btn"
        aria-label="Close settings"
        @click="emit('close')"
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
          :class="{ active: !props.isPetMode }"
          data-testid="settings-mode-desktop"
          @click="emit('request-set-display-mode', 'desktop'); emit('close')"
        >
          🖥 3D
        </button>
        <button
          class="settings-mode-btn"
          data-testid="settings-mode-chatbox"
          @click="emit('request-set-display-mode', 'chatbox'); emit('close')"
        >
          💬 Chat
        </button>
        <button
          class="settings-mode-btn"
          data-testid="settings-mode-pet"
          @click="emit('request-toggle-pet-mode'); emit('close')"
        >
          🐾 Pet
        </button>
      </div>
    </div>

    <!-- Quest progress portal — corner-cluster mounts content here -->
    <div class="dropdown-section">
      <label class="dropdown-label">Quests</label>
      <div
        id="corner-cluster-portal"
        class="settings-quest-portal"
      />
    </div>

    <!-- Character model + profile editor -->
    <div class="dropdown-section">
      <label class="dropdown-label">Character</label>
      <select
        class="model-selector"
        data-testid="settings-model-selector"
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
            list="settings-voice-suggestions"
            placeholder="Pick a voice or type a custom id"
          >
          <datalist id="settings-voice-suggestions">
            <option
              v-for="voice in voiceCatalogue"
              :key="voice.id"
              :value="voice.id"
            >
              {{ voice.label }}
            </option>
          </datalist>
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

    <!-- Mood / pose selector — mirrors PetContextMenu's mood submenu -->
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
            :checked="props.bgmEnabled"
            @change="handleBgmToggle"
          >
          <span class="bgm-slider" />
        </label>
        <span class="bgm-status">{{ props.bgmEnabled ? 'On' : 'Off' }}</span>
      </div>
      <select
        v-show="props.bgmEnabled"
        class="model-selector"
        :value="props.bgmTrackId"
        @change="handleBgmTrackChange"
      >
        <option
          v-for="track in props.bgm.allTracks.value"
          :key="track.id"
          :value="track.id"
        >
          {{ track.name }}
        </option>
      </select>
      <div
        v-if="props.bgmEnabled"
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
        v-if="props.bgmEnabled && props.bgm.customTracks.value.length"
        class="bgm-custom-list"
      >
        <div
          v-for="track in props.bgm.customTracks.value"
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
      <div class="bgm-volume-row">
        <span class="bgm-vol-icon">🔈</span>
        <input
          type="range"
          class="bgm-volume-slider"
          min="0"
          max="100"
          :value="Math.round(props.bgmVolume * 100)"
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
        data-testid="settings-toggle-system-info"
        @click="emit('toggle-system-info')"
      >
        📊 System Information
      </button>
      <button
        class="dropdown-btn"
        data-testid="settings-toggle-audio-controls"
        @click="emit('toggle-audio-controls')"
      >
        🎛️ Audio Controls
      </button>
    </div>

    <!-- URL dialog rendered at <body> level so the FloatingMenu's
         overflow-y: auto / max-height doesn't clip it. -->
    <Teleport to="body">
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
    </Teleport>
  </FloatingMenu>
</template>

<script setup lang="ts">
/**
 * SettingsPanel — the gear-dropdown content. Extracted from
 * `CharacterViewport.vue` (UI-SETTINGS-1) so the 2000+ line viewport
 * component owns only the 3D scene + the gear trigger; this component
 * owns every settings section (view mode, character profile editor,
 * mood grid, background picker, BGM, karaoke, theme, system/audio
 * panel toggles).
 *
 * BGM state (enabled / volume / track id) stays in the parent because
 * other systems consume it (AudioControlsPanel, ChatView's BGM-quest
 * `enableBgm()`, the audio-store mute watcher). We accept it via
 * `v-model:bgm-*` props and operate on the parent's `BgmPlayerHandle`
 * instance through the `bgm` prop, so there is exactly one BGM
 * instance per viewport.
 */
import { ref, computed, watch, onBeforeUnmount } from 'vue';
import { useCharacterStore } from '../stores/character';
import { useBackgroundStore } from '../stores/background';
import { useSettingsStore } from '../stores/settings';
import { GENDER_VOICES, type ModelGender } from '../config/default-models';
import { VOICE_CATALOGUE } from '../config/voice-catalogue';
import {
  PERSONA_ENGLISH_ACCENT_OPTIONS,
  PERSONA_VOICE_AGE_OPTIONS,
  PERSONA_VOICE_GENDER_OPTIONS,
  PERSONA_VOICE_PITCH_OPTIONS,
  PERSONA_VOICE_STYLE_OPTIONS,
  type PersonaEnglishAccent,
  type PersonaVoiceAge,
  type PersonaVoicePitch,
  type PersonaVoiceStyle,
} from '../stores/persona-types';
import { MOOD_ENTRIES, isMoodActive, applyMood, type MoodEntry } from '../config/moods';
import { BGM_TRACKS, type BgmTrack, type BgmPlayerHandle } from '../composables/useBgmPlayer';
import FloatingMenu from './ui/FloatingMenu.vue';
import ThemePicker from './ThemePicker.vue';

const props = defineProps<{
  /** Whether the viewport is currently in pet mode (controls the "3D"
   *  button's active highlight). */
  isPetMode: boolean;
  /** Parent's BGM player handle. Shared so play/stop/customTracks all
   *  reflect a single audio element. */
  bgm: BgmPlayerHandle;
  /** v-model: master BGM on/off. */
  bgmEnabled: boolean;
  /** v-model: master BGM volume (0–1). */
  bgmVolume: number;
  /** v-model: currently selected track id. */
  bgmTrackId: string;
}>();

const emit = defineEmits<{
  close: [];
  'request-set-display-mode': [mode: 'desktop' | 'chatbox'];
  'request-toggle-pet-mode': [];
  'toggle-system-info': [];
  'toggle-audio-controls': [];
  'update:bgmEnabled': [v: boolean];
  'update:bgmVolume': [v: number];
  'update:bgmTrackId': [v: string];
  /** Fires whenever a modal child overlay is opened/closed (URL dialog).
   *  Parent uses it to update its global `overlay-open` outward emit. */
  'url-dialog-toggle': [open: boolean];
}>();

const characterStore = useCharacterStore();
const backgroundStore = useBackgroundStore();
const settingsStore = useSettingsStore();

// ── Character profile editor (draft state, local to this panel) ────────────
const genderOptions = PERSONA_VOICE_GENDER_OPTIONS;
const ageOptions = PERSONA_VOICE_AGE_OPTIONS;
const pitchOptions = PERSONA_VOICE_PITCH_OPTIONS;
const styleOptions = PERSONA_VOICE_STYLE_OPTIONS;
const englishAccentOptions = PERSONA_ENGLISH_ACCENT_OPTIONS;
const voiceCatalogue = VOICE_CATALOGUE;

const profileDraftName = ref('');
const profileDraftGender = ref<ModelGender>('female');
const profileDraftPersona = ref('');
const profileDraftVoiceName = ref('');
const profileDraftAge = ref<PersonaVoiceAge>('adult');
const profileDraftPitch = ref<PersonaVoicePitch>('medium');
const profileDraftStyle = ref<PersonaVoiceStyle>('natural');
const profileDraftEnglishAccent = ref<PersonaEnglishAccent>('american');
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

// ── Model picker / VRM import ──────────────────────────────────────────────
const vrmInputRef = ref<HTMLInputElement | null>(null);
const localVrmObjectUrl = ref<string | null>(null);

function handleModelChange(e: Event) {
  const select = e.target as HTMLSelectElement;
  characterStore.selectModel(select.value);
}

function openVrmPicker() {
  vrmInputRef.value?.click();
}

async function handleVrmImport(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;

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

onBeforeUnmount(() => {
  if (localVrmObjectUrl.value) {
    URL.revokeObjectURL(localVrmObjectUrl.value);
    localVrmObjectUrl.value = null;
  }
});

// ── Background picker ──────────────────────────────────────────────────────
const backgroundInputRef = ref<HTMLInputElement | null>(null);

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

// ── Mood ───────────────────────────────────────────────────────────────────
function handleMoodPick(mood: MoodEntry) {
  applyMood(mood, characterStore);
}

// ── Karaoke ────────────────────────────────────────────────────────────────
const karaokeDialogEnabled = computed(
  () => settingsStore.settings.karaoke_dialog_enabled !== false,
);

function handleKaraokeToggle(e: Event) {
  const checked = (e.target as HTMLInputElement).checked;
  void settingsStore.setKaraokeDialogEnabled(checked);
}

// ── BGM controls — mutate parent state via v-model and call parent's
//     BgmPlayerHandle for actual playback. ─────────────────────────────────
const bgmFileInputRef = ref<HTMLInputElement | null>(null);
const showUrlDialog = ref(false);
const urlInput = ref('');

watch(showUrlDialog, (open) => emit('url-dialog-toggle', open));

function handleBgmToggle(e: Event) {
  const checked = (e.target as HTMLInputElement).checked;
  emit('update:bgmEnabled', checked);
  if (checked) {
    props.bgm.setVolume(props.bgmVolume);
    props.bgm.play(props.bgmTrackId);
  } else {
    props.bgm.stop();
  }
  settingsStore.saveBgmState(checked, props.bgmVolume, props.bgmTrackId);
}

function handleBgmTrackChange(e: Event) {
  const id = (e.target as HTMLSelectElement).value;
  emit('update:bgmTrackId', id);
  if (props.bgmEnabled) {
    props.bgm.play(id);
  }
  settingsStore.saveBgmState(props.bgmEnabled, props.bgmVolume, id);
}

function handleBgmVolumeChange(e: Event) {
  const v = parseInt((e.target as HTMLInputElement).value, 10) / 100;
  emit('update:bgmVolume', v);
  props.bgm.setVolume(v);
  settingsStore.saveBgmState(props.bgmEnabled, v, props.bgmTrackId);
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
  const id = props.bgm.addCustomTrack(name, objectUrl);
  emit('update:bgmTrackId', id);
  if (props.bgmEnabled) {
    props.bgm.play(id);
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
  // Derive a name from the URL (last path segment or hostname).
  let name = 'Custom Track';
  try {
    const parsed = new URL(url);
    const seg = parsed.pathname.split('/').filter(Boolean).pop();
    if (seg) name = decodeURIComponent(seg).replace(/\.[^.]+$/, '');
  } catch { /* keep default name */ }
  const id = props.bgm.addCustomTrack(name, url);
  emit('update:bgmTrackId', id);
  if (props.bgmEnabled) {
    props.bgm.play(id);
  }
  persistCustomTracks();
  showUrlDialog.value = false;
}

function cancelUrlDialog() {
  showUrlDialog.value = false;
}

function handleRemoveTrack(trackId: string) {
  const wasPlaying = props.bgmTrackId === trackId;
  props.bgm.removeTrack(trackId);
  if (wasPlaying) {
    const fallbackId = BGM_TRACKS[0].id;
    emit('update:bgmTrackId', fallbackId);
    if (props.bgmEnabled) {
      props.bgm.play(fallbackId);
    }
  }
  persistCustomTracks();
}

function persistCustomTracks() {
  // Only persist tracks with stable (non-blob) URLs — blob URLs don't
  // survive an app restart.
  const persistable = props.bgm.customTracks.value
    .filter(t => t.src && !t.src.startsWith('blob:'))
    .map(({ id, name, src }) => ({ id, name, src }));
  settingsStore.saveSettings({ bgm_custom_tracks: persistable as BgmTrack[] });
}
</script>

<style scoped>
/* Dropdown container — class is applied to <FloatingMenu> which is this
   component's template root, so the scoped attribute reaches the rendered
   <div>. No `:deep()` needed. */
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

.settings-quest-portal {
  display: flex;
  align-items: center;
  gap: 8px;
}

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
.profile-field-full { width: 100%; }
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

.hidden-file-input { display: none; }

.bg-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
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
.mood-chip-emoji { font-size: 1.05rem; line-height: 1; }
.mood-chip-label {
  font-size: 0.62rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}

/* BGM controls */
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
.bgm-switch input { opacity: 0; width: 0; height: 0; }
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
  color: var(--ts-text-secondary);
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

.fade-enter-active, .fade-leave-active {
  transition: opacity 0.4s ease;
}
.fade-enter-from, .fade-leave-to {
  opacity: 0;
}
</style>

<!-- Unscoped block for the Teleported URL dialog (rendered at <body>).
     Scoped styles don't attach their hash to teleported content, so the
     dialog styles must live in a global block. -->
<style>
.url-dialog-backdrop {
  position: fixed;
  inset: 0;
  z-index: 2000;
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
