<template>
  <div
    class="memory-graph-shell"
    :class="{ 'mode-3d': mode === '3d', 'memory-graph-shell--fullscreen': fullscreen }"
    data-testid="memory-graph"
  >
    <div
      class="mg-mode-toggle"
      role="tablist"
      aria-label="Graph renderer"
    >
      <button
        type="button"
        :class="['mg-mode-btn', { active: mode === '3d' }]"
        data-testid="mg-mode-3d"
        @click="mode = '3d'"
      >
        3D
      </button>
      <button
        type="button"
        :class="['mg-mode-btn', { active: mode === '2d' }]"
        data-testid="mg-mode-2d"
        @click="mode = '2d'"
      >
        Lite 2D
      </button>
    </div>

    <div
      class="mg-topbar"
      @pointerdown.stop
      @pointermove.stop
      @pointerup.stop
    >
      <div class="mg-title">
        <span
          class="mg-title-icon"
          aria-hidden="true"
        >⌘</span>
        <span>Graph view</span>
      </div>
      <div class="mg-top-actions">
        <button
          type="button"
          class="mg-icon-button"
          title="Fit graph"
          aria-label="Fit graph"
          @click="onFitClick"
        >
          ⌖
        </button>
        <button
          type="button"
          class="mg-icon-button"
          :title="fullscreen ? 'Exit fullscreen (Esc)' : 'Fullscreen'"
          :aria-label="fullscreen ? 'Exit fullscreen' : 'Enter fullscreen'"
          data-testid="mg-fullscreen"
          @click="toggleFullscreen"
        >
          {{ fullscreen ? '⊠' : '⛶' }}
        </button>
        <button
          type="button"
          class="mg-icon-button"
          title="Toggle panel"
          aria-label="Toggle graph controls"
          @click="panelCollapsed = !panelCollapsed"
        >
          ⚙
        </button>
      </div>
    </div>

    <MemoryGalaxy
      v-if="mode === '3d'"
      :memories="memories"
      :edges="edges"
      :edge-mode="edgeMode"
      :selected-ids="selectedIds"
      :search-text="searchText"
      :search-mode="searchMode"
      :search-fields="searchFields"
      :highlight-filter-active="highlightFilterActive"
      :show-orphans="showOrphans"
      :min-degree="minDegree"
      :show-labels="showLabels"
      :show-arrows="showArrows"
      :node-size-mul="nodeSizeMul"
      :link-width-mul="linkWidthMul"
      :fit-trigger="fitTrigger"
      @select="on3DSelect"
      @toggle-selected="toggleSelectedId"
      @keep-only-selection="(ids) => emit('keep-only-selection', ids)"
    />

    <div
      v-else
      ref="container"
      class="memory-graph"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointerleave="onPointerLeave"
      @wheel.prevent="onWheel"
      @dblclick="onDoubleClick"
    >
      <canvas
        ref="canvasEl"
        class="mg-canvas"
      />
    </div>

    <GraphControlPanel
      title="Graph controls"
      :collapsed="panelCollapsed"
      :node-count="nodeCount"
      :edge-count="edgeCount"
      :show-views="true"
      :show-filters="true"
      :show-orphans="showOrphans"
      :min-degree="minDegree"
      :search-text="searchText"
      :search-mode="searchMode"
      :search-fields="searchFields"
      :highlight-filter-active="highlightFilterActive"
      :match-count="effectiveMatchCount"
      :selected-count="selectedCount"
      :visible-node-count="effectiveVisibleCount"
      :legend="legend"
      :show-display="true"
      :show-labels="showLabels"
      :show-arrows="showArrows"
      :text-fade-threshold="textFadeThreshold"
      :node-size-mul="nodeSizeMul"
      :link-width-mul="linkWidthMul"
      :show-forces="mode === '2d'"
      :repulsion="repulsion"
      :link-distance="linkDistance"
      :gravity="gravity"
      @update:collapsed="panelCollapsed = $event"
      @update:show-orphans="showOrphans = $event"
      @update:min-degree="minDegree = $event"
      @update:search-text="searchText = $event"
      @update:search-mode="searchMode = $event as 'contains' | 'starts' | 'ends'"
      @update:search-field="(p) => (searchFields[p.field] = p.value)"
      @update:highlight-filter-active="highlightFilterActive = $event"
      @update:show-labels="showLabels = $event"
      @update:show-arrows="showArrows = $event"
      @update:text-fade-threshold="textFadeThreshold = $event"
      @update:node-size-mul="nodeSizeMul = $event"
      @update:link-width-mul="linkWidthMul = $event"
      @update:repulsion="repulsion = $event"
      @update:link-distance="linkDistance = $event"
      @update:gravity="gravity = $event"
      @select-matches="selectMatches"
      @add-matches="addMatches"
      @remove-matches="removeMatches"
      @select-all-visible="selectAllVisible"
      @clear-selection="clearSelection"
    />

    <div
      v-if="hoverLabel && mode === '2d'"
      class="mg-hover-card"
    >
      <span
        class="mg-hover"
      >{{ hoverLabel }}</span>
    </div>

    <div
      v-if="isBuilding && mode === '2d'"
      class="mg-loading"
    >
      Building graph…
    </div>

    <SelectedNodesPanel
      v-if="selectedIds.size > 0"
      :selected-ids="selectedIds"
      :nodes="selectionPanelNodes"
      @toggle="toggleSelectedId"
      @range-toggle="onRangeToggle"
      @clear="clearSelection"
      @keep-only="onKeepOnly"
      @focus="(id) => emit('select', id)"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted, onBeforeUnmount, watch, nextTick } from 'vue';
// d3-force-3d ships no TS types and the ambient resolution picks up an
// incomplete shape. Import as `any` so we can use the full runtime API
// (forceCollide / forceX / forceY / distanceMax) without fighting types.
 
import * as d3force from 'd3-force-3d';
const forceSimulation = (d3force as any).forceSimulation;
const forceManyBody = (d3force as any).forceManyBody;
const forceLink = (d3force as any).forceLink;
const forceCenter = (d3force as any).forceCenter;
const forceCollide = (d3force as any).forceCollide;
const forceX = (d3force as any).forceX;
const forceY = (d3force as any).forceY;
import type { MemoryEdge, MemoryEntry } from '../types';

const props = withDefaults(
  defineProps<{
    memories: MemoryEntry[];
    edges?: MemoryEdge[];
    edgeMode?: 'typed' | 'tag' | 'both';
  }>(),
  { edges: () => [], edgeMode: 'typed' },
);
const emit = defineEmits<{
  (e: 'select', id: number): void;
  (e: 'select-edge', id: number): void;
  (e: 'keep-only-selection', ids: number[]): void;
}>();

import SelectedNodesPanel from './SelectedNodesPanel.vue';
import GraphControlPanel from './GraphControlPanel.vue';
import MemoryGalaxy from './MemoryGalaxy.vue';

// ── Render mode (2D Canvas / 3D WebGL) ──────────────────────────────────────
// The 3D path delegates to `MemoryGalaxy.vue`.
// User preference is persisted across sessions; default is 3D. We bumped the
// storage key once we replaced the old viewport with the Knowledge-Galaxy
// renderer so any legacy `'2d'` preference no longer pins users to the
// lite Canvas path on first launch after upgrade.
const MODE_KEY = 'memory-graph-mode-v2';
const LEGACY_MODE_KEY = 'memory-graph-mode';
function loadMode(): '2d' | '3d' {
  try {
    // One-time migration: drop the legacy key so an old `'2d'` value can't
    // override the new 3D-by-default behaviour.
    if (localStorage.getItem(LEGACY_MODE_KEY) !== null) {
      localStorage.removeItem(LEGACY_MODE_KEY);
    }
    const v = localStorage.getItem(MODE_KEY);
    return v === '2d' ? '2d' : '3d';
  } catch {
    return '3d';
  }
}
const mode = ref<'2d' | '3d'>(loadMode());
watch(mode, (m) => {
  try {
    localStorage.setItem(MODE_KEY, m);
  } catch {
    /* ignore quota / disabled storage */
  }
  // When switching back to 2D, the canvas DOM is recreated by v-if/v-else.
  // Re-initialize the 2D rendering pipeline after Vue inserts the new DOM.
  if (m === '2d') {
    nextTick(() => {
      resizeObserver?.disconnect();
      resizeCanvas();
      resizeObserver = new ResizeObserver(() => resizeCanvas());
      if (container.value) resizeObserver.observe(container.value);
      rebuildData();
      startSim();
      setTimeout(fitToView, 400);
    });
  }
});

// ── Graph data model ────────────────────────────────────────────────────────
interface GNode {
  id: number;
  label: string;
  full: string;
  memoryType: string;
  importance: number;
  groupKey: string;
  tagsCsv: string;
  degree: number;
  /** BRAIN-REPO-RAG-2a: optional source provenance for repo-sourced chunks. */
  sourceId?: string;
  sourceLabel?: string;
  filePath?: string;
  parentSymbol?: string;
  // d3-force mutates these in place
  x?: number;
  y?: number;
  vx?: number;
  vy?: number;
  fx?: number | null;
  fy?: number | null;
}
interface GLink {
  source: number | GNode;
  target: number | GNode;
  kind: 'typed' | 'tag';
  rel?: string;
  edgeId?: number;
  color: string;
  weight: number;
}

const container = ref<HTMLDivElement | null>(null);
const canvasEl = ref<HTMLCanvasElement | null>(null);
let ctx: CanvasRenderingContext2D | null = null;
let dpr = 1;
let viewW = 0;
let viewH = 0;
let raf = 0;
let resizeObserver: ResizeObserver | null = null;

const nodes = ref<GNode[]>([]);
const links = ref<GLink[]>([]);
 
let sim: any = null;

const nodeCount = computed(() => nodes.value.length);
const edgeCount = computed(() => links.value.length);

// ── Camera ──────────────────────────────────────────────────────────────────
const zoom = ref(1);
const camX = ref(0);
const camY = ref(0);
let panActive = false;
let panLastX = 0;
let panLastY = 0;

// ── Interaction state ───────────────────────────────────────────────────────
const hoverId = ref<number | null>(null);
const draggingNode = ref<GNode | null>(null);
let pointerDownX = 0;
let pointerDownY = 0;
let pointerMoved = false;

// ── Settings ────────────────────────────────────────────────────────────────
const panelCollapsed = ref(false);
const showOrphans = ref(true);
const minDegree = ref(0);
const showLabels = ref(false);
const showArrows = ref(false);
const textFadeThreshold = ref(2.1);
const nodeSizeMul = ref(0.85);
const linkWidthMul = ref(0.8);
const repulsion = ref(-70);
const linkDistance = ref(38);
const gravity = ref(0.08);

const isBuilding = ref(false);

// ── Search & persistent selection ───────────────────────────────────────────
// `searchText` + `searchMode` + `searchFields` drive a live filter. When
// `highlightFilterActive` is true the matches glow in real time; when false
// the filter is ignored but the persistent selection (built by Enter / "Add
// matches" / Shift-drag) stays highlighted.
type SearchMode = 'contains' | 'starts' | 'ends';
const searchText = ref('');
const searchMode = ref<SearchMode>('contains');
const searchFields = reactive({
  label: true,
  tags: true,
  body: false,
  community: true,
});
const highlightFilterActive = ref(true);
const selectedIds = ref<Set<number>>(new Set());

function nodeMatchesQuery(n: GNode, q: string, mode: SearchMode): boolean {
  if (!q) return false;
  const needle = q.toLowerCase();
  const candidates: string[] = [];
  if (searchFields.label && n.label) candidates.push(n.label.toLowerCase());
  if (searchFields.tags && n.tagsCsv) candidates.push(n.tagsCsv.toLowerCase());
  if (searchFields.body && n.full) candidates.push(n.full.toLowerCase());
  if (searchFields.community && n.groupKey) candidates.push(n.groupKey.toLowerCase());
  if (candidates.length === 0) return false;
  if (mode === 'contains') return candidates.some((c) => c.includes(needle));
  if (mode === 'starts') return candidates.some((c) => c.startsWith(needle));
  return candidates.some((c) => c.endsWith(needle));
}

const matchedIds = computed<Set<number>>(() => {
  const q = searchText.value.trim();
  if (!q) return new Set();
  const out = new Set<number>();
  for (const n of nodes.value) {
    if (nodeMatchesQuery(n, q, searchMode.value)) out.add(n.id);
  }
  return out;
});

const matchCount = computed(() => matchedIds.value.size);
const selectedCount = computed(() => selectedIds.value.size);
const visibleNodeCount = computed(() => filteredNodes().length);

// ── Memory-level search (mirrors `nodeMatchesQuery` for the 3D galaxy) ─────
// In 3D mode there are no GNodes, so we match directly on `MemoryEntry` so
// that the same Search box / filter toggles drive both renderers.
function memoryMatchesQuery(m: MemoryEntry, q: string, mode: SearchMode): boolean {
  if (!q) return false;
  const needle = q.toLowerCase();
  const candidates: string[] = [];
  const label = (m.content.split(/\r?\n/)[0] ?? '').toLowerCase();
  if (searchFields.label && label) candidates.push(label);
  if (searchFields.tags && m.tags) candidates.push(m.tags.toLowerCase());
  if (searchFields.body && m.content) candidates.push(m.content.toLowerCase());
  if (searchFields.community && m.memory_type) candidates.push(m.memory_type.toLowerCase());
  if (candidates.length === 0) return false;
  if (mode === 'contains') return candidates.some((c) => c.includes(needle));
  if (mode === 'starts') return candidates.some((c) => c.startsWith(needle));
  return candidates.some((c) => c.endsWith(needle));
}

const matchedMemoryIds = computed<Set<number>>(() => {
  const q = searchText.value.trim();
  if (!q) return new Set();
  const out = new Set<number>();
  for (const m of props.memories) {
    if (memoryMatchesQuery(m, q, searchMode.value)) out.add(m.id);
  }
  return out;
});

// In 3D mode the canvas GNode list is empty, so report counts derived from
// the underlying memory list instead so the control-panel chips stay
// meaningful when the user is in galaxy view.
const effectiveMatchCount = computed(() =>
  mode.value === '3d' ? matchedMemoryIds.value.size : matchCount.value,
);
const effectiveVisibleCount = computed(() => {
  if (mode.value === '2d') return visibleNodeCount.value;
  // For 3D, "visible" means memories that pass the orphan/min-degree gates.
  // Degrees in 3D come from `edges` referencing memory ids.
  const deg = new Map<number, number>();
  for (const e of (props.edges ?? [])) {
    deg.set(e.src_id, (deg.get(e.src_id) ?? 0) + 1);
    deg.set(e.dst_id, (deg.get(e.dst_id) ?? 0) + 1);
  }
  let n = 0;
  for (const m of props.memories) {
    const d = deg.get(m.id) ?? 0;
    if (!showOrphans.value && d === 0) continue;
    if (d < minDegree.value) continue;
    n++;
  }
  return n;
});

// Fit-to-view bridge — the toolbar button toggles `fitTrigger`, MemoryGalaxy
// watches it and resets its camera. In 2D we just call `fitToView()` directly.
const fitTrigger = ref(0);
function onFitClick(): void {
  if (mode.value === '2d') fitToView();
  else fitTrigger.value++;
}

// Fullscreen toggle — works for both 2D canvas and 3D galaxy. Pins the shell
// to viewport via fixed positioning; ESC exits. Both renderers re-render when
// their parent box resizes (canvas RAF + Three.js renderer.setSize watcher).
const fullscreen = ref(false);
function toggleFullscreen(): void {
  fullscreen.value = !fullscreen.value;
}
function onFullscreenKey(e: KeyboardEvent): void {
  if (e.key === 'Escape' && fullscreen.value) {
    fullscreen.value = false;
  }
}
onMounted(() => window.addEventListener('keydown', onFullscreenKey));
onBeforeUnmount(() => window.removeEventListener('keydown', onFullscreenKey));

// 3D click → select. Shift-click toggles the persistent selection just like
// the 2D canvas does, otherwise it focuses/replaces selection with that id.
function on3DSelect(id: number, shift: boolean): void {
  if (shift) {
    toggleSelectedId(id);
  } else {
    emit('select', id);
  }
}

/** Nodes that should appear "lit" in the renderer. Persistent selection
 *  always counts; live search matches only count while the highlight toggle
 *  is on. */
const highlightedIds = computed<Set<number>>(() => {
  const out = new Set<number>(selectedIds.value);
  if (highlightFilterActive.value) {
    for (const id of matchedIds.value) out.add(id);
  }
  return out;
});

function selectMatches(): void {
  // Replace selection with current matches.
  selectedIds.value = new Set(matchedIds.value);
  requestRender();
}

function addMatches(): void {
  if (matchedIds.value.size === 0) return;
  const next = new Set(selectedIds.value);
  for (const id of matchedIds.value) next.add(id);
  selectedIds.value = next;
  requestRender();
}

function removeMatches(): void {
  if (matchedIds.value.size === 0 || selectedIds.value.size === 0) return;
  const next = new Set(selectedIds.value);
  for (const id of matchedIds.value) next.delete(id);
  selectedIds.value = next;
  requestRender();
}

function selectAllVisible(): void {
  const next = new Set(selectedIds.value);
  for (const n of filteredNodes()) next.add(n.id);
  selectedIds.value = next;
  requestRender();
}

function clearSelection(): void {
  if (selectedIds.value.size === 0) return;
  selectedIds.value = new Set();
  requestRender();
}

function toggleSelectedId(id: number): void {
  const next = new Set(selectedIds.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  selectedIds.value = next;
  requestRender();
}

/**
 * Range-toggle from `SelectedNodesPanel`: every row between the anchor and
 * the shift-clicked row is flipped to match the anchor's *new* state. The
 * panel only emits ids that are currently in the selection (they come from
 * the displayed rows), so the natural semantics here is "remove all of them
 * at once" — Shift-click on a row already in the panel removes the contiguous
 * stretch from the persistent selection.
 */
function onRangeToggle(ids: readonly number[]): void {
  if (ids.length === 0) return;
  const next = new Set(selectedIds.value);
  for (const id of ids) next.delete(id);
  selectedIds.value = next;
  requestRender();
}

function onKeepOnly(ids: readonly number[]): void {
  // Forward to the host (MemoryView) — destruction must be confirmed there
  // because we don't own the store.
  emit('keep-only-selection', [...ids]);
}

// Rows for the right-side `<SelectedNodesPanel>`.
const selectionPanelNodes = computed(() =>
  nodes.value.map((n) => ({
    id: n.id,
    label: n.label,
    full: n.full,
    community: n.sourceLabel
      ? `📦 ${n.sourceLabel}${n.filePath ? ' · ' + n.filePath : ''}${n.parentSymbol ? '::' + n.parentSymbol : ''}`
      : n.groupKey || n.memoryType,
    colour: n.sourceId
      ? theme.value.repo
      : groupColor(n.groupKey || n.memoryType),
  })),
);

// Box-select (rubber band). Activated by Shift-drag on empty space.
let lassoActive = false;
let lassoMode: 'add' | 'remove' = 'add';
let lassoStartScreen: [number, number] | null = null;
let lassoEndScreen: [number, number] | null = null;

// ── Theme tokens (read once per init, refreshed on theme change) ────────────
const theme = ref({
  bg: '#040a12',
  text: '#e0f0ff',
  textMuted: '#94a3b8',
  accent: '#7c6fff',
  danger: '#ef4444',
  border: '#1e293b',
  edge: '#475569',
  repo: '#d4a14a',
});

function refreshTheme(): void {
  if (typeof window === 'undefined') return;
  const cs = getComputedStyle(document.documentElement);
  const tok = (n: string, f: string) => cs.getPropertyValue(n).trim() || f;
  theme.value = {
    bg: tok('--ts-bg-base', '#040a12'),
    text: tok('--ts-text-primary', '#e0f0ff'),
    textMuted: tok('--ts-text-muted', '#94a3b8'),
    accent: tok('--ts-accent', '#7c6fff'),
    danger: tok('--ts-danger', '#ef4444'),
    border: tok('--ts-border', '#1e293b'),
    edge: tok('--ts-text-dim', '#475569'),
    repo: tok('--ts-warning', '#d4a14a'),
  };
}

// ── Group coloring ──────────────────────────────────────────────────────────
// Obsidian colors clusters by folder/tag. We color by dominant tag, falling
// back to memory type. A small fixed palette keeps the field readable while
// still giving Obsidian's "explosion of color" feel on dense graphs.
const GROUP_PALETTE = [
  '#ef4444', // red
  '#f97316', // orange
  '#f43f5e', // rose
  '#a855f7', // purple
  '#e879f9', // pink
  '#fb7185', // coral
  '#facc15', // yellow
  '#60a5fa', // blue
  '#22d3ee', // cyan
  '#34d399', // green
  '#8b5cf6', // violet
  '#cbd5e1', // white-blue
];

function hash(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
  return h;
}

function groupColor(key: string): string {
  if (!key) return '#94a3b8';
  return GROUP_PALETTE[hash(key) % GROUP_PALETTE.length];
}

function dominantTag(tagsCsv: string): string {
  const list = tagsCsv.split(',').map((t) => t.trim()).filter(Boolean);
  return list[0] ?? '';
}

const legend = computed<{ label: string; color: string; count: number }[]>(() => {
  const counts = new Map<string, number>();
  for (const n of nodes.value) {
    counts.set(n.groupKey || 'untagged', (counts.get(n.groupKey || 'untagged') ?? 0) + 1);
  }
  const arr = [...counts.entries()].map(([k, v]) => ({ label: k, color: groupColor(k), count: v }));
  arr.sort((a, b) => b.count - a.count);
  return arr.slice(0, 10);
});

// ── Build graph data from props ─────────────────────────────────────────────
function rebuildData(): void {
  const ms = props.memories;
  const incomingEdges = props.edges ?? [];
  const knownIds = new Set(ms.map((m) => m.id));

  const degree = new Map<number, number>();
  for (const m of ms) degree.set(m.id, 0);

  const useTyped = (props.edgeMode === 'typed' || props.edgeMode === 'both') && incomingEdges.length > 0;
  const useTag = props.edgeMode === 'tag' || props.edgeMode === 'both' || (!useTyped && incomingEdges.length === 0);

  const newLinks: GLink[] = [];

  if (useTyped) {
    for (const e of incomingEdges) {
      if (!knownIds.has(e.src_id) || !knownIds.has(e.dst_id)) continue;
      degree.set(e.src_id, (degree.get(e.src_id) ?? 0) + 1);
      degree.set(e.dst_id, (degree.get(e.dst_id) ?? 0) + 1);
      newLinks.push({
        source: e.src_id,
        target: e.dst_id,
        kind: 'typed',
        rel: e.rel_type,
        edgeId: e.id,
        color: groupColor(e.rel_type),
        weight: e.confidence ?? 0.8,
      });
    }
  }

  if (useTag) {
    const TAG_EDGE_CAP = 600;
    let count = 0;
    const tagListByIdx = ms.map((m) => m.tags.split(',').map((t) => t.trim()).filter(Boolean));
    outer:
    for (let i = 0; i < ms.length; i++) {
      const a = tagListByIdx[i];
      if (a.length === 0) continue;
      for (let j = i + 1; j < ms.length; j++) {
        const b = tagListByIdx[j];
        if (b.length === 0) continue;
        let shared = 0;
        for (const t of a) if (b.includes(t)) { shared++; if (shared >= 2) break; }
        if (shared > 0) {
          newLinks.push({
            source: ms[i].id,
            target: ms[j].id,
            kind: 'tag',
            color: theme.value.edge,
            weight: shared,
          });
          degree.set(ms[i].id, (degree.get(ms[i].id) ?? 0) + 1);
          degree.set(ms[j].id, (degree.get(ms[j].id) ?? 0) + 1);
          if (++count >= TAG_EDGE_CAP) break outer;
        }
      }
    }
  }

  // Preserve existing positions so updates feel continuous instead of restarting
  const prev = new Map<number, GNode>();
  for (const n of nodes.value) prev.set(n.id, n);

  const newNodes: GNode[] = ms.map((m, i) => {
    const old = prev.get(m.id);
    const angle = (i / Math.max(1, ms.length)) * Math.PI * 2;
    const radius = 80 + Math.sqrt(i) * 6;
    const group = dominantTag(m.tags) || m.memory_type;
    return {
      id: m.id,
      label: m.content.length > 60 ? m.content.slice(0, 60) + '…' : m.content,
      full: m.content,
      memoryType: m.memory_type,
      importance: m.importance,
      groupKey: group,
      tagsCsv: m.tags ?? '',
      degree: degree.get(m.id) ?? 0,
      sourceId: m.source_id,
      sourceLabel: m.source_label,
      filePath: m.file_path,
      parentSymbol: m.parent_symbol,
      x: old?.x ?? Math.cos(angle) * radius,
      y: old?.y ?? Math.sin(angle) * radius,
      vx: old?.vx ?? 0,
      vy: old?.vy ?? 0,
      fx: null,
      fy: null,
    };
  });

  nodes.value = newNodes;
  links.value = newLinks;
}

// ── Simulation ──────────────────────────────────────────────────────────────
function startSim(): void {
  sim?.stop();
  // Filter for live simulation based on filters
  const activeNodes = filteredNodes();
  const idToNode = new Map(activeNodes.map((n) => [n.id, n]));
  const activeLinks = links.value.filter((l) => {
    const sid = typeof l.source === 'object' ? l.source.id : l.source;
    const tid = typeof l.target === 'object' ? l.target.id : l.target;
    return idToNode.has(sid) && idToNode.has(tid);
  });

  sim = forceSimulation(activeNodes, 2)
    .force(
      'link',
      forceLink(activeLinks)
        .id((d: GNode) => d.id)
        .distance(linkDistance.value)
        .strength((l: GLink) => Math.min(0.6, 0.15 + (l.weight ?? 1) * 0.1)),
    )
    .force('charge', forceManyBody().strength(repulsion.value).distanceMax(400))
    .force('center', forceCenter(0, 0).strength(0.04))
    .force('x', forceX(0).strength(gravity.value))
    .force('y', forceY(0).strength(gravity.value))
    .force('collide', forceCollide().radius((n: GNode) => nodeRadius(n) + 2).strength(0.7))
    .alpha(1)
    .alphaDecay(0.018)
    .velocityDecay(0.4)
    .on('tick', requestRender);

  isBuilding.value = false;
  requestRender();
}

function nudgeSim(alpha = 0.4): void {
  if (!sim) return;
  sim.alpha(alpha).restart();
}

// ── Filtering ────────────────────────────────────────────────────────────────
function filteredNodes(): GNode[] {
  return nodes.value.filter((n) => {
    if (!showOrphans.value && n.degree === 0) return false;
    if (n.degree < minDegree.value) return false;
    return true;
  });
}

// ── Rendering ───────────────────────────────────────────────────────────────
function nodeRadius(n: GNode): number {
  const base = 1.8 + Math.log2(n.degree + 1) * 1.45 + n.importance * 0.14;
  return base * nodeSizeMul.value;
}

function requestRender(): void {
  if (raf) return;
  raf = requestAnimationFrame(() => {
    raf = 0;
    draw();
  });
}

function resizeCanvas(): void {
  if (!canvasEl.value || !container.value) return;
  const rect = container.value.getBoundingClientRect();
  dpr = window.devicePixelRatio || 1;
  viewW = rect.width;
  viewH = rect.height;
  canvasEl.value.width = Math.max(1, Math.floor(viewW * dpr));
  canvasEl.value.height = Math.max(1, Math.floor(viewH * dpr));
  canvasEl.value.style.width = `${viewW}px`;
  canvasEl.value.style.height = `${viewH}px`;
  ctx = canvasEl.value.getContext('2d');
  ctx?.setTransform(dpr, 0, 0, dpr, 0, 0);

  requestRender();
}

function worldToScreen(wx: number, wy: number): [number, number] {
  return [viewW / 2 + (wx - camX.value) * zoom.value, viewH / 2 + (wy - camY.value) * zoom.value];
}

function screenToWorld(sx: number, sy: number): [number, number] {
  return [(sx - viewW / 2) / zoom.value + camX.value, (sy - viewH / 2) / zoom.value + camY.value];
}

function draw(): void {
  if (!ctx) return;
  const t = theme.value;
  ctx.clearRect(0, 0, viewW, viewH);

  // Subtle background gradient (Obsidian uses near-flat dark with soft vignette)
  const bgGrad = ctx.createRadialGradient(viewW / 2, viewH / 2, 0, viewW / 2, viewH / 2, Math.max(viewW, viewH) * 0.7);
  bgGrad.addColorStop(0, t.bg);
  bgGrad.addColorStop(1, t.bg);
  ctx.fillStyle = bgGrad;
  ctx.fillRect(0, 0, viewW, viewH);

  const visible = filteredNodes();
  const visibleIds = new Set(visible.map((n) => n.id));
  if (visible.length === 0) {
    drawEmptyState();
    return;
  }
  const hovered = hoverId.value;
  const hoverNeighbours = new Set<number>();
  if (hovered !== null) {
    hoverNeighbours.add(hovered);
    for (const l of links.value) {
      const sid = typeof l.source === 'object' ? l.source.id : l.source;
      const tid = typeof l.target === 'object' ? l.target.id : l.target;
      if (sid === hovered) hoverNeighbours.add(tid);
      else if (tid === hovered) hoverNeighbours.add(sid);
    }
  }

  // Highlight set (persistent selection ∪ optional live search matches).
  const highlights = highlightedIds.value;
  const hasHighlight = highlights.size > 0;
  function isLit(id: number): boolean {
    if (!hasHighlight && hovered === null) return true;
    if (hasHighlight && highlights.has(id)) return true;
    if (hovered !== null && hoverNeighbours.has(id)) return true;
    return false;
  }

  // ── Edges ──
  ctx.lineCap = 'round';
  for (const l of links.value) {
    const s = typeof l.source === 'object' ? l.source : nodes.value.find((n) => n.id === l.source);
    const tg = typeof l.target === 'object' ? l.target : nodes.value.find((n) => n.id === l.target);
    if (!s || !tg || s.x == null || s.y == null || tg.x == null || tg.y == null) continue;
    if (!visibleIds.has(s.id) || !visibleIds.has(tg.id)) continue;

    const [sx, sy] = worldToScreen(s.x, s.y);
    const [tx, ty] = worldToScreen(tg.x, tg.y);

    const focused = isLit(s.id) && isLit(tg.id);
    const baseAlpha = l.kind === 'typed' ? 0.32 : 0.14;
    const alpha = focused ? baseAlpha : baseAlpha * 0.14;
    const sourceColor = groupColor(s.groupKey || s.memoryType);

    ctx.globalAlpha = alpha;
    ctx.strokeStyle = focused ? sourceColor : t.edge;
    ctx.lineWidth = (l.kind === 'typed' ? 0.62 : 0.38) * linkWidthMul.value * (focused ? 1.25 : 1);
    ctx.beginPath();
    ctx.moveTo(sx, sy);
    ctx.lineTo(tx, ty);
    ctx.stroke();

    // Optional arrows for typed edges
    if (showArrows.value && l.kind === 'typed' && focused) {
      const ang = Math.atan2(ty - sy, tx - sx);
      const r = nodeRadius(tg) * zoom.value + 2;
      const ax = tx - Math.cos(ang) * r;
      const ay = ty - Math.sin(ang) * r;
      const arrowSize = 6;
      ctx.fillStyle = sourceColor;
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      ctx.lineTo(ax - Math.cos(ang - 0.4) * arrowSize, ay - Math.sin(ang - 0.4) * arrowSize);
      ctx.lineTo(ax - Math.cos(ang + 0.4) * arrowSize, ay - Math.sin(ang + 0.4) * arrowSize);
      ctx.closePath();
      ctx.fill();
    }
  }
  ctx.globalAlpha = 1;

  // ── Nodes ──
  // Sort: hovered + neighbours last so they paint on top
  const drawOrder = visible.slice().sort((a, b) => {
    const ah = (highlights.has(a.id) ? 2 : 0) + (hoverNeighbours.has(a.id) ? 1 : 0);
    const bh = (highlights.has(b.id) ? 2 : 0) + (hoverNeighbours.has(b.id) ? 1 : 0);
    return ah - bh;
  });

  for (const n of drawOrder) {
    if (n.x == null || n.y == null) continue;
    const [sx, sy] = worldToScreen(n.x, n.y);
    const r = nodeRadius(n) * zoom.value;
    // BRAIN-REPO-RAG-2a: repo-sourced chunks render in the warning hue so
    // they're visually distinct from personal memories.
    const color = n.sourceId
      ? theme.value.repo
      : groupColor(n.groupKey || n.memoryType);
    const lit = isLit(n.id);
    const isHighlighted = highlights.has(n.id);
    const alpha = lit ? 0.94 : 0.16;
    ctx.globalAlpha = alpha;

    // Soft halo for hovered, highlighted, and high-degree nodes
    if (hovered === n.id || isHighlighted || (n.degree >= 8 && lit)) {
      const haloR = r * (hovered === n.id ? 4.6 : isHighlighted ? 3.8 : 2.6);
      const halo = ctx.createRadialGradient(sx, sy, r * 0.6, sx, sy, haloR);
      const haloColor = isHighlighted ? t.accent : color;
      halo.addColorStop(
        0,
        hexAlpha(haloColor, hovered === n.id ? 0.5 : isHighlighted ? 0.45 : 0.2),
      );
      halo.addColorStop(1, hexAlpha(haloColor, 0));
      ctx.fillStyle = halo;
      ctx.beginPath();
      ctx.arc(sx, sy, haloR, 0, Math.PI * 2);
      ctx.fill();
    }

    // Node dot
    ctx.beginPath();
    ctx.arc(sx, sy, Math.max(1.15, r), 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();

    if (n.degree >= 12 && lit) {
      ctx.globalAlpha = Math.min(1, alpha + 0.05);
      ctx.lineWidth = 0.8;
      ctx.strokeStyle = hexAlpha('#ffffff', 0.45);
      ctx.stroke();
    }

    // Selection ring — bright accent for highlighted/selected nodes
    // (persists when the live filter is toggled off), plus the hover ring.
    if (isHighlighted) {
      ctx.globalAlpha = 1;
      ctx.lineWidth = 2;
      ctx.strokeStyle = t.accent;
      ctx.stroke();
    }
    if (hovered === n.id) {
      ctx.globalAlpha = 1;
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = t.accent;
      ctx.stroke();
    }
  }
  ctx.globalAlpha = 1;

  // ── Labels ──
  const labelAlpha = showLabels.value
    ? 1
    : Math.max(0, Math.min(1, (zoom.value - textFadeThreshold.value) / 0.5));

  if (labelAlpha > 0.02 || hovered !== null || hasHighlight) {
    ctx.font = '11px var(--ts-font-sans, system-ui, sans-serif)';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';
    for (const n of drawOrder) {
      if (n.x == null || n.y == null) continue;
      const isHover = hovered === n.id;
      const isNeighbour = hovered !== null && hoverNeighbours.has(n.id);
      const isHighlighted = highlights.has(n.id);
      // Skip dim (non-lit) nodes' labels unless they qualify for a label
      // through hover/highlight membership.
      if (!isLit(n.id)) continue;
      const showThisLabel = isHover || isNeighbour || isHighlighted || labelAlpha > 0.02;
      if (!showThisLabel) continue;
      const [sx, sy] = worldToScreen(n.x, n.y);
      const r = nodeRadius(n) * zoom.value;
      const a = isHover ? 1 : (isHighlighted ? 1 : isNeighbour ? 0.9 : labelAlpha);
      ctx.globalAlpha = a;
      // Soft shadow for legibility
      ctx.fillStyle = t.bg;
      const text = n.label;
      const ty = sy + r + 4;
      // Background pill (subtle)
      const w = ctx.measureText(text).width;
      ctx.globalAlpha = a * 0.6;
      ctx.fillStyle = t.bg;
      ctx.fillRect(sx - w / 2 - 3, ty - 1, w + 6, 14);
      ctx.globalAlpha = a;
      ctx.fillStyle = (isHover || isHighlighted) ? t.accent : t.text;
      ctx.fillText(text, sx, ty);
    }
    ctx.globalAlpha = 1;
  }

  // ── Lasso rectangle (Shift-drag box-select) ──
  if (lassoActive && lassoStartScreen && lassoEndScreen) {
    const [x0, y0] = lassoStartScreen;
    const [x1, y1] = lassoEndScreen;
    const lx = Math.min(x0, x1);
    const ly = Math.min(y0, y1);
    const lw = Math.abs(x1 - x0);
    const lh = Math.abs(y1 - y0);
    ctx.save();
    ctx.globalAlpha = 0.18;
    const lassoColour = lassoMode === 'remove' ? t.danger : t.accent;
    ctx.fillStyle = lassoColour;
    ctx.fillRect(lx, ly, lw, lh);
    ctx.globalAlpha = 0.9;
    ctx.strokeStyle = lassoColour;
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 3]);
    ctx.strokeRect(lx, ly, lw, lh);
    ctx.restore();
  }
}

function drawEmptyState(): void {
  if (!ctx) return;
  const t = theme.value;
  const cx = viewW / 2;
  const cy = viewH / 2 + 10;
  const scale = Math.max(0.8, Math.min(1.6, Math.min(viewW, viewH) / 520));

  ctx.save();
  ctx.globalAlpha = 1;
  for (let i = 0; i < emptyConstellation.length; i++) {
    const a = emptyConstellation[i];
    const ax = cx + a.x * scale;
    const ay = cy + a.y * scale;
    if (i % 3 !== 0) continue;
    for (let j = i + 1; j < Math.min(emptyConstellation.length, i + 7); j++) {
      const b = emptyConstellation[j];
      const bx = cx + b.x * scale;
      const by = cy + b.y * scale;
      const d = Math.hypot(ax - bx, ay - by);
      if (d > 82 * scale) continue;
      ctx.globalAlpha = 0.12;
      ctx.strokeStyle = a.color;
      ctx.lineWidth = 0.5;
      ctx.beginPath();
      ctx.moveTo(ax, ay);
      ctx.lineTo(bx, by);
      ctx.stroke();
    }
  }

  for (const p of emptyConstellation) {
    const x = cx + p.x * scale;
    const y = cy + p.y * scale;
    const r = p.r * scale;
    ctx.globalAlpha = 0.42;
    ctx.fillStyle = p.color;
    ctx.beginPath();
    ctx.arc(x, y, r, 0, Math.PI * 2);
    ctx.fill();
  }

  const halo = ctx.createRadialGradient(cx, cy, 8, cx, cy, 170 * scale);
  halo.addColorStop(0, hexAlpha(t.accent, 0.12));
  halo.addColorStop(1, hexAlpha(t.accent, 0));
  ctx.globalAlpha = 1;
  ctx.fillStyle = halo;
  ctx.beginPath();
  ctx.arc(cx, cy, 170 * scale, 0, Math.PI * 2);
  ctx.fill();

  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.font = '600 13px var(--ts-font-sans, system-ui, sans-serif)';
  ctx.fillStyle = t.text;
  ctx.globalAlpha = 0.92;
  ctx.fillText('No memories yet', cx, cy + 6);
  ctx.font = '11px var(--ts-font-sans, system-ui, sans-serif)';
  ctx.fillStyle = t.textMuted;
  ctx.globalAlpha = 0.75;
  ctx.fillText('Add memories to populate the graph', cx, cy + 26);
  ctx.restore();
}

function hexAlpha(hex: string, a: number): string {
  // Accepts #rrggbb or #rgb; falls through for rgb()/var()
  if (hex.startsWith('#')) {
    let h = hex.slice(1);
    if (h.length === 3) h = h.split('').map((c) => c + c).join('');
    if (h.length === 6) {
      const r = parseInt(h.slice(0, 2), 16);
      const g = parseInt(h.slice(2, 4), 16);
      const b = parseInt(h.slice(4, 6), 16);
      return `rgba(${r}, ${g}, ${b}, ${a})`;
    }
  }
  return hex;
}

// ── Pointer interaction ────────────────────────────────────────────────────
function pickNode(sx: number, sy: number): GNode | null {
  const [wx, wy] = screenToWorld(sx, sy);
  let best: GNode | null = null;
  let bestD = Infinity;
  for (const n of filteredNodes()) {
    if (n.x == null || n.y == null) continue;
    const r = nodeRadius(n) + 4 / zoom.value;
    const dx = n.x - wx;
    const dy = n.y - wy;
    const d = dx * dx + dy * dy;
    if (d <= r * r && d < bestD) {
      bestD = d;
      best = n;
    }
  }
  return best;
}

function onPointerDown(ev: PointerEvent): void {
  if (!container.value) return;
  container.value.setPointerCapture(ev.pointerId);
  const rect = container.value.getBoundingClientRect();
  const sx = ev.clientX - rect.left;
  const sy = ev.clientY - rect.top;
  pointerDownX = sx;
  pointerDownY = sy;
  pointerMoved = false;
  const n = pickNode(sx, sy);
  // Shift + click-drag on empty space → rubber-band box-select.
  // Shift + click on a node → toggle that node in the persistent selection.
  // Alt + Shift + drag → subtractive rubber-band (removes intersected nodes).
  if (ev.shiftKey) {
    if (n) {
      const next = new Set(selectedIds.value);
      if (next.has(n.id)) next.delete(n.id);
      else next.add(n.id);
      selectedIds.value = next;
      requestRender();
      return;
    }
    lassoActive = true;
    lassoMode = ev.altKey ? 'remove' : 'add';
    lassoStartScreen = [sx, sy];
    lassoEndScreen = [sx, sy];
    requestRender();
    return;
  }
  if (n) {
    draggingNode.value = n;
    n.fx = n.x;
    n.fy = n.y;
    nudgeSim(0.3);
  } else {
    panActive = true;
    panLastX = sx;
    panLastY = sy;
  }
}

function onPointerMove(ev: PointerEvent): void {
  if (!container.value) return;
  const rect = container.value.getBoundingClientRect();
  const sx = ev.clientX - rect.left;
  const sy = ev.clientY - rect.top;

  if (lassoActive) {
    lassoEndScreen = [sx, sy];
    pointerMoved = Math.hypot(sx - pointerDownX, sy - pointerDownY) > 3;
    requestRender();
    return;
  }
  if (draggingNode.value) {
    const [wx, wy] = screenToWorld(sx, sy);
    draggingNode.value.fx = wx;
    draggingNode.value.fy = wy;
    pointerMoved = true;
    nudgeSim(0.2);
    return;
  }
  if (panActive) {
    const dx = sx - panLastX;
    const dy = sy - panLastY;
    panLastX = sx;
    panLastY = sy;
    camX.value -= dx / zoom.value;
    camY.value -= dy / zoom.value;
    pointerMoved = Math.hypot(sx - pointerDownX, sy - pointerDownY) > 3;
    requestRender();
    return;
  }
  // Hover detection
  const n = pickNode(sx, sy);
  const newId = n?.id ?? null;
  if (newId !== hoverId.value) {
    hoverId.value = newId;
    requestRender();
  }
}

const hoverLabel = computed(() => {
  if (hoverId.value == null) return '';
  const n = nodes.value.find((m) => m.id === hoverId.value);
  return n?.full ?? '';
});

const emptyConstellation = Array.from({ length: 72 }, (_, i) => {
  const angle = i * 2.399963229728653;
  const radius = 34 + Math.sqrt(i) * 15;
  return {
    x: Math.cos(angle) * radius,
    y: Math.sin(angle) * radius,
    color: GROUP_PALETTE[i % GROUP_PALETTE.length],
    r: 1 + (i % 9 === 0 ? 2 : i % 5 === 0 ? 1.2 : 0),
  };
});

function onPointerUp(ev: PointerEvent): void {
  if (!container.value) return;
  container.value.releasePointerCapture?.(ev.pointerId);

  if (lassoActive && lassoStartScreen && lassoEndScreen) {
    const [x0, y0] = lassoStartScreen;
    const [x1, y1] = lassoEndScreen;
    if (pointerMoved) {
      const [wx0, wy0] = screenToWorld(Math.min(x0, x1), Math.min(y0, y1));
      const [wx1, wy1] = screenToWorld(Math.max(x0, x1), Math.max(y0, y1));
      const next = new Set(selectedIds.value);
      for (const n of filteredNodes()) {
        if (n.x == null || n.y == null) continue;
        if (n.x >= wx0 && n.x <= wx1 && n.y >= wy0 && n.y <= wy1) {
          if (lassoMode === 'remove') next.delete(n.id);
          else next.add(n.id);
        }
      }
      selectedIds.value = next;
    }
    lassoActive = false;
    lassoMode = 'add';
    lassoStartScreen = null;
    lassoEndScreen = null;
    requestRender();
    return;
  }

  const wasDraggingNode = draggingNode.value;
  if (wasDraggingNode) {
    // Release pin so node returns to physics
    wasDraggingNode.fx = null;
    wasDraggingNode.fy = null;
    draggingNode.value = null;
    if (!pointerMoved) {
      emit('select', wasDraggingNode.id);
    }
    nudgeSim(0.2);
  } else if (panActive && !pointerMoved) {
    // Tap on empty space — clear hover
    hoverId.value = null;
    requestRender();
  }
  panActive = false;
}

function onPointerLeave(): void {
  if (hoverId.value !== null) {
    hoverId.value = null;
    requestRender();
  }
}

function onWheel(ev: WheelEvent): void {
  if (!container.value) return;
  const rect = container.value.getBoundingClientRect();
  const sx = ev.clientX - rect.left;
  const sy = ev.clientY - rect.top;
  const [wxBefore, wyBefore] = screenToWorld(sx, sy);
  const factor = Math.exp(-ev.deltaY * 0.0015);
  const newZoom = Math.max(0.1, Math.min(6, zoom.value * factor));
  zoom.value = newZoom;
  const [wxAfter, wyAfter] = screenToWorld(sx, sy);
  camX.value += wxBefore - wxAfter;
  camY.value += wyBefore - wyAfter;
  requestRender();
}

function onDoubleClick(ev: MouseEvent): void {
  if (!container.value) return;
  const rect = container.value.getBoundingClientRect();
  const sx = ev.clientX - rect.left;
  const sy = ev.clientY - rect.top;
  const n = pickNode(sx, sy);
  if (n && n.x != null && n.y != null) {
    camX.value = n.x;
    camY.value = n.y;
    zoom.value = Math.max(zoom.value, 2);
    requestRender();
  } else {
    fitToView();
  }
}

function fitToView(): void {
  const visible = filteredNodes();
  if (visible.length === 0) return;
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const n of visible) {
    if (n.x == null || n.y == null) continue;
    if (n.x < minX) minX = n.x;
    if (n.y < minY) minY = n.y;
    if (n.x > maxX) maxX = n.x;
    if (n.y > maxY) maxY = n.y;
  }
  if (!isFinite(minX)) return;
  const w = Math.max(1, maxX - minX);
  const h = Math.max(1, maxY - minY);
  const pad = 60;
  zoom.value = Math.min(2, Math.min((viewW - pad * 2) / w, (viewH - pad * 2) / h));
  camX.value = (minX + maxX) / 2;
  camY.value = (minY + maxY) / 2;
  requestRender();
}

// ── Lifecycle ──────────────────────────────────────────────────────────────
let rebuildTimer: ReturnType<typeof setTimeout> | null = null;
function scheduleRebuild(): void {
  if (rebuildTimer) clearTimeout(rebuildTimer);
  rebuildTimer = setTimeout(() => {
    rebuildTimer = null;
    isBuilding.value = true;
    refreshTheme();
    rebuildData();
    startSim();
    if (zoom.value === 1 && camX.value === 0 && camY.value === 0) {
      // First load: center & light fit after a few ticks
      setTimeout(fitToView, 400);
    }
  }, 120);
}

onMounted(() => {
  refreshTheme();
  resizeCanvas();
  const onResize: ResizeObserverCallback = () => {
    resizeCanvas();
  };
  resizeObserver = new ResizeObserver(onResize);
  if (container.value) resizeObserver.observe(container.value);
  rebuildData();
  startSim();
  setTimeout(fitToView, 400);
  window.addEventListener('keydown', onWindowKeydown);
});

onUnmounted(() => {
  if (rebuildTimer) clearTimeout(rebuildTimer);
  if (raf) cancelAnimationFrame(raf);
  sim?.stop();
  sim = null;
  resizeObserver?.disconnect();
  window.removeEventListener('keydown', onWindowKeydown);
});

/**
 * Global Esc handler: clear selection only when the canvas is the active
 * surface (no input/select/textarea/contenteditable focused). Avoids
 * stealing Esc from search inputs, modals, or other text fields.
 */
function onWindowKeydown(e: KeyboardEvent) {
  if (e.key !== 'Escape') return;
  if (selectedIds.value.size === 0) return;
  const target = e.target as HTMLElement | null;
  const tag = target?.tagName ?? '';
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
  if (target?.isContentEditable) return;
  selectedIds.value = new Set();
  requestRender();
}

watch(() => props.memories, scheduleRebuild, { deep: true });
watch(() => props.edges, scheduleRebuild, { deep: true });
watch(() => props.edgeMode, scheduleRebuild);

// Re-tick simulation when force sliders change
watch([repulsion, linkDistance, gravity], () => {
  if (!sim) return;
  sim.force('charge', forceManyBody().strength(repulsion.value).distanceMax(400));
   
  const lf = sim.force('link') as any;
  lf?.distance(linkDistance.value);
  sim.force('x', forceX(0).strength(gravity.value));
  sim.force('y', forceY(0).strength(gravity.value));
  nudgeSim(0.5);
});

watch([showOrphans, minDegree], () => {
  startSim();
});

watch([showLabels, showArrows, textFadeThreshold, nodeSizeMul, linkWidthMul], requestRender);
watch([searchText, searchMode, highlightFilterActive], requestRender);
watch(searchFields, requestRender, { deep: true });
watch(selectedIds, requestRender, { deep: true });
</script>

<style scoped>
.memory-graph-shell {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 320px;
}
.memory-graph-shell > .memory-graph,
.memory-graph-shell > .brain-graph-viewport {
  flex: 1;
  min-height: 0;
}

/* Fullscreen: pin shell to viewport so both 2D canvas and 3D galaxy
   take the full screen. ESC exits. */
.memory-graph-shell--fullscreen {
  position: fixed !important;
  inset: 0;
  width: 100vw;
  height: 100vh;
  height: 100dvh;
  z-index: 9000;
  background: var(--ts-bg-app, #0a0a0f);
  border-radius: 0;
}

.mg-mode-toggle {
  position: absolute;
  /* Sit well below the 34px topbar so the pill no longer crowds the back
     button, breadcrumb title, or top-right action icons. */
  top: calc(34px + 24px);
  left: 50%;
  transform: translateX(-50%);
  z-index: 30;
  display: inline-flex;
  gap: 0;
  padding: 2px;
  background: color-mix(in srgb, var(--ts-bg-surface, #0b1220) 80%, transparent);
  border: 1px solid color-mix(in srgb, var(--ts-text-muted, #475569) 30%, transparent);
  border-radius: 999px;
  backdrop-filter: blur(6px);
}
.mg-mode-btn {
  appearance: none;
  background: transparent;
  border: 0;
  padding: 0.25rem 0.7rem;
  font: inherit;
  font-size: 0.72rem;
  color: var(--ts-text-muted, #94a3b8);
  border-radius: 999px;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}
.mg-mode-btn:hover {
  color: var(--ts-text, #e2e8f0);
}
.mg-mode-btn.active {
  background: color-mix(in srgb, var(--ts-accent, #7c6fff) 30%, transparent);
  color: var(--ts-text, #e2e8f0);
}

.memory-graph {
  width: 100%;
  height: 100%;
  background: #070b0f;
  border-radius: 4px;
  position: relative;
  overflow: hidden;
  cursor: grab;
  user-select: none;
  touch-action: none;
}
.memory-graph:active {
  cursor: grabbing;
}
.mg-canvas {
  position: absolute;
  inset: 0;
  display: block;
}

.mg-topbar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 34px;
  z-index: 4;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px;
  background: #171b21;
  border-bottom: 1px solid #242a33;
  color: #d8e6fb;
  pointer-events: auto;
}
.mg-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 14px;
  font-weight: 700;
  letter-spacing: 0;
}
.mg-title-icon {
  width: 16px;
  color: #9fb7d8;
  font-size: 13px;
  line-height: 1;
}
.mg-top-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}
.mg-icon-button {
  width: 24px;
  height: 24px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: #b8c7dd;
  font-size: 15px;
  line-height: 1;
  cursor: pointer;
}
.mg-icon-button:hover {
  background: #252c36;
  color: #e6f0ff;
}

.mg-panel {
  position: absolute;
  top: 46px;
  left: 6px;
  z-index: 3;
  width: 140px;
  background: #171b21;
  border: 1px solid #2a313b;
  border-radius: 0;
  color: #d8e6fb;
  font-size: 13px;
  box-shadow: none;
  overflow: hidden;
  pointer-events: auto;
}
.mg-panel-collapsed {
  display: none;
}
.mg-panel-body {
  padding: 5px 0;
  max-height: 70vh;
  overflow-y: auto;
}
.mg-section {
  border-top: none;
  padding: 0;
}
.mg-section > summary {
  cursor: pointer;
  font-weight: 600;
  font-size: 13px;
  text-transform: none;
  letter-spacing: 0;
  color: #d6e7fb;
  padding: 3px 10px;
  list-style: none;
  line-height: 1.45;
}
.mg-section > summary::-webkit-details-marker {
  display: none;
}
.mg-section > summary::before {
  content: '▸';
  display: inline-block;
  margin-right: 5px;
  color: #5b6d85;
  transition: transform 0.15s ease;
}
.mg-section[open] > summary::before {
  transform: rotate(90deg);
}
.mg-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 10px 4px 18px;
  font-size: 11px;
  color: #b6c4d7;
}
.mg-row-stack {
  flex-direction: column;
  align-items: stretch;
  gap: 4px;
}
.mg-row-buttons {
  gap: 4px;
}
.mg-input,
.mg-select {
  width: 100%;
  background: #0d1117;
  border: 1px solid #2a313b;
  border-radius: 3px;
  color: #d8e6fb;
  font-size: 11px;
  padding: 4px 6px;
  outline: none;
  box-sizing: border-box;
}
.mg-input:focus,
.mg-select:focus {
  border-color: var(--ts-accent, #7c6fff);
}
.mg-btn {
  display: block;
  width: 100%;
  background: #0d1117;
  border: 1px solid #2a313b;
  border-radius: 3px;
  color: #d8e6fb;
  font-size: 11px;
  padding: 4px 6px;
  text-align: left;
  cursor: pointer;
  font-family: inherit;
}
.mg-btn:hover:not(:disabled) {
  background: #1a212b;
  border-color: #3a4554;
}
.mg-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.mg-hint {
  padding: 4px 10px 6px 18px;
  font-size: 10px;
  color: #6b7a8d;
  line-height: 1.35;
}
.mg-row input[type='checkbox'] {
  accent-color: var(--ts-accent, #7c6fff);
}
.mg-row input[type='range'] {
  flex: 1;
  accent-color: var(--ts-accent, #7c6fff);
  min-width: 52px;
}
.mg-row > span:first-child {
  flex: 1;
  color: #a8b5c8;
}
.mg-val {
  width: 22px;
  text-align: right;
  font-variant-numeric: tabular-nums;
  color: #a8b5c8;
}
.mg-readout {
  display: grid;
  gap: 2px;
  padding: 2px 10px 7px 18px;
  font-size: 11px;
  color: #a8b5c8;
  font-variant-numeric: tabular-nums;
}

.mg-legend {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 2px 10px 7px 18px;
}
.mg-legend-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
}
.mg-legend-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.mg-legend-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #d6e7fb;
}
.mg-legend-count {
  color: #a8b5c8;
  font-variant-numeric: tabular-nums;
}

.mg-hover-card {
  position: absolute;
  left: 12px;
  bottom: 12px;
  z-index: 2;
  padding: 6px 10px;
  background: rgba(13, 17, 23, 0.78);
  border: 1px solid #2a313b;
  border-radius: 3px;
  font-size: 11px;
  color: #a8b5c8;
  max-width: min(560px, 60%);
  pointer-events: none;
}
.mg-hover {
  color: #d8e6fb;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  display: block;
  max-width: 100%;
}

.mg-loading {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--ts-text-sm);
  color: var(--ts-text-muted);
  pointer-events: none;
  z-index: 5;
  background: color-mix(in srgb, var(--ts-bg-base, #040a12) 50%, transparent);
}
</style>
