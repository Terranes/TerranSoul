<template>
  <section class="ck-panel">
    <header class="ck-header">
      <h3>💻 Code knowledge (GitNexus)</h3>
      <span class="ck-subtitle">
        Mirror an indexed repo's knowledge graph into your brain so
        memory traversal can reason over code structure. Strictly opt-in
        — nothing syncs at startup.
        <a
          class="ck-link"
          href="https://github.com/Terranes/TerranSoul/blob/main/docs/brain-advanced-design.md"
          target="_blank"
          rel="noopener"
        >Phase 13 design →</a>
      </span>
    </header>

    <!-- ── Sync form ──────────────────────────────────────────────── -->
    <div class="ck-row">
      <label class="ck-field">
        <span class="ck-label">Repo scope</span>
        <input
          v-model="syncScope"
          type="text"
          class="ck-input"
          placeholder="repo:owner/name@sha"
          :disabled="syncBusy"
          @keydown.enter.prevent="onSync"
        >
      </label>
      <button
        type="button"
        class="ck-btn ck-btn-primary"
        :disabled="!canSync"
        @click="onSync"
      >
        {{ syncBusy ? 'Syncing…' : 'Sync KG' }}
      </button>
    </div>

    <p
      v-if="lastSyncReport"
      class="ck-report ck-report-ok"
    >
      ✓ Mirrored
      <strong>{{ lastSyncReport.nodes_inserted }}</strong> new nodes
      (<strong>{{ lastSyncReport.nodes_reused }}</strong> reused),
      <strong>{{ lastSyncReport.edges_inserted }}</strong> edges
      ({{ lastSyncReport.edges_skipped }} skipped) under
      <code>{{ lastSyncReport.edge_source }}</code>.
    </p>
    <p
      v-if="lastError"
      class="ck-report ck-report-err"
      role="alert"
    >
      ⚠ {{ lastError }}
    </p>

    <!-- ── Mirror list ─────────────────────────────────────────────── -->
    <div class="ck-mirrors">
      <h4>Mirrored repos</h4>
      <p
        v-if="mirrors.length === 0 && !mirrorsLoading"
        class="ck-empty"
      >
        No repos mirrored yet. Sync one above to get started.
      </p>
      <ul
        v-else
        class="ck-mirror-list"
      >
        <li
          v-for="m in mirrors"
          :key="m.edge_source"
          class="ck-mirror-row"
        >
          <div class="ck-mirror-info">
            <code class="ck-mirror-scope">{{ m.scope }}</code>
            <span class="ck-mirror-meta">
              {{ m.edge_count }} edge{{ m.edge_count === 1 ? '' : 's' }}
              · last sync {{ formatTimestamp(m.last_synced_at) }}
            </span>
          </div>
          <button
            type="button"
            class="ck-btn ck-btn-danger"
            :disabled="busyScope === m.scope"
            @click="onUnmirror(m.scope)"
          >
            {{ busyScope === m.scope ? 'Removing…' : 'Unmirror' }}
          </button>
        </li>
      </ul>
    </div>

    <!-- ── Blast radius ────────────────────────────────────────────── -->
    <div class="ck-row ck-impact">
      <label class="ck-field">
        <span class="ck-label">Blast-radius pre-flight</span>
        <input
          v-model="impactSymbol"
          type="text"
          class="ck-input"
          placeholder="module::path::Symbol"
          :disabled="impactBusy"
          @keydown.enter.prevent="onImpact"
        >
      </label>
      <button
        type="button"
        class="ck-btn"
        :disabled="!canImpact"
        @click="onImpact"
      >
        {{ impactBusy ? 'Probing…' : 'Probe impact' }}
      </button>
    </div>
    <p
      v-if="impactSummary"
      class="ck-report ck-report-info"
    >
      🎯 {{ impactSummary }}
    </p>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

/** Mirror summary as returned by the `gitnexus_list_mirrors` Tauri command. */
interface MirrorSummary {
  edge_source: string;
  scope: string;
  edge_count: number;
  last_synced_at: number;
}

/** Insert/skip counts returned by `gitnexus_sync`. */
interface MirrorReport {
  edge_source: string;
  nodes_inserted: number;
  nodes_reused: number;
  edges_inserted: number;
  edges_skipped: number;
}

const syncScope = ref('');
const syncBusy = ref(false);
const lastSyncReport = ref<MirrorReport | null>(null);
const lastError = ref<string | null>(null);

const mirrors = ref<MirrorSummary[]>([]);
const mirrorsLoading = ref(false);
const busyScope = ref<string | null>(null);

const impactSymbol = ref('');
const impactBusy = ref(false);
const impactSummary = ref<string | null>(null);

const canSync = computed(
  () => !syncBusy.value && syncScope.value.trim().length > 0,
);
const canImpact = computed(
  () => !impactBusy.value && impactSymbol.value.trim().length > 0,
);

async function refreshMirrors(): Promise<void> {
  mirrorsLoading.value = true;
  try {
    const result = await invoke<MirrorSummary[]>('gitnexus_list_mirrors');
    // Defensive: an unmocked Tauri stub may resolve `undefined` in tests.
    // Always normalise to an array so the template never crashes.
    mirrors.value = Array.isArray(result) ? result : [];
  } catch (err) {
    // Non-fatal — surface in a separate slot so a list-only failure
    // doesn't wipe the form.
    lastError.value = `Could not list mirrored repos: ${stringifyError(err)}`;
  } finally {
    mirrorsLoading.value = false;
  }
}

async function onSync(): Promise<void> {
  if (!canSync.value) return;
  syncBusy.value = true;
  lastError.value = null;
  try {
    const report = await invoke<MirrorReport>('gitnexus_sync', {
      repoLabel: syncScope.value.trim(),
    });
    lastSyncReport.value = report;
    await refreshMirrors();
  } catch (err) {
    lastError.value = stringifyError(err);
  } finally {
    syncBusy.value = false;
  }
}

async function onUnmirror(scope: string): Promise<void> {
  if (busyScope.value !== null) return;
  busyScope.value = scope;
  lastError.value = null;
  try {
    await invoke<number>('gitnexus_unmirror', { repoLabel: scope });
    await refreshMirrors();
  } catch (err) {
    lastError.value = stringifyError(err);
  } finally {
    busyScope.value = null;
  }
}

async function onImpact(): Promise<void> {
  if (!canImpact.value) return;
  impactBusy.value = true;
  lastError.value = null;
  impactSummary.value = null;
  try {
    const raw = await invoke<unknown>('gitnexus_impact', {
      symbol: impactSymbol.value.trim(),
    });
    impactSummary.value = summariseImpact(raw);
  } catch (err) {
    lastError.value = stringifyError(err);
  } finally {
    impactBusy.value = false;
  }
}

/**
 * Format an Unix-ms timestamp as an `Intl.DateTimeFormat` short string.
 * Returns "—" for falsy values so empty rows don't render `Invalid Date`.
 */
function formatTimestamp(ms: number): string {
  if (!ms || ms <= 0) return '—';
  try {
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(new Date(ms));
  } catch {
    return '—';
  }
}

/**
 * Pull a one-line summary out of a GitNexus impact response. The
 * upstream payload shape varies; we try the two most common ones
 * (`{ symbol, dependents: [...] }` and `{ count, items: [...] }`),
 * then fall back to a JSON snippet.
 */
function summariseImpact(raw: unknown): string {
  if (raw && typeof raw === 'object') {
    const obj = raw as Record<string, unknown>;
    const arr = (obj['dependents'] ?? obj['items'] ?? obj['symbols']) as
      | unknown[]
      | undefined;
    if (Array.isArray(arr)) {
      const n = arr.length;
      const sym = (obj['symbol'] as string | undefined) ?? impactSymbol.value;
      return `${n} dependent${n === 1 ? '' : 's'} of ${sym}`;
    }
    const count = obj['count'];
    if (typeof count === 'number') {
      return `${count} dependents`;
    }
  }
  return JSON.stringify(raw).slice(0, 200);
}

function stringifyError(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === 'string') return err;
  return JSON.stringify(err);
}

onMounted(() => {
  void refreshMirrors();
});

defineExpose({ refreshMirrors, summariseImpact, formatTimestamp });
</script>

<style scoped>
.ck-panel {
  background: var(--ts-bg-card, rgba(20, 18, 38, 0.85));
  border: 1px solid var(--ts-border, rgba(168, 156, 255, 0.18));
  border-radius: var(--ts-radius-lg, 12px);
  padding: var(--ts-space-lg, 18px);
  display: flex;
  flex-direction: column;
  gap: var(--ts-space-md, 14px);
  color: var(--ts-text, #ece8ff);
}

.ck-header {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.ck-header h3 {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 700;
}

.ck-subtitle {
  font-size: 0.85rem;
  color: var(--ts-text-muted, #b6acdb);
  line-height: 1.4;
}

.ck-link {
  color: var(--ts-accent, #a89cff);
  text-decoration: none;
}

.ck-link:hover {
  text-decoration: underline;
}

.ck-row {
  display: flex;
  align-items: flex-end;
  gap: var(--ts-space-md, 12px);
  flex-wrap: wrap;
}

.ck-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1 1 240px;
}

.ck-label {
  font-size: 0.75rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--ts-text-muted, #b6acdb);
}

.ck-input {
  background: var(--ts-bg-input, rgba(0, 0, 0, 0.35));
  border: 1px solid var(--ts-border, rgba(168, 156, 255, 0.2));
  border-radius: var(--ts-radius-md, 8px);
  padding: 8px 12px;
  color: inherit;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.9rem;
}

.ck-input:focus {
  outline: 2px solid var(--ts-accent, #a89cff);
  outline-offset: 1px;
}

.ck-btn {
  background: var(--ts-bg-input, rgba(0, 0, 0, 0.35));
  border: 1px solid var(--ts-border, rgba(168, 156, 255, 0.25));
  color: inherit;
  border-radius: var(--ts-radius-md, 8px);
  padding: 8px 14px;
  font-weight: 600;
  cursor: pointer;
  transition: background 120ms ease, border-color 120ms ease;
}

.ck-btn:hover:not(:disabled) {
  border-color: var(--ts-accent, #a89cff);
}

.ck-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.ck-btn-primary {
  background: var(--ts-accent, #a89cff);
  color: var(--ts-bg-base);
  border-color: var(--ts-accent, #a89cff);
}

.ck-btn-primary:hover:not(:disabled) {
  filter: brightness(1.08);
}

.ck-btn-danger {
  border-color: var(--ts-error);
  color: var(--ts-error);
}

.ck-btn-danger:hover:not(:disabled) {
  background: var(--ts-error-bg);
}

.ck-report {
  font-size: 0.85rem;
  line-height: 1.4;
  margin: 0;
  padding: 8px 12px;
  border-radius: var(--ts-radius-md, 8px);
}

.ck-report code {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.8rem;
  background: var(--ts-bg-panel);
  padding: 1px 6px;
  border-radius: 4px;
}

.ck-report-ok {
  background: var(--ts-success-bg, rgba(120, 220, 160, 0.1));
  border: 1px solid var(--ts-success);
  color: var(--ts-success);
}

.ck-report-err {
  background: var(--ts-error-bg);
  border: 1px solid var(--ts-error);
  color: var(--ts-error);
}

.ck-report-info {
  background: var(--ts-accent-glow);
  border: 1px solid var(--ts-accent-violet);
  color: var(--ts-accent, #c4baff);
}

.ck-mirrors h4 {
  margin: 0 0 6px;
  font-size: 0.9rem;
  font-weight: 700;
  color: var(--ts-text-muted, #b6acdb);
}

.ck-empty {
  font-size: 0.85rem;
  color: var(--ts-text-muted, #b6acdb);
  font-style: italic;
  margin: 0;
}

.ck-mirror-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.ck-mirror-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--ts-space-md, 12px);
  background: var(--ts-bg-panel);
  border: 1px solid var(--ts-border, rgba(168, 156, 255, 0.12));
  border-radius: var(--ts-radius-md, 8px);
  padding: 8px 12px;
}

.ck-mirror-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.ck-mirror-scope {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.85rem;
  color: var(--ts-text, #ece8ff);
  word-break: break-all;
}

.ck-mirror-meta {
  font-size: 0.75rem;
  color: var(--ts-text-muted, #b6acdb);
}

.ck-impact {
  border-top: 1px dashed var(--ts-border, rgba(168, 156, 255, 0.18));
  padding-top: var(--ts-space-md, 12px);
}
</style>
