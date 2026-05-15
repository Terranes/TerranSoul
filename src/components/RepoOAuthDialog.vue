<script setup lang="ts">
/**
 * BRAIN-REPO-RAG-1e — GitHub OAuth Device Flow dialog.
 *
 * Walks the user through GitHub's RFC 8628 device authorization grant:
 *   1. Click "Connect GitHub" → backend requests a device code.
 *   2. Dialog surfaces `user_code` + a copy button + a link to
 *      `verification_uri` so the user enters the code in a browser.
 *   3. Frontend polls `repo_oauth_github_poll` every `interval` seconds
 *      until the user completes auth (or expires/denies).
 *   4. On success, the token is persisted to
 *      `<data_dir>/oauth/github.json` with FS-permission hardening so
 *      future private-repo clones can authenticate transparently.
 *
 * The token is never read or rendered in this component — only the
 * `RepoOAuthStatus` summary (`linked`, `scope`, `expired`) is shown.
 */
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { useMemorySourcesStore } from '../stores/memory-sources';

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ (e: 'close'): void }>();

const store = useMemorySourcesStore();

const phase = ref<'idle' | 'awaiting' | 'success' | 'expired' | 'denied' | 'error'>('idle');
const errorMessage = ref<string>('');
const copied = ref(false);
let pollTimer: ReturnType<typeof setTimeout> | null = null;

function stopPolling(): void {
  if (pollTimer !== null) {
    clearTimeout(pollTimer);
    pollTimer = null;
  }
}

async function startFlow(): Promise<void> {
  errorMessage.value = '';
  phase.value = 'idle';
  try {
    const resp = await store.startGitHubOAuth('repo');
    phase.value = 'awaiting';
    schedulePoll(Math.max(resp.interval, 1));
  } catch (e) {
    phase.value = 'error';
    errorMessage.value = String(e);
  }
}

function schedulePoll(intervalSeconds: number): void {
  stopPolling();
  pollTimer = setTimeout(() => {
    void poll(intervalSeconds);
  }, intervalSeconds * 1000);
}

async function poll(intervalSeconds: number): Promise<void> {
  const result = await store.pollGitHubOAuth();
  if (result.status === 'success') {
    phase.value = 'success';
    stopPolling();
    return;
  }
  if (result.status === 'expired') {
    phase.value = 'expired';
    stopPolling();
    return;
  }
  if (result.status === 'denied') {
    phase.value = 'denied';
    stopPolling();
    return;
  }
  if (result.status === 'error') {
    phase.value = 'error';
    errorMessage.value = result.message;
    stopPolling();
    return;
  }
  // pending → keep polling
  schedulePoll(intervalSeconds);
}

async function copyUserCode(): Promise<void> {
  const code = store.oauthDeviceCode?.user_code ?? '';
  if (!code) return;
  try {
    await navigator.clipboard.writeText(code);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1500);
  } catch {
    // clipboard unavailable — user can still type the code manually.
  }
}

async function unlink(): Promise<void> {
  await store.clearGitHubOAuth();
  phase.value = 'idle';
}

function closeDialog(): void {
  stopPolling();
  emit('close');
}

onMounted(() => {
  void store.fetchGitHubOAuthStatus();
});

onBeforeUnmount(() => {
  stopPolling();
});
</script>

<template>
  <div
    v-if="props.open"
    class="repo-oauth-overlay"
    data-testid="repo-oauth-dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="repo-oauth-title"
  >
    <div class="repo-oauth-card">
      <header class="repo-oauth-head">
        <h2 id="repo-oauth-title" class="repo-oauth-title">
          Connect GitHub for private repos
        </h2>
        <button
          type="button"
          class="repo-oauth-close"
          aria-label="Close"
          @click="closeDialog"
        >
          ✕
        </button>
      </header>

      <p class="repo-oauth-blurb">
        Authorize TerranSoul through GitHub's device flow. The token is
        stored locally under <code>oauth/github.json</code> with
        user-only FS permissions and is used only when cloning
        repositories you add as memory sources.
      </p>

      <section v-if="store.oauthStatus?.linked" class="repo-oauth-status">
        <div class="repo-oauth-status-row">
          <span class="repo-oauth-status-label">Linked</span>
          <span class="repo-oauth-status-value">Yes</span>
        </div>
        <div class="repo-oauth-status-row">
          <span class="repo-oauth-status-label">Scope</span>
          <span class="repo-oauth-status-value">{{ store.oauthStatus.scope || '—' }}</span>
        </div>
        <div class="repo-oauth-status-row">
          <span class="repo-oauth-status-label">Expired</span>
          <span class="repo-oauth-status-value">{{ store.oauthStatus.expired ? 'Yes' : 'No' }}</span>
        </div>
        <button type="button" class="repo-oauth-action repo-oauth-action--danger" @click="unlink">
          Unlink GitHub
        </button>
      </section>

      <section v-else-if="phase === 'idle'" class="repo-oauth-idle">
        <button
          type="button"
          class="repo-oauth-action repo-oauth-action--primary"
          data-testid="repo-oauth-start"
          @click="startFlow"
        >
          Connect GitHub
        </button>
      </section>

      <section
        v-else-if="phase === 'awaiting' && store.oauthDeviceCode"
        class="repo-oauth-awaiting"
      >
        <p class="repo-oauth-instructions">
          Visit
          <a
            :href="store.oauthDeviceCode.verification_uri"
            target="_blank"
            rel="noopener noreferrer"
          >{{ store.oauthDeviceCode.verification_uri }}</a>
          and enter the code:
        </p>
        <div class="repo-oauth-code-row">
          <code class="repo-oauth-code" data-testid="repo-oauth-user-code">
            {{ store.oauthDeviceCode.user_code }}
          </code>
          <button type="button" class="repo-oauth-action" @click="copyUserCode">
            {{ copied ? 'Copied' : 'Copy' }}
          </button>
        </div>
        <p class="repo-oauth-poll-hint">
          Waiting for authorization… (polling every
          {{ store.oauthDeviceCode.interval }}s)
        </p>
      </section>

      <section v-else-if="phase === 'success'" class="repo-oauth-success">
        <p>GitHub linked successfully.</p>
      </section>

      <section v-else-if="phase === 'expired'" class="repo-oauth-warn">
        <p>The device code expired before authorization completed.</p>
        <button type="button" class="repo-oauth-action" @click="startFlow">Try again</button>
      </section>

      <section v-else-if="phase === 'denied'" class="repo-oauth-warn">
        <p>Authorization was denied.</p>
        <button type="button" class="repo-oauth-action" @click="startFlow">Try again</button>
      </section>

      <section v-else-if="phase === 'error'" class="repo-oauth-error">
        <p>{{ errorMessage || 'Unexpected error.' }}</p>
        <button type="button" class="repo-oauth-action" @click="startFlow">Retry</button>
      </section>
    </div>
  </div>
</template>

<style scoped>
.repo-oauth-overlay {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.55);
  z-index: 9000;
}
.repo-oauth-card {
  background: var(--ts-bg-panel, #1a1a1f);
  color: var(--ts-fg, #e6e6e6);
  border: 1px solid var(--ts-border, #2a2a32);
  border-radius: var(--ts-radius-lg, 12px);
  width: min(440px, 92vw);
  padding: 1.25rem 1.5rem 1.5rem;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
}
.repo-oauth-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.75rem;
}
.repo-oauth-title {
  font-size: 1.05rem;
  margin: 0;
}
.repo-oauth-close {
  background: transparent;
  border: none;
  color: var(--ts-fg-muted, #999);
  font-size: 1rem;
  cursor: pointer;
  padding: 0.25rem 0.5rem;
}
.repo-oauth-close:hover {
  color: var(--ts-fg, #fff);
}
.repo-oauth-blurb {
  font-size: 0.85rem;
  color: var(--ts-fg-muted, #aaa);
  line-height: 1.5;
  margin: 0 0 1rem;
}
.repo-oauth-blurb code {
  font-size: 0.8rem;
  background: var(--ts-bg-code, #111);
  padding: 0.1rem 0.35rem;
  border-radius: 4px;
}
.repo-oauth-status-row {
  display: flex;
  justify-content: space-between;
  padding: 0.35rem 0;
  border-bottom: 1px solid var(--ts-border-soft, #232328);
  font-size: 0.85rem;
}
.repo-oauth-status-label {
  color: var(--ts-fg-muted, #888);
}
.repo-oauth-action {
  margin-top: 0.85rem;
  padding: 0.5rem 0.9rem;
  border-radius: var(--ts-radius-md, 8px);
  border: 1px solid var(--ts-border, #2a2a32);
  background: var(--ts-bg-elevated, #22222a);
  color: var(--ts-fg, #e6e6e6);
  cursor: pointer;
  font-size: 0.85rem;
}
.repo-oauth-action:hover {
  background: var(--ts-bg-elevated-hover, #2c2c36);
}
.repo-oauth-action--primary {
  background: var(--ts-accent, #4a7cff);
  color: var(--ts-on-accent, #fff);
  border-color: transparent;
  font-weight: 600;
}
.repo-oauth-action--danger {
  border-color: var(--ts-danger, #c14a4a);
  color: var(--ts-danger, #d27272);
}
.repo-oauth-instructions {
  font-size: 0.9rem;
  margin: 0 0 0.75rem;
}
.repo-oauth-instructions a {
  color: var(--ts-accent, #4a7cff);
  text-decoration: underline;
  word-break: break-all;
}
.repo-oauth-code-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.85rem;
}
.repo-oauth-code {
  flex: 1;
  font-family: var(--ts-font-mono, monospace);
  font-size: 1.4rem;
  letter-spacing: 0.15em;
  text-align: center;
  background: var(--ts-bg-code, #111);
  padding: 0.65rem 0.5rem;
  border-radius: var(--ts-radius-md, 8px);
  border: 1px solid var(--ts-border, #2a2a32);
}
.repo-oauth-poll-hint {
  font-size: 0.75rem;
  color: var(--ts-fg-muted, #888);
  margin: 0;
}
.repo-oauth-error,
.repo-oauth-warn,
.repo-oauth-success {
  font-size: 0.9rem;
}
.repo-oauth-error p {
  color: var(--ts-danger, #d27272);
}
.repo-oauth-success p {
  color: var(--ts-success, #6fb86f);
}
</style>
