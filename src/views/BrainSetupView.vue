<template>
  <div class="brain-setup">
    <!-- Step indicator -->
    <div class="bs-steps">
      <div
        v-for="(label, i) in stepLabels"
        :key="i"
        :class="['bs-step', { active: step === i, done: step > i }]"
      >
        <span class="bs-step-dot">{{ step > i ? '✓' : i + 1 }}</span>
        <span class="bs-step-label">{{ label }}</span>
      </div>
    </div>

    <!-- ── Step 0: Choose brain tier ── -->
    <div
      v-if="step === 0"
      class="bs-card"
    >
      <h2>🧠 Choose how to power your Brain</h2>
      <p class="bs-desc">
        TerranSoul needs an AI brain for conversations. Choose how you'd like to connect it.
      </p>
      <div class="bs-tiers">
        <div
          :class="['bs-tier', { selected: selectedTier === 'free' }]"
          @click="selectedTier = 'free'"
        >
          <div class="bs-tier-header">
            <strong>☁️ Free Cloud API</strong>
            <span class="bs-badge-free">Instant — no setup</span>
          </div>
          <p>Use free LLM providers (Groq, Cerebras, etc.) with zero configuration.</p>
          <small>No API key needed for some providers · Rate-limited</small>
        </div>
        <div
          :class="['bs-tier', { selected: selectedTier === 'paid' }]"
          @click="selectedTier = 'paid'"
        >
          <div class="bs-tier-header">
            <strong>💳 Paid Cloud API</strong>
          </div>
          <p>Use OpenAI, Anthropic, or other paid providers with your own API key.</p>
          <small>Best quality · Requires API key</small>
        </div>
        <div
          :class="['bs-tier', { selected: selectedTier === 'local' }]"
          @click="selectedTier = 'local'"
        >
          <div class="bs-tier-header">
            <strong>🖥 Local LLM (Ollama)</strong>
          </div>
          <p>Run a model locally on your machine with Ollama. Fully private, no internet needed.</p>
          <small>Requires Ollama installed · Best for privacy</small>
        </div>
      </div>
      <button
        class="btn-primary"
        :disabled="!selectedTier"
        @click="goToTierStep"
      >
        Next →
      </button>
    </div>

    <!-- ── Step 1A: Free API setup (auto-select provider) ── -->
    <div
      v-else-if="step === 1 && selectedTier === 'free'"
      class="bs-card"
    >
      <h2>☁️ Free Cloud API</h2>
      <p class="bs-desc">
        Select a free LLM provider. These use OpenAI-compatible APIs with generous free tiers.
      </p>
      <div class="bs-providers">
        <div
          v-for="p in brain.freeProviders"
          :key="p.id"
          :class="['bs-provider', { selected: selectedProvider === p.id }]"
          @click="selectedProvider = p.id"
        >
          <div class="bs-provider-header">
            <strong>{{ p.display_name }}</strong>
            <span
              v-if="p.id === 'pollinations'"
              class="bs-badge"
            >⭐ Recommended</span>
          </div>
          <p>{{ p.notes }}</p>
          <small>Model: <code>{{ p.model }}</code> · {{ p.rpm_limit }} RPM</small>
        </div>
      </div>
      <div
        v-if="freeApiKey !== null"
        class="bs-api-key"
      >
        <label>API Key (optional for some providers):</label>
        <input
          v-model="freeApiKeyInput"
          type="password"
          placeholder="Enter API key…"
          class="bs-input"
        >
      </div>
      <div class="bs-nav">
        <button
          class="btn-secondary"
          @click="step = 0"
        >
          ← Back
        </button>
        <button
          class="btn-primary"
          :disabled="!selectedProvider"
          @click="activateFreeApi"
        >
          Activate →
        </button>
      </div>
    </div>

    <!-- ── Step 1B: Paid API setup ── -->
    <div
      v-else-if="step === 1 && selectedTier === 'paid'"
      class="bs-card"
    >
      <h2>💳 Paid Cloud API</h2>
      <p class="bs-desc">
        Enter your API credentials. We support any OpenAI-compatible endpoint.
      </p>
      <div class="bs-form">
        <label for="paid-provider-select">Provider:</label>
        <select
          id="paid-provider-select"
          v-model="paidProvider"
          class="bs-select"
        >
          <option value="openai">
            OpenAI
          </option>
          <option value="anthropic">
            Anthropic
          </option>
          <option value="custom">
            Custom endpoint
          </option>
        </select>
        <label for="paid-api-key-input">API Key:</label>
        <input
          id="paid-api-key-input"
          v-model="paidApiKey"
          type="password"
          placeholder="sk-…"
          class="bs-input"
        >
        <label for="paid-model-input">Model:</label>
        <input
          id="paid-model-input"
          v-model="paidModel"
          type="text"
          placeholder="gpt-4o"
          class="bs-input"
        >
        <label
          v-if="paidProvider === 'custom'"
          for="paid-base-url-input"
        >Base URL:</label>
        <input
          v-if="paidProvider === 'custom'"
          id="paid-base-url-input"
          v-model="paidBaseUrl"
          type="url"
          placeholder="https://api.example.com"
          class="bs-input"
        >
      </div>
      <div class="bs-nav">
        <button
          class="btn-secondary"
          @click="step = 0"
        >
          ← Back
        </button>
        <button
          class="btn-primary"
          :disabled="!paidApiKey || !paidModel"
          @click="activatePaidApi"
        >
          Activate →
        </button>
      </div>
    </div>

    <!-- ── Step 1C: Local Ollama setup — Hardware analysis ── -->
    <div
      v-else-if="step === 1 && selectedTier === 'local'"
      class="bs-card"
    >
      <h2>🖥 Local LLM Setup</h2>
      <p class="bs-desc">
        We'll analyse your hardware and recommend the best model for your machine.
      </p>
      <div
        v-if="brain.systemInfo"
        class="bs-hw"
      >
        <div class="bs-hw-row">
          <span>💾 RAM</span>
          <strong>{{ formatRam(brain.systemInfo.total_ram_mb) }} ({{ brain.systemInfo.ram_tier_label }})</strong>
        </div>
        <div class="bs-hw-row">
          <span>🖥 CPU</span>
          <strong>{{ brain.systemInfo.cpu_name }} · {{ brain.systemInfo.cpu_cores }} cores</strong>
        </div>
        <div class="bs-hw-row">
          <span>🗂 OS</span>
          <strong>{{ brain.systemInfo.os_name }} ({{ brain.systemInfo.arch }})</strong>
        </div>
      </div>
      <p
        v-else
        class="bs-loading"
      >
        Analysing hardware…
      </p>
      <div class="bs-nav">
        <button
          class="btn-secondary"
          @click="step = 0"
        >
          ← Back
        </button>
        <button
          class="btn-primary"
          :disabled="!brain.systemInfo"
          @click="step = 2"
        >
          Next →
        </button>
      </div>
    </div>

    <!-- ── Step 2: Choose local model ── -->
    <div
      v-else-if="step === 2"
      class="bs-card"
    >
      <h2>Choose your Brain</h2>
      <p class="bs-desc">
        Based on your {{ formatRam(brain.systemInfo?.total_ram_mb ?? 0) }} of RAM, we recommend:
      </p>
      <ul class="bs-models">
        <li
          v-for="m in brain.recommendations"
          :key="m.model_tag"
          :class="['bs-model', { top: m.is_top_pick, selected: selectedModel === m.model_tag }]"
          @click="selectedModel = m.model_tag"
        >
          <div class="bs-model-header">
            <strong>{{ m.display_name }}</strong>
            <span
              v-if="m.is_top_pick"
              class="bs-badge"
            >⭐ Recommended</span>
            <span
              v-if="m.is_cloud"
              class="bs-badge bs-cloud"
            >☁️ Cloud</span>
          </div>
          <p>{{ m.description }}</p>
          <small v-if="m.is_cloud">Cloud-routed · no local RAM needed · tag: <code>{{ m.model_tag }}</code></small>
          <small v-else>Requires {{ formatRam(m.required_ram_mb) }} RAM · tag: <code>{{ m.model_tag }}</code></small>
        </li>
      </ul>
      <div class="bs-nav">
        <button
          class="btn-secondary"
          @click="step = 1"
        >
          ← Back
        </button>
        <button
          class="btn-primary"
          :disabled="!selectedModel"
          @click="step = 3"
        >
          Next →
        </button>
      </div>
    </div>

    <!-- ── Step 3: Check / install Ollama ── -->
    <div
      v-else-if="step === 3"
      class="bs-card"
    >
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
          @click="step = 2"
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
          @click="step = 4"
        >
          Next →
        </button>
      </div>
    </div>

    <!-- ── Step 4: Download model ── -->
    <div
      v-else-if="step === 4"
      class="bs-card"
    >
      <h2>Download {{ selectedModel }}</h2>
      <p class="bs-desc">
        This will download the model via Ollama. It may take several minutes depending on
        your connection speed.
      </p>

      <div
        v-if="modelAlreadyInstalled"
        class="bs-status-indicator ok"
      >
        ✅ Model already installed locally
      </div>
      <div
        v-else-if="brain.isPulling"
        class="bs-pulling"
      >
        <div class="bs-spinner" />
        <span>Downloading… this may take a few minutes.</span>
      </div>
      <div
        v-else-if="brain.pullError"
        class="bs-status-indicator error"
      >
        ❌ {{ brain.pullError }}
      </div>

      <div class="bs-nav">
        <button
          class="btn-secondary"
          @click="step = 3"
        >
          ← Back
        </button>
        <button
          v-if="!modelAlreadyInstalled && !brain.isPulling"
          class="btn-primary"
          @click="doPull"
        >
          ⬇ Download model
        </button>
        <button
          v-if="modelAlreadyInstalled || pullDone"
          class="btn-primary"
          @click="finishLocal"
        >
          Next →
        </button>
      </div>
    </div>

    <!-- ── Step 5 (or done): Brain connected ── -->
    <div
      v-else-if="step === 5 || step === 99"
      class="bs-card bs-done"
    >
      <div class="bs-done-icon">
        🎉
      </div>
      <h2>Brain connected!</h2>
      <p v-if="selectedTier === 'free'">
        Using <strong>{{ selectedProviderName }}</strong> (free cloud API).
        TerranSoul is ready to chat — no setup required!
      </p>
      <p v-else-if="selectedTier === 'paid'">
        Using <strong>{{ paidModel }}</strong> via paid API.
        TerranSoul will use it for all future conversations.
      </p>
      <p v-else>
        <strong>{{ selectedModel }}</strong> is now your local brain.
        TerranSoul will use it for all future conversations, memory extraction, and smart recall.
      </p>
      <button
        class="btn-primary"
        @click="emit('done')"
      >
        Start chatting →
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useBrainStore } from '../stores/brain';

const emit = defineEmits<{ (e: 'done'): void }>();

const brain = useBrainStore();
const step = ref(0);
const selectedTier = ref<'free' | 'paid' | 'local'>('free');
const selectedModel = ref('');
const selectedProvider = ref('pollinations');
const freeApiKey = ref<string | null>(null);
const freeApiKeyInput = ref('');
const pullDone = ref(false);

// Paid API fields
const paidProvider = ref('openai');
const paidApiKey = ref('');
const paidModel = ref('gpt-4o');
const paidBaseUrl = ref('https://api.openai.com');

const stepLabels = computed(() => {
  if (selectedTier.value === 'free') return ['Choose', 'Provider', 'Done'];
  if (selectedTier.value === 'paid') return ['Choose', 'API Key', 'Done'];
  return ['Choose', 'Hardware', 'Model', 'Ollama', 'Download', 'Done'];
});

const modelAlreadyInstalled = computed(() =>
  brain.installedModels.some((m) => m.name === selectedModel.value),
);

const selectedProviderName = computed(() =>
  brain.freeProviders.find((p) => p.id === selectedProvider.value)?.display_name ?? selectedProvider.value,
);

const paidBaseUrlResolved = computed(() => {
  switch (paidProvider.value) {
    case 'openai': return 'https://api.openai.com';
    case 'anthropic': return 'https://api.anthropic.com';
    default: return paidBaseUrl.value;
  }
});

function formatRam(mb: number): string {
  return mb >= 1024 ? `${(mb / 1024).toFixed(0)} GB` : `${mb} MB`;
}

function goToTierStep() {
  step.value = 1;
}

async function activateFreeApi() {
  try {
    await brain.setBrainMode({
      mode: 'free_api',
      provider_id: selectedProvider.value,
      api_key: freeApiKeyInput.value || null,
    });
  } catch {
    // Tauri unavailable — set locally
    brain.brainMode = {
      mode: 'free_api',
      provider_id: selectedProvider.value,
      api_key: freeApiKeyInput.value || null,
    };
  }
  step.value = 99;
}

async function activatePaidApi() {
  try {
    await brain.setBrainMode({
      mode: 'paid_api',
      provider: paidProvider.value,
      api_key: paidApiKey.value,
      model: paidModel.value,
      base_url: paidBaseUrlResolved.value,
    });
  } catch {
    brain.brainMode = {
      mode: 'paid_api',
      provider: paidProvider.value,
      api_key: paidApiKey.value,
      model: paidModel.value,
      base_url: paidBaseUrlResolved.value,
    };
  }
  step.value = 99;
}

async function doPull() {
  const ok = await brain.pullModel(selectedModel.value);
  if (ok) {
    pullDone.value = true;
    step.value = 5;
  }
}

async function finishLocal() {
  await brain.setActiveBrain(selectedModel.value);
  try {
    await brain.setBrainMode({
      mode: 'local_ollama',
      model: selectedModel.value,
    });
  } catch {
    brain.brainMode = {
      mode: 'local_ollama',
      model: selectedModel.value,
    };
  }
  step.value = 5;
}

onMounted(async () => {
  await brain.initialise();
  if (brain.topRecommendation) {
    selectedModel.value = brain.topRecommendation.model_tag;
  }
  if (brain.freeProviders.length > 0) {
    selectedProvider.value = brain.freeProviders[0].id;
  }
});
</script>

<style scoped>
.brain-setup { display: flex; flex-direction: column; align-items: center; gap: 1.5rem; padding: 2rem; min-height: 100%; background: var(--ts-bg-base); }
.bs-steps { display: flex; gap: 0.5rem; align-items: center; }
.bs-step { display: flex; align-items: center; gap: 0.4rem; font-size: 0.8rem; color: var(--ts-text-muted); }
.bs-step.active .bs-step-dot { background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); }
.bs-step.done .bs-step-dot { background: var(--ts-success-dim); color: var(--ts-text-on-accent); }
.bs-step-dot { width: 24px; height: 24px; border-radius: 50%; background: var(--ts-bg-surface); display: flex; align-items: center; justify-content: center; font-size: 0.75rem; color: var(--ts-text-muted); }
.bs-step-label { display: none; }
@media (min-width: 480px) { .bs-step-label { display: inline; } }
.bs-card { background: var(--ts-bg-surface); border: 1px solid var(--ts-border); border-radius: 12px; padding: 2rem; width: min(600px, 100%); display: flex; flex-direction: column; gap: 1rem; }
.bs-card h2 { margin: 0; font-size: 1.3rem; color: var(--ts-text-primary); }
.bs-desc { color: var(--ts-text-secondary); margin: 0; line-height: 1.5; }
.bs-loading { color: var(--ts-text-muted); }

/* Tier selection */
.bs-tiers { display: flex; flex-direction: column; gap: 0.5rem; }
.bs-tier { padding: 1rem; background: var(--ts-bg-base); border-radius: 8px; border: 2px solid var(--ts-border); cursor: pointer; transition: border-color var(--ts-transition-fast), background var(--ts-transition-fast); }
.bs-tier:hover { border-color: var(--ts-border-medium); background: var(--ts-bg-hover); }
.bs-tier.selected { border-color: var(--ts-success-dim); background: var(--ts-bg-selected); }
.bs-tier:first-child { border-color: var(--ts-accent-blue-hover); }
.bs-tier:first-child.selected { border-color: var(--ts-success-dim); }
.bs-tier-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.25rem; }
.bs-badge-free { font-size: 0.7rem; background: var(--ts-success-dim); color: var(--ts-text-on-accent); padding: 0.15rem 0.5rem; border-radius: 999px; }
.bs-tier p { margin: 0.25rem 0; font-size: 0.85rem; color: var(--ts-text-secondary); }
.bs-tier small { color: var(--ts-text-muted); font-size: 0.75rem; }

/* Providers */
.bs-providers { display: flex; flex-direction: column; gap: 0.5rem; max-height: 300px; overflow-y: auto; }
.bs-provider { padding: 0.75rem 1rem; background: var(--ts-bg-base); border-radius: 8px; border: 2px solid var(--ts-border); cursor: pointer; transition: border-color var(--ts-transition-fast), background var(--ts-transition-fast); }
.bs-provider:hover { border-color: var(--ts-border-medium); }
.bs-provider.selected { border-color: var(--ts-success-dim); background: var(--ts-bg-selected); }
.bs-provider-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.25rem; }
.bs-provider p { margin: 0; font-size: 0.8rem; color: var(--ts-text-secondary); }
.bs-provider small { color: var(--ts-text-muted); font-size: 0.75rem; }
.bs-provider code { background: var(--ts-bg-surface); padding: 0.1rem 0.3rem; border-radius: 3px; color: var(--ts-text-primary); }

/* Form elements */
.bs-form { display: flex; flex-direction: column; gap: 0.5rem; }
.bs-form label { font-size: 0.85rem; color: var(--ts-text-secondary); }
.bs-api-key { display: flex; flex-direction: column; gap: 0.3rem; }
.bs-api-key label { font-size: 0.8rem; color: var(--ts-text-secondary); }
.bs-input { padding: 0.5rem 0.75rem; background: var(--ts-bg-input); border: 1px solid var(--ts-border-medium); border-radius: 6px; color: var(--ts-text-primary); font-size: 0.85rem; outline: none; transition: border-color var(--ts-transition-fast); }
.bs-input:focus { border-color: var(--ts-accent-blue-hover); box-shadow: 0 0 0 3px var(--ts-accent-glow); }
.bs-input::placeholder { color: var(--ts-text-dim); }
.bs-select { padding: 0.5rem 0.75rem; background: var(--ts-bg-input); border: 1px solid var(--ts-border-medium); border-radius: 6px; color: var(--ts-text-primary); font-size: 0.85rem; }

/* Hardware */
.bs-hw { display: flex; flex-direction: column; gap: 0.4rem; background: var(--ts-bg-base); border-radius: 8px; padding: 0.75rem 1rem; }
.bs-hw-row { display: flex; justify-content: space-between; font-size: 0.9rem; color: var(--ts-text-primary); }

/* Models */
.bs-models { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.5rem; }
.bs-model { padding: 0.75rem 1rem; background: var(--ts-bg-base); border-radius: 8px; border: 2px solid var(--ts-border); cursor: pointer; transition: border-color var(--ts-transition-fast), background var(--ts-transition-fast); }
.bs-model:hover { border-color: var(--ts-border-medium); background: var(--ts-bg-hover); }
.bs-model.top { border-color: var(--ts-accent-blue-hover); }
.bs-model.selected { border-color: var(--ts-success-dim); background: var(--ts-bg-selected); }
.bs-model-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.25rem; }
.bs-badge { font-size: 0.75rem; background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); padding: 0.1rem 0.5rem; border-radius: 999px; }
.bs-cloud { background: var(--ts-accent-violet-hover); }
.bs-model p { margin: 0 0 0.25rem; font-size: 0.85rem; color: var(--ts-text-secondary); }
.bs-model small { color: var(--ts-text-muted); font-size: 0.75rem; }
.bs-model code { background: var(--ts-bg-surface); padding: 0.1rem 0.3rem; border-radius: 3px; color: var(--ts-text-primary); }

/* Status */
.bs-status-indicator { padding: 0.75rem 1rem; border-radius: 8px; font-weight: 500; }
.bs-status-indicator.ok { background: var(--ts-success-bg); color: var(--ts-success); }
.bs-status-indicator.error { background: var(--ts-error-bg); color: var(--ts-error); }
.bs-install-hint { background: var(--ts-bg-base); border-radius: 8px; padding: 0.75rem 1rem; font-size: 0.85rem; color: var(--ts-text-secondary); }
.bs-install-hint ol { margin: 0.5rem 0 0 1.25rem; line-height: 1.8; }
.bs-pulling { display: flex; align-items: center; gap: 0.75rem; color: var(--ts-text-secondary); }
.bs-spinner { width: 20px; height: 20px; border: 3px solid var(--ts-border-medium); border-top-color: var(--ts-accent-blue-hover); border-radius: 50%; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

/* Navigation */
.bs-nav { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }

/* Done */
.bs-done { align-items: center; text-align: center; }
.bs-done-icon { font-size: 3rem; }

/* Buttons */
.btn-primary { padding: 0.5rem 1.25rem; background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); border: none; border-radius: 8px; cursor: pointer; font-size: 0.9rem; transition: background var(--ts-transition-fast); }
.btn-primary:hover { background: var(--ts-accent-blue); }
.btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-secondary { padding: 0.5rem 1.25rem; background: var(--ts-bg-elevated); color: var(--ts-text-primary); border: none; border-radius: 8px; cursor: pointer; font-size: 0.9rem; transition: background var(--ts-transition-fast); }
.btn-secondary:hover { background: var(--ts-bg-hover); }
a { color: var(--ts-accent-blue); }
</style>
