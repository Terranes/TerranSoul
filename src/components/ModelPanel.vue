<template>
  <div class="model-panel-overlay" @click.stop.self="$emit('close')">
    <div class="model-panel" @click.stop>
      <div class="panel-header">
        <h3>3D Models</h3>
        <button class="close-btn" @click="$emit('close')" aria-label="Close">&times;</button>
      </div>

      <div class="panel-body">
        <div class="model-select-section">
          <label class="select-label" for="model-select">Default Model</label>
          <select
            id="model-select"
            class="model-select"
            :value="characterStore.selectedModelId"
            :disabled="isLoading"
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
        </div>

        <div class="import-section">
          <button class="import-btn" @click="handleImport" :disabled="isLoading">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
            </svg>
            {{ isLoading ? 'Loading…' : 'Import VRM Model' }}
          </button>
          <p class="import-hint">Supports .vrm files (VRM 0.0 and 1.0)</p>
        </div>

        <div v-if="characterStore.loadError" class="error-banner">
          {{ characterStore.loadError }}
        </div>

        <div class="models-list">
          <div
            v-for="model in characterStore.defaultModels"
            :key="model.id"
            class="model-card"
            :class="{ active: characterStore.selectedModelId === model.id && !customVrmActive }"
            @click="handleSelectModel(model.id)"
          >
            <img v-if="model.thumbnail" :src="model.thumbnail" :alt="model.name" class="model-thumb" />
            <div v-else class="model-icon">👤</div>
            <div class="model-info">
              <span class="model-name">{{ model.name }}</span>
              <span class="model-author">{{ model.path }}</span>
            </div>
          </div>

          <div
            v-if="customVrmActive"
            class="model-card active"
          >
            <div class="model-icon">📁</div>
            <div class="model-info">
              <span class="model-name">{{ characterStore.vrmMetadata?.title ?? 'Custom VRM' }}</span>
              <span class="model-author">{{ characterStore.vrmMetadata?.author ?? 'Unknown author' }}</span>
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
import { ref, computed } from 'vue';
import { useCharacterStore } from '../stores/character';
import { DEFAULT_MODELS } from '../config/default-models';

defineEmits<{ close: [] }>();

const characterStore = useCharacterStore();
const isLoading = ref(false);

const customVrmActive = computed(() => {
  if (!characterStore.vrmPath) return false;
  return !DEFAULT_MODELS.some(m => m.path === characterStore.vrmPath);
});

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

async function handleImport() {
  isLoading.value = true;
  characterStore.setLoadError(undefined);
  try {
    // TODO: Replace with @tauri-apps/plugin-dialog for native file picker in production.
    // Currently uses window.prompt() as a fallback for browser preview mode.
    const path = window.prompt('Enter the path to a .vrm file:');
    if (path) {
      await characterStore.loadVrm(path);
    }
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
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: flex-start;
  justify-content: flex-end;
  padding: 8px;
}

.model-panel {
  width: 300px;
  max-height: calc(100% - 16px);
  background: rgba(20, 20, 35, 0.95);
  border: 1px solid rgba(255, 255, 255, 0.12);
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
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.panel-header h3 {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: #e8e8f0;
}

.close-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 50%;
  color: rgba(255, 255, 255, 0.7);
  font-size: 1.1rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s;
}

.close-btn:hover {
  background: rgba(255, 80, 80, 0.3);
  color: #ff6b6b;
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
  border: 1px dashed rgba(108, 99, 255, 0.5);
  border-radius: 8px;
  background: rgba(108, 99, 255, 0.1);
  color: #a09aff;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
}

.import-btn:hover:not(:disabled) {
  background: rgba(108, 99, 255, 0.2);
  border-color: #6c63ff;
}

.import-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.import-hint {
  margin: 0;
  font-size: 0.7rem;
  color: rgba(255, 255, 255, 0.35);
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
  color: rgba(255, 255, 255, 0.6);
}

.model-select {
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  background: rgba(255, 255, 255, 0.06);
  color: #e8e8f0;
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
  border-color: rgba(108, 99, 255, 0.6);
}

.model-select:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.model-select option {
  background: #1a1a2e;
  color: #e8e8f0;
}

.error-banner {
  padding: 8px 12px;
  border-radius: 6px;
  background: rgba(255, 60, 60, 0.15);
  border: 1px solid rgba(255, 60, 60, 0.3);
  color: #ff8888;
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
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
}

.model-card:hover {
  background: rgba(255, 255, 255, 0.08);
}

.model-card.active {
  border-color: rgba(108, 99, 255, 0.5);
  background: rgba(108, 99, 255, 0.1);
}

.model-icon {
  font-size: 1.5rem;
  flex-shrink: 0;
}

.model-thumb {
  width: 56px;
  height: 56px;
  border-radius: 6px;
  object-fit: contain;
  flex-shrink: 0;
  background: rgba(255, 255, 255, 0.04);
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
  color: #e8e8f0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.model-author {
  font-size: 0.7rem;
  color: rgba(255, 255, 255, 0.4);
}

.panel-footer {
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  padding-top: 10px;
}

.footer-note {
  margin: 0;
  font-size: 0.7rem;
  color: rgba(255, 255, 255, 0.3);
  line-height: 1.4;
}

.footer-note code {
  background: rgba(255, 255, 255, 0.08);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 0.68rem;
}
</style>
