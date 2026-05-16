<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick, watch } from 'vue';
import AppTabIcon from './AppTabIcon.vue';

type TabId = 'chat' | 'skills' | 'brain' | 'memory' | 'marketplace' | 'mobile' | 'voice' | 'settings';

interface TabDef {
  id: TabId;
  label: string;
}

const props = defineProps<{
  tabs: readonly TabDef[];
  activeTab: TabId;
  isMcpMode: boolean;
  isDevBuild: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:activeTab', id: TabId): void;
}>();

// ── Scroll-overflow detection ───────────────────────────────────────────────
const scroller = ref<HTMLElement | null>(null);
const scrollLeft = ref(0);
const scrollWidth = ref(0);
const clientWidth = ref(0);

// Small dead-zone so 1-px subpixel rounding doesn't flicker the arrows.
const EDGE_EPSILON = 4;

const canScrollLeft = computed(() => scrollLeft.value > EDGE_EPSILON);
const canScrollRight = computed(
  () => scrollWidth.value - clientWidth.value - scrollLeft.value > EDGE_EPSILON,
);

function measure(): void {
  const el = scroller.value;
  if (!el) return;
  scrollLeft.value = el.scrollLeft;
  scrollWidth.value = el.scrollWidth;
  clientWidth.value = el.clientWidth;
}

function onScroll(): void {
  measure();
}

function scrollByDir(dir: 1 | -1): void {
  const el = scroller.value;
  if (!el) return;
  // Page-scroll ~80% of the visible width so the user always sees a couple
  // of overlapping tabs as continuity anchors.
  const delta = Math.max(el.clientWidth * 0.8, 120) * dir;
  el.scrollBy({ left: delta, behavior: 'smooth' });
}

// ── Drag-to-scroll (desktop mouse) — native touch scroll is untouched ──────
let dragging = false;
let dragMoved = false;
let dragStartX = 0;
let dragStartScroll = 0;

function onPointerDown(e: PointerEvent): void {
  // Only left mouse button; ignore touch (native horizontal scroll handles it)
  // and pen (probably touch-equivalent on tablets).
  if (e.pointerType !== 'mouse' || e.button !== 0) return;
  const el = scroller.value;
  if (!el) return;
  dragging = true;
  dragMoved = false;
  dragStartX = e.clientX;
  dragStartScroll = el.scrollLeft;
}

function onPointerMove(e: PointerEvent): void {
  if (!dragging) return;
  const el = scroller.value;
  if (!el) return;
  const dx = e.clientX - dragStartX;
  if (Math.abs(dx) > 4) dragMoved = true;
  el.scrollLeft = dragStartScroll - dx;
}

function endDrag(): void {
  dragging = false;
  // Reset dragMoved on next tick so the upcoming click event (if any) can
  // still see it and suppress the tab activation when the user was scrubbing.
  setTimeout(() => {
    dragMoved = false;
  }, 0);
}

function onTabClick(id: TabId, e: MouseEvent): void {
  if (dragMoved) {
    e.preventDefault();
    e.stopPropagation();
    return;
  }
  emit('update:activeTab', id);
}

// ── Lifecycle ──────────────────────────────────────────────────────────────
let resizeObs: ResizeObserver | null = null;

onMounted(() => {
  nextTick(() => {
    measure();
    const el = scroller.value;
    if (el && typeof ResizeObserver !== 'undefined') {
      resizeObs = new ResizeObserver(() => measure());
      resizeObs.observe(el);
      for (const child of Array.from(el.children)) {
        resizeObs.observe(child as Element);
      }
    }
    window.addEventListener('resize', measure);
  });
});

onBeforeUnmount(() => {
  resizeObs?.disconnect();
  resizeObs = null;
  window.removeEventListener('resize', measure);
});

// Scroll the active tab into view whenever it changes (e.g. external nav).
watch(
  () => props.activeTab,
  async (id) => {
    await nextTick();
    const el = scroller.value;
    if (!el) return;
    const target = el.querySelector<HTMLElement>(`[data-tab-id="${id}"]`);
    if (target) {
      target.scrollIntoView({ behavior: 'smooth', inline: 'center', block: 'nearest' });
    }
  },
);
</script>

<template>
  <nav
    class="mobile-bottom-nav"
    data-testid="mobile-bottom-nav"
  >
    <!-- Build-mode indicator — pinned to the left edge so it never scrolls
         out of view. MCP mode takes priority over DEV. -->
    <span
      v-if="props.isMcpMode"
      class="mobile-mcp-indicator"
      title="MCP mode"
    >MCP</span>
    <span
      v-else-if="props.isDevBuild"
      class="mobile-dev-indicator"
      title="Development build"
    >DEV</span>

    <!-- Left edge-arrow — only visible when there's content scrolled off
         the left side. -->
    <button
      v-show="canScrollLeft"
      type="button"
      class="mbn-arrow mbn-arrow--left"
      aria-label="Scroll tabs left"
      data-testid="mobile-nav-arrow-left"
      @click="scrollByDir(-1)"
    >
      <svg
        viewBox="0 0 24 24"
        width="16"
        height="16"
        aria-hidden="true"
      >
        <path
          d="M15 6 L9 12 L15 18"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>

    <!-- Scrollable tab strip — horizontal scroll with hidden scrollbar.
         Native touch scroll works; pointer drag works on mouse. -->
    <div
      ref="scroller"
      class="mbn-scroller"
      data-testid="mobile-nav-scroller"
      @scroll="onScroll"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="endDrag"
      @pointercancel="endDrag"
      @pointerleave="endDrag"
    >
      <button
        v-for="tab in props.tabs"
        :key="tab.id"
        type="button"
        :class="['mobile-tab', { active: props.activeTab === tab.id }]"
        :data-tab-id="tab.id"
        @click="onTabClick(tab.id, $event)"
      >
        <span class="mobile-tab-icon">
          <AppTabIcon :name="tab.id" />
        </span>
        <span class="mobile-tab-label">{{ tab.label }}</span>
      </button>
    </div>

    <!-- Right edge-arrow — only visible when there's content scrolled off
         the right side. -->
    <button
      v-show="canScrollRight"
      type="button"
      class="mbn-arrow mbn-arrow--right"
      aria-label="Scroll tabs right"
      data-testid="mobile-nav-arrow-right"
      @click="scrollByDir(1)"
    >
      <svg
        viewBox="0 0 24 24"
        width="16"
        height="16"
        aria-hidden="true"
      >
        <path
          d="M9 6 L15 12 L9 18"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  </nav>
</template>

<style scoped>
/* NOTE: outer positioning, fixed bottom anchoring, glass background, height
   and the desktop-hide media-query live in src/App.css under the global
   `.mobile-bottom-nav` selector. This scoped block only owns the internal
   scroller + tab pills + edge arrows. */

/* ── Scrollable strip — hide native scrollbar on every engine ── */
.mbn-scroller {
  flex: 1 1 auto;
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 2px;
  height: 100%;
  overflow-x: auto;
  overflow-y: hidden;
  scroll-behavior: smooth;
  /* Native horizontal touch scroll + momentum on iOS/Android */
  -webkit-overflow-scrolling: touch;
  overscroll-behavior-x: contain;
  /* Firefox: hide scrollbar */
  scrollbar-width: none;
  /* Edge legacy */
  -ms-overflow-style: none;
  padding: 0 4px;
}
/* WebKit (Chrome/Safari/WebView2): hide scrollbar */
.mbn-scroller::-webkit-scrollbar {
  display: none;
  width: 0;
  height: 0;
}

.mobile-tab {
  display: flex;
  flex: 0 0 auto;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  border: none;
  background: transparent;
  cursor: pointer;
  color: var(--ts-text-muted);
  padding: 6px 8px;
  border-radius: var(--ts-radius-md);
  transition: color var(--ts-transition-fast), background var(--ts-transition-fast);
  min-width: 52px;
  position: relative;
  /* Prevent the browser's text-selection drag from competing with our
     pointer-drag-to-scroll handler. */
  user-select: none;
  -webkit-user-select: none;
}
.mobile-tab-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  pointer-events: none;
}
.mobile-tab-icon :deep(svg) { width: 20px; height: 20px; }
.mobile-tab-label {
  font-size: 0.58rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  line-height: 1;
  pointer-events: none;
}
.mobile-tab:hover { color: var(--ts-text-secondary); }
.mobile-tab.active { color: var(--ts-accent); }
.mobile-tab.active::after {
  content: '';
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  width: 20px;
  height: 2px;
  background: var(--ts-accent);
  border-radius: 0 0 2px 2px;
}

/* ── Edge arrows ── */
.mbn-arrow {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 36px;
  border: none;
  background: transparent;
  color: var(--ts-text-secondary);
  cursor: pointer;
  border-radius: var(--ts-radius-md);
  transition: color var(--ts-transition-fast), background var(--ts-transition-fast);
  /* Faint fade so the arrow visually "lifts" the edge content under it,
     reinforcing the "there's more this way" affordance. */
  position: relative;
  z-index: 1;
}
.mbn-arrow:hover {
  color: var(--ts-accent);
  background: var(--ts-bg-hover);
}
.mbn-arrow:focus-visible {
  outline: 2px solid var(--ts-accent);
  outline-offset: -2px;
}
</style>
