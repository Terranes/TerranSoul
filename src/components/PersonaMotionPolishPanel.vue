<template>
  <section
    class="pmp-root"
    data-testid="pmp-root"
  >
    <header class="pmp-header">
      <div>
        <h4>Polish learned motion</h4>
        <p class="pmp-note">
          Preview a smoothed copy of a saved clip, then save it as a new motion.
        </p>
      </div>
    </header>

    <p
      v-if="store.learnedMotions.length === 0"
      class="pmp-empty"
    >
      Save a motion first, then polish a non-destructive copy here.
    </p>

    <template v-else>
      <div class="pmp-controls">
        <label class="pmp-field">
          <span>Source clip</span>
          <select
            v-model="sourceId"
            class="pmp-select"
            data-testid="pmp-source"
            :disabled="busy || saving"
          >
            <option
              v-for="motion in store.learnedMotions"
              :key="motion.id"
              :value="motion.id"
            >
              {{ motion.name }} · {{ motion.duration_s.toFixed(1) }}s
            </option>
          </select>
        </label>

        <div
          class="pmp-presets"
          aria-label="Smoothing preset"
        >
          <button
            v-for="item in presets"
            :key="item.id"
            class="pmp-preset"
            :class="{ 'pmp-preset--active': preset === item.id }"
            :disabled="busy || saving"
            type="button"
            :title="item.summary"
            :data-testid="`pmp-preset-${item.id}`"
            @click="preset = item.id"
          >
            {{ item.label }}
          </button>
        </div>

        <button
          class="pmp-btn pmp-btn-primary"
          type="button"
          :disabled="!sourceMotion || busy || saving"
          data-testid="pmp-build"
          @click="buildPreview"
        >
          {{ busy ? 'Polishing...' : 'Build preview' }}
        </button>
      </div>

      <p
        v-if="error"
        class="pmp-error"
        role="alert"
        data-testid="pmp-error"
      >
        {{ error }}
      </p>

      <div
        v-if="preview"
        class="pmp-preview"
        data-testid="pmp-preview"
      >
        <div class="pmp-preview-main">
          <div>
            <strong>{{ preview.candidateMotion.name }}</strong>
            <p class="pmp-note">
              {{ preview.candidateMotion.frames.length }} frames · {{ preview.candidateMotion.fps }}fps · {{ appliedSummary }}
            </p>
          </div>
          <div
            class="pmp-toggle"
            aria-label="Preview clip"
          >
            <button
              class="pmp-toggle-btn"
              :class="{ 'pmp-toggle-btn--active': previewMode === 'original' }"
              type="button"
              data-testid="pmp-play-original"
              @click="playOriginal"
            >
              Original
            </button>
            <button
              class="pmp-toggle-btn"
              :class="{ 'pmp-toggle-btn--active': previewMode === 'polished' }"
              type="button"
              data-testid="pmp-play-polished"
              @click="playPolished"
            >
              Polished
            </button>
          </div>
        </div>

        <dl class="pmp-stats">
          <div>
            <dt>Mean displacement</dt>
            <dd data-testid="pmp-mean-displacement">
              {{ formatRadians(meanDisplacement) }}
            </dd>
          </div>
          <div>
            <dt>Max displacement</dt>
            <dd data-testid="pmp-max-displacement">
              {{ formatRadians(preview.maxDisplacement) }}
            </dd>
          </div>
          <div>
            <dt>Endpoint lock</dt>
            <dd>{{ preview.appliedConfig.pinEndpoints ? 'On' : 'Off' }}</dd>
          </div>
        </dl>

        <div
          v-if="boneStats.length > 0"
          class="pmp-bones"
        >
          <div
            v-for="[bone, value] in boneStats"
            :key="bone"
            class="pmp-bone-row"
          >
            <span>{{ bone }}</span>
            <meter
              min="0"
              :max="boneMeterMax"
              :value="value"
            />
            <span>{{ formatRadians(value) }}</span>
          </div>
        </div>

        <ul
          v-if="preview.warnings.length > 0"
          class="pmp-warnings"
        >
          <li
            v-for="warning in preview.warnings"
            :key="warning"
          >
            {{ warning }}
          </li>
        </ul>

        <div class="pmp-actions">
          <button
            class="pmp-btn pmp-btn-primary"
            type="button"
            :disabled="saving || busy"
            data-testid="pmp-save"
            @click="saveCandidate"
          >
            {{ saving ? 'Saving...' : 'Save as new clip' }}
          </button>
          <button
            class="pmp-btn pmp-btn-ghost"
            type="button"
            :disabled="saving || busy"
            data-testid="pmp-discard"
            @click="discardPreview"
          >
            Discard
          </button>
        </div>
      </div>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { usePersonaStore } from '../stores/persona';
import type { LearnedMotion, MotionPolishPreset, MotionPolishPreview } from '../stores/persona-types';

const store = usePersonaStore();

const presets: Array<{ id: MotionPolishPreset; label: string; summary: string }> = [
  { id: 'light', label: 'Light', summary: 'Small cleanup for subtle camera jitter.' },
  { id: 'medium', label: 'Medium', summary: 'Balanced smoothing for most recorded clips.' },
  { id: 'heavy', label: 'Heavy', summary: 'Strong smoothing for noisy captures.' },
];

const sourceId = ref('');
const preset = ref<MotionPolishPreset>('medium');
const preview = ref<MotionPolishPreview | null>(null);
const previewMode = ref<'original' | 'polished'>('polished');
const busy = ref(false);
const saving = ref(false);
const error = ref('');

const sourceMotion = computed(() =>
  store.learnedMotions.find((motion) => motion.id === sourceId.value) ??
  store.learnedMotions[0] ??
  null,
);

const meanDisplacement = computed(() => {
  if (!preview.value) return 0;
  const metadataMean = preview.value.candidateMotion.polish?.meanDisplacement;
  if (typeof metadataMean === 'number') return metadataMean;
  const values = Object.values(preview.value.meanDisplacementByBone);
  return values.length === 0 ? 0 : values.reduce((sum, value) => sum + value, 0) / values.length;
});

const boneStats = computed(() => {
  if (!preview.value) return [];
  return Object.entries(preview.value.meanDisplacementByBone)
    .sort((left, right) => right[1] - left[1])
    .slice(0, 6);
});

const boneMeterMax = computed(() => {
  const top = boneStats.value[0]?.[1] ?? 0;
  return Math.max(top, 0.001);
});

const appliedSummary = computed(() => {
  if (!preview.value) return '';
  const config = preview.value.appliedConfig;
  const radius = config.radius === null ? 'auto radius' : `${config.radius} frame radius`;
  return `${config.preset} · sigma ${config.sigma.toFixed(1)} · ${radius}`;
});

watch(
  () => store.learnedMotions.map((motion) => motion.id).join('|'),
  () => {
    if (!store.learnedMotions.some((motion) => motion.id === sourceId.value)) {
      sourceId.value = store.learnedMotions[0]?.id ?? '';
    }
  },
  { immediate: true },
);

watch([sourceId, preset], () => {
  preview.value = null;
  previewMode.value = 'polished';
  error.value = '';
});

async function buildPreview(): Promise<void> {
  if (!sourceMotion.value || busy.value || saving.value) return;
  busy.value = true;
  error.value = '';
  preview.value = null;
  try {
    preview.value = await store.polishLearnedMotion(sourceMotion.value.id, {
      preset: preset.value,
      pinEndpoints: true,
    });
    previewMode.value = 'polished';
    store.requestMotionPreview(preview.value.candidateMotion);
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    busy.value = false;
  }
}

function playOriginal(): void {
  if (!sourceMotion.value) return;
  previewMode.value = 'original';
  store.requestMotionPreview(sourceMotion.value);
}

function playPolished(): void {
  if (!preview.value) return;
  previewMode.value = 'polished';
  store.requestMotionPreview(preview.value.candidateMotion);
}

async function saveCandidate(): Promise<void> {
  if (!preview.value || saving.value) return;
  saving.value = true;
  error.value = '';
  const motion: LearnedMotion = {
    ...preview.value.candidateMotion,
    polish: preview.value.candidateMotion.polish
      ? { ...preview.value.candidateMotion.polish, acceptedByUser: true }
      : undefined,
  };
  try {
    await store.saveLearnedMotion(motion);
    sourceId.value = motion.id;
    preview.value = null;
    previewMode.value = 'polished';
    store.requestMotionPreview(motion);
  } catch (err) {
    error.value = `Could not save polished motion: ${err instanceof Error ? err.message : String(err)}`;
  } finally {
    saving.value = false;
  }
}

function discardPreview(): void {
  preview.value = null;
  previewMode.value = 'polished';
  error.value = '';
}

function formatRadians(value: number): string {
  const degrees = value * (180 / Math.PI);
  return `${value.toFixed(3)} rad (${degrees.toFixed(1)} deg)`;
}
</script>

<style scoped>
.pmp-root {
  margin-top: var(--ts-space-md, 12px);
  padding: var(--ts-space-md, 12px);
  border: 1px solid var(--ts-border-subtle);
  border-radius: var(--ts-radius-md, 8px);
  background: var(--ts-bg-card);
}

.pmp-header {
  display: flex;
  justify-content: space-between;
  gap: var(--ts-space-sm, 8px);
  align-items: flex-start;
}

.pmp-header h4,
.pmp-note,
.pmp-empty {
  margin: 0;
}

.pmp-note,
.pmp-empty {
  color: var(--ts-text-muted);
  font-size: 0.8rem;
}

.pmp-controls,
.pmp-actions,
.pmp-preview-main,
.pmp-presets {
  display: flex;
  flex-wrap: wrap;
  gap: var(--ts-space-sm, 8px);
  align-items: flex-end;
}

.pmp-controls,
.pmp-preview,
.pmp-actions {
  margin-top: var(--ts-space-sm, 8px);
}

.pmp-field {
  display: flex;
  flex: 1 1 220px;
  flex-direction: column;
  gap: 4px;
  color: var(--ts-text-muted);
  font-size: 0.8rem;
}

.pmp-select {
  min-height: 34px;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm, 6px);
  background: var(--ts-bg-input);
  color: var(--ts-text-primary);
  font: inherit;
  padding: 4px 8px;
}

.pmp-preset,
.pmp-toggle-btn,
.pmp-btn {
  min-height: 34px;
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm, 6px);
  background: transparent;
  color: var(--ts-text-primary);
  cursor: pointer;
  font: inherit;
  padding: 4px 10px;
}

.pmp-preset--active,
.pmp-toggle-btn--active,
.pmp-btn-primary {
  border-color: var(--ts-accent);
  background: var(--ts-accent);
  color: var(--ts-text-on-accent);
}

.pmp-btn-ghost {
  color: var(--ts-text-muted);
}

.pmp-preset:disabled,
.pmp-toggle-btn:disabled,
.pmp-btn:disabled,
.pmp-select:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.pmp-error,
.pmp-warnings {
  margin: var(--ts-space-sm, 8px) 0 0;
  color: var(--ts-warning);
  font-size: 0.8rem;
}

.pmp-preview {
  padding: var(--ts-space-sm, 8px);
  border-radius: var(--ts-radius-sm, 6px);
  background: var(--ts-bg-input);
}

.pmp-preview-main {
  justify-content: space-between;
}

.pmp-toggle {
  display: inline-flex;
  gap: 4px;
}

.pmp-stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: var(--ts-space-sm, 8px);
  margin: var(--ts-space-sm, 8px) 0 0;
}

.pmp-stats div {
  padding: 6px 8px;
  border: 1px solid var(--ts-border-subtle);
  border-radius: var(--ts-radius-sm, 6px);
}

.pmp-stats dt {
  color: var(--ts-text-muted);
  font-size: 0.72rem;
}

.pmp-stats dd {
  margin: 2px 0 0;
  color: var(--ts-text-primary);
  font-size: 0.85rem;
  font-weight: 600;
}

.pmp-bones {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: var(--ts-space-sm, 8px);
}

.pmp-bone-row {
  display: grid;
  grid-template-columns: minmax(90px, 1fr) minmax(80px, 2fr) minmax(110px, auto);
  gap: var(--ts-space-sm, 8px);
  align-items: center;
  color: var(--ts-text-muted);
  font-size: 0.75rem;
}

.pmp-bone-row meter {
  width: 100%;
}

.pmp-warnings {
  padding-left: 1rem;
}
</style>

	