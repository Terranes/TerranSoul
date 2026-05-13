<template>
  <div
    ref="containerRef"
    class="brain-graph-viewport"
    data-testid="brain-graph-viewport"
    @wheel.prevent="onWheel"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointerleave="onPointerLeave"
  >
    <canvas ref="canvasRef" />

    <nav
      class="bgv-breadcrumb"
      data-testid="bgv-breadcrumb"
      @pointerdown.stop
      @pointermove.stop
      @pointerup.stop
    >
      <button
        type="button"
        class="bgv-crumb"
        :class="{ active: viewMode === 'universe' }"
        :disabled="viewMode === 'universe'"
        data-testid="bgv-crumb-universe"
        @click="exitPlanet"
      >
        🌌 Universe
      </button>
      <span
        v-if="viewMode === 'planet' && activePlanetKey"
        class="bgv-crumb-sep"
      >›</span>
      <span
        v-if="viewMode === 'planet' && activePlanetKey"
        class="bgv-crumb active"
      >
        <span
          class="bgv-crumb-dot"
          :style="{ background: activePlanetColour }"
        />
        {{ activePlanetKey }}
        <small>({{ activePlanetCount }})</small>
      </span>
    </nav>

    <button
      type="button"
      class="bgv-capture-btn"
      title="Save 3D graph PNG to Downloads (Ctrl+Shift+S)"
      data-testid="bgv-capture-btn"
      @pointerdown.stop
      @click.stop="captureScreenshot"
    >
      📸
    </button>

    <div
      v-if="memories.length === 0"
      class="bgv-empty"
      data-testid="bgv-empty"
    >
      No memory nodes yet.
    </div>

    <GraphControlPanel
      title="Graph controls"
      :collapsed="panelCollapsed"
      :node-count="graphStats.nodeCount"
      :edge-count="graphStats.linkCount"
      :show-views="true"
      :show-filters="true"
      :show-orphans="showOrphans"
      :min-degree="minDegree"
      :search-text="searchText"
      :search-mode="searchMode"
      :search-fields="selection.searchFields"
      :highlight-filter-active="highlightFilterActive"
      :match-count="matchCount"
      :selected-count="selectedCount"
      :visible-node-count="visibleNodeCount"
      :legend="panelLegend"
      :show-display="true"
      :show-labels="showLabels"
      :show-arrows="showArrows"
      :text-fade-threshold="textFadeThreshold"
      :node-size-mul="nodeSizeMul"
      :link-width-mul="linkWidthMul"
      :show-forces="true"
      :repulsion="repulsion"
      :link-distance="linkDistance"
      :gravity="gravity"
      @update:collapsed="panelCollapsed = $event"
      @update:show-orphans="showOrphans = $event"
      @update:min-degree="minDegree = $event"
      @update:search-text="searchText = $event"
      @update:search-mode="searchMode = $event as 'contains' | 'starts' | 'ends'"
      @update:search-field="(p) => (selection.searchFields[p.field] = p.value)"
      @update:highlight-filter-active="highlightFilterActive = $event"
      @update:show-labels="showLabels = $event"
      @update:show-arrows="showArrows = $event"
      @update:text-fade-threshold="textFadeThreshold = $event"
      @update:node-size-mul="nodeSizeMul = $event"
      @update:link-width-mul="linkWidthMul = $event"
      @update:repulsion="repulsion = $event"
      @update:link-distance="linkDistance = $event"
      @update:gravity="gravity = $event"
      @select-matches="selectAllMatches"
      @add-matches="addMatches"
      @remove-matches="removeMatches"
      @select-all-visible="selectAllVisible"
      @clear-selection="clearSelection"
    >
      <template #views>
        <div
          class="bgv-mode-tabs"
          role="tablist"
          aria-label="3D graph lens"
        >
          <button
            type="button"
            :class="['bgv-mode-tab', { active: focusMode === 'overview' }]"
            @click="setFocusMode('overview')"
          >
            Global
          </button>
          <button
            type="button"
            :class="['bgv-mode-tab', { active: focusMode === 'communities' }]"
            @click="setFocusMode('communities')"
          >
            Clusters
          </button>
          <button
            type="button"
            :class="['bgv-mode-tab', { active: focusMode === 'gaps' }]"
            @click="setFocusMode('gaps')"
          >
            Gaps
          </button>
        </div>
        <div class="bgv-metrics">
          <span><strong>{{ graphStats.communityCount }}</strong> clusters</span>
          <span><strong>{{ graphStats.gapCount }}</strong> gap links</span>
        </div>
        <button
          type="button"
          class="bgv-btn bgv-fit-btn"
          title="Fit graph"
          @click="fitToView"
        >
          ⌖ Fit graph
        </button>
      </template>
    </GraphControlPanel>

    <div
      v-if="lassoVisible"
      class="bgv-lasso"
      :class="{ 'bgv-lasso-remove': lassoModeRef === 'remove' }"
      :style="lassoStyle"
    />

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

    <aside
      v-if="activeMemory"
      class="bgv-inspector"
      data-testid="bgv-inspector"
      @pointerdown.stop
      @pointermove.stop
      @pointerup.stop
    >
      <div class="bgv-inspector-top">
        <span
          class="bgv-inspector-dot"
          :style="{ background: activeNodeColour }"
        />
        <span>#{{ activeMemory.id }}</span>
        <small>{{ activeNode?.community ?? classifyMemoryKind(activeMemory) }}</small>
      </div>
      <p>{{ truncate(activeMemory.content, 180) }}</p>
      <div class="bgv-inspector-meta">
        <span>{{ classifyMemoryKind(activeMemory) }}</span>
        <span>{{ activeNode?.degree ?? 0 }} links</span>
        <span>{{ activeMemory.importance }} importance</span>
      </div>
    </aside>

    <div
      v-if="hoveredMemory"
      class="bgv-tooltip"
      :style="tooltipStyle"
      data-testid="bgv-tooltip"
    >
      <strong>#{{ hoveredMemory.id }}</strong>
      <span>{{ classifyMemoryKind(hoveredMemory) }}</span>
      <p>{{ truncate(hoveredMemory.content, 110) }}</p>
    </div>

    <div
      v-if="communityLegend.length > 0"
      class="bgv-legend bgv-community-legend"
      data-testid="bgv-legend"
    >
      <span class="bgv-legend-head">Sectors</span>
      <span
        v-for="item in communityLegend"
        :key="item.label"
        class="bgv-legend-item"
      >
        <span
          class="bgv-legend-dot"
          :style="{ background: item.colour }"
        />{{ item.label }} <em>{{ item.count }}</em>
      </span>
    </div>

    <div
      class="bgv-hud-brand"
      data-testid="bgv-hud-brand"
      aria-hidden="true"
    >
      <span class="mark" />
      <span class="titles">
        <span class="t1">Memory</span>
        <span class="t2">Knowledge Graph</span>
      </span>
    </div>

    <div
      class="bgv-hud-status"
      data-testid="bgv-hud-status"
      aria-label="Graph statistics"
    >
      <span><span class="dot" />ACTIVE</span>
      <span><span class="stat-num">{{ graphStats.nodeCount.toLocaleString() }}</span> NODES</span>
      <span><span class="stat-num">{{ graphStats.linkCount.toLocaleString() }}</span> LINKS</span>
      <span
        v-if="graphStats.communityCount > 0"
      ><span class="stat-num">{{ graphStats.communityCount.toLocaleString() }}</span> SECTORS</span>
    </div>

    <div
      class="bgv-hud-hint"
      data-testid="bgv-hud-hint"
      aria-hidden="true"
    >
      <span class="k"><kbd>Drag</kbd> orbit</span>
      <span class="k"><kbd>Scroll</kbd> zoom</span>
      <span class="k"><kbd>Click</kbd> select</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import {
  ref,
  onMounted,
  onBeforeUnmount,
  watch,
  computed,
  type CSSProperties,
} from 'vue';
import {
  Scene,
  PerspectiveCamera,
  WebGLRenderer,
  BufferGeometry,
  BufferAttribute,
  LineSegments,
  Points,
  ShaderMaterial,
  CanvasTexture,
  Color,
  Raycaster,
  Vector2,
  Vector3,
  FogExp2,
  Group,
} from 'three';
import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';
import {
  forceSimulation,
  forceLink,
  forceManyBody,
  forceCollide,
  type SimulationNodeDatum,
  type SimulationLinkDatum,
  type Force,
} from 'd3-force-3d';
import type { MemoryEntry, MemoryEdge } from '../types';
import { type CognitiveKind } from '../utils/cognitive-kind';

const props = withDefaults(defineProps<{
  memories: MemoryEntry[];
  edges: MemoryEdge[];
  selectedId?: number | null;
  edgeMode?: 'typed' | 'tag' | 'both';
}>(), {
  selectedId: null,
  edgeMode: 'typed',
});

const emit = defineEmits<{
  select: [id: number];
  'keep-only-selection': [ids: number[]];
}>();

import {
  classifyMemoryKind,
  relTypeColour,
  safeCoord,
  truncate,
  dominantTag,
  communityColour,
} from './BrainGraphHelpers';
import { makeSpriteTexture, NODE_FS, GLOW_FS, HALO_FS, makeNodeMaterial as makeNodeMaterialBase, buildStarfield as createStarfield, buildTwinkleStarfield, buildEdgeLines as createEdgeLines, type TwinkleStarfieldResult } from './BrainGraphShaders';
import { createGraphSelection, type SearchableNode } from './useGraphSelection';
import SelectedNodesPanel from './SelectedNodesPanel.vue';
import GraphControlPanel from './GraphControlPanel.vue';

interface GraphNode extends SimulationNodeDatum {
  id: number;
  kind: CognitiveKind;
  content: string;
  tags: string;
  memoryType: string;
  importance: number;
  degree: number;
  community: string;
  communityIndex: number;
  colour: string;
  radius: number;
}

interface GraphLink extends SimulationLinkDatum<GraphNode> {
  source: GraphNode | number;
  target: GraphNode | number;
  relType: string;
  confidence: number;
  kind: 'typed' | 'tag';
  crossCommunity: boolean;
  colour: string;
}

type FocusMode = 'overview' | 'communities' | 'gaps';
type ViewMode = 'universe' | 'planet';

interface Planet {
  key: string;
  center: Vector3;
  colour: string;
  count: number;
  nodeIds: Set<number>;
  /** Visual radius used when rendering the planet billboard. */
  radius: number;
}

const containerRef = ref<HTMLDivElement | null>(null);
const canvasRef = ref<HTMLCanvasElement | null>(null);
const hoveredId = ref<number | null>(null);
const tooltipStyle = ref<CSSProperties>({});
const focusMode = ref<FocusMode>('communities');
const viewMode = ref<ViewMode>('universe');
const activePlanetKey = ref<string | null>(null);
const graphVersion = ref(0);

// Search + persistent selection state shared with the 2D MemoryGraph via the
// `useGraphSelection` composable so both panels behave identically.
const selection = createGraphSelection<SearchableNode>(() =>
  nodes.map((n) => ({
    id: n.id,
    label: n.content,
    tags: n.tags,
    body: n.content,
    community: n.community ?? '',
  })),
);
const {
  searchText,
  searchMode,
  highlightFilterActive,
  selectedIds,
  highlightedIds,
  matchCount,
  selectedCount,
} = selection;

function selectAllMatches() {
  selection.selectMatches();
  updateHighlighting();
}
function addMatches() {
  selection.addMatches();
  updateHighlighting();
}
function removeMatches() {
  selection.removeMatches();
  updateHighlighting();
}
function selectAllVisible() {
  if (nodes.length === 0) return;
  const next = new Set(selectedIds.value);
  for (const n of nodes) next.add(n.id);
  selectedIds.value = next;
  updateHighlighting();
}
function clearSelection() {
  selection.clearSelection();
  updateHighlighting();
}
function toggleSelectedId(id: number) {
  selection.toggleSelected(id);
  updateHighlighting();
}

// ── Shared GraphControlPanel state (parity with 2D MemoryGraph) ────────────
// All sliders/checkboxes the panel exposes. The 3D viewport wires the ones
// that make sense (node size, link width, labels, group filter) and leaves
// the rest reactive-but-passive so toggling them in 2D and 3D feels uniform.
const panelCollapsed = ref(false);
const showOrphans = ref(true);
const minDegree = ref(0);
const showLabels = ref(false);
const showArrows = ref(false);
const textFadeThreshold = ref(2.1);
const nodeSizeMul = ref(1);
const linkWidthMul = ref(1);
const repulsion = ref(-70);
const linkDistance = ref(38);
const gravity = ref(0.08);

// Lasso (Shift-drag box select) — screen-space rectangle that captures
// every node whose projected screen position falls inside it.
const lassoVisible = ref(false);
const lassoStyle = ref<CSSProperties>({});
let lassoActive = false;
let lassoMode: 'add' | 'remove' = 'add';
const lassoModeRef = ref<'add' | 'remove'>('add');
let lassoStartScreen: [number, number] | null = null;
let lassoEndScreen: [number, number] | null = null;

let scene: Scene | null = null;
let camera: PerspectiveCamera | null = null;
let renderer: WebGLRenderer | null = null;
let composer: EffectComposer | null = null;
let sim: ReturnType<typeof forceSimulation<GraphNode>> | null = null;
let rafId = 0;
let nodes: GraphNode[] = [];
let links: GraphLink[] = [];
let nodePoints: Points | null = null;
let nodeMaterial: ShaderMaterial | null = null;
let glowPoints: Points | null = null;
let glowMaterial: ShaderMaterial | null = null;
let starsPoints: Points | null = null;
let linesMesh: LineSegments | null = null;
let haloPoints: Points | null = null;
let haloMaterial: ShaderMaterial | null = null;
let worldGroup: Group | null = null;
let spriteTexture: CanvasTexture | null = null;
let resizeObserver: ResizeObserver | null = null;
let disposed = false;
let lastFrame = 0;

// ── Universe-mode resources (planets + inter-planet arcs) ──────────────────
let planetGroup: Group | null = null;
let planetPoints: Points | null = null;
let planetGlow: Points | null = null;
let planetArcs: LineSegments | null = null;
let planets: Planet[] = [];
// Snapshot of every planet's nodes so we can fly back into a planet view
// without re-running the full force sim on the whole graph.
const planetByKey: Map<string, Planet> = new Map();

let isDragging = false;
let movedDuringPointer = false;
let pendingHitId: number | null = null;
let pendingPlanetHit: string | null = null;
let prevX = 0;
let prevY = 0;
let theta = -0.55;
// Equatorial-ish 3D orbit so the cluster reads as a ball (not a flat disc).
// Inspired by 3D knowledge-graph viewers (e.g. ivgraph.com) where the camera
// orbits a dense sphere of nodes rather than looking down on a galaxy.
// phi = polar angle from +Y. π/2 is horizontal (camera level with disc).
// π/2.15 ≈ 84° gives a low-angle 2.5D "flying over" view of the disc.
let phi = Math.PI / 2.15;
let radius = 720;
const autoRotate = false;
let userInteractedAt = 0;

const selectedId = computed(() => props.selectedId ?? null);
const activeId = computed(() => hoveredId.value ?? selectedId.value);
const memoriesById = computed(() => new Map(props.memories.map((memory) => [memory.id, memory])));
const hoveredMemory = computed(() => hoveredId.value == null ? null : memoriesById.value.get(hoveredId.value) ?? null);
const activeMemory = computed(() => activeId.value == null ? null : memoriesById.value.get(activeId.value) ?? null);

const activeNode = computed(() => {
  void graphVersion.value;
  return activeId.value == null ? null : nodes.find((node) => node.id === activeId.value) ?? null;
});

const activeNodeColour = computed(() => activeNode.value?.colour ?? '#94a3b8');

const communityLegend = computed(() => {
  void graphVersion.value;
  const counts = new Map<string, { count: number; colour: string }>();
  for (const node of nodes) {
    const current = counts.get(node.community);
    if (current) current.count++;
    else counts.set(node.community, { count: 1, colour: node.colour });
  }
  return [...counts.entries()]
    .map(([label, value]) => ({ label, ...value }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 8);
});

const graphStats = computed(() => {
  void graphVersion.value;
  return {
    nodeCount: nodes.length,
    linkCount: links.length,
    communityCount: new Set(nodes.map((node) => node.community)).size,
    gapCount: links.filter((link) => link.crossCommunity).length,
  };
});

// Rows for the right-side `<SelectedNodesPanel>`. Recomputed on every graph
// rebuild via `graphVersion` so a freshly built node list shows up correctly.
const selectionPanelNodes = computed(() => {
  void graphVersion.value;
  return nodes.map((n) => ({
    id: n.id,
    label: truncate(n.content, 56),
    full: n.content,
    community: n.community,
    colour: n.colour,
  }));
});

function onRangeToggle(ids: readonly number[]) {
  if (ids.length === 0) return;
  const next = new Set(selectedIds.value);
  for (const id of ids) next.delete(id);
  selectedIds.value = next;
  updateHighlighting();
}

function onKeepOnly(ids: readonly number[]) {
  emit('keep-only-selection', [...ids]);
}

// Legend rows shaped for `GraphControlPanel` (label + colour + count).
const panelLegend = computed(() => {
  void graphVersion.value;
  return communityLegend.value.map((entry) => ({
    label: entry.label,
    color: entry.colour,
    count: entry.count,
  }));
});

const visibleNodeCount = computed(() => {
  void graphVersion.value;
  return nodes.length;
});

const activePlanetColour = computed(() => {
  void graphVersion.value;
  if (!activePlanetKey.value) return '#94a3b8';
  return planetByKey.get(activePlanetKey.value)?.colour ?? '#94a3b8';
});

const activePlanetCount = computed(() => {
  void graphVersion.value;
  if (!activePlanetKey.value) return 0;
  return planetByKey.get(activePlanetKey.value)?.count ?? 0;
});

// ── Search/selection helpers ───────────────────────────────────────────────
// All search + persistent-selection logic lives in `useGraphSelection.ts` so
// the 2D MemoryGraph and 3D BrainGraphViewport stay in sync.

function updateCameraFromOrbit() {
  if (!camera) return;
  camera.position.set(
    radius * Math.sin(phi) * Math.cos(theta),
    radius * Math.cos(phi),
    radius * Math.sin(phi) * Math.sin(theta),
  );
  camera.lookAt(0, 0, 0);
}

// Single source of truth for the universe-mode disc radius. The graph is
// laid out as a flat 2.5D disc (y ≈ 0) viewed from a low camera angle, like
// Obsidian's graph view with a perspective tilt. Updated per build in
// `buildGraph` based on node count.
let discRadius = 320;
// Vertical slab thickness — how far above/below the y=0 plane nodes may
// drift. Small relative to `discRadius` so the graph reads as 2.5D, not 3D.
const DISC_SLAB = 28;

function clusterCenter(index: number, total: number): Vector3 {
  const count = Math.max(1, total);
  // Sunflower / vogel disc distribution — clusters spread across a flat
  // disc (XZ plane). y is jittered slightly per-cluster so different
  // communities sit on faintly different "shelves" but the whole graph
  // still reads as a planar map.
  const golden = Math.PI * (3 - Math.sqrt(5));
  const t = (index + 0.5) / count;
  const r = Math.sqrt(t) * discRadius;
  const angle = golden * index;
  // Stable but varied y-offset per cluster (deterministic from index).
  const yOffset = (((index * 9301 + 49297) % 233280) / 233280 - 0.5) * DISC_SLAB * 1.4;
  return new Vector3(
    Math.cos(angle) * r,
    yOffset,
    Math.sin(angle) * r,
  );
}

function nodeRadius(node: Pick<GraphNode, 'degree' | 'importance'>): number {
  return 3.6 + Math.log2(node.degree + 1) * 2.1 + Math.max(1, node.importance) * 0.6;
}

function communityStrength(): number {
  if (focusMode.value === 'gaps') return 0.22;
  if (focusMode.value === 'communities') return 0.20;
  return 0.16;
}

function createClusterForce(totalCommunities: number): Force<GraphNode> {
  let forceNodes: GraphNode[] = [];
  const force = ((alpha: number) => {
    const strength = communityStrength() * alpha;
    for (const node of forceNodes) {
      const center = clusterCenter(node.communityIndex, totalCommunities);
      // Equal pull on all three axes — keeps clusters as 3D blobs in a sphere
      // rather than collapsing to a flat disc.
      node.vx = (node.vx ?? 0) + (center.x - safeCoord(node.x)) * strength;
      node.vy = (node.vy ?? 0) + (center.y - safeCoord(node.y)) * strength;
      node.vz = (node.vz ?? 0) + (center.z - safeCoord(node.z)) * strength;
    }
  }) as Force<GraphNode>;
  force.initialize = (nextNodes: GraphNode[]) => {
    forceNodes = nextNodes;
  };
  return force;
}

/**
 * Disc-slab force — pulls every node toward its cluster's target y-offset
 * so the whole graph stays in a thin horizontal slab (y ≈ ±DISC_SLAB).
 * This is what gives the 2.5D "flat map viewed from a low camera" look,
 * instead of a 3D ball-of-yarn.
 */
function createSlabForce(totalCommunities: number): Force<GraphNode> {
  let forceNodes: GraphNode[] = [];
  const force = ((alpha: number) => {
    const k = 0.45 * alpha;
    for (const node of forceNodes) {
      const targetY = clusterCenter(node.communityIndex, totalCommunities).y;
      const y = safeCoord(node.y);
      node.vy = (node.vy ?? 0) + (targetY - y) * k;
    }
  }) as Force<GraphNode>;
  force.initialize = (nextNodes: GraphNode[]) => {
    forceNodes = nextNodes;
  };
  return force;
}

function makeTagLinks(memories: MemoryEntry[], knownIds: Set<number>, degree: Map<number, number>): GraphLink[] {
  if (props.edgeMode !== 'tag' && props.edgeMode !== 'both') return [];
  const tagToIds = new Map<string, number[]>();
  for (const memory of memories) {
    for (const raw of (memory.tags ?? '').split(',')) {
      const tag = raw.trim().toLowerCase();
      if (!tag) continue;
      const list = tagToIds.get(tag) ?? [];
      if (list.length < 48) list.push(memory.id);
      tagToIds.set(tag, list);
    }
  }

  const pairWeights = new Map<string, number>();
  for (const ids of tagToIds.values()) {
    for (let i = 0; i < ids.length; i++) {
      for (let j = i + 1; j < ids.length; j++) {
        const a = Math.min(ids[i], ids[j]);
        const b = Math.max(ids[i], ids[j]);
        const key = `${a}:${b}`;
        pairWeights.set(key, (pairWeights.get(key) ?? 0) + 1);
      }
    }
  }

  const tagLinks: GraphLink[] = [];
  for (const [key, weight] of [...pairWeights.entries()].sort((a, b) => b[1] - a[1]).slice(0, 700)) {
    const [srcRaw, dstRaw] = key.split(':');
    const srcId = Number(srcRaw);
    const dstId = Number(dstRaw);
    if (!knownIds.has(srcId) || !knownIds.has(dstId)) continue;
    degree.set(srcId, (degree.get(srcId) ?? 0) + 1);
    degree.set(dstId, (degree.get(dstId) ?? 0) + 1);
    tagLinks.push({
      source: srcId,
      target: dstId,
      relType: 'co_tagged',
      confidence: Math.min(1, weight / 4),
      kind: 'tag',
      crossCommunity: false,
      colour: '#64748b',
    });
  }
  return tagLinks;
}

function buildGraph() {
  if (!scene) return;
  const allMemories = props.memories;

  // ── Step 1: build the planet catalogue from the full memory set, keyed by
  // `memory_type`. The universe view always renders these as billboards;
  // planet view filters the per-node graph down to one planet's nodes.
  buildPlanets(allMemories);

  // ── Step 2: in planet mode, restrict the per-node force graph to nodes
  // owned by the active planet so the spinning sphere only contains that
  // planet's memories. Universe mode keeps the full set so search/select
  // can still target any node by id.
  const planet = viewMode.value === 'planet' && activePlanetKey.value
    ? planetByKey.get(activePlanetKey.value)
    : null;
  const memories = planet
    ? allMemories.filter((m) => planet.nodeIds.has(m.id))
    : allMemories;

  const knownIds = new Set(memories.map((memory) => memory.id));
  const degree = new Map<number, number>();
  for (const memory of memories) degree.set(memory.id, 0);

  const typedLinks: GraphLink[] = (props.edgeMode === 'typed' || props.edgeMode === 'both')
    ? props.edges
        .filter((edge) => knownIds.has(edge.src_id) && knownIds.has(edge.dst_id))
        .map((edge) => {
          degree.set(edge.src_id, (degree.get(edge.src_id) ?? 0) + 1);
          degree.set(edge.dst_id, (degree.get(edge.dst_id) ?? 0) + 1);
          return {
            source: edge.src_id,
            target: edge.dst_id,
            relType: edge.rel_type,
            confidence: edge.confidence ?? 0.8,
            kind: 'typed' as const,
            crossCommunity: false,
            colour: relTypeColour(edge.rel_type),
          };
        })
    : [];

  const tagLinks = makeTagLinks(memories, knownIds, degree);
  const communities = [...new Set(memories.map((memory) => dominantTag(memory.tags) || classifyMemoryKind(memory)))].sort();
  const communityIndex = new Map(communities.map((community, index) => [community, index]));

  nodes = memories.map((memory, index) => {
    const community = dominantTag(memory.tags) || classifyMemoryKind(memory);
    const cIndex = communityIndex.get(community) ?? 0;
    const center = clusterCenter(cIndex, communities.length);
    const angle = index * 2.399963229728653;
    const localRadius = 18 + Math.sqrt(index % 41) * 6;
    const d = degree.get(memory.id) ?? 0;
    // Isotropic 3D jitter around each cluster center so the cluster reads as
    // a spherical blob, not a ring on the XZ plane.
    const jitter = (seed: number) => Math.sin(index * seed) * (10 + (index % 7));
    return {
      id: memory.id,
      kind: classifyMemoryKind(memory),
      content: memory.content,
      tags: memory.tags ?? '',
      memoryType: memory.memory_type,
      importance: memory.importance,
      degree: d,
      community,
      communityIndex: cIndex,
      colour: communityColour(community, cIndex),
      radius: nodeRadius({ degree: d, importance: memory.importance }),
      x: center.x + Math.cos(angle) * localRadius + jitter(1.61),
      y: center.y + Math.sin(angle * 0.7) * localRadius + jitter(2.31),
      z: center.z + Math.sin(angle) * localRadius + jitter(0.93),
    };
  });

  const nodeById = new Map(nodes.map((node) => [node.id, node]));
  links = [...typedLinks, ...tagLinks].map((link) => {
    const srcId = typeof link.source === 'number' ? link.source : link.source.id;
    const dstId = typeof link.target === 'number' ? link.target : link.target.id;
    const src = nodeById.get(srcId);
    const dst = nodeById.get(dstId);
    return {
      ...link,
      crossCommunity: Boolean(src && dst && src.community !== dst.community),
      colour: src && dst && src.community !== dst.community ? '#fbbf24' : link.colour,
    };
  });

  sim?.stop();
  // d3-force-3d accepts numDimensions as a constructor arg (the @types
  // declarations only describe the 2D overload).
  // 2.5D disc layout: clusters spread across a flat XZ disc, slab force
  // keeps everything close to y=0, collision keeps nodes from overlapping,
  // cluster force pulls each node toward its community's disc position.
  discRadius = 240 + Math.min(360, Math.sqrt(nodes.length) * 18);
  sim = (forceSimulation as unknown as (n: GraphNode[], dims: number) => ReturnType<typeof forceSimulation<GraphNode>>)(nodes, 3)
    .force(
      'link',
      forceLink<GraphNode, GraphLink>(links)
        .id((node) => node.id)
        .distance((link) => link.crossCommunity ? 130 : 32)
        .strength((link) => link.crossCommunity ? 0.015 : Math.min(0.32, 0.12 + link.confidence * 0.1)),
    )
    .force('charge', (forceManyBody<GraphNode>()
      .strength((node) => -12 - node.radius * 1.8) as unknown as { distanceMax: (n: number) => unknown })
      .distanceMax(80) as Force<GraphNode>)
    .force('collide', forceCollide<GraphNode>()
      .radius((node) => node.radius + 2)
      .strength(0.85)
      .iterations(2) as Force<GraphNode>)
    .force('cluster', createClusterForce(communities.length))
    .force('slab', createSlabForce(communities.length))
    .alphaDecay(0.028)
    .velocityDecay(0.74)
    .on('tick', null);

  sim.tick(focusMode.value === 'overview' ? 140 : 220);
  rebuildGraphics();
  rebuildUniverseGraphics();
  applyViewMode();
  graphVersion.value++;
  updateHighlighting();
  window.setTimeout(fitToView, 80);
}

// ── Universe mode (multiple planets in a top-down 2.5D sky) ────────────────
/**
 * Group memories by `memory_type` and lay each group out as a planet on a
 * wide ring at y≈0 (the universe plane). The user sees these as glowing
 * billboards from a top-down camera; clicking one zooms in.
 */
function buildPlanets(allMemories: MemoryEntry[]) {
  planetByKey.clear();
  const grouped = new Map<string, number[]>();
  for (const memory of allMemories) {
    const key = (memory.memory_type || 'memory').toLowerCase();
    const bucket = grouped.get(key);
    if (bucket) bucket.push(memory.id);
    else grouped.set(key, [memory.id]);
  }
  const keys = [...grouped.keys()].sort();
  const total = Math.max(1, keys.length);
  // Universe ring radius scales gently with planet count so dense memory
  // sets do not visually crowd each planet's billboard.
  const ringRadius = 360 + Math.min(360, total * 28);
  planets = keys.map((key, index) => {
    const ids = grouped.get(key)!;
    const angle = (index / total) * Math.PI * 2;
    return {
      key,
      center: new Vector3(Math.cos(angle) * ringRadius, 0, Math.sin(angle) * ringRadius),
      colour: communityColour(key, index),
      count: ids.length,
      nodeIds: new Set(ids),
      // Visual radius (in scene units) — logarithmic so a 5-node planet and
      // a 500-node planet are both readable.
      radius: 38 + Math.min(70, Math.log2(1 + ids.length) * 10),
    } satisfies Planet;
  });
  for (const planet of planets) planetByKey.set(planet.key, planet);
}

function rebuildUniverseGraphics() {
  if (!planetGroup) return;
  if (planetPoints) {
    planetGroup.remove(planetPoints);
    planetPoints.geometry.dispose();
    planetPoints = null;
  }
  if (planetGlow) {
    planetGroup.remove(planetGlow);
    planetGlow.geometry.dispose();
    planetGlow = null;
  }
  if (planetArcs) {
    planetGroup.remove(planetArcs);
    planetArcs.geometry.dispose();
    (planetArcs.material as ShaderMaterial).dispose?.();
    planetArcs = null;
  }
  if (planets.length === 0) return;

  const positions = new Float32Array(planets.length * 3);
  const colors = new Float32Array(planets.length * 3);
  const coreSizes = new Float32Array(planets.length);
  const glowSizes = new Float32Array(planets.length);
  const emphasis = new Float32Array(planets.length);
  const tmp = new Color();
  for (let i = 0; i < planets.length; i++) {
    const planet = planets[i];
    positions[i * 3 + 0] = planet.center.x;
    positions[i * 3 + 1] = planet.center.y;
    positions[i * 3 + 2] = planet.center.z;
    tmp.set(planet.colour);
    colors[i * 3 + 0] = tmp.r;
    colors[i * 3 + 1] = tmp.g;
    colors[i * 3 + 2] = tmp.b;
    coreSizes[i] = planet.radius;
    glowSizes[i] = planet.radius * 3.2;
    emphasis[i] = 1.0;
  }

  const coreGeo = new BufferGeometry();
  coreGeo.setAttribute('position', new BufferAttribute(positions, 3));
  coreGeo.setAttribute('nodeColor', new BufferAttribute(colors, 3));
  coreGeo.setAttribute('size', new BufferAttribute(coreSizes, 1));
  coreGeo.setAttribute('emphasis', new BufferAttribute(emphasis, 1));
  planetPoints = new Points(coreGeo, makeNodeMaterialBase(spriteTexture!, NODE_FS));
  planetPoints.frustumCulled = false;
  planetGroup.add(planetPoints);

  const glowGeo = new BufferGeometry();
  glowGeo.setAttribute('position', new BufferAttribute(positions.slice(), 3));
  glowGeo.setAttribute('nodeColor', new BufferAttribute(colors.slice(), 3));
  glowGeo.setAttribute('size', new BufferAttribute(glowSizes, 1));
  glowGeo.setAttribute('emphasis', new BufferAttribute(emphasis.slice(), 1));
  planetGlow = new Points(glowGeo, makeNodeMaterialBase(spriteTexture!, GLOW_FS));
  planetGlow.frustumCulled = false;
  planetGroup.add(planetGlow);

  // ── Inter-planet arcs: faint quadratic curves between planet centers,
  // weighted by how many cross-type edges connect each pair.
  const pairCount = new Map<string, number>();
  const memoryToPlanet = new Map<number, string>();
  for (const planet of planets) {
    for (const id of planet.nodeIds) memoryToPlanet.set(id, planet.key);
  }
  for (const edge of props.edges) {
    const srcKey = memoryToPlanet.get(edge.src_id);
    const dstKey = memoryToPlanet.get(edge.dst_id);
    if (!srcKey || !dstKey || srcKey === dstKey) continue;
    const k = srcKey < dstKey ? `${srcKey}|${dstKey}` : `${dstKey}|${srcKey}`;
    pairCount.set(k, (pairCount.get(k) ?? 0) + 1);
  }

  const SEGMENTS = 24;
  const pairs = [...pairCount.entries()];
  if (pairs.length > 0) {
    // Slice each Bezier curve into short straight segments so we can reuse
    // the existing `buildEdgeLines` helper (it expects {src, dst, alpha}).
    type SegmentNode = { x: number; y: number; z: number; colour: string };
    const arcEdges: { src: SegmentNode; dst: SegmentNode; alpha: number }[] = [];
    const a = new Vector3();
    const b = new Vector3();
    const mid = new Vector3();
    for (const [pairKey, count] of pairs) {
      const [k1, k2] = pairKey.split('|');
      const p1 = planetByKey.get(k1);
      const p2 = planetByKey.get(k2);
      if (!p1 || !p2) continue;
      a.copy(p1.center);
      b.copy(p2.center);
      const dist = a.distanceTo(b);
      mid.copy(a).add(b).multiplyScalar(0.5);
      mid.y += Math.min(220, dist * 0.35);
      const alpha = Math.min(0.7, 0.18 + count / 12);
      const bezier = (t: number): SegmentNode => {
        const it = 1 - t;
        return {
          x: it * it * a.x + 2 * it * t * mid.x + t * t * b.x,
          y: it * it * a.y + 2 * it * t * mid.y + t * t * b.y,
          z: it * it * a.z + 2 * it * t * mid.z + t * t * b.z,
          colour: '#7c8ca8',
        };
      };
      for (let s = 0; s < SEGMENTS; s++) {
        const t0 = s / SEGMENTS;
        const t1 = (s + 1) / SEGMENTS;
        arcEdges.push({ src: bezier(t0), dst: bezier(t1), alpha });
      }
    }
    planetArcs = createEdgeLines(arcEdges);
    if (planetArcs) planetGroup.add(planetArcs);
  }
}

function applyViewMode() {
  const isUniverse = viewMode.value === 'universe';
  if (planetGroup) planetGroup.visible = isUniverse;
  if (worldGroup) {
    if (nodePoints) nodePoints.visible = !isUniverse;
    if (glowPoints) glowPoints.visible = !isUniverse;
    if (haloPoints) haloPoints.visible = !isUniverse;
    if (linesMesh) linesMesh.visible = !isUniverse;
  }
  // Universe = mostly top-down so the planet ring reads as a "fake 2.5D"
  // top-down map. Planet = closer-to-equator orbit so the cluster sphere
  // is readable.
  // Universe = top-down for a planet-ring read; planet = low-angle 2.5D.
  phi = isUniverse ? Math.PI / 5.5 : Math.PI / 2.15;
  updateCameraFromOrbit();
}

function enterPlanet(key: string) {
  if (!planetByKey.has(key)) return;
  activePlanetKey.value = key;
  viewMode.value = 'planet';
  buildGraph();
}

function exitPlanet() {
  if (viewMode.value === 'universe') return;
  activePlanetKey.value = null;
  viewMode.value = 'universe';
  buildGraph();
}

/**
 * Debug capture: snapshot the current 3D canvas to a PNG and trigger a
 * browser download. Used during visual-design iteration so the agent /
 * user can attach the file in chat without setting up CDP screenshotting.
 *
 * Filename: `terransoul-3d-<view>-<timestamp>.png`. Ctrl+Shift+S
 * (`onCaptureHotkey` listener installed in `onMounted`) does the same.
 */
async function captureScreenshot(): Promise<void> {
  const canvas = canvasRef.value;
  if (!canvas || !renderer || !scene || !camera) return;
  // Render once synchronously so the back buffer matches the latest
  // simulation tick — `preserveDrawingBuffer: true` keeps it readable
  // after this call returns.
  renderer.render(scene, camera);
  const blob: Blob | null = await new Promise(resolve => {
    canvas.toBlob(b => resolve(b), 'image/png');
  });
  if (!blob) return;
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
  a.href = url;
  a.download = `terransoul-3d-${viewMode.value}-${ts}.png`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  // Revoke after a tick so the browser has time to start the download.
  setTimeout(() => URL.revokeObjectURL(url), 1_000);
}

function onCaptureHotkey(ev: KeyboardEvent) {
  if (ev.ctrlKey && ev.shiftKey && (ev.key === 'S' || ev.key === 's')) {
    ev.preventDefault();
    void captureScreenshot();
  }
}

function rebuildGraphics() {
  if (!worldGroup) return;
  if (nodePoints) { worldGroup.remove(nodePoints); nodePoints.geometry.dispose(); nodePoints = null; }
  if (glowPoints) { worldGroup.remove(glowPoints); glowPoints.geometry.dispose(); glowPoints = null; }
  if (haloPoints) { worldGroup.remove(haloPoints); haloPoints.geometry.dispose(); haloPoints = null; }
  if (linesMesh) { worldGroup.remove(linesMesh); linesMesh.geometry.dispose(); (linesMesh.material as ShaderMaterial).dispose?.(); linesMesh = null; }

  if (nodes.length === 0) return;

  const positions = new Float32Array(nodes.length * 3);
  const colors = new Float32Array(nodes.length * 3);
  const sizes = new Float32Array(nodes.length);
  const glowSizes = new Float32Array(nodes.length);
  const emphasis = new Float32Array(nodes.length);
  const tmp = new Color();
  for (let i = 0; i < nodes.length; i++) {
    const n = nodes[i];
    positions[i * 3 + 0] = safeCoord(n.x);
    positions[i * 3 + 1] = safeCoord(n.y);
    positions[i * 3 + 2] = safeCoord(n.z);
    tmp.set(n.colour);
    colors[i * 3 + 0] = tmp.r;
    colors[i * 3 + 1] = tmp.g;
    colors[i * 3 + 2] = tmp.b;
    sizes[i] = n.radius * 1.4;
    glowSizes[i] = n.radius * 3.6;
    emphasis[i] = 1.0;
  }

  const coreGeo = new BufferGeometry();
  coreGeo.setAttribute('position', new BufferAttribute(positions, 3));
  coreGeo.setAttribute('nodeColor', new BufferAttribute(colors, 3));
  coreGeo.setAttribute('size', new BufferAttribute(sizes, 1));
  coreGeo.setAttribute('emphasis', new BufferAttribute(emphasis, 1));
  nodeMaterial = makeNodeMaterialBase(spriteTexture!, NODE_FS);
  nodePoints = new Points(coreGeo, nodeMaterial);
  nodePoints.frustumCulled = false;
  worldGroup.add(nodePoints);

  const glowGeo = new BufferGeometry();
  glowGeo.setAttribute('position', new BufferAttribute(positions.slice(), 3));
  glowGeo.setAttribute('nodeColor', new BufferAttribute(colors.slice(), 3));
  glowGeo.setAttribute('size', new BufferAttribute(glowSizes, 1));
  glowGeo.setAttribute('emphasis', new BufferAttribute(emphasis.slice(), 1));
  glowMaterial = makeNodeMaterialBase(spriteTexture!, GLOW_FS);
  glowPoints = new Points(glowGeo, glowMaterial);
  glowPoints.frustumCulled = false;
  worldGroup.add(glowPoints);

  const haloPos = new Float32Array(3);
  const haloCol = new Float32Array([1, 1, 1]);
  const haloSize = new Float32Array([0]);
  const haloEmph = new Float32Array([0]);
  const haloGeo = new BufferGeometry();
  haloGeo.setAttribute('position', new BufferAttribute(haloPos, 3));
  haloGeo.setAttribute('nodeColor', new BufferAttribute(haloCol, 3));
  haloGeo.setAttribute('size', new BufferAttribute(haloSize, 1));
  haloGeo.setAttribute('emphasis', new BufferAttribute(haloEmph, 1));
  haloMaterial = makeNodeMaterialBase(spriteTexture!, HALO_FS);
  haloPoints = new Points(haloGeo, haloMaterial);
  haloPoints.frustumCulled = false;
  worldGroup.add(haloPoints);

  buildEdges();
}

function buildEdges() {
  if (!worldGroup) return;
  if (linesMesh) { worldGroup.remove(linesMesh); linesMesh.geometry.dispose(); (linesMesh.material as ShaderMaterial).dispose?.(); linesMesh = null; }
  if (links.length === 0) return;
  const isGapsFocus = focusMode.value === 'gaps';
  const edges = links.map((link) => {
    const src = link.source as GraphNode;
    const dst = link.target as GraphNode;
    const isGap = link.crossCommunity;
    const alpha = isGapsFocus && !isGap ? 0.18 : (isGap ? 0.95 : 0.55);
    return { src, dst, alpha };
  });
  linesMesh = createEdgeLines(edges);
  if (linesMesh) worldGroup.add(linesMesh);
}

let twinkleStars: TwinkleStarfieldResult | null = null;

function buildStarfield() {
  if (!scene || !spriteTexture) return;
  // Legacy single-layer starfield kept as fallback reference
  starsPoints = createStarfield(spriteTexture);
  starsPoints.visible = false;
  scene.add(starsPoints);
  // Two-layer twinkling starfield for galaxy aesthetic
  twinkleStars = buildTwinkleStarfield(spriteTexture);
  scene.add(twinkleStars.bg);
  scene.add(twinkleStars.fg);
}

function updateNodeAttributes() {
  if (!nodePoints || !glowPoints) return;
  const active = activeId.value;
  const neighbours = neighbourIds(active);
  const highlights = highlightedIds.value;
  const hasHighlight = highlights.size > 0;
  const corePos = nodePoints.geometry.getAttribute('position') as BufferAttribute;
  const coreCol = nodePoints.geometry.getAttribute('nodeColor') as BufferAttribute;
  const coreSize = nodePoints.geometry.getAttribute('size') as BufferAttribute;
  const coreEmph = nodePoints.geometry.getAttribute('emphasis') as BufferAttribute;
  const glowPos = glowPoints.geometry.getAttribute('position') as BufferAttribute;
  const glowCol = glowPoints.geometry.getAttribute('nodeColor') as BufferAttribute;
  const glowSize = glowPoints.geometry.getAttribute('size') as BufferAttribute;
  const glowEmph = glowPoints.geometry.getAttribute('emphasis') as BufferAttribute;
  const tmp = new Color();

  for (let i = 0; i < nodes.length; i++) {
    const n = nodes[i];
    corePos.setXYZ(i, safeCoord(n.x), safeCoord(n.y), safeCoord(n.z));
    glowPos.setXYZ(i, safeCoord(n.x), safeCoord(n.y), safeCoord(n.z));
    tmp.set(n.colour);
    const isHighlighted = highlights.has(n.id);
    // A node is "lit" if (a) nothing is hovered/highlighted at all, or
    // (b) it's the active hover/its neighbour, or (c) it's in the
    // persistent + matched highlight set.
    const focused = (!hasHighlight && active == null)
      || n.id === active
      || neighbours.has(n.id)
      || isHighlighted;
    let r = tmp.r, g = tmp.g, b = tmp.b;
    if (!focused) { r *= 0.18; g *= 0.18; b *= 0.22; }
    coreCol.setXYZ(i, r, g, b);
    glowCol.setXYZ(i, r, g, b);
    let sizeScale = 1;
    if (n.id === active) sizeScale = 1.7;
    else if (isHighlighted) sizeScale = 1.45;
    else if (neighbours.has(n.id)) sizeScale = 1.25;
    coreSize.setX(i, n.radius * sizeScale);
    glowSize.setX(i, n.radius * 4.2 * sizeScale);
    let e = focused ? 1 : 0.35;
    if (n.id === active) e = 1.6;
    else if (isHighlighted) e = 1.35;
    else if (neighbours.has(n.id)) e = 1.15;
    coreEmph.setX(i, e);
    glowEmph.setX(i, e);
  }
  corePos.needsUpdate = true; coreCol.needsUpdate = true; coreSize.needsUpdate = true; coreEmph.needsUpdate = true;
  glowPos.needsUpdate = true; glowCol.needsUpdate = true; glowSize.needsUpdate = true; glowEmph.needsUpdate = true;

  // Halo for selected
  if (haloPoints) {
    const hp = haloPoints.geometry.getAttribute('position') as BufferAttribute;
    const hc = haloPoints.geometry.getAttribute('nodeColor') as BufferAttribute;
    const hs = haloPoints.geometry.getAttribute('size') as BufferAttribute;
    const he = haloPoints.geometry.getAttribute('emphasis') as BufferAttribute;
    const a = activeNode.value;
    if (a) {
      hp.setXYZ(0, safeCoord(a.x), safeCoord(a.y), safeCoord(a.z));
      tmp.set(a.colour);
      hc.setXYZ(0, tmp.r, tmp.g, tmp.b);
      hs.setX(0, a.radius * 10);
      he.setX(0, 1);
    } else {
      hs.setX(0, 0);
      he.setX(0, 0);
    }
    hp.needsUpdate = true; hc.needsUpdate = true; hs.needsUpdate = true; he.needsUpdate = true;
  }
}

function updateEdgePositions() {
  if (!linesMesh || links.length === 0) return;
  const posAttr = linesMesh.geometry.getAttribute('position') as BufferAttribute;
  let idx = 0;
  for (const link of links) {
    const src = link.source as GraphNode;
    const dst = link.target as GraphNode;
    posAttr.setXYZ(idx++, safeCoord(src.x), safeCoord(src.y), safeCoord(src.z));
    posAttr.setXYZ(idx++, safeCoord(dst.x), safeCoord(dst.y), safeCoord(dst.z));
  }
  posAttr.needsUpdate = true;
}

function updatePositions() {
  updateNodeAttributes();
  updateEdgePositions();
}

function neighbourIds(id: number | null): Set<number> {
  const out = new Set<number>();
  if (id == null) return out;
  for (const link of links) {
    const src = link.source as GraphNode;
    const dst = link.target as GraphNode;
    if (src.id === id) out.add(dst.id);
    if (dst.id === id) out.add(src.id);
  }
  return out;
}

function updateHighlighting() {
  updateNodeAttributes();
  buildEdges();
}

function animate(now?: number) {
  if (disposed || !renderer || !scene || !camera) return;
  rafId = requestAnimationFrame(animate);
  const t = now ?? performance.now();
  const dt = lastFrame === 0 ? 16 : Math.min(64, t - lastFrame);
  lastFrame = t;
  if (sim && sim.alpha() > sim.alphaMin()) {
    sim.tick(1);
    updatePositions();
  }
  if (autoRotate && !isDragging && t - userInteractedAt > 2500 && worldGroup && viewMode.value === 'planet') {
    // Only spin the per-planet sphere of nodes; keep the universe layout
    // static so the planet ring stays oriented for a top-down read.
    worldGroup.rotation.y += dt * 0.00012;
  }
  // Drive twinkle animation
  if (twinkleStars) twinkleStars.update(t * 0.001);
  // Render through bloom composer when available, else direct
  if (composer) composer.render();
  else renderer.render(scene, camera);
}

function onResize() {
  const container = containerRef.value;
  if (!container || !camera || !renderer) return;
  const width = Math.max(1, container.clientWidth);
  const height = Math.max(1, container.clientHeight);
  camera.aspect = width / height;
  camera.updateProjectionMatrix();
  renderer.setSize(width, height, false);
  if (composer) composer.setSize(width, height);
}

function onWheel(event: WheelEvent) {
  radius = Math.max(120, Math.min(2000, radius + event.deltaY * 0.32));
  userInteractedAt = performance.now();
  updateCameraFromOrbit();
  // Auto-exit to the universe when the user zooms way out inside a planet.
  if (viewMode.value === 'planet' && radius > 1400) {
    exitPlanet();
  }
}

function onKeyDown(event: KeyboardEvent) {
  if (event.key !== 'Escape') return;
  // Skip when typing in inputs/textareas/contenteditable.
  const target = event.target as HTMLElement | null;
  const tag = target?.tagName ?? '';
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;
  if (target?.isContentEditable) return;

  // Esc has two effects in 3D, in priority order:
  //   1) exit the universe-mode planet view, if we're inside one
  //   2) otherwise clear the persistent selection
  if (viewMode.value === 'planet') {
    event.preventDefault();
    exitPlanet();
    return;
  }
  if (selectedIds.value.size > 0) {
    event.preventDefault();
    selectedIds.value = new Set();
    updateHighlighting();
  }
}

function onPointerDown(event: PointerEvent) {
  if (!containerRef.value) return;
  const rect = containerRef.value.getBoundingClientRect();
  const sx = event.clientX - rect.left;
  const sy = event.clientY - rect.top;

  // Shift-drag → screen-space lasso (box select). Shift-click on a node
  // toggles that single node in/out of the persistent selection.
  if (event.shiftKey && event.button === 0) {
    const hit = raycast(event);
    if (hit != null) {
      const next = new Set(selectedIds.value);
      if (next.has(hit)) next.delete(hit);
      else next.add(hit);
      selectedIds.value = next;
      updateHighlighting();
      return;
    }
    lassoActive = true;
    lassoMode = event.altKey ? 'remove' : 'add';
    lassoModeRef.value = lassoMode;
    lassoStartScreen = [sx, sy];
    lassoEndScreen = [sx, sy];
    lassoVisible.value = true;
    updateLassoStyle();
    containerRef.value.setPointerCapture(event.pointerId);
    return;
  }

  pendingHitId = event.button === 0 ? raycast(event) : null;
  // In universe view a left-click that does not land on a node may still
  // land on a planet billboard — capture it so onPointerUp can act on it
  // if the user didn't actually drag.
  pendingPlanetHit = (event.button === 0 && viewMode.value === 'universe')
    ? raycastPlanet(event)
    : null;
  isDragging = true;
  movedDuringPointer = false;
  prevX = event.clientX;
  prevY = event.clientY;
  containerRef.value.setPointerCapture(event.pointerId);
}

function onPointerMove(event: PointerEvent) {
  if (lassoActive && containerRef.value) {
    const rect = containerRef.value.getBoundingClientRect();
    lassoEndScreen = [event.clientX - rect.left, event.clientY - rect.top];
    updateLassoStyle();
    userInteractedAt = performance.now();
    return;
  }
  if (isDragging) {
    const dx = event.clientX - prevX;
    const dy = event.clientY - prevY;
    if (Math.hypot(dx, dy) > 1) movedDuringPointer = true;
    theta -= dx * 0.005;
    phi = Math.max(0.12, Math.min(Math.PI - 0.12, phi + dy * 0.005));
    prevX = event.clientX;
    prevY = event.clientY;
    userInteractedAt = performance.now();
    updateCameraFromOrbit();
    return;
  }

  const hit = raycast(event);
  hoveredId.value = hit;
  if (hit != null && containerRef.value) {
    const rect = containerRef.value.getBoundingClientRect();
    tooltipStyle.value = {
      left: `${event.clientX - rect.left + 14}px`,
      top: `${event.clientY - rect.top + 14}px`,
    };
  }
}

function onPointerUp(event: PointerEvent) {
  containerRef.value?.releasePointerCapture?.(event.pointerId);
  if (lassoActive) {
    commitLasso();
    lassoActive = false;
    lassoMode = 'add';
    lassoModeRef.value = 'add';
    lassoVisible.value = false;
    lassoStartScreen = null;
    lassoEndScreen = null;
    return;
  }
  if (pendingHitId != null && !movedDuringPointer) {
    emit('select', pendingHitId);
  } else if (
    pendingPlanetHit != null
    && !movedDuringPointer
    && viewMode.value === 'universe'
  ) {
    // Click on a planet billboard in universe mode → fly into that planet.
    enterPlanet(pendingPlanetHit);
  }
  pendingHitId = null;
  pendingPlanetHit = null;
  isDragging = false;
  movedDuringPointer = false;
}

function updateLassoStyle() {
  if (!lassoStartScreen || !lassoEndScreen) {
    lassoStyle.value = {};
    return;
  }
  const [x0, y0] = lassoStartScreen;
  const [x1, y1] = lassoEndScreen;
  lassoStyle.value = {
    left: `${Math.min(x0, x1)}px`,
    top: `${Math.min(y0, y1)}px`,
    width: `${Math.abs(x1 - x0)}px`,
    height: `${Math.abs(y1 - y0)}px`,
  };
}

function commitLasso() {
  if (!lassoStartScreen || !lassoEndScreen || !camera || !containerRef.value) return;
  const [x0, y0] = lassoStartScreen;
  const [x1, y1] = lassoEndScreen;
  if (Math.hypot(x1 - x0, y1 - y0) < 4) return;
  const minX = Math.min(x0, x1);
  const maxX = Math.max(x0, x1);
  const minY = Math.min(y0, y1);
  const maxY = Math.max(y0, y1);
  const rect = containerRef.value.getBoundingClientRect();
  const projected = new Vector3();
  // Account for the auto-rotate transform on worldGroup so projected points
  // match what's currently on screen.
  const groupRot = worldGroup ? worldGroup.rotation.y : 0;
  const cosR = Math.cos(groupRot);
  const sinR = Math.sin(groupRot);
  const next = new Set(selectedIds.value);
  for (const n of nodes) {
    const nx = safeCoord(n.x);
    const ny = safeCoord(n.y);
    const nz = safeCoord(n.z);
    // Rotate around Y to match worldGroup.rotation.y
    const wx = nx * cosR + nz * sinR;
    const wz = -nx * sinR + nz * cosR;
    projected.set(wx, ny, wz).project(camera);
    const sx = (projected.x * 0.5 + 0.5) * rect.width;
    const sy = (-projected.y * 0.5 + 0.5) * rect.height;
    if (sx >= minX && sx <= maxX && sy >= minY && sy <= maxY && projected.z < 1) {
      if (lassoMode === 'remove') next.delete(n.id);
      else next.add(n.id);
    }
  }
  selectedIds.value = next;
  updateHighlighting();
}

function onPointerLeave() {
  if (!isDragging) hoveredId.value = null;
}

const raycaster = new Raycaster();
const pointer = new Vector2();

function raycast(event: PointerEvent): number | null {
  if (!nodePoints || !camera || !containerRef.value) return null;
  const rect = containerRef.value.getBoundingClientRect();
  pointer.set(
    ((event.clientX - rect.left) / rect.width) * 2 - 1,
    -((event.clientY - rect.top) / rect.height) * 2 + 1,
  );
  raycaster.setFromCamera(pointer, camera);
  raycaster.params.Points = { threshold: 6 };
  const hits = raycaster.intersectObject(nodePoints, false);
  if (hits.length === 0) return null;
  const best = hits.reduce((a, b) => (b.distance < a.distance ? b : a));
  const idx = best.index;
  if (idx == null || idx >= nodes.length) return null;
  return nodes[idx].id;
}

function raycastPlanet(event: PointerEvent): string | null {
  if (!planetPoints || !camera || !containerRef.value || planets.length === 0) return null;
  const rect = containerRef.value.getBoundingClientRect();
  pointer.set(
    ((event.clientX - rect.left) / rect.width) * 2 - 1,
    -((event.clientY - rect.top) / rect.height) * 2 + 1,
  );
  raycaster.setFromCamera(pointer, camera);
  // Planet billboards are much larger than node points, so use a generous
  // threshold so a click anywhere on the glow halo still registers.
  raycaster.params.Points = { threshold: 30 };
  const hits = raycaster.intersectObject(planetPoints, false);
  if (hits.length === 0) return null;
  const best = hits.reduce((a, b) => (b.distance < a.distance ? b : a));
  const idx = best.index;
  if (idx == null || idx >= planets.length) return null;
  return planets[idx].key;
}

function fitToView() {
  // Universe mode: frame the full planet ring with a comfortable margin.
  if (viewMode.value === 'universe') {
    if (planets.length === 0) return;
    const ringExtent = Math.max(
      ...planets.map((p) => Math.hypot(p.center.x, p.center.z) + p.radius),
    );
    radius = Math.max(620, Math.min(1800, ringExtent * 2.2 + 160));
    updateCameraFromOrbit();
    return;
  }
  if (nodes.length === 0) return;
  // Planet mode: use the full 3D extent so a sphere of clusters frames
  // cleanly. 92nd percentile keeps a couple of stray outliers from blowing
  // the framing out.
  const distances: number[] = [];
  for (const node of nodes) {
    distances.push(Math.hypot(safeCoord(node.x), safeCoord(node.y), safeCoord(node.z)));
  }
  distances.sort((a, b) => a - b);
  const pct = distances[Math.floor(distances.length * 0.92)] || 200;
  radius = Math.max(380, Math.min(1100, pct * 2.0 + 120));
  updateCameraFromOrbit();
}

function setFocusMode(mode: FocusMode) {
  if (focusMode.value === mode) return;
  focusMode.value = mode;
  buildGraph();
}

const dataVersion = computed(() => {
  const memoryVersion = props.memories
    .map((memory) => `${memory.id}:${memory.memory_type}:${memory.tags}:${memory.content}:${memory.importance}`)
    .join('|');
  const edgeVersion = props.edges
    .map((edge) => `${edge.id}:${edge.src_id}:${edge.dst_id}:${edge.rel_type}:${edge.confidence}`)
    .join('|');
  return `${props.edgeMode}::${memoryVersion}::${edgeVersion}`;
});

onMounted(() => {
  const container = containerRef.value!;
  const canvas = canvasRef.value!;
  const width = container.clientWidth || 600;
  const height = container.clientHeight || 420;
  const background = new Color('#04030a');

  scene = new Scene();
  scene.fog = new FogExp2(0x05040a, 0.0012);
  camera = new PerspectiveCamera(55, width / height, 0.1, 4000);
  updateCameraFromOrbit();

  // `preserveDrawingBuffer: true` keeps the WebGL back buffer alive between
  // frames so the debug "📸 Capture" button (and Ctrl+Shift+S) can read a
  // PNG via `canvas.toBlob()` reliably without the racy render-then-read
  // pattern. The perf cost on this low-poly memory graph is negligible.
  renderer = new WebGLRenderer({ canvas, antialias: true, alpha: false, powerPreference: 'high-performance', preserveDrawingBuffer: true });
  renderer.setClearColor(background, 1);
  renderer.setSize(width, height, false);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));

  // Bloom postprocessing via UnrealBloomPass
  composer = new EffectComposer(renderer);
  composer.addPass(new RenderPass(scene, camera));
  const bloomPass = new UnrealBloomPass(new Vector2(width, height), 0.5, 0.85, 0.35);
  composer.addPass(bloomPass);

  spriteTexture = makeSpriteTexture();
  worldGroup = new Group();
  scene.add(worldGroup);
  planetGroup = new Group();
  scene.add(planetGroup);

  buildStarfield();
  buildGraph();
  animate();

  resizeObserver = new ResizeObserver(onResize);
  resizeObserver.observe(container);
  window.addEventListener('keydown', onKeyDown);
  window.addEventListener('keydown', onCaptureHotkey);
});

onBeforeUnmount(() => {
  disposed = true;
  cancelAnimationFrame(rafId);
  sim?.stop();
  resizeObserver?.disconnect();
  window.removeEventListener('keydown', onKeyDown);
  window.removeEventListener('keydown', onCaptureHotkey);
  if (worldGroup && scene) scene.remove(worldGroup);
  if (planetGroup && scene) scene.remove(planetGroup);
  if (nodePoints) { nodePoints.geometry.dispose(); }
  if (glowPoints) { glowPoints.geometry.dispose(); }
  if (haloPoints) { haloPoints.geometry.dispose(); }
  if (linesMesh) { linesMesh.geometry.dispose(); (linesMesh.material as ShaderMaterial).dispose?.(); }
  if (planetPoints) { planetPoints.geometry.dispose(); (planetPoints.material as ShaderMaterial).dispose?.(); }
  if (planetGlow) { planetGlow.geometry.dispose(); (planetGlow.material as ShaderMaterial).dispose?.(); }
  if (planetArcs) { planetArcs.geometry.dispose(); (planetArcs.material as ShaderMaterial).dispose?.(); }
  if (starsPoints) { starsPoints.geometry.dispose(); scene?.remove(starsPoints); }
  if (twinkleStars) {
    twinkleStars.bg.geometry.dispose(); (twinkleStars.bg.material as ShaderMaterial).dispose();
    twinkleStars.fg.geometry.dispose(); (twinkleStars.fg.material as ShaderMaterial).dispose();
    scene?.remove(twinkleStars.bg); scene?.remove(twinkleStars.fg);
    twinkleStars = null;
  }
  nodeMaterial?.dispose();
  glowMaterial?.dispose();
  haloMaterial?.dispose();
  spriteTexture?.dispose();
  composer?.dispose();
  renderer?.dispose();
});

watch(dataVersion, () => buildGraph());
watch(activeId, () => updateHighlighting());
watch([searchText, searchMode, highlightFilterActive], () => updateHighlighting());
watch(selectedIds, () => updateHighlighting(), { deep: true });
</script>

<style scoped src="./MemoryGraph3D.css"></style>
