<template>
  <PanelShell
    variant="overlay-absolute"
    class="model-panel-shell"
    card-class="model-panel-card"
    title="3D Models"
    test-id="model-panel"
    :on-close="handleClose"
    @close="handleClose"
  >
    <section
      v-if="activeModelProfile"
      class="active-persona-summary"
      data-testid="mp-active-persona"
    >
      <div class="active-persona-summary__header">
        <span class="active-persona-summary__label">Active persona</span>
        <button
          type="button"
          class="active-persona-summary__edit"
          :disabled="isLoading || !activeModel"
          @click="activeModel && openEditDialog(activeModel)"
        >
          Edit
        </button>
      </div>
      <div class="active-persona-summary__name">
        {{ activeModelProfile.name }}
        <span
          v-if="activeModelProfile.persona"
          class="active-persona-summary__role"
        >· {{ activeModelProfile.persona }}</span>
      </div>
      <div
        v-if="activeModel"
        class="active-persona-summary__meta"
      >
        {{ modelProfileSummary(activeModel) }}
      </div>
      <div
        v-if="activeModel"
        class="active-persona-summary__voice"
      >
        {{ modelVoiceSummary(activeModel) }}
      </div>
    </section>

    <div class="model-select-section">
      <label
        class="select-label"
        for="model-select"
      >Active Model</label>
      <select
        id="model-select"
        class="model-select"
        :value="characterStore.selectedModelId"
        :disabled="isLoading"
        @change="handleModelChange"
      >
        <optgroup label="Bundled">
          <option
            v-for="model in characterStore.defaultModels"
            :key="model.id"
            :value="model.id"
          >
            {{ modelDisplayName(model) }}
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
            {{ modelDisplayName(model) }}
          </option>
        </optgroup>
      </select>
    </div>

    <div class="import-section">
      <button
        v-if="!showImportForm"
        class="import-btn"
        :disabled="isLoading"
        @click="showImportForm = true"
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z" />
        </svg>
        {{ isLoading ? 'Loading…' : 'Import VRM Model' }}
      </button>

      <!-- Import form with gender + persona selection -->
      <div
        v-if="showImportForm"
        class="import-form"
      >
        <h4 class="import-form__title">
          Import New Model
        </h4>
        <label class="import-form__label">
          Name
          <input
            v-model="importName"
            type="text"
            class="import-form__input"
            placeholder="Model name (optional)"
          >
        </label>
        <label class="import-form__label">
          Gender
          <select
            v-model="importGender"
            class="import-form__select"
            @change="onImportGenderChange"
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
        <label class="import-form__label">
          Persona
          <select
            v-model="importPersona"
            class="import-form__select"
          >
            <option value="">
              None (use active persona)
            </option>
            <option
              v-for="personaName in personaOptions"
              :key="personaName"
              :value="personaName"
            >
              {{ personaName }}
            </option>
          </select>
        </label>
        <label class="import-form__label">
          Voice
          <input
            v-model="importVoiceName"
            type="text"
            class="import-form__input"
            list="mp-import-voice-suggestions"
            placeholder="Pick a voice or type a custom id"
          >
          <datalist id="mp-import-voice-suggestions">
            <option
              v-for="voice in voiceCatalogue"
              :key="voice.id"
              :value="voice.id"
            >
              {{ voice.label }}
            </option>
          </datalist>
        </label>
        <div class="voice-grid">
          <label class="import-form__label">
            Age
            <select
              v-model="importAge"
              class="import-form__select"
            >
              <option
                v-for="option in ageOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>
          <label class="import-form__label">
            Pitch
            <select
              v-model="importPitch"
              class="import-form__select"
            >
              <option
                v-for="option in pitchOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>
        </div>
        <div class="voice-grid">
          <label class="import-form__label">
            Style
            <select
              v-model="importStyle"
              class="import-form__select"
            >
              <option
                v-for="option in styleOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>
          <label class="import-form__label">
            English accent
            <select
              v-model="importEnglishAccent"
              class="import-form__select"
            >
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
        <div class="import-form__actions">
          <button
            class="import-form__btn import-form__btn--primary"
            :disabled="isLoading"
            @click="handleImport"
          >
            {{ isLoading ? 'Importing…' : 'Choose File & Import' }}
          </button>
          <button
            class="import-form__btn import-form__btn--cancel"
            :disabled="isLoading"
            @click="showImportForm = false"
          >
            Cancel
          </button>
        </div>
      </div>

      <p class="import-hint">
        Imported models are copied into your user data folder and persist
        across app updates.
      </p>
    </div>

    <div
      v-if="characterStore.loadError"
      class="error-banner"
    >
      {{ characterStore.loadError }}
    </div>

    <div class="models-list">
      <div
        v-for="model in characterStore.defaultModels"
        :key="model.id"
        class="model-card"
        :class="{ active: characterStore.selectedModelId === model.id }"
        @click="handleSelectModel(model.id)"
      >
        <VrmThumbnail
          :cache-key="model.id"
          :model-path="model.path"
          :alt="modelDisplayName(model)"
        />
        <div class="model-info">
          <span class="model-name">{{ modelDisplayName(model) }}</span>
          <span class="model-author">Bundled · {{ modelProfileSummary(model) }}</span>
          <span class="model-persona">{{ modelVoiceSummary(model) }}</span>
        </div>
        <div class="model-actions">
          <button
            class="edit-btn"
            :disabled="isLoading"
            :aria-label="`Edit ${modelDisplayName(model)}`"
            title="Edit model"
            @click.stop="openEditDialog(model)"
          >
            ✎
          </button>
        </div>
      </div>

      <div
        v-for="model in characterStore.userModels"
        :key="model.id"
        class="model-card"
        :class="{ active: characterStore.selectedModelId === model.id }"
        @click="handleSelectModel(model.id)"
      >
        <VrmThumbnail
          :cache-key="model.id"
          :user-model-id="model.id"
          :alt="modelDisplayName(model)"
        />
        <div class="model-info">
          <span class="model-name">{{ modelDisplayName(model) }}</span>
          <span class="model-author">{{ modelProfileSummary(model) }} · {{ model.original_filename }}</span>
          <span
            v-if="characterStore.resolveModelProfile(model).persona"
            class="model-persona"
          >{{ characterStore.resolveModelProfile(model).persona }}</span>
          <span class="model-persona">{{ modelVoiceSummary(model) }}</span>
        </div>
        <div class="model-actions">
          <button
            class="edit-btn"
            :disabled="isLoading"
            :aria-label="`Edit ${modelDisplayName(model)}`"
            title="Edit model"
            @click.stop="openEditDialog(model)"
          >
            ✎
          </button>
          <button
            class="delete-btn"
            :disabled="isLoading"
            :aria-label="`Delete ${model.name}`"
            @click.stop="handleDelete(model.id)"
          >
            &times;
          </button>
        </div>
      </div>

      <!-- Edit dialog for user models -->
      <div
        v-if="editingModel"
        class="edit-dialog-backdrop"
        @click.self="editingModel = null"
      >
        <div class="edit-dialog">
          <h4 class="edit-dialog__title">
            Edit Model
          </h4>
          <label class="import-form__label">
            Name
            <input
              v-model="editName"
              type="text"
              class="import-form__input"
            >
          </label>
          <label class="import-form__label">
            Gender
            <select
              v-model="editGender"
              class="import-form__select"
              @change="onEditGenderChange"
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
          <label class="import-form__label">
            Persona
            <select
              v-model="editPersona"
              class="import-form__select"
            >
              <option value="">
                None (use active persona)
              </option>
              <option
                v-for="personaName in personaOptions"
                :key="personaName"
                :value="personaName"
              >
                {{ personaName }}
              </option>
            </select>
          </label>
          <label class="import-form__label">
            Voice
            <input
              v-model="editVoiceName"
              type="text"
              class="import-form__input"
              list="mp-edit-voice-suggestions"
              placeholder="Pick a voice or type a custom id"
            >
            <datalist id="mp-edit-voice-suggestions">
              <option
                v-for="voice in voiceCatalogue"
                :key="voice.id"
                :value="voice.id"
              >
                {{ voice.label }}
              </option>
            </datalist>
          </label>
          <div class="voice-grid">
            <label class="import-form__label">
              Age
              <select
                v-model="editAge"
                class="import-form__select"
              >
                <option
                  v-for="option in ageOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </option>
              </select>
            </label>
            <label class="import-form__label">
              Pitch
              <select
                v-model="editPitch"
                class="import-form__select"
              >
                <option
                  v-for="option in pitchOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </option>
              </select>
            </label>
          </div>
          <div class="voice-grid">
            <label class="import-form__label">
              Style
              <select
                v-model="editStyle"
                class="import-form__select"
              >
                <option
                  v-for="option in styleOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </option>
              </select>
            </label>
            <label class="import-form__label">
              English accent
              <select
                v-model="editEnglishAccent"
                class="import-form__select"
              >
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
          <div class="import-form__actions">
            <button
              class="import-form__btn import-form__btn--primary"
              :disabled="isLoading"
              @click="handleSaveEdit"
            >
              {{ isLoading ? 'Saving…' : 'Save' }}
            </button>
            <button
              class="import-form__btn import-form__btn--cancel"
              @click="editingModel = null"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    </div>

    <div class="panel-footer">
      <p class="footer-note">
        See <code>instructions/</code> folder for guides on importing models
        and extending TerranSoul with custom characters.
      </p>
    </div>
  </PanelShell>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useCharacterStore, type UserModel } from '../stores/character';
import { usePersonaStore } from '../stores/persona';
import { GENDER_VOICES, type DefaultModel, type ModelGender } from '../config/default-models';
import { VOICE_CATALOGUE } from '../config/voice-catalogue';
import {
  PERSONA_ENGLISH_ACCENT_OPTIONS,
  PERSONA_VOICE_AGE_OPTIONS,
  PERSONA_VOICE_GENDER_OPTIONS,
  PERSONA_VOICE_PITCH_OPTIONS,
  PERSONA_VOICE_STYLE_OPTIONS,
  defaultPersonaVoiceProfile,
  type PersonaEnglishAccent,
  type PersonaVoiceAge,
  type PersonaVoicePitch,
  type PersonaVoiceStyle,
} from '../stores/persona-types';
import VrmThumbnail from './VrmThumbnail.vue';
import PanelShell from './ui/PanelShell.vue';
import { preGenerateUserThumbnail } from '../composables/useVrmThumbnail';

const emit = defineEmits<{ close: [] }>();

function handleClose() {
  emit('close');
}

const characterStore = useCharacterStore();
const personaStore = usePersonaStore();
const isLoading = ref(false);

// ── Import form state ─────────────────────────────────────────────────────
const showImportForm = ref(false);
const importName = ref('');
const importGender = ref<ModelGender>('female');
const importPersona = ref('');
const importVoiceName = ref(GENDER_VOICES.female.edgeVoice);
const importAge = ref<PersonaVoiceAge>(defaultPersonaVoiceProfile().age);
const importPitch = ref<PersonaVoicePitch>(defaultPersonaVoiceProfile().pitch);
const importStyle = ref<PersonaVoiceStyle>(defaultPersonaVoiceProfile().style);
const importEnglishAccent = ref<PersonaEnglishAccent>(defaultPersonaVoiceProfile().englishAccent);

// ── Edit dialog state ─────────────────────────────────────────────────────
type EditableModel = DefaultModel | UserModel;

const editingModel = ref<EditableModel | null>(null);
const editName = ref('');
const editGender = ref<ModelGender>('female');
const editPersona = ref('');
const editVoiceName = ref('');
const editAge = ref<PersonaVoiceAge>('adult');
const editPitch = ref<PersonaVoicePitch>('medium');
const editStyle = ref<PersonaVoiceStyle>('natural');
const editEnglishAccent = ref<PersonaEnglishAccent>('american');

const genderOptions = PERSONA_VOICE_GENDER_OPTIONS;
const ageOptions = PERSONA_VOICE_AGE_OPTIONS;
const pitchOptions = PERSONA_VOICE_PITCH_OPTIONS;
const styleOptions = PERSONA_VOICE_STYLE_OPTIONS;
const englishAccentOptions = PERSONA_ENGLISH_ACCENT_OPTIONS;
const voiceCatalogue = VOICE_CATALOGUE;

/** Persona options: the current active persona name + "Custom" for typed input. */
const personaOptions = computed<string[]>(() => {
  const names = new Set<string>();
  if (personaStore.traits.name) {
    names.add(personaStore.traits.name);
  }
  // Add any personas already assigned to user models
  for (const model of characterStore.userModels) {
    const profile = characterStore.resolveModelProfile(model);
    if (profile.persona) names.add(profile.persona);
  }
  return [...names].sort();
});

/** Currently selected model (default or imported). Used to render the
 *  active-persona summary block at the top of the panel without making
 *  the user dig into per-model Edit dialogs. */
const activeModel = computed<EditableModel | null>(() => {
  const id = characterStore.selectedModelId;
  return (
    characterStore.defaultModels.find((m) => m.id === id) ??
    characterStore.userModels.find((m) => m.id === id) ??
    null
  );
});

/** Resolved profile (name/persona/voice) of the currently selected model. */
const activeModelProfile = computed(() => {
  const m = activeModel.value;
  return m ? characterStore.resolveModelProfile(m) : null;
});

function modelDisplayName(model: EditableModel): string {
  return characterStore.resolveModelProfile(model).name;
}

function modelProfileSummary(model: EditableModel): string {
  const profile = characterStore.resolveModelProfile(model);
  return `${labelFor(profile.gender, genderOptions)} · ${labelFor(profile.voiceProfile.age, ageOptions)} · ${labelFor(profile.voiceProfile.pitch, pitchOptions)} pitch`;
}

function modelVoiceSummary(model: EditableModel): string {
  const profile = characterStore.resolveModelProfile(model);
  return `${profile.voiceProfile.voiceName || 'default voice'} · ${labelFor(profile.voiceProfile.style, styleOptions)} · ${labelFor(profile.voiceProfile.englishAccent, englishAccentOptions)} EN`;
}

function labelFor<T extends string>(value: T, options: readonly { value: T; label: string }[]): string {
  return options.find(option => option.value === value)?.label ?? value;
}

function defaultVoiceNameForGender(gender: ModelGender): string {
  return GENDER_VOICES[gender].edgeVoice;
}

function usesGenderDefaultVoice(voiceName: string): boolean {
  const defaults = Object.values(GENDER_VOICES).map(voice => voice.edgeVoice);
  return !voiceName.trim() || defaults.includes(voiceName.trim());
}

function onImportGenderChange(): void {
  if (usesGenderDefaultVoice(importVoiceName.value)) {
    importVoiceName.value = defaultVoiceNameForGender(importGender.value);
  }
}

function onEditGenderChange(): void {
  if (usesGenderDefaultVoice(editVoiceName.value)) {
    editVoiceName.value = defaultVoiceNameForGender(editGender.value);
  }
}

function openEditDialog(model: EditableModel) {
  const profile = characterStore.resolveModelProfile(model);
  editingModel.value = model;
  editName.value = profile.name;
  editGender.value = profile.gender;
  editPersona.value = profile.persona || '';
  editVoiceName.value = profile.voiceProfile.voiceName;
  editAge.value = profile.voiceProfile.age;
  editPitch.value = profile.voiceProfile.pitch;
  editStyle.value = profile.voiceProfile.style;
  editEnglishAccent.value = profile.voiceProfile.englishAccent;
}

async function handleSaveEdit() {
  if (!editingModel.value) return;
  isLoading.value = true;
  characterStore.setLoadError(undefined);
  try {
    await characterStore.updateModelProfile(editingModel.value.id, {
      name: editName.value || undefined,
      gender: editGender.value,
      persona: editPersona.value,
      voiceProfile: {
        gender: editGender.value,
        age: editAge.value,
        pitch: editPitch.value,
        style: editStyle.value,
        englishAccent: editEnglishAccent.value,
        voiceName: editVoiceName.value,
      },
    });
    editingModel.value = null;
  } catch (err) {
    characterStore.setLoadError(`Update failed: ${err}`);
  } finally {
    isLoading.value = false;
  }
}

async function handleModelChange(event: Event) {
  const target = event.target as HTMLSelectElement;
  isLoading.value = true;
  characterStore.setLoadError(undefined);
  try {
    await characterStore.selectModel(target.value);
  } finally {
    isLoading.value = false;
  }
}

async function handleSelectModel(modelId: string) {
  isLoading.value = true;
  characterStore.setLoadError(undefined);
  try {
    await characterStore.selectModel(modelId);
  } finally {
    isLoading.value = false;
  }
}

/**
 * Prompt the user for a VRM file path and ask the Rust backend to copy it
 * into the per-user data directory with the chosen gender/persona.
 * Once imported, immediately select it.
 */
async function handleImport() {
  const path = window.prompt('Enter the absolute path to a .vrm file:');
  if (!path) return;
  isLoading.value = true;
  characterStore.setLoadError(undefined);
  try {
    const entry = await characterStore.importUserModel(path.trim(), {
      name: importName.value || undefined,
      gender: importGender.value,
      persona: importPersona.value || undefined,
    });
    await characterStore.updateModelProfile(entry.id, {
      name: importName.value || entry.name,
      gender: importGender.value,
      persona: importPersona.value,
      voiceProfile: {
        gender: importGender.value,
        age: importAge.value,
        pitch: importPitch.value,
        style: importStyle.value,
        englishAccent: importEnglishAccent.value,
        voiceName: importVoiceName.value,
      },
    });
    // Pre-generate thumbnail in background so it's cached for next open
    preGenerateUserThumbnail(entry.id).catch(() => { /* non-critical */ });
    await characterStore.selectModel(entry.id);
    // Reset form
    showImportForm.value = false;
    importName.value = '';
    importGender.value = 'female';
    importPersona.value = '';
    importVoiceName.value = GENDER_VOICES.female.edgeVoice;
    importAge.value = defaultPersonaVoiceProfile().age;
    importPitch.value = defaultPersonaVoiceProfile().pitch;
    importStyle.value = defaultPersonaVoiceProfile().style;
    importEnglishAccent.value = defaultPersonaVoiceProfile().englishAccent;
  } catch (err) {
    characterStore.setLoadError(`Import failed: ${err}`);
  } finally {
    isLoading.value = false;
  }
}

async function handleDelete(id: string) {
  const target = characterStore.userModels.find(m => m.id === id);
  if (!target) return;
  const confirmed = window.confirm(
    `Delete "${target.name}"? The VRM file will be removed from your user data folder.`
  );
  if (!confirmed) return;
  isLoading.value = true;
  characterStore.setLoadError(undefined);
  try {
    await characterStore.deleteUserModel(id);
  } catch (err) {
    characterStore.setLoadError(`Delete failed: ${err}`);
  } finally {
    isLoading.value = false;
  }
}
</script>

<style scoped>
.model-panel-shell {
  align-items: flex-start;
  justify-content: flex-end;
  padding: 8px;
  background: var(--ts-bg-backdrop);
  z-index: 20;
}

.model-panel-shell :deep(.model-panel-card) {
  width: min(360px, calc(100vw - 16px));
  max-height: calc(100% - 16px);
}

.import-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.import-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 16px;
  border: 1px dashed var(--ts-accent-glow);
  border-radius: 8px;
  background: var(--ts-bg-input);
  color: var(--ts-accent-violet);
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
}

.import-btn:hover:not(:disabled) {
  background: var(--ts-accent-glow);
  border-color: var(--ts-accent);
}

.import-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.import-hint {
  margin: 0;
  font-size: 0.7rem;
  color: var(--ts-text-dim);
  text-align: center;
}

.model-select-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.active-persona-summary {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 12px 14px;
  border-radius: 10px;
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-overlay);
}

.active-persona-summary__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.active-persona-summary__label {
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--ts-text-dim);
}

.active-persona-summary__edit {
  appearance: none;
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-card);
  color: var(--ts-text-primary);
  font-size: 0.72rem;
  padding: 3px 10px;
  border-radius: 999px;
  cursor: pointer;
}

.active-persona-summary__edit:hover:not(:disabled) {
  border-color: var(--ts-accent);
  background: var(--ts-accent);
  color: var(--ts-text-on-accent, #fff);
}

.active-persona-summary__edit:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.active-persona-summary__name {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--ts-text-primary);
}

.active-persona-summary__role {
  font-weight: 400;
  color: var(--ts-text-secondary);
  margin-left: 4px;
}

.active-persona-summary__meta,
.active-persona-summary__voice {
  font-size: 0.78rem;
  color: var(--ts-text-secondary);
}

.select-label {
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--ts-text-secondary);
}

.model-select {
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  font-size: 0.85rem;
  cursor: pointer;
  outline: none;
  transition: border-color 0.2s;
  appearance: none;
  -webkit-appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='%23888'%3E%3Cpath d='M7 10l5 5 5-5z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
}

.model-select:focus {
  border-color: var(--ts-accent);
}

.model-select:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.model-select option {
  background: var(--ts-bg-surface);
  color: var(--ts-text-primary);
}

.error-banner {
  padding: 8px 12px;
  border-radius: 6px;
  background: var(--ts-error-bg);
  border: 1px solid rgba(255, 60, 60, 0.3);
  color: var(--ts-error);
  font-size: 0.78rem;
}

.models-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.model-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-input);
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
}

.model-card:hover {
  background: var(--ts-bg-hover);
}

.model-card.active {
  border-color: var(--ts-accent);
  background: var(--ts-accent-glow);
}

.model-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1 1 auto;
  min-width: 0;
}

.model-name {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--ts-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.model-author {
  font-size: 0.7rem;
  color: var(--ts-text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.delete-btn {
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 50%;
  background: var(--ts-bg-hover);
  color: var(--ts-text-secondary);
  font-size: 1rem;
  line-height: 1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.2s, color 0.2s;
}

.delete-btn:hover:not(:disabled) {
  background: var(--ts-error-bg);
  color: var(--ts-error);
}

.delete-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.model-actions {
  margin-left: auto;
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

.edit-btn {
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 50%;
  background: var(--ts-bg-hover);
  color: var(--ts-text-secondary);
  font-size: 0.85rem;
  line-height: 1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.2s, color 0.2s;
}

.edit-btn:hover:not(:disabled) {
  background: var(--ts-accent-glow);
  color: var(--ts-accent);
}

.edit-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.model-persona {
  font-size: 0.68rem;
  color: var(--ts-accent-violet, var(--ts-text-dim));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.import-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 12px;
  border: 1px solid var(--ts-border);
  border-radius: 8px;
  background: var(--ts-bg-input);
}

.import-form__title {
  margin: 0;
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--ts-text-primary);
}

.import-form__label {
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--ts-text-secondary);
}

.import-form__input {
  padding: 6px 10px;
  border: 1px solid var(--ts-border);
  border-radius: 6px;
  background: var(--ts-bg-surface);
  color: var(--ts-text-primary);
  font-size: 0.8rem;
  outline: none;
  transition: border-color 0.2s;
}

.import-form__input:focus {
  border-color: var(--ts-accent);
}

.import-form__select {
  padding: 6px 10px;
  border: 1px solid var(--ts-border);
  border-radius: 6px;
  background: var(--ts-bg-surface);
  color: var(--ts-text-primary);
  font-size: 0.8rem;
  cursor: pointer;
  outline: none;
  transition: border-color 0.2s;
}

.import-form__select:focus {
  border-color: var(--ts-accent);
}

.import-form__actions {
  display: flex;
  gap: 8px;
  margin-top: 4px;
}

.voice-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 8px;
}

.import-form__btn {
  flex: 1;
  padding: 7px 12px;
  border: none;
  border-radius: 6px;
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s, opacity 0.2s;
}

.import-form__btn--primary {
  background: var(--ts-accent);
  color: #fff;
}

.import-form__btn--primary:hover:not(:disabled) {
  opacity: 0.85;
}

.import-form__btn--primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.import-form__btn--cancel {
  background: var(--ts-bg-hover);
  color: var(--ts-text-secondary);
}

.import-form__btn--cancel:hover {
  background: var(--ts-border);
}

.edit-dialog-backdrop {
  position: fixed;
  inset: 0;
  z-index: 30;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
}

.edit-dialog {
  width: min(380px, calc(100vw - 32px));
  max-height: calc(100vh - 48px);
  overflow-y: auto;
  padding: 16px;
  border-radius: 12px;
  background: var(--ts-bg-panel);
  border: 1px solid var(--ts-border);
  display: flex;
  flex-direction: column;
  gap: 10px;
  box-shadow: var(--ts-shadow-lg, 0 8px 32px rgba(0, 0, 0, 0.4));
}

.edit-dialog__title {
  margin: 0;
  font-size: 0.9rem;
  font-weight: 650;
  color: var(--ts-text-primary);
}

.panel-footer {
  border-top: 1px solid var(--ts-border-subtle);
  padding-top: 10px;
}

.footer-note {
  margin: 0;
  font-size: 0.7rem;
  color: var(--ts-text-dim);
  line-height: 1.4;
}

.footer-note code {
  background: var(--ts-bg-hover);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 0.68rem;
}
</style>
