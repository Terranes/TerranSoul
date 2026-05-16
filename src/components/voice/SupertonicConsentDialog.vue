<template>
  <Teleport to="body">
    <Transition name="sc-fade">
      <div
        v-if="visible"
        class="sc-backdrop"
        role="dialog"
        aria-modal="true"
        aria-labelledby="sc-title"
        :data-testid="testid"
        @click.self="onDismiss"
        @keydown.esc="onDismiss"
      >
        <div
          class="sc-card"
          tabindex="-1"
        >
          <header class="sc-header">
            <span
              class="sc-icon"
              aria-hidden="true"
            >🎙️</span>
            <h2
              id="sc-title"
              class="sc-title"
            >
              Install Supertonic on-device TTS?
            </h2>
          </header>

          <!-- ── Consent stage ───────────────────────────────────────────── -->
          <div
            v-if="stage === 'consent'"
            class="sc-body"
          >
            <p class="sc-lead">
              Supertonic is a neural text-to-speech model that runs
              <strong>entirely on your device</strong>. No audio, prompts,
              or telemetry leave your computer.
            </p>

            <dl class="sc-facts">
              <div class="sc-fact">
                <dt>Download size</dt>
                <dd>~268&nbsp;MB (first run only)</dd>
              </div>
              <div class="sc-fact">
                <dt>License</dt>
                <dd>
                  Model weights: <strong>OpenRAIL-M v1</strong> ·
                  Sample code: MIT
                </dd>
              </div>
              <div class="sc-fact">
                <dt>Runtime</dt>
                <dd>ONNX Runtime in-process — no Python sidecar</dd>
              </div>
            </dl>

            <h3 class="sc-h3">
              Use restrictions (OpenRAIL-M v1, plain English)
            </h3>
            <p class="sc-hint">
              By accepting, you agree these restrictions apply to your use
              of the model and to anything you redistribute that uses it:
            </p>
            <ul class="sc-restrictions">
              <li>No discrimination, harassment, or targeting of individuals or groups.</li>
              <li>No mass surveillance or covert audio impersonation.</li>
              <li>No malicious mis- or disinformation.</li>
              <li>No CSAM or non-consensual intimate imagery/audio.</li>
              <li>No automated legal, medical, or financial advice without human review.</li>
              <li>The same restrictions travel with any redistribution.</li>
            </ul>

            <p class="sc-links">
              <a
                :href="modelCardUrl"
                target="_blank"
                rel="noopener noreferrer"
                data-testid="sc-model-card-link"
              >Upstream model card ↗</a>
              <span class="sc-sep">·</span>
              <a
                :href="licensingAuditUrl"
                target="_blank"
                rel="noopener noreferrer"
                data-testid="sc-licensing-link"
              >TerranSoul licensing audit ↗</a>
            </p>
          </div>

          <!-- ── Progress stage ──────────────────────────────────────────── -->
          <div
            v-else-if="stage === 'downloading'"
            class="sc-body"
          >
            <p class="sc-lead">
              Downloading Supertonic model files…
            </p>
            <div
              class="sc-progress"
              role="progressbar"
              :aria-valuenow="overallPercent"
              aria-valuemin="0"
              aria-valuemax="100"
              data-testid="sc-progress-bar"
            >
              <div
                class="sc-progress__fill"
                :style="{ width: `${overallPercent}%` }"
              />
            </div>
            <p
              class="sc-progress__label"
              data-testid="sc-progress-label"
            >
              {{ overallPercent }}% — file {{ fileIndex + 1 }} of {{ fileCount }}
              <span
                v-if="currentFile"
                class="sc-progress__file"
              >({{ currentFile }})</span>
            </p>
            <p class="sc-hint">
              You can hide this dialog; the download will continue in the
              background. The model will be ready when it finishes.
            </p>
          </div>

          <!-- ── Error stage ─────────────────────────────────────────────── -->
          <div
            v-else-if="stage === 'error'"
            class="sc-body"
          >
            <div
              class="sc-error"
              role="alert"
              data-testid="sc-error"
            >
              ❌ {{ errorMessage }}
            </div>
            <p class="sc-hint">
              {{ errorHint }}
            </p>
          </div>

          <!-- ── Done stage ──────────────────────────────────────────────── -->
          <div
            v-else
            class="sc-body"
          >
            <p
              class="sc-done"
              data-testid="sc-done"
            >
              ✅ Supertonic installed. It will be used for voice output.
            </p>
          </div>

          <footer class="sc-actions">
            <template v-if="stage === 'consent'">
              <button
                ref="cancelBtnRef"
                class="bp-btn bp-btn--ghost"
                type="button"
                data-testid="sc-cancel"
                @click="onCancel"
              >
                Cancel
              </button>
              <button
                class="bp-btn bp-btn--primary"
                type="button"
                data-testid="sc-accept"
                @click="onAccept"
              >
                Accept &amp; download (~268&nbsp;MB)
              </button>
            </template>
            <template v-else-if="stage === 'downloading'">
              <button
                class="bp-btn bp-btn--ghost"
                type="button"
                data-testid="sc-hide"
                @click="onDismiss"
              >
                Hide
              </button>
            </template>
            <template v-else-if="stage === 'error'">
              <button
                class="bp-btn bp-btn--ghost"
                type="button"
                data-testid="sc-error-close"
                @click="onDismiss"
              >
                Close
              </button>
              <button
                class="bp-btn bp-btn--primary"
                type="button"
                data-testid="sc-retry"
                @click="onAccept"
              >
                Retry
              </button>
            </template>
            <template v-else>
              <button
                class="bp-btn bp-btn--primary"
                type="button"
                data-testid="sc-done-close"
                @click="onDismiss"
              >
                Done
              </button>
            </template>
          </footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';

export interface SupertonicDownloadProgress {
  current_file: string;
  current_bytes: number;
  current_total: number;
  overall_bytes: number;
  overall_total: number;
  file_index: number;
  file_count: number;
}

const props = withDefaults(
  defineProps<{
    visible: boolean;
    /** External-controlled stage. Parent owns the state machine so the
     *  dialog can be hidden + re-opened without losing in-progress state. */
    stage: 'consent' | 'downloading' | 'error' | 'done';
    progress?: SupertonicDownloadProgress | null;
    errorMessage?: string;
    modelCardUrl?: string;
    licensingAuditUrl?: string;
    testid?: string;
  }>(),
  {
    progress: null,
    errorMessage: '',
    modelCardUrl: 'https://huggingface.co/Supertone/supertonic-2',
    licensingAuditUrl: 'https://github.com/TerranSoul/TerranSoul/blob/master/docs/licensing-audit.md#supertonic-tts--model-weights-supertonesupertonic-3',
    testid: 'supertonic-consent-dialog',
  },
);

const emit = defineEmits<{
  accept: [];
  cancel: [];
  dismiss: [];
}>();

const cancelBtnRef = ref<HTMLButtonElement | null>(null);

const overallPercent = computed(() => {
  const p = props.progress;
  if (!p || p.overall_total === 0) return 0;
  return Math.min(100, Math.floor((p.overall_bytes / p.overall_total) * 100));
});
const fileIndex = computed(() => props.progress?.file_index ?? 0);
const fileCount = computed(() => props.progress?.file_count ?? 0);
const currentFile = computed(() => props.progress?.current_file ?? '');

const errorHint = computed(() => {
  const msg = (props.errorMessage || '').toLowerCase();
  if (msg.includes('network') || msg.includes('http') || msg.includes('offline')) {
    return 'Check your internet connection and retry. Model files are hosted on Hugging Face.';
  }
  if (msg.includes('size mismatch') || msg.includes('hash') || msg.includes('sha')) {
    return 'A downloaded file failed integrity verification. Retry to redownload.';
  }
  if (msg.includes('io') || msg.includes('disk') || msg.includes('space') || msg.includes('permission')) {
    return 'Disk write failed. Ensure you have ~300 MB free and that TerranSoul has write access to its data directory.';
  }
  return 'Retry the download, or close and try again later.';
});

function onAccept(): void {
  emit('accept');
}
function onCancel(): void {
  emit('cancel');
}
function onDismiss(): void {
  // Treated as cancel when no download in progress; otherwise as a hide.
  if (props.stage === 'consent') {
    emit('cancel');
  } else {
    emit('dismiss');
  }
}

watch(
  () => props.visible,
  async (v) => {
    if (v && props.stage === 'consent') {
      await nextTick();
      cancelBtnRef.value?.focus();
    }
  },
);
</script>

<style scoped>
.sc-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.72);
  backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9500;
  padding: 16px;
  pointer-events: auto;
}

.sc-card {
  background: var(--ts-surface, #1a1626);
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.12));
  border-radius: 14px;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.55);
  width: min(560px, 94vw);
  max-height: 90vh;
  overflow-y: auto;
  color: var(--ts-text-primary, #eaecf4);
  outline: none;
  padding: 20px 22px;
}

.sc-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.sc-icon {
  font-size: 22px;
}

.sc-title {
  font-size: 1.15rem;
  font-weight: 600;
  margin: 0;
  color: var(--ts-text-primary, #eaecf4);
}

.sc-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin: 12px 0 16px;
}

.sc-lead {
  margin: 0;
  font-size: 0.95rem;
  line-height: 1.5;
  color: var(--ts-text-primary, #eaecf4);
}

.sc-facts {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px 16px;
  margin: 0;
  padding: 12px;
  background: var(--ts-surface-sunken, rgba(255, 255, 255, 0.04));
  border-radius: 8px;
  font-size: 0.85rem;
}

.sc-fact {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.sc-fact dt {
  font-size: 0.75rem;
  color: var(--ts-text-muted, #9ba3b4);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.sc-fact dd {
  margin: 0;
  color: var(--ts-text-primary, #eaecf4);
}

.sc-h3 {
  font-size: 0.95rem;
  font-weight: 600;
  margin: 4px 0 0;
  color: var(--ts-text-primary, #eaecf4);
}

.sc-hint {
  font-size: 0.8rem;
  color: var(--ts-text-muted, #9ba3b4);
  margin: 0;
  line-height: 1.5;
}

.sc-restrictions {
  margin: 0;
  padding-left: 20px;
  font-size: 0.85rem;
  line-height: 1.6;
  color: var(--ts-text-secondary, #c8cdda);
}

.sc-restrictions li {
  margin-bottom: 2px;
}

.sc-links {
  font-size: 0.85rem;
  margin: 0;
}

.sc-links a {
  color: var(--ts-accent, #7aa2f7);
  text-decoration: none;
}

.sc-links a:hover {
  text-decoration: underline;
}

.sc-sep {
  margin: 0 8px;
  color: var(--ts-text-muted, #9ba3b4);
}

.sc-progress {
  width: 100%;
  height: 10px;
  background: var(--ts-surface-sunken, rgba(255, 255, 255, 0.06));
  border-radius: 5px;
  overflow: hidden;
}

.sc-progress__fill {
  height: 100%;
  background: var(--ts-accent, #7aa2f7);
  transition: width 0.2s ease;
}

.sc-progress__label {
  font-size: 0.85rem;
  color: var(--ts-text-secondary, #c8cdda);
  margin: 0;
}

.sc-progress__file {
  color: var(--ts-text-muted, #9ba3b4);
  margin-left: 6px;
}

.sc-error {
  padding: 10px 12px;
  background: rgba(248, 113, 113, 0.1);
  border: 1px solid rgba(248, 113, 113, 0.4);
  border-radius: 6px;
  color: var(--ts-text-primary, #eaecf4);
  font-size: 0.9rem;
}

.sc-done {
  font-size: 0.95rem;
  color: var(--ts-text-primary, #eaecf4);
  margin: 0;
}

.sc-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.sc-fade-enter-active,
.sc-fade-leave-active {
  transition: opacity 0.18s ease;
}
.sc-fade-enter-from,
.sc-fade-leave-to {
  opacity: 0;
}
</style>
