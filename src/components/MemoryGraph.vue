<template>
  <div
    ref="container"
    class="memory-graph"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointerleave="onPointerLeave"
    @wheel.prevent="onWheel"
    @dblclick="onDoubleClick"
  >
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
          class="mg-icon-button mg-mode-button"
          :title="renderMode === 'webgl' ? 'Switch to Lite (Canvas2D)' : 'Switch to WebGL (GPU)'"
          :aria-label="renderMode === 'webgl' ? 'Switch to Lite renderer' : 'Switch to WebGL renderer'"
          @click="toggleRenderMode"
        >
          {{ renderMode === 'webgl' ? 'GPU' : 'Lite' }}
        </button>
        <button
          type="button"
          class="mg-icon-button"
          title="Fit graph"
          aria-label="Fit graph"
          @click="fitToView"
        >
          ⌖
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

    <canvas
      ref="canvasEl"
      class="mg-canvas"
      :class="{ 'mg-canvas-hidden': renderMode === 'webgl' && webglReady }"
    />
    <div
      v-show="renderMode === 'webgl'"
      ref="webglEl"
      class="mg-webgl"
    />

    <div
      class="mg-panel"
      :class="{ 'mg-panel-collapsed': panelCollapsed }"
      @pointerdown.stop
      @pointermove.stop
      @pointerup.stop
    >
      <div
        v-if="!panelCollapsed"
        class="mg-panel-body"
      >
        <details class="mg-section">
          <summary>Views</summary>
          <div class="mg-readout">
            <span>{{ nodeCount }} nodes</span>
            <span>{{ edgeCount }} links</span>
          </div>
        </details>
        <details class="mg-section">
          <summary>Filters</summary>
          <label class="mg-row">
            <input
              v-model="showOrphans"
              type="checkbox"
            >
            <span>Show orphan nodes</span>
          </label>
          <label class="mg-row">
            <span>Min connections</span>
            <input
              v-model.number="minDegree"
              type="range"
              min="0"
              max="10"
              step="1"
            >
            <span class="mg-val">{{ minDegree }}</span>
          </label>
        </details>
        <details class="mg-section">
          <summary>Groups</summary>
          <div class="mg-legend">
            <div
              v-for="g in legend"
              :key="g.label"
              class="mg-legend-item"
            >
              <span
                class="mg-legend-dot"
                :style="{ background: g.color }"
              />
              <span class="mg-legend-label">{{ g.label }}</span>
              <span class="mg-legend-count">{{ g.count }}</span>
            </div>
          </div>
        </details>
        <details class="mg-section">
          <summary>Display</summary>
          <label class="mg-row">
            <input
              v-model="showLabels"
              type="checkbox"
            >
            <span>Always show labels</span>
          </label>
          <label class="mg-row">
            <input
              v-model="showArrows"
              type="checkbox"
            >
            <span>Show arrows</span>
          </label>
          <label class="mg-row">
            <span>Text fade</span>
            <input
              v-model.number="textFadeThreshold"
              type="range"
              min="0.4"
              max="3"
              step="0.05"
            >
          </label>
          <label class="mg-row">
            <span>Node size</span>
            <input
              v-model.number="nodeSizeMul"
              type="range"
              min="0.4"
              max="3"
              step="0.05"
            >
          </label>
          <label class="mg-row">
            <span>Link thickness</span>
            <input
              v-model.number="linkWidthMul"
              type="range"
              min="0.2"
              max="3"
              step="0.05"
            >
          </label>
        </details>
        <details class="mg-section">
          <summary>Forces</summary>
          <label class="mg-row">
            <span>Repulsion</span>
            <input
              v-model.number="repulsion"
              type="range"
              min="-400"
              max="-20"
              step="5"
            >
          </label>
          <label class="mg-row">
            <span>Link distance</span>
            <input
              v-model.number="linkDistance"
              type="range"
              min="20"
              max="200"
              step="2"
            >
          </label>
          <label class="mg-row">
            <span>Gravity</span>
            <input
              v-model.number="gravity"
              type="range"
              min="0"
              max="1"
              step="0.01"
            >
          </label>
        </details>
      </div>
    </div>

    <div
      v-if="hoverLabel"
      class="mg-hover-card"
    >
      <span
        class="mg-hover"
      >{{ hoverLabel }}</span>
    </div>

    <div
      v-if="isBuilding"
      class="mg-loading"
    >
      Building graph…
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
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
}>();

// ── Graph data model ────────────────────────────────────────────────────────
interface GNode {
  id: number;
  label: string;
  full: string;
  memoryType: string;
  importance: number;
  groupKey: string;
  degree: number;
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

// ── Theme tokens (read once per init, refreshed on theme change) ────────────
const theme = ref({
  bg: '#040a12',
  text: '#e0f0ff',
  textMuted: '#94a3b8',
  accent: '#7c6fff',
  border: '#1e293b',
  edge: '#475569',
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
    border: tok('--ts-border', '#1e293b'),
    edge: tok('--ts-text-dim', '#475569'),
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
      degree: degree.get(m.id) ?? 0,
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

  // ── Edges ──
  ctx.lineCap = 'round';
  for (const l of links.value) {
    const s = typeof l.source === 'object' ? l.source : nodes.value.find((n) => n.id === l.source);
    const tg = typeof l.target === 'object' ? l.target : nodes.value.find((n) => n.id === l.target);
    if (!s || !tg || s.x == null || s.y == null || tg.x == null || tg.y == null) continue;
    if (!visibleIds.has(s.id) || !visibleIds.has(tg.id)) continue;

    const [sx, sy] = worldToScreen(s.x, s.y);
    const [tx, ty] = worldToScreen(tg.x, tg.y);

    const focused = hovered === null || (hoverNeighbours.has(s.id) && hoverNeighbours.has(tg.id));
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
    const ah = hoverNeighbours.has(a.id) ? 1 : 0;
    const bh = hoverNeighbours.has(b.id) ? 1 : 0;
    return ah - bh;
  });

  for (const n of drawOrder) {
    if (n.x == null || n.y == null) continue;
    const [sx, sy] = worldToScreen(n.x, n.y);
    const r = nodeRadius(n) * zoom.value;
    const color = groupColor(n.groupKey || n.memoryType);
    const focused = hovered === null || hoverNeighbours.has(n.id);
    const alpha = focused ? 0.94 : 0.16;
    ctx.globalAlpha = alpha;

    // Soft halo for hovered & high-degree nodes
    if (hovered === n.id || (n.degree >= 8 && focused)) {
      const haloR = r * (hovered === n.id ? 4.6 : 2.6);
      const halo = ctx.createRadialGradient(sx, sy, r * 0.6, sx, sy, haloR);
      halo.addColorStop(0, hexAlpha(color, hovered === n.id ? 0.5 : 0.2));
      halo.addColorStop(1, hexAlpha(color, 0));
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

    if (n.degree >= 12 && focused) {
      ctx.globalAlpha = Math.min(1, alpha + 0.05);
      ctx.lineWidth = 0.8;
      ctx.strokeStyle = hexAlpha('#ffffff', 0.45);
      ctx.stroke();
    }

    // Selection ring on hover
    if (hovered === n.id) {
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

  if (labelAlpha > 0.02 || hovered !== null) {
    ctx.font = '11px var(--ts-font-sans, system-ui, sans-serif)';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';
    for (const n of drawOrder) {
      if (n.x == null || n.y == null) continue;
      const isHover = hovered === n.id;
      const isNeighbour = hovered !== null && hoverNeighbours.has(n.id);
      // Skip dim nodes' labels unless hovered
      if (hovered !== null && !isNeighbour) continue;
      const showThisLabel = isHover || isNeighbour || labelAlpha > 0.02;
      if (!showThisLabel) continue;
      const [sx, sy] = worldToScreen(n.x, n.y);
      const r = nodeRadius(n) * zoom.value;
      const a = isHover ? 1 : (isNeighbour ? 0.9 : labelAlpha);
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
      ctx.fillStyle = isHover ? t.accent : t.text;
      ctx.fillText(text, sx, ty);
    }
    ctx.globalAlpha = 1;
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
    if (renderMode.value === 'webgl' && webglReady.value) {
      syncWebglGraph();
    }
  }, 120);
}

// ── Render mode (Lite Canvas2D ↔ WebGL via sigma.js) ────────────────────────
//
// "Lite" mode is the default and always works (Canvas2D, no GPU required).
// "WebGL" mode lazy-loads sigma.js + graphology and uses the GPU for large
// graphs (5k+ visible nodes). The toggle is persisted to localStorage.
//
// Tests run under jsdom, which has no WebGL — they always render via Lite.
const RENDER_MODE_KEY = 'terransoul:memory-graph:render-mode';
const initialRenderMode = (() => {
  if (typeof window === 'undefined') return 'lite' as const;
  try {
    const saved = window.localStorage.getItem(RENDER_MODE_KEY);
    return saved === 'webgl' ? ('webgl' as const) : ('lite' as const);
  } catch {
    return 'lite' as const;
  }
})();
const renderMode = ref<'lite' | 'webgl'>(initialRenderMode);
const webglReady = ref(false);
const webglEl = ref<HTMLDivElement | null>(null);
// sigma.js + graphology are lazy-loaded so jsdom tests don't import WebGL.
 
let sigmaInstance: any = null;
 
let sigmaGraph: any = null;

async function enableWebgl(): Promise<void> {
  if (sigmaInstance || !webglEl.value) return;
  // Detect WebGL — jsdom and old hardware lack it. Fall back to lite.
  if (typeof document !== 'undefined') {
    const probe = document.createElement('canvas');
    const gl =
      (probe.getContext('webgl2') as WebGL2RenderingContext | null) ||
      (probe.getContext('webgl') as WebGLRenderingContext | null);
    if (!gl) {
      renderMode.value = 'lite';
      return;
    }
  }
  try {
    const [{ default: Graph }, sigmaModule] = await Promise.all([
      import('graphology'),
      import('sigma'),
    ]);
     
    const Sigma = (sigmaModule as any).default ?? (sigmaModule as any).Sigma;
    sigmaGraph = new Graph({ multi: false, type: 'directed', allowSelfLoops: false });
    sigmaInstance = new Sigma(sigmaGraph, webglEl.value, {
      renderEdgeLabels: false,
      labelDensity: 0.07,
      labelGridCellSize: 60,
      labelRenderedSizeThreshold: 8,
      defaultEdgeColor: theme.value.edge,
      defaultNodeColor: theme.value.accent,
    });
     
    sigmaInstance.on('clickNode', (payload: { node: string }) => {
      const id = Number(payload.node);
      if (!Number.isNaN(id)) emit('select', id);
    });
    webglReady.value = true;
    syncWebglGraph();
  } catch (err) {
    // Sigma failed to load — fall back to lite gracefully.
    console.warn('[MemoryGraph] WebGL renderer failed to load, using Lite:', err);
    renderMode.value = 'lite';
    teardownWebgl();
  }
}

function teardownWebgl(): void {
  if (sigmaInstance) {
    try {
      sigmaInstance.kill();
    } catch {
      /* ignore */
    }
  }
  sigmaInstance = null;
  sigmaGraph = null;
  webglReady.value = false;
}

function syncWebglGraph(): void {
  if (!sigmaInstance || !sigmaGraph) return;
  sigmaGraph.clear();
  for (const n of nodes.value) {
    const color = groupColor(n.groupKey);
    const size = Math.max(2, 2 + Math.sqrt(n.degree) * 1.4 * nodeSizeMul.value);
    sigmaGraph.addNode(String(n.id), {
      x: n.x ?? Math.random() * 100 - 50,
      y: n.y ?? Math.random() * 100 - 50,
      size,
      label: showLabels.value ? n.label : '',
      color,
    });
  }
  for (const l of links.value) {
    const sId = String(typeof l.source === 'object' ? l.source.id : l.source);
    const tId = String(typeof l.target === 'object' ? l.target.id : l.target);
    if (!sigmaGraph.hasNode(sId) || !sigmaGraph.hasNode(tId)) continue;
    if (sigmaGraph.hasEdge(sId, tId)) continue;
    sigmaGraph.addEdgeWithKey(`${sId}-${tId}`, sId, tId, {
      size: Math.max(0.4, 0.6 * linkWidthMul.value),
      color: l.color,
    });
  }
}

function toggleRenderMode(): void {
  const next = renderMode.value === 'webgl' ? 'lite' : 'webgl';
  renderMode.value = next;
  try {
    window.localStorage.setItem(RENDER_MODE_KEY, next);
  } catch {
    /* ignore */
  }
  if (next === 'webgl') {
    void enableWebgl();
  } else {
    teardownWebgl();
    requestRender();
  }
}

onMounted(() => {
  refreshTheme();
  resizeCanvas();
  const onResize: ResizeObserverCallback = () => {
    resizeCanvas();
    if (renderMode.value === 'webgl' && sigmaInstance) {
      try {
        sigmaInstance.refresh();
      } catch {
        /* sigma not ready */
      }
    }
  };
  resizeObserver = new ResizeObserver(onResize);
  if (container.value) resizeObserver.observe(container.value);
  rebuildData();
  startSim();
  setTimeout(fitToView, 400);
  if (renderMode.value === 'webgl') {
    void enableWebgl();
  }
});

onUnmounted(() => {
  if (rebuildTimer) clearTimeout(rebuildTimer);
  if (raf) cancelAnimationFrame(raf);
  sim?.stop();
  sim = null;
  resizeObserver?.disconnect();
  teardownWebgl();
});

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
</script>

<style scoped>
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
.mg-canvas-hidden {
  visibility: hidden;
  pointer-events: none;
}
.mg-webgl {
  position: absolute;
  inset: 34px 0 0 0;
  pointer-events: auto;
  background: transparent;
}
.mg-mode-button {
  width: auto !important;
  min-width: 38px;
  padding: 0 8px !important;
  font-size: 11px !important;
  font-weight: 600;
  letter-spacing: 0.02em;
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
