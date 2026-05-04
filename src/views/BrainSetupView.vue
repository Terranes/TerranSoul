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
            <span class="bs-badge-free">Free-tier key</span>
          </div>
          <p>Use free-tier LLM providers such as OpenRouter, Gemini, NVIDIA NIM, or Pollinations with your own key/token.</p>
          <small>Provider authorization required · Rate limits vary</small>
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
            <strong>🖥 Local LLM</strong>
          </div>
          <p>Run a model locally on your machine. Fully private, no internet needed.</p>
          <small>Supports Ollama, LM Studio, and more · Best for privacy</small>
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
          @click="selectFreeProvider(p.id)"
        >
          <div class="bs-provider-header">
            <strong>{{ p.display_name }}</strong>
            <span
              v-if="p.id === 'openrouter'"
              class="bs-badge"
            >⭐ Recommended</span>
          </div>
          <p>{{ p.notes }}</p>
          <small>Model: <code>{{ p.id === selectedProvider ? selectedFreeModelResolved : p.model }}</code> · {{ p.rpm_limit }} RPM</small>
        </div>
      </div>
      <div
        v-if="selectedFreeProviderModelOptions.length"
        class="bs-api-key"
      >
        <label for="free-model-select">Free model:</label>
        <select
          id="free-model-select"
          v-model="selectedFreeModel"
          class="bs-input"
        >
          <option
            v-for="option in selectedFreeProviderModelOptions"
            :key="option.model"
            :value="option.model"
          >
            {{ option.label }}
          </option>
        </select>
      </div>
      <a
        v-if="selectedFreeProviderAuthUrl"
        class="btn-primary bs-auth-link"
        :href="selectedFreeProviderAuthUrl"
        target="_blank"
        rel="noopener"
      >
        Open provider page
      </a>
      <button
        type="button"
        class="btn-secondary bs-manual-toggle"
        :aria-expanded="manualFreeKeyOpen"
        @click="manualFreeKeyOpen = !manualFreeKeyOpen"
      >
        {{ manualFreeKeyOpen ? 'Hide manual key/token' : 'Manual API key/token option' }}
      </button>
      <div
        v-if="manualFreeKeyOpen"
        class="bs-api-key"
      >
        <label>API key/token from the provider page:</label>
        <input
          v-model="freeApiKeyInput"
          type="password"
          placeholder="Enter API key or token..."
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
          :disabled="!selectedProvider || (selectedFreeProviderRequiresKey && !freeApiKeyInput.trim())"
          @click="activateFreeApi"
        >
          Connect provider →
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
        Pick a familiar provider, then authorize it with your API key.
      </p>
      <div class="bs-auth-grid">
        <button
          v-for="provider in paidProviderOptions"
          :key="provider.id"
          type="button"
          :class="['bs-auth-card', { selected: paidProvider === provider.id }]"
          @click="selectPaidProvider(provider.id)"
        >
          <strong>{{ provider.label }}</strong>
          <small>{{ provider.hint }}</small>
        </button>
      </div>
      <a
        v-if="selectedPaidProvider.authUrl"
        class="btn-primary bs-auth-link"
        :href="selectedPaidProvider.authUrl"
        target="_blank"
        rel="noopener"
      >
        Open {{ selectedPaidProvider.shortLabel }} page
      </a>
      <button
        type="button"
        class="btn-secondary bs-manual-toggle"
        :aria-expanded="manualPaidKeyOpen"
        @click="manualPaidKeyOpen = !manualPaidKeyOpen"
      >
        {{ manualPaidKeyOpen ? 'Hide manual API key' : 'Manual API key option' }}
      </button>
      <div
        v-if="manualPaidKeyOpen"
        class="bs-form"
      >
        <label for="paid-api-key-input">API Key:</label>
        <input
          id="paid-api-key-input"
          v-model="paidApiKey"
          type="password"
          :placeholder="selectedPaidProvider?.placeholder ?? 'Provider API key'"
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
          Connect {{ selectedPaidProvider?.shortLabel ?? 'Provider' }} →
        </button>
      </div>
    </div>

    <!-- ── Step 1C: Local LLM setup — Provider selection ── -->
    <div
      v-else-if="step === 1 && selectedTier === 'local'"
      class="bs-card"
    >
      <h2>🖥 Choose Local Provider</h2>
      <p class="bs-desc">
        Which local runtime do you want to use? Each provides an OpenAI-compatible server.
      </p>
      <div class="bs-tiers">
        <div
          :class="['bs-tier', { selected: localRuntime === 'ollama' }]"
          @click="localRuntime = 'ollama'"
        >
          <div class="bs-tier-header">
            <strong>Ollama</strong>
            <span
              v-if="brain.ollamaStatus.running"
              class="bs-badge-free"
            >✅ Running</span>
          </div>
          <p>CLI-based, lightweight. Download models with <code>ollama pull</code>.</p>
          <small>Best for: developers · CLI users · headless servers</small>
        </div>
        <div
          :class="['bs-tier', { selected: localRuntime === 'lm_studio' }]"
          @click="localRuntime = 'lm_studio'"
        >
          <div class="bs-tier-header">
            <strong>LM Studio</strong>
            <span
              v-if="brain.lmStudioStatus?.running"
              class="bs-badge-free"
            >✅ Running</span>
          </div>
          <p>GUI app with model browser. Manage models visually.</p>
          <small>Best for: visual management · model browsing · embedding models</small>
        </div>
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
          :disabled="!localRuntime"
          @click="localRuntime === 'lm_studio' ? (step = 10) : (step = 2)"
        >
          Next →
        </button>
      </div>
    </div>

    <!-- ── Step 1C-2: Local Ollama setup — Hardware analysis ── -->
    <div
      v-else-if="step === 2 && selectedTier === 'local'"
      class="bs-card"
    >
      <h2>🖥 Local LLM Setup — Ollama</h2>
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
          @click="step = 1"
        >
          ← Back
        </button>
        <button
          class="btn-primary"
          :disabled="!brain.systemInfo"
          @click="step = 3"
        >
          Next →
        </button>
      </div>
    </div>

    <!-- ── Step 3: Choose local model ── -->
    <div
      v-else-if="step === 3"
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
          @click="step = 2"
        >
          ← Back
        </button>
        <button
          class="btn-primary"
          :disabled="!selectedModel"
          @click="step = 4"
        >
          Next →
        </button>
      </div>
    </div>

    <!-- ── Step 4: Check / install Ollama ── -->
    <div
      v-else-if="step === 4"
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
          @click="step = 3"
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
          @click="step = 5"
        >
          Next →
        </button>
      </div>
    </div>

    <!-- ── Step 5: Download model ── -->
    <div
      v-else-if="step === 5"
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
          @click="step = 4"
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

    <!-- ── Step 10: LM Studio configuration ── -->
    <div
      v-else-if="step === 10"
      class="bs-card"
    >
      <h2>🖥 Local LLM Setup — LM Studio</h2>
      <p class="bs-desc">
        Configure your LM Studio local server connection.
      </p>
      <div class="bs-form">
        <label for="lms-base-url">Base URL:</label>
        <input
          id="lms-base-url"
          v-model="lmStudioBaseUrl"
          type="url"
          placeholder="http://127.0.0.1:1234"
          class="bs-input"
        >
        <label for="lms-api-key">API token (optional):</label>
        <input
          id="lms-api-key"
          v-model="lmStudioApiKey"
          type="password"
          placeholder="Optional"
          class="bs-input"
        >
      </div>
      <div :class="['bs-status-indicator', brain.lmStudioStatus?.running ? 'ok' : 'error']">
        {{ brain.lmStudioStatus?.running ? `✅ LM Studio is running (${brain.lmStudioStatus.model_count} models)` : '❌ LM Studio is not running — start its local server' }}
      </div>
      <button
        class="btn-secondary btn-sm"
        @click="refreshLmStudioCheck"
      >
        🔄 Check connection
      </button>
      <div class="bs-form">
        <label for="lms-model">Chat model:</label>
        <input
          id="lms-model"
          v-model="lmStudioModel"
          type="text"
          placeholder="gemma-4-12b-it"
          class="bs-input"
        >
        <label for="lms-embed-model">Embedding model (optional):</label>
        <input
          id="lms-embed-model"
          v-model="lmStudioEmbeddingModel"
          type="text"
          placeholder="qwen3-embedding-0.6b"
          class="bs-input"
        >
      </div>
      <div class="bs-nav">
        <button
          class="btn-secondary"
          @click="step = 1"
        >
          ← Back
        </button>
        <button
          class="btn-primary"
          :disabled="!brain.lmStudioStatus?.running || !lmStudioModel"
          @click="finishLmStudio"
        >
          Activate →
        </button>
      </div>
    </div>

    <!-- ── Step 6 (or done): Brain connected ── -->
    <div
      v-else-if="step === 6 || step === 99"
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
import {
  NVIDIA_FREE_MODELS,
  OPENROUTER_FREE_MODELS,
  POLLINATIONS_MODELS,
  useBrainStore,
  type BrowserAuthModelOption,
} from '../stores/brain';

const emit = defineEmits<{ (e: 'done'): void }>();

const brain = useBrainStore();
const step = ref(0);
const selectedTier = ref<'free' | 'paid' | 'local'>('free');
const localRuntime = ref<'ollama' | 'lm_studio'>('ollama');
const selectedModel = ref('');
const selectedProvider = ref('openrouter');
const selectedFreeModel = ref(OPENROUTER_FREE_MODELS[0]?.model ?? '');
const freeApiKeyInput = ref('');
const manualFreeKeyOpen = ref(false);
const pullDone = ref(false);

// Paid API fields
const paidProvider = ref('openai');
const paidApiKey = ref('');
const paidModel = ref('gpt-4o-mini');
const paidBaseUrl = ref('https://api.openai.com');
const manualPaidKeyOpen = ref(false);

const paidProviderOptions = [
  {
    id: 'openai',
    label: 'Authorize with ChatGPT',
    shortLabel: 'ChatGPT',
    hint: 'OpenAI key, GPT models',
    model: 'gpt-4o-mini',
    baseUrl: 'https://api.openai.com',
    placeholder: 'OpenAI API key',
    authUrl: 'https://platform.openai.com/api-keys',
  },
  {
    id: 'gemini',
    label: 'Authorize with Gemini',
    shortLabel: 'Gemini',
    hint: 'Google AI Studio key',
    model: 'gemini-3-flash-preview',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai',
    placeholder: 'Google AI Studio API key',
    authUrl: 'https://aistudio.google.com/app/apikey',
  },
  {
    id: 'anthropic',
    label: 'Authorize with Claude',
    shortLabel: 'Claude',
    hint: 'Anthropic key',
    model: 'claude-sonnet-4-20250514',
    baseUrl: 'https://api.anthropic.com',
    placeholder: 'Anthropic API key',
    authUrl: 'https://console.anthropic.com/settings/keys',
  },
  {
    id: 'custom',
    label: 'Custom endpoint',
    shortLabel: 'Custom',
    hint: 'Any OpenAI-compatible API',
    model: '',
    baseUrl: '',
    placeholder: 'Provider API key',
    authUrl: '',
  },
] as const;

// LM Studio fields
const lmStudioBaseUrl = ref('http://127.0.0.1:1234');
const lmStudioApiKey = ref('');
const lmStudioModel = ref('');
const lmStudioEmbeddingModel = ref('');

const stepLabels = computed(() => {
  if (selectedTier.value === 'free') return ['Choose', 'Provider', 'Done'];
  if (selectedTier.value === 'paid') return ['Choose', 'Authorize', 'Done'];
  if (localRuntime.value === 'lm_studio') return ['Choose', 'Provider', 'Configure', 'Done'];
  return ['Choose', 'Provider', 'Hardware', 'Model', 'Ollama', 'Download', 'Done'];
});

const modelAlreadyInstalled = computed(() =>
  brain.installedModels.some((m) => m.name === selectedModel.value),
);

const selectedProviderName = computed(() =>
  brain.freeProviders.find((p) => p.id === selectedProvider.value)?.display_name ?? selectedProvider.value,
);

const selectedFreeProvider = computed(() =>
  brain.freeProviders.find((p) => p.id === selectedProvider.value) ?? null,
);

const selectedFreeProviderRequiresKey = computed(() => selectedFreeProvider.value?.requires_api_key ?? true);

const selectedFreeProviderAuthUrl = computed(() => freeProviderAuthUrl(selectedProvider.value));

const selectedFreeProviderModelOptions = computed<BrowserAuthModelOption[]>(() => modelOptionsForFreeProvider(selectedProvider.value));

const selectedFreeModelResolved = computed(() => selectedFreeModel.value || selectedFreeProvider.value?.model || '');

const paidBaseUrlResolved = computed(() => {
  switch (paidProvider.value) {
    case 'openai': return 'https://api.openai.com';
    case 'gemini': return 'https://generativelanguage.googleapis.com/v1beta/openai';
    case 'anthropic': return 'https://api.anthropic.com';
    default: return paidBaseUrl.value;
  }
});

const selectedPaidProvider = computed(() =>
  paidProviderOptions.find((provider) => provider.id === paidProvider.value) ?? paidProviderOptions[0],
);

function selectPaidProvider(providerId: string) {
  paidProvider.value = providerId;
  const provider = paidProviderOptions.find((item) => item.id === providerId);
  if (!provider) return;
  paidModel.value = provider.model || paidModel.value;
  paidBaseUrl.value = provider.baseUrl;
  paidApiKey.value = '';
  manualPaidKeyOpen.value = provider.id === 'custom';
}

function selectFreeProvider(providerId: string) {
  selectedProvider.value = providerId;
  freeApiKeyInput.value = '';
  manualFreeKeyOpen.value = false;
  selectedFreeModel.value = modelOptionsForFreeProvider(providerId)[0]?.model
    ?? brain.freeProviders.find((provider) => provider.id === providerId)?.model
    ?? '';
}

function modelOptionsForFreeProvider(providerId: string): BrowserAuthModelOption[] {
  if (providerId === 'openrouter') return OPENROUTER_FREE_MODELS;
  if (providerId === 'nvidia-nim') return NVIDIA_FREE_MODELS;
  if (providerId === 'pollinations') return POLLINATIONS_MODELS;
  return [];
}

function freeProviderAuthUrl(providerId: string): string | null {
  switch (providerId) {
    case 'openrouter': return 'https://openrouter.ai/keys';
    case 'gemini': return 'https://aistudio.google.com/app/apikey';
    case 'nvidia-nim': return 'https://build.nvidia.com/explore/discover';
    case 'pollinations': return 'https://enter.pollinations.ai/';
    case 'groq': return 'https://console.groq.com/keys';
    case 'cerebras': return 'https://cloud.cerebras.ai/platform/';
    case 'mistral': return 'https://console.mistral.ai/api-keys';
    case 'github-models': return 'https://github.com/settings/tokens';
    case 'siliconflow': return 'https://cloud.siliconflow.cn/account/ak';
    default: return null;
  }
}

function formatRam(mb: number): string {
  return mb >= 1024 ? `${(mb / 1024).toFixed(0)} GB` : `${mb} MB`;
}

function goToTierStep() {
  step.value = 1;
}

async function activateFreeApi() {
  const model = selectedFreeModelResolved.value || null;
  if (model) brain.setFallbackProviderModel(selectedProvider.value, model);
  try {
    await brain.setBrainMode({
      mode: 'free_api',
      provider_id: selectedProvider.value,
      api_key: freeApiKeyInput.value || null,
      model,
    });
  } catch {
    // Tauri unavailable — set locally
    brain.brainMode = {
      mode: 'free_api',
      provider_id: selectedProvider.value,
      api_key: freeApiKeyInput.value || null,
      model,
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
    step.value = 6;
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
  step.value = 6;
}

async function refreshLmStudioCheck() {
  await brain.checkLmStudioStatus(
    lmStudioBaseUrl.value,
    lmStudioApiKey.value || null,
  );
}

async function finishLmStudio() {
  const mode = {
    mode: 'local_lm_studio' as const,
    model: lmStudioModel.value,
    base_url: lmStudioBaseUrl.value,
    api_key: lmStudioApiKey.value || null,
    embedding_model: lmStudioEmbeddingModel.value || null,
  };
  try {
    await brain.setBrainMode(mode);
  } catch {
    brain.brainMode = mode;
  }
  step.value = 99;
}

onMounted(async () => {
  await brain.initialise();
  if (brain.topRecommendation) {
    selectedModel.value = brain.topRecommendation.model_tag;
  }
  if (brain.freeProviders.length > 0) {
    selectedProvider.value = brain.freeProviders.some((provider) => provider.id === 'openrouter')
      ? 'openrouter'
      : brain.freeProviders[0].id;
    selectedFreeModel.value = modelOptionsForFreeProvider(selectedProvider.value)[0]?.model
      ?? brain.freeProviders.find((provider) => provider.id === selectedProvider.value)?.model
      ?? '';
  }
  if (brain.hasBrain) {
    step.value = 99;
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
.bs-auth-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 0.5rem; }
.bs-auth-card { display: grid; gap: 0.25rem; min-height: 4.4rem; padding: 0.75rem 0.85rem; border: 2px solid var(--ts-border); border-radius: var(--ts-radius-md); color: var(--ts-text-primary); text-align: left; background: var(--ts-bg-base); cursor: pointer; transition: border-color var(--ts-transition-fast), background var(--ts-transition-fast); }
.bs-auth-card:hover { border-color: var(--ts-border-medium); }
.bs-auth-card.selected { border-color: var(--ts-success-dim); background: var(--ts-bg-selected); }
.bs-auth-card small { color: var(--ts-text-muted); line-height: 1.35; }
.bs-form { display: flex; flex-direction: column; gap: 0.5rem; }
.bs-form label { font-size: 0.85rem; color: var(--ts-text-secondary); }
.bs-api-key { display: flex; flex-direction: column; gap: 0.3rem; }
.bs-api-key label { font-size: 0.8rem; color: var(--ts-text-secondary); }
.bs-auth-link { display: inline-flex; justify-content: center; text-decoration: none; }
.bs-manual-toggle { align-self: flex-start; }
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
