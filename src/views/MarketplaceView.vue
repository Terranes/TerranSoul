<template>
  <div class="marketplace-view">
    <header class="mp-header">
      <h2>🏪 Agent Marketplace</h2>
      <div class="mp-header-actions">
        <button class="btn-secondary" @click="refreshAll" :disabled="isLoading || !tauriAvailable">
          {{ isLoading ? 'Loading…' : '🔄 Refresh' }}
        </button>
      </div>
    </header>

    <p v-if="packageStore.error && tauriAvailable" class="mp-error">{{ packageStore.error }}</p>

    <!-- Tabs -->
    <nav class="mp-tabs">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        :class="['mp-tab', { active: activeTab === tab.id }]"
        @click="activeTab = tab.id"
      >{{ tab.icon }} {{ tab.label }}</button>
    </nav>

    <!-- ── Browse tab ── -->
    <div v-if="activeTab === 'browse'" class="mp-panel">
      <!-- No Tauri: show inline Tauri notification banner -->
      <template v-if="!tauriAvailable">
        <div class="tauri-banner">
          <!-- Header row -->
          <div class="tauri-banner-main">
            <span class="tauri-banner-icon">⚠️</span>
            <div class="tauri-banner-text">
              <strong>Tauri Desktop Backend Unavailable</strong>
              <span class="tauri-banner-sub">
                {{ hostingContext }}
                — Agents, local Ollama, and device pairing require the desktop app.
              </span>
            </div>
          </div>

          <!-- Brain status -->
          <div v-if="brainStore.hasBrain" class="tauri-brain-row">
            <span class="tauri-brain-dot" />
            <span v-if="brainStore.isFreeApiMode">☁️ Free Cloud LLM active — <strong>{{ activeProviderName }}</strong></span>
            <span v-else-if="brainStore.brainMode?.mode === 'paid_api'">💳 Paid API active — <strong>{{ brainStore.brainMode.model }}</strong></span>
            <span class="tauri-brain-badge">✅ Ready to chat</span>
          </div>

          <!-- LLM configuration section -->
          <div class="llm-config">
            <div class="llm-config-header" @click="showLlmConfig = !showLlmConfig">
              <span>🔧</span>
              <strong>Configure LLM</strong>
              <span class="llm-config-hint">Change your AI model {{ showLlmConfig ? '▾' : '▸' }}</span>
            </div>

            <div v-if="showLlmConfig" class="llm-config-body">
              <!-- Tab bar: Free / Paid -->
              <div class="llm-tier-tabs">
                <button :class="['llm-tier-tab', { active: llmTier === 'free' }]" @click="llmTier = 'free'">☁️ Free Cloud</button>
                <button :class="['llm-tier-tab', { active: llmTier === 'paid' }]" @click="llmTier = 'paid'">💳 Paid API</button>
              </div>

              <!-- Free provider selection -->
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

              <!-- Paid API configuration -->
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

              <!-- Confirmation after switching -->
              <div v-if="llmConfirmation" class="llm-confirmation">
                <span class="llm-confirm-icon">✅</span>
                <div>
                  <strong>{{ llmConfirmation.name }}</strong> is now active.
                  <span v-if="llmConfirmation.url" class="llm-confirm-url">
                    Verify at: <a :href="llmConfirmation.url" target="_blank" rel="noopener">{{ llmConfirmation.url }}</a>
                  </span>
                </div>
              </div>

              <!-- Chat hint -->
              <p class="llm-chat-hint">
                💬 <strong>Tip:</strong> You can also ask TerranSoul in chat to change the model —
                e.g. <em>"Switch to Groq"</em> or <em>"Use my OpenAI API key"</em>.
              </p>
            </div>
          </div>

          <!-- Collapsible details -->
          <button class="tauri-details-toggle" @click="showDetails = !showDetails">
            {{ showDetails ? '▾ Hide details' : '▸ Show details — why & how to fix' }}
          </button>

          <div v-if="showDetails" class="tauri-details">
            <div class="tauri-section">
              <h4>Why am I seeing this?</h4>
              <p>
                TerranSoul uses <a href="https://v2.tauri.app" target="_blank" rel="noopener">Tauri</a>,
                a Rust-based desktop runtime.
                When running as a web app {{ isVercel ? 'on Vercel' : 'in the browser' }},
                the native backend isn't available.
                A free cloud LLM ({{ activeProviderName || 'Pollinations AI' }}) was auto-configured
                so you can still chat.
              </p>
            </div>

            <div class="tauri-section">
              <h4>What works {{ isVercel ? 'on Vercel' : 'in browser mode' }}?</h4>
              <ul class="tauri-feature-list">
                <li class="avail">✅ Chat with free cloud LLM</li>
                <li class="avail">✅ 3D character &amp; animations</li>
                <li class="avail">✅ Model / background selection</li>
                <li class="unavail">❌ Agent Marketplace (install / manage agents)</li>
                <li class="unavail">❌ Local Ollama models</li>
                <li class="unavail">❌ Long-term memory persistence</li>
                <li class="unavail">❌ Device pairing &amp; sync</li>
              </ul>
            </div>

            <div v-if="isVercel" class="tauri-section">
              <h4>Deploying on Vercel (UAT)</h4>
              <p>
                Vercel serves only the static frontend — it cannot run Tauri's Rust backend.
                This is expected for UAT testing of the web UI. To get full functionality:
              </p>
              <ol class="tauri-steps">
                <li>
                  <strong>For full desktop features:</strong> build the Tauri app locally with
                  <code>npm run tauri build</code> or <code>npm run tauri dev</code>.
                </li>
                <li>
                  <strong>For Vercel UAT:</strong> the web-only mode auto-configures a free cloud LLM.
                  No additional Vercel config is needed — it works out of the box.
                </li>
                <li>
                  <strong>Custom provider (optional):</strong> set
                  <code>VITE_DEFAULT_PROVIDER</code> in Vercel project settings to override the
                  default free provider (e.g. <code>groq</code>), and
                  <code>VITE_FREE_API_KEY</code> for providers that require an API key.
                </li>
              </ol>
            </div>

            <div v-else class="tauri-section">
              <h4>Getting the full experience</h4>
              <p>
                Download the TerranSoul desktop app or run
                <code>npm run tauri dev</code> locally to access all features including the
                agent marketplace, local Ollama models, and device pairing.
              </p>
            </div>
          </div>
        </div>
      </template>

      <!-- Desktop mode: full marketplace -->
      <template v-else>
        <!-- LLM Configuration section (also available on desktop) -->
        <div class="llm-config llm-config-desktop">
          <div class="llm-config-header" @click="showLlmConfig = !showLlmConfig">
            <span>🧠</span>
            <strong>Configure LLM</strong>
            <span v-if="brainStore.hasBrain" class="llm-active-badge">
              {{ activeBrainBadge }}
            </span>
            <span class="llm-config-hint">{{ showLlmConfig ? '▾' : '▸' }}</span>
          </div>

          <div v-if="showLlmConfig" class="llm-config-body">
            <!-- Tab bar: Free / Paid / Local -->
            <div class="llm-tier-tabs">
              <button :class="['llm-tier-tab', { active: llmTier === 'free' }]" @click="llmTier = 'free'">☁️ Free Cloud</button>
              <button :class="['llm-tier-tab', { active: llmTier === 'paid' }]" @click="llmTier = 'paid'">💳 Paid API</button>
              <button :class="['llm-tier-tab', { active: llmTier === 'local' }]" @click="llmTier = 'local'">🖥 Local Ollama</button>
            </div>

            <!-- Free provider selection -->
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

            <!-- Paid API configuration -->
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

            <!-- Local Ollama configuration -->
            <div v-if="llmTier === 'local'" class="llm-local-form">
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

            <!-- Confirmation after switching -->
            <div v-if="llmConfirmation" class="llm-confirmation">
              <span class="llm-confirm-icon">✅</span>
              <div>
                <strong>{{ llmConfirmation.name }}</strong> is now active.
                <span v-if="llmConfirmation.url" class="llm-confirm-url">
                  Verify at: <a :href="llmConfirmation.url" target="_blank" rel="noopener">{{ llmConfirmation.url }}</a>
                </span>
              </div>
            </div>

            <!-- Chat hint -->
            <p class="llm-chat-hint">
              💬 <strong>Tip:</strong> You can also ask TerranSoul in chat to change the model —
              e.g. <em>"Switch to Groq"</em> or <em>"Use my OpenAI API key"</em>.
            </p>
          </div>
        </div>

        <h3 class="mp-section-title">🤖 Agents</h3>
        <div class="mp-search-row">
          <input
            v-model="searchQuery"
            placeholder="Search agents…"
            class="mp-search"
            aria-label="Search agents"
            @keyup.enter="doSearch"
          />
          <button class="btn-secondary" @click="doSearch">🔍 Search</button>
        </div>

        <p v-if="isLoading" class="mp-status">Loading agents…</p>
        <p v-else-if="displayedAgents.length === 0" class="mp-status">No agents found.</p>

        <div v-else class="mp-grid">
          <div
            v-for="agent in displayedAgents"
            :key="agent.name"
            :class="['mp-card', { 'mp-card-local-llm': agent.kind === 'local_llm' }]"
          >
            <div class="mp-card-header">
              <h3 class="mp-agent-name">
                <span v-if="agent.kind === 'local_llm'" class="mp-kind-icon" title="Local LLM model">🖥</span>
                {{ agentDisplayName(agent) }}
              </h3>
              <span class="mp-version">v{{ agent.version }}</span>
            </div>
            <p class="mp-description">{{ agent.description }}</p>
            <div class="mp-caps">
              <span
                v-for="cap in agent.capabilities"
                :key="cap"
                class="mp-cap-badge"
              >{{ cap }}</span>
              <span v-if="agent.is_top_pick" class="mp-cap-badge mp-cap-rec">⭐ Recommended</span>
              <span v-if="agent.is_cloud" class="mp-cap-badge mp-cap-cloud">☁️ Cloud</span>
              <span v-if="agent.required_ram_mb" class="mp-cap-badge mp-cap-ram">
                {{ formatRam(agent.required_ram_mb) }} RAM
              </span>
            </div>
            <div v-if="agent.homepage" class="mp-homepage">
              <span class="mp-link-label">🔗 {{ agent.homepage }}</span>
            </div>
            <div class="mp-card-actions">
              <template v-if="agent.kind === 'local_llm'">
                <span v-if="isLocalLlmActive(agent)" class="mp-installed-badge">✅ Active brain</span>
                <span v-else-if="isLocalLlmInstalled(agent)" class="mp-installed-badge">✅ Installed</span>
                <button
                  class="btn-primary btn-sm"
                  :disabled="brainStore.isPulling || !brainStore.ollamaStatus.running || isLocalLlmActive(agent)"
                  :aria-label="localLlmActionAriaLabel(agent)"
                  @click="handleLocalLlmAction(agent)"
                >
                  {{ localLlmActionLabel(agent) }}
                </button>
                <p
                  v-if="!brainStore.ollamaStatus.running"
                  class="mp-card-hint"
                  role="alert"
                  aria-live="polite"
                >
                  ⚠️ Ollama is not running — start it with <code>ollama serve</code>.
                </p>
              </template>
              <template v-else-if="isInstalled(agent.name)">
                <span class="mp-installed-badge">✅ Installed</span>
                <button
                  class="btn-secondary btn-sm"
                  @click="handleUpdate(agent)"
                  :disabled="isLoading"
                >⬆ Update</button>
                <button
                  class="btn-danger btn-sm"
                  @click="handleRemove(agent.name)"
                  :disabled="isLoading"
                >🗑 Remove</button>
              </template>
              <template v-else>
                <button
                  class="btn-primary btn-sm"
                  @click="promptInstall(agent)"
                  :disabled="isLoading"
                >⬇ Install</button>
              </template>
            </div>
          </div>
        </div>
      </template>
    </div>

    <!-- ── Installed tab ── -->
    <div v-if="activeTab === 'installed'" class="mp-panel">
      <template v-if="!tauriAvailable">
        <div class="tauri-banner tauri-banner-compact">
          <div class="tauri-banner-main">
            <span class="tauri-banner-icon">📦</span>
            <div class="tauri-banner-text">
              <strong>No Desktop Agents</strong>
              <span class="tauri-banner-sub">
                Agent installation requires the TerranSoul desktop app
                (<code>npm run tauri dev</code>).
                In {{ isVercel ? 'Vercel' : 'browser' }} mode, the free cloud LLM handles conversations.
              </span>
            </div>
          </div>
        </div>
      </template>
      <p v-else-if="packageStore.installedAgents.length === 0" class="mp-status">No agents installed yet.</p>

      <div v-else class="mp-grid">
        <div
          v-for="agent in packageStore.installedAgents"
          :key="agent.name"
          class="mp-card mp-card-installed"
        >
          <div class="mp-card-header">
            <h3 class="mp-agent-name">{{ agent.name }}</h3>
            <span class="mp-version">v{{ agent.version }}</span>
          </div>
          <p class="mp-description">{{ agent.description }}</p>
          <div class="mp-sandbox-status">
            <span class="mp-sandbox-badge" :class="sandboxBadgeClass(agent.name)">
              {{ sandboxLabel(agent.name) }}
            </span>
          </div>
          <div class="mp-card-actions">
            <button
              class="btn-secondary btn-sm"
              @click="viewCapabilities(agent.name)"
            >🔐 Capabilities</button>
            <button
              class="btn-danger btn-sm"
              @click="handleRemove(agent.name)"
              :disabled="isLoading"
            >🗑 Remove</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Consent dialog -->
    <CapabilityConsentDialog
      v-if="consentAgent"
      :agent-name="consentAgent.name"
      :capabilities="consentAgent.capabilities"
      :sensitive-capabilities="consentAgent.sensitiveCapabilities"
      @confirm="confirmInstall"
      @cancel="consentAgent = null"
    />

    <!-- Capabilities detail modal -->
    <div v-if="capDetailAgent" class="mp-modal-backdrop" @click.self="capDetailAgent = null">
      <div class="mp-modal">
        <h3>🔐 {{ capDetailAgent }} — Capabilities</h3>
        <p v-if="sandboxStore.isLoading" class="mp-status">Loading…</p>
        <ul v-else-if="sandboxStore.consents.length > 0" class="mp-cap-list">
          <li
            v-for="c in sandboxStore.consents"
            :key="c.capability"
            class="mp-cap-row"
          >
            <span>{{ c.capability }}</span>
            <span :class="['mp-grant-badge', c.granted ? 'granted' : 'denied']">
              {{ c.granted ? '✅ Granted' : '❌ Denied' }}
            </span>
          </li>
        </ul>
        <p v-else class="mp-status">No capability consents recorded.</p>
        <div class="mp-modal-btns">
          <button class="btn-secondary" @click="capDetailAgent = null">Close</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { usePackageStore } from '../stores/package';
import { useSandboxStore } from '../stores/sandbox';
import { useBrainStore } from '../stores/brain';
import CapabilityConsentDialog from '../components/CapabilityConsentDialog.vue';
import type { AgentSearchResult } from '../types';

const packageStore = usePackageStore();
const sandboxStore = useSandboxStore();
const brainStore = useBrainStore();

/** Whether the Tauri IPC bridge is available. */
const tauriAvailable = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Detect Vercel hosting via known URL patterns. */
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

/** Human-readable badge text for the currently active brain mode. */
const activeBrainBadge = computed(() => {
  const mode = brainStore.brainMode;
  if (!mode) return '';
  if (mode.mode === 'free_api') return '☁️ ' + activeProviderName.value;
  if (mode.mode === 'paid_api') return '💳 ' + (mode as { model?: string }).model;
  return '🖥 Local';
});

const showDetails = ref(false);

// ── LLM configuration state ──────────────────────────────────────────────────
const showLlmConfig = ref(false);
const llmTier = ref<'free' | 'paid' | 'local'>('free');
const llmSelectedProvider = ref(
  brainStore.brainMode?.mode === 'free_api' ? brainStore.brainMode.provider_id : 'pollinations',
);
const llmFreeApiKey = ref('');
const llmConfirmation = ref<{ name: string; url: string } | null>(null);

// Paid API fields
const llmPaidProvider = ref('openai');
const llmPaidApiKey = ref('');
const llmPaidModel = ref('gpt-4o');
const llmPaidBaseUrl = ref('');

// Local Ollama fields
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
  brainStore.brainMode = {
    mode: 'free_api',
    provider_id: providerId,
    api_key: apiKey,
  };
  // Try to persist via Tauri (will fail in browser mode — that's fine)
  brainStore.setBrainMode(brainStore.brainMode).catch(() => { /* expected in browser */ });

  const provider = brainStore.freeProviders.find((fp) => fp.id === providerId);
  llmConfirmation.value = {
    name: provider?.display_name ?? providerId,
    url: provider?.base_url ?? '',
  };
}

function applyPaidProvider() {
  const baseUrl = resolvedPaidBaseUrl();
  brainStore.brainMode = {
    mode: 'paid_api',
    provider: llmPaidProvider.value,
    api_key: llmPaidApiKey.value,
    model: llmPaidModel.value,
    base_url: baseUrl,
  };
  brainStore.setBrainMode(brainStore.brainMode).catch(() => { /* expected in browser */ });

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

const activeTab = ref<'browse' | 'installed'>('browse');
const tabs = [
  { id: 'browse' as const, icon: '🔍', label: 'Browse' },
  { id: 'installed' as const, icon: '📦', label: 'Installed' },
];

const searchQuery = ref('');
const isLoading = computed(() => packageStore.isLoading);

const displayedAgents = computed(() => {
  return packageStore.searchResults;
});

// ── Local-LLM marketplace agent helpers ──────────────────────────────────────

/** Pretty display name for an agent card (strips `ollama:` prefix on local-LLM agents). */
function agentDisplayName(agent: AgentSearchResult): string {
  if (agent.kind === 'local_llm' && agent.name.startsWith('ollama:')) {
    return agent.name.slice('ollama:'.length);
  }
  return agent.name;
}

/** Format a RAM size in MB as `"6.0 GB"` / `"512 MB"`. */
function formatRam(mb: number): string {
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
  return `${mb} MB`;
}

/** True iff the local-LLM agent's underlying model has been pulled into Ollama. */
function isLocalLlmInstalled(agent: AgentSearchResult): boolean {
  if (agent.kind !== 'local_llm' || !agent.model_tag) return false;
  return brainStore.installedModels.some((m) => m.name === agent.model_tag);
}

/** True iff the local-LLM agent is the currently active brain. */
function isLocalLlmActive(agent: AgentSearchResult): boolean {
  if (agent.kind !== 'local_llm' || !agent.model_tag) return false;
  const mode = brainStore.brainMode;
  if (mode?.mode === 'local_ollama' && mode.model === agent.model_tag) return true;
  return brainStore.activeBrain === agent.model_tag;
}

function localLlmActionLabel(agent: AgentSearchResult): string {
  if (brainStore.isPulling) return 'Pulling…';
  if (isLocalLlmActive(agent)) return '✅ Active';
  if (isLocalLlmInstalled(agent)) return '🧠 Activate';
  return '⬇ Install & Activate';
}

/** Verbose action description for assistive tech, including the disabled reason. */
function localLlmActionAriaLabel(agent: AgentSearchResult): string {
  const base = localLlmActionLabel(agent);
  if (isLocalLlmActive(agent)) return `${base} — this model is already the active brain`;
  if (!brainStore.ollamaStatus.running) return `${base} — disabled because Ollama is not running`;
  if (brainStore.isPulling) return `${base} — disabled while another model is being pulled`;
  return base;
}

/** Pull (if needed) and activate a local Ollama model. */
async function handleLocalLlmAction(agent: AgentSearchResult): Promise<void> {
  if (agent.kind !== 'local_llm' || !agent.model_tag) return;
  const tag = agent.model_tag;
  if (!isLocalLlmInstalled(agent)) {
    const ok = await brainStore.pullModel(tag);
    if (!ok) return;
  }
  await brainStore.setActiveBrain(tag);
  await brainStore.setBrainMode({ mode: 'local_ollama', model: tag });
  llmConfirmation.value = {
    name: agentDisplayName(agent),
    url: agent.homepage ?? '',
  };
}

// Consent dialog state
const consentAgent = ref<{
  name: string;
  capabilities: string[];
  sensitiveCapabilities: string[];
} | null>(null);

// Capability detail modal
const capDetailAgent = ref<string | null>(null);

// Sandbox badge state cache
const agentSandboxStatus = ref<Record<string, boolean>>({});

function isInstalled(name: string): boolean {
  return packageStore.installedAgents.some((a) => a.name === name);
}

function sandboxBadgeClass(name: string): string {
  if (agentSandboxStatus.value[name] === true) return 'sandboxed';
  if (agentSandboxStatus.value[name] === false) return 'unrestricted';
  return 'unknown';
}

function sandboxLabel(name: string): string {
  if (agentSandboxStatus.value[name] === true) return '🔒 Sandboxed';
  if (agentSandboxStatus.value[name] === false) return '🔓 Unrestricted';
  return '❓ Unknown';
}

async function refreshAll() {
  if (!tauriAvailable) return;
  await packageStore.searchAgents('');
  await packageStore.fetchInstalledAgents();
  await refreshSandboxStatus();
  // Refresh local-LLM-related brain state so the marketplace can correctly
  // mark which Ollama models are already pulled and which is active.
  await Promise.allSettled([
    brainStore.checkOllamaStatus(),
    brainStore.fetchInstalledModels(),
    brainStore.loadActiveBrain(),
    brainStore.loadBrainMode(),
    brainStore.fetchRecommendations(),
  ]);
}

async function refreshSandboxStatus() {
  for (const agent of packageStore.installedAgents) {
    const caps = await sandboxStore.listCapabilities(agent.name);
    agentSandboxStatus.value[agent.name] = caps.length > 0;
  }
}

async function doSearch() {
  await packageStore.searchAgents(searchQuery.value);
}

function promptInstall(agent: AgentSearchResult) {
  // Determine sensitive capabilities for the consent dialog
  const sensitiveCaps = agent.capabilities.filter((c) =>
    ['filesystem', 'network', 'clipboard', 'process_spawn'].includes(c),
  );
  consentAgent.value = {
    name: agent.name,
    capabilities: agent.capabilities,
    sensitiveCapabilities: sensitiveCaps,
  };
}

async function confirmInstall() {
  if (!consentAgent.value) return;
  const name = consentAgent.value.name;
  const sensitiveCaps = consentAgent.value.sensitiveCapabilities;
  consentAgent.value = null;

  // Grant sensitive capabilities the user consented to
  for (const cap of sensitiveCaps) {
    const capNames = capabilityToSandboxNames(cap);
    for (const capName of capNames) {
      await sandboxStore.grantCapability(name, capName);
    }
  }

  await packageStore.installAgent(name);
  await refreshSandboxStatus();
}

function capabilityToSandboxNames(
  cap: string,
): ('file_read' | 'file_write' | 'clipboard' | 'network' | 'process_spawn')[] {
  const map: Record<string, ('file_read' | 'file_write' | 'clipboard' | 'network' | 'process_spawn')[]> = {
    filesystem: ['file_read', 'file_write'],
    network: ['network'],
    clipboard: ['clipboard'],
    process_spawn: ['process_spawn'],
  };
  return map[cap] ?? [];
}

async function handleUpdate(agent: AgentSearchResult) {
  await packageStore.updateAgent(agent.name);
}

async function handleRemove(name: string) {
  await packageStore.removeAgent(name);
  await sandboxStore.clearCapabilities(name);
  await refreshSandboxStatus();
}

async function viewCapabilities(name: string) {
  capDetailAgent.value = name;
  await sandboxStore.listCapabilities(name);
}

onMounted(async () => {
  // In browser mode, ensure fallback providers are available for the LLM config UI
  if (!tauriAvailable && brainStore.freeProviders.length === 0) {
    brainStore.autoConfigureFreeApi();
  }
  await refreshAll();
});
</script>

<style scoped>
.marketplace-view { display: flex; flex-direction: column; height: 100%; padding: 1rem; gap: 0.75rem; overflow: hidden; }
.mp-header { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 0.5rem; }
.mp-header h2 { margin: 0; font-size: 1.25rem; }
.mp-header-actions { display: flex; gap: 0.5rem; }
.mp-error { padding: 0.5rem 1rem; background: var(--ts-error-bg); color: var(--ts-error); border-radius: 6px; margin: 0; }
.mp-tabs { display: flex; gap: 0.25rem; }
.mp-tab { padding: 0.4rem 1rem; border: none; border-radius: 6px; cursor: pointer; background: var(--ts-bg-surface); color: var(--ts-text-secondary); font-size: var(--ts-text-sm); transition: background var(--ts-transition-fast), color var(--ts-transition-fast); }
.mp-tab:hover { background: var(--ts-bg-hover); color: var(--ts-text-primary); }
.mp-tab.active { background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); }
.mp-panel { flex: 1; display: flex; flex-direction: column; gap: 0.75rem; overflow-y: auto; min-height: 0; }
.mp-search-row { display: flex; gap: 0.5rem; }
.mp-search { flex: 1; padding: 0.4rem 0.75rem; background: var(--ts-bg-input); border: 1px solid var(--ts-border-medium); border-radius: 6px; color: var(--ts-text-primary); outline: none; transition: border-color var(--ts-transition-fast); }
.mp-search:focus { border-color: var(--ts-accent-blue); }
.mp-search::placeholder { color: var(--ts-text-dim); }
.mp-status { color: var(--ts-text-muted); text-align: center; padding: 2rem; }
.mp-grid { display: flex; flex-direction: column; gap: 0.75rem; }
.mp-card { padding: 1rem; background: var(--ts-bg-surface); border-radius: 8px; border-left: 4px solid var(--ts-accent-blue); display: flex; flex-direction: column; gap: 0.5rem; transition: background var(--ts-transition-fast); }
.mp-card:hover { background: var(--ts-bg-elevated); }
.mp-card-installed { border-left-color: var(--ts-success); }
.mp-card-local-llm { border-left-color: var(--ts-accent-purple, var(--ts-accent-blue)); }
.mp-card-hint { font-size: 0.75rem; color: var(--ts-text-muted); margin: 0; }
.mp-kind-icon { margin-right: 0.25rem; }
.mp-cap-rec { background: var(--ts-warning-bg, var(--ts-bg-base)); color: var(--ts-warning, var(--ts-text-secondary)); }
.mp-cap-cloud { background: var(--ts-info-bg, var(--ts-bg-base)); color: var(--ts-info, var(--ts-text-secondary)); }
.mp-cap-ram { background: var(--ts-bg-base); color: var(--ts-text-muted); }
.mp-card-header { display: flex; align-items: baseline; gap: 0.5rem; }
.mp-agent-name { margin: 0; font-size: 1rem; color: var(--ts-text-primary); }
.mp-version { font-size: 0.75rem; color: var(--ts-text-muted); }
.mp-description { margin: 0; color: var(--ts-text-secondary); font-size: 0.85rem; }
.mp-caps { display: flex; gap: 0.3rem; flex-wrap: wrap; }
.mp-cap-badge { font-size: 0.7rem; padding: 0.15rem 0.5rem; background: var(--ts-bg-base); border-radius: 999px; color: var(--ts-text-secondary); }
.mp-homepage { font-size: 0.75rem; color: var(--ts-text-muted); }
.mp-link-label { word-break: break-all; }
.mp-card-actions { display: flex; align-items: center; gap: 0.5rem; margin-top: 0.25rem; }
.mp-installed-badge { font-size: 0.8rem; color: var(--ts-success); margin-right: auto; }
.mp-sandbox-status { display: flex; gap: 0.5rem; }
.mp-sandbox-badge { font-size: 0.75rem; padding: 0.2rem 0.6rem; border-radius: 999px; }
.mp-sandbox-badge.sandboxed { background: var(--ts-success-bg); color: var(--ts-success); }
.mp-sandbox-badge.unrestricted { background: var(--ts-warning-bg); color: var(--ts-warning); }
.mp-sandbox-badge.unknown { background: var(--ts-bg-surface); color: var(--ts-text-muted); }
.btn-primary { padding: 0.4rem 1rem; background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); border: none; border-radius: 6px; cursor: pointer; transition: background var(--ts-transition-fast); }
.btn-primary:hover { background: var(--ts-accent-blue); }
.btn-secondary { padding: 0.4rem 1rem; background: var(--ts-bg-elevated); color: var(--ts-text-primary); border: none; border-radius: 6px; cursor: pointer; transition: background var(--ts-transition-fast); }
.btn-secondary:hover { background: var(--ts-bg-hover); }
.btn-danger { padding: 0.35rem 0.75rem; background: var(--ts-error-bg); color: var(--ts-error); border: none; border-radius: 6px; cursor: pointer; }
.btn-sm { padding: 0.3rem 0.6rem; font-size: 0.8rem; }
.mp-modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; backdrop-filter: blur(4px); }
.mp-modal { background: var(--ts-bg-surface); border: 1px solid var(--ts-border); border-radius: 12px; padding: 1.5rem; width: min(480px, 90vw); display: flex; flex-direction: column; gap: 0.75rem; box-shadow: var(--ts-shadow-lg); }
.mp-cap-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.4rem; }
.mp-cap-row { display: flex; justify-content: space-between; align-items: center; padding: 0.5rem 0.75rem; background: var(--ts-bg-base); border-radius: 6px; font-size: 0.85rem; }
.mp-grant-badge { font-size: 0.75rem; }
.mp-grant-badge.granted { color: var(--ts-success); }
.mp-grant-badge.denied { color: var(--ts-error); }
.mp-modal-btns { display: flex; gap: 0.5rem; justify-content: flex-end; }

/* ── Tauri unavailable banner (inline in marketplace) ── */
.tauri-banner {
  background: linear-gradient(135deg, rgba(251, 191, 36, 0.10), rgba(245, 158, 11, 0.06));
  border: 1px solid rgba(251, 191, 36, 0.25);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  gap: 0;
  overflow: hidden;
}
.tauri-banner-compact { border-color: rgba(100, 116, 139, 0.25); background: rgba(30, 41, 59, 0.6); }

.tauri-banner-main {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.75rem 1rem;
}
.tauri-banner-icon { font-size: 1.1rem; flex-shrink: 0; }
.tauri-banner-text { flex: 1; min-width: 0; }
.tauri-banner-text strong { color: var(--ts-warning); font-size: 0.88rem; }
.tauri-banner-compact .tauri-banner-text strong { color: var(--ts-text-primary); }
.tauri-banner-sub { display: block; color: var(--ts-text-secondary); font-size: 0.78rem; margin-top: 2px; }
.tauri-banner-sub code { background: var(--ts-bg-surface); padding: 1px 4px; border-radius: 3px; font-size: 0.74rem; color: var(--ts-text-primary); }

/* Brain status row */
.tauri-brain-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 6px 1rem;
  background: var(--ts-success-bg);
  border-top: 1px solid rgba(34, 197, 94, 0.12);
  font-size: 0.78rem;
  color: var(--ts-success);
}
.tauri-brain-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--ts-success-dim); animation: pulse-dot 2s ease-in-out infinite; flex-shrink: 0; }
@keyframes pulse-dot { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
.tauri-brain-badge { margin-left: auto; font-size: 0.72rem; color: var(--ts-success); white-space: nowrap; }

/* Details toggle */
.tauri-details-toggle {
  background: none;
  border: none;
  border-top: 1px solid rgba(251, 191, 36, 0.12);
  color: var(--ts-warning);
  font-size: 0.76rem;
  padding: 6px 1rem;
  text-align: left;
  cursor: pointer;
}
.tauri-details-toggle:hover { background: rgba(251, 191, 36, 0.06); }

/* Expandable details */
.tauri-details {
  padding: 0.5rem 1rem 1rem 2.5rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  border-top: 1px solid rgba(251, 191, 36, 0.08);
}
.tauri-section h4 { margin: 0 0 0.25rem; font-size: 0.82rem; color: var(--ts-text-primary); }
.tauri-section p { margin: 0; color: var(--ts-text-secondary); font-size: 0.78rem; line-height: 1.5; }
.tauri-section a { color: var(--ts-accent-blue); text-decoration: none; }
.tauri-section a:hover { text-decoration: underline; }

.tauri-feature-list {
  list-style: none; margin: 0; padding: 0;
  display: grid; grid-template-columns: 1fr 1fr; gap: 2px 1rem;
  font-size: 0.78rem;
}
.tauri-feature-list .avail { color: var(--ts-success); }
.tauri-feature-list .unavail { color: var(--ts-text-secondary); }

.tauri-steps {
  margin: 0.25rem 0 0; padding-left: 1.25rem;
  font-size: 0.78rem; color: #94a3b8;
  display: flex; flex-direction: column; gap: 0.4rem; line-height: 1.5;
}
.tauri-steps code { background: var(--ts-bg-surface); padding: 1px 5px; border-radius: 3px; font-size: 0.74rem; color: var(--ts-text-primary); }
.tauri-steps strong { color: var(--ts-text-primary); }

/* ── LLM configuration section ── */
.llm-config {
  border-top: 1px solid rgba(59, 130, 246, 0.15);
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

/* Tier tabs */
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

/* Provider cards */
.llm-providers { display: flex; flex-direction: column; gap: 0.4rem; }
.llm-provider-card {
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

/* Confirmation */
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
.mp-section-title { font-size: 1rem; color: var(--ts-text-secondary); margin: 0.5rem 0; }
.bs-status-indicator { padding: 0.75rem 1rem; border-radius: 8px; font-weight: 500; font-size: 0.85rem; }
.bs-status-indicator.ok { background: var(--ts-success-bg); color: var(--ts-success); }
.bs-status-indicator.error { background: var(--ts-error-bg); color: var(--ts-error); }
.llm-local-form { display: flex; flex-direction: column; gap: 0.5rem; }
.llm-local-models { display: flex; flex-direction: column; gap: 0.4rem; max-height: 200px; overflow-y: auto; }
</style>
