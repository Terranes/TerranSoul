import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type {
  CodingWorkflowConfig,
  CodingWorkflowPreview,
} from '../types';

/**
 * Default config used when the backend has no persisted file yet.
 * Matches the Rust `CodingWorkflowConfig::default` impl exactly so the
 * "Reset" button never has to round-trip through Tauri to know what
 * the defaults are.
 */
export function defaultCodingWorkflowConfig(): CodingWorkflowConfig {
  return {
    include_dirs: ['rules', 'instructions', 'docs'],
    include_files: ['README.md', 'AGENTS.md'],
    exclude_paths: [],
    max_file_chars: 4_000,
    max_total_chars: 30_000,
  };
}

function cloneConfig(c: CodingWorkflowConfig): CodingWorkflowConfig {
  return {
    include_dirs: [...c.include_dirs],
    include_files: [...c.include_files],
    exclude_paths: [...c.exclude_paths],
    max_file_chars: c.max_file_chars,
    max_total_chars: c.max_total_chars,
  };
}

function configsEqual(a: CodingWorkflowConfig, b: CodingWorkflowConfig): boolean {
  if (a.max_file_chars !== b.max_file_chars) return false;
  if (a.max_total_chars !== b.max_total_chars) return false;
  const arrEq = (x: string[], y: string[]) =>
    x.length === y.length && x.every((v, i) => v === y[i]);
  return (
    arrEq(a.include_dirs, b.include_dirs) &&
    arrEq(a.include_files, b.include_files) &&
    arrEq(a.exclude_paths, b.exclude_paths)
  );
}

function isValidConfig(c: unknown): c is CodingWorkflowConfig {
  if (!c || typeof c !== 'object') return false;
  const obj = c as Partial<CodingWorkflowConfig>;
  return (
    Array.isArray(obj.include_dirs) &&
    Array.isArray(obj.include_files) &&
    Array.isArray(obj.exclude_paths) &&
    typeof obj.max_file_chars === 'number' &&
    typeof obj.max_total_chars === 'number'
  );
}

/**
 * Pinia store backing the CodingWorkflowConfigPanel. Tracks a working
 * draft (`config`) plus a `pristine` snapshot from the last successful
 * load/save so the UI can show an "unsaved changes" affordance.
 */
export const useCodingWorkflowStore = defineStore('coding-workflow', () => {
  const config = ref<CodingWorkflowConfig>(defaultCodingWorkflowConfig());
  const pristine = ref<CodingWorkflowConfig>(defaultCodingWorkflowConfig());
  const preview = ref<CodingWorkflowPreview | null>(null);
  const loading = ref(false);
  const saving = ref(false);
  const previewing = ref(false);
  const lastError = ref<string | null>(null);

  const isDirty = computed(() => !configsEqual(config.value, pristine.value));

  async function load(): Promise<void> {
    loading.value = true;
    lastError.value = null;
    try {
      const cfg = await invoke<CodingWorkflowConfig | null>('get_coding_workflow_config');
      // Resilience: if the backend command is unavailable or returns
      // a non-object (test stub, mocked env, dev-time hot-reload), fall
      // back to in-memory defaults so the panel still renders.
      const safe = isValidConfig(cfg) ? cfg : defaultCodingWorkflowConfig();
      config.value = cloneConfig(safe);
      pristine.value = cloneConfig(safe);
    } catch (err) {
      lastError.value = String(err);
    } finally {
      loading.value = false;
    }
  }

  async function save(): Promise<void> {
    saving.value = true;
    lastError.value = null;
    try {
      const saved = await invoke<CodingWorkflowConfig>('set_coding_workflow_config', {
        config: cloneConfig(config.value),
      });
      config.value = cloneConfig(saved);
      pristine.value = cloneConfig(saved);
    } catch (err) {
      lastError.value = String(err);
      throw err;
    } finally {
      saving.value = false;
    }
  }

  async function reset(): Promise<void> {
    saving.value = true;
    lastError.value = null;
    try {
      const fresh = await invoke<CodingWorkflowConfig>('reset_coding_workflow_config');
      config.value = cloneConfig(fresh);
      pristine.value = cloneConfig(fresh);
    } catch (err) {
      lastError.value = String(err);
      throw err;
    } finally {
      saving.value = false;
    }
  }

  async function refreshPreview(useDraft = true): Promise<void> {
    previewing.value = true;
    try {
      const result = await invoke<CodingWorkflowPreview | null>(
        'preview_coding_workflow_context',
        useDraft ? { config: cloneConfig(config.value) } : { config: null },
      );
      preview.value =
        result && typeof result === 'object' && Array.isArray(result.documents)
          ? result
          : null;
    } catch (err) {
      lastError.value = String(err);
      preview.value = null;
    } finally {
      previewing.value = false;
    }
  }

  function discardChanges(): void {
    config.value = cloneConfig(pristine.value);
  }

  function addEntry(field: 'include_dirs' | 'include_files' | 'exclude_paths', value: string): boolean {
    const trimmed = value.trim();
    if (!trimmed) return false;
    if (config.value[field].includes(trimmed)) return false;
    config.value[field] = [...config.value[field], trimmed];
    return true;
  }

  function removeEntry(field: 'include_dirs' | 'include_files' | 'exclude_paths', value: string): void {
    config.value[field] = config.value[field].filter((v) => v !== value);
  }

  return {
    config,
    pristine,
    preview,
    loading,
    saving,
    previewing,
    lastError,
    isDirty,
    load,
    save,
    reset,
    refreshPreview,
    discardChanges,
    addEntry,
    removeEntry,
  };
});
