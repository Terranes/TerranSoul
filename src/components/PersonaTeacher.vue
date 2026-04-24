<script setup lang="ts">
/**
 * PersonaTeacher.vue — "Teach an expression" camera panel.
 *
 * Flow:
 *   1. User clicks "Start Camera" → consent dialog shown
 *   2. On accept: camera opens, FaceLandmarker streams weights → live preview on avatar
 *   3. User strikes a pose and clicks "Capture" → snapshot current weights
 *   4. Name the expression + pick a trigger word → Save
 *
 * This panel is only reachable when the `expressions-pack` quest is in
 * discovery mode (the activation gate reads `learnedExpressions.length > 0`).
 */

import { ref, computed, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useCameraCapture } from '../composables/useCameraCapture';
import { usePersonaStore } from '../stores/persona';
import type { VrmExpressionWeights } from '../renderer/face-mirror';
import type { LearnedExpression } from '../stores/persona-types';

// ── Props / Emits ────────────────────────────────────────────────────────────

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  /** Emitted every frame with latest expression weights for live avatar preview. */
  (e: 'mirror-weights', w: VrmExpressionWeights): void;
  /** Emitted when camera stops. */
  (e: 'mirror-stop'): void;
}>();

// ── Stores ───────────────────────────────────────────────────────────────────

const persona = usePersonaStore();
const camera = useCameraCapture();

// ── Local state ──────────────────────────────────────────────────────────────

/** Consent step: ask user before opening camera. */
const showConsent = ref(false);

/** Captured snapshot weights (null = no capture yet). */
const capturedWeights = ref<VrmExpressionWeights | null>(null);

/** Name + trigger for saving. */
const exprName = ref('');
const exprTrigger = ref('');

/** Saving state. */
const saving = ref(false);
const saveError = ref('');

/** rAF handle for mirror loop. */
let rafId = 0;
let lastTime = 0;

// ── Computed ─────────────────────────────────────────────────────────────────

const canSave = computed(() =>
  capturedWeights.value !== null &&
  exprName.value.trim().length > 0 &&
  exprTrigger.value.trim().length > 0,
);

/** Session-scoped camera ID — unique per app session, not per chat. */
const sessionId = `cam-${Date.now()}`;

// ── Camera lifecycle ─────────────────────────────────────────────────────────

/** Step 1: show consent dialog. */
function requestCamera(): void {
  showConsent.value = true;
}

/** Step 2: user accepted → start camera + mirror loop. */
async function acceptConsent(): Promise<void> {
  showConsent.value = false;
  await camera.start(sessionId);
  capturedWeights.value = null;
  exprName.value = '';
  exprTrigger.value = '';
  startMirrorLoop();
}

function declineConsent(): void {
  showConsent.value = false;
}

function startMirrorLoop(): void {
  lastTime = performance.now();
  function loop(): void {
    const now = performance.now();
    const dt = (now - lastTime) / 1000;
    lastTime = now;

    const w = camera.update(dt);
    emit('mirror-weights', w);
    if (camera.active.value) rafId = requestAnimationFrame(loop);
  }
  rafId = requestAnimationFrame(loop);
}

function stopCamera(): void {
  if (rafId) cancelAnimationFrame(rafId);
  rafId = 0;
  camera.stop();
  emit('mirror-stop');
}

// ── Capture + Save ───────────────────────────────────────────────────────────

/** Snapshot the current smoothed weights. */
function capture(): void {
  capturedWeights.value = { ...camera.weights.value };
}

/** Clear captured snapshot and return to live preview. */
function retake(): void {
  capturedWeights.value = null;
  saveError.value = '';
}

/** Save the captured expression to persona store + Tauri backend. */
async function saveExpression(): Promise<void> {
  if (!capturedWeights.value) return;
  saving.value = true;
  saveError.value = '';

  const expr: LearnedExpression = {
    id: crypto.randomUUID(),
    kind: 'expression',
    name: exprName.value.trim(),
    trigger: exprTrigger.value.trim().toLowerCase(),
    weights: {
      happy: capturedWeights.value.happy,
      sad: capturedWeights.value.sad,
      angry: capturedWeights.value.angry,
      relaxed: capturedWeights.value.relaxed,
      surprised: capturedWeights.value.surprised,
      neutral: capturedWeights.value.neutral,
      aa: capturedWeights.value.aa,
      ih: capturedWeights.value.ih,
      ou: capturedWeights.value.ou,
      ee: capturedWeights.value.ee,
      oh: capturedWeights.value.oh,
    },
    blink: capturedWeights.value.blink,
    lookAt: { x: capturedWeights.value.lookAtX, y: capturedWeights.value.lookAtY },
    learnedAt: Date.now(),
  };

  try {
    await invoke('save_learned_expression', { json: JSON.stringify(expr) });
    persona.learnedExpressions = [...persona.learnedExpressions, expr];
    // Reset for another capture
    capturedWeights.value = null;
    exprName.value = '';
    exprTrigger.value = '';
  } catch (e) {
    saveError.value = String(e);
  } finally {
    saving.value = false;
  }
}

// ── Auto-stop when panel hides or chat changes ──────────────────────────────

watch(() => props.visible, (vis) => {
  if (!vis && camera.active.value) stopCamera();
});

onUnmounted(stopCamera);
</script>

<template>
  <div v-if="visible" class="pt-root">
    <h3 class="pt-title">🎭 Teach Expression</h3>

    <!-- ── Consent dialog ──────────────────────────────────────────── -->
    <div v-if="showConsent" class="pt-consent">
      <p class="pt-consent-msg">
        TerranSoul needs camera access to capture your facial expression.<br />
        <strong>This permission is for this session only</strong> — it is never saved.
      </p>
      <div class="pt-consent-actions">
        <button class="pt-btn pt-btn--primary" @click="acceptConsent">
          Allow This Session
        </button>
        <button class="pt-btn pt-btn--ghost" @click="declineConsent">
          Cancel
        </button>
      </div>
    </div>

    <!-- ── Camera off ──────────────────────────────────────────────── -->
    <div v-else-if="!camera.active.value" class="pt-start">
      <button class="pt-btn pt-btn--primary" @click="requestCamera">
        📷 Start Camera
      </button>
      <p class="pt-hint">Camera is used only during this session.</p>
    </div>

    <!-- ── Camera active ───────────────────────────────────────────── -->
    <div v-else class="pt-active">
      <!-- Live badge -->
      <div class="pt-live-badge">
        <span class="pt-live-dot" /> CAMERA LIVE
      </div>

      <!-- Loading indicator -->
      <div v-if="camera.loading.value" class="pt-loading">
        Loading face tracker…
      </div>

      <!-- Video preview -->
      <video
        v-if="camera.videoEl.value"
        ref="previewVideo"
        class="pt-video"
        :srcObject="camera.videoEl.value.srcObject"
        autoplay
        playsinline
        muted
      />

      <!-- Capture / save flow -->
      <div v-if="!capturedWeights" class="pt-capture-controls">
        <button class="pt-btn pt-btn--accent" @click="capture">
          📸 Capture Pose
        </button>
        <button class="pt-btn pt-btn--ghost" @click="stopCamera">
          Stop Camera
        </button>
      </div>

      <div v-else class="pt-save-form">
        <p class="pt-captured-label">✅ Expression captured!</p>

        <label class="pt-field">
          <span>Name</span>
          <input
            v-model="exprName"
            type="text"
            placeholder="e.g. Smirk, Eyebrow Raise"
            maxlength="50"
            class="pt-input"
          />
        </label>

        <label class="pt-field">
          <span>Trigger word</span>
          <input
            v-model="exprTrigger"
            type="text"
            placeholder="e.g. smirk, wink"
            maxlength="30"
            class="pt-input"
          />
        </label>

        <p v-if="saveError" class="pt-error">{{ saveError }}</p>

        <div class="pt-save-actions">
          <button
            class="pt-btn pt-btn--primary"
            :disabled="!canSave || saving"
            @click="saveExpression"
          >
            {{ saving ? 'Saving…' : '💾 Save Expression' }}
          </button>
          <button class="pt-btn pt-btn--ghost" @click="retake">
            ↩ Retake
          </button>
        </div>
      </div>
    </div>

    <!-- ── Saved expressions list ────────────────────────────────── -->
    <div v-if="persona.learnedExpressions.length > 0" class="pt-saved">
      <h4 class="pt-saved-title">Saved Expressions ({{ persona.learnedExpressions.length }})</h4>
      <ul class="pt-saved-list">
        <li v-for="expr in persona.learnedExpressions" :key="expr.id" class="pt-saved-item">
          <span class="pt-saved-name">{{ expr.name }}</span>
          <code class="pt-saved-trigger">{{ expr.trigger }}</code>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.pt-root {
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.pt-title {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--ts-text-primary, #fff);
  margin: 0;
}

/* ── Consent dialog ──────────────────────────────────────────── */
.pt-consent {
  background: var(--ts-bg-secondary, #1a1a2e);
  border: 1px solid var(--ts-border, #333);
  border-radius: 0.75rem;
  padding: 1rem;
}
.pt-consent-msg {
  font-size: 0.85rem;
  color: var(--ts-text-secondary, #aaa);
  line-height: 1.5;
  margin: 0 0 0.75rem;
}
.pt-consent-actions {
  display: flex;
  gap: 0.5rem;
}

/* ── Buttons ────────────────────────────────────────────────── */
.pt-btn {
  padding: 0.5rem 1rem;
  border-radius: 0.5rem;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  border: none;
  transition: background 0.15s, opacity 0.15s;
}
.pt-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.pt-btn--primary {
  background: var(--ts-accent, #6366f1);
  color: #fff;
}
.pt-btn--primary:hover:not(:disabled) {
  background: var(--ts-accent-hover, #818cf8);
}
.pt-btn--accent {
  background: var(--ts-success, #22c55e);
  color: #fff;
}
.pt-btn--accent:hover:not(:disabled) {
  background: #16a34a;
}
.pt-btn--ghost {
  background: transparent;
  color: var(--ts-text-secondary, #aaa);
  border: 1px solid var(--ts-border, #333);
}
.pt-btn--ghost:hover:not(:disabled) {
  background: var(--ts-bg-secondary, #1a1a2e);
}

/* ── Start / hint ────────────────────────────────────────────── */
.pt-start {
  text-align: center;
  padding: 1.5rem 0;
}
.pt-hint {
  font-size: 0.75rem;
  color: var(--ts-text-muted, #666);
  margin: 0.5rem 0 0;
}

/* ── Camera active ───────────────────────────────────────────── */
.pt-active {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.pt-live-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: #ef4444;
  text-transform: uppercase;
}
.pt-live-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #ef4444;
  animation: pt-pulse 1.2s infinite;
}
@keyframes pt-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
.pt-loading {
  font-size: 0.8rem;
  color: var(--ts-text-secondary, #aaa);
  padding: 0.5rem 0;
}
.pt-video {
  width: 100%;
  max-width: 320px;
  border-radius: 0.75rem;
  transform: scaleX(-1);
  background: #000;
}

/* ── Capture / save ──────────────────────────────────────────── */
.pt-capture-controls {
  display: flex;
  gap: 0.5rem;
}
.pt-save-form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.pt-captured-label {
  font-size: 0.85rem;
  color: var(--ts-success, #22c55e);
  font-weight: 600;
  margin: 0;
}
.pt-field {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  font-size: 0.8rem;
  color: var(--ts-text-secondary, #aaa);
}
.pt-input {
  padding: 0.4rem 0.6rem;
  border-radius: 0.4rem;
  border: 1px solid var(--ts-border, #333);
  background: var(--ts-bg-primary, #0a0a1a);
  color: var(--ts-text-primary, #fff);
  font-size: 0.85rem;
}
.pt-error {
  color: #ef4444;
  font-size: 0.75rem;
  margin: 0;
}
.pt-save-actions {
  display: flex;
  gap: 0.5rem;
}

/* ── Saved expressions ───────────────────────────────────────── */
.pt-saved {
  border-top: 1px solid var(--ts-border, #333);
  padding-top: 0.75rem;
}
.pt-saved-title {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--ts-text-primary, #fff);
  margin: 0 0 0.5rem;
}
.pt-saved-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}
.pt-saved-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.3rem 0.5rem;
  background: var(--ts-bg-secondary, #1a1a2e);
  border-radius: 0.4rem;
  font-size: 0.8rem;
}
.pt-saved-name {
  color: var(--ts-text-primary, #fff);
}
.pt-saved-trigger {
  font-size: 0.7rem;
  color: var(--ts-accent, #6366f1);
  background: var(--ts-bg-primary, #0a0a1a);
  padding: 0.1rem 0.4rem;
  border-radius: 0.25rem;
}
</style>
