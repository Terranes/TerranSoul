<template>
  <div class="marketplace-view">
    <header class="mp-header">
      <h2>🧠 Brain Configuration</h2>
      <div class="mp-header-actions">
        <button class="btn-secondary" @click="refreshBrain" :disabled="brainStore.isLoading">
          {{ brainStore.isLoading ? 'Loading…' : '🔄 Refresh' }}
        </button>
      </div>
    </header>

    <template v-if="!tauriAvailable">
      <div class="tauri-banner">
        <div class="tauri-banner-main">
          <span class="tauri-banner-icon">⚠️</span>
          <div class="tauri-banner-text">
            <strong>Tauri Desktop Backend Unavailable</strong>
            <span class="tauri-banner-sub">
              {{ hostingContext }} — local Ollama, persistent memory, and device pairing require the desktop app.
            </span>
          </div>
        </div>
      </div>
    </template>

    <div class="llm-config" :class="{ 'llm-config-desktop': tauriAvailable }">
      <div class="llm-config-header" @click="showLlmConfig = !showLlmConfig">
        <span>🔧</span>
        <strong>Configure LLM</strong>
        <span v-if="brainStore.hasBrain" class="llm-active-badge">{{ activeBrainBadge }}</span>
        <span class="llm-config-hint">{{ showLlmConfig ? '▾' : '▸' }}</span>
      </div>

      <div v-if="showLlmConfig" class="llm-config-body">
        <div class="llm-tier-tabs">
          <button :class="['llm-tier-tab', { active: llmTier === 'free' }]" @click="llmTier = 'free'">☁️ Free Cloud</button>
          <button :class="['llm-tier-tab', { active: llmTier === 'paid' }]" @click="llmTier = 'paid'">💳 Paid API</button>
          <button
            v-if="tauriAvailable"
            :class="['llm-tier-tab', { active: llmTier === 'local' }]"
            @click="llmTier = 'local'"
          >🖥 Local Ollama</button>
        </div>

        <div v-if="llmTier === 'free'" class="llm-providers">
          <div
            v-for="p in brainStore.freeProviders"
            :key="p.id"
            :class="['llm-provider-card', { active: llmSelectedProvider === p.id }]"
            @click="llmSelectedProvider = p.id"
          >
            <div class="llm-provider-row">
              <strong>{{ p.display_name }}</strong>
              <span v-if="p.id === currentFreeProviderId" class="llm-current-badge">current</span>
              <span v-if="p.id === 'pollinations'" class="llm-rec-badge">⭐ no key needed</span>
            </div>
            <small>{{ p.notes }}</small>
            <small class="llm-provider-model">Model: <code>{{ p.model }}</code> · {{ p.rpm_limit }} RPM{{ p.requires_api_key ? ' · API key required' : '' }}</small>
          </div>

          <div v-if="selectedFreeProviderNeedsKey" class="llm-field">
            <label>API Key:</label>
            <input v-model="llmFreeApiKey" type="password" placeholder="Enter API key…" class="llm-input" />
          </div>

          <button
            class="btn-primary btn-sm llm-apply-btn"
            :disabled="!llmSelectedProvider || (selectedFreeProviderNeedsKey && !llmFreeApiKey)"
            @click="applyFreeProvider"
          >
            Apply {{ llmSelectedProviderName }}
          </button>
        </div>

        <div v-if="llmTier === 'paid'" class="llm-paid-form">
          <div class="llm-field">
            <label>Provider:</label>
            <select v-model="llmPaidProvider" class="llm-select">
              <option value="openai">OpenAI</option>
              <option value="anthropic">Anthropic</option>
              <option value="custom">Custom endpoint</option>
            </select>
          </div>
          <div class="llm-field">
            <label>API Key:</label>
            <input v-model="llmPaidApiKey" type="password" placeholder="sk-…" class="llm-input" />
          </div>
          <div class="llm-field">
            <label>Model:</label>
            <input v-model="llmPaidModel" type="text" placeholder="gpt-4o" class="llm-input" />
          </div>
          <div v-if="llmPaidProvider === 'custom'" class="llm-field">
            <label>Base URL:</label>
            <input v-model="llmPaidBaseUrl" type="url" placeholder="https://api.example.com" class="llm-input" />
          </div>
          <button
            class="btn-primary btn-sm llm-apply-btn"
            :disabled="!llmPaidApiKey || !llmPaidModel"
            @click="applyPaidProvider"
          >
            Apply {{ llmPaidProvider === 'custom' ? 'Custom' : llmPaidProvider }} API
          </button>
        </div>

        <div v-if="llmTier === 'local' && tauriAvailable" class="llm-local-form">
          <div :class="['bs-status-indicator', brainStore.ollamaStatus.running ? 'ok' : 'error']">
            {{ brainStore.ollamaStatus.running ? '✅ Ollama is running' : '❌ Ollama is not running — start it with `ollama serve`' }}
          </div>
          <div v-if="brainStore.recommendations.length" class="llm-local-models">
            <div
              v-for="m in brainStore.recommendations"
              :key="m.model_tag"
              :class="['llm-provider-card', { active: llmLocalModel === m.model_tag }]"
              @click="llmLocalModel = m.model_tag"
            >
              <div class="llm-provider-row">
                <strong>{{ m.display_name }}</strong>
                <span v-if="m.is_top_pick" class="llm-rec-badge">⭐ Recommended</span>
              </div>
              <small>{{ m.description }}</small>
            </div>
          </div>
          <button
            class="btn-primary btn-sm llm-apply-btn"
            :disabled="!brainStore.ollamaStatus.running || !llmLocalModel"
            @click="applyLocalModel"
          >
            Install & Activate {{ llmLocalModel || '…' }}
          </button>
        </div>

        <div v-if="llmConfirmation" class="llm-confirmation">
          <span class="llm-confirm-icon">✅</span>
          <div>
            <strong>{{ llmConfirmation.name }}</strong> is now active.
            <span v-if="llmConfirmation.url" class="llm-confirm-url">
              Verify at: <a :href="llmConfirmation.url" target="_blank" rel="noopener">{{ llmConfirmation.url }}</a>
            </span>
          </div>
        </div>

        <p class="llm-chat-hint">
          💬 <strong>Tip:</strong> You can also ask TerranSoul in chat to change the model —
          e.g. <em>"Switch to Groq"</em> or <em>"Use my OpenAI API key"</em>.
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useBrainStore } from '../stores/brain';

const brainStore = useBrainStore();

const tauriAvailable = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

const isVercel = computed(() => {
  if (typeof window === 'undefined') return false;
  const host = window.location.hostname;
  return host.endsWith('.vercel.app') || host.endsWith('.vercel.sh') || host.includes('.now.sh');
});

const hostingContext = computed(() => {
  if (isVercel.value) return 'Running on Vercel (web-only mode)';
  if (typeof window !== 'undefined' && window.location.protocol === 'file:') return 'Running from a local file';
  return 'Running in browser mode';
});

const activeProviderName = computed(() => {
  const mode = brainStore.brainMode;
  if (!mode || mode.mode !== 'free_api') return '';
  const p = brainStore.freeProviders.find((fp) => fp.id === mode.provider_id);
  return p?.display_name ?? mode.provider_id ?? '';
});

const activeBrainBadge = computed(() => {
  const mode = brainStore.brainMode;
  if (!mode) return '';
  if (mode.mode === 'free_api') return '☁️ ' + activeProviderName.value;
  if (mode.mode === 'paid_api') return '💳 ' + (mode as { model?: string }).model;
  return '🖥 Local';
});

const showLlmConfig = ref(true);
const llmTier = ref<'free' | 'paid' | 'local'>('free');
const llmSelectedProvider = ref(
  brainStore.brainMode?.mode === 'free_api' ? brainStore.brainMode.provider_id : 'pollinations',
);
const llmFreeApiKey = ref('');
const llmConfirmation = ref<{ name: string; url: string } | null>(null);

const llmPaidProvider = ref('openai');
const llmPaidApiKey = ref('');
const llmPaidModel = ref('gpt-4o');
const llmPaidBaseUrl = ref('');

const llmLocalModel = ref(brainStore.topRecommendation?.model_tag ?? '');

const currentFreeProviderId = computed(() =>
  brainStore.brainMode?.mode === 'free_api' ? brainStore.brainMode.provider_id : null,
);

const selectedFreeProviderNeedsKey = computed(() => {
  const p = brainStore.freeProviders.find((fp) => fp.id === llmSelectedProvider.value);
  return p?.requires_api_key ?? false;
});

const llmSelectedProviderName = computed(() => {
  const p = brainStore.freeProviders.find((fp) => fp.id === llmSelectedProvider.value);
  return p?.display_name ?? llmSelectedProvider.value;
});

function resolvedPaidBaseUrl(): string {
  switch (llmPaidProvider.value) {
    case 'openai': return 'https://api.openai.com';
    case 'anthropic': return 'https://api.anthropic.com';
    default: return llmPaidBaseUrl.value;
  }
}

function applyFreeProvider() {
  const providerId = llmSelectedProvider.value;
  const apiKey = llmFreeApiKey.value || null;
  const mode = {
    mode: 'free_api' as const,
    provider_id: providerId,
    api_key: apiKey,
  };
  brainStore.brainMode = mode;
  brainStore.setBrainMode(mode).catch(() => { /* expected in browser */ });

  const provider = brainStore.freeProviders.find((fp) => fp.id === providerId);
  llmConfirmation.value = {
    name: provider?.display_name ?? providerId,
    url: provider?.base_url ?? '',
  };
}

function applyPaidProvider() {
  const baseUrl = resolvedPaidBaseUrl();
  const mode = {
    mode: 'paid_api' as const,
    provider: llmPaidProvider.value,
    api_key: llmPaidApiKey.value,
    model: llmPaidModel.value,
    base_url: baseUrl,
  };
  brainStore.brainMode = mode;
  brainStore.setBrainMode(mode).catch(() => { /* expected in browser */ });

  llmConfirmation.value = {
    name: `${llmPaidProvider.value} / ${llmPaidModel.value}`,
    url: baseUrl,
  };
}

async function applyLocalModel() {
  const model = llmLocalModel.value;
  if (!model) return;
  const installed = brainStore.installedModels.some((m) => m.name === model);
  if (!installed) {
    const ok = await brainStore.pullModel(model);
    if (!ok) return;
  }
  await brainStore.setActiveBrain(model);
  const mode = { mode: 'local_ollama' as const, model };
  brainStore.brainMode = mode;
  brainStore.setBrainMode(mode).catch(() => { /* expected in browser */ });

  const rec = brainStore.recommendations.find((m) => m.model_tag === model);
  llmConfirmation.value = {
    name: rec?.display_name ?? model,
    url: '',
  };
}

async function refreshBrain() {
  if (!tauriAvailable) {
    if (brainStore.freeProviders.length === 0) {
      brainStore.autoConfigureFreeApi();
    }
    return;
  }

  await Promise.allSettled([
    brainStore.fetchFreeProviders(),
    brainStore.loadBrainMode(),
    brainStore.checkOllamaStatus(),
    brainStore.fetchRecommendations(),
    brainStore.fetchInstalledModels(),
  ]);
}

onMounted(async () => {
  await refreshBrain();
});
</script>

<style scoped>
.marketplace-view { display: flex; flex-direction: column; height: 100%; padding: 1rem; gap: 0.75rem; overflow: auto; }
.mp-header { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 0.5rem; }
.mp-header h2 { margin: 0; font-size: 1.25rem; }
.mp-header-actions { display: flex; gap: 0.5rem; }

.btn-primary { padding: 0.4rem 1rem; background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); border: none; border-radius: 6px; cursor: pointer; transition: background var(--ts-transition-fast); }
.btn-primary:hover { background: var(--ts-accent-blue); }
.btn-secondary { padding: 0.4rem 1rem; background: var(--ts-bg-elevated); color: var(--ts-text-primary); border: none; border-radius: 6px; cursor: pointer; transition: background var(--ts-transition-fast); }
.btn-secondary:hover { background: var(--ts-bg-hover); }
.btn-sm { padding: 0.3rem 0.6rem; font-size: 0.8rem; }

.tauri-banner {
  background: linear-gradient(135deg, rgba(251, 191, 36, 0.10), rgba(245, 158, 11, 0.06));
  border: 1px solid rgba(251, 191, 36, 0.25);
  border-radius: 10px;
  overflow: hidden;
}
.tauri-banner-main {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.75rem 1rem;
}
.tauri-banner-icon { font-size: 1.1rem; flex-shrink: 0; }
.tauri-banner-text { flex: 1; min-width: 0; }
.tauri-banner-text strong { color: var(--ts-warning); font-size: 0.88rem; }
.tauri-banner-sub { display: block; color: var(--ts-text-secondary); font-size: 0.78rem; margin-top: 2px; }

.llm-config {
  border: 1px solid rgba(59, 130, 246, 0.15);
  border-radius: 10px;
}
.llm-config-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 8px 1rem;
  cursor: pointer;
  user-select: none;
}
.llm-config-header strong { color: var(--ts-text-primary); font-size: 0.84rem; }
.llm-config-hint { margin-left: auto; font-size: 0.72rem; color: var(--ts-text-muted); }

.llm-config-body {
  padding: 0.5rem 1rem 1rem;
  border-top: 1px solid rgba(59, 130, 246, 0.08);
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.llm-tier-tabs { display: flex; gap: 0.25rem; }
.llm-tier-tab {
  flex: 1;
  padding: 0.35rem 0.5rem;
  border: 1px solid var(--ts-border-medium);
  border-radius: 6px;
  background: transparent;
  color: var(--ts-text-secondary);
  font-size: 0.78rem;
  cursor: pointer;
  text-align: center;
  transition: background var(--ts-transition-fast), color var(--ts-transition-fast);
}
.llm-tier-tab:hover { background: var(--ts-bg-hover); color: var(--ts-text-primary); }
.llm-tier-tab.active { background: var(--ts-bg-surface); color: var(--ts-text-primary); border-color: var(--ts-accent-blue-hover); }

.llm-providers { display: flex; flex-direction: column; gap: 0.4rem; }
.llm-provider-card {
  padding: 0.45rem 0.6rem;
  background: var(--ts-bg-base);
  border: 1px solid var(--ts-border);
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.78rem;
  display: flex;
  flex-direction: column;
  gap: 2px;
  transition: border-color var(--ts-transition-fast), background var(--ts-transition-fast);
}
.llm-provider-card:hover { border-color: var(--ts-border-medium); }
.llm-provider-card.active { border-color: var(--ts-accent-blue-hover); background: rgba(59, 130, 246, 0.06); }
.llm-provider-row { display: flex; align-items: center; gap: 0.4rem; }
.llm-provider-row strong { font-size: 0.82rem; color: var(--ts-text-primary); }
.llm-current-badge { font-size: 0.65rem; background: var(--ts-success-dim); color: var(--ts-text-on-accent); padding: 1px 6px; border-radius: 999px; }
.llm-rec-badge { font-size: 0.65rem; color: var(--ts-warning); }
.llm-provider-card small { color: var(--ts-text-muted); font-size: 0.72rem; }
.llm-provider-model code { background: var(--ts-bg-surface); padding: 0 3px; border-radius: 2px; font-size: 0.70rem; color: var(--ts-text-primary); }

.llm-field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
.llm-input {
  padding: 0.35rem 0.6rem;
  background: var(--ts-bg-base);
  border: 1px solid var(--ts-border-medium);
  border-radius: 5px;
  color: var(--ts-text-primary);
  font-size: 0.8rem;
  outline: none;
}
.llm-input:focus { border-color: var(--ts-accent-blue-hover); }
.llm-select {
  padding: 0.35rem 0.6rem;
  background: var(--ts-bg-base);
  border: 1px solid var(--ts-border-medium);
  border-radius: 5px;
  color: var(--ts-text-primary);
  font-size: 0.8rem;
}

.llm-apply-btn { align-self: flex-end; margin-top: 0.25rem; }

.llm-confirmation {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: rgba(34, 197, 94, 0.08);
  border: 1px solid rgba(34, 197, 94, 0.2);
  border-radius: 6px;
}
.llm-confirm-icon { flex-shrink: 0; }
.llm-confirmation strong { color: var(--ts-success); }
.llm-confirm-url { display: block; margin-top: 2px; font-size: 0.72rem; color: var(--ts-text-secondary); }
.llm-confirm-url a { color: var(--ts-accent-blue); text-decoration: none; }
.llm-confirm-url a:hover { text-decoration: underline; }
.llm-chat-hint {
  font-size: 0.78rem;
  color: var(--ts-text-muted);
  line-height: 1.4;
  padding-top: 0.25rem;
  border-top: 1px solid var(--ts-border-subtle);
}
.llm-chat-hint strong { color: var(--ts-text-secondary); }
.llm-chat-hint em { color: var(--ts-accent-blue); }
.llm-active-badge { font-size: 0.75rem; background: var(--ts-success-bg); color: var(--ts-success); padding: 0.1rem 0.5rem; border-radius: 999px; margin-left: 0.5rem; }

.bs-status-indicator { padding: 0.75rem 1rem; border-radius: 8px; font-weight: 500; font-size: 0.85rem; }
.bs-status-indicator.ok { background: var(--ts-success-bg); color: var(--ts-success); }
.bs-status-indicator.error { background: var(--ts-error-bg); color: var(--ts-error); }
.llm-local-form { display: flex; flex-direction: column; gap: 0.5rem; }
.llm-local-models { display: flex; flex-direction: column; gap: 0.4rem; max-height: 200px; overflow-y: auto; }
</style>
