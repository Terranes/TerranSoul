<template>
  <div
    class="bp-shell marketplace-view"
    data-density="cozy"
  >
    <AppBreadcrumb
      here="MARKETPLACE"
      @navigate="emit('navigate', $event)"
    />
    <section class="bp-module">
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">01</span> Marketplace
          </div>
          <h2 class="bp-module-title">
            🏪 Agent Marketplace
          </h2>
        </div>
        <div class="mp-header-actions">
          <button
            class="bp-btn bp-btn--ghost bp-btn--sm"
            :disabled="isLoading || !tauriAvailable"
            @click="refreshAll"
          >
            {{ isLoading ? 'Loading…' : '🔄 Refresh' }}
          </button>
        </div>
      </header>
    </section>

    <p
      v-if="packageStore.error && tauriAvailable"
      class="mp-error"
    >
      {{ packageStore.error }}
    </p>

    <!-- Tabs -->
    <nav class="mp-tabs">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        :class="['mp-tab', { active: activeTab === tab.id }]"
        @click="selectTab(tab.id)"
      >
        {{ tab.icon }} {{ tab.label }}
      </button>
    </nav>

    <!-- ── Browse tab ── -->
    <div
      v-if="activeTab === 'browse'"
      class="mp-panel"
    >
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
          <div
            v-if="brainStore.hasBrain"
            class="tauri-brain-row"
          >
            <span class="tauri-brain-dot" />
            <span v-if="brainStore.isFreeApiMode">☁️ Free Cloud LLM active — <strong>{{ activeProviderName }}</strong></span>
            <span v-else-if="brainStore.brainMode?.mode === 'paid_api'">💳 Paid API active — <strong>{{ brainStore.brainMode.model }}</strong></span>
            <span class="tauri-brain-badge">✅ Ready to chat</span>
          </div>

          <!-- LLM configuration section -->
          <div class="llm-config">
            <div
              class="llm-config-header"
              @click="showLlmConfig = !showLlmConfig"
            >
              <span>🔧</span>
              <strong>Configure LLM</strong>
              <span class="llm-config-hint">Change your AI model {{ showLlmConfig ? '▾' : '▸' }}</span>
            </div>

            <div
              v-if="showLlmConfig"
              class="llm-config-body"
            >
              <!-- Tab bar: Free / Paid -->
              <div class="llm-tier-tabs">
                <button
                  :class="['llm-tier-tab', { active: llmTier === 'free' }]"
                  @click="llmTier = 'free'"
                >
                  ☁️ Free Cloud
                </button>
                <button
                  :class="['llm-tier-tab', { active: llmTier === 'paid' }]"
                  @click="llmTier = 'paid'"
                >
                  💳 Paid API
                </button>
              </div>

              <!-- Free provider selection -->
              <div
                v-if="llmTier === 'free'"
                class="llm-providers"
              >
                <div
                  v-for="p in brainStore.freeProviders"
                  :key="p.id"
                  :class="['llm-provider-card', { active: llmSelectedProvider === p.id }]"
                  @click="selectFreeProvider(p.id)"
                >
                  <div class="llm-provider-row">
                    <strong>{{ p.display_name }}</strong>
                    <span
                      v-if="p.id === currentFreeProviderId"
                      class="llm-current-badge"
                    >current</span>
                    <span
                      v-if="p.id === 'openrouter'"
                      class="llm-rec-badge"
                    >Recommended</span>
                  </div>
                  <small>{{ p.notes }}</small>
                  <small class="llm-provider-model">Model: <code>{{ p.model }}</code> · {{ p.rpm_limit }} RPM{{ p.requires_api_key ? ' · API key required' : '' }}</small>
                </div>
                <a
                  v-if="selectedFreeProviderAuthUrl"
                  class="btn-primary btn-sm llm-auth-link"
                  :href="selectedFreeProviderAuthUrl"
                  target="_blank"
                  rel="noopener"
                >
                  Open provider page
                </a>
                <button
                  type="button"
                  class="btn-secondary btn-sm llm-manual-toggle"
                  :aria-expanded="llmManualFreeKeyOpen"
                  @click="llmManualFreeKeyOpen = !llmManualFreeKeyOpen"
                >
                  {{ llmManualFreeKeyOpen ? 'Hide manual key/token' : 'Manual API key/token option' }}
                </button>
                <div
                  v-if="selectedFreeProviderNeedsKey && llmManualFreeKeyOpen"
                  class="llm-field"
                >
                  <label>API key/token:</label>
                  <input
                    v-model="llmFreeApiKey"
                    type="password"
                    placeholder="Enter API key or token..."
                    class="llm-input"
                  >
                </div>
                <div
                  v-if="selectedFreeProviderModelOptions.length"
                  class="llm-field"
                >
                  <label>Free model:</label>
                  <select
                    v-model="llmFreeModel"
                    class="llm-select"
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
                <button
                  class="btn-primary btn-sm llm-apply-btn"
                  :disabled="!llmSelectedProvider || (selectedFreeProviderNeedsKey && !llmFreeApiKey)"
                  @click="applyFreeProvider"
                >
                  Apply {{ llmSelectedProviderName }}
                </button>
              </div>

              <!-- Paid API configuration -->
              <div
                v-if="llmTier === 'paid'"
                class="llm-paid-form"
              >
                <div class="llm-auth-provider-grid">
                  <button
                    v-for="provider in paidProviderOptions"
                    :key="provider.id"
                    type="button"
                    :class="['llm-auth-provider-btn', { active: llmPaidProvider === provider.id }]"
                    @click="selectPaidProvider(provider.id)"
                  >
                    <strong>{{ provider.label }}</strong>
                    <small>{{ provider.hint }}</small>
                  </button>
                </div>
                <a
                  v-if="selectedPaidProviderAuthUrl"
                  class="btn-primary btn-sm llm-auth-link"
                  :href="selectedPaidProviderAuthUrl"
                  target="_blank"
                  rel="noopener"
                >
                  Open provider page
                </a>
                <button
                  type="button"
                  class="btn-secondary btn-sm llm-manual-toggle"
                  :aria-expanded="llmManualPaidKeyOpen"
                  @click="llmManualPaidKeyOpen = !llmManualPaidKeyOpen"
                >
                  {{ llmManualPaidKeyOpen ? 'Hide manual API key' : 'Manual API key option' }}
                </button>
                <template v-if="llmManualPaidKeyOpen">
                  <div class="llm-field">
                    <label>API Key:</label>
                    <input
                      v-model="llmPaidApiKey"
                      type="password"
                      placeholder="sk-..."
                      class="llm-input"
                    >
                  </div>
                  <div class="llm-field">
                    <label>Model:</label>
                    <input
                      v-model="llmPaidModel"
                      type="text"
                      placeholder="gpt-4o"
                      class="llm-input"
                    >
                  </div>
                  <div
                    v-if="llmPaidProvider === 'custom'"
                    class="llm-field"
                  >
                    <label>Base URL:</label>
                    <input
                      v-model="llmPaidBaseUrl"
                      type="url"
                      placeholder="https://api.example.com"
                      class="llm-input"
                    >
                  </div>
                </template>
                <button
                  class="btn-primary btn-sm llm-apply-btn"
                  :disabled="!llmPaidApiKey || !llmPaidModel"
                  @click="applyPaidProvider"
                >
                  Apply {{ selectedPaidProviderLabel }}
                </button>
              </div>

              <!-- Confirmation after switching -->
              <div
                v-if="llmConfirmation"
                class="llm-confirmation"
              >
                <span class="llm-confirm-icon">✅</span>
                <div>
                  <strong>{{ llmConfirmation.name }}</strong> is now active.
                  <span
                    v-if="llmConfirmation.url"
                    class="llm-confirm-url"
                  >
                    Verify at: <a
                      :href="llmConfirmation.url"
                      target="_blank"
                      rel="noopener"
                    >{{ llmConfirmation.url }}</a>
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
          <button
            class="tauri-details-toggle"
            @click="showDetails = !showDetails"
          >
            {{ showDetails ? '▾ Hide details' : '▸ Show details — why & how to fix' }}
          </button>

          <div
            v-if="showDetails"
            class="tauri-details"
          >
            <div class="tauri-section">
              <h4>Why am I seeing this?</h4>
              <p>
                TerranSoul uses <a
                  href="https://v2.tauri.app"
                  target="_blank"
                  rel="noopener"
                >Tauri</a>,
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
                <li class="avail">
                  ✅ Chat with free cloud LLM
                </li>
                <li class="avail">
                  ✅ 3D character &amp; animations
                </li>
                <li class="avail">
                  ✅ Model / background selection
                </li>
                <li class="unavail">
                  ❌ Agent Marketplace (install / manage agents)
                </li>
                <li class="unavail">
                  ❌ Local Ollama models
                </li>
                <li class="unavail">
                  ❌ Long-term memory persistence
                </li>
                <li class="unavail">
                  ❌ Device pairing &amp; sync
                </li>
              </ul>
            </div>

            <div
              v-if="isVercel"
              class="tauri-section"
            >
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

            <div
              v-else
              class="tauri-section"
            >
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
        <section class="bp-module">
          <header class="bp-module-head">
            <div class="bp-module-head-left">
              <div class="bp-module-eyebrow">
                <span class="ix">01</span> LLM Configuration
              </div>
            </div>
          </header>
          <div class="llm-config llm-config-desktop">
            <div
              class="llm-config-header"
              @click="showLlmConfig = !showLlmConfig"
            >
              <span>🧠</span>
              <strong>Configure LLM</strong>
              <span
                v-if="brainStore.hasBrain"
                class="llm-active-badge"
              >
                {{ activeBrainBadge }}
              </span>
              <span class="llm-config-hint">{{ showLlmConfig ? '▾' : '▸' }}</span>
            </div>

            <div
              v-if="showLlmConfig"
              class="llm-config-body"
            >
              <!-- Tab bar: Free / Paid / Local LLM -->
              <div class="llm-tier-tabs">
                <button
                  :class="['llm-tier-tab', { active: llmTier === 'free' }]"
                  @click="llmTier = 'free'"
                >
                  ☁️ Free Cloud
                </button>
                <button
                  :class="['llm-tier-tab', { active: llmTier === 'paid' }]"
                  @click="llmTier = 'paid'"
                >
                  💳 Paid API
                </button>
                <button
                  :class="['llm-tier-tab', { active: llmTier === 'local' || llmTier === 'lm_studio' }]"
                  @click="llmTier = llmLocalProvider === 'lm_studio' ? 'lm_studio' : 'local'"
                >
                  🖥 Local LLM
                </button>
              </div>

              <!-- Free provider selection -->
              <div
                v-if="llmTier === 'free'"
                class="llm-providers"
              >
                <div
                  v-for="p in brainStore.freeProviders"
                  :key="p.id"
                  :class="['llm-provider-card', { active: llmSelectedProvider === p.id }]"
                  @click="selectFreeProvider(p.id)"
                >
                  <div class="llm-provider-row">
                    <strong>{{ p.display_name }}</strong>
                    <span
                      v-if="p.id === currentFreeProviderId"
                      class="llm-current-badge"
                    >current</span>
                    <span
                      v-if="p.id === 'openrouter'"
                      class="llm-rec-badge"
                    >Recommended</span>
                  </div>
                  <small>{{ p.notes }}</small>
                  <small class="llm-provider-model">Model: <code>{{ p.model }}</code> · {{ p.rpm_limit }} RPM{{ p.requires_api_key ? ' · API key required' : '' }}</small>
                </div>
                <a
                  v-if="selectedFreeProviderAuthUrl"
                  class="btn-primary btn-sm llm-auth-link"
                  :href="selectedFreeProviderAuthUrl"
                  target="_blank"
                  rel="noopener"
                >
                  Open provider page
                </a>
                <button
                  type="button"
                  class="btn-secondary btn-sm llm-manual-toggle"
                  :aria-expanded="llmManualFreeKeyOpen"
                  @click="llmManualFreeKeyOpen = !llmManualFreeKeyOpen"
                >
                  {{ llmManualFreeKeyOpen ? 'Hide manual key/token' : 'Manual API key/token option' }}
                </button>
                <div
                  v-if="selectedFreeProviderNeedsKey && llmManualFreeKeyOpen"
                  class="llm-field"
                >
                  <label>API key/token:</label>
                  <input
                    v-model="llmFreeApiKey"
                    type="password"
                    placeholder="Enter API key or token..."
                    class="llm-input"
                  >
                </div>
                <div
                  v-if="selectedFreeProviderModelOptions.length"
                  class="llm-field"
                >
                  <label>Free model:</label>
                  <select
                    v-model="llmFreeModel"
                    class="llm-select"
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
                <button
                  class="btn-primary btn-sm llm-apply-btn"
                  :disabled="!llmSelectedProvider || (selectedFreeProviderNeedsKey && !llmFreeApiKey)"
                  @click="applyFreeProvider"
                >
                  Apply {{ llmSelectedProviderName }}
                </button>
              </div>

              <!-- Paid API configuration -->
              <div
                v-if="llmTier === 'paid'"
                class="llm-paid-form"
              >
                <div class="llm-auth-provider-grid">
                  <button
                    v-for="provider in paidProviderOptions"
                    :key="provider.id"
                    type="button"
                    :class="['llm-auth-provider-btn', { active: llmPaidProvider === provider.id }]"
                    @click="selectPaidProvider(provider.id)"
                  >
                    <strong>{{ provider.label }}</strong>
                    <small>{{ provider.hint }}</small>
                  </button>
                </div>
                <a
                  v-if="selectedPaidProviderAuthUrl"
                  class="btn-primary btn-sm llm-auth-link"
                  :href="selectedPaidProviderAuthUrl"
                  target="_blank"
                  rel="noopener"
                >
                  Open provider page
                </a>
                <button
                  type="button"
                  class="btn-secondary btn-sm llm-manual-toggle"
                  :aria-expanded="llmManualPaidKeyOpen"
                  @click="llmManualPaidKeyOpen = !llmManualPaidKeyOpen"
                >
                  {{ llmManualPaidKeyOpen ? 'Hide manual API key' : 'Manual API key option' }}
                </button>
                <template v-if="llmManualPaidKeyOpen">
                  <div class="llm-field">
                    <label>API Key:</label>
                    <input
                      v-model="llmPaidApiKey"
                      type="password"
                      placeholder="sk-..."
                      class="llm-input"
                    >
                  </div>
                  <div class="llm-field">
                    <label>Model:</label>
                    <input
                      v-model="llmPaidModel"
                      type="text"
                      placeholder="gpt-4o"
                      class="llm-input"
                    >
                  </div>
                  <div
                    v-if="llmPaidProvider === 'custom'"
                    class="llm-field"
                  >
                    <label>Base URL:</label>
                    <input
                      v-model="llmPaidBaseUrl"
                      type="url"
                      placeholder="https://api.example.com"
                      class="llm-input"
                    >
                  </div>
                </template>
                <button
                  class="btn-primary btn-sm llm-apply-btn"
                  :disabled="!llmPaidApiKey || !llmPaidModel"
                  @click="applyPaidProvider"
                >
                  Apply {{ selectedPaidProviderLabel }}
                </button>
              </div>

              <!-- Local Ollama configuration -->
              <div
                v-if="llmTier === 'local' || llmTier === 'lm_studio'"
                class="llm-local-form"
              >
                <!-- Provider pill switcher -->
                <div class="llm-provider-pills">
                  <button
                    :class="['llm-provider-pill', { active: llmLocalProvider === 'ollama' }]"
                    @click="llmLocalProvider = 'ollama'; llmTier = 'local'"
                  >
                    Ollama
                  </button>
                  <button
                    :class="['llm-provider-pill', { active: llmLocalProvider === 'lm_studio' }]"
                    @click="llmLocalProvider = 'lm_studio'; llmTier = 'lm_studio'"
                  >
                    LM Studio
                  </button>
                  <button
                    class="llm-provider-pill"
                    disabled
                    title="Coming soon"
                  >
                    HuggingFace 🔜
                  </button>
                </div>

                <!-- Ollama sub-panel -->
                <template v-if="llmLocalProvider === 'ollama'">
                  <div :class="['bs-status-indicator', brainStore.ollamaStatus.running ? 'ok' : 'error']">
                    {{ brainStore.ollamaStatus.running ? '✅ Ollama is running' : '❌ Ollama is not running — start it with `ollama serve`' }}
                  </div>
                  <div
                    v-if="brainStore.recommendations.length"
                    class="llm-local-models"
                  >
                    <div
                      v-for="m in brainStore.recommendations"
                      :key="m.model_tag"
                      :class="['llm-provider-card', { active: llmLocalModel === m.model_tag }]"
                      @click="llmLocalModel = m.model_tag"
                    >
                      <div class="llm-provider-row">
                        <strong>{{ m.display_name }}</strong>
                        <span
                          v-if="m.is_top_pick"
                          class="llm-rec-badge"
                        >⭐ Recommended</span>
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
                </template>

                <!-- LM Studio sub-panel -->
                <template v-if="llmLocalProvider === 'lm_studio'">
                  <div class="llm-field">
                    <label>Base URL:</label>
                    <input
                      v-model="llmLmStudioBaseUrl"
                      type="url"
                      placeholder="http://127.0.0.1:1234"
                      class="llm-input"
                    >
                  </div>
                  <div class="llm-field">
                    <label>API token (optional):</label>
                    <input
                      v-model="llmLmStudioApiKey"
                      type="password"
                      placeholder="Optional"
                      class="llm-input"
                    >
                  </div>
                  <div :class="['bs-status-indicator', brainStore.lmStudioStatus?.running ? 'ok' : 'error']">
                    {{ brainStore.lmStudioStatus?.running ? `✅ LM Studio is running (${brainStore.lmStudioStatus.model_count} models)` : '❌ LM Studio is not running — start its local server' }}
                  </div>
                  <button
                    class="btn-secondary btn-sm"
                    @click="refreshLmStudioRuntime"
                  >
                    Refresh LM Studio
                  </button>
                  <div class="llm-field">
                    <label>Model:</label>
                    <input
                      v-model="llmLmStudioModel"
                      type="text"
                      placeholder="qwen/qwen3-4b or Hugging Face URL"
                      class="llm-input"
                    >
                  </div>
                  <div class="llm-field">
                    <label>Embedding model (optional):</label>
                    <input
                      v-model="llmLmStudioEmbeddingModel"
                      type="text"
                      placeholder="text-embedding-nomic-embed-text-v1.5"
                      class="llm-input"
                    >
                  </div>
                  <div
                    v-if="brainStore.lmStudioModels?.length"
                    class="llm-local-models"
                  >
                    <div
                      v-for="m in brainStore.lmStudioModels"
                      :key="m.key"
                      :class="['llm-provider-card', { active: llmLmStudioModel === m.key }]"
                      @click="llmLmStudioModel = m.key"
                    >
                      <div class="llm-provider-row">
                        <strong>{{ m.display_name || m.key }}</strong>
                        <span
                          v-if="m.loaded_instances.length"
                          class="llm-current-badge"
                        >loaded</span>
                      </div>
                      <small>{{ m.publisher || 'Local model' }} · {{ m.type }} · {{ formatBytes(m.size_bytes) }}</small>
                      <small class="llm-provider-model"><code>{{ m.key }}</code></small>
                    </div>
                  </div>
                  <div
                    v-if="brainStore.lmStudioDownload"
                    class="bs-status-indicator ok"
                  >
                    Download status: {{ brainStore.lmStudioDownload.status }}
                  </div>
                  <div
                    v-if="brainStore.lmStudioError"
                    class="bs-status-indicator error"
                  >
                    {{ brainStore.lmStudioError }}
                  </div>
                  <div class="llm-lmstudio-actions">
                    <button
                      class="btn-secondary btn-sm"
                      :disabled="!llmLmStudioModel"
                      @click="downloadLmStudioModel"
                    >
                      Download
                    </button>
                    <button
                      class="btn-secondary btn-sm"
                      :disabled="!llmLmStudioModel"
                      @click="loadLmStudioModel"
                    >
                      Load
                    </button>
                    <button
                      class="btn-primary btn-sm llm-apply-btn"
                      :disabled="!brainStore.lmStudioStatus?.running || !llmLmStudioModel"
                      @click="applyLmStudioModel"
                    >
                      Activate {{ llmLmStudioModel || '…' }}
                    </button>
                  </div>
                </template>
              </div>

              <!-- Confirmation after switching -->
              <div
                v-if="llmConfirmation"
                class="llm-confirmation"
              >
                <span class="llm-confirm-icon">✅</span>
                <div>
                  <strong>{{ llmConfirmation.name }}</strong> is now active.
                  <span
                    v-if="llmConfirmation.url"
                    class="llm-confirm-url"
                  >
                    Verify at: <a
                      :href="llmConfirmation.url"
                      target="_blank"
                      rel="noopener"
                    >{{ llmConfirmation.url }}</a>
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
        </section>

        <section class="bp-module">
          <header class="bp-module-head">
            <div class="bp-module-head-left">
              <div class="bp-module-eyebrow">
                <span class="ix">02</span> Agent Catalogue
              </div>
            </div>
          </header>
          <div class="mp-toolbar">
            <div class="mp-search-row">
              <span
                class="mp-search-icon"
                aria-hidden="true"
              >🔍</span>
              <input
                v-model="searchQuery"
                placeholder="Search agents by name, description, or capability…"
                class="mp-search"
                aria-label="Search agents"
                @keyup.enter="doSearch"
              >
              <button
                v-if="searchQuery"
                class="mp-search-clear"
                aria-label="Clear search"
                @click="clearSearch"
              >
                ✕
              </button>
            </div>
            <div class="mp-filter-row">
              <label class="mp-filter">
                <span class="mp-filter-label">Kind</span>
                <select
                  v-model="filterKind"
                  class="mp-select"
                  aria-label="Filter by kind"
                >
                  <option value="all">
                    All kinds
                  </option>
                  <option value="package">
                    📦 Package
                  </option>
                  <option value="local_llm">
                    🖥 Local LLM
                  </option>
                  <option value="cloud">
                    ☁️ Cloud
                  </option>
                </select>
              </label>
              <label class="mp-filter">
                <span class="mp-filter-label">Status</span>
                <select
                  v-model="filterStatus"
                  class="mp-select"
                  aria-label="Filter by install status"
                >
                  <option value="all">
                    Any status
                  </option>
                  <option value="installed">
                    ✅ Installed
                  </option>
                  <option value="not_installed">
                    ⬇ Not installed
                  </option>
                </select>
              </label>
              <label class="mp-filter">
                <span class="mp-filter-label">Max RAM</span>
                <select
                  v-model="filterRam"
                  class="mp-select"
                  aria-label="Filter by maximum RAM"
                >
                  <option value="any">
                    Any RAM
                  </option>
                  <option value="4">
                    ≤4 GB
                  </option>
                  <option value="8">
                    ≤8 GB
                  </option>
                  <option value="16">
                    ≤16 GB
                  </option>
                  <option value="32">
                    ≤32 GB
                  </option>
                </select>
              </label>
              <label class="mp-filter">
                <span class="mp-filter-label">Sort</span>
                <select
                  v-model="sortBy"
                  class="mp-select"
                  aria-label="Sort agents"
                >
                  <option value="recommended">
                    ⭐ Recommended
                  </option>
                  <option value="name_asc">
                    Name (A→Z)
                  </option>
                  <option value="name_desc">
                    Name (Z→A)
                  </option>
                  <option value="ram_asc">
                    RAM (low→high)
                  </option>
                  <option value="ram_desc">
                    RAM (high→low)
                  </option>
                </select>
              </label>
              <button
                v-if="hasActiveFilters"
                class="bp-btn bp-btn--ghost bp-btn--sm mp-reset-btn"
                @click="resetFilters"
              >
                Reset filters
              </button>
            </div>
            <p
              v-if="!isLoading"
              class="mp-result-count"
              role="status"
              aria-live="polite"
            >
              Showing <strong>{{ displayedAgents.length }}</strong>
              of {{ packageStore.searchResults.length }} agents
            </p>
          </div>

          <p
            v-if="isLoading"
            class="mp-status"
          >
            ⏳ Loading agents…
          </p>
          <div
            v-else-if="packageStore.searchResults.length === 0"
            class="mp-status mp-empty"
          >
            <p class="mp-empty-title">
              📦 No agents available
            </p>
            <p class="mp-empty-sub">
              The agent registry returned no results.
              Try 🔄 Refresh, or check that the desktop app is running.
            </p>
          </div>
          <div
            v-else-if="displayedAgents.length === 0"
            class="mp-status mp-empty"
          >
            <p class="mp-empty-title">
              🔍 No agents match your filters
            </p>
            <p class="mp-empty-sub">
              Try clearing the search or resetting filters.
            </p>
            <button
              class="bp-btn bp-btn--ghost bp-btn--sm"
              @click="resetFilters"
            >
              Reset filters
            </button>
          </div>

          <div
            v-else
            class="mp-grid"
          >
            <div
              v-for="agent in displayedAgents"
              :key="agent.name"
              :class="['mp-card', { 'mp-card-local-llm': agent.kind === 'local_llm' }]"
            >
              <div class="mp-card-header">
                <h3 class="mp-agent-name">
                  <span
                    v-if="agent.kind === 'local_llm'"
                    class="mp-kind-icon"
                    title="Local LLM model"
                  >🖥</span>
                  {{ agentDisplayName(agent) }}
                </h3>
                <span class="mp-version">v{{ agent.version }}</span>
              </div>
              <p class="mp-description">
                {{ agent.description }}
              </p>
              <div class="mp-caps">
                <span
                  v-for="cap in agent.capabilities"
                  :key="cap"
                  class="mp-cap-badge"
                >{{ cap }}</span>
              </div>
              <div
                v-if="agent.is_top_pick || agent.is_cloud || agent.required_ram_mb"
                class="mp-meta"
              >
                <span
                  v-if="agent.is_top_pick"
                  class="mp-meta-badge mp-meta-rec"
                >⭐ Recommended</span>
                <span
                  v-if="agent.is_cloud"
                  class="mp-meta-badge mp-meta-cloud"
                >☁️ Cloud</span>
                <span
                  v-if="agent.required_ram_mb"
                  class="mp-meta-badge mp-meta-ram"
                >
                  {{ formatRam(agent.required_ram_mb) }} RAM
                </span>
              </div>
              <div
                v-if="agent.homepage"
                class="mp-homepage"
              >
                <a
                  :href="agent.homepage"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="mp-link"
                >
                  🔗 {{ agent.homepage }}
                </a>
              </div>
              <div class="mp-card-actions">
                <template v-if="agent.kind === 'local_llm'">
                  <span
                    v-if="isLocalLlmActive(agent)"
                    class="mp-installed-badge"
                  >✅ Active brain</span>
                  <span
                    v-else-if="isLocalLlmInstalled(agent)"
                    class="mp-installed-badge"
                  >✅ Installed</span>
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
                    :disabled="isLoading"
                    @click="handleUpdate(agent)"
                  >
                    ⬆ Update
                  </button>
                  <button
                    class="btn-danger btn-sm"
                    :disabled="isLoading"
                    @click="handleRemove(agent.name)"
                  >
                    🗑 Remove
                  </button>
                </template>
                <template v-else>
                  <button
                    class="btn-primary btn-sm"
                    :disabled="isLoading"
                    @click="promptInstall(agent)"
                  >
                    ⬇ Install
                  </button>
                </template>
              </div>
            </div>
          </div>
        </section>
      </template>
    </div>

    <!-- ── Installed tab ── -->
    <div
      v-if="activeTab === 'installed'"
      class="mp-panel"
    >
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
      <p
        v-else-if="packageStore.installedAgents.length === 0"
        class="mp-status mp-empty"
      >
        <span class="mp-empty-title">📦 No agents installed yet</span>
        <span class="mp-empty-sub">Install agents from the Browse tab to extend TerranSoul’s capabilities.</span>
        <button
          class="bp-btn bp-btn--primary bp-btn--sm"
          @click="selectTab('browse')"
        >
          🔍 Browse Marketplace
        </button>
      </p>

      <template v-else>
        <div class="mp-toolbar">
          <div class="mp-search-row">
            <span
              class="mp-search-icon"
              aria-hidden="true"
            >🔍</span>
            <input
              v-model="installedSearchQuery"
              placeholder="Filter installed agents…"
              class="mp-search"
              aria-label="Filter installed agents"
            >
            <button
              v-if="installedSearchQuery"
              class="mp-search-clear"
              aria-label="Clear filter"
              @click="installedSearchQuery = ''"
            >
              ✕
            </button>
          </div>
          <p
            class="mp-result-count"
            role="status"
            aria-live="polite"
          >
            Showing <strong>{{ displayedInstalledAgents.length }}</strong>
            of {{ packageStore.installedAgents.length }} installed
          </p>
        </div>

        <p
          v-if="displayedInstalledAgents.length === 0"
          class="mp-status"
        >
          No installed agents match your filter.
        </p>

        <div
          v-else
          class="mp-grid"
        >
          <div
            v-for="agent in displayedInstalledAgents"
            :key="agent.name"
            class="mp-card mp-card-installed"
          >
            <div class="mp-card-header">
              <h3 class="mp-agent-name">
                {{ agent.name }}
              </h3>
              <span class="mp-version">v{{ agent.version }}</span>
            </div>
            <p class="mp-description">
              {{ agent.description }}
            </p>
            <div class="mp-sandbox-status">
              <span
                class="mp-sandbox-badge"
                :class="sandboxBadgeClass(agent.name)"
              >
                {{ sandboxLabel(agent.name) }}
              </span>
            </div>
            <div class="mp-card-actions">
              <button
                class="btn-secondary btn-sm"
                @click="viewCapabilities(agent.name)"
              >
                🔐 Capabilities
              </button>
              <button
                class="btn-danger btn-sm"
                :disabled="isLoading"
                @click="handleRemove(agent.name)"
              >
                🗑 Remove
              </button>
            </div>
          </div>
        </div>
      </template>
    </div>

    <!-- ── Companions tab ── -->
    <div
      v-if="activeTab === 'companions'"
      class="mp-panel"
    >
      <CompanionsPanel />
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
    <div
      v-if="capDetailAgent"
      class="mp-modal-backdrop"
      @click.self="capDetailAgent = null"
    >
      <div class="mp-modal">
        <h3>🔐 {{ capDetailAgent }} — Capabilities</h3>
        <p
          v-if="sandboxStore.isLoading"
          class="mp-status"
        >
          Loading…
        </p>
        <ul
          v-else-if="sandboxStore.consents.length > 0"
          class="mp-cap-list"
        >
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
        <p
          v-else
          class="mp-status"
        >
          No capability consents recorded.
        </p>
        <div class="mp-modal-btns">
          <button
            class="btn-secondary"
            @click="capDetailAgent = null"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { usePackageStore } from '../stores/package';
import { useSandboxStore } from '../stores/sandbox';
import {
  NVIDIA_FREE_MODELS,
  OPENROUTER_FREE_MODELS,
  POLLINATIONS_MODELS,
  useBrainStore,
  type BrowserAuthModelOption,
} from '../stores/brain';
import AppBreadcrumb from '../components/ui/AppBreadcrumb.vue';

const emit = defineEmits<{
  navigate: [target: string];
}>();
import CapabilityConsentDialog from '../components/CapabilityConsentDialog.vue';
import CompanionsPanel from '../components/CompanionsPanel.vue';
import type { AgentSearchResult } from '../types';
import { formatRam } from '../utils/format';

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
  if (mode.mode === 'local_ollama') return '🖥 Ollama';
  if (mode.mode === 'local_lm_studio') return '🖥 LM Studio';
  return 'Local';
});

const showDetails = ref(false);

// ── LLM configuration state ──────────────────────────────────────────────────
const showLlmConfig = ref(false);
const llmTier = ref<'free' | 'paid' | 'local' | 'lm_studio'>(
  brainStore.brainMode?.mode === 'paid_api'
    ? 'paid'
    : brainStore.brainMode?.mode === 'local_ollama'
      ? 'local'
      : brainStore.brainMode?.mode === 'local_lm_studio'
        ? 'lm_studio'
        : 'free',
);
const llmLocalProvider = ref<'ollama' | 'lm_studio'>(
  brainStore.brainMode?.mode === 'local_lm_studio' ? 'lm_studio' : 'ollama',
);
const llmSelectedProvider = ref(
  brainStore.brainMode?.mode === 'free_api' ? brainStore.brainMode.provider_id : 'openrouter',
);
const llmFreeApiKey = ref('');
const llmFreeModel = ref('');
const llmManualFreeKeyOpen = ref(false);
const llmConfirmation = ref<{ name: string; url: string } | null>(null);

// Paid API fields
const llmPaidProvider = ref('openai');
const llmPaidApiKey = ref('');
const llmPaidModel = ref('gpt-4o');
const llmPaidBaseUrl = ref('');
const llmManualPaidKeyOpen = ref(false);

const paidProviderOptions = [
  {
    id: 'openai',
    label: 'Authorize with ChatGPT',
    hint: 'OpenAI key, GPT models',
    model: 'gpt-4o-mini',
    baseUrl: 'https://api.openai.com',
    authUrl: 'https://platform.openai.com/api-keys',
  },
  {
    id: 'gemini',
    label: 'Authorize with Gemini',
    hint: 'Google AI Studio key',
    model: 'gemini-3-flash-preview',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai',
    authUrl: 'https://aistudio.google.com/app/apikey',
  },
  {
    id: 'anthropic',
    label: 'Authorize with Claude',
    hint: 'Anthropic key',
    model: 'claude-sonnet-4-20250514',
    baseUrl: 'https://api.anthropic.com',
    authUrl: 'https://console.anthropic.com/settings/keys',
  },
  {
    id: 'custom',
    label: 'Custom endpoint',
    hint: 'Any OpenAI-compatible API',
    model: '',
    baseUrl: '',
    authUrl: '',
  },
] as const;

// Local Ollama fields
const llmLocalModel = ref(brainStore.topRecommendation?.model_tag ?? '');

// Local LM Studio fields
const llmLmStudioBaseUrl = ref(
  brainStore.brainMode?.mode === 'local_lm_studio'
    ? brainStore.brainMode.base_url
    : 'http://127.0.0.1:1234',
);
const llmLmStudioApiKey = ref(
  brainStore.brainMode?.mode === 'local_lm_studio'
    ? brainStore.brainMode.api_key ?? ''
    : '',
);
const llmLmStudioModel = ref(
  brainStore.brainMode?.mode === 'local_lm_studio'
    ? brainStore.brainMode.model
    : '',
);
const llmLmStudioEmbeddingModel = ref(
  brainStore.brainMode?.mode === 'local_lm_studio'
    ? brainStore.brainMode.embedding_model ?? ''
    : '',
);

const currentFreeProviderId = computed(() =>
  brainStore.brainMode?.mode === 'free_api' ? brainStore.brainMode.provider_id : null,
);

const selectedFreeProviderNeedsKey = computed(() => {
  const p = brainStore.freeProviders.find((fp) => fp.id === llmSelectedProvider.value);
  return p?.requires_api_key ?? false;
});

const selectedFreeProviderModelOptions = computed<BrowserAuthModelOption[]>(() => {
  if (llmSelectedProvider.value === 'openrouter') return OPENROUTER_FREE_MODELS;
  if (llmSelectedProvider.value === 'nvidia-nim') return NVIDIA_FREE_MODELS;
  if (llmSelectedProvider.value === 'pollinations') return POLLINATIONS_MODELS;
  return [];
});

const llmSelectedProviderName = computed(() => {
  const p = brainStore.freeProviders.find((fp) => fp.id === llmSelectedProvider.value);
  return p?.display_name ?? llmSelectedProvider.value;
});

const selectedPaidProviderLabel = computed(() =>
  paidProviderOptions.find((provider) => provider.id === llmPaidProvider.value)?.label ?? 'Paid API',
);

const selectedFreeProviderAuthUrl = computed(() => freeProviderAuthUrl(llmSelectedProvider.value));

const selectedPaidProviderAuthUrl = computed(() =>
  paidProviderOptions.find((provider) => provider.id === llmPaidProvider.value)?.authUrl ?? '',
);

watch([llmSelectedProvider, selectedFreeProviderModelOptions], () => {
  llmFreeModel.value = selectedFreeProviderModelOptions.value[0]?.model ?? '';
}, { immediate: true });

function selectPaidProvider(providerId: string) {
  llmPaidProvider.value = providerId;
  const provider = paidProviderOptions.find((item) => item.id === providerId);
  if (!provider) return;
  llmPaidModel.value = provider.model || llmPaidModel.value;
  llmPaidBaseUrl.value = provider.baseUrl;
  llmPaidApiKey.value = '';
  llmManualPaidKeyOpen.value = provider.id === 'custom';
}

function selectFreeProvider(providerId: string) {
  llmSelectedProvider.value = providerId;
  llmFreeApiKey.value = '';
  llmManualFreeKeyOpen.value = false;
}

function freeProviderAuthUrl(providerId: string): string {
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
    default: return '';
  }
}

function resolvedPaidBaseUrl(): string {
  switch (llmPaidProvider.value) {
    case 'openai': return 'https://api.openai.com';
    case 'gemini': return 'https://generativelanguage.googleapis.com/v1beta/openai';
    case 'anthropic': return 'https://api.anthropic.com';
    default: return llmPaidBaseUrl.value;
  }
}

function applyFreeProvider() {
  const providerId = llmSelectedProvider.value;
  const apiKey = llmFreeApiKey.value || null;
  if (llmFreeModel.value) {
    brainStore.setFallbackProviderModel(providerId, llmFreeModel.value);
  }
  brainStore.brainMode = {
    mode: 'free_api',
    provider_id: providerId,
    api_key: apiKey,
    model: llmFreeModel.value || null,
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

function formatBytes(bytes: number): string {
  if (!bytes) return 'unknown size';
  const gb = bytes / 1024 / 1024 / 1024;
  if (gb >= 1) return `${gb.toFixed(1)} GB`;
  return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
}

async function refreshLmStudioRuntime() {
  await brainStore.checkLmStudioStatus(
    llmLmStudioBaseUrl.value,
    llmLmStudioApiKey.value || null,
  );
  if (brainStore.lmStudioStatus?.running) {
    await brainStore.fetchLmStudioModels(
      llmLmStudioBaseUrl.value,
      llmLmStudioApiKey.value || null,
    );
    if (!llmLmStudioModel.value && brainStore.lmStudioModels.length > 0) {
      const firstLlm = brainStore.lmStudioModels.find((m) => m.type === 'llm') ?? brainStore.lmStudioModels[0];
      llmLmStudioModel.value = firstLlm.key;
    }
  }
}

async function downloadLmStudioModel() {
  if (!llmLmStudioModel.value) return;
  await brainStore.downloadLmStudioModel({
    model: llmLmStudioModel.value,
    baseUrl: llmLmStudioBaseUrl.value,
    apiKey: llmLmStudioApiKey.value || null,
    quantization: null,
  });
  await refreshLmStudioRuntime();
}

async function loadLmStudioModel() {
  if (!llmLmStudioModel.value) return;
  await brainStore.loadLmStudioModel({
    model: llmLmStudioModel.value,
    baseUrl: llmLmStudioBaseUrl.value,
    apiKey: llmLmStudioApiKey.value || null,
    contextLength: null,
  });
  await refreshLmStudioRuntime();
}

async function applyLmStudioModel() {
  if (!llmLmStudioModel.value) return;
  const mode = {
    mode: 'local_lm_studio' as const,
    model: llmLmStudioModel.value,
    base_url: llmLmStudioBaseUrl.value,
    api_key: llmLmStudioApiKey.value || null,
    embedding_model: llmLmStudioEmbeddingModel.value || null,
  };
  brainStore.brainMode = mode;
  await brainStore.setBrainMode(mode);
  llmConfirmation.value = {
    name: `LM Studio / ${llmLmStudioModel.value}`,
    url: llmLmStudioBaseUrl.value,
  };
}

const activeTab = ref<'browse' | 'installed' | 'companions'>('browse');
const tabs = [
  { id: 'browse' as const, icon: '🔍', label: 'Browse' },
  { id: 'installed' as const, icon: '📦', label: 'Installed' },
  { id: 'companions' as const, icon: '🤝', label: 'Companions' },
];

function selectTab(id: 'browse' | 'installed' | 'companions'): void {
  activeTab.value = id;
  if (id === 'companions') {
    try {
      localStorage.setItem('ts-companions-tab-visited', '1');
    } catch {
      /* localStorage unavailable in some embedded contexts */
    }
  }
}

const searchQuery = ref('');
const installedSearchQuery = ref('');
const filterKind = ref<'all' | 'package' | 'local_llm' | 'cloud'>('all');
const filterStatus = ref<'all' | 'installed' | 'not_installed'>('all');
const filterRam = ref<'any' | '4' | '8' | '16' | '32'>('any');
const sortBy = ref<'recommended' | 'name_asc' | 'name_desc' | 'ram_asc' | 'ram_desc'>('recommended');
const isLoading = computed(() => packageStore.isLoading);

const hasActiveFilters = computed(() =>
  searchQuery.value.trim() !== ''
  || filterKind.value !== 'all'
  || filterStatus.value !== 'all'
  || filterRam.value !== 'any'
  || sortBy.value !== 'recommended',
);

function resetFilters() {
  searchQuery.value = '';
  filterKind.value = 'all';
  filterStatus.value = 'all';
  filterRam.value = 'any';
  sortBy.value = 'recommended';
}

function clearSearch() {
  searchQuery.value = '';
}

function matchesInstallStatus(agent: AgentSearchResult, want: 'installed' | 'not_installed'): boolean {
  const installed = agent.kind === 'local_llm'
    ? isLocalLlmInstalled(agent)
    : isInstalled(agent.name);
  return want === 'installed' ? installed : !installed;
}

const displayedAgents = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  let list = packageStore.searchResults.slice();

  // Text filter (client-side, instant — no backend refetch on every keystroke).
  if (q) {
    list = list.filter((a) =>
      a.name.toLowerCase().includes(q)
      || a.description.toLowerCase().includes(q)
      || a.capabilities.some((c) => c.toLowerCase().includes(q)),
    );
  }

  // Kind filter — `cloud` covers `is_cloud === true`, `local_llm` covers
  // on-device Ollama models, `package` is the default for everything else.
  if (filterKind.value === 'cloud') {
    list = list.filter((a) => a.is_cloud === true);
  } else if (filterKind.value === 'local_llm') {
    list = list.filter((a) => a.kind === 'local_llm' && !a.is_cloud);
  } else if (filterKind.value === 'package') {
    list = list.filter((a) => (a.kind ?? 'package') === 'package');
  }

  // Install-status filter.
  if (filterStatus.value !== 'all') {
    list = list.filter((a) => matchesInstallStatus(a, filterStatus.value as 'installed' | 'not_installed'));
  }

  // Max-RAM filter (in GB). Agents without a known RAM requirement are kept
  // because the constraint doesn't apply to them.
  if (filterRam.value !== 'any') {
    const capMb = Number(filterRam.value) * 1024;
    list = list.filter((a) => a.required_ram_mb == null || a.required_ram_mb <= capMb);
  }

  // Sort.
  switch (sortBy.value) {
    case 'name_asc':
      list.sort((a, b) => agentDisplayName(a).localeCompare(agentDisplayName(b)));
      break;
    case 'name_desc':
      list.sort((a, b) => agentDisplayName(b).localeCompare(agentDisplayName(a)));
      break;
    case 'ram_asc':
      list.sort((a, b) => (a.required_ram_mb ?? Number.POSITIVE_INFINITY) - (b.required_ram_mb ?? Number.POSITIVE_INFINITY));
      break;
    case 'ram_desc':
      list.sort((a, b) => (b.required_ram_mb ?? Number.NEGATIVE_INFINITY) - (a.required_ram_mb ?? Number.NEGATIVE_INFINITY));
      break;
    case 'recommended':
    default: {
      list.sort((a, b) => {
        const at = a.is_top_pick ? 0 : 1;
        const bt = b.is_top_pick ? 0 : 1;
        if (at !== bt) return at - bt;
        return agentDisplayName(a).localeCompare(agentDisplayName(b));
      });
      break;
    }
  }

  return list;
});

const displayedInstalledAgents = computed(() => {
  const q = installedSearchQuery.value.trim().toLowerCase();
  if (!q) return packageStore.installedAgents.slice();
  return packageStore.installedAgents.filter((a) =>
    a.name.toLowerCase().includes(q)
    || a.description.toLowerCase().includes(q),
  );
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
    brainStore.checkLmStudioStatus(),
    brainStore.fetchLmStudioModels(),
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
.marketplace-view { display: flex; flex-direction: column; height: 100%; min-height: 0; padding: 1rem; gap: 0.75rem; overflow-x: hidden; overflow-y: auto; scrollbar-gutter: stable; }
.mp-header { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 0.5rem; }
.mp-header h2 { margin: 0; font-size: 1.25rem; }
.mp-header-actions { display: flex; gap: 0.5rem; }
.mp-error { padding: 0.5rem 1rem; background: var(--ts-error-bg); color: var(--ts-error); border-radius: 6px; margin: 0; }
.mp-tabs { display: flex; gap: 0.25rem; }
.mp-tab { padding: 0.4rem 1rem; border: none; border-radius: 6px; cursor: pointer; background: var(--ts-bg-surface); color: var(--ts-text-secondary); font-size: var(--ts-text-sm); transition: background var(--ts-transition-fast), color var(--ts-transition-fast); }
.mp-tab:hover { background: var(--ts-bg-hover); color: var(--ts-text-primary); }
.mp-tab.active { background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); }
.mp-panel { flex: 1; display: flex; flex-direction: column; gap: 0.75rem; overflow-y: auto; min-height: 0; }
.mp-toolbar { display: flex; flex-direction: column; gap: 0.5rem; }
.mp-search-row { display: flex; gap: 0.5rem; align-items: center; position: relative; }
.mp-search-icon { position: absolute; left: 0.6rem; pointer-events: none; color: var(--ts-text-muted); font-size: 0.85rem; }
.mp-search { flex: 1; padding: 0.4rem 2rem 0.4rem 2rem; background: var(--ts-bg-input); border: 1px solid var(--ts-border-medium); border-radius: 6px; color: var(--ts-text-primary); outline: none; transition: border-color var(--ts-transition-fast); }
.mp-search:focus { border-color: var(--ts-accent-blue); }
.mp-search::placeholder { color: var(--ts-text-dim); }
.mp-search-clear { position: absolute; right: 0.4rem; width: 1.5rem; height: 1.5rem; padding: 0; display: flex; align-items: center; justify-content: center; background: transparent; color: var(--ts-text-muted); border: none; border-radius: 4px; cursor: pointer; font-size: 0.85rem; transition: background var(--ts-transition-fast), color var(--ts-transition-fast); }
.mp-search-clear:hover { background: var(--ts-bg-hover); color: var(--ts-text-primary); }
.mp-filter-row { display: flex; flex-wrap: wrap; gap: 0.5rem; align-items: flex-end; }
.mp-filter { display: flex; flex-direction: column; gap: 0.2rem; min-width: 8rem; }
.mp-filter-label { font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--ts-text-muted); font-weight: 600; }
.mp-select { padding: 0.35rem 0.55rem; background: var(--ts-bg-input); border: 1px solid var(--ts-border-medium); border-radius: 6px; color: var(--ts-text-primary); font-size: 0.85rem; outline: none; cursor: pointer; transition: border-color var(--ts-transition-fast); }
.mp-select:focus { border-color: var(--ts-accent-blue); }
.mp-select:hover { border-color: var(--ts-border-strong, var(--ts-border-medium)); }
.mp-reset-btn { align-self: flex-end; }
.mp-result-count { margin: 0; font-size: 0.75rem; color: var(--ts-text-secondary); }
.mp-result-count strong { color: var(--ts-text-primary); }
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
.mp-meta { display: flex; gap: 0.3rem; flex-wrap: wrap; }
.mp-meta-badge { font-size: 0.7rem; padding: 0.15rem 0.5rem; border-radius: 999px; font-weight: 600; }
.mp-meta-rec { background: var(--ts-warning-bg, var(--ts-bg-base)); color: var(--ts-warning, var(--ts-text-secondary)); }
.mp-meta-cloud { background: var(--ts-info-bg, var(--ts-bg-base)); color: var(--ts-info, var(--ts-text-secondary)); }
.mp-meta-ram { background: var(--ts-bg-base); color: var(--ts-text-muted); }
.mp-card-header { display: flex; align-items: baseline; gap: 0.5rem; }
.mp-agent-name { margin: 0; font-size: 1rem; color: var(--ts-text-primary); }
.mp-version { font-size: 0.75rem; color: var(--ts-text-muted); }
.mp-description { margin: 0; color: var(--ts-text-secondary); font-size: 0.85rem; }
.mp-caps { display: flex; gap: 0.3rem; flex-wrap: wrap; }
.mp-cap-badge { font-size: 0.7rem; padding: 0.15rem 0.5rem; background: var(--ts-bg-base); border-radius: 999px; color: var(--ts-text-secondary); }
.mp-homepage { font-size: 0.75rem; }
.mp-link { color: var(--ts-accent-blue); text-decoration: none; word-break: break-all; }
.mp-link:hover { text-decoration: underline; }
.mp-empty { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 2.5rem 1rem; }
.mp-empty-title { font-size: 1rem; color: var(--ts-text-primary); font-weight: 600; margin: 0; }
.mp-empty-sub { font-size: 0.85rem; color: var(--ts-text-secondary); max-width: 32rem; text-align: center; margin: 0; line-height: 1.4; }
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
.mp-modal-backdrop { position: fixed; inset: 0; background: var(--ts-bg-backdrop); display: flex; align-items: center; justify-content: center; z-index: 100; backdrop-filter: blur(4px); }
.mp-modal { background: var(--ts-bg-surface); border: 1px solid var(--ts-border); border-radius: 12px; padding: 1.5rem; width: min(480px, 90vw); display: flex; flex-direction: column; gap: 0.75rem; box-shadow: var(--ts-shadow-lg); }
.mp-cap-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.4rem; }
.mp-cap-row { display: flex; justify-content: space-between; align-items: center; padding: 0.5rem 0.75rem; background: var(--ts-bg-base); border-radius: 6px; font-size: 0.85rem; }
.mp-grant-badge { font-size: 0.75rem; }
.mp-grant-badge.granted { color: var(--ts-success); }
.mp-grant-badge.denied { color: var(--ts-error); }
.mp-modal-btns { display: flex; gap: 0.5rem; justify-content: flex-end; }

/* ── Tauri unavailable banner (inline in marketplace) ── */
.tauri-banner {
  background: linear-gradient(135deg, var(--ts-warning-bg), rgba(245, 158, 11, 0.06));
  border: 1px solid var(--ts-warning);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  gap: 0;
  overflow: hidden;
}
.tauri-banner-compact { border-color: var(--ts-border); background: var(--ts-bg-panel); }

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
  border-top: 1px solid var(--ts-success);
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

/* Local LLM provider pill switcher */
.llm-provider-pills { display: flex; gap: 0.25rem; margin-bottom: 0.25rem; }
.llm-provider-pill {
  padding: 0.25rem 0.7rem;
  border: 1px solid var(--ts-border-medium);
  border-radius: 20px;
  background: transparent;
  color: var(--ts-text-secondary);
  font-size: 0.75rem;
  cursor: pointer;
  transition: background var(--ts-transition-fast), color var(--ts-transition-fast), border-color var(--ts-transition-fast);
}
.llm-provider-pill:hover:not(:disabled) { background: var(--ts-bg-hover); color: var(--ts-text-primary); }
.llm-provider-pill.active { background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); border-color: var(--ts-accent-blue-hover); }
.llm-provider-pill:disabled { opacity: 0.5; cursor: not-allowed; }

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
.llm-auth-provider-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 0.4rem;
}
.llm-auth-provider-btn {
  display: grid;
  gap: 0.2rem;
  min-height: 4.2rem;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  padding: 0.55rem 0.65rem;
  color: var(--ts-text-primary);
  text-align: left;
  background: var(--ts-bg-base);
  cursor: pointer;
}
.llm-auth-provider-btn:hover,
.llm-auth-provider-btn.active {
  border-color: var(--ts-accent-blue-hover);
  background: color-mix(in srgb, var(--ts-accent) 10%, var(--ts-bg-base));
}
.llm-auth-provider-btn small {
  color: var(--ts-text-muted);
  line-height: 1.3;
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

.llm-auth-link {
  justify-content: center;
  text-decoration: none;
}

.llm-manual-toggle {
  align-self: flex-start;
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
.llm-lmstudio-actions { display: flex; gap: 0.5rem; flex-wrap: wrap; justify-content: flex-end; }

/* ── Responsive: Tablet ── */
@media (max-width: 840px) {
  .mp-header { flex-direction: column; align-items: stretch; }
  .mp-header-actions { flex-wrap: wrap; }
  .mp-search-row { flex-wrap: wrap; }
  .mp-search { flex-basis: 100%; }
}

/* ── Responsive: Mobile ── */
@media (max-width: 640px) {
  .marketplace-view { padding: 0.75rem 0.5rem; gap: 0.5rem; }
  .mp-header h2 { font-size: 1.1rem; }
  .mp-header-actions { gap: 0.35rem; }
  .mp-tabs { flex-wrap: wrap; }
  .mp-tab { padding: 0.35rem 0.75rem; font-size: 0.78rem; }
  .mp-card { padding: 0.75rem; gap: 0.35rem; }
  .mp-modal { padding: 1rem; width: min(400px, 95vw); }
  .tauri-feature-list { grid-template-columns: 1fr; }
  .tauri-details { padding: 0.5rem 0.75rem 1rem; }
  .llm-tier-tabs { flex-wrap: wrap; }
  .llm-tier-tab { padding: 0.3rem 0.4rem; font-size: 0.72rem; }
}
</style>
