<template>
  <section
    class="persona-panel"
    data-testid="persona-panel"
  >
    <header class="pp-header">
      <h3>🎭 Persona</h3>
      <span class="pp-subtitle">
        Teach your companion who they are. The persona block is injected into every
        chat as <code>[PERSONA]</code>.
      </span>
    </header>

    <!-- ── Traits editor ─────────────────────────────────────────────── -->
    <div class="pp-traits">
      <div class="pp-row">
        <label class="pp-toggle">
          <input
            type="checkbox"
            :checked="draft.active"
            data-testid="pp-active"
            @change="onActiveToggle"
          >
          Inject persona into every chat
        </label>
      </div>

      <div class="pp-row">
        <label class="pp-field">
          <span>Name</span>
          <input
            v-model="draft.name"
            type="text"
            maxlength="60"
            placeholder="e.g. Lia"
            data-testid="pp-name"
            @input="markDirty"
          >
        </label>
        <label class="pp-field">
          <span>Role</span>
          <input
            v-model="draft.role"
            type="text"
            maxlength="80"
            placeholder="e.g. studious librarian"
            data-testid="pp-role"
            @input="markDirty"
          >
        </label>
      </div>

      <label class="pp-field pp-field-block">
        <span>Background (max ~500 chars rendered)</span>
        <textarea
          v-model="draft.bio"
          rows="3"
          maxlength="2000"
          placeholder="A few sentences of backstory…"
          data-testid="pp-bio"
          @input="markDirty"
        />
      </label>

      <PersonaListEditor
        label="Tone"
        placeholder="warm, concise, lightly sarcastic…"
        :items="draft.tone"
        data-testid="pp-tone"
        @update="(items: string[]) => { draft.tone = items; markDirty(); }"
      />

      <PersonaListEditor
        label="Quirks"
        placeholder="ends sentences with 'indeed'…"
        :items="draft.quirks"
        data-testid="pp-quirks"
        @update="(items: string[]) => { draft.quirks = items; markDirty(); }"
      />

      <PersonaListEditor
        label="Never (hard avoid)"
        placeholder="don't give medical advice…"
        :items="draft.avoid"
        data-testid="pp-avoid"
        @update="(items: string[]) => { draft.avoid = items; markDirty(); }"
      />

      <PersonaListEditor
        label="Example dialogue"
        placeholder="User: How are you? / Assistant: Splendid, thanks for asking!"
        :items="draft.exampleDialogue"
        data-testid="pp-example-dialogue"
        @update="(items: string[]) => { draft.exampleDialogue = items; markDirty(); }"
      />

      <div class="pp-actions">
        <button
          class="pp-btn pp-btn-primary"
          :disabled="!isDirty || isSaving"
          data-testid="pp-save"
          @click="save"
        >
          {{ isSaving ? 'Saving…' : 'Save persona' }}
        </button>
        <button
          class="pp-btn pp-btn-secondary"
          :disabled="isSaving"
          data-testid="pp-reset"
          @click="resetDraftFromStore"
        >
          Discard changes
        </button>
        <button
          class="pp-btn pp-btn-ghost"
          :disabled="isSaving"
          data-testid="pp-default"
          @click="resetToDefault"
        >
          Reset to default
        </button>
        <button
          class="pp-btn pp-btn-ghost"
          :disabled="isSaving || isSuggesting"
          data-testid="pp-suggest"
          :title="suggestTooltip"
          @click="suggestFromBrain"
        >
          {{ isSuggesting ? 'Asking the brain…' : '✨ Suggest from my chats' }}
        </button>
        <span
          v-if="lastSavedAt"
          class="pp-saved"
        >Saved {{ relativeTime(lastSavedAt) }}</span>
      </div>

      <!-- ── Brain-suggested persona review card (Chunk 14.2) ─────────── -->
      <div
        v-if="suggestion"
        class="pp-suggestion"
        data-testid="pp-suggestion"
      >
        <header class="pp-suggestion-header">
          <h4>✨ Brain-suggested persona</h4>
          <span class="pp-suggestion-note">
            Review before applying — nothing is saved yet.
          </span>
        </header>
        <dl class="pp-suggestion-fields">
          <dt>Name</dt><dd data-testid="pp-suggestion-name">
            {{ suggestion.name }}
          </dd>
          <dt>Role</dt><dd>{{ suggestion.role }}</dd>
          <dt>Bio</dt><dd>{{ suggestion.bio }}</dd>
          <dt v-if="suggestion.tone?.length">
            Tone
          </dt>
          <dd v-if="suggestion.tone?.length">
            {{ suggestion.tone.join(', ') }}
          </dd>
          <dt v-if="suggestion.quirks?.length">
            Quirks
          </dt>
          <dd v-if="suggestion.quirks?.length">
            {{ suggestion.quirks.join(', ') }}
          </dd>
          <dt v-if="suggestion.avoid?.length">
            Avoid
          </dt>
          <dd v-if="suggestion.avoid?.length">
            {{ suggestion.avoid.join(', ') }}
          </dd>
        </dl>
        <div class="pp-suggestion-actions">
          <button
            class="pp-btn pp-btn-primary"
            data-testid="pp-suggestion-apply"
            @click="applySuggestion"
          >
            Apply
          </button>
          <button
            class="pp-btn pp-btn-secondary"
            data-testid="pp-suggestion-edit"
            @click="loadSuggestionIntoDraft"
          >
            Load into editor
          </button>
          <button
            class="pp-btn pp-btn-ghost"
            data-testid="pp-suggestion-discard"
            @click="discardSuggestion"
          >
            Discard
          </button>
        </div>
      </div>
      <p
        v-else-if="suggestionError"
        class="pp-suggestion-error"
        data-testid="pp-suggestion-error"
        role="alert"
      >
        {{ suggestionError }}
      </p>
    </div>

    <!-- ── Live preview of the rendered [PERSONA] block ─────────────── -->
    <details
      class="pp-preview"
      data-testid="pp-preview"
    >
      <summary>Preview the system-prompt block</summary>
      <pre v-if="previewBlock">{{ previewBlock }}</pre>
      <p
        v-else
        class="pp-preview-empty"
      >
        No persona block will be injected (persona inactive or all fields empty).
      </p>
    </details>

    <!-- ── Learned expressions library (review + delete) ────────────── -->
    <div class="pp-library">
      <header class="pp-lib-header">
        <h4>🎭 Learned expressions ({{ store.learnedExpressions?.length ?? 0 }})</h4>
        <span class="pp-lib-note">
          Captured by the camera-mirror side quest (ships later — see Quests).
        </span>
      </header>
      <ul
        v-if="(store.learnedExpressions?.length ?? 0) > 0"
        class="pp-lib-list"
      >
        <li
          v-for="exp in (store.learnedExpressions ?? [])"
          :key="exp.id"
          class="pp-lib-item"
        >
          <span class="pp-lib-name">{{ exp.name }}</span>
          <span class="pp-lib-trigger">trigger: <code>{{ exp.trigger }}</code></span>
          <span class="pp-lib-meta">{{ relativeTime(exp.learnedAt) }}</span>
          <button
            class="pp-btn pp-btn-secondary"
            :data-testid="`pp-preview-exp-${exp.id}`"
            @click="store.requestExpressionPreview(exp)"
          >
            ▶ Preview
          </button>
          <button
            class="pp-btn pp-btn-danger"
            :data-testid="`pp-delete-exp-${exp.id}`"
            @click="deleteExpression(exp.id)"
          >
            Delete
          </button>
        </li>
      </ul>
      <p
        v-if="(store.learnedExpressions?.length ?? 0) === 0"
        class="pp-lib-empty"
      >
        No learned expressions yet. The "Mask of a Thousand Faces" side quest
        unlocks per-session camera capture for adding presets.
      </p>
    </div>

    <!-- ── Learned motions library (review + delete) ────────────────── -->
    <div class="pp-library">
      <header class="pp-lib-header">
        <h4>🪩 Learned motions ({{ store.learnedMotions?.length ?? 0 }})</h4>
        <span class="pp-lib-note">
          Captured by the camera-mirror side quest (ships later — see Quests).
        </span>
      </header>
      <ul
        v-if="(store.learnedMotions?.length ?? 0) > 0"
        class="pp-lib-list"
      >
        <li
          v-for="m in (store.learnedMotions ?? [])"
          :key="m.id"
          class="pp-lib-item"
        >
          <span class="pp-lib-name">{{ m.name }}</span>
          <span class="pp-lib-trigger">trigger: <code>{{ m.trigger }}</code></span>
          <span class="pp-lib-meta">{{ m.duration_s.toFixed(1) }}s · {{ m.fps }}fps</span>
          <span class="pp-lib-meta">{{ relativeTime(m.learnedAt) }}</span>
          <button
            class="pp-btn pp-btn-secondary"
            :data-testid="`pp-play-motion-${m.id}`"
            @click="store.requestMotionPreview(m)"
          >
            ▶ Play
          </button>
          <button
            class="pp-btn pp-btn-danger"
            :data-testid="`pp-delete-motion-${m.id}`"
            @click="deleteMotion(m.id)"
          >
            Delete
          </button>
        </li>
      </ul>
      <p
        v-if="(store.learnedMotions?.length ?? 0) === 0"
        class="pp-lib-empty"
      >
        No learned motions yet. The "Mirror Dance" side quest unlocks
        per-session camera capture for adding clips.
      </p>
    </div>

    <!-- ── Learned-motion polish preview (Chunk 27.5c) ──────────────── -->
    <PersonaMotionPolishPanel />

    <!-- ── LLM-as-Animator: generate motion from text (Chunk 14.16c3) ─ -->
    <PersonaMotionGenerator />

    <!-- ── Persona pack: export / import (Chunk 14.7) ─────────────────── -->
    <PersonaPackPanel @imported="onPackImported" />

    <p class="pp-privacy">
      🔒 Camera capture is opt-in <strong>per chat session</strong>. There is no
      "always on" persistent camera flag. See the Persona design doc for details.
    </p>
  </section>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { usePersonaStore } from '../stores/persona';
import {
  type PersonaTraits,
} from '../stores/persona-types';
import { buildPersonaBlock } from '../utils/persona-prompt';
import PersonaListEditor from './PersonaListEditor.vue';
import PersonaPackPanel from './PersonaPackPanel.vue';
import PersonaMotionPolishPanel from './PersonaMotionPolishPanel.vue';
import PersonaMotionGenerator from './PersonaMotionGenerator.vue';

const store = usePersonaStore();

/** A working copy of traits the user is editing. We `save()` back to the store. */
const draft = ref<PersonaTraits>(cloneTraits(store.traits));
const isDirty = ref(false);
const isSaving = ref(false);
const lastSavedAt = ref<number | null>(null);

function cloneTraits(t: PersonaTraits): PersonaTraits {
  return {
    ...t,
    tone: [...t.tone],
    quirks: [...t.quirks],
    avoid: [...t.avoid],
    exampleDialogue: [...(t.exampleDialogue ?? [])],
  };
}

function resetDraftFromStore(): void {
  draft.value = cloneTraits(store.traits);
  isDirty.value = false;
}

function markDirty(): void {
  isDirty.value = true;
}

function onActiveToggle(e: Event): void {
  draft.value.active = (e.target as HTMLInputElement).checked;
  markDirty();
}

async function save(): Promise<void> {
  if (!isDirty.value || isSaving.value) return;
  isSaving.value = true;
  try {
    await store.saveTraits(draft.value);
    lastSavedAt.value = Date.now();
    isDirty.value = false;
    // Keep the draft synced with the freshly-saved + timestamped store value.
    draft.value = cloneTraits(store.traits);
  } finally {
    isSaving.value = false;
  }
}

async function resetToDefault(): Promise<void> {
  if (isSaving.value) return;
  if (typeof window !== 'undefined' && typeof window.confirm === 'function') {
    if (!window.confirm('Reset persona to the default "Soul" companion?')) return;
  }
  isSaving.value = true;
  try {
    await store.resetToDefault();
    draft.value = cloneTraits(store.traits);
    isDirty.value = false;
    lastSavedAt.value = Date.now();
  } finally {
    isSaving.value = false;
  }
}

// ── Brain-suggested persona (Chunk 14.2 — Master-Echo loop) ────────────
const isSuggesting = ref(false);
const suggestion = ref<Partial<PersonaTraits> | null>(null);
const suggestionError = ref<string | null>(null);

/**
 * Tooltip wording for the suggest button — surfaces the "no brain
 * configured" case so the user understands why the action might be
 * inert (per `docs/persona-design.md` § 13).
 */
const suggestTooltip = computed(() =>
  isSuggesting.value
    ? 'Asking the brain to propose a persona…'
    : 'Ask the configured brain to propose a persona based on recent chats and your personal memories.',
);

async function suggestFromBrain(): Promise<void> {
  if (isSuggesting.value) return;
  isSuggesting.value = true;
  suggestion.value = null;
  suggestionError.value = null;
  try {
    const candidate = await store.suggestPersonaFromBrain();
    if (candidate) {
      suggestion.value = candidate;
    } else {
      suggestionError.value =
        'The brain could not propose a persona right now. Try again, or chat a bit more so it has more to go on.';
    }
  } catch (err) {
    suggestionError.value =
      err instanceof Error ? err.message : 'Failed to ask the brain for a persona.';
  } finally {
    isSuggesting.value = false;
  }
}

/** Apply the suggestion straight to disk via the existing save path. */
async function applySuggestion(): Promise<void> {
  if (!suggestion.value || isSaving.value) return;
  loadSuggestionIntoDraft();
  await save();
  suggestion.value = null;
}

/** Load the suggestion into the draft so the user can edit before saving. */
function loadSuggestionIntoDraft(): void {
  if (!suggestion.value) return;
  draft.value = {
    ...draft.value,
    name: suggestion.value.name ?? draft.value.name,
    role: suggestion.value.role ?? draft.value.role,
    bio: suggestion.value.bio ?? draft.value.bio,
    tone: suggestion.value.tone ? [...suggestion.value.tone] : draft.value.tone,
    quirks: suggestion.value.quirks ? [...suggestion.value.quirks] : draft.value.quirks,
    avoid: suggestion.value.avoid ? [...suggestion.value.avoid] : draft.value.avoid,
    exampleDialogue: suggestion.value.exampleDialogue ? [...suggestion.value.exampleDialogue] : draft.value.exampleDialogue,
  };
  isDirty.value = true;
}

function discardSuggestion(): void {
  suggestion.value = null;
  suggestionError.value = null;
}

/**
 * Sync the editor draft with the freshly-loaded persona after a pack
 * import (the store's `importPack` calls `load()` which mutates
 * `store.traits`).
 */
function onPackImported(): void {
  draft.value = cloneTraits(store.traits);
  isDirty.value = false;
  lastSavedAt.value = Date.now();
}

async function deleteExpression(id: string): Promise<void> {
  if (typeof window !== 'undefined' && typeof window.confirm === 'function') {
    if (!window.confirm(`Delete this learned expression?`)) return;
  }
  try {
    await invoke('delete_learned_expression', { id });
    store.learnedExpressions = store.learnedExpressions.filter((e) => e.id !== id);
  } catch (e) {
    // Tauri-only; ignore in browser.
    console.warn('[persona] delete expression failed:', e);
  }
}

async function deleteMotion(id: string): Promise<void> {
  if (typeof window !== 'undefined' && typeof window.confirm === 'function') {
    if (!window.confirm(`Delete this learned motion clip?`)) return;
  }
  try {
    await invoke('delete_learned_motion', { id });
    store.learnedMotions = store.learnedMotions.filter((m) => m.id !== id);
  } catch (e) {
    console.warn('[persona] delete motion failed:', e);
  }
}

/** Live preview of the rendered persona block. */
const previewBlock = computed(() =>
  buildPersonaBlock(draft.value, store.learnedMotionRefs).trim(),
);

function relativeTime(ts: number): string {
  if (!ts) return 'never';
  const diff = Date.now() - ts;
  if (diff < 0) return 'just now';
  const sec = Math.round(diff / 1000);
  if (sec < 60) return `${sec}s ago`;
  const min = Math.round(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.round(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.round(hr / 24);
  return `${day}d ago`;
}

onMounted(async () => {
  if (!store.traitsLoaded) {
    await store.load();
  }
  resetDraftFromStore();
});

// Whenever the store value changes externally (e.g. after `load()` finishes
// or another panel saves), and the user has no pending edits, sync the draft.
watch(
  () => store.traits,
  (next) => {
    if (!isDirty.value) draft.value = cloneTraits(next);
  },
  { deep: true },
);

// Make sure the default-persona reset also sneaks in on first paint when
// the store loads asynchronously.
watch(
  () => store.traitsLoaded,
  (loaded) => {
    if (loaded && !isDirty.value) draft.value = cloneTraits(store.traits);
  },
);
</script>

<style scoped>
.persona-panel {
  background: var(--ts-card-bg, rgba(255, 255, 255, 0.04));
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.08));
  border-radius: var(--ts-radius-md, 12px);
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}
.pp-header h3 { margin: 0 0 0.25rem; }
.pp-subtitle {
  display: block;
  font-size: 0.85rem;
  color: var(--ts-text-muted, #aab);
}
.pp-traits { display: flex; flex-direction: column; gap: 0.75rem; }
.pp-row { display: flex; gap: 0.75rem; flex-wrap: wrap; }
.pp-field { display: flex; flex-direction: column; gap: 0.25rem; flex: 1 1 200px; }
.pp-field-block { width: 100%; }
.pp-field span { font-size: 0.8rem; color: var(--ts-text-muted, #aab); }
.pp-field input,
.pp-field textarea {
  background: var(--ts-input-bg, rgba(0, 0, 0, 0.25));
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.12));
  color: var(--ts-text, #eee);
  border-radius: var(--ts-radius-sm, 6px);
  padding: 0.45rem 0.6rem;
  font: inherit;
}
.pp-field textarea { resize: vertical; min-height: 4rem; }
.pp-toggle { display: inline-flex; align-items: center; gap: 0.5rem; cursor: pointer; }
.pp-actions { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
.pp-btn {
  padding: 0.4rem 0.85rem;
  border-radius: var(--ts-radius-sm, 6px);
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.15));
  background: transparent;
  color: var(--ts-text, #eee);
  cursor: pointer;
  font: inherit;
}
.pp-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.pp-btn-primary {
  background: var(--ts-accent, #4a7);
  border-color: var(--ts-accent, #4a7);
  color: var(--ts-text-on-accent);
}
.pp-btn-secondary { background: var(--ts-bg-input); }
.pp-btn-ghost { color: var(--ts-text-muted, #aab); }
.pp-btn-danger {
  border-color: var(--ts-danger, #c44);
  color: var(--ts-danger, #c44);
}
.pp-btn-danger:hover { background: var(--ts-danger, #c44); color: var(--ts-text-on-accent); }
.pp-saved { font-size: 0.8rem; color: var(--ts-text-muted, #aab); }
.pp-suggestion {
  margin-top: 0.5rem;
  border: 1px solid var(--ts-accent, #4a7);
  border-radius: var(--ts-radius-md, 12px);
  padding: 0.75rem 0.9rem;
  background: var(--ts-success-bg, rgba(80, 200, 140, 0.06));
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.pp-suggestion-header { display: flex; align-items: baseline; gap: 0.6rem; flex-wrap: wrap; }
.pp-suggestion-header h4 { margin: 0; font-size: 0.95rem; }
.pp-suggestion-note { font-size: 0.75rem; color: var(--ts-text-muted, #aab); }
.pp-suggestion-fields {
  margin: 0;
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: 0.25rem 0.75rem;
  font-size: 0.85rem;
}
.pp-suggestion-fields dt { color: var(--ts-text-muted, #aab); }
.pp-suggestion-fields dd { margin: 0; word-break: break-word; }
.pp-suggestion-actions { display: flex; gap: 0.5rem; flex-wrap: wrap; }
.pp-suggestion-error {
  margin: 0.5rem 0 0;
  padding: 0.5rem 0.7rem;
  font-size: 0.8rem;
  color: var(--ts-warning, #c80);
  background: var(--ts-warning-bg, rgba(255, 200, 80, 0.06));
  border: 1px solid var(--ts-warning, rgba(255, 200, 80, 0.4));
  border-radius: var(--ts-radius-sm, 6px);
}
.pp-preview pre {
  background: var(--ts-bg-panel);
  border-radius: var(--ts-radius-sm, 6px);
  padding: 0.75rem;
  overflow-x: auto;
  font-size: 0.8rem;
  white-space: pre-wrap;
}
.pp-preview-empty { font-size: 0.85rem; color: var(--ts-text-muted, #aab); }
.pp-library {
  border-top: 1px dashed var(--ts-border, rgba(255, 255, 255, 0.08));
  padding-top: 0.75rem;
}
.pp-lib-header { display: flex; justify-content: space-between; flex-wrap: wrap; gap: 0.25rem; }
.pp-lib-header h4 { margin: 0; font-size: 1rem; }
.pp-lib-note { font-size: 0.75rem; color: var(--ts-text-muted, #aab); }
.pp-lib-list { list-style: none; padding: 0; margin: 0.5rem 0 0; display: flex; flex-direction: column; gap: 0.4rem; }
.pp-lib-item {
  display: grid;
  grid-template-columns: 1fr auto auto auto;
  gap: 0.5rem;
  align-items: center;
  padding: 0.4rem 0.6rem;
  background: var(--ts-bg-input);
  border-radius: var(--ts-radius-sm, 6px);
}
.pp-lib-name { font-weight: 600; }
.pp-lib-trigger code,
.pp-lib-meta {
  font-size: 0.8rem;
  color: var(--ts-text-muted, #aab);
}
.pp-lib-empty {
  margin: 0.5rem 0 0;
  font-size: 0.85rem;
  color: var(--ts-text-muted, #aab);
}
.pp-privacy {
  margin: 0;
  padding: 0.6rem 0.8rem;
  background: var(--ts-warning-bg, rgba(255, 200, 80, 0.06));
  border: 1px solid var(--ts-warning-bg, rgba(255, 200, 80, 0.25));
  border-radius: var(--ts-radius-sm, 6px);
  font-size: 0.8rem;
  color: var(--ts-text, #eee);
}
</style>
