<script setup lang="ts">
/**
 * PersonaTeacher.vue — "Teach an expression / motion" camera panel.
 *
 * Expression flow:
 *   1. User clicks "Start Camera" → consent dialog shown
 *   2. On accept: camera opens, FaceLandmarker streams weights → live preview on avatar
 *   3. User strikes a pose and clicks "Capture" → snapshot current weights
 *   4. Name the expression + pick a trigger word → Save
 *
 * Motion flow:
 *   1. Same camera consent flow
 *   2. PoseLandmarker streams bone rotations → live puppet on avatar
 *   3. User clicks "Record" → frames accumulate at 30 fps
 *   4. User clicks "Stop" → clip saved → name + trigger → Save
 */

import { ref, computed, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useCameraCapture } from '../composables/useCameraCapture';
import { usePersonaStore } from '../stores/persona';
import { PoseMirror, type VrmBonePose } from '../renderer/pose-mirror';
import type { VrmExpressionWeights } from '../renderer/face-mirror';
import type { LearnedExpression, LearnedMotion } from '../stores/persona-types';

// ── Props / Emits ────────────────────────────────────────────────────────────

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  /** Emitted every frame with latest expression weights for live avatar preview. */
  (e: 'mirror-weights', w: VrmExpressionWeights): void;
  /** Emitted every frame with latest bone rotations for live puppet. */
  (e: 'mirror-bones', b: VrmBonePose): void;
  /** Emitted when camera stops. */
  (e: 'mirror-stop'): void;
}>();

// ── Stores ───────────────────────────────────────────────────────────────────

const persona = usePersonaStore();
const camera = useCameraCapture();

// ── Tab mode ─────────────────────────────────────────────────────────────────

type TeachMode = 'expression' | 'motion';
const mode = ref<TeachMode>('expression');

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
  let frameAccum = 0;
  const frameInterval = 1 / RECORD_FPS;

  function loop(): void {
    const now = performance.now();
    const dt = (now - lastTime) / 1000;
    lastTime = now;

    // Expression mirror
    if (mode.value === 'expression' || !poseMirror) {
      const w = camera.update(dt);
      emit('mirror-weights', w);
    }

    // Pose mirror
    if (mode.value === 'motion' && poseMirror && camera.videoEl.value) {
      const bones = poseMirror.update(camera.videoEl.value, dt);
      emit('mirror-bones', bones);

      // Record frames at fixed FPS
      if (motionRecording.value) {
        frameAccum += dt;
        while (frameAccum >= frameInterval) {
          frameAccum -= frameInterval;
          const t = (now - recordStartTime) / 1000;

          // Auto-stop at max duration
          if (t >= MAX_RECORD_SECONDS) {
            stopMotionRecording();
            break;
          }

          const boneSnapshot: Record<string, [number, number, number]> = {};
          for (const [key, val] of Object.entries(bones)) {
            if (val) boneSnapshot[key] = [...val] as [number, number, number];
          }
          motionFrames.value.push({ t, bones: boneSnapshot });
        }
      }
    }

    if (camera.active.value) rafId = requestAnimationFrame(loop);
  }
  rafId = requestAnimationFrame(loop);
}

function stopCamera(): void {
  if (rafId) cancelAnimationFrame(rafId);
  rafId = 0;
  motionRecording.value = false;
  if (poseMirror) {
    poseMirror.dispose();
    poseMirror = null;
  }
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

// ── Motion capture state ─────────────────────────────────────────────────────

let poseMirror: PoseMirror | null = null;
const motionRecording = ref(false);
const motionFrames = ref<LearnedMotion['frames']>([]);
const motionName = ref('');
const motionTrigger = ref('');
const motionDone = ref(false);
let recordStartTime = 0;
const RECORD_FPS = 30;
const MAX_RECORD_SECONDS = 10;

const canSaveMotion = computed(() =>
  motionDone.value &&
  motionFrames.value.length > 0 &&
  motionName.value.trim().length > 0 &&
  motionTrigger.value.trim().length > 0,
);

/** Init PoseMirror on demand (separate from FaceMirror). */
async function ensurePoseMirror(): Promise<void> {
  if (!poseMirror) {
    poseMirror = new PoseMirror();
    await poseMirror.init();
  }
}

/** Start recording motion frames. */
async function startMotionRecording(): Promise<void> {
  await ensurePoseMirror();
  motionFrames.value = [];
  motionDone.value = false;
  motionName.value = '';
  motionTrigger.value = '';
  recordStartTime = performance.now();
  motionRecording.value = true;
}

/** Stop recording and prepare for save. */
function stopMotionRecording(): void {
  motionRecording.value = false;
  motionDone.value = true;
}

/** Discard recorded motion and start over. */
function discardMotion(): void {
  motionFrames.value = [];
  motionDone.value = false;
  motionRecording.value = false;
}

/** Save the recorded motion clip to persona store + Tauri backend. */
async function saveMotion(): Promise<void> {
  if (motionFrames.value.length === 0) return;
  saving.value = true;
  saveError.value = '';

  const durationS = motionFrames.value.length / RECORD_FPS;
  const motion: LearnedMotion = {
    id: crypto.randomUUID(),
    kind: 'motion',
    name: motionName.value.trim(),
    trigger: motionTrigger.value.trim().toLowerCase(),
    fps: RECORD_FPS,
    duration_s: durationS,
    frames: motionFrames.value,
    learnedAt: Date.now(),
  };

  try {
    await invoke('save_learned_motion', { json: JSON.stringify(motion) });
    persona.learnedMotions = [...persona.learnedMotions, motion];
    motionFrames.value = [];
    motionDone.value = false;
    motionName.value = '';
    motionTrigger.value = '';
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
    <!-- ── Mode tabs ───────────────────────────────────────────────── -->
    <div class="pt-tabs">
      <button
        class="pt-tab"
        :class="{ 'pt-tab--active': mode === 'expression' }"
        @click="mode = 'expression'"
      >🎭 Expression</button>
      <button
        class="pt-tab"
        :class="{ 'pt-tab--active': mode === 'motion' }"
        @click="mode = 'motion'"
      >🪩 Motion</button>
    </div>

    <!-- ── Consent dialog ──────────────────────────────────────────── -->
    <div v-if="showConsent" class="pt-consent">
      <p class="pt-consent-msg">
        TerranSoul needs camera access to capture your {{ mode === 'expression' ? 'facial expression' : 'body motion' }}.<br />
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
        <span v-if="motionRecording" class="pt-rec-badge">● REC {{ motionFrames.length }} frames</span>
      </div>

      <!-- Loading indicator -->
      <div v-if="camera.loading.value" class="pt-loading">
        Loading {{ mode === 'expression' ? 'face tracker' : 'pose tracker' }}…
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

      <!-- ═══ Expression mode ═══════════════════════════════════════ -->
      <template v-if="mode === 'expression'">
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
      </template>

      <!-- ═══ Motion mode ═══════════════════════════════════════════ -->
      <template v-else>
        <div v-if="!motionDone" class="pt-capture-controls">
          <button
            v-if="!motionRecording"
            class="pt-btn pt-btn--accent"
            @click="startMotionRecording"
          >
            ⏺ Start Recording
          </button>
          <button
            v-else
            class="pt-btn pt-btn--primary"
            @click="stopMotionRecording"
          >
            ⏹ Stop ({{ Math.min(motionFrames.length / RECORD_FPS, MAX_RECORD_SECONDS).toFixed(1) }}s)
          </button>
          <button class="pt-btn pt-btn--ghost" @click="stopCamera">
            Stop Camera
          </button>
        </div>

        <div v-else class="pt-save-form">
          <p class="pt-captured-label">
            ✅ Motion captured! {{ motionFrames.length }} frames ({{ (motionFrames.length / RECORD_FPS).toFixed(1) }}s)
          </p>

          <label class="pt-field">
            <span>Name</span>
            <input
              v-model="motionName"
              type="text"
              placeholder="e.g. Shrug, Wave"
              maxlength="50"
              class="pt-input"
            />
          </label>

          <label class="pt-field">
            <span>Trigger word</span>
            <input
              v-model="motionTrigger"
              type="text"
              placeholder="e.g. shrug, wave"
              maxlength="30"
              class="pt-input"
            />
          </label>

          <p v-if="saveError" class="pt-error">{{ saveError }}</p>

          <div class="pt-save-actions">
            <button
              class="pt-btn pt-btn--primary"
              :disabled="!canSaveMotion || saving"
              @click="saveMotion"
            >
              {{ saving ? 'Saving…' : '💾 Save Motion' }}
            </button>
            <button class="pt-btn pt-btn--ghost" @click="discardMotion">
              ↩ Discard
            </button>
          </div>
        </div>
      </template>
    </div>

    <!-- ── Saved expressions list ────────────────────────────────── -->
    <div v-if="persona.learnedExpressions.length > 0 && mode === 'expression'" class="pt-saved">
      <h4 class="pt-saved-title">Saved Expressions ({{ persona.learnedExpressions.length }})</h4>
      <ul class="pt-saved-list">
        <li v-for="expr in persona.learnedExpressions" :key="expr.id" class="pt-saved-item">
          <span class="pt-saved-name">{{ expr.name }}</span>
          <code class="pt-saved-trigger">{{ expr.trigger }}</code>
        </li>
      </ul>
    </div>

    <!-- ── Saved motions list ────────────────────────────────────── -->
    <div v-if="persona.learnedMotions.length > 0 && mode === 'motion'" class="pt-saved">
      <h4 class="pt-saved-title">Saved Motions ({{ persona.learnedMotions.length }})</h4>
      <ul class="pt-saved-list">
        <li v-for="motion in persona.learnedMotions" :key="motion.id" class="pt-saved-item">
          <span class="pt-saved-name">{{ motion.name }}</span>
          <span class="pt-saved-dur">{{ motion.duration_s.toFixed(1) }}s</span>
          <code class="pt-saved-trigger">{{ motion.trigger }}</code>
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

/* ── Mode tabs ───────────────────────────────────────────────── */
.pt-tabs {
  display: flex;
  gap: 0.25rem;
  border-bottom: 1px solid var(--ts-border, #333);
  padding-bottom: 0.5rem;
}
.pt-tab {
  padding: 0.4rem 0.75rem;
  border-radius: 0.4rem 0.4rem 0 0;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  border: none;
  background: transparent;
  color: var(--ts-text-secondary, #aaa);
  transition: background 0.15s, color 0.15s;
}
.pt-tab:hover {
  background: var(--ts-bg-secondary, #1a1a2e);
}
.pt-tab--active {
  background: var(--ts-accent, #6366f1);
  color: #fff;
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
  color: var(--ts-text-on-accent);
}
.pt-btn--primary:hover:not(:disabled) {
  background: var(--ts-accent-hover, #818cf8);
}
.pt-btn--accent {
  background: var(--ts-success, #22c55e);
  color: var(--ts-text-on-accent);
}
.pt-btn--accent:hover:not(:disabled) {
  background: var(--ts-success-dim);
}
.pt-btn--ghost {
  background: transparent;
  color: var(--ts-text-secondary, #aaa);
  border: 1px solid var(--ts-border, #333);
}
.pt-btn--ghost:hover:not(:disabled) {
  background: var(--ts-bg-hover);
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
.pt-rec-badge {
  margin-left: 0.5rem;
  font-size: 0.7rem;
  color: #ef4444;
  font-weight: 700;
}
.pt-saved-dur {
  font-size: 0.7rem;
  color: var(--ts-text-muted, #666);
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
