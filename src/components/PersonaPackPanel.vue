<!--
  Persona pack export / import card (Chunk 14.7).

  Lives next to the main PersonaPanel and talks to the same Pinia
  store (`exportPack` / `previewImportPack` / `importPack`). Extracted
  out of PersonaPanel.vue so neither file blows past the 800-line Vue
  budget.

  Privacy: this component is camera-free. Packs only contain JSON
  artifacts (traits + learned-expression presets + learned-motion
  clips) — the same data already on disk under `<app_data_dir>/persona`.
-->
<template>
  <div
    class="pp-pack"
    data-testid="pp-pack"
  >
    <header class="pp-lib-header">
      <h4>📦 Persona pack</h4>
      <span class="pp-lib-note">
        Share or back up your full persona setup as a single JSON document.
      </span>
    </header>

    <div class="pp-pack-row">
      <input
        v-model="exportNote"
        class="pp-pack-note"
        placeholder="Optional note (shown when importing)…"
        :disabled="isExporting"
        data-testid="pp-pack-note"
      >
      <button
        class="pp-btn pp-btn-primary"
        :disabled="isExporting"
        data-testid="pp-pack-export"
        @click="exportPack"
      >
        {{ isExporting ? 'Exporting…' : '⬇ Export' }}
      </button>
      <button
        v-if="exportedJson"
        class="pp-btn pp-btn-secondary"
        data-testid="pp-pack-copy"
        @click="copyExported"
      >
        {{ copiedAt ? '✓ Copied' : '📋 Copy' }}
      </button>
      <button
        v-if="exportedJson"
        class="pp-btn pp-btn-secondary"
        data-testid="pp-pack-download"
        @click="downloadExported"
      >
        💾 Save .json
      </button>
    </div>

    <details
      v-if="exportedJson"
      class="pp-pack-details"
    >
      <summary>Show exported JSON ({{ exportedJson.length.toLocaleString() }} chars)</summary>
      <pre data-testid="pp-pack-export-output">{{ exportedJson }}</pre>
    </details>

    <div class="pp-pack-row pp-pack-import-row">
      <label
        for="pp-pack-import-textarea"
        class="pp-pack-label"
      >Paste a pack to import:</label>
    </div>
    <textarea
      id="pp-pack-import-textarea"
      v-model="importJson"
      class="pp-pack-textarea"
      rows="3"
      placeholder="{ &quot;packVersion&quot;: 1, ... }"
      :disabled="isImporting"
      data-testid="pp-pack-import-input"
    />
    <div class="pp-pack-row">
      <button
        class="pp-btn pp-btn-secondary"
        :disabled="!importJson.trim() || isImporting"
        data-testid="pp-pack-preview"
        @click="previewImport"
      >
        🔍 Preview
      </button>
      <button
        class="pp-btn pp-btn-primary"
        :disabled="!importJson.trim() || isImporting"
        data-testid="pp-pack-apply"
        @click="applyImport"
      >
        {{ isImporting ? 'Importing…' : '⤴ Apply import' }}
      </button>
      <button
        v-if="importJson || importReport || importError"
        class="pp-btn pp-btn-ghost"
        data-testid="pp-pack-clear"
        @click="clearImport"
      >
        Clear
      </button>
    </div>
    <p
      v-if="importError"
      class="pp-pack-error"
      data-testid="pp-pack-error"
      role="alert"
    >
      {{ importError }}
    </p>
    <div
      v-if="importReport"
      class="pp-pack-report"
      data-testid="pp-pack-report"
    >
      <strong>{{ importReportTitle }}:</strong>
      traits {{ importReport.traits_applied ? 'applied' : 'unchanged' }},
      {{ importReport.expressions_accepted }} expression{{ importReport.expressions_accepted === 1 ? '' : 's' }},
      {{ importReport.motions_accepted }} motion{{ importReport.motions_accepted === 1 ? '' : 's' }}<span
        v-if="importReport.skipped.length"
      >, {{ importReport.skipped.length }} skipped</span>.
      <ul
        v-if="importReport.skipped.length"
        class="pp-pack-skipped"
      >
        <li
          v-for="(reason, i) in importReport.skipped"
          :key="i"
        >
          {{ reason }}
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { usePersonaStore } from '../stores/persona';

interface ImportReport {
  traits_applied: boolean;
  expressions_accepted: number;
  motions_accepted: number;
  skipped: string[];
}

const store = usePersonaStore();
const emit = defineEmits<{ (e: 'imported'): void }>();

const exportNote = ref('');
const exportedJson = ref('');
const isExporting = ref(false);
const copiedAt = ref<number | null>(null);

const importJson = ref('');
const isImporting = ref(false);
const importReport = ref<ImportReport | null>(null);
const importReportTitle = ref<string>('Import preview');
const importError = ref<string | null>(null);

async function exportPack(): Promise<void> {
  if (isExporting.value) return;
  isExporting.value = true;
  try {
    const json = await store.exportPack(exportNote.value);
    exportedJson.value = typeof json === 'string' ? json : '';
    copiedAt.value = null;
  } finally {
    isExporting.value = false;
  }
}

/**
 * Copy the exported pack to the clipboard. Uses `navigator.clipboard`
 * (available in Tauri's WebView). Falls back to a console warn if the
 * Clipboard API is unavailable — the user can still see the JSON in
 * the `<pre>` block and copy it manually.
 */
async function copyExported(): Promise<void> {
  if (!exportedJson.value) return;
  try {
    await navigator.clipboard.writeText(exportedJson.value);
    copiedAt.value = Date.now();
    setTimeout(() => {
      if (copiedAt.value && Date.now() - copiedAt.value >= 1500) {
        copiedAt.value = null;
      }
    }, 1600);
  } catch (e) {
    console.warn('[persona] clipboard write failed:', e);
  }
}

/**
 * Trigger a browser file-save for the exported pack. Uses the
 * `Blob`+`<a download>` pattern that works inside Tauri's WebView
 * without needing Tauri's `dialog` plugin.
 */
function downloadExported(): void {
  if (!exportedJson.value) return;
  const stamp = new Date()
    .toISOString()
    .replace(/[:.]/g, '-')
    .slice(0, 19);
  const blob = new Blob([exportedJson.value], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `terransoul-persona-${stamp}.json`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

async function previewImport(): Promise<void> {
  if (isImporting.value) return;
  importReport.value = null;
  importError.value = null;
  try {
    importReport.value = await store.previewImportPack(importJson.value);
    importReportTitle.value = 'Import preview';
  } catch (e) {
    importError.value = e instanceof Error ? e.message : String(e);
  }
}

async function applyImport(): Promise<void> {
  if (isImporting.value) return;
  isImporting.value = true;
  importReport.value = null;
  importError.value = null;
  try {
    importReport.value = await store.importPack(importJson.value);
    importReportTitle.value = 'Imported';
    emit('imported');
  } catch (e) {
    importError.value = e instanceof Error ? e.message : String(e);
  } finally {
    isImporting.value = false;
  }
}

function clearImport(): void {
  importJson.value = '';
  importReport.value = null;
  importError.value = null;
}
</script>

<style scoped>
.pp-pack {
  border-top: 1px dashed var(--ts-border, rgba(255, 255, 255, 0.08));
  padding-top: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.pp-lib-header { display: flex; justify-content: space-between; flex-wrap: wrap; gap: 0.25rem; }
.pp-lib-header h4 { margin: 0; font-size: 1rem; }
.pp-lib-note { font-size: 0.75rem; color: var(--ts-text-muted, #aab); }
.pp-pack-row { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
.pp-pack-import-row { margin-top: 0.25rem; }
.pp-pack-label { font-size: 0.85rem; color: var(--ts-text-muted, #aab); }
.pp-pack-note,
.pp-pack-textarea {
  background: var(--ts-input-bg, rgba(0, 0, 0, 0.25));
  border: 1px solid var(--ts-border, rgba(255, 255, 255, 0.12));
  color: var(--ts-text, #eee);
  border-radius: var(--ts-radius-sm, 6px);
  padding: 0.45rem 0.6rem;
  font: inherit;
}
.pp-pack-note { flex: 1 1 200px; }
.pp-pack-textarea { width: 100%; resize: vertical; min-height: 4rem; font-family: var(--ts-mono, monospace); font-size: 0.8rem; }
.pp-pack-details summary { cursor: pointer; font-size: 0.85rem; color: var(--ts-text-muted, #aab); }
.pp-pack-details pre {
  background: var(--ts-bg-panel);
  border-radius: var(--ts-radius-sm, 6px);
  padding: 0.6rem;
  overflow: auto;
  max-height: 14rem;
  font-size: 0.75rem;
}
.pp-pack-error {
  margin: 0.5rem 0 0;
  padding: 0.5rem 0.7rem;
  font-size: 0.8rem;
  color: var(--ts-warning, #c80);
  background: var(--ts-warning-bg, rgba(255, 200, 80, 0.06));
  border: 1px solid var(--ts-warning, rgba(255, 200, 80, 0.4));
  border-radius: var(--ts-radius-sm, 6px);
}
.pp-pack-report {
  margin: 0;
  padding: 0.5rem 0.7rem;
  font-size: 0.85rem;
  background: var(--ts-success-bg, rgba(80, 200, 140, 0.06));
  border: 1px solid var(--ts-accent, rgba(80, 200, 140, 0.4));
  border-radius: var(--ts-radius-sm, 6px);
}
.pp-pack-skipped { margin: 0.4rem 0 0 1.2rem; font-size: 0.8rem; color: var(--ts-text-muted, #aab); }
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
</style>
