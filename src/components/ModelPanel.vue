<template>
  <div
    class="model-panel-overlay"
    @click.stop.self="$emit('close')"
  >
    <div
      class="model-panel"
      @click.stop
    >
      <div class="panel-header">
        <h3>3D Models</h3>
        <button
          class="close-btn"
          aria-label="Close"
          @click="$emit('close')"
        >
          &times;
        </button>
      </div>

      <div class="panel-body">
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
                {{ model.name }}
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
                {{ model.name }}
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
              >
                <option value="female">
                  Female
                </option>
                <option value="male">
                  Male
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
                  v-for="p in personaOptions"
                  :key="p"
                  :value="p"
                >
                  {{ p }}
                </option>
              </select>
            </label>
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
              :alt="model.name"
            />
            <div class="model-info">
              <span class="model-name">{{ model.name }}</span>
              <span class="model-author">Bundled</span>
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
              :alt="model.name"
            />
            <div class="model-info">
              <span class="model-name">{{ model.name }}</span>
              <span class="model-author">{{ model.gender === 'male' ? '🧑' : '👩' }} {{ model.gender }} · {{ model.original_filename }}</span>
              <span
                v-if="model.persona"
                class="model-persona"
              >🎭 {{ model.persona }}</span>
            </div>
            <div class="model-actions">
              <button
                class="edit-btn"
                :disabled="isLoading"
                :aria-label="`Edit ${model.name}`"
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
                >
                  <option value="female">
                    Female
                  </option>
                  <option value="male">
                    Male
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
                    v-for="p in personaOptions"
                    :key="p"
                    :value="p"
                  >
                    {{ p }}
                  </option>
                </select>
              </label>
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
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useCharacterStore, type UserModel } from '../stores/character';
import { usePersonaStore } from '../stores/persona';
import type { ModelGender } from '../config/default-models';
import VrmThumbnail from './VrmThumbnail.vue';
import { preGenerateUserThumbnail } from '../composables/useVrmThumbnail';

defineEmits<{ close: [] }>();

const characterStore = useCharacterStore();
const personaStore = usePersonaStore();
const isLoading = ref(false);

// ── Import form state ─────────────────────────────────────────────────────
const showImportForm = ref(false);
const importName = ref('');
const importGender = ref<ModelGender>('female');
const importPersona = ref('');

// ── Edit dialog state ─────────────────────────────────────────────────────
const editingModel = ref<UserModel | null>(null);
const editName = ref('');
const editGender = ref<ModelGender>('female');
const editPersona = ref('');

/** Persona options: the current active persona name + "Custom" for typed input. */
const personaOptions = computed<string[]>(() => {
  const names = new Set<string>();
  if (personaStore.traits.name) {
    names.add(personaStore.traits.name);
  }
  // Add any personas already assigned to user models
  for (const m of characterStore.userModels) {
    if (m.persona) names.add(m.persona);
  }
  return [...names].sort();
});

function openEditDialog(model: UserModel) {
  editingModel.value = model;
  editName.value = model.name;
  editGender.value = model.gender;
  editPersona.value = model.persona || '';
}

async function handleSaveEdit() {
  if (!editingModel.value) return;
  isLoading.value = true;
  characterStore.setLoadError(undefined);
  try {
    await characterStore.updateUserModel(editingModel.value.id, {
      name: editName.value || undefined,
      gender: editGender.value,
      persona: editPersona.value,
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
    // Pre-generate thumbnail in background so it's cached for next open
    preGenerateUserThumbnail(entry.id).catch(() => { /* non-critical */ });
    await characterStore.selectModel(entry.id);
    // Reset form
    showImportForm.value = false;
    importName.value = '';
    importGender.value = 'female';
    importPersona.value = '';
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
.model-panel-overlay {
  position: absolute;
  inset: 0;
  z-index: 20;
  background: var(--ts-bg-backdrop);
  display: flex;
  align-items: flex-start;
  justify-content: flex-end;
  padding: 8px;
}

.model-panel {
  width: 300px;
  max-height: calc(100% - 16px);
  background: var(--ts-bg-panel);
  border: 1px solid var(--ts-border);
  border-radius: 12px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  backdrop-filter: blur(12px);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--ts-border);
}

.panel-header h3 {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--ts-text-primary);
}

.close-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: var(--ts-bg-hover);
  border-radius: 50%;
  color: var(--ts-text-secondary);
  font-size: 1.1rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s;
}

.close-btn:hover {
  background: var(--ts-error-bg);
  color: var(--ts-error);
}

.panel-body {
  padding: 14px 16px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 14px;
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
  width: min(300px, calc(100vw - 32px));
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
