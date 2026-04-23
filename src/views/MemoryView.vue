<template>
  <div class="memory-view">
    <header class="mv-header">
      <h2>🧠 Memory</h2>
      <div class="mv-header-actions">
        <button class="btn-secondary" @click="handleExtract" :disabled="isActing">
          {{ isActing ? 'Working…' : '⬇ Extract from session' }}
        </button>
        <button class="btn-secondary" @click="handleSummarize" :disabled="isActing">
          📄 Summarize session
        </button>
        <button class="btn-secondary" @click="handleDecay" :disabled="isActing" title="Apply time-decay to all memories">
          ⏳ Decay
        </button>
        <button class="btn-secondary" @click="handleGC" :disabled="isActing" title="Remove fully decayed memories">
          🧹 GC
        </button>
        <button class="btn-primary" @click="showAdd = true">＋ Add memory</button>
      </div>
    </header>

    <p v-if="feedback" class="mv-feedback">{{ feedback }}</p>

    <!-- Stats dashboard -->
    <div v-if="store.stats" class="mv-stats">
      <div class="mv-stat">
        <span class="mv-stat-value">{{ store.stats.total }}</span>
        <span class="mv-stat-label">Total</span>
      </div>
      <div class="mv-stat tier-short">
        <span class="mv-stat-value">{{ store.stats.short_count }}</span>
        <span class="mv-stat-label">Short</span>
      </div>
      <div class="mv-stat tier-working">
        <span class="mv-stat-value">{{ store.stats.working_count }}</span>
        <span class="mv-stat-label">Working</span>
      </div>
      <div class="mv-stat tier-long">
        <span class="mv-stat-value">{{ store.stats.long_count }}</span>
        <span class="mv-stat-label">Long</span>
      </div>
      <div class="mv-stat">
        <span class="mv-stat-value">{{ formatTokens(store.stats.total_tokens) }}</span>
        <span class="mv-stat-label">Tokens</span>
      </div>
      <div class="mv-stat">
        <span class="mv-stat-value">{{ store.stats.avg_decay.toFixed(2) }}</span>
        <span class="mv-stat-label">Avg Decay</span>
      </div>
    </div>

    <!-- Tabs -->
    <nav class="mv-tabs">
      <button
        v-for="tab in tabs"
        :key="tab"
        :class="['mv-tab', { active: activeTab === tab }]"
        @click="activeTab = tab"
      >{{ tab }}</button>
    </nav>

    <!-- ── Graph tab ── -->
    <div v-if="activeTab === 'Graph'" class="mv-graph-panel">
      <div class="mv-graph-main">
        <div class="mv-graph-toolbar">
          <label class="mv-graph-toggle">
            <span>Edges:</span>
            <select v-model="edgeMode" class="mv-edge-mode">
              <option value="typed">Typed (knowledge graph)</option>
              <option value="tag">Tag co-occurrence</option>
              <option value="both">Both</option>
            </select>
          </label>
          <button
            class="btn-secondary"
            @click="handleExtractEdges"
            :disabled="isActing || store.memories.length < 2"
            :title="store.memories.length < 2 ? 'Add at least 2 memories first' : 'Use the brain to propose edges'"
          >🔗 Extract edges</button>
          <span v-if="store.edgeStats" class="mv-edge-counter">
            {{ store.edgeStats.total_edges }} edge{{ store.edgeStats.total_edges === 1 ? '' : 's' }}
            · {{ store.edgeStats.connected_memories }} connected
          </span>
        </div>
        <MemoryGraph
          :memories="store.memories"
          :edges="store.edges"
          :edge-mode="edgeMode"
          @select="onNodeSelect"
        />
      </div>
      <aside v-if="selectedEntry" class="mv-node-detail">
        <h3>{{ selectedEntry.content }}</h3>
        <p><strong>Type:</strong> {{ selectedEntry.memory_type }}</p>
        <p><strong>Tier:</strong> <span :class="'mv-tier-badge tier-' + selectedEntry.tier">{{ selectedEntry.tier }}</span></p>
        <p><strong>Tags:</strong> {{ selectedEntry.tags || '—' }}</p>
        <p><strong>Importance:</strong> {{ '★'.repeat(selectedEntry.importance) }}</p>
        <p><strong>Decay:</strong> {{ (selectedEntry.decay_score * 100).toFixed(0) }}%</p>
        <p><strong>Accessed:</strong> {{ selectedEntry.access_count }}×</p>
        <div v-if="selectedEdges.length" class="mv-node-edges">
          <strong>Edges ({{ selectedEdges.length }}):</strong>
          <ul>
            <li v-for="e in selectedEdges" :key="e.id" class="mv-node-edge">
              <span class="mv-rel-pill">{{ e.rel_type }}</span>
              <span class="mv-edge-direction">
                {{ e.src_id === selectedEntry.id ? '→' : '←' }}
                #{{ e.src_id === selectedEntry.id ? e.dst_id : e.src_id }}
              </span>
              <button class="mv-edge-del" title="Delete edge" @click="handleDeleteEdge(e.id)">×</button>
            </li>
          </ul>
        </div>
        <div class="mv-node-btns">
          <button class="btn-secondary" @click="startEdit(selectedEntry)">✏ Edit</button>
          <button class="btn-danger" @click="confirmDelete(selectedEntry.id)">🗑 Delete</button>
        </div>
      </aside>
    </div>

    <!-- ── List tab ── -->
    <div v-else-if="activeTab === 'List'" class="mv-list-panel">
      <div class="mv-search-row">
        <input
          v-model="searchQuery"
          placeholder="Search memories…"
          class="mv-search"
          @keyup.enter="doSearch"
        />
        <button class="btn-secondary" @click="doSearch">🔍 Search</button>
        <button class="btn-secondary" @click="doSemanticSearch" title="Brain-powered semantic search">
          🤖 Semantic
        </button>
        <button class="btn-primary" @click="doHybridSearch" title="6-signal hybrid search">
          ⚡ Hybrid
        </button>
      </div>

      <div class="mv-filter-row">
        <span class="mv-filter-label">Type:</span>
        <button
          v-for="t in allTypes"
          :key="t"
          :class="['mv-type-chip', { active: typeFilter === t }]"
          @click="typeFilter = typeFilter === t ? null : t"
        >{{ t }}</button>
        <span class="mv-filter-divider">|</span>
        <span class="mv-filter-label">Tier:</span>
        <button
          v-for="tier in allTiers"
          :key="tier"
          :class="['mv-tier-chip', 'tier-' + tier, { active: tierFilter === tier }]"
          @click="tierFilter = tierFilter === tier ? null : tier"
        >{{ tier }}</button>
      </div>

      <p v-if="store.isLoading" class="mv-status">Loading…</p>
      <p v-else-if="displayedMemories.length === 0" class="mv-status">No memories yet.</p>

      <ul v-else class="mv-list">
        <li
          v-for="m in displayedMemories"
          :key="m.id"
          :class="['mv-card', `type-${m.memory_type}`]"
        >
          <div class="mv-card-header">
            <span class="mv-chip">{{ m.memory_type }}</span>
            <span :class="'mv-tier-badge tier-' + m.tier">{{ m.tier }}</span>
            <span class="mv-stars">{{ '★'.repeat(m.importance) }}</span>
            <span class="mv-decay-bar" :title="'Decay: ' + (m.decay_score * 100).toFixed(0) + '%'">
              <span class="mv-decay-fill" :style="{ width: (m.decay_score * 100) + '%' }" />
            </span>
          </div>
          <p class="mv-content">{{ m.content }}</p>
          <div v-if="m.tags" class="mv-tags">
            <span v-for="tag in m.tags.split(',')" :key="tag" class="mv-tag">{{ tag.trim() }}</span>
          </div>
          <div class="mv-card-footer">
            <span class="mv-ts">{{ formatDate(m.created_at) }}</span>
            <span v-if="m.token_count" class="mv-token-count" title="Token count">{{ m.token_count }}t</span>
            <button
              v-if="m.tier !== 'long'"
              class="btn-icon"
              @click="handlePromote(m.id, promoteTier(m.tier))"
              :title="'Promote to ' + promoteTier(m.tier)"
            >⬆</button>
            <button class="btn-icon" @click="startEdit(m)" title="Edit">✏</button>
            <button class="btn-icon danger" @click="confirmDelete(m.id)" title="Delete">🗑</button>
          </div>
        </li>
      </ul>
    </div>

    <!-- ── Session tab ── -->
    <div v-else class="mv-session-panel">
      <p class="mv-session-hint">
        Short-term memory — the last 20 messages of the current session that the brain reads
        before every reply.
      </p>
      <p v-if="shortTerm.length === 0" class="mv-status">No conversation yet.</p>
      <ul v-else class="mv-session-list">
        <li v-for="msg in shortTerm" :key="msg.id" :class="['mv-session-msg', msg.role]">
          <strong>{{ msg.role === 'user' ? 'You' : '🤖' }}</strong>
          <span>{{ msg.content }}</span>
        </li>
      </ul>
    </div>

    <!-- Add / Edit modal -->
    <div v-if="showAdd || editTarget" class="mv-modal-backdrop" @click.self="closeModal">
      <div class="mv-modal">
        <h3>{{ editTarget ? 'Edit memory' : 'Add memory' }}</h3>
        <label>Content
          <textarea v-model="form.content" rows="3" placeholder="What should I remember?" />
        </label>
        <label>Tags (comma-separated)
          <input v-model="form.tags" placeholder="python, work, project" />
        </label>
        <label>Type
          <select v-model="form.memory_type">
            <option v-for="t in allTypes" :key="t" :value="t">{{ t }}</option>
          </select>
        </label>
        <label>Importance (1–5)
          <input v-model.number="form.importance" type="range" min="1" max="5" />
          <span>{{ form.importance }}</span>
        </label>
        <div class="mv-modal-btns">
          <button class="btn-primary" @click="saveMemory">Save</button>
          <button class="btn-secondary" @click="closeModal">Cancel</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useMemoryStore } from '../stores/memory';
import MemoryGraph from '../components/MemoryGraph.vue';
import type { MemoryEntry, MemoryType, MemoryTier } from '../types';

const store = useMemoryStore();

const activeTab = ref<'Graph' | 'List' | 'Session'>('List');
const tabs: Array<'Graph' | 'List' | 'Session'> = ['List', 'Graph', 'Session'];
const allTypes: MemoryType[] = ['fact', 'preference', 'context', 'summary'];
const allTiers: MemoryTier[] = ['short', 'working', 'long'];

// Search & filter
const searchQuery = ref('');
const typeFilter = ref<MemoryType | null>(null);
const tierFilter = ref<MemoryTier | null>(null);
const searchResults = ref<MemoryEntry[] | null>(null);

const displayedMemories = computed(() => {
  const source = searchResults.value ?? store.memories;
  return source.filter((m) => {
    if (typeFilter.value && m.memory_type !== typeFilter.value) return false;
    if (tierFilter.value && m.tier !== tierFilter.value) return false;
    return true;
  });
});

async function doSearch() {
  if (!searchQuery.value.trim()) {
    searchResults.value = null;
    return;
  }
  searchResults.value = await store.search(searchQuery.value);
}

async function doSemanticSearch() {
  if (!searchQuery.value.trim()) return;
  searchResults.value = await store.semanticSearch(searchQuery.value);
}

async function doHybridSearch() {
  if (!searchQuery.value.trim()) return;
  searchResults.value = await store.hybridSearch(searchQuery.value);
}

// Session tab — uses Message type (chat messages), not MemoryEntry
const shortTerm = ref<import('../types').Message[]>([]);
async function loadShortTerm() {
  shortTerm.value = await store.getShortTermMemory(20);
}

// Graph node selection
const selectedEntry = ref<MemoryEntry | null>(null);
const edgeMode = ref<'typed' | 'tag' | 'both'>('typed');

const selectedEdges = computed(() => {
  if (!selectedEntry.value) return [];
  const id = selectedEntry.value.id;
  return store.edges.filter((e) => e.src_id === id || e.dst_id === id);
});

function onNodeSelect(id: number) {
  selectedEntry.value = store.memories.find((m) => m.id === id) ?? null;
}

async function handleExtractEdges() {
  isActing.value = true;
  feedback.value = '';
  const count = await store.extractEdgesViaBrain();
  feedback.value = count > 0
    ? `🔗 ${count} new edge${count === 1 ? '' : 's'} extracted by the brain.`
    : '🤔 No new edges proposed (or brain unreachable).';
  await store.getEdgeStats();
  isActing.value = false;
  setTimeout(() => (feedback.value = ''), 4000);
}

async function handleDeleteEdge(edgeId: number) {
  await store.deleteEdge(edgeId);
  await store.getEdgeStats();
}

// Add / Edit modal
const showAdd = ref(false);
const editTarget = ref<MemoryEntry | null>(null);
const form = ref({ content: '', tags: '', importance: 3, memory_type: 'fact' as MemoryType });

function startEdit(m: MemoryEntry) {
  editTarget.value = m;
  form.value = { content: m.content, tags: m.tags, importance: m.importance, memory_type: m.memory_type };
}

function closeModal() {
  showAdd.value = false;
  editTarget.value = null;
  form.value = { content: '', tags: '', importance: 3, memory_type: 'fact' };
}

function promoteTier(current: MemoryTier): MemoryTier {
  return current === 'short' ? 'working' : 'long';
}

async function handlePromote(id: number, tier: MemoryTier) {
  await store.promoteMemory(id, tier);
  await store.getStats();
}

async function saveMemory() {
  if (!form.value.content.trim()) return;
  if (editTarget.value) {
    await store.updateMemory(editTarget.value.id, { ...form.value });
  } else {
    await store.addMemory({ ...form.value });
  }
  closeModal();
}

async function confirmDelete(id: number) {
  if (confirm('Delete this memory?')) {
    await store.deleteMemory(id);
    selectedEntry.value = null;
  }
}

// Brain actions
const isActing = ref(false);
const feedback = ref('');

async function handleExtract() {
  isActing.value = true;
  feedback.value = '';
  const count = await store.extractFromSession();
  feedback.value = count > 0 ? `✅ ${count} new memories extracted.` : '🤔 Nothing new to remember.';
  isActing.value = false;
  setTimeout(() => (feedback.value = ''), 4000);
}

async function handleSummarize() {
  isActing.value = true;
  feedback.value = '';
  const summary = await store.summarizeSession();
  feedback.value = summary ? `✅ Session summarized and saved.` : '❌ Could not summarize. Is the brain active?';
  isActing.value = false;
  setTimeout(() => (feedback.value = ''), 4000);
}

async function handleDecay() {
  isActing.value = true;
  feedback.value = '';
  const count = await store.applyDecay();
  feedback.value = count > 0 ? `⏳ Decay applied to ${count} memories.` : '✅ All memories already decayed.';
  isActing.value = false;
  await store.getStats();
  setTimeout(() => (feedback.value = ''), 4000);
}

async function handleGC() {
  isActing.value = true;
  feedback.value = '';
  const count = await store.gcMemories();
  feedback.value = count > 0 ? `🧹 ${count} decayed memories removed.` : '✅ Nothing to clean up.';
  isActing.value = false;
  await store.getStats();
  setTimeout(() => (feedback.value = ''), 4000);
}

function formatDate(ts: number) {
  return new Date(ts).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
}

function formatTokens(n: number) {
  return n >= 1000 ? (n / 1000).toFixed(1) + 'k' : String(n);
}

onMounted(async () => {
  await store.fetchAll();
  await Promise.all([loadShortTerm(), store.getStats(), store.fetchEdges(), store.getEdgeStats()]);
});
</script>

<style scoped>
.memory-view { display: flex; flex-direction: column; height: 100%; padding: 1rem; gap: 0.75rem; overflow: hidden; }
.mv-header { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 0.5rem; }
.mv-header h2 { margin: 0; font-size: 1.25rem; }
.mv-header-actions { display: flex; gap: 0.5rem; flex-wrap: wrap; }
.mv-feedback { padding: 0.5rem 1rem; background: var(--ts-success-bg); color: var(--ts-success); border-radius: 6px; margin: 0; }

/* ── Stats dashboard ── */
.mv-stats { display: flex; gap: 0.5rem; flex-wrap: wrap; }
.mv-stat { display: flex; flex-direction: column; align-items: center; padding: 0.4rem 0.75rem; background: var(--ts-bg-surface); border-radius: 8px; border: 1px solid var(--ts-border); min-width: 60px; }
.mv-stat-value { font-size: 1.1rem; font-weight: 700; color: var(--ts-text-primary); }
.mv-stat-label { font-size: 0.7rem; color: var(--ts-text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
.mv-stat.tier-short { border-color: var(--ts-warning); }
.mv-stat.tier-short .mv-stat-value { color: var(--ts-warning); }
.mv-stat.tier-working { border-color: var(--ts-accent-blue); }
.mv-stat.tier-working .mv-stat-value { color: var(--ts-accent-blue); }
.mv-stat.tier-long { border-color: var(--ts-success); }
.mv-stat.tier-long .mv-stat-value { color: var(--ts-success); }

.mv-tabs { display: flex; gap: 0.25rem; }
.mv-tab { padding: 0.4rem 1rem; border: none; border-radius: 6px; cursor: pointer; background: var(--ts-bg-surface); color: var(--ts-text-secondary); font-size: var(--ts-text-sm); transition: background var(--ts-transition-fast), color var(--ts-transition-fast); }
.mv-tab:hover { background: var(--ts-bg-hover); color: var(--ts-text-primary); }
.mv-tab.active { background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); }
.mv-graph-panel { display: flex; flex: 1; gap: 0.75rem; overflow: hidden; min-height: 0; }
.mv-graph-panel > :first-child { flex: 1; min-width: 0; }
.mv-graph-main { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.5rem; }
.mv-graph-toolbar { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; padding: 0.4rem 0; }
.mv-graph-toggle { display: flex; align-items: center; gap: 0.4rem; font-size: var(--ts-text-sm); color: var(--ts-text-secondary); }
.mv-edge-mode { background: var(--ts-bg-surface); color: var(--ts-text-primary); border: 1px solid var(--ts-border); border-radius: 6px; padding: 0.25rem 0.5rem; font-size: var(--ts-text-sm); }
.mv-edge-counter { font-size: var(--ts-text-sm); color: var(--ts-text-muted); }
.mv-graph-main > .memory-graph { flex: 1; }
.mv-node-edges { display: flex; flex-direction: column; gap: 0.3rem; padding-top: 0.4rem; border-top: 1px solid var(--ts-border); }
.mv-node-edges ul { list-style: none; padding: 0; margin: 0.3rem 0 0; display: flex; flex-direction: column; gap: 0.25rem; }
.mv-node-edge { display: flex; align-items: center; gap: 0.4rem; font-size: var(--ts-text-sm); color: var(--ts-text-secondary); }
.mv-rel-pill { background: var(--ts-bg-hover); color: var(--ts-text-primary); padding: 0.05rem 0.5rem; border-radius: 999px; font-size: 0.7rem; }
.mv-edge-direction { color: var(--ts-text-muted); font-family: monospace; }
.mv-edge-del { background: none; border: none; color: var(--ts-text-muted); cursor: pointer; padding: 0 0.25rem; line-height: 1; font-size: 1rem; margin-left: auto; }
.mv-edge-del:hover { color: var(--ts-danger); }
.mv-node-detail { width: 240px; padding: 1rem; background: var(--ts-bg-surface); border: 1px solid var(--ts-border); border-radius: 8px; display: flex; flex-direction: column; gap: 0.5rem; overflow-y: auto; }
.mv-node-btns { display: flex; gap: 0.5rem; margin-top: auto; }
.mv-list-panel { flex: 1; display: flex; flex-direction: column; gap: 0.5rem; overflow: hidden; min-height: 0; }
.mv-search-row { display: flex; gap: 0.5rem; }
.mv-search { flex: 1; padding: 0.4rem 0.75rem; background: var(--ts-bg-input); border: 1px solid var(--ts-border-medium); border-radius: 6px; color: var(--ts-text-primary); outline: none; transition: border-color var(--ts-transition-fast); }
.mv-search:focus { border-color: var(--ts-accent-blue); }
.mv-search::placeholder { color: var(--ts-text-dim); }
.mv-filter-row { display: flex; gap: 0.4rem; flex-wrap: wrap; align-items: center; }
.mv-filter-label { font-size: 0.75rem; color: var(--ts-text-muted); text-transform: uppercase; letter-spacing: 0.04em; }
.mv-filter-divider { color: var(--ts-text-dim); font-size: 0.85rem; margin: 0 0.15rem; }
.mv-type-chip { padding: 0.25rem 0.75rem; border-radius: 999px; border: 1px solid var(--ts-border-medium); cursor: pointer; background: var(--ts-bg-surface); color: var(--ts-text-secondary); font-size: 0.8rem; transition: background var(--ts-transition-fast), color var(--ts-transition-fast), border-color var(--ts-transition-fast); }
.mv-type-chip:hover { background: var(--ts-bg-hover); color: var(--ts-text-primary); }
.mv-type-chip.active { background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); border-color: var(--ts-accent-blue-hover); }

/* ── Tier chips & badges ── */
.mv-tier-chip { padding: 0.25rem 0.75rem; border-radius: 999px; border: 1px solid var(--ts-border-medium); cursor: pointer; background: var(--ts-bg-surface); color: var(--ts-text-secondary); font-size: 0.8rem; transition: background var(--ts-transition-fast), color var(--ts-transition-fast), border-color var(--ts-transition-fast); }
.mv-tier-chip:hover { background: var(--ts-bg-hover); }
.mv-tier-chip.tier-short { border-color: var(--ts-warning); color: var(--ts-warning); }
.mv-tier-chip.tier-working { border-color: var(--ts-accent-blue); color: var(--ts-accent-blue); }
.mv-tier-chip.tier-long { border-color: var(--ts-success); color: var(--ts-success); }
.mv-tier-chip.active.tier-short { background: var(--ts-warning); color: var(--ts-bg-base); }
.mv-tier-chip.active.tier-working { background: var(--ts-accent-blue); color: var(--ts-bg-base); }
.mv-tier-chip.active.tier-long { background: var(--ts-success); color: var(--ts-bg-base); }
.mv-tier-badge { font-size: 0.65rem; padding: 0.1rem 0.45rem; border-radius: 4px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em; }
.mv-tier-badge.tier-short { background: rgba(251,191,36,0.15); color: var(--ts-warning); }
.mv-tier-badge.tier-working { background: rgba(96,165,250,0.15); color: var(--ts-accent-blue); }
.mv-tier-badge.tier-long { background: rgba(74,222,128,0.15); color: var(--ts-success); }

.mv-status { color: var(--ts-text-muted); text-align: center; padding: 2rem; }
.mv-list { list-style: none; margin: 0; padding: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 0.5rem; }
.mv-card { padding: 0.75rem 1rem; background: var(--ts-bg-surface); border-radius: 8px; border-left: 4px solid var(--ts-text-muted); display: flex; flex-direction: column; gap: 0.3rem; transition: background var(--ts-transition-fast); }
.mv-card:hover { background: var(--ts-bg-elevated); }
.mv-card.type-fact { border-left-color: var(--ts-accent-blue); }
.mv-card.type-preference { border-left-color: var(--ts-success); }
.mv-card.type-context { border-left-color: var(--ts-warning); }
.mv-card.type-summary { border-left-color: var(--ts-accent-violet); }
.mv-card-header { display: flex; align-items: center; gap: 0.5rem; }
.mv-chip { font-size: 0.7rem; padding: 0.1rem 0.5rem; background: var(--ts-bg-elevated); border-radius: 4px; color: var(--ts-text-secondary); }
.mv-stars { color: var(--ts-warning); font-size: 0.8rem; }
.mv-content { margin: 0; font-size: 0.9rem; color: var(--ts-text-primary); }
.mv-tags { display: flex; gap: 0.25rem; flex-wrap: wrap; }
.mv-tag { font-size: 0.7rem; padding: 0.1rem 0.4rem; background: var(--ts-bg-base); border-radius: 4px; color: var(--ts-text-secondary); }
.mv-card-footer { display: flex; align-items: center; gap: 0.5rem; margin-top: 0.25rem; }
.mv-ts { font-size: 0.75rem; color: var(--ts-text-muted); flex: 1; }
.mv-token-count { font-size: 0.7rem; color: var(--ts-text-dim); }

/* ── Decay bar ── */
.mv-decay-bar { width: 40px; height: 6px; background: var(--ts-bg-base); border-radius: 3px; overflow: hidden; margin-left: auto; }
.mv-decay-fill { display: block; height: 100%; background: var(--ts-success); border-radius: 3px; transition: width 0.3s ease; }

.mv-session-panel { flex: 1; display: flex; flex-direction: column; gap: 0.5rem; overflow: hidden; min-height: 0; }
.mv-session-hint { color: var(--ts-text-muted); font-size: 0.85rem; margin: 0; }
.mv-session-list { list-style: none; margin: 0; padding: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 0.4rem; }
.mv-session-msg { display: flex; gap: 0.5rem; padding: 0.5rem 0.75rem; border-radius: 6px; background: var(--ts-bg-surface); font-size: 0.85rem; color: var(--ts-text-primary); }
.mv-session-msg.user { background: rgba(96, 165, 250, 0.10); }
.mv-modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; backdrop-filter: blur(4px); }
.mv-modal { background: var(--ts-bg-surface); border: 1px solid var(--ts-border); border-radius: 12px; padding: 1.5rem; width: min(480px, 90vw); display: flex; flex-direction: column; gap: 0.75rem; box-shadow: var(--ts-shadow-lg); }
.mv-modal h3 { color: var(--ts-text-primary); }
.mv-modal label { display: flex; flex-direction: column; gap: 0.25rem; font-size: 0.85rem; color: var(--ts-text-secondary); }
.mv-modal input, .mv-modal textarea, .mv-modal select { background: var(--ts-bg-input); border: 1px solid var(--ts-border-medium); border-radius: 6px; padding: 0.4rem 0.75rem; color: var(--ts-text-primary); outline: none; transition: border-color var(--ts-transition-fast); }
.mv-modal input:focus, .mv-modal textarea:focus, .mv-modal select:focus { border-color: var(--ts-accent-blue); }
.mv-modal-btns { display: flex; gap: 0.5rem; justify-content: flex-end; }
.btn-primary { padding: 0.4rem 1rem; background: var(--ts-accent-blue-hover); color: var(--ts-text-on-accent); border: none; border-radius: 6px; cursor: pointer; transition: background var(--ts-transition-fast); }
.btn-primary:hover { background: var(--ts-accent-blue); }
.btn-secondary { padding: 0.4rem 1rem; background: var(--ts-bg-elevated); color: var(--ts-text-primary); border: none; border-radius: 6px; cursor: pointer; transition: background var(--ts-transition-fast); }
.btn-secondary:hover { background: var(--ts-bg-hover); }
.btn-icon { background: none; border: none; cursor: pointer; padding: 0.2rem 0.4rem; color: var(--ts-text-muted); transition: color var(--ts-transition-fast); }
.btn-icon:hover { color: var(--ts-text-primary); }
.btn-icon.danger { color: var(--ts-error); }
.btn-danger { padding: 0.35rem 0.75rem; background: var(--ts-error-bg); color: var(--ts-error); border: none; border-radius: 6px; cursor: pointer; }
</style>
