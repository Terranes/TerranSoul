<template>
  <div
    class="bp-shell voice-setup"
    data-density="cozy"
    data-testid="voice-setup-view"
  >
    <!-- ── Breadcrumb (preserved for desktop nav + e2e) ─────────────────── -->
    <AppBreadcrumb
      here="VOICE SETUP"
      @navigate="emit('navigate', $event)"
    />

    <!-- ── Header ───────────────────────────────────────────────────────── -->
    <section class="bp-module vs-card vs-header-card">
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">🎙</span> Voice
          </div>
          <h2 class="bp-module-title">
            Voice Setup
          </h2>
        </div>
        <div class="bp-module-head-right">
          <span
            v-if="voice.isTextOnly"
            class="vs-status-chip vs-status-chip--muted"
            data-testid="voice-text-only-pill"
          >
            🔇 Text-only
          </span>
          <span
            v-else
            class="vs-status-chip vs-status-chip--ok"
            data-testid="voice-active-pill"
          >
            ✅ Voice enabled
          </span>
        </div>
      </header>
      <p class="vs-desc">
        Pick how TerranSoul speaks and listens. Each provider works on its own —
        you can mix and match (e.g. browser input + cloud output). Changes take
        effect immediately.
      </p>
      <div
        v-if="voice.error"
        class="vs-alert vs-alert--error"
        role="alert"
      >
        ⚠️ {{ voice.error }}
      </div>
    </section>

    <!-- ── TTS providers ────────────────────────────────────────────────── -->
    <section
      class="bp-module vs-card"
      data-testid="voice-tts-section"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">🔊</span> Output
          </div>
          <h2 class="bp-module-title">
            Voice Output (TTS)
          </h2>
        </div>
      </header>
      <p class="vs-desc">
        How TerranSoul speaks to you. Click <strong>Test</strong> to audition a
        provider, then <strong>Set as default</strong> to switch.
      </p>

      <!-- Test sample text — Test buttons below speak this exact text. -->
      <div class="vs-test-sample">
        <label
          for="vs-test-sample-input"
          class="vs-test-sample__label"
        >
          Test sample text
        </label>
        <textarea
          id="vs-test-sample-input"
          v-model="testSampleText"
          class="vs-input vs-test-sample__textarea"
          rows="2"
          maxlength="600"
          :placeholder="DEFAULT_SAMPLE_TEXT"
          data-testid="voice-test-sample-text"
        />
        <p class="vs-test-sample__hint">
          The <strong>Test</strong> buttons below will speak this text.
          Leave blank to use the default sample.
        </p>
      </div>

      <!-- Ready-to-use providers: installed + (free OR API key already saved). -->
      <div
        v-if="readyTtsProviders.length > 0"
        class="vs-provider-group"
        data-testid="tts-ready-group"
      >
        <h3 class="vs-group-heading">
          <span aria-hidden="true">✅</span> Ready to use
        </h3>
        <ul class="vs-provider-grid">
          <VoiceProviderCard
            v-for="provider in readyTtsProviders"
            :key="`tts-ready-${provider.id}`"
            :provider="provider"
            kind="tts"
            :is-active="voice.config.tts_provider === provider.id"
            :needs-setup="false"
            :testing="testingProviderId === provider.id"
            :test-result="testResultByProvider[provider.id] ?? null"
            :tts-voice="voice.config.tts_voice"
            :default-voice="defaultVoiceFor(provider.id)"
            :voice-catalogue="VOICE_CATALOGUE"
            @set-default="onSetTts(provider.id)"
            @clear-default="onSetTts(null)"
            @test="onTestTts(provider)"
            @voice-change="onVoiceChange"
          />
        </ul>
      </div>

      <!-- Needs-setup providers: download required OR missing API key. -->
      <div
        v-if="setupTtsProviders.length > 0"
        class="vs-provider-group vs-provider-group--setup"
        data-testid="tts-setup-group"
      >
        <h3 class="vs-group-heading">
          <span aria-hidden="true">🛠</span> Needs setup
        </h3>
        <p class="vs-group-hint">
          These providers need a one-time step before they can speak. Pick one
          to start the setup.
        </p>
        <ul class="vs-provider-grid">
          <VoiceProviderCard
            v-for="provider in setupTtsProviders"
            :key="`tts-setup-${provider.id}`"
            :provider="provider"
            kind="tts"
            :is-active="voice.config.tts_provider === provider.id"
            :needs-setup="true"
            :testing="testingProviderId === provider.id"
            :test-result="testResultByProvider[provider.id] ?? null"
            :tts-voice="voice.config.tts_voice"
            :default-voice="defaultVoiceFor(provider.id)"
            :voice-catalogue="VOICE_CATALOGUE"
            @set-default="onSetTts(provider.id)"
            @clear-default="onSetTts(null)"
            @test="onTestTts(provider)"
            @voice-change="onVoiceChange"
          />
        </ul>
      </div>
    </section>

    <!-- ── ASR providers ────────────────────────────────────────────────── -->
    <section
      class="bp-module vs-card"
      data-testid="voice-asr-section"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">🎤</span> Input
          </div>
          <h2 class="bp-module-title">
            Voice Input (ASR)
          </h2>
        </div>
      </header>
      <p class="vs-desc">
        How TerranSoul hears you. Pick a provider to enable push-to-talk and
        live dictation in the chat composer.
      </p>

      <div
        v-if="readyAsrProviders.length > 0"
        class="vs-provider-group"
        data-testid="asr-ready-group"
      >
        <h3 class="vs-group-heading">
          <span aria-hidden="true">✅</span> Ready to use
        </h3>
        <ul class="vs-provider-grid">
          <VoiceProviderCard
            v-for="provider in readyAsrProviders"
            :key="`asr-ready-${provider.id}`"
            :provider="provider"
            kind="asr"
            :is-active="voice.config.asr_provider === provider.id"
            :needs-setup="false"
            @set-default="onSetAsr(provider.id)"
            @clear-default="onSetAsr(null)"
          />
        </ul>
      </div>

      <div
        v-if="setupAsrProviders.length > 0"
        class="vs-provider-group vs-provider-group--setup"
        data-testid="asr-setup-group"
      >
        <h3 class="vs-group-heading">
          <span aria-hidden="true">🛠</span> Needs setup
        </h3>
        <p class="vs-group-hint">
          These providers need a one-time step before they can listen.
        </p>
        <ul class="vs-provider-grid">
          <VoiceProviderCard
            v-for="provider in setupAsrProviders"
            :key="`asr-setup-${provider.id}`"
            :provider="provider"
            kind="asr"
            :is-active="voice.config.asr_provider === provider.id"
            :needs-setup="true"
            @set-default="onSetAsr(provider.id)"
            @clear-default="onSetAsr(null)"
          />
        </ul>
      </div>
    </section>

    <!-- ── API key (shown when any active provider needs it) ────────────── -->
    <section
      v-if="needsApiKey"
      class="bp-module vs-card"
      data-testid="voice-api-key-section"
    >
      <header class="bp-module-head">
        <div class="bp-module-head-left">
          <div class="bp-module-eyebrow">
            <span class="ix">🔑</span> Credentials
          </div>
          <h2 class="bp-module-title">
            API key
          </h2>
        </div>
      </header>
      <p class="vs-desc">
        Required by your selected cloud provider(s):
        <strong>{{ apiKeyProviderNames }}</strong>. The key is stored locally
        and never sent anywhere other than the provider you chose.
      </p>
      <div class="vs-form">
        <label for="vs-api-key">Key</label>
        <input
          id="vs-api-key"
          v-model="apiKeyDraft"
          type="password"
          placeholder="sk-… / gsk_…"
          class="vs-input"
          autocomplete="off"
          spellcheck="false"
          data-testid="voice-api-key-input"
        >
        <div class="vs-form__actions">
          <button
            type="button"
            class="bp-btn bp-btn--primary"
            :disabled="apiKeyDraft === (voice.config.api_key ?? '')"
            data-testid="voice-api-key-save"
            @click="onSaveApiKey"
          >
            Save key
          </button>
          <button
            v-if="voice.config.api_key"
            type="button"
            class="bp-btn bp-btn--ghost"
            data-testid="voice-api-key-clear"
            @click="onClearApiKey"
          >
            Clear
          </button>
        </div>
      </div>
    </section>

    <!-- ── Footer: text-only ↔ voice toggle ─────────────────────────────── -->
    <section class="bp-module vs-card vs-footer-card">
      <div class="vs-footer-row">
        <div class="vs-footer-text">
          <strong class="vs-footer-label">Voice output</strong>
          <p class="vs-desc vs-desc--inline">
            <span v-if="voice.isTextOnly">
              Voice is currently <strong>off</strong> — TerranSoul will reply in
              text only. Toggle this on to re-enable voice using the free
              Web Speech provider.
            </span>
            <span v-else>
              Voice is currently <strong>on</strong>. Toggle off to switch back
              to text-only mode. You can re-enable voice any time.
            </span>
          </p>
        </div>
        <button
          type="button"
          class="bp-switch"
          :data-on="voice.isTextOnly ? 'false' : 'true'"
          :aria-pressed="!voice.isTextOnly"
          :aria-label="voice.isTextOnly ? 'Enable voice output' : 'Disable voice output (use text only)'"
          data-testid="voice-text-only-toggle"
          @click="onToggleTextOnly"
        />
      </div>
    </section>

    <!-- Supertonic auto-promotion banner + consent dialog. -->
    <SupertonicSection ref="supertonicSectionRef" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useVoiceStore } from '../stores/voice';
import { VOICE_CATALOGUE } from '../config/voice-catalogue';
import AppBreadcrumb from '../components/ui/AppBreadcrumb.vue';
import SupertonicSection from '../components/voice/SupertonicSection.vue';
import VoiceProviderCard from '../components/voice/VoiceProviderCard.vue';
import type { VoiceProviderInfo } from '../types';

// `done` is preserved for backward compatibility with App.vue and the
// first-launch wizard, but the panel is no longer a wizard — it never
// emits `done` on its own.
const emit = defineEmits<{
  done: [];
  navigate: [target: string];
}>();

const voice = useVoiceStore();

const testingProviderId = ref<string | null>(null);
const testResultByProvider = ref<Record<string, { ok: boolean; message: string }>>({});
const apiKeyDraft = ref('');

const SAMPLE_TEXT = 'Hello, this is a TerranSoul voice test.';
const DEFAULT_SAMPLE_TEXT = SAMPLE_TEXT;

/** User-editable test phrase. Empty string falls back to `DEFAULT_SAMPLE_TEXT`. */
const testSampleText = ref('');

/** Resolve the text the Test buttons should speak — user-provided or default. */
function resolveTestText(): string {
  const trimmed = testSampleText.value.trim();
  return trimmed.length > 0 ? trimmed : DEFAULT_SAMPLE_TEXT;
}

const needsApiKey = computed(() => {
  const tts = voice.selectedTtsProvider;
  const asr = voice.selectedAsrProvider;
  return Boolean(tts?.requires_api_key) || Boolean(asr?.requires_api_key);
});

const apiKeyProviderNames = computed(() => {
  const names: string[] = [];
  if (voice.selectedTtsProvider?.requires_api_key) {
    names.push(voice.selectedTtsProvider.display_name);
  }
  if (
    voice.selectedAsrProvider?.requires_api_key
    && voice.selectedAsrProvider.id !== voice.selectedTtsProvider?.id
  ) {
    names.push(voice.selectedAsrProvider.display_name);
  }
  return names.join(', ') || 'the selected provider';
});

function providerNeedsSetup(p: VoiceProviderInfo): boolean {
  if (p.requires_install && !p.installed) return true;
  if (p.requires_api_key && !(voice.config.api_key ?? '').trim()) return true;
  return false;
}

const readyTtsProviders = computed(() =>
  voice.ttsProviders.filter((p) => !providerNeedsSetup(p)),
);
const setupTtsProviders = computed(() =>
  voice.ttsProviders.filter((p) => providerNeedsSetup(p)),
);
const readyAsrProviders = computed(() =>
  voice.asrProviders.filter((p) => !providerNeedsSetup(p)),
);
const setupAsrProviders = computed(() =>
  voice.asrProviders.filter((p) => providerNeedsSetup(p)),
);

function defaultVoiceFor(providerId: string): string {
  if (providerId === 'web-speech') return 'browser default';
  if (providerId === 'openai-tts') return 'alloy';
  if (providerId === 'supertonic') return 'F3';
  return VOICE_CATALOGUE[0]?.id ?? 'default';
}

async function onSetTts(providerId: string | null): Promise<void> {
  testResultByProvider.value = {};
  // Route Supertonic clicks through the consent flow when the model is not
  // yet installed. The composable inspects the provider catalogue + opens
  // its modal — short-circuit `setTtsProvider` when consent is needed.
  if (supertonicSectionRef.value?.consent.openIfNeeded(providerId)) return;
  await voice.setTtsProvider(providerId);
}

// Ref to the Supertonic banner+dialog section so we can call into its
// exposed `consent` helper (see `defineExpose` in SupertonicSection.vue).
const supertonicSectionRef = ref<InstanceType<typeof SupertonicSection> | null>(null);

async function onSetAsr(providerId: string | null): Promise<void> {
  await voice.setAsrProvider(providerId);
}

async function onVoiceChange(value: string): Promise<void> {
  const trimmed = value.trim();
  // Reuse `set_tts_voice` Tauri command via store config update. The
  // dedicated `setTtsVoice` action is not exposed yet, so update via
  // local config mutation through a fresh setTtsProvider call would
  // overwrite. Instead, persist via the invoke shim used elsewhere.
  voice.config.tts_voice = trimmed.length > 0 ? trimmed : null;
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('set_tts_voice', { voiceName: trimmed.length > 0 ? trimmed : null });
  } catch {
    // Tauri unavailable — local state still updated.
  }
}

async function onTestTts(provider: VoiceProviderInfo): Promise<void> {
  if (provider.requires_install && !provider.installed) return;
  testingProviderId.value = provider.id;
  const sample = resolveTestText();
  testResultByProvider.value = { ...testResultByProvider.value, [provider.id]: { ok: true, message: 'Synthesizing…' } };
  try {
    const audio = await voice.testTtsProvider(provider.id, sample);
    if (audio.length === 0) {
      // Browser-side provider — fall back to SpeechSynthesis.
      if (typeof window !== 'undefined' && 'speechSynthesis' in window) {
        const utter = new SpeechSynthesisUtterance(sample);
        const voiceId = voice.config.tts_voice;
        if (voiceId) {
          const match = window.speechSynthesis
            .getVoices()
            .find((v) => v.name === voiceId || v.lang === voiceId);
          if (match) utter.voice = match;
        }
        window.speechSynthesis.cancel();
        window.speechSynthesis.speak(utter);
        testResultByProvider.value = { ...testResultByProvider.value, [provider.id]: { ok: true, message: '✅ Playing via browser speech synthesis.' } };
      } else {
        testResultByProvider.value = { ...testResultByProvider.value, [provider.id]: { ok: false, message: 'Browser speechSynthesis is unavailable here.' } };
      }
    } else {
      const blob = new Blob([new Uint8Array(audio).buffer as ArrayBuffer], { type: 'audio/wav' });
      const url = URL.createObjectURL(blob);
      const audioEl = new Audio(url);
      audioEl.addEventListener('ended', () => URL.revokeObjectURL(url));
      await audioEl.play();
      testResultByProvider.value = { ...testResultByProvider.value, [provider.id]: { ok: true, message: '✅ Playing test sample.' } };
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    testResultByProvider.value = { ...testResultByProvider.value, [provider.id]: { ok: false, message: `❌ ${message}` } };
  } finally {
    testingProviderId.value = null;
  }
}

async function onSaveApiKey(): Promise<void> {
  await voice.setApiKey(apiKeyDraft.value.length > 0 ? apiKeyDraft.value : null);
}

async function onClearApiKey(): Promise<void> {
  apiKeyDraft.value = '';
  await voice.setApiKey(null);
}

async function onDisableAll(): Promise<void> {
  testResultByProvider.value = {};
  await voice.clearConfig();
  apiKeyDraft.value = '';
}

/**
 * Footer toggle: flip between text-only and voice-enabled.
 *   - Going to text-only: clears the voice config (same as the old
 *     "Use text only" button).
 *   - Going back to voice: auto-configures the free Web Speech provider
 *     for both ASR and TTS so the user immediately has something working.
 *     They can pick a different provider from the cards above afterwards.
 */
async function onToggleTextOnly(): Promise<void> {
  if (voice.isTextOnly) {
    await voice.autoConfigureVoice();
  } else {
    await onDisableAll();
  }
}

watch(
  () => voice.config.api_key,
  (next) => {
    apiKeyDraft.value = next ?? '';
  },
  { immediate: true },
);

onMounted(async () => {
  await voice.initialise();
  // Promote Supertonic when already installed and user is on the browser default.
  await voice.autoPromoteSupertonicIfReady();
});
</script>

<style scoped>
.voice-setup {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 1.5rem;
  padding: 2rem;
  height: 100%;
  min-height: 100%;
  overflow-x: hidden;
  overflow-y: auto;
  scrollbar-gutter: stable;
  background: var(--ts-bg-base);
}

.vs-card {
  background: var(--ts-bg-surface);
  border: 1px solid var(--ts-border);
  border-radius: 12px;
  padding: 1.5rem;
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.vs-header-card .bp-module-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
}

.vs-desc {
  color: var(--ts-text-secondary);
  margin: 0;
  line-height: 1.5;
}

.vs-status-chip {
  font-size: 0.75rem;
  font-weight: 600;
  padding: 0.25rem 0.6rem;
  border-radius: 999px;
  white-space: nowrap;
}

.vs-status-chip--ok {
  background: var(--ts-success-bg);
  color: var(--ts-success);
}

.vs-status-chip--muted {
  background: var(--ts-bg-elevated);
  color: var(--ts-text-muted);
}

/* Provider grid */
.vs-provider-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.vs-provider-group + .vs-provider-group {
  margin-top: 0.5rem;
  padding-top: 0.75rem;
  border-top: 1px dashed var(--ts-border-subtle, var(--ts-border));
}

.vs-group-heading {
  font-size: 0.85rem;
  font-weight: 600;
  margin: 0;
  display: flex;
  align-items: center;
  gap: 0.4rem;
  color: var(--ts-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.vs-provider-group--setup .vs-group-heading {
  color: var(--ts-warning, var(--ts-text-secondary));
}

.vs-group-hint {
  margin: 0 0 0.25rem 0;
  font-size: 0.8rem;
  color: var(--ts-text-muted);
  line-height: 1.4;
}

.vs-provider-grid {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 0.75rem;
}

.vs-provider-card {
  background: var(--ts-bg-base);
  border: 1px solid var(--ts-border);
  border-radius: 10px;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  transition: border-color var(--ts-transition-fast), background var(--ts-transition-fast);
}

.vs-provider-card--active {
  border-color: var(--ts-accent-violet);
  background: var(--ts-bg-selected);
  box-shadow: 0 0 0 1px var(--ts-accent-violet);
}

.vs-provider-card--coming-soon {
  opacity: 0.65;
}

.vs-provider-card__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.vs-provider-card__title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.95rem;
  color: var(--ts-text-primary);
}

.vs-provider-card__icon {
  font-size: 1.1rem;
  line-height: 1;
}

.vs-provider-card__pills {
  display: flex;
  gap: 0.3rem;
  flex-wrap: wrap;
}

.vs-pill {
  font-size: 0.65rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  padding: 0.15rem 0.5rem;
  border-radius: 999px;
  text-transform: uppercase;
  white-space: nowrap;
}

.vs-pill--default {
  background: var(--ts-accent-violet);
  color: var(--ts-text-on-accent);
}

.vs-pill--neutral {
  background: var(--ts-bg-elevated);
  color: var(--ts-text-secondary);
}

.vs-pill--warn {
  background: var(--ts-warning-bg);
  color: var(--ts-warning);
}

.vs-pill--muted {
  background: var(--ts-bg-elevated);
  color: var(--ts-text-muted);
}

.vs-provider-card__desc {
  margin: 0;
  font-size: 0.82rem;
  line-height: 1.4;
  color: var(--ts-text-secondary);
}

.vs-provider-card__voice {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  font-size: 0.8rem;
}

.vs-provider-card__voice label {
  color: var(--ts-text-secondary);
}

.vs-provider-card__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  margin-top: 0.25rem;
}

/* Form */
.vs-form {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.vs-form label {
  font-size: 0.8rem;
  color: var(--ts-text-secondary);
}

.vs-input {
  padding: 0.5rem 0.75rem;
  background: var(--ts-bg-input);
  border: 1px solid var(--ts-border-medium);
  border-radius: 6px;
  color: var(--ts-text-primary);
  font-size: 0.85rem;
  outline: none;
  transition: border-color var(--ts-transition-fast), box-shadow var(--ts-transition-fast);
}

.vs-input:focus {
  border-color: var(--ts-accent-violet);
  box-shadow: 0 0 0 3px var(--ts-accent-glow);
}

.vs-form__actions {
  display: flex;
  gap: 0.4rem;
  flex-wrap: wrap;
  margin-top: 0.4rem;
}

/* Alert / inline message */
.vs-alert {
  font-size: 0.8rem;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  line-height: 1.4;
}

.vs-alert--ok {
  background: var(--ts-success-bg);
  color: var(--ts-success);
}

.vs-alert--error {
  background: var(--ts-error-bg);
  color: var(--ts-error);
}

.vs-footer-card {
  background: var(--ts-bg-base);
}

.vs-footer-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.vs-footer-text {
  flex: 1;
  min-width: 0;
}

.vs-footer-label {
  display: block;
  color: var(--ts-fg-default);
  font-weight: 600;
  margin-bottom: 0.25rem;
}

.vs-desc--inline {
  margin: 0;
}

.vs-test-sample {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  margin: 0.75rem 0 1rem;
  padding: 0.75rem;
  background: var(--ts-bg-elevated, var(--ts-bg-base));
  border: 1px solid var(--ts-border-subtle, transparent);
  border-radius: 0.5rem;
}

.vs-test-sample__label {
  font-weight: 600;
  color: var(--ts-fg-default);
  font-size: 0.875rem;
}

.vs-test-sample__textarea {
  resize: vertical;
  min-height: 2.5rem;
  font-family: inherit;
}

.vs-test-sample__hint {
  margin: 0;
  font-size: 0.8125rem;
  color: var(--ts-fg-muted);
}

@media (max-width: 640px) {
  .voice-setup {
    padding: 1rem;
    gap: 1rem;
  }
  .vs-card {
    padding: 1rem;
  }
  .vs-provider-grid {
    grid-template-columns: 1fr;
  }
}
</style>
