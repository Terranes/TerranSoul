<template>
  <Teleport to="body">
    <div
      class="ts-hermes-overlay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="hermes-dispatch-title"
      data-testid="hermes-dispatch-dialog"
      @click.self="$emit('close')"
    >
      <div class="ts-hermes-card">
        <header>
          <h3 id="hermes-dispatch-title">
            Dispatch Hermes Job
          </h3>
          <button
            type="button"
            class="close"
            aria-label="Close dispatch dialog"
            @click="$emit('close')"
          >
            ✕
          </button>
        </header>

        <p class="hint">
          Send a task to Hermes Desktop. Each dispatch starts one staff
          worker in Hermes; TerranSoul tracks progress in the notification
          panel and pops a notification on completion.
        </p>

        <section
          class="office-status"
          :class="officeStatusClass"
          data-testid="hermes-office-status"
          :aria-busy="officeLoading"
        >
          <div class="office-status-row">
            <span class="dot" :aria-hidden="true" />
            <strong>Hermes Office</strong>
            <span class="office-summary">{{ officeSummary }}</span>
            <button
              type="button"
              class="office-refresh"
              :disabled="officeLoading"
              data-testid="hermes-office-refresh"
              @click="refreshOfficeStatus"
            >
              {{ officeLoading ? 'Checking…' : 'Refresh' }}
            </button>
          </div>
          <p
            v-if="officeStatus?.message"
            class="office-message"
          >
            {{ officeStatus.message }}
          </p>
          <p
            v-if="officeStatus && !officeStatus.installed"
            class="office-message"
          >
            Install from
            <a
              href="https://github.com/fathah/hermes-desktop/releases"
              target="_blank"
              rel="noopener noreferrer"
            >github.com/fathah/hermes-desktop</a>.
          </p>
        </section>

        <label class="field">
          <span>Working folder</span>
          <input
            v-model="workingFolder"
            type="text"
            spellcheck="false"
            placeholder="C:\\path\\to\\project"
            data-testid="hermes-folder-input"
          >
        </label>

        <label class="field">
          <span>Prompt</span>
          <textarea
            v-model="prompt"
            rows="6"
            placeholder="What should this staff work on?"
            data-testid="hermes-prompt-input"
          />
        </label>

        <label class="field optional">
          <span>Label (optional)</span>
          <input
            v-model="label"
            type="text"
            placeholder="Defaults to first line of prompt"
          >
        </label>

        <p
          v-if="error"
          class="error"
          role="alert"
        >
          {{ error }}
        </p>

        <footer>
          <button
            type="button"
            class="secondary"
            @click="$emit('close')"
          >
            Cancel
          </button>
          <button
            type="button"
            class="primary"
            :disabled="!canSubmit || submitting"
            data-testid="hermes-dispatch-submit"
            @click="submit"
          >
            {{ submitting ? 'Dispatching…' : 'Dispatch' }}
          </button>
        </footer>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useNotificationsStore, type HermesOfficeStatus } from '../stores/notifications';

const emit = defineEmits<{ close: [] }>();
const store = useNotificationsStore();

const workingFolder = ref('');
const prompt = ref('');
const label = ref('');
const submitting = ref(false);
const error = ref<string | null>(null);

const officeStatus = ref<HermesOfficeStatus | null>(null);
const officeLoading = ref(false);

const officeStatusClass = computed(() => {
  if (!officeStatus.value) return 'unknown';
  if (officeStatus.value.installed && officeStatus.value.gateway_running) return 'ok';
  if (officeStatus.value.installed) return 'warn';
  return 'error';
});

const officeSummary = computed(() => {
  if (officeLoading.value && !officeStatus.value) return 'Checking…';
  if (!officeStatus.value) return 'Unknown';
  if (officeStatus.value.installed && officeStatus.value.gateway_running) return 'Running';
  if (officeStatus.value.installed) return 'Installed (gateway offline)';
  return 'Not installed';
});

async function refreshOfficeStatus() {
  officeLoading.value = true;
  try {
    officeStatus.value = await store.fetchHermesOfficeStatus();
  } catch (e) {
    officeStatus.value = {
      installed: false,
      install_path: null,
      gateway_running: false,
      gateway_url: 'http://127.0.0.1:8642',
      message: String(e),
    };
  } finally {
    officeLoading.value = false;
  }
}

onMounted(() => {
  void refreshOfficeStatus();
});

const canSubmit = computed(
  () => workingFolder.value.trim().length > 0 && prompt.value.trim().length > 0,
);

async function submit() {
  if (!canSubmit.value) return;
  error.value = null;
  submitting.value = true;
  try {
    await store.dispatchHermesJob({
      prompt: prompt.value,
      working_folder: workingFolder.value,
      label: label.value.trim() || undefined,
    });
    emit('close');
  } catch (e) {
    error.value = String(e);
  } finally {
    submitting.value = false;
  }
}
</script>

<style scoped>
.ts-hermes-overlay {
  position: fixed;
  inset: 0;
  background: var(--ts-bg-backdrop);
  z-index: 1600;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.ts-hermes-card {
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-border);
  border-radius: 10px;
  width: min(520px, 100%);
  padding: 18px;
  color: var(--ts-text-primary);
  display: flex;
  flex-direction: column;
  gap: 12px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.6);
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

header h3 {
  margin: 0;
  color: var(--ts-text-bright);
  font-size: 1rem;
}

.close {
  background: transparent;
  border: none;
  color: var(--ts-text-muted);
  cursor: pointer;
  font-size: 1rem;
}

.hint {
  margin: 0;
  font-size: 0.78rem;
  color: var(--ts-text-secondary);
  line-height: 1.4;
}

.office-status {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 10px;
  border: 1px solid var(--ts-border);
  border-radius: 6px;
  background: var(--ts-bg-elevated, rgba(255, 255, 255, 0.02));
  font-size: 0.78rem;
}
.office-status-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.office-status .dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--ts-text-muted);
  flex-shrink: 0;
}
.office-status.ok .dot {
  background: var(--ts-color-success, #4ade80);
}
.office-status.warn .dot {
  background: var(--ts-color-warning, #fbbf24);
}
.office-status.error .dot {
  background: var(--ts-color-danger, #f87171);
}
.office-status strong {
  color: var(--ts-text-primary);
}
.office-summary {
  color: var(--ts-text-secondary);
  flex: 1 1 auto;
}
.office-refresh {
  background: transparent;
  border: 1px solid var(--ts-border);
  color: var(--ts-text-secondary);
  font-size: 0.72rem;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
}
.office-refresh:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.office-message {
  margin: 0;
  color: var(--ts-text-muted);
  font-size: 0.72rem;
  line-height: 1.4;
}
.office-message a {
  color: var(--ts-text-link, var(--ts-accent, #60a5fa));
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.field > span {
  font-size: 0.7rem;
  color: var(--ts-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.field input,
.field textarea {
  width: 100%;
  background: var(--ts-bg-input);
  border: 1px solid var(--ts-border);
  border-radius: 6px;
  padding: 8px 10px;
  color: var(--ts-text-primary);
  font-family: inherit;
  font-size: 0.85rem;
  box-sizing: border-box;
}

.field input:focus,
.field textarea:focus {
  outline: none;
  border-color: var(--ts-accent);
  box-shadow: 0 0 0 2px var(--ts-accent-glow);
}

.field textarea {
  resize: vertical;
  min-height: 80px;
  font-family: var(--ts-font-mono, ui-monospace, SFMono-Regular, monospace);
}

.folder-row {
  display: flex;
  gap: 6px;
}
.folder-row input {
  flex: 1;
}
.folder-row .pick {
  background: var(--ts-bg-hover);
  border: 1px solid var(--ts-border);
  color: var(--ts-text-primary);
  padding: 0 12px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.78rem;
}
.folder-row .pick:hover:not(:disabled) {
  border-color: var(--ts-accent);
}
.folder-row .pick:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.error {
  margin: 0;
  color: var(--ts-accent-pink);
  font-size: 0.78rem;
  background: rgba(255, 107, 157, 0.08);
  padding: 8px 10px;
  border-radius: 6px;
}

footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

footer button {
  padding: 7px 14px;
  border-radius: 6px;
  border: 1px solid var(--ts-border);
  cursor: pointer;
  font-size: 0.82rem;
}

footer .secondary {
  background: transparent;
  color: var(--ts-text-secondary);
}
footer .secondary:hover {
  color: var(--ts-text-primary);
  border-color: var(--ts-border-strong);
}

footer .primary {
  background: var(--ts-accent);
  border-color: var(--ts-accent);
  color: var(--ts-text-on-accent);
  font-weight: 600;
}
footer .primary:hover:not(:disabled) {
  background: var(--ts-accent-hover);
}
footer .primary:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
