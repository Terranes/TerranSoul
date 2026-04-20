<template>
  <Teleport to="body">
    <Transition name="constellation">
      <div
        v-if="visible"
        class="skill-constellation"
        role="dialog"
        aria-label="Skill Constellation"
        @keydown.esc="onEsc"
        tabindex="-1"
        ref="rootRef"
      >
        <!-- Star-field background -->
        <div class="sc-starfield" aria-hidden="true">
          <div class="sc-stars sc-stars-1" />
          <div class="sc-stars sc-stars-2" />
          <div class="sc-stars sc-stars-3" />
          <div class="sc-nebula" />
        </div>

        <!-- Top bar: breadcrumb + close -->
        <div class="sc-topbar">
          <div class="sc-breadcrumb" data-testid="constellation-breadcrumb">
            <button
              class="sc-crumb sc-crumb--root"
              :class="{ 'sc-crumb--active': !focusedClusterId && !selectedNode }"
              @click="goHome"
            >
              ✦ All Clusters
            </button>
            <template v-if="focusedClusterId">
              <span class="sc-crumb-sep">›</span>
              <button
                class="sc-crumb"
                :class="{ 'sc-crumb--active': focusedClusterId && !selectedNode }"
                @click="goCluster"
              >
                {{ focusedClusterMeta?.icon }} {{ focusedClusterMeta?.label }}
              </button>
            </template>
            <template v-if="selectedNode">
              <span class="sc-crumb-sep">›</span>
              <span class="sc-crumb sc-crumb--active">{{ selectedNode.icon }} {{ selectedNode.name }}</span>
            </template>
          </div>
          <div class="sc-topbar-actions">
            <button
              v-if="focusedClusterId || selectedNode"
              class="sc-back-btn"
              @click="goBack"
              data-testid="constellation-back"
              title="Back"
            >‹ Back</button>
            <button class="sc-close-btn" @click="$emit('close')" aria-label="Close" title="Close (Esc)">✕</button>
          </div>
        </div>

        <!-- Pannable / zoomable viewport -->
        <div
          class="sc-viewport"
          @mousedown="onDragStart"
          @wheel.prevent="onWheel"
          @touchstart.passive="onTouchStart"
          @touchmove="onTouchMove"
          @touchend="onTouchEnd"
          ref="viewportRef"
          data-testid="constellation-viewport"
        >
          <div
            class="sc-world"
            :style="worldTransform"
            data-testid="constellation-world"
          >
            <!-- Constellation lines between clusters -->
            <svg class="sc-bg-svg" :viewBox="`0 0 ${WORLD_W} ${WORLD_H}`" preserveAspectRatio="none" aria-hidden="true">
              <line
                v-for="(line, i) in interClusterLines"
                :key="'icl-' + i"
                :x1="line.x1" :y1="line.y1" :x2="line.x2" :y2="line.y2"
                class="sc-constellation-line"
              />
            </svg>

            <!-- Each cluster -->
            <div
              v-for="cluster in clusters"
              :key="cluster.id"
              class="sc-cluster"
              :class="[`sc-cluster--${cluster.id}`, { 'sc-cluster--focused': focusedClusterId === cluster.id }]"
              :style="{
                left: cluster.cx + 'px',
                top: cluster.cy + 'px',
              }"
            >
              <!-- Diamond border -->
              <div class="sc-cluster-diamond" />

              <!-- Connection lines between prereq nodes -->
              <svg class="sc-cluster-svg" :viewBox="`-${CLUSTER_R} -${CLUSTER_R} ${CLUSTER_R*2} ${CLUSTER_R*2}`" aria-hidden="true">
                <line
                  v-for="(edge, i) in cluster.edges"
                  :key="'edge-' + i"
                  :x1="edge.x1" :y1="edge.y1" :x2="edge.x2" :y2="edge.y2"
                  class="sc-edge"
                  :class="{ 'sc-edge--active': edge.active }"
                />
              </svg>

              <!-- Concentric rings (decorative) -->
              <div class="sc-ring sc-ring--inner" />
              <div class="sc-ring sc-ring--mid" />
              <div class="sc-ring sc-ring--outer" />

              <!-- Center emblem -->
              <button
                class="sc-cluster-emblem"
                :title="cluster.label"
                :data-testid="`cluster-emblem-${cluster.id}`"
                @click.stop="zoomToCluster(cluster.id)"
              >
                <span class="sc-cluster-icon">{{ cluster.icon }}</span>
                <span class="sc-cluster-label">{{ cluster.label }}</span>
                <span class="sc-cluster-cost">{{ cluster.activeCount }}/{{ cluster.nodes.length }} AP</span>
              </button>

              <!-- Skill nodes around rings -->
              <button
                v-for="placedNode in cluster.placedNodes"
                :key="placedNode.node.id"
                class="sc-node"
                :class="[
                  `sc-node--${placedNode.status}`,
                  `sc-node--ring-${placedNode.ring}`,
                ]"
                :style="{
                  left: placedNode.x + 'px',
                  top: placedNode.y + 'px',
                }"
                :title="placedNode.node.name"
                :data-testid="`skill-node-${placedNode.node.id}`"
                @click.stop="selectNode(placedNode.node.id)"
              >
                <span class="sc-node-gem">
                  <span class="sc-node-icon">{{ placedNode.node.icon }}</span>
                  <span v-if="placedNode.status === 'active'" class="sc-node-glow" />
                </span>
                <span class="sc-node-cost">{{ placedNode.node.name }}</span>
              </button>
            </div>
          </div>
        </div>

        <!-- Zoom controls -->
        <div class="sc-zoom-controls" aria-label="Zoom controls">
          <button class="sc-zoom-btn" @click="zoomIn" data-testid="zoom-in" title="Zoom in">＋</button>
          <button class="sc-zoom-btn" @click="zoomOut" data-testid="zoom-out" title="Zoom out">－</button>
          <button class="sc-zoom-btn sc-zoom-btn--reset" @click="resetView" data-testid="zoom-reset" title="Reset view">⟲</button>
        </div>

        <!-- Minimap -->
        <div class="sc-minimap" data-testid="constellation-minimap" aria-label="Cluster minimap">
          <svg :viewBox="`0 0 ${WORLD_W} ${WORLD_H}`" preserveAspectRatio="xMidYMid meet">
            <rect x="0" y="0" :width="WORLD_W" :height="WORLD_H" class="sc-minimap-bg" />
            <line
              v-for="(line, i) in interClusterLines"
              :key="'mm-icl-' + i"
              :x1="line.x1" :y1="line.y1" :x2="line.x2" :y2="line.y2"
              class="sc-minimap-line"
            />
            <g v-for="cluster in clusters" :key="'mm-' + cluster.id">
              <circle
                :cx="cluster.cx" :cy="cluster.cy" :r="CLUSTER_R"
                class="sc-minimap-cluster"
                :class="`sc-minimap-cluster--${cluster.id}`"
                :data-testid="`minimap-cluster-${cluster.id}`"
              />
              <circle
                v-for="(placedNode, j) in cluster.placedNodes"
                :key="'mm-n-' + j"
                :cx="cluster.cx + placedNode.x"
                :cy="cluster.cy + placedNode.y"
                r="14"
                class="sc-minimap-dot"
                :class="`sc-minimap-dot--${placedNode.status}`"
              />
            </g>
            <!-- Viewport rectangle -->
            <rect
              :x="viewportRect.x" :y="viewportRect.y"
              :width="viewportRect.w" :height="viewportRect.h"
              class="sc-minimap-viewport"
            />
          </svg>
        </div>

        <!-- Selected node detail overlay -->
        <Transition name="sc-detail">
          <div
            v-if="selectedNode"
            class="sc-detail"
            :class="`sc-detail--${selectedClusterId}`"
            data-testid="constellation-detail"
            @click.stop
          >
            <div class="sc-detail-top">
              <span class="sc-detail-gem">{{ selectedNode.icon }}</span>
              <div class="sc-detail-title-area">
                <div class="sc-detail-name">{{ selectedNode.name }}</div>
                <div class="sc-detail-tagline">{{ selectedNode.tagline }}</div>
              </div>
              <button class="sc-detail-close" @click="closeDetail" aria-label="Close detail">✕</button>
            </div>

            <p class="sc-detail-desc">{{ selectedNode.description }}</p>

            <!-- Quest steps -->
            <div v-if="selectedNode.questSteps.length" class="sc-detail-section">
              <div class="sc-detail-section-label">◆ Objectives</div>
              <div v-for="(step, i) in selectedNode.questSteps" :key="i" class="sc-step">
                <span class="sc-step-num">{{ stepNumeral(i) }}</span>
                <span class="sc-step-text">{{ step.label }}</span>
                <button
                  v-if="step.target"
                  class="sc-step-go"
                  @click="$emit('navigate', step.target!)"
                >▸</button>
              </div>
            </div>

            <!-- Rewards -->
            <div v-if="selectedNode.rewards.length" class="sc-detail-section">
              <div class="sc-detail-section-label">◆ Rewards</div>
              <div class="sc-reward-list">
                <span
                  v-for="(reward, i) in selectedNode.rewards"
                  :key="i"
                  class="sc-reward"
                >{{ selectedNode.rewardIcons[i] || '🎁' }} {{ reward }}</span>
              </div>
            </div>

            <!-- Prerequisites -->
            <div v-if="selectedNode.requires.length" class="sc-detail-section">
              <div class="sc-detail-section-label">◆ Prerequisites</div>
              <div class="sc-prereq-list">
                <span
                  v-for="reqId in selectedNode.requires"
                  :key="reqId"
                  class="sc-prereq"
                  :class="{ 'sc-prereq--met': skillTree.getSkillStatus(reqId) === 'active' }"
                >{{ getNodeIcon(reqId) }} {{ getNodeName(reqId) }} {{ skillTree.getSkillStatus(reqId) === 'active' ? '◆' : '◇' }}</span>
              </div>
            </div>

            <!-- Actions -->
            <div class="sc-detail-actions">
              <button
                v-if="!isPinned(selectedNode.id)"
                class="sc-btn sc-btn--secondary"
                @click="skillTree.pinQuest(selectedNode!.id)"
              >📌 Pin</button>
              <button
                v-else
                class="sc-btn sc-btn--secondary"
                @click="skillTree.unpinQuest(selectedNode!.id)"
              >📌 Unpin</button>
              <button
                v-if="selectedStatus !== 'locked'"
                class="sc-btn sc-btn--primary"
                @click="beginQuest(selectedNode!.id)"
                data-testid="constellation-begin"
              >⚔️ Begin Quest</button>
            </div>
          </div>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import { useSkillTreeStore } from '../stores/skill-tree';
import type { SkillNode, SkillTier } from '../stores/skill-tree';

const props = defineProps<{ visible: boolean }>();

const emit = defineEmits<{
  close: [];
  navigate: [target: string];
  begin: [questId: string];
}>();

const skillTree = useSkillTreeStore();

// ── World layout constants ───────────────────────────────────────────────
const WORLD_W = 1600;
const WORLD_H = 1200;
const CLUSTER_R = 240; // visual radius for outer ring
const RING_RADII: Record<SkillTier, number> = {
  foundation: 90,
  advanced: 155,
  ultimate: 220,
};

interface ClusterMeta {
  id: 'brain' | 'voice' | 'avatar' | 'social' | 'utility';
  label: string;
  icon: string;
  cx: number;
  cy: number;
}

const CLUSTER_META: ClusterMeta[] = [
  { id: 'brain',   label: 'Brain',   icon: '🧠', cx: 800,  cy: 250 },
  { id: 'voice',   label: 'Voice',   icon: '🗣️', cx: 1280, cy: 520 },
  { id: 'avatar',  label: 'Avatar',  icon: '✨', cx: 1080, cy: 980 },
  { id: 'social',  label: 'Social',  icon: '🔗', cx: 520,  cy: 980 },
  { id: 'utility', label: 'Utility', icon: '📀', cx: 320,  cy: 520 },
];

// ── Build cluster placements ─────────────────────────────────────────────
interface PlacedNode {
  node: SkillNode;
  /** Position relative to cluster center. */
  x: number;
  y: number;
  ring: SkillTier;
  status: 'locked' | 'available' | 'active';
}

interface ClusterEdge { x1: number; y1: number; x2: number; y2: number; active: boolean }

interface BuiltCluster extends ClusterMeta {
  nodes: SkillNode[];
  placedNodes: PlacedNode[];
  edges: ClusterEdge[];
  activeCount: number;
}

const clusters = computed<BuiltCluster[]>(() => {
  return CLUSTER_META.map(meta => {
    const clusterNodes = skillTree.nodes.filter(n => n.category === meta.id);
    const byTier: Record<SkillTier, SkillNode[]> = { foundation: [], advanced: [], ultimate: [] };
    for (const n of clusterNodes) byTier[n.tier].push(n);

    const placed: PlacedNode[] = [];
    for (const tier of ['foundation', 'advanced', 'ultimate'] as SkillTier[]) {
      const ring = byTier[tier];
      const r = RING_RADII[tier];
      const count = ring.length;
      // Spread nodes around the full circle. Offset by tier so adjacent rings stagger.
      const offset = tier === 'foundation' ? -Math.PI / 2 : tier === 'advanced' ? -Math.PI / 2 + Math.PI / Math.max(count, 6) : -Math.PI / 2 + Math.PI / Math.max(count, 8);
      for (let i = 0; i < count; i++) {
        const angle = offset + (2 * Math.PI * i) / Math.max(count, 1);
        placed.push({
          node: ring[i],
          x: Math.cos(angle) * r,
          y: Math.sin(angle) * r,
          ring: tier,
          status: skillTree.getSkillStatus(ring[i].id),
        });
      }
    }

    // Compute prereq edges (only when both endpoints inside this cluster).
    const edges: ClusterEdge[] = [];
    const lookup = new Map(placed.map(p => [p.node.id, p]));
    for (const p of placed) {
      for (const reqId of p.node.requires) {
        const parent = lookup.get(reqId);
        if (!parent) continue;
        edges.push({
          x1: parent.x, y1: parent.y, x2: p.x, y2: p.y,
          active: parent.status === 'active' && p.status === 'active',
        });
      }
    }

    const activeCount = placed.filter(p => p.status === 'active').length;
    return { ...meta, nodes: clusterNodes, placedNodes: placed, edges, activeCount };
  });
});

// Faint constellation lines between cluster centres (decorative).
const interClusterLines = computed(() => {
  const lines: { x1: number; y1: number; x2: number; y2: number }[] = [];
  for (let i = 0; i < CLUSTER_META.length; i++) {
    const a = CLUSTER_META[i];
    const b = CLUSTER_META[(i + 1) % CLUSTER_META.length];
    lines.push({ x1: a.cx, y1: a.cy, x2: b.cx, y2: b.cy });
  }
  return lines;
});

// ── Pan / zoom state ─────────────────────────────────────────────────────
const tx = ref(0);
const ty = ref(0);
const scale = ref(1);
const animating = ref(false);
const MIN_SCALE = 0.35;
const MAX_SCALE = 2.5;

const worldTransform = computed(() => ({
  width: WORLD_W + 'px',
  height: WORLD_H + 'px',
  transform: `translate(${tx.value}px, ${ty.value}px) scale(${scale.value})`,
  transition: animating.value ? 'transform 0.45s cubic-bezier(0.4, 0, 0.2, 1)' : 'none',
}));

// Approximate viewport rectangle inside world coords for the minimap.
const viewportRef = ref<HTMLElement | null>(null);
const viewportSize = ref({ w: 1280, h: 720 });
const viewportRect = computed(() => {
  const w = viewportSize.value.w / scale.value;
  const h = viewportSize.value.h / scale.value;
  const x = -tx.value / scale.value;
  const y = -ty.value / scale.value;
  return {
    x: Math.max(0, Math.min(WORLD_W - 1, x)),
    y: Math.max(0, Math.min(WORLD_H - 1, y)),
    w: Math.max(1, Math.min(WORLD_W, w)),
    h: Math.max(1, Math.min(WORLD_H, h)),
  };
});

function fitInitial() {
  const vw = viewportSize.value.w;
  const vh = viewportSize.value.h;
  const s = Math.min(vw / WORLD_W, vh / WORLD_H) * 0.95;
  scale.value = Math.max(MIN_SCALE, Math.min(MAX_SCALE, s));
  tx.value = (vw - WORLD_W * scale.value) / 2;
  ty.value = (vh - WORLD_H * scale.value) / 2;
}

function measureViewport() {
  const el = viewportRef.value;
  if (el) {
    const rect = el.getBoundingClientRect();
    viewportSize.value = { w: rect.width || 1280, h: rect.height || 720 };
  }
}

// ── Drag-to-pan ──────────────────────────────────────────────────────────
let dragLast: { x: number; y: number } | null = null;
function onDragStart(e: MouseEvent) {
  // Only drag when clicking on the viewport itself or sc-world (not on nodes/buttons).
  const target = e.target as HTMLElement;
  if (target.closest('button') || target.closest('.sc-detail')) return;
  dragLast = { x: e.clientX, y: e.clientY };
  animating.value = false;
  window.addEventListener('mousemove', onDragMove);
  window.addEventListener('mouseup', onDragEnd);
}
function onDragMove(e: MouseEvent) {
  if (!dragLast) return;
  const dx = e.clientX - dragLast.x;
  const dy = e.clientY - dragLast.y;
  tx.value += dx;
  ty.value += dy;
  dragLast = { x: e.clientX, y: e.clientY };
}
function onDragEnd() {
  dragLast = null;
  window.removeEventListener('mousemove', onDragMove);
  window.removeEventListener('mouseup', onDragEnd);
}

// ── Wheel zoom ───────────────────────────────────────────────────────────
function onWheel(e: WheelEvent) {
  const factor = e.deltaY > 0 ? 0.9 : 1.1;
  zoomBy(factor, e.clientX, e.clientY);
}

function zoomBy(factor: number, anchorClientX?: number, anchorClientY?: number) {
  const next = Math.max(MIN_SCALE, Math.min(MAX_SCALE, scale.value * factor));
  if (next === scale.value) return;
  // Anchor zoom around the cursor position so it feels natural.
  const el = viewportRef.value;
  if (el && anchorClientX !== undefined && anchorClientY !== undefined) {
    const rect = el.getBoundingClientRect();
    const ax = anchorClientX - rect.left;
    const ay = anchorClientY - rect.top;
    const wx = (ax - tx.value) / scale.value;
    const wy = (ay - ty.value) / scale.value;
    scale.value = next;
    tx.value = ax - wx * next;
    ty.value = ay - wy * next;
  } else {
    scale.value = next;
  }
  animating.value = false;
}

function zoomIn() { animating.value = true; zoomBy(1.2); }
function zoomOut() { animating.value = true; zoomBy(1 / 1.2); }
function resetView() {
  animating.value = true;
  fitInitial();
  focusedClusterId.value = null;
}

// ── Touch (pan + pinch) ──────────────────────────────────────────────────
let touchState: { x: number; y: number; dist: number; mode: 'pan' | 'pinch' } | null = null;

function onTouchStart(e: TouchEvent) {
  const target = e.target as HTMLElement;
  if (target.closest('button') || target.closest('.sc-detail')) return;
  if (e.touches.length === 1) {
    touchState = { x: e.touches[0].clientX, y: e.touches[0].clientY, dist: 0, mode: 'pan' };
  } else if (e.touches.length === 2) {
    const dx = e.touches[0].clientX - e.touches[1].clientX;
    const dy = e.touches[0].clientY - e.touches[1].clientY;
    touchState = {
      x: (e.touches[0].clientX + e.touches[1].clientX) / 2,
      y: (e.touches[0].clientY + e.touches[1].clientY) / 2,
      dist: Math.hypot(dx, dy),
      mode: 'pinch',
    };
  }
  animating.value = false;
}
function onTouchMove(e: TouchEvent) {
  if (!touchState) return;
  if (touchState.mode === 'pan' && e.touches.length === 1) {
    e.preventDefault();
    tx.value += e.touches[0].clientX - touchState.x;
    ty.value += e.touches[0].clientY - touchState.y;
    touchState.x = e.touches[0].clientX;
    touchState.y = e.touches[0].clientY;
  } else if (touchState.mode === 'pinch' && e.touches.length === 2) {
    e.preventDefault();
    const dx = e.touches[0].clientX - e.touches[1].clientX;
    const dy = e.touches[0].clientY - e.touches[1].clientY;
    const dist = Math.hypot(dx, dy);
    if (touchState.dist > 0) {
      const factor = dist / touchState.dist;
      zoomBy(factor, touchState.x, touchState.y);
    }
    touchState.dist = dist;
  }
}
function onTouchEnd() { touchState = null; }

// ── Cluster focus / detail selection ─────────────────────────────────────
const focusedClusterId = ref<ClusterMeta['id'] | null>(null);
const selectedQuestId = ref<string | null>(null);

const focusedClusterMeta = computed(() => CLUSTER_META.find(c => c.id === focusedClusterId.value) ?? null);
const selectedNode = computed<SkillNode | null>(() =>
  selectedQuestId.value ? skillTree.nodes.find(n => n.id === selectedQuestId.value) ?? null : null,
);
const selectedStatus = computed(() =>
  selectedNode.value ? skillTree.getSkillStatus(selectedNode.value.id) : 'locked',
);
const selectedClusterId = computed(() => selectedNode.value?.category ?? 'brain');

function zoomToCluster(id: ClusterMeta['id']) {
  const meta = CLUSTER_META.find(c => c.id === id);
  if (!meta) return;
  focusedClusterId.value = id;
  selectedQuestId.value = null;
  animating.value = true;
  const targetScale = Math.min(MAX_SCALE, 1.6);
  scale.value = targetScale;
  tx.value = viewportSize.value.w / 2 - meta.cx * targetScale;
  ty.value = viewportSize.value.h / 2 - meta.cy * targetScale;
}

function selectNode(id: string) {
  const node = skillTree.nodes.find(n => n.id === id);
  if (!node) return;
  // Auto-focus the cluster the node belongs to (so detail context makes sense).
  if (focusedClusterId.value !== node.category) {
    zoomToCluster(node.category as ClusterMeta['id']);
  }
  selectedQuestId.value = id;
}

function closeDetail() { selectedQuestId.value = null; }

function goBack() {
  if (selectedQuestId.value) {
    selectedQuestId.value = null;
  } else if (focusedClusterId.value) {
    focusedClusterId.value = null;
    animating.value = true;
    fitInitial();
  }
}
function goCluster() { selectedQuestId.value = null; }
function goHome() { resetView(); selectedQuestId.value = null; }

function beginQuest(id: string) {
  emit('begin', id);
}

function getNodeIcon(id: string): string { return skillTree.nodes.find(n => n.id === id)?.icon ?? '?'; }
function getNodeName(id: string): string { return skillTree.nodes.find(n => n.id === id)?.name ?? id; }
function isPinned(id: string): boolean { return skillTree.tracker.pinnedQuestIds.includes(id); }
function stepNumeral(i: number): string { return ['Ⅰ', 'Ⅱ', 'Ⅲ', 'Ⅳ', 'Ⅴ'][i] ?? String(i + 1); }

function onEsc() {
  if (selectedQuestId.value || focusedClusterId.value) goBack();
  else emit('close');
}

// ── Lifecycle ────────────────────────────────────────────────────────────
const rootRef = ref<HTMLElement | null>(null);
let resizeObserver: ResizeObserver | null = null;

function handleResize() { measureViewport(); }

watch(() => props.visible, async (v) => {
  if (v) {
    await nextTick();
    measureViewport();
    fitInitial();
    focusedClusterId.value = null;
    selectedQuestId.value = null;
    rootRef.value?.focus();
  }
});

onMounted(() => {
  if (props.visible) {
    nextTick(() => { measureViewport(); fitInitial(); });
  }
  window.addEventListener('resize', handleResize);
  if (typeof ResizeObserver !== 'undefined' && viewportRef.value) {
    resizeObserver = new ResizeObserver(() => measureViewport());
    resizeObserver.observe(viewportRef.value);
  }
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  window.removeEventListener('mousemove', onDragMove);
  window.removeEventListener('mouseup', onDragEnd);
  resizeObserver?.disconnect();
});
</script>

<style scoped>
/* ═══════════════════════════════════════════════════════════════════════
   Skill Constellation — FF16 Abilities-style full-screen map
   ═══════════════════════════════════════════════════════════════════════ */
.skill-constellation {
  position: fixed;
  inset: 0;
  z-index: 50;
  overflow: hidden;
  background: radial-gradient(ellipse at 50% 30%, #0c1538 0%, #050818 55%, #02040d 100%);
  color: #e8eaff;
  outline: none;
  user-select: none;
}

/* ── Star-field background ───────────────────────────────────────────── */
.sc-starfield { position: absolute; inset: 0; pointer-events: none; }
.sc-stars {
  position: absolute; inset: 0;
  background-repeat: repeat;
  background-size: 220px 220px;
}
.sc-stars-1 {
  background-image:
    radial-gradient(1px 1px at 20px 30px, rgba(255,255,255,0.7), transparent 60%),
    radial-gradient(1px 1px at 80px 120px, rgba(180,200,255,0.6), transparent 60%),
    radial-gradient(1.5px 1.5px at 160px 60px, rgba(255,240,200,0.5), transparent 60%);
  animation: sc-drift 240s linear infinite;
}
.sc-stars-2 {
  background-size: 380px 380px;
  background-image:
    radial-gradient(1px 1px at 50px 90px, rgba(200,220,255,0.55), transparent 60%),
    radial-gradient(1.5px 1.5px at 250px 200px, rgba(255,255,255,0.6), transparent 60%),
    radial-gradient(1px 1px at 320px 320px, rgba(180,180,220,0.45), transparent 60%);
  animation: sc-drift 360s linear infinite reverse;
  opacity: 0.7;
}
.sc-stars-3 {
  background-size: 540px 540px;
  background-image:
    radial-gradient(2px 2px at 100px 140px, rgba(255,255,255,0.45), transparent 60%),
    radial-gradient(1px 1px at 400px 80px, rgba(220,200,255,0.4), transparent 60%);
  animation: sc-twinkle 6s ease-in-out infinite;
  opacity: 0.55;
}
@keyframes sc-drift {
  0% { transform: translate(0, 0); }
  100% { transform: translate(-220px, -220px); }
}
@keyframes sc-twinkle {
  0%, 100% { opacity: 0.55; }
  50% { opacity: 0.9; }
}
.sc-nebula {
  position: absolute;
  inset: -10%;
  background:
    radial-gradient(circle at 30% 20%, rgba(80, 60, 180, 0.18), transparent 55%),
    radial-gradient(circle at 70% 70%, rgba(40, 100, 180, 0.18), transparent 55%),
    radial-gradient(circle at 80% 30%, rgba(180, 60, 60, 0.10), transparent 50%);
  filter: blur(40px);
  pointer-events: none;
}

/* ── Top bar ─────────────────────────────────────────────────────────── */
.sc-topbar {
  position: absolute;
  top: 12px; left: 12px; right: 12px;
  display: flex; justify-content: space-between; align-items: center;
  z-index: 5;
  pointer-events: none;
}
.sc-topbar > * { pointer-events: auto; }
.sc-breadcrumb {
  display: flex; align-items: center; gap: 6px;
  background: rgba(8, 12, 32, 0.7);
  border: 1px solid rgba(180, 160, 100, 0.22);
  border-radius: 4px;
  padding: 6px 12px;
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  backdrop-filter: blur(8px);
}
.sc-crumb {
  background: none; border: none; color: rgba(200, 200, 220, 0.6); cursor: pointer;
  padding: 2px 4px; font-size: inherit; letter-spacing: inherit;
  transition: color 0.15s ease;
}
.sc-crumb:hover { color: #dcc36e; }
.sc-crumb--active { color: #dcc36e; font-weight: 700; cursor: default; }
.sc-crumb-sep { color: rgba(180, 160, 100, 0.4); }
.sc-topbar-actions { display: flex; gap: 6px; }
.sc-back-btn, .sc-close-btn {
  background: rgba(8, 12, 32, 0.7);
  border: 1px solid rgba(180, 160, 100, 0.22);
  color: #dcc36e;
  padding: 6px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  transition: all 0.15s ease;
  backdrop-filter: blur(8px);
}
.sc-back-btn:hover, .sc-close-btn:hover {
  border-color: rgba(220, 195, 110, 0.5);
  background: rgba(20, 24, 48, 0.8);
}

/* ── Viewport ────────────────────────────────────────────────────────── */
.sc-viewport {
  position: absolute;
  inset: 0;
  cursor: grab;
  touch-action: none;
}
.sc-viewport:active { cursor: grabbing; }
.sc-world {
  position: relative;
  transform-origin: 0 0;
  will-change: transform;
}

.sc-bg-svg {
  position: absolute;
  top: 0; left: 0;
  width: 100%; height: 100%;
  pointer-events: none;
}
.sc-constellation-line {
  stroke: rgba(180, 160, 100, 0.08);
  stroke-width: 1.5;
  stroke-dasharray: 6 8;
}

/* ── Cluster ─────────────────────────────────────────────────────────── */
.sc-cluster {
  position: absolute;
  width: 0; height: 0;
  pointer-events: none; /* children re-enable */
}
.sc-cluster > * { pointer-events: auto; }

/* Diamond border behind cluster */
.sc-cluster-diamond {
  position: absolute;
  left: 0; top: 0;
  width: 480px; height: 480px;
  transform: translate(-50%, -50%) rotate(45deg);
  border: 1.5px solid var(--cluster-color, rgba(180,160,100,0.3));
  border-radius: 12px;
  opacity: 0.45;
  filter: drop-shadow(0 0 12px var(--cluster-glow, rgba(180,160,100,0.25)));
  pointer-events: none;
}

/* Cluster colour theming */
.sc-cluster--brain   { --cluster-color: rgba(220, 80, 80, 0.55);  --cluster-glow: rgba(220, 80, 80, 0.35);  }
.sc-cluster--voice   { --cluster-color: rgba(80, 200, 130, 0.55); --cluster-glow: rgba(80, 200, 130, 0.35); }
.sc-cluster--avatar  { --cluster-color: rgba(220, 195, 110, 0.6); --cluster-glow: rgba(220, 195, 110, 0.4); }
.sc-cluster--social  { --cluster-color: rgba(100, 160, 240, 0.55);--cluster-glow: rgba(100, 160, 240, 0.35);}
.sc-cluster--utility { --cluster-color: rgba(180, 120, 220, 0.55);--cluster-glow: rgba(180, 120, 220, 0.35);}

.sc-cluster-svg {
  position: absolute;
  left: 0; top: 0;
  width: 480px; height: 480px;
  transform: translate(-50%, -50%);
  pointer-events: none;
  overflow: visible;
}
.sc-edge {
  stroke: var(--cluster-color, rgba(180,160,100,0.3));
  stroke-width: 1.2;
  opacity: 0.35;
}
.sc-edge--active {
  stroke: var(--cluster-glow, rgba(180,160,100,0.5));
  stroke-width: 2;
  opacity: 0.8;
  filter: drop-shadow(0 0 4px var(--cluster-glow, rgba(180,160,100,0.5)));
}

/* ── Chunk 133: Brain Evolution Path ─────────────────────────────────────
 * Inside the brain cluster, edges become glowing "neural pathways":
 *   - inactive edges: very dim, hint of structure only
 *   - active edges: pulse a flowing dashed signal along the wire
 * The animation is scoped to the brain cluster only so the other clusters
 * keep their cleaner constellation look. */
.sc-cluster--brain .sc-edge {
  stroke: rgba(220, 80, 80, 0.18);
  stroke-width: 1.5;
  opacity: 0.55;
}
.sc-cluster--brain .sc-edge--active {
  stroke: rgba(255, 150, 150, 0.95);
  stroke-width: 2.5;
  opacity: 1;
  stroke-dasharray: 6 6;
  animation: sc-neural-flow 2.4s linear infinite;
  filter: drop-shadow(0 0 6px rgba(255, 100, 100, 0.7));
}
@keyframes sc-neural-flow {
  from { stroke-dashoffset: 24; }
  to   { stroke-dashoffset: 0; }
}
/* Locked brain nodes look like dormant synapses. */
.sc-cluster--brain .sc-node--locked {
  filter: grayscale(0.7) brightness(0.55);
  opacity: 0.6;
}
.sc-cluster--brain .sc-node--active {
  box-shadow: 0 0 14px rgba(255, 120, 120, 0.55), inset 0 0 8px rgba(255, 200, 200, 0.25);
}

/* Concentric rings */
.sc-ring {
  position: absolute;
  left: 0; top: 0;
  border: 1px dashed var(--cluster-color, rgba(180,160,100,0.2));
  border-radius: 50%;
  transform: translate(-50%, -50%);
  opacity: 0.25;
  pointer-events: none;
}
.sc-ring--inner  { width: 180px; height: 180px; }
.sc-ring--mid    { width: 310px; height: 310px; }
.sc-ring--outer  { width: 440px; height: 440px; }

/* Cluster centre emblem */
.sc-cluster-emblem {
  position: absolute;
  left: 0; top: 0;
  transform: translate(-50%, -50%);
  width: 78px; height: 78px;
  border-radius: 50%;
  border: 2px solid var(--cluster-color, rgba(180,160,100,0.4));
  background: radial-gradient(circle at 40% 35%, rgba(30, 32, 60, 0.95), rgba(8, 10, 24, 0.98));
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  cursor: pointer;
  box-shadow:
    0 0 24px var(--cluster-glow, rgba(180,160,100,0.4)),
    inset 0 0 20px rgba(100, 140, 220, 0.08);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
  z-index: 2;
}
.sc-cluster-emblem:hover {
  transform: translate(-50%, -50%) scale(1.08);
  box-shadow: 0 0 36px var(--cluster-glow, rgba(180,160,100,0.6)), inset 0 0 22px rgba(100, 140, 220, 0.12);
}
.sc-cluster--focused .sc-cluster-emblem {
  border-width: 3px;
}
.sc-cluster-icon { font-size: 1.6rem; line-height: 1; }
.sc-cluster-label {
  font-size: 0.55rem; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase;
  color: #dcc36e; margin-top: 2px;
}
.sc-cluster-cost {
  font-size: 0.5rem; color: rgba(200, 200, 220, 0.55); letter-spacing: 0.05em;
}

/* ── Skill nodes ─────────────────────────────────────────────────────── */
.sc-node {
  position: absolute;
  left: 0; top: 0;
  transform: translate(-50%, -50%);
  background: none; border: none; cursor: pointer; padding: 0;
  display: flex; flex-direction: column; align-items: center; gap: 4px;
  z-index: 3;
}
.sc-node-gem {
  width: 46px; height: 46px;
  border-radius: 50%;
  border: 2px solid rgba(100, 100, 130, 0.4);
  background: radial-gradient(circle at 40% 35%, rgba(40, 38, 65, 0.92), rgba(15, 13, 30, 0.97));
  display: flex; align-items: center; justify-content: center;
  position: relative;
  transition: all 0.2s ease;
}
.sc-node:hover .sc-node-gem { transform: scale(1.12); border-color: rgba(220, 195, 110, 0.65); }
.sc-node-icon { font-size: 1.15rem; z-index: 1; }
.sc-node-glow {
  position: absolute; inset: -4px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(142, 200, 246, 0.25) 0%, transparent 70%);
  animation: sc-glow-rotate 5s linear infinite;
}
@keyframes sc-glow-rotate { from { transform: rotate(0); } to { transform: rotate(360deg); } }
.sc-node-cost {
  font-size: 0.55rem; font-weight: 600; color: rgba(200, 200, 220, 0.85);
  text-align: center; max-width: 90px; line-height: 1.15;
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.8);
}

.sc-node--locked { opacity: 0.35; filter: saturate(0.3); }
.sc-node--locked .sc-node-gem { border-color: rgba(80, 80, 100, 0.4); }

.sc-node--available .sc-node-gem {
  border-color: rgba(220, 195, 110, 0.7);
  animation: sc-node-breathe 2.5s ease-in-out infinite;
}
@keyframes sc-node-breathe {
  0%, 100% { box-shadow: 0 0 6px rgba(220, 195, 110, 0.25); }
  50% { box-shadow: 0 0 16px rgba(220, 195, 110, 0.55); }
}

.sc-node--active .sc-node-gem {
  border-color: rgba(142, 200, 246, 0.75);
  box-shadow: 0 0 14px rgba(142, 200, 246, 0.35);
}
.sc-node--active .sc-node-cost { color: #8ec8f6; }

/* ── Zoom controls ───────────────────────────────────────────────────── */
.sc-zoom-controls {
  position: absolute;
  right: 16px; bottom: 16px;
  display: flex; flex-direction: column; gap: 4px;
  z-index: 5;
}
.sc-zoom-btn {
  width: 36px; height: 36px;
  border-radius: 50%;
  background: rgba(8, 12, 32, 0.8);
  border: 1px solid rgba(180, 160, 100, 0.25);
  color: #dcc36e;
  font-size: 1.1rem;
  cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  backdrop-filter: blur(6px);
  transition: all 0.15s ease;
}
.sc-zoom-btn:hover { border-color: rgba(220, 195, 110, 0.6); transform: scale(1.08); }
.sc-zoom-btn--reset { font-size: 0.95rem; }

/* ── Minimap ─────────────────────────────────────────────────────────── */
.sc-minimap {
  position: absolute;
  left: 16px; bottom: 16px;
  width: 180px; height: 135px;
  border: 1px solid rgba(180, 160, 100, 0.25);
  border-radius: 4px;
  background: rgba(4, 6, 18, 0.85);
  overflow: hidden;
  z-index: 5;
  backdrop-filter: blur(6px);
}
.sc-minimap svg { width: 100%; height: 100%; display: block; }
.sc-minimap-bg { fill: rgba(10, 14, 36, 0.6); }
.sc-minimap-line { stroke: rgba(180, 160, 100, 0.15); stroke-width: 2; }
.sc-minimap-cluster { fill: rgba(60, 60, 100, 0.3); stroke-width: 2; }
.sc-minimap-cluster--brain   { stroke: rgba(220, 80, 80, 0.7); }
.sc-minimap-cluster--voice   { stroke: rgba(80, 200, 130, 0.7); }
.sc-minimap-cluster--avatar  { stroke: rgba(220, 195, 110, 0.75); }
.sc-minimap-cluster--social  { stroke: rgba(100, 160, 240, 0.7); }
.sc-minimap-cluster--utility { stroke: rgba(180, 120, 220, 0.7); }
.sc-minimap-dot { fill: rgba(120, 120, 140, 0.5); }
.sc-minimap-dot--available { fill: rgba(220, 195, 110, 0.85); }
.sc-minimap-dot--active    { fill: rgba(142, 200, 246, 0.95); }
.sc-minimap-viewport {
  fill: none;
  stroke: rgba(220, 195, 110, 0.55);
  stroke-width: 6;
  stroke-dasharray: 12 8;
}

/* ── Detail overlay ──────────────────────────────────────────────────── */
.sc-detail {
  position: absolute;
  right: 16px; top: 64px;
  width: 360px;
  max-height: calc(100vh - 240px);
  overflow-y: auto;
  background: linear-gradient(170deg, rgba(16, 14, 36, 0.98), rgba(8, 6, 22, 0.99));
  border: 1px solid rgba(180, 160, 100, 0.3);
  border-radius: 6px;
  box-shadow: 0 12px 48px rgba(0,0,0,0.7);
  padding: 16px;
  z-index: 6;
}
.sc-detail--brain   { border-color: rgba(220, 80, 80, 0.45); }
.sc-detail--voice   { border-color: rgba(80, 200, 130, 0.45); }
.sc-detail--avatar  { border-color: rgba(220, 195, 110, 0.55); }
.sc-detail--social  { border-color: rgba(100, 160, 240, 0.45); }
.sc-detail--utility { border-color: rgba(180, 120, 220, 0.45); }

.sc-detail-top { display: flex; align-items: flex-start; gap: 10px; }
.sc-detail-gem { font-size: 1.6rem; flex-shrink: 0; }
.sc-detail-title-area { flex: 1; }
.sc-detail-name { font-size: 0.95rem; font-weight: 700; color: #dcc36e; letter-spacing: 0.04em; }
.sc-detail-tagline { font-size: 0.72rem; color: rgba(180, 180, 200, 0.55); margin-top: 2px; }
.sc-detail-close {
  background: none; border: 1px solid rgba(180, 160, 100, 0.2); color: rgba(200, 200, 220, 0.5);
  cursor: pointer; font-size: 0.75rem; padding: 2px 6px; border-radius: 2px;
}
.sc-detail-close:hover { border-color: rgba(180, 160, 100, 0.5); color: #dcc36e; }
.sc-detail-desc { font-size: 0.78rem; color: rgba(200, 200, 220, 0.7); line-height: 1.55; margin: 12px 0; }
.sc-detail-section { margin-top: 12px; }
.sc-detail-section-label {
  font-size: 0.65rem; font-weight: 700; color: #dcc36e;
  letter-spacing: 0.12em; text-transform: uppercase; margin-bottom: 6px;
}
.sc-step { display: flex; align-items: center; gap: 8px; padding: 3px 0; }
.sc-step-num { width: 18px; font-size: 0.65rem; font-weight: 700; color: rgba(180, 160, 100, 0.55); text-align: center; }
.sc-step-text { font-size: 0.74rem; color: rgba(200, 200, 220, 0.75); flex: 1; }
.sc-step-go {
  font-size: 0.72rem; color: #dcc36e; background: none;
  border: 1px solid rgba(180, 160, 100, 0.25); border-radius: 2px;
  padding: 1px 8px; cursor: pointer;
}
.sc-step-go:hover { background: rgba(180, 160, 100, 0.1); }
.sc-reward-list, .sc-prereq-list { display: flex; flex-wrap: wrap; gap: 4px; }
.sc-reward {
  font-size: 0.7rem; padding: 3px 8px; border-radius: 2px;
  background: rgba(180, 160, 100, 0.08); color: #dcc36e;
  border: 1px solid rgba(180, 160, 100, 0.18);
}
.sc-prereq {
  font-size: 0.7rem; padding: 3px 8px; border-radius: 2px;
  color: rgba(200, 200, 220, 0.5); border: 1px solid rgba(100, 100, 130, 0.2);
}
.sc-prereq--met { color: #8ec8f6; border-color: rgba(142, 200, 246, 0.3); }
.sc-detail-actions { display: flex; gap: 6px; margin-top: 14px; }
.sc-btn {
  flex: 1; padding: 8px 0; font-size: 0.74rem; font-weight: 700;
  cursor: pointer; border-radius: 3px; letter-spacing: 0.04em; text-transform: uppercase;
  transition: all 0.15s ease;
}
.sc-btn--secondary { background: transparent; border: 1px solid rgba(180, 160, 100, 0.25); color: rgba(200, 200, 220, 0.65); }
.sc-btn--secondary:hover { border-color: rgba(180, 160, 100, 0.5); color: #dcc36e; }
.sc-btn--primary {
  background: linear-gradient(180deg, rgba(180, 160, 100, 0.18), rgba(180, 160, 100, 0.06));
  border: 1px solid rgba(220, 195, 110, 0.45);
  color: #dcc36e;
  text-shadow: 0 0 8px rgba(220, 195, 110, 0.25);
}
.sc-btn--primary:hover {
  background: linear-gradient(180deg, rgba(180, 160, 100, 0.28), rgba(180, 160, 100, 0.12));
  box-shadow: 0 0 14px rgba(220, 195, 110, 0.18);
}

/* ── Transitions ─────────────────────────────────────────────────────── */
.constellation-enter-active, .constellation-leave-active { transition: opacity 0.3s ease; }
.constellation-enter-from, .constellation-leave-to { opacity: 0; }
.sc-detail-enter-active, .sc-detail-leave-active { transition: all 0.2s ease; }
.sc-detail-enter-from, .sc-detail-leave-to { opacity: 0; transform: translateX(8px); }

/* ── Mobile ──────────────────────────────────────────────────────────── */
@media (max-width: 640px) {
  .sc-detail {
    width: calc(100vw - 24px);
    right: 12px; left: 12px;
    top: auto; bottom: 12px;
    max-height: 50vh;
  }
  .sc-minimap { width: 110px; height: 80px; left: 12px; bottom: 12px; }
  .sc-zoom-controls { right: 12px; bottom: 12px; }
}
</style>
