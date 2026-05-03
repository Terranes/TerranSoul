<template>
  <section
    class="browser-auth-panel"
    aria-labelledby="browser-auth-title"
  >
    <div>
      <p class="card-kicker">
        Vercel test environment
      </p>
      <h2 id="browser-auth-title">
        One click. No installs. No keys to type.
      </h2>
      <p>
        Pick a familiar account button and TerranSoul remembers that
        browser-only test session in Pinia + localStorage. The live web
        demo still runs without a backend, so Vercel only serves static
        assets while each visitor uses client-side state.
      </p>
      <p
        v-if="browserAuthSession"
        class="auth-status"
      >
        Connected: <strong>{{ browserAuthSession.label }}</strong>
      </p>
    </div>
    <div class="auth-actions">
      <button
        v-for="provider in browserAuthProviders"
        :key="provider.id"
        type="button"
        class="auth-action"
        :class="{ active: browserAuthSession?.providerId === provider.id }"
        @click="authoriseProvider(provider.id)"
      >
        <span>{{ provider.label }}</span>
        <small>{{ provider.privacyLabel }}</small>
      </button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useBrainStore, type BrowserAuthProviderId } from '../stores/brain';

const brain = useBrainStore();
const browserAuthProviders = computed(() => brain.browserAuthProviders);
const browserAuthSession = computed(() => brain.browserAuthSession);

function authoriseProvider(providerId: BrowserAuthProviderId): void {
  brain.authoriseBrowserProvider(providerId);
}
</script>

<style scoped>
.browser-auth-panel {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(220px, 0.82fr);
  gap: var(--ts-space-md);
  margin-top: var(--ts-space-xl);
  padding: var(--ts-space-md);
  border: 1px solid color-mix(in srgb, var(--ts-accent) 28%, var(--ts-border));
  border-radius: var(--ts-radius-xl);
  background: color-mix(in srgb, var(--ts-bg-panel) 72%, transparent);
  box-shadow: var(--ts-shadow-md);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

.browser-auth-panel h2 {
  margin-bottom: 0.55rem;
  font-size: clamp(1.25rem, 2.4vw, 1.85rem);
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

.auth-actions {
  display: grid;
  gap: var(--ts-space-sm);
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

.auth-action small {
  color: var(--ts-text-secondary);
  line-height: 1.35;
}

@media (max-width: 720px) {
  .browser-auth-panel {
    grid-template-columns: 1fr;
  }
}
</style>
