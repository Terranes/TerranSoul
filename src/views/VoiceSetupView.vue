<template>
  <div class="voice-setup">
    <!-- Step indicator -->
    <div class="vs-steps">
      <div
        v-for="(label, i) in stepLabels"
        :key="i"
        :class="['vs-step', { active: step === i, done: step > i }]"
      >
        <span class="vs-step-dot">{{ step > i ? '✓' : i + 1 }}</span>
        <span class="vs-step-label">{{ label }}</span>
      </div>
    </div>

    <!-- ── Step 0: Choose voice mode ── -->
    <div
      v-if="step === 0"
      class="vs-card"
    >
      <h2>🎤 Voice Setup</h2>
      <p class="vs-desc">
        Add voice input/output to TerranSoul. Choose how you'd like to handle speech.
      </p>
      <div class="vs-tiers">
        <div
          :class="['vs-tier', { selected: selectedMode === 'browser' }]"
          @click="selectedMode = 'browser'"
        >
          <div class="vs-tier-header">
            <strong>🖥 Browser Voice</strong>
            <span class="vs-badge-rec">⭐ Free</span>
          </div>
          <p>Use the browser's built-in Web Speech API for input, and Edge TTS for high-quality output.</p>
          <small>No downloads or API keys needed · Works out of the box</small>
        </div>
        <div
          :class="['vs-tier', { selected: selectedMode === 'cloud' }]"
          @click="selectedMode = 'cloud'"
        >
          <div class="vs-tier-header">
            <strong>☁️ Cloud API</strong>
          </div>
          <p>Use OpenAI Whisper (ASR) and/or OpenAI TTS with your own API key.</p>
          <small>Best quality · Requires API key</small>
        </div>
        <div
          :class="['vs-tier', { selected: selectedMode === 'groq' }]"
          @click="selectedMode = 'groq'"
        >
          <div class="vs-tier-header">
            <strong>⚡ Groq (fast)</strong>
            <span class="vs-badge-rec">⭐ Free tier</span>
          </div>
          <p>Groq Whisper for ultra-fast speech recognition. OpenAI-compatible, generous free tier.</p>
          <small>Very fast ASR · Requires Groq API key</small>
        </div>
        <div
          :class="['vs-tier', { selected: selectedMode === 'text-only' }]"
          @click="selectedMode = 'text-only'"
        >
          <div class="vs-tier-header">
            <strong>⌨ Text Only</strong>
          </div>
          <p>Skip voice setup. You can always enable it later.</p>
        </div>
      </div>
      <button
        class="btn-primary"
        :disabled="!selectedMode"
        @click="goToConfig"
      >
        Next →
      </button>
    </div>

    <!-- ── Step 1A: Browser voice ── -->
    <div
      v-else-if="step === 1 && selectedMode === 'browser'"
      class="vs-card"
    >
      <h2>🖥 Browser Voice</h2>
      <p class="vs-desc">
        Uses the Web Speech API for speech input and Edge TTS for high-quality voice output.
        No downloads or API keys needed.
      </p>
      <div class="vs-provider-list">
        <div class="vs-provider-item">
          <strong>ASR:</strong> Web Speech API (browser-native)
        </div>
        <div class="vs-provider-item">
          <strong>TTS:</strong> Edge TTS (free, Microsoft neural voices)
        </div>
      </div>
      <p class="vs-note">
        💡 The Web Speech API works best in Chrome. Edge TTS provides high-quality
        neural voices in many languages — all running through Tauri's Rust backend.
      </p>
      <div class="vs-nav">
        <button
          class="btn-secondary"
          @click="step = 0"
        >
          ← Back
        </button>
        <button
          class="btn-primary"
          @click="activateBrowser"
        >
          Activate →
        </button>
      </div>
    </div>

    <!-- ── Step 1B: Cloud API ── -->
    <div
      v-else-if="step === 1 && selectedMode === 'cloud'"
      class="vs-card"
    >
      <h2>☁️ Cloud Voice API</h2>
      <p class="vs-desc">
        Use OpenAI's voice APIs. Requires an API key.
      </p>
      <div class="vs-form">
        <label for="cloud-api-key-input">API Key:</label>
        <input
          id="cloud-api-key-input"
          v-model="cloudApiKey"
          type="password"
          placeholder="sk-…"
          class="vs-input"
        >
        <div class="vs-checkboxes">
          <label class="vs-checkbox">
            <input
              v-model="cloudEnableAsr"
              type="checkbox"
            >
            Enable ASR (Whisper API) — speech input
          </label>
          <label class="vs-checkbox">
            <input
              v-model="cloudEnableTts"
              type="checkbox"
            >
            Enable TTS (OpenAI TTS) — voice output
          </label>
        </div>
      </div>
      <div class="vs-nav">
        <button
          class="btn-secondary"
          @click="step = 0"
        >
          ← Back
        </button>
        <button
          class="btn-primary"
          :disabled="!cloudApiKey || (!cloudEnableAsr && !cloudEnableTts)"
          @click="activateCloud"
        >
          Activate →
        </button>
      </div>
    </div>

    <!-- ── Step 1C: Groq ── -->
    <div
      v-else-if="step === 1 && selectedMode === 'groq'"
      class="vs-card"
    >
      <h2>⚡ Groq Voice</h2>
      <p class="vs-desc">
        Groq provides ultra-fast Whisper transcription with a generous free tier.
        Requires a Groq API key (free at console.groq.com).
      </p>
      <div class="vs-form">
        <label for="groq-api-key-input">Groq API Key:</label>
        <input
          id="groq-api-key-input"
          v-model="groqApiKey"
          type="password"
          placeholder="gsk_…"
          class="vs-input"
        >
        <div class="vs-checkboxes">
          <label class="vs-checkbox">
            <input
              v-model="groqEnableTts"
              type="checkbox"
            >
            Also enable TTS (OpenAI TTS) — voice output
          </label>
        </div>
      </div>
      <div class="vs-nav">
        <button
          class="btn-secondary"
          @click="step = 0"
        >
          ← Back
        </button>
        <button
          class="btn-primary"
          :disabled="!groqApiKey"
          @click="activateGroq"
        >
          Activate →
        </button>
      </div>
    </div>

    <!-- ── Done ── -->
    <div
      v-else-if="step === 99"
      class="vs-card vs-done"
    >
      <div class="vs-done-icon">
        🎉
      </div>
      <h2>Voice configured!</h2>
      <p v-if="selectedMode === 'browser'">
        Using <strong>Web Speech API</strong> for speech input and
        <strong>Edge TTS</strong> for voice output.
      </p>
      <p v-else-if="selectedMode === 'cloud'">
        Using <strong>OpenAI</strong> cloud APIs for
        {{ cloudEnableAsr && cloudEnableTts ? 'ASR + TTS' : cloudEnableAsr ? 'ASR' : 'TTS' }}.
      </p>
      <p v-else-if="selectedMode === 'groq'">
        Using <strong>Groq Whisper</strong> for fast speech recognition{{ groqEnableTts ? ' + OpenAI TTS for voice output' : '' }}.
      </p>
      <p v-else>
        Voice is <strong>disabled</strong>. You can enable it anytime from settings.
      </p>
      <button
        class="btn-primary"
        @click="emit('done')"
      >
        Continue →
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useVoiceStore } from '../stores/voice';

const emit = defineEmits<{ (e: 'done'): void }>();

const voice = useVoiceStore();
const step = ref(0);
const selectedMode = ref<'browser' | 'cloud' | 'groq' | 'text-only' | null>(null);

const stepLabels = ['Choose', 'Configure', 'Done'];

// Cloud config
const cloudApiKey = ref('');
const cloudEnableAsr = ref(true);
const cloudEnableTts = ref(true);

// Groq config
const groqApiKey = ref('');
const groqEnableTts = ref(false);

function goToConfig() {
  if (selectedMode.value === 'text-only') {
    activateTextOnly();
  } else {
    step.value = 1;
  }
}

async function activateBrowser() {
  await voice.setAsrProvider('web-speech');
  await voice.setTtsProvider('web-speech');
  step.value = 99;
}

async function activateCloud() {
  await voice.setAsrProvider(cloudEnableAsr.value ? 'whisper-api' : null);
  await voice.setTtsProvider(cloudEnableTts.value ? 'openai-tts' : null);
  await voice.setApiKey(cloudApiKey.value);
  step.value = 99;
}

async function activateGroq() {
  await voice.setAsrProvider('groq-whisper');
  await voice.setTtsProvider(groqEnableTts.value ? 'openai-tts' : null);
  await voice.setApiKey(groqApiKey.value);
  step.value = 99;
}

async function activateTextOnly() {
  await voice.clearConfig();
  step.value = 99;
}

onMounted(async () => {
  await voice.initialise();
});
</script>

<style scoped>
.voice-setup { display: flex; flex-direction: column; align-items: center; gap: 1.5rem; padding: 2rem; height: 100%; min-height: 100%; overflow-x: hidden; overflow-y: auto; scrollbar-gutter: stable; background: var(--ts-bg-base); }
.vs-steps { display: flex; gap: 0.5rem; align-items: center; }
.vs-step { display: flex; align-items: center; gap: 0.4rem; font-size: 0.8rem; color: var(--ts-text-muted); }
.vs-step.active .vs-step-dot { background: var(--ts-accent-violet-hover); color: var(--ts-text-on-accent); }
.vs-step.done .vs-step-dot { background: var(--ts-success-dim); color: var(--ts-text-on-accent); }
.vs-step-dot { width: 24px; height: 24px; border-radius: 50%; background: var(--ts-bg-surface); display: flex; align-items: center; justify-content: center; font-size: 0.75rem; color: var(--ts-text-muted); }
.vs-step-label { display: none; }
@media (min-width: 480px) { .vs-step-label { display: inline; } }
.vs-card { background: var(--ts-bg-surface); border: 1px solid var(--ts-border); border-radius: 12px; padding: 2rem; width: min(600px, 100%); display: flex; flex-direction: column; gap: 1rem; }
.vs-card h2 { margin: 0; font-size: 1.3rem; color: var(--ts-text-primary); }
.vs-desc { color: var(--ts-text-secondary); margin: 0; line-height: 1.5; }

/* Tier selection */
.vs-tiers { display: flex; flex-direction: column; gap: 0.5rem; }
.vs-tier { padding: 1rem; background: var(--ts-bg-base); border-radius: 8px; border: 2px solid var(--ts-border); cursor: pointer; transition: border-color var(--ts-transition-fast), background var(--ts-transition-fast); }
.vs-tier:hover { border-color: var(--ts-border-medium); background: var(--ts-bg-hover); }
.vs-tier.selected { border-color: var(--ts-success-dim); background: var(--ts-bg-selected); }
.vs-tier:first-child { border-color: var(--ts-accent-violet-hover); }
.vs-tier:first-child.selected { border-color: var(--ts-success-dim); }
.vs-tier-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.25rem; }
.vs-badge-rec { font-size: 0.7rem; background: var(--ts-accent-violet-hover); color: var(--ts-text-on-accent); padding: 0.15rem 0.5rem; border-radius: 999px; }
.vs-tier p { margin: 0.25rem 0; font-size: 0.85rem; color: var(--ts-text-secondary); }
.vs-tier small { color: var(--ts-text-muted); font-size: 0.75rem; }

/* Form */
.vs-form { display: flex; flex-direction: column; gap: 0.5rem; }
.vs-form label { font-size: 0.85rem; color: var(--ts-text-secondary); }
.vs-input { padding: 0.5rem 0.75rem; background: var(--ts-bg-input); border: 1px solid var(--ts-border-medium); border-radius: 6px; color: var(--ts-text-primary); font-size: 0.85rem; outline: none; transition: border-color var(--ts-transition-fast); }
.vs-input:focus { border-color: var(--ts-accent-violet-hover); box-shadow: 0 0 0 3px var(--ts-accent-glow); }

/* Checkboxes */
.vs-checkboxes { display: flex; flex-direction: column; gap: 0.4rem; margin-top: 0.5rem; }
.vs-checkbox { display: flex; align-items: center; gap: 0.5rem; font-size: 0.85rem; color: var(--ts-text-secondary); cursor: pointer; }
.vs-checkbox input { accent-color: var(--ts-accent-violet-hover); }

/* Status */
.vs-status { padding: 0.75rem 1rem; border-radius: 8px; font-weight: 500; display: flex; align-items: center; gap: 0.5rem; min-height: 3rem; }
.vs-status.ok { background: var(--ts-success-bg); color: var(--ts-success); }
.vs-status.error { background: var(--ts-error-bg); color: var(--ts-error); }

/* Provider list */
.vs-provider-list { display: flex; flex-direction: column; gap: 0.4rem; background: var(--ts-bg-base); border-radius: 8px; padding: 0.75rem 1rem; }
.vs-provider-item { font-size: 0.85rem; color: var(--ts-text-secondary); }
.vs-note { font-size: 0.8rem; color: var(--ts-text-muted); background: var(--ts-bg-base); border-radius: 8px; padding: 0.75rem 1rem; margin: 0; }

/* Navigation */
.vs-nav { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }

/* Done */
.vs-done { align-items: center; text-align: center; }
.vs-done-icon { font-size: 3rem; }

/* Buttons */
.btn-primary { padding: 0.5rem 1.25rem; background: var(--ts-accent-violet-hover); color: var(--ts-text-on-accent); border: none; border-radius: 8px; cursor: pointer; font-size: 0.9rem; transition: background var(--ts-transition-fast); }
.btn-primary:hover { background: var(--ts-accent-violet); }
.btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-secondary { padding: 0.5rem 1.25rem; background: var(--ts-bg-elevated); color: var(--ts-text-primary); border: none; border-radius: 8px; cursor: pointer; font-size: 0.9rem; transition: background var(--ts-transition-fast); }
.btn-secondary:hover { background: var(--ts-bg-hover); }
code { background: var(--ts-bg-base); padding: 0.1rem 0.3rem; border-radius: 3px; font-size: 0.8rem; color: var(--ts-text-primary); }
a { color: var(--ts-accent-violet); }
</style>
