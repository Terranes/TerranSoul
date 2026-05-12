<template>
  <div
    ref="containerRef"
    class="brain-graph-viewport"
    data-testid="brain-graph-viewport"
    @wheel.prevent="onWheel"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
  >
    <canvas ref="canvasRef" />
    <div
      v-if="memories.length === 0"
      class="bgv-empty"
      data-testid="bgv-empty"
    >
      No memory nodes yet.
    </div>
    <div
      v-if="hovered"
      class="bgv-tooltip"
      :style="tooltipStyle"
      data-testid="bgv-tooltip"
    >
      <strong>#{{ hovered.id }}</strong>
      <span class="bgv-tooltip-kind">{{ classifyMemoryKind(hovered) }}</span>
      <p>{{ truncate(hovered.content, 100) }}</p>
    </div>
    <div
      class="bgv-legend bgv-kind-legend"
      data-testid="bgv-legend"
    >
      <span
        v-for="(colour, kind) in COGNITIVE_COLOURS"
        :key="kind"
        class="bgv-legend-item"
      >
        <span
          class="bgv-legend-dot"
          :style="{ background: colour }"
        />{{ kind }}
      </span>
    </div>
    <div
      v-if="edgeLegend.length > 0"
      class="bgv-legend bgv-edge-legend"
      data-testid="bgv-edge-legend"
    >
      <span
        v-for="item in edgeLegend"
        :key="item.relType"
        class="bgv-legend-item"
      >
        <span
          class="bgv-legend-line"
          :style="{ background: item.colour }"
        />{{ item.relType }}
      </span>
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
  Float32BufferAttribute,
  LineSegments,
  LineBasicMaterial,
  SphereGeometry,
  InstancedMesh,
  Matrix4,
  Color,
  Raycaster,
  Vector2,
  MeshBasicMaterial,
  AmbientLight,
} from 'three';
import {
  forceSimulation,
  forceLink,
  forceManyBody,
  forceCenter,
  type SimulationNodeDatum,
  type SimulationLinkDatum,
} from 'd3-force-3d';
import type { MemoryEntry, MemoryEdge } from '../types';
import { classifyCognitiveKind, type CognitiveKind } from '../utils/cognitive-kind';

// ── Props & emits ────────────────────────────────────────────────────────────
const props = defineProps<{
  memories: MemoryEntry[];
  edges: MemoryEdge[];
}>();

const emit = defineEmits<{
  select: [id: number];
}>();

function classifyMemoryKind(m: MemoryEntry): CognitiveKind {
  return classifyCognitiveKind(m.memory_type, m.tags ?? '', m.content);
}

// ── Colours ──────────────────────────────────────────────────────────────────
const COGNITIVE_COLOURS: Record<CognitiveKind, string> = {
  episodic: 'var(--ts-warning)',
  semantic: 'var(--ts-accent-blue-hover)',
  procedural: 'var(--ts-success-dim)',
  judgment: 'var(--ts-accent-violet-hover)',
};

const EDGE_COLOURS = [
  'var(--ts-text-secondary)',
  'var(--ts-warning)',
  'var(--ts-info)',
  'var(--ts-error)',
  'var(--ts-success-dim)',
  'var(--ts-accent)',
  'var(--ts-success)',
  'var(--ts-accent-violet)',
] as const;

/** Hash rel_type → one of 8 edge colours. */
function relTypeColour(rel: string): string {
  let h = 0;
  for (let i = 0; i < rel.length; i++) h = ((h << 5) - h + rel.charCodeAt(i)) | 0;
  return EDGE_COLOURS[Math.abs(h) % EDGE_COLOURS.length];
}

function resolveCanvasColour(colour: string): string {
  const match = /^var\((--[^)]+)\)$/.exec(colour);
  if (!match || typeof window === 'undefined') return colour;
  return getComputedStyle(document.documentElement).getPropertyValue(match[1]).trim() || '#ffffff';
}

/** Safely read a d3-force coordinate — convert NaN / undefined / null to 0. */
function safeCoord(v: number | undefined | null): number {
  return (v != null && Number.isFinite(v)) ? v : 0;
}

function truncate(text: string, max: number): string {
  if (!text) return '';
  return text.length <= max ? text : text.slice(0, max - 1) + '…';
}

const edgeLegend = computed(() => {
  const relTypes = Array.from(new Set(props.edges.map((e) => e.rel_type).filter(Boolean))).sort();
  return relTypes.slice(0, 8).map((relType) => ({ relType, colour: relTypeColour(relType) }));
});

// ── Simulation node/link types ───────────────────────────────────────────────
interface GraphNode extends SimulationNodeDatum {
  id: number;
  kind: CognitiveKind;
}

interface GraphLink extends SimulationLinkDatum<GraphNode> {
  relType: string;
}

// ── Refs ─────────────────────────────────────────────────────────────────────
const containerRef = ref<HTMLDivElement | null>(null);
const canvasRef = ref<HTMLCanvasElement | null>(null);
const hovered = ref<MemoryEntry | null>(null);
const tooltipStyle = ref<CSSProperties>({});

let scene: Scene;
let camera: PerspectiveCamera;
let renderer: WebGLRenderer;
let sim: ReturnType<typeof forceSimulation<GraphNode>>;
let rafId = 0;
let nodes: GraphNode[] = [];
let links: GraphLink[] = [];
let mesh: InstancedMesh | null = null;
let linesMesh: LineSegments | null = null;
let disposed = false;

// ── Camera orbit state ───────────────────────────────────────────────────────
let isDragging = false;
let prevX = 0;
let prevY = 0;
let theta = 0;
let phi = Math.PI / 4;
let radius = 180;

function updateCameraFromOrbit() {
  camera.position.set(
    radius * Math.sin(phi) * Math.cos(theta),
    radius * Math.cos(phi),
    radius * Math.sin(phi) * Math.sin(theta),
  );
  camera.lookAt(0, 0, 0);
}

// ── Init ─────────────────────────────────────────────────────────────────────
onMounted(() => {
  const container = containerRef.value!;
  const canvas = canvasRef.value!;
  const w = container.clientWidth || 600;
  const h = container.clientHeight || 400;

  scene = new Scene();
  camera = new PerspectiveCamera(60, w / h, 0.1, 2000);
  updateCameraFromOrbit();

  renderer = new WebGLRenderer({ canvas, antialias: true, alpha: true });
  renderer.setSize(w, h);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

  scene.add(new AmbientLight(0xffffff, 1.2));

  buildGraph();
  animate();

  window.addEventListener('resize', onResize);
});

onBeforeUnmount(() => {
  disposed = true;
  cancelAnimationFrame(rafId);
  sim?.stop();
  renderer?.dispose();
  window.removeEventListener('resize', onResize);
});

// ── Build graph ──────────────────────────────────────────────────────────────
function buildGraph() {
  // Map memories → nodes (initialise x/y/z so d3-force-3d v3 has seed coords)
  const idSet = new Set(props.memories.map((m) => m.id));
  nodes = props.memories.map((m, i) => ({
    id: m.id,
    kind: classifyMemoryKind(m),
    x: Math.cos(i * 2.399) * 20,
    y: Math.sin(i * 1.618) * 20,
    z: Math.sin(i * 3.14159 * 0.618) * 20,
  }));

  // Map edges → links (only if both endpoints exist)
  links = props.edges
    .filter((e) => idSet.has(e.src_id) && idSet.has(e.dst_id))
    .map((e) => ({
      source: e.src_id,
      target: e.dst_id,
      relType: e.rel_type,
    }));

  // d3-force-3d simulation
  sim = forceSimulation<GraphNode>(nodes)
    .numDimensions(3)
    .force(
      'link',
      forceLink<GraphNode, GraphLink>(links)
        .id((d) => d.id)
        .distance(30),
    )
    .force('charge', forceManyBody<GraphNode>().strength(-40))
    .force('center', forceCenter<GraphNode>(0, 0, 0))
    .alphaDecay(0.02)
    .on('tick', null); // we read positions in RAF loop

  // Pre-warm 80 ticks for stable layout
  sim.tick(80);

  createMesh();
  createEdges();
}

// ── Three.js meshes ──────────────────────────────────────────────────────────
const NODE_RADIUS = 2;

function createMesh() {
  if (mesh) {
    scene.remove(mesh);
    mesh.dispose();
  }
  if (nodes.length === 0) {
    mesh = null;
    return;
  }
  const geo = new SphereGeometry(NODE_RADIUS, 12, 8);
  const mat = new MeshBasicMaterial({ color: 0xffffff });
  mesh = new InstancedMesh(geo, mat, nodes.length);
  const dummy = new Matrix4();
  const col = new Color();
  for (let i = 0; i < nodes.length; i++) {
    const n = nodes[i];
    dummy.makeTranslation(safeCoord(n.x), safeCoord(n.y), safeCoord(n.z));
    mesh.setMatrixAt(i, dummy);
    const c = COGNITIVE_COLOURS[n.kind] ?? COGNITIVE_COLOURS.semantic;
    mesh.setColorAt(i, col.set(resolveCanvasColour(c)));
  }
  mesh.instanceMatrix.needsUpdate = true;
  if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
  scene.add(mesh);
}

function createEdges() {
  if (linesMesh) {
    scene.remove(linesMesh);
    linesMesh.geometry.dispose();
    (linesMesh.material as LineBasicMaterial).dispose();
  }
  if (links.length === 0) return;

  const positions: number[] = [];
  const colors: number[] = [];
  const col = new Color();

  for (const l of links) {
    const s = l.source as GraphNode;
    const t = l.target as GraphNode;
    positions.push(safeCoord(s.x), safeCoord(s.y), safeCoord(s.z));
    positions.push(safeCoord(t.x), safeCoord(t.y), safeCoord(t.z));
    col.set(resolveCanvasColour(relTypeColour(l.relType)));
    colors.push(col.r, col.g, col.b);
    colors.push(col.r, col.g, col.b);
  }

  const geo = new BufferGeometry();
  geo.setAttribute('position', new Float32BufferAttribute(positions, 3));
  geo.setAttribute('color', new Float32BufferAttribute(colors, 3));
  const mat = new LineBasicMaterial({ vertexColors: true, opacity: 0.6, transparent: true });
  linesMesh = new LineSegments(geo, mat);
  scene.add(linesMesh);
}

function updatePositions() {
  if (!mesh) return;
  const dummy = new Matrix4();
  for (let i = 0; i < nodes.length; i++) {
    const n = nodes[i];
    dummy.makeTranslation(safeCoord(n.x), safeCoord(n.y), safeCoord(n.z));
    mesh.setMatrixAt(i, dummy);
  }
  mesh.instanceMatrix.needsUpdate = true;

  if (linesMesh && links.length > 0) {
    const posAttr = linesMesh.geometry.getAttribute('position');
    let idx = 0;
    for (const l of links) {
      const s = l.source as GraphNode;
      const t = l.target as GraphNode;
      posAttr.setXYZ(idx++, safeCoord(s.x), safeCoord(s.y), safeCoord(s.z));
      posAttr.setXYZ(idx++, safeCoord(t.x), safeCoord(t.y), safeCoord(t.z));
    }
    posAttr.needsUpdate = true;
  }
}

// ── RAF loop ─────────────────────────────────────────────────────────────────
function animate() {
  if (disposed) return;
  rafId = requestAnimationFrame(animate);
  if (sim.alpha() > sim.alphaMin()) {
    sim.tick(1);
    updatePositions();
  }
  renderer.render(scene, camera);
}

// ── Events ───────────────────────────────────────────────────────────────────
function onResize() {
  const c = containerRef.value;
  if (!c) return;
  const w = c.clientWidth;
  const h = c.clientHeight;
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
  renderer.setSize(w, h);
}

function onWheel(e: WheelEvent) {
  radius = Math.max(30, Math.min(600, radius + e.deltaY * 0.15));
  updateCameraFromOrbit();
}

function onPointerDown(e: PointerEvent) {
  // Right/middle drag for orbit, left-click for select
  if (e.button === 0) {
    // Check if hit a node first
    const hit = raycast(e);
    if (hit != null) {
      emit('select', hit);
      return;
    }
  }
  isDragging = true;
  prevX = e.clientX;
  prevY = e.clientY;
  (e.target as HTMLElement)?.setPointerCapture(e.pointerId);
}

function onPointerMove(e: PointerEvent) {
  if (isDragging) {
    const dx = e.clientX - prevX;
    const dy = e.clientY - prevY;
    theta -= dx * 0.005;
    phi = Math.max(0.1, Math.min(Math.PI - 0.1, phi + dy * 0.005));
    prevX = e.clientX;
    prevY = e.clientY;
    updateCameraFromOrbit();
  } else {
    // Hover tooltip
    const hit = raycast(e);
    if (hit != null) {
      hovered.value = props.memories.find((m) => m.id === hit) ?? null;
      tooltipStyle.value = {
        left: `${e.offsetX + 12}px`,
        top: `${e.offsetY + 12}px`,
      };
    } else {
      hovered.value = null;
    }
  }
}

function onPointerUp(e: PointerEvent) {
  isDragging = false;
  (e.target as HTMLElement)?.releasePointerCapture(e.pointerId);
}

// ── Raycasting ───────────────────────────────────────────────────────────────
const raycaster = new Raycaster();
raycaster.params.Points = { threshold: NODE_RADIUS * 2 };

function raycast(e: PointerEvent): number | null {
  if (!mesh || !containerRef.value) return null;
  const rect = containerRef.value.getBoundingClientRect();
  const ndc = new Vector2(
    ((e.clientX - rect.left) / rect.width) * 2 - 1,
    -((e.clientY - rect.top) / rect.height) * 2 + 1,
  );
  raycaster.setFromCamera(ndc, camera);
  const hits = raycaster.intersectObject(mesh);
  if (hits.length === 0) return null;
  const idx = hits[0].instanceId;
  if (idx == null || idx >= nodes.length) return null;
  return nodes[idx].id;
}

// ── Watch for data changes ───────────────────────────────────────────────────
const dataVersion = computed(() => {
  const memoryVersion = props.memories
    .map((m) => `${m.id}:${m.memory_type}:${m.tags}:${m.content}`)
    .join('|');
  const edgeVersion = props.edges
    .map((e) => `${e.id}:${e.src_id}:${e.dst_id}:${e.rel_type}`)
    .join('|');
  return `${memoryVersion}::${edgeVersion}`;
});
watch(dataVersion, () => {
  sim?.stop();
  buildGraph();
});
</script>

<style scoped>
.brain-graph-viewport {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 320px;
  overflow: hidden;
  cursor: grab;
  border-radius: 8px;
  background: var(--ts-bg-base);
}
.brain-graph-viewport:active {
  cursor: grabbing;
}
.brain-graph-viewport canvas {
  display: block;
  width: 100%;
  height: 100%;
}
.bgv-tooltip {
  position: absolute;
  pointer-events: none;
  background: var(--ts-bg-elevated);
  border: 1px solid var(--ts-border);
  border-radius: 6px;
  padding: 0.4rem 0.6rem;
  font-size: 0.78rem;
  color: var(--ts-text-primary);
  max-width: 240px;
  box-shadow: var(--ts-shadow-md);
  z-index: 10;
}
.bgv-tooltip strong {
  margin-right: 0.35rem;
}
.bgv-tooltip-kind {
  font-size: 0.7rem;
  text-transform: uppercase;
  color: var(--ts-text-muted);
}
.bgv-tooltip p {
  margin: 0.2rem 0 0;
  white-space: pre-wrap;
  line-height: 1.3;
}
.bgv-empty {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  color: var(--ts-text-muted);
  font-size: 0.85rem;
  pointer-events: none;
}
.bgv-legend {
  position: absolute;
  left: 0.5rem;
  display: flex;
  gap: 0.6rem;
  font-size: 0.7rem;
  color: var(--ts-text-muted);
  background: var(--ts-bg-surface);
  border-radius: 4px;
  padding: 0.25rem 0.5rem;
}
.bgv-kind-legend {
  bottom: 0.5rem;
}
.bgv-edge-legend {
  top: 0.5rem;
  max-width: calc(100% - 1rem);
  overflow: hidden;
}
.bgv-legend-item {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  white-space: nowrap;
}
.bgv-legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
.bgv-legend-line {
  width: 14px;
  height: 3px;
  border-radius: 999px;
}
</style>
