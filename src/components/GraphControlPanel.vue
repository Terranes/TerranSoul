<template>
  <div
    class="gcp-panel"
    :class="{ 'gcp-collapsed': collapsed }"
    data-testid="graph-control-panel"
    @pointerdown.stop
    @pointermove.stop
    @pointerup.stop
  >
    <header class="gcp-head">
      <span class="gcp-title">{{ title }}</span>
      <button
        type="button"
        class="gcp-icon-button"
        :title="collapsed ? 'Expand panel' : 'Collapse panel'"
        :aria-label="collapsed ? 'Expand graph controls' : 'Collapse graph controls'"
        @click="$emit('update:collapsed', !collapsed)"
      >
        {{ collapsed ? '▸' : '◂' }}
      </button>
    </header>

    <div
      v-if="!collapsed"
      class="gcp-body"
    >
      <details
        v-if="showViews"
        class="gcp-section"
      >
        <summary>Views</summary>
        <div class="gcp-readout">
          <span>{{ nodeCount }} nodes</span>
          <span>{{ edgeCount }} links</span>
        </div>
        <slot name="views" />
      </details>

      <details
        v-if="showFilters"
        class="gcp-section"
      >
        <summary>Filters</summary>
        <label class="gcp-row">
          <input
            :checked="showOrphans"
            type="checkbox"
            @change="$emit('update:showOrphans', ($event.target as HTMLInputElement).checked)"
          >
          <span>Show orphan nodes</span>
        </label>
        <label class="gcp-row">
          <span>Min connections</span>
          <input
            :value="minDegree"
            type="range"
            min="0"
            max="10"
            step="1"
            @input="$emit('update:minDegree', Number(($event.target as HTMLInputElement).value))"
          >
          <span class="gcp-val">{{ minDegree }}</span>
        </label>
      </details>

      <details
        class="gcp-section"
        open
      >
        <summary>Search &amp; Select</summary>
        <div class="gcp-row gcp-row-stack">
          <input
            :value="searchText"
            type="text"
            placeholder="Filter nodes…"
            class="gcp-input"
            data-testid="gcp-search-input"
            @input="$emit('update:searchText', ($event.target as HTMLInputElement).value)"
            @keydown.enter.exact.prevent="$emit('select-matches')"
            @keydown.enter.shift.prevent="$emit('add-matches')"
            @keydown.enter.alt.prevent="$emit('remove-matches')"
            @keydown.escape.prevent="$emit('update:searchText', '')"
          >
        </div>
        <div class="gcp-row gcp-row-stack">
          <select
            :value="searchMode"
            class="gcp-select"
            data-testid="gcp-search-mode"
            @change="$emit('update:searchMode', ($event.target as HTMLSelectElement).value)"
          >
            <option value="contains">
              Contains
            </option>
            <option value="starts">
              Starts with
            </option>
            <option value="ends">
              Ends with
            </option>
          </select>
        </div>
        <div class="gcp-fields">
          <span class="gcp-fields-label">Search in:</span>
          <label class="gcp-field-chip">
            <input
              :checked="searchFields.label"
              type="checkbox"
              data-testid="gcp-field-label"
              @change="$emit('update:searchField', { field: 'label', value: ($event.target as HTMLInputElement).checked })"
            >
            <span>Label</span>
          </label>
          <label class="gcp-field-chip">
            <input
              :checked="searchFields.tags"
              type="checkbox"
              data-testid="gcp-field-tags"
              @change="$emit('update:searchField', { field: 'tags', value: ($event.target as HTMLInputElement).checked })"
            >
            <span>Tags</span>
          </label>
          <label class="gcp-field-chip">
            <input
              :checked="searchFields.body"
              type="checkbox"
              data-testid="gcp-field-body"
              @change="$emit('update:searchField', { field: 'body', value: ($event.target as HTMLInputElement).checked })"
            >
            <span>Body</span>
          </label>
          <label class="gcp-field-chip">
            <input
              :checked="searchFields.community"
              type="checkbox"
              data-testid="gcp-field-community"
              @change="$emit('update:searchField', { field: 'community', value: ($event.target as HTMLInputElement).checked })"
            >
            <span>Cluster</span>
          </label>
        </div>
        <label class="gcp-row">
          <input
            :checked="highlightFilterActive"
            type="checkbox"
            data-testid="gcp-highlight-toggle"
            @change="$emit('update:highlightFilterActive', ($event.target as HTMLInputElement).checked)"
          >
          <span>Highlight matches on canvas</span>
        </label>
        <div class="gcp-readout">
          <span><strong>{{ matchCount }}</strong> match{{ matchCount === 1 ? '' : 'es' }}</span>
          <span><strong>{{ selectedCount }}</strong> selected</span>
          <span><strong>{{ visibleNodeCount }}</strong> visible</span>
        </div>
        <div class="gcp-row gcp-row-stack gcp-row-buttons">
          <button
            type="button"
            class="gcp-btn gcp-btn-primary"
            :disabled="matchCount === 0"
            title="Replace selection with current matches (Enter)"
            data-testid="gcp-select-matches"
            @click="$emit('select-matches')"
          >
            ◉ Select matches
          </button>
          <div class="gcp-btn-pair">
            <button
              type="button"
              class="gcp-btn"
              :disabled="matchCount === 0"
              title="Add matches to selection (Shift+Enter)"
              data-testid="gcp-add-matches"
              @click="$emit('add-matches')"
            >
              ⊕ Add
            </button>
            <button
              type="button"
              class="gcp-btn"
              :disabled="matchCount === 0 || selectedCount === 0"
              title="Remove matches from selection (Alt+Enter)"
              data-testid="gcp-remove-matches"
              @click="$emit('remove-matches')"
            >
              ⊖ Remove
            </button>
          </div>
          <button
            type="button"
            class="gcp-btn"
            :disabled="visibleNodeCount === 0"
            data-testid="gcp-select-all-visible"
            @click="$emit('select-all-visible')"
          >
            ▦ Select all visible
          </button>
          <button
            type="button"
            class="gcp-btn"
            :disabled="selectedCount === 0"
            data-testid="gcp-clear-selection"
            @click="$emit('clear-selection')"
          >
            ✕ Clear selection
          </button>
        </div>
        <div class="gcp-hint">
          <strong>Enter</strong> selects matches · <strong>Shift+Enter</strong> adds ·
          <strong>Alt+Enter</strong> removes.
          <br>
          <strong>Shift+drag</strong> adds · <strong>Alt+Shift+drag</strong> removes.
          <br>
          <strong>Esc</strong> (in canvas) clears selection.
        </div>
      </details>

      <details
        v-if="legend.length > 0"
        class="gcp-section"
      >
        <summary>Groups</summary>
        <div class="gcp-legend">
          <div
            v-for="g in legend"
            :key="g.label"
            class="gcp-legend-item"
          >
            <span
              class="gcp-legend-dot"
              :style="{ background: g.color }"
            />
            <span class="gcp-legend-label">{{ g.label }}</span>
            <span class="gcp-legend-count">{{ g.count }}</span>
          </div>
        </div>
      </details>

      <details
        v-if="showDisplay"
        class="gcp-section"
      >
        <summary>Display</summary>
        <label class="gcp-row">
          <input
            :checked="showLabels"
            type="checkbox"
            @change="$emit('update:showLabels', ($event.target as HTMLInputElement).checked)"
          >
          <span>Always show labels</span>
        </label>
        <label class="gcp-row">
          <input
            :checked="showArrows"
            type="checkbox"
            @change="$emit('update:showArrows', ($event.target as HTMLInputElement).checked)"
          >
          <span>Show arrows</span>
        </label>
        <label class="gcp-row">
          <span>Text fade</span>
          <input
            :value="textFadeThreshold"
            type="range"
            min="0.4"
            max="3"
            step="0.05"
            @input="$emit('update:textFadeThreshold', Number(($event.target as HTMLInputElement).value))"
          >
        </label>
        <label class="gcp-row">
          <span>Node size</span>
          <input
            :value="nodeSizeMul"
            type="range"
            min="0.4"
            max="3"
            step="0.05"
            @input="$emit('update:nodeSizeMul', Number(($event.target as HTMLInputElement).value))"
          >
        </label>
        <label class="gcp-row">
          <span>Link thickness</span>
          <input
            :value="linkWidthMul"
            type="range"
            min="0.2"
            max="3"
            step="0.05"
            @input="$emit('update:linkWidthMul', Number(($event.target as HTMLInputElement).value))"
          >
        </label>
      </details>

      <details
        v-if="showForces"
        class="gcp-section"
      >
        <summary>Forces</summary>
        <label class="gcp-row">
          <span>Repulsion</span>
          <input
            :value="repulsion"
            type="range"
            min="-400"
            max="-20"
            step="5"
            @input="$emit('update:repulsion', Number(($event.target as HTMLInputElement).value))"
          >
        </label>
        <label class="gcp-row">
          <span>Link distance</span>
          <input
            :value="linkDistance"
            type="range"
            min="20"
            max="200"
            step="2"
            @input="$emit('update:linkDistance', Number(($event.target as HTMLInputElement).value))"
          >
        </label>
        <label class="gcp-row">
          <span>Gravity</span>
          <input
            :value="gravity"
            type="range"
            min="0"
            max="1"
            step="0.01"
            @input="$emit('update:gravity', Number(($event.target as HTMLInputElement).value))"
          >
        </label>
      </details>

      <slot name="extra" />
    </div>
  </div>
</template>

<script setup lang="ts">
/**
 * Shared left-side control panel for both the 2D `MemoryGraph` and 3D
 * `BrainGraphViewport`. The panel renders identical sections in both modes;
 * the host viewport decides which sections to enable via the `show*` flags
 * and which sliders to actually act on.
 *
 * State is owned by the host (v-model bindings) so the panel can stay a
 * pure presentational component.
 */
interface LegendItem {
  label: string;
  color: string;
  count: number;
}

type SearchFieldKey = 'label' | 'tags' | 'body' | 'community';

interface SearchFieldsState {
  label: boolean;
  tags: boolean;
  body: boolean;
  community: boolean;
}

defineProps<{
  title?: string;
  collapsed: boolean;
  // ── Views ───────────────────────────────────────
  nodeCount: number;
  edgeCount: number;
  showViews?: boolean;
  // ── Filters ─────────────────────────────────────
  showFilters?: boolean;
  showOrphans: boolean;
  minDegree: number;
  // ── Search & Select ─────────────────────────────
  searchText: string;
  searchMode: string;
  searchFields: SearchFieldsState;
  highlightFilterActive: boolean;
  matchCount: number;
  selectedCount: number;
  visibleNodeCount: number;
  // ── Groups ──────────────────────────────────────
  legend: readonly LegendItem[];
  // ── Display ─────────────────────────────────────
  showDisplay?: boolean;
  showLabels: boolean;
  showArrows: boolean;
  textFadeThreshold: number;
  nodeSizeMul: number;
  linkWidthMul: number;
  // ── Forces ──────────────────────────────────────
  showForces?: boolean;
  repulsion: number;
  linkDistance: number;
  gravity: number;
}>();

defineEmits<{
  (e: 'update:collapsed', v: boolean): void;
  (e: 'update:showOrphans', v: boolean): void;
  (e: 'update:minDegree', v: number): void;
  (e: 'update:searchText', v: string): void;
  (e: 'update:searchMode', v: string): void;
  (e: 'update:searchField', v: { field: SearchFieldKey; value: boolean }): void;
  (e: 'update:highlightFilterActive', v: boolean): void;
  (e: 'update:showLabels', v: boolean): void;
  (e: 'update:showArrows', v: boolean): void;
  (e: 'update:textFadeThreshold', v: number): void;
  (e: 'update:nodeSizeMul', v: number): void;
  (e: 'update:linkWidthMul', v: number): void;
  (e: 'update:repulsion', v: number): void;
  (e: 'update:linkDistance', v: number): void;
  (e: 'update:gravity', v: number): void;
  (e: 'select-matches'): void;
  (e: 'add-matches'): void;
  (e: 'remove-matches'): void;
  (e: 'select-all-visible'): void;
  (e: 'clear-selection'): void;
}>();
</script>

<style scoped>
.gcp-panel {
  position: absolute;
  top: 0.7rem;
  left: 0.7rem;
  z-index: 5;
  width: min(280px, calc(100% - 1.4rem));
  max-height: calc(100% - 1.4rem);
  display: flex;
  flex-direction: column;
  background: color-mix(in srgb, var(--ts-bg-base, #0b1426) 88%, transparent);
  border: 1px solid color-mix(in srgb, var(--ts-text-muted, #475569) 26%, transparent);
  border-radius: 6px;
  color: var(--ts-text-primary, #e0f0ff);
  box-shadow: var(--ts-shadow-lg, 0 12px 28px rgba(0, 0, 0, 0.45));
  backdrop-filter: blur(16px);
  overflow: hidden;
  font-size: 0.78rem;
}

.gcp-collapsed {
  width: auto;
}

.gcp-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.45rem 0.55rem;
  border-bottom: 1px solid color-mix(in srgb, var(--ts-text-muted, #475569) 18%, transparent);
}

.gcp-collapsed .gcp-head {
  border-bottom: 0;
}

.gcp-title {
  font-weight: 600;
}

.gcp-icon-button {
  border: 0;
  background: transparent;
  color: var(--ts-text-secondary, #cbd5e1);
  cursor: pointer;
  font-size: 0.9rem;
  padding: 0.1rem 0.4rem;
  border-radius: 4px;
}

.gcp-icon-button:hover {
  background: color-mix(in srgb, var(--ts-text-muted, #475569) 18%, transparent);
}

.gcp-body {
  overflow-y: auto;
  padding: 0.25rem 0.55rem 0.55rem;
}

.gcp-section {
  border-top: 1px solid color-mix(in srgb, var(--ts-text-muted, #475569) 12%, transparent);
  padding: 0.25rem 0;
}

.gcp-section:first-of-type {
  border-top: 0;
}

.gcp-section summary {
  cursor: pointer;
  padding: 0.25rem 0;
  font-weight: 600;
  font-size: 0.74rem;
  color: var(--ts-text-secondary, #cbd5e1);
  list-style: none;
}

.gcp-section summary::-webkit-details-marker {
  display: none;
}

.gcp-row {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.18rem 0;
  font-size: 0.72rem;
}

.gcp-row > span:first-child {
  flex: 1;
  min-width: 0;
}

.gcp-row-stack {
  flex-direction: column;
  align-items: stretch;
  gap: 0.25rem;
}

.gcp-row-buttons {
  margin-top: 0.25rem;
}

.gcp-input,
.gcp-select {
  width: 100%;
  padding: 0.3rem 0.4rem;
  font-size: 0.74rem;
  background: color-mix(in srgb, var(--ts-bg-surface, #131a26) 80%, transparent);
  color: var(--ts-text-primary, #e0f0ff);
  border: 1px solid color-mix(in srgb, var(--ts-text-muted, #475569) 26%, transparent);
  border-radius: 4px;
  font-family: inherit;
}

.gcp-input:focus,
.gcp-select:focus {
  outline: 0;
  border-color: color-mix(in srgb, var(--ts-accent, #7c6fff) 55%, transparent);
}

.gcp-readout {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.2rem 0;
  font-size: 0.7rem;
  color: var(--ts-text-muted, #94a3b8);
  font-variant-numeric: tabular-nums;
}

.gcp-btn {
  padding: 0.36rem 0.5rem;
  font-size: 0.72rem;
  background: color-mix(in srgb, var(--ts-bg-surface, #131a26) 80%, transparent);
  color: var(--ts-text-primary, #e0f0ff);
  border: 1px solid color-mix(in srgb, var(--ts-text-muted, #475569) 26%, transparent);
  border-radius: 4px;
  cursor: pointer;
  font-family: inherit;
  text-align: left;
}

.gcp-btn:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--ts-accent, #7c6fff) 55%, transparent);
}

.gcp-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.gcp-btn-primary {
  background: color-mix(in srgb, var(--ts-accent, #7c6fff) 22%, transparent);
  border-color: color-mix(in srgb, var(--ts-accent, #7c6fff) 55%, transparent);
}

.gcp-btn-primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--ts-accent, #7c6fff) 35%, transparent);
}

.gcp-btn-pair {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.3rem;
}

.gcp-fields {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.3rem;
  padding: 0.2rem 0;
}

.gcp-fields-label {
  font-size: 0.66rem;
  color: var(--ts-text-muted, #94a3b8);
  margin-right: 0.15rem;
}

.gcp-field-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.12rem 0.4rem;
  border: 1px solid color-mix(in srgb, var(--ts-text-muted, #475569) 26%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, var(--ts-bg-surface, #131a26) 60%, transparent);
  font-size: 0.66rem;
  cursor: pointer;
}

.gcp-field-chip input {
  width: 11px;
  height: 11px;
  margin: 0;
}

.gcp-hint {
  margin-top: 0.35rem;
  font-size: 0.66rem;
  line-height: 1.35;
  color: var(--ts-text-muted, #94a3b8);
}

.gcp-val {
  min-width: 1.6rem;
  text-align: right;
  font-variant-numeric: tabular-nums;
  color: var(--ts-text-muted, #94a3b8);
}

.gcp-legend {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  padding: 0.15rem 0;
}

.gcp-legend-item {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.7rem;
}

.gcp-legend-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  flex-shrink: 0;
}

.gcp-legend-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.gcp-legend-count {
  color: var(--ts-text-muted, #94a3b8);
  font-variant-numeric: tabular-nums;
}

input[type="range"] {
  flex: 1;
  min-width: 60px;
}

input[type="checkbox"] {
  accent-color: var(--ts-accent, #7c6fff);
}
</style>
