<template>
  <div class="bs-card">
    <h2>🖥 Local LLM Setup — LM Studio</h2>
    <p class="bs-desc">
      Configure your LM Studio local server connection.
    </p>
    <div class="bs-form">
      <label for="lms-base-url">Base URL:</label>
      <input
        id="lms-base-url"
        v-model="baseUrl"
        type="url"
        placeholder="http://127.0.0.1:1234"
        class="bs-input"
      >
      <label for="lms-api-key">API token (optional):</label>
      <input
        id="lms-api-key"
        v-model="apiKey"
        type="password"
        placeholder="Optional"
        class="bs-input"
      >
    </div>
    <div :class="['bs-status-indicator', brain.lmStudioStatus?.running ? 'ok' : 'error']">
      {{ brain.lmStudioStatus?.running
        ? `✅ LM Studio is running (${brain.lmStudioStatus.model_count} models)`
        : '❌ LM Studio is not running — start its local server' }}
    </div>
    <button
      class="btn-secondary btn-sm"
      @click="refreshCheck"
    >
      🔄 Check connection
    </button>
    <div class="bs-form">
      <label for="lms-model">Chat model:</label>
      <input
        id="lms-model"
        v-model="chatModel"
        type="text"
        placeholder="gemma-4-12b-it"
        class="bs-input"
      >
      <label for="lms-embed-model">Embedding model (optional):</label>
      <input
        id="lms-embed-model"
        v-model="embedModel"
        type="text"
        placeholder="qwen3-embedding-0.6b"
        class="bs-input"
      >
    </div>
    <div class="bs-nav">
      <button
        class="btn-secondary"
        @click="emit('back')"
      >
        ← Back
      </button>
      <button
        class="btn-primary"
        :disabled="!brain.lmStudioStatus?.running || !chatModel"
        @click="finish"
      >
        Activate →
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useBrainStore } from '../stores/brain';

const brain = useBrainStore();
const emit = defineEmits<{ (e: 'back'): void; (e: 'done'): void }>();

const baseUrl = ref('http://127.0.0.1:1234');
const apiKey = ref('');
const chatModel = ref('');
const embedModel = ref('');

async function refreshCheck(): Promise<void> {
  await brain.checkLmStudioStatus(baseUrl.value, apiKey.value || null);
}

async function finish(): Promise<void> {
  const mode = {
    mode: 'local_lm_studio' as const,
    model: chatModel.value,
    base_url: baseUrl.value,
    api_key: apiKey.value || null,
    embedding_model: embedModel.value || null,
  };
  try {
    await brain.setBrainMode(mode);
  } catch {
    brain.brainMode = mode;
  }
  emit('done');
}
</script>

<style src="./BrainSetupView.css" scoped />
