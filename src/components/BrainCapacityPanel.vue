<script setup lang="ts">
import { computed } from 'vue';
import {
  useSettingsStore,
  DEFAULT_MAX_LONG_TERM_ENTRIES,
  DEFAULT_RELEVANCE_THRESHOLD,
  DEFAULT_MAINTENANCE_INTERVAL_HOURS,
  DEFAULT_SQLITE_CACHE_MB,
  DEFAULT_SQLITE_MMAP_MB,
  DEFAULT_CODE_INDEX_CACHE_MB,
  DEFAULT_CODE_INDEX_MMAP_MB,
} from '../stores/settings';
import { useMemoryStore } from '../stores/memory';

const appSettings = useSettingsStore();
const memoryStore = useMemoryStore();

// ── Computed settings with defaults ──────────────────────────────────────────

const maxLongTermEntries = computed(() => appSettings.settings?.max_long_term_entries ?? DEFAULT_MAX_LONG_TERM_ENTRIES);
const maxMemoryGb = computed(() => appSettings.settings?.max_memory_gb ?? 10);
const maxMemoryMb = computed(() => appSettings.settings?.max_memory_mb ?? 10);
const relevanceThreshold = computed(() => appSettings.settings?.relevance_threshold ?? DEFAULT_RELEVANCE_THRESHOLD);
const contextualRetrieval = computed(() => appSettings.settings?.contextual_retrieval ?? false);
const lateChunking = computed(() => appSettings.settings?.late_chunking ?? false);
const webSearchEnabled = computed(() => appSettings.settings?.web_search_enabled ?? false);
const backgroundMaintenance = computed(() => appSettings.settings?.background_maintenance_enabled ?? true);
const debugLogging = computed(() => appSettings.settings?.debug_logging ?? false);
const maintenanceInterval = computed(() => appSettings.settings?.maintenance_interval_hours ?? DEFAULT_MAINTENANCE_INTERVAL_HOURS);
const maintenanceIdle = computed(() => appSettings.settings?.maintenance_idle_minimum_minutes ?? 5);
const dataRoot = computed(() => appSettings.settings?.data_root ?? '');

// ── SQLite tuning computeds ──────────────────────────────────────────────────

const sqliteCacheMb = computed(() => appSettings.settings?.sqlite_cache_mb ?? DEFAULT_SQLITE_CACHE_MB);
const sqliteMmapMb = computed(() => appSettings.settings?.sqlite_mmap_mb ?? DEFAULT_SQLITE_MMAP_MB);
const codeIndexCacheMb = computed(() => appSettings.settings?.code_index_cache_mb ?? DEFAULT_CODE_INDEX_CACHE_MB);
const codeIndexMmapMb = computed(() => appSettings.settings?.code_index_mmap_mb ?? DEFAULT_CODE_INDEX_MMAP_MB);

// ── Memory stats ─────────────────────────────────────────────────────────────

const stats = computed(() => memoryStore.stats);
const totalMemories = computed(() => stats.value?.total ?? 0);
const shortCount = computed(() => stats.value?.short_count ?? 0);
const workingCount = computed(() => stats.value?.working_count ?? 0);
const longCount = computed(() => stats.value?.long_count ?? 0);
const storageBytes = computed(() => stats.value?.storage_bytes ?? 0);

// ── Derived metrics ──────────────────────────────────────────────────────────

const capacityPercent = computed(() => {
  if (maxLongTermEntries.value <= 0) return 0;
  return Math.min(100, Math.round((longCount.value / maxLongTermEntries.value) * 100));
});

const storageUsedMb = computed(() => Math.round((storageBytes.value / (1024 * 1024)) * 10) / 10);
const storageCapGb = computed(() => maxMemoryGb.value);
const storagePercent = computed(() => {
  const capBytes = storageCapGb.value * 1024 * 1024 * 1024;
  if (capBytes <= 0) return 0;
  return Math.min(100, Math.round((storageBytes.value / capBytes) * 100));
});

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return String(n);
}

// ── Event handlers ───────────────────────────────────────────────────────────

function onCapacityChange(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  appSettings.saveMaxLongTermEntries(val);
}

function onStorageGbChange(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  appSettings.saveMaxMemoryGb(val);
}

function onCacheMbChange(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  appSettings.saveMaxMemoryMb(val);
}

function onRelevanceChange(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  appSettings.saveRelevanceThreshold(val);
}

function onToggleContextualRetrieval(checked: boolean) {
  appSettings.saveSettings({ contextual_retrieval: checked });
}

function onToggleLateChunking(checked: boolean) {
  appSettings.saveSettings({ late_chunking: checked });
}

function onToggleWebSearch(checked: boolean) {
  appSettings.saveSettings({ web_search_enabled: checked });
}

function onToggleMaintenance(checked: boolean) {
  appSettings.saveSettings({ background_maintenance_enabled: checked });
}

function onToggleDebugLogging(checked: boolean) {
  appSettings.saveSettings({ debug_logging: checked });
}

function onMaintenanceIntervalChange(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  appSettings.saveMaintenanceInterval(val);
}

function onMaintenanceIdleChange(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  appSettings.saveSettings({ maintenance_idle_minimum_minutes: Math.max(1, Math.round(val)) });
}

function onDataRootChange(e: Event) {
  const val = (e.target as HTMLInputElement).value.trim();
  appSettings.saveSettings({ data_root: val || undefined });
}

function onSqliteCacheMbChange(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  appSettings.saveSqliteCacheMb(val);
}

function onSqliteMmapMbChange(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  appSettings.saveSqliteMmapMb(val);
}

function onCodeIndexCacheMbChange(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  appSettings.saveCodeIndexCacheMb(val);
}

function onCodeIndexMmapMbChange(e: Event) {
  const val = Number((e.target as HTMLInputElement).value);
  appSettings.saveCodeIndexMmapMb(val);
}
</script>

<template>
  <div
    class="bcp"
    data-testid="brain-capacity-panel"
  >
    <!-- ── Header ──────────────────────────────────────────────────────────── -->
    <header class="bcp-header">
      <h3 class="bcp-title">
        🧠 Brain Capacity & Storage
      </h3>
      <span class="bcp-subtitle">
        Memory tiers, RAG tuning, and resource limits — all in one place
      </span>
    </header>

    <!-- ── Memory Tiers Overview ───────────────────────────────────────────── -->
    <section class="bcp-section">
      <h4 class="bcp-section-title">
        Memory Tiers
      </h4>
      <div class="bcp-tiers">
        <div class="bcp-tier bcp-tier--short">
          <span class="bcp-tier-num">{{ formatNumber(shortCount) }}</span>
          <span class="bcp-tier-label">Short-term</span>
          <span class="bcp-tier-desc">Ephemeral, session-scoped</span>
        </div>
        <div class="bcp-tier bcp-tier--working">
          <span class="bcp-tier-num">{{ formatNumber(workingCount) }}</span>
          <span class="bcp-tier-label">Working</span>
          <span class="bcp-tier-desc">Active context window</span>
        </div>
        <div class="bcp-tier bcp-tier--long">
          <span class="bcp-tier-num">{{ formatNumber(longCount) }}</span>
          <span class="bcp-tier-label">Long-term</span>
          <span class="bcp-tier-desc">Persistent, RAG-indexed</span>
        </div>
        <div class="bcp-tier bcp-tier--total">
          <span class="bcp-tier-num">{{ formatNumber(totalMemories) }}</span>
          <span class="bcp-tier-label">Total</span>
          <span class="bcp-tier-desc">All entries combined</span>
        </div>
      </div>
    </section>

    <!-- ── Capacity Bar ────────────────────────────────────────────────────── -->
    <section class="bcp-section">
      <h4 class="bcp-section-title">
        Long-term Capacity
      </h4>
      <div class="bcp-capacity-bar-wrap">
        <div class="bcp-capacity-bar">
          <div
            class="bcp-capacity-fill"
            :style="{ width: capacityPercent + '%' }"
            :class="{ 'bcp-capacity-fill--warn': capacityPercent >= 80, 'bcp-capacity-fill--crit': capacityPercent >= 95 }"
          />
        </div>
        <span class="bcp-capacity-label">
          {{ formatNumber(longCount) }} / {{ formatNumber(maxLongTermEntries) }} entries
          ({{ capacityPercent }}%)
        </span>
      </div>
      <div class="bcp-field">
        <label class="bcp-label">Max long-term entries</label>
        <input
          type="range"
          :min="1000"
          :max="10000000"
          :step="10000"
          :value="maxLongTermEntries"
          class="bcp-range"
          data-testid="bcp-max-entries-range"
          @change="onCapacityChange"
        >
        <span class="bcp-value">{{ formatNumber(maxLongTermEntries) }}</span>
      </div>
      <p class="bcp-hint">
        When long-term count exceeds this cap, lowest-value entries are evicted
        (respecting protected + importance ≥ 4) until count ≤ cap × 0.95.
      </p>
    </section>

    <!-- ── Storage & RAM ───────────────────────────────────────────────────── -->
    <section class="bcp-section">
      <h4 class="bcp-section-title">
        Storage & RAM
      </h4>

      <div class="bcp-storage-row">
        <div class="bcp-storage-card">
          <span class="bcp-storage-icon">💾</span>
          <div class="bcp-storage-info">
            <span class="bcp-storage-metric">{{ storageUsedMb }} MB used</span>
            <div class="bcp-mini-bar">
              <div
                class="bcp-mini-fill"
                :style="{ width: storagePercent + '%' }"
              />
            </div>
            <span class="bcp-storage-cap">of {{ storageCapGb }} GB cap</span>
          </div>
        </div>
        <div class="bcp-storage-card">
          <span class="bcp-storage-icon">🧮</span>
          <div class="bcp-storage-info">
            <span class="bcp-storage-metric">{{ maxMemoryMb }} MB</span>
            <span class="bcp-storage-cap">in-memory cache limit</span>
          </div>
        </div>
      </div>

      <div class="bcp-field">
        <label class="bcp-label">Max storage (GB)</label>
        <input
          type="range"
          :min="1"
          :max="100"
          :step="1"
          :value="maxMemoryGb"
          class="bcp-range"
          data-testid="bcp-storage-gb-range"
          @change="onStorageGbChange"
        >
        <span class="bcp-value">{{ maxMemoryGb }} GB</span>
      </div>
      <div class="bcp-field">
        <label class="bcp-label">RAM cache (MB)</label>
        <input
          type="range"
          :min="1"
          :max="1024"
          :step="1"
          :value="maxMemoryMb"
          class="bcp-range"
          data-testid="bcp-cache-mb-range"
          @change="onCacheMbChange"
        >
        <span class="bcp-value">{{ maxMemoryMb }} MB</span>
      </div>
      <div class="bcp-field">
        <label class="bcp-label">Data directory override</label>
        <input
          type="text"
          :value="dataRoot"
          placeholder="(default platform path)"
          class="bcp-input"
          data-testid="bcp-data-root"
          @change="onDataRootChange"
        >
      </div>
    </section>

    <!-- ── Database Tuning (SQLite) ────────────────────────────────────────── -->
    <section class="bcp-section">
      <h4 class="bcp-section-title">
        Database Tuning
      </h4>
      <p class="bcp-desc">
        SQLite page-cache and mmap sizes control how much RAM the database engine uses.
        Larger values speed up queries but consume more memory.
        <strong>Changes take effect on next app restart.</strong>
      </p>

      <h5 class="bcp-subsection-title">
        Memory DB (memory.db)
      </h5>
      <div class="bcp-field">
        <label class="bcp-label">Page cache (MiB)</label>
        <input
          type="number"
          :min="2"
          :max="512"
          :value="sqliteCacheMb"
          class="bcp-input bcp-input--sm"
          data-testid="bcp-sqlite-cache-mb"
          @change="onSqliteCacheMbChange"
        >
        <span class="bcp-value">MiB</span>
      </div>
      <div class="bcp-field">
        <label class="bcp-label">Mmap window (MiB)</label>
        <input
          type="number"
          :min="0"
          :max="2048"
          :value="sqliteMmapMb"
          class="bcp-input bcp-input--sm"
          data-testid="bcp-sqlite-mmap-mb"
          @change="onSqliteMmapMbChange"
        >
        <span class="bcp-value">MiB</span>
      </div>

      <h5 class="bcp-subsection-title">
        Code Index (code_index.sqlite)
      </h5>
      <div class="bcp-field">
        <label class="bcp-label">Page cache (MiB)</label>
        <input
          type="number"
          :min="2"
          :max="256"
          :value="codeIndexCacheMb"
          class="bcp-input bcp-input--sm"
          data-testid="bcp-code-index-cache-mb"
          @change="onCodeIndexCacheMbChange"
        >
        <span class="bcp-value">MiB</span>
      </div>
      <div class="bcp-field">
        <label class="bcp-label">Mmap window (MiB)</label>
        <input
          type="number"
          :min="0"
          :max="1024"
          :value="codeIndexMmapMb"
          class="bcp-input bcp-input--sm"
          data-testid="bcp-code-index-mmap-mb"
          @change="onCodeIndexMmapMbChange"
        >
        <span class="bcp-value">MiB</span>
      </div>

      <p class="bcp-hint">
        Page cache = hot pages kept in RAM. Mmap = OS-managed virtual memory window (0 = disabled).
        Defaults: memory.db 16/64 MiB, code index 8/32 MiB.
      </p>
    </section>

    <!-- ── RAG Pipeline Tuning ─────────────────────────────────────────────── -->
    <section class="bcp-section">
      <h4 class="bcp-section-title">
        RAG Pipeline
      </h4>
      <p class="bcp-desc">
        6-signal hybrid scoring: vector (40%) · keyword (20%) · recency (15%) · importance (10%) · decay (10%) · tier (5%).
        Fused via Reciprocal Rank Fusion (k=60).
      </p>

      <div class="bcp-field">
        <label class="bcp-label">Relevance threshold</label>
        <input
          type="range"
          :min="0"
          :max="1"
          :step="0.01"
          :value="relevanceThreshold"
          class="bcp-range"
          data-testid="bcp-relevance-range"
          @change="onRelevanceChange"
        >
        <span class="bcp-value">{{ (relevanceThreshold * 100).toFixed(0) }}%</span>
      </div>
      <p class="bcp-hint">
        Only memories scoring above this threshold are injected into RAG context.
        Lower = more recall, higher = more precision.
      </p>

      <div class="bcp-toggles">
        <label class="bcp-toggle">
          <input
            type="checkbox"
            :checked="contextualRetrieval"
            data-testid="bcp-contextual-retrieval"
            @change="onToggleContextualRetrieval(($event.target as HTMLInputElement).checked)"
          >
          <span class="bcp-toggle-text">
            <strong>Contextual retrieval</strong>
            <small>Prepend document context to chunks before embedding (Anthropic 2024)</small>
          </span>
        </label>
        <label class="bcp-toggle">
          <input
            type="checkbox"
            :checked="lateChunking"
            data-testid="bcp-late-chunking"
            @change="onToggleLateChunking(($event.target as HTMLInputElement).checked)"
          >
          <span class="bcp-toggle-text">
            <strong>Late chunking</strong>
            <small>Whole-document token vectors for per-chunk pooling (local only)</small>
          </span>
        </label>
        <label class="bcp-toggle">
          <input
            type="checkbox"
            :checked="webSearchEnabled"
            data-testid="bcp-web-search"
            @change="onToggleWebSearch(($event.target as HTMLInputElement).checked)"
          >
          <span class="bcp-toggle-text">
            <strong>Web search fallback</strong>
            <small>CRAG: if local retrieval is insufficient, query DuckDuckGo for supplemental context</small>
          </span>
        </label>
      </div>
    </section>

    <!-- ── Maintenance ─────────────────────────────────────────────────────── -->
    <section class="bcp-section">
      <h4 class="bcp-section-title">
        Background Maintenance
      </h4>
      <label class="bcp-toggle">
        <input
          type="checkbox"
          :checked="backgroundMaintenance"
          data-testid="bcp-maintenance-toggle"
          @change="onToggleMaintenance(($event.target as HTMLInputElement).checked)"
        >
        <span class="bcp-toggle-text">
          <strong>Enable background jobs</strong>
          <small>Decay scoring, GC, tier promotion, embedding self-heal</small>
        </span>
      </label>

      <template v-if="backgroundMaintenance">
        <div class="bcp-field">
          <label class="bcp-label">Run every</label>
          <input
            type="number"
            :min="1"
            :max="168"
            :value="maintenanceInterval"
            class="bcp-input bcp-input--sm"
            data-testid="bcp-maintenance-interval"
            @change="onMaintenanceIntervalChange"
          >
          <span class="bcp-value">hours</span>
        </div>
        <div class="bcp-field">
          <label class="bcp-label">Idle minimum</label>
          <input
            type="number"
            :min="1"
            :max="60"
            :value="maintenanceIdle"
            class="bcp-input bcp-input--sm"
            data-testid="bcp-maintenance-idle"
            @change="onMaintenanceIdleChange"
          >
          <span class="bcp-value">minutes</span>
        </div>
        <p class="bcp-hint">
          Maintenance only fires after the user has been idle for at least
          this many minutes — avoids interrupting active sessions.
        </p>
      </template>
    </section>

    <!-- ── Debug ───────────────────────────────────────────────────────────── -->
    <section class="bcp-section">
      <h4 class="bcp-section-title">
        Debug
      </h4>
      <label class="bcp-toggle">
        <input
          type="checkbox"
          :checked="debugLogging"
          data-testid="bcp-debug-logging-toggle"
          @change="onToggleDebugLogging(($event.target as HTMLInputElement).checked)"
        >
        <span class="bcp-toggle-text">
          <strong>Verbose debug logging</strong>
          <small>Print brain internals to stderr (e.g. chat-rewarm timings, embed status). Disable in production.</small>
        </span>
      </label>
    </section>
  </div>
</template>

<style scoped>
.bcp {
  --bcp-radius: 14px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  width: 100%;
  min-width: 0;
  padding: 24px;
  overflow-x: hidden;
  background: var(--ts-glass-bg, rgba(15, 15, 30, 0.7));
  border: 1px solid var(--ts-glass-border, rgba(255, 255, 255, 0.08));
  border-radius: var(--bcp-radius);
  backdrop-filter: blur(var(--ts-glass-blur, 12px));
}

.bcp-header {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.bcp-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--ts-text, #f0f0f0);
}

.bcp-subtitle {
  font-size: 0.85rem;
  color: var(--ts-text-muted, #aaa);
}

/* ── Sections ─────────────────────────────────────────────────────────────── */

.bcp-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 16px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 10px;
}

.bcp-section-title {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--ts-text, #f0f0f0);
}

.bcp-subsection-title {
  margin: 6px 0 0;
  font-size: 0.84rem;
  font-weight: 600;
  color: var(--ts-text-muted, #ccc);
}

.bcp-desc {
  margin: 0;
  font-size: 0.82rem;
  color: var(--ts-text-muted, #aaa);
  line-height: 1.4;
}

.bcp-hint {
  margin: 0;
  font-size: 0.78rem;
  color: var(--ts-text-muted, #888);
  font-style: italic;
}

/* ── Tiers ────────────────────────────────────────────────────────────────── */

.bcp-tiers {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  gap: 10px;
}

.bcp-tier {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 12px 8px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.06);
  text-align: center;
}

.bcp-tier--short { border-color: rgba(255, 200, 80, 0.25); }
.bcp-tier--working { border-color: rgba(80, 180, 255, 0.25); }
.bcp-tier--long { border-color: rgba(120, 255, 160, 0.25); }
.bcp-tier--total { border-color: rgba(200, 140, 255, 0.25); }

.bcp-tier-num {
  font-size: 1.4rem;
  font-weight: 700;
  color: var(--ts-text, #f0f0f0);
}

.bcp-tier-label {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--ts-text, #ddd);
}

.bcp-tier-desc {
  font-size: 0.72rem;
  color: var(--ts-text-muted, #888);
}

/* ── Capacity bar ─────────────────────────────────────────────────────────── */

.bcp-capacity-bar-wrap {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.bcp-capacity-bar {
  height: 10px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 5px;
  overflow: hidden;
}

.bcp-capacity-fill {
  height: 100%;
  background: linear-gradient(90deg, #6ee7b7, #34d399);
  border-radius: 5px;
  transition: width 0.4s ease;
}

.bcp-capacity-fill--warn {
  background: linear-gradient(90deg, #fbbf24, #f59e0b);
}

.bcp-capacity-fill--crit {
  background: linear-gradient(90deg, #f87171, #ef4444);
}

.bcp-capacity-label {
  font-size: 0.78rem;
  color: var(--ts-text-muted, #aaa);
}

/* ── Storage cards ────────────────────────────────────────────────────────── */

.bcp-storage-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 10px;
}

.bcp-storage-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.bcp-storage-icon {
  font-size: 1.6rem;
}

.bcp-storage-info {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.bcp-storage-metric {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--ts-text, #f0f0f0);
}

.bcp-storage-cap {
  font-size: 0.75rem;
  color: var(--ts-text-muted, #888);
}

.bcp-mini-bar {
  width: 80px;
  height: 4px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 2px;
  overflow: hidden;
}

.bcp-mini-fill {
  height: 100%;
  background: var(--ts-accent, #7c5bff);
  border-radius: 2px;
  transition: width 0.3s ease;
}

/* ── Fields ───────────────────────────────────────────────────────────────── */

.bcp-field {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.bcp-label {
  min-width: 140px;
  font-size: 0.82rem;
  color: var(--ts-text-muted, #bbb);
  white-space: nowrap;
}

.bcp-range {
  flex: 1;
  min-width: 0;
  accent-color: var(--ts-accent, #7c5bff);
}

.bcp-value {
  min-width: 60px;
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--ts-text, #f0f0f0);
  text-align: right;
}

.bcp-input {
  flex: 1;
  min-width: 0;
  padding: 6px 10px;
  font-size: 0.82rem;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  color: var(--ts-text, #f0f0f0);
  outline: none;
  transition: border-color 0.2s;
}

.bcp-input:focus {
  border-color: var(--ts-accent, #7c5bff);
}

.bcp-input--sm {
  flex: 0;
  width: 72px;
  text-align: center;
}

/* ── Toggles ──────────────────────────────────────────────────────────────── */

.bcp-toggles {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.bcp-toggle {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s;
}

.bcp-toggle:hover {
  background: rgba(255, 255, 255, 0.04);
}

.bcp-toggle input[type="checkbox"] {
  margin-top: 3px;
  accent-color: var(--ts-accent, #7c5bff);
}

.bcp-toggle-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.bcp-toggle-text strong {
  font-size: 0.85rem;
  color: var(--ts-text, #f0f0f0);
}

.bcp-toggle-text small {
  font-size: 0.75rem;
  color: var(--ts-text-muted, #888);
}

@media (max-width: 480px) {
  .bcp {
    gap: 14px;
    padding: 14px;
  }
  .bcp-section {
    padding: 12px;
  }
  .bcp-tiers,
  .bcp-storage-row {
    grid-template-columns: 1fr;
  }
  .bcp-field {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px 10px;
  }
  .bcp-label {
    grid-column: 1 / -1;
    min-width: 0;
    white-space: normal;
  }
  .bcp-input:not(.bcp-input--sm) {
    grid-column: 1 / -1;
  }
  .bcp-value {
    min-width: 0;
  }
}
</style>
