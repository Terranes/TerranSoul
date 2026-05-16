<template>
  <li
    class="vs-provider-card"
    :class="{
      'vs-provider-card--active': isActive,
      'vs-provider-card--coming-soon': needsInstall,
    }"
    :data-testid="`${kind}-provider-${provider.id}`"
  >
    <div class="vs-provider-card__head">
      <div class="vs-provider-card__title">
        <span
          class="vs-provider-card__icon"
          aria-hidden="true"
        >
          {{ icon }}
        </span>
        <strong>{{ provider.display_name }}</strong>
      </div>
      <div class="vs-provider-card__pills">
        <span
          v-if="isActive"
          class="vs-pill vs-pill--default"
        >Default</span>
        <span
          v-if="provider.kind === 'cloud'"
          class="vs-pill vs-pill--neutral"
        >Cloud</span>
        <span
          v-else
          class="vs-pill vs-pill--neutral"
        >Local</span>
        <span
          v-if="needsKey"
          class="vs-pill vs-pill--warn"
        >API key</span>
        <span
          v-if="needsInstall"
          class="vs-pill vs-pill--muted"
        >Download required</span>
      </div>
    </div>
    <p class="vs-provider-card__desc">
      {{ provider.description }}
    </p>

    <div
      v-if="kind === 'tts' && isActive"
      class="vs-provider-card__voice"
    >
      <label :for="`tts-voice-${provider.id}`">Voice</label>
      <input
        :id="`tts-voice-${provider.id}`"
        :list="`tts-voice-list-${provider.id}`"
        type="text"
        class="vs-input"
        :value="ttsVoice ?? ''"
        :placeholder="defaultVoice"
        @change="emit('voiceChange', ($event.target as HTMLInputElement).value)"
      >
      <datalist :id="`tts-voice-list-${provider.id}`">
        <option
          v-for="v in voiceCatalogue"
          :key="v.id"
          :value="v.id"
        >
          {{ v.label }}
        </option>
      </datalist>
    </div>

    <div class="vs-provider-card__actions">
      <button
        v-if="kind === 'tts'"
        type="button"
        class="bp-btn bp-btn--ghost"
        :disabled="testing || needsInstall"
        :data-testid="`test-tts-${provider.id}`"
        @click="emit('test')"
      >
        <span v-if="testing">⏳ Testing…</span>
        <span v-else>🔊 Test</span>
      </button>
      <button
        v-if="!isActive"
        type="button"
        class="bp-btn bp-btn--primary"
        :data-testid="`set-default-${kind}-${provider.id}`"
        @click="emit('setDefault')"
      >
        {{ setupActionLabel }}
      </button>
      <button
        v-else
        type="button"
        class="bp-btn bp-btn--ghost"
        :data-testid="`clear-${kind}-${provider.id}`"
        @click="emit('clearDefault')"
      >
        {{ kind === 'tts' ? 'Disable output' : 'Disable input' }}
      </button>
    </div>

    <div
      v-if="testResult"
      class="vs-alert"
      :class="testResult.ok ? 'vs-alert--ok' : 'vs-alert--error'"
    >
      {{ testResult.message }}
    </div>
  </li>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { VoiceProviderInfo } from '../../types';

export interface VoiceCatalogueEntry {
  id: string;
  label: string;
}

const props = defineProps<{
  provider: VoiceProviderInfo;
  kind: 'tts' | 'asr';
  isActive: boolean;
  /** Whether this provider needs a setup step (install or API key) before it can be used. */
  needsSetup: boolean;
  /** True only while the parent is running a `Test` invocation for this provider. */
  testing?: boolean;
  testResult?: { ok: boolean; message: string } | null;
  ttsVoice?: string | null;
  defaultVoice?: string;
  voiceCatalogue?: ReadonlyArray<VoiceCatalogueEntry>;
}>();

const emit = defineEmits<{
  setDefault: [];
  clearDefault: [];
  test: [];
  voiceChange: [value: string];
}>();

const needsInstall = computed(
  () => props.provider.requires_install && !props.provider.installed,
);
const needsKey = computed(() => props.provider.requires_api_key);

const icon = computed(() => {
  if (needsInstall.value) return '⬇️';
  if (props.provider.kind === 'cloud') return '☁️';
  return '💻';
});

const setupActionLabel = computed(() => {
  if (needsInstall.value) return 'Download & set as default';
  if (props.needsSetup && needsKey.value) return 'Add API key';
  return 'Set as default';
});
</script>

<style scoped>
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
  opacity: 0.85;
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
</style>
