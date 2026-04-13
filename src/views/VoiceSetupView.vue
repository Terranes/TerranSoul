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
    <div v-if="step === 0" class="vs-card">
      <h2>🎤 Voice Setup</h2>
      <p class="vs-desc">
        Add voice input/output to TerranSoul. Choose how you'd like to handle speech.
      </p>
      <div class="vs-tiers">
        <div
          :class="['vs-tier', { selected: selectedMode === 'open-llm-vtuber' }]"
          @click="selectedMode = 'open-llm-vtuber'"
        >
          <div class="vs-tier-header">
            <strong>🌐 Open-LLM-VTuber</strong>
            <span class="vs-badge-rec">⭐ Recommended</span>
          </div>
          <p>Connect to a running Open-LLM-VTuber server. Supports 18+ TTS and 7+ ASR engines.</p>
          <small>Requires Open-LLM-VTuber running locally · Free</small>
        </div>
        <div
          :class="['vs-tier', { selected: selectedMode === 'browser' }]"
          @click="selectedMode = 'browser'"
        >
          <div class="vs-tier-header">
            <strong>🖥 Browser Voice</strong>
          </div>
          <p>Use the browser's built-in Web Speech API. Zero setup, limited accuracy.</p>
          <small>No downloads needed · Works offline on supported browsers</small>
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
          :class="['vs-tier', { selected: selectedMode === 'text-only' }]"
          @click="selectedMode = 'text-only'"
        >
          <div class="vs-tier-header">
            <strong>⌨ Text Only</strong>
          </div>
          <p>Skip voice setup. You can always enable it later.</p>
        </div>
      </div>
      <button class="btn-primary" :disabled="!selectedMode" @click="goToConfig">
        Next →
      </button>
    </div>

    <!-- ── Step 1A: Open-LLM-VTuber config ── -->
    <div v-else-if="step === 1 && selectedMode === 'open-llm-vtuber'" class="vs-card">
      <h2>🌐 Open-LLM-VTuber Connection</h2>
      <p class="vs-desc">
        Enter the WebSocket URL of your running Open-LLM-VTuber server.
        The default URL is <code>ws://localhost:12393/client-ws</code>.
      </p>
      <div class="vs-form">
        <label for="ollv-url-input">WebSocket URL:</label>
        <input
          id="ollv-url-input"
          v-model="ollvUrl"
          type="url"
          placeholder="ws://localhost:12393/client-ws"
          class="vs-input"
        />
      </div>
      <div :class="['vs-status', ollvHealthy === true ? 'ok' : ollvHealthy === false ? 'error' : '']">
        <template v-if="ollvChecking">
          <span class="vs-spinner" /> Checking connection…
        </template>
        <template v-else-if="ollvHealthy === true">
          ✅ Connected to Open-LLM-VTuber server
        </template>
        <template v-else-if="ollvHealthy === false">
          ❌ Cannot connect. Make sure Open-LLM-VTuber is running.
        </template>
      </div>
      <div class="vs-install-hint">
        <p><strong>Don't have Open-LLM-VTuber?</strong></p>
        <ol>
          <li>Clone: <code>git clone https://github.com/Open-LLM-VTuber/Open-LLM-VTuber</code></li>
          <li>Install dependencies and configure your preferred TTS/ASR engines</li>
          <li>Run the server: <code>python run_server.py</code></li>
          <li>Click "Check Connection" below</li>
        </ol>
      </div>
      <div class="vs-nav">
        <button class="btn-secondary" @click="step = 0">← Back</button>
        <button class="btn-secondary" @click="checkOllvHealth">🔄 Check Connection</button>
        <button class="btn-primary" :disabled="ollvHealthy !== true" @click="activateOllv">
          Activate →
        </button>
      </div>
    </div>

    <!-- ── Step 1B: Browser voice ── -->
    <div v-else-if="step === 1 && selectedMode === 'browser'" class="vs-card">
      <h2>🖥 Browser Voice</h2>
      <p class="vs-desc">
        Uses the Web Speech API built into your browser. No downloads or API keys needed.
      </p>
      <div class="vs-provider-list">
        <div class="vs-provider-item">
          <strong>ASR:</strong> Web Speech API (browser-native)
        </div>
        <div class="vs-provider-item">
          <strong>TTS:</strong> Not available (text-only output)
        </div>
      </div>
      <p class="vs-note">
        💡 The Web Speech API works best in Chrome. For higher-quality voice, consider
        connecting to Open-LLM-VTuber.
      </p>
      <div class="vs-nav">
        <button class="btn-secondary" @click="step = 0">← Back</button>
        <button class="btn-primary" @click="activateBrowser">Activate →</button>
      </div>
    </div>

    <!-- ── Step 1C: Cloud API ── -->
    <div v-else-if="step === 1 && selectedMode === 'cloud'" class="vs-card">
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
        />
        <div class="vs-checkboxes">
          <label class="vs-checkbox">
            <input v-model="cloudEnableAsr" type="checkbox" />
            Enable ASR (Whisper API) — speech input
          </label>
          <label class="vs-checkbox">
            <input v-model="cloudEnableTts" type="checkbox" />
            Enable TTS (OpenAI TTS) — voice output
          </label>
        </div>
      </div>
      <div class="vs-nav">
        <button class="btn-secondary" @click="step = 0">← Back</button>
        <button
          class="btn-primary"
          :disabled="!cloudApiKey || (!cloudEnableAsr && !cloudEnableTts)"
          @click="activateCloud"
        >
          Activate →
        </button>
      </div>
    </div>

    <!-- ── Done ── -->
    <div v-else-if="step === 99" class="vs-card vs-done">
      <div class="vs-done-icon">🎉</div>
      <h2>Voice configured!</h2>
      <p v-if="selectedMode === 'open-llm-vtuber'">
        Connected to <strong>Open-LLM-VTuber</strong> at <code>{{ ollvUrl }}</code>.
        TTS and ASR are handled by the server.
      </p>
      <p v-else-if="selectedMode === 'browser'">
        Using <strong>Web Speech API</strong> for speech input.
      </p>
      <p v-else-if="selectedMode === 'cloud'">
        Using <strong>OpenAI</strong> cloud APIs for
        {{ cloudEnableAsr && cloudEnableTts ? 'ASR + TTS' : cloudEnableAsr ? 'ASR' : 'TTS' }}.
      </p>
      <p v-else>
        Voice is <strong>disabled</strong>. You can enable it anytime from settings.
      </p>
      <button class="btn-primary" @click="emit('done')">Continue →</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useVoiceStore } from '../stores/voice';
import { OllvClient, DEFAULT_OLLV_WS_URL } from '../utils/ollv-client';

const emit = defineEmits<{ (e: 'done'): void }>();

const voice = useVoiceStore();
const step = ref(0);
const selectedMode = ref<'open-llm-vtuber' | 'browser' | 'cloud' | 'text-only' | null>(null);

const stepLabels = ['Choose', 'Configure', 'Done'];

// Open-LLM-VTuber config
const ollvUrl = ref(DEFAULT_OLLV_WS_URL);
const ollvHealthy = ref<boolean | null>(null);
const ollvChecking = ref(false);

// Cloud config
const cloudApiKey = ref('');
const cloudEnableAsr = ref(true);
const cloudEnableTts = ref(true);

function goToConfig() {
  if (selectedMode.value === 'text-only') {
    activateTextOnly();
  } else {
    step.value = 1;
  }
}

async function checkOllvHealth() {
  ollvChecking.value = true;
  ollvHealthy.value = null;
  try {
    ollvHealthy.value = await OllvClient.healthCheck(ollvUrl.value, 3000);
  } catch {
    ollvHealthy.value = false;
  } finally {
    ollvChecking.value = false;
  }
}

async function activateOllv() {
  await voice.setAsrProvider('open-llm-vtuber');
  await voice.setTtsProvider('open-llm-vtuber');
  await voice.setEndpointUrl(ollvUrl.value);
  step.value = 99;
}

async function activateBrowser() {
  await voice.setAsrProvider('web-speech');
  await voice.setTtsProvider(null);
  step.value = 99;
}

async function activateCloud() {
  await voice.setAsrProvider(cloudEnableAsr.value ? 'whisper-api' : null);
  await voice.setTtsProvider(cloudEnableTts.value ? 'openai-tts' : null);
  await voice.setApiKey(cloudApiKey.value);
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
.voice-setup { display: flex; flex-direction: column; align-items: center; gap: 1.5rem; padding: 2rem; min-height: 100%; background: #0f172a; }
.vs-steps { display: flex; gap: 0.5rem; align-items: center; }
.vs-step { display: flex; align-items: center; gap: 0.4rem; font-size: 0.8rem; color: #475569; }
.vs-step.active .vs-step-dot { background: #8b5cf6; color: #fff; }
.vs-step.done .vs-step-dot { background: #22c55e; color: #fff; }
.vs-step-dot { width: 24px; height: 24px; border-radius: 50%; background: #1e293b; display: flex; align-items: center; justify-content: center; font-size: 0.75rem; }
.vs-step-label { display: none; }
@media (min-width: 480px) { .vs-step-label { display: inline; } }
.vs-card { background: #1e293b; border-radius: 12px; padding: 2rem; width: min(600px, 100%); display: flex; flex-direction: column; gap: 1rem; }
.vs-card h2 { margin: 0; font-size: 1.3rem; }
.vs-desc { color: #94a3b8; margin: 0; line-height: 1.5; }

/* Tier selection */
.vs-tiers { display: flex; flex-direction: column; gap: 0.5rem; }
.vs-tier { padding: 1rem; background: #0f172a; border-radius: 8px; border: 2px solid transparent; cursor: pointer; transition: border-color 0.15s; }
.vs-tier:hover { border-color: #334155; }
.vs-tier.selected { border-color: #22c55e; background: #1a2e1a; }
.vs-tier:first-child { border-color: #8b5cf6; }
.vs-tier:first-child.selected { border-color: #22c55e; }
.vs-tier-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.25rem; }
.vs-badge-rec { font-size: 0.7rem; background: #7c3aed; color: #fff; padding: 0.15rem 0.5rem; border-radius: 999px; }
.vs-tier p { margin: 0.25rem 0; font-size: 0.85rem; color: #94a3b8; }
.vs-tier small { color: #475569; font-size: 0.75rem; }

/* Form */
.vs-form { display: flex; flex-direction: column; gap: 0.5rem; }
.vs-form label { font-size: 0.85rem; color: #94a3b8; }
.vs-input { padding: 0.5rem 0.75rem; background: #0f172a; border: 1px solid #334155; border-radius: 6px; color: #f1f5f9; font-size: 0.85rem; }
.vs-input:focus { outline: none; border-color: #8b5cf6; }

/* Checkboxes */
.vs-checkboxes { display: flex; flex-direction: column; gap: 0.4rem; margin-top: 0.5rem; }
.vs-checkbox { display: flex; align-items: center; gap: 0.5rem; font-size: 0.85rem; color: #94a3b8; cursor: pointer; }
.vs-checkbox input { accent-color: #8b5cf6; }

/* Status */
.vs-status { padding: 0.75rem 1rem; border-radius: 8px; font-weight: 500; display: flex; align-items: center; gap: 0.5rem; min-height: 3rem; }
.vs-status.ok { background: #1a2e1a; color: #86efac; }
.vs-status.error { background: #2d1c1c; color: #fca5a5; }
.vs-spinner { width: 16px; height: 16px; border: 2px solid #334155; border-top-color: #8b5cf6; border-radius: 50%; animation: vs-spin 0.8s linear infinite; display: inline-block; }
@keyframes vs-spin { to { transform: rotate(360deg); } }

/* Provider list */
.vs-provider-list { display: flex; flex-direction: column; gap: 0.4rem; background: #0f172a; border-radius: 8px; padding: 0.75rem 1rem; }
.vs-provider-item { font-size: 0.85rem; color: #94a3b8; }
.vs-note { font-size: 0.8rem; color: #64748b; background: #0f172a; border-radius: 8px; padding: 0.75rem 1rem; margin: 0; }

/* Install hint */
.vs-install-hint { background: #0f172a; border-radius: 8px; padding: 0.75rem 1rem; font-size: 0.85rem; color: #94a3b8; }
.vs-install-hint ol { margin: 0.5rem 0 0 1.25rem; line-height: 1.8; }

/* Navigation */
.vs-nav { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }

/* Done */
.vs-done { align-items: center; text-align: center; }
.vs-done-icon { font-size: 3rem; }

/* Buttons */
.btn-primary { padding: 0.5rem 1.25rem; background: #8b5cf6; color: #fff; border: none; border-radius: 8px; cursor: pointer; font-size: 0.9rem; }
.btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-secondary { padding: 0.5rem 1.25rem; background: #334155; color: #f1f5f9; border: none; border-radius: 8px; cursor: pointer; font-size: 0.9rem; }
code { background: #0f172a; padding: 0.1rem 0.3rem; border-radius: 3px; font-size: 0.8rem; }
a { color: #a78bfa; }
</style>
