<template>
  <aside
    class="snp-panel"
    data-testid="selected-nodes-panel"
    @pointerdown.stop
    @pointermove.stop
    @pointerup.stop
  >
    <header class="snp-head">
      <strong>Selected nodes</strong>
      <span class="snp-count">{{ items.length }}</span>
    </header>

    <div
      v-if="items.length === 0"
      class="snp-empty"
    >
      No nodes selected. Use the search panel or
      <kbd>Shift</kbd>+drag to box-select.
    </div>

    <ul
      v-else
      class="snp-list"
    >
      <li
        v-for="item in items"
        :key="item.id"
        class="snp-item"
        :data-testid="`snp-item-${item.id}`"
      >
        <button
          type="button"
          class="snp-untick"
          :title="checked(item.id) ? 'Untick (remove). Shift+click to range-toggle from last click.' : 'Retick (add back). Shift+click for range.'"
          :aria-pressed="checked(item.id)"
          @click="(e) => onRowToggle(item.id, e)"
        >
          <span
            class="snp-check"
            :class="{ on: checked(item.id) }"
          >{{ checked(item.id) ? '☑' : '☐' }}</span>
        </button>
        <span
          class="snp-dot"
          :style="{ background: item.colour ?? 'var(--ts-text-muted)' }"
        />
        <button
          type="button"
          class="snp-label"
          :title="item.full ?? item.label"
          @click="$emit('focus', item.id)"
        >
          <span class="snp-label-text">{{ item.label }}</span>
          <span
            v-if="item.community"
            class="snp-meta"
          >{{ item.community }}</span>
        </button>
      </li>
    </ul>

    <footer class="snp-actions">
      <button
        type="button"
        class="snp-btn snp-btn-danger"
        :disabled="items.length === 0"
        title="Delete every memory NOT in this selection. This cannot be undone."
        data-testid="snp-keep-only"
        @click="$emit('keep-only', items.map((i) => i.id))"
      >
        ☠ Keep only
      </button>
      <button
        type="button"
        class="snp-btn"
        :disabled="items.length === 0"
        data-testid="snp-clear-all"
        @click="$emit('clear')"
      >
        ✕ Clear all
      </button>
    </footer>
  </aside>
</template>

<script setup lang="ts">
/**
 * Shared right-side panel that lists every node in the persistent selection
 * built by the graph viewports (2D `MemoryGraph` and 3D `BrainGraphViewport`).
 *
 * Each row is a checkbox + colour dot + label. Unticking removes a node from
 * the selection; the parent re-renders and the row visually flips to "off"
 * (so users can re-tick to add it back). The footer exposes the two
 * shortcuts the user asked for: Confirm (emits the current id list) and
 * Clear all (drops the entire selection).
 *
 * Both viewports use the same component so the UX is identical between 2D
 * and 3D modes.
 */
import { computed, ref } from 'vue';

interface SelectedItem {
  id: number;
  label: string;
  full?: string;
  community?: string;
  colour?: string;
}

const props = defineProps<{
  /** Current persistent selection. */
  selectedIds: ReadonlySet<number>;
  /** Lookup of *all* graph nodes the panel might display. */
  nodes: readonly SelectedItem[];
}>();

const emit = defineEmits<{
  (e: 'toggle', id: number): void;
  (e: 'range-toggle', ids: number[]): void;
  (e: 'clear'): void;
  (e: 'keep-only', ids: number[]): void;
  (e: 'focus', id: number): void;
}>();

// Render rows for every node currently in the selection, preserving the
// graph's natural ordering so the list stays stable between renders.
const items = computed<SelectedItem[]>(() => {
  if (props.selectedIds.size === 0) return [];
  const out: SelectedItem[] = [];
  for (const node of props.nodes) {
    if (props.selectedIds.has(node.id)) out.push(node);
  }
  return out;
});

const checked = (id: number) => props.selectedIds.has(id);

// Range-select state: most-recently-clicked row becomes the anchor; a
// Shift+click on another row emits a range-toggle for every row between the
// two (inclusive), in the panel's current display order.
const anchorId = ref<number | null>(null);

function onRowToggle(id: number, event: MouseEvent) {
  if (event.shiftKey && anchorId.value !== null && anchorId.value !== id) {
    const list = items.value.map((i) => i.id);
    const a = list.indexOf(anchorId.value);
    const b = list.indexOf(id);
    if (a >= 0 && b >= 0) {
      const [lo, hi] = a < b ? [a, b] : [b, a];
      emit('range-toggle', list.slice(lo, hi + 1));
      anchorId.value = id;
      return;
    }
  }
  anchorId.value = id;
  emit('toggle', id);
}
</script>

<style scoped>
.snp-panel {
  position: absolute;
  top: 0.7rem;
  right: 0.7rem;
  z-index: 5;
  width: min(260px, calc(100% - 1.4rem));
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
}

.snp-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.55rem 0.7rem;
  border-bottom: 1px solid color-mix(in srgb, var(--ts-text-muted, #475569) 18%, transparent);
  font-size: 0.78rem;
}

.snp-count {
  padding: 0.06rem 0.45rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--ts-accent, #7c6fff) 25%, transparent);
  color: var(--ts-text-primary, #e0f0ff);
  font-size: 0.72rem;
  font-variant-numeric: tabular-nums;
}

.snp-empty {
  padding: 0.7rem;
  font-size: 0.72rem;
  line-height: 1.4;
  color: var(--ts-text-muted, #94a3b8);
}

.snp-empty kbd {
  padding: 0.05rem 0.3rem;
  border-radius: 3px;
  background: color-mix(in srgb, var(--ts-text-muted, #475569) 18%, transparent);
  font-family: inherit;
  font-size: 0.68rem;
}

.snp-list {
  list-style: none;
  margin: 0;
  padding: 0.25rem;
  overflow-y: auto;
  flex: 1;
}

.snp-item {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.25rem 0.35rem;
  border-radius: 4px;
}

.snp-item:hover {
  background: color-mix(in srgb, var(--ts-text-muted, #475569) 12%, transparent);
}

.snp-untick {
  border: 0;
  background: transparent;
  color: var(--ts-text-secondary, #cbd5e1);
  cursor: pointer;
  font-size: 0.95rem;
  line-height: 1;
  padding: 0;
  display: grid;
  place-items: center;
  width: 18px;
  height: 18px;
}

.snp-check.on {
  color: var(--ts-accent, #7c6fff);
}

.snp-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  flex-shrink: 0;
}

.snp-label {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.05rem;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
  padding: 0;
  font-family: inherit;
  text-align: left;
}

.snp-label-text {
  font-size: 0.74rem;
  line-height: 1.25;
  color: var(--ts-text-primary, #e0f0ff);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.snp-meta {
  font-size: 0.64rem;
  color: var(--ts-text-muted, #94a3b8);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.snp-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.3rem;
  padding: 0.5rem 0.55rem;
  border-top: 1px solid color-mix(in srgb, var(--ts-text-muted, #475569) 18%, transparent);
  background: color-mix(in srgb, var(--ts-bg-surface, #131a26) 60%, transparent);
}

.snp-btn {
  padding: 0.36rem 0.5rem;
  font-size: 0.74rem;
  background: color-mix(in srgb, var(--ts-bg-surface, #131a26) 80%, transparent);
  color: var(--ts-text-primary, #e0f0ff);
  border: 1px solid color-mix(in srgb, var(--ts-text-muted, #475569) 26%, transparent);
  border-radius: 4px;
  cursor: pointer;
  font-family: inherit;
}

.snp-btn:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--ts-accent, #7c6fff) 60%, transparent);
}

.snp-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.snp-btn-danger {
  background: color-mix(in srgb, var(--ts-danger, #ef4444) 20%, transparent);
  border-color: color-mix(in srgb, var(--ts-danger, #ef4444) 55%, transparent);
  color: var(--ts-text-primary, #fff);
}

.snp-btn-danger:hover:not(:disabled) {
  background: color-mix(in srgb, var(--ts-danger, #ef4444) 38%, transparent);
}
</style>
