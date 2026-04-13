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
        <button class="btn-primary" @click="showAdd = true">＋ Add memory</button>
      </div>
    </header>

    <p v-if="feedback" class="mv-feedback">{{ feedback }}</p>

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
      <MemoryGraph :memories="store.memories" @select="onNodeSelect" />
      <aside v-if="selectedEntry" class="mv-node-detail">
        <h3>{{ selectedEntry.content }}</h3>
        <p><strong>Type:</strong> {{ selectedEntry.memory_type }}</p>
        <p><strong>Tags:</strong> {{ selectedEntry.tags || '—' }}</p>
        <p><strong>Importance:</strong> {{ '★'.repeat(selectedEntry.importance) }}</p>
        <p><strong>Accessed:</strong> {{ selectedEntry.access_count }}×</p>
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
      </div>

      <div class="mv-filter-row">
        <button
          v-for="t in allTypes"
          :key="t"
          :class="['mv-type-chip', { active: typeFilter === t }]"
          @click="typeFilter = typeFilter === t ? null : t"
        >{{ t }}</button>
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
            <span class="mv-stars">{{ '★'.repeat(m.importance) }}</span>
          </div>
          <p class="mv-content">{{ m.content }}</p>
          <div v-if="m.tags" class="mv-tags">
            <span v-for="tag in m.tags.split(',')" :key="tag" class="mv-tag">{{ tag.trim() }}</span>
          </div>
          <div class="mv-card-footer">
            <span class="mv-ts">{{ formatDate(m.created_at) }}</span>
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
import type { MemoryEntry, MemoryType } from '../types';

const store = useMemoryStore();

const activeTab = ref<'Graph' | 'List' | 'Session'>('List');
const tabs: Array<'Graph' | 'List' | 'Session'> = ['List', 'Graph', 'Session'];
const allTypes: MemoryType[] = ['fact', 'preference', 'context', 'summary'];

// Search & filter
const searchQuery = ref('');
const typeFilter = ref<MemoryType | null>(null);
const searchResults = ref<MemoryEntry[] | null>(null);

const displayedMemories = computed(() => {
  const source = searchResults.value ?? store.memories;
  if (!typeFilter.value) return source;
  return source.filter((m) => m.memory_type === typeFilter.value);
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

// Session tab — uses Message type (chat messages), not MemoryEntry
const shortTerm = ref<import('../types').Message[]>([]);
async function loadShortTerm() {
  shortTerm.value = await store.getShortTermMemory(20);
}

// Graph node selection
const selectedEntry = ref<MemoryEntry | null>(null);
function onNodeSelect(id: number) {
  selectedEntry.value = store.memories.find((m) => m.id === id) ?? null;
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

function formatDate(ts: number) {
  return new Date(ts).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
}

onMounted(async () => {
  await store.fetchAll();
  await loadShortTerm();
});
</script>

<style scoped>
.memory-view { display: flex; flex-direction: column; height: 100%; padding: 1rem; gap: 0.75rem; overflow: hidden; }
.mv-header { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 0.5rem; }
.mv-header h2 { margin: 0; font-size: 1.25rem; }
.mv-header-actions { display: flex; gap: 0.5rem; flex-wrap: wrap; }
.mv-feedback { padding: 0.5rem 1rem; background: #1e3a2f; color: #6ee7b7; border-radius: 6px; margin: 0; }
.mv-tabs { display: flex; gap: 0.25rem; }
.mv-tab { padding: 0.4rem 1rem; border: none; border-radius: 6px; cursor: pointer; background: #1e293b; color: #94a3b8; }
.mv-tab.active { background: #3b82f6; color: #fff; }
.mv-graph-panel { display: flex; flex: 1; gap: 0.75rem; overflow: hidden; min-height: 0; }
.mv-graph-panel > :first-child { flex: 1; min-width: 0; }
.mv-node-detail { width: 240px; padding: 1rem; background: #1e293b; border-radius: 8px; display: flex; flex-direction: column; gap: 0.5rem; overflow-y: auto; }
.mv-node-btns { display: flex; gap: 0.5rem; margin-top: auto; }
.mv-list-panel { flex: 1; display: flex; flex-direction: column; gap: 0.5rem; overflow: hidden; min-height: 0; }
.mv-search-row { display: flex; gap: 0.5rem; }
.mv-search { flex: 1; padding: 0.4rem 0.75rem; background: #1e293b; border: 1px solid #334155; border-radius: 6px; color: #f1f5f9; }
.mv-filter-row { display: flex; gap: 0.4rem; flex-wrap: wrap; }
.mv-type-chip { padding: 0.25rem 0.75rem; border-radius: 999px; border: 1px solid #334155; cursor: pointer; background: #1e293b; color: #94a3b8; font-size: 0.8rem; }
.mv-type-chip.active { background: #3b82f6; color: #fff; border-color: #3b82f6; }
.mv-status { color: #64748b; text-align: center; padding: 2rem; }
.mv-list { list-style: none; margin: 0; padding: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 0.5rem; }
.mv-card { padding: 0.75rem 1rem; background: #1e293b; border-radius: 8px; border-left: 4px solid #475569; display: flex; flex-direction: column; gap: 0.3rem; }
.mv-card.type-fact { border-left-color: #60a5fa; }
.mv-card.type-preference { border-left-color: #34d399; }
.mv-card.type-context { border-left-color: #fbbf24; }
.mv-card.type-summary { border-left-color: #c084fc; }
.mv-card-header { display: flex; justify-content: space-between; }
.mv-chip { font-size: 0.7rem; padding: 0.1rem 0.5rem; background: #334155; border-radius: 4px; }
.mv-stars { color: #fbbf24; font-size: 0.8rem; }
.mv-content { margin: 0; font-size: 0.9rem; }
.mv-tags { display: flex; gap: 0.25rem; flex-wrap: wrap; }
.mv-tag { font-size: 0.7rem; padding: 0.1rem 0.4rem; background: #0f172a; border-radius: 4px; color: #94a3b8; }
.mv-card-footer { display: flex; align-items: center; gap: 0.5rem; margin-top: 0.25rem; }
.mv-ts { font-size: 0.75rem; color: #64748b; flex: 1; }
.mv-session-panel { flex: 1; display: flex; flex-direction: column; gap: 0.5rem; overflow: hidden; min-height: 0; }
.mv-session-hint { color: #64748b; font-size: 0.85rem; margin: 0; }
.mv-session-list { list-style: none; margin: 0; padding: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 0.4rem; }
.mv-session-msg { display: flex; gap: 0.5rem; padding: 0.5rem 0.75rem; border-radius: 6px; background: #1e293b; font-size: 0.85rem; }
.mv-session-msg.user { background: #1e3a5f; }
.mv-modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; }
.mv-modal { background: #1e293b; border-radius: 12px; padding: 1.5rem; width: min(480px, 90vw); display: flex; flex-direction: column; gap: 0.75rem; }
.mv-modal label { display: flex; flex-direction: column; gap: 0.25rem; font-size: 0.85rem; color: #94a3b8; }
.mv-modal input, .mv-modal textarea, .mv-modal select { background: #0f172a; border: 1px solid #334155; border-radius: 6px; padding: 0.4rem 0.75rem; color: #f1f5f9; }
.mv-modal-btns { display: flex; gap: 0.5rem; justify-content: flex-end; }
.btn-primary { padding: 0.4rem 1rem; background: #3b82f6; color: #fff; border: none; border-radius: 6px; cursor: pointer; }
.btn-secondary { padding: 0.4rem 1rem; background: #334155; color: #f1f5f9; border: none; border-radius: 6px; cursor: pointer; }
.btn-icon { background: none; border: none; cursor: pointer; padding: 0.2rem 0.4rem; }
.btn-icon.danger { color: #f87171; }
.btn-danger { padding: 0.35rem 0.75rem; background: #7f1d1d; color: #fca5a5; border: none; border-radius: 6px; cursor: pointer; }
</style>
