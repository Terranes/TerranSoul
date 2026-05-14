<template>
  <div class="bp-shell brain-setup" data-density="cozy">
    <!-- ── Breadcrumb ──────────────────────────────────────────────────────────────── -->
    <div class="bp-crumb">
      <span>TERRANSOUL</span>
      <span class="bp-crumb-sep">›</span>
      <span>COMPANION</span>
      <span class="bp-crumb-sep">›</span>
      <span class="bp-crumb-now">BRAIN SETUP</span>
    </div>

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
    <section
      v-if="step === 0"
      class="bp-module bs-card"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">01</span> Brain Tier
          </div>
          <h2 class="bp-module-title">🧠 Choose how to power your Brain</h2>
        </div>
      </header>
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
        class="bp-btn bp-btn--primary"
        :disabled="!selectedTier"
        @click="goToTierStep"
      >
        Next →
      </button>
    </section>

    <!-- ── Step 1A: Free API setup (auto-select provider) ── -->
    <section
      v-else-if="step === 1 && selectedTier === 'free'"
      class="bp-module bs-card"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">02</span> Configure
          </div>
          <h2 class="bp-module-title">☁️ Free Cloud API</h2>
        </div>
      </header>
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
        class="bp-btn bp-btn--primary bs-auth-link"
        :href="selectedFreeProviderAuthUrl"
        target="_blank"
        rel="noopener"
      >
        Open provider page
      </a>
      <button
        type="button"
        class="bp-btn bp-btn--ghost bs-manual-toggle"
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
          class="bp-btn bp-btn--ghost"
          @click="step = 0"
        >
          ← Back
        </button>
        <button
          class="bp-btn bp-btn--primary"
          :disabled="!selectedProvider || (selectedFreeProviderRequiresKey && !freeApiKeyInput.trim())"
          @click="activateFreeApi"
        >
          Connect provider →
        </button>
      </div>
    </section>

    <!-- ── Step 1B: Paid API setup ── -->
    <section
      v-else-if="step === 1 && selectedTier === 'paid'"
      class="bp-module bs-card"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">02</span> Configure
          </div>
          <h2 class="bp-module-title">💳 Paid Cloud API</h2>
        </div>
      </header>
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
        class="bp-btn bp-btn--primary bs-auth-link"
        :href="selectedPaidProvider.authUrl"
        target="_blank"
        rel="noopener"
      >
        Open {{ selectedPaidProvider.shortLabel }} page
      </a>
      <button
        type="button"
        class="bp-btn bp-btn--ghost bs-manual-toggle"
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
          class="bp-btn bp-btn--ghost"
          @click="step = 0"
        >
          ← Back
        </button>
        <button
          class="bp-btn bp-btn--primary"
          :disabled="!paidApiKey || !paidModel"
          @click="activatePaidApi"
        >
          Connect {{ selectedPaidProvider?.shortLabel ?? 'Provider' }} →
        </button>
      </div>
    </section>

    <!-- ── Step 1C: Local LLM setup — Provider selection ── -->
    <section
      v-else-if="step === 1 && selectedTier === 'local'"
      class="bp-module bs-card"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">02</span> Provider
          </div>
          <h2 class="bp-module-title">🖥 Choose Local Provider</h2>
        </div>
      </header>
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
          class="bp-btn bp-btn--ghost"
          @click="step = 0"
        >
          ← Back
        </button>
        <button
          class="bp-btn bp-btn--primary"
          :disabled="!localRuntime"
          @click="localRuntime === 'lm_studio' ? (step = 10) : (step = 2)"
        >
          Next →
        </button>
      </div>
    </section>

    <!-- ── Step 1C-2: Local Ollama setup — Hardware analysis ── -->
    <section
      v-else-if="step === 2 && selectedTier === 'local'"
      class="bp-module bs-card"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">03</span> Hardware
          </div>
          <h2 class="bp-module-title">🖥 Local LLM Setup — Ollama</h2>
        </div>
      </header>
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
          class="bp-btn bp-btn--ghost"
          @click="step = 1"
        >
          ← Back
        </button>
        <button
          class="bp-btn bp-btn--primary"
          :disabled="!brain.systemInfo"
          @click="step = 3"
        >
          Next →
        </button>
      </div>
    </section>

    <!-- ── Step 3: Choose local model ── -->
    <section
      v-else-if="step === 3"
      class="bp-module bs-card"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">04</span> Model
          </div>
          <h2 class="bp-module-title">Choose your Brain</h2>
        </div>
      </header>
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
          class="bp-btn bp-btn--ghost"
          @click="step = 2"
        >
          ← Back
        </button>
        <button
          class="bp-btn bp-btn--primary"
          :disabled="!selectedModel"
          @click="step = 4"
        >
          Next →
        </button>
      </div>
    </section>

    <!-- ── Step 4: Check / install Ollama ── -->
    <BrainSetupOllamaStep
      v-else-if="step === 4"
      @prev="step = 3"
      @next="step = 5"
    />

    <!-- ── Step 5: Download model ── -->
    <section
      v-else-if="step === 5"
      class="bp-module bs-card"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">06</span> Download
          </div>
          <h2 class="bp-module-title">Download {{ selectedModel }}</h2>
        </div>
      </header>
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
          class="bp-btn bp-btn--ghost"
          @click="step = 4"
        >
          ← Back
        </button>
        <button
          v-if="!modelAlreadyInstalled && !brain.isPulling"
          class="bp-btn bp-btn--primary"
          @click="doPull"
        >
          ⬇ Download model
        </button>
        <button
          v-if="modelAlreadyInstalled || pullDone"
          class="bp-btn bp-btn--primary"
          @click="finishLocal"
        >
          Next →
        </button>
      </div>
    </section>

    <!-- ── Step 10: LM Studio configuration ── -->
    <BrainSetupLmStudioStep
      v-else-if="step === 10"
      @back="step = 1"
      @done="step = 99"
    />

    <!-- ── Step 6 (or done): Brain connected ── -->
    <section
      v-else-if="step === 6 || step === 99"
      class="bp-module bs-card bs-done"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">✓</span> Complete
          </div>
          <h2 class="bp-module-title">Brain connected!</h2>
        </div>
      </header>
      <div class="bs-done-icon">
        🎉
      </div>
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
        class="bp-btn bp-btn--primary"
        @click="emit('done')"
      >
        Start chatting →
      </button>
    </section>
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
import BrainSetupOllamaStep from './BrainSetupOllamaStep.vue';
import BrainSetupLmStudioStep from './BrainSetupLmStudioStep.vue';

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

// LM Studio configuration is handled by BrainSetupLmStudioStep.vue

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

<style src="./BrainSetupView.css" scoped />
