<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useCodingWorkflowStore } from '../stores/coding-workflow';

const store = useCodingWorkflowStore();

const dirInput = ref('');
const fileInput = ref('');
const excludeInput = ref('');

const totalChars = computed(() => store.preview?.total_chars ?? 0);
const fileCount = computed(() => store.preview?.file_count ?? 0);

const totalCapPct = computed(() => {
  if (!store.config.max_total_chars) return 0;
  return Math.min(100, Math.round((totalChars.value / store.config.max_total_chars) * 100));
});

const totalCapColor = computed(() => {
  const pct = totalCapPct.value;
  if (pct >= 95) return 'var(--ts-warning, #fbbf24)';
  if (pct >= 80) return 'var(--ts-accent-violet, #a78bfa)';
  return 'var(--ts-success, #34d399)';
});

function fmtKb(chars: number): string {
  return `${(chars / 1024).toFixed(1)} KB`;
}

function snippetPct(chars: number): number {
  if (!store.config.max_file_chars) return 0;
  return Math.min(100, Math.round((chars / store.config.max_file_chars) * 100));
}

function isTruncated(chars: number): boolean {
  return chars >= store.config.max_file_chars;
}

async function addDir() {
  if (store.addEntry('include_dirs', dirInput.value)) {
    dirInput.value = '';
    await store.refreshPreview();
  }
}
async function addFile() {
  if (store.addEntry('include_files', fileInput.value)) {
    fileInput.value = '';
    await store.refreshPreview();
  }
}
async function addExclude() {
  if (store.addEntry('exclude_paths', excludeInput.value)) {
    excludeInput.value = '';
    await store.refreshPreview();
  }
}

async function removeChip(field: 'include_dirs' | 'include_files' | 'exclude_paths', v: string) {
  store.removeEntry(field, v);
  await store.refreshPreview();
}

async function onSave() {
  try {
    await store.save();
    await store.refreshPreview();
  } catch {
    // lastError is already set by the store; UI shows it.
  }
}

async function onReset() {
  await store.reset();
  await store.refreshPreview();
}

onMounted(async () => {
  try {
    await store.load();
    await store.refreshPreview();
  } catch {
    // Tauri backend unavailable — panel renders with defaults
  }
});

// Re-run preview when slider values settle.
let previewDebounce: ReturnType<typeof setTimeout> | null = null;
watch(
  () => [store.config.max_file_chars, store.config.max_total_chars],
  () => {
    if (previewDebounce) clearTimeout(previewDebounce);
    previewDebounce = setTimeout(() => store.refreshPreview(), 250);
  },
);
</script>

<template>
  <section
    class="cw-panel"
    aria-labelledby="cw-heading"
  >
    <header class="cw-header">
      <div>
        <h2
          id="cw-heading"
          class="cw-title"
        >
          Coding Workflow Context
        </h2>
        <p class="cw-subtitle">
          Choose which project files every coding task can read. Applies to
          the reusable <code>run_coding_task</code> runner and the autonomous
          self-improve planner — provider-agnostic, regardless of whether
          your Coding LLM is Claude, OpenAI, custom, or a local endpoint.
        </p>
      </div>
      <div
        v-if="store.preview?.repo_root"
        class="cw-repo"
      >
        <span class="cw-repo-label">Repo</span>
        <code>{{ store.preview.repo_root }}</code>
      </div>
    </header>

    <div class="cw-grid">
      <!-- Include directories -->
      <div class="cw-field">
        <label
          class="cw-label"
          for="cw-dir-input"
        >
          Include directories
          <span class="cw-hint">Repo-relative folders. Markdown files only, non-recursive.</span>
        </label>
        <div
          class="cw-chips"
          role="list"
        >
          <span
            v-for="dir in store.config.include_dirs"
            :key="dir"
            class="cw-chip"
            role="listitem"
          >
            <span class="cw-chip-text">{{ dir }}</span>
            <button
              type="button"
              class="cw-chip-x"
              :aria-label="`Remove ${dir}`"
              @click="removeChip('include_dirs', dir)"
            >×</button>
          </span>
          <span
            v-if="store.config.include_dirs.length === 0"
            class="cw-empty"
          >
            No directories — context will be empty.
          </span>
        </div>
        <div class="cw-add-row">
          <input
            id="cw-dir-input"
            v-model="dirInput"
            class="cw-input"
            placeholder="e.g. rules"
            @keydown.enter.prevent="addDir"
          >
          <button
            type="button"
            class="cw-btn-secondary"
            :disabled="!dirInput.trim()"
            @click="addDir"
          >
            Add directory
          </button>
        </div>
      </div>

      <!-- Include files -->
      <div class="cw-field">
        <label
          class="cw-label"
          for="cw-file-input"
        >
          Include files
          <span class="cw-hint">Specific repo-relative files, e.g. <code>README.md</code>.</span>
        </label>
        <div
          class="cw-chips"
          role="list"
        >
          <span
            v-for="f in store.config.include_files"
            :key="f"
            class="cw-chip"
            role="listitem"
          >
            <span class="cw-chip-text">{{ f }}</span>
            <button
              type="button"
              class="cw-chip-x"
              :aria-label="`Remove ${f}`"
              @click="removeChip('include_files', f)"
            >×</button>
          </span>
          <span
            v-if="store.config.include_files.length === 0"
            class="cw-empty"
          >
            No explicit files configured.
          </span>
        </div>
        <div class="cw-add-row">
          <input
            id="cw-file-input"
            v-model="fileInput"
            class="cw-input"
            placeholder="e.g. README.md"
            @keydown.enter.prevent="addFile"
          >
          <button
            type="button"
            class="cw-btn-secondary"
            :disabled="!fileInput.trim()"
            @click="addFile"
          >
            Add file
          </button>
        </div>
      </div>

      <!-- Exclude paths -->
      <div class="cw-field">
        <label
          class="cw-label"
          for="cw-exclude-input"
        >
          Exclude paths
          <span class="cw-hint">Skip these by exact path or basename match.</span>
        </label>
        <div
          class="cw-chips"
          role="list"
        >
          <span
            v-for="p in store.config.exclude_paths"
            :key="p"
            class="cw-chip cw-chip-exclude"
            role="listitem"
          >
            <span class="cw-chip-text">{{ p }}</span>
            <button
              type="button"
              class="cw-chip-x"
              :aria-label="`Remove ${p}`"
              @click="removeChip('exclude_paths', p)"
            >×</button>
          </span>
          <span
            v-if="store.config.exclude_paths.length === 0"
            class="cw-empty"
          >
            Nothing excluded — every matched file is loaded.
          </span>
        </div>
        <div class="cw-add-row">
          <input
            id="cw-exclude-input"
            v-model="excludeInput"
            class="cw-input"
            placeholder="e.g. backlog.md"
            @keydown.enter.prevent="addExclude"
          >
          <button
            type="button"
            class="cw-btn-secondary"
            :disabled="!excludeInput.trim()"
            @click="addExclude"
          >
            Add exclusion
          </button>
        </div>
      </div>

      <!-- Sliders -->
      <div class="cw-field cw-sliders">
        <div class="cw-slider">
          <label
            class="cw-label"
            for="cw-file-cap"
          >
            Per-file limit
            <span class="cw-value">{{ fmtKb(store.config.max_file_chars) }}</span>
          </label>
          <input
            id="cw-file-cap"
            v-model.number="store.config.max_file_chars"
            type="range"
            min="1000"
            max="8000"
            step="500"
            class="cw-range"
          >
          <div class="cw-range-bounds">
            <span>1.0 KB</span>
            <span>8.0 KB</span>
          </div>
        </div>

        <div class="cw-slider">
          <label
            class="cw-label"
            for="cw-total-cap"
          >
            Total budget
            <span class="cw-value">{{ fmtKb(store.config.max_total_chars) }}</span>
          </label>
          <input
            id="cw-total-cap"
            v-model.number="store.config.max_total_chars"
            type="range"
            min="10000"
            max="60000"
            step="1000"
            class="cw-range"
          >
          <div class="cw-range-bounds">
            <span>10 KB</span>
            <span>60 KB</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Live preview -->
    <section
      class="cw-preview"
      aria-live="polite"
    >
      <header class="cw-preview-header">
        <h3 class="cw-preview-title">
          Live preview
        </h3>
        <div class="cw-preview-summary">
          <span class="cw-stat">
            <strong>{{ fileCount }}</strong>
            <span>files</span>
          </span>
          <span class="cw-stat">
            <strong>{{ fmtKb(totalChars) }}</strong>
            <span>of {{ fmtKb(store.config.max_total_chars) }}</span>
          </span>
          <span
            v-if="store.previewing"
            class="cw-stat cw-stat-muted"
          >refreshing…</span>
        </div>
      </header>
      <div
        class="cw-cap-bar"
        :style="{ '--cap-color': totalCapColor }"
      >
        <div
          class="cw-cap-fill"
          :style="{ width: totalCapPct + '%' }"
        />
      </div>
      <ul
        v-if="store.preview && store.preview.documents.length"
        class="cw-doc-list"
      >
        <li
          v-for="d in store.preview.documents"
          :key="d.label"
          class="cw-doc"
        >
          <span class="cw-doc-label">{{ d.label }}</span>
          <span class="cw-doc-bar">
            <span
              class="cw-doc-bar-fill"
              :class="{ 'cw-doc-bar-fill--trunc': isTruncated(d.char_count) }"
              :style="{ width: snippetPct(d.char_count) + '%' }"
            />
          </span>
          <span class="cw-doc-size">{{ fmtKb(d.char_count) }}</span>
        </li>
      </ul>
      <p
        v-else
        class="cw-empty cw-empty-block"
      >
        No matching files in the bound repository. Add an include directory or
        verify your repo path.
      </p>
    </section>

    <!-- Footer -->
    <footer class="cw-footer">
      <p
        v-if="store.lastError"
        class="cw-error"
        role="alert"
      >
        {{ store.lastError }}
      </p>
      <div class="cw-footer-actions">
        <button
          type="button"
          class="cw-btn-secondary"
          :disabled="store.saving"
          @click="onReset"
        >
          Reset to defaults
        </button>
        <button
          type="button"
          class="cw-btn-secondary"
          :disabled="!store.isDirty || store.saving"
          @click="store.discardChanges"
        >
          Discard changes
        </button>
        <button
          type="button"
          class="cw-btn-primary"
          :disabled="!store.isDirty || store.saving"
          @click="onSave"
        >
          {{ store.saving ? 'Saving…' : 'Save changes' }}
        </button>
      </div>
    </footer>
  </section>
</template>

<style scoped>
.cw-panel {
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-md, 12px);
  padding: var(--ts-space-md, 12px);
  background: var(--ts-bg-card);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-lg, 14px);
  color: var(--ts-text-primary);
}

.cw-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--ts-space-md, 12px);
  flex-wrap: wrap;
}
.cw-title {
  margin: 0 0 4px 0;
  font-size: 1.05rem;
  font-weight: 600;
  color: var(--ts-text-bright);
}
.cw-subtitle {
  margin: 0;
  color: var(--ts-text-secondary);
  font-size: 0.85rem;
  line-height: 1.45;
  max-width: 60ch;
}
.cw-subtitle code,
.cw-hint code {
  background: var(--ts-bg-input);
  padding: 1px 6px;
  border-radius: var(--ts-radius-sm, 6px);
  font-size: 0.78rem;
  color: var(--ts-text-bright);
}
.cw-repo {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  font-size: 0.75rem;
}
.cw-repo-label {
  color: var(--ts-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  font-size: 0.65rem;
}
.cw-repo code {
  color: var(--ts-text-link);
  background: transparent;
}

.cw-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--ts-space-md, 12px);
}
@media (max-width: 720px) {
  .cw-grid { grid-template-columns: 1fr; }
}

.cw-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  background: var(--ts-bg-elevated);
  border: 1px solid var(--ts-border-subtle);
  border-radius: var(--ts-radius-md, 10px);
  padding: 10px;
}
.cw-sliders {
  grid-column: 1 / -1;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--ts-space-md, 12px);
  background: transparent;
  border: none;
  padding: 0;
}
@media (max-width: 720px) {
  .cw-sliders { grid-template-columns: 1fr; }
}
.cw-slider {
  background: var(--ts-bg-elevated);
  border: 1px solid var(--ts-border-subtle);
  border-radius: var(--ts-radius-md, 10px);
  padding: 10px;
}

.cw-label {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--ts-space-sm, 8px);
  font-size: 0.82rem;
  color: var(--ts-text-bright);
  font-weight: 600;
}
.cw-hint {
  font-weight: 400;
  color: var(--ts-text-muted);
  font-size: 0.75rem;
  margin-left: auto;
}
.cw-value {
  font-variant-numeric: tabular-nums;
  color: var(--ts-accent);
  background: var(--ts-accent-glow);
  padding: 2px 8px;
  border-radius: var(--ts-radius-pill, 999px);
  font-size: 0.78rem;
}

.cw-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  min-height: 32px;
  padding: 4px 0;
}
.cw-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: var(--ts-bg-input);
  color: var(--ts-text-bright);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-pill, 999px);
  padding: 4px 4px 4px 10px;
  font-size: 0.78rem;
  font-family: ui-monospace, 'SFMono-Regular', monospace;
}
.cw-chip-exclude {
  border-color: rgba(251, 191, 36, 0.4);
  background: var(--ts-warning-bg, rgba(251, 191, 36, 0.12));
  color: var(--ts-warning, #fbbf24);
}
.cw-chip-x {
  appearance: none;
  background: transparent;
  border: none;
  color: inherit;
  cursor: pointer;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 0.95rem;
  line-height: 1;
}
.cw-chip-x:hover {
  background: var(--ts-bg-hover);
}

.cw-empty {
  color: var(--ts-text-muted);
  font-size: 0.78rem;
  font-style: italic;
}
.cw-empty-block {
  padding: 12px;
  text-align: center;
  border: 1px dashed var(--ts-border);
  border-radius: var(--ts-radius-md, 10px);
  margin: 0;
}

.cw-add-row {
  display: flex;
  gap: 6px;
}
.cw-input {
  flex: 1;
  background: var(--ts-bg-input);
  border: 1px solid var(--ts-border);
  border-radius: var(--ts-radius-sm, 6px);
  color: var(--ts-text-primary);
  padding: 6px 10px;
  font-size: 0.82rem;
  font-family: ui-monospace, 'SFMono-Regular', monospace;
}
.cw-input:focus {
  outline: none;
  border-color: var(--ts-border-focus);
  box-shadow: 0 0 0 3px var(--ts-accent-glow);
}

.cw-range {
  width: 100%;
  margin: 8px 0 4px 0;
  accent-color: var(--ts-accent);
}
.cw-range-bounds {
  display: flex;
  justify-content: space-between;
  color: var(--ts-text-muted);
  font-size: 0.7rem;
  font-variant-numeric: tabular-nums;
}

.cw-preview {
  background: var(--ts-bg-elevated);
  border: 1px solid var(--ts-border-subtle);
  border-radius: var(--ts-radius-md, 10px);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.cw-preview-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: var(--ts-space-md, 12px);
  flex-wrap: wrap;
}
.cw-preview-title {
  margin: 0;
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--ts-text-bright);
}
.cw-preview-summary {
  display: flex;
  gap: 12px;
  align-items: center;
  font-size: 0.78rem;
}
.cw-stat {
  display: inline-flex;
  gap: 4px;
  align-items: baseline;
  color: var(--ts-text-secondary);
}
.cw-stat strong {
  color: var(--ts-text-bright);
  font-variant-numeric: tabular-nums;
}
.cw-stat-muted { color: var(--ts-text-muted); font-style: italic; }

.cw-cap-bar {
  height: 6px;
  background: var(--ts-bg-input);
  border-radius: var(--ts-radius-pill, 999px);
  overflow: hidden;
}
.cw-cap-fill {
  height: 100%;
  background: var(--cap-color);
  transition: width 200ms ease, background 200ms ease;
}

.cw-doc-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 220px;
  overflow-y: auto;
}
.cw-doc {
  display: grid;
  grid-template-columns: minmax(0, 2fr) 1fr auto;
  gap: 8px;
  align-items: center;
  font-size: 0.78rem;
  font-family: ui-monospace, 'SFMono-Regular', monospace;
  color: var(--ts-text-secondary);
}
.cw-doc-label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.cw-doc-bar {
  height: 4px;
  background: var(--ts-bg-input);
  border-radius: var(--ts-radius-pill, 999px);
  overflow: hidden;
}
.cw-doc-bar-fill {
  display: block;
  height: 100%;
  background: var(--ts-success);
}
.cw-doc-bar-fill--trunc {
  background: var(--ts-warning, #fbbf24);
}
.cw-doc-size {
  color: var(--ts-text-muted);
  font-variant-numeric: tabular-nums;
}

.cw-footer {
  border-top: 1px solid var(--ts-border-subtle);
  padding-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.cw-footer-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
}
.cw-error {
  margin: 0;
  color: var(--ts-warning, #fbbf24);
  background: var(--ts-warning-bg, rgba(251, 191, 36, 0.12));
  border: 1px solid rgba(251, 191, 36, 0.3);
  border-radius: var(--ts-radius-sm, 6px);
  padding: 6px 10px;
  font-size: 0.8rem;
}

.cw-btn-primary,
.cw-btn-secondary {
  appearance: none;
  border-radius: var(--ts-radius-sm, 6px);
  padding: 7px 14px;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 120ms ease, border-color 120ms ease, color 120ms ease;
}
.cw-btn-primary {
  background: var(--ts-accent);
  color: var(--ts-text-on-accent, #fff);
  border: 1px solid var(--ts-accent);
}
.cw-btn-primary:hover:not(:disabled) {
  background: var(--ts-accent-hover);
  border-color: var(--ts-accent-hover);
}
.cw-btn-secondary {
  background: var(--ts-bg-input);
  color: var(--ts-text-bright);
  border: 1px solid var(--ts-border);
}
.cw-btn-secondary:hover:not(:disabled) {
  background: var(--ts-bg-hover);
}
.cw-btn-primary:disabled,
.cw-btn-secondary:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
