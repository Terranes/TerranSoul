<template>
  <section
    class="browser-auth-panel"
    :class="{ compact: props.compact }"
    aria-labelledby="browser-auth-title"
  >
    <div>
      <p class="card-kicker">
        {{ props.compact ? 'Browser LLM' : 'Vercel test environment' }}
      </p>
      <h2 id="browser-auth-title">
        Choose your web LLM.
      </h2>
      <p>
        Open the provider page first, authorize or create a restricted key there,
        then connect it here only if you want direct browser chat.
      </p>
      <p class="auth-recommendation">
        Best free default today: <strong>OpenRouter</strong>, because it gives this
        backendless build one key for many current free models.
      </p>
      <p
        v-if="browserAuthSession"
        class="auth-status"
      >
        Connected: <strong>{{ browserAuthSession.label }}</strong>
        <span v-if="browserAuthSession.model"> · {{ browserAuthSession.model }}</span>
      </p>
    </div>
    <div class="auth-actions">
      <button
        v-for="provider in browserAuthProviders"
        :key="provider.id"
        type="button"
        class="auth-action"
        :class="{ active: selectedProviderId === provider.id || browserAuthSession?.providerId === provider.id }"
        @click="selectProvider(provider.id)"
      >
        <span>{{ provider.label }}</span>
        <em v-if="provider.recommendation">{{ provider.recommendation }}</em>
        <small>{{ provider.privacyLabel }}</small>
      </button>

      <div
        v-if="selectedProvider"
        class="auth-prompt"
      >
        <div class="auth-provider-summary">
          <strong>{{ selectedProvider.label }}</strong>
          <span>{{ selectedProvider.privacyLabel }}</span>
        </div>

        <label
          v-if="selectedProvider.modelOptions?.length"
          class="auth-field"
        >
          <span>Model</span>
          <select v-model="selectedModel">
            <option
              v-for="option in selectedProvider.modelOptions"
              :key="option.model"
              :value="option.model"
            >
              {{ option.label }}
            </option>
          </select>
        </label>

        <a
          class="auth-confirm auth-provider-link"
          :href="selectedProvider.authorizationUrl"
          target="_blank"
          rel="noopener"
          @click="openedAuthorization = true"
        >
          {{ selectedProvider.authorizationLabel }}
        </a>

        <p
          v-if="openedAuthorization"
          class="auth-after-open"
        >
          Finish authorization in the provider tab, then use the manual option
          only if the provider gives you a key or token to paste back here.
        </p>

        <button
          type="button"
          class="auth-manual-toggle"
          :aria-expanded="manualKeyOpen"
          @click="manualKeyOpen = !manualKeyOpen"
        >
          {{ manualKeyOpen ? 'Hide manual API key' : 'Manual API key option' }}
        </button>

        <div
          v-if="manualKeyOpen"
          class="auth-manual-panel"
        >
          <p>
            Paste the key or token from the provider page. It is stored only in
            this browser session/localStorage for the static web app.
          </p>
          <label
            v-if="selectedProvider.requiresApiKey"
            class="auth-field"
          >
            <span>API key</span>
            <input
              v-model="apiKey"
              type="password"
              :placeholder="selectedProvider.apiKeyPlaceholder ?? 'Provider API key'"
            >
          </label>

          <button
            type="button"
            class="auth-confirm"
            :disabled="selectedProvider.requiresApiKey && !apiKey.trim()"
            @click="authoriseSelectedProvider"
          >
            Connect with this key
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useBrainStore, type BrowserAuthProviderId, type BrowserAuthSession } from '../stores/brain';

const props = withDefaults(defineProps<{
  compact?: boolean;
}>(), {
  compact: false,
});

const emit = defineEmits<{
  configured: [session: BrowserAuthSession];
}>();

const brain = useBrainStore();
const providerPriority: Record<BrowserAuthProviderId, number> = {
  openrouter: 0,
  gemini: 1,
  'nvidia-nim': 2,
  pollinations: 3,
  chatgpt: 4,
  google: 5,
};
const browserAuthProviders = computed(() =>
  [...brain.browserAuthProviders].sort((left, right) => providerPriority[left.id] - providerPriority[right.id]),
);
const browserAuthSession = computed(() => brain.browserAuthSession);
const selectedProviderId = ref<BrowserAuthProviderId>(
  browserAuthSession.value?.providerId ?? 'openrouter',
);
const apiKey = ref('');
const selectedModel = ref('');
const manualKeyOpen = ref(false);
const openedAuthorization = ref(false);

const selectedProvider = computed(() =>
  browserAuthProviders.value.find((provider) => provider.id === selectedProviderId.value) ?? null,
);

watch(selectedProvider, (provider) => {
  apiKey.value = '';
  selectedModel.value = provider?.modelOptions?.[0]?.model ?? '';
  manualKeyOpen.value = false;
  openedAuthorization.value = false;
}, { immediate: true });

function selectProvider(providerId: BrowserAuthProviderId): void {
  selectedProviderId.value = providerId;
}

function authoriseSelectedProvider(): void {
  if (!selectedProvider.value) return;
  const session = brain.authoriseBrowserProvider(selectedProvider.value.id, {
    apiKey: apiKey.value,
    model: selectedModel.value,
  });
  emit('configured', session);
}
</script>

<style scoped>
.browser-auth-panel {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(220px, 0.82fr);
  gap: var(--ts-space-md);
  margin-top: var(--ts-space-xl);
  max-height: min(720px, calc(100dvh - 2rem));
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
  padding: var(--ts-space-md);
  border: 1px solid color-mix(in srgb, var(--ts-accent) 28%, var(--ts-border));
  border-radius: var(--ts-radius-xl);
  background: color-mix(in srgb, var(--ts-bg-panel) 72%, transparent);
  box-shadow: var(--ts-shadow-md);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

.browser-auth-panel.compact {
  margin-top: 0;
  grid-template-columns: 1fr;
  max-height: min(620px, calc(100dvh - 6.5rem));
  padding: var(--ts-space-sm);
}

.browser-auth-panel h2 {
  margin-bottom: 0.55rem;
  font-size: 1.65rem;
}

.browser-auth-panel p {
  margin-bottom: 0;
  color: var(--ts-text-secondary);
  line-height: 1.55;
}

.browser-auth-panel .auth-status {
  margin-top: var(--ts-space-sm);
  color: var(--ts-text-primary);
}

.auth-recommendation {
  margin-top: var(--ts-space-sm);
  padding: 0.55rem 0.7rem;
  border: 1px solid color-mix(in srgb, var(--ts-success) 34%, var(--ts-border));
  border-radius: var(--ts-radius-md);
  background: color-mix(in srgb, var(--ts-success-bg, var(--ts-bg-selected)) 70%, transparent);
}

.auth-actions {
  display: grid;
  gap: var(--ts-space-sm);
  min-height: 0;
}

.auth-action {
  display: grid;
  gap: 0.18rem;
  width: 100%;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-lg);
  padding: 0.72rem 0.85rem;
  color: var(--ts-text-primary);
  text-align: left;
  background: var(--ts-bg-input);
  cursor: pointer;
  transition: transform var(--ts-transition-fast, 0.15s ease), border-color var(--ts-transition-fast, 0.15s ease), background var(--ts-transition-fast, 0.15s ease);
}

.auth-action:hover,
.auth-action:focus-visible,
.auth-action.active {
  transform: translateY(-1px);
  border-color: color-mix(in srgb, var(--ts-accent) 58%, var(--ts-border));
  background: color-mix(in srgb, var(--ts-accent) 14%, var(--ts-bg-input));
}

.auth-action span {
  font-weight: 900;
}

.auth-action em {
  width: fit-content;
  border-radius: var(--ts-radius-pill);
  padding: 0.12rem 0.42rem;
  color: var(--ts-success);
  background: color-mix(in srgb, var(--ts-success) 14%, transparent);
  font-size: 0.68rem;
  font-style: normal;
  font-weight: 900;
}

.auth-action small {
  color: var(--ts-text-secondary);
  line-height: 1.35;
}

.auth-prompt {
  display: grid;
  gap: var(--ts-space-sm);
  padding: var(--ts-space-sm);
  border: 1px solid color-mix(in srgb, var(--ts-accent) 28%, var(--ts-border));
  border-radius: var(--ts-radius-lg);
  background: color-mix(in srgb, var(--ts-bg-panel) 86%, transparent);
}

.auth-provider-summary {
  display: grid;
  gap: 0.25rem;
}

.auth-provider-summary span,
.auth-after-open,
.auth-manual-panel p {
  margin: 0;
  color: var(--ts-text-secondary);
  font-size: 0.82rem;
  line-height: 1.45;
}

.auth-field {
  display: grid;
  gap: 0.35rem;
  color: var(--ts-text-secondary);
  font-size: 0.84rem;
  font-weight: 800;
}

.auth-field input,
.auth-field select {
  min-height: 2.35rem;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  padding: 0.45rem 0.6rem;
  color: var(--ts-text-primary);
  background: var(--ts-bg-input);
}

.auth-confirm {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 2.4rem;
  border: 0;
  border-radius: var(--ts-radius-md);
  padding: 0.55rem 0.8rem;
  color: var(--ts-text-on-accent);
  font-weight: 900;
  background: var(--ts-accent);
  cursor: pointer;
  text-decoration: none;
}

.auth-confirm:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.auth-manual-toggle {
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-md);
  padding: 0.55rem 0.8rem;
  color: var(--ts-text-primary);
  background: var(--ts-bg-input);
  font-weight: 850;
  cursor: pointer;
}

.auth-manual-toggle:hover {
  border-color: color-mix(in srgb, var(--ts-accent) 45%, var(--ts-border));
}

.auth-manual-panel {
  display: grid;
  gap: var(--ts-space-sm);
  padding-top: var(--ts-space-xs);
}

@media (max-width: 640px) {
  .browser-auth-panel {
    grid-template-columns: 1fr;
    max-height: calc(100dvh - 2rem);
  }
}
</style>
