<!--
  PersonaMotionGenerator.vue — "Describe a motion, brain animates it" panel.

  Chunk 14.16c3 (LLM-as-Animator UI). Sits next to the Learned-motions
  library inside `PersonaPanel.vue`. The user types a short description
  ("wave hello", "shy bow", "happy spin"), the active brain returns a
  multi-frame VRM motion clip, the user previews it on the avatar, and
  on Accept it's saved into the persona's learned-motion library —
  reusing the existing `LearnedMotionPlayer` runtime (no new player
  needed).

  Privacy: this component is camera-free and has no microphone access.
  The only data crossing the IPC boundary is the user's typed
  description and the brain's JSON reply.
-->
<template>
  <div
    class="pmg-root"
    data-testid="pmg-root"
  >
    <header class="pp-lib-header">
      <h4>✨ Generate motion with AI</h4>
      <span class="pp-lib-note">
        Describe a motion in plain language — your brain will animate it.
      </span>
    </header>

    <p
      v-if="stats && stats.total > 0"
      class="pp-lib-note pmg-stats"
      data-testid="pmg-stats"
    >
      You've taught me {{ stats.accepted }} accepted motion{{ stats.accepted === 1 ? '' : 's' }}
      across {{ stats.total }} tries<span v-if="topTrusted">; favourites: <code>{{ topTrusted }}</code></span>.
    </p>

    <div class="pmg-row">
      <input
        v-model="description"
        class="pmg-input"
        type="text"
        maxlength="500"
        placeholder="e.g. wave hello, shy bow, excited little jump…"
        :disabled="busy"
        data-testid="pmg-description"
        @keydown.enter.prevent="generate"
      >
    </div>

    <div class="pmg-row pmg-options">
      <label class="pmg-label">
        Duration
        <input
          v-model.number="durationS"
          class="pmg-num"
          type="number"
          min="0.5"
          max="30"
          step="0.5"
          :disabled="busy"
          data-testid="pmg-duration"
        >
        s
      </label>
      <label class="pmg-label">
        FPS
        <input
          v-model.number="fps"
          class="pmg-num"
          type="number"
          min="6"
          max="60"
          step="1"
          :disabled="busy"
          data-testid="pmg-fps"
        >
      </label>
      <button
        class="pp-btn pp-btn-primary"
        :disabled="!canGenerate || busy"
        data-testid="pmg-generate"
        @click="generate"
      >
        {{ busy ? 'Generating…' : '🪄 Generate' }}
      </button>
    </div>

    <p
      v-if="error"
      class="pp-pack-error"
      data-testid="pmg-error"
      role="alert"
    >
      {{ error }}
    </p>

    <div
      v-if="candidate"
      class="pmg-preview"
      data-testid="pmg-preview"
    >
      <div class="pmg-preview-row">
        <strong>{{ candidate.name }}</strong>
        <span class="pp-lib-meta">trigger: <code>{{ candidate.trigger }}</code></span>
        <span class="pp-lib-meta">{{ candidate.duration_s.toFixed(1) }}s · {{ candidate.fps }}fps · {{ candidate.frames.length }} frames</span>
      </div>
      <p
        v-if="diagSummary"
        class="pp-lib-note pmg-diag"
        data-testid="pmg-diagnostics"
      >
        {{ diagSummary }}
      </p>
      <div class="pmg-row">
        <button
          class="pp-btn pp-btn-secondary"
          data-testid="pmg-play"
          @click="playPreview"
        >
          ▶ Play preview
        </button>
        <button
          class="pp-btn pp-btn-primary"
          :disabled="busy"
          data-testid="pmg-accept"
          @click="acceptAndSave"
        >
          {{ saving ? 'Saving…' : '✓ Accept & save' }}
        </button>
        <button
          class="pp-btn pp-btn-ghost"
          :disabled="busy"
          data-testid="pmg-reject"
          @click="reject"
        >
          ✗ Discard
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { usePersonaStore } from '../stores/persona';
import type { LearnedMotion } from '../stores/persona-types';

const store = usePersonaStore();

const description = ref('');
const durationS = ref(3);
const fps = ref(24);
const busy = ref(false);
const saving = ref(false);
const error = ref('');
const candidate = ref<LearnedMotion | null>(null);
const diagnostics = ref<{
  droppedFrames: number;
  clampedComponents: number;
  repairedTimestamps: boolean;
  repairedDuration: boolean;
} | null>(null);

interface FeedbackStats {
  total: number;
  accepted: number;
  rejected: number;
  trusted_triggers: Array<{ trigger: string; accepted: number; rejected: number }>;
  discouraged_descriptions: string[];
}
const stats = ref<FeedbackStats | null>(null);

async function refreshStats(): Promise<void> {
  stats.value = await store.fetchMotionFeedbackStats();
}

onMounted(refreshStats);

const canGenerate = computed(
  () => description.value.trim().length > 0 && !busy.value,
);

const topTrusted = computed(() => {
  const top = stats.value?.trusted_triggers.slice(0, 3) ?? [];
  return top.map((t) => t.trigger).join(', ');
});

const diagSummary = computed(() => {
  const d = diagnostics.value;
  if (!d) return '';
  const parts: string[] = [];
  if (d.droppedFrames > 0) parts.push(`${d.droppedFrames} frame(s) dropped`);
  if (d.clampedComponents > 0) parts.push(`${d.clampedComponents} bone component(s) clamped`);
  if (d.repairedTimestamps) parts.push('timestamps repaired');
  if (d.repairedDuration) parts.push('duration normalised');
  return parts.length > 0 ? `Cleanup applied: ${parts.join(', ')}.` : '';
});

async function generate(): Promise<void> {
  if (!canGenerate.value) return;
  busy.value = true;
  error.value = '';
  candidate.value = null;
  diagnostics.value = null;
  try {
    const result = await store.generateMotionFromText(description.value.trim(), {
      durationS: durationS.value,
      fps: fps.value,
    });
    if (!result) {
      error.value = 'No brain configured. Set up a brain first.';
      return;
    }
    candidate.value = result.motion;
    diagnostics.value = result.diagnostics;
  } catch (e) {
    error.value = (e instanceof Error ? e.message : String(e)) ||
      'Could not generate a motion. Try again.';
  } finally {
    busy.value = false;
  }
}

function playPreview(): void {
  if (candidate.value) {
    store.requestMotionPreview(candidate.value);
  }
}

async function acceptAndSave(): Promise<void> {
  if (!candidate.value) return;
  saving.value = true;
  busy.value = true;
  error.value = '';
  try {
    const motion: LearnedMotion = {
      ...candidate.value,
      provenance: 'generated',
    };
    await store.saveLearnedMotion(motion);
    // Self-improve loop (Chunk 14.16e): record the accept verdict
    // *after* the save succeeds so a save failure doesn't pollute
    // the feedback log.
    await store.recordMotionFeedback({
      description: description.value.trim(),
      trigger: motion.trigger,
      verdict: 'accepted',
      durationS: motion.duration_s,
      fps: motion.fps,
      droppedFrames: diagnostics.value?.droppedFrames ?? 0,
      clampedComponents: diagnostics.value?.clampedComponents ?? 0,
    });
    candidate.value = null;
    diagnostics.value = null;
    description.value = '';
    await refreshStats();
  } catch (e) {
    error.value = `Could not save motion: ${e instanceof Error ? e.message : String(e)}`;
  } finally {
    saving.value = false;
    busy.value = false;
  }
}

function reject(): void {
  if (candidate.value) {
    // Fire-and-forget: discarding shouldn't block the UI on IPC.
    void store.recordMotionFeedback({
      description: description.value.trim(),
      trigger: candidate.value.trigger,
      verdict: 'rejected',
      durationS: candidate.value.duration_s,
      fps: candidate.value.fps,
      droppedFrames: diagnostics.value?.droppedFrames ?? 0,
      clampedComponents: diagnostics.value?.clampedComponents ?? 0,
    }).then(refreshStats);
  }
  candidate.value = null;
  diagnostics.value = null;
  error.value = '';
}
</script>

<style scoped>
.pmg-root {
  margin-top: var(--ts-space-md, 12px);
  padding: var(--ts-space-md, 12px);
  border: 1px solid var(--ts-border-subtle, rgba(255, 255, 255, 0.08));
  border-radius: var(--ts-radius-md, 8px);
  background: var(--ts-surface-elev-1, rgba(255, 255, 255, 0.02));
}

.pmg-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ts-space-sm, 8px);
  align-items: center;
  margin-top: var(--ts-space-sm, 8px);
}

.pmg-options {
  align-items: flex-end;
}

.pmg-input {
  flex: 1 1 240px;
  min-width: 180px;
  padding: 6px 10px;
  border-radius: var(--ts-radius-sm, 4px);
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.12));
  background: var(--ts-surface, rgba(0, 0, 0, 0.2));
  color: var(--ts-text, inherit);
  font: inherit;
}

.pmg-label {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--ts-text-muted, rgba(255, 255, 255, 0.7));
  font-size: 0.85em;
}

.pmg-num {
  width: 4em;
  padding: 4px 6px;
  border-radius: var(--ts-radius-sm, 4px);
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.12));
  background: var(--ts-surface, rgba(0, 0, 0, 0.2));
  color: var(--ts-text, inherit);
  font: inherit;
}

.pmg-preview {
  margin-top: var(--ts-space-md, 12px);
  padding: var(--ts-space-sm, 8px);
  border-radius: var(--ts-radius-sm, 4px);
  background: var(--ts-surface, rgba(0, 0, 0, 0.15));
}

.pmg-preview-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ts-space-sm, 8px);
  align-items: baseline;
}

.pmg-diag {
  margin: 4px 0 0;
}
</style>
