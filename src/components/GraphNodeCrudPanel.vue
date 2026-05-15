<!--
  GraphNodeCrudPanel.vue

  Full CRUD surface for a single memory node in the knowledge-graph viewport.
  Replaces the legacy inline "node detail" sidebar with a polished glass-card
  layout that supports:

  - Inline edit of node fields (content, tags, importance, memory_type)
  - Delete node (with confirm)
  - Separate "Parents" (incoming) and "Children" (outgoing) edge lists
  - Inline edit of each edge (rel_type + confidence)
  - Delete individual edges; detach all
  - "Link" form to attach a new edge to another memory (as parent or child)
    with target combobox, rel_type select, and confidence slider

  Backed by the Pinia memory store (add/update/delete edge, detach, update
  memory). All actions are optimistic-via-store and surface inline error/success
  feedback for one-action-at-a-time clarity.
-->
<template>
  <aside
    class="gncp"
    data-testid="graph-node-crud-panel"
  >
    <header class="gncp-header">
      <div class="gncp-id-row">
        <span
          class="gncp-kind-dot"
          :style="{ background: kindColour }"
          :title="kind"
        />
        <span class="gncp-id">#{{ entry.id }}</span>
        <span class="gncp-kind-label">{{ kind }}</span>
        <button
          class="gncp-icon-btn"
          title="Close"
          @click="$emit('close')"
        >
          ✕
        </button>
      </div>
      <div
        v-if="!isEditing"
        class="gncp-content-preview"
      >
        {{ entry.content }}
      </div>
    </header>

    <!-- ── Edit mode ──────────────────────────────────────────── -->
    <section
      v-if="isEditing"
      class="gncp-edit"
    >
      <label class="gncp-field">
        <span class="gncp-field-label">Content</span>
        <textarea
          v-model="editForm.content"
          rows="4"
          class="gncp-textarea"
          data-testid="gncp-edit-content"
        />
      </label>
      <label class="gncp-field">
        <span class="gncp-field-label">Tags</span>
        <input
          v-model="editForm.tags"
          type="text"
          class="gncp-input"
          placeholder="comma,separated,tags"
        >
      </label>
      <div class="gncp-field-row">
        <label class="gncp-field">
          <span class="gncp-field-label">Type</span>
          <select
            v-model="editForm.memory_type"
            class="gncp-input"
          >
            <option value="fact">fact</option>
            <option value="preference">preference</option>
            <option value="episode">episode</option>
            <option value="procedure">procedure</option>
          </select>
        </label>
        <label class="gncp-field">
          <span class="gncp-field-label">Importance ({{ editForm.importance }})</span>
          <input
            v-model.number="editForm.importance"
            type="range"
            min="1"
            max="5"
            class="gncp-range"
          >
        </label>
      </div>
      <div class="gncp-actions">
        <button
          class="gncp-btn gncp-btn-ghost"
          @click="cancelEdit"
        >
          Cancel
        </button>
        <button
          class="gncp-btn gncp-btn-primary"
          :disabled="!editForm.content.trim()"
          data-testid="gncp-save-edit"
          @click="saveEdit"
        >
          💾 Save
        </button>
      </div>
    </section>

    <!-- ── Read mode metadata ─────────────────────────────────── -->
    <section
      v-else
      class="gncp-meta"
    >
      <div class="gncp-stat">
        <span class="gncp-stat-label">Type</span>
        <span class="gncp-stat-val">{{ entry.memory_type }}</span>
      </div>
      <div class="gncp-stat">
        <span class="gncp-stat-label">Tier</span>
        <span :class="['gncp-tier', `gncp-tier-${entry.tier}`]">{{ entry.tier }}</span>
      </div>
      <div class="gncp-stat">
        <span class="gncp-stat-label">Importance</span>
        <span class="gncp-stat-val">{{ '★'.repeat(entry.importance) }}</span>
      </div>
      <div class="gncp-stat">
        <span class="gncp-stat-label">Decay</span>
        <span class="gncp-stat-val">{{ Math.round(entry.decay_score * 100) }}%</span>
      </div>
      <div
        v-if="entry.tags"
        class="gncp-tags"
      >
        <span
          v-for="t in tagList"
          :key="t"
          class="gncp-tag"
        >{{ t }}</span>
      </div>
    </section>

    <!-- ── Edges: parents / children ──────────────────────────── -->
    <section
      v-if="!isEditing"
      class="gncp-edges"
    >
      <div class="gncp-edges-header">
        <h4 class="gncp-section-title">
          Relationships
          <span class="gncp-count">{{ parents.length + children.length }}</span>
        </h4>
        <button
          v-if="parents.length + children.length > 0"
          class="gncp-link-btn gncp-link-btn-danger"
          :title="`Remove all ${parents.length + children.length} edges`"
          data-testid="gncp-detach-all"
          @click="detachAll"
        >
          Detach all
        </button>
      </div>

      <!-- Parents -->
      <div class="gncp-edge-group">
        <div class="gncp-edge-group-title">
          <span>← Parents</span>
          <span class="gncp-count-sm">{{ parents.length }}</span>
        </div>
        <ul
          v-if="parents.length"
          class="gncp-edge-list"
        >
          <li
            v-for="e in parents"
            :key="e.id"
            class="gncp-edge"
          >
            <button
              class="gncp-edge-rel"
              :class="{ 'gncp-edge-rel-editing': editingEdgeId === e.id }"
              :style="{ background: relColour(e.rel_type) }"
              @click="beginEdgeEdit(e)"
            >
              {{ e.rel_type }}
            </button>
            <button
              class="gncp-neighbour"
              :title="neighbourPreview(e.src_id)"
              @click="$emit('navigate', e.src_id)"
            >
              #{{ e.src_id }} <span class="gncp-neigh-text">{{ neighbourShort(e.src_id) }}</span>
            </button>
            <span class="gncp-conf">{{ Math.round(e.confidence * 100) }}%</span>
            <button
              class="gncp-icon-btn gncp-icon-btn-danger"
              :title="`Delete edge #${e.id}`"
              @click="deleteEdge(e.id)"
            >
              ✕
            </button>
          </li>
        </ul>
        <p
          v-else
          class="gncp-empty-mini"
        >
          No parents.
        </p>
      </div>

      <!-- Children -->
      <div class="gncp-edge-group">
        <div class="gncp-edge-group-title">
          <span>→ Children</span>
          <span class="gncp-count-sm">{{ children.length }}</span>
        </div>
        <ul
          v-if="children.length"
          class="gncp-edge-list"
        >
          <li
            v-for="e in children"
            :key="e.id"
            class="gncp-edge"
          >
            <button
              class="gncp-edge-rel"
              :class="{ 'gncp-edge-rel-editing': editingEdgeId === e.id }"
              :style="{ background: relColour(e.rel_type) }"
              @click="beginEdgeEdit(e)"
            >
              {{ e.rel_type }}
            </button>
            <button
              class="gncp-neighbour"
              :title="neighbourPreview(e.dst_id)"
              @click="$emit('navigate', e.dst_id)"
            >
              #{{ e.dst_id }} <span class="gncp-neigh-text">{{ neighbourShort(e.dst_id) }}</span>
            </button>
            <span class="gncp-conf">{{ Math.round(e.confidence * 100) }}%</span>
            <button
              class="gncp-icon-btn gncp-icon-btn-danger"
              :title="`Delete edge #${e.id}`"
              @click="deleteEdge(e.id)"
            >
              ✕
            </button>
          </li>
        </ul>
        <p
          v-else
          class="gncp-empty-mini"
        >
          No children.
        </p>
      </div>

      <!-- Inline edit edge -->
      <div
        v-if="editingEdgeId !== null"
        class="gncp-edge-edit"
        data-testid="gncp-edge-edit"
      >
        <div class="gncp-edge-edit-row">
          <label class="gncp-field gncp-field-inline">
            <span class="gncp-field-label">Relation</span>
            <select
              v-model="edgeEditForm.relType"
              class="gncp-input gncp-input-sm"
            >
              <option
                v-for="r in COMMON_RELATIONS"
                :key="r"
                :value="r"
              >
                {{ r }}
              </option>
            </select>
          </label>
          <label class="gncp-field gncp-field-inline">
            <span class="gncp-field-label">Confidence {{ Math.round(edgeEditForm.confidence * 100) }}%</span>
            <input
              v-model.number="edgeEditForm.confidence"
              type="range"
              min="0"
              max="1"
              step="0.05"
              class="gncp-range"
            >
          </label>
        </div>
        <div class="gncp-actions gncp-actions-tight">
          <button
            class="gncp-btn gncp-btn-ghost gncp-btn-sm"
            @click="cancelEdgeEdit"
          >
            Cancel
          </button>
          <button
            class="gncp-btn gncp-btn-primary gncp-btn-sm"
            data-testid="gncp-save-edge"
            @click="saveEdgeEdit"
          >
            Save edge
          </button>
        </div>
      </div>
    </section>

    <!-- ── Link to another memory ─────────────────────────────── -->
    <section
      v-if="!isEditing"
      class="gncp-link"
    >
      <h4 class="gncp-section-title">
        Link to memory
      </h4>
      <div class="gncp-link-form">
        <label class="gncp-field">
          <span class="gncp-field-label">Direction</span>
          <div class="gncp-seg">
            <button
              :class="['gncp-seg-btn', { 'gncp-seg-active': linkForm.direction === 'out' }]"
              @click="linkForm.direction = 'out'"
            >
              → as parent of…
            </button>
            <button
              :class="['gncp-seg-btn', { 'gncp-seg-active': linkForm.direction === 'in' }]"
              @click="linkForm.direction = 'in'"
            >
              ← as child of…
            </button>
          </div>
        </label>
        <label class="gncp-field">
          <span class="gncp-field-label">Target memory</span>
          <input
            v-model="linkForm.targetQuery"
            type="text"
            class="gncp-input"
            placeholder="Search by content or #id…"
            data-testid="gncp-target-search"
            @input="onTargetSearch"
          >
          <ul
            v-if="targetMatches.length > 0 && linkForm.targetQuery.length > 0 && linkForm.targetId === null"
            class="gncp-target-results"
          >
            <li
              v-for="m in targetMatches"
              :key="m.id"
              class="gncp-target-result"
              @click="pickTarget(m)"
            >
              <span class="gncp-target-id">#{{ m.id }}</span>
              <span class="gncp-target-snippet">{{ truncate(m.content, 80) }}</span>
            </li>
          </ul>
          <div
            v-if="linkForm.targetId !== null && pickedTarget"
            class="gncp-target-picked"
          >
            <span class="gncp-target-id">#{{ pickedTarget.id }}</span>
            <span class="gncp-target-snippet">{{ truncate(pickedTarget.content, 80) }}</span>
            <button
              class="gncp-icon-btn"
              title="Clear target"
              @click="clearTarget"
            >
              ✕
            </button>
          </div>
        </label>
        <div class="gncp-field-row">
          <label class="gncp-field">
            <span class="gncp-field-label">Relation</span>
            <select
              v-model="linkForm.relType"
              class="gncp-input"
              data-testid="gncp-link-rel"
            >
              <option
                v-for="r in COMMON_RELATIONS"
                :key="r"
                :value="r"
              >
                {{ r }}
              </option>
            </select>
          </label>
          <label class="gncp-field">
            <span class="gncp-field-label">Confidence {{ Math.round(linkForm.confidence * 100) }}%</span>
            <input
              v-model.number="linkForm.confidence"
              type="range"
              min="0"
              max="1"
              step="0.05"
              class="gncp-range"
            >
          </label>
        </div>
        <div class="gncp-actions">
          <button
            class="gncp-btn gncp-btn-primary"
            :disabled="linkForm.targetId === null || linkBusy"
            data-testid="gncp-link-submit"
            @click="submitLink"
          >
            🔗 Attach edge
          </button>
        </div>
      </div>
    </section>

    <!-- ── Footer node actions ────────────────────────────────── -->
    <footer
      v-if="!isEditing"
      class="gncp-footer"
    >
      <button
        class="gncp-btn gncp-btn-ghost"
        data-testid="gncp-edit"
        @click="beginEdit"
      >
        ✏ Edit
      </button>
      <button
        class="gncp-btn gncp-btn-danger"
        data-testid="gncp-delete"
        @click="confirmDelete"
      >
        🗑 Delete node
      </button>
    </footer>

    <!-- ── Inline feedback ────────────────────────────────────── -->
    <div
      v-if="feedback"
      :class="['gncp-feedback', `gncp-feedback-${feedbackKind}`]"
      role="status"
    >
      {{ feedback }}
    </div>
  </aside>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue';
import type { MemoryEntry, MemoryEdge, MemoryType } from '../types';
import { useMemoryStore } from '../stores/memory';
import { classifyCognitiveKind } from '../utils/cognitive-kind';

const COMMON_RELATIONS = [
  'related_to',
  'contains',
  'part_of',
  'depends_on',
  'cites',
  'derived_from',
  'supports',
  'contradicts',
  'supersedes',
  'mentions',
  'governs',
  'mother_of',
  'child_of',
] as const;

const COGNITIVE_COLOURS: Record<string, string> = {
  episodic: 'var(--ts-warning)',
  semantic: 'var(--ts-accent-blue-hover)',
  procedural: 'var(--ts-success-dim)',
  judgment: 'var(--ts-accent-violet-hover)',
};

const props = defineProps<{
  entry: MemoryEntry;
  edges: MemoryEdge[];
  allMemories: MemoryEntry[];
}>();

const emit = defineEmits<{
  close: [];
  navigate: [id: number];
  changed: [];
}>();

const store = useMemoryStore();

const isEditing = ref(false);
const editForm = reactive({
  content: '',
  tags: '',
  importance: 3,
  memory_type: 'fact' as MemoryType,
});

const editingEdgeId = ref<number | null>(null);
const edgeEditForm = reactive({ relType: 'related_to', confidence: 1.0 });

const linkForm = reactive({
  direction: 'out' as 'in' | 'out',
  targetQuery: '',
  targetId: null as number | null,
  relType: 'related_to',
  confidence: 1.0,
});
const linkBusy = ref(false);

const feedback = ref('');
const feedbackKind = ref<'ok' | 'err'>('ok');

const kind = computed(() => classifyCognitiveKind(props.entry.memory_type, props.entry.tags ?? '', props.entry.content));
const kindColour = computed(() => COGNITIVE_COLOURS[kind.value] ?? 'var(--ts-accent)');

const tagList = computed(() =>
  (props.entry.tags ?? '')
    .split(',')
    .map((t) => t.trim())
    .filter(Boolean),
);

const parents = computed(() => props.edges.filter((e) => e.dst_id === props.entry.id));
const children = computed(() => props.edges.filter((e) => e.src_id === props.entry.id));

const memoriesById = computed(() => {
  const map = new Map<number, MemoryEntry>();
  for (const m of props.allMemories) map.set(m.id, m);
  return map;
});

const pickedTarget = computed(() =>
  linkForm.targetId !== null ? memoriesById.value.get(linkForm.targetId) ?? null : null,
);

const targetMatches = computed(() => {
  const q = linkForm.targetQuery.trim().toLowerCase();
  if (!q) return [];
  const idMatch = q.startsWith('#') ? Number(q.slice(1)) : Number(q);
  const out: MemoryEntry[] = [];
  for (const m of props.allMemories) {
    if (m.id === props.entry.id) continue;
    if (!Number.isNaN(idMatch) && m.id === idMatch) {
      out.unshift(m);
      continue;
    }
    if (m.content.toLowerCase().includes(q) || (m.tags ?? '').toLowerCase().includes(q)) {
      out.push(m);
    }
    if (out.length >= 8) break;
  }
  return out;
});

function truncate(s: string, max: number) {
  if (!s) return '';
  return s.length > max ? `${s.slice(0, max - 1)}…` : s;
}

function neighbourShort(id: number): string {
  const m = memoriesById.value.get(id);
  return m ? truncate(m.content, 40) : '';
}

function neighbourPreview(id: number): string {
  const m = memoriesById.value.get(id);
  return m ? m.content : `#${id}`;
}

const EDGE_COLOURS = [
  'var(--ts-text-secondary)',
  'var(--ts-warning)',
  'var(--ts-info)',
  'var(--ts-error)',
  'var(--ts-success-dim)',
  'var(--ts-accent)',
  'var(--ts-success)',
  'var(--ts-accent-violet)',
];
function relColour(rel: string): string {
  let h = 0;
  for (let i = 0; i < rel.length; i++) h = ((h << 5) - h + rel.charCodeAt(i)) | 0;
  return EDGE_COLOURS[Math.abs(h) % EDGE_COLOURS.length];
}

// ── Node edit ──────────────────────────────────────────────────
function beginEdit() {
  editForm.content = props.entry.content;
  editForm.tags = props.entry.tags ?? '';
  editForm.importance = props.entry.importance;
  editForm.memory_type = props.entry.memory_type;
  isEditing.value = true;
}

function cancelEdit() {
  isEditing.value = false;
}

async function saveEdit() {
  if (!editForm.content.trim()) return;
  const ok = await store.updateMemory(props.entry.id, { ...editForm });
  if (ok) {
    flash('Node updated', 'ok');
    isEditing.value = false;
    emit('changed');
  } else {
    flash(store.error ?? 'Update failed', 'err');
  }
}

async function confirmDelete() {
  if (!confirm(`Delete node #${props.entry.id}? This also removes all its edges.`)) return;
  const ok = await store.deleteMemory(props.entry.id);
  if (ok) {
    emit('changed');
    emit('close');
  } else {
    flash(store.error ?? 'Delete failed', 'err');
  }
}

// ── Edge edit ──────────────────────────────────────────────────
function beginEdgeEdit(e: MemoryEdge) {
  editingEdgeId.value = e.id;
  edgeEditForm.relType = e.rel_type;
  edgeEditForm.confidence = e.confidence;
}

function cancelEdgeEdit() {
  editingEdgeId.value = null;
}

async function saveEdgeEdit() {
  if (editingEdgeId.value === null) return;
  const updated = await store.updateEdge(editingEdgeId.value, {
    relType: edgeEditForm.relType,
    confidence: edgeEditForm.confidence,
  });
  if (updated) {
    flash('Edge updated', 'ok');
    editingEdgeId.value = null;
    emit('changed');
  } else {
    flash(store.error ?? 'Update failed', 'err');
  }
}

async function deleteEdge(edgeId: number) {
  const ok = await store.deleteEdge(edgeId);
  if (ok) {
    flash('Edge removed', 'ok');
    emit('changed');
  } else {
    flash(store.error ?? 'Delete failed', 'err');
  }
}

async function detachAll() {
  const total = parents.value.length + children.value.length;
  if (!confirm(`Remove all ${total} edges on node #${props.entry.id}?`)) return;
  const n = await store.detachNode(props.entry.id);
  flash(`Detached ${n} edge${n === 1 ? '' : 's'}`, 'ok');
  emit('changed');
}

// ── Link form ──────────────────────────────────────────────────
function onTargetSearch() {
  // Reset picked target when user starts typing again.
  if (linkForm.targetId !== null && linkForm.targetQuery !== `#${linkForm.targetId}`) {
    linkForm.targetId = null;
  }
}

function pickTarget(m: MemoryEntry) {
  linkForm.targetId = m.id;
  linkForm.targetQuery = `#${m.id}`;
}

function clearTarget() {
  linkForm.targetId = null;
  linkForm.targetQuery = '';
}

async function submitLink() {
  if (linkForm.targetId === null) return;
  linkBusy.value = true;
  const src = linkForm.direction === 'out' ? props.entry.id : linkForm.targetId;
  const dst = linkForm.direction === 'out' ? linkForm.targetId : props.entry.id;
  const edge = await store.addEdge(src, dst, linkForm.relType, linkForm.confidence, 'user');
  linkBusy.value = false;
  if (edge) {
    flash('Edge attached', 'ok');
    clearTarget();
    emit('changed');
  } else {
    flash(store.error ?? 'Failed to attach edge', 'err');
  }
}

// ── Feedback helper ────────────────────────────────────────────
let feedbackTimer: ReturnType<typeof setTimeout> | null = null;
function flash(msg: string, kind: 'ok' | 'err') {
  feedback.value = msg;
  feedbackKind.value = kind;
  if (feedbackTimer) clearTimeout(feedbackTimer);
  feedbackTimer = setTimeout(() => {
    feedback.value = '';
  }, 2400);
}

// Reset transient state when the selected entry changes.
watch(
  () => props.entry.id,
  () => {
    isEditing.value = false;
    editingEdgeId.value = null;
    clearTarget();
    feedback.value = '';
  },
);
</script>

<style scoped>
.gncp {
  width: 320px;
  max-width: 100%;
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  padding: 0.9rem;
  background: var(--ts-glass-bg, var(--ts-bg-surface));
  border: 1px solid var(--ts-glass-border, var(--ts-border));
  border-radius: var(--ts-radius-lg, 14px);
  backdrop-filter: var(--ts-glass-blur, blur(12px) saturate(140%));
  -webkit-backdrop-filter: var(--ts-glass-blur, blur(12px) saturate(140%));
  box-shadow: var(--ts-shadow-glow, 0 8px 32px rgba(0, 0, 0, 0.25));
  overflow-y: auto;
  overscroll-behavior: contain;
  font-size: var(--ts-text-sm);
  color: var(--ts-text-primary);
}

.gncp-header {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding-bottom: 0.6rem;
  border-bottom: 1px solid var(--ts-border);
}
.gncp-id-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.gncp-kind-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  flex: none;
  box-shadow: 0 0 6px currentColor;
}
.gncp-id {
  font-family: var(--ts-font-mono, ui-monospace, monospace);
  font-size: var(--ts-text-sm);
  color: var(--ts-text-secondary);
}
.gncp-kind-label {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--ts-text-muted);
  padding: 0.05rem 0.4rem;
  background: var(--ts-bg-hover);
  border-radius: 999px;
}
.gncp-content-preview {
  font-size: var(--ts-text-base, 0.9rem);
  line-height: 1.4;
  color: var(--ts-text-primary);
  display: -webkit-box;
  -webkit-line-clamp: 4;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* Meta */
.gncp-meta {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.4rem 0.85rem;
}
.gncp-stat {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}
.gncp-stat-label {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--ts-text-muted);
}
.gncp-stat-val {
  font-size: var(--ts-text-sm);
  color: var(--ts-text-primary);
}
.gncp-tier {
  font-size: 0.7rem;
  padding: 0.05rem 0.45rem;
  border-radius: 999px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  align-self: flex-start;
  background: var(--ts-bg-hover);
  color: var(--ts-text-primary);
}
.gncp-tier-short {
  background: color-mix(in srgb, var(--ts-warning) 25%, transparent);
  color: var(--ts-warning);
}
.gncp-tier-working {
  background: color-mix(in srgb, var(--ts-info) 25%, transparent);
  color: var(--ts-info);
}
.gncp-tier-long {
  background: color-mix(in srgb, var(--ts-success-dim) 25%, transparent);
  color: var(--ts-success);
}
.gncp-tags {
  grid-column: span 2;
  display: flex;
  flex-wrap: wrap;
  gap: 0.3rem;
}
.gncp-tag {
  font-size: 0.7rem;
  padding: 0.05rem 0.5rem;
  border-radius: 999px;
  background: var(--ts-bg-hover);
  color: var(--ts-text-secondary);
}

/* Section titles */
.gncp-section-title {
  margin: 0;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--ts-text-secondary);
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
.gncp-count {
  background: var(--ts-bg-hover);
  color: var(--ts-text-primary);
  padding: 0.05rem 0.4rem;
  border-radius: 999px;
  font-size: 0.65rem;
  letter-spacing: 0;
  text-transform: none;
}

/* Edges */
.gncp-edges {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}
.gncp-edges-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.gncp-edge-group {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}
.gncp-edge-group-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.7rem;
  color: var(--ts-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.gncp-count-sm {
  font-size: 0.65rem;
  color: var(--ts-text-muted);
}
.gncp-edge-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
.gncp-edge {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.3rem 0.4rem;
  border-radius: 8px;
  background: var(--ts-bg-base);
  border: 1px solid transparent;
  transition: border-color 120ms ease, transform 120ms ease;
}
.gncp-edge:hover {
  border-color: var(--ts-border);
  transform: translateX(2px);
}
.gncp-edge-rel {
  flex: none;
  font-size: 0.7rem;
  padding: 0.1rem 0.45rem;
  border-radius: 999px;
  border: none;
  cursor: pointer;
  color: var(--ts-bg-base);
  font-weight: 600;
}
.gncp-edge-rel:focus-visible,
.gncp-edge-rel-editing {
  outline: 2px solid var(--ts-accent);
  outline-offset: 1px;
}
.gncp-neighbour {
  flex: 1;
  min-width: 0;
  text-align: left;
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
  color: var(--ts-text-primary);
  font-size: 0.75rem;
  display: flex;
  align-items: center;
  gap: 0.35rem;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.gncp-neighbour:hover {
  color: var(--ts-accent);
}
.gncp-neigh-text {
  color: var(--ts-text-muted);
  font-size: 0.7rem;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.gncp-conf {
  flex: none;
  font-size: 0.65rem;
  color: var(--ts-text-muted);
  font-variant-numeric: tabular-nums;
}
.gncp-empty-mini {
  font-size: 0.7rem;
  color: var(--ts-text-muted);
  margin: 0;
  padding: 0.25rem 0.4rem;
  font-style: italic;
}

.gncp-edge-edit {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.55rem;
  border-radius: 10px;
  background: var(--ts-bg-elevated);
  border: 1px solid var(--ts-accent);
}
.gncp-edge-edit-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.55rem;
}

/* Link form */
.gncp-link {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding-top: 0.5rem;
  border-top: 1px solid var(--ts-border);
}
.gncp-link-form {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.gncp-seg {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0;
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--ts-border);
}
.gncp-seg-btn {
  background: var(--ts-bg-base);
  border: none;
  padding: 0.35rem 0.5rem;
  font-size: 0.7rem;
  color: var(--ts-text-secondary);
  cursor: pointer;
  transition: background 120ms ease, color 120ms ease;
}
.gncp-seg-btn:hover {
  background: var(--ts-bg-hover);
  color: var(--ts-text-primary);
}
.gncp-seg-active {
  background: var(--ts-accent);
  color: var(--ts-bg-base);
}

.gncp-target-results {
  list-style: none;
  margin: 0.25rem 0 0;
  padding: 0.2rem;
  border-radius: 8px;
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-elevated);
  max-height: 160px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}
.gncp-target-result {
  display: flex;
  gap: 0.4rem;
  padding: 0.3rem 0.4rem;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.75rem;
}
.gncp-target-result:hover {
  background: var(--ts-bg-hover);
}
.gncp-target-id {
  flex: none;
  font-family: var(--ts-font-mono, ui-monospace, monospace);
  color: var(--ts-accent);
}
.gncp-target-snippet {
  flex: 1;
  min-width: 0;
  color: var(--ts-text-secondary);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.gncp-target-picked {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.3rem 0.5rem;
  border-radius: 8px;
  background: var(--ts-bg-elevated);
  border: 1px solid var(--ts-accent);
  margin-top: 0.25rem;
  font-size: 0.75rem;
}

/* Fields */
.gncp-field {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
.gncp-field-inline {
  flex: 1;
}
.gncp-field-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.55rem;
}
.gncp-field-label {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--ts-text-muted);
}
.gncp-input,
.gncp-textarea {
  background: var(--ts-bg-base);
  border: 1px solid var(--ts-border);
  color: var(--ts-text-primary);
  border-radius: 8px;
  padding: 0.4rem 0.55rem;
  font-size: var(--ts-text-sm);
  width: 100%;
  font-family: inherit;
}
.gncp-input-sm {
  padding: 0.3rem 0.4rem;
  font-size: 0.75rem;
}
.gncp-input:focus,
.gncp-textarea:focus {
  outline: none;
  border-color: var(--ts-accent);
}
.gncp-range {
  width: 100%;
  accent-color: var(--ts-accent);
}

/* Buttons */
.gncp-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.4rem;
  flex-wrap: wrap;
}
.gncp-actions-tight {
  margin-top: 0.1rem;
}
.gncp-btn {
  border: 1px solid var(--ts-border);
  background: var(--ts-bg-base);
  color: var(--ts-text-primary);
  border-radius: 8px;
  padding: 0.4rem 0.85rem;
  font-size: 0.78rem;
  cursor: pointer;
  transition: background 120ms ease, transform 120ms ease, border-color 120ms ease;
}
.gncp-btn-sm {
  padding: 0.3rem 0.65rem;
  font-size: 0.72rem;
}
.gncp-btn:hover:not(:disabled) {
  background: var(--ts-bg-hover);
  transform: translateY(-1px);
}
.gncp-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.gncp-btn-primary {
  background: var(--ts-accent);
  color: var(--ts-bg-base);
  border-color: transparent;
}
.gncp-btn-primary:hover:not(:disabled) {
  background: var(--ts-accent-hover, var(--ts-accent));
  filter: brightness(1.08);
}
.gncp-btn-danger {
  border-color: color-mix(in srgb, var(--ts-danger) 60%, transparent);
  color: var(--ts-danger);
}
.gncp-btn-danger:hover:not(:disabled) {
  background: color-mix(in srgb, var(--ts-danger) 18%, transparent);
}
.gncp-btn-ghost {
  background: transparent;
}
.gncp-link-btn {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 0.7rem;
  color: var(--ts-text-secondary);
  text-decoration: underline;
  padding: 0;
}
.gncp-link-btn:hover {
  color: var(--ts-text-primary);
}
.gncp-link-btn-danger {
  color: var(--ts-danger);
}

.gncp-icon-btn {
  background: none;
  border: none;
  color: var(--ts-text-muted);
  cursor: pointer;
  padding: 0.1rem 0.3rem;
  line-height: 1;
  font-size: 0.9rem;
  border-radius: 6px;
  transition: background 120ms ease, color 120ms ease;
  margin-left: auto;
}
.gncp-icon-btn:hover {
  background: var(--ts-bg-hover);
  color: var(--ts-text-primary);
}
.gncp-icon-btn-danger:hover {
  background: color-mix(in srgb, var(--ts-danger) 18%, transparent);
  color: var(--ts-danger);
}

/* Footer */
.gncp-footer {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  padding-top: 0.5rem;
  border-top: 1px solid var(--ts-border);
}

/* Feedback */
.gncp-feedback {
  margin-top: 0.25rem;
  padding: 0.4rem 0.6rem;
  border-radius: 8px;
  font-size: 0.75rem;
}
.gncp-feedback-ok {
  background: color-mix(in srgb, var(--ts-success) 18%, transparent);
  color: var(--ts-success);
}
.gncp-feedback-err {
  background: color-mix(in srgb, var(--ts-danger) 18%, transparent);
  color: var(--ts-danger);
}

@media (max-width: 840px) {
  .gncp {
    width: 100%;
    height: auto;
  }
}

@media (max-width: 640px) {
  .gncp {
    padding: 0.75rem;
    gap: 0.7rem;
  }
  .gncp-meta,
  .gncp-field-row,
  .gncp-edge-edit-row {
    grid-template-columns: 1fr;
  }
  .gncp-tags {
    grid-column: span 1;
  }
  .gncp-edge {
    align-items: flex-start;
    flex-wrap: wrap;
  }
  .gncp-neighbour,
  .gncp-neigh-text {
    flex-basis: 100%;
  }
  .gncp-footer {
    position: sticky;
    bottom: -0.75rem;
    background: var(--ts-glass-bg, var(--ts-bg-surface));
    padding: 0.65rem 0 0;
  }
  .gncp-footer,
  .gncp-actions {
    justify-content: stretch;
  }
  .gncp-footer .gncp-btn,
  .gncp-actions .gncp-btn {
    flex: 1 1 auto;
  }
}
</style>
