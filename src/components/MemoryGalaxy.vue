<template>
  <div
    ref="shellRef"
    class="kg-shell"
    :class="{ 'kg-mode-galaxy': view === 'galaxy', 'kg-mode-planet': view === 'planet' }"
    data-testid="memory-galaxy"
  >
    <div
      ref="stageRef"
      class="kg-stage"
    />

    <!-- HUD: brand mark (top-left) -->
    <div
      class="kg-brand"
      aria-hidden="true"
    >
      <span class="kg-brand-dot" />
      <span class="kg-brand-label">Memory · Galaxy</span>
    </div>

    <!-- HUD: status (top-right) -->
    <div
      class="kg-status"
      role="status"
      aria-live="polite"
    >
      <span class="kg-status-dot" />
      <span class="kg-status-label">{{ statusLabel }}</span>
    </div>

    <!-- HUD: crumb (top-center) -->
    <Transition name="kg-fade">
      <div
        v-if="view === 'planet' && activePlanet"
        class="kg-crumb"
      >
        <button
          type="button"
          class="kg-crumb-btn"
          aria-label="Back to galaxy"
          @click="exitPlanet()"
        >
          <span aria-hidden="true">←</span>
          <span>Galaxy</span>
        </button>
        <span
          class="kg-crumb-sep"
          aria-hidden="true"
        >/</span>
        <span class="kg-crumb-name">{{ activePlanet.name }}</span>
      </div>
    </Transition>

    <!-- HUD: invite when galaxy idle -->
    <Transition name="kg-fade">
      <div
        v-if="view === 'galaxy' && !hoveredPlanet"
        class="kg-invite"
      >
        Click a planet to dive in
      </div>
    </Transition>

    <!-- HUD: hint (bottom-left) -->
    <div
      class="kg-hint"
      aria-hidden="true"
    >
      <span><kbd>drag</kbd> orbit</span>
      <span><kbd>scroll</kbd> zoom</span>
      <span><kbd>click</kbd> select</span>
      <span><kbd>esc</kbd> back</span>
    </div>

    <!-- HUD: legend (bottom-right) — only shown when info panel is collapsed
         so the planet stats card doesn't compete with redundant legend chips. -->
    <Transition name="kg-fade">
      <div
        v-if="view === 'planet' && infoCollapsed"
        class="kg-legend"
      >
        <div class="kg-legend-row">
          <span
            class="kg-legend-swatch"
            style="background: var(--kg-doc)"
          />
          <span>Memory</span>
        </div>
        <div class="kg-legend-row">
          <span
            class="kg-legend-swatch"
            style="background: var(--kg-chunk)"
          />
          <span>Chunk</span>
        </div>
        <div class="kg-legend-row">
          <span
            class="kg-legend-swatch"
            style="background: var(--kg-entity)"
          />
          <span>Tag / Entity</span>
        </div>
      </div>
    </Transition>

    <!-- HUD: info panel (right) -->
    <Transition name="kg-slide-right">
      <aside
        v-if="view === 'planet' && activePlanet"
        class="kg-info"
        :class="{ 'kg-info--collapsed': infoCollapsed }"
        aria-label="Planet details"
      >
        <button
          type="button"
          class="kg-info-collapse"
          :aria-label="infoCollapsed ? 'Expand planet details' : 'Collapse planet details'"
          :title="infoCollapsed ? 'Expand' : 'Collapse'"
          @click="infoCollapsed = !infoCollapsed"
        >
          <span aria-hidden="true">{{ infoCollapsed ? '◀' : '▶' }}</span>
        </button>
        <template v-if="!infoCollapsed">
          <div class="kg-info-eyebrow">
            {{ activePlanet.eyebrow }}
          </div>
          <h3 class="kg-info-title">
            {{ activePlanet.name }}
          </h3>
          <p class="kg-info-sub">
            {{ activePlanet.blurb }}
          </p>
          <div class="kg-info-stats">
            <div class="kg-stat">
              <div class="kg-stat-num">
                {{ activePlanet.memories.length }}
              </div>
              <div class="kg-stat-label">
                memories
              </div>
            </div>
            <div class="kg-stat">
              <div class="kg-stat-num">
                {{ planetChunkCount }}
              </div>
              <div class="kg-stat-label">
                chunks
              </div>
            </div>
            <div class="kg-stat">
              <div class="kg-stat-num">
                {{ planetEntityCount }}
              </div>
              <div class="kg-stat-label">
                tags
              </div>
            </div>
          </div>

          <!-- Brain-design: tier breakdown (short / working / long) -->
          <div class="kg-info-section-label">
            Tiers
          </div>
          <div
            class="kg-tier-row"
            role="list"
          >
            <span
              v-for="t in TIER_KEYS"
              :key="t"
              class="kg-tier-chip"
              :class="`kg-tier-${t}`"
              role="listitem"
              :title="`${t}: ${planetTierCounts[t]} memories`"
            >
              <span class="kg-tier-name">{{ t }}</span>
              <span class="kg-tier-count">{{ planetTierCounts[t] }}</span>
            </span>
          </div>

          <!-- Brain-design: importance + decay summary -->
          <div class="kg-info-meta-row">
            <span class="kg-info-meta">
              <span class="kg-info-meta-label">avg importance</span>
              <span class="kg-info-meta-value">{{ planetAvgImportance.toFixed(1) }}</span>
            </span>
            <span class="kg-info-meta">
              <span class="kg-info-meta-label">avg decay</span>
              <span class="kg-info-meta-value">{{ planetAvgDecay.toFixed(2) }}</span>
            </span>
          </div>

          <!-- Brain-design: category breakdown -->
          <div
            v-if="planetCategoryCounts.length"
            class="kg-info-section-label"
          >
            Categories
          </div>
          <div
            v-if="planetCategoryCounts.length"
            class="kg-category-row"
            role="list"
          >
            <span
              v-for="c in planetCategoryCounts"
              :key="c.key"
              class="kg-cat-chip"
              role="listitem"
            >
              <span class="kg-cat-name">{{ c.key }}</span>
              <span class="kg-cat-count">{{ c.count }}</span>
            </span>
          </div>
          <div class="kg-info-docs-label">
            Top memories
          </div>
          <ul class="kg-info-docs">
            <li
              v-for="m in topMemoriesPreview"
              :key="m.id"
              :class="{ active: selectedSubId === m.id }"
              @click="onMemoryListClick(m.id)"
            >
              <span class="kg-info-doc-title">{{ memoryTitle(m) }}</span>
              <span class="kg-info-doc-meta">imp {{ m.importance }}</span>
            </li>
            <li
              v-if="!topMemoriesPreview.length"
              class="kg-info-empty"
            >
              No memories in this kind yet.
            </li>
          </ul>
        </template>
      </aside>
    </Transition>

    <!-- HUD: tooltip (follows cursor) -->
    <div
      v-show="tooltip.visible"
      class="kg-tooltip"
      :style="{ left: tooltip.x + 'px', top: tooltip.y + 'px' }"
      role="tooltip"
    >
      <div class="kg-tooltip-title">
        {{ tooltip.title }}
      </div>
      <div
        v-if="tooltip.sub"
        class="kg-tooltip-sub"
      >
        {{ tooltip.sub }}
      </div>
    </div>

    <!-- HUD: intro splash (until first render) -->
    <Transition name="kg-fade">
      <div
        v-if="!introDone"
        class="kg-intro"
        aria-hidden="true"
      >
        <div class="kg-intro-spinner" />
        <div class="kg-intro-label">
          Booting memory galaxy…
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, computed, watch } from 'vue';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';
import type { MemoryEdge, MemoryEntry } from '../types';
import { classifyCognitiveKind, type CognitiveKind } from '../utils/cognitive-kind';
import {
  mulberry32,
  makeGlowTexture,
  makeNebulaTexture,
  buildStars,
  makePlanetMaterial,
  easeInOutCubic,
  makeLabelSprite,
  PLANET_DEFS,
  type PlanetSpec,
} from './MemoryGalaxyHelpers';

// ── Props / emits ────────────────────────────────────────────────────────────
type SearchMode = 'contains' | 'starts' | 'ends';
interface SearchFields { label: boolean; tags: boolean; body: boolean; community: boolean }

const props = withDefaults(
  defineProps<{
    memories: MemoryEntry[];
    edges?: MemoryEdge[];
    /** Persistent multi-selection driven by the shared SelectedNodesPanel. */
    selectedIds?: Set<number>;
    edgeMode?: 'typed' | 'tag' | 'both';
    /** Live search query (shared with the 2D control panel). */
    searchText?: string;
    searchMode?: SearchMode;
    searchFields?: SearchFields;
    highlightFilterActive?: boolean;
    /** Filter toggles mirrored from the 2D `GraphControlPanel`. */
    showOrphans?: boolean;
    minDegree?: number;
    /** Display toggles. */
    showLabels?: boolean;
    showArrows?: boolean;
    nodeSizeMul?: number;
    linkWidthMul?: number;
    /** Counter the parent increments when the user clicks "Fit graph". */
    fitTrigger?: number;
  }>(),
  {
    edges: () => [],
    selectedIds: () => new Set<number>(),
    edgeMode: 'typed',
    searchText: '',
    searchMode: 'contains',
    searchFields: () => ({ label: true, tags: true, body: false, community: true }),
    highlightFilterActive: true,
    showOrphans: true,
    minDegree: 0,
    showLabels: false,
    showArrows: false,
    nodeSizeMul: 1,
    linkWidthMul: 1,
    fitTrigger: 0,
  },
);

const emit = defineEmits<{
  /** Single-click (non-shift): focus this memory. */
  (e: 'select', id: number, shift: boolean): void;
  /** Shift-click on a node: toggle in the persistent selection. */
  (e: 'toggle-selected', id: number): void;
  (e: 'keep-only-selection', ids: number[]): void;
}>();

// ── Refs ────────────────────────────────────────────────────────────────────
const shellRef = ref<HTMLElement | null>(null);
const stageRef = ref<HTMLElement | null>(null);

// ── View state ──────────────────────────────────────────────────────────────
type View = 'galaxy' | 'planet';
const view = ref<View>('galaxy');
const introDone = ref(false);
const hoveredPlanet = ref<PlanetSpec | null>(null);
// Right-side planet details panel can be collapsed to a thin tab so users can
// see the 3D scene unobstructed (especially useful when Semantic content
// scrolls long).
const infoCollapsed = ref(false);
const selectedSubId = ref<number | null>(null);
const tooltip = ref({ visible: false, x: 0, y: 0, title: '', sub: '' });

const statusLabel = computed(() => {
  const total = filteredMemories.value.length;
  const all = props.memories.length;
  const filteredSuffix = total !== all ? ` of ${all.toLocaleString()}` : '';
  if (view.value === 'galaxy') {
    return `${total.toLocaleString()}${filteredSuffix} memories · live`;
  }
  if (activePlanet.value) {
    return `${activePlanet.value.memories.length} in ${activePlanet.value.name.toLowerCase()}`;
  }
  return 'live';
});

const planets = ref<PlanetSpec[]>([]);
const activePlanet = ref<PlanetSpec | null>(null);

const planetChunkCount = computed(() => {
  if (!activePlanet.value) return 0;
  return activePlanet.value.memories.reduce((acc, m) => acc + Math.max(1, Math.ceil(m.token_count / 256)), 0);
});
const planetEntityCount = computed(() => {
  if (!activePlanet.value) return 0;
  const tags = new Set<string>();
  for (const m of activePlanet.value.memories) {
    for (const t of m.tags.split(/[,\s]+/)) {
      const tag = t.trim().toLowerCase();
      if (tag) tags.add(tag);
    }
  }
  return tags.size;
});
const topMemoriesPreview = computed(() => {
  if (!activePlanet.value) return [];
  return [...activePlanet.value.memories]
    .sort((a, b) => b.importance - a.importance || b.created_at - a.created_at)
    .slice(0, 12);
});

// ── Brain-advanced-design coverage: tier / category / decay breakdowns ────
// `brain-advanced-design.md` defines three lifecycle tiers (short / working /
// long) and eight categories (personal / relations / habits / domain /
// skills / emotional / world / meta). We surface both in the planet info
// panel so the user can see at a glance what's inside this cognitive kind.
const TIER_KEYS = ['short', 'working', 'long'] as const;
type TierKey = (typeof TIER_KEYS)[number];

const CATEGORY_KEYS = [
  'personal', 'relations', 'habits', 'domain',
  'skills', 'emotional', 'world', 'meta',
] as const;
type CategoryKey = (typeof CATEGORY_KEYS)[number];

function classifyCategory(m: MemoryEntry): CategoryKey | null {
  const tags = (m.tags || '').toLowerCase();
  for (const c of CATEGORY_KEYS) {
    // Tag-prefix convention from brain-advanced-design.md (e.g. `personal:*`,
    // `domain:law:*`). Also accept the bare tag as a sentinel.
    if (tags.includes(`${c}:`) || tags.split(/[\s,]+/).includes(c)) return c;
  }
  return null;
}

const planetTierCounts = computed<Record<TierKey, number>>(() => {
  const out: Record<TierKey, number> = { short: 0, working: 0, long: 0 };
  if (!activePlanet.value) return out;
  for (const m of activePlanet.value.memories) {
    const tier = (m.tier as TierKey) || 'long';
    if (tier in out) out[tier]++;
  }
  return out;
});

const planetCategoryCounts = computed<Array<{ key: CategoryKey; count: number }>>(() => {
  if (!activePlanet.value) return [];
  const acc = new Map<CategoryKey, number>();
  for (const m of activePlanet.value.memories) {
    const c = classifyCategory(m);
    if (c) acc.set(c, (acc.get(c) ?? 0) + 1);
  }
  return [...acc.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([key, count]) => ({ key, count }));
});

const planetAvgDecay = computed(() => {
  if (!activePlanet.value || activePlanet.value.memories.length === 0) return 0;
  let sum = 0;
  for (const m of activePlanet.value.memories) sum += m.decay_score ?? 1;
  return sum / activePlanet.value.memories.length;
});

const planetAvgImportance = computed(() => {
  if (!activePlanet.value || activePlanet.value.memories.length === 0) return 0;
  let sum = 0;
  for (const m of activePlanet.value.memories) sum += m.importance ?? 0;
  return sum / activePlanet.value.memories.length;
});

function memoryTitle(m: MemoryEntry): string {
  const head = m.content.split(/\r?\n/)[0] ?? '';
  return head.length > 64 ? head.slice(0, 61) + '…' : (head || `Memory #${m.id}`);
}

// ── Shared filter / selection plumbing ─────────────────────────────────────
// Mirror of `nodeMatchesQuery` from MemoryGraph.vue but operating on the raw
// MemoryEntry, so the same search box drives both renderers.
function matchesQuery(m: MemoryEntry): boolean {
  const q = props.searchText.trim();
  if (!q) return false;
  const needle = q.toLowerCase();
  const f = props.searchFields;
  const candidates: string[] = [];
  const label = (m.content.split(/\r?\n/)[0] ?? '').toLowerCase();
  if (f.label && label) candidates.push(label);
  if (f.tags && m.tags) candidates.push(m.tags.toLowerCase());
  if (f.body && m.content) candidates.push(m.content.toLowerCase());
  if (f.community && m.memory_type) candidates.push(m.memory_type.toLowerCase());
  if (candidates.length === 0) return false;
  if (props.searchMode === 'contains') return candidates.some((c) => c.includes(needle));
  if (props.searchMode === 'starts') return candidates.some((c) => c.startsWith(needle));
  return candidates.some((c) => c.endsWith(needle));
}

const degreeById = computed<Map<number, number>>(() => {
  const deg = new Map<number, number>();
  for (const e of props.edges) {
    deg.set(e.src_id, (deg.get(e.src_id) ?? 0) + 1);
    deg.set(e.dst_id, (deg.get(e.dst_id) ?? 0) + 1);
  }
  return deg;
});

/** Memories that pass the orphan + min-degree gates. */
const filteredMemories = computed<MemoryEntry[]>(() => {
  const deg = degreeById.value;
  return props.memories.filter((m) => {
    const d = deg.get(m.id) ?? 0;
    if (!props.showOrphans && d === 0) return false;
    if (d < props.minDegree) return false;
    return true;
  });
});

/** Search-matched memory ids (used for in-scene highlight dimming). */
const matchedMemoryIds = computed<Set<number>>(() => {
  if (!props.searchText.trim()) return new Set();
  const out = new Set<number>();
  for (const m of filteredMemories.value) {
    if (matchesQuery(m)) out.add(m.id);
  }
  return out;
});

/** "Lit" set = persistent selection ∪ live matches (when toggle on). */
const highlightedIds = computed<Set<number>>(() => {
  const out = new Set<number>(props.selectedIds);
  if (props.highlightFilterActive) {
    for (const id of matchedMemoryIds.value) out.add(id);
  }
  return out;
});

const isFilterActive = computed(
  () => props.searchText.trim().length > 0 && props.highlightFilterActive,
);

// ── Three.js state ──────────────────────────────────────────────────────────
let renderer: THREE.WebGLRenderer | null = null;
let scene: THREE.Scene | null = null;
let camera: THREE.PerspectiveCamera | null = null;
let controls: OrbitControls | null = null;
let composer: EffectComposer | null = null;
let bloomPass: UnrealBloomPass | null = null;
let raycaster: THREE.Raycaster | null = null;
const pointerNdc = new THREE.Vector2(0, 0);
const pointerScreen = { x: 0, y: 0 };
let pointerDownAt = { x: 0, y: 0, t: 0 };
let isDragging = false;

let galaxyGroup: THREE.Group | null = null;
let subgraphGroup: THREE.Group | null = null;
let coreSprite: THREE.Sprite | null = null;
let accretionRing: THREE.Mesh | null = null;

let rafId = 0;
let resizeObs: ResizeObserver | null = null;
let camTween: { start: number; dur: number; fromPos: THREE.Vector3; toPos: THREE.Vector3; fromTarget: THREE.Vector3; toTarget: THREE.Vector3 } | null = null;
const clock = new THREE.Clock();


// ── Galaxy build ────────────────────────────────────────────────────────────
function buildGalaxy() {
  if (!scene) return;
  // Clean previous
  if (galaxyGroup) {
    scene.remove(galaxyGroup);
    galaxyGroup.traverse((o) => {
      if ((o as THREE.Mesh).geometry) (o as THREE.Mesh).geometry.dispose();
      const mat = (o as THREE.Mesh).material;
      if (Array.isArray(mat)) mat.forEach((m) => m.dispose());
      else if (mat) (mat as THREE.Material).dispose();
    });
  }
  galaxyGroup = new THREE.Group();
  scene.add(galaxyGroup);

  // Bucket FILTERED memories by cognitive kind (search/orphans/min-degree).
  const buckets: Record<CognitiveKind, MemoryEntry[]> = {
    episodic: [], semantic: [], procedural: [], judgment: [],
  };
  for (const m of filteredMemories.value) {
    const k = classifyCognitiveKind(m.memory_type, m.tags, m.content);
    buckets[k].push(m);
  }

  // Size planets by count (log-scaled), assign orbits
  const maxCount = Math.max(1, ...Object.values(buckets).map((b) => b.length));
  const specs: PlanetSpec[] = PLANET_DEFS.map((def, i) => {
    const mems = buckets[def.key];
    const scale = Math.log10(mems.length + 1) / Math.log10(maxCount + 1);
    const r = 0.85 + scale * 1.4;
    return {
      ...def,
      memories: mems,
      planetRadius: r,
      orbitRadius: 7 + i * 3.4,
      orbitSpeed: 0.06 + (PLANET_DEFS.length - i) * 0.018,
    };
  });
  planets.value = specs;

  // Core (galactic center)
  const coreGeo = new THREE.SphereGeometry(0.28, 32, 32);
  const coreMat = new THREE.MeshBasicMaterial({ color: 0xfff7e6 });
  const coreSphere = new THREE.Mesh(coreGeo, coreMat);
  galaxyGroup.add(coreSphere);

  const coreGlowMat = new THREE.SpriteMaterial({
    map: makeGlowTexture(0xfff0c8),
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
  });
  coreSprite = new THREE.Sprite(coreGlowMat);
  coreSprite.scale.set(4.5, 4.5, 1);
  galaxyGroup.add(coreSprite);

  // Accretion ring
  const ringGeo = new THREE.RingGeometry(2.4, 2.46, 96);
  const ringMat = new THREE.MeshBasicMaterial({
    color: 0xb9ace8, side: THREE.DoubleSide, transparent: true, opacity: 0.55,
    depthWrite: false, blending: THREE.AdditiveBlending,
  });
  accretionRing = new THREE.Mesh(ringGeo, ringMat);
  accretionRing.rotation.x = Math.PI / 2;
  galaxyGroup.add(accretionRing);

  // Build each planet
  for (const spec of specs) {
    const pivot = new THREE.Group();
    pivot.rotation.y = Math.random() * Math.PI * 2;
    galaxyGroup.add(pivot);

    // Orbit ring
    const orbitGeo = new THREE.RingGeometry(spec.orbitRadius - 0.01, spec.orbitRadius + 0.01, 128);
    const orbitMat = new THREE.MeshBasicMaterial({
      color: spec.color, side: THREE.DoubleSide, transparent: true, opacity: 0.16,
      depthWrite: false, blending: THREE.AdditiveBlending,
    });
    const orbit = new THREE.Mesh(orbitGeo, orbitMat);
    orbit.rotation.x = Math.PI / 2;
    galaxyGroup.add(orbit);

    const group = new THREE.Group();
    group.position.set(spec.orbitRadius, 0, 0);
    pivot.add(group);

    const geo = new THREE.SphereGeometry(spec.planetRadius, 48, 48);
    const mat = makePlanetMaterial(spec.color, spec.glow);
    const mesh = new THREE.Mesh(geo, mat);
    mesh.userData.planetKey = spec.key;
    group.add(mesh);

    // Halo
    const haloMat = new THREE.SpriteMaterial({
      map: makeGlowTexture(spec.glow),
      transparent: true,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
    const halo = new THREE.Sprite(haloMat);
    halo.scale.setScalar(spec.planetRadius * 3.4);
    group.add(halo);

    spec.pivot = pivot;
    spec.group = group;
    spec.mesh = mesh;
    void halo;
  }
}

// ── Sub-graph build (real memory subgraph for selected kind) ───────────────
interface SubNode {
  id: string;          // 'mem-<id>' or 'tag-<name>'
  kind: 'memory' | 'tag';
  memoryId?: number;
  label: string;
  color: number;
  size: number;
  pos: THREE.Vector3;
  vel: THREE.Vector3;
  mesh?: THREE.Mesh;
}
interface SubLink {
  a: SubNode;
  b: SubNode;
  line?: THREE.Line;
  arrow?: THREE.Mesh;
}
const subNodes: SubNode[] = [];
const subLinks: SubLink[] = [];

const NODE_COLOR = { doc: 0xc8d9f5, chunk: 0x97baba, entity: 0xe8ce96 };
/** BRAIN-REPO-RAG-2a: warning hue for repo-sourced chunks in the galaxy. */
const REPO_NODE_COLOR = 0xd4a14a;

function buildSubgraph(spec: PlanetSpec) {
  if (!scene) return;
  // Clean
  if (subgraphGroup) {
    scene.remove(subgraphGroup);
    subgraphGroup.traverse((o) => {
      const m = o as THREE.Mesh;
      if (m.geometry) m.geometry.dispose();
      const mat = m.material;
      if (Array.isArray(mat)) mat.forEach((x) => x.dispose());
      else if (mat) (mat as THREE.Material).dispose();
    });
  }
  subgraphGroup = new THREE.Group();
  scene.add(subgraphGroup);
  subNodes.length = 0;
  subLinks.length = 0;

  const top = [...spec.memories]
    .sort((a, b) => b.importance - a.importance || b.created_at - a.created_at)
    .slice(0, 40);
  if (!top.length) return;

  const rng = mulberry32(0x51EED ^ spec.key.charCodeAt(0));
  const tagFrequency = new Map<string, number>();
  for (const m of top) {
    for (const raw of m.tags.split(/[,\s]+/)) {
      const t = raw.trim().toLowerCase();
      if (t.length < 2 || t.length > 24) continue;
      tagFrequency.set(t, (tagFrequency.get(t) ?? 0) + 1);
    }
  }
  const topTags = [...tagFrequency.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 12)
    .filter(([, n]) => n >= 2)
    .map(([t]) => t);

  // Memory nodes — distributed in a sphere via Fibonacci lattice so they
  // never spawn on top of each other (random sphere sampling caused visible
  // overlap when the planet has only a few memories).
  const memById = new Map<number, SubNode>();
  const N = top.length;
  const golden = Math.PI * (3 - Math.sqrt(5));
  for (let i = 0; i < N; i++) {
    const m = top[i];
    const y = N <= 1 ? 0 : 1 - (i / (N - 1)) * 2; // -1 .. 1
    const radiusXZ = Math.sqrt(1 - y * y);
    const theta = golden * i + rng() * 0.4;
    const baseR = 6 + (i % 3) * 1.2;
    const jitter = 0.6 + rng() * 0.4;
    const pos = new THREE.Vector3(
      Math.cos(theta) * radiusXZ * baseR * jitter,
      y * baseR * jitter,
      Math.sin(theta) * radiusXZ * baseR * jitter,
    );
    const node: SubNode = {
      id: `mem-${m.id}`,
      kind: 'memory',
      memoryId: m.id,
      label: memoryTitle(m),
      color: m.source_id ? REPO_NODE_COLOR : NODE_COLOR.doc,
      size: 0.16 + Math.min(0.28, m.importance * 0.04),
      pos,
      vel: new THREE.Vector3(),
    };
    subNodes.push(node);
    memById.set(m.id, node);
  }

  // Tag nodes
  const tagNodes = new Map<string, SubNode>();
  for (let i = 0; i < topTags.length; i++) {
    const t = topTags[i];
    const r = 9;
    const theta = (i / Math.max(1, topTags.length)) * Math.PI * 2;
    const pos = new THREE.Vector3(r * Math.cos(theta), (rng() - 0.5) * 4, r * Math.sin(theta));
    const node: SubNode = {
      id: `tag-${t}`,
      kind: 'tag',
      label: '#' + t,
      color: NODE_COLOR.entity,
      size: 0.22,
      pos,
      vel: new THREE.Vector3(),
    };
    subNodes.push(node);
    tagNodes.set(t, node);
  }

  // Memory ↔ tag edges
  for (const m of top) {
    const mn = memById.get(m.id);
    if (!mn) continue;
    for (const raw of m.tags.split(/[,\s]+/)) {
      const t = raw.trim().toLowerCase();
      const tn = tagNodes.get(t);
      if (tn) subLinks.push({ a: mn, b: tn });
    }
  }
  // KG edges between memories of this kind
  if (props.edges && props.edges.length) {
    for (const e of props.edges) {
      const a = memById.get(e.src_id);
      const b = memById.get(e.dst_id);
      if (a && b) subLinks.push({ a, b });
    }
  }

  // Build meshes
  for (const n of subNodes) {
    const geo = new THREE.SphereGeometry(n.size, 18, 18);
    const mat = new THREE.MeshBasicMaterial({ color: n.color, transparent: true, opacity: 0.92 });
    const mesh = new THREE.Mesh(geo, mat);
    mesh.position.copy(n.pos);
    mesh.userData.subNode = n;
    subgraphGroup.add(mesh);
    n.mesh = mesh;
  }
  const lineMat = new THREE.LineBasicMaterial({ color: 0x6f86b3, transparent: true, opacity: 0.22 });
  for (const l of subLinks) {
    const g = new THREE.BufferGeometry().setFromPoints([l.a.pos, l.b.pos]);
    const line = new THREE.Line(g, lineMat);
    subgraphGroup.add(line);
    l.line = line;
  }
}

// ── Force layout (lightweight Verlet, runs while planet view active) ──────
function stepSubgraphForces(dt: number) {
  if (!subgraphGroup || subNodes.length === 0) return;
  const REPULSION = 2.4;
  const LINK_K = 0.022;
  const LINK_REST = 2.2;
  const CENTER_K = 0.004;
  const DAMP = 0.86;
  const SEPARATION_PAD = 0.45; // extra clearance beyond mesh radii

  // O(n²) repulsion + hard minimum-distance separation so bubbles never visibly
  // overlap. The hard term fires only when nodes are within (rA+rB+pad).
  for (let i = 0; i < subNodes.length; i++) {
    const a = subNodes[i];
    for (let j = i + 1; j < subNodes.length; j++) {
      const b = subNodes[j];
      const dx = a.pos.x - b.pos.x;
      const dy = a.pos.y - b.pos.y;
      const dz = a.pos.z - b.pos.z;
      const d2 = dx * dx + dy * dy + dz * dz + 0.001;
      const d = Math.sqrt(d2);
      const minDist = a.size + b.size + SEPARATION_PAD;
      let f = REPULSION / d2;
      if (d < minDist) {
        // strong push proportional to overlap depth
        f += (minDist - d) * 0.6;
      }
      const fx = (dx / d) * f, fy = (dy / d) * f, fz = (dz / d) * f;
      a.vel.x += fx; a.vel.y += fy; a.vel.z += fz;
      b.vel.x -= fx; b.vel.y -= fy; b.vel.z -= fz;
    }
  }
  for (const l of subLinks) {
    const dx = l.b.pos.x - l.a.pos.x;
    const dy = l.b.pos.y - l.a.pos.y;
    const dz = l.b.pos.z - l.a.pos.z;
    const d = Math.sqrt(dx * dx + dy * dy + dz * dz) + 0.001;
    const f = (d - LINK_REST) * LINK_K;
    const fx = (dx / d) * f, fy = (dy / d) * f, fz = (dz / d) * f;
    l.a.vel.x += fx; l.a.vel.y += fy; l.a.vel.z += fz;
    l.b.vel.x -= fx; l.b.vel.y -= fy; l.b.vel.z -= fz;
  }
  for (const n of subNodes) {
    n.vel.x -= n.pos.x * CENTER_K;
    n.vel.y -= n.pos.y * CENTER_K;
    n.vel.z -= n.pos.z * CENTER_K;
    n.vel.multiplyScalar(DAMP);
    n.pos.x += n.vel.x * dt * 60;
    n.pos.y += n.vel.y * dt * 60;
    n.pos.z += n.vel.z * dt * 60;
    if (n.mesh) n.mesh.position.copy(n.pos);
  }
  for (const l of subLinks) {
    if (!l.line) continue;
    const arr = (l.line.geometry.attributes.position as THREE.BufferAttribute).array as Float32Array;
    arr[0] = l.a.pos.x; arr[1] = l.a.pos.y; arr[2] = l.a.pos.z;
    arr[3] = l.b.pos.x; arr[4] = l.b.pos.y; arr[5] = l.b.pos.z;
    (l.line.geometry.attributes.position as THREE.BufferAttribute).needsUpdate = true;
  }
}

// ── Camera tween ────────────────────────────────────────────────────────────
function tweenCameraTo(pos: THREE.Vector3, target: THREE.Vector3, dur = 1100) {
  if (!camera || !controls) return;
  camTween = {
    start: performance.now(),
    dur,
    fromPos: camera.position.clone(),
    toPos: pos.clone(),
    fromTarget: controls.target.clone(),
    toTarget: target.clone(),
  };
}

function enterPlanet(spec: PlanetSpec) {
  if (!camera || !controls) return;
  activePlanet.value = spec;
  view.value = 'planet';
  selectedSubId.value = null;
  // Hide galaxy meshes
  if (galaxyGroup) galaxyGroup.visible = false;
  buildSubgraph(spec);
  applyHighlight();
  rebuildLabels();
  rebuildArrows();
  tweenCameraTo(new THREE.Vector3(0, 8, 18), new THREE.Vector3(0, 0, 0), 1100);
}

function exitPlanet(skipCamera = false) {
  if (!camera || !controls) return;
  view.value = 'galaxy';
  activePlanet.value = null;
  selectedSubId.value = null;
  if (galaxyGroup) galaxyGroup.visible = true;
  if (subgraphGroup) {
    scene?.remove(subgraphGroup);
    subgraphGroup = null;
    subNodes.length = 0;
    subLinks.length = 0;
  }
  controls.autoRotate = true;
  if (!skipCamera) {
    tweenCameraTo(new THREE.Vector3(0, 16, 42), new THREE.Vector3(0, 0, 0), 1100);
  }
}

function onMemoryListClick(id: number) {
  selectedSubId.value = id;
  emit('select', id, false);
  // Pulse highlight: scale mesh briefly
  const node = subNodes.find((n) => n.memoryId === id);
  if (node?.mesh) {
    const m = node.mesh;
    const t0 = performance.now();
    const dur = 700;
    const pulse = () => {
      const k = (performance.now() - t0) / dur;
      if (k >= 1) { m.scale.setScalar(1); return; }
      const s = 1 + Math.sin(k * Math.PI) * 0.7;
      m.scale.setScalar(s);
      requestAnimationFrame(pulse);
    };
    pulse();
  }
}

// ── Pointer / interaction ───────────────────────────────────────────────────
function updatePointer(ev: PointerEvent) {
  if (!stageRef.value) return;
  const rect = stageRef.value.getBoundingClientRect();
  pointerScreen.x = ev.clientX - rect.left;
  pointerScreen.y = ev.clientY - rect.top;
  pointerNdc.x = (pointerScreen.x / rect.width) * 2 - 1;
  pointerNdc.y = -(pointerScreen.y / rect.height) * 2 + 1;
}

function onPointerDown(ev: PointerEvent) {
  pointerDownAt = { x: ev.clientX, y: ev.clientY, t: performance.now() };
  isDragging = false;
}
function onPointerMove(ev: PointerEvent) {
  updatePointer(ev);
  const dx = ev.clientX - pointerDownAt.x;
  const dy = ev.clientY - pointerDownAt.y;
  if (Math.sqrt(dx * dx + dy * dy) > 4) isDragging = true;
  // Hover ray
  hoveredPlanet.value = null;
  if (!raycaster || !camera) return;
  raycaster.setFromCamera(pointerNdc, camera);
  if (view.value === 'galaxy') {
    const meshes = planets.value.map((p) => p.mesh!).filter(Boolean);
    const hits = raycaster.intersectObjects(meshes, false);
    if (hits.length) {
      const key = hits[0].object.userData.planetKey;
      const p = planets.value.find((pp) => pp.key === key);
      if (p) {
        hoveredPlanet.value = p;
        tooltip.value = {
          visible: true,
          x: pointerScreen.x + 14,
          y: pointerScreen.y + 14,
          title: p.name,
          sub: `${p.memories.length} memories · ${p.eyebrow}`,
        };
        return;
      }
    }
    tooltip.value = { ...tooltip.value, visible: false };
  } else {
    if (!subgraphGroup) return;
    const meshes: THREE.Object3D[] = [];
    subgraphGroup.traverse((o) => { if ((o as THREE.Mesh).isMesh) meshes.push(o); });
    const hits = raycaster.intersectObjects(meshes, false);
    if (hits.length) {
      const node: SubNode = hits[0].object.userData.subNode;
      if (node) {
        tooltip.value = {
          visible: true,
          x: pointerScreen.x + 14,
          y: pointerScreen.y + 14,
          title: node.label,
          sub: node.kind === 'memory' ? `Memory #${node.memoryId}` : 'Tag',
        };
        return;
      }
    }
    tooltip.value = { ...tooltip.value, visible: false };
  }
}
function onPointerUp(ev: PointerEvent) {
  updatePointer(ev);
  const elapsed = performance.now() - pointerDownAt.t;
  if (isDragging || elapsed > 400) { isDragging = false; return; }
  if (!raycaster || !camera) return;
  raycaster.setFromCamera(pointerNdc, camera);
  if (view.value === 'galaxy') {
    const meshes = planets.value.map((p) => p.mesh!).filter(Boolean);
    const hits = raycaster.intersectObjects(meshes, false);
    if (hits.length) {
      const key = hits[0].object.userData.planetKey;
      const p = planets.value.find((pp) => pp.key === key);
      if (p) enterPlanet(p);
    }
  } else if (subgraphGroup) {
    const meshes: THREE.Object3D[] = [];
    subgraphGroup.traverse((o) => { if ((o as THREE.Mesh).isMesh) meshes.push(o); });
    const hits = raycaster.intersectObjects(meshes, false);
    if (hits.length) {
      const node: SubNode = hits[0].object.userData.subNode;
      if (node?.memoryId != null) {
        selectedSubId.value = node.memoryId;
        // Shift-click toggles persistent selection; plain click focuses.
        if (ev.shiftKey) {
          emit('toggle-selected', node.memoryId);
        } else {
          emit('select', node.memoryId, false);
        }
      }
    }
  }
}
function onPointerLeave() {
  tooltip.value = { ...tooltip.value, visible: false };
  hoveredPlanet.value = null;
}
function onKey(ev: KeyboardEvent) {
  if (ev.key === 'Escape' && view.value === 'planet') exitPlanet();
}

// ── Animate loop ────────────────────────────────────────────────────────────
function animate() {
  rafId = requestAnimationFrame(animate);
  const dt = Math.min(0.05, clock.getDelta());
  const elapsed = clock.elapsedTime;

  // Update planet pivots + shader time
  for (const p of planets.value) {
    if (p.pivot) p.pivot.rotation.y += p.orbitSpeed * dt;
    if (p.group) p.group.rotation.y += dt * 0.4;
    const u = (p.mesh?.material as THREE.ShaderMaterial | null)?.uniforms;
    if (u) u.uTime.value = elapsed;
  }
  if (accretionRing) accretionRing.rotation.z += dt * 0.4;
  if (coreSprite) {
    const s = 4.5 + Math.sin(elapsed * 2.0) * 0.25;
    coreSprite.scale.set(s, s, 1);
  }

  if (view.value === 'planet') {
    stepSubgraphForces(dt);
    updateArrowTransforms();
  }

  // Camera tween
  if (camTween && camera && controls) {
    const k = Math.min(1, (performance.now() - camTween.start) / camTween.dur);
    const e = easeInOutCubic(k);
    camera.position.lerpVectors(camTween.fromPos, camTween.toPos, e);
    controls.target.lerpVectors(camTween.fromTarget, camTween.toTarget, e);
    if (k >= 1) camTween = null;
  }

  // Camera tween
  if (camTween && camera && controls) {
    const k = Math.min(1, (performance.now() - camTween.start) / camTween.dur);
    const e = easeInOutCubic(k);
    camera.position.lerpVectors(camTween.fromPos, camTween.toPos, e);
    controls.target.lerpVectors(camTween.fromTarget, camTween.toTarget, e);
    if (k >= 1) camTween = null;
  }

  // Camera tween
  if (camTween && camera && controls) {
    const k = Math.min(1, (performance.now() - camTween.start) / camTween.dur);
    const e = easeInOutCubic(k);
    camera.position.lerpVectors(camTween.fromPos, camTween.toPos, e);
    controls.target.lerpVectors(camTween.fromTarget, camTween.toTarget, e);
    if (k >= 1) camTween = null;
  }

  controls?.update();
  composer?.render();

  if (!introDone.value && elapsed > 0.4) introDone.value = true;
}

// ── Resize ──────────────────────────────────────────────────────────────────
function handleResize() {
  if (!stageRef.value || !renderer || !camera || !composer) return;
  const w = stageRef.value.clientWidth;
  const h = stageRef.value.clientHeight;
  if (w < 4 || h < 4) return;
  renderer.setSize(w, h, false);
  composer.setSize(w, h);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}

// ── Mount / unmount ─────────────────────────────────────────────────────────
onMounted(() => {
  if (!stageRef.value) return;
  const stage = stageRef.value;
  const w = Math.max(4, stage.clientWidth);
  const h = Math.max(4, stage.clientHeight);

  renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: 'high-performance' });
  renderer.setPixelRatio(Math.min(2, window.devicePixelRatio));
  renderer.setSize(w, h, false);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.1;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  stage.appendChild(renderer.domElement);

  scene = new THREE.Scene();
  scene.fog = new THREE.FogExp2(0x05040a, 0.012);

  camera = new THREE.PerspectiveCamera(55, w / h, 0.1, 1000);
  camera.position.set(0, 16, 42);

  controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  controls.dampingFactor = 0.06;
  controls.autoRotate = true;
  controls.autoRotateSpeed = 0.35;
  controls.minDistance = 6;
  controls.maxDistance = 90;

  raycaster = new THREE.Raycaster();

  // Background stars
  const starsFar = buildStars(4500, 200, 0.6, 1.6, [0xffffff, 0xdfe7ff, 0xfff2e0]);
  scene.add(starsFar);
  const starsNear = buildStars(900, 90, 1.2, 3.5, [0xb9ace8, 0x97baba, 0xe8ce96, 0xa3ca8c, 0xffffff]);
  scene.add(starsNear);

  // Nebulas
  applyHighlight();
  rebuildLabels();
  const nebulaHues = [268, 188, 42, 90];
  for (let i = 0; i < nebulaHues.length; i++) {
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({
      map: makeNebulaTexture(nebulaHues[i]),
      transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, opacity: 0.55,
    }));
    sprite.scale.setScalar(70);
    sprite.position.set((i % 2 === 0 ? -1 : 1) * 60, (i < 2 ? 1 : -1) * 25, -70 - i * 10);
    scene.add(sprite);
  }

  buildGalaxy();

  // Post FX
  composer = new EffectComposer(renderer);
  composer.addPass(new RenderPass(scene, camera));
  bloomPass = new UnrealBloomPass(new THREE.Vector2(w, h), 0.55, 0.85, 0.35);
  composer.addPass(bloomPass);

  // Events
  const el = renderer.domElement;
  el.addEventListener('pointerdown', onPointerDown);
  el.addEventListener('pointermove', onPointerMove);
  el.addEventListener('pointerup', onPointerUp);
  el.addEventListener('pointerleave', onPointerLeave);
  window.addEventListener('keydown', onKey);

  resizeObs = new ResizeObserver(handleResize);
  resizeObs.observe(stage);

  clock.start();
  animate();
});

onBeforeUnmount(() => {
  cancelAnimationFrame(rafId);
  window.removeEventListener('keydown', onKey);
  resizeObs?.disconnect();
  if (renderer) {
    const el = renderer.domElement;
    el.removeEventListener('pointerdown', onPointerDown);
    el.removeEventListener('pointermove', onPointerMove);
    el.removeEventListener('pointerup', onPointerUp);
    el.removeEventListener('pointerleave', onPointerLeave);
    renderer.dispose();
    el.parentElement?.removeChild(el);
  }
  if (scene) {
    scene.traverse((o) => {
      const m = o as THREE.Mesh;
      if (m.geometry) m.geometry.dispose();
      const mat = m.material;
      if (Array.isArray(mat)) mat.forEach((x) => x.dispose());
      else if (mat) (mat as THREE.Material).dispose();
    });
  }
  renderer = null; scene = null; camera = null; controls = null;
  composer = null; bloomPass = null; raycaster = null;
  galaxyGroup = null; subgraphGroup = null;
  coreSprite = null; accretionRing = null;
});

// Rebuild when filters change (orphan / min-degree / search-driven filter
// surface). We rebuild the galaxy buckets and, when in drill-in mode, the
// subgraph so filtered memories don't appear as ghost nodes.
watch(
  () => [
    props.showOrphans, props.minDegree, props.searchText, props.searchMode,
    props.searchFields.label, props.searchFields.tags,
    props.searchFields.body, props.searchFields.community,
    props.highlightFilterActive, props.edges?.length ?? 0,
  ],
  () => {
    if (!scene) return;
    if (view.value === 'galaxy') {
      buildGalaxy();
    } else if (activePlanet.value) {
      buildSubgraph(activePlanet.value);
    }
  },
);

// Multi-selection visual update: highlighted memories pulse brighter; the
// rest dim when a filter is active. Cheap because we just tweak opacities.
watch(
  () => [highlightedIds.value, isFilterActive.value, props.nodeSizeMul, props.linkWidthMul],
  () => applyHighlight(),
  { deep: false },
);

function applyHighlight(): void {
  // Drill-in nodes
  if (subgraphGroup) {
    for (const n of subNodes) {
      if (!n.mesh) continue;
      const lit = n.memoryId != null && highlightedIds.value.has(n.memoryId);
      const dim = isFilterActive.value && !lit && n.kind === 'memory';
      const mat = n.mesh.material as THREE.MeshBasicMaterial;
      mat.opacity = dim ? 0.18 : 0.92;
      const base = n.size * props.nodeSizeMul;
      const scale = lit ? 1.45 : 1.0;
      n.mesh.scale.setScalar(scale * (base / n.size));
    }
    for (const l of subLinks) {
      if (!l.line) continue;
      const mat = l.line.material as THREE.LineBasicMaterial;
      mat.opacity = (0.22 * Math.max(0.4, props.linkWidthMul));
    }
  }
  // Galaxy planets: dim ones with no highlighted memories when filter is on.
  if (galaxyGroup) {
    for (const p of planets.value) {
      if (!p.mesh) continue;
      let dim = false;
      if (isFilterActive.value) {
        let any = false;
        for (const m of p.memories) {
          if (highlightedIds.value.has(m.id)) { any = true; break; }
        }
        dim = !any;
      }
      const mat = p.mesh.material as THREE.ShaderMaterial;
      if (mat?.uniforms?.uOpacity) mat.uniforms.uOpacity.value = dim ? 0.35 : 1.0;
      else (p.mesh as THREE.Mesh).visible = !dim || !isFilterActive.value;
    }
  }
}

// Toggle planet / node sprite labels when the display switch is flipped.
const labelGroup = new THREE.Group();
let labelGroupAttached = false;

watch(
  () => [props.showLabels, view.value, activePlanet.value?.key, planets.value.length, subNodes.length],
  () => rebuildLabels(),
);

function rebuildLabels(): void {
  if (!scene) return;
  if (!labelGroupAttached) {
    scene.add(labelGroup);
    labelGroupAttached = true;
  }
  // Clear
  while (labelGroup.children.length) {
    const s = labelGroup.children.pop() as THREE.Sprite;
    (s.material as THREE.SpriteMaterial).map?.dispose();
    (s.material as THREE.SpriteMaterial).dispose();
  }
  if (!props.showLabels) return;
  if (view.value === 'galaxy') {
    for (const p of planets.value) {
      if (!p.group) continue;
      const label = makeLabelSprite(`${p.name} \u00b7 ${p.memories.length}`, '#ffffff', 1.2);
      // Position above the planet (in pivot-local space). We attach to the
      // group so the label rides along the orbit.
      label.position.set(0, p.planetRadius + 0.9, 0);
      p.group.add(label);
    }
  } else {
    for (const n of subNodes) {
      if (n.kind !== 'memory' || !n.mesh) continue;
      if (n.memoryId == null) continue;
      const lit = highlightedIds.value.has(n.memoryId);
      // Only show labels on highlighted nodes to keep the scene readable.
      if (isFilterActive.value && !lit) continue;
      const label = makeLabelSprite(n.label.slice(0, 28), '#ffffff', 0.7);
      label.position.set(0, n.size + 0.3, 0);
      n.mesh.add(label);
    }
  }
}

// Arrow markers on KG edges when the user enables "Show arrows" — a small
// cone at the destination of each typed edge so directionality is visible
// (mirrors the 2D show-arrows toggle).
const arrowGroup = new THREE.Group();
let arrowGroupAttached = false;

watch(
  () => [props.showArrows, view.value, subLinks.length],
  () => rebuildArrows(),
);

function rebuildArrows(): void {
  if (!scene) return;
  if (!arrowGroupAttached) {
    scene.add(arrowGroup);
    arrowGroupAttached = true;
  }
  while (arrowGroup.children.length) {
    const obj = arrowGroup.children.pop()!;
    (obj as THREE.Mesh).geometry?.dispose();
    const mat = (obj as THREE.Mesh).material;
    if (Array.isArray(mat)) mat.forEach((m) => m.dispose());
    else if (mat) (mat as THREE.Material).dispose();
  }
  if (!props.showArrows || view.value !== 'planet') return;
  const coneGeo = new THREE.ConeGeometry(0.12, 0.32, 10);
  const coneMat = new THREE.MeshBasicMaterial({ color: 0xb9ace8, transparent: true, opacity: 0.7 });
  for (const l of subLinks) {
    if (l.b.kind !== 'memory') continue; // tag-edges aren't directional
    const cone = new THREE.Mesh(coneGeo, coneMat);
    // Position cone near `b` along (b - a) direction.
    arrowGroup.add(cone);
    l.arrow = cone;
  }
  updateArrowTransforms();
}

function updateArrowTransforms(): void {
  if (!props.showArrows) return;
  const tmp = new THREE.Vector3();
  const up = new THREE.Vector3(0, 1, 0);
  for (const l of subLinks) {
    if (!l.arrow) continue;
    tmp.subVectors(l.b.pos, l.a.pos);
    const len = tmp.length();
    if (len < 0.01) { l.arrow.visible = false; continue; }
    l.arrow.visible = true;
    // Place cone just before `b` along the line.
    const dir = tmp.clone().multiplyScalar(1 / len);
    l.arrow.position.copy(l.b.pos).addScaledVector(dir, -(l.b.size + 0.18));
    l.arrow.quaternion.setFromUnitVectors(up, dir);
  }
}

// Fit-to-view bridge — when the parent's `fitTrigger` increments, reset the
// camera to its mode-appropriate framing.
watch(() => props.fitTrigger, () => {
  if (view.value === 'galaxy') {
    tweenCameraTo(new THREE.Vector3(0, 16, 42), new THREE.Vector3(0, 0, 0), 900);
  } else {
    tweenCameraTo(new THREE.Vector3(0, 8, 18), new THREE.Vector3(0, 0, 0), 900);
  }
});

// Rebuild when memories list changes substantially.
watch(() => props.memories.length, () => {
  if (!scene || view.value !== 'galaxy') return;
  buildGalaxy();
  applyHighlight();
  rebuildLabels();
});
</script>

<style scoped src="./MemoryGalaxy.css"></style>
