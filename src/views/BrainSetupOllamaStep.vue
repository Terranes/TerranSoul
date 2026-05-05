<template>
  <div class="bs-card">
    <h2>Check Ollama</h2>
    <p class="bs-desc">
      TerranSoul uses <a
        href="https://ollama.ai"
        target="_blank"
      >Ollama</a> to run models
      locally. It must be running before we can download your brain.
    </p>
    <div :class="['bs-status-indicator', brain.ollamaStatus.running ? 'ok' : 'error']">
      {{ brain.ollamaStatus.running ? '✅ Ollama is running' : '❌ Ollama is not running' }}
    </div>
    <div
      v-if="!brain.ollamaStatus.running"
      class="bs-install-hint"
    >
      <p>Install and start Ollama:</p>
      <ol>
        <li>
          Download from <a
            href="https://ollama.ai"
            target="_blank"
          >ollama.ai</a>
        </li>
        <li>Run <code>ollama serve</code> in a terminal</li>
        <li>Click Retry below</li>
      </ol>
    </div>
    <div class="bs-nav">
      <button
        class="btn-secondary"
        @click="emit('prev')"
      >
        ← Back
      </button>
      <button
        class="btn-secondary"
        @click="brain.checkOllamaStatus()"
      >
        🔄 Retry
      </button>
      <button
        class="btn-primary"
        :disabled="!brain.ollamaStatus.running"
        @click="emit('next')"
      >
        Next →
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useBrainStore } from '../stores/brain';

const brain = useBrainStore();
const emit = defineEmits<{ (e: 'prev'): void; (e: 'next'): void }>();
</script>

<style src="./BrainSetupView.css" scoped />
