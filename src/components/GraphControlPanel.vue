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
  top: 4.8rem;
  left: 1.2rem;
  z-index: 5;
  width: min(260px, calc(100% - 2.4rem));
  max-height: calc(100% - 6rem);
  display: flex;
  flex-direction: column;
  background: var(--ts-galaxy-hud-bg, rgba(20, 18, 28, 0.62));
  border: 1px solid var(--ts-galaxy-hud-border, rgba(255, 255, 255, 0.08));
  border-radius: 14px;
  color: var(--ts-galaxy-hud-fg, rgba(255, 255, 255, 0.92));
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
  overflow: hidden;
  font-size: 0.76rem;
  font-family: var(--ts-font-family, Inter, system-ui, sans-serif);
}

.gcp-collapsed {
  width: auto;
}

.gcp-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.5rem 0.7rem;
  border-bottom: 1px solid var(--ts-galaxy-hud-border, rgba(255, 255, 255, 0.08));
}

.gcp-collapsed .gcp-head {
  border-bottom: 0;
}

.gcp-title {
  font-weight: 600;
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--ts-galaxy-hud-fg-muted, rgba(255, 255, 255, 0.72));
}

.gcp-icon-button {
  border: 0;
  background: transparent;
  color: var(--ts-galaxy-hud-fg-muted, rgba(255, 255, 255, 0.72));
  cursor: pointer;
  font-size: 0.85rem;
  padding: 0.15rem 0.4rem;
  border-radius: 6px;
  transition: background 140ms var(--ts-galaxy-ease, ease);
}

.gcp-icon-button:hover {
  background: rgba(255, 255, 255, 0.08);
}

.gcp-body {
  overflow-y: auto;
  padding: 0.3rem 0.7rem 0.7rem;
}

.gcp-section {
  border-top: 1px solid var(--ts-galaxy-hud-border, rgba(255, 255, 255, 0.08));
  padding: 0.3rem 0;
}

.gcp-section:first-of-type {
  border-top: 0;
}

.gcp-section summary {
  cursor: pointer;
  padding: 0.25rem 0;
  font-weight: 600;
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--ts-galaxy-hud-fg-muted, rgba(255, 255, 255, 0.72));
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
  padding: 0.35rem 0.5rem;
  font-size: 0.72rem;
  background: rgba(10, 8, 20, 0.55);
  color: var(--ts-galaxy-hud-fg, rgba(255, 255, 255, 0.92));
  border: 1px solid var(--ts-galaxy-hud-border, rgba(255, 255, 255, 0.08));
  border-radius: 8px;
  font-family: inherit;
  transition: border-color 140ms var(--ts-galaxy-ease, ease);
}

.gcp-input:focus,
.gcp-select:focus {
  outline: 0;
  border-color: var(--ts-galaxy-violet, #b9ace8);
}

.gcp-readout {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.2rem 0;
  font-size: 0.68rem;
  color: var(--ts-galaxy-hud-fg-dim, rgba(255, 255, 255, 0.48));
  font-variant-numeric: tabular-nums;
}

.gcp-btn {
  padding: 0.36rem 0.55rem;
  font-size: 0.7rem;
  background: rgba(10, 8, 20, 0.45);
  color: var(--ts-galaxy-hud-fg, rgba(255, 255, 255, 0.92));
  border: 1px solid var(--ts-galaxy-hud-border, rgba(255, 255, 255, 0.08));
  border-radius: 8px;
  cursor: pointer;
  font-family: inherit;
  text-align: left;
  transition: border-color 140ms var(--ts-galaxy-ease, ease), background 140ms var(--ts-galaxy-ease, ease);
}

.gcp-btn:hover:not(:disabled) {
  border-color: var(--ts-galaxy-violet, #b9ace8);
  background: rgba(185, 172, 232, 0.08);
}

.gcp-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.gcp-btn-primary {
  background: rgba(185, 172, 232, 0.12);
  border-color: rgba(185, 172, 232, 0.3);
}

.gcp-btn-primary:hover:not(:disabled) {
  background: rgba(185, 172, 232, 0.2);
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
  font-size: 0.64rem;
  color: var(--ts-galaxy-hud-fg-dim, rgba(255, 255, 255, 0.48));
  margin-right: 0.15rem;
}

.gcp-field-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.14rem 0.45rem;
  border: 1px solid var(--ts-galaxy-hud-border, rgba(255, 255, 255, 0.08));
  border-radius: 999px;
  background: rgba(10, 8, 20, 0.4);
  font-size: 0.64rem;
  cursor: pointer;
  transition: border-color 140ms var(--ts-galaxy-ease, ease);
}

.gcp-field-chip:hover {
  border-color: rgba(255, 255, 255, 0.18);
}

.gcp-field-chip input {
  width: 11px;
  height: 11px;
  margin: 0;
}

.gcp-hint {
  margin-top: 0.35rem;
  font-size: 0.64rem;
  line-height: 1.4;
  color: var(--ts-galaxy-hud-fg-dim, rgba(255, 255, 255, 0.48));
}

.gcp-val {
  min-width: 1.6rem;
  text-align: right;
  font-variant-numeric: tabular-nums;
  color: var(--ts-galaxy-hud-fg-dim, rgba(255, 255, 255, 0.48));
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
  color: var(--ts-galaxy-hud-fg-dim, rgba(255, 255, 255, 0.48));
  font-variant-numeric: tabular-nums;
}

input[type="range"] {
  flex: 1;
  min-width: 60px;
}

input[type="checkbox"] {
  accent-color: var(--ts-galaxy-violet, #b9ace8);
}
</style>
